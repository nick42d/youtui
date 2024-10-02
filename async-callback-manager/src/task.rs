use crate::{DynBackendTask, DynFallibleFuture, KillHandle, SenderId, TaskId};
use std::any::TypeId;
use tokio::sync::{mpsc, oneshot};

pub(crate) struct TaskFromFrontend<Bkend> {
    pub(crate) type_id: TypeId,
    pub(crate) task: DynBackendTask<Bkend>,
    pub(crate) receiver: TaskReceiver,
    pub(crate) sender_id: SenderId,
    pub(crate) constraint: Option<Constraint>,
    pub(crate) kill_handle: KillHandle,
}

pub(crate) struct Task {
    pub(crate) type_id: TypeId,
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

impl<Bkend> TaskFromFrontend<Bkend> {
    pub(crate) fn new(
        type_id: TypeId,
        task: impl FnOnce(Bkend) -> DynFallibleFuture + 'static,
        receiver: impl Into<TaskReceiver>,
        sender_id: SenderId,
        constraint: Option<Constraint>,
        kill_handle: KillHandle,
    ) -> Self {
        Self {
            type_id,
            task: Box::new(task),
            receiver: receiver.into(),
            sender_id,
            constraint,
            kill_handle,
        }
    }
}

impl Task {
    pub(crate) fn new(
        type_id: TypeId,
        receiver: TaskReceiver,
        sender_id: SenderId,
        task_id: TaskId,
        kill_handle: KillHandle,
    ) -> Self {
        Self {
            type_id,
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
