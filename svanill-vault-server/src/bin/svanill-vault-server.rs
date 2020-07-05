use actix_web::middleware::Logger;
use actix_web::{get, guard, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use serde::Deserialize;
use serde_json::json;
use std::env;
use structopt::StructOpt;
use svanill_vault_server::models::AskForTheChallengeResponse;
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
}

#[get("/")]
async fn index() -> impl Responder {
    format!("todo")
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

    let username = q
        .username
        .clone()
        .ok_or_else(|| VaultError::FieldRequired {
            field: "username".to_owned(),
        })?;

    let maybe_user =
        web::block(move || db::actions::find_user_by_username(&conn, &username)).await?;

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

#[get("/auth/answer-challenge")]
async fn auth_user_answer_challenge() -> Result<HttpResponse, Error> {
    unimplemented!()
}

#[get("/users/")]
async fn new_user() -> Result<HttpResponse, Error> {
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
    env::set_var("RUST_LOG", "actix_web=debug,actix_server=info");
    env_logger::init();

    let opt = Opt::from_args();

    // set up database connection pool
    let connspec = opt.db_url;
    let manager = ConnectionManager::<SqliteConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create database connection pool");

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .wrap(Logger::default())
            .service(index)
            .service(auth_user_request_challenge)
            .service(auth_user_answer_challenge)
            .service(new_user)
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
