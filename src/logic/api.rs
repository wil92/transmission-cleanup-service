use crate::logic::database::models::File;
use base64::prelude::*;
use reqwest::StatusCode;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug)]
pub struct ReqListArgs {
    fields: Vec<String>,
    format: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReqDeleteArgs {
    #[serde(rename = "delete-local-data")]
    pub delete_local_data: bool,
    pub ids: Vec<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ReqArgs {
    List(ReqListArgs),
    Delete(ReqDeleteArgs),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReqListBody {
    pub arguments: ReqArgs,
    method: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum Value {
    Int(i64),
    Text(String),
    Bool(bool),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ResListArgs {
    pub torrents: Vec<Vec<Value>>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ResDeleteArgs {}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
pub enum ResArgs {
    List(ResListArgs),
    Delete(ResDeleteArgs),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ResListBody {
    pub arguments: ResArgs,
    pub result: String,
}

pub struct Api {
    auth_token: String,
    api_url: String,
    session_id: Option<String>,
}

impl Api {
    pub fn new(username: String, password: String, api_url: String) -> Self {
        Api {
            auth_token: BASE64_STANDARD.encode(&format!("{}:{}", username, password)),
            api_url,
            session_id: None,
        }
    }

    pub fn fetch_files(&mut self) -> Result<Vec<File>, String> {
        let data = ReqListBody {
            arguments: ReqArgs::List(ReqListArgs {
                fields: vec![
                    "id".to_string(),
                    "addedDate".to_string(),
                    "isFinished".to_string(),
                ],
                format: "table".to_string(),
            }),
            method: "torrent-get".to_string(),
        };
        let res = self.post_to_api(&data).expect("Failed to send request");
        let body = res.text().expect("Failed to read response text");

        let res_body: ResListBody =
            serde_json::from_str(&body).expect("Failed to parse list of files to json");
        if res_body.result != "success" {
            return Err(format!("API returned error: {}", res_body.result));
        }

        let mut i = 1;
        let mut files: Vec<File> = Vec::new();
        let args = match &res_body.arguments {
            ResArgs::List(args) => args,
            _ => {
                return Err("Invalid response arguments".to_string());
            }
        };
        while i < args.torrents.len() {
            let mut id: Option<i32> = None;
            let mut added_date: Option<i64> = None;
            let mut is_finished: Option<bool> = None;

            let mut j = 0;
            for item in &args.torrents[0] {
                if let Value::Text(column_name) = item {
                    match column_name.as_str() {
                        "id" => {
                            if let Value::Int(id_v) = &args.torrents[i][j] {
                                id = Some(*id_v as i32);
                            } else {
                                return Err(format!("Invalid id value {:?}", &args.torrents[i][j]));
                            }
                        }
                        "addedDate" => {
                            if let Value::Int(added_date_v) = &args.torrents[i][j] {
                                added_date = Some(*added_date_v);
                            } else {
                                return Err(format!(
                                    "Invalid addedDate value {:?}",
                                    &args.torrents[i][j]
                                ));
                            }
                        }
                        "isFinished" => {
                            if let Value::Bool(is_finished_v) = &args.torrents[i][j] {
                                is_finished = Some(*is_finished_v);
                            } else {
                                return Err(format!(
                                    "Invalid isFinished value {:?}",
                                    &args.torrents[i][j]
                                ));
                            }
                        }
                        _ => {}
                    }
                } else {
                    return Err(format!("Invalid torrent item {:?}", item));
                }
                j += 1;
            }

            if id.is_some() && added_date.is_some() && is_finished.is_some() {
                let millis = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                files.push(File {
                    id: 0,
                    server_id: id.expect("Missing torrent id") as i32,
                    added_date: added_date.expect("Missing torrent added date"),
                    finish_date: if is_finished.expect("Missing isFinished value") {
                        Some(millis)
                    } else {
                        None
                    },
                })
            } else {
                return Err(format!(
                    "Missing torrent fields: id={:?}, added_date={:?}, is_finished={:?}",
                    id, added_date, is_finished
                ));
            }
            i += 1;
        }

        Ok(files)
    }

    pub fn delete_file(&mut self, ids: &Vec<i32>) -> Result<(), String> {
        let data = ReqListBody {
            arguments: ReqArgs::Delete(ReqDeleteArgs {
                delete_local_data: true,
                ids: ids.clone(),
            }),
            method: "torrent-remove".to_string(),
        };
        let res = self.post_to_api(&data).expect("Failed to send request");

        let res_body: ResListBody = res.json().expect("Failed to parse list of files to json");
        if res_body.result != "success" {
            return Err(format!("API returned error: {}", res_body.result));
        }
        Ok(())
    }

    fn post_to_api(&mut self, data: &ReqListBody) -> Result<reqwest::blocking::Response, String> {
        let client = reqwest::blocking::Client::new();

        let mut session_exchange_flag = false;

        loop {
            let url = format!("{}/transmission/rpc", self.api_url);
            let mut response = client
                .post(&url)
                .header("authorization", format!("Basic {}", self.auth_token))
                .json(data);

            if let Some(session_id) = &self.session_id {
                response = response.header("x-transmission-session-id", session_id);
            }

            let response = response
                .send()
                .map_err(|e| format!("Failed to send request to API: {}", e))?;

            if response.status() == StatusCode::CONFLICT {
                if session_exchange_flag {
                    return Err("Failed to exchange session ID with API".to_string());
                }
                session_exchange_flag = true;
                self.session_id = Some(
                    response
                        .headers()
                        .get("x-transmission-session-id")
                        .expect("Missing session ID")
                        .to_str()
                        .expect("Invalid session ID")
                        .to_string(),
                );
                continue;
            } else if response.status() != StatusCode::OK {
                return Err(format!(
                    "API returned unexpected status code: {}",
                    response.status()
                ));
            }

            return Ok(response);
        }
    }
}
