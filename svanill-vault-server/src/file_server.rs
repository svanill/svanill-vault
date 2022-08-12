use crate::post_policy::PostPolicy;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::error::DeleteObjectError;
use aws_sdk_s3::error::HeadObjectError;
use aws_sdk_s3::error::ListObjectsV2Error;
use aws_sdk_s3::presigning::config::PresigningConfig;
use aws_sdk_s3::presigning::request::PresignedRequest;
use aws_smithy_client::SdkError;
use aws_smithy_types::date_time::DateTime;
use aws_smithy_types::{timeout, tristate::TriState};
use aws_types::credentials::CredentialsError;
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
        region_provider: RegionProviderChain,
        bucket: String,
        maybe_endpoint: Option<aws_sdk_s3::Endpoint>,
        presigned_url_timeout: std::time::Duration,
    ) -> Result<FileServer, FileServerError> {
        let shared_config = aws_config::from_env().region(region_provider).load().await;
        let region: Region = shared_config
            .region()
            .ok_or(FileServerError::RegionNotConfigured)?
            .clone();

        let credentials = shared_config
            .credentials_provider()
            .ok_or(FileServerError::MissingCredentialsProviderError)?
            .as_ref()
            .provide_credentials()
            .await?;

        let api_timeouts = timeout::Api::new()
            .with_call_attempt_timeout(TriState::Set(std::time::Duration::from_millis(200)));
        let timeout_config = timeout::Config::new().with_api_timeouts(api_timeouts);

        let mut s3_config_builder =
            aws_sdk_s3::config::Builder::from(&shared_config).timeout_config(timeout_config);

        if let Some(endpoint) = maybe_endpoint {
            s3_config_builder = s3_config_builder.endpoint_resolver(endpoint);
        }

        let s3_config = s3_config_builder.build();

        let client = aws_sdk_s3::Client::from_conf(s3_config);

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
            .prefix(format!("users/{}/", username))
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
            .or(Err(FileServerError::CannotSignRequest))?;

        self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(conf)
            .await
            .or(Err(FileServerError::CannotSignRequest))
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
        let retrieve_url = format!("{}", retrieve_url_as_uri);

        let upload_url = format!(
            "{}://{}.{}{}",
            retrieve_url_as_uri.scheme_str().unwrap_or("https"),
            self.bucket,
            retrieve_url_as_uri
                .host()
                .expect("signed uri does not have hostname"),
            retrieve_url_as_uri
                .port()
                .map(|x| format!(":{}", x))
                .unwrap_or_default()
        );

        Ok((upload_url, retrieve_url, form_data))
    }
}

fn build_object_key(username: &str, filename: &str) -> String {
    format!("users/{}/{}", username, filename)
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
