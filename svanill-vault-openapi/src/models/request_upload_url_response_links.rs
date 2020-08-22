


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RequestUploadUrlResponseLinks {
    #[serde(rename = "upload_url")]
    pub upload_url: super::HateoasFileUploadUrl,
    #[serde(rename = "retrieve_url")]
    pub retrieve_url: super::HateoasFileRetrieveUrl,
}

impl RequestUploadUrlResponseLinks {
    pub fn new(upload_url: super::HateoasFileUploadUrl, retrieve_url: super::HateoasFileRetrieveUrl) -> RequestUploadUrlResponseLinks {
        RequestUploadUrlResponseLinks {
            upload_url,
            retrieve_url,
        }
    }
}


