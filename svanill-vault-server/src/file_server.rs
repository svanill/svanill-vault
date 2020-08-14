use crate::openapi_models::RetrieveListOfUserFilesResponseContentItemContent;
use crate::rusoto_extra::PostPolicy;
use chrono::Utc;
use futures::future::try_join_all;
use rusoto_core::request::TlsError;
use rusoto_core::{HttpClient, Region, RusotoError};
use rusoto_credential::{AwsCredentials, ChainProvider, CredentialsError, ProvideAwsCredentials};
use rusoto_s3::util::{PreSignedRequest, PreSignedRequestOption};
use rusoto_s3::{
    DeleteObjectError, DeleteObjectRequest, GetObjectRequest, HeadObjectError, HeadObjectRequest,
    ListObjectsV2Request, S3Client, S3,
};
use std::default::Default;
use std::{collections::HashMap, ops::Add};
use thiserror::Error;

type FileDTO = RetrieveListOfUserFilesResponseContentItemContent;

#[derive(Error, Debug)]
pub enum FileServerError {
    #[error("cannot retrieve object metadata")]
    CannotRetrieveMetadata(#[from] RusotoError<HeadObjectError>),
    #[error("cannot retrieve files list")]
    CannotRetrieveFilesList(String),
    #[error("TLS error")]
    TlsError(#[from] TlsError),
    #[error("failed to obtain S3 credentials")]
    CredentialsError(#[from] CredentialsError),
    #[error("cannot delete file")]
    CannotDelete(#[from] RusotoError<DeleteObjectError>),
    #[error("cannot generate policy data form")]
    PolicyDataError(String),
}

pub struct FileServer {
    pub region: Region,
    pub bucket: String,
    pub client: S3Client,
    pub credentials: AwsCredentials,
    pub presigned_url_timeout: std::time::Duration,
}

impl FileServer {
    pub async fn new(
        region: Region,
        bucket: String,
        presigned_url_timeout: std::time::Duration,
    ) -> Result<FileServer, FileServerError> {
        let mut chain = ChainProvider::new();
        chain.set_timeout(std::time::Duration::from_millis(200));

        let credentials = chain.credentials().await?;

        let client = S3Client::new_with(HttpClient::new()?, chain, region.clone());

        Ok(FileServer {
            region,
            bucket,
            client,
            credentials,
            presigned_url_timeout,
        })
    }

    pub async fn get_files_list(&self, username: &str) -> Result<Vec<FileDTO>, FileServerError> {
        let list_request = ListObjectsV2Request {
            bucket: self.bucket.clone(),
            prefix: Some(format!("users/{}/", username)),
            ..Default::default()
        };

        let s3_objects = self
            .client
            .list_objects_v2(list_request)
            .await
            .map_err(|e| FileServerError::CannotRetrieveFilesList(e.to_string()))?;

        let files: Vec<FileDTO> = try_join_all(
            s3_objects
                .contents
                .unwrap_or_default()
                .into_iter()
                .filter(|x| x.key.is_some())
                .map(|obj| async {
                    let key = obj.key.unwrap(); // always ok, we filtered for Some(key) only
                    let size = obj.size.unwrap_or_default();

                    let etag = if let Some(etag) = obj.e_tag {
                        etag
                    } else {
                        // e.g. etag may be missing, e.g. minio without 'erasure' option
                        let head_req = HeadObjectRequest {
                            bucket: self.bucket.clone(),
                            key: key.clone(),
                            ..Default::default()
                        };

                        self.client
                            .head_object(head_req)
                            .await?
                            .e_tag
                            .unwrap_or_default()
                    };

                    let url = self.get_presigned_retrieve_url(key.clone());

                    Ok::<FileDTO, FileServerError>(FileDTO {
                        filename: key,
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

        let delete_req = DeleteObjectRequest {
            bucket: self.bucket.clone(),
            key: key.to_string(),
            ..Default::default()
        };

        self.client.delete_object(delete_req).await?;
        Ok(())
    }

    pub fn get_presigned_retrieve_url(&self, key: String) -> String {
        GetObjectRequest {
            bucket: self.bucket.clone(),
            key,
            ..Default::default()
        }
        .get_presigned_url(
            &self.region,
            &self.credentials,
            &PreSignedRequestOption {
                expires_in: self.presigned_url_timeout,
            },
        )
    }

    pub fn get_post_policy_data(
        &self,
        username: &str,
        filename: &str,
    ) -> Result<(String, String, HashMap<String, String>), FileServerError> {
        let bytes_range_min = 10;
        let bytes_range_max = 1_048_576;

        let key = build_object_key(username, filename);

        let expiration_date = Utc::now()
            .add(chrono::Duration::from_std(self.presigned_url_timeout).expect("time overflow"));

        let (upload_url, form_data) = PostPolicy::default()
            .set_bucket_name(&self.bucket)
            .set_region(&self.region)
            .set_access_key_id(&self.credentials.aws_access_key_id())
            .set_secret_access_key(&self.credentials.aws_secret_access_key())
            .set_key(&key)
            .set_content_length_range(bytes_range_min, bytes_range_max)
            .set_expiration(expiration_date)
            .build_form_data()
            .map_err(FileServerError::PolicyDataError)?;

        let retrieve_url = self.get_presigned_retrieve_url(key);

        Ok((upload_url, retrieve_url, form_data))
    }
}

fn build_object_key(username: &str, filename: &str) -> String {
    format!("users/{}/{}", username, filename)
}

#[cfg(test)]
mod tests {
    use super::build_object_key;

    #[test]
    fn can_format_an_object_key() {
        assert_eq!("users/foo/bar", build_object_key("foo", "bar"));
    }
}
