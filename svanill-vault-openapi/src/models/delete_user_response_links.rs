


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DeleteUserResponseLinks {
    #[serde(rename = "create_user")]
    pub create_user: super::HateoasCreateUser,
}

impl DeleteUserResponseLinks {
    pub fn new(create_user: super::HateoasCreateUser) -> DeleteUserResponseLinks {
        DeleteUserResponseLinks {
            create_user,
        }
    }
}


