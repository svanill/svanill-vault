#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AnswerUserChallengeResponse {
    #[serde(rename = "status")]
    pub status: i32,
    #[serde(rename = "content")]
    pub content: Box<super::AnswerUserChallengeResponseContent>,
    #[serde(rename = "links")]
    pub links: Box<super::AnswerUserChallengeResponseLinks>,
}

impl AnswerUserChallengeResponse {
    pub fn new(
        status: i32,
        content: super::AnswerUserChallengeResponseContent,
        links: super::AnswerUserChallengeResponseLinks,
    ) -> AnswerUserChallengeResponse {
        AnswerUserChallengeResponse {
            status,
            content: Box::new(content),
            links: Box::new(links),
        }
    }
}
