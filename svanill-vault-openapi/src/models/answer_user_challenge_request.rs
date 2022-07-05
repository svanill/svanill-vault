#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AnswerUserChallengeRequest {
    /// the username of the user you claim to be
    #[serde(rename = "username")]
    pub username: String,
    /// the answer to the challenge
    #[serde(rename = "answer")]
    pub answer: String,
}

impl AnswerUserChallengeRequest {
    pub fn new(username: String, answer: String) -> AnswerUserChallengeRequest {
        AnswerUserChallengeRequest { username, answer }
    }
}
