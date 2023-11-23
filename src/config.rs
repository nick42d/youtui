use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use ytmapi_rs::auth::{BrowserToken, OAuthToken};

use crate::get_config_dir;

const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Debug, Serialize, Deserialize)]
pub enum ApiKey {
    // XXX: These could actually take the appropriate tokens from the API, if that part of the interface is opened.
    // If that's the case we can do some additional parsing before we reach the app.
    // Currently OAuthToken is public but not BrowserToken
    OAuthToken(OAuthToken),
    BrowserToken(String),
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Config {
    auth_type: AuthType,
}

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub enum AuthType {
    OAuth,
    #[default]
    Browser,
}

impl Config {
    pub fn new() -> Result<Self> {
        let config_dir = get_config_dir()?;
        if let Ok(config_file) = std::fs::read_to_string(config_dir.join(CONFIG_FILE_NAME)) {
            Ok(toml::from_str(&config_file)?)
        } else {
            Ok(Self::default())
        }
    }
    pub fn get_auth_type(&self) -> AuthType {
        self.auth_type
    }
}
