use fp::logic::database::Database;
use fp::logic::database::models::{File, MigrationVersion};
use rusqlite::fallible_streaming_iterator::FallibleStreamingIterator;

async fn is_migration_version_table_available(db: &Database) -> bool {
    let table_count = db
        .connection
        .lock()
        .await
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='migration_version';")
        .unwrap()
        .query_map([], |row| Ok(row.get::<usize, String>(0).unwrap()))
        .unwrap()
        .count();
    table_count > 0
}

#[tokio::test]
async fn test_create_database() {
    let mut db = Database::new(None);

    db.connect().await.expect("Failed to connect to database");

    // Check if the migration_version table exists
    assert!(is_migration_version_table_available(&db).await);

    // get all versions
    let versions: Vec<MigrationVersion> = db
        .connection
        .lock()
        .await
        .prepare("SELECT * FROM migration_version;")
        .unwrap()
        .query_map([], |row| {
            Ok(MigrationVersion {
                id: row.get(0)?,
                version: row.get(1)?,
                description: row.get(2)?,
            })
        })
        .expect("Failed to query migration_version table")
        .collect::<Result<_, _>>()
        .unwrap();

    // validate the number of versions (update this if new migrations are added)
    assert_eq!(versions.len(), 1);

    // Check initial migration version
    let initial_version = 1;
    assert_eq!(versions[0].version, initial_version);
    assert_eq!(versions[0].description, "Initial migration".to_string());
}

#[tokio::test]
async fn test_validate_initial_and_reconnection() {
    // Connect for the first time and execute migrations
    let mut db = Database::new(None);
    db.connect().await.expect("Failed to connect to database");
    assert!(is_migration_version_table_available(&db).await);
    db.disconnect();

    // Connect again and validate the migration_version table still exists
    db.connect().await.expect("Failed to connect to database");
    assert!(is_migration_version_table_available(&db).await);
}

#[tokio::test]
async fn test_validate_file_table_creation() {
    let mut db = Database::new(None);
    db.connect().await.expect("Failed to connect to database");

    // Check if the file table exists
    assert!(
        db.connection
            .lock()
            .await
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='file';")
            .unwrap()
            .query([])
            .unwrap()
            .count()
            .unwrap()
            > 0
    );
}

async fn get_file_by_id(db: &Database, file_id: i32) -> Option<File> {
    let result = db
        .connection
        .lock()
        .await
        .prepare("SELECT * FROM file WHERE id = ?1;")
        .unwrap()
        .query_map([file_id], |row| {
            Ok(File {
                id: row.get(0)?,
                server_id: row.get(1)?,
                added_date: row.get(2)?,
                finish_date: row.get(3)?,
            })
        })
        .expect("Failed to query file table")
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    if result.is_empty() {
        None
    } else {
        Some(result[0].clone())
    }
}

fn assert_files_equal(file1: &File, file2: &File) {
    assert_eq!(file1.server_id, file2.server_id);
    assert_eq!(file1.added_date, file2.added_date);
    assert_eq!(file1.finish_date, file2.finish_date);
}

#[tokio::test]
async fn test_create_file() {
    let mut db = Database::new(None);
    db.connect().await.expect("Failed to connect to database");

    let file = File {
        id: 0,
        server_id: 1,
        added_date: 1625079600,
        finish_date: None,
    };
    let id1 = db.create_or_update_file(file.clone()).await;

    let created_file = get_file_by_id(&db, id1)
        .await
        .expect("File not found after creation");
    assert_files_equal(&created_file, &file);

    // Update the same file
    let updated_file = File {
        id: 0,
        server_id: 1,
        added_date: 1625079601,
        finish_date: Some(1625083200),
    };
    let id2 = db.create_or_update_file(updated_file.clone()).await;
    assert_eq!(id1, id2, "File IDs should be the same after update");
    let fetched_updated_file = get_file_by_id(&db, id2)
        .await
        .expect("File not found after update");
    assert_files_equal(&fetched_updated_file, &updated_file);

    // Update with new finish_date only
    let finish_date_only_update = File {
        id: 0,
        server_id: 1,
        added_date: 1625079601,
        finish_date: Some(1625086800),
    };
    let id3 = db
        .create_or_update_file(finish_date_only_update.clone())
        .await;
    assert_eq!(
        id1, id3,
        "File IDs should be the same after finish_date update"
    );
    let fetched_finish_date_only_file = get_file_by_id(&db, id3)
        .await
        .expect("File not found after finish_date update");
    assert_eq!(fetched_finish_date_only_file.finish_date, Some(1625083200));
}

#[tokio::test]
async fn test_remove_no_matching_files_ids() {
    let mut db = Database::new(None);
    db.connect().await.expect("Failed to connect to database");

    let file1 = File {
        id: 0,
        server_id: 1,
        added_date: 1625079600,
        finish_date: None,
    };
    let file2 = File {
        id: 0,
        server_id: 2,
        added_date: 1625079601,
        finish_date: None,
    };
    let file3 = File {
        id: 0,
        server_id: 3,
        added_date: 1625079602,
        finish_date: None,
    };

    let id1 = db.create_or_update_file(file1).await;
    let id2 = db.create_or_update_file(file2).await;
    let id3 = db.create_or_update_file(file3).await;

    db.remove_no_matching_files_ids(&vec![id1, id3]).await;

    assert!(
        get_file_by_id(&db, id1).await.is_some(),
        "File 1 should exist"
    );
    assert!(
        get_file_by_id(&db, id2).await.is_none(),
        "File 2 should be removed"
    );
    assert!(
        get_file_by_id(&db, id3).await.is_some(),
        "File 3 should exist"
    );
}

#[tokio::test]
async fn test_list_files() {
    let mut db = Database::new(None);
    db.connect().await.expect("Failed to connect to database");

    let file1 = File {
        id: 0,
        server_id: 1,
        added_date: 1625079600,
        finish_date: None,
    };
    let file2 = File {
        id: 0,
        server_id: 2,
        added_date: 1625079601,
        finish_date: Some(1625083200),
    };

    db.create_or_update_file(file1.clone()).await;
    db.create_or_update_file(file2.clone()).await;

    let files: Vec<File> = db
        .connection
        .lock()
        .await
        .prepare("SELECT * FROM file;")
        .unwrap()
        .query_map([], |row| {
            Ok(File {
                id: row.get(0)?,
                server_id: row.get(1)?,
                added_date: row.get(2)?,
                finish_date: row.get(3)?,
            })
        })
        .expect("Failed to query file table")
        .collect::<Result<_, _>>()
        .unwrap();

    assert_eq!(files.len(), 2, "There should be 2 files in the database");
    assert_files_equal(&files[0], &file1);
    assert_files_equal(&files[1], &file2);
}

#[tokio::test]
async fn test_get_file_by_server_id() {
    let mut db = Database::new(None);
    db.connect().await.expect("Failed to connect to database");

    let file = File {
        id: 0,
        server_id: 42,
        added_date: 1625079600,
        finish_date: None,
    };

    db.create_or_update_file(file.clone()).await;

    let fetched_file = db
        .get_file_by_server_id(42)
        .await
        .expect("File not found by server ID");
    assert_files_equal(&fetched_file, &file);

    let non_existent_file = db.get_file_by_server_id(999).await;
    assert!(
        non_existent_file.is_none(),
        "Non-existent file should return None"
    );
}
