use actix_http::ResponseBuilder;
use actix_web::{error, http::header, http::StatusCode, HttpResponse};
use serde::ser::Serializer;
use std::fmt::{self, Display};
use thiserror::Error;

fn statuscode_to_u16<S>(x: &StatusCode, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_u16(x.as_u16())
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    error: ApiErrorDetail,
    #[serde(rename = "status", serialize_with = "statuscode_to_u16")]
    http_status: StatusCode,
}

#[derive(Serialize, Debug)]
pub struct ApiErrorDetail {
    code: u32,
    message: String,
}

impl Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(serde_json::to_string(self).unwrap().as_ref())
    }
}

impl ApiError {
    fn new(http_status: StatusCode, code: u32, message: String) -> Self {
        ApiError {
            http_status,
            error: ApiErrorDetail { code, message },
        }
    }
}

impl error::ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        ResponseBuilder::new(self.status_code())
            .set_header(header::CONTENT_TYPE, "application/json; charset=utf-8")
            .body(self.to_string())
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
    UserDoesNotExist,
    DatabaseError(#[from] diesel::result::Error),
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
            VaultError::UserDoesNotExist => ApiError::new(
                StatusCode::UNAUTHORIZED,
                1005,
                String::from("The user does not exist"),
            ),
            VaultError::DatabaseError(_) => ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                1021,
                String::from("Internal Server Error"),
            ),
        }
    }
}

impl error::ResponseError for VaultError {
    fn error_response(&self) -> HttpResponse {
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
