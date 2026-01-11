use crate::task::dyn_task::{
    self, DynFutureTask, DynMutationFuture, DynMutationStream, DynStateMutation, DynStreamTask,
    IntoDynFutureTask, IntoDynStreamTask,
};
use futures::FutureExt;
use tokio_stream::StreamExt;

pub struct MapDynFutureTask<Frntend, Bkend, Md, F> {
    pub(crate) task: Box<dyn IntoDynFutureTask<Frntend, Bkend, Md>>,
    pub(crate) map_fn: F,
}
pub struct MapDynStreamTask<Frntend, Bkend, Md, F> {
    pub(crate) task: Box<dyn IntoDynStreamTask<Frntend, Bkend, Md>>,
    pub(crate) map_fn: F,
}

#[cfg(feature = "task-debug")]
impl<Frntend, Bkend, Md, F> Debug for MapDynFutureTask<Frntend, Bkend, Md, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapDynFutureTask")
            .field("task", &self.task)
            .field("map_fn", &"{{closure}}")
            .finish()
    }
}

#[cfg(feature = "task-debug")]
impl<Frntend, Bkend, Md, F> Debug for MapDynStreamTask<Frntend, Bkend, Md, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapDynStreamTask")
            .field("task", &self.task)
            .field("map_fn", &"{{closure}}")
            .finish()
    }
}

#[cfg(feature = "task-equality")]
impl<Frntend, Bkend, Md, F> dyn_task::DynPartialEq for MapDynFutureTask<Frntend, Bkend, Md, F>
where
    F: 'static,
    Md: 'static,
    Frntend: 'static,
    Bkend: 'static,
{
    fn dyn_partial_eq(&self, other: &dyn dyn_task::DynPartialEq) -> bool {
        // Note - map function is not checked. It's assumed that this doesn't change the
        // equality in any meaningful way.
        let Some(other) = (other as &dyn Any).downcast_ref::<Self>() else {
            return false;
        };
        self.task.dyn_partial_eq(other.task.as_ref())
    }
}

#[cfg(feature = "task-equality")]
impl<Frntend, Bkend, Md, F> dyn_task::DynPartialEq for MapDynStreamTask<Frntend, Bkend, Md, F>
where
    F: 'static,
    Md: 'static,
    Frntend: 'static,
    Bkend: 'static,
{
    fn dyn_partial_eq(&self, other: &dyn dyn_task::DynPartialEq) -> bool {
        // Note - map function is not checked. It's assumed that this doesn't change the
        // equality in any meaningful way.
        let Some(other) = (other as &dyn Any).downcast_ref::<Self>() else {
            return false;
        };
        self.task.as_ref().dyn_partial_eq(other.task.as_ref())
    }
}
impl<F, Frntend, NewFrntend, Bkend, Md> IntoDynFutureTask<NewFrntend, Bkend, Md>
    for MapDynFutureTask<Frntend, Bkend, Md, F>
where
    F: FnOnce(&mut NewFrntend) -> &mut Frntend + Clone + Send + 'static,
    Md: 'static,
    Frntend: 'static,
    Bkend: 'static,
{
    fn into_dyn_task(self: Box<Self>) -> dyn_task::DynFutureTask<NewFrntend, Bkend, Md> {
        let Self { task, map_fn } = *self;
        let future = task.into_dyn_task();
        map_dyn_future_task(future, map_fn)
    }
}
impl<F, Frntend, NewFrntend, Bkend, Md> IntoDynStreamTask<NewFrntend, Bkend, Md>
    for MapDynStreamTask<Frntend, Bkend, Md, F>
where
    F: FnOnce(&mut NewFrntend) -> &mut Frntend + Clone + Send + 'static,
    Md: 'static,
    Frntend: 'static,
    Bkend: 'static,
{
    fn into_dyn_stream(self: Box<Self>) -> dyn_task::DynStreamTask<NewFrntend, Bkend, Md> {
        let Self { task, map_fn } = *self;
        let stream = task.into_dyn_stream();
        map_dyn_stream_task(stream, map_fn)
    }
}

pub(crate) fn map_dyn_stream_task<NewFrntend, Frntend, Bkend, Md>(
    task: DynStreamTask<Frntend, Bkend, Md>,
    f: impl FnOnce(&mut NewFrntend) -> &mut Frntend + Clone + Send + 'static,
) -> DynStreamTask<NewFrntend, Bkend, Md>
where
    Frntend: 'static,
    Bkend: 'static,
    Md: 'static,
{
    Box::new(move |b: &Bkend| {
        let stream = task(b);
        Box::new({
            stream.map(move |output| {
                let f = f.clone();
                Box::new(move |frontend: &mut NewFrntend| {
                    output(f.clone()(frontend)).map_frontend(f)
                }) as DynStateMutation<NewFrntend, Bkend, Md>
            })
        }) as DynMutationStream<NewFrntend, Bkend, Md>
    }) as DynStreamTask<NewFrntend, Bkend, Md>
}

pub(crate) fn map_dyn_future_task<NewFrntend, Frntend, Bkend, Md>(
    task: DynFutureTask<Frntend, Bkend, Md>,
    f: impl FnOnce(&mut NewFrntend) -> &mut Frntend + Clone + Send + 'static,
) -> DynFutureTask<NewFrntend, Bkend, Md>
where
    Frntend: 'static,
    Bkend: 'static,
    Md: 'static,
{
    Box::new(move |b: &Bkend| {
        let task = task(b);
        Box::new({
            task.map(move |output| {
                Box::new(move |frontend: &mut NewFrntend| {
                    output(f.clone()(frontend)).map_frontend(f)
                }) as DynStateMutation<NewFrntend, Bkend, Md>
            })
        }) as DynMutationFuture<NewFrntend, Bkend, Md>
    }) as DynFutureTask<NewFrntend, Bkend, Md>
}
