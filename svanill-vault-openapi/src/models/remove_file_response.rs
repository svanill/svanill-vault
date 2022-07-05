#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RemoveFileResponse {
    #[serde(rename = "status")]
    pub status: i32,
}

impl RemoveFileResponse {
    pub fn new(status: i32) -> RemoveFileResponse {
        RemoveFileResponse { status }
    }
}
