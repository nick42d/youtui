use crate::{FrontendMutation, TaskHandler};

pub(crate) struct MappedHandler<H, F> {
    handler: H,
    map_fn: F,
}
impl<H, F> MappedHandler<H, F> {
    fn new<NewFrntend, Frntend>(handler: H, map_fn: F) -> MappedHandler<H, F>
    where
        F: Fn(&mut NewFrntend) -> &mut Frntend,
    {
        Self { handler, map_fn }
    }
}
impl<H, F, Output, NewFrntend, Frntend, Bkend, Md> TaskHandler<Output, NewFrntend, Bkend, Md>
    for MappedHandler<H, F>
where
    H: TaskHandler<Output, Frntend, Bkend, Md>,
    F: Fn(&mut NewFrntend) -> &mut Frntend + Clone + Send + 'static,
    Md: 'static,
    Frntend: 'static,
    Bkend: 'static,
{
    fn handle(self, output: Output) -> impl crate::FrontendMutation<NewFrntend, Bkend, Md> {
        let Self { handler, map_fn } = self;
        let mutation = handler.handle(output);
        MappedMutation { mutation, map_fn }
    }
}
pub(crate) struct MappedMutation<M, F> {
    mutation: M,
    map_fn: F,
}
impl<M, F> MappedMutation<M, F> {
    fn new<NewFrntend, Frntend>(mutation: M, map_fn: F) -> MappedMutation<M, F>
    where
        F: Fn(&mut NewFrntend) -> &mut Frntend,
    {
        Self { mutation, map_fn }
    }
}
impl<M, F, NewFrntend, Frntend, Bkend, Md> FrontendMutation<NewFrntend, Bkend, Md>
    for MappedMutation<M, F>
where
    M: FrontendMutation<Frntend, Bkend, Md>,
    F: Fn(&mut NewFrntend) -> &mut Frntend + Clone + Send + 'static,
    Md: 'static,
    Frntend: 'static,
    Bkend: 'static,
{
    fn apply(self, target: &mut NewFrntend) -> crate::AsyncTask<NewFrntend, Bkend, Md> {
        let Self { mutation, map_fn } = self;
        let target = map_fn(target);
        mutation.apply(target).map(map_fn)
    }
}
