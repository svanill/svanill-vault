


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HateoasFilesList {
    #[serde(rename = "href")]
    pub href: String,
    #[serde(rename = "rel")]
    pub rel: String,
}

impl HateoasFilesList {
    pub fn new(href: String, rel: String) -> HateoasFilesList {
        HateoasFilesList {
            href,
            rel,
        }
    }
}


