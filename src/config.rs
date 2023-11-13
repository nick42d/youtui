use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::get_config_dir;

const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ApiKey {
    raw_text: String,
    auth_type: AuthType,
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
            Ok(toml::from_str(&config_file).unwrap())
        } else {
            Ok(Self::default())
        }
    }
    pub fn get_auth_type(&self) -> AuthType {
        self.auth_type
    }
}

impl ApiKey {
    pub fn new(raw_text: impl Into<String>, auth_type: AuthType) -> Self {
        let raw_text = raw_text.into();
        Self {
            raw_text,
            auth_type,
        }
    }
}
