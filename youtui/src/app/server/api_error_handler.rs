use crate::{core::get_limited_sequential_file, get_data_dir};
use anyhow::Result;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tracing::error;
use ytmapi_rs::error::ErrorKind;

const MAX_JSON_FILES: u16 = 5;
const JSON_FILE_NAME: &str = "source";
const JSON_FILE_EXT: &str = "json";

/// A simple logger of json files that caused errors.
pub struct ApiErrorHandler;

pub enum ApiErrorKind {
    YtmapiErrorNonJson,
    YtmapiErrorJson,
    OtherError,
}

impl ApiErrorHandler {
    pub fn new() -> Self {
        Self
    }
    /// Apply the appropriate handling to an api error.
    /// e.g. Log to tracing and write the faulty json (if exists) to log
    /// directory.
    /// Returns the kind of error.
    pub async fn handle_error(&self, e: anyhow::Error, message: String) -> ApiErrorKind {
        let e = match e.downcast::<ytmapi_rs::Error>().map(|e| e.into_kind()) {
            Err(e) => {
                error!("{message} <{e}>");
                return ApiErrorKind::OtherError;
            }
            Ok(e) => e,
        };
        let ErrorKind::JsonParsing(e) = e else {
            error!("{message} <{e}>");
            return ApiErrorKind::YtmapiErrorNonJson;
        };
        let (json, key) = e.get_json_and_key();
        error!("{message} at key {:?}", key);
        match log_json(json).await {
            Ok(path) => {
                error!(
                    "Source json has been logged to disk at <{}>",
                    path.display()
                );
            }
            Err(e) => error!("Error logging source json to file <{e}>"),
        }
        ApiErrorKind::YtmapiErrorJson
    }
}

/// Writes json file to disk at the next available sequential file.
/// Returns path of file.
async fn log_json(json: String) -> Result<PathBuf> {
    let (mut json_file, json_file_name) = get_limited_sequential_file(
        &get_data_dir()?,
        JSON_FILE_NAME,
        JSON_FILE_EXT,
        MAX_JSON_FILES,
    )
    .await?;
    json_file.write_all(json.as_bytes()).await?;
    Ok(json_file_name)
}
