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

/// Represents the handler for a task output.
pub trait TaskHandler<Output, Frntend, Bkend, Md> {
    fn handle(self, output: Output) -> impl FrontendMutation<Frntend, Bkend = Bkend, Md = Md>;
}

impl<T, Output, Frntend, Bkend, Md> TaskHandler<Output, Frntend, Bkend, Md> for T
where
    T: FnOnce(&mut Frntend, Output) -> AsyncTask<Frntend, Bkend, Md> + Send + 'static,
{
    fn handle(self, output: Output) -> impl FrontendMutation<Frntend, Bkend = Bkend, Md = Md> {
        |frontend: &mut Frntend| self(frontend, output)
    }
}

/// Represents a mutation that can be applied to some state, returning an
/// effect.
pub trait FrontendMutation<Frntend> {
    type Bkend;
    type Md;
    fn apply(self, target: &mut Frntend) -> AsyncTask<Frntend, Self::Bkend, Self::Md>;
}

impl<T, Frntend, Bkend, Md> FrontendMutation<Frntend> for T
where
    T: FnOnce(&mut Frntend) -> AsyncTask<Frntend, Bkend, Md>,
{
    type Bkend = Bkend;
    type Md = Md;
    fn apply(self, target: &mut Frntend) -> AsyncTask<Frntend, Self::Bkend, Self::Md> {
        self(target)
    }
}
