use crate::config::AuthType;
use std::{fmt::Display, path::PathBuf};
use tokio::{sync::mpsc, task::JoinError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    OAuthNotYetSupportedByApp,
    Communication,
    UnknownAPIError,
    DirectoryNameError,
    IoError(std::io::Error),
    JoinError(JoinError),
    // TODO: More advanced error conversions
    ApiError(ytmapi_rs::Error),
    JsonError(serde_json::Error),
    TomlDeserializationError(toml::de::Error),
    AuthTokenError {
        token_type: AuthType,
        token_location: PathBuf,
        io_error: std::io::Error,
    },
    AuthTokenParseError {
        token_type: AuthType,
        token_location: PathBuf,
    },
    ErrorCreatingDirectory {
        directory: PathBuf,
        io_error: std::io::Error,
    },
    // TODO: Remove this, catchall currentl
    Other(String),
}
impl Error {
    // Consider taking into pathbuf.
    pub fn new_auth_token_error(
        token_type: AuthType,
        token_location: PathBuf,
        io_error: std::io::Error,
    ) -> Self {
        Self::AuthTokenError {
            token_type,
            token_location,
            io_error,
        }
    }
    // Consider taking into pathbuf.
    pub fn new_auth_token_parse_error(token_type: AuthType, token_location: PathBuf) -> Self {
        Self::AuthTokenParseError {
            token_type,
            token_location,
        }
    }
    pub fn new_error_creating_directory(directory: PathBuf, io_error: std::io::Error) -> Self {
        Self::ErrorCreatingDirectory {
            directory,
            io_error,
        }
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Communication => write!(f, "Error sending message to channel"),
            Error::DirectoryNameError => write!(f, "Error generating application directory for your host system. See README.md for more information about application directories."),
            Error::OAuthNotYetSupportedByApp => write!(f, "App does not currently support Oauth tokens for authentication. Use browser authentication. See README.md for more information."),
            Error::UnknownAPIError => write!(f, "Unknown API error."),
            Error::Other(s) => write!(f, "Unknown error with message \"{s}\""),
            Error::IoError(e) => write!(f, "Standard io error <{e}>"),
            Error::JoinError(e) => write!(f, "Join error <{e}>"),
            Error::ApiError(e) => write!(f, "Api error <{e}>"),
            Error::JsonError(e) => write!(f, "Json error <{e}>"),
            Error::TomlDeserializationError(e) => write!(f, "Toml deserialization error:\n{e}"),
            // TODO: Better display format for token_type.
            // XXX: Consider displaying the io error.
            Error::AuthTokenError { token_type, token_location, io_error: _} => write!(f, "Error loading {:?} auth token from {}. Does the file exist? See README.md for more information on auth tokens.", token_type, token_location.display()),
            Error::AuthTokenParseError { token_type, token_location, } => write!(f, "Error parsing {:?} auth token from {}. See README.md for more information on auth tokens.", token_type, token_location.display()),
            Error::ErrorCreatingDirectory{  directory, io_error: _} => write!(f, "Error creating required directory {} for the application. Do you have the required permissions? See README.md for more information on application directories.",  directory.display()),
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
        Error::IoError(value)
    }
}
impl From<JoinError> for Error {
    fn from(value: JoinError) -> Self {
        Error::JoinError(value)
    }
}
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::JsonError(value)
    }
}
impl From<toml::de::Error> for Error {
    fn from(value: toml::de::Error) -> Self {
        Error::TomlDeserializationError(value)
    }
}
impl From<ytmapi_rs::Error> for Error {
    fn from(value: ytmapi_rs::Error) -> Self {
        Error::ApiError(value)
    }
}
