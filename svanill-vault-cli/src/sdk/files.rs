use crate::config::Config;
use crate::models::{
    HateoasFileUploadUrl, RequestUploadUrlRequestBody, RequestUploadUrlResponse,
    RequestUploadUrlResponseLinks,
};
use crate::sdk::response_error::SdkError;

pub fn retrieve(url: &str) -> Result<Vec<u8>, SdkError> {
    let client = reqwest::blocking::Client::new();
    let res = client.get(url).send()?;
    match res.bytes() {
        Err(x) => Err(x.into()),
        Ok(x) => Ok(x.to_vec()),
    }
}

pub fn request_upload_url(
    conf: &Config,
    filename: &str,
) -> Result<RequestUploadUrlResponseLinks, SdkError> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/files/request-upload-url", conf.base_url);
    let request_body = RequestUploadUrlRequestBody::new(filename.to_owned());

    let res = client
        .post(&url)
        .bearer_auth(&conf.token)
        .json(&request_body)
        .send()?;

    let status = res.status();
    let content = res.text()?;

    if status.is_success() {
        let opt_entity: Option<RequestUploadUrlResponse> = serde_json::from_str(&content).ok();

        if let Some(entity) = opt_entity {
            return Ok(entity.links);
        }
    };

    vault_error!(status, content)
}

pub fn upload(
    upload_info: HateoasFileUploadUrl,
    remote_name: String,
    content: String,
) -> Result<(), SdkError> {
    let client = reqwest::blocking::Client::new();

    let url = upload_info.href;
    let params = upload_info.form_data;
    let mut form = reqwest::blocking::multipart::Form::new();

    for (key, value) in params {
        form = form.text(key, value);
    }

    let content_part = reqwest::blocking::multipart::Part::text(content)
        .file_name(remote_name)
        .mime_str("text/plain")?;

    form = form.part("file", content_part);

    let res = client.post(&url).multipart(form).send()?;

    let status = res.status();
    let content = res.text()?;

    if status.is_success() {
        return Ok(());
    };

    vault_error!(status, content)
}
