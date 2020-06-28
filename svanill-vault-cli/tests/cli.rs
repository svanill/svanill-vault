use assert_cmd::Command;
use mockito::mock;
use serde_json::json;

#[test]
fn it_output_version() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    let assert = cmd.args(&["-V"]).assert();

    assert.success().stdout("svanill-vault-cli 0.1.0\n");
}

#[test]
fn it_exit_with_error_if_the_user_does_not_exist() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    let m = mock("GET", "/auth/request-challenge?username=test_user")
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

    let base_url = &mockito::server_url();
    let assert = cmd
        .args(&["-u", "test_user", "-h", base_url, "ls"])
        .assert();

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

    let base_url = &mockito::server_url();

    let m1 = mock("GET", "/auth/request-challenge?username=test_user")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "content": {"challenge":"somechallenge"},
                "links":
                    {
                        "answer_auth_challenge":{
                            "href": format!("{}/auth/answer-challenge", base_url),
                            "rel":"auth"
                        },
                        "create_user":{
                            "href": format!("{}/users/", base_url),
                            "rel":"user"
                        }
                    },
                "status":200
            })
            .to_string(),
        )
        .create();

    let m2 = mock("POST", "/auth/answer-challenge")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                    "content":{"token":"a-secure-token"},
                    "links":{
                        "files_list":{
                            "href":format!("{}/files/", base_url),
                            "rel":"file"
                        },
                        "request_upload_url":{
                            "href":format!("{}/files/request-upload-url/", base_url),
                            "rel":"file"
                        }
                    },
                    "status":200
                }
            )
            .to_string(),
        )
        .create();

    let m3 = mock("GET", "/files/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "content":[
                    {
                        "content":{
                            "checksum":"a9a1bdddeacc612db8b5c01a830af1c3",
                            "filename":"this-is-a-test-file",
                            "size":123,
                            "url":"some_url_1"
                        },
                        "links":{
                            "delete":{
                                "href":format!("{}/files/", base_url),
                                "rel":"file"
                            },
                            "read":{
                                "href":"some_url_1",
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
        .create();

    let assert = cmd
        .args(&[
            "-h",
            base_url,
            "-u",
            "test_user",
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
