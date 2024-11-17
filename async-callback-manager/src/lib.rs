use futures::Future;
use futures::FutureExt;
use futures::Stream;
use futures::StreamExt;
use std::any::Any;
use std::pin::Pin;
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

pub struct Then<T, F> {
    first: T,
    create_next: F,
}

impl<T, F> Then<T, F> {
    pub fn new<Bkend, T2>(first: T, create_next: F) -> Then<T, F>
    where
        T: BackendTask<Bkend>,
        T2: BackendTask<Bkend>,
        F: FnOnce(T::Output) -> T2,
    {
        Then { first, create_next }
    }
    pub fn new_stream<Bkend, S>(first: T, create_next: F) -> Then<T, F>
    where
        T: BackendTask<Bkend>,
        S: BackendStreamingTask<Bkend>,
        F: FnOnce(T::Output) -> S,
    {
        Then { first, create_next }
    }
}

impl<Bkend, T, T2, F, Ct> BackendTask<Bkend> for Then<T, F>
where
    Bkend: Clone + Send + 'static,
    F: Sync + Send + 'static,
    T: BackendTask<Bkend, ConstraintType = Ct>,
    T2: BackendTask<Bkend, ConstraintType = Ct>,
    Ct: PartialEq,
    F: FnOnce(T::Output) -> T2,
{
    type Output = T2::Output;
    type ConstraintType = Ct;
    fn into_future(self, backend: &Bkend) -> impl Future<Output = Self::Output> + Send + 'static {
        let Then { first, create_next } = self;
        let backend = backend.clone();
        async move {
            let output = first.into_future(&backend).await;
            let next = create_next(output);
            next.into_future(&backend).await
        }
    }
    fn metadata() -> Vec<Self::ConstraintType> {
        let mut first = T::metadata();
        let mut second = T2::metadata();
        second.append(&mut first);
        second
    }
}

impl<Bkend, T, S, F, Ct> BackendStreamingTask<Bkend> for Then<T, F>
where
    Bkend: Clone + Sync + Send + 'static,
    F: Sync + Send + 'static,
    T: BackendTask<Bkend, ConstraintType = Ct>,
    S: BackendStreamingTask<Bkend, ConstraintType = Ct>,
    Ct: PartialEq,
    F: FnOnce(T::Output) -> S + Copy,
{
    type Output = S::Output;
    type ConstraintType = Ct;
    fn into_stream(
        self,
        backend: &Bkend,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin + 'static {
        let Then { first, create_next } = self;
        let backend = backend.clone();
        Box::pin(futures::stream::unfold(
            (backend, Some(first), None::<Pin<Box<_>>>),
            move |(backend, mut seed, stream)| {
                async move {
                    if let Some(seed) = seed.take() {
                        let output = seed.into_future(&backend).await;
                        let task = create_next(output);
                        let stream = Box::pin(task.into_stream(&backend));
                        let (next, stream) = stream.into_future().await;
                        if let Some(next) = next {
                            return Some((next, (backend.clone(), None, Some(stream))));
                        }
                    };
                    // if let Some(stream) = stream.take() {
                    // let mut stream = Box::pin(stream);
                    // let next = stream.next().await;
                    // return Some((next, (None, Some(stream))));
                    // };
                    None
                }
            },
        ))
    }
    fn metadata() -> Vec<Self::ConstraintType> {
        let mut first = T::metadata();
        let mut second = S::metadata();
        second.append(&mut first);
        second
    }
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
