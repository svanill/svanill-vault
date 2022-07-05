#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AnswerUserChallengeResponseLinks {
    #[serde(rename = "files_list")]
    pub files_list: Box<super::HateoasFilesList>,
    #[serde(rename = "request_upload_url")]
    pub request_upload_url: Box<super::HateoasRequestUploadUrl>,
}

impl AnswerUserChallengeResponseLinks {
    pub fn new(
        files_list: super::HateoasFilesList,
        request_upload_url: super::HateoasRequestUploadUrl,
    ) -> AnswerUserChallengeResponseLinks {
        AnswerUserChallengeResponseLinks {
            files_list: Box::new(files_list),
            request_upload_url: Box::new(request_upload_url),
        }
    }
}
