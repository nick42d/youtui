use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::get_config_dir;

const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Config {
    auth_type: AuthType,
}

#[derive(Default, Debug, Serialize, Deserialize)]
enum AuthType {
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
}
