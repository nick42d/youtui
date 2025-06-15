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
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;
use tokio::fs::DirEntry;
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

/// Search directory for files matching the pattern {filename}{NUMBER}.{filext}
/// and ext fileext, creating one at {filename}{NUMBER+1}.{filext}.
/// If there are more than max_files with this pattern, delete the
/// oldest surplus ones.
pub async fn get_limited_sequential_file(
    dir: &Path,
    filename: impl AsRef<str>,
    fileext: impl AsRef<str>,
    max_files: u16,
) -> Result<(fs_err::tokio::File, PathBuf), anyhow::Error> {
    if max_files == 0 {
        bail!("Requested zero file handles")
    }
    let filename = filename.as_ref();
    let fileext = fileext.as_ref();
    let stream = tokio::fs::read_dir(dir).await?;
    #[derive(Debug)]
    struct ValidEntry {
        entry: DirEntry,
        file_number: usize,
    }
    let get_valid_entry = |entry: DirEntry| {
        let entry_file_name = entry.file_name().into_string().ok()?;
        let file_number = entry_file_name
            .trim_start_matches(filename)
            .trim_end_matches(fileext)
            .trim_end_matches(".")
            .parse::<usize>()
            .ok()?;
        if entry_file_name.starts_with(filename) && entry_file_name.ends_with(fileext) {
            Some(ValidEntry { entry, file_number })
        } else {
            None
        }
    };
    let mut entries = ReadDirStream::new(stream)
        .filter_map(|try_entry| {
            let entry = match try_entry {
                Ok(entry) => entry,
                Err(e) => return Some(Err(e)),
            };
            get_valid_entry(entry).map(Ok)
        })
        .collect::<Result<Vec<ValidEntry>, _>>()
        .await?;
    entries.sort_by_key(|f| f.file_number);
    let next_number = entries.last().map(|e| e.file_number + 1).unwrap_or(0);
    let next_filename = format!("{filename}{}.{}", next_number, fileext);
    // If there are max_files files or more, remove the extra files.
    let surplus_files = entries
        .len()
        // Add an additional 1, as we are going to create a file bringing us up to max_files.
        .add(1)
        .saturating_sub(max_files as usize);
    let _files_deleted = entries
        .into_iter()
        .take(surplus_files)
        .map(|entry| fs_err::tokio::remove_file(entry.entry.path()))
        .collect::<FuturesUnordered<_>>()
        .try_collect::<Vec<_>>()
        .await?
        .len();
    let next_filepath = dir.join(next_filename);
    Ok((
        fs_err::tokio::File::create_new(&next_filepath).await?,
        next_filepath,
    ))
}

/// Either creates a new directory at dir, or deletes all files in the directory
/// starting with managed_file_prefix that are older (last modified) than
/// max_age. Returns the number of files cleaned up, if any.
pub async fn create_or_clean_directory(
    dir: &Path,
    managed_file_prefix: impl AsRef<str>,
    max_age: std::time::Duration,
) -> std::io::Result<usize> {
    fs_err::tokio::create_dir_all(dir).await?;
    let time_now = SystemTime::now();
    let album_art_dir_reader = tokio::fs::read_dir(dir).await?;
    let filename_prefix_matches = |entry: &DirEntry| {
        let matches = entry
            .file_name()
            .to_str()
            .is_some_and(|s| s.starts_with(managed_file_prefix.as_ref()));
        async move { matches }
    };
    let delete_file_if_aged = |entry: DirEntry| async move {
        let last_modified = entry.metadata().await?.modified()?;
        if !time_now
            .duration_since(last_modified)
            .is_ok_and(|dif| dif <= max_age)
        {
            fs_err::tokio::remove_file(entry.path()).await.map(Some)
        } else {
            Ok(None)
        }
    };
    let files_deleted = ReadDirStream::new(album_art_dir_reader)
        .try_filter(filename_prefix_matches)
        .try_filter_map(delete_file_if_aged)
        .try_collect::<Vec<_>>()
        .await?
        .len();
    Ok(files_deleted)
}

/// From serde documentation: [<https://serde.rs/string-or-struct.html>]
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
            std::time::Duration::from_secs(u64::MAX),
        )
        .await
        .unwrap();
        assert!(fs_err::tokio::remove_dir(target_dir).await.is_ok());
    }
    #[tokio::test]
    async fn test_create_or_clean_directory_deletes_aged() {
        let tmpdir = TempDir::new().unwrap();
        let target_dir = tmpdir.path().join("test_dir");
        let target_file = target_dir.join("test_file");
        fs_err::tokio::create_dir_all(&target_dir).await.unwrap();
        let file = fs_err::tokio::File::create(&target_file).await.unwrap();
        file.into_std()
            .await
            .into_file()
            .set_modified(SystemTime::now() - Duration::from_secs(60))
            .unwrap();
        create_or_clean_directory(&target_dir, "test_", std::time::Duration::from_secs(59))
            .await
            .unwrap();
        assert!(fs_err::tokio::File::open(target_file).await.is_err());
    }
    #[tokio::test]
    async fn test_create_or_clean_directory_doesnt_delete_aged_wrong_prefix() {
        let tmpdir = TempDir::new().unwrap();
        let target_dir = tmpdir.path().join("test_dir");
        let target_file = target_dir.join("users file");
        fs_err::tokio::create_dir_all(&target_dir).await.unwrap();
        let file = fs_err::tokio::File::create(&target_file).await.unwrap();
        file.into_std()
            .await
            .into_file()
            .set_modified(SystemTime::now() - Duration::from_secs(60))
            .unwrap();
        create_or_clean_directory(&target_dir, "test_", std::time::Duration::from_secs(59))
            .await
            .unwrap();
        assert!(fs_err::tokio::File::open(target_file).await.is_ok());
    }
    #[tokio::test]
    async fn test_create_or_clean_directory_doesnt_delete_unaged() {
        let tmpdir = TempDir::new().unwrap();
        let target_dir = tmpdir.path().join("test_dir");
        let target_file = target_dir.join("test_file");
        fs_err::tokio::create_dir_all(&target_dir).await.unwrap();
        let file = fs_err::tokio::File::create(&target_file).await.unwrap();
        drop(file);
        create_or_clean_directory(
            &target_dir,
            "test_",
            std::time::Duration::from_secs(u64::MAX),
        )
        .await
        .unwrap();
        assert!(fs_err::tokio::File::open(target_file).await.is_ok());
    }
    #[tokio::test]
    async fn test_get_limited_sequential_file_has_correct_filename() {
        let tmpdir = TempDir::new().unwrap();
        let _file = get_limited_sequential_file(tmpdir.path(), "test_filename", "txt", 5)
            .await
            .unwrap();
        let filename = fs_err::tokio::read_dir(tmpdir.path())
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
        assert!(timestamp.parse::<usize>().is_ok())
    }
    #[tokio::test]
    async fn test_get_limited_sequential_file_deletes_oldest() {
        let tmpdir = TempDir::new().unwrap();
        let _f1 = get_limited_sequential_file(tmpdir.path(), "test_filename", "txt", 2)
            .await
            .unwrap();
        let f1_name = fs_err::tokio::read_dir(tmpdir.path())
            .await
            .unwrap()
            .next_entry()
            .await
            .unwrap()
            .unwrap()
            .file_name()
            .into_string()
            .unwrap();
        let _f2 = get_limited_sequential_file(tmpdir.path(), "test_filename", "txt", 2)
            .await
            .unwrap();
        let files_count = std::fs::read_dir(tmpdir.path()).unwrap().count();
        assert_eq!(files_count, 2);
        let _f3 = get_limited_sequential_file(tmpdir.path(), "test_filename", "txt", 2)
            .await
            .unwrap();
        let files_count = std::fs::read_dir(tmpdir.path()).unwrap().count();
        assert_eq!(files_count, 2);
        assert!(
            fs_err::tokio::File::open(tmpdir.path().join(f1_name))
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
        let (Ok(_f1), Ok(_f2)) = tokio::join!(
            fs_err::tokio::File::create_new(tmpdir.path().join("xxx.txt")),
            fs_err::tokio::File::create_new(tmpdir.path().join("test_filename_xxx")),
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
            fs_err::tokio::File::open(tmpdir.path().join("test_filename_xxx"))
                .await
                .is_ok(),
            "test_filename_xxx should not have been deleted"
        );
        assert!(
            fs_err::tokio::File::open(tmpdir.path().join("xxx.txt"))
                .await
                .is_ok(),
            "xxx.txt should not have been deleted"
        )
    }
}
