//! Re-usable core functionality.
use anyhow::bail;
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{
    borrow::Borrow,
    convert::Infallible,
    fmt,
    marker::PhantomData,
    num::NonZero,
    path::Path,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReadDirStream, StreamExt};
use tracing::error;

/// Send a message to the specified Tokio mpsc::Sender, and if sending fails,
/// log an error with Tracing.
pub async fn send_or_error<T, S: Borrow<mpsc::Sender<T>>>(tx: S, msg: T) {
    tx.borrow()
        .send(msg)
        .await
        .unwrap_or_else(|e| error!("Error {e} received when sending message"));
}

/// Create monotonically increasing file handles with prefix filename and ext
/// fileext, but if there are more than max_files with this pattern, delete the
/// lowest one first.
pub async fn get_limited_sequential_file(
    dir: &Path,
    filename: impl AsRef<str>,
    fileext: impl AsRef<str>,
    max_files: u16,
    timestamp: SystemTime,
) -> Result<tokio::fs::File, anyhow::Error> {
    if max_files == 0 {
        bail!("Requested zero file handles")
    }
    let filename = filename.as_ref();
    let stream = tokio::fs::read_dir(dir).await?;
    let mut entries = ReadDirStream::new(stream)
        .filter(|try_entry| {
            try_entry
                .as_ref()
                .ok()
                .and_then(|entry| entry.file_name().into_string().ok())
                .map(|entry_file_name| entry_file_name.starts_with(filename))
                .is_some_and(|entry_file_name_matches| entry_file_name_matches)
        })
        .collect::<Result<Vec<_>, _>>()
        .await?;
    entries.sort_by_key(|f| f.file_name());
    // TODO: don't use timestamp debug representation.
    let timestamp = timestamp.duration_since(UNIX_EPOCH)?.as_secs();
    // Use 20 characters left padding of zeros - this ensures all timestamps up to
    // usize::MAX still sort in ascending order once stringified.
    let next_filename = format!("{filename}{:020}.{}", timestamp, fileext.as_ref());
    if let Some(target_file) = entries
        .into_iter()
        .rev()
        .nth(max_files.checked_sub(1).expect("Zero should be guarded") as usize)
    {
        tokio::fs::remove_file(target_file.path()).await?;
    }
    Ok(tokio::fs::File::open(dir.with_file_name(next_filename)).await?)
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
    use crate::core::get_limited_sequential_file;
    use std::time::SystemTime;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_get_limited_sequential_file_is_monotonic() {
        let tmpdir = TempDir::new();
        let f1 =
            get_limited_sequential_file(tmpdir, "test_is_monotonic", "txt", 5, SystemTime::now())
                .await
                .unwrap();
        let f2 =
            get_limited_sequential_file(tmpdir, "test_is_monotonic", "txt", 5, SystemTime::now())
                .await
                .unwrap();
    }
    #[tokio::test]
    async fn test_get_limited_sequential_file_deletes_one() {}
    #[tokio::test]
    async fn test_get_limited_sequential_file_creates_one() {}
    #[tokio::test]
    async fn test_get_limited_sequential_file_doesnt_delete_others() {}
}
