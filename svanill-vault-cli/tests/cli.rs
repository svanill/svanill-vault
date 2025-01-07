use assert_cmd::Command;
use mockito::Matcher;
use serde_json::json;

#[test]
fn it_output_version() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    let assert = cmd.args(["-V"]).assert();

    assert.success().stdout("svanill-vault-cli 0.1.2\n");
}

#[test]
fn it_exit_with_error_if_the_user_does_not_exist() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    let mut server = mockito::Server::new();
    let base_url = &server.url();

    let m = server
        .mock("GET", "/auth/request-challenge?username=test_user")
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_body(
            r#"
        {
            "error": {"code":1005,"message":"The user does not exist"},
            "links":{
                "create_user":{
                    "href":"https://api.svanill.com/users/",
                    "rel":"user"
                }
            },
            "status":401
        }
            "#,
        )
        .create();

    let assert = cmd.args(["-u", "test_user", "-h", base_url, "ls"]).assert();

    m.assert();
    assert
        .failure()
        .code(1)
        .stdout("")
        .stderr("Error: Status: 401, Code: 1005, Message: \"The user does not exist\"\n");
}

#[test]
fn it_list_remote_files() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let username = "test_user";

    let mut server = mockito::Server::new();
    let base_url = &server.url();

    let (m1, m2) = mock_successful_authentication_requests(&mut server);

    let m3 = mock_list_files_happy_path(&mut server, username);

    let assert = cmd
        .args([
            "-h",
            base_url,
            "-u",
            username,
            "--answer",
            "test answer",
            "ls",
        ])
        .assert();

    m1.assert();
    m2.assert();
    m3.assert();
    assert.success().stdout(
        r#"       Bytes | Filename
         123 | this-is-a-test-file
"#,
    );
}

#[test]
fn it_delete_files() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    let mut server = mockito::Server::new();
    let base_url = &server.url();

    let (m1, m2) = mock_successful_authentication_requests(&mut server);

    let m3 = server
        .mock("DELETE", "/files/?filename=some-file-to-delete")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"status":200}"#)
        .create();

    let assert = cmd
        .args([
            "-h",
            base_url,
            "-u",
            "test_user",
            "--answer",
            "test answer",
            "rm",
            "some-file-to-delete",
        ])
        .assert();

    m1.assert();
    m2.assert();
    m3.assert();
    assert
        .success()
        .stdout("Success: deleted file \"some-file-to-delete\"\n");
}

#[test]
fn it_pull_remote_file_output_to_stdout() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let username = "test_user";

    let mut server = mockito::Server::new();
    let base_url = &server.url();

    let (m1, m2) = mock_successful_authentication_requests(&mut server);
    let m3 = mock_list_files_happy_path(&mut server, username);

    let m4 = server
        .mock("GET", "/imaginary/url/this-is-a-test-file")
        .with_status(200)
        .with_header("content-type", "text/plain")
        .with_body("imaginary content")
        .create();

    let assert = cmd
        .args([
            "-h",
            base_url,
            "-u",
            username,
            "--answer",
            "test answer",
            "pull",
            "this-is-a-test-file",
            "-s",
        ])
        .assert();

    m1.assert();
    m2.assert();
    m3.assert();
    m4.assert();
    assert.success().stdout("imaginary content");
}

#[test]
fn it_push_content_from_stdin_to_remote_file() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    let mut server = mockito::Server::new();
    let base_url = &server.url();

    let (m1, m2) = mock_successful_authentication_requests(&mut server);

    let m3 = server.mock("POST", "/files/request-upload-url")
        .match_body(r#"{"filename":"some-remote-filename"}"#)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json!({
            "links":{
                "retrieve_url":{
                    "href":format!("{base_url}/some/imaginary/retrieve/url"),
                    "rel":"file"
                },
                "upload_url":{
                    "form_data":{
                        "bucket":"some-bucket",
                        "key":"users/test_user/some-remote-filename",
                        "policy":"xxx",
                        "x-amz-algorithm":"AWS4-HMAC-SHA256",
                        "x-amz-credential":"AMIAIT3W6RZTLLGAZEXQ/10230679/us-west-2/s3/aws4_request",
                        "x-amz-date":"20200629T014946Z",
                        "x-amz-signature":"50cf7a0fa5ec900de9a6d7b05f7ecdd8e3c082bb144b7604d71118852e386c2d"
                    },
                    "href":format!("{base_url}/some/imaginary/upload/url"),
                    "rel":"file"
                }
            },
            "status":200
        })
        .to_string())
        .create();

    let m4 = server
        .mock("POST", "/some/imaginary/upload/url")
        .match_header(
            "Content-Type",
            Matcher::Regex("^multipart/form-data(;.*)?".to_string()),
        )
        .match_body(Matcher::Regex("Content-Disposition".to_string()))
        .with_status(204)
        .with_header("ETag", "2e01e17dd92be3c7fab5e08505ed64c9")
        .create();

    let assert = cmd
        .args([
            "-h",
            base_url,
            "-u",
            "test_user",
            "--answer",
            "test answer",
            "push",
            "-r",
            "some-remote-filename",
            "-",
        ])
        .write_stdin("blob of data")
        .assert();

    m1.assert();
    m2.assert();
    m3.assert();
    m4.assert();
    assert
        .success()
        .stdout("Successfully pushed file, using as remote name \"some-remote-filename\"\n");
}

fn mock_successful_authentication_requests(
    server: &mut mockito::ServerGuard,
) -> (mockito::Mock, mockito::Mock) {
    let base_url = server.url();

    let m1 = server
        .mock("GET", "/auth/request-challenge?username=test_user")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "content": {"challenge":"somechallenge"},
                "links":
                    {
                        "answer_auth_challenge":{
                            "href": format!("{base_url}/auth/answer-challenge"),
                            "rel":"auth"
                        },
                        "create_user":{
                            "href": format!("{base_url}/users/"),
                            "rel":"user"
                        }
                    },
                "status":200
            })
            .to_string(),
        )
        .create();

    let m2 = server
        .mock("POST", "/auth/answer-challenge")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                    "content":{"token":"a-secure-token"},
                    "links":{
                        "files_list":{
                            "href":format!("{base_url}/files/"),
                            "rel":"file"
                        },
                        "request_upload_url":{
                            "href":format!("{base_url}/files/request-upload-url/"),
                            "rel":"file"
                        }
                    },
                    "status":200
                }
            )
            .to_string(),
        )
        .create();

    (m1, m2)
}

fn mock_list_files_happy_path(server: &mut mockito::ServerGuard, username: &str) -> mockito::Mock {
    let base_url = server.url();

    server
        .mock("GET", "/files/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "content":[
                    {
                        "content":{
                            "checksum":"a9a1bdddeacc612db8b5c01a830af1c3",
                            "filename": format!("users/{username}/this-is-a-test-file"),
                            "size":123,
                            "url":format!("{base_url}/imaginary/url/this-is-a-test-file"),
                        },
                        "links":{
                            "delete":{
                                "href":format!("{base_url}/files/"),
                                "rel":"file"
                            },
                            "read":{
                                "href":format!("{base_url}/imaginary/url/this-is-a-test-file"),
                                "rel":"file"
                            }
                        }
                    }
                ],
                "status":200
            }
            )
            .to_string(),
        )
        .create()
}
