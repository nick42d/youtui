//! Re-usable core functionality.
use futures::TryStreamExt;
use serde::{
    de::{self, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{
    borrow::Borrow, convert::Infallible, fmt, marker::PhantomData, path::Path, str::FromStr,
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

/// Get a file handle to the next available logfile.
pub async fn next_debug_file_handle(
    dir: &Path,
    filename: impl AsRef<str>,
    max_debug_files: u16,
) -> Result<tokio::fs::File, tokio::io::Error> {
    let filename = filename.as_ref();
    let mut stream = tokio::fs::read_dir(dir).await?;
    let mut entries = vec![];
    while let Some(entry) = stream.next_entry().await? {
        if entry
            .file_name()
            .into_string()
            .unwrap()
            .starts_with(filename)
        {
            entries.push(entry);
        }
    }
    entries.sort_by_key(|f| f.file_name());
    entries.into_iter().take(max_debug_files as usize);
    todo!()
    // for entry in  {}
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
