use crate::{
    task::{
        AsyncTask, AsyncTaskKind, FutureTask, ResponseInformation, SpawnedTask, StreamTask,
        TaskInformation, TaskList, TaskOutcome, TaskWaiter,
    },
    Constraint, DEFAULT_STREAM_CHANNEL_SIZE,
};
use futures::{Stream, StreamExt};
use std::{any::TypeId, future::Future};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TaskId(pub(crate) usize);

pub(crate) type DynStateMutation<Frntend, Bkend, Md> =
    Box<dyn FnOnce(&mut Frntend) -> AsyncTask<Frntend, Bkend, Md> + Send>;
pub(crate) type DynMutationFuture<Frntend, Bkend, Md> =
    Box<dyn Future<Output = DynStateMutation<Frntend, Bkend, Md>> + Unpin + Send>;
pub(crate) type DynMutationStream<Frntend, Bkend, Md> =
    Box<dyn Stream<Item = DynStateMutation<Frntend, Bkend, Md>> + Unpin + Send>;
pub(crate) type DynFutureTask<Frntend, Bkend, Md> =
    Box<dyn FnOnce(&Bkend) -> DynMutationFuture<Frntend, Bkend, Md>>;
pub(crate) type DynStreamTask<Frntend, Bkend, Md> =
    Box<dyn FnOnce(&Bkend) -> DynMutationStream<Frntend, Bkend, Md>>;

pub(crate) type DynTaskSpawnCallback<Cstrnt> = dyn Fn(TaskInformation<Cstrnt>);
pub(crate) type DynResponseReceivedCallback = dyn Fn(ResponseInformation);

pub struct AsyncCallbackManager<Frntend, Bkend, Md> {
    next_task_id: usize,
    tasks_list: TaskList<Frntend, Bkend, Md>,
    // It could be possible to make these generic instead of dynamic, however this type would then
    // require 2 more type parameters.
    on_task_spawn: Box<DynTaskSpawnCallback<Md>>,
    on_response_received: Box<DynResponseReceivedCallback>,
    on_id_overflow: Box<dyn Fn()>,
}

impl<Frntend, Bkend, Md: PartialEq> Default for AsyncCallbackManager<Frntend, Bkend, Md> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Frntend, Bkend, Md: PartialEq> AsyncCallbackManager<Frntend, Bkend, Md> {
    /// Get a new AsyncCallbackManager.
    pub fn new() -> Self {
        Self {
            next_task_id: Default::default(),
            tasks_list: TaskList::new(),
            on_task_spawn: Box::new(|_| {}),
            on_response_received: Box::new(|_| {}),
            on_id_overflow: Box::new(|| {}),
        }
    }
    pub fn with_on_id_overflow_callback(mut self, cb: impl Fn() + 'static) -> Self {
        self.on_id_overflow = Box::new(cb);
        self
    }
    pub fn with_on_task_spawn_callback(
        mut self,
        cb: impl Fn(TaskInformation<Md>) + 'static,
    ) -> Self {
        self.on_task_spawn = Box::new(cb);
        self
    }
    // TODO: when is this called?
    pub fn with_on_response_received_callback(
        mut self,
        cb: impl Fn(ResponseInformation) + 'static,
    ) -> Self {
        todo!();
        self.on_response_received = Box::new(cb);
        self
    }
    /// Await for the next response from one of the spawned tasks, or returns
    /// None if no tasks were in the list.
    pub async fn get_next_response(&mut self) -> Option<TaskOutcome<Frntend, Bkend, Md>> {
        self.tasks_list.get_next_response().await
    }
    pub fn spawn_task(&mut self, backend: &Bkend, task: AsyncTask<Frntend, Bkend, Md>)
    where
        Frntend: 'static,
        Bkend: 'static,
        Md: 'static,
    {
        let AsyncTask {
            task,
            constraint,
            metadata,
        } = task;
        let (waiter, type_id, type_name) = match task {
            AsyncTaskKind::Future(future_task) => {
                self.spawn_future_task(backend, future_task, &constraint)
            }
            AsyncTaskKind::Stream(stream_task) => {
                self.spawn_stream_task(backend, stream_task, &constraint)
            }
            // Don't call (self.on_task_spawn)() for NoOp.
            AsyncTaskKind::NoOp => return,
        };
        let sp = SpawnedTask {
            type_id,
            task_id: TaskId(self.next_task_id),
            type_name,
            receiver: waiter,
            metadata,
        };
        let (new_id, overflowed) = self.next_task_id.overflowing_add(1);
        if overflowed {
            // Note that the danger of overflow is that kill/block will stop working
            // correctly. An alternative could be to use a BigInt such as
            // ibig::Uint for the ids, however this would make TaskId no longer Copy.
            (self.on_id_overflow)()
        }
        self.next_task_id = new_id;
        if let Some(constraint) = constraint {
            self.tasks_list.handle_constraint(constraint, type_id);
        }
        self.tasks_list.push(sp);
    }
    fn spawn_future_task(
        &self,
        backend: &Bkend,
        future_task: FutureTask<Frntend, Bkend, Md>,
        constraint: &Option<Constraint<Md>>,
    ) -> (TaskWaiter<Frntend, Bkend, Md>, TypeId, &'static str)
    where
        Frntend: 'static,
        Bkend: 'static,
        Md: 'static,
    {
        (self.on_task_spawn)(TaskInformation {
            type_id: future_task.type_id,
            type_name: future_task.type_name,
            type_debug: future_task.type_debug,
            constraint,
        });
        let future = (future_task.task)(backend);
        let handle = tokio::spawn(future);
        (
            TaskWaiter::Future(handle),
            future_task.type_id,
            future_task.type_name,
        )
    }
    fn spawn_stream_task(
        &self,
        backend: &Bkend,
        stream_task: StreamTask<Frntend, Bkend, Md>,
        constraint: &Option<Constraint<Md>>,
    ) -> (TaskWaiter<Frntend, Bkend, Md>, TypeId, &'static str)
    where
        Frntend: 'static,
        Bkend: 'static,
        Md: 'static,
    {
        let StreamTask {
            task,
            type_id,
            type_name,
            type_debug,
        } = stream_task;
        (self.on_task_spawn)(TaskInformation {
            type_id,
            type_name,
            type_debug,
            constraint,
        });
        let mut stream = task(backend);
        let (tx, rx) = tokio::sync::mpsc::channel(DEFAULT_STREAM_CHANNEL_SIZE);
        let abort_handle = tokio::spawn(async move {
            loop {
                if let Some(mutation) = stream.next().await {
                    // Error could occur here if receiver is dropped.
                    // Doesn't seem to be a big deal to ignore this error.
                    let _ = tx.send(mutation).await;
                    continue;
                }
                return;
            }
        })
        .abort_handle();
        (
            TaskWaiter::Stream {
                receiver: rx,
                abort_handle,
            },
            type_id,
            type_name,
        )
    }
}
