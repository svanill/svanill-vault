


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HateoasRequestUploadUrl {
    #[serde(rename = "href")]
    pub href: String,
    #[serde(rename = "rel")]
    pub rel: String,
}

impl HateoasRequestUploadUrl {
    pub fn new(href: String, rel: String) -> HateoasRequestUploadUrl {
        HateoasRequestUploadUrl {
            href,
            rel,
        }
    }
}


