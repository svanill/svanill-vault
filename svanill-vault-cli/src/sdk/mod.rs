mod ls;
pub use ls::ls;

mod response_error;

pub struct Config {
    pub host: String,
    pub username: String,
    pub token: String,
}
