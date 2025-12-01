use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

#[async_trait::async_trait]
pub trait Migration: Send + Sync {
    async fn apply(&self, connection: Arc<Mutex<Connection>>);
    fn version(&self) -> u16;
    fn description(&self) -> String;
}

// MIGRATIONS BEGIN
pub struct InitialMigration {}

#[async_trait::async_trait]
impl Migration for InitialMigration {
    async fn apply(&self, connection: Arc<Mutex<Connection>>) {
        println!("Creating file table...");
        connection
            .lock()
            .await
            .execute(
                "CREATE TABLE file ( id INTEGER PRIMARY KEY, serverId INTEGER UNIQUE NOT NULL, addedDate INTEGER NOT NULL, finishDate INTEGER );",
                [],
            )
            .expect("Error creating file table");
    }

    fn version(&self) -> u16 {
        1
    }

    fn description(&self) -> String {
        "Initial migration".to_string()
    }
}
// MIGRATIONS END

pub struct MigrationsManager {
    pub current_version: u16,
}

impl MigrationsManager {
    pub fn new() -> Self {
        MigrationsManager { current_version: 1 }
    }

    pub fn get_migrations(&self) -> Vec<Box<dyn Migration>> {
        vec![Box::new(InitialMigration {})]
    }
}
