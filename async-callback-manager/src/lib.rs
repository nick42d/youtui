use futures::Future;
use futures::FutureExt;
use futures::Stream;
use std::any::Any;
use tokio::sync::oneshot;

mod error;
mod manager;
mod sender;
mod task;

pub use error::*;
pub use manager::*;
pub use sender::*;
pub use task::Constraint;

pub trait BkendMap<Bkend> {
    fn map(backend: &Bkend) -> &Self;
}

/// A task of kind T that can be run on a backend, returning a future of output
/// Output. The type must implement Any, as the
/// TypeId is used as part of the task management process.
pub trait BackendTask<Bkend>: Send + Any {
    type Output: Send;
    type ConstraintType: PartialEq;
    fn into_future(self, backend: &Bkend) -> impl Future<Output = Self::Output> + Send + 'static;
    /// Metadata provides a way of grouping different tasks for use in
    /// constraints, if you override the default implementation.
    fn metadata() -> Vec<Self::ConstraintType> {
        vec![]
    }
}

/// A task of kind T that can be run on a backend, returning a stream of outputs
/// Output. The type must implement Any, as the TypeId is used as part of the
/// task management process.
pub trait BackendStreamingTask<Bkend>: Send + Any {
    type Output: Send;
    type ConstraintType: PartialEq;
    fn into_stream(
        self,
        backend: &Bkend,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin + 'static;
    /// Metadata provides a way of grouping different tasks for use in
    /// constraints, if you override the default implementation.
    fn metadata() -> Vec<Self::ConstraintType> {
        vec![]
    }
}

struct KillHandle(Option<oneshot::Sender<()>>);
struct KillSignal(oneshot::Receiver<()>);

impl KillHandle {
    fn kill(&mut self) -> Result<()> {
        if let Some(tx) = self.0.take() {
            return tx.send(()).map_err(|_| Error::ErrorSending);
        }
        Ok(())
    }
}
impl Future for KillSignal {
    type Output = Result<()>;
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.0.poll_unpin(cx).map_err(|_| Error::ReceiverDropped)
    }
}
fn kill_channel() -> (KillHandle, KillSignal) {
    let (tx, rx) = oneshot::channel();
    (KillHandle(Some(tx)), KillSignal(rx))
}

type DynFallibleFuture = Box<dyn Future<Output = Result<()>> + Unpin + Send>;
type DynCallbackFn<Frntend> = Box<dyn FnOnce(&mut Frntend) + Send>;
type DynBackendTask<Bkend> = Box<dyn FnOnce(&Bkend) -> DynFallibleFuture>;
