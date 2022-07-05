#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DeleteUserResponseLinks {
    #[serde(rename = "create_user")]
    pub create_user: Box<super::HateoasCreateUser>,
}

impl DeleteUserResponseLinks {
    pub fn new(create_user: super::HateoasCreateUser) -> DeleteUserResponseLinks {
        DeleteUserResponseLinks {
            create_user: Box::new(create_user),
        }
    }
}
