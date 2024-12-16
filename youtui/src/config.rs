use crate::get_config_dir;
use anyhow::Result;
use clap::ValueEnum;
use keymap::YoutuiKeymap;
use keymap::YoutuiKeymapIR;
use keymap::YoutuiModeNamesIR;
use serde::{Deserialize, Serialize};
use ytmapi_rs::auth::OAuthToken;

const CONFIG_FILE_NAME: &str = "config.toml";

pub mod keymap;

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
}

#[derive(Default, Debug, Deserialize)]
#[serde(default)]
/// Intermediate representation of Config for serde.
pub struct ConfigIR {
    pub auth_type: AuthType,
    pub keybinds: YoutuiKeymapIR,
    pub mode_names: YoutuiModeNamesIR,
}

impl TryFrom<ConfigIR> for Config {
    type Error = anyhow::Error;
    fn try_from(value: ConfigIR) -> std::result::Result<Self, Self::Error> {
        let ConfigIR {
            auth_type,
            keybinds,
            mode_names,
        } = value;
        Ok(Config {
            auth_type,
            keybinds: YoutuiKeymap::try_from_stringy(keybinds, mode_names)?,
        })
    }
}

impl Config {
    pub async fn new(debug: bool) -> Result<Self> {
        let config_dir = get_config_dir()?;
        let config_file_location = config_dir.join(CONFIG_FILE_NAME);
        if let Ok(config_file) = tokio::fs::read_to_string(&config_file_location).await {
            // NOTE: This happens before logging / app is initialised, so `println!` is
            // used instead of `info!`
            if debug {
                println!(
                    "Loading config from {}",
                    config_file_location.to_string_lossy()
                );
            }
            let ir: ConfigIR = toml::from_str(&config_file)?;
            Ok(Config::try_from(ir)?)
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
    use crate::config::{keymap::YoutuiKeymap, Config, ConfigIR};
    use pretty_assertions::{assert_eq, assert_ne};

    async fn example_config_file() -> String {
        tokio::fs::read_to_string("./config/config.toml")
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_deserialize_default_config_to_ir() {
        let config_file = example_config_file().await;
        toml::from_str::<ConfigIR>(&config_file).unwrap();
    }
    #[tokio::test]
    async fn test_convert_ir_to_config() {
        let config_file = example_config_file().await;
        let ir: ConfigIR = toml::from_str(&config_file).unwrap();
        Config::try_from(ir).unwrap();
    }
    #[tokio::test]
    async fn test_default_config_equals_deserialized_config() {
        let config_file = example_config_file().await;
        let ir: ConfigIR = toml::from_str(&config_file).unwrap();
        let Config {
            auth_type,
            keybinds,
        } = Config::try_from(ir).unwrap();
        let YoutuiKeymap {
            global,
            playlist,
            browser,
            browser_artists,
            browser_search,
            browser_songs,
            help,
            sort,
            filter,
            text_entry,
            list,
            log,
        } = keybinds;
        let Config {
            auth_type: def_auth_type,
            keybinds: def_keybinds,
        } = Config::default();
        let YoutuiKeymap {
            global: def_global,
            playlist: def_playlist,
            browser: def_browser,
            browser_artists: def_browser_artists,
            browser_search: def_browser_search,
            browser_songs: def_browser_songs,
            help: def_help,
            sort: def_sort,
            filter: def_filter,
            text_entry: def_text_entry,
            list: def_list,
            log: def_log,
        } = def_keybinds;
        // Assertions are split up here, to better narrow down errors.
        assert_eq!(auth_type, def_auth_type, "auth_type keybinds don't match");
        assert_eq!(global, def_global, "global keybinds don't match");
        assert_eq!(playlist, def_playlist, "playlist keybinds don't match");
        assert_eq!(browser, def_browser, "browser keybinds don't match");
        assert_eq!(
            browser_artists, def_browser_artists,
            "browser_artists keybinds don't match"
        );
        assert_eq!(
            browser_search, def_browser_search,
            "browser_search keybinds don't match"
        );
        assert_eq!(
            browser_songs, def_browser_songs,
            "browser_songs keybinds don't match"
        );
        assert_eq!(help, def_help, "help keybinds don't match");
        assert_eq!(sort, def_sort, "sort keybinds don't match");
        assert_eq!(filter, def_filter, "filter keybinds don't match");
        assert_eq!(
            text_entry, def_text_entry,
            "text_entry keybinds don't match"
        );
        assert_eq!(list, def_list, "list keybinds don't match");
        assert_eq!(log, def_log, "log keybinds don't match");
    }
    #[tokio::test]
    async fn test_default_config_equals_blank_config() {
        let ir: ConfigIR = toml::from_str("").unwrap();
        let Config {
            auth_type,
            keybinds,
        } = Config::try_from(ir).unwrap();
        let YoutuiKeymap {
            global,
            playlist,
            browser,
            browser_artists,
            browser_search,
            browser_songs,
            help,
            sort,
            filter,
            text_entry,
            list,
            log,
        } = keybinds;
        let Config {
            auth_type: def_auth_type,
            keybinds: def_keybinds,
        } = Config::default();
        let YoutuiKeymap {
            global: def_global,
            playlist: def_playlist,
            browser: def_browser,
            browser_artists: def_browser_artists,
            browser_search: def_browser_search,
            browser_songs: def_browser_songs,
            help: def_help,
            sort: def_sort,
            filter: def_filter,
            text_entry: def_text_entry,
            list: def_list,
            log: def_log,
        } = def_keybinds;
        // Assertions are split up here, to better narrow down errors.
        assert_eq!(auth_type, def_auth_type);
        assert_eq!(global, def_global);
        assert_eq!(playlist, def_playlist);
        assert_eq!(browser, def_browser);
        assert_eq!(browser_artists, def_browser_artists);
        assert_eq!(browser_search, def_browser_search);
        assert_eq!(browser_songs, def_browser_songs);
        assert_eq!(help, def_help);
        assert_eq!(sort, def_sort);
        assert_eq!(filter, def_filter);
        assert_eq!(text_entry, def_text_entry);
        assert_eq!(list, def_list);
        assert_eq!(log, def_log);
    }
    #[tokio::test]
    async fn test_different_config_to_default() {
        let config_file = tokio::fs::read_to_string("./config/config.toml.vim-example")
            .await
            .unwrap();
        let ir: ConfigIR = toml::from_str(&config_file).unwrap();
        let config = Config::try_from(ir).unwrap();
        let def_config = Config::default();
        assert_ne!(config, def_config)
    }
}
