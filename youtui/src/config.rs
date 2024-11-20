use std::collections::HashMap;

use crate::get_config_dir;
use crate::Result;
use clap::ValueEnum;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use serde::{Deserialize, Serialize};
use ytmapi_rs::auth::OAuthToken;

const CONFIG_FILE_NAME: &str = "config.toml";

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

#[derive(PartialEq, Debug, Clone)]
pub struct Keybind {
    code: KeyCode,
    modifiers: KeyModifiers,
}

pub struct KeybindBasic {
    sequence: Vec<Keybind>,
    // Consider - can there be multiple actions?
    // Consider - can an action access global commands? Or commands from another component?
    action: String,
    // Eg header, standard, hidden.
    visibility: u8,
}
pub struct ModeBasic {
    sequence: Vec<Keybind>,
    name: String,
}

pub struct Keybinds {
    binds: Vec<Keybind>,
    mode_names: Vec<ModeBasic>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Config {
    pub auth_type: AuthType,
    // Consider - should keybinds be per module, e.g keybinds.playlist, keybinds.browser.
    pub keybinds: HashMap<String, String>,
    pub mode_names: HashMap<String, String>,
}

#[derive(ValueEnum, Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub enum AuthType {
    #[value(name = "oauth")]
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
}
