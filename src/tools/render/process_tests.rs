use super::super::fixtures;
use super::*;
use crate::library::attempts::Attempt;
use crate::tools::{registration::Descriptor, render::ResolvedRender};
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};
pub(crate) fn setup(mode: &str) -> (tempfile::TempDir, ResolvedRender) {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    let bin = BIN.get_or_init(|| {
        assert!(
            crate::process::spawn_std(std::process::Command::new(env!("CARGO")).args([
                "build",
                "--locked",
                "--example",
                "sample-lab"
            ]))
            .unwrap()
            .wait()
            .unwrap()
            .success()
        );
        Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab")
    });
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let run = root.join("saved run");
    std::fs::create_dir(&run).unwrap();
    std::fs::write(run.join("untouched"), "saved evidence").unwrap();
    let descriptor = Descriptor {
        id: "sample".into(),
        executable: bin.clone(),
        args: vec![],
        config: None,
        cwd: root.clone(),
        roots: vec![root],
    }
    .validate()
    .unwrap();
    let mut document = fixtures::description("0.3.0");
    document["actions"][0]["renders"] = json!([{"name":"test","kind":"document","command":["sample-lab","--render-test",mode,"{run.dir}","$(literal); value"]}]);
    let mut result = fixtures::result("completed");
    result["run"]["dir"] = json!(run);
    let attempt: Attempt = serde_json::from_value(json!({"id":"saved","sequence":1,"created_ms":0,
      "request":{"id":"saved","tool":"sample","action":"mark_band","revision":"saved","params":{},"images":{}},
      "arguments":{},"declaration":{"document":document},"inputs":{},"wire":null,"execution":"completed",
      "result":result,"preservation":{"contract":"complete"},"outputs":[],"journal":null,"notices":[]})).unwrap();
    let resolved =
        ResolvedRender::resolve(&descriptor, &attempt, "test", "http://127.0.0.1:4801").unwrap();
    (temp, resolved)
}
async fn wait(process: &RenderProcess, predicate: impl Fn(&ProcessState) -> bool) -> ProcessState {
    let mut state = process.observe();
    tokio::time::timeout(Duration::from_secs(5), state.wait_for(predicate))
        .await
        .unwrap()
        .unwrap()
        .clone()
}
#[tokio::test]
async fn renderer_executes_literal_args_in_a_minimal_environment_without_mutating_the_run() {
    let (_temp, plan) = setup("echo");
    let process = RenderProcess::spawn(&plan).unwrap();
    let state = wait(&process, |s| s.phase == ProcessPhase::Exited).await;
    assert_eq!(state.exit_code, Some(0));
    let output: serde_json::Value = serde_json::from_slice(&state.stdout.bytes).unwrap();
    assert_eq!(output["args"], json!(["$(literal); value"]));
    for name in output["env_names"].as_array().unwrap() {
        assert!(
            [
                "HOME",
                "XDG_CONFIG_HOME",
                "XDG_DATA_HOME",
                "XDG_CACHE_HOME",
                "TMPDIR",
                "LANG",
                "LC_ALL",
                "PATH"
            ]
            .contains(&name.as_str().unwrap())
        );
    }
    assert_eq!(
        std::fs::read_to_string(plan.run_dir().join("untouched")).unwrap(),
        "saved evidence"
    );
    assert_eq!(std::fs::read_dir(plan.run_dir()).unwrap().count(), 1);
}
#[tokio::test]
async fn closing_stdin_stops_a_conforming_renderer_and_dropping_the_handle_also_requests_close() {
    for drop_handle in [false, true] {
        let (_temp, plan) = setup("hold");
        let process = RenderProcess::spawn(&plan).unwrap();
        wait(&process, |s| !s.stdout.bytes.is_empty()).await;
        assert_eq!(process.snapshot().phase, ProcessPhase::Running);
        let mut observer = process.observe();
        if drop_handle {
            drop(process);
        } else {
            assert!(process.close().await.unwrap());
        }
        let state = tokio::time::timeout(
            Duration::from_secs(5),
            observer.wait_for(|s| s.phase == ProcessPhase::Exited),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(state.exit_code, Some(0));
    }
}
#[tokio::test]
async fn output_limits_and_early_exit_preserve_bounded_diagnostics() {
    for (mode, stdout) in [("stdout-flood", true), ("stderr-flood", false)] {
        let (_temp, plan) = setup(mode);
        let process = RenderProcess::spawn(&plan).unwrap();
        let state = wait(&process, |s| s.phase == ProcessPhase::Exited).await;
        let output = if stdout { state.stdout } else { state.stderr };
        assert!(output.overflow);
        assert!(output.finished);
        assert_eq!(
            output.bytes.len(),
            if stdout { STDOUT_LIMIT } else { STDERR_LIMIT }
        );
    }
    let (_temp, plan) = setup("early-error");
    let process = RenderProcess::spawn(&plan).unwrap();
    let state = wait(&process, |s| s.phase == ProcessPhase::Exited).await;
    assert_eq!(state.exit_code, Some(17));
    assert!(String::from_utf8_lossy(&state.stderr.bytes).contains("Synthetic render failure"));
}
#[tokio::test]
async fn stuck_renderer_remains_owned_until_an_explicit_force_stop() {
    let (_temp, plan) = setup("stuck");
    let process = RenderProcess::spawn(&plan).unwrap();
    wait(&process, |s| !s.stdout.bytes.is_empty()).await;
    assert!(process.force_stop().await.is_err());
    assert!(!process.close().await.unwrap());
    assert_eq!(process.snapshot().phase, ProcessPhase::NeedsForce);
    assert!(process.force_stop().await.unwrap());
    assert_eq!(process.snapshot().phase, ProcessPhase::Exited);
    assert_ne!(process.snapshot().exit_code, Some(0));
}

#[tokio::test]
async fn stdin_ignoring_renderer_exits_cleanly_on_graceful_sigterm() {
    let (_temp, plan) = setup("term");
    let process = RenderProcess::spawn(&plan).unwrap();
    wait(&process, |s| !s.stdout.bytes.is_empty()).await;
    assert!(process.close().await.unwrap());
    assert_eq!(process.snapshot().exit_code, Some(0));
}

#[tokio::test]
async fn surviving_descendants_hold_the_group_until_graceful_or_explicit_force_cleanup() {
    for mode in ["descendant", "orphan"] {
        let (temp, plan) = setup(mode);
        let process = RenderProcess::spawn(&plan).unwrap();
        wait(&process, |s| !s.stdout.bytes.is_empty()).await;
        let child: i32 = std::fs::read_to_string(temp.path().join("descendant-ready"))
            .unwrap()
            .parse()
            .unwrap();
        if mode == "descendant" {
            assert!(process.close().await.unwrap());
        } else {
            // The leader exited, but its SIGTERM-ignoring descendant must retain the slot.
            wait(&process, |s| s.phase == ProcessPhase::NeedsForce).await;
            assert!(process.force_stop().await.unwrap());
        }
        assert_eq!(process.snapshot().phase, ProcessPhase::Exited);
        assert_eq!(process.snapshot().exit_code, Some(0));
        let observed = crate::process::spawn_std(
            std::process::Command::new("/bin/ps")
                .args(["-p", &child.to_string(), "-o", "stat="])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped()),
        )
        .unwrap()
        .wait_with_output()
        .unwrap();
        assert!(
            observed.stdout.is_empty()
                || String::from_utf8_lossy(&observed.stdout)
                    .trim()
                    .starts_with('Z'),
            "descendant is still executing"
        );
        assert_eq!(
            std::fs::read_to_string(plan.run_dir().join("untouched")).unwrap(),
            "saved evidence"
        );
    }
}
