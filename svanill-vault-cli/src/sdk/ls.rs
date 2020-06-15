use crate::config::Config;
use crate::models::{
    RetrieveListOfUserFilesResponse, RetrieveListOfUserFilesResponseContentItemContent,
};
use crate::sdk::response_error::SdkError;

pub fn ls(
    conf: &Config,
) -> Result<Vec<RetrieveListOfUserFilesResponseContentItemContent>, SdkError> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/files/", conf.base_url);
    let res = client.get(&url).bearer_auth(&conf.token).send()?;

    let status = res.status();
    let content = res.text()?;

    if status.is_success() {
        let opt_entity: Option<RetrieveListOfUserFilesResponse> =
            serde_json::from_str(&content).ok();

        if let Some(entity) = opt_entity {
            return Ok(entity.content.into_iter().map(|c| c.content).collect());
        }
    };

    vault_error!(status, content)
}
