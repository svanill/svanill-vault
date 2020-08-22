


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnswerUserChallengeResponseContent {
    #[serde(rename = "token")]
    pub token: String,
}

impl AnswerUserChallengeResponseContent {
    pub fn new(token: String) -> AnswerUserChallengeResponseContent {
        AnswerUserChallengeResponseContent {
            token,
        }
    }
}


