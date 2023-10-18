use std::borrow::Borrow;

use tokio::sync::mpsc;
use tracing::error;

/// Send a message to the specified Tokio mpsc::Sender, and if sending fails, log an error with Tracing.
// TODO: test this - unsure how.
pub async fn send_or_error<T, S: Borrow<mpsc::Sender<T>>>(tx: S, msg: T) {
    tx.borrow()
        .send(msg)
        .await
        .unwrap_or_else(|e| error!("Error {e} received when sending message"));
}
/// Send a message to the specified Tokio mpsc::Sender, and if sending fails, log an error with Tracing.
/// Synchronous version
// TODO: test this - unsure how.
pub fn blocking_send_or_error<T, S: Borrow<mpsc::Sender<T>>>(tx: S, msg: T) {
    tx.borrow()
        .try_send(msg)
        .unwrap_or_else(|e| error!("Error {e} received when sending message"));
}
