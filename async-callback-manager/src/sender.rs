use crate::{
    kill_channel,
    task::{
        Constraint, ConstraitType, ResponseInformation, TaskFromFrontend, TaskInformation,
        TaskReceiver,
    },
    BackendStreamingTask, BackendTask, DynCallbackFn, DynFallibleFuture, Error, KillHandle,
    KillSignal, ManagedEventType, Result, SenderId, TaskId,
};
use futures::{stream::FuturesUnordered, FutureExt, Stream, StreamExt};
use std::{
    any::{type_name, Any, TypeId},
    future::Future,
};
use tokio::{
    sync::{
        mpsc::{self, Receiver, Sender, UnboundedSender},
        oneshot,
    },
    task::JoinHandle,
};

pub struct AsyncCallbackSender<Bkend, Frntend, Cstrnt> {
    pub(crate) id: SenderId,
    pub(crate) this_sender: Sender<DynCallbackFn<Frntend>>,
    pub(crate) this_receiver: Receiver<DynCallbackFn<Frntend>>,
    pub(crate) runner_sender: UnboundedSender<TaskFromFrontend<Bkend, Cstrnt>>,
}

type DynStateMutation<B, F, C> = Box<dyn FnOnce(&mut F) -> AsyncTask<B, F, C> + Send>;
type DynFutureMutation<B, F, C> =
    Box<dyn Future<Output = DynStateMutation<B, F, C>> + Unpin + Send>;
type DynFutureTask<B, F, C> = Box<dyn FnOnce(&B) -> DynFutureMutation<B, F, C>>;
type DynStreamTask<B, F, C> =
    Box<dyn FnOnce(&B) -> Box<dyn Stream<Item = DynStateMutation<B, F, C>>>>;
type DynTaskReceivedCallback<Cstrnt> = dyn FnMut(TaskInformation<Cstrnt>);
type DynResponseReceivedCallback = dyn FnMut(ResponseInformation);

pub(crate) enum SimpleTaskWaiter<Frntend, Bkend, Md> {
    Future(JoinHandle<DynStateMutation<Frntend, Bkend, Md>>),
    Stream {
        receiver: mpsc::Receiver<DynStateMutation<Frntend, Bkend, Md>>,
        kill_handle: KillHandle,
    },
}
impl<Frntend, Bkend, Md> SimpleTaskWaiter<Frntend, Bkend, Md> {
    fn kill(&mut self) -> Result<()> {
        match self {
            SimpleTaskWaiter::Future(handle) => Ok(handle.abort()),
            SimpleTaskWaiter::Stream { kill_handle, .. } => kill_handle.kill(),
        }
    }
}

pub(crate) struct SimpleSpawnedTask<Frntend, Bkend, Md> {
    pub(crate) type_id: TypeId,
    pub(crate) type_name: &'static str,
    pub(crate) receiver: SimpleTaskWaiter<Frntend, Bkend, Md>,
    pub(crate) task_id: TaskId,
    pub(crate) metadata: Vec<Md>,
}

pub struct SimpleTaskList<Frntend, Bkend, Md> {
    inner: Vec<SimpleSpawnedTask<Frntend, Bkend, Md>>,
}
pub struct SimpleManager<Frntend, Bkend, Md> {
    next_sender_id: usize,
    next_task_id: usize,
    tasks_list: SimpleTaskList<Frntend, Bkend, Md>,
    // TODO: Make generic instead of dynamic.
    on_task_received: Box<DynTaskReceivedCallback<Md>>,
    on_response_received: Box<DynResponseReceivedCallback>,
}

pub struct AsyncTask<B, F, C> {
    task: AsyncTaskKind<B, F, C>,
    type_id: TypeId,
    type_name: &'static str,
    constraint: Option<Constraint<C>>,
    metadata: Vec<C>,
}

pub(crate) enum AsyncTaskKind<B, F, C> {
    Future(DynFutureTask<B, F, C>),
    Stream(DynStreamTask<B, F, C>),
    NoOp,
}

impl<B, F, C> AsyncTask<B, F, C> {
    pub fn new_no_op() -> Self {
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
        handler: impl FnOnce(&mut F, R::Output) + Send + 'static,
        constraint: Option<Constraint<C>>,
    ) -> AsyncTask<B, F, C>
    where
        R: BackendTask<B, MetadataType = C> + 'static,
        B: Send + 'static,
        F: 'static,
    {
        let metadata = R::metadata();
        let type_id = request.type_id();
        let type_name = type_name::<R>();
        let task = Box::new(move |b: &B| {
            Box::new({
                let future = request.into_future(b);
                Box::pin(async move {
                    let output = future.await;
                    Box::new(move |frontend: &mut F| {
                        handler(frontend, output);
                        AsyncTask::new_no_op()
                    }) as DynStateMutation<B, F, C>
                })
            }) as DynFutureMutation<B, F, C>
        }) as DynFutureTask<B, F, C>;
        AsyncTask {
            task: AsyncTaskKind::Future(task),
            constraint,
            metadata,
            type_id,
            type_name,
        }
    }
}

/// A set of state mutations, that can be applied to a Frntend.
pub struct StateMutationBundle<Frntend> {
    mutation_list: Vec<DynCallbackFn<Frntend>>,
}
impl<Frntend: 'static> StateMutationBundle<Frntend> {
    pub fn map<NewFrntend>(
        self,
        mut nf: impl FnMut(&mut NewFrntend) -> &mut Frntend + Send + Copy + 'static,
    ) -> StateMutationBundle<NewFrntend> {
        let Self { mutation_list } = self;
        let mutation_list: Vec<DynCallbackFn<NewFrntend>> = mutation_list
            .into_iter()
            .map(|m| {
                let closure = move |x: &mut NewFrntend| m(nf(x));
                Box::new(closure) as DynCallbackFn<NewFrntend>
            })
            .collect();
        StateMutationBundle { mutation_list }
    }
}
impl<Frntend> StateMutationBundle<Frntend> {
    pub fn apply(self, frontend: &mut Frntend) {
        self.mutation_list
            .into_iter()
            .for_each(|mutation| mutation(frontend));
    }
}

impl<Bkend, Frntend, Cstrnt> AsyncCallbackSender<Bkend, Frntend, Cstrnt> {
    pub async fn get_next_mutations(
        &mut self,
        max_mutations: usize,
    ) -> StateMutationBundle<Frntend> {
        let mut mutation_list = Vec::new();
        self.this_receiver
            .recv_many(&mut mutation_list, max_mutations)
            .await;
        StateMutationBundle { mutation_list }
    }
    /// # Errors
    /// This will return an error if the manager has been dropped.
    pub fn add_stream_callback<R>(
        &self,
        request: R,
        // TODO: Relax Clone bounds if possible.
        handler: impl FnOnce(&mut Frntend, R::Output) + Send + Clone + 'static,
        constraint: Option<Constraint<Cstrnt>>,
    ) -> Result<()>
    where
        R: BackendStreamingTask<Bkend, MetadataType = Cstrnt> + 'static,
        Bkend: Send + 'static,
        Frntend: 'static,
    {
        // TODO: channel size
        let (tx, rx) = mpsc::channel(50);
        let (kill_tx, kill_rx) = kill_channel();
        let completed_task_sender = self.this_sender.clone();
        let func = move |backend: &Bkend| {
            Box::new(
                stream_request_func(
                    request,
                    backend,
                    handler,
                    completed_task_sender,
                    tx,
                    kill_rx,
                )
                .boxed(),
            ) as DynFallibleFuture
        };
        self.send_task::<R>(func, R::metadata(), rx, constraint, kill_tx)
    }
    /// # Errors
    /// This will return an error if the manager has been dropped.
    pub fn add_callback<R>(
        &self,
        request: R,
        handler: impl FnOnce(&mut Frntend, R::Output) + Send + 'static,
        constraint: Option<Constraint<Cstrnt>>,
    ) -> Result<()>
    where
        R: BackendTask<Bkend, MetadataType = Cstrnt> + 'static,
        Bkend: Send + 'static,
        Frntend: 'static,
    {
        let (tx, rx) = oneshot::channel();
        let (kill_tx, kill_rx) = kill_channel();
        let completed_task_sender = self.this_sender.clone();
        let func = move |backend: &Bkend| {
            Box::new(
                request_func(
                    request,
                    backend,
                    handler,
                    completed_task_sender,
                    tx,
                    kill_rx,
                )
                .boxed(),
            ) as DynFallibleFuture
        };
        self.send_task::<R>(func, R::metadata(), rx, constraint, kill_tx)
    }
    /// # Errors
    /// This will return an error if the manager has been dropped.
    fn send_task<R: Any + 'static>(
        &self,
        func: impl FnOnce(&Bkend) -> DynFallibleFuture + 'static,
        metadata: Vec<Cstrnt>,
        rx: impl Into<TaskReceiver>,
        constraint: Option<Constraint<Cstrnt>>,
        kill_handle: KillHandle,
    ) -> Result<()> {
        self.runner_sender
            .send(TaskFromFrontend::new(
                TypeId::of::<R>(),
                std::any::type_name::<R>(),
                metadata,
                func,
                rx,
                self.id,
                constraint,
                kill_handle,
            ))
            .map_err(|_| Error::ReceiverDropped)
    }
}

fn stream_request_func<R, Bkend, Frntend, H>(
    request: R,
    backend: &Bkend,
    handler: H,
    sender: mpsc::Sender<DynCallbackFn<Frntend>>,
    forwarder: mpsc::Sender<DynFallibleFuture>,
    kill_signal: KillSignal,
) -> impl Future<Output = Result<()>>
where
    H: FnOnce(&mut Frntend, R::Output) + Send + Clone + 'static,
    R: BackendStreamingTask<Bkend> + 'static,
    Bkend: Send + 'static,
    Frntend: 'static,
{
    let future_stream_tasks = request
        .into_stream(backend)
        .then(move |output| {
            process_stream_item(output, handler.clone(), sender.clone(), forwarder.clone())
        })
        .collect::<Vec<_>>();
    async move {
        tokio::select! {
            _ = future_stream_tasks => Ok(()),
            Ok(()) = kill_signal => Ok(()),
        }
    }
    .boxed()
}

async fn process_stream_item<O, Frntend, H>(
    output: O,
    handler: H,
    sender: mpsc::Sender<DynCallbackFn<Frntend>>,
    forwarder: mpsc::Sender<DynFallibleFuture>,
) -> Result<()>
where
    O: Send + 'static,
    H: FnOnce(&mut Frntend, O) + Send + Clone + 'static,
    Frntend: 'static,
{
    let handler = handler.clone();
    let sender = sender.clone();
    let callback = move |frontend: &mut Frntend| handler(frontend, output);
    let forward_message_task = forward_message_task(callback, sender).boxed();
    if forwarder
        .send(Box::new(forward_message_task))
        .await
        .is_err()
    {
        return Err(Error::ReceiverDropped);
    }
    Ok(())
}

fn request_func<R, Bkend, Frntend, H>(
    request: R,
    backend: &Bkend,
    handler: H,
    sender: mpsc::Sender<DynCallbackFn<Frntend>>,
    forwarder: oneshot::Sender<DynFallibleFuture>,
    kill_signal: KillSignal,
) -> impl Future<Output = Result<()>> + Send + 'static
where
    H: FnOnce(&mut Frntend, R::Output) + Send + 'static,
    R: BackendTask<Bkend> + 'static,
    Bkend: Send + 'static,
    Frntend: 'static,
{
    let fut = request.into_future(backend);
    async move {
        let output = tokio::select! {
            output = fut => output,
            Ok(()) = kill_signal => return Ok(()),
        };
        let callback = |frontend: &mut Frntend| handler(frontend, output);
        let forward_message_task = forward_message_task(callback, sender).boxed();
        forwarder
            .send(Box::new(forward_message_task))
            .map_err(|_| Error::ReceiverDropped)
    }
    .boxed()
}

async fn forward_message_task<Frntend>(
    callback: impl FnOnce(&mut Frntend) + Send + 'static,
    sender: mpsc::Sender<DynCallbackFn<Frntend>>,
) -> Result<()> {
    sender
        .send(Box::new(callback))
        .await
        .map_err(|_| Error::ReceiverDropped)
}

impl<Bkend, Frntend, Md: PartialEq> SimpleTaskList<Frntend, Bkend, Md> {
    pub(crate) fn new() -> Self {
        Self { inner: vec![] }
    }
    /// Returns Some(ResponseInformation, Option<DynFallibleFuture>) if a task
    /// existed in the list, and it was processed. Returns None, if no tasks
    /// were in the list. The DynFallibleFuture represents a future that
    /// forwards messages from the manager back to the sender.
    pub(crate) async fn process_next_response(
        &mut self,
    ) -> Option<(ResponseInformation, Option<DynFallibleFuture>)> {
        let task_completed = self
            .inner
            .iter_mut()
            .enumerate()
            .map(|(idx, task)| async move {
                match task.receiver {
                    SimpleTaskWaiter::Future(ref mut receiver) => {
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
                    SimpleTaskWaiter::Stream {
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
        let (maybe_completed_id, maybe_forwarder, type_id, type_name, task_id) = task_completed?;
        if let Some(task_completed) = maybe_completed_id {
            // Safe - this value is in range as produced from enumerate on
            // original list.
            self.inner.swap_remove(task_completed);
        };
        // Some((
        //     ResponseInformation {
        //         type_id,
        //         type_name,
        //         sender_id,
        //         task_id,
        //         task_is_now_finished: maybe_completed_id.is_some(),
        //     },
        //     maybe_forwarder,
        // ))
        None
    }
    pub(crate) fn push(&mut self, task: SimpleSpawnedTask<Frntend, Bkend, Md>) {
        self.inner.push(task)
    }
    // TODO: Tests
    pub(crate) fn handle_constraint(&mut self, constraint: Constraint<Md>, type_id: TypeId) {
        // TODO: Consider the situation where one component kills tasks belonging to
        // another component.
        //
        // Assuming here that kill implies block also.
        let task_doesnt_match_constraint =
            |task: &SimpleSpawnedTask<_, _, _>| (task.type_id != type_id);
        let task_doesnt_match_metadata =
            |task: &SimpleSpawnedTask<_, _, _>, constraint| !task.metadata.contains(constraint);
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
impl<Frntend, Bkend, Md: PartialEq> SimpleManager<Frntend, Bkend, Md> {
    /// Get a new AsyncCallbackManager.
    // TODO: Consider if this should be bounded. Unbounded has been chose for now as
    // it allows senders to send without blocking.
    pub fn new() -> Self {
        Self {
            next_sender_id: 0,
            next_task_id: 0,
            tasks_list: SimpleTaskList::new(),
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
    pub async fn manage_next_event(&mut self, backend: &Bkend) -> Option<ManagedEventType> {
        let Some((response, forwarder)) = self.tasks_list.process_next_response().await else {
            return None;
        };
        if let Some(forwarder) = forwarder {
            let _ = forwarder.await;
        }
        (self.on_response_received)(response);
        Some(ManagedEventType::ReceivedResponse)
    }
    /// Spawns the next incoming task from a sender.
    /// Returns Some(ResponseInformation), if a task was spawned.
    /// Returns None, if no senders.
    /// Note that the 'on_next_response' callback is not called, you're given
    /// the ResponseInformation directly.
    pub async fn process_next_response(&mut self) -> Option<ResponseInformation> {
        let (response, forwarder) = self.tasks_list.process_next_response().await?;
        if let Some(forwarder) = forwarder {
            let _ = forwarder.await;
        }
        Some(response)
    }
    fn spawn_task(&mut self, backend: &Bkend, task: AsyncTask<Bkend, Frntend, Md>)
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

                SimpleTaskWaiter::Future(handle)
            }
            AsyncTaskKind::Stream(_) => todo!(),
            AsyncTaskKind::NoOp => return,
        };
        let sp = SimpleSpawnedTask {
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
