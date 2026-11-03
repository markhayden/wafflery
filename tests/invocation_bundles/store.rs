use super::*;
use wafflery::library::{
    bundle_store::BundleStore,
    cleanup::{Cleanup, Selection},
};
#[tokio::test]
async fn saved_bundles_keep_identical_bytes_and_input_ownership_after_source_deletion_and_restart()
{
    let (root, assets, attempt, _marker) = fixture().await;
    let built = bundles::build(&assets, &attempt, &Options::default())
        .await
        .unwrap();
    let bytes = std::fs::read(built.file.path()).unwrap();
    let store = BundleStore(assets.clone());
    let saved = store.save(&attempt.id, built).await.unwrap();
    assert_eq!(saved.sha256, saved.id);
    let second = store
        .save(
            &attempt.id,
            bundles::build(&assets, &attempt, &Options::default())
                .await
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(saved.id, second.id);
    assert_eq!(store.list(None, None).await.unwrap().items.len(), 1);
    let selection = Selection {
        attempts: vec![attempt.id.clone()],
        unused_assets: true,
        ..Default::default()
    };
    let preview = Cleanup(assets.clone())
        .preview(selection.clone())
        .await
        .unwrap();
    assert!(preview.assets.is_empty());
    assert!(preview.native_blobs.is_empty());
    assert_eq!(preview.bundle_count, 1);
    assert_eq!(preview.total_bundle_bytes, saved.size);
    Cleanup(assets.clone())
        .confirm(selection, preview.token)
        .await
        .unwrap();
    assert!(
        AttemptStore::new(assets.database.clone())
            .get(&attempt.id)
            .await
            .is_err()
    );
    assert_eq!(
        store
            .list(None, Some(attempt.id))
            .await
            .unwrap()
            .items
            .len(),
        0
    );
    assert_eq!(store.list(None, None).await.unwrap().items.len(), 1);
    assert_eq!(stored_bytes(&store, &saved.id).await, bytes);
    assets.database.shutdown().await.unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let store = BundleStore(assets.clone());
    assert_eq!(store.get(&saved.id).await.unwrap().manifest, saved.manifest);
    assert_eq!(stored_bytes(&store, &saved.id).await, bytes);
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn failed_publication_commits_no_bundle_or_input_ownership() {
    let (_root, assets, attempt, _marker) = fixture().await;
    let built = bundles::build(&assets, &attempt, &Options::default())
        .await
        .unwrap();
    let id = built.sha256.clone();
    assets.database.call(|db|{db.execute_batch("CREATE TRIGGER fail_bundle BEFORE INSERT ON invocation_bundles BEGIN SELECT RAISE(ABORT,'synthetic save failure'); END;")?;Ok(())}).await.unwrap();
    let store = BundleStore(assets.clone());
    assert!(store.save(&attempt.id, built).await.is_err());
    assert!(store.list(None, None).await.unwrap().items.is_empty());
    assert!(
        BlobStore(assets.clone())
            .links(Owner::Bundle(id.clone()))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        !assets
            .root()
            .join("bundles/sha256")
            .join(&id[..2])
            .join(id)
            .exists()
    );
    assert_eq!(
        std::fs::read_dir(assets.root().join("staging"))
            .unwrap()
            .count(),
        0
    );
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn downloads_verify_the_stored_archive_and_never_accept_caller_paths() {
    let (_root, assets, attempt, _marker) = fixture().await;
    let store = BundleStore(assets.clone());
    let saved = store
        .save(
            &attempt.id,
            bundles::build(&assets, &attempt, &Options::default())
                .await
                .unwrap(),
        )
        .await
        .unwrap();
    for id in ["../../escape", "", "ABC"] {
        assert!(store.get(id).await.is_err());
        assert!(store.open(id).await.is_err());
    }
    let path = assets
        .root()
        .join("bundles/sha256")
        .join(&saved.id[..2])
        .join(&saved.id);
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(path, b"changed archive").unwrap();
    assert!(store.open(&saved.id).await.is_err());
    assets.database.shutdown().await.unwrap();
}

async fn stored_bytes(store: &BundleStore, id: &str) -> Vec<u8> {
    let (mut file, _) = store.open(id).await.unwrap();
    let mut bytes = vec![];
    file.read_to_end(&mut bytes).unwrap();
    bytes
}

#[tokio::test]
async fn private_bundle_api_reviews_saved_evidence_before_a_repeatable_download_without_dispatch() {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use std::sync::Arc;
    use tower::ServiceExt;
    use wafflery::{
        host::{api, auth::Auth},
        tools::coordinator::Coordinator,
    };
    let (_root, assets, attempt, _) = fixture().await;
    let original = attempt.clone();
    AttemptStore::new(assets.database.clone())
        .update(&attempt.id, move |saved| {
            *saved = original;
            Ok(())
        })
        .await
        .unwrap();
    let c = Arc::new(Coordinator::open(assets.clone(), vec![]).await.unwrap());
    let auth = Auth::new().unwrap();
    let session = auth.exchange(&auth.issue().unwrap()).unwrap().unwrap();
    let (quit, _) = tokio::sync::watch::channel(false);
    let origin = "http://127.0.0.1:45678";
    let app = api::router_with_tools(origin, auth, quit, assets.clone(), Some(c.clone()));
    let request = |method: &str, path: &str, body: &str, authorized: bool| {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header("host", "127.0.0.1:45678")
            .header("origin", origin)
            .header("content-type", "application/json");
        if authorized {
            request = request.header("authorization", format!("Bearer {}", session.token));
        }
        request.body(Body::from(body.to_owned())).unwrap()
    };
    let denied = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/attempts/selected/bundles",
            "{}",
            false,
        ))
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    let wrong = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/attempts/selected/bundles",
            r#"{"path":"/private/unrelated"}"#,
            true,
        ))
        .await
        .unwrap();
    assert_eq!(wrong.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/attempts/selected/bundles",
            "{}",
            true,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let saved: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    let id = saved["id"].as_str().unwrap();
    assert_eq!(
        saved["manifest"]["steps"][0]["invocation"]["arguments"]["amount"],
        0.75
    );
    let path = format!("/api/bundles/{id}/download");
    let interrupted = app
        .clone()
        .oneshot(request("GET", &path, "", true))
        .await
        .unwrap();
    assert_eq!(interrupted.status(), StatusCode::OK);
    drop(interrupted);
    let response = app
        .clone()
        .oneshot(request("GET", &path, "", true))
        .await
        .unwrap();
    assert_eq!(response.headers()["content-type"], "application/x-tar");
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(hash(&bytes), id);
    let response = app
        .clone()
        .oneshot(request("GET", "/api/bundles", "", true))
        .await
        .unwrap();
    let page: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(c.close().await);
    assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn source_deletion_between_assembly_and_commit_is_a_friendly_conflict_with_no_publication() {
    let (_root, assets, attempt, _) = fixture().await;
    let built = bundles::build(&assets, &attempt, &Options::default())
        .await
        .unwrap();
    let id = built.sha256.clone();
    let source = attempt.id.clone();
    assets
        .database
        .call(move |db| {
            db.execute("DELETE FROM attempts WHERE id=?1", [source])?;
            Ok(())
        })
        .await
        .unwrap();
    let store = BundleStore(assets.clone());
    assert!(matches!(
        store.save(&attempt.id, built).await,
        Err(wafflery::library::Error::Conflict(_))
    ));
    assert!(store.list(None, None).await.unwrap().items.is_empty());
    assert!(
        !assets
            .root()
            .join("bundles/sha256")
            .join(&id[..2])
            .join(&id)
            .exists()
    );
    assert_eq!(
        std::fs::read_dir(assets.root().join("staging"))
            .unwrap()
            .count(),
        0
    );
    assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_explicitly_imported_archive_is_retained_without_inventing_an_attempt() {
    let (_source_root, source, bytes) = super::reader::exported().await;
    let parsed = wafflery::tools::bundle_reader::read(bytes.clone()).unwrap();
    let root = tempfile::tempdir().unwrap();
    let target = AssetStore::open(&root.path().join("Library")).unwrap();
    let saved = wafflery::tools::bundle_import::retain(&target, &parsed)
        .await
        .unwrap();
    let again = wafflery::tools::bundle_import::retain(&target, &parsed)
        .await
        .unwrap();
    assert_eq!(saved, again);
    assert_eq!(saved.id, hash(&bytes));
    let (mut file, _) = BundleStore(target.clone()).open(&saved.id).await.unwrap();
    let mut retained = vec![];
    file.read_to_end(&mut retained).unwrap();
    assert_eq!(retained, bytes);
    assert!(
        AttemptStore::new(target.database.clone())
            .history(None, 10, None, None)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        BundleStore(target.clone())
            .list(None, None)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    assert!(
        target
            .database
            .call(|db| Ok(
                db.query_row("SELECT count(*) FROM bundle_attempts", [], |r| r
                    .get::<_, u32>(0))?
            ))
            .await
            .unwrap()
            == 0
    );
    source.database.shutdown().await.unwrap();
    target.database.shutdown().await.unwrap();
}
