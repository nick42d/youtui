use crate::{AsyncTask, BackendStreamingTask, BackendTask, FrontendEffect, TaskHandler};
use futures::StreamExt;
#[derive(Debug, PartialEq)]
struct Task1;
#[derive(Debug, PartialEq)]
struct Task2;
#[derive(Debug, PartialEq)]
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
    #[derive(PartialEq, Debug, Clone)]
    struct Handler1;
    #[derive(PartialEq, Debug, Clone)]
    struct Handler2;
    #[derive(PartialEq, Debug)]
    struct Effect1;
    #[derive(PartialEq, Debug)]
    struct Effect2;
    impl TaskHandler<(), (), (), ()> for Handler1 {
        fn handle(self, _: ()) -> impl crate::FrontendEffect<(), (), ()> {
            Effect1
        }
    }
    impl TaskHandler<(), (), (), ()> for Handler2 {
        fn handle(self, _: ()) -> impl crate::FrontendEffect<(), (), ()> {
            Effect2
        }
    }
    impl FrontendEffect<(), (), ()> for Effect1 {
        fn apply(self, _: &mut ()) -> AsyncTask<(), (), ()> {
            AsyncTask::new_future(Task2, Handler2, None)
        }
    }
    impl FrontendEffect<(), (), ()> for Effect2 {
        fn apply(self, _: &mut ()) -> AsyncTask<(), (), ()> {
            AsyncTask::new_no_op()
        }
    }

    #[cfg(not(any(feature = "task-equality", feature = "task-debug")))]
    let handler = |_: &mut (), _| {
        AsyncTask::new_future(
            Task1,
            |_: &mut (), _| AsyncTask::new_future(Task2, |_: &mut (), _| {}, None),
            None,
        )
    };
    #[cfg(all(feature = "task-equality", feature = "task-debug"))]
    let handler = Handler1;

    let recursive_task = AsyncTask::new_stream(StreamingTask, handler, None);
    // Here, it's expected that this is succesful.
    // TODO: Run the task for an expected outcome.
    #[allow(unused_must_use)]
    let _ = recursive_task.map_frontend(|tmp: &mut ()| tmp);
}
