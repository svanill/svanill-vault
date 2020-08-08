use super::auth_middleware::auth_validator;
use super::handlers;
use crate::auth::auth_token::AuthToken;
use crate::auth::tokens_cache::TokensCache;
use crate::auth::Username;
use crate::file_server;
use crate::openapi_models::{
    AnswerUserChallengeRequest, AnswerUserChallengeResponse, AskForTheChallengeResponse,
    GetStartingEndpointsResponse, RemoveFileResponse, RequestUploadUrlRequestBody,
    RequestUploadUrlResponse, RetrieveListOfUserFilesResponse,
    RetrieveListOfUserFilesResponseContentItemContent,
};
use crate::{db, errors::VaultError};
use actix_web::{delete, get, guard, http, post, web, Error, HttpRequest, HttpResponse};
use actix_web_httpauth::middleware::HttpAuthentication;
use anyhow::Result;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use serde::Deserialize;
use serde_json::json;
use std::sync::{Arc, RwLock};

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

#[get("/")]
async fn index(req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().json(
        serde_json::from_value::<GetStartingEndpointsResponse>(json!({
            "status": 200,
            "links": {
                "request_auth_challenge": hateoas_auth_user_request_challenge(&req),
                "create_user": hateoas_new_user(&req)
            }
        }))
        .unwrap(),
    )
}

#[get("/favicon.ico")]
async fn favicon() -> HttpResponse {
    HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "image/svg+xml")
        .body(include_str!("../../favicon.svg"))
}

#[derive(Deserialize)]
pub struct AuthRequestChallengeQueryFields {
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

    if q.username.is_none() {
        return Err(VaultError::FieldRequired {
            field: "username".into(),
        }
        .into());
    };

    let maybe_user =
        web::block(move || db::actions::find_user_by_username(&conn, q.username.as_ref().unwrap()))
            .await?;

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

async fn auth_user_answer_challenge(
    req: HttpRequest,
    payload: web::Json<AnswerUserChallengeRequest>,
    pool: web::Data<DbPool>,
    crypto_key: web::Data<std::sync::Arc<ring::hmac::Key>>,
    tokens_cache: web::Data<Arc<RwLock<TokensCache>>>,
) -> Result<HttpResponse, Error> {
    let conn = pool.get().expect("couldn't get db connection from pool");
    let answer = payload.answer.clone();

    let maybe_user =
        web::block(move || db::actions::find_user_by_username(&conn, &payload.username)).await?;

    if let Some(user) = maybe_user {
        let correct_answer = user.answer;

        if answer != correct_answer {
            return Err(VaultError::ChallengeMismatchError.into());
        }

        // Generate a new signed token
        let token = AuthToken::new(&*crypto_key);
        let token_as_string = token.to_string();

        // Store the token, alongside the user it represent
        tokens_cache.write().unwrap().insert(token, user.username);

        Ok(HttpResponse::Ok().json(
            serde_json::from_value::<AnswerUserChallengeResponse>(json!({
                "content": {
                    "token": token_as_string
                },
                "links": {
                    "files_list": hateoas_list_user_files(&req),
                    "request_upload_url": hateoas_request_upload_url(&req),
                },
                "status":200
            }))
            .unwrap(),
        ))
    } else {
        Err(VaultError::UserDoesNotExist.into())
    }
}

#[get("/users/")]
async fn new_user() -> Result<HttpResponse, Error> {
    unimplemented!()
}

#[post("/files/request-upload-url")]
async fn request_upload_url(
    req: HttpRequest,
    payload: web::Json<RequestUploadUrlRequestBody>,
    s3_fs: web::Data<Arc<file_server::FileServer>>,
) -> Result<HttpResponse, Error> {
    let filename = &payload.filename;

    if filename.is_empty() {
        return Err(VaultError::FieldRequired {
            field: "username".into(),
        }
        .into());
    };

    let exts = req.extensions();
    let username = &exts.get::<Username>().unwrap().0;

    let (upload_url, retrieve_url, form_data) = s3_fs
        .get_post_policy_data(username, filename)
        .map_err(VaultError::UnexpectedError)?;

    Ok(HttpResponse::Ok().json(
        serde_json::from_value::<RequestUploadUrlResponse>(json!({
            "links": {
                "retrieve_url": {
                    "href": retrieve_url,
                    "rel": "file",
                },
                "upload_url": {
                    "form_data": form_data,
                    "href": upload_url,
                    "rel": "file",
                }
            },
            "status":200
        }))
        .unwrap(),
    ))
}

#[get("/files/")]
async fn list_user_files(
    req: HttpRequest,
    s3_fs: web::Data<Arc<file_server::FileServer>>,
) -> Result<HttpResponse, Error> {
    let exts = req.extensions();
    let username = &exts.get::<Username>().unwrap().0;

    let files = s3_fs
        .get_files_list(username)
        .await
        .map_err(VaultError::S3Error)?;

    Ok(HttpResponse::Ok().json(
        serde_json::from_value::<RetrieveListOfUserFilesResponse>(json!({
            "content": files.iter().map(|f| {
                json!({
                    "content": f,
                    "links": {
                        "delete": hateoas_file_delete(&req, &f.filename),
                        "read": hateoas_file_read(f),
                    },
                    "status":200
                })
            }).collect::<Vec<serde_json::value::Value>>(),
            "status":200,
        }))
        .unwrap(),
    ))
}

#[derive(Deserialize)]
pub struct RemoveFileQueryFields {
    // XXX this is optional, but it shouldn't be. Maybe make it part of the URI?
    filename: Option<String>,
}

#[delete("/files/")]
async fn remove_file(
    req: HttpRequest,
    s3_fs: web::Data<Arc<file_server::FileServer>>,
    q: web::Query<RemoveFileQueryFields>,
) -> Result<HttpResponse, Error> {
    if q.filename.is_none() {
        return Err(VaultError::FieldRequired {
            field: "username".into(),
        }
        .into());
    };

    let exts = req.extensions();
    let username = &exts.get::<Username>().unwrap().0;

    s3_fs
        .remove_file(&username, q.filename.as_ref().unwrap())
        .await
        .map_err(VaultError::S3Error)?;

    Ok(HttpResponse::Ok().json(
        serde_json::from_value::<RemoveFileResponse>(json!({
            "status": 200,
        }))
        .unwrap(),
    ))
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

fn hateoas_auth_user_request_challenge(req: &HttpRequest) -> serde_json::Value {
    let url = req.url_for_static("auth_user_request_challenge").unwrap();
    json!({
        "href": url.as_str(),
        "rel": "auth"
    })
}

fn hateoas_list_user_files(req: &HttpRequest) -> serde_json::Value {
    let url = req.url_for_static("list_user_files").unwrap();
    json!({
        "href": url.as_str(),
        "rel": "file"
    })
}

fn hateoas_request_upload_url(req: &HttpRequest) -> serde_json::Value {
    let url = req.url_for_static("request_upload_url").unwrap();
    json!({
        "href": url.as_str(),
        "rel": "file"
    })
}

fn hateoas_file_read(f: &RetrieveListOfUserFilesResponseContentItemContent) -> serde_json::Value {
    json!({
        "href": &f.url,
        "rel": "file"
    })
}

fn hateoas_file_delete(req: &HttpRequest, filename: &str) -> serde_json::Value {
    let url = req.url_for_static("remove_file").unwrap();
    json!({
        "href": format!("{}?filename={}", url.as_str(), filename),
        "rel": "file"
    })
}

/// 404 handler
async fn p404() -> Result<&'static str, Error> {
    Err(VaultError::NotFound.into())
}

/// Not allowed handler
async fn method_not_allowed() -> Result<&'static str, Error> {
    Err(VaultError::MethodNotAllowed.into())
}

pub fn config_handlers(cfg: &mut web::ServiceConfig) {
    // Setup authentication middleware
    let auth = HttpAuthentication::bearer(auth_validator);

    cfg.service(handlers::favicon)
        .service(handlers::index)
        .service(handlers::auth_user_request_challenge)
        .service(
            web::resource("/auth/answer-challenge")
                .route(web::post().to(handlers::auth_user_answer_challenge))
                .name("auth_user_answer_challenge")
                .data(web::JsonConfig::default().limit(512)),
        )
        .service(
            web::scope("")
                .wrap(auth)
                .service(handlers::new_user)
                .service(handlers::request_upload_url)
                .service(handlers::list_user_files)
                .service(handlers::remove_file)
                .default_service(
                    // 404 for GET request
                    web::resource("/a/b")
                        .route(web::get().to(handlers::p404))
                        // all requests that are not `GET`
                        .route(
                            web::route()
                                .guard(guard::Not(guard::Get()))
                                .to(handlers::method_not_allowed),
                        ),
                ),
        );
}
