use actix_http::http::StatusCode;
use actix_web::{dev::Service, http::Method, middleware::errhandlers::ErrorHandlers, test, App};
use ctor::ctor;
use diesel::{
    r2d2::{self, ConnectionManager},
    RunQueryDsl, SqliteConnection,
};
use r2d2::Pool;
use ring::hmac;
use ring::test::rand::FixedByteRandom;
use rusoto_core::Region;
use rusoto_credential::ProvideAwsCredentials;
use rusoto_mock::{MockCredentialsProvider, MockRequestDispatcher};
use std::sync::{Arc, RwLock};
use svanill_vault_server::auth::auth_token::AuthToken;
use svanill_vault_server::auth::{tokens_cache::TokensCache, Username};
use svanill_vault_server::errors::ApiError;
use svanill_vault_server::http::handlers::{config_handlers, render_40x};
use svanill_vault_server::{
    file_server,
    openapi_models::{
        AnswerUserChallengeRequest, AnswerUserChallengeResponse, AskForTheChallengeResponse,
        GetStartingEndpointsResponse, RequestUploadUrlRequestBody, RequestUploadUrlResponse,
    },
};

#[macro_use]
extern crate diesel_migrations;
embed_migrations!();

#[cfg(test)]
#[ctor]
fn init_color_backtrace() {
    color_backtrace::install();
}

fn prepare_tokens_cache(token: &str, username: &str) -> Arc<RwLock<TokensCache>> {
    let mut tokens_cache = TokensCache::default();
    tokens_cache.insert(AuthToken(token.to_string()), username.to_string());
    Arc::new(RwLock::new(tokens_cache))
}

fn setup_fake_random_key() -> std::sync::Arc<hmac::Key> {
    let rng = FixedByteRandom { byte: 0 };
    std::sync::Arc::new(
        hmac::Key::generate(hmac::HMAC_SHA256, &rng).expect("Cannot generate cryptographyc key"),
    )
}

#[actix_rt::test]
async fn noauth_noroute_must_return_401() {
    let tokens_cache: Arc<RwLock<TokensCache>> = Arc::new(RwLock::new(TokensCache::default()));

    let mut app =
        test::init_service(App::new().data(tokens_cache).configure(config_handlers)).await;

    let req = test::TestRequest::with_header("Authorization", "Bearer dummy-invalid-token")
        .uri("/not-exist")
        .to_request();

    let resp = app.call(req).await;

    // app call fails because the auth middleware interrupts it early
    let web_error = resp.err().unwrap();
    let json_resp: &ApiError = web_error.as_error::<ApiError>().unwrap();

    assert_eq!(401, json_resp.http_status);
    assert_eq!(401, json_resp.error.code);
}

#[actix_rt::test]
async fn auth_noroute_noget_must_return_405() {
    let tokens_cache = prepare_tokens_cache("dummy-valid-token", "test_user");

    let mut app =
        test::init_service(App::new().data(tokens_cache).configure(config_handlers)).await;

    let req = test::TestRequest::with_header("Authorization", "Bearer dummy-valid-token")
        .method(Method::PATCH)
        .uri("/not-exist")
        .to_request();

    let resp: ApiError = test::read_response_json(&mut app, req).await;

    assert_eq!(405, resp.http_status);
    assert_eq!(405, resp.error.code);
}

#[actix_rt::test]
async fn auth_noroute_get_must_return_404() {
    let tokens_cache = prepare_tokens_cache("dummy-valid-token", "test_user");

    let mut app =
        test::init_service(App::new().data(tokens_cache).configure(config_handlers)).await;

    let req = test::TestRequest::with_header("Authorization", "Bearer dummy-valid-token")
        .uri("/not-exist")
        .to_request();

    let resp: ApiError = test::read_response_json(&mut app, req).await;

    assert_eq!(404, resp.http_status);
    assert_eq!(404, resp.error.code);
}

#[actix_rt::test]
async fn root() {
    let mut app = test::init_service(App::new().configure(config_handlers)).await;

    let req = test::TestRequest::get().uri("/").to_request();

    let resp: GetStartingEndpointsResponse = test::read_response_json(&mut app, req).await;

    assert_eq!(200, resp.status);
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
    let pool = setup_test_db();

    let mut app =
        test::init_service(App::new().data(pool.clone()).configure(config_handlers)).await;

    let req = test::TestRequest::get()
        .uri("/auth/request-challenge")
        .to_request();

    let resp: ApiError = test::read_response_json(&mut app, req).await;

    assert_eq!(409, resp.http_status);
    assert_eq!(1002, resp.error.code);
}

#[actix_rt::test]
async fn get_auth_challenge_username_not_found() {
    let pool = setup_test_db();

    let mut app =
        test::init_service(App::new().data(pool.clone()).configure(config_handlers)).await;

    let req = test::TestRequest::get()
        .uri("/auth/request-challenge?username=notfound")
        .to_request();

    let resp: ApiError = test::read_response_json(&mut app, req).await;

    assert_eq!(401, resp.http_status);
    assert_eq!(1005, resp.error.code);
}

#[actix_rt::test]
async fn get_auth_challenge_ok() {
    let pool = setup_test_db_with_user();

    let mut app =
        test::init_service(App::new().data(pool.clone()).configure(config_handlers)).await;

    let req = test::TestRequest::get()
        .uri("/auth/request-challenge?username=test_user_2")
        .to_request();

    let resp = app.call(req).await.expect("failed to make the request");
    let json_resp: AskForTheChallengeResponse = to_json_response(resp).await.unwrap();

    assert_eq!(200, json_resp.status);
    assert_eq!("challenge2", json_resp.content.challenge);
}

#[actix_rt::test]
async fn answer_auth_challenge_username_not_found() {
    let pool = setup_test_db_with_user();
    let tokens_cache: Arc<RwLock<TokensCache>> = Arc::new(RwLock::new(TokensCache::default()));
    let random_key = setup_fake_random_key();

    let mut app = test::init_service(
        App::new()
            .data(pool)
            .data(tokens_cache)
            .data(random_key)
            .configure(config_handlers),
    )
    .await;

    let payload = AnswerUserChallengeRequest {
        username: "notfound".to_owned(),
        answer: "any_answer".to_owned(),
    };

    let req = test::TestRequest::post()
        .uri("/auth/answer-challenge")
        .set_json(&payload)
        .to_request();

    let resp: ApiError = test::read_response_json(&mut app, req).await;

    assert_eq!(401, resp.http_status);
    assert_eq!(1005, resp.error.code);
}

#[actix_rt::test]
async fn answer_auth_challenge_wrong_answer() {
    let pool = setup_test_db_with_user();
    let tokens_cache: Arc<RwLock<TokensCache>> = Arc::new(RwLock::new(TokensCache::default()));
    let random_key = setup_fake_random_key();

    let mut app = test::init_service(
        App::new()
            .data(pool)
            .data(tokens_cache)
            .data(random_key)
            .configure(config_handlers),
    )
    .await;

    let payload = AnswerUserChallengeRequest {
        username: "test_user_2".to_owned(),
        answer: "wrong_answer".to_owned(),
    };

    let req = test::TestRequest::post()
        .uri("/auth/answer-challenge")
        .set_json(&payload)
        .to_request();

    let resp: ApiError = test::read_response_json(&mut app, req).await;

    assert_eq!(401, resp.http_status);
    assert_eq!(1006, resp.error.code);
}

#[actix_rt::test]
async fn answer_auth_challenge_ok() {
    let pool = setup_test_db_with_user();
    let tokens_cache: Arc<RwLock<TokensCache>> = Arc::new(RwLock::new(TokensCache::default()));
    let random_key = setup_fake_random_key();

    let mut app = test::init_service(
        App::new()
            .data(pool)
            .data(tokens_cache)
            .data(random_key)
            .configure(config_handlers),
    )
    .await;

    let payload = AnswerUserChallengeRequest {
        username: "test_user_2".to_owned(),
        answer: "answer2".to_owned(),
    };

    let req = test::TestRequest::post()
        .uri("/auth/answer-challenge")
        .set_json(&payload)
        .to_request();

    let resp = app.call(req).await.expect("failed to make the request");
    let json_resp: AnswerUserChallengeResponse = to_json_response(resp).await.unwrap();

    assert_eq!(200, json_resp.status);
    assert!(!json_resp.content.token.is_empty());

    // Do the same request again and verify that every token is unique
    let req2 = test::TestRequest::post()
        .uri("/auth/answer-challenge")
        .set_json(&payload)
        .to_request();

    let resp2 = app.call(req2).await.expect("failed to make the request");
    let json_resp2: AnswerUserChallengeResponse = to_json_response(resp2).await.unwrap();

    assert_eq!(200, json_resp2.status);
    assert_ne!(json_resp.content.token, json_resp2.content.token);
}

async fn setup_s3_fs(s3_resp_mock: MockRequestDispatcher) -> Arc<file_server::FileServer> {
    let region = Region::EuCentral1;
    let bucket = "test_bucket".to_string();

    let provider = MockCredentialsProvider;
    let credentials = provider.credentials().await.unwrap();

    let client = rusoto_s3::S3Client::new_with(s3_resp_mock, provider, Default::default());

    let fileserver = file_server::FileServer {
        region,
        bucket,
        client,
        credentials,
        presigned_url_timeout: std::time::Duration::from_secs(10),
    };

    Arc::new(fileserver)
}

#[actix_rt::test]
async fn request_upload_url_ok() {
    let pool = setup_test_db_with_user();
    let tokens_cache = prepare_tokens_cache("dummy-valid-token", "test_user_2");

    let s3_resp_mock = MockRequestDispatcher::default();
    let s3_fs = setup_s3_fs(s3_resp_mock).await;

    let mut app = test::init_service(
        App::new()
            .data(pool)
            .data(s3_fs)
            .data(tokens_cache)
            .configure(config_handlers),
    )
    .await;

    let payload = RequestUploadUrlRequestBody {
        filename: "test_filename".to_owned(),
    };

    let req_username = Username("test_user_2".to_owned());

    let mut req = test::TestRequest::with_header("Authorization", "Bearer dummy-valid-token")
        .method(Method::POST)
        .uri("/files/request-upload-url")
        .set_json(&payload)
        .to_request();

    req.head_mut().extensions_mut().insert(req_username);

    let resp = app.call(req).await.expect("failed to make the request");
    let json_resp: RequestUploadUrlResponse = to_json_response(resp).await.unwrap();

    assert_eq!(200, json_resp.status);
    assert!(!json_resp.links.upload_url.href.is_empty());
    assert!(!json_resp.links.retrieve_url.href.is_empty());
}

#[actix_rt::test]
async fn request_upload_url_empty_filename() {
    let pool = setup_test_db_with_user();
    let tokens_cache = prepare_tokens_cache("dummy-valid-token", "test_user_2");

    let s3_resp_mock = MockRequestDispatcher::default().with_body("ciao");
    let s3_fs = setup_s3_fs(s3_resp_mock).await;

    let mut app = test::init_service(
        App::new()
            .data(pool)
            .data(s3_fs)
            .data(tokens_cache)
            .configure(config_handlers),
    )
    .await;

    let req_username = Username("test_user_2".to_owned());

    let payload = RequestUploadUrlRequestBody {
        filename: "".to_owned(),
    };

    let mut req = test::TestRequest::with_header("Authorization", "Bearer dummy-valid-token")
        .method(Method::POST)
        .uri("/files/request-upload-url")
        .set_json(&payload)
        .to_request();

    req.head_mut().extensions_mut().insert(req_username);

    let resp = app.call(req).await.expect("failed to make the request");
    let json_resp: ApiError = to_json_response(resp).await.unwrap();

    assert_eq!(409, json_resp.http_status);
    assert_eq!(1002, json_resp.error.code);
}

#[actix_rt::test]
async fn request_upload_url_wrong_payload() {
    let pool = setup_test_db_with_user();
    let tokens_cache = prepare_tokens_cache("dummy-valid-token", "test_user_2");

    let s3_resp_mock = MockRequestDispatcher::default().with_body("ciao");
    let s3_fs = setup_s3_fs(s3_resp_mock).await;

    let mut app = test::init_service(
        App::new()
            .wrap(ErrorHandlers::new().handler(StatusCode::BAD_REQUEST, render_40x))
            .data(pool)
            .data(s3_fs)
            .data(tokens_cache)
            .configure(config_handlers),
    )
    .await;

    let req_username = Username("test_user_2".to_owned());

    let payload = "not a proper payload".to_string();

    let mut req = test::TestRequest::with_header("Authorization", "Bearer dummy-valid-token")
        .method(Method::POST)
        .uri("/files/request-upload-url")
        .set_json(&payload)
        .to_request();

    req.head_mut().extensions_mut().insert(req_username);

    let resp = app.call(req).await.expect("failed to make the request");
    let json_resp: ApiError = to_json_response(resp).await.unwrap();

    assert_eq!(400, json_resp.http_status);
    assert_eq!(1024, json_resp.error.code);
    assert!(json_resp
        .error
        .message
        .contains("expected struct RequestUploadUrlRequestBody"));
}

/**
 * Convert json body to the expected format.
 *
 * Contrary to test::read_response_json it provides
 * a better error output if the handler returned a
 * ApiError.
 */
async fn to_json_response<T>(resp: actix_web::dev::ServiceResponse) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let status = resp.status();
    let body = test::read_body(resp).await;

    if body.is_empty() {
        return Err(format!("Body is empty. HTTP Status was: {}", status));
    }

    serde_json::from_slice::<T>(&body).map_err(|_| {
        // failed to deserialize. Was perhaps an ApiError?
        let res = serde_json::from_slice::<ApiError>(&body);

        if let Ok(api_error) = res {
            serde_json::to_string(&api_error).unwrap()
        } else {
            format!(
                "Response body does not match expected JSON format. Got: {}",
                std::str::from_utf8(&body).unwrap()
            )
        }
    })
}
