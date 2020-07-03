use actix_web::{get, App, HttpServer, Responder};
use std::env;

#[get("/")]
async fn index() -> impl Responder {
    format!("todo")
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=debug,actix_server=info");
    env_logger::init();

    HttpServer::new(|| App::new().service(index))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
