use crate::logic::database::models::File;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

mod migrations_manager;
pub mod models;

pub struct Database {
    in_memory: bool,
    database_path: Option<String>,
    pub connection: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(database_path: Option<String>) -> Self {
        Database {
            in_memory: database_path.is_none(),
            database_path,
            connection: Arc::new(Mutex::new(
                Connection::open_in_memory().expect("Failed to open in-memory database"),
            )),
        }
    }

    pub async fn connect(&mut self) -> Result<(), ()> {
        if !self.in_memory {
            let path = self.database_path.clone().unwrap();
            self.connection = Arc::new(Mutex::new(
                Connection::open(path).expect("Failed to open in-memory database"),
            ));
        }

        self.create_database().await;
        self.check_migrations().await;

        Ok(())
    }

    pub fn disconnect(&self) {
        std::mem::drop(self.connection.clone());
    }

    pub async fn create_database(&self) {
        let table_count = {
            let connection = self.connection.lock().await;
            connection.prepare(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='migration_version';",
            )
            .unwrap()
            .query_map([], |row| Ok(row.get::<usize, String>(0).unwrap()))
            .unwrap()
            .count()
        };

        if table_count == 0 {
            {
                self.connection
                    .lock()
                    .await
                    .execute(
                        "CREATE TABLE migration_version ( id INTEGER PRIMARY KEY, version INTEGER NOT NULL, description TEXT );",
                        [],
                    )
                    .expect("Error creating migration_version table");
            }

            self.apply_migrations().await;
        }
    }

    pub async fn check_migrations(&self) {
        let result = self
            .connection
            .lock()
            .await
            .prepare("SELECT * FROM migration_version ORDER BY version ASC LIMIT 1;")
            .unwrap()
            .query_one([], |row| {
                Ok(models::MigrationVersion {
                    id: row.get(0)?,
                    version: row.get(1)?,
                    description: row.get(2)?,
                })
            });
        if result.is_err()
            || result.unwrap().version
                < migrations_manager::MigrationsManager::new().current_version
        {
            self.apply_migrations().await;
        }
    }

    pub async fn apply_migrations(&self) {
        let migrations_manager_ins = migrations_manager::MigrationsManager::new();
        for version in migrations_manager_ins.get_migrations() {
            let version_number = version.version();
            let result = {
                self.connection
                    .lock()
                    .await
                    .prepare("SELECT version FROM migration_version WHERE version = ?1;")
                    .expect("Failed to prepare statement")
                    .query_one([version_number], |row| row.get::<usize, u16>(0))
            };

            if result.is_err() {
                version.apply(self.connection.clone()).await;
                self.connection
                    .lock()
                    .await
                    .execute(
                        "INSERT INTO migration_version (version, description) VALUES (?1, ?2);",
                        (version_number, version.description().as_str()),
                    )
                    .expect("Failed to insert migration version");
            }
        }
    }

    pub async fn create_or_update_file(&self, file: File) -> i32 {
        let existing_file = self.get_file_by_server_id(file.server_id).await;
        if existing_file.is_none() {
            self.connection
                .lock()
                .await
                .execute(
                    "INSERT INTO file (serverId, addedDate, finishDate) VALUES (?1, ?2, ?3);",
                    (file.server_id, file.added_date, file.finish_date),
                )
                .expect("Failed to insert file into database");
            let new_file = self.get_file_by_server_id(file.server_id).await;
            new_file
                .expect("Failed to retrieve newly inserted file")
                .server_id
        } else {
            let finish_date = if existing_file.clone().unwrap().finish_date.is_some() {
                existing_file.clone().unwrap().finish_date
            } else {
                file.finish_date
            };
            self.connection
                .lock()
                .await
                .execute(
                    "UPDATE file SET addedDate = ?1, finishDate = ?2 WHERE serverId = ?3;",
                    (file.added_date, finish_date, file.server_id),
                )
                .expect("Failed to update file in database");
            existing_file.unwrap().id
        }
    }

    pub async fn get_file_by_server_id(&self, server_id: i32) -> Option<File> {
        let result = self
            .connection
            .lock()
            .await
            .prepare("SELECT * FROM file WHERE serverId = ?1;")
            .unwrap()
            .query_map([server_id], |row| {
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

    pub async fn remove_no_matching_files_ids(&self, ids: &Vec<i32>) {
        let ids_placeholders: Vec<String> =
            ids.iter().map(|v| format!("{}", v).to_string()).collect();
        let sql = format!(
            "DELETE FROM file WHERE serverId NOT IN ({});",
            ids_placeholders.join(", ")
        );

        self.connection
            .lock()
            .await
            .execute(sql.as_str(), [])
            .expect("Failed to delete non-matching files");
    }

    pub async fn list_of_file_ids(&self) -> Vec<File> {
        self.connection
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
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }
}
