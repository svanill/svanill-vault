#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AskForTheChallengeResponseContent {
    #[serde(rename = "challenge")]
    pub challenge: String,
}

impl AskForTheChallengeResponseContent {
    pub fn new(challenge: String) -> AskForTheChallengeResponseContent {
        AskForTheChallengeResponseContent { challenge }
    }
}
