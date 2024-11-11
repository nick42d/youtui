use crate::{DynBackendTask, DynFallibleFuture, KillHandle, SenderId, TaskId};
use futures::{stream::FuturesUnordered, StreamExt};
use std::any::TypeId;
use tokio::sync::{mpsc, oneshot};

#[derive(Default)]
pub(crate) struct TaskList {
    pub inner: Vec<Task>,
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
pub struct TaskInformation<'a> {
    pub type_id: TypeId,
    pub type_name: &'static str,
    pub sender_id: SenderId,
    pub constraint: &'a Option<Constraint>,
}

pub(crate) struct TaskFromFrontend<Bkend> {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) task: DynBackendTask<Bkend>,
    pub(crate) receiver: TaskReceiver,
    pub(crate) sender_id: SenderId,
    pub(crate) constraint: Option<Constraint>,
    pub(crate) kill_handle: KillHandle,
}

pub(crate) struct Task {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) receiver: TaskReceiver,
    pub(crate) sender_id: SenderId,
    pub(crate) task_id: TaskId,
    pub(crate) kill_handle: KillHandle,
}

#[derive(Eq, PartialEq, Debug)]
pub struct Constraint {
    pub(crate) constraint_type: ConstraitType,
}

#[derive(Eq, PartialEq, Debug)]
pub enum ConstraitType {
    BlockSameType,
    KillSameType,
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

impl TaskList {
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
                            let _ = forwarder.await;
                            return (
                                Some(idx),
                                task.type_id,
                                task.type_name,
                                task.sender_id,
                                task.task_id,
                            );
                        }
                        (
                            None,
                            task.type_id,
                            task.type_name,
                            task.sender_id,
                            task.task_id,
                        )
                    }
                    TaskReceiver::Stream(ref mut receiver) => {
                        if let Some(forwarder) = receiver.recv().await {
                            let _ = forwarder.await;
                            return (
                                None,
                                task.type_id,
                                task.type_name,
                                task.sender_id,
                                task.task_id,
                            );
                        }
                        (
                            Some(idx),
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
        if let Some((Some(task_completed), ..)) = task_completed {
            // Safe - this value is in range as produced from enumerate on original list.
            self.inner.swap_remove(task_completed);
        }
        task_completed.map(|(maybe_idx, type_id, type_name, sender_id, task_id)| {
            ResponseInformation {
                type_id,
                type_name,
                sender_id,
                task_id,
                task_is_now_finished: maybe_idx.is_some(),
            }
        })
    }
    pub(crate) fn push(&mut self, task: Task) {
        self.inner.push(task)
    }
    pub(crate) fn handle_constraint(
        &mut self,
        constraint: Constraint,
        type_id: TypeId,
        sender_id: SenderId,
    ) {
        // Assuming here that kill implies block also.
        let task_doesnt_match_constraint =
            |task: &Task| (task.type_id != type_id) || (task.sender_id != sender_id);
        match constraint.constraint_type {
            ConstraitType::BlockSameType => {
                self.inner.retain(task_doesnt_match_constraint);
            }
            ConstraitType::KillSameType => self.inner.retain_mut(|task| {
                if !task_doesnt_match_constraint(task) {
                    task.kill_handle.kill().unwrap();
                    return false;
                }
                true
            }),
        }
    }
}

impl<Bkend> TaskFromFrontend<Bkend> {
    pub(crate) fn new(
        type_id: TypeId,
        type_name: &'static str,
        task: impl FnOnce(&Bkend) -> DynFallibleFuture + 'static,
        receiver: impl Into<TaskReceiver>,
        sender_id: SenderId,
        constraint: Option<Constraint>,
        kill_handle: KillHandle,
    ) -> Self {
        Self {
            type_id,
            type_name,
            task: Box::new(task),
            receiver: receiver.into(),
            sender_id,
            constraint,
            kill_handle,
        }
    }
    pub(crate) fn get_information(&self) -> TaskInformation<'_> {
        TaskInformation {
            type_id: self.type_id,
            type_name: self.type_name,
            sender_id: self.sender_id,
            constraint: &self.constraint,
        }
    }
}

impl Task {
    pub(crate) fn new(
        type_id: TypeId,
        type_name: &'static str,
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
        }
    }
}

impl Constraint {
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
}
