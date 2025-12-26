use crate::task::DynStateMutation;
use crate::{Constraint, ConstraitType, TaskId};
use futures::stream::FuturesUnordered;
use std::any::TypeId;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::{JoinError, JoinHandle};
use tokio_stream::StreamExt;

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

pub(crate) enum TaskWaiter<Frntend, Bkend, Md> {
    Future(JoinHandle<DynStateMutation<Frntend, Bkend, Md>>),
    Stream {
        receiver: mpsc::Receiver<DynStateMutation<Frntend, Bkend, Md>>,
        join_handle: JoinHandle<()>,
    },
}

impl<Frntend, Bkend, Md> TaskWaiter<Frntend, Bkend, Md> {
    fn kill(&mut self) {
        match self {
            TaskWaiter::Future(handle) => handle.abort(),
            TaskWaiter::Stream { join_handle, .. } => join_handle.abort_handle().abort(),
        }
    }
}

pub enum TaskOutcome<Frntend, Bkend, Md> {
    /// The stream has completed, it won't be sending any more tasks.
    StreamFinished {
        type_id: TypeId,
        type_name: &'static str,
        type_debug: Arc<String>,
        task_id: TaskId,
    },
    /// The stream has panicked, it won't be sending any more tasks.
    StreamPanicked {
        error: JoinError,
        type_id: TypeId,
        type_name: &'static str,
        type_debug: Arc<String>,
        task_id: TaskId,
    },
    /// No task was recieved because the next task panicked.
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
                        ref mut receiver,
                        ref mut join_handle,
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
                        };
                        match join_handle.await {
                            Err(error) if error.is_panic() => (
                                Some(idx),
                                TaskOutcome::StreamPanicked {
                                    error,
                                    type_id: task.type_id,
                                    type_name: task.type_name,
                                    type_debug: task.type_debug.clone(),
                                    task_id: task.task_id,
                                },
                            ),
                            // Ok case or Err case where Err is not a panic (ie, it's an abort).
                            _ => (
                                Some(idx),
                                TaskOutcome::StreamFinished {
                                    type_id: task.type_id,
                                    type_name: task.type_name,
                                    type_debug: task.type_debug.clone(),
                                    task_id: task.task_id,
                                },
                            ),
                        }
                    }
                }
            })
            .collect::<FuturesUnordered<_>>()
            .next()
            .await;
        let (maybe_completed_idx, outcome) = task_completed?;
        if let Some(completed_idx) = maybe_completed_idx {
            // Safe - this value is in range as produced from enumerate on
            // original list.
            self.inner.swap_remove(completed_idx);
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
        let task_doesnt_match_constraint = |task: &SpawnedTask<_, _, _>| task.type_id != type_id;
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
