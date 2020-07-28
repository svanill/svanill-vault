


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetrieveListOfUserFilesResponseContentItem {
    #[serde(rename = "links")]
    pub links: super::RetrieveListOfUserFilesResponseContentItemLinks,
    #[serde(rename = "content")]
    pub content: super::RetrieveListOfUserFilesResponseContentItemContent,
}

impl RetrieveListOfUserFilesResponseContentItem {
    pub fn new(links: super::RetrieveListOfUserFilesResponseContentItemLinks, content: super::RetrieveListOfUserFilesResponseContentItemContent) -> RetrieveListOfUserFilesResponseContentItem {
        RetrieveListOfUserFilesResponseContentItem {
            links,
            content,
        }
    }
}


