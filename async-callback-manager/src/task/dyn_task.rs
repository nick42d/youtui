//! When storing and generating tasks dynamic dispatch is used for two reasons;
//! 1. Ease of use due to type erasure - caller just needs to know Frntend,
//!    Bkend and Md types, not Task, Handler or Output types. This prevents the
//!    need to juggle effects in Either type structs which may cause issues.
//! 2. Ease of storage in task list due to heap allocation - manager can store
//!    tasks directly in a Vec as they are all the same size.
use crate::{AsyncTask, BackendStreamingTask, BackendTask, FrontendEffect, OptDebug, TaskHandler};
use futures::Stream;
use tokio_stream::StreamExt;

mod handlers;

pub use handlers::*;

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

/// Type erasure helper trait
pub(crate) trait IntoDynFutureTask<Frntend, Bkend, Md>: OptDynPartialEq + OptDebug {
    fn into_dyn_task(self: Box<Self>) -> DynFutureTask<Frntend, Bkend, Md>;
}
/// Type erasure helper trait
pub(crate) trait IntoDynStreamTask<Frntend, Bkend, Md>: OptDynPartialEq + OptDebug {
    fn into_dyn_stream(self: Box<Self>) -> DynStreamTask<Frntend, Bkend, Md>;
}
/// feature(where_clauses) on nightly would prevent this.
#[cfg(not(feature = "task-equality"))]
pub trait OptDynPartialEq {}
#[cfg(feature = "task-equality")]
pub trait OptDynPartialEq: DynPartialEq {}
#[cfg(feature = "task-equality")]
impl<T: DynPartialEq> OptDynPartialEq for T {}
#[cfg(not(feature = "task-equality"))]
impl<T> OptDynPartialEq for T {}
/// Type erasure helper trait
#[cfg(feature = "task-equality")]
pub(crate) trait DynPartialEq: std::any::Any {
    fn dyn_partial_eq(&self, other: &dyn DynPartialEq) -> bool;
}

/// Combination of Task and Handler, which can then have task and handler types
/// erased into an IntoDyn{Future/Stream}Task<Frntend, Bkend, Md>.
#[derive(PartialEq, Debug)]
pub(crate) struct FusedTask<T, H> {
    pub(crate) task: T,
    pub(crate) handler: H,
}

#[cfg(feature = "task-equality")]
impl<T, H> DynPartialEq for FusedTask<T, H>
where
    T: PartialEq + 'static,
    H: PartialEq + 'static,
{
    fn dyn_partial_eq(&self, other: &dyn DynPartialEq) -> bool {
        let Some(other) = (other as &dyn std::any::Any).downcast_ref::<Self>() else {
            return false;
        };
        self == other
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
                    Box::new(move |frontend: &mut Frntend| {
                        handler.handle(output).apply(frontend).into()
                    }) as DynStateMutation<Frntend, Bkend, T::MetadataType>
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
                        move |frontend: &mut Frntend| {
                            handler.clone().handle(output).apply(frontend).into()
                        }
                    }) as DynStateMutation<Frntend, Bkend, T::MetadataType>
                })
            }) as DynMutationStream<Frntend, Bkend, T::MetadataType>
        }) as DynStreamTask<Frntend, Bkend, T::MetadataType>
    }
}

impl<T, H> FusedTask<T, H> {
    pub(crate) fn new_future<Bkend, Frntend, Md>(request: T, handler: H) -> Self
    where
        T: BackendTask<Bkend>,
        H: TaskHandler<T::Output, Frntend, Bkend, Md>,
    {
        Self {
            task: request,
            handler,
        }
    }
    pub(crate) fn new_stream<Bkend, Frntend, Md>(request: T, handler: H) -> Self
    where
        T: BackendStreamingTask<Bkend>,
        H: TaskHandler<T::Output, Frntend, Bkend, Md> + Clone,
    {
        Self {
            task: request,
            handler,
        }
    }
}
