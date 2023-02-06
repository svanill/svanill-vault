use crate::post_policy::PostPolicy;
use aws_config::SdkConfig;
use aws_credential_types::provider::error::CredentialsError;
use aws_sdk_s3::config::Config as S3Config;
use aws_sdk_s3::error::DeleteObjectError;
use aws_sdk_s3::error::HeadObjectError;
use aws_sdk_s3::error::ListObjectsV2Error;
use aws_sdk_s3::presigning::config::PresigningConfig;
use aws_sdk_s3::presigning::request::PresignedRequest;
use aws_sdk_s3::types::SdkError;
use aws_smithy_types::date_time::DateTime;
use aws_types::region::Region;
use futures::future::try_join_all;
use std::collections::HashMap;
use std::default::Default;
use std::time::SystemTime;
use svanill_vault_openapi::RetrieveListOfUserFilesResponseContentItemContent;
use thiserror::Error;

type FileDTO = RetrieveListOfUserFilesResponseContentItemContent;

#[derive(Error, Debug)]
pub enum FileServerError {
    #[error("cannot retrieve object metadata")]
    CannotRetrieveMetadata(#[from] SdkError<HeadObjectError>),
    #[error("cannot retrieve files list")]
    CannotRetrieveFilesList(#[from] SdkError<ListObjectsV2Error>),
    #[error("failed to obtain S3 credentials")]
    CredentialsError(#[from] CredentialsError),
    #[error("missing credentials provider")]
    MissingCredentialsProviderError,
    #[error("cannot delete file")]
    CannotDelete(#[from] SdkError<DeleteObjectError>),
    #[error("cannot generate policy data form")]
    PolicyDataError(String),
    #[error("cannot configure S3 region")]
    RegionNotConfigured,
    #[error("cannot sign request")]
    CannotSignRequest,
}

pub struct FileServer {
    pub region: Region,
    pub bucket: String,
    pub client: aws_sdk_s3::Client,
    pub credentials: aws_sdk_s3::Credentials,
    pub presigned_url_timeout: std::time::Duration,
}

impl FileServer {
    pub async fn new(
        aws_sdk_conf: SdkConfig,
        aws_s3_conf: S3Config,
        bucket: String,
        presigned_url_timeout: std::time::Duration,
    ) -> Result<FileServer, FileServerError> {
        let region: Region = aws_sdk_conf
            .region()
            .ok_or(FileServerError::RegionNotConfigured)?
            .clone();

        let credentials = aws_sdk_conf
            .credentials_provider()
            .ok_or(FileServerError::MissingCredentialsProviderError)?
            .as_ref()
            .provide_credentials()
            .await?;

        let client = aws_sdk_s3::Client::from_conf(aws_s3_conf);

        Ok(FileServer {
            region,
            bucket,
            client,
            credentials,
            presigned_url_timeout,
        })
    }

    pub async fn get_files_list(&self, username: &str) -> Result<Vec<FileDTO>, FileServerError> {
        let s3_objects = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(format!("users/{username}/"))
            .send()
            .await
            .map_err(FileServerError::CannotRetrieveFilesList)?;

        let files: Vec<FileDTO> = try_join_all(
            s3_objects
                .contents
                .unwrap_or_default()
                .into_iter()
                .filter(|x| x.key.is_some())
                .map(|obj| async move {
                    let key = obj.key.unwrap(); // always ok, we filtered for Some(key) only
                    let size = obj.size;

                    let etag = if let Some(etag) = obj.e_tag {
                        etag
                    } else {
                        self.client
                            .head_object()
                            .bucket(&self.bucket)
                            .key(&key)
                            .send()
                            .await?
                            .e_tag
                            .unwrap_or_default()
                    };

                    let url = self.get_presigned_retrieve_url_as_string(&key).await?;

                    let (_, filename) = split_object_key(username, &key)
                        .expect("object key does not match user prefix");

                    Ok::<FileDTO, FileServerError>(FileDTO {
                        filename: filename.to_owned(),
                        checksum: etag,
                        size: size as i32,
                        url,
                    })
                }),
        )
        .await?;

        Ok(files)
    }

    pub async fn remove_file(&self, username: &str, filename: &str) -> Result<(), FileServerError> {
        let key = build_object_key(username, filename);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await?;

        Ok(())
    }

    async fn get_presigned_retrieve_url_as_string(
        &self,
        key: &str,
    ) -> Result<String, FileServerError> {
        let presigned_request = self.get_presigned_retrieve_url_as_req(key).await?;

        Ok(format!("{}", presigned_request.uri()))
    }

    async fn get_presigned_retrieve_url_as_req(
        &self,
        key: &str,
    ) -> Result<PresignedRequest, FileServerError> {
        let conf = PresigningConfig::expires_in(self.presigned_url_timeout)
            .map_err(|_| FileServerError::CannotSignRequest)?;

        self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(conf)
            .await
            .map_err(|_| FileServerError::CannotSignRequest)
    }

    pub async fn get_post_policy_data(
        &self,
        username: &str,
        filename: &str,
    ) -> Result<(String, String, HashMap<String, String>), FileServerError> {
        let bytes_range_min = 10;
        let bytes_range_max = 1_048_576;

        let key = build_object_key(username, filename);

        let expiration_date = DateTime::from(
            SystemTime::now()
                .checked_add(self.presigned_url_timeout)
                .expect("time overflow"),
        );

        let form_data = PostPolicy::default()
            .set_bucket_name(&self.bucket)
            .set_region(&self.region)
            .set_access_key_id(self.credentials.access_key_id())
            .set_secret_access_key(self.credentials.secret_access_key())
            .set_key(&key)
            .set_content_length_range(bytes_range_min, bytes_range_max)
            .set_expiration(&expiration_date)
            .build_form_data()
            .map_err(FileServerError::PolicyDataError)?;

        let retrieve_url_as_req = self.get_presigned_retrieve_url_as_req(&key).await?;
        let retrieve_url_as_uri = retrieve_url_as_req.uri();
        let retrieve_url = format!("{retrieve_url_as_uri}");

        let upload_url = format!(
            "{}://{}.{}{}",
            retrieve_url_as_uri.scheme_str().unwrap_or("https"),
            self.bucket,
            retrieve_url_as_uri
                .host()
                .expect("signed uri does not have hostname"),
            retrieve_url_as_uri
                .port()
                .map(|x| format!(":{x}"))
                .unwrap_or_default()
        );

        Ok((upload_url, retrieve_url, form_data))
    }
}

fn build_object_key(username: &str, filename: &str) -> String {
    format!("users/{username}/{filename}")
}

fn split_object_key<'a>(username: &str, key: &'a str) -> Option<(&'a str, &'a str)> {
    let prefix_len = "users/".len() + username.len() + "/".len();

    if prefix_len > key.len() {
        None
    } else {
        Some(key.split_at(prefix_len))
    }
}

#[cfg(test)]
mod tests {
    use super::build_object_key;

    #[test]
    fn can_format_an_object_key() {
        assert_eq!("users/foo/bar", build_object_key("foo", "bar"));
    }
}
