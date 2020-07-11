use actix_web::middleware::Logger;
use actix_web::{get, guard, http, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use ring::{hmac, rand};
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;
use svanill_vault_server::auth_token::AuthToken;
use svanill_vault_server::db::auth::TokensCache;
use svanill_vault_server::models::{
    AnswerUserChallengeRequest, AnswerUserChallengeResponse, AskForTheChallengeResponse,
    GetStartingEndpointsResponse,
};
use svanill_vault_server::{db, errors::VaultError};

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "svanill-vault-cli",
    about = "Read/Write data from/to a svanill vault server"
)]
struct Opt {
    /// Server host
    #[structopt(short = "H", default_value = "localhost", env = "SVANILL_VAULT_HOST")]
    host: String,
    /// Server port
    #[structopt(short = "P", default_value = "8080", env = "SVANILL_VAULT_PORT")]
    port: u16,
    /// Database url
    #[structopt(short = "d", default_value = "vault.db", env = "SVANILL_VAULT_DB")]
    db_url: String,
    /// Authorization Token timeout in minutes
    #[structopt(
        short = "t",
        default_value = "60",
        env = "SVANILL_VAULT_AUTH_TOKEN_TIMEOUT"
    )]
    auth_token_timeout: u32,
    /// Max number of concurrent users
    #[structopt(
        short = "u",
        default_value = "1000",
        env = "SVANILL_VAULT_MAX_CONC_USERS"
    )]
    max_concurrent_users: usize,
}

#[get("/")]
async fn index(req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().json(
        serde_json::from_value::<GetStartingEndpointsResponse>(json!({
            "status": 200,
            "links": {
                "request_auth_challenge": hateoas_auth_user_request_challenge(&req),
                "create_user": hateoas_new_user(&req)
            }
        }))
        .unwrap(),
    )
}

#[get("/favicon.ico")]
async fn favicon() -> HttpResponse {
    HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "image/svg+xml")
        .body(include_str!("../../favicon.svg"))
}

#[derive(Deserialize)]
struct AuthRequestChallengeQueryFields {
    // XXX this is optional, but it shouldn't be. Maybe make it part of the URI?
    username: Option<String>,
}

#[get("/auth/request-challenge")]
async fn auth_user_request_challenge(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    q: web::Query<AuthRequestChallengeQueryFields>,
) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("couldn't get db connection from pool");

    if q.username.is_none() {
        return Err(VaultError::FieldRequired {
            field: "username".into(),
        }
        .into());
    };

    let maybe_user =
        web::block(move || db::actions::find_user_by_username(&conn, q.username.as_ref().unwrap()))
            .await?;

    if let Some(user) = maybe_user {
        Ok(HttpResponse::Ok().json(
            serde_json::from_value::<AskForTheChallengeResponse>(json!({
                "status": 200,
                "content": {
                    "challenge": user.challenge,
                },
                "links": {
                    "answer_auth_challenge": hateoas_auth_user_answer_challenge(&req),
                    "create_user": hateoas_new_user(&req)
                }
            }))
            .unwrap(),
        ))
    } else {
        Err(VaultError::UserDoesNotExist.into())
    }
}

async fn auth_user_answer_challenge(
    req: HttpRequest,
    payload: web::Json<AnswerUserChallengeRequest>,
    pool: web::Data<DbPool>,
    crypto_key: web::Data<std::sync::Arc<ring::hmac::Key>>,
    tokens_cache: web::Data<Arc<Mutex<TokensCache>>>,
) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("couldn't get db connection from pool");
    let answer = payload.answer.clone();

    let maybe_user =
        web::block(move || db::actions::find_user_by_username(&conn, &payload.username)).await?;

    if let Some(user) = maybe_user {
        let correct_answer = user.answer;

        if answer != correct_answer {
            return Err(VaultError::ChallengeMismatchError.into());
        }

        // Generate a new signed token
        let token = AuthToken::new(&*crypto_key);
        let token_as_string = token.to_string();

        // Store the token, alongside the user it represent
        tokens_cache.lock().unwrap().insert(token, user.username);

        Ok(HttpResponse::Ok().json(
            serde_json::from_value::<AnswerUserChallengeResponse>(json!({
                "content": {
                    "token": token_as_string
                },
                "links": {
                    "files_list": hateoas_list_user_files(&req),
                    "request_upload_url": hateoas_request_upload_url(&req),
                },
                "status":200
            }))
            .unwrap(),
        ))
    } else {
        Err(VaultError::UserDoesNotExist.into())
    }
}

#[get("/users/")]
async fn new_user() -> Result<HttpResponse, Error> {
    unimplemented!()
}

#[get("/files/request-upload-url")]
async fn request_upload_url() -> Result<HttpResponse, Error> {
    unimplemented!()
}

#[get("/files/")]
async fn list_user_files() -> Result<HttpResponse, Error> {
    unimplemented!()
}

fn hateoas_new_user(req: &HttpRequest) -> serde_json::Value {
    let url = req.url_for_static("new_user").unwrap();
    json!({
        "href": url.as_str(),
        "rel": "user"
    })
}

fn hateoas_auth_user_answer_challenge(req: &HttpRequest) -> serde_json::Value {
    let url = req.url_for_static("auth_user_answer_challenge").unwrap();
    json!({
        "href": url.as_str(),
        "rel": "auth"
    })
}

fn hateoas_auth_user_request_challenge(req: &HttpRequest) -> serde_json::Value {
    let url = req.url_for_static("auth_user_request_challenge").unwrap();
    json!({
        "href": url.as_str(),
        "rel": "auth"
    })
}

fn hateoas_list_user_files(req: &HttpRequest) -> serde_json::Value {
    let url = req.url_for_static("list_user_files").unwrap();
    json!({
        "href": url.as_str(),
        "rel": "file"
    })
}

fn hateoas_request_upload_url(req: &HttpRequest) -> serde_json::Value {
    let url = req.url_for_static("request_upload_url").unwrap();
    json!({
        "href": url.as_str(),
        "rel": "file"
    })
}

/// 404 handler
async fn p404() -> Result<&'static str, Error> {
    Err(VaultError::NotFound.into())
}

// Not allowed handler
async fn method_not_allowed() -> Result<&'static str, Error> {
    Err(VaultError::MethodNotAllowed.into())
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env::set_var(
        "RUST_LOG",
        "actix_http=debug,actix_web=debug,actix_server=info",
    );
    env_logger::init();

    let opt = Opt::from_args();

    // set up database connection pool
    let connspec = opt.db_url;
    let manager = ConnectionManager::<SqliteConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create database connection pool");

    // generate server key, used to sign and verify tokens
    let rng = rand::SystemRandom::new();
    let key = std::sync::Arc::new(
        hmac::Key::generate(hmac::HMAC_SHA256, &rng).expect("Cannot generate cryptographyc key"),
    );

    // Use a LRU cache to store tokens, until we add redis support
    let tokens_cache: Arc<Mutex<TokensCache>> = Arc::new(Mutex::new(TokensCache::new(
        opt.max_concurrent_users,
        std::time::Duration::from_secs(60 * opt.auth_token_timeout as u64),
    )));

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .data(key.clone())
            .data(tokens_cache.clone())
            .wrap(Logger::default())
            .service(favicon)
            .service(index)
            .service(auth_user_request_challenge)
            .service(
                web::resource("/auth/answer-challenge")
                    .route(web::post().to(auth_user_answer_challenge))
                    .name("auth_user_answer_challenge")
                    .data(web::JsonConfig::default().limit(512)),
            )
            .service(new_user)
            .service(request_upload_url)
            .service(list_user_files)
            .default_service(
                // 404 for GET request
                web::resource("")
                    .route(web::get().to(p404))
                    // all requests that are not `GET`
                    .route(
                        web::route()
                            .guard(guard::Not(guard::Get()))
                            .to(method_not_allowed),
                    ),
            )
    })
    .bind((opt.host, opt.port))?
    .run()
    .await
}
