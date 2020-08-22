


#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnswerUserChallengeResponseLinks {
    #[serde(rename = "files_list")]
    pub files_list: super::HateoasFilesList,
    #[serde(rename = "request_upload_url")]
    pub request_upload_url: super::HateoasRequestUploadUrl,
}

impl AnswerUserChallengeResponseLinks {
    pub fn new(files_list: super::HateoasFilesList, request_upload_url: super::HateoasRequestUploadUrl) -> AnswerUserChallengeResponseLinks {
        AnswerUserChallengeResponseLinks {
            files_list,
            request_upload_url,
        }
    }
}


