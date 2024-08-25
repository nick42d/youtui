use std::{borrow::Borrow, fmt::Debug};
use tokio::sync::{mpsc, oneshot};
use tracing::error;

/// Send a message to the specified Tokio mpsc::Sender, and if sending fails,
/// log an error with Tracing.
pub async fn send_or_error<T, S: Borrow<mpsc::Sender<T>>>(tx: S, msg: T) {
    tx.borrow()
        .send(msg)
        .await
        .unwrap_or_else(|e| error!("Error {e} received when sending message"));
}

/// Send a message to the specified Tokio oneshot::Sender, and if sending fails,
/// log an error with Tracing.
pub fn oneshot_send_or_error<T: Debug, S: Into<oneshot::Sender<T>>>(tx: S, msg: T) {
    tx.into()
        .send(msg)
        .unwrap_or_else(|e| error!("Error received when sending message {:?}", e));
}
