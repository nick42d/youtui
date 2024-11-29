use crate::app::component::actionhandler::Action;
use crate::get_config_dir;
use crate::Result;
use clap::ValueEnum;
use keybinds::YoutuiKeymap;
use keybinds::YoutuiKeymapIR;
use keybinds::YoutuiModeNames;
use serde::{Deserialize, Serialize};
use ytmapi_rs::auth::OAuthToken;

const CONFIG_FILE_NAME: &str = "config.toml";

pub mod keybinds;

#[derive(Serialize, Deserialize)]
pub enum ApiKey {
    OAuthToken(OAuthToken),
    // BrowserToken takes the cookie, not the BrowserToken itself. This is because to obtain the
    // BrowserToken you must make a web request, and we want to obtain it as lazily as possible.
    BrowserToken(String),
}

impl std::fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiKey::OAuthToken(_) => write!(f, "OAuthToken(/* private fields */"),
            ApiKey::BrowserToken(_) => write!(f, "BrowserToken(/* private fields */"),
        }
    }
}

#[derive(ValueEnum, Copy, PartialEq, Clone, Default, Debug, Serialize, Deserialize)]
pub enum AuthType {
    #[value(name = "oauth")]
    OAuth,
    #[default]
    Browser,
}

#[derive(Debug, Default, PartialEq)]
pub struct Config {
    pub auth_type: AuthType,
    pub keybinds: YoutuiKeymap,
    pub mode_names: YoutuiModeNames,
}

#[derive(Default, Debug, Deserialize)]
#[serde(default)]
/// Intermediate representation of Config for serde.
pub struct ConfigIR {
    pub auth_type: AuthType,
    pub keybinds: YoutuiKeymapIR,
    pub mode_names: YoutuiModeNames,
}

impl TryFrom<ConfigIR> for Config {
    type Error = String;
    fn try_from(value: ConfigIR) -> std::result::Result<Self, Self::Error> {
        let ConfigIR {
            auth_type,
            keybinds,
            mode_names,
        } = value;
        Ok(Config {
            auth_type,
            keybinds: keybinds.try_into()?,
            mode_names,
        })
    }
}

impl Config {
    pub async fn new(debug: bool) -> Result<Self> {
        let config_dir = get_config_dir()?;
        let config_file_location = config_dir.join(CONFIG_FILE_NAME);
        if let Ok(config_file) = tokio::fs::read_to_string(&config_file_location).await {
            // NOTE: This happens before logging / app is initialised, so `eprintln!` is
            // used instead of `info!`
            if debug {
                println!(
                    "Loading config from {}",
                    config_file_location.to_string_lossy()
                );
            }
            let ir: ConfigIR = toml::from_str(&config_file)?;
            Ok(Config::try_from(ir).map_err(|e| crate::Error::Other(e))?)
        } else {
            if debug {
                println!(
                    "Config file not found in {}, using defaults",
                    config_file_location.to_string_lossy()
                );
            }
            Ok(Self::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{Config, ConfigIR, CONFIG_FILE_NAME},
        get_config_dir,
    };

    #[tokio::test]
    async fn test_deserialize_default_config_to_ir() {
        let config_dir = get_config_dir().unwrap();
        let config_file_location = config_dir.join(CONFIG_FILE_NAME);
        let config_file = tokio::fs::read_to_string(&config_file_location)
            .await
            .unwrap();
        toml::from_str::<ConfigIR>(&config_file).unwrap();
    }
    #[tokio::test]
    async fn test_convert_ir_to_config() {
        let config_dir = get_config_dir().unwrap();
        let config_file_location = config_dir.join(CONFIG_FILE_NAME);
        let config_file = tokio::fs::read_to_string(&config_file_location)
            .await
            .unwrap();
        let ir: ConfigIR = toml::from_str(&config_file).unwrap();
        Config::try_from(ir).unwrap();
    }
    #[tokio::test]
    async fn test_default_config_equals_deserialized_config() {
        let config_dir = get_config_dir().unwrap();
        let config_file_location = config_dir.join(CONFIG_FILE_NAME);
        let config_file = tokio::fs::read_to_string(&config_file_location)
            .await
            .unwrap();
        let ir: ConfigIR = toml::from_str(&config_file).unwrap();
        let output = Config::try_from(ir).unwrap();
        let expected = Config::default();
        assert_eq!(output, expected);
    }
}
