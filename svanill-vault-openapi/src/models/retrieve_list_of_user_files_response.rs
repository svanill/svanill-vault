#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RetrieveListOfUserFilesResponse {
    #[serde(rename = "status")]
    pub status: i32,
    #[serde(rename = "content")]
    pub content: Vec<super::RetrieveListOfUserFilesResponseContentItem>,
}

impl RetrieveListOfUserFilesResponse {
    pub fn new(
        status: i32,
        content: Vec<super::RetrieveListOfUserFilesResponseContentItem>,
    ) -> RetrieveListOfUserFilesResponse {
        RetrieveListOfUserFilesResponse { status, content }
    }
}
