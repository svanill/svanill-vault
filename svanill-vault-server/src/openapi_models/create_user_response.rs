


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreateUserResponse {
    #[serde(rename = "status")]
    pub status: i32,
    #[serde(rename = "content")]
    pub content: super::CreateUserResponseContent,
    #[serde(rename = "links")]
    pub links: super::CreateUserResponseLinks,
}

impl CreateUserResponse {
    pub fn new(status: i32, content: super::CreateUserResponseContent, links: super::CreateUserResponseLinks) -> CreateUserResponse {
        CreateUserResponse {
            status,
            content,
            links,
        }
    }
}


