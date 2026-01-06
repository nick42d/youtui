use crate::{
    BackendStreamingTask, BackendTask, DEFAULT_STREAM_CHANNEL_SIZE, PanickingReceiverStream,
};
use futures::{Stream, StreamExt};
use std::any::Any;
use std::fmt::Debug;
use std::future::Future;
use std::marker::PhantomData;

impl<Bkend, T: BackendTask<Bkend>> BackendTaskExt<Bkend> for T {}
impl<Bkend, T: BackendTask<Bkend, Output = Result<O, E>>, O, E> TryBackendTaskExt<Bkend> for T {
    type Error = E;
    type Ok = O;
}

pub trait MapFn<T> {
    type Output;
    fn apply(self, input: T) -> Self::Output;
}
impl<T, F, O> MapFn<T> for F
where
    F: FnOnce(T) -> O,
{
    type Output = O;
    fn apply(self, input: T) -> Self::Output {
        self(input)
    }
}

pub trait TryBackendTaskExt<Bkend>: BackendTask<Bkend> {
    type Error;
    type Ok;
    fn map_stream<S, F>(self, create_next: F) -> Map<Self, F, S>
    where
        Self: Sized,
        S: BackendStreamingTask<Bkend>,
        F: MapFn<Self::Ok, Output = S>,
    {
        Map {
            first: self,
            create_next,
            create_next_type: PhantomData,
        }
    }
}
pub trait BackendTaskExt<Bkend>: BackendTask<Bkend> {
    fn then<T, F>(self, create_next: F) -> Then<Self, F>
    where
        Self: Sized,
        T: BackendTask<Bkend>,
        F: FnOnce(Self::Output) -> T,
    {
        Then {
            first: self,
            create_next,
        }
    }
    fn then_stream<S, F>(self, create_next: F) -> Then<Self, F>
    where
        Self: Sized,
        S: BackendStreamingTask<Bkend>,
        F: FnOnce(Self::Output) -> S,
    {
        Then {
            first: self,
            create_next,
        }
    }
}

pub struct Map<T, F, Ty> {
    first: T,
    create_next: F,
    /// Used for introspection / debugging (ie, consumer can pring output type
    /// name.
    create_next_type: PhantomData<Ty>,
}

impl<T, F, Ty> PartialEq for Map<T, F, Ty>
where
    T: PartialEq,
    F: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.first == other.first && self.create_next == other.create_next
    }
}

impl<T, F, Ty> Debug for Map<T, F, Ty>
where
    T: Debug,
    Ty: Any,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Map")
            .field("first", &self.first)
            // TODO: we could deduce the type name returned by the closure
            .field(
                "create_next",
                &format!("{{MapFn}}->{{{}}}", std::any::type_name::<Ty>()),
            )
            .finish()
    }
}

impl<Bkend, T, S, F, Ct, O, E, Ty> BackendStreamingTask<Bkend> for Map<T, F, Ty>
where
    Bkend: Clone + Sync + Send + 'static,
    F: Sync + Send + 'static,
    T: BackendTask<Bkend, MetadataType = Ct, Output = std::result::Result<O, E>>,
    S: BackendStreamingTask<Bkend, MetadataType = Ct>,
    Ct: PartialEq,
    F: MapFn<O, Output = S>,
    E: Send + 'static,
    Ty: Send + 'static,
    O: Send,
{
    type Output = std::result::Result<S::Output, E>;
    type MetadataType = Ct;
    fn into_stream(
        self,
        backend: &Bkend,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin + 'static {
        let Map {
            first, create_next, ..
        } = self;
        let backend = backend.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(DEFAULT_STREAM_CHANNEL_SIZE);
        let handle = tokio::task::spawn(async move {
            let seed = BackendTask::into_future(first, &backend).await;
            match seed {
                Ok(seed) => {
                    let mut stream = create_next.apply(seed).into_stream(&backend);
                    while let Some(item) = stream.next().await {
                        let _ = tx.send(Ok(item)).await;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                }
            }
        });
        PanickingReceiverStream::new(rx, handle)
    }
    fn metadata() -> Vec<Self::MetadataType> {
        let mut first = T::metadata();
        let mut second = S::metadata();
        second.append(&mut first);
        second
    }
}

pub struct Then<T, F> {
    first: T,
    create_next: F,
}

impl<Bkend, T, T2, F, Ct> BackendTask<Bkend> for Then<T, F>
where
    Bkend: Clone + Send + 'static,
    F: Sync + Send + 'static,
    T: BackendTask<Bkend, MetadataType = Ct>,
    T2: BackendTask<Bkend, MetadataType = Ct>,
    Ct: PartialEq,
    F: FnOnce(T::Output) -> T2,
{
    type Output = T2::Output;
    type MetadataType = Ct;
    fn into_future(self, backend: &Bkend) -> impl Future<Output = Self::Output> + Send + 'static {
        let Then { first, create_next } = self;
        let backend = backend.clone();
        async move {
            let output = BackendTask::into_future(first, &backend).await;
            let next = create_next(output);
            BackendTask::into_future(next, &backend).await
        }
    }
    fn metadata() -> Vec<Self::MetadataType> {
        let mut first = T::metadata();
        let mut second = T2::metadata();
        second.append(&mut first);
        second
    }
}

impl<Bkend, T, S, F, Ct> BackendStreamingTask<Bkend> for Then<T, F>
where
    Bkend: Clone + Sync + Send + 'static,
    F: Sync + Send + 'static,
    T: BackendTask<Bkend, MetadataType = Ct>,
    S: BackendStreamingTask<Bkend, MetadataType = Ct>,
    Ct: PartialEq,
    F: FnOnce(T::Output) -> S,
{
    type Output = S::Output;
    type MetadataType = Ct;
    fn into_stream(
        self,
        backend: &Bkend,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin + 'static {
        let Then { first, create_next } = self;
        let backend = backend.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(DEFAULT_STREAM_CHANNEL_SIZE);
        let handle = tokio::task::spawn(async move {
            let seed = BackendTask::into_future(first, &backend).await;
            let mut stream = create_next(seed).into_stream(&backend);
            while let Some(item) = stream.next().await {
                let _ = tx.send(item).await;
            }
        });
        PanickingReceiverStream::new(rx, handle)
    }
    fn metadata() -> Vec<Self::MetadataType> {
        let mut first = T::metadata();
        let mut second = S::metadata();
        second.append(&mut first);
        second
    }
}
