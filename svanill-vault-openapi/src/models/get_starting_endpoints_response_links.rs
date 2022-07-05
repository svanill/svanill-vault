#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GetStartingEndpointsResponseLinks {
    #[serde(rename = "create_user")]
    pub create_user: Box<super::HateoasCreateUser>,
    #[serde(rename = "request_auth_challenge")]
    pub request_auth_challenge: Box<super::HateoasRequestAuthChallenge>,
}

impl GetStartingEndpointsResponseLinks {
    pub fn new(
        create_user: super::HateoasCreateUser,
        request_auth_challenge: super::HateoasRequestAuthChallenge,
    ) -> GetStartingEndpointsResponseLinks {
        GetStartingEndpointsResponseLinks {
            create_user: Box::new(create_user),
            request_auth_challenge: Box::new(request_auth_challenge),
        }
    }
}
