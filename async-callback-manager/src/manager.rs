use crate::{
    kill_channel,
    task::{Constraint, ConstraitType, Task, TaskFromFrontend, TaskReceiver},
    utils::{mpsc_try_recv_many, TryRecvManyOutcome},
    CallbackSender,
};
use std::any::TypeId;
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
    pub fn process_messages(&mut self, backend: Bkend) {
        let buffer = match mpsc_try_recv_many(&mut self.this_receiver) {
            TryRecvManyOutcome::Finished(vec) => vec,
            TryRecvManyOutcome::NotFinished(vec) => vec,
        };
        for task in buffer {
            println!("Got a task");
            if let Some(constraint) = task.constraint {
                println!("Task had a constraint: {:?}", constraint);
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
        println!(
            "Constraints list is {} long before checking receivers",
            self.tasks_list.len()
        );
        self.check_tasks();
        println!(
            "Constraints list is {} long after checking receivers",
            self.tasks_list.len()
        );
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
    fn check_tasks(&mut self) {
        self.tasks_list.retain_mut(|Task { receiver, .. }| {
            match receiver {
                TaskReceiver::Future(receiver) => {
                    if let Ok(rx) = receiver.try_recv() {
                        tokio::spawn(rx);
                        return false;
                    }
                }
                TaskReceiver::Stream(receiver) => match mpsc_try_recv_many(receiver) {
                    TryRecvManyOutcome::Finished(tasks) => {
                        tasks.into_iter().for_each(|task| {
                            tokio::spawn(task);
                        });
                        return false;
                    }
                    TryRecvManyOutcome::NotFinished(tasks) => {
                        tasks.into_iter().for_each(|task| {
                            tokio::spawn(task);
                        });
                    }
                },
            }
            true
        });
    }
    // Should be test only?
    // The problem with this is that now I'm testing this path instead of the
    // standard path.
    /// Waits for all tasks to complete, consuming self.
    pub async fn drain(mut self, backend: Bkend) {
        let mut buffer = vec![];
        // TODO: Size
        self.this_receiver.recv_many(&mut buffer, 999).await;
        for task in buffer {
            println!("Got a task");
            if let Some(constraint) = task.constraint {
                println!("Task had a constraint: {:?}", constraint);
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
        for Task { receiver, .. } in self.tasks_list {
            match receiver {
                TaskReceiver::Future(receiver) => receiver.await.unwrap().await.unwrap(),
                TaskReceiver::Stream(mut receiver) => {
                    while let Some(msg) = receiver.recv().await {
                        msg.await.unwrap()
                    }
                }
            }
        }
    }
}
