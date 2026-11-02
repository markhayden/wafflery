#[path = "../testkit/result_file.rs"]
mod result_file;
use serde_json::{Value, json};
use std::sync::{
    Barrier,
    atomic::{AtomicBool, Ordering},
};

#[test]
fn concurrent_readers_only_observe_whole_result_documents() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("run.json");
    let payload = "synthetic evidence ".repeat(16_384);
    result_file::publish(&path, &json!({"sequence":0,"body":payload})).unwrap();
    let start = Barrier::new(2);
    let done = AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            start.wait();
            for sequence in 1..=64 {
                result_file::publish(&path, &json!({"sequence":sequence,"body":payload})).unwrap();
            }
            done.store(true, Ordering::Release);
        });
        start.wait();
        let mut reads = 0;
        loop {
            let bytes = std::fs::read(&path).unwrap();
            let document: Value = serde_json::from_slice(&bytes)
                .expect("published result must never be truncated or partial");
            assert_eq!(document["body"], payload);
            assert!(document["sequence"].as_u64().unwrap() <= 64);
            reads += 1;
            if done.load(Ordering::Acquire) {
                break;
            }
        }
        assert!(reads > 0);
    });
    let final_value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(final_value["sequence"], 64);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn failed_publication_cleans_up_staging_without_replacing_the_destination() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("run.json");
    std::fs::create_dir(&destination).unwrap();
    std::fs::write(destination.join("keep"), b"sentinel").unwrap();
    assert!(result_file::publish(&destination, &json!({"status":"completed"})).is_err());
    assert_eq!(
        std::fs::read(destination.join("keep")).unwrap(),
        b"sentinel"
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}
