#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HateoasFileRetrieveUrl {
    #[serde(rename = "href")]
    pub href: String,
    #[serde(rename = "rel")]
    pub rel: String,
}

impl HateoasFileRetrieveUrl {
    pub fn new(href: String, rel: String) -> HateoasFileRetrieveUrl {
        HateoasFileRetrieveUrl { href, rel }
    }
}
