use super::Config;
use crate::models::{
    RetrieveListOfUserFilesResponse, RetrieveListOfUserFilesResponseContentItemContent,
};
use crate::sdk::response_error::{ResponseError, SdkError};

pub fn ls(
    conf: Config,
) -> Result<Vec<RetrieveListOfUserFilesResponseContentItemContent>, SdkError> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/files/", conf.host);
    let res = client.get(&url).bearer_auth(conf.token).send()?;

    let status = res.status();
    let content = res.text()?;

    if status.is_success() {
        let opt_entity: Option<RetrieveListOfUserFilesResponse> =
            serde_json::from_str(&content).ok();

        if let Some(entity) = opt_entity {
            return Ok(entity.content.into_iter().map(|c| c.content).collect());
        }
    };

    match serde_json::from_str::<ResponseError>(&content) {
        Ok(parsed_err) => Err(parsed_err.into()),
        Err(_) => Err(SdkError::UnexpectedError {
            status: status.as_u16().into(),
        }),
    }
}
