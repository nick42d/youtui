// Utilising nightly until async trait stabilised
#![feature(async_fn_in_trait)]

mod app;
mod appevent;
mod core;

pub use error::Result;
use std::path::PathBuf;

use directories::ProjectDirs;
use error::Error;
use tokio::runtime;

const HEADER_FILENAME: &str = "headers.txt";

// XXX Should err
pub fn run_app() -> Result<()> {
    let rt = runtime::Runtime::new()?;
    rt.block_on(async {
        // TODO: Handle errors
        let mut app = app::Youtui::new()?;
        app.run().await;
        Ok(())
    })
}

pub fn get_data_dir() -> Result<PathBuf> {
    let directory = if let Ok(s) = std::env::var("YOUTUI_DATA_DIR") {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = ProjectDirs::from("com", "nick42", "youtui") {
        proj_dirs.data_local_dir().to_path_buf()
    } else {
        return Err(Error::DirectoryNotFound);
    };
    Ok(directory)
}

pub fn get_config_dir() -> Result<PathBuf> {
    let directory = if let Ok(s) = std::env::var("YOUTUI_CONFIG_DIR") {
        PathBuf::from(s)
    } else if let Some(proj_dirs) = ProjectDirs::from("com", "nick42", "youtui") {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        return Err(Error::DirectoryNotFound);
    };
    Ok(directory)
}

pub mod error {
    use std::fmt::Display;

    use tokio::{sync::mpsc, task::JoinError};

    pub type Result<T> = std::result::Result<T, Error>;

    #[derive(Debug)]
    pub enum Error {
        Communication,
        DirectoryNotFound,
        IoError(std::io::Error),
        JoinError(JoinError),
    }
    impl Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Communication => write!(f, "Error sending message to channel"),
                Error::DirectoryNotFound => write!(f, "Directory not found"),
                Error::IoError(e) => write!(f, "Standard io error <{e}>"),
                Error::JoinError(e) => write!(f, "Join error <{e}>"),
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
}
