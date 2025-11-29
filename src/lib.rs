use std::collections::HashSet;
use std::sync::Arc;

use crate::logic::api::Api;
use crate::logic::database::Database;

pub mod logic;

pub struct Monitor {
    scan_interval: u32,
    files_lifetime_after_copied: u32,
    files_lifetime: u32,

    database: Database,
    api: Api,
}

impl Monitor {
    pub fn new(
        monitoring_url: &str,
        database_path: Option<String>,
        scan_interval: Option<u32>,
        files_lifetime: Option<u32>,
        files_lifetime_after_copied: Option<u32>,
        username: &str,
        password: &str,
    ) -> Self {
        Monitor {
            // Default to 60 seconds
            scan_interval: scan_interval.unwrap_or(60),
            // Default to 7 days
            files_lifetime: files_lifetime.unwrap_or(604800),
            // Default to 5 hours
            files_lifetime_after_copied: files_lifetime_after_copied.unwrap_or(18000),

            api: Api::new(
                username.to_string(),
                password.to_string(),
                monitoring_url.to_string(),
            ),

            database: Database::new(database_path),
        }
    }

    pub fn run(&mut self, stop_signal: Option<Arc<std::sync::atomic::AtomicBool>>) {
        let mut scan_interval_it = 0;

        self.database
            .connect()
            .expect("Failed to connect to database");

        loop {
            if let Some(stop_signal) = &stop_signal {
                if stop_signal.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }
            }

            if scan_interval_it >= self.scan_interval {
                self.scan_files_and_cleanup();
                scan_interval_it = 0;
            }

            std::thread::sleep(std::time::Duration::from_secs(1));
            scan_interval_it += 1;
        }
    }

    fn scan_files_and_cleanup(&self) {
        // Fetch files from API and update database
        let files = self.api.fetch_files().expect("Failed to fetch files");
        let mut updated_files: Vec<i32> = vec![];
        for file in files {
            updated_files.push(self.database.create_or_update_file(file));
        }

        // Remove files that are no longer present
        self.database.remove_no_matching_files_ids(&updated_files);

        // Cleanup old files based on lifetime
        let files = self.database.list_of_file_ids();
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Remove files older copied files
        let mut already_removed_files: HashSet<i32> = HashSet::new();
        let mut files_to_remove: Vec<i32> = vec![];
        for file in files.clone() {
            if let Some(finish_date) = file.finish_date {
                if current_time - finish_date > self.files_lifetime_after_copied as i64 {
                    files_to_remove.push(file.server_id);
                    already_removed_files.insert(file.id);
                }
            }
        }

        // Remove files older than lifetime
        for file in files {
            if current_time - file.added_date > self.files_lifetime as i64
                && !already_removed_files.contains(&file.id)
            {
                files_to_remove.push(file.server_id);
            }
        }

        if !files_to_remove.is_empty() {
            self.api
                .delete_file(&files_to_remove)
                .expect("Failed to delete file via API");
        }
    }
}
