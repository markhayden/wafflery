use super::{finish, setup};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use wafflery::{
    library::attempts::{Attempt, Submission},
    tools::coordinator::Coordinator,
};

async fn settled(c: &Arc<Coordinator>, id: &str) -> Attempt {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let attempt = c.attempts.get(id).await.unwrap();
            if attempt.preservation["contract"] != "pending"
                && c.snapshot().await.unwrap()["tools"][0]["busy"] == false
            {
                return attempt;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
async fn quote(c: &Arc<Coordinator>, request: &Submission) {
    c.prepare(request.clone()).await.unwrap();
    assert_eq!(
        c.estimate_prepared(&request.id).await.unwrap().estimate["allowed"],
        true
    );
}
#[tokio::test]
async fn paid_errors_preserve_zero_actual_and_unresolved_spend() {
    for (label, code, usd, unresolved) in [
        ("paid-stale", "stale_estimate", 0.0, 0.0),
        ("paid-cap", "cap", 0.0, 0.0),
        ("paid-credential", "credential", 0.0, 0.0),
        ("paid-unresolved", "provider", 0.12, 0.12),
        ("paid-failed", "provider", 0.08, 0.0),
    ] {
        let (_dir, c, request) = setup(label).await;
        quote(&c, &request).await;
        c.submit_prepared(request.clone()).await.unwrap();
        let attempt = settled(&c, &request.id).await;
        let result = attempt.result.as_ref().unwrap();
        assert_eq!(result["error"]["code"], code, "{label}");
        assert_eq!(result["spend"]["usd"], usd);
        assert_eq!(result["spend"]["unresolved_usd"], unresolved);
        assert_eq!(attempt.observations["session_after"]["spent_usd"], usd);
        assert_eq!(
            attempt.observations["session_after"]["unresolved_usd"],
            unresolved
        );
        assert_eq!(
            attempt.observations["session_after"]["reserved_usd"].as_f64(),
            Some(0.0)
        );
        assert_eq!(attempt.preservation["contract"], "complete");
        if code == "credential" {
            let declaration = c.description("fixture").await.unwrap();
            assert_eq!(
                declaration.document["actions"][1]["readiness"]["state"],
                "needs_setup"
            );
            assert_eq!(
                declaration.document["actions"][1]["readiness"]["requirements"][0]["state"],
                "invalid"
            );
        }
        finish(&c).await;
    }
}
#[tokio::test]
async fn a_later_budget_refusal_keeps_the_same_readiness_and_both_earlier_quotes() {
    let (_dir, c, first) = setup("paid-full").await;
    let mut second = first.clone();
    second.id = "second".into();
    second.params["label"] = json!("paid normal");
    quote(&c, &first).await;
    quote(&c, &second).await;
    c.submit_prepared(first.clone()).await.unwrap();
    let done = settled(&c, &first.id).await;
    assert_eq!(done.result.as_ref().unwrap()["spend"]["usd"], 0.12);
    c.submit_prepared(second.clone()).await.unwrap();
    let refused = settled(&c, &second.id).await;
    assert_eq!(refused.result.as_ref().unwrap()["error"]["code"], "budget");
    assert_eq!(refused.result.as_ref().unwrap()["spend"]["usd"], 0);
    assert_eq!(
        done.declaration["revision"],
        refused.declaration["revision"]
    );
    assert_eq!(
        c.attempts
            .history(None, 10, None, None)
            .await
            .unwrap()
            .len(),
        2
    );
    finish(&c).await;
}
#[tokio::test]
async fn provider_validation_invalidates_an_earlier_quote_without_a_second_dispatch() {
    let (dir, c, first) = setup("paid-validate").await;
    let mut second = first.clone();
    second.id = "second".into();
    second.params["label"] = json!("paid normal");
    quote(&c, &first).await;
    quote(&c, &second).await;
    c.submit_prepared(first.clone()).await.unwrap();
    settled(&c, &first.id).await;
    assert!(c.submit_prepared(second.clone()).await.is_err());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("runs/calls.ndjson"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert_eq!(
        c.attempts
            .history(None, 10, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    finish(&c).await;
}

#[tokio::test]
async fn missing_setup_before_allocation_is_a_no_run_refusal_and_refreshes_setup() {
    let (dir, c, request) = setup("paid-not-ready").await;
    quote(&c, &request).await;
    c.submit_prepared(request.clone()).await.unwrap();
    let attempt = settled(&c, &request.id).await;
    assert_eq!(attempt.execution, "rejected");
    assert!(attempt.result.is_none());
    assert_eq!(attempt.observations["refusal"]["reason"], "not_ready");
    assert_eq!(attempt.observations["spent_usd"], 0);
    assert!(!dir.path().join("runs/calls.ndjson").exists());
    assert_eq!(
        c.description("fixture").await.unwrap().document["actions"][1]["readiness"]["requirements"]
            [0]["state"],
        "missing"
    );
    finish(&c).await;
}

#[tokio::test]
async fn cancelling_a_sent_paid_request_preserves_its_unresolved_worst_case_and_controls() {
    let (dir, c, request) = setup("paid-held").await;
    quote(&c, &request).await;
    c.submit_prepared(request.clone()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !dir.path().join("runs/paid-provider-started").exists() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let live = c.session("fixture").await.unwrap();
    assert_eq!(live["reserved_usd"], 0.12);
    assert!((live["remaining_usd"].as_f64().unwrap() - 0.08).abs() < 1e-6);
    c.refresh("fixture").await.unwrap();
    let mut another = request.clone();
    another.id = "while-running".into();
    another.params["label"] = json!("paid normal");
    c.prepare(another.clone()).await.unwrap();
    assert_eq!(
        c.estimate_prepared(&another.id).await.unwrap().estimate["refused"],
        "budget"
    );
    tokio::time::timeout(Duration::from_secs(3), async {
        while c.snapshot().await.unwrap()["tools"][0]["progress"]["message"]
            != "Waiting for synthetic provider"
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("held paid work must report a later heartbeat");
    c.cancel(&request.id).await.unwrap();
    let attempt = settled(&c, &request.id).await;
    let result = attempt.result.as_ref().unwrap();
    assert_eq!(result["status"], "cancelled");
    assert_eq!(result["spend"]["usd"], 0.12);
    assert_eq!(result["spend"]["unresolved_usd"], 0.12);
    assert_eq!(
        attempt.observations["session_after"]["reserved_usd"].as_f64(),
        Some(0.0)
    );
    assert_eq!(attempt.observations["session_after"]["spent_usd"], 0.12);
    assert_eq!(attempt.preservation["contract"], "complete");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("runs/calls.ndjson"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    finish(&c).await;
}
