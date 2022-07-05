#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct HateoasFileUploadUrl {
    #[serde(rename = "href")]
    pub href: String,
    #[serde(rename = "rel")]
    pub rel: String,
    #[serde(rename = "form_data")]
    pub form_data: ::std::collections::HashMap<String, String>,
}

impl HateoasFileUploadUrl {
    pub fn new(
        href: String,
        rel: String,
        form_data: ::std::collections::HashMap<String, String>,
    ) -> HateoasFileUploadUrl {
        HateoasFileUploadUrl {
            href,
            rel,
            form_data,
        }
    }
}
