#[path = "../testkit/v04_runs.rs"]
mod runs;
#[allow(dead_code)]
#[path = "../testkit/v04_bindings.rs"]
mod v04_bindings;
#[allow(dead_code)]
#[path = "../testkit/v04_fixtures.rs"]
mod v04_fixtures;
#[allow(dead_code)]
#[path = "../testkit/v04_results.rs"]
mod v04_results;
#[allow(dead_code)]
#[path = "../testkit/v04_store.rs"]
mod v04_store;
use serde_json::{Value, json};
use std::sync::{Arc, Barrier};

const SESSION: &str = "9fb05336-a9af-47da-8364-60fa98c650c2";
fn book(root: &std::path::Path) -> runs::Runs {
    runs::Runs::new(root, "sample-lab", SESSION)
}
fn completed(info: &v04_results::RunInfo) -> Value {
    let store = v04_store::Store::new(&info.root, &info.tool, vec![]).unwrap();
    let prepared = v04_bindings::prepare(&store, &info.tool, "summarize", json!({})).unwrap();
    v04_results::execute(&store, &prepared, info, || false)
}

#[test]
fn claims_are_atomic_and_only_no_run_refusals_release_them() {
    let root = tempfile::tempdir().unwrap();
    let runs = Arc::new(book(root.path()));
    let barrier = Arc::new(Barrier::new(12));
    let jobs: Vec<_> = (0..12)
        .map(|_| {
            let runs = runs.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                runs.claim(Some("shared"))
            })
        })
        .collect();
    let answers: Vec<_> = jobs.into_iter().map(|j| j.join().unwrap()).collect();
    assert_eq!(answers.iter().filter(|r| r.is_ok()).count(), 1);
    let mut winner = None;
    for answer in answers {
        let error = match answer {
            Ok(claim) => {
                winner = claim;
                continue;
            }
            Err(error) => error,
        };
        assert_eq!(error["code"], -32602);
        assert_eq!(
            error["data"],
            json!({"reason":"client_run_id_reused","run":null})
        );
    }
    // A queued cancellation keeps its claim; only an explicit pre-run refusal releases it.
    let duplicate = runs.claim(Some("shared"));
    assert!(duplicate.is_err());
    runs.release(duplicate.ok().flatten().as_ref());
    assert!(runs.claim(Some("shared")).is_err());
    runs.release(winner.as_ref());
    let current = runs.claim(Some("shared")).unwrap();
    // Late cleanup by the prior owner must not release the new owner.
    runs.release(winner.as_ref());
    assert!(runs.claim(Some("shared")).is_err());
    assert!(runs.allocate("summarize", winner.as_ref()).is_err());
    let allocated = runs.allocate("summarize", current.as_ref()).unwrap();
    runs.release(current.as_ref());
    let refusal = runs.claim(Some("shared")).unwrap_err();
    let mut reference = allocated.reference.clone();
    reference.as_object_mut().unwrap().remove("dir");
    assert_eq!(refusal["data"]["run"], reference);
    for bad in ["", "with space", "../escape", "é"] {
        assert!(runs.claim(Some(bad)).is_err());
    }
    assert!(runs.claim(Some(&"a".repeat(129))).is_err());
    runs.claim(Some(&"a".repeat(128))).unwrap();
    runs.claim(None).unwrap();
    runs.claim(None).unwrap();
}

#[test]
fn allocation_is_discoverable_before_work_and_completion_is_immutable() {
    let root = tempfile::tempdir().unwrap();
    let runs = book(root.path());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    let claim = runs.claim(Some("one")).unwrap();
    let info = runs.allocate("summarize", claim.as_ref()).unwrap();
    let id = info.reference["id"].as_str().unwrap();
    let page = runs.list(None).unwrap();
    let _: table_manners_v04::RunsList = serde_json::from_value(page.clone()).unwrap();
    assert_eq!(page["runs"][0]["status"], "running");
    assert_eq!(page["runs"][0]["client_run_id"], "one");
    assert!(page["runs"][0].get("dir").is_none());
    assert!(runs.read(id, "run.json").is_err());
    let journal = runs.read(id, "journal").unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(journal.trim()).unwrap()["event"],
        "allocated"
    );
    runs.journal(&info, "outputs ready").unwrap();
    let final_run = completed(&info);
    runs.finish(&info, &final_run).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&runs.read(id, "run.json").unwrap()).unwrap(),
        final_run
    );
    assert_eq!(runs.list(None).unwrap()["runs"][0]["status"], "completed");
    let journal = runs.read(id, "journal").unwrap();
    assert_eq!(journal.lines().count(), 3);
    runs.finish(&info, &final_run).unwrap();
    assert_eq!(runs.read(id, "journal").unwrap(), journal);
    let mut changed = final_run.clone();
    changed["status"] = json!("cancelled");
    assert!(runs.finish(&info, &changed).is_err());
    assert!(runs.journal(&info, "too late").is_err());
    assert_eq!(
        serde_json::from_str::<Value>(&runs.read(id, "run.json").unwrap()).unwrap(),
        final_run
    );
}

#[test]
fn restart_retains_runs_but_starts_new_claims_and_relocation_still_reads() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("original");
    std::fs::create_dir(&root).unwrap();
    let runs = book(&root);
    let claim = runs.claim(Some("again")).unwrap();
    let info = runs.allocate("summarize", claim.as_ref()).unwrap();
    runs.finish(&info, &completed(&info)).unwrap();
    drop(runs);
    let moved = parent.path().join("moved");
    std::fs::rename(root, &moved).unwrap();
    let restarted = runs::Runs::new(&moved, "sample-lab", "0c97a131-4fa4-4a1a-b6f0-79710fcd0cb7");
    let claim = restarted.claim(Some("again")).unwrap();
    let new = restarted.allocate("summarize", claim.as_ref()).unwrap();
    assert_ne!(new.reference["session_id"], info.reference["session_id"]);
    assert_eq!(
        restarted.list(None).unwrap()["runs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let saved: Value = serde_json::from_str(
        &restarted
            .read(info.reference["id"].as_str().unwrap(), "run.json")
            .unwrap(),
    )
    .unwrap();
    let mut current_reference = info.reference.clone();
    current_reference["dir"] = json!(
        moved
            .join("runs")
            .join(info.reference["id"].as_str().unwrap())
    );
    assert_eq!(saved["run"], current_reference);
    let original: Value = serde_json::from_slice(
        &std::fs::read(
            moved
                .join("runs")
                .join(info.reference["id"].as_str().unwrap())
                .join("run.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(original["run"], info.reference);
}

#[test]
fn pagination_encodes_a_position_and_new_runs_do_not_shift_later_pages() {
    let root = tempfile::tempdir().unwrap();
    let runs = book(root.path());
    let mut expected = vec![];
    for _ in 0..5 {
        let info = runs.allocate("summarize", None).unwrap();
        expected.push((
            time::OffsetDateTime::parse(
                &info.started,
                &time::format_description::well_known::Rfc3339,
            )
            .unwrap(),
            info.reference["id"].as_str().unwrap().to_owned(),
        ));
    }
    expected.sort_by(|a, b| b.cmp(a));
    let expected: Vec<Value> = expected.into_iter().map(|(_, id)| json!(id)).collect();
    let first = runs.list(None).unwrap();
    assert_eq!(first["runs"].as_array().unwrap().len(), 2);
    let mut actual: Vec<Value> = first["runs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].clone())
        .collect();
    let mut cursor = first["next_cursor"].clone();
    runs.allocate("summarize", None).unwrap();
    while let Some(next) = cursor.as_str() {
        assert!(
            next.len() <= 256
                && next
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "._~-".contains(c))
        );
        let page = runs.list(Some(next)).unwrap();
        actual.extend(
            page["runs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["id"].clone()),
        );
        cursor = page["next_cursor"].clone();
    }
    assert_eq!(actual, expected);
    for bad in ["../escape", "unknown", "", "%00"] {
        assert!(runs.list(Some(bad)).is_err());
    }
    for bad in ["../outside", "/tmp/outside", "", "not-a-run"] {
        assert!(runs.read(bad, "journal").is_err());
    }
    assert!(
        runs.read(expected[0].as_str().unwrap(), "../outside")
            .is_err()
    );
}

#[test]
fn allocation_failure_does_not_publish_a_run_or_forget_the_claim() {
    let root = tempfile::tempdir().unwrap();
    let runs = book(root.path());
    std::fs::write(root.path().join("runs"), b"blocked").unwrap();
    let claim = runs.claim(Some("retry")).unwrap();
    assert!(runs.allocate("summarize", claim.as_ref()).is_err());
    assert_eq!(
        runs.claim(Some("retry")).unwrap_err()["data"]["run"],
        Value::Null
    );
    runs.release(claim.as_ref());
    std::fs::remove_file(root.path().join("runs")).unwrap();
    assert_eq!(runs.list(None).unwrap()["runs"], json!([]));
    let claim = runs.claim(Some("retry")).unwrap();
    assert!(runs.allocate("summarize", claim.as_ref()).is_ok());
}

#[test]
fn completion_cannot_change_the_allocated_reference() {
    let root = tempfile::tempdir().unwrap();
    let runs = book(root.path());
    let foreign = book(root.path()).claim(Some("unclaimed")).unwrap();
    assert!(runs.allocate("summarize", foreign.as_ref()).is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    let mut info = runs.allocate("summarize", None).unwrap();
    let reference = info.reference.clone();
    info.reference["client_run_id"] = json!("forged");
    let run = completed(&info);
    assert!(runs.finish(&info, &run).is_err());
    assert!(
        runs.read(reference["id"].as_str().unwrap(), "run.json")
            .is_err()
    );
    info.reference = reference;
    runs.finish(&info, &completed(&info)).unwrap();
}

#[test]
fn paging_orders_instants_not_rfc3339_strings_and_breaks_ties_by_id() {
    let root = tempfile::tempdir().unwrap();
    let runs = book(root.path());
    let first = runs.allocate("summarize", None).unwrap();
    let second = runs.allocate("summarize", None).unwrap();
    let third = runs.allocate("summarize", None).unwrap();
    for (info, time) in [
        (&first, "2026-11-02T12:00:00Z"),
        (&second, "2026-11-02T12:00:00.1Z"),
        (&third, "2026-11-02T05:00:00.100-07:00"),
    ] {
        let path = root
            .path()
            .join("runs")
            .join(info.reference["id"].as_str().unwrap())
            .join("entry.json");
        let mut entry: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        entry["started"] = json!(time);
        std::fs::write(path, serde_json::to_vec(&entry).unwrap()).unwrap();
    }
    let mut tied = [
        second.reference["id"].as_str().unwrap(),
        third.reference["id"].as_str().unwrap(),
    ];
    tied.sort_by(|a, b| b.cmp(a));
    let page = runs.list(None).unwrap();
    assert_eq!(page["runs"][0]["id"], tied[0]);
    assert_eq!(page["runs"][1]["id"], tied[1]);
    let last = runs.list(page["next_cursor"].as_str()).unwrap();
    assert_eq!(last["runs"][0]["id"], first.reference["id"]);
    assert_eq!(last["next_cursor"], Value::Null);
}

#[test]
fn unpublished_allocation_journals_are_not_resources() {
    let root = tempfile::tempdir().unwrap();
    let runs = book(root.path());
    let id = "0123456789abcdef0123456789abcdef";
    let pending = root.path().join("runs").join(id);
    std::fs::create_dir_all(&pending).unwrap();
    std::fs::write(pending.join("journal"), "unpublished\n").unwrap();
    assert!(runs.read(id, "journal").is_err());
    assert_eq!(runs.list(None).unwrap()["runs"], json!([]));
}
