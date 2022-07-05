#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AnswerUserChallengeResponseContent {
    #[serde(rename = "token")]
    pub token: String,
}

impl AnswerUserChallengeResponseContent {
    pub fn new(token: String) -> AnswerUserChallengeResponseContent {
        AnswerUserChallengeResponseContent { token }
    }
}
