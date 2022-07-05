#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HateoasFileDelete {
    #[serde(rename = "rel")]
    pub rel: String,
    #[serde(rename = "href")]
    pub href: String,
}

impl HateoasFileDelete {
    pub fn new(rel: String, href: String) -> HateoasFileDelete {
        HateoasFileDelete { rel, href }
    }
}
