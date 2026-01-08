use futures::{Future, Stream};
use std::any::Any;

mod adaptors;
mod constraint;
mod error;
mod manager;
mod panicking_receiver_stream;
mod task;

pub use adaptors::*;
pub use constraint::*;
pub use error::*;
pub use manager::task_list::{TaskInformation, TaskOutcome};
pub use manager::*;
pub use panicking_receiver_stream::*;
pub use task::AsyncTask;

// Size of the channel used for each stream task.
// In future, this could be settable.
pub(crate) const DEFAULT_STREAM_CHANNEL_SIZE: usize = 20;

/// A task of kind T that can be run on a backend, returning a future of output
/// Output. The type must implement Any, as the
/// TypeId is used as part of the task management process.
pub trait BackendTask<Bkend>: Send + Any {
    type Output: Send;
    type MetadataType: PartialEq;
    fn into_future(self, backend: &Bkend) -> impl Future<Output = Self::Output> + Send + 'static;
    /// Metadata provides a way of grouping different tasks for use in
    /// constraints, if you override the default implementation.
    fn metadata() -> Vec<Self::MetadataType> {
        vec![]
    }
}

/// A task of kind T that can be run on a backend, returning a stream of outputs
/// Output. The type must implement Any, as the TypeId is used as part of the
/// task management process.
pub trait BackendStreamingTask<Bkend>: Send + Any {
    type Output: Send;
    type MetadataType: PartialEq;
    fn into_stream(
        self,
        backend: &Bkend,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin + 'static;
    /// Metadata provides a way of grouping different tasks for use in
    /// constraints, if you override the default implementation.
    fn metadata() -> Vec<Self::MetadataType> {
        vec![]
    }
}

/// Represents the handler for a task output.
pub trait TaskHandler<Input, Frntend, Bkend, Md> {
    fn handle(self, input: Input) -> impl FrontendEffect<Frntend, Bkend, Md>;
}

/// Represents a mutation that can be applied to some state, returning an
/// effect.
pub trait FrontendEffect<Frntend, Bkend, Md> {
    fn apply(self, target: &mut Frntend) -> AsyncTask<Frntend, Bkend, Md>;
}

/// Helper trait, representing an equality that may be indeterminate for some
/// values, e,g comparing two closures where equality is indeterminate via
/// algorithms due to the halting problem.
///
/// e.g `(|x| x + 2).maybe_eq(|x| x + 1 + 1) == None`
pub trait MaybeEq<T> {
    fn maybe_eq(&self, other: &T) -> Option<bool>;
}
