#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RetrieveListOfUserFilesResponseContentItemLinks {
    #[serde(rename = "read")]
    pub read: Box<super::HateoasFileRead>,
    #[serde(rename = "delete")]
    pub delete: Box<super::HateoasFileDelete>,
}

impl RetrieveListOfUserFilesResponseContentItemLinks {
    pub fn new(
        read: super::HateoasFileRead,
        delete: super::HateoasFileDelete,
    ) -> RetrieveListOfUserFilesResponseContentItemLinks {
        RetrieveListOfUserFilesResponseContentItemLinks {
            read: Box::new(read),
            delete: Box::new(delete),
        }
    }
}
