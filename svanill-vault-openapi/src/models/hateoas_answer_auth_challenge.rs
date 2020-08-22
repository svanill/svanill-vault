


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HateoasAnswerAuthChallenge {
    #[serde(rename = "rel")]
    pub rel: String,
    #[serde(rename = "href")]
    pub href: String,
}

impl HateoasAnswerAuthChallenge {
    pub fn new(rel: String, href: String) -> HateoasAnswerAuthChallenge {
        HateoasAnswerAuthChallenge {
            rel,
            href,
        }
    }
}


