use crate::config::Config;
use crate::sdk::response_error::SdkError;
use svanill_vault_openapi::{
    AnswerUserChallengeRequest, AnswerUserChallengeResponse, AskForTheChallengeResponse,
};

pub fn request_challenge(conf: &Config) -> Result<String, SdkError> {
    let client = reqwest::blocking::Client::new();
    let url = format!(
        "{}/auth/request-challenge?username={}",
        conf.base_url, conf.username
    );
    let res = client.get(url).send()?;

    let status = res.status();
    let content = res.text()?;

    if status.is_success() {
        let opt_entity: Option<AskForTheChallengeResponse> = serde_json::from_str(&content).ok();

        if let Some(entity) = opt_entity {
            return Ok(entity.content.challenge);
        }
    };

    vault_error!(status, content)
}

pub fn answer_challenge(conf: &Config, answer: &str) -> Result<String, SdkError> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/auth/answer-challenge", conf.base_url);
    let res = client
        .post(url)
        .json(&AnswerUserChallengeRequest::new(
            conf.username.to_owned(),
            answer.to_owned(),
        ))
        .send()?;

    let status = res.status();
    let content = res.text()?;

    if status.is_success() {
        let opt_entity: Option<AnswerUserChallengeResponse> = serde_json::from_str(&content).ok();

        if let Some(entity) = opt_entity {
            return Ok(entity.content.token);
        }
    };

    vault_error!(status, content)
}
