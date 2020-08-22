


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AskForTheChallengeResponse {
    #[serde(rename = "status")]
    pub status: i32,
    #[serde(rename = "content")]
    pub content: super::AskForTheChallengeResponseContent,
    #[serde(rename = "links")]
    pub links: super::AskForTheChallengeResponseLinks,
}

impl AskForTheChallengeResponse {
    pub fn new(status: i32, content: super::AskForTheChallengeResponseContent, links: super::AskForTheChallengeResponseLinks) -> AskForTheChallengeResponse {
        AskForTheChallengeResponse {
            status,
            content,
            links,
        }
    }
}


