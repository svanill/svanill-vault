#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RequestUploadUrlResponseLinks {
    #[serde(rename = "upload_url")]
    pub upload_url: Box<super::HateoasFileUploadUrl>,
    #[serde(rename = "retrieve_url")]
    pub retrieve_url: Box<super::HateoasFileRetrieveUrl>,
}

impl RequestUploadUrlResponseLinks {
    pub fn new(
        upload_url: super::HateoasFileUploadUrl,
        retrieve_url: super::HateoasFileRetrieveUrl,
    ) -> RequestUploadUrlResponseLinks {
        RequestUploadUrlResponseLinks {
            upload_url: Box::new(upload_url),
            retrieve_url: Box::new(retrieve_url),
        }
    }
}
