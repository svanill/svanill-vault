use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub version: f32,
    pub base_url: String,
    pub username: String,
    pub challenges: HashMap<String, String>,
    #[serde(skip)]
    pub token: String,
}

/// `Config` implements `Default`
impl ::std::default::Default for Config {
    fn default() -> Self {
        Self {
            version: 0.1,
            base_url: "https://api.svanill.com".into(),
            username: "".into(),
            token: "".into(),
            challenges: HashMap::new(),
        }
    }
}
