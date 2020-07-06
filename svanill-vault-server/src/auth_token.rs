use ring::{hmac, rand::SecureRandom};
use std::fmt;
pub struct AuthToken(String);

impl AuthToken {
    pub fn new(crypto_key: &ring::hmac::Key) -> AuthToken {
        let token = generate_token();
        let tag = hmac::sign(&crypto_key, &token);
        let signed_token = format!("{}{}", hex::encode(tag), hex::encode(token.as_ref()));
        AuthToken(signed_token)
    }
}

impl fmt::Display for AuthToken {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

const TOKEN_LENGTH: usize = 32;
fn generate_token() -> [u8; TOKEN_LENGTH] {
    let mut token = [0u8; TOKEN_LENGTH];
    ring::rand::SystemRandom::new().fill(&mut token).unwrap();
    token
}
