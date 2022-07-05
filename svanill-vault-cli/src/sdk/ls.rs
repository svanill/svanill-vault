use crate::config::Config;
use crate::sdk::response_error::SdkError;
use svanill_vault_openapi::{
    RetrieveListOfUserFilesResponse, RetrieveListOfUserFilesResponseContentItemContent,
};

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
            return Ok(entity
                .content
                .into_iter()
                .map(|mut c| {
                    c.content.filename = c
                        .content
                        .filename
                        .trim_start_matches(&format!("users/{}/", &conf.username))
                        .to_owned();
                    *c.content
                })
                // ignore keys containing `/` (not pushed by svanill-cli)
                .filter(|c| !c.filename.contains('/'))
                .collect());
        }
    };

    vault_error!(status, content)
}
