use actix_web::{dev::Service, http::Method, test, App};
use ctor::ctor;
use diesel::{
    r2d2::{self, ConnectionManager},
    RunQueryDsl, SqliteConnection,
};
use r2d2::Pool;
use ring::hmac;
use ring::test::rand::FixedByteRandom;
use std::sync::{Arc, RwLock};
use svanill_vault_server::auth::auth_token::AuthToken;
use svanill_vault_server::auth::tokens_cache::TokensCache;
use svanill_vault_server::errors::ApiError;
use svanill_vault_server::http::handlers::config_handlers;
use svanill_vault_server::openapi_models::{
    AnswerUserChallengeRequest, AnswerUserChallengeResponse, AskForTheChallengeResponse,
    GetStartingEndpointsResponse,
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
    let body = test::read_body(resp).await;
    let json_resp: AskForTheChallengeResponse = to_json_response(&body).unwrap();

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

    let body = test::read_body(resp).await;
    let json_resp: AnswerUserChallengeResponse = to_json_response(&body).unwrap();

    assert_eq!(200, json_resp.status);
    assert!(!json_resp.content.token.is_empty());
}

/**
 * Convert json body to the expected format.
 *
 * Contrary to test::read_response_json it provides
 * a better error output if the handler returned a
 * ApiError.
 */
fn to_json_response<T>(body: &[u8]) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    if body.is_empty() {
        return Err(String::from("Response body is empty"));
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
