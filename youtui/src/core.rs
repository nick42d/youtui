use async_callback_manager::{AsyncCallbackSender, BackendStreamingTask, BackendTask, Constraint};
use std::borrow::Borrow;
use tokio::sync::mpsc;
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
pub fn add_stream_cb_or_error<Bkend, Frntend, Cstrnt, R>(
    sender: &AsyncCallbackSender<Bkend, Frntend, Cstrnt>,
    // Bounds are from AsyncCallbackSender's own impl.
    request: R,
    handler: impl FnOnce(&mut Frntend, R::Output) + Send + Clone + 'static,
    constraint: Option<Constraint<Cstrnt>>,
) where
    R: BackendStreamingTask<Bkend, MetadataType = Cstrnt> + 'static,
    Bkend: Send + 'static,
    Frntend: 'static,
{
    sender
        .add_stream_callback(request, handler, constraint)
        .unwrap_or_else(|e| error!("Error {e} received when sending message"));
}

/// Send a message to the specified Tokio mpsc::Sender, and if sending fails,
/// log an error with Tracing.
pub fn add_cb_or_error<Bkend, Frntend, Cstrnt, R>(
    sender: &AsyncCallbackSender<Bkend, Frntend, Cstrnt>,
    // Bounds are from AsyncCallbackSender's own impl.
    request: R,
    handler: impl FnOnce(&mut Frntend, R::Output) + Send + 'static,
    constraint: Option<Constraint<Cstrnt>>,
) where
    R: BackendTask<Bkend, MetadataType = Cstrnt> + 'static,
    Bkend: Send + 'static,
    Frntend: 'static,
{
    sender
        .add_callback(request, handler, constraint)
        .unwrap_or_else(|e| error!("Error {e} received when sending message"));
}
