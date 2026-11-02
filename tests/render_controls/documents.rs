use super::*;

#[tokio::test]
async fn document_view_remains_ready_after_renderer_exit_and_activity_keeps_its_identity() {
    let h = Host::new().await;
    let before = serde_json::to_value(h.c.attempts.get("saved").await.unwrap()).unwrap();
    let path = "/api/attempts/saved/views";
    let (status, opened) = h.request("POST", path, json!({"name":"document"})).await;
    assert_eq!(status, StatusCode::OK);
    let id = opened["id"].as_str().unwrap();
    let ready = h.state(id, "ready").await;
    assert_eq!(ready["exit_code"], 0);
    assert_eq!(ready["kind"], "document");
    assert!(ready["error"].is_null());
    for key in ["url", "stdout", "path"] {
        assert!(ready.get(key).is_none());
    }
    let (status, activity) = h
        .request("POST", &format!("/api/views/{id}/activity"), json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(activity["phase"], "ready");
    assert_eq!(
        h.request("POST", path, json!({"name":"document"})).await.1["id"],
        id
    );
    assert_eq!(
        h.request("POST", path, json!({"name":"document-gated"}))
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        h.request("POST", &format!("/api/views/{id}/close"), json!({}))
            .await
            .0,
        StatusCode::OK
    );
    h.state(id, "exited").await;
    assert_eq!(
        h.request("POST", &format!("/api/views/{id}/activity"), json!({}))
            .await
            .0,
        StatusCode::CONFLICT
    );
    let next = h.request("POST", path, json!({"name":"document"})).await;
    assert_eq!(next.0, StatusCode::OK);
    assert_ne!(next.1["id"], id);
    h.state(next.1["id"].as_str().unwrap(), "ready").await;
    assert_eq!(
        serde_json::to_value(h.c.attempts.get("saved").await.unwrap()).unwrap(),
        before
    );
    h.finish().await;
    h.state(next.1["id"].as_str().unwrap(), "exited").await;
}

#[tokio::test]
async fn invalid_document_outputs_never_change_the_saved_run_or_become_ready() {
    let h = Host::new().await;
    let before = serde_json::to_value(h.c.attempts.get("saved").await.unwrap()).unwrap();
    for name in ["document-missing", "document-extra", "document-inside"] {
        let (status, opened) = h
            .request("POST", "/api/attempts/saved/views", json!({"name":name}))
            .await;
        assert_eq!(status, StatusCode::OK);
        let state = h.state(opened["id"].as_str().unwrap(), "exited").await;
        assert!(state["error"].as_str().unwrap().contains("standalone file"));
        assert_eq!(
            serde_json::to_value(h.c.attempts.get("saved").await.unwrap()).unwrap(),
            before
        );
    }
    h.finish().await;
}

#[tokio::test]
async fn completed_document_renderers_still_occupy_their_live_view_slots() {
    let h = Host::new().await;
    let original = h.c.attempts.get("saved").await.unwrap();
    let mut ids = Vec::new();
    for index in 0..17 {
        let attempt = if index == 0 {
            "saved".into()
        } else {
            let mut request = original.request.clone();
            request.id = format!("document-{index}");
            h.c.submit(request.clone()).await.unwrap();
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if h.c.attempts.get(&request.id).await.unwrap().preservation["contract"]
                        == "complete"
                        && h.c.snapshot().await.unwrap()["tools"][0]["busy"] == false
                    {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            request.id
        };
        let path = format!("/api/attempts/{attempt}/views");
        let (status, opened) = h.request("POST", &path, json!({"name":"document"})).await;
        if index < 16 {
            assert_eq!(status, StatusCode::OK);
            let id = opened["id"].as_str().unwrap().to_owned();
            assert_eq!(h.state(&id, "ready").await["exit_code"], 0);
            ids.push(id);
        } else {
            assert_eq!(status, StatusCode::CONFLICT);
            h.request("POST", &format!("/api/views/{}/close", ids[0]), json!({}))
                .await;
            h.state(&ids[0], "exited").await;
            assert_eq!(
                h.request("POST", &path, json!({"name":"document"})).await.0,
                StatusCode::OK
            );
        }
    }
    h.finish().await;
}
