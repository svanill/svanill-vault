use actix_web::middleware::Logger;
use actix_web::{get, web, App, Error, HttpResponse, HttpServer, Responder};
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use std::env;
use structopt::StructOpt;
use svanill_vault_server::db;

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

#[get("/auth/request-challenge")]
async fn auth_user_request_challenge(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    use svanill_vault_server::db::schema::user::dsl::*;
    let conn = pool.get().expect("couldn't get db connection from pool");

    let maybe_users = web::block(move || user.load::<db::models::User>(&conn)).await;

    if let Ok(users) = maybe_users {
        Ok(HttpResponse::Ok().json(users))
    } else {
        Ok(HttpResponse::InternalServerError().finish())
    }
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
    })
    .bind((opt.host, opt.port))?
    .run()
    .await
}
