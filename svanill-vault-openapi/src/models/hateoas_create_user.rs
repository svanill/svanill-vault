#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HateoasCreateUser {
    #[serde(rename = "href")]
    pub href: String,
    #[serde(rename = "rel")]
    pub rel: String,
}

impl HateoasCreateUser {
    pub fn new(href: String, rel: String) -> HateoasCreateUser {
        HateoasCreateUser { href, rel }
    }
}
