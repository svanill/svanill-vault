use crate::auth::auth_token::AuthToken;
use crate::auth::tokens_cache::TokensCache;
use crate::auth::Username;
use crate::errors::ApiError;
use actix_web::{dev::ServiceRequest, http::StatusCode, web, Error, HttpMessage};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use anyhow::Result;
use std::sync::{Arc, RwLock};

fn validate_token(
    tokens_cache: &web::Data<Arc<RwLock<TokensCache>>>,
    token: AuthToken,
) -> Option<Username> {
    tokens_cache
        .write()
        .unwrap() // PANIC on token's lock poisoned
        .get_username(&token)
        .map(Username)
}

pub async fn auth_validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, Error> {
    let maybe_tokens_cache = req.app_data::<web::Data<Arc<RwLock<TokensCache>>>>();
    let tokens_cache =
        maybe_tokens_cache.expect("the tokens_cache have not been setup to this route");

    match validate_token(tokens_cache, AuthToken(credentials.token().to_owned())) {
        Some(user) => {
            req.extensions_mut().insert(user);
            Ok(req)
        }
        None => {
            Err(ApiError::new(StatusCode::UNAUTHORIZED, 401, "Unhauthorized".to_owned()).into())
        }
    }
}
