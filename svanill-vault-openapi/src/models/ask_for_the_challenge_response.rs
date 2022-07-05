#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AskForTheChallengeResponse {
    #[serde(rename = "status")]
    pub status: i32,
    #[serde(rename = "content")]
    pub content: Box<super::AskForTheChallengeResponseContent>,
    #[serde(rename = "links")]
    pub links: Box<super::AskForTheChallengeResponseLinks>,
}

impl AskForTheChallengeResponse {
    pub fn new(
        status: i32,
        content: super::AskForTheChallengeResponseContent,
        links: super::AskForTheChallengeResponseLinks,
    ) -> AskForTheChallengeResponse {
        AskForTheChallengeResponse {
            status,
            content: Box::new(content),
            links: Box::new(links),
        }
    }
}
