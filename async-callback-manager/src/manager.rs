use crate::{
    task::{ResponseInformation, Task, TaskFromFrontend, TaskInformation, TaskList},
    AsyncCallbackSender,
};
use tokio::sync::mpsc::{self, Receiver, Sender};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SenderId(usize);
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TaskId(usize);

type DynTaskReceivedCallback = dyn FnMut(TaskInformation);
type DynResponseReceivedCallback = dyn FnMut(ResponseInformation);

pub struct AsyncCallbackManager<Bkend> {
    next_sender_id: usize,
    next_task_id: usize,
    this_sender: Sender<TaskFromFrontend<Bkend>>,
    this_receiver: Receiver<TaskFromFrontend<Bkend>>,
    tasks_list: TaskList,
    // TODO: Make generic instead of dynamic.
    on_task_received: Box<DynTaskReceivedCallback>,
    on_response_received: Box<DynResponseReceivedCallback>,
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum ManagedEventType {
    SpawnedTask,
    ReceivedResponse,
}
impl ManagedEventType {
    pub fn is_spawned_task(&self) -> bool {
        self == &ManagedEventType::SpawnedTask
    }
    pub fn is_received_response(&self) -> bool {
        self == &ManagedEventType::ReceivedResponse
    }
}

impl<Bkend> AsyncCallbackManager<Bkend> {
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
            on_task_received: Box::new(|_| {}),
            on_response_received: Box::new(|_| {}),
        }
    }
    pub fn with_on_task_received_callback(
        mut self,
        cb: impl FnMut(TaskInformation) + 'static,
    ) -> Self {
        self.on_task_received = Box::new(cb);
        self
    }
    pub fn with_on_response_received_callback(
        mut self,
        cb: impl FnMut(ResponseInformation) + 'static,
    ) -> Self {
        self.on_response_received = Box::new(cb);
        self
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
        if overflowed {
            eprintln!("WARN: SenderID has overflowed");
        }
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
    /// Returns Some(ManagedEventType), if something was processed.
    /// Returns None, if no senders or tasks exist.
    pub async fn manage_next_event(&mut self, backend: &Bkend) -> Option<ManagedEventType> {
        tokio::select! {
            Some(task) = self.this_receiver.recv() => {
                self.spawn_task(backend, task);
                Some(ManagedEventType::SpawnedTask)
            },
            Some(response) = self.tasks_list.process_next_response() => {
                (self.on_response_received)(response);
                Some(ManagedEventType::ReceivedResponse)
            }
            else => None
        }
    }
    /// Spawns the next incoming task from a sender.
    /// Returns Some(()), if a task was spawned.
    /// Returns None, if no senders.
    pub async fn spawn_next_task(&mut self, backend: &Bkend) -> Option<()> {
        let task = self.this_receiver.recv().await?;
        self.spawn_task(backend, task);
        Some(())
    }
    /// Spawns the next incoming task from a sender.
    /// Returns Some(ResponseInformation), if a task was spawned.
    /// Returns None, if no senders.
    /// Note that the 'on_next_response' callback is not called!
    pub async fn process_next_response(&mut self) -> Option<ResponseInformation> {
        self.tasks_list.process_next_response().await
    }
    fn spawn_task(&mut self, backend: &Bkend, task: TaskFromFrontend<Bkend>) {
        (self.on_task_received)(task.get_information());
        if let Some(constraint) = task.constraint {
            self.tasks_list
                .handle_constraint(constraint, task.type_id, task.sender_id);
        }
        self.tasks_list.push(Task::new(
            task.type_id,
            task.type_name,
            task.receiver,
            task.sender_id,
            TaskId(self.next_task_id),
            task.kill_handle,
        ));
        let (new_id, overflowed) = self.next_task_id.overflowing_add(1);
        if overflowed {
            eprintln!("WARN: TaskID has overflowed");
        }
        self.next_task_id = new_id;
        let fut = (task.task)(backend);
        tokio::spawn(fut);
    }
}