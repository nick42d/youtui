use crate::{
    task::{Task, TaskFromFrontend, TaskList},
    AsyncCallbackSender,
};
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
    tasks_list: TaskList,
}

impl<Bkend: Clone> AsyncCallbackManager<Bkend> {
    /// Get a new AsyncCallbackManager. Channel size refers to number of
    /// messages that can be buffered from senders.
    pub fn new(channel_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(channel_size);
        AsyncCallbackManager {
            next_sender_id: 0,
            next_task_id: 0,
            this_receiver: rx,
            this_sender: tx,
            tasks_list: Default::default(),
        }
    }
    /// Creates a new AsyncCallbackSender that sends to this Manager.
    /// Channel size refers to number of number of state mutations that can be
    /// buffered from tasks.
    pub fn new_sender<Frntend>(
        &mut self,
        channel_size: usize,
    ) -> AsyncCallbackSender<Bkend, Frntend> {
        let (tx, rx) = mpsc::channel(channel_size);
        let task_function_sender = self.this_sender.clone();
        let id = SenderId(self.next_sender_id);
        let (new_id, overflowed) = self.next_sender_id.overflowing_add(1);
        self.next_sender_id = new_id;
        AsyncCallbackSender {
            id,
            this_sender: tx,
            this_receiver: rx,
            runner_sender: task_function_sender,
        }
    }
    /// Manage the next event in the queue.
    /// Combination of spawn_next_task and process_next_response.
    /// Returns Some(()), if something was processed.
    /// Returns None, if no senders or tasks exist.
    pub async fn manage_next_event(&mut self, backend: Bkend) -> Option<()> {
        tokio::select! {
            Some(task) = self.this_receiver.recv() => self.spawn_task(backend, task),
            Some(_) = self.tasks_list.process_next_task() => (),
            else => return None
        }
        Some(())
    }
    /// Spawns the next incoming task from a sender.
    /// Returns Some(()), if a task was spawned.
    /// Returns None, if no senders.
    pub async fn spawn_next_task(&mut self, backend: Bkend) -> Option<()> {
        let task = self.this_receiver.recv().await?;
        self.spawn_task(backend, task);
        Some(())
    }
    /// Spawns the next incoming task from a sender.
    /// Returns Some(()), if a task was spawned.
    /// Returns None, if no senders.
    pub async fn process_next_response(&mut self) -> Option<()> {
        self.tasks_list.process_next_task().await
    }
    fn spawn_task(&mut self, backend: Bkend, task: TaskFromFrontend<Bkend>) {
        if let Some(constraint) = task.constraint {
            self.tasks_list
                .handle_constraint(constraint, task.type_id, task.sender_id);
        }
        self.tasks_list.push(Task::new(
            task.type_id,
            task.receiver,
            task.sender_id,
            TaskId(self.next_task_id),
            task.kill_handle,
        ));
        let (new_id, overflowed) = self.next_task_id.overflowing_add(1);
        self.next_task_id = new_id;
        let fut = (task.task)(&backend);
        tokio::spawn(fut);
    }
}
