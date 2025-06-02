use bytes::Bytes;
use rusty_ytdl::VideoError;

/// Helper function to use rusty_ytdl::stream::Stream is if it were a
/// futures::Stream.
// NOTE: Potentially could be upstreamed: https://github.com/Mithronn/rusty_ytdl/issues/34.
pub fn into_futures_stream(
    youtube_stream: Box<dyn rusty_ytdl::stream::Stream + Send>,
) -> impl futures::Stream<Item = Result<Bytes, VideoError>> + Send {
    // Second value of initialisation tuple represents if the previous iteration of
    // the stream errored. If so, stream will close, as no future iterations of
    // the stream are expected to return Ok.
    futures::stream::unfold((youtube_stream, false), |(state, err)| async move {
        if err {
            return None;
        };
        let chunk = state.chunk().await;
        match chunk {
            // Return error value on this iteration, on the next iteration return None.
            Err(e) => Some((Err(e), (state, true))),
            // Happy path
            Ok(Some(bytes)) => Some((Ok(bytes), (state, false))),
            // Stream has closed.
            Ok(None) => None,
        }
    })
}
