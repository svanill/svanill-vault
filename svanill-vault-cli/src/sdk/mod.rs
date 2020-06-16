macro_rules! vault_error {
    ($status:ident, $content:ident) => {
        match serde_json::from_str::<crate::sdk::response_error::ResponseError>(&$content) {
            Ok(parsed_err) => Err(parsed_err.into()),
            Err(_) => Err(crate::sdk::response_error::SdkError::UnexpectedError {
                status: $status.as_u16().into(),
            }),
        }
    };
}

mod ls;
pub use ls::ls;
mod auth;
pub use auth::answer_challenge;
pub use auth::request_challenge;
mod files;
pub use files::retrieve;
mod response_error;
