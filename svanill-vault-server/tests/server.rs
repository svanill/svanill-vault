use actix_web::{http::Method, test, App};
use ctor::ctor;
use diesel::{
    r2d2::{self, ConnectionManager},
    SqliteConnection,
};
use r2d2::Pool;
use std::sync::{Arc, RwLock};
use svanill_vault_server::auth::auth_token::AuthToken;
use svanill_vault_server::auth::tokens_cache::TokensCache;
use svanill_vault_server::errors::ApiError;
use svanill_vault_server::http::handlers::config_handlers;
use svanill_vault_server::openapi_models::GetStartingEndpointsResponse;

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

#[actix_rt::test]
#[ignore]
async fn noauth_noroute_must_return_401() {
    let tokens_cache: Arc<RwLock<TokensCache>> = Arc::new(RwLock::new(TokensCache::default()));

    let mut app =
        test::init_service(App::new().data(tokens_cache).configure(config_handlers)).await;

    let req = test::TestRequest::with_header("Authorization", "Bearer dummy-invalid-token")
        .uri("/not-exist")
        .to_request();

    let resp: ApiError = test::read_response_json(&mut app, req).await;

    assert_eq!(401, resp.http_status);
    assert_eq!(401, resp.error.code);
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
