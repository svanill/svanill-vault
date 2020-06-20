use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub code: i32,
    pub message: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResponseError {
    pub status: i32,
    pub error: ErrorDetails,
}
#[derive(Error, Debug)]
pub enum SdkError {
    #[error("Status: {status:?}, Code: {code:?}, Message: {message:?}")]
    ParsedError {
        status: i32,
        code: i32,
        message: String,
    },
    #[error("Status: {status:?}, Unexpected error")]
    UnexpectedError { status: i32 },
    #[error("Status: {status:?}, Content:\n{content}")]
    CannotParseError { status: i32, content: String },
    #[error("NetworkError")]
    NetworkError(#[from] reqwest::Error),
}

impl From<ResponseError> for SdkError {
    fn from(res_error: ResponseError) -> Self {
        SdkError::ParsedError {
            status: res_error.status,
            code: res_error.error.code,
            message: res_error.error.message,
        }
    }
}
