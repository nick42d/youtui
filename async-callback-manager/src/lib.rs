use futures::Future;
use futures::Stream;
use std::any::Any;

mod adaptors;
mod error;
mod manager;
mod task;

pub use adaptors::*;
pub use error::*;
pub use manager::*;
pub use task::{AsyncTask, Constraint, TaskOutcome};

// Size of the channel used for each stream task.
// In future, this could be settable.
pub(crate) const DEFAULT_STREAM_CHANNEL_SIZE: usize = 20;

pub trait BkendMap<Bkend> {
    fn map(backend: &Bkend) -> &Self;
}

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
