


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetrieveListOfUserFilesResponseContentItemContent {
    #[serde(rename = "checksum")]
    pub checksum: String,
    #[serde(rename = "filename")]
    pub filename: String,
    #[serde(rename = "size")]
    pub size: i32,
    #[serde(rename = "url")]
    pub url: String,
}

impl RetrieveListOfUserFilesResponseContentItemContent {
    pub fn new(checksum: String, filename: String, size: i32, url: String) -> RetrieveListOfUserFilesResponseContentItemContent {
        RetrieveListOfUserFilesResponseContentItemContent {
            checksum,
            filename,
            size,
            url,
        }
    }
}


