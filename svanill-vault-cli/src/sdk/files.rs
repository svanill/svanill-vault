use crate::sdk::response_error::SdkError;

pub fn retrieve(url: &str) -> Result<Vec<u8>, SdkError> {
    let client = reqwest::blocking::Client::new();
    let res = client.get(url).send()?;
    match res.bytes() {
        Err(x) => Err(x.into()),
        Ok(x) => Ok(x.to_vec()),
    }
}
