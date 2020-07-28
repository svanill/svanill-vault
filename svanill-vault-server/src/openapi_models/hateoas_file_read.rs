


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HateoasFileRead {
    #[serde(rename = "rel")]
    pub rel: String,
    #[serde(rename = "href")]
    pub href: String,
}

impl HateoasFileRead {
    pub fn new(rel: String, href: String) -> HateoasFileRead {
        HateoasFileRead {
            rel,
            href,
        }
    }
}


