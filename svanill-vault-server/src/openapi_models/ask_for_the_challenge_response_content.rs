


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AskForTheChallengeResponseContent {
    #[serde(rename = "challenge")]
    pub challenge: String,
}

impl AskForTheChallengeResponseContent {
    pub fn new(challenge: String) -> AskForTheChallengeResponseContent {
        AskForTheChallengeResponseContent {
            challenge,
        }
    }
}


