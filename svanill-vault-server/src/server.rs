use actix_web::dev::Server;
use actix_web::middleware::{errhandlers::ErrorHandlers, Logger};
use actix_web::{http, App, HttpServer};

use crate::auth::tokens_cache::TokensCache;
use crate::file_server::FileServer;
use crate::http::handlers::{config_handlers, render_40x, render_500};
use diesel::{
    r2d2::{self, ConnectionManager},
    SqliteConnection,
};
use r2d2::Pool;
use ring::hmac;
use std::sync::{Arc, RwLock};

pub fn run(
    host: String,
    port: u16,
    tokens_cache: Arc<RwLock<TokensCache>>,
    crypto_key: Arc<hmac::Key>,
    pool: Pool<ConnectionManager<SqliteConnection>>,
    s3_fs: Arc<FileServer>,
) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .data(crypto_key.clone())
            .data(tokens_cache.clone())
            .data(s3_fs.clone())
            .wrap(ErrorHandlers::new().handler(http::StatusCode::INTERNAL_SERVER_ERROR, render_500))
            .wrap(ErrorHandlers::new().handler(http::StatusCode::BAD_REQUEST, render_40x))
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
            .configure(config_handlers)
    })
    .bind((host, port))?
    .run();

    // No .await here!
    Ok(server)
}
