//! When storing and generating tasks dynamic dispatch is used for two reasons;
//! 1. Ease of use due to type erasure - caller just needs to know Frntend,
//!    Bkend and Md types, not Task, Handler or Output types. This prevents the
//!    need to juggle effects in Either type structs which may cause issues.
//! 2. Ease of storage in task list due to heap allocation - manager can store
//!    tasks directly in a Vec as they are all the same size.
use crate::{AsyncTask, BackendStreamingTask, BackendTask, FrontendMutation, TaskHandler};
use futures::Stream;
use std::any::Any;
use tokio_stream::StreamExt;

pub(crate) type DynStateMutation<Frntend, Bkend, Md> =
    Box<dyn FnOnce(&mut Frntend) -> AsyncTask<Frntend, Bkend, Md> + Send>;
pub(crate) type DynMutationFuture<Frntend, Bkend, Md> =
    Box<dyn Future<Output = DynStateMutation<Frntend, Bkend, Md>> + Unpin + Send>;
pub(crate) type DynMutationStream<Frntend, Bkend, Md> =
    Box<dyn Stream<Item = DynStateMutation<Frntend, Bkend, Md>> + Unpin + Send>;
pub(crate) type DynFutureTask<Frntend, Bkend, Md> =
    Box<dyn FnOnce(&Bkend) -> DynMutationFuture<Frntend, Bkend, Md>>;
pub(crate) type DynStreamTask<Frntend, Bkend, Md> =
    Box<dyn FnOnce(&Bkend) -> DynMutationStream<Frntend, Bkend, Md>>;

pub(crate) trait IntoDynFutureTask<Frntend, Bkend, Md>: MaybeDynEq {
    fn into_dyn_task(self: Box<Self>) -> DynFutureTask<Frntend, Bkend, Md>;
}
pub(crate) trait IntoDynStreamTask<Frntend, Bkend, Md>: MaybeDynEq {
    fn into_dyn_stream(self: Box<Self>) -> DynStreamTask<Frntend, Bkend, Md>;
}
pub(crate) trait MaybeDynEq: std::any::Any {
    fn maybe_dyn_eq(&self, other: &dyn MaybeDynEq) -> Option<bool>;
}

/// Combination of Task and Handler.
pub(crate) struct FusedTask<T, H> {
    pub(crate) task: T,
    pub(crate) handler: H,
    pub(crate) eq_fn: Option<fn(&Self, &Self) -> bool>,
}

impl<T, H> FusedTask<T, H> {
    pub(crate) fn new_future_with_closure_handler<Bkend, Frntend, Md>(
        request: T,
        handler: H,
    ) -> Self
    where
        T: BackendTask<Bkend>,
        H: FnOnce(&mut Frntend, T::Output) -> AsyncTask<Frntend, Bkend, Md> + Send + 'static,
    {
        Self {
            task: request,
            handler,
            eq_fn: None,
        }
    }
    pub(crate) fn new_future_eq<Bkend, Frntend, Md>(request: T, handler: H) -> Self
    where
        T: BackendTask<Bkend>,
        H: TaskHandler<T::Output, Frntend, Bkend, Md>,
        T: PartialEq,
        H: PartialEq,
    {
        Self {
            task: request,
            handler,
            eq_fn: Some(|t1, t2| t1.task == t2.task && t1.handler == t2.handler),
        }
    }
    pub(crate) fn new_stream_with_closure_handler<Bkend, Frntend, Md>(
        request: T,
        handler: H,
    ) -> Self
    where
        T: BackendStreamingTask<Bkend>,
        H: FnOnce(&mut Frntend, T::Output) -> AsyncTask<Frntend, Bkend, Md>
            + Clone
            + Send
            + 'static,
    {
        Self {
            task: request,
            handler,
            eq_fn: None,
        }
    }
    pub(crate) fn new_stream_eq<Bkend, Frntend, Md>(request: T, handler: H) -> Self
    where
        T: BackendStreamingTask<Bkend>,
        H: TaskHandler<T::Output, Frntend, Bkend, Md> + Clone,
        T: PartialEq,
        H: PartialEq,
    {
        Self {
            task: request,
            handler,
            eq_fn: Some(|t1, t2| t1.task == t2.task && t1.handler == t2.handler),
        }
    }
}

impl<T, H> MaybeDynEq for FusedTask<T, H>
where
    T: 'static,
    H: 'static,
{
    fn maybe_dyn_eq(&self, other: &dyn MaybeDynEq) -> Option<bool> {
        let eq_fn = self.eq_fn?;
        let other = (other as &dyn Any).downcast_ref::<Self>()?;
        Some(eq_fn(self, other))
    }
}

impl<T, H, Bkend, Frntend> IntoDynFutureTask<Frntend, Bkend, T::MetadataType> for FusedTask<T, H>
where
    T: Send + 'static,
    H: Send + 'static,
    T: BackendTask<Bkend>,
    H: TaskHandler<T::Output, Frntend, Bkend, T::MetadataType>,
    T::Output: 'static,
{
    fn into_dyn_task(self: Box<Self>) -> DynFutureTask<Frntend, Bkend, T::MetadataType> {
        let Self { task, handler, .. } = *self;
        Box::new(move |b: &Bkend| {
            Box::new({
                let future = task.into_future(b);
                Box::pin(async move {
                    let output = future.await;
                    Box::new(move |frontend: &mut Frntend| handler.handle(output).apply(frontend))
                        as DynStateMutation<Frntend, Bkend, T::MetadataType>
                })
            }) as DynMutationFuture<Frntend, Bkend, T::MetadataType>
        }) as DynFutureTask<Frntend, Bkend, T::MetadataType>
    }
}
impl<T, H, Bkend, Frntend> IntoDynStreamTask<Frntend, Bkend, T::MetadataType> for FusedTask<T, H>
where
    T: Send + 'static,
    H: Send + 'static,
    T: BackendStreamingTask<Bkend>,
    H: TaskHandler<T::Output, Frntend, Bkend, T::MetadataType> + Clone,
    T::Output: 'static,
{
    fn into_dyn_stream(self: Box<Self>) -> DynStreamTask<Frntend, Bkend, T::MetadataType> {
        let Self { task, handler, .. } = *self;
        Box::new(move |b: &Bkend| {
            let stream = task.into_stream(b);
            Box::new({
                stream.map(move |output| {
                    Box::new({
                        let handler = handler.clone();
                        move |frontend: &mut Frntend| handler.clone().handle(output).apply(frontend)
                    }) as DynStateMutation<Frntend, Bkend, T::MetadataType>
                })
            }) as DynMutationStream<Frntend, Bkend, T::MetadataType>
        }) as DynStreamTask<Frntend, Bkend, T::MetadataType>
    }
}
