//! Re-usable core functionality.
use anyhow::bail;
use futures::stream::FuturesUnordered;
use futures::TryStreamExt;
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::borrow::Borrow;
use std::convert::Infallible;
use std::fmt;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReadDirStream;
use tokio_stream::StreamExt;
use tracing::error;

/// Send a message to the specified Tokio mpsc::Sender, and if sending fails,
/// log an error with Tracing.
pub async fn send_or_error<T, S: Borrow<mpsc::Sender<T>>>(tx: S, msg: T) {
    tx.borrow()
        .send(msg)
        .await
        .unwrap_or_else(|e| error!("Error {e} received when sending message"));
}

/// Send a message to the specified Tokio mpsc::Sender, and if sending fails,
/// log an error with Tracing.
pub fn blocking_send_or_error<T, S: Borrow<mpsc::Sender<T>>>(tx: S, msg: T) {
    tx.borrow()
        .blocking_send(msg)
        .unwrap_or_else(|e| error!("Error {e} received when sending message"));
}

/// Create timestamped file handle in dir with prefix filename and ext
/// fileext, and if there are more than max_files with this pattern, delete the
/// one with the oldest timestamp.
pub async fn get_limited_sequential_file(
    dir: &Path,
    filename: impl AsRef<str>,
    fileext: impl AsRef<str>,
    max_files: u16,
) -> Result<(tokio::fs::File, PathBuf), anyhow::Error> {
    if max_files == 0 {
        bail!("Requested zero file handles")
    }
    let filename = filename.as_ref();
    let fileext = fileext.as_ref();
    let stream = tokio::fs::read_dir(dir).await?;
    let mut entries = ReadDirStream::new(stream)
        .filter(|try_entry| {
            try_entry
                .as_ref()
                .ok()
                .and_then(|entry| entry.file_name().into_string().ok())
                .map(|entry_file_name| {
                    entry_file_name.starts_with(filename) && entry_file_name.ends_with(fileext)
                })
                .is_some_and(|entry_file_name_matches| entry_file_name_matches)
        })
        .collect::<Result<Vec<_>, _>>()
        .await?;
    entries.sort_by_key(|f| f.file_name());
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    // Use 20 characters left padding of zeros - this ensures all timestamps up to
    // usize::MAX still sort in ascending order once stringified.
    let next_filename = format!("{filename}{:020}.{}", timestamp, fileext);
    if let Some(target_file) = entries
        .into_iter()
        .rev()
        .nth(max_files.checked_sub(1).expect("Zero should be guarded") as usize)
    {
        tokio::fs::remove_file(target_file.path()).await?;
    }
    let next_filepath = dir.join(next_filename);
    Ok((
        tokio::fs::File::create_new(&next_filepath).await?,
        next_filepath,
    ))
}

/// Either creates a new directory at dir, or deletes all files in the directory
/// starting managed_file_prefix that are older (last modified) than max_age.
// TODO: Unit tests
pub async fn create_or_clean_directory(
    dir: &Path,
    managed_file_prefix: impl AsRef<str>,
    time_now: SystemTime,
    max_age: std::time::Duration,
) -> std::io::Result<()> {
    tokio::fs::create_dir_all(dir).await?;
    // The below block is a candidate for replacement with Stream code, although for
    // pragmatic reasons it's done here with a for loop.
    let delete_old_files_futures = FuturesUnordered::new();
    let mut album_art_dir_reader = tokio::fs::read_dir(dir).await?;
    while let Some(entry) = album_art_dir_reader.next_entry().await? {
        if entry
            .file_name()
            .to_str()
            .is_some_and(|s| s.starts_with(managed_file_prefix.as_ref()))
        {
            let last_modified = entry.metadata().await?.modified()?;
            if !time_now
                .duration_since(last_modified)
                .is_ok_and(|dif| dif <= max_age)
            {
                delete_old_files_futures.push(tokio::fs::remove_file(entry.path()))
            };
        }
    }
    delete_old_files_futures.try_collect::<()>().await
}

/// From serde documentation: [https://serde.rs/string-or-struct.html]
pub fn string_or_struct<'de, T, D>(deserializer: D) -> std::result::Result<T, D::Error>
where
    T: Deserialize<'de> + FromStr<Err = Infallible>,
    D: Deserializer<'de>,
{
    // This is a Visitor that forwards string types to T's `FromStr` impl and
    // forwards map types to T's `Deserialize` impl. The `PhantomData` is to
    // keep the compiler from complaining about T being an unused generic type
    // parameter. We need T in order to know the Value type for the Visitor
    // impl.
    struct StringOrStruct<T>(PhantomData<fn() -> T>);
    impl<'de, T> Visitor<'de> for StringOrStruct<T>
    where
        T: Deserialize<'de> + FromStr<Err = Infallible>,
    {
        type Value = T;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("string or map")
        }
        fn visit_str<E>(self, value: &str) -> std::result::Result<T, E>
        where
            E: de::Error,
        {
            Ok(FromStr::from_str(value).unwrap())
        }
        fn visit_map<M>(self, map: M) -> std::result::Result<T, M::Error>
        where
            M: MapAccess<'de>,
        {
            // `MapAccessDeserializer` is a wrapper that turns a `MapAccess`
            // into a `Deserializer`, allowing it to be used as the input to T's
            // `Deserialize` implementation. T then deserializes itself using
            // the entries from the map visitor.
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }
    deserializer.deserialize_any(StringOrStruct(PhantomData))
}

/// Get monotonically increasing file handles with prefix filename and ext
/// fileext, but if there are more than max_files with this pattern, delete the
/// lowest one first.
#[cfg(test)]
mod tests {
    use crate::core::{create_or_clean_directory, get_limited_sequential_file};
    use pretty_assertions::assert_eq;
    use std::time::{Duration, SystemTime};
    use tempfile::TempDir;
    use tokio_stream::wrappers::ReadDirStream;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn test_create_or_clean_directory_creates_directory() {
        let tmpdir = TempDir::new().unwrap();
        let target_dir = tmpdir.path().join("test_dir");
        create_or_clean_directory(
            &target_dir,
            "test_",
            SystemTime::now(),
            std::time::Duration::from_secs(u64::MAX),
        )
        .await
        .unwrap();
        assert!(tokio::fs::remove_dir(target_dir).await.is_ok());
    }
    #[tokio::test]
    async fn test_create_or_clean_directory_deletes_aged() {
        let tmpdir = TempDir::new().unwrap();
        let target_dir = tmpdir.path().join("test_dir");
        let target_file = target_dir.join("test_file");
        tokio::fs::create_dir_all(&target_dir).await.unwrap();
        let file = tokio::fs::File::create(&target_file).await.unwrap();
        file.into_std()
            .await
            .set_modified(SystemTime::now() - Duration::from_secs(60))
            .unwrap();
        create_or_clean_directory(
            &target_dir,
            "test_",
            SystemTime::now(),
            std::time::Duration::from_secs(59),
        )
        .await
        .unwrap();
        assert!(tokio::fs::File::open(target_file).await.is_err());
    }
    #[tokio::test]
    async fn test_create_or_clean_directory_doesnt_delete_aged_wrong_prefix() {
        let tmpdir = TempDir::new().unwrap();
        let target_dir = tmpdir.path().join("test_dir");
        let target_file = target_dir.join("users file");
        tokio::fs::create_dir_all(&target_dir).await.unwrap();
        let file = tokio::fs::File::create(&target_file).await.unwrap();
        file.into_std()
            .await
            .set_modified(SystemTime::now() - Duration::from_secs(60))
            .unwrap();
        create_or_clean_directory(
            &target_dir,
            "test_",
            SystemTime::now(),
            std::time::Duration::from_secs(59),
        )
        .await
        .unwrap();
        assert!(tokio::fs::File::open(target_file).await.is_ok());
    }
    #[tokio::test]
    async fn test_create_or_clean_directory_doesnt_delete_unaged() {
        let tmpdir = TempDir::new().unwrap();
        let target_dir = tmpdir.path().join("test_dir");
        let target_file = target_dir.join("test_file");
        tokio::fs::create_dir_all(&target_dir).await.unwrap();
        let file = tokio::fs::File::create(&target_file).await.unwrap();
        drop(file);
        create_or_clean_directory(
            &target_dir,
            "test_",
            SystemTime::now(),
            std::time::Duration::from_secs(u64::MAX),
        )
        .await
        .unwrap();
        assert!(tokio::fs::File::open(target_file).await.is_ok());
    }
    #[tokio::test]
    async fn test_get_limited_sequential_file_has_correct_filename() {
        let tmpdir = TempDir::new().unwrap();
        let _file = get_limited_sequential_file(tmpdir.path(), "test_filename", "txt", 5)
            .await
            .unwrap();
        let filename = tokio::fs::read_dir(tmpdir.path())
            .await
            .unwrap()
            .next_entry()
            .await
            .unwrap()
            .unwrap()
            .file_name()
            .into_string()
            .unwrap();
        assert!(filename.starts_with("test_filename"));
        assert!(filename.ends_with(".txt"));
        let timestamp = filename
            .trim_start_matches("test_filename")
            .trim_end_matches(".txt");
        assert!(timestamp.len() == 20);
        assert!(timestamp.parse::<usize>().is_ok())
    }
    #[tokio::test]
    async fn test_get_limited_sequential_file_deletes_oldest() {
        let tmpdir = TempDir::new().unwrap();
        let _f1 = get_limited_sequential_file(tmpdir.path(), "test_filename", "txt", 2)
            .await
            .unwrap();
        let f1_name = tokio::fs::read_dir(tmpdir.path())
            .await
            .unwrap()
            .next_entry()
            .await
            .unwrap()
            .unwrap()
            .file_name()
            .into_string()
            .unwrap();
        // Timestamp is in seconds, we need a delay to ensure timestamp increases.
        tokio::time::sleep(Duration::from_secs(1)).await;
        let _f2 = get_limited_sequential_file(tmpdir.path(), "test_filename", "txt", 2)
            .await
            .unwrap();
        let files_count = std::fs::read_dir(tmpdir.path()).unwrap().count();
        assert_eq!(files_count, 2);
        // Timestamp is in seconds, we need a delay to ensure timestamp increases.
        tokio::time::sleep(Duration::from_secs(1)).await;
        let _f3 = get_limited_sequential_file(tmpdir.path(), "test_filename", "txt", 2)
            .await
            .unwrap();
        let files_count = std::fs::read_dir(tmpdir.path()).unwrap().count();
        assert_eq!(files_count, 2);
        assert!(
            tokio::fs::File::open(tmpdir.path().join(f1_name))
                .await
                .is_err(),
            "_f1 should have been deleted"
        )
    }
    #[tokio::test]
    async fn test_get_limited_sequential_file_doesnt_delete_others() {
        let tmpdir = TempDir::new().unwrap();
        let _f = get_limited_sequential_file(tmpdir.path(), "test_filename", "txt", 1)
            .await
            .unwrap();
        let (Ok(_f1), Ok(_f2), _) = tokio::join!(
            tokio::fs::File::create_new(tmpdir.path().join("xxx.txt")),
            tokio::fs::File::create_new(tmpdir.path().join("test_filename_xxx")),
            tokio::time::sleep(Duration::from_secs(1)),
        ) else {
            panic!("Error creating test files")
        };
        let _f = get_limited_sequential_file(tmpdir.path(), "test_filename", "txt", 1)
            .await
            .unwrap();
        let files_in_dir = ReadDirStream::new(tokio::fs::read_dir(tmpdir.path()).await.unwrap())
            .collect::<Vec<_>>()
            .await;
        assert_eq!(files_in_dir.len(), 3);
        assert!(
            tokio::fs::File::open(tmpdir.path().join("test_filename_xxx"))
                .await
                .is_ok(),
            "test_filename_xxx should not have been deleted"
        );
        assert!(
            tokio::fs::File::open(tmpdir.path().join("xxx.txt"))
                .await
                .is_ok(),
            "xxx.txt should not have been deleted"
        )
    }
}
