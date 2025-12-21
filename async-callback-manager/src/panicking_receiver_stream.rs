use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tokio_stream::Stream;
use tokio_stream::wrappers::ReceiverStream; // or std::future::Future

/// A modification to tokio's ReceiverStream that awaits a JoinHandle prior to
/// reporting closed, rethrowing the panic if there was one. Use this when the
/// ReceiverStream is driven by a task that may panic.
pub struct PanickingReceiverStream<T> {
    pub inner: ReceiverStream<T>,
    pub handle: JoinHandle<()>,
}

impl<T> PanickingReceiverStream<T> {
    pub fn new(recv: Receiver<T>, join_handle: JoinHandle<()>) -> Self {
        Self {
            inner: ReceiverStream::new(recv),
            handle: join_handle,
        }
    }
}

impl<T> Stream for PanickingReceiverStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Some(item)),
            Poll::Ready(None) => {
                match Pin::new(&mut self.handle).poll(cx) {
                    // Task is still tearing down; wait for it to finish to capture the panic.
                    Poll::Pending => Poll::Pending,
                    // Task panicked! Rethrow it.
                    Poll::Ready(Err(e)) if e.is_panic() => {
                        std::panic::resume_unwind(e.into_panic());
                    }
                    // Task finished normally or was cancelled.
                    _ => Poll::Ready(None),
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::PanickingReceiverStream;
    use futures::StreamExt;
    use tokio_stream::wrappers::ReceiverStream;

    #[tokio::test]
    async fn assert_tokio_receiver_stream_does_not_panic_if_task_panics() {
        let (tx, rx) = tokio::sync::mpsc::channel(30);
        tokio::spawn(async move {
            for i in 0..=10 {
                if i == 6 {
                    panic!();
                }
                tx.send(i).await.unwrap();
            }
        });
        let stream = ReceiverStream::new(rx);
        let output: Vec<_> = stream.collect().await;
        assert_eq!(output, vec![0, 1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    #[should_panic]
    async fn panicking_receiver_stream_should_panic_if_task_panics() {
        let (tx, rx) = tokio::sync::mpsc::channel(30);
        let handle = tokio::spawn(async move {
            for i in 0..=10 {
                if i == 6 {
                    panic!();
                }
                tx.send(i).await.unwrap();
            }
        });
        let stream = PanickingReceiverStream::new(rx, handle);
        let output: Vec<_> = stream.collect().await;
        assert_eq!(output, vec![0, 1, 2, 3, 4, 5]);
    }
}
