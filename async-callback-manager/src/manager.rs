use crate::task::{
    AsyncTask, AsyncTaskKind, FutureTask, SpawnedTask, StreamTask, TaskInformation, TaskList,
    TaskOutcome, TaskWaiter,
};
use crate::{Constraint, DEFAULT_STREAM_CHANNEL_SIZE};
use futures::{Stream, StreamExt};
use std::any::TypeId;
use std::future::Future;
use std::sync::Arc;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TaskId(pub(crate) u64);

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

pub struct AsyncCallbackManager<Frntend, Bkend, Md> {
    next_task_id: u64,
    tasks_list: TaskList<Frntend, Bkend, Md>,
    // It could be possible to make this generic instead of dynamic, however this type would then
    // require more type parameters.
    on_task_spawn: Box<DynTaskSpawnCallback<Md>>,
}

/// Temporary struct to store task details before it is added to the task list.
pub(crate) struct TempSpawnedTask<Frntend, Bkend, Md> {
    waiter: TaskWaiter<Frntend, Bkend, Md>,
    type_id: TypeId,
    type_name: &'static str,
    type_debug: Arc<String>,
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
        }
    }
    pub fn with_on_task_spawn_callback(
        mut self,
        cb: impl Fn(TaskInformation<Md>) + 'static,
    ) -> Self {
        self.on_task_spawn = Box::new(cb);
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
        match task {
            AsyncTaskKind::Future(future_task) => {
                let outcome = self.spawn_future_task(backend, future_task, &constraint);
                self.add_task_to_list(outcome, metadata, constraint);
            }
            AsyncTaskKind::Stream(stream_task) => {
                let outcome = self.spawn_stream_task(backend, stream_task, &constraint);
                self.add_task_to_list(outcome, metadata, constraint);
            }
            // Don't call (self.on_task_spawn)() for NoOp.
            AsyncTaskKind::Multi(tasks) => {
                for task in tasks {
                    self.spawn_task(backend, task)
                }
            }
            AsyncTaskKind::NoOp => (),
        }
    }
    fn add_task_to_list(
        &mut self,
        details: TempSpawnedTask<Frntend, Bkend, Md>,
        metadata: Vec<Md>,
        constraint: Option<Constraint<Md>>,
    ) {
        let TempSpawnedTask {
            waiter,
            type_id,
            type_name,
            type_debug,
        } = details;
        let sp = SpawnedTask {
            type_id,
            task_id: TaskId(self.next_task_id),
            type_name,
            type_debug,
            receiver: waiter,
            metadata,
        };
        // At one task per nanosecond, it would take 584.6 years for a library user to
        // trigger overflow.
        //
        // https://www.wolframalpha.com/input?i=2%5E64+nanoseconds
        let new_id = self
            .next_task_id
            .checked_add(1)
            .expect("u64 shouldn't overflow!");
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
    ) -> TempSpawnedTask<Frntend, Bkend, Md>
    where
        Frntend: 'static,
        Bkend: 'static,
        Md: 'static,
    {
        (self.on_task_spawn)(TaskInformation {
            type_id: future_task.type_id,
            type_name: future_task.type_name,
            type_debug: &future_task.type_debug,
            constraint,
        });
        let future = (future_task.task)(backend);
        let handle = tokio::spawn(future);
        TempSpawnedTask {
            waiter: TaskWaiter::Future(handle),
            type_id: future_task.type_id,
            type_name: future_task.type_name,
            type_debug: Arc::new(future_task.type_debug),
        }
    }
    fn spawn_stream_task(
        &self,
        backend: &Bkend,
        stream_task: StreamTask<Frntend, Bkend, Md>,
        constraint: &Option<Constraint<Md>>,
    ) -> TempSpawnedTask<Frntend, Bkend, Md>
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
            type_debug: &type_debug,
            constraint,
        });
        let mut stream = task(backend);
        let (tx, rx) = tokio::sync::mpsc::channel(DEFAULT_STREAM_CHANNEL_SIZE);
        let join_handle = tokio::spawn(async move {
            loop {
                if let Some(mutation) = stream.next().await {
                    // Error could occur here if receiver is dropped.
                    // Doesn't seem to be a big deal to ignore this error.
                    let _ = tx.send(mutation).await;
                    continue;
                }
                return;
            }
        });
        TempSpawnedTask {
            waiter: TaskWaiter::Stream {
                receiver: rx,
                join_handle,
            },
            type_id,
            type_name,
            type_debug: Arc::new(type_debug),
        }
    }
}
