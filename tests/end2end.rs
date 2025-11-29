use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use fp::Monitor;
use fp::logic::api::ResArgs::List;
use fp::logic::api::Value;
use fp::logic::api::{ReqArgs, ReqListBody, ResListBody};
use mockito::{Matcher, Mock, ServerGuard};

const TEST_TIMEOUT_SECS: i64 = 4;

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

#[test]
fn test_end_to_end() {
    let test_start_time = get_now_timestamp();

    let mut server = mockito::Server::new();
    let username = "test_user";
    let password = "test_password";
    let session_id = "cVuDHwPg4GEXflXQ5TaUateyGUDdSYrD549gdtT750G9SwXr";

    let list_res = ResListBody {
        arguments: fp::logic::api::ResArgs::List(fp::logic::api::ResListArgs {
            torrents: vec![
                vec![
                    Value::Text("id".to_string()),
                    Value::Text("addedDate".to_string()),
                    Value::Text("name".to_string()),
                    Value::Text("isFinished".to_string()),
                ],
                vec![
                    Value::Int(1),
                    Value::Int(get_now_timestamp()),
                    Value::Text("test_file.txt".to_string()),
                    Value::Bool(false),
                ],
                vec![
                    Value::Int(2),
                    Value::Int(get_now_timestamp()),
                    Value::Text("test_file2.txt".to_string()),
                    Value::Bool(true),
                ],
            ],
        }),
        result: "success".to_string(),
    };
    let list_res = Arc::new(Mutex::new(list_res));
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
            for server_id in args.ids {
                let mut res = list_res_clone.lock().unwrap();
                if let List(res_args) = &mut res.arguments {
                    res_args.torrents.retain(|torrent| {
                        if let Value::Int(id) = &torrent[0] {
                            *id != server_id as i64
                        } else {
                            true
                        }
                    });
                }
            }
        }

        "{ \"arguments\": { }, \"result\": \"success\" }".into()
    })
    .expect_at_least(1)
    .create();

    // run monitor in thread
    let url = server.url();
    let stop_signal: Arc<std::sync::atomic::AtomicBool> = Arc::new(AtomicBool::new(false));
    let stop_signal_ins = stop_signal.clone();
    let app_thread = thread::spawn(move || {
        let mut monitor = Monitor::new(
            url.as_ref(),
            None,
            Some(0),
            Some(0),
            Some(0),
            username,
            password,
        );
        monitor.run(Some(stop_signal_ins));
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

        println!("Providing list response {}", String::from_utf8(request.body().unwrap().clone()).unwrap());

        let res = list_res_clone.lock().unwrap();
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

    // stop server
    stop_signal.store(true, Ordering::SeqCst);
    app_thread.join().expect("failed to join stop thread");

    // validate files where deleted
    if let List(res) = &list_res.lock().unwrap().arguments {
        assert_eq!(res.torrents.len(), 1);
    } else {
        panic!("Invalid response arguments");
    }

    // test database state (todo)
}
