use crate::file_server::FileServer;
use actix_http::http::StatusCode;
use ctor::ctor;
use diesel::{
    r2d2::{self, ConnectionManager},
    RunQueryDsl, SqliteConnection,
};
use r2d2::Pool;
use ring::hmac;
use ring::test::rand::FixedByteRandom;
use rusoto_credential::AwsCredentials;
use rusoto_mock::{MockCredentialsProvider, MockRequestDispatcher};
use rusoto_signature::region::Region;
use std::net::TcpListener;
use svanill_vault_openapi::{
    AnswerUserChallengeRequest, AnswerUserChallengeResponse, AskForTheChallengeResponse,
    GetStartingEndpointsResponse, RemoveFileResponse, RequestUploadUrlRequestBody,
    RequestUploadUrlResponse, RetrieveListOfUserFilesResponse,
};
use svanill_vault_server::auth::auth_token::AuthToken;
use svanill_vault_server::auth::tokens_cache::TokensCache;
use svanill_vault_server::errors::ApiError;
use svanill_vault_server::{file_server, server::AppData};

#[macro_use]
extern crate diesel_migrations;
embed_migrations!();

#[cfg(test)]
#[ctor]
fn init_color_backtrace() {
    color_backtrace::install();
}

fn setup_tokens_cache(token: &str, username: &str) -> TokensCache {
    let mut tokens_cache = TokensCache::default();
    tokens_cache.insert(AuthToken(token.to_string()), username.to_string());
    tokens_cache
}

fn setup_fake_random_key() -> hmac::Key {
    let rng = FixedByteRandom { byte: 0 };
    hmac::Key::generate(hmac::HMAC_SHA256, &rng).expect("Cannot generate cryptographyc key")
}

pub trait AppDataBuilder {
    fn new() -> Self;
    fn tokens_cache(self, tokens_cache: TokensCache) -> Self;
    fn crypto_key(self, crypto_key: hmac::Key) -> Self;
    fn pool(self, pool: Pool<ConnectionManager<SqliteConnection>>) -> Self;
    fn s3_fs(self, s3_fs: FileServer) -> Self;
}

impl AppDataBuilder for AppData {
    fn new() -> AppData {
        let tokens_cache = TokensCache::default();
        let crypto_key = setup_fake_random_key();
        let pool = setup_test_db();
        let s3_fs = setup_s3_fs(MockRequestDispatcher::default());

        AppData {
            tokens_cache,
            crypto_key,
            pool,
            s3_fs,
        }
    }

    fn tokens_cache(mut self, tokens_cache: TokensCache) -> Self {
        self.tokens_cache = tokens_cache;
        self
    }

    fn crypto_key(mut self, crypto_key: hmac::Key) -> Self {
        self.crypto_key = crypto_key;
        self
    }

    fn pool(mut self, pool: Pool<ConnectionManager<SqliteConnection>>) -> Self {
        self.pool = pool;
        self
    }
    fn s3_fs(mut self, s3_fs: FileServer) -> Self {
        self.s3_fs = s3_fs;
        self
    }
}

fn spawn_app(data: AppData) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");

    // Retrieve the port assigned to us by the OS
    let port = listener.local_addr().unwrap().port();

    let server = svanill_vault_server::server::run(listener, data).expect("Failed to bind address");
    let _ = tokio::spawn(server);

    // We return the application address to the caller!
    format!("http://127.0.0.1:{}", port)
}

#[actix_rt::test]
async fn noauth_noroute_must_return_401() {
    let address = spawn_app(AppData::new());
    let client = reqwest::Client::new();

    let resp = client
        .get(&format!("{}/not-exist", &address))
        .header("Authorization", "Bearer dummy-invalid-token")
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: ApiError = resp
        .json::<ApiError>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(401, json_resp.http_status);
    assert_eq!(401, json_resp.error.code);
}

#[actix_rt::test]
async fn auth_noroute_noget_must_return_405() {
    let tokens_cache = setup_tokens_cache("dummy-valid-token", "test_user");

    let address = spawn_app(AppData::new().tokens_cache(tokens_cache));

    let client = reqwest::Client::new();
    let resp = client
        // use a "not GET" request on an unexistent route
        .patch(&format!("{}/not-exist", &address))
        .header("Authorization", "Bearer dummy-valid-token")
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: ApiError = resp
        .json::<ApiError>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(405, json_resp.http_status);
    assert_eq!(405, json_resp.error.code);
}

#[actix_rt::test]
async fn auth_noroute_get_must_return_404() {
    let tokens_cache = setup_tokens_cache("dummy-valid-token", "test_user");

    let address = spawn_app(AppData::new().tokens_cache(tokens_cache));

    let client = reqwest::Client::new();
    let resp = client
        .get(&format!("{}/not-exist", &address))
        .header("Authorization", "Bearer dummy-valid-token")
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: ApiError = resp
        .json::<ApiError>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(404, json_resp.http_status);
    assert_eq!(404, json_resp.error.code);
}

#[actix_rt::test]
async fn root() {
    let address = spawn_app(AppData::new());

    let client = reqwest::Client::new();
    let resp = client
        .get(&format!("{}/", &address))
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: GetStartingEndpointsResponse = resp
        .json::<GetStartingEndpointsResponse>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(200, json_resp.status);
}

fn setup_test_db() -> Pool<ConnectionManager<SqliteConnection>> {
    let connspec = ":memory:";
    let manager = ConnectionManager::<SqliteConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create database connection pool");

    let connection = pool.get().expect("couldn't get db connection from pool");

    embedded_migrations::run(&connection).expect("failed to run migrations");
    pool
}

fn setup_test_db_with_user() -> Pool<ConnectionManager<SqliteConnection>> {
    let pool = setup_test_db();

    let connection = pool.get().expect("couldn't get db connection from pool");

    let query = diesel::sql_query(
        r#"
        INSERT INTO user VALUES
        ('test_user_1', 'challenge1', 'answer1'),
        ('test_user_2', 'challenge2', 'answer2')
    "#,
    );

    query
        .execute(&connection)
        .expect("failed to insert db test values");

    pool
}

#[actix_rt::test]
async fn get_auth_challenge_no_username_provided() {
    let address = spawn_app(AppData::new());

    let client = reqwest::Client::new();
    let resp = client
        .get(&format!("{}/auth/request-challenge", &address))
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: ApiError = resp
        .json::<ApiError>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(409, json_resp.http_status);
    assert_eq!(1002, json_resp.error.code);
}

#[actix_rt::test]
async fn get_auth_challenge_username_not_found() {
    let address = spawn_app(AppData::new());

    let client = reqwest::Client::new();
    let resp = client
        .get(&format!(
            "{}/auth/request-challenge?username=notfound",
            &address
        ))
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: ApiError = resp
        .json::<ApiError>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(401, json_resp.http_status);
    assert_eq!(1005, json_resp.error.code);
}

#[actix_rt::test]
async fn get_auth_challenge_ok() {
    let pool = setup_test_db_with_user();
    let address = spawn_app(AppData::new().pool(pool));

    let client = reqwest::Client::new();
    let resp = client
        .get(&format!(
            "{}/auth/request-challenge?username=test_user_2",
            &address
        ))
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: AskForTheChallengeResponse = resp
        .json::<AskForTheChallengeResponse>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(200, json_resp.status);
    assert_eq!("challenge2", json_resp.content.challenge);
}

#[actix_rt::test]
async fn answer_auth_challenge_username_not_found() {
    let pool = setup_test_db_with_user();
    let address = spawn_app(AppData::new().pool(pool));

    let payload = AnswerUserChallengeRequest {
        username: "notfound".to_owned(),
        answer: "any_answer".to_owned(),
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/auth/answer-challenge", &address))
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: ApiError = resp
        .json::<ApiError>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(401, json_resp.http_status);
    assert_eq!(1005, json_resp.error.code);
}

#[actix_rt::test]
async fn answer_auth_challenge_wrong_answer() {
    let pool = setup_test_db_with_user();
    let address = spawn_app(AppData::new().pool(pool));

    let payload = AnswerUserChallengeRequest {
        username: "test_user_2".to_owned(),
        answer: "wrong_answer".to_owned(),
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/auth/answer-challenge", &address))
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: ApiError = resp
        .json::<ApiError>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(401, json_resp.http_status);
    assert_eq!(1006, json_resp.error.code);
}

#[actix_rt::test]
async fn answer_auth_challenge_ok() {
    let pool = setup_test_db_with_user();
    let address = spawn_app(AppData::new().pool(pool));

    let payload = AnswerUserChallengeRequest {
        username: "test_user_2".to_owned(),
        answer: "answer2".to_owned(),
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/auth/answer-challenge", &address))
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: AnswerUserChallengeResponse = resp
        .json::<AnswerUserChallengeResponse>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(200, json_resp.status);
    assert!(!json_resp.content.token.is_empty());

    // Do the same request again and verify that every token is unique
    let resp2 = client
        .post(&format!("{}/auth/answer-challenge", &address))
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp2: AnswerUserChallengeResponse = resp2
        .json::<AnswerUserChallengeResponse>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(200, json_resp2.status);
    assert_ne!(json_resp.content.token, json_resp2.content.token);
}

fn setup_s3_fs(s3_resp_mock: MockRequestDispatcher) -> FileServer {
    let region = Region::EuCentral1;
    let bucket = "test_bucket".to_string();

    let provider = MockCredentialsProvider;
    let credentials = AwsCredentials::new("mock_key", "mock_secret", None, None);

    let client = rusoto_s3::S3Client::new_with(s3_resp_mock, provider, Default::default());

    FileServer {
        region,
        bucket,
        client,
        credentials,
        presigned_url_timeout: std::time::Duration::from_secs(10),
    }
}

#[actix_rt::test]
async fn request_upload_url_ok() {
    let pool = setup_test_db_with_user();
    let tokens_cache = setup_tokens_cache("dummy-valid-token", "test_user_2");

    let address = spawn_app(AppData::new().pool(pool).tokens_cache(tokens_cache));

    let payload = RequestUploadUrlRequestBody {
        filename: "test_filename".to_owned(),
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/files/request-upload-url", &address))
        .header("Authorization", "Bearer dummy-valid-token")
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: RequestUploadUrlResponse = resp
        .json::<RequestUploadUrlResponse>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(200, json_resp.status);
    assert!(!json_resp.links.upload_url.href.is_empty());
    assert!(!json_resp.links.retrieve_url.href.is_empty());
}

#[actix_rt::test]
async fn request_upload_url_empty_filename() {
    let pool = setup_test_db_with_user();
    let tokens_cache = setup_tokens_cache("dummy-valid-token", "test_user_2");

    let address = spawn_app(AppData::new().pool(pool).tokens_cache(tokens_cache));

    let payload = RequestUploadUrlRequestBody {
        filename: "".to_owned(),
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/files/request-upload-url", &address))
        .header("Authorization", "Bearer dummy-valid-token")
        .json(&payload)
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: ApiError = resp
        .json::<ApiError>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(409, json_resp.http_status);
    assert_eq!(1002, json_resp.error.code);
}

#[actix_rt::test]
async fn request_upload_url_wrong_payload() {
    let pool = setup_test_db_with_user();
    let tokens_cache = setup_tokens_cache("dummy-valid-token", "test_user_2");

    let address = spawn_app(AppData::new().pool(pool).tokens_cache(tokens_cache));

    let payload = "not a proper payload";

    let client = reqwest::Client::new();
    let resp = client
        .post(&format!("{}/files/request-upload-url", &address))
        .header("Authorization", "Bearer dummy-valid-token")
        .body(payload)
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: ApiError = resp
        .json::<ApiError>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(400, json_resp.http_status);
    assert_eq!(1024, json_resp.error.code);
    assert_eq!(json_resp.error.message, "Content type error");
}

#[actix_rt::test]
async fn list_user_files_ok() {
    let pool = setup_test_db_with_user();
    let tokens_cache = setup_tokens_cache("dummy-valid-token", "test_user_2");

    let s3_resp_mock = MockRequestDispatcher::default().with_body(
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
          <Name>quotes</Name>
          <KeyCount>1</KeyCount>
          <MaxKeys>3</MaxKeys>
          <IsTruncated>false</IsTruncated>
          <Contents>
            <Key>users/test_user_2/some_object_1.txt</Key>
            <LastModified>2013-09-17T18:07:53.000Z</LastModified>
            <ETag>"599bab3ed2c697f1d26842727561fd94"</ETag>
            <Size>857</Size>
            <StorageClass>REDUCED_REDUNDANCY</StorageClass>
          </Contents>
          <Contents>
            <Key>users/test_user_2/any/path/is/ok.txt</Key>
            <LastModified>2013-09-17T18:07:53.000Z</LastModified>
            <ETag>"d26842727561fd94599bab3ed2c697f1"</ETag>
            <Size>346</Size>
            <StorageClass>REDUCED_REDUNDANCY</StorageClass>
          </Contents>
        </ListBucketResult>"#,
    );
    let s3_fs = setup_s3_fs(s3_resp_mock);

    let address = spawn_app(
        AppData::new()
            .pool(pool)
            .tokens_cache(tokens_cache)
            .s3_fs(s3_fs),
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&format!("{}/files/", &address))
        .header("Authorization", "Bearer dummy-valid-token")
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: RetrieveListOfUserFilesResponse = resp
        .json::<RetrieveListOfUserFilesResponse>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(200, json_resp.status);
    assert_eq!(2, json_resp.content.len());
    assert_eq!("some_object_1.txt", json_resp.content[0].content.filename);
    assert_eq!(
        "\"599bab3ed2c697f1d26842727561fd94\"",
        json_resp.content[0].content.checksum
    );
    assert_eq!(857, json_resp.content[0].content.size);
    assert!(json_resp.content[0].content.url.starts_with(
        "https://s3.eu-central-1.amazonaws.com/test_bucket/users/test_user_2/some_object_1.txt"
    ));

    assert_eq!("any/path/is/ok.txt", json_resp.content[1].content.filename);
    assert_eq!(
        "\"d26842727561fd94599bab3ed2c697f1\"",
        json_resp.content[1].content.checksum
    );
    assert_eq!(346, json_resp.content[1].content.size);
    assert!(json_resp.content[1].content.url.starts_with(
        "https://s3.eu-central-1.amazonaws.com/test_bucket/users/test_user_2/any/path/is/ok.txt"
    ));
}

#[actix_rt::test]
async fn list_user_files_s3_error() {
    let pool = setup_test_db_with_user();
    let tokens_cache = setup_tokens_cache("dummy-valid-token", "test_user_2");

    let s3_resp_mock = MockRequestDispatcher::with_status(500).with_body("gibberish");
    let s3_fs = setup_s3_fs(s3_resp_mock);

    let address = spawn_app(
        AppData::new()
            .pool(pool)
            .tokens_cache(tokens_cache)
            .s3_fs(s3_fs),
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&format!("{}/files/", &address))
        .header("Authorization", "Bearer dummy-valid-token")
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: ApiError = resp
        .json::<ApiError>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(500, json_resp.http_status);
    assert_eq!(1022, json_resp.error.code);
}

#[actix_rt::test]
async fn delete_files_s3_error() {
    let pool = setup_test_db_with_user();
    let tokens_cache = setup_tokens_cache("dummy-valid-token", "test_user_2");

    let s3_resp_mock = MockRequestDispatcher::with_status(400);
    let s3_fs = setup_s3_fs(s3_resp_mock);

    let address = spawn_app(
        AppData::new()
            .pool(pool)
            .tokens_cache(tokens_cache)
            .s3_fs(s3_fs),
    );

    let client = reqwest::Client::new();
    let resp = client
        .delete(&format!("{}/files/?filename=doesnotmatter", &address))
        .header("Authorization", "Bearer dummy-valid-token")
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: ApiError = resp
        .json::<ApiError>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(500, json_resp.http_status);
    assert_eq!(1022, json_resp.error.code);
}

#[actix_rt::test]
async fn delete_files_missing_filename() {
    let pool = setup_test_db_with_user();
    let tokens_cache = setup_tokens_cache("dummy-valid-token", "test_user_2");

    let s3_resp_mock = MockRequestDispatcher::with_status(400);
    let s3_fs = setup_s3_fs(s3_resp_mock);

    let address = spawn_app(
        AppData::new()
            .pool(pool)
            .tokens_cache(tokens_cache)
            .s3_fs(s3_fs),
    );

    let client = reqwest::Client::new();
    let resp = client
        .delete(&format!("{}/files/", &address))
        .header("Authorization", "Bearer dummy-valid-token")
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: ApiError = resp
        .json::<ApiError>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(409, json_resp.http_status);
    assert_eq!(1002, json_resp.error.code);
}

#[actix_rt::test]
async fn delete_files_ok() {
    let pool = setup_test_db_with_user();
    let tokens_cache = setup_tokens_cache("dummy-valid-token", "test_user_2");

    let s3_resp_mock = MockRequestDispatcher::with_status(StatusCode::NO_CONTENT.as_u16());
    let s3_fs = setup_s3_fs(s3_resp_mock);

    let address = spawn_app(
        AppData::new()
            .pool(pool)
            .tokens_cache(tokens_cache)
            .s3_fs(s3_fs),
    );

    let client = reqwest::Client::new();
    let resp = client
        .delete(&format!(
            "{}/files/?filename=same-wheter-it-exist-or-not",
            &address
        ))
        .header("Authorization", "Bearer dummy-valid-token")
        .send()
        .await
        .expect("Failed to execute request");

    let json_resp: RemoveFileResponse = resp
        .json::<RemoveFileResponse>()
        .await
        .expect("Cannot decode JSON response");

    assert_eq!(200, json_resp.status);
}
