//! When storing and generating tasks dynamic dispatch is used for two reasons;
//! 1. Ease of use due to type erasure - caller just needs to know Frntend,
//!    Bkend and Md types, not Task, Handler or Output types. This prevents the
//!    need to juggle effects in Either type structs which may cause issues.
//! 2. Ease of storage in task list due to heap allocation - manager can store
//!    tasks directly in a Vec as they are all the same size.
use crate::{AsyncTask, BackendStreamingTask, BackendTask, FrontendEffect, OptDebug, TaskHandler};
use futures::Stream;
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

/// Allow closures to be accepted as TaskHandlers if equality and debug features
/// are not required.
#[cfg(all(not(feature = "task-equality"), not(feature = "task-debug")))]
impl<T, F, Input, Frntend, Bkend, Md> TaskHandler<Input, Frntend, Bkend, Md> for F
where
    F: FnOnce(&mut Frntend, Input) -> T,
    T: Into<AsyncTask<Frntend, Bkend, Md>>,
    Input: 'static,
{
    fn handle(self, input: Input) -> impl FrontendEffect<Frntend, Bkend, Md> {
        |this: &mut Frntend| self(this, input)
    }
}

/// Allow closures to be accepted as TaskHandlers if equality and debug features
/// are not required.
impl<F, T, Frntend, Bkend, Md> FrontendEffect<Frntend, Bkend, Md> for F
where
    F: FnOnce(&mut Frntend) -> T,
    T: Into<AsyncTask<Frntend, Bkend, Md>>,
{
    fn apply(self, target: &mut Frntend) -> impl Into<AsyncTask<Frntend, Bkend, Md>> {
        self(target).into()
    }
}

/// Helper handler for a task that returns a Result<T,E>
#[derive(PartialEq, Clone, Debug)]
pub(crate) struct TryHandler<OkH, ErrH> {
    pub(crate) ok_handler: OkH,
    pub(crate) err_handler: ErrH,
}

impl<OkH, ErrH, T, E, Frntend, Bkend, Md> TaskHandler<Result<T, E>, Frntend, Bkend, Md>
    for TryHandler<OkH, ErrH>
where
    OkH: TaskHandler<T, Frntend, Bkend, Md>,
    ErrH: TaskHandler<E, Frntend, Bkend, Md>,
{
    fn handle(self, output: Result<T, E>) -> impl FrontendEffect<Frntend, Bkend, Md> {
        let Self {
            ok_handler,
            err_handler,
        } = self;
        match output {
            Ok(x) => Either::Left(ok_handler.handle(x)),
            Err(e) => Either::Right(err_handler.handle(e)),
        }
    }
}

/// Helper to utilise static dispatch when returning different types of impl
/// Trait.
#[derive(PartialEq, Clone, Debug)]
pub(crate) enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R, Frntend, Bkend, Md> FrontendEffect<Frntend, Bkend, Md> for Either<L, R>
where
    L: FrontendEffect<Frntend, Bkend, Md>,
    R: FrontendEffect<Frntend, Bkend, Md>,
{
    fn apply(self, target: &mut Frntend) -> impl std::convert::Into<AsyncTask<Frntend, Bkend, Md>> {
        match self {
            Either::Left(x) => x.apply(target).into(),
            Either::Right(x) => x.apply(target).into(),
        }
    }
}

/// Helper handler for a task that returns Option<T>
#[derive(PartialEq, Clone, Debug)]
pub(crate) struct OptionHandler<SomeH>(pub(crate) SomeH);

impl<SomeH, T, Frntend, Bkend, Md> TaskHandler<Option<T>, Frntend, Bkend, Md>
    for OptionHandler<SomeH>
where
    SomeH: TaskHandler<T, Frntend, Bkend, Md>,
{
    fn handle(self, output: Option<T>) -> impl FrontendEffect<Frntend, Bkend, Md> {
        output.map(|output| self.0.handle(output))
    }
}
impl<M, Frntend, Bkend, Md> FrontendEffect<Frntend, Bkend, Md> for Option<M>
where
    M: FrontendEffect<Frntend, Bkend, Md>,
{
    fn apply(self, target: &mut Frntend) -> impl std::convert::Into<AsyncTask<Frntend, Bkend, Md>> {
        let Some(mutation) = self else {
            return AsyncTask::new_no_op();
        };
        mutation.apply(target).into()
    }
}

/// Combination of Task and Handler, which can then have task and handler types
/// erased into an IntoDyn{Future/Stream}Task<Frntend, Bkend, Md>.
#[derive(PartialEq, Debug)]
pub(crate) struct FusedTask<T, H> {
    pub(crate) task: T,
    pub(crate) handler: H,
    // pub(crate) eq_fn: Option<fn(&Self, &Self) -> bool>,
    // NOTE: This could be feature gated.
    // pub(crate) debug_fn: fn(&Self, &mut std::fmt::Formatter) -> Result<(), std::fmt::Error>,
}

#[cfg(feature = "task-equality")]
impl<T, H> DynPartialEq for FusedTask<T, H>
where
    T: PartialEq + 'static,
    H: PartialEq + 'static,
{
    fn dyn_partial_eq(&self, other: &dyn DynPartialEq) -> bool {
        // let eq_fn = self.eq_fn?;

        use std::any::Any;
        let Some(other) = (other as &dyn Any).downcast_ref::<Self>() else {
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
            // eq_fn: Some(|t1, t2| t1.task == t2.task && t1.handler == t2.handler),
            // debug_fn: |this, f| {
            //     f.debug_struct("FusedTask")
            //         .field("task", &this.task)
            //         .field("handler", &this.handler)
            //         .finish_non_exhaustive()
            // },
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
            // eq_fn: None,
            // debug_fn: |_, f| {
            //     f.debug_struct("FusedTask")
            //         .field("task", &"{{BackendStreamingTask}}")
            //         .field("handler", &"{{TaskHandler}}")
            //         .finish_non_exhaustive()
            // },
        }
    }
}
