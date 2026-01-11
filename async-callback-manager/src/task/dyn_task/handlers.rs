use crate::{AsyncTask, FrontendEffect, TaskHandler};

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

#[derive(PartialEq, Clone, Copy, Debug)]
pub struct NoOpHandler;
impl<Input, Frntend, Bkend, Md> TaskHandler<Input, Frntend, Bkend, Md> for NoOpHandler {
    fn handle(self, _: Input) -> impl FrontendEffect<Frntend, Bkend, Md> {
        |_: &mut Frntend| AsyncTask::new_no_op()
    }
}
