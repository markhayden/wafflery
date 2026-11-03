use super::*;
use wafflery::library::{
    cleanup::{Cleanup, Selection},
    examples::ExampleStore,
};

#[tokio::test]
async fn a_preserved_example_survives_source_deletion_and_restores_into_a_new_registration() {
    let h = Host::native(&[]).await;
    let mut seed = h.request("seed", "make_token", json!({})).await;
    seed.images
        .insert("source".into(), vec![h.image.id.clone()]);
    h.c.submit(seed).await.unwrap();
    h.settled("seed").await;
    // Native preservation runs independently of action completion.
    tokio::time::timeout(Duration::from_secs(5), async {
        while h.c.attempts.get("seed").await.unwrap().preservation["native"] != "preserved" {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let seed = h.c.attempts.get("seed").await.unwrap();
    let token = seed.result.unwrap()["artifacts"]["out"][0]["artifact"].clone();
    let mut soften = h
        .request(
            "softened",
            "soften_token",
            json!({"token":[{"kind":"artifact","reference":token}]}),
        )
        .await;
    soften.lineage =
        serde_json::from_value(json!({"token":[{"attempt":"seed","port":"token","occurrence":0}]}))
            .unwrap();
    h.c.submit(soften).await.unwrap();
    h.settled("softened").await;
    tokio::time::timeout(Duration::from_secs(5), async {
        while h.c.attempts.get("softened").await.unwrap().preservation["native"] != "preserved" {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let examples = ExampleStore(h.c.assets.database.clone());
    let saved = examples
        .save_with_options(
            "softened".into(),
            "Native soften".into(),
            "Change radius".into(),
            false,
        )
        .await
        .unwrap();
    assert!(!saved.workspace_dependent);
    assert_eq!(saved.input_artifacts[0]["artifact"], token);
    let selection = Selection {
        attempts: vec!["seed".into(), "softened".into()],
        ..Default::default()
    };
    let preview = Cleanup(h.c.assets.clone())
        .preview(selection.clone())
        .await
        .unwrap();
    h.c.cleanup(selection, preview.token).await.unwrap();
    h.close().await;
    std::fs::remove_dir_all(h.root.path().join("peer/artifacts")).unwrap();
    let c = Arc::new(
        Coordinator::open(
            h.c.assets.clone(),
            vec![Descriptor {
                id: "moved".into(),
                executable: Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("target/debug/examples/sample-lab"),
                args: vec![
                    h.root.path().join("new-store").display().to_string(),
                    "v04-operations".into(),
                ],
                cwd: h.root.path().into(),
                allow_dirs: vec![h.root.path().into()],
                evidence_dirs: vec![h.root.path().into()],
                config: None,
            }],
        )
        .await
        .unwrap(),
    );
    assert!(
        c.inspect_example_artifact(&saved.id, "token", 0, "moved")
            .await
            .is_err()
    );
    c.import_example_artifact(&saved.id, "token", 0, "moved")
        .await
        .unwrap();
    c.inspect_example_artifact(&saved.id, "token", 0, "moved")
        .await
        .unwrap();
    let mut request = saved.request.clone();
    request.id = "from-example".into();
    request.tool = "moved".into();
    request.revision = c.description("moved").await.unwrap().revision;
    request.lineage = serde_json::from_value(
        json!({"token":[{"example":saved.id,"port":"token","occurrence":0}]}),
    )
    .unwrap();
    request.params["amount"] = json!(0.5);
    let mut forged = request.clone();
    forged.id = "wrong-source".into();
    forged.lineage = serde_json::from_value(
        json!({"token":[{"example":saved.id,"port":"missing","occurrence":0}]}),
    )
    .unwrap();
    assert!(c.submit(forged).await.is_err());
    assert!(
        c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .is_empty()
    );
    c.submit(request).await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while c.attempts.get("from-example").await.unwrap().preservation["contract"] == "pending" {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        c.attempts.get("from-example").await.unwrap().execution,
        "completed"
    );
    assert_eq!(
        c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        !h.root.path().join("peer/artifacts").exists(),
        "the host never searches or recreates the old store"
    );
    let (tx, mut rx) = tokio::sync::watch::channel(false);
    c.begin_shutdown(true, tx).await.unwrap();
    tokio::time::timeout(Duration::from_secs(8), rx.wait_for(|v| *v))
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn an_unpreserved_native_example_requires_explicit_workspace_dependent_confirmation() {
    let h = Host::new().await;
    let mut seed = h.request("seed", "make_token", json!({})).await;
    seed.images
        .insert("source".into(), vec![h.image.id.clone()]);
    h.c.submit(seed).await.unwrap();
    let seed = h.settled("seed").await;
    let reference = seed.result.unwrap()["artifacts"]["out"][0]["artifact"].clone();
    h.c.submit(
        h.request(
            "native",
            "soften_token",
            json!({"token":[{"kind":"artifact","reference":reference}]}),
        )
        .await,
    )
    .await
    .unwrap();
    h.settled("native").await;
    let examples = ExampleStore(h.c.assets.database.clone());
    assert!(
        examples
            .save_with_options("native".into(), "Needs store".into(), "".into(), false)
            .await
            .is_err()
    );
    assert!(examples.list(None).await.unwrap().is_empty());
    let saved = examples
        .save_with_options("native".into(), "Needs store".into(), "".into(), true)
        .await
        .unwrap();
    assert!(saved.workspace_dependent);
    h.close().await;
}

#[tokio::test]
async fn historical_examples_without_input_occurrences_can_use_a_live_store_but_cannot_claim_import()
 {
    let h = Host::native(&[]).await;
    let mut request = h.request("seed", "make_token", json!({})).await;
    request
        .images
        .insert("source".into(), vec![h.image.id.clone()]);
    h.c.submit(request).await.unwrap();
    let seed = h.settled("seed").await;
    let reference = seed.result.unwrap()["artifacts"]["out"][0]["artifact"].clone();
    h.c.submit(
        h.request(
            "source",
            "soften_token",
            json!({"token":[{"kind":"artifact","reference":reference}]}),
        )
        .await,
    )
    .await
    .unwrap();
    h.settled("source").await;
    let saved = ExampleStore(h.c.assets.database.clone())
        .save_with_options("source".into(), "Historical native".into(), "".into(), true)
        .await
        .unwrap();
    let id = saved.id.clone();
    // This is the exact conservative metadata produced by the schema-6 migration case in examples.rs.
    h.c.assets.database.call(move|db|{db.execute("UPDATE examples SET record=json_set(record,'$.input_artifacts',json('[]'),'$.workspace_dependent',json('true')) WHERE id=?1",[id])?;Ok(())}).await.unwrap();
    h.c.inspect_example_artifact(&saved.id, "token", 0, "peer")
        .await
        .unwrap();
    let error =
        h.c.import_example_artifact(&saved.id, "token", 0, "peer")
            .await
            .unwrap_err();
    assert!(error.to_string().contains("occurrence is unavailable"));
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
