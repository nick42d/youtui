use crate::{
    kill_channel,
    task::{Constraint, TaskFromFrontend, TaskReceiver},
    BackendStreamingTask, BackendTask, DynCallbackFn, DynFallibleFuture, Error, KillHandle,
    KillSignal, Result, SenderId,
};
use futures::{FutureExt, StreamExt};
use std::{
    any::{Any, TypeId},
    future::Future,
};
use tokio::sync::{
    mpsc::{self, Receiver, Sender, UnboundedSender},
    oneshot,
};

pub struct AsyncCallbackSender<Bkend, Frntend, Cstrnt> {
    pub(crate) id: SenderId,
    pub(crate) this_sender: Sender<DynCallbackFn<Frntend>>,
    pub(crate) this_receiver: Receiver<DynCallbackFn<Frntend>>,
    pub(crate) runner_sender: UnboundedSender<TaskFromFrontend<Bkend, Cstrnt>>,
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
