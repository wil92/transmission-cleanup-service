use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use fp::Monitor;
use fp::logic::api::ResArgs::List;
use fp::logic::api::Value;
use fp::logic::api::{ReqArgs, ReqListBody, ResListBody};
use mockito::Matcher;

fn get_now_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_secs() as i64
}

#[test]
fn test_end_to_end() {
    let mut server = mockito::Server::new();
    let username = "test_user";
    let password = "test_password";

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

    // handle fetch_files
    let list_res_clone = list_res.clone();
    let list_mock = server
        .mock("POST", "/transmission/rpc")
        .match_header(
            "authorization",
            Matcher::Exact(format!(
                "Basic {}",
                BASE64_STANDARD.encode(format!("{}:{}", username, password))
            )),
        )
        .match_body(Matcher::Regex("\"torrent-get\"".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json; charset=UTF-8")
        .with_body_from_request(move |_request| {
            let res = list_res_clone.lock().unwrap();
            serde_json::to_string(&*res).unwrap().into()
        })
        .expect_at_least(2)
        .create();
    // handle delete_file
    let list_res_clone = list_res.clone();
    let delete_mock = server
        .mock("POST", "/transmission/rpc")
        .match_header(
            "authorization",
            Matcher::Exact(format!(
                "Basic {}",
                BASE64_STANDARD.encode(format!("{}:{}", username, password))
            )),
        )
        .match_body(Matcher::Regex("\"torrent-remove\"".to_string()))
        .with_status(200)
        .with_header("content-type", "application/json; charset=UTF-8")
        .with_body_from_request(move |request| {
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

    // wait until fetch_files is called
    while !list_mock.matched() {}

    // wait until delete_file is called
    while !delete_mock.matched() {}

    // stop server
    stop_signal.store(true, Ordering::SeqCst);
    app_thread.join().expect("failed to join stop thread");

    if let List(res) = &list_res.lock().unwrap().arguments {
        assert_eq!(res.torrents.len(), 1);
    } else {
        panic!("Invalid response arguments");
    }
}
