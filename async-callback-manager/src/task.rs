use crate::{
    BackendStreamingTask, BackendTask, DynFutureMutation, DynFutureTask, DynStateMutation,
    DynStreamMutation, DynStreamTask, KillHandle, TaskId,
};
use futures::{stream::FuturesUnordered, StreamExt};
use std::any::{type_name, TypeId};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

pub struct AsyncTask<Frntend, Bkend, Md> {
    pub(crate) task: AsyncTaskKind<Frntend, Bkend, Md>,
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) constraint: Option<Constraint<Md>>,
    pub(crate) metadata: Vec<Md>,
}

pub(crate) enum AsyncTaskKind<Frntend, Bkend, Md> {
    Future(DynFutureTask<Frntend, Bkend, Md>),
    Stream(DynStreamTask<Frntend, Bkend, Md>),
    NoOp,
}

impl<Frntend, Bkend, Md> AsyncTask<Frntend, Bkend, Md> {
    pub fn new_no_op() -> AsyncTask<Frntend, Bkend, Md> {
        Self {
            task: AsyncTaskKind::NoOp,
            constraint: None,
            metadata: vec![],
            type_id: todo!(),
            type_name: todo!(),
        }
    }
    pub fn new_future<R>(
        request: R,
        handler: impl FnOnce(&mut Frntend, R::Output) + Send + 'static,
        constraint: Option<Constraint<Md>>,
    ) -> AsyncTask<Frntend, Bkend, Md>
    where
        R: BackendTask<Bkend, MetadataType = Md> + 'static,
        Bkend: Send + 'static,
        Frntend: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
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
            }) as DynFutureMutation<Frntend, Bkend, Md>
        }) as DynFutureTask<Frntend, Bkend, Md>;
        AsyncTask {
            task: AsyncTaskKind::Future(task),
            constraint,
            metadata,
            type_id,
            type_name,
        }
    }
    pub fn new_stream<R>(
        request: R,
        // TODO: Review Clone bounds.
        handler: impl FnOnce(&mut Frntend, R::Output) + Send + Clone + 'static,
        constraint: Option<Constraint<Md>>,
    ) -> AsyncTask<Frntend, Bkend, Md>
    where
        R: BackendStreamingTask<Bkend, MetadataType = Md> + 'static,
        Bkend: Send + 'static,
        Frntend: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
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
            }) as DynStreamMutation<Frntend, Bkend, Md>
        }) as DynStreamTask<Frntend, Bkend, Md>;
        AsyncTask {
            task: AsyncTaskKind::Stream(task),
            constraint,
            metadata,
            type_id,
            type_name,
        }
    }
}

pub(crate) struct TaskList<Bkend, Frntend, Md> {
    pub inner: Vec<SpawnedTask<Bkend, Frntend, Md>>,
}

pub(crate) struct SpawnedTask<Frntend, Bkend, Md> {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) receiver: TaskWaiter<Frntend, Bkend, Md>,
    pub(crate) task_id: TaskId,
    pub(crate) metadata: Vec<Md>,
}

// User visible struct for introspection.
#[derive(Debug, Clone)]
pub struct ResponseInformation {
    pub type_id: TypeId,
    pub type_name: &'static str,
    pub sender_id: (),
    pub task_id: TaskId,
    pub task_is_now_finished: bool,
}

// User visible struct for introspection.
#[derive(Debug, Clone)]
pub struct TaskInformation<'a, Cstrnt> {
    pub type_id: TypeId,
    pub type_name: &'static str,
    pub sender_id: (),
    pub constraint: &'a Option<Constraint<Cstrnt>>,
}

// pub(crate) struct TaskFromFrontend<Bkend, Cstrnt> {
//     pub(crate) type_id: TypeId,
//     pub(crate) type_name: &'static str,
//     pub(crate) metadata: Vec<Cstrnt>,
//     pub(crate) task: DynBackendTask<Bkend>,
//     pub(crate) receiver: TaskReceiver,
//     pub(crate) sender_id: SenderId,
//     pub(crate) constraint: Option<Constraint<Cstrnt>>,
//     pub(crate) kill_handle: KillHandle,
// }

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
        kill_handle: KillHandle,
    },
}

impl<Frntend, Bkend, Md> TaskWaiter<Frntend, Bkend, Md> {
    fn kill(&mut self) -> crate::Result<()> {
        match self {
            TaskWaiter::Future(handle) => Ok(handle.abort()),
            TaskWaiter::Stream { kill_handle, .. } => kill_handle.kill(),
        }
    }
}

impl<Bkend, Frntend, Md: PartialEq> TaskList<Frntend, Bkend, Md> {
    pub(crate) fn new() -> Self {
        Self { inner: vec![] }
    }
    /// Returns Some(ResponseInformation, Option<DynFallibleFuture>) if a task
    /// existed in the list, and it was processed. Returns None, if no tasks
    /// were in the list. The DynFallibleFuture represents a future that
    /// forwards messages from the manager back to the sender.
    // TODO: How do I indicate the difference between None (no tasks in list) and
    // None (stream closed)?
    pub(crate) async fn process_next_response(
        &mut self,
    ) -> Option<(
        DynStateMutation<Frntend, Bkend, Md>,
        TypeId,
        &'static str,
        TaskId,
    )> {
        let task_completed = self
            .inner
            .iter_mut()
            .enumerate()
            .map(|(idx, task)| async move {
                match task.receiver {
                    TaskWaiter::Future(ref mut receiver) => {
                        let Ok(mutation) = receiver.await else {
                            todo!()
                        };

                        (
                            Some(idx),
                            Some(mutation),
                            task.type_id,
                            task.type_name,
                            task.task_id,
                        )
                    }
                    TaskWaiter::Stream {
                        ref mut receiver, ..
                    } => {
                        if let Some(mutation) = receiver.recv().await {
                            return (
                                None,
                                Some(mutation),
                                task.type_id,
                                task.type_name,
                                task.task_id,
                            );
                        }
                        (Some(idx), None, task.type_id, task.type_name, task.task_id)
                    }
                }
            })
            .collect::<FuturesUnordered<_>>()
            .next()
            .await;
        let (maybe_completed_id, maybe_mutation, type_id, type_name, task_id) = task_completed?;
        if let Some(task_completed) = maybe_completed_id {
            // Safe - this value is in range as produced from enumerate on
            // original list.
            self.inner.swap_remove(task_completed);
        };
        Some((maybe_mutation?, type_id, type_name, task_id))
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
                    // TODO: Handle this condition better.
                    task.receiver.kill().expect("Task should still be alive");
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
