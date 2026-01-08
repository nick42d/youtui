use crate::{AsyncTask, BackendStreamingTask, BackendTask};
use futures::StreamExt;
#[derive(Debug)]
struct Task1;
#[derive(Debug)]
struct Task2;
#[derive(Debug)]
struct StreamingTask;
impl BackendTask<()> for Task1 {
    type Output = ();
    type MetadataType = ();
    #[allow(clippy::manual_async_fn)]
    fn into_future(
        self,
        _: &(),
    ) -> impl std::future::Future<Output = Self::Output> + Send + 'static {
        async {}
    }
}
impl BackendTask<()> for Task2 {
    type Output = ();
    type MetadataType = ();
    #[allow(clippy::manual_async_fn)]
    fn into_future(
        self,
        _: &(),
    ) -> impl std::future::Future<Output = Self::Output> + Send + 'static {
        async {}
    }
}
impl BackendStreamingTask<()> for StreamingTask {
    type Output = ();
    type MetadataType = ();
    fn into_stream(
        self,
        _: &(),
    ) -> impl futures::Stream<Item = Self::Output> + Send + Unpin + 'static {
        futures::stream::once(async move {}).boxed()
    }
}
#[tokio::test]
async fn test_recursive_map() {
    let recursive_task = AsyncTask::new_stream_with_closure_handler_chained(
        StreamingTask,
        |_: &mut (), _| {
            AsyncTask::new_future_with_closure_handler_chained(
                Task1,
                |_: &mut (), _| {
                    AsyncTask::new_future_with_closure_handler(Task2, |_: &mut (), _| {}, None)
                },
                None,
            )
        },
        None,
    );
    // Here, it's expected that this is succesful.
    // TODO: Run the task for an expected outcome.
    #[allow(unused_must_use)]
    let _ = recursive_task.map(|tmp: &mut ()| tmp);
}
