#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RequestUploadUrlResponse {
    #[serde(rename = "status")]
    pub status: i32,
    #[serde(rename = "links")]
    pub links: Box<super::RequestUploadUrlResponseLinks>,
}

impl RequestUploadUrlResponse {
    pub fn new(
        status: i32,
        links: super::RequestUploadUrlResponseLinks,
    ) -> RequestUploadUrlResponse {
        RequestUploadUrlResponse {
            status,
            links: Box::new(links),
        }
    }
}
