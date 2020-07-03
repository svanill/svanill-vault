use actix_web::{get, App, HttpServer, Responder};
use std::env;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "svanill-vault-cli",
    about = "Read/Write data from/to a svanill vault server"
)]
struct Opt {
    /// Server host
    #[structopt(short = "H", default_value = "localhost")]
    host: String,
    /// Server port
    #[structopt(short = "P", default_value = "8080")]
    port: u16,
}

#[get("/")]
async fn index() -> impl Responder {
    format!("todo")
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=debug,actix_server=info");
    env_logger::init();

    let opt = Opt::from_args();

    HttpServer::new(|| App::new().service(index))
        .bind((opt.host, opt.port))?
        .run()
        .await
}
