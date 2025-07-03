use crate::{
    BackendStreamingTask, BackendTask, DynFutureTask, DynMutationFuture, DynMutationStream,
    DynStateMutation, DynStreamTask, TaskId,
};
use futures::stream::FuturesUnordered;
use futures::{FutureExt, StreamExt};
use std::any::{type_name, TypeId};
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::{AbortHandle, JoinError, JoinHandle};

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

pub(crate) struct StreamTask<Frntend, Bkend, Md> {
    pub(crate) task: DynStreamTask<Frntend, Bkend, Md>,
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) type_debug: String,
}

pub(crate) struct FutureTask<Frntend, Bkend, Md> {
    pub(crate) task: DynFutureTask<Frntend, Bkend, Md>,
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) type_debug: String,
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
    pub fn new_future<R>(
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
        let task = Box::new(move |b: &Bkend| {
            Box::new({
                let future = request.into_future(b);
                Box::pin(async move {
                    let output = future.await;
                    Box::new(move |frontend: &mut Frntend| {
                        handler(frontend, output);
                        AsyncTask::new_no_op()
                    }) as DynStateMutation<Frntend, Bkend, Md>
                })
            }) as DynMutationFuture<Frntend, Bkend, Md>
        }) as DynFutureTask<Frntend, Bkend, Md>;
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
    pub fn new_future_chained<R>(
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
        let task = Box::new(move |b: &Bkend| {
            Box::new({
                let future = request.into_future(b);
                Box::pin(async move {
                    let output = future.await;
                    Box::new(move |frontend: &mut Frntend| handler(frontend, output))
                        as DynStateMutation<Frntend, Bkend, Md>
                })
            }) as DynMutationFuture<Frntend, Bkend, Md>
        }) as DynFutureTask<Frntend, Bkend, Md>;
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
        let task = Box::new(move |b: &Bkend| {
            let stream = request.into_stream(b);
            Box::new({
                stream.map(move |output| {
                    Box::new({
                        let handler = handler.clone();
                        move |frontend: &mut Frntend| {
                            handler.clone()(frontend, output);
                            AsyncTask::new_no_op()
                        }
                    }) as DynStateMutation<Frntend, Bkend, Md>
                })
            }) as DynMutationStream<Frntend, Bkend, Md>
        }) as DynStreamTask<Frntend, Bkend, Md>;
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
    pub fn new_stream_chained<R>(
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
        let task = Box::new(move |b: &Bkend| {
            let stream = request.into_stream(b);
            Box::new({
                stream.map(move |output| {
                    Box::new({
                        let handler = handler.clone();
                        move |frontend: &mut Frntend| handler.clone()(frontend, output)
                    }) as DynStateMutation<Frntend, Bkend, Md>
                })
            }) as DynMutationStream<Frntend, Bkend, Md>
        }) as DynStreamTask<Frntend, Bkend, Md>;
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
                let task = Box::new(|b: &Bkend| {
                    Box::new(task(b).map(|task| {
                        Box::new(|nf: &mut NewFrntend| {
                            let task = task(f(nf));
                            task.map(f)
                        }) as DynStateMutation<NewFrntend, Bkend, Md>
                    })) as DynMutationFuture<NewFrntend, Bkend, Md>
                }) as DynFutureTask<NewFrntend, Bkend, Md>;
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
                let task = Box::new(|b: &Bkend| {
                    Box::new({
                        task(b).map(move |task| {
                            Box::new({
                                let f = f.clone();
                                move |nf: &mut NewFrntend| {
                                    let task = task(f(nf));
                                    task.map(f.clone())
                                }
                            })
                                as DynStateMutation<NewFrntend, Bkend, Md>
                        })
                    }) as DynMutationStream<NewFrntend, Bkend, Md>
                }) as DynStreamTask<NewFrntend, Bkend, Md>;
                let stream_task = StreamTask {
                    task,
                    type_id,
                    type_name,
                    type_debug,
                };
                AsyncTask {
                    task: AsyncTaskKind::Stream(stream_task),
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

pub(crate) struct TaskList<Bkend, Frntend, Md> {
    pub inner: Vec<SpawnedTask<Bkend, Frntend, Md>>,
}

pub(crate) struct SpawnedTask<Frntend, Bkend, Md> {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) type_debug: Arc<String>,
    pub(crate) receiver: TaskWaiter<Frntend, Bkend, Md>,
    pub(crate) task_id: TaskId,
    pub(crate) metadata: Vec<Md>,
}

/// User visible struct for introspection.
#[derive(Debug, Clone)]
pub struct TaskInformation<'a, Cstrnt> {
    pub type_id: TypeId,
    pub type_name: &'static str,
    pub type_debug: &'a str,
    pub constraint: &'a Option<Constraint<Cstrnt>>,
}

#[derive(Eq, PartialEq, Debug)]
pub struct Constraint<Cstrnt> {
    pub(crate) constraint_type: ConstraitType<Cstrnt>,
}

#[derive(Eq, PartialEq, Debug)]
pub enum ConstraitType<Cstrnt> {
    BlockSameType,
    KillSameType,
    BlockMatchingMetatdata(Cstrnt),
}

pub(crate) enum TaskWaiter<Frntend, Bkend, Md> {
    Future(JoinHandle<DynStateMutation<Frntend, Bkend, Md>>),
    Stream {
        receiver: mpsc::Receiver<DynStateMutation<Frntend, Bkend, Md>>,
        abort_handle: AbortHandle,
    },
}

impl<Frntend, Bkend, Md> TaskWaiter<Frntend, Bkend, Md> {
    fn kill(&mut self) {
        match self {
            TaskWaiter::Future(handle) => handle.abort(),
            TaskWaiter::Stream {
                abort_handle: abort,
                ..
            } => abort.abort(),
        }
    }
}

pub enum TaskOutcome<Frntend, Bkend, Md> {
    /// No task was recieved because a stream closed, but there are still more
    /// tasks.
    StreamClosed,
    /// No task was recieved because the next task panicked.
    /// Currently only applicable to Future type tasks.
    // TODO: Implement for Stream type tasks.
    TaskPanicked {
        error: JoinError,
        type_id: TypeId,
        type_name: &'static str,
        type_debug: Arc<String>,
        task_id: TaskId,
    },
    /// Mutation was received from a task.
    MutationReceived {
        mutation: DynStateMutation<Frntend, Bkend, Md>,
        type_id: TypeId,
        type_name: &'static str,
        type_debug: Arc<String>,
        task_id: TaskId,
    },
}

impl<Bkend, Frntend, Md: PartialEq> TaskList<Frntend, Bkend, Md> {
    pub(crate) fn new() -> Self {
        Self { inner: vec![] }
    }
    /// Await for the next response from one of the spawned tasks.
    pub(crate) async fn get_next_response(&mut self) -> Option<TaskOutcome<Frntend, Bkend, Md>> {
        let task_completed = self
            .inner
            .iter_mut()
            .enumerate()
            .map(|(idx, task)| async move {
                match task.receiver {
                    TaskWaiter::Future(ref mut receiver) => match receiver.await {
                        Ok(mutation) => (
                            Some(idx),
                            TaskOutcome::MutationReceived {
                                mutation,
                                type_id: task.type_id,
                                type_debug: task.type_debug.clone(),
                                task_id: task.task_id,
                                type_name: task.type_name,
                            },
                        ),
                        Err(error) => (
                            Some(idx),
                            TaskOutcome::TaskPanicked {
                                type_id: task.type_id,
                                type_name: task.type_name,
                                type_debug: task.type_debug.clone(),
                                task_id: task.task_id,
                                error,
                            },
                        ),
                    },
                    TaskWaiter::Stream {
                        ref mut receiver, ..
                    } => {
                        if let Some(mutation) = receiver.recv().await {
                            return (
                                None,
                                TaskOutcome::MutationReceived {
                                    mutation,
                                    type_id: task.type_id,
                                    type_name: task.type_name,
                                    task_id: task.task_id,
                                    type_debug: task.type_debug.clone(),
                                },
                            );
                        }
                        (Some(idx), TaskOutcome::StreamClosed)
                    }
                }
            })
            .collect::<FuturesUnordered<_>>()
            .next()
            .await;
        let (maybe_completed_id, outcome) = task_completed?;
        if let Some(completed_id) = maybe_completed_id {
            // Safe - this value is in range as produced from enumerate on
            // original list.
            self.inner.swap_remove(completed_id);
        };
        Some(outcome)
    }
    pub(crate) fn push(&mut self, task: SpawnedTask<Frntend, Bkend, Md>) {
        self.inner.push(task)
    }
    // TODO: Tests
    pub(crate) fn handle_constraint(&mut self, constraint: Constraint<Md>, type_id: TypeId) {
        // TODO: Consider the situation where one component kills tasks belonging to
        // another component.
        //
        // Assuming here that kill implies block also.
        let task_doesnt_match_constraint = |task: &SpawnedTask<_, _, _>| (task.type_id != type_id);
        let task_doesnt_match_metadata =
            |task: &SpawnedTask<_, _, _>, constraint| !task.metadata.contains(constraint);
        match constraint.constraint_type {
            ConstraitType::BlockMatchingMetatdata(metadata) => self
                .inner
                .retain(|task| task_doesnt_match_metadata(task, &metadata)),
            ConstraitType::BlockSameType => {
                self.inner.retain(task_doesnt_match_constraint);
            }
            ConstraitType::KillSameType => self.inner.retain_mut(|task| {
                if !task_doesnt_match_constraint(task) {
                    task.receiver.kill();
                    return false;
                }
                true
            }),
        }
    }
}

impl<Cstrnt> Constraint<Cstrnt> {
    pub fn new_block_same_type() -> Self {
        Self {
            constraint_type: ConstraitType::BlockSameType,
        }
    }
    pub fn new_kill_same_type() -> Self {
        Self {
            constraint_type: ConstraitType::KillSameType,
        }
    }
    pub fn new_block_matching_metadata(metadata: Cstrnt) -> Self {
        Self {
            constraint_type: ConstraitType::BlockMatchingMetatdata(metadata),
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
        let recursive_task = AsyncTask::new_stream_chained(
            StreamingTask,
            |_: &mut (), _| {
                AsyncTask::new_future_chained(
                    Task1,
                    |_: &mut (), _| AsyncTask::new_future(Task2, |_: &mut (), _| {}, None),
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
