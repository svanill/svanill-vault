


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HateoasRequestAuthChallenge {
    #[serde(rename = "href")]
    pub href: String,
    #[serde(rename = "rel")]
    pub rel: String,
}

impl HateoasRequestAuthChallenge {
    pub fn new(href: String, rel: String) -> HateoasRequestAuthChallenge {
        HateoasRequestAuthChallenge {
            href,
            rel,
        }
    }
}


