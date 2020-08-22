


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AskForTheChallengeResponseLinks {
    #[serde(rename = "answer_auth_challenge")]
    pub answer_auth_challenge: super::HateoasAnswerAuthChallenge,
    #[serde(rename = "create_user")]
    pub create_user: super::HateoasCreateUser,
}

impl AskForTheChallengeResponseLinks {
    pub fn new(answer_auth_challenge: super::HateoasAnswerAuthChallenge, create_user: super::HateoasCreateUser) -> AskForTheChallengeResponseLinks {
        AskForTheChallengeResponseLinks {
            answer_auth_challenge,
            create_user,
        }
    }
}


