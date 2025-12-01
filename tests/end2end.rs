use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use mockito::{Matcher, Mock, ServerGuard};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use fp::Monitor;

const TEST_TIMEOUT_SECS: i64 = 4;

#[derive(Serialize, Deserialize, Debug)]
struct ReqListArgs {
    fields: Vec<String>,
    format: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ReqDeleteArgs {
    #[serde(rename = "delete-local-data")]
    pub delete_local_data: bool,
    pub ids: Vec<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum ReqArgs {
    List(ReqListArgs),
    Delete(ReqDeleteArgs),
}

#[derive(Serialize, Deserialize, Debug)]
struct ReqListBody {
    pub arguments: ReqArgs,
    method: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct ResItem {
    id: i64,
    #[serde(rename = "addedDate")]
    added_date: i64,
    #[serde(rename = "isFinished")]
    is_finished: bool,
    #[serde(rename = "percentDone")]
    percent_done: f64
}

#[derive(Deserialize, Serialize, Debug)]
struct ResListArgs {
    pub torrents: Vec<ResItem>,
}

#[derive(Deserialize, Serialize, Debug)]
struct ResDeleteArgs {}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
enum ResArgs {
    List(ResListArgs),
    Delete(ResDeleteArgs),
}

#[derive(Deserialize, Serialize, Debug)]
struct ResListBody {
    pub arguments: ResArgs,
    pub result: String,
}

fn get_now_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_secs() as i64
}

fn setup_base_mock(
    server: &mut ServerGuard,
    username: &str,
    password: &str,
    session_id: &str,
    method: String,
    status: usize,
) -> Mock {
    server
        .mock("POST", "/transmission/rpc")
        .match_header(
            "authorization",
            Matcher::Exact(format!(
                "Basic {}",
                BASE64_STANDARD.encode(format!("{}:{}", username, password))
            )),
        )
        .match_body(Matcher::Regex(method))
        .with_status(status)
        .with_header("x-transmission-session-id", session_id)
        .with_header("content-type", "text/html; charset=UTF-8")
}

#[tokio::test]
async fn test_end_to_end() {
    let test_start_time = get_now_timestamp();

    let mut server = mockito::Server::new_async().await;
    let username = "test_user";
    let password = "test_password";
    let session_id = "cVuDHwPg4GEXflXQ5TaUateyGUDdSYrD549gdtT750G9SwXr";

    let list_res = ResListBody {
        arguments: ResArgs::List(ResListArgs {
            torrents: vec![
                ResItem {
                    id: 1,
                    added_date: get_now_timestamp(),
                    is_finished: false,
                    percent_done: 0.5,
                },
                ResItem {
                    id: 2,
                    added_date: get_now_timestamp(),
                    is_finished: true,
                    percent_done: 1.0,
                },
            ],
        }),
        result: "success".to_string(),
    };
    let list_res = Arc::new(std::sync::Mutex::new(list_res));
    let session_exchanged = Arc::new(AtomicBool::new(false));

    // handle fetch_files
    let session_exchanged_clone = session_exchanged.clone();
    let list_mock = setup_base_mock(
        &mut server,
        username,
        password,
        session_id,
        "\"torrent-get\"".to_string(),
        409,
    )
    .with_body_from_request(move |request| {
        println!("Exchanging session id");
        assert!(!request.has_header("x-transmission-session-id"));
        session_exchanged_clone.store(true, Ordering::SeqCst);
        "start session id exchange".into()
    })
    .expect_at_least(1)
    .create();
    // handle delete_file
    let list_res_clone = list_res.clone();
    let session_id_clone = session_id.to_string();
    let delete_mock = setup_base_mock(
        &mut server,
        username,
        password,
        session_id,
        "\"torrent-remove\"".to_string(),
        200,
    )
    .with_body_from_request(move |request| {
        assert!(request.has_header("x-transmission-session-id"));
        assert_eq!(request.header("x-transmission-session-id").len(), 1);
        assert_eq!(
            request.header("x-transmission-session-id")[0]
                .to_str()
                .unwrap(),
            session_id_clone.as_str()
        );

        let body = String::from_utf8(request.body().expect("Failed to read body").clone())
            .expect("Failed to parse response body");
        println!("Body: {}", body.clone());
        let req_body: ReqListBody =
            serde_json::from_str(&body).expect("Failed to parse delete request body to json");

        if let ReqArgs::Delete(args) = req_body.arguments {
            assert!(args.delete_local_data);
            for remove_id in args.ids {
                let mut res = list_res_clone.lock().unwrap();
                if let ResArgs::List(res_args) = &mut res.arguments {
                    res_args.torrents.retain(|item| item.id != remove_id);
                }
            }
        }

        "{ \"arguments\": { }, \"result\": \"success\" }".into()
    })
    .expect_at_least(1)
    .create();

    // run monitor in thread
    let stop_signal: Arc<Mutex<AtomicBool>> = Arc::new(Mutex::new(AtomicBool::new(false)));
    let stop_signal_clone = stop_signal.clone();
    let mut monitor = Monitor::new(
        format!("{}/transmission/rpc", server.url()).as_str(),
        None,
        Some(0),
        Some(0),
        Some(0),
        username,
        password,
    );
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app_thread = rt.spawn(async move {
        monitor.run(Some(stop_signal_clone)).await;
    });

    // validate session exchange
    while !session_exchanged.load(Ordering::SeqCst) && !list_mock.matched() {
        if get_now_timestamp() - test_start_time > TEST_TIMEOUT_SECS {
            panic!("Timeout waiting for session exchange");
        }
    }
    list_mock.remove();

    let list_res_clone = list_res.clone();
    let session_id_clone = session_id.to_string();
    let list_mock = setup_base_mock(
        &mut server,
        username,
        password,
        session_id,
        "\"torrent-get\"".to_string(),
        200,
    )
    .with_body_from_request(move |request| {
        assert!(request.has_header("x-transmission-session-id"));
        assert_eq!(request.header("x-transmission-session-id").len(), 1);
        assert_eq!(
            request.header("x-transmission-session-id")[0]
                .to_str()
                .unwrap(),
            session_id_clone.as_str()
        );


        let res = list_res_clone.lock().unwrap();
        println!("{}", serde_json::to_string(&*res).unwrap().as_str());
        serde_json::to_string(&*res).unwrap().into()
    })
    .expect_at_least(2)
    .create();

    // wait until fetch_files is called
    while !list_mock.matched() {
        if get_now_timestamp() - test_start_time > TEST_TIMEOUT_SECS {
            panic!("Timeout waiting for list files calls");
        }
    }

    // wait until delete_file is called
    while !delete_mock.matched() {
        if get_now_timestamp() - test_start_time > TEST_TIMEOUT_SECS {
            panic!("Timeout waiting for delete file calls");
        }
    }

    // test database state (todo)

    // stop server
    stop_signal.lock().await.store(true, Ordering::SeqCst);
    _ = tokio::join!(app_thread);
    rt.shutdown_background();

    // validate files where deleted
    if let ResArgs::List(res) = &list_res.lock().unwrap().arguments {
        assert_eq!(res.torrents.len(), 0);
    } else {
        panic!("Invalid response arguments");
    }
}
