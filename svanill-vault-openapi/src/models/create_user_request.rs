#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CreateUserRequest {
    /// one or more ascii letters or numbers, underscores and hyphens. Must start with letters or numbers.
    #[serde(rename = "username")]
    pub username: String,
    /// a public blob of text that will be presented to anyone that wants to log as this user
    #[serde(rename = "challenge")]
    pub challenge: String,
    /// a private blob of text that has to be provided during authorization in response to the challenge
    #[serde(rename = "answer")]
    pub answer: String,
}

impl CreateUserRequest {
    pub fn new(username: String, challenge: String, answer: String) -> CreateUserRequest {
        CreateUserRequest {
            username,
            challenge,
            answer,
        }
    }
}
