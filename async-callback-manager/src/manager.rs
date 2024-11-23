use std::{any::TypeId, future::Future};

use crate::{
    task::{
        AsyncTask, AsyncTaskKind, ResponseInformation, SpawnedTask, TaskInformation, TaskList,
        TaskWaiter,
    },
    Constraint,
};
use futures::Stream;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TaskId(pub(crate) usize);

pub(crate) type DynStateMutation<Frntend, Bkend, Md> =
    Box<dyn FnOnce(&mut Frntend) -> AsyncTask<Frntend, Bkend, Md> + Send>;
pub(crate) type DynFutureMutation<Frntend, Bkend, Md> =
    Box<dyn Future<Output = DynStateMutation<Frntend, Bkend, Md>> + Unpin + Send>;
pub(crate) type DynStreamMutation<Frntend, Bkend, Md> =
    Box<dyn Stream<Item = DynStateMutation<Frntend, Bkend, Md>> + Unpin + Send>;
pub(crate) type DynFutureTask<Frntend, Bkend, Md> =
    Box<dyn FnOnce(&Bkend) -> DynFutureMutation<Frntend, Bkend, Md>>;
pub(crate) type DynStreamTask<Frntend, Bkend, Md> =
    Box<dyn FnOnce(&Bkend) -> DynStreamMutation<Frntend, Bkend, Md>>;
pub(crate) type DynTaskReceivedCallback<Cstrnt> = dyn FnMut(TaskInformation<Cstrnt>);
pub(crate) type DynResponseReceivedCallback = dyn FnMut(ResponseInformation);

/// A set of state mutations, that can be applied to a Frntend.
pub struct StateMutationBundle<Frntend, Bkend, Md> {
    mutation_list: Vec<DynStateMutation<Frntend, Bkend, Md>>,
}
// impl<Frntend: 'static> StateMutationBundle<Frntend> {
//     pub fn map<NewFrntend>(
//         self,
//         mut nf: impl FnMut(&mut NewFrntend) -> &mut Frntend + Send + Copy +
// 'static,     ) -> StateMutationBundle<NewFrntend> {
//         let Self { mutation_list } = self;
//         let mutation_list: Vec<DynCallbackFn<NewFrntend>> = mutation_list
//             .into_iter()
//             .map(|m| {
//                 let closure = move |x: &mut NewFrntend| m(nf(x));
//                 Box::new(closure) as DynCallbackFn<NewFrntend>
//             })
//             .collect();
//         StateMutationBundle { mutation_list }
//     }
// }
impl<Frntend, Bkend, Md> StateMutationBundle<Frntend, Bkend, Md> {
    pub fn apply(self, frontend: &mut Frntend) -> Vec<AsyncTask<Frntend, Bkend, Md>> {
        self.mutation_list
            .into_iter()
            .map(|mutation| mutation(frontend))
            .collect()
    }
}

pub struct AsyncCallbackManager<Frntend, Bkend, Md> {
    next_sender_id: usize,
    next_task_id: usize,
    tasks_list: TaskList<Frntend, Bkend, Md>,
    // TODO: Make generic instead of dynamic.
    on_task_received: Box<DynTaskReceivedCallback<Md>>,
    on_response_received: Box<DynResponseReceivedCallback>,
}

impl<Frntend, Bkend, Md: PartialEq> Default for AsyncCallbackManager<Frntend, Bkend, Md> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Frntend, Bkend, Md: PartialEq> AsyncCallbackManager<Frntend, Bkend, Md> {
    /// Get a new AsyncCallbackManager.
    // TODO: Consider if this should be bounded. Unbounded has been chose for now as
    // it allows senders to send without blocking.
    pub fn new() -> Self {
        Self {
            next_sender_id: 0,
            next_task_id: 0,
            tasks_list: TaskList::new(),
            on_task_received: Box::new(|_| {}),
            on_response_received: Box::new(|_| {}),
        }
    }
    pub fn with_on_task_received_callback(
        mut self,
        cb: impl FnMut(TaskInformation<Md>) + 'static,
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
    /// Manage the next event in the queue.
    /// Combination of spawn_next_task and process_next_response.
    /// Returns Some(ManagedEventType), if something was processed.
    /// Returns None, if no senders or tasks exist.
    pub async fn manage_next_event(
        &mut self,
        backend: &Bkend,
    ) -> Option<DynStateMutation<Frntend, Bkend, Md>> {
        let (mutation, _, _, _) = self.tasks_list.process_next_response().await?;
        Some(mutation)
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
            type_id,
            type_name,
        } = task;
        let waiter = match task {
            AsyncTaskKind::Future(f) => {
                let future = f(backend);
                let handle = tokio::spawn(future);

                TaskWaiter::Future(handle)
            }
            AsyncTaskKind::Stream(_) => todo!(),
            AsyncTaskKind::NoOp => return,
        };
        let sp = SpawnedTask {
            type_id,
            task_id: TaskId(self.next_task_id),
            type_name,
            receiver: waiter,
            metadata,
        };
        // TODO: BigInt
        let (new_id, overflowed) = self.next_task_id.overflowing_add(1);
        if overflowed {
            eprintln!("WARN: TaskID has overflowed");
        }
        self.next_task_id = new_id;
        if let Some(constraint) = constraint {
            self.tasks_list.handle_constraint(constraint, type_id);
        }
        self.tasks_list.push(sp);
    }
}
