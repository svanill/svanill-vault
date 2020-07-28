


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnswerUserChallengeResponse {
    #[serde(rename = "status")]
    pub status: i32,
    #[serde(rename = "content")]
    pub content: super::AnswerUserChallengeResponseContent,
    #[serde(rename = "links")]
    pub links: super::AnswerUserChallengeResponseLinks,
}

impl AnswerUserChallengeResponse {
    pub fn new(status: i32, content: super::AnswerUserChallengeResponseContent, links: super::AnswerUserChallengeResponseLinks) -> AnswerUserChallengeResponse {
        AnswerUserChallengeResponse {
            status,
            content,
            links,
        }
    }
}


