use crate::{
    task::{Constraint, ConstraitType, Task, TaskFromFrontend, TaskReceiver},
    utils::{mpsc_try_recv_many, TryRecvManyOutcome},
    CallbackSender,
};
use futures::{stream::FuturesUnordered, StreamExt, TryStreamExt};
use std::{any::TypeId, convert::identity, mem};
use tokio::sync::mpsc::{self, Receiver, Sender};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SenderId(usize);
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TaskId(usize);

pub struct AsyncCallbackManager<Bkend> {
    next_sender_id: usize,
    next_task_id: usize,
    this_sender: Sender<TaskFromFrontend<Bkend>>,
    this_receiver: Receiver<TaskFromFrontend<Bkend>>,
    tasks_list: Vec<Task>,
}

impl<Bkend: Clone> AsyncCallbackManager<Bkend> {
    pub fn new(channel_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(channel_size);
        AsyncCallbackManager {
            next_sender_id: 0,
            next_task_id: 0,
            this_receiver: rx,
            this_sender: tx,
            tasks_list: Vec::new(),
        }
    }
    pub fn new_sender<Frntend>(&mut self, channel_size: usize) -> CallbackSender<Bkend, Frntend> {
        let (tx, rx) = mpsc::channel(channel_size);
        let task_function_sender = self.this_sender.clone();
        let id = SenderId(self.next_sender_id);
        self.next_sender_id += 1;
        CallbackSender {
            id,
            this_sender: tx,
            this_receiver: rx,
            runner_sender: task_function_sender,
        }
    }
    pub async fn process_messages(&mut self, backend: Bkend) {
        let tasks = match mpsc_try_recv_many(&mut self.this_receiver) {
            TryRecvManyOutcome::Finished(vec) => vec,
            TryRecvManyOutcome::NotFinished(vec) => vec,
        };
        self.spawn_tasks(backend, tasks);
        self.check_tasks().await;
    }
    fn spawn_tasks(&mut self, backend: Bkend, tasks: Vec<TaskFromFrontend<Bkend>>) {
        for task in tasks {
            if let Some(constraint) = task.constraint {
                self.handle_constraint(constraint, task.type_id, task.sender_id);
            }
            self.tasks_list.push(Task::new(
                task.type_id,
                task.receiver,
                task.sender_id,
                TaskId(self.next_task_id),
                task.kill_handle,
            ));
            self.next_sender_id += 1;
            let fut = (task.task)(backend.clone());
            tokio::spawn(fut);
        }
    }
    fn handle_constraint(&mut self, constraint: Constraint, type_id: TypeId, sender_id: SenderId) {
        // Assuming here that kill implies block also.
        let task_doesnt_match_constraint =
            |task: &Task| (task.type_id != type_id) || (task.sender_id != sender_id);
        match constraint.constraint_type {
            ConstraitType::BlockSameType => {
                self.tasks_list.retain(task_doesnt_match_constraint);
            }
            ConstraitType::KillSameType => self.tasks_list.retain_mut(|task| {
                if !task_doesnt_match_constraint(task) {
                    task.kill_handle.kill().unwrap();
                    return false;
                }
                true
            }),
        }
    }
    // TODO: the receivers just get a message to forward on. No need to spawn a task
    // for each one, instead we can await.
    async fn check_tasks(&mut self) {
        let tasks_list = mem::take(&mut self.tasks_list);
        let new_tasks_list = tokio_stream::StreamExt::filter_map(
            tasks_list
                .into_iter()
                .map(|mut task| async {
                    match task.receiver {
                        TaskReceiver::Future(ref mut receiver) => {
                            if let Ok(rx) = receiver.try_recv() {
                                rx.await;
                                return None;
                            }
                            Some(task)
                        }
                        TaskReceiver::Stream(ref mut receiver) => {
                            match mpsc_try_recv_many(receiver) {
                                TryRecvManyOutcome::Finished(tasks) => {
                                    tasks
                                        .into_iter()
                                        .collect::<FuturesUnordered<_>>()
                                        .try_collect::<Vec<_>>()
                                        .await;
                                    None
                                }
                                TryRecvManyOutcome::NotFinished(tasks) => {
                                    tasks
                                        .into_iter()
                                        .collect::<FuturesUnordered<_>>()
                                        .try_collect::<Vec<_>>()
                                        .await;
                                    Some(task)
                                }
                            }
                        }
                    }
                })
                .collect::<FuturesUnordered<_>>(),
            identity,
        )
        .collect::<Vec<_>>()
        .await;
        self.tasks_list = new_tasks_list;
    }
    // Should be test only?
    /// Waits for all tasks to complete.
    pub async fn drain(mut self, backend: Bkend) {
        let mut buffer = vec![];
        // TODO: Size
        self.this_receiver.recv_many(&mut buffer, 999).await;
        self.spawn_tasks(backend, buffer);
        self.tasks_list
            .into_iter()
            .map(|Task { receiver, .. }| async {
                match receiver {
                    TaskReceiver::Future(receiver) => {
                        if let Ok(rx) = receiver.await {
                            rx.await;
                        }
                    }
                    TaskReceiver::Stream(mut receiver) => {
                        while let Some(msg) = receiver.recv().await {
                            msg.await.unwrap();
                        }
                    }
                }
            })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;
    }
}
