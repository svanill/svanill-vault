use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use ring::{hmac, rand};
use rusoto_core::Region;
use std::env;
use std::net::TcpListener;
use std::sync::{Arc, RwLock};
use structopt::StructOpt;
use svanill_vault_server::auth::tokens_cache::TokensCache;
use svanill_vault_server::file_server;
use svanill_vault_server::server::run;

#[macro_use]
extern crate diesel_migrations;

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
    /// S3 bucket
    #[structopt(long = "s3-bucket", env = "SVANILL_VAULT_S3_BUCKET")]
    s3_bucket: String,
    /// S3 region
    #[structopt(long = "s3-region")]
    s3_region: Option<String>,
    /// S3 access key id
    #[structopt(long = "s3-access-key-id")]
    s3_access_key_id: Option<String>,
    /// S3 secret access key
    #[structopt(long = "s3-secret-access-key")]
    s3_secret_access_key: Option<String>,
    /// S3 endpoint (optional, for S3 compatible servers)
    #[structopt(long = "s3-endpoint", env = "SVANILL_VAULT_S3_ENDPOINT")]
    s3_endpoint: Option<String>,
    /// Max number of concurrent users
    #[structopt(
        long = "url-duration",
        default_value = "5",
        env = "SVANILL_VAULT_URL_DURATION"
    )]
    presigned_url_duration_in_min: u32,
}

#[actix_rt::main]
async fn main() -> Result<()> {
    #[cfg(debug_assertions)]
    color_backtrace::install();

    env::set_var(
        "RUST_LOG",
        "info,rusoto=warn,actix_http=debug,actix_web=debug,actix_server=info",
    );
    env_logger::init();

    let opt = Opt::from_args();

    if let Some(region) = opt.s3_region {
        env::set_var("AWS_DEFAULT_REGION", region);
    }

    let region = if let Some(s3_endpoint) = opt.s3_endpoint {
        Region::Custom {
            name: env::var("AWS_DEFAULT_REGION").unwrap_or_else(|_| "us-east-1".to_owned()),
            endpoint: s3_endpoint,
        }
    } else {
        Region::default()
    };

    let s3_fs = Arc::new(
        file_server::FileServer::new(
            region,
            opt.s3_bucket,
            std::time::Duration::from_secs(opt.presigned_url_duration_in_min as u64 * 60),
        )
        .await?,
    );

    // set up database connection pool
    let connspec = opt.db_url;
    let manager = ConnectionManager::<SqliteConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create database connection pool");

    embed_migrations!();
    let connection = pool.get().expect("couldn't get db connection from pool");

    embedded_migrations::run(&connection).expect("failed to run migrations");

    // generate server key, used to sign and verify tokens
    let rng = rand::SystemRandom::new();
    let key = std::sync::Arc::new(
        hmac::Key::generate(hmac::HMAC_SHA256, &rng).expect("Cannot generate cryptographyc key"),
    );

    // Use a LRU cache to store tokens, until we add redis support
    let tokens_cache: Arc<RwLock<TokensCache>> = Arc::new(RwLock::new(TokensCache::new(
        opt.max_concurrent_users,
        std::time::Duration::from_secs(60 * opt.auth_token_timeout as u64),
    )));

    let listener =
        TcpListener::bind(format!("{}:{}", opt.host, opt.port)).expect("Failed to bind port");

    let _server = run(listener, tokens_cache, key, pool, s3_fs)?.await;

    Ok::<(), anyhow::Error>(())
}
