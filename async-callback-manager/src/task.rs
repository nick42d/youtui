use crate::{DynBackendTask, DynFallibleFuture, KillHandle, SenderId, TaskId};
use futures::{stream::FuturesUnordered, StreamExt};
use std::any::TypeId;
use tokio::sync::{mpsc, oneshot};

pub(crate) struct TaskList<Cstrnt> {
    pub inner: Vec<Task<Cstrnt>>,
}

// User visible struct for introspection.
#[derive(Debug, Clone)]
pub struct ResponseInformation {
    pub type_id: TypeId,
    pub type_name: &'static str,
    pub sender_id: SenderId,
    pub task_id: TaskId,
    pub task_is_now_finished: bool,
}

// User visible struct for introspection.
#[derive(Debug, Clone)]
pub struct TaskInformation<'a, Cstrnt> {
    pub type_id: TypeId,
    pub type_name: &'static str,
    pub sender_id: SenderId,
    pub constraint: &'a Option<Constraint<Cstrnt>>,
}

pub(crate) struct TaskFromFrontend<Bkend, Cstrnt> {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) metadata: Vec<Cstrnt>,
    pub(crate) task: DynBackendTask<Bkend>,
    pub(crate) receiver: TaskReceiver,
    pub(crate) sender_id: SenderId,
    pub(crate) constraint: Option<Constraint<Cstrnt>>,
    pub(crate) kill_handle: KillHandle,
}

pub(crate) struct Task<Cstrnt> {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) receiver: TaskReceiver,
    pub(crate) sender_id: SenderId,
    pub(crate) task_id: TaskId,
    pub(crate) kill_handle: KillHandle,
    pub(crate) metadata: Vec<Cstrnt>,
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

pub(crate) enum TaskReceiver {
    Future(oneshot::Receiver<DynFallibleFuture>),
    Stream(mpsc::Receiver<DynFallibleFuture>),
}
impl From<oneshot::Receiver<DynFallibleFuture>> for TaskReceiver {
    fn from(value: oneshot::Receiver<DynFallibleFuture>) -> Self {
        Self::Future(value)
    }
}
impl From<mpsc::Receiver<DynFallibleFuture>> for TaskReceiver {
    fn from(value: mpsc::Receiver<DynFallibleFuture>) -> Self {
        Self::Stream(value)
    }
}

impl<Cstrnt: PartialEq> TaskList<Cstrnt> {
    pub(crate) fn new() -> Self {
        Self { inner: vec![] }
    }
    /// Returns Some(ResponseInformation) if a task existed in the list, and it
    /// was processed. Returns None, if no tasks were in the list.
    pub(crate) async fn process_next_response(&mut self) -> Option<ResponseInformation> {
        let task_completed = self
            .inner
            .iter_mut()
            .enumerate()
            .map(|(idx, task)| async move {
                match task.receiver {
                    TaskReceiver::Future(ref mut receiver) => {
                        if let Ok(forwarder) = receiver.await {
                            return (
                                Some(idx),
                                Some(forwarder),
                                task.type_id,
                                task.type_name,
                                task.sender_id,
                                task.task_id,
                            );
                        }
                        (
                            Some(idx),
                            None,
                            task.type_id,
                            task.type_name,
                            task.sender_id,
                            task.task_id,
                        )
                    }
                    TaskReceiver::Stream(ref mut receiver) => {
                        if let Some(forwarder) = receiver.recv().await {
                            return (
                                None,
                                Some(forwarder),
                                task.type_id,
                                task.type_name,
                                task.sender_id,
                                task.task_id,
                            );
                        }
                        (
                            Some(idx),
                            None,
                            task.type_id,
                            task.type_name,
                            task.sender_id,
                            task.task_id,
                        )
                    }
                }
            })
            .collect::<FuturesUnordered<_>>()
            .next()
            .await;
        let (maybe_completed_id, maybe_forwarder, type_id, type_name, sender_id, task_id) =
            task_completed?;
        if let Some(forwarder) = maybe_forwarder {
            // Whilst this seems inefficient, this removes an await point and therefore
            // makes this function cancellation safe.
            tokio::spawn(forwarder);
        }
        if let Some(task_completed) = maybe_completed_id {
            // Safe - this value is in range as produced from enumerate on original list.
            self.inner.swap_remove(task_completed);
        }
        Some(ResponseInformation {
            type_id,
            type_name,
            sender_id,
            task_id,
            task_is_now_finished: maybe_completed_id.is_some(),
        })
    }
    pub(crate) fn push(&mut self, task: Task<Cstrnt>) {
        self.inner.push(task)
    }
    // TODO: Tests
    pub(crate) fn handle_constraint(
        &mut self,
        constraint: Constraint<Cstrnt>,
        type_id: TypeId,
        sender_id: SenderId,
    ) {
        // Assuming here that kill implies block also.
        let task_doesnt_match_constraint =
            |task: &Task<_>| (task.type_id != type_id) || (task.sender_id != sender_id);
        let task_doesnt_match_metadata =
            |task: &Task<_>, constraint| !task.metadata.contains(constraint);
        match constraint.constraint_type {
            ConstraitType::BlockMatchingMetatdata(metadata) => self
                .inner
                .retain(|task| task_doesnt_match_metadata(task, &metadata)),
            ConstraitType::BlockSameType => {
                self.inner.retain(task_doesnt_match_constraint);
            }
            ConstraitType::KillSameType => self.inner.retain_mut(|task| {
                if !task_doesnt_match_constraint(task) {
                    task.kill_handle.kill().expect("Task should still be alive");
                    return false;
                }
                true
            }),
        }
    }
}

impl<Bkend, Cstrnt> TaskFromFrontend<Bkend, Cstrnt> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        type_id: TypeId,
        type_name: &'static str,
        metadata: Vec<Cstrnt>,
        task: impl FnOnce(&Bkend) -> DynFallibleFuture + 'static,
        receiver: impl Into<TaskReceiver>,
        sender_id: SenderId,
        constraint: Option<Constraint<Cstrnt>>,
        kill_handle: KillHandle,
    ) -> Self {
        Self {
            type_id,
            type_name,
            metadata,
            task: Box::new(task),
            receiver: receiver.into(),
            sender_id,
            constraint,
            kill_handle,
        }
    }
    pub(crate) fn get_information(&self) -> TaskInformation<'_, Cstrnt> {
        TaskInformation {
            type_id: self.type_id,
            type_name: self.type_name,
            sender_id: self.sender_id,
            constraint: &self.constraint,
        }
    }
}

impl<Cstrnt> Task<Cstrnt> {
    pub(crate) fn new(
        type_id: TypeId,
        type_name: &'static str,
        metadata: Vec<Cstrnt>,
        receiver: TaskReceiver,
        sender_id: SenderId,
        task_id: TaskId,
        kill_handle: KillHandle,
    ) -> Self {
        Self {
            type_id,
            type_name,
            receiver,
            sender_id,
            kill_handle,
            task_id,
            metadata,
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
