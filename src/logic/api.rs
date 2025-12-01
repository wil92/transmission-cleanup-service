use transmission_rpc::TransClient;
use transmission_rpc::types::Id::Id;
use transmission_rpc::types::{BasicAuth, TorrentGetField};
use url::Url;

use crate::logic::database::models::File;

pub struct Api {
    auth: BasicAuth,
    api_url: String,
    client: Option<TransClient>,
}

impl Api {
    pub fn new(username: String, password: String, api_url: String) -> Self {
        Api {
            auth: BasicAuth {
                user: username,
                password,
            },
            api_url,
            client: None,
        }
    }

    pub async fn fetch_files(&mut self) -> Result<Vec<File>, String> {
        self.create_client();

        let list = self
            .client
            .as_mut()
            .unwrap()
            .torrent_get(
                Some(vec![
                    TorrentGetField::Id,
                    TorrentGetField::AddedDate,
                    TorrentGetField::IsFinished,
                    TorrentGetField::PercentDone,
                ]),
                None,
            )
            .await
            .expect("Failed to fetch torrents from Transmission API");

        let mut files: Vec<File> = vec![];
        for item in list.arguments.torrents {
            let millis = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            files.push(File {
                id: 0,
                server_id: item.id.expect("Missing torrent ID") as i32,
                added_date: item.added_date.expect("Missing addedDate").timestamp(),
                finish_date: if item.is_finished.expect("Missing isFinished value")
                    || item.percent_done.expect("Missing percentDone") >= 1.0
                {
                    Some(millis)
                } else {
                    None
                },
            });
        }

        Ok(files)
    }

    pub async fn delete_file(&mut self, ids: &Vec<i32>) -> Result<(), String> {
        self.create_client();

        println!("Deleting files with IDs: {:?}", ids);
        println!(
            "Deleting files with IDs: {:?}",
            ids.iter().map(|&id| Id(id as i64)).collect::<Vec<_>>()
        );

        let res = self
            .client
            .as_mut()
            .unwrap()
            .torrent_remove(ids.iter().map(|&id| Id(id as i64)).collect(), true)
            .await
            .expect("Failed to delete file from Transmission API");

        println!("Delete response: {:?}", res);

        if res.result != "success" {
            return Err(format!("Failed to delete files: {}", res.result));
        }
        Ok(())
    }

    fn create_client(&mut self) {
        if self.client.is_none() {
            self.client = Some(TransClient::with_auth(
                Url::parse(&self.api_url).expect("Invalid API URL"),
                self.auth.clone(),
            ));
        }
    }
}
