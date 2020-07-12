use crate::auth::auth_token::AuthToken;
use crate::auth::tokens_cache::TokensCache;
use crate::auth::Username;
use actix_http::HttpMessage;
use actix_web::{dev::ServiceRequest, web, Error};
use actix_web_httpauth::extractors::bearer::{BearerAuth, Config};
use actix_web_httpauth::extractors::AuthenticationError;
use anyhow::Result;
use std::sync::{Arc, RwLock};

fn validate_token(
    tokens_cache: web::Data<Arc<RwLock<TokensCache>>>,
    token: AuthToken,
) -> Option<Username> {
    tokens_cache
        .as_ref()
        .write()
        .unwrap() // PANIC on token's lock poisoned
        .get_username(&token)
        .map(Username)
}

pub async fn auth_validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, Error> {
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
