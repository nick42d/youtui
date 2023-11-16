use std::fmt::Display;

use tokio::{sync::mpsc, task::JoinError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Communication,
    DirectoryNameError,
    IoError(std::io::Error),
    JoinError(JoinError),
    // TODO: More advanced error conversions
    ApiError(ytmapi_rs::Error),
    JsonError(serde_json::Error),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Communication => write!(f, "Error sending message to channel"),
            Error::DirectoryNameError => write!(f, "Error generating project directory name"),
            Error::IoError(e) => write!(f, "Standard io error <{e}>"),
            Error::JoinError(e) => write!(f, "Join error <{e}>"),
            Error::ApiError(e) => write!(f, "Api error <{e}>"),
            Error::JsonError(e) => write!(f, "Json error <{e}>"),
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
impl From<ytmapi_rs::Error> for Error {
    fn from(value: ytmapi_rs::Error) -> Self {
        Error::ApiError(value)
    }
}
