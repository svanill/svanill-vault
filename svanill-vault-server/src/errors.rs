use crate::file_server::FileServerError;
use actix_http::ResponseBuilder;
use actix_web::{error, http::header, http::StatusCode};
use serde::Deserializer;
use serde::{ser::Serializer, Deserialize};
use std::fmt::{self, Display};
use thiserror::Error;

fn statuscode_to_u16<S>(x: &StatusCode, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_u16(x.as_u16())
}

pub fn u16_to_statuscode<'de, D>(deserializer: D) -> Result<StatusCode, D::Error>
where
    D: Deserializer<'de>,
{
    let v = u16::deserialize(deserializer)?;
    StatusCode::from_u16(v).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiError {
    pub error: ApiErrorDetail,
    #[serde(
        rename = "status",
        serialize_with = "statuscode_to_u16",
        deserialize_with = "u16_to_statuscode"
    )]
    pub http_status: StatusCode,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ApiErrorDetail {
    pub code: u32,
    pub message: String,
}

impl Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(serde_json::to_string(self).unwrap().as_ref())
    }
}

impl ApiError {
    pub fn new(http_status: StatusCode, code: u32, message: String) -> Self {
        ApiError {
            http_status,
            error: ApiErrorDetail { code, message },
        }
    }
}

impl error::ResponseError for ApiError {
    fn error_response(&self) -> actix_web::HttpResponse {
        ResponseBuilder::new(self.status_code())
            .insert_header((header::CONTENT_TYPE, "application/json; charset=utf-8"))
            .body(self.to_string())
            .into()
    }

    fn status_code(&self) -> StatusCode {
        self.http_status
    }
}

#[derive(Error, Debug)]
pub enum VaultError {
    NotFound,
    MethodNotAllowed,
    FieldRequired { field: String },
    GenericBadRequest(String),
    UserDoesNotExist,
    DatabaseError(#[from] diesel::result::Error),
    ChallengeMismatchError,
    S3Error(FileServerError),
    UnexpectedError(String),
    PolicyDataError(FileServerError),
}

impl From<&VaultError> for ApiError {
    fn from(error: &VaultError) -> Self {
        match error {
            VaultError::NotFound => ApiError::new(
                StatusCode::NOT_FOUND,
                StatusCode::NOT_FOUND.as_u16().into(),
                String::from("Not Found"),
            ),
            VaultError::MethodNotAllowed => ApiError::new(
                StatusCode::METHOD_NOT_ALLOWED,
                StatusCode::METHOD_NOT_ALLOWED.as_u16().into(),
                String::from("Method Not Allowed"),
            ),
            VaultError::FieldRequired { field } => ApiError::new(
                StatusCode::CONFLICT,
                1002,
                format!("This field is required: {}", field),
            ),
            VaultError::GenericBadRequest(msg) => {
                ApiError::new(StatusCode::BAD_REQUEST, 1024, msg.to_owned())
            }
            VaultError::UserDoesNotExist => ApiError::new(
                StatusCode::UNAUTHORIZED,
                1005,
                String::from("The user does not exist"),
            ),
            VaultError::ChallengeMismatchError => ApiError::new(
                StatusCode::UNAUTHORIZED,
                1006,
                String::from("The challenge does not match"),
            ),
            VaultError::DatabaseError(_) => ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                1021,
                String::from("Internal Server Error"),
            ),
            VaultError::S3Error(e) => {
                ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, 1022, e.to_string())
            }
            VaultError::UnexpectedError(_) => ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                1023,
                String::from("Internal Server Error"),
            ),
            VaultError::PolicyDataError(e) => {
                ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, 1025, e.to_string())
            }
        }
    }
}

impl error::ResponseError for VaultError {
    fn error_response(&self) -> actix_web::HttpResponse {
        let as_api_err: ApiError = self.into();
        as_api_err.error_response()
    }

    fn status_code(&self) -> StatusCode {
        let as_api_err: ApiError = self.into();
        as_api_err.status_code()
    }
}

impl Display for VaultError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let as_api_err: ApiError = self.into();
        as_api_err.fmt(formatter)
    }
}
