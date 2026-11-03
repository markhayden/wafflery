use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use wafflery::library::{assets::AssetStore, blobs::BlobStore};
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[tokio::test]
async fn opaque_publication_is_verified_deduplicated_and_survives_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&temp.path().join("Library")).unwrap();
    let blobs = BlobStore(assets.clone());
    let bytes = b"native bytes, not a PNG\0\xff".to_vec();
    let hash = digest(&bytes);
    let first = blobs.publish(bytes.clone(), hash.clone()).await.unwrap();
    let second = blobs.publish(bytes.clone(), hash.clone()).await.unwrap();
    assert_eq!(first, second);
    assert_eq!(blobs.read(&hash).await.unwrap(), bytes);
    assert!(
        blobs
            .publish(b"corrupt".to_vec(), hash.clone())
            .await
            .is_err()
    );
    assets.database.shutdown().await.unwrap();
    let reopened = BlobStore(AssetStore::open(&temp.path().join("Library")).unwrap());
    assert_eq!(reopened.read(&hash).await.unwrap(), bytes);
    assert_eq!(reopened.list().await.unwrap().len(), 1);
}
#[tokio::test]
async fn failed_publication_leaves_no_metadata_and_never_clobbers_existing_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&temp.path().join("Library")).unwrap();
    let blobs = BlobStore(assets.clone());
    let hash = digest(b"correct");
    let parent = temp.path().join("Library/blobs/sha256").join(&hash[..2]);
    std::fs::create_dir(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(parent.join(&hash), b"wrong").unwrap();
    assert!(
        blobs
            .publish(b"correct".to_vec(), hash.clone())
            .await
            .is_err()
    );
    assert!(blobs.list().await.unwrap().is_empty());
    assert_eq!(std::fs::read(parent.join(hash)).unwrap(), b"wrong");
}

#[tokio::test]
async fn a_database_failure_after_publication_is_reconciled_without_overwriting_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&temp.path().join("Library")).unwrap();
    let blobs = BlobStore(assets.clone());
    assets
        .database
        .call(|db| {
            db.pragma_update(None, "query_only", true)?;
            Ok(())
        })
        .await
        .unwrap();
    let bytes = b"interrupted native export".to_vec();
    let hash = digest(&bytes);
    assert!(blobs.publish(bytes.clone(), hash.clone()).await.is_err());
    assert!(blobs.list().await.unwrap().is_empty());
    assets
        .database
        .call(|db| {
            db.pragma_update(None, "query_only", false)?;
            Ok(())
        })
        .await
        .unwrap();
    blobs.publish(bytes.clone(), hash.clone()).await.unwrap();
    assert_eq!(blobs.read(&hash).await.unwrap(), bytes);
}

#[tokio::test]
async fn a_pre_native_database_is_backed_up_and_migrated_without_losing_history() {
    let temp = tempfile::tempdir().unwrap();
    let library = temp.path().join("Library");
    std::os::unix::fs::DirBuilderExt::mode(&mut std::fs::DirBuilder::new(), 0o700)
        .create(&library)
        .unwrap();
    let path = library.join("library.sqlite3");
    let connection = rusqlite::Connection::open(&path).unwrap();
    for sql in [
        include_str!("../src/library/migrations/001-library.sql"),
        include_str!("../src/library/migrations/002-assets.sql"),
        include_str!("../src/library/migrations/003-attempts.sql"),
        include_str!("../src/library/migrations/004-baselines.sql"),
        include_str!("../src/library/migrations/005-examples.sql"),
    ] {
        connection.execute_batch(sql).unwrap();
    }
    let old_bytes = include_bytes!("../ui/tests/fixtures/sample.png");
    let old_hash = digest(old_bytes);
    let old_id = "0".repeat(32);
    let parent = library.join("assets/sha256").join(&old_hash[..2]);
    std::os::unix::fs::DirBuilderExt::mode(std::fs::DirBuilder::new().recursive(true), 0o700)
        .create(&parent)
        .unwrap();
    std::fs::write(parent.join(&old_hash), old_bytes).unwrap();
    connection.execute("INSERT INTO assets(id,sha256,size,width,height,media_type) VALUES(?1,?2,?3,160,100,'image/png')",rusqlite::params![old_id,old_hash,old_bytes.len() as u32]).unwrap();
    let request = serde_json::json!({"id":"retained","tool":"sample","action":"mark","revision":"r","params":{},"images":{"source":[old_id]}});
    let record = serde_json::json!({"id":"retained","sequence":1,"created_ms":0,"request":request,"arguments":{},"declaration":{},"inputs":{},"wire":null,"raw_response":null,"observations":{},"result":null,"execution":"completed","preservation":{"contract":"complete","native":"unavailable","report":"unavailable"},"outputs":[],"journal":null,"notices":[]});
    connection
        .execute(
            "INSERT INTO attempts(id,submission,record) VALUES('retained',?1,?2)",
            rusqlite::params![request.to_string(), record.to_string()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO attempt_assets(attempt_id,asset_id) VALUES('retained',?1)",
            [&old_id],
        )
        .unwrap();
    connection.pragma_update(None, "user_version", 5).unwrap();
    drop(connection);
    let assets = AssetStore::open(&temp.path().join("Library")).unwrap();
    let count: i64 = assets
        .database
        .call(|db| {
            Ok(db.query_row(
                "SELECT count(*) FROM attempts WHERE id='retained'",
                [],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(
        wafflery::library::attempts::AttemptStore::new(assets.database.clone())
            .get("retained")
            .await
            .unwrap()
            .request
            .images["source"],
        vec![old_id.clone()]
    );
    assert_eq!(assets.read(&old_id).await.unwrap().1, old_bytes);

    let backup = std::fs::read_dir(temp.path().join("Library"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("before-v7-")
        })
        .unwrap();
    let old = rusqlite::Connection::open(backup.path()).unwrap();
    assert_eq!(
        old.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        5
    );
    assert!(BlobStore(assets).list().await.unwrap().is_empty());
}

#[tokio::test]
async fn bundle_owners_share_bytes_without_overwriting_occurrence_metadata() {
    use serde_json::json;
    use wafflery::library::blob_links::{BlobLink, Owner};
    let temp = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&temp.path().join("Library")).unwrap();
    let blobs = BlobStore(assets);
    let bytes = b"opaque shared native bytes".to_vec();
    let hash = digest(&bytes);
    blobs.publish(bytes.clone(), hash.clone()).await.unwrap();
    let link = BlobLink {
        direction: "bundle".into(),
        occurrence: 0,
        sha256: hash.clone(),
        media_type: "application/octet-stream".into(),
        artifact: json!({"identity":"opaque"}),
    };
    blobs
        .attach(Owner::Bundle("one".into()), link.clone())
        .await
        .unwrap();
    blobs
        .attach(Owner::Bundle("two".into()), link.clone())
        .await
        .unwrap();
    blobs
        .attach(Owner::Bundle("one".into()), link.clone())
        .await
        .unwrap();
    let mut changed = link.clone();
    changed.artifact = json!({"identity":"different"});
    assert!(
        blobs
            .attach(Owner::Bundle("one".into()), changed)
            .await
            .is_err()
    );
    blobs.release_bundle("one".into()).await.unwrap();
    assert_eq!(
        blobs.links(Owner::Bundle("two".into())).await.unwrap(),
        vec![link]
    );
    assert_eq!(blobs.read(&hash).await.unwrap(), bytes);
}

#[tokio::test]
async fn attempts_examples_and_bundles_hold_native_bytes_independently_during_cleanup() {
    use serde_json::json;
    use wafflery::library::{
        attempts::{AttemptStore, Submission},
        blob_links::{BlobLink, Owner},
        cleanup::{Cleanup, Selection},
        examples::ExampleStore,
    };
    let temp = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&temp.path().join("Library")).unwrap();
    let blobs = BlobStore(assets.clone());
    let attempts = AttemptStore::new(assets.database.clone());
    for id in ["first", "second"] {
        let request:Submission=serde_json::from_value(json!({"id":id,"tool":"sample","action":"shape","revision":"r","params":{},"images":{}})).unwrap();
        attempts
            .create(request, json!({}), json!({}), json!({}))
            .await
            .unwrap();
    }
    let bytes = b"shared portable native data".to_vec();
    let hash = digest(&bytes);
    blobs.publish(bytes.clone(), hash.clone()).await.unwrap();
    let link = BlobLink {
        direction: "in".into(),
        occurrence: 0,
        sha256: hash.clone(),
        media_type: "application/octet-stream".into(),
        artifact: json!({"identity":"native"}),
    };
    blobs
        .attach(Owner::Attempt("first".into()), link.clone())
        .await
        .unwrap();
    blobs
        .attach(
            Owner::Attempt("first".into()),
            BlobLink {
                direction: "out".into(),
                ..link.clone()
            },
        )
        .await
        .unwrap();
    blobs
        .attach(
            Owner::Attempt("second".into()),
            BlobLink {
                direction: "out".into(),
                ..link.clone()
            },
        )
        .await
        .unwrap();
    let cleanup = Cleanup(assets.clone());
    let selection = Selection {
        attempts: vec!["first".into(), "second".into()],
        ..Default::default()
    };
    let stale = cleanup.preview(selection.clone()).await.unwrap();
    assert_eq!(stale.reclaimable_bytes, bytes.len() as u64);
    let example = ExampleStore(assets.database.clone())
        .save("first".into(), "Keep input".into(), "".into())
        .await
        .unwrap();
    assert!(
        cleanup
            .confirm(selection.clone(), stale.token)
            .await
            .is_err()
    );
    blobs
        .attach(
            Owner::Bundle("bundle".into()),
            BlobLink {
                direction: "bundle".into(),
                ..link.clone()
            },
        )
        .await
        .unwrap();
    let preview = cleanup.preview(selection.clone()).await.unwrap();
    assert!(preview.native_blobs.is_empty());
    cleanup.confirm(selection, preview.token).await.unwrap();
    assert_eq!(
        blobs
            .links(Owner::Example(example.id.clone()))
            .await
            .unwrap(),
        vec![link]
    );
    assert_eq!(blobs.read(&hash).await.unwrap(), bytes);
    let selection = Selection {
        examples: vec![example.id],
        ..Default::default()
    };
    let preview = cleanup.preview(selection.clone()).await.unwrap();
    assert_eq!(preview.reclaimable_bytes, 0);
    cleanup.confirm(selection, preview.token).await.unwrap();
    blobs.release_bundle("bundle".into()).await.unwrap();
    let selection = Selection {
        unused_assets: true,
        ..Default::default()
    };
    let preview = cleanup.preview(selection.clone()).await.unwrap();
    assert_eq!(preview.native_blobs.len(), 1);
    assert_eq!(preview.reclaimable_bytes, bytes.len() as u64);
    cleanup.confirm(selection, preview.token).await.unwrap();
    assert!(blobs.list().await.unwrap().is_empty());
    assert!(blobs.read(&hash).await.is_err());
}

#[tokio::test]
async fn cleanup_unlinks_only_the_owned_path_even_if_it_was_replaced_with_a_symlink() {
    use wafflery::library::cleanup::{Cleanup, Selection};
    let temp = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&temp.path().join("Library")).unwrap();
    let blobs = BlobStore(assets.clone());
    let bytes = b"tool workspace export".to_vec();
    let hash = digest(&bytes);
    let external = temp.path().join("tool-export.bin");
    std::fs::write(&external, &bytes).unwrap();
    blobs.publish(bytes.clone(), hash.clone()).await.unwrap();
    let owned = assets
        .root()
        .join("blobs/sha256")
        .join(&hash[..2])
        .join(&hash);
    std::fs::remove_file(&owned).unwrap();
    std::os::unix::fs::symlink(&external, &owned).unwrap();
    assert!(blobs.read(&hash).await.is_err());
    let cleanup = Cleanup(assets);
    let selection = Selection {
        unused_assets: true,
        ..Default::default()
    };
    let preview = cleanup.preview(selection.clone()).await.unwrap();
    cleanup.confirm(selection, preview.token).await.unwrap();
    assert_eq!(std::fs::read(external).unwrap(), bytes);
    assert!(!owned.exists());
}

#[tokio::test]
async fn native_publication_rejects_oversize_bytes_and_unsafe_digests_before_writing() {
    use wafflery::library::blobs::MAX_BLOB_BYTES;
    let temp = tempfile::tempdir().unwrap();
    let store = BlobStore(AssetStore::open(&temp.path().join("Library")).unwrap());
    assert!(store.publish(vec![], "../outside".into()).await.is_err());
    assert!(
        store
            .publish(vec![0; (MAX_BLOB_BYTES + 1) as usize], "a".repeat(64))
            .await
            .is_err()
    );
    assert!(store.list().await.unwrap().is_empty());
}

#[tokio::test]
async fn stale_metadata_never_becomes_a_new_owner_of_missing_bytes() {
    use serde_json::json;
    use wafflery::library::blob_links::{BlobLink, Owner};
    let temp = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&temp.path().join("Library")).unwrap();
    let store = BlobStore(assets.clone());
    let hash = digest(b"missing bytes");
    store
        .publish(b"missing bytes".to_vec(), hash.clone())
        .await
        .unwrap();
    std::fs::remove_file(
        assets
            .root()
            .join("blobs/sha256")
            .join(&hash[..2])
            .join(&hash),
    )
    .unwrap();
    let link = BlobLink {
        direction: "bundle".into(),
        occurrence: 0,
        sha256: hash,
        media_type: "application/octet-stream".into(),
        artifact: json!({"identity":"native"}),
    };
    assert!(
        store
            .attach(Owner::Bundle("never-created".into()), link)
            .await
            .is_err()
    );
    assert!(
        store
            .links(Owner::Bundle("never-created".into()))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn deleting_one_attempt_never_reclaims_a_surviving_attempts_blob() {
    use serde_json::json;
    use wafflery::library::{
        attempts::{AttemptStore, Submission},
        blob_links::{BlobLink, Owner},
        cleanup::{Cleanup, Selection},
    };
    let temp = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&temp.path().join("Library")).unwrap();
    let blobs = BlobStore(assets.clone());
    let bytes = b"two attempt owners".to_vec();
    let hash = digest(&bytes);
    blobs.publish(bytes.clone(), hash.clone()).await.unwrap();
    let attempts = AttemptStore::new(assets.database.clone());
    for id in ["one", "two"] {
        let request:Submission=serde_json::from_value(json!({"id":id,"tool":"sample","action":"shape","revision":"r","params":{},"images":{}})).unwrap();
        attempts
            .create(request, json!({}), json!({}), json!({}))
            .await
            .unwrap();
        blobs
            .attach(
                Owner::Attempt(id.into()),
                BlobLink {
                    direction: "out".into(),
                    occurrence: 0,
                    sha256: hash.clone(),
                    media_type: "application/octet-stream".into(),
                    artifact: json!({"identity":"native"}),
                },
            )
            .await
            .unwrap();
    }
    let cleanup = Cleanup(assets);
    for (id, expected) in [("one", 0), ("two", bytes.len() as u64)] {
        let selection = Selection {
            attempts: vec![id.into()],
            ..Default::default()
        };
        let preview = cleanup.preview(selection.clone()).await.unwrap();
        assert_eq!(preview.reclaimable_bytes, expected);
        cleanup.confirm(selection, preview.token).await.unwrap();
        if id == "one" {
            assert_eq!(blobs.read(&hash).await.unwrap(), bytes);
            assert_eq!(
                blobs
                    .links(Owner::Attempt("two".into()))
                    .await
                    .unwrap()
                    .len(),
                1
            );
        }
    }
    assert!(blobs.list().await.unwrap().is_empty());
}

#[tokio::test]
async fn owner_occurrence_bounds_do_not_truncate_successfully_attached_links() {
    use serde_json::json;
    use wafflery::library::blob_links::{BlobLink, Owner};
    let temp = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&temp.path().join("Library")).unwrap();
    let store = BlobStore(assets.clone());
    let hash = digest(b"bounded links");
    store
        .publish(b"bounded links".to_vec(), hash.clone())
        .await
        .unwrap();
    let saved = hash.clone();
    assets.database.call(move|db|{db.execute("WITH RECURSIVE items(n) AS (VALUES(0) UNION ALL SELECT n+1 FROM items WHERE n<999) INSERT INTO native_links(owner_kind,owner_id,direction,occurrence,sha256,media_type,artifact) SELECT 'bundle','limited','bundle',n,?1,'application/octet-stream','{}' FROM items",[saved])?;Ok(())}).await.unwrap();
    let link = BlobLink {
        direction: "bundle".into(),
        occurrence: 0,
        sha256: hash,
        media_type: "application/octet-stream".into(),
        artifact: json!({}),
    };
    store
        .attach(Owner::Bundle("limited".into()), link.clone())
        .await
        .unwrap();
    assert!(
        store
            .attach(
                Owner::Bundle("limited".into()),
                BlobLink {
                    occurrence: 1000,
                    ..link
                }
            )
            .await
            .is_err()
    );
    assert_eq!(
        store
            .links(Owner::Bundle("limited".into()))
            .await
            .unwrap()
            .len(),
        1000
    );
}
