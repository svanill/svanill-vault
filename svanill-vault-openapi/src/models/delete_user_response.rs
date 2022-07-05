#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DeleteUserResponse {
    #[serde(rename = "status")]
    pub status: i32,
    #[serde(rename = "links")]
    pub links: Box<super::DeleteUserResponseLinks>,
}

impl DeleteUserResponse {
    pub fn new(status: i32, links: super::DeleteUserResponseLinks) -> DeleteUserResponse {
        DeleteUserResponse {
            status,
            links: Box::new(links),
        }
    }
}
