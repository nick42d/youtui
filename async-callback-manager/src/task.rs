use crate::task::dyn_task::{
    FusedTask, IntoDynFutureTask, IntoDynStreamTask, OptionHandler, TryHandler,
};
use crate::task::map::{MapDynFutureTask, MapDynStreamTask};
use crate::{BackendStreamingTask, BackendTask, Constraint, TaskHandler};
use std::any::{TypeId, type_name};
use std::boxed::Box;
use std::fmt::Debug;

pub mod dyn_task;
mod map;
#[cfg(test)]
mod tests;

/// An asynchrnonous task that can generate state mutations and/or more tasks to
/// be spawned by an AsyncCallbackManager.
#[must_use = "AsyncTasks do nothing unless you run them"]
pub struct AsyncTask<Frntend, Bkend, Md> {
    pub(crate) task: AsyncTaskKind<Frntend, Bkend, Md>,
    pub(crate) constraint: Option<Constraint<Md>>,
    pub(crate) metadata: Vec<Md>,
}
pub(crate) enum AsyncTaskKind<Frntend, Bkend, Md> {
    Future(FutureTask<Frntend, Bkend, Md>),
    Stream(StreamTask<Frntend, Bkend, Md>),
    Multi(Vec<AsyncTask<Frntend, Bkend, Md>>),
    NoOp,
}
pub(crate) struct FutureTask<Frntend, Bkend, Md> {
    pub(crate) task: Box<dyn IntoDynFutureTask<Frntend, Bkend, Md>>,
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) type_debug: String,
}
pub(crate) struct StreamTask<Frntend, Bkend, Md> {
    pub(crate) task: Box<dyn IntoDynStreamTask<Frntend, Bkend, Md>>,
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) type_debug: String,
}

// Allow conversion of () into no-op Task.
impl<Frntend, Bkend, Md> From<()> for AsyncTask<Frntend, Bkend, Md> {
    fn from(_: ()) -> Self {
        AsyncTask::new_no_op()
    }
}

// Debug must be implemented manually to remove Frntend, Bkend Debug bounds.
impl<Frntend, Bkend, Md> Debug for AsyncTask<Frntend, Bkend, Md>
where
    Md: Debug,
    AsyncTaskKind<Frntend, Bkend, Md>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncTask")
            .field("task", &self.task)
            .field("constraint", &self.constraint)
            .field("metadata", &self.metadata)
            .finish()
    }
}
// Debug must be implemented manually to remove Frntend, Bkend Debug bounds.
impl<Frntend, Bkend, Md> Debug for AsyncTaskKind<Frntend, Bkend, Md>
where
    Md: Debug,
    FutureTask<Frntend, Bkend, Md>: Debug,
    StreamTask<Frntend, Bkend, Md>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Future(arg0) => f.debug_tuple("Future").field(arg0).finish(),
            Self::Stream(arg0) => f.debug_tuple("Stream").field(arg0).finish(),
            Self::Multi(arg0) => f.debug_tuple("Multi").field(arg0).finish(),
            Self::NoOp => write!(f, "NoOp"),
        }
    }
}
// Debug must be implemented manually to remove Frntend, Bkend Debug bounds.
impl<Frntend, Bkend, Md> Debug for FutureTask<Frntend, Bkend, Md>
where
    dyn IntoDynFutureTask<Frntend, Bkend, Md>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FutureTask")
            .field("task", &self.task)
            .field("type_id", &self.type_id)
            .field("type_name", &self.type_name)
            .field("type_debug", &self.type_debug)
            .finish()
    }
}
// Debug must be implemented manually to remove Frntend, Bkend Debug bounds.
impl<Frntend, Bkend, Md> Debug for StreamTask<Frntend, Bkend, Md>
where
    dyn IntoDynStreamTask<Frntend, Bkend, Md>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamTask")
            .field("task", &self.task)
            .field("type_id", &self.type_id)
            .field("type_name", &self.type_name)
            .field("type_debug", &self.type_debug)
            .finish()
    }
}

// PartialEq must be implemented manually to remove Frntend, Bkend PartialEq
// bounds.
impl<Frntend, Bkend, Md> PartialEq for AsyncTask<Frntend, Bkend, Md>
where
    Md: PartialEq + 'static,
    Frntend: 'static,
    Bkend: 'static,
    AsyncTaskKind<Frntend, Bkend, Md>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.task == other.task
            && self.constraint == other.constraint
            && self.metadata == other.metadata
    }
}
// PartialEq must be implemented manually to remove Frntend, Bkend PartialEq
// bounds.
impl<Frntend, Bkend, Md> PartialEq for AsyncTaskKind<Frntend, Bkend, Md>
where
    Md: PartialEq + 'static,
    Frntend: 'static,
    Bkend: 'static,
    FutureTask<Frntend, Bkend, Md>: PartialEq,
    StreamTask<Frntend, Bkend, Md>: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Future(l0), Self::Future(r0)) => l0 == r0,
            (Self::Stream(l0), Self::Stream(r0)) => l0 == r0,
            (Self::Multi(l0), Self::Multi(r0)) => l0 == r0,
            (Self::NoOp, Self::NoOp) => true,
            _ => false,
        }
    }
}
// PartialEq must be implemented manually to remove Frntend, Bkend PartialEq
// bounds and use dyn_partial_eq function.
#[cfg(feature = "task-equality")]
impl<Frntend, Bkend, Md> PartialEq for FutureTask<Frntend, Bkend, Md>
where
    Frntend: 'static,
    Bkend: 'static,
    Md: 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.task.dyn_partial_eq(other.task.as_ref())
            && self.type_id == other.type_id
            && self.type_name == other.type_name
            && self.type_debug == other.type_debug
    }
}
// PartialEq must be implemented manually to remove Frntend, Bkend PartialEq
// bounds and use dyn_partial_eq function.
#[cfg(feature = "task-equality")]
impl<Frntend, Bkend, Md> PartialEq for StreamTask<Frntend, Bkend, Md>
where
    Frntend: 'static,
    Bkend: 'static,
    Md: 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.task.dyn_partial_eq(other.task.as_ref())
            && self.type_id == other.type_id
            && self.type_name == other.type_name
            && self.type_debug == other.type_debug
    }
}

impl<Frntend, Bkend, Md> FromIterator<AsyncTask<Frntend, Bkend, Md>>
    for AsyncTask<Frntend, Bkend, Md>
{
    fn from_iter<T: IntoIterator<Item = AsyncTask<Frntend, Bkend, Md>>>(iter: T) -> Self {
        let v = iter.into_iter().collect();
        // TODO: Better handle constraints / metadata.
        AsyncTask {
            task: AsyncTaskKind::Multi(v),
            constraint: None,
            metadata: vec![],
        }
    }
}

impl<Frntend, Bkend, Md> AsyncTask<Frntend, Bkend, Md>
where
    Md: PartialEq + 'static,
    Frntend: 'static,
    Bkend: 'static,
    Self: PartialEq,
{
    /// Assert that this effect contains at least other effect (it may contain
    /// multiple effects).
    pub fn contains(&self, other: &AsyncTask<Frntend, Bkend, Md>) -> bool {
        match &self.task {
            AsyncTaskKind::Multi(self_tasks) => {
                // Contains is used here to guard against nested multi tasks
                self_tasks.iter().any(|self_task| self_task.contains(other))
            }
            _ => self == other,
        }
    }
}

impl<Frntend, Bkend, Md> AsyncTask<Frntend, Bkend, Md> {
    pub fn push(self, next: AsyncTask<Frntend, Bkend, Md>) -> AsyncTask<Frntend, Bkend, Md> {
        match self.task {
            AsyncTaskKind::Future(_) | AsyncTaskKind::Stream(_) => {
                let v = vec![self, next];
                AsyncTask {
                    task: AsyncTaskKind::Multi(v),
                    constraint: None,
                    metadata: vec![],
                }
            }
            AsyncTaskKind::Multi(mut m) => {
                m.push(next);
                AsyncTask {
                    task: AsyncTaskKind::Multi(m),
                    constraint: self.constraint,
                    metadata: self.metadata,
                }
            }
            AsyncTaskKind::NoOp => next,
        }
    }
    pub fn is_no_op(&self) -> bool {
        matches!(self.task, AsyncTaskKind::NoOp)
    }
    pub fn new_no_op() -> AsyncTask<Frntend, Bkend, Md> {
        Self {
            task: AsyncTaskKind::NoOp,
            constraint: None,
            metadata: vec![],
        }
    }
    pub fn new_future<R>(
        request: R,
        handler: impl TaskHandler<R::Output, Frntend, Bkend, Md> + Send + 'static,
        constraint: Option<Constraint<Md>>,
    ) -> AsyncTask<Frntend, Bkend, Md>
    where
        R: BackendTask<Bkend, MetadataType = Md> + Send + Debug + 'static,
        Bkend: 'static,
        Frntend: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
        let type_debug = format!("{request:?}");
        let task = Box::new(FusedTask::new_future(request, handler));
        let task = FutureTask {
            task,
            type_id,
            type_name,
            type_debug,
        };
        AsyncTask {
            task: AsyncTaskKind::Future(task),
            constraint,
            metadata,
        }
    }
    pub fn new_future_try<R, T, E>(
        request: R,
        ok_handler: impl TaskHandler<T, Frntend, Bkend, Md> + Send + 'static,
        err_handler: impl TaskHandler<E, Frntend, Bkend, Md> + Send + 'static,
        constraint: Option<Constraint<Md>>,
    ) -> AsyncTask<Frntend, Bkend, Md>
    where
        R: BackendTask<Bkend, MetadataType = Md, Output = Result<T, E>> + Send + Debug + 'static,
        Bkend: 'static,
        Frntend: 'static,
        E: 'static,
        T: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
        let type_debug = format!("{request:?}");
        let task = Box::new(FusedTask::new_future(
            request,
            TryHandler {
                ok_handler,
                err_handler,
            },
        ));
        let task = FutureTask {
            task,
            type_id,
            type_name,
            type_debug,
        };
        AsyncTask {
            task: AsyncTaskKind::Future(task),
            constraint,
            metadata,
        }
    }
    pub fn new_future_option<R, T>(
        request: R,
        some_handler: impl TaskHandler<T, Frntend, Bkend, Md> + Send + 'static,
        constraint: Option<Constraint<Md>>,
    ) -> AsyncTask<Frntend, Bkend, Md>
    where
        R: BackendTask<Bkend, MetadataType = Md, Output = Option<T>> + Send + Debug + 'static,
        Bkend: 'static,
        Frntend: 'static,
        T: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
        let type_debug = format!("{request:?}");
        let task = Box::new(FusedTask::new_future(request, OptionHandler(some_handler)));
        let task = FutureTask {
            task,
            type_id,
            type_name,
            type_debug,
        };
        AsyncTask {
            task: AsyncTaskKind::Future(task),
            constraint,
            metadata,
        }
    }
    pub fn new_stream<R>(
        request: R,
        // TODO: Review Clone bounds.
        handler: impl TaskHandler<R::Output, Frntend, Bkend, Md> + Send + Clone + 'static,
        constraint: Option<Constraint<Md>>,
    ) -> AsyncTask<Frntend, Bkend, Md>
    where
        R: BackendStreamingTask<Bkend, MetadataType = Md> + Send + Debug + 'static,
        Bkend: 'static,
        Frntend: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
        let type_debug = format!("{request:?}");
        let task = Box::new(FusedTask::new_stream(request, handler));
        let task = StreamTask {
            task,
            type_id,
            type_name,
            type_debug,
        };
        AsyncTask {
            task: AsyncTaskKind::Stream(task),
            constraint,
            metadata,
        }
    }
    pub fn new_stream_try<R, T, E>(
        request: R,
        ok_handler: impl TaskHandler<T, Frntend, Bkend, Md> + Send + Clone + 'static,
        err_handler: impl TaskHandler<E, Frntend, Bkend, Md> + Send + Clone + 'static,
        constraint: Option<Constraint<Md>>,
    ) -> AsyncTask<Frntend, Bkend, Md>
    where
        R: BackendStreamingTask<Bkend, MetadataType = Md, Output = Result<T, E>>
            + Send
            + Debug
            + 'static,
        Bkend: 'static,
        Frntend: 'static,
        E: 'static,
        T: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
        let type_debug = format!("{request:?}");
        let task = Box::new(FusedTask::new_stream(
            request,
            TryHandler {
                ok_handler,
                err_handler,
            },
        ));
        let task = StreamTask {
            task,
            type_id,
            type_name,
            type_debug,
        };
        AsyncTask {
            task: AsyncTaskKind::Stream(task),
            constraint,
            metadata,
        }
    }
    pub fn new_stream_option<R, T>(
        request: R,
        some_handler: impl TaskHandler<T, Frntend, Bkend, Md> + Send + Clone + 'static,
        constraint: Option<Constraint<Md>>,
    ) -> AsyncTask<Frntend, Bkend, Md>
    where
        R: BackendStreamingTask<Bkend, MetadataType = Md, Output = Option<T>>
            + Send
            + Debug
            + 'static,
        Bkend: 'static,
        Frntend: 'static,
        T: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
        let type_debug = format!("{request:?}");
        let task = Box::new(FusedTask::new_stream(request, OptionHandler(some_handler)));
        let task = StreamTask {
            task,
            type_id,
            type_name,
            type_debug,
        };
        AsyncTask {
            task: AsyncTaskKind::Stream(task),
            constraint,
            metadata,
        }
    }
    /// # Warning
    /// This is recursive, if you have set up a cycle of AsyncTasks, map may
    /// overflow.
    pub fn map_frontend<NewFrntend>(
        self,
        f: impl FnOnce(&mut NewFrntend) -> &mut Frntend + Clone + Send + 'static,
    ) -> AsyncTask<NewFrntend, Bkend, Md>
    where
        Bkend: 'static,
        Frntend: 'static,
        Md: 'static,
    {
        let Self {
            task,
            constraint,
            metadata,
        } = self;
        match task {
            AsyncTaskKind::Future(FutureTask {
                task,
                type_id,
                type_name,
                type_debug,
            }) => {
                let map = MapDynFutureTask { task, map_fn: f };
                let task = Box::new(map);
                let task = FutureTask {
                    task,
                    type_id,
                    type_name,
                    type_debug,
                };
                AsyncTask {
                    task: AsyncTaskKind::Future(task),
                    constraint,
                    metadata,
                }
            }
            AsyncTaskKind::Stream(StreamTask {
                task,
                type_id,
                type_name,
                type_debug,
            }) => {
                let map = MapDynStreamTask { task, map_fn: f };
                let task = Box::new(map);
                let task = StreamTask {
                    task,
                    type_id,
                    type_name,
                    type_debug,
                };
                AsyncTask {
                    task: AsyncTaskKind::Stream(task),
                    constraint,
                    metadata,
                }
            }
            AsyncTaskKind::NoOp => AsyncTask {
                task: AsyncTaskKind::NoOp,
                constraint,
                metadata,
            },
            AsyncTaskKind::Multi(v) => {
                let mapped = v
                    .into_iter()
                    .map(|task| task.map_frontend(f.clone()))
                    .collect();
                AsyncTask {
                    task: AsyncTaskKind::Multi(mapped),
                    constraint,
                    metadata,
                }
            }
        }
    }
}
