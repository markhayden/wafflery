use super::*;
use crate::tools::render::process::{Output, ProcessPhase, ProcessState};
use std::os::unix::{fs::symlink, net::UnixListener};

fn setup() -> (tempfile::TempDir, PathBuf, DocumentGrant) {
    let root = tempfile::tempdir().unwrap();
    let run = root.path().join("saved run");
    std::fs::create_dir(&run).unwrap();
    std::fs::write(run.join("evidence"), "unchanged").unwrap();
    let grant = DocumentGrant::new(&run).unwrap();
    (root, run, grant)
}
fn completed(path: &Path) -> ProcessState {
    ProcessState {
        phase: ProcessPhase::Exited,
        close_requested: false,
        exit_code: Some(0),
        stdout: Output {
            bytes: format!("{}\n", path.display()).into_bytes(),
            finished: true,
            ..Output::default()
        },
        stderr: Output {
            finished: true,
            ..Output::default()
        },
        error: None,
    }
}

#[tokio::test]
async fn successful_document_reads_only_the_named_outside_file_into_an_immutable_copy() {
    let (root, run, grant) = setup();
    let output = root.path().join("report with spaces.html");
    std::fs::write(&output, b"<!doctype html><h1>Report</h1>").unwrap();
    std::fs::write(root.path().join("neighbor"), "not part of report").unwrap();
    let document = grant.read(&completed(&output)).await.unwrap();
    assert_eq!(document.bytes(), b"<!doctype html><h1>Report</h1>");
    assert_eq!(document.name(), "report with spaces.html");
    assert!(output.exists(), "the tool owns its original file");
    std::fs::write(&output, "changed later").unwrap();
    assert_eq!(document.bytes(), b"<!doctype html><h1>Report</h1>");
    assert_eq!(
        std::fs::read_to_string(run.join("evidence")).unwrap(),
        "unchanged"
    );
}

#[tokio::test]
async fn success_requires_exit_zero_complete_bounded_output_and_exactly_one_absolute_path() {
    let (root, _, grant) = setup();
    let output = root.path().join("report.html");
    std::fs::write(&output, "report").unwrap();
    for bytes in [
        b"relative.html\n".to_vec(),
        b"file:///report\n".to_vec(),
        b"\xff\n".to_vec(),
        b"/bad\0name\n".to_vec(),
        format!("{}\nextra\n", output.display()).into_bytes(),
        output.display().to_string().into_bytes(),
    ] {
        let mut state = completed(&output);
        state.stdout.bytes = bytes;
        assert!(grant.read(&state).await.is_err());
    }
    let mut state = completed(&output);
    state.exit_code = Some(17);
    assert!(grant.read(&state).await.is_err());
    state = completed(&output);
    state.phase = ProcessPhase::Running;
    assert!(grant.read(&state).await.is_err());
    state = completed(&output);
    state.stdout.finished = false;
    assert!(grant.read(&state).await.is_err());
    state = completed(&output);
    state.stderr.overflow = true;
    assert!(grant.read(&state).await.is_err());
    state = completed(&output);
    state.stdout.error = Some("synthetic".into());
    assert!(grant.read(&state).await.is_err());
}

#[tokio::test]
async fn missing_in_run_symlink_and_non_regular_outputs_are_refused() {
    let (root, run, grant) = setup();
    let output = root.path().join("report.html");
    std::fs::write(&output, "report").unwrap();
    let link = root.path().join("linked.html");
    symlink(&output, &link).unwrap();
    let inside_link = root.path().join("run-alias");
    symlink(&run, &inside_link).unwrap();
    let case_alias = root.path().join("SAVED RUN/evidence");
    if case_alias.exists() {
        assert!(
            grant.read(&completed(&case_alias)).await.is_err(),
            "case aliases must not bypass the run boundary"
        );
    }
    let socket = root.path().join("socket");
    let _listener = UnixListener::bind(&socket).unwrap();
    for path in [
        root.path().join("missing"),
        run.join("evidence"),
        link,
        inside_link.join("evidence"),
        root.path().to_owned(),
        socket,
    ] {
        assert!(
            grant.read(&completed(&path)).await.is_err(),
            "{}",
            path.display()
        );
    }
}

#[tokio::test]
async fn oversized_documents_and_replaced_run_directories_are_refused() {
    let (root, run, grant) = setup();
    let output = root.path().join("report.html");
    let file = std::fs::File::create(&output).unwrap();
    file.set_len(DOCUMENT_LIMIT as u64 + 1).unwrap();
    assert!(grant.read(&completed(&output)).await.is_err());
    std::fs::write(&output, "report").unwrap();
    std::fs::rename(&run, root.path().join("moved run")).unwrap();
    std::fs::create_dir(&run).unwrap();
    assert!(grant.read(&completed(&output)).await.is_err());
    let copied = DocumentGrant::new(&root.path().join("moved run")).unwrap();
    assert_eq!(
        copied.read(&completed(&output)).await.unwrap().bytes(),
        b"report"
    );
}

#[test]
fn replacement_between_path_inspection_and_open_cannot_change_the_granted_file() {
    for use_symlink in [false, true] {
        let (root, _, grant) = setup();
        let output = root.path().join("report.html");
        std::fs::write(&output, "original").unwrap();
        let other = root.path().join("other.html");
        std::fs::write(&other, "different").unwrap();
        let result = grant.read_file(&output, || {
            std::fs::rename(&output, root.path().join("original.html")).unwrap();
            if use_symlink {
                symlink(&other, &output).unwrap();
            } else {
                std::fs::rename(&other, &output).unwrap();
            }
        });
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn timed_out_read_keeps_its_capacity_until_the_blocking_worker_finishes() {
    let (root, _, grant) = setup();
    let output = root.path().join("report.html");
    std::fs::write(&output, "report").unwrap();
    let slots = Arc::new(tokio::sync::Semaphore::new(1));
    let (entered, ready) = tokio::sync::oneshot::channel();
    let (release, blocked) = std::sync::mpsc::channel();
    let task = grant
        .spawn_read(output.clone(), &slots, move || {
            let _ = entered.send(());
            blocked.recv().unwrap();
        })
        .unwrap();
    ready.await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(10), task)
            .await
            .is_err()
    );
    assert!(
        grant.spawn_read(output, &slots, || {}).is_err(),
        "timeout must not free capacity still held by the reader"
    );
    release.send(()).unwrap();
    let permit = tokio::time::timeout(Duration::from_secs(2), slots.acquire())
        .await
        .unwrap()
        .unwrap();
    drop(permit);
}
