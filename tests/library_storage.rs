use wafflehouse::library::db::Database;

#[tokio::test]
async fn database_reopens_with_the_same_identity_and_required_pragmas() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("library.sqlite3");
    let db = Database::open(&path).unwrap();
    let identity: String = db
        .call(|connection| {
            Ok(connection.query_row("SELECT id FROM library_meta", [], |row| row.get(0))?)
        })
        .await
        .unwrap();
    let values: (String, i64, i64, i64) = db
        .call(|connection| {
            Ok((
                connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?,
                connection.pragma_query_value(None, "synchronous", |row| row.get(0))?,
                connection.pragma_query_value(None, "foreign_keys", |row| row.get(0))?,
                connection.pragma_query_value(None, "busy_timeout", |row| row.get(0))?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(values, ("delete".into(), 2, 1, 5000));
    tokio::time::timeout(std::time::Duration::from_secs(2), db.shutdown())
        .await
        .unwrap()
        .unwrap();
    assert!(db.call(|_| Ok(())).await.is_err());
    let db = Database::open(&path).unwrap();
    let reopened: String = db
        .call(|connection| {
            Ok(connection.query_row("SELECT id FROM library_meta", [], |row| row.get(0))?)
        })
        .await
        .unwrap();
    assert_eq!(identity, reopened);
    db.shutdown().await.unwrap();
}

#[test]
fn newer_schemas_are_rejected_without_changing_the_file() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("library.sqlite3");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("CREATE TABLE future_data(value TEXT); INSERT INTO future_data VALUES ('preserve me'); PRAGMA user_version=999;").unwrap();
    drop(connection);
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        Database::open(&path),
        Err(wafflehouse::library::Error::NewerSchema(999))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[tokio::test]
async fn write_errors_are_reported_without_losing_prior_data() {
    let temp = tempfile::tempdir().unwrap();
    let db = Database::open(&temp.path().join("library.sqlite3")).unwrap();
    db.call(|connection| {
        connection.pragma_update(None, "query_only", true)?;
        Ok(())
    })
    .await
    .unwrap();
    assert!(
        db.call(|connection| {
            connection.execute("DELETE FROM library_meta", [])?;
            Ok(())
        })
        .await
        .is_err()
    );
    let count: i64 = db
        .call(|connection| {
            Ok(connection.query_row("SELECT count(*) FROM library_meta", [], |row| row.get(0))?)
        })
        .await
        .unwrap();
    assert_eq!(count, 1);
    db.shutdown().await.unwrap();
}

#[test]
fn migration_failure_rolls_back_and_retains_a_readable_backup() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("library.sqlite3");
    let original = rusqlite::Connection::open(&path).unwrap();
    original
        .execute_batch(
            "CREATE TABLE library_meta(note TEXT); INSERT INTO library_meta VALUES ('keep');",
        )
        .unwrap();
    drop(original);
    assert!(Database::open(&path).is_err());
    let original = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        original
            .query_row("SELECT note FROM library_meta", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
    assert_eq!(
        original
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        0
    );
    let backup = std::fs::read_dir(temp.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .find(|entry| entry.file_name().to_string_lossy().starts_with("before-v"))
        .unwrap();
    let connection = rusqlite::Connection::open(backup.path()).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT note FROM library_meta", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
}

#[test]
fn database_symlinks_are_not_followed() {
    let temp = tempfile::tempdir().unwrap();
    let original = temp.path().join("untouched");
    std::fs::write(&original, "untouched").unwrap();
    let link = temp.path().join("library.sqlite3");
    std::os::unix::fs::symlink(&original, &link).unwrap();
    assert!(Database::open(&link).is_err());
    assert_eq!(std::fs::read_to_string(original).unwrap(), "untouched");
}

#[test]
fn invalid_schema_revisions_are_reported_without_worker_panics() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("library.sqlite3");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.pragma_update(None, "user_version", -1).unwrap();
    drop(connection);
    assert!(matches!(
        Database::open(&path),
        Err(wafflehouse::library::Error::InvalidSchema(-1))
    ));
}
