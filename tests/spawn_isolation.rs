//! Siblings must never hold one another's EOF writers across concurrent launches.
use std::{
    collections::BTreeSet, os::fd::AsRawFd, os::unix::fs::FileTypeExt, path::Path, sync::Arc,
};
use tokio::sync::Barrier;
use wafflery::tools::{client::ToolClient, registration::Descriptor};

#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn concurrent_tool_launches_do_not_inherit_sibling_pipes() {
    assert!(
        wafflery::process::spawn_std(std::process::Command::new(env!("CARGO")).args([
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
    let binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab");
    let root = tempfile::tempdir().unwrap();
    // Model caller-owned pipes such as those inherited from a hosted test runner.
    let ambient = nix::unistd::pipe().unwrap();
    // Do not attribute caller-owned descriptors to the concurrent launches below.
    // Keep the synthetic ambient descriptors open until all observations are checked.
    let baseline: BTreeSet<u32> = std::fs::read_dir("/dev/fd")
        .unwrap()
        .map(Result::unwrap)
        .filter_map(|entry| {
            let fd = entry.file_name().to_str()?.parse::<u32>().ok()?;
            (fd > 2 && entry.path().metadata().ok()?.file_type().is_fifo()).then_some(fd)
        })
        .collect();
    let ambient_fds = [ambient.0.as_raw_fd() as u32, ambient.1.as_raw_fd() as u32];
    assert!(ambient_fds.iter().all(|fd| baseline.contains(fd)));
    let mut leaked = Vec::new();
    let mut ambient_seen = BTreeSet::new();
    for batch in 0..8 {
        let barrier = Arc::new(Barrier::new(24));
        let mut tasks = Vec::new();
        for index in 0..24 {
            let cwd = root.path().join(format!("{batch}-{index}"));
            std::fs::create_dir(&cwd).unwrap();
            let descriptor = Descriptor {
                id: format!("peer-{index}"),
                executable: binary.clone(),
                args: vec![
                    cwd.join("runs").display().to_string(),
                    "pipe-inheritance".into(),
                ],
                cwd: cwd.clone(),
                roots: vec![cwd.clone()],
                config: None,
            };
            let barrier = barrier.clone();
            tasks.push(tokio::spawn(async move {
                barrier.wait().await;
                let mut client = ToolClient::spawn(descriptor).unwrap();
                client.initialize().await.unwrap();
                let inherited: Vec<u32> = serde_json::from_slice(
                    &std::fs::read(cwd.join("runs/inherited-pipes.json")).unwrap(),
                )
                .unwrap();
                (client, inherited)
            }));
        }
        let mut clients = Vec::new();
        for task in tasks {
            let (client, inherited) = task.await.unwrap();
            ambient_seen.extend(
                inherited
                    .iter()
                    .copied()
                    .filter(|fd| ambient_fds.contains(fd)),
            );
            leaked.extend(inherited.into_iter().filter(|fd| !baseline.contains(fd)));
            clients.push(client);
        }
        // Keep all the other clients alive while each owner closes its own stdin.
        for client in &clients {
            if !client.close().await.unwrap() {
                assert!(
                    client.force_stop().await.unwrap(),
                    "test cleanup must reap its child"
                );
                leaked.push(u32::MAX);
            }
        }
    }
    drop(ambient);
    assert!(
        ambient_fds.iter().all(|fd| ambient_seen.contains(fd)),
        "the peer observations must include the synthetic caller-owned pipes"
    );
    assert!(
        leaked.is_empty(),
        "sibling pipe descriptors were inherited: {leaked:?}"
    );
}
