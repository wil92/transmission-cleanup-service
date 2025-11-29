use base64::prelude::*;
use fp::logic::api::Api;
use mockito::Matcher;

#[test]
fn test_api_list_files() {
    let mut server = mockito::Server::new();
    let username = "test_user";
    let password = "test_password";

    let api = Api::new(username.to_string(), password.to_string(), server.url());

    server
        .mock("POST", "/transmission/rpc")
        .match_header("authorization", Matcher::Exact(format!("Basic {}", BASE64_STANDARD.encode(format!("{}:{}", username, password)))))
        .with_status(200)
        .with_header("content-type", "application/json; charset=UTF-8")
        .with_body("{ \"arguments\": { \"torrents\": [ [\"id\",\"addedDate\",\"name\",\"isFinished\"], [1, 1763580763, \"test_file.txt\", false] ] }, \"result\": \"success\" }")
        .create();

    match api.fetch_files() {
        Ok(files) => {
            assert_eq!(files[0].id, 0);
            assert_eq!(files[0].server_id, 1);
            assert_eq!(files[0].name, "test_file.txt");
            assert_eq!(files[0].added_date, 1763580763);
            assert_eq!(files[0].finish_date, None);
        }
        Err(e) => panic!("API fetch_files failed: {}", e),
    }
}

#[test]
fn test_api_list_files_with_finish() {
    let mut server = mockito::Server::new();
    let username = "test_user";
    let password = "test_password";

    let api = Api::new(username.to_string(), password.to_string(), server.url());

    server
        .mock("POST", "/transmission/rpc")
        .with_status(200)
        .match_header("authorization", Matcher::Exact(format!("Basic {}", BASE64_STANDARD.encode(format!("{}:{}", username, password)))))
        .with_header("content-type", "application/json; charset=UTF-8")
        .with_body("{ \"arguments\": { \"torrents\": [ [\"id\",\"addedDate\",\"name\",\"isFinished\"], [1, 1763580763, \"test_file.txt\", true] ] }, \"result\": \"success\" }")
        .create();

    match api.fetch_files() {
        Ok(files) => {
            assert_eq!(files[0].id, 0);
            assert_eq!(files[0].server_id, 1);
            assert_eq!(files[0].name, "test_file.txt");
            assert_eq!(files[0].added_date, 1763580763);
            assert!(files[0].finish_date.is_some());
        }
        Err(e) => panic!("API fetch_files failed: {}", e),
    }
}

#[test]
fn test_api_delete_file() {
    let mut server = mockito::Server::new();
    let username = "test_user";
    let password = "test_password";

    let api = Api::new(username.to_string(), password.to_string(), server.url());

    server
        .mock("POST", "/transmission/rpc")
        .with_status(200)
        .match_header(
            "authorization",
            Matcher::Exact(format!(
                "Basic {}",
                BASE64_STANDARD.encode(format!("{}:{}", username, password))
            )),
        )
        .with_header("content-type", "application/json; charset=UTF-8")
        .with_body("{ \"arguments\": { }, \"result\": \"success\" }")
        .create();

    match api.delete_file(&vec![1, 2, 3]) {
        Ok(_) => {}
        Err(e) => panic!("API delete_file failed: {}", e),
    }
}
