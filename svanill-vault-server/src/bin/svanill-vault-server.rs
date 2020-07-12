use actix_http::HttpMessage;
use actix_web::middleware::Logger;
use actix_web::{dev::ServiceRequest, guard, http, web, App, Error, HttpServer};
use actix_web_httpauth::extractors::bearer::{BearerAuth, Config};
use actix_web_httpauth::extractors::AuthenticationError;
use actix_web_httpauth::middleware::HttpAuthentication;
use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use ring::{hmac, rand};
use rusoto_core::Region;
use std::env;
use std::sync::{Arc, RwLock};
use structopt::StructOpt;
use svanill_vault_server::auth::TokensCache;
use svanill_vault_server::auth_token::AuthToken;
use svanill_vault_server::errors::VaultError;
use svanill_vault_server::file_server;
use svanill_vault_server::http as vault_http;
use svanill_vault_server::http::Username;

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

pub fn validate_token(
    tokens_cache: web::Data<Arc<RwLock<TokensCache>>>,
    token: AuthToken,
) -> Option<Username> {
    tokens_cache
        .write()
        .unwrap() // PANIC on token's lock poisoned
        .get_username(&token)
        .map(Username)
}

async fn validator(req: ServiceRequest, credentials: BearerAuth) -> Result<ServiceRequest, Error> {
    let config = req
        .app_data::<Config>()
        .map(|data| data.get_ref().clone())
        .unwrap_or_else(Default::default);

    let maybe_tokens_cache = req.app_data::<Arc<RwLock<TokensCache>>>();
    let tokens_cache = maybe_tokens_cache.unwrap(); // PANIC on missing tokens cache

    match validate_token(tokens_cache, AuthToken(credentials.token().to_owned())) {
        Some(user) => {
            req.extensions_mut().insert(user);
            Ok(req)
        }
        None => Err(AuthenticationError::from(config).into()),
    }
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
async fn main() -> Result<()> {
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

    HttpServer::new(move || {
        // Setup authentication middleware
        let auth = HttpAuthentication::bearer(validator);

        App::new()
            .data(pool.clone())
            .data(key.clone())
            .data(tokens_cache.clone())
            .data(s3_fs.clone())
            .wrap(Logger::default())
            .wrap(
                actix_cors::Cors::new()
                    .allowed_methods(vec!["HEAD", "OPTIONS", "GET", "POST", "PUT", "DELETE"])
                    .allowed_headers(vec![
                        http::header::AUTHORIZATION,
                        http::header::ACCEPT,
                        http::header::CONTENT_TYPE,
                    ])
                    .max_age(86400)
                    .finish(),
            )
            .service(vault_http::handlers::favicon)
            .service(vault_http::handlers::index)
            .service(vault_http::handlers::auth_user_request_challenge)
            .service(
                web::resource("/auth/answer-challenge")
                    .route(web::post().to(vault_http::handlers::auth_user_answer_challenge))
                    .name("auth_user_answer_challenge")
                    .data(web::JsonConfig::default().limit(512)),
            )
            .service(
                web::scope("")
                    .wrap(auth)
                    .service(vault_http::handlers::new_user)
                    .service(vault_http::handlers::request_upload_url)
                    .service(vault_http::handlers::list_user_files),
            )
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
    .await?;

    Ok::<(), anyhow::Error>(())
}
