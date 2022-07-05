use crate::config::Config;
use crate::sdk::response_error::SdkError;
use md5::{Digest, Md5};
use std::{collections::HashMap, io::Read};
use svanill_vault_openapi::{
    HateoasFileUploadUrl, RemoveFileResponse, RequestUploadUrlRequestBody,
    RequestUploadUrlResponse, RequestUploadUrlResponseLinks,
};

pub fn retrieve(url: &str) -> Result<impl Read, SdkError> {
    let client = reqwest::blocking::Client::new();
    client.get(url).send().map_err(|e| e.into())
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
            return Ok(*entity.links);
        }
    };

    vault_error!(status, content)
}

fn md5sum(v: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(v);
    format!("{:x}", hasher.finalize())
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

    let checksum = md5sum(&content);
    let content_part = reqwest::blocking::multipart::Part::text(content)
        .file_name(remote_name)
        .mime_str("text/plain")?;

    form = form.part("file", content_part);

    let res = client.post(&url).multipart(form).send()?;
    let status = res.status();

    if status.is_success() {
        let etag = if res.headers().get("etag").is_none() {
            ""
        } else {
            res.headers()
                .get("etag")
                .unwrap()
                .to_str()
                .unwrap_or("")
                .trim_matches('"')
        };

        if etag == checksum {
            return Ok(());
        } else {
            return Err(SdkError::ChecksumMismatch {
                local: checksum,
                remote: etag.to_owned(),
            });
        }
    };

    let content = res.text()?;
    vault_error!(status, content)
}

pub fn delete(conf: &Config, filename: &str) -> Result<(), SdkError> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/files/", conf.base_url);

    // Right now the server expect the filename as JSON body,
    // in future it will expect it as query param.
    // We provide both.

    let mut request_body = HashMap::new();
    request_body.insert("filename", filename);

    let res = client
        .delete(&url)
        .bearer_auth(&conf.token)
        .query(&[("filename", filename)])
        .json(&request_body)
        .send()?;

    let status = res.status();
    let content = res.text()?;

    if status.is_success() {
        let opt_entity: Option<RemoveFileResponse> = serde_json::from_str(&content).ok();

        if opt_entity.is_some() {
            return Ok(());
        }
    };

    vault_error!(status, content)
}
