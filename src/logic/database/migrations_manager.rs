use std::rc::Rc;

use rusqlite::Connection;

pub trait Migration {
    fn apply(&self, connection: Rc<Connection>);
    fn version(&self) -> u16;
    fn description(&self) -> String;
}

// MIGRATIONS BEGIN
pub struct InitialMigration {}
impl Migration for InitialMigration {
    fn apply(&self, connection: Rc<Connection>) {
        println!("Creating file table...");
        connection
            .execute(
                "CREATE TABLE file ( id INTEGER PRIMARY KEY, serverId INTEGER UNIQUE NOT NULL, name TEXT, addedDate INTEGER NOT NULL, finishDate INTEGER );",
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
