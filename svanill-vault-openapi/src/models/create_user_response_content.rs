#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CreateUserResponseContent {
    #[serde(rename = "challenge")]
    pub challenge: String,
    #[serde(rename = "token")]
    pub token: String,
}

impl CreateUserResponseContent {
    pub fn new(challenge: String, token: String) -> CreateUserResponseContent {
        CreateUserResponseContent { challenge, token }
    }
}
