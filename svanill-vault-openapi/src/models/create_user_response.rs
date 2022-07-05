#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CreateUserResponse {
    #[serde(rename = "status")]
    pub status: i32,
    #[serde(rename = "content")]
    pub content: Box<super::CreateUserResponseContent>,
    #[serde(rename = "links")]
    pub links: Box<super::CreateUserResponseLinks>,
}

impl CreateUserResponse {
    pub fn new(
        status: i32,
        content: super::CreateUserResponseContent,
        links: super::CreateUserResponseLinks,
    ) -> CreateUserResponse {
        CreateUserResponse {
            status,
            content: Box::new(content),
            links: Box::new(links),
        }
    }
}
