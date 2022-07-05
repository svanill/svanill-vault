#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GetStartingEndpointsResponse {
    #[serde(rename = "status")]
    pub status: i32,
    #[serde(rename = "links")]
    pub links: Box<super::GetStartingEndpointsResponseLinks>,
}

impl GetStartingEndpointsResponse {
    pub fn new(
        status: i32,
        links: super::GetStartingEndpointsResponseLinks,
    ) -> GetStartingEndpointsResponse {
        GetStartingEndpointsResponse {
            status,
            links: Box::new(links),
        }
    }
}
