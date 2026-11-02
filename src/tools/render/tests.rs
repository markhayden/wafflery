use super::*;
use serde_json::json;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

fn setup() -> (tempfile::TempDir, Descriptor, Attempt) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let run = root.join("runs/run with spaces; literal");
    fs::create_dir_all(&run).unwrap();
    let bin = root.join("installed tool");
    fs::write(&bin, "fixture executable").unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o700)).unwrap();
    let descriptor = Descriptor {
        id: "registered".into(),
        executable: bin,
        args: vec!["serve".into()],
        config: None,
        cwd: root.clone(),
        allow_dirs: vec![root.join("runs")],
        evidence_dirs: vec![root.join("runs")],
    }
    .validate()
    .unwrap();
    let mut document = fixtures::description("0.4.0");
    document["actions"][0]["renders"] = json!([
      {"name":"report", "kind":"server", "ready_timeout_s":7,
       "command":["sample-lab","view","{run.dir}","--frame-ancestor","{frame_ancestor}","$(not-a-shell); literal"]},
      {"name":"document", "kind":"document", "command":["sample-lab","report","{run.dir}"]}
    ]);
    let mut result = fixtures::result("completed");
    result["run"]["dir"] = json!(run);
    let attempt: Attempt = serde_json::from_value(json!({
      "id":"saved", "sequence":1,"created_ms":0,
      "request":{"id":"saved","tool":"registered","action":"mark_band","revision":"saved-revision","params":{},"images":{}},
      "arguments":{},"declaration":{"document":document},"inputs":{},"wire":null,
      "execution":"completed","result":result,"preservation":{"contract":"complete"},
      "outputs":[],"journal":null,"notices":[]
    })).unwrap();
    (temp, descriptor, attempt)
}
fn origin() -> &'static str {
    "http://127.0.0.1:4801"
}
#[test]
fn saved_renderer_uses_registered_binary_and_literal_whole_arguments() {
    let (_temp, descriptor, attempt) = setup();
    let render = ResolvedRender::resolve(&descriptor, &attempt, "report", origin()).unwrap();
    let command = render.command().unwrap();
    let actual = command.as_std();
    assert_eq!(actual.get_program(), descriptor.executable);
    let args: Vec<_> = actual
        .get_args()
        .map(|v| v.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        args,
        vec![
            "view".to_owned(),
            attempt.result.as_ref().unwrap()["run"]["dir"]
                .as_str()
                .unwrap()
                .to_owned(),
            "--frame-ancestor".into(),
            origin().into(),
            "$(not-a-shell); literal".into()
        ]
    );
    assert_eq!(render.kind(), RenderKind::Server);
    assert_eq!(
        render.ready_timeout(),
        Some(std::time::Duration::from_secs(7))
    );
    assert_eq!(actual.get_current_dir(), Some(descriptor.cwd.as_path()));
    let env: Vec<_> = actual
        .get_envs()
        .map(|(name, _)| name.to_string_lossy().into_owned())
        .collect();
    assert!(env.iter().all(|name| {
        [
            "HOME",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "TMPDIR",
            "LANG",
            "LC_ALL",
            "PATH",
        ]
        .contains(&name.as_str())
    }));
    let doc = ResolvedRender::resolve(&descriptor, &attempt, "document", origin()).unwrap();
    assert_eq!(doc.kind(), RenderKind::Document);
    assert_eq!(doc.ready_timeout(), None);
}
#[test]
fn saved_v03_evidence_cannot_launch_a_live_renderer() {
    let (_temp, descriptor, attempt) = setup();
    for old_description in [true, false] {
        let mut saved = attempt.clone();
        if old_description {
            saved.declaration["document"]["contract"] = json!("0.3.0");
        } else {
            saved.result.as_mut().unwrap()["contract"] = json!("0.3.0");
        }
        let error = match ResolvedRender::resolve(&descriptor, &saved, "report", origin()) {
            Ok(_) => panic!("old saved evidence must not launch a live renderer"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("0.4.0"), "{error}");
    }
}
#[test]
fn no_other_binary_or_ambiguous_renderer_can_be_selected() {
    let (_temp, descriptor, attempt) = setup();
    for name in ["/bin/sh", "sh", "./sample-lab", "other-tool"] {
        let mut bad = attempt.clone();
        bad.declaration["document"]["actions"][0]["renders"][0]["command"][0] = json!(name);
        assert!(
            ResolvedRender::resolve(&descriptor, &bad, "report", origin()).is_err(),
            "{name}"
        );
    }
    let mut bad = attempt.clone();
    let renders = bad.declaration["document"]["actions"][0]["renders"]
        .as_array_mut()
        .unwrap();
    renders.push(renders[0].clone());
    assert!(ResolvedRender::resolve(&descriptor, &bad, "report", origin()).is_err());
    assert!(ResolvedRender::resolve(&descriptor, &attempt, "missing", origin()).is_err());
    bad = attempt.clone();
    bad.request.tool = "other".into();
    assert!(ResolvedRender::resolve(&descriptor, &bad, "report", origin()).is_err());
    bad = attempt.clone();
    bad.result.as_mut().unwrap()["action"] = json!("other_action");
    assert!(ResolvedRender::resolve(&descriptor, &bad, "report", origin()).is_err());
}
#[test]
fn malformed_placeholders_origins_and_limits_fail_before_execution() {
    let (_temp, descriptor, attempt) = setup();
    for arg in [
        "--run={run.dir}",
        "{unknown}",
        "prefix{frame_ancestor}",
        "bad\0arg",
    ] {
        let mut bad = attempt.clone();
        bad.declaration["document"]["actions"][0]["renders"][0]["command"][2] = json!(arg);
        assert!(
            ResolvedRender::resolve(&descriptor, &bad, "report", origin()).is_err(),
            "{arg:?}"
        );
    }
    for host in [
        "https://127.0.0.1:4801",
        "http://localhost:4801",
        "http://127.0.0.1:4801/path",
        "http://user@127.0.0.1:4801",
        "http://127.0.0.1:4801/?x=1",
        "http://127.0.0.1:0",
        "http://127.0.0.1:4801#x",
    ] {
        assert!(
            ResolvedRender::resolve(&descriptor, &attempt, "report", host).is_err(),
            "{host}"
        );
    }
    let mut bad = attempt.clone();
    bad.declaration["document"]["actions"][0]["renders"][0]["ready_timeout_s"] = json!(61);
    assert!(ResolvedRender::resolve(&descriptor, &bad, "report", origin()).is_err());
    bad = attempt.clone();
    bad.declaration["document"]["actions"][0]["renders"][0]["command"] =
        json!(vec!["sample-lab"; 129]);
    assert!(ResolvedRender::resolve(&descriptor, &bad, "report", origin()).is_err());
    bad = attempt.clone();
    bad.declaration["document"]["actions"][0]["renders"][0]["command"][5] =
        json!("x".repeat(131073));
    assert!(ResolvedRender::resolve(&descriptor, &bad, "report", origin()).is_err());
}
#[test]
fn missing_outside_and_symlink_escaped_run_directories_are_unavailable() {
    let (temp, descriptor, attempt) = setup();
    let outside = temp.path().join("elsewhere");
    fs::create_dir(&outside).unwrap();
    let link = descriptor.evidence_dirs[0].join("escape");
    symlink(&outside, &link).unwrap();
    for path in [
        serde_json::Value::Null,
        json!("relative"),
        json!(outside),
        json!(link),
        json!(descriptor.evidence_dirs[0].join("missing")),
        json!(descriptor.executable),
    ] {
        let mut bad = attempt.clone();
        bad.result.as_mut().unwrap()["run"]["dir"] = path;
        assert!(ResolvedRender::resolve(&descriptor, &bad, "report", origin()).is_err());
    }
    let render = ResolvedRender::resolve(&descriptor, &attempt, "report", origin()).unwrap();
    let run = attempt.result.as_ref().unwrap()["run"]["dir"]
        .as_str()
        .unwrap();
    fs::rename(run, descriptor.evidence_dirs[0].join("old run")).unwrap();
    fs::create_dir(run).unwrap();
    assert!(
        render.command().is_err(),
        "changed directory identity must be checked again before launch"
    );
}
#[test]
fn copied_run_directory_is_used_only_when_explicitly_selected_and_authorized() {
    let (_temp, descriptor, mut attempt) = setup();
    let copied = descriptor.evidence_dirs[0].join("copied run");
    fs::create_dir(&copied).unwrap();
    attempt.result.as_mut().unwrap()["run"]["dir"] = json!(copied);
    let render = ResolvedRender::resolve(&descriptor, &attempt, "document", origin()).unwrap();
    assert_eq!(render.run_dir(), copied);
    assert_eq!(
        render
            .command()
            .unwrap()
            .as_std()
            .get_args()
            .nth(1)
            .unwrap(),
        copied
    );
}

#[test]
fn canonical_registration_targets_cannot_be_redirected_after_serving() {
    let (temp, descriptor, attempt) = setup();
    let resolved = ResolvedRender::resolve(&descriptor, &attempt, "report", origin()).unwrap();
    let other = temp.path().join("other executable");
    fs::rename(&descriptor.executable, &other).unwrap();
    symlink(&other, &descriptor.executable).unwrap();
    assert!(ResolvedRender::resolve(&descriptor, &attempt, "report", origin()).is_err());
    assert!(resolved.command().is_err());

    fs::remove_file(&descriptor.executable).unwrap();
    fs::rename(&other, &descriptor.executable).unwrap();
    let moved = temp.path().join("moved root");
    fs::rename(&descriptor.evidence_dirs[0], &moved).unwrap();
    symlink(&moved, &descriptor.evidence_dirs[0]).unwrap();
    assert!(ResolvedRender::resolve(&descriptor, &attempt, "report", origin()).is_err());
}

#[test]
fn input_permission_does_not_authorize_report_reads() {
    let (_temp, mut descriptor, attempt) = setup();
    descriptor.evidence_dirs.clear();
    let error = ResolvedRender::resolve(&descriptor, &attempt, "report", origin())
        .err()
        .unwrap();
    assert!(error.to_string().contains("outside registered"));
    descriptor.evidence_dirs = descriptor.allow_dirs.clone();
    descriptor.allow_dirs.clear();
    assert!(ResolvedRender::resolve(&descriptor, &attempt, "report", origin()).is_ok());
}
