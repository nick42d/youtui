use crate::config::AuthType;
use std::{fmt::Display, path::PathBuf};
use tokio::{sync::mpsc, task::JoinError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Communication,
    UnknownAPI,
    DirectoryName,
    Io(std::io::Error),
    Join(JoinError),
    // TODO: More advanced error conversions
    Api(ytmapi_rs::Error),
    ApiErrorString(String),
    Json(serde_json::Error),
    TomlDeserialization(toml::de::Error),
    WrongAuthType {
        current_authtype: AuthType,
        expected_authtype: AuthType,
        query_type: &'static str,
    },
    AuthToken {
        token_type: AuthType,
        token_location: PathBuf,
        io_error: std::io::Error,
    },
    PoToken {
        token_location: PathBuf,
        io_error: std::io::Error,
    },
    AuthTokenParse {
        token_type: AuthType,
        token_location: PathBuf,
    },
    CreatingDirectory {
        directory: PathBuf,
        io_error: std::io::Error,
    },
    ManagerDropped,
    // TODO: Remove this, catchall currently
    Other(String),
}
impl Error {
    pub fn new_api_error_string(e: String) -> Self {
        Self::ApiErrorString(e)
    }
    pub fn new_wrong_auth_token_error_browser<Q>(_query: Q, current_authtype: AuthType) -> Self {
        let expected_authtype = AuthType::Browser;
        let query_type = std::any::type_name::<Q>();
        Self::WrongAuthType {
            current_authtype,
            expected_authtype,
            query_type,
        }
    }
    pub fn new_wrong_auth_token_error_oauth<Q>(_query: Q, current_authtype: AuthType) -> Self {
        let expected_authtype = AuthType::OAuth;
        let query_type = std::any::type_name::<Q>();
        Self::WrongAuthType {
            current_authtype,
            expected_authtype,
            query_type,
        }
    }
    // Consider taking into pathbuf.
    pub fn new_auth_token_error(
        token_type: AuthType,
        token_location: PathBuf,
        io_error: std::io::Error,
    ) -> Self {
        Self::AuthToken {
            token_type,
            token_location,
            io_error,
        }
    }
    // Consider taking into pathbuf.
    pub fn new_po_token_error(token_location: PathBuf, io_error: std::io::Error) -> Self {
        Self::PoToken {
            token_location,
            io_error,
        }
    }
    // Consider taking into pathbuf.
    pub fn new_auth_token_parse_error(token_type: AuthType, token_location: PathBuf) -> Self {
        Self::AuthTokenParse {
            token_type,
            token_location,
        }
    }
    pub fn new_error_creating_directory(directory: PathBuf, io_error: std::io::Error) -> Self {
        Self::CreatingDirectory {
            directory,
            io_error,
        }
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Communication => write!(f, "Error sending message to channel"),
            Error::DirectoryName => write!(f, "Error generating application directory for your host system. See README.md for more information about application directories."),
            Error::UnknownAPI => write!(f, "Unknown API error."),
            Error::Other(s) => write!(f, "Unknown error with message \"{s}\""),
            Error::Io(e) => write!(f, "Standard io error <{e}>"),
            Error::Join(e) => write!(f, "Join error <{e}>"),
            Error::Api(e) => write!(f, "Api error <{e}>"),
            Error::Json(e) => write!(f, "Json error <{e}>"),
            Error::TomlDeserialization(e) => write!(f, "Toml deserialization error:\n{e}"),
            // TODO: Better display format for token_type.
            // XXX: Consider displaying the io error.
            Error::PoToken { token_location, io_error: _} => write!(f, "Error loading po_token from {}. Does the file exist?", token_location.display()),
            Error::AuthToken { token_type, token_location, io_error: _} => ,
            Error::AuthTokenParse { token_type, token_location, } => ),
            Error::CreatingDirectory{  directory, io_error: _} => write!(f, "Error creating required directory {} for the application. Do you have the required permissions? See README.md for more information on application directories.",  directory.display()),
            Error::ApiErrorString(s) => write!(f, "{s}"),
            Error::ManagerDropped => write!(f, "Async callback manager dropped"),
        }
    }
}
impl<T> From<mpsc::error::SendError<T>> for Error {
    fn from(_value: mpsc::error::SendError<T>) -> Self {
        Error::Communication
    }
}
impl From<mpsc::error::TryRecvError> for Error {
    fn from(_value: mpsc::error::TryRecvError) -> Self {
        Error::Communication
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}
impl From<JoinError> for Error {
    fn from(value: JoinError) -> Self {
        Error::Join(value)
    }
}
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Json(value)
    }
}
impl From<toml::de::Error> for Error {
    fn from(value: toml::de::Error) -> Self {
        Error::TomlDeserialization(value)
    }
}
impl From<ytmapi_rs::Error> for Error {
    fn from(value: ytmapi_rs::Error) -> Self {
        Error::Api(value)
    }
}
impl From<async_callback_manager::Error> for Error {
    fn from(_: async_callback_manager::Error) -> Self {
        Error::ManagerDropped
    }
}
