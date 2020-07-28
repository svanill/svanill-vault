


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RequestUploadUrlRequestBody {
    /// the name of the file to upload
    #[serde(rename = "filename")]
    pub filename: String,
}

impl RequestUploadUrlRequestBody {
    pub fn new(filename: String) -> RequestUploadUrlRequestBody {
        RequestUploadUrlRequestBody {
            filename,
        }
    }
}


