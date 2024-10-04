use crate::{
    kill_channel,
    task::{Constraint, TaskFromFrontend, TaskReceiver},
    BackendStreamingTask, BackendTask, DynCallbackFn, DynFallibleFuture, Error, KillHandle,
    KillSignal, Result, SenderId,
};
use futures::{FutureExt, StreamExt};
use std::any::{Any, TypeId};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    oneshot,
};

pub struct CallbackSender<Bkend, Frntend> {
    pub(crate) id: SenderId,
    pub(crate) this_sender: Sender<DynCallbackFn<Frntend>>,
    pub(crate) this_receiver: Receiver<DynCallbackFn<Frntend>>,
    pub(crate) runner_sender: Sender<TaskFromFrontend<Bkend>>,
}

/// A set of state mutations, that can be applied to a Frntend.
pub struct StateMutationBundle<Frntend> {
    mutation_list: Vec<DynCallbackFn<Frntend>>,
}
impl<Frntend> StateMutationBundle<Frntend> {
    pub fn apply(self, frontend: &mut Frntend) {
        self.mutation_list
            .into_iter()
            .for_each(|mutation| mutation(frontend));
    }
}

impl<Bkend, Frntend> CallbackSender<Bkend, Frntend> {
    pub async fn get_messages(&mut self) -> StateMutationBundle<Frntend> {
        let mut mutation_list = Vec::new();
        while let Ok(mutation) = self.this_receiver.try_recv() {
            mutation_list.push(mutation);
        }
        StateMutationBundle { mutation_list }
    }
    pub async fn add_stream_callback<R>(
        &self,
        request: R,
        // TODO: Relax Clone bounds if possible.
        handler: impl FnOnce(&mut Frntend, R::Output) + Send + Clone + 'static,
        constraint: Option<Constraint>,
    ) -> Result<()>
    where
        R: BackendStreamingTask<Bkend> + 'static,
        Bkend: Send + 'static,
        Frntend: 'static,
    {
        // TODO: channel size
        let (tx, rx) = mpsc::channel(50);
        let (kill_tx, kill_rx) = kill_channel();
        let completed_task_sender = self.this_sender.clone();
        let func = move |backend: Bkend| {
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
        self.send_task::<R>(func, rx, constraint, kill_tx).await
    }
    pub async fn add_callback<R>(
        &self,
        request: R,
        handler: impl FnOnce(&mut Frntend, R::Output) + Send + 'static,
        constraint: Option<Constraint>,
    ) -> Result<()>
    where
        R: BackendTask<Bkend> + 'static,
        Bkend: Send + 'static,
        Frntend: 'static,
    {
        let (tx, rx) = oneshot::channel();
        let (kill_tx, kill_rx) = kill_channel();
        let completed_task_sender = self.this_sender.clone();
        let func = move |backend: Bkend| {
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
        self.send_task::<R>(func, rx, constraint, kill_tx).await
    }
    async fn send_task<R: Any + 'static>(
        &self,
        func: impl FnOnce(Bkend) -> DynFallibleFuture + 'static,
        rx: impl Into<TaskReceiver>,
        constraint: Option<Constraint>,
        kill_handle: KillHandle,
    ) -> Result<()> {
        self.runner_sender
            .send(TaskFromFrontend::new(
                TypeId::of::<R>(),
                func,
                rx,
                self.id,
                constraint,
                kill_handle,
            ))
            .await
            .map_err(|_| Error::ErrorSending)
    }
}

async fn stream_request_func<R, Bkend, Frntend, H>(
    request: R,
    backend: Bkend,
    handler: H,
    sender: mpsc::Sender<DynCallbackFn<Frntend>>,
    forwarder: mpsc::Sender<DynFallibleFuture>,
    kill_signal: KillSignal,
) -> Result<()>
where
    H: FnOnce(&mut Frntend, R::Output) + Send + Clone + 'static,
    R: BackendStreamingTask<Bkend> + 'static,
    Bkend: Send + 'static,
    Frntend: 'static,
{
    let mut stream = request.into_stream(backend);
    tokio::select! {
        output = async move {
            // Consider using Stream combinators like .then() here.
            while let Some(output) = stream.next().await {
                let handler = handler.clone();
                let sender = sender.clone();
                let callback = move |frontend: &mut Frntend| handler(frontend, output);
                let forward_message_task = forward_message_task(callback, sender).boxed();
                if forwarder
                    .send(Box::new(forward_message_task))
                    .await
                    .is_err()
                {
                    // Consider if we actually want to return early if Task is dropped.
                    return Err(Error::ErrorSending);
                }
            };
            Ok(())
        } => output,
        Ok(()) = kill_signal => Ok(()),
    }
}

async fn request_func<R, Bkend, Frntend, H>(
    request: R,
    backend: Bkend,
    handler: H,
    sender: mpsc::Sender<DynCallbackFn<Frntend>>,
    forwarder: oneshot::Sender<DynFallibleFuture>,
    kill_signal: KillSignal,
) -> Result<()>
where
    H: FnOnce(&mut Frntend, R::Output) + Send + 'static,
    R: BackendTask<Bkend> + 'static,
    Bkend: Send + 'static,
    Frntend: 'static,
{
    let output = tokio::select! {
        output = request.into_future(backend) => output,
        Ok(()) = kill_signal => return Ok(()),
    };
    let callback = |frontend: &mut Frntend| handler(frontend, output);
    let forward_message_task = forward_message_task(callback, sender).boxed();
    forwarder
        .send(Box::new(forward_message_task))
        .map_err(|_| Error::ErrorSending)
}

async fn forward_message_task<Frntend>(
    callback: impl FnOnce(&mut Frntend) + Send + 'static,
    sender: mpsc::Sender<DynCallbackFn<Frntend>>,
) -> Result<()> {
    sender
        .send(Box::new(callback))
        .await
        .map_err(|_| Error::ErrorSending)
}
