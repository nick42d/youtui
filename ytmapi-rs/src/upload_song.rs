use crate::auth::{AuthToken, BrowserToken, LoggedIn};
use crate::client::Body;
use crate::common::ApiOutcome;
use crate::utils::constants::DEFAULT_X_GOOG_AUTHUSER;
use crate::{Client, Result};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;

/// Allowed upload file types - check by trying to upload something outside this
/// list on YTM.
const ALLOWED_UPLOAD_EXTENSIONS: &[&str] = &["mp3", "m4a", "wma", "flac", "ogg"];

/// Upload a song to your YouTube Music Library.
pub async fn upload_song(
    file_path: impl AsRef<Path>,
    token: &BrowserToken,
    client: &Client,
) -> Result<ApiOutcome> {
    let file_path = file_path.as_ref();

    // Internal validation first
    let upload_fileext = file_path.extension().and_then(OsStr::to_str).unwrap();
    if !ALLOWED_UPLOAD_EXTENSIONS
        .iter()
        .any(|ext| upload_fileext == *ext)
    {
        panic!(
            "Fileext not in allowed list. Allowed values: {:?}",
            ALLOWED_UPLOAD_EXTENSIONS
        );
    }
    let song_file = tokio::fs::File::open(&file_path).await.unwrap();
    let upload_filesize_bytes = song_file.metadata().await.unwrap().len();
    const MAX_UPLOAD_FILESIZE_MB: u64 = 300;
    if upload_filesize_bytes > MAX_UPLOAD_FILESIZE_MB * (1024 * 1024) {
        panic!(
            "Unable to upload song greater than {} MB, size is {} MB",
            MAX_UPLOAD_FILESIZE_MB,
            upload_filesize_bytes / (1024 * 1024)
        );
    }

    // Headers to get upload url
    let additional_headers: [(&str, Cow<str>); 4] = [
        (
            "Content-Type",
            "application/x-www-form-urlencoded;charset=utf-8".into(),
        ),
        ("X-Goog-Upload-Command", "start".into()),
        (
            "X-Goog-Upload-Header-Content-Length",
            upload_filesize_bytes.to_string().into(),
        ),
        ("X-Goog-Upload-Protocol", "resumable".into()),
    ];
    // Deduplicate with token's headers.
    let mut combined_headers = token
        .headers()
        .unwrap()
        .into_iter()
        .chain(additional_headers)
        .collect::<HashMap<_, _>>();
    let upload_url = client
        .post_query(
            "https://upload.youtube.com/upload/usermusic/http",
            combined_headers
                .iter()
                .map(|(k, v)| (*k, v.as_ref().into())),
            Body::FromString(format!(
                "filename={}",
                file_path.file_name().unwrap().to_string_lossy()
            )),
            &[("authuser", DEFAULT_X_GOOG_AUTHUSER)],
        )
        .await
        .unwrap()
        .headers
        .into_iter()
        .find(|(k, _)| k == "x-goog-upload-url")
        .unwrap()
        .1;
    // Additional headers required to upload.
    combined_headers.extend([
        ("X-Goog-Upload-Command", "upload, finalize".into()),
        ("X-Goog-Upload-Offset", "0".into()),
    ]);
    if client
        .post_query(
            upload_url,
            combined_headers,
            Body::FromFileRef(&song_file),
            &(),
        )
        .await
        .unwrap()
        .status_code
        == 200
    {
        Ok(ApiOutcome::Success)
    } else {
        // Consider returning the error code here.
        Ok(ApiOutcome::Failure)
    }
}
