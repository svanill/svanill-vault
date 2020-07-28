


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetrieveListOfUserFilesResponseContentItemLinks {
    #[serde(rename = "read")]
    pub read: super::HateoasFileRead,
    #[serde(rename = "delete")]
    pub delete: super::HateoasFileDelete,
}

impl RetrieveListOfUserFilesResponseContentItemLinks {
    pub fn new(read: super::HateoasFileRead, delete: super::HateoasFileDelete) -> RetrieveListOfUserFilesResponseContentItemLinks {
        RetrieveListOfUserFilesResponseContentItemLinks {
            read,
            delete,
        }
    }
}


