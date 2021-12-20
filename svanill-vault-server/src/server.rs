use actix_web::dev::Server;
use actix_web::middleware::{ErrorHandlers, Logger};
use actix_web::{http, web, App, HttpServer};

use crate::auth::tokens_cache::TokensCache;
use crate::file_server::FileServer;
use crate::http::handlers::{config_handlers, render_40x, render_500};
use crate::http::sentry_middleware::enrich_sentry_event_with_request_metadata;
use actix_web::dev::Service;
use diesel::{
    r2d2::{self, ConnectionManager},
    SqliteConnection,
};
use r2d2::Pool;
use ring::hmac;
use std::net::TcpListener;
use std::sync::{Arc, RwLock};

pub struct AppData {
    pub tokens_cache: TokensCache,
    pub crypto_key: hmac::Key,
    pub pool: Pool<ConnectionManager<SqliteConnection>>,
    pub s3_fs: FileServer,
    pub cors_origin: String,
}

pub fn run(listener: TcpListener, data: AppData) -> Result<Server, std::io::Error> {
    let tokens_cache = Arc::new(RwLock::new(data.tokens_cache));
    let crypto_key = Arc::new(data.crypto_key);
    let pool = data.pool;
    let s3_fs = Arc::new(data.s3_fs);
    let cors_origin = data.cors_origin;

    let server = HttpServer::new(move || {
        let cors_origin = &cors_origin;

        let mut cors_handler = actix_cors::Cors::default()
            .allowed_methods(vec!["HEAD", "OPTIONS", "GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                http::header::CONTENT_TYPE,
            ])
            .max_age(86400);

        cors_handler = if cors_origin == "*" {
            cors_handler.allow_any_origin()
        } else {
            cors_handler.allowed_origin(cors_origin)
        };

        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(crypto_key.clone()))
            .app_data(web::Data::new(tokens_cache.clone()))
            .app_data(web::Data::new(s3_fs.clone()))
            .wrap(ErrorHandlers::new().handler(http::StatusCode::INTERNAL_SERVER_ERROR, render_500))
            .wrap(ErrorHandlers::new().handler(http::StatusCode::BAD_REQUEST, render_40x))
            .wrap(Logger::default())
            .wrap(cors_handler)
            .wrap_fn(|req, srv| {
                enrich_sentry_event_with_request_metadata(&req);
                srv.call(req)
            })
            .configure(config_handlers)
    })
    .listen(listener)?
    .run();

    // No .await here!
    Ok(server)
}
