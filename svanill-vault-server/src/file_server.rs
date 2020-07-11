use crate::models::RetrieveListOfUserFilesResponseContentItemContent;
use futures::future::try_join_all;
use rusoto_core::request::TlsError;
use rusoto_core::{HttpClient, Region, RusotoError};
use rusoto_credential::{AwsCredentials, ChainProvider, CredentialsError, ProvideAwsCredentials};
use rusoto_s3::util::{PreSignedRequest, PreSignedRequestOption};
use rusoto_s3::{
    GetObjectRequest, HeadObjectError, HeadObjectRequest, ListObjectsV2Error, ListObjectsV2Request,
    S3Client, S3,
};
use std::default::Default;
use thiserror::Error;

type FileDTO = RetrieveListOfUserFilesResponseContentItemContent;

#[derive(Error, Debug)]
pub enum FileServerError {
    #[error("cannot retrieve object metadata")]
    CannotRetrieveMetadata(#[from] RusotoError<HeadObjectError>),
    #[error("cannot retrieve files list")]
    CannotRetrieveFilesList(#[from] RusotoError<ListObjectsV2Error>),
    #[error("TLS error")]
    TlsError(#[from] TlsError),
    #[error("failed to obtain S3 credentials")]
    CredentialsError(#[from] CredentialsError),
}

pub struct FileServer {
    region: Region,
    bucket: String,
    client: S3Client,
    credentials: AwsCredentials,
    presigned_url_timeout: std::time::Duration,
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

    pub async fn get_files_list(&self, username: &str) -> Result<FilesList, FileServerError> {
        let list_request = ListObjectsV2Request {
            bucket: self.bucket.clone(),
            prefix: Some(username.to_string()),
            ..Default::default()
        };

        let s3_objects = self.client.list_objects_v2(list_request).await?;

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

        Ok(FilesList { files })
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
}

#[derive(Debug)]
pub struct FilesList {
    pub files: Vec<FileDTO>,
}
