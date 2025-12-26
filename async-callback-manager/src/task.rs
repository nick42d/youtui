use crate::task::dyn_task::{FusedTask, IntoDynFutureTask, IntoDynStreamTask, MaybeDynEq};
use crate::{BackendStreamingTask, BackendTask, Constraint, FrontendMutation, TaskHandler};
use futures::{FutureExt, Stream, StreamExt};
use std::any::{Any, TypeId, type_name};
use std::boxed::Box;
use std::fmt::Debug;

pub mod dyn_task;

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
pub struct MapDynFutureTask<Frntend, Bkend, Md, F> {
    task: Box<dyn IntoDynFutureTask<Frntend, Bkend, Md>>,
    map_fn: F,
}
pub struct MapDynStreamTask<Frntend, Bkend, Md, F> {
    task: Box<dyn IntoDynStreamTask<Frntend, Bkend, Md>>,
    map_fn: F,
}
impl<Frntend, Bkend, Md, F> MaybeDynEq for MapDynFutureTask<Frntend, Bkend, Md, F>
where
    F: 'static,
    Md: 'static,
    Frntend: 'static,
    Bkend: 'static,
{
    fn maybe_dyn_eq(&self, other: &dyn MaybeDynEq) -> Option<bool> {
        todo!()
    }
}
impl<Frntend, Bkend, Md, F> MaybeDynEq for MapDynStreamTask<Frntend, Bkend, Md, F>
where
    F: 'static,
    Md: 'static,
    Frntend: 'static,
    Bkend: 'static,
{
    fn maybe_dyn_eq(&self, other: &dyn MaybeDynEq) -> Option<bool> {
        todo!()
    }
}
impl<F, Frntend, NewFrntend, Bkend, Md> IntoDynFutureTask<NewFrntend, Bkend, Md>
    for MapDynFutureTask<Frntend, Bkend, Md, F>
where
    F: Fn(&mut NewFrntend) -> &mut Frntend + Clone + Send + 'static,
    Md: 'static,
    Frntend: 'static,
    Bkend: 'static,
{
    fn into_dyn_task(self: Box<Self>) -> dyn_task::DynFutureTask<NewFrntend, Bkend, Md> {
        todo!()
    }
}
impl<F, Frntend, NewFrntend, Bkend, Md> IntoDynStreamTask<NewFrntend, Bkend, Md>
    for MapDynStreamTask<Frntend, Bkend, Md, F>
where
    F: Fn(&mut NewFrntend) -> &mut Frntend + Clone + Send + 'static,
    Md: 'static,
    Frntend: 'static,
    Bkend: 'static,
{
    fn into_dyn_stream(self: Box<Self>) -> dyn_task::DynStreamTask<NewFrntend, Bkend, Md> {
        todo!()
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
    pub fn new_future_eq<R>(
        request: R,
        handler: impl TaskHandler<R::Output, Frntend, Bkend, Md> + Send + PartialEq + 'static,
        constraint: Option<Constraint<Md>>,
    ) -> AsyncTask<Frntend, Bkend, Md>
    where
        R: BackendTask<Bkend, MetadataType = Md> + Send + Debug + PartialEq + 'static,
        Bkend: 'static,
        Frntend: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
        let type_debug = format!("{request:?}");
        let task = Box::new(FusedTask::new_future_eq(request, handler));
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
    pub fn new_future_with_closure_handler<R>(
        request: R,
        handler: impl FnOnce(&mut Frntend, R::Output) + Send + 'static,
        constraint: Option<Constraint<Md>>,
    ) -> AsyncTask<Frntend, Bkend, Md>
    where
        R: BackendTask<Bkend, MetadataType = Md> + Debug + 'static,
        Bkend: 'static,
        Frntend: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
        let type_debug = format!("{request:?}");
        let handler = |frontend: &mut Frntend, output| {
            handler(frontend, output);
            AsyncTask::new_no_op()
        };
        let task = Box::new(FusedTask::new_future_with_closure_handler(request, handler));
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
    pub fn new_future_with_closure_handler_chained<R>(
        request: R,
        handler: impl FnOnce(&mut Frntend, R::Output) -> AsyncTask<Frntend, Bkend, Md> + Send + 'static,
        constraint: Option<Constraint<Md>>,
    ) -> AsyncTask<Frntend, Bkend, Md>
    where
        R: BackendTask<Bkend, MetadataType = Md> + Debug + 'static,
        Bkend: 'static,
        Frntend: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
        let type_debug = format!("{request:?}");
        let task = Box::new(FusedTask::new_future_with_closure_handler(request, handler));
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
    pub fn new_stream_eq<R>(
        request: R,
        // TODO: Review Clone bounds.
        handler: impl TaskHandler<R::Output, Frntend, Bkend, Md> + Send + PartialEq + Clone + 'static,
        constraint: Option<Constraint<Md>>,
    ) -> AsyncTask<Frntend, Bkend, Md>
    where
        R: BackendStreamingTask<Bkend, MetadataType = Md> + Send + Debug + PartialEq + 'static,
        Bkend: 'static,
        Frntend: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
        let type_debug = format!("{request:?}");
        let task = Box::new(FusedTask::new_stream_eq(request, handler));
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
    pub fn new_stream_with_closure_handler<R>(
        request: R,
        // TODO: Review Clone bounds.
        handler: impl FnOnce(&mut Frntend, R::Output) + Send + Clone + 'static,
        constraint: Option<Constraint<Md>>,
    ) -> AsyncTask<Frntend, Bkend, Md>
    where
        R: BackendStreamingTask<Bkend, MetadataType = Md> + Debug + 'static,
        Bkend: 'static,
        Frntend: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
        let type_debug = format!("{request:?}");
        let handler = |frontend: &mut Frntend, output| {
            handler(frontend, output);
            AsyncTask::new_no_op()
        };
        let task = Box::new(FusedTask::new_stream_with_closure_handler(request, handler));
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
    pub fn new_stream_with_closure_handler_chained<R>(
        request: R,
        // TODO: Review Clone bounds.
        handler: impl FnOnce(&mut Frntend, R::Output) -> AsyncTask<Frntend, Bkend, Md>
        + Send
        + Clone
        + 'static,
        constraint: Option<Constraint<Md>>,
    ) -> AsyncTask<Frntend, Bkend, Md>
    where
        R: BackendStreamingTask<Bkend, MetadataType = Md> + Debug + 'static,
        Bkend: 'static,
        Frntend: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
        let type_debug = format!("{request:?}");
        let task = Box::new(FusedTask::new_stream_with_closure_handler(request, handler));
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
    pub fn map<NewFrntend>(
        self,
        f: impl Fn(&mut NewFrntend) -> &mut Frntend + Clone + Send + 'static,
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
                let mapped = v.into_iter().map(|task| task.map(f.clone())).collect();
                AsyncTask {
                    task: AsyncTaskKind::Multi(mapped),
                    constraint,
                    metadata,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{AsyncTask, BackendStreamingTask, BackendTask};
    use futures::StreamExt;
    #[derive(Debug)]
    struct Task1;
    #[derive(Debug)]
    struct Task2;
    #[derive(Debug)]
    struct StreamingTask;
    impl BackendTask<()> for Task1 {
        type Output = ();
        type MetadataType = ();
        #[allow(clippy::manual_async_fn)]
        fn into_future(
            self,
            _: &(),
        ) -> impl std::future::Future<Output = Self::Output> + Send + 'static {
            async {}
        }
    }
    impl BackendTask<()> for Task2 {
        type Output = ();
        type MetadataType = ();
        #[allow(clippy::manual_async_fn)]
        fn into_future(
            self,
            _: &(),
        ) -> impl std::future::Future<Output = Self::Output> + Send + 'static {
            async {}
        }
    }
    impl BackendStreamingTask<()> for StreamingTask {
        type Output = ();
        type MetadataType = ();
        fn into_stream(
            self,
            _: &(),
        ) -> impl futures::Stream<Item = Self::Output> + Send + Unpin + 'static {
            futures::stream::once(async move {}).boxed()
        }
    }
    #[tokio::test]
    async fn test_recursive_map() {
        let recursive_task = AsyncTask::new_stream_with_closure_handler_chained(
            StreamingTask,
            |_: &mut (), _| {
                AsyncTask::new_future_with_closure_handler_chained(
                    Task1,
                    |_: &mut (), _| {
                        AsyncTask::new_future_with_closure_handler(Task2, |_: &mut (), _| {}, None)
                    },
                    None,
                )
            },
            None,
        );
        // Here, it's expected that this is succesful.
        // TODO: Run the task for an expected outcome.
        #[allow(unused_must_use)]
        let _ = recursive_task.map(|tmp: &mut ()| tmp);
    }
}
