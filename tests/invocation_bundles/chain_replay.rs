use super::chain_bundle_cases::{seed, soften};
use super::*;
use wafflery::tools::{bundle_reader::ValidatedBundle, bundles, chain_replay::ReplayStatus};
async fn original() -> Arc<ValidatedBundle> {
    original_with_external(false).await
}
async fn original_with_external(mixed: bool) -> Arc<ValidatedBundle> {
    let h = Host::native(&[]).await;
    let seed = seed(&h, "original-seed").await;
    let a = soften(&h, "original-producer", &seed).await;
    let reference = a.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"].clone();
    let second = if mixed {
        seed.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"].clone()
    } else {
        reference.clone()
    };
    let mut request=h.request("original-consumer","collect_tokens",json!({"tokens":[{"kind":"artifact","reference":reference},{"kind":"artifact","reference":second}]})).await;
    request.lineage=serde_json::from_value(json!({"tokens":[{"attempt":a.id,"port":"token","occurrence":0},if mixed{Value::Null}else{json!({"attempt":a.id,"port":"token","occurrence":0})}]})).unwrap();
    h.c.submit(request).await.unwrap();
    let b = super::bundle_import_cases::owned(&h, "original-consumer").await;
    h.settled(&b.id).await;
    let built = bundles::build_chain(&h.c.assets, &[a, b]).await.unwrap();
    let parsed = Arc::new(
        wafflery::tools::bundle_reader::read(std::fs::read(built.file.path()).unwrap()).unwrap(),
    );
    h.close().await;
    std::fs::remove_dir_all(h.root.path().join("peer")).unwrap();
    parsed
}
async fn waiting(h: &Host, id: &str) -> ReplayStatus {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let status = h.c.chain_replay(id).await.unwrap();
            if status.phase == "awaiting_run" {
                return status;
            }
            assert_ne!(status.phase, "blocked", "{:?}", status.error);
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
async fn request(h: &Host, id: &str, status: &ReplayStatus) -> Submission {
    let progress = status.import.as_ref().unwrap();
    let mut bindings = serde_json::Map::new();
    let mut lineage = serde_json::Map::new();
    for input in &progress.inputs {
        bindings
            .entry(input.name.clone())
            .or_insert(json!([]))
            .as_array_mut()
            .unwrap()
            .push(serde_json::to_value(&input.binding).unwrap());
        lineage
            .entry(input.name.clone())
            .or_insert(json!([]))
            .as_array_mut()
            .unwrap()
            .push(serde_json::to_value(&input.source).unwrap());
    }
    let mut request = h
        .request(id, &progress.action, Value::Object(bindings))
        .await;
    request.params = progress.params.clone();
    request.lineage = serde_json::from_value(Value::Object(lineage)).unwrap();
    request.source_bundle=Some(serde_json::from_value(json!({"id":status.bundle,"step":status.step,"replay":{"id":status.id,"generation":status.generation}})).unwrap());
    request
}
#[tokio::test]
async fn guided_replay_uses_changed_outputs_without_hidden_dispatch_or_stale_internal_imports() {
    let bundle = original().await;
    let h = Host::native(&[]).await;
    let state =
        h.c.start_chain_replay(bundle.clone(), vec!["peer".into(), "peer".into()])
            .await
            .unwrap();
    assert_eq!(state.phase, "ready");
    assert!(
        h.c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .is_empty()
    );
    h.c.stage_chain_replay(&state.id, false).await.unwrap();
    let ready = waiting(&h, &state.id).await;
    let mut first = request(&h, "new-producer", &ready).await;
    first.params["amount"] = json!(0.75);
    h.c.submit(first).await.unwrap();
    let first = super::bundle_import_cases::owned(&h, "new-producer").await;
    h.settled("new-producer").await;
    let next = h.c.chain_replay(&state.id).await.unwrap();
    assert_eq!(next.step, 1);
    assert_eq!(next.phase, "ready");
    let controls = std::fs::read_to_string(h.root.path().join("peer/control-calls.jsonl")).unwrap();
    h.c.stage_chain_replay(&state.id, false).await.unwrap();
    let ready = waiting(&h, &state.id).await;
    let progress = ready.import.as_ref().unwrap();
    assert_eq!(progress.inputs.len(), 2);
    assert!(
        progress
            .inputs
            .iter()
            .all(|input| input.changed && input.source.is_some())
    );
    assert_eq!(
        std::fs::read_to_string(h.root.path().join("peer/control-calls.jsonl")).unwrap(),
        controls
    );
    let second = request(&h, "new-consumer", &ready).await;
    let new_ref = &first.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"];
    assert!(second.bindings["tokens"].iter().all(|binding|matches!(binding,wafflery::library::attempts::InputBinding::Artifact{reference} if reference==new_ref)));
    let mut stale = second.clone();
    if let wafflery::library::attempts::InputBinding::Artifact { reference } =
        &mut stale.bindings.get_mut("tokens").unwrap()[0]
    {
        *reference = bundle.manifest()["steps"][1]["invocation"]["arguments"]["tokens"][0].clone();
    }
    assert!(h.c.submit(stale).await.is_err());
    assert!(h.c.attempts.get("new-consumer").await.is_err());
    h.c.submit(second).await.unwrap();
    super::bundle_import_cases::owned(&h, "new-consumer").await;
    h.settled("new-consumer").await;
    let done = h.c.chain_replay(&state.id).await.unwrap();
    assert_eq!(done.phase, "completed");
    assert_eq!(done.attempts.len(), 2);
    assert_eq!(
        h.c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        2
    );
    h.close().await;
}
#[tokio::test]
async fn stopping_invalidates_the_staged_draft_and_explicit_resume_uses_a_new_context() {
    let h = Host::native(&[]).await;
    let state =
        h.c.start_chain_replay(original().await, vec!["peer".into(), "peer".into()])
            .await
            .unwrap();
    h.c.stage_chain_replay(&state.id, false).await.unwrap();
    let ready = waiting(&h, &state.id).await;
    let old = request(&h, "stopped-draft", &ready).await;
    assert_eq!(
        h.c.stop_chain_replay(&state.id).await.unwrap().phase,
        "stopped"
    );
    assert!(h.c.submit(old.clone()).await.is_err());
    assert!(h.c.stage_chain_replay(&state.id, false).await.is_err());
    h.c.stage_chain_replay(&state.id, true).await.unwrap();
    let resumed = waiting(&h, &state.id).await;
    assert!(resumed.generation > ready.generation);
    assert!(h.c.prepare(old).await.is_err());
    assert!(
        h.c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .is_empty()
    );
    h.close().await;
}

#[tokio::test]
async fn stopping_during_staging_keeps_its_owner_until_planning_drains() {
    let h = Host::native(&[]).await;
    let state =
        h.c.start_chain_replay(original().await, vec!["peer".into(), "peer".into()])
            .await
            .unwrap();
    h.c.stage_chain_replay(&state.id, false).await.unwrap();
    let ready = waiting(&h, &state.id).await;
    h.c.submit(request(&h, "staging-producer", &ready).await)
        .await
        .unwrap();
    super::bundle_import_cases::owned(&h, "staging-producer").await;
    h.settled("staging-producer").await;
    assert_eq!(h.c.chain_replay(&state.id).await.unwrap().step, 1);
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let db = h.c.assets.database.clone();
    let held = tokio::spawn(async move {
        db.call(move |_| {
            let _ = entered_tx.send(());
            let _ = release_rx.recv();
            Ok(())
        })
        .await
        .unwrap()
    });
    entered_rx.await.unwrap();
    let c = h.c.clone();
    let id = state.id.clone();
    let staged = tokio::spawn(async move { c.stage_chain_replay(&id, false).await });
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if h.c.chain_replay(&state.id).await.unwrap().phase == "preparing" {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    h.c.stop_chain_replay(&state.id).await.unwrap();
    let resumed = tokio::time::timeout(
        Duration::from_secs(2),
        h.c.stage_chain_replay(&state.id, true),
    )
    .await;
    let dismissed = h.c.dismiss_chain_replay(&state.id).await;
    let _ = release_tx.send(());
    held.await.unwrap();
    let _ = staged.await.unwrap();
    h.close().await;
    assert!(
        matches!(resumed, Ok(Err(_))),
        "Resume must reject while the previous staging owner is still draining"
    );
    assert!(dismissed.is_err());
}

#[tokio::test]
async fn each_paid_step_needs_a_fresh_quote_and_stop_invalidates_only_this_chains_preparations() {
    let h = Host::native(&["--priced-operations"]).await;
    let state =
        h.c.start_chain_replay(original().await, vec!["peer".into(), "peer".into()])
            .await
            .unwrap();
    h.c.stage_chain_replay(&state.id, false).await.unwrap();
    let ready = waiting(&h, &state.id).await;
    assert!(
        ready
            .import
            .as_ref()
            .unwrap()
            .params
            .get("max_spend_usd")
            .is_none()
    );
    let mut first = request(&h, "paid-producer", &ready).await;
    first.params["max_spend_usd"] = json!(0.5);
    h.c.prepare(first.clone()).await.unwrap();
    assert!(h.c.submit_prepared(first.clone()).await.is_err());
    assert!(h.c.attempts.get(&first.id).await.is_err());
    h.c.estimate_prepared(&first.id).await.unwrap();
    h.c.submit_prepared(first).await.unwrap();
    let producer = super::bundle_import_cases::owned(&h, "paid-producer").await;
    h.settled("paid-producer").await;
    assert_eq!(h.c.chain_replay(&state.id).await.unwrap().step, 1);
    h.c.stage_chain_replay(&state.id, false).await.unwrap();
    let ready = waiting(&h, &state.id).await;
    let mut second = request(&h, "paid-consumer", &ready).await;
    second.params["max_spend_usd"] = json!(0.25);
    h.c.prepare(second.clone()).await.unwrap();
    assert!(h.c.submit_prepared(second.clone()).await.is_err());
    h.c.estimate_prepared(&second.id).await.unwrap();
    let mut independent = h
        .request(
            "independent-paid-draft",
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await;
    independent.params = json!({"max_spend_usd":0.5});
    h.c.prepare(independent.clone()).await.unwrap();
    let session = h.c.description("peer").await.unwrap().tool_session["id"].clone();
    h.c.stop_chain_replay(&state.id).await.unwrap();
    assert!(h.c.preparation(&second.id).await.is_err());
    assert!(h.c.prepared_estimate(&second.id).await.is_err());
    assert!(h.c.submit_prepared(second).await.is_err());
    assert!(h.c.preparation(&independent.id).await.is_ok());
    assert_eq!(
        h.c.description("peer").await.unwrap().tool_session["id"],
        session
    );
    assert_eq!(
        h.c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        h.c.attempts.get("paid-producer").await.unwrap().result,
        producer.result
    );
    h.close().await;
}

#[tokio::test]
async fn resume_waits_for_a_completed_runs_owned_native_preservation_instead_of_repeating_it() {
    let h = Host::native(&["--hold-control"]).await;
    let state =
        h.c.start_chain_replay(original().await, vec!["peer".into(), "peer".into()])
            .await
            .unwrap();
    h.c.stage_chain_replay(&state.id, false).await.unwrap();
    let peer = h.root.path().join("peer");
    tokio::time::timeout(Duration::from_secs(3), async {
        while !peer.join("holding-control-0").exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    std::fs::write(peer.join("release-control"), "release").unwrap();
    let ready = waiting(&h, &state.id).await;
    std::fs::remove_file(peer.join("release-control")).unwrap();
    h.c.submit(request(&h, "held-native-producer", &ready).await)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let saved = h.c.attempts.get("held-native-producer").await.unwrap();
            if saved.execution == "completed"
                && saved.preservation["native"] == "preserving"
                && h.c.snapshot().await.unwrap()["tools"][0]["busy"] == false
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    h.c.stop_chain_replay(&state.id).await.unwrap();
    let resumed = h.c.stage_chain_replay(&state.id, true).await;
    std::fs::write(peer.join("release-control"), "release").unwrap();
    super::bundle_import_cases::owned(&h, "held-native-producer").await;
    h.close().await;
    assert!(
        resumed.is_err(),
        "A still-owned producer must settle before resume can advance or reprepare its step"
    );
}
#[tokio::test]
async fn private_chain_api_uses_retained_ids_and_keeps_stop_resume_and_dismiss_explicit() {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use wafflery::host::{api, auth::Auth};
    let h = Host::native(&[]).await;
    let bundle = original().await;
    let record = wafflery::tools::bundle_import::retain(&h.c.assets, &bundle)
        .await
        .unwrap();
    let auth = Auth::new().unwrap();
    let token = auth
        .exchange(&auth.issue().unwrap())
        .unwrap()
        .unwrap()
        .token;
    let (quit, _) = tokio::sync::watch::channel(false);
    let app = api::router_with_tools(
        "http://127.0.0.1:45678",
        auth,
        quit,
        h.c.assets.clone(),
        Some(h.c.clone()),
    );
    let request = |method: &str, path: &str, body: Value, authorized: bool| {
        let mut r = Request::builder()
            .method(method)
            .uri(path)
            .header("host", "127.0.0.1:45678")
            .header("origin", "http://127.0.0.1:45678")
            .header("content-type", "application/json");
        if authorized {
            r = r.header("authorization", format!("Bearer {token}"));
        }
        r.body(Body::from(body.to_string())).unwrap()
    };
    let input = json!({"bundle":record.id,"tools":["peer","peer"]});
    assert_eq!(
        app.clone()
            .oneshot(request("POST", "/api/chain-replays", input.clone(), false))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let mut wrong = input.clone();
    wrong["path"] = json!("/unrelated");
    assert_eq!(
        app.clone()
            .oneshot(request("POST", "/api/chain-replays", wrong, true))
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let response = app
        .clone()
        .oneshot(request("POST", "/api/chain-replays", input, true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let status: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1_000_000).await.unwrap()).unwrap();
    let id = status["id"].as_str().unwrap();
    let path = format!("/api/chain-replays/{id}");
    assert_eq!(status["phase"], "ready");
    assert_eq!(
        app.clone()
            .oneshot(request("POST", &format!("{path}/stage"), json!({}), true))
            .await
            .unwrap()
            .status(),
        StatusCode::ACCEPTED
    );
    let ready = waiting(&h, id).await;
    assert!(ready.import.unwrap().replay.is_some());
    assert_eq!(
        app.clone()
            .oneshot(request("DELETE", &path, json!({}), true))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        app.clone()
            .oneshot(request("POST", &format!("{path}/stop"), json!({}), true))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.clone()
            .oneshot(request("POST", &format!("{path}/stage"), json!({}), true))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        app.clone()
            .oneshot(request(
                "POST",
                &format!("{path}/stage"),
                json!({"resume":true}),
                true
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::ACCEPTED
    );
    waiting(&h, id).await;
    app.clone()
        .oneshot(request("POST", &format!("{path}/stop"), json!({}), true))
        .await
        .unwrap();
    assert_eq!(
        app.clone()
            .oneshot(request("DELETE", &path, json!({}), true))
            .await
            .unwrap()
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.clone()
            .oneshot(request("GET", &path, json!({}), true))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    assert!(
        h.c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        wafflery::library::bundle_store::BundleStore(h.c.assets.clone())
            .get(&record.id)
            .await
            .is_ok()
    );
    h.close().await;
}

#[tokio::test]
async fn explicitly_reordering_a_many_port_keeps_fresh_edges_and_records_the_new_order() {
    let h = Host::native(&[]).await;
    let state =
        h.c.start_chain_replay(
            original_with_external(true).await,
            vec!["peer".into(), "peer".into()],
        )
        .await
        .unwrap();
    h.c.stage_chain_replay(&state.id, false).await.unwrap();
    let ready = waiting(&h, &state.id).await;
    let mut first = request(&h, "order-producer", &ready).await;
    first.params["amount"] = json!(0.75);
    h.c.submit(first).await.unwrap();
    let producer = super::bundle_import_cases::owned(&h, "order-producer").await;
    h.settled("order-producer").await;
    h.c.chain_replay(&state.id).await.unwrap();
    h.c.stage_chain_replay(&state.id, false).await.unwrap();
    let ready = waiting(&h, &state.id).await;
    let mut next = request(&h, "order-consumer", &ready).await;
    next.bindings.get_mut("tokens").unwrap().reverse();
    next.lineage.get_mut("tokens").unwrap().reverse();
    h.c.submit(next).await.unwrap();
    let result = super::bundle_import_cases::owned(&h, "order-consumer").await;
    h.settled("order-consumer").await;
    assert_eq!(
        result.result.as_ref().unwrap()["invocation"]["arguments"]["tokens"][1],
        producer.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"]
    );
    h.close().await;
}
