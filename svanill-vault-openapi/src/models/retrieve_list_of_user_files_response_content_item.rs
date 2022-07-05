#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RetrieveListOfUserFilesResponseContentItem {
    #[serde(rename = "links")]
    pub links: Box<super::RetrieveListOfUserFilesResponseContentItemLinks>,
    #[serde(rename = "content")]
    pub content: Box<super::RetrieveListOfUserFilesResponseContentItemContent>,
}

impl RetrieveListOfUserFilesResponseContentItem {
    pub fn new(
        links: super::RetrieveListOfUserFilesResponseContentItemLinks,
        content: super::RetrieveListOfUserFilesResponseContentItemContent,
    ) -> RetrieveListOfUserFilesResponseContentItem {
        RetrieveListOfUserFilesResponseContentItem {
            links: Box::new(links),
            content: Box::new(content),
        }
    }
}
