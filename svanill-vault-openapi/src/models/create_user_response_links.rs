


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CreateUserResponseLinks {
    #[serde(rename = "files_list")]
    pub files_list: super::HateoasFilesList,
    #[serde(rename = "request_upload_url")]
    pub request_upload_url: super::HateoasRequestUploadUrl,
}

impl CreateUserResponseLinks {
    pub fn new(files_list: super::HateoasFilesList, request_upload_url: super::HateoasRequestUploadUrl) -> CreateUserResponseLinks {
        CreateUserResponseLinks {
            files_list,
            request_upload_url,
        }
    }
}


