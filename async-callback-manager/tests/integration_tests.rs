//! Integration tests for async-callback-manager.
use async_callback_manager::{
    AsyncCallbackManager, AsyncTask, BackendStreamingTask, BackendTask, Constraint, FrontendEffect,
    TaskHandler,
};
use futures::{FutureExt, StreamExt};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Debug, PartialEq)]
struct TextTask(String);
#[derive(Debug, PartialEq)]
struct DelayedBackendMutatingRequest(String);
#[derive(Debug, PartialEq)]
struct StreamingCounterTask(usize);
#[derive(Debug, PartialEq)]
struct PanickingStreamingCounterTask {
    count: usize,
    panic_on: usize,
}
#[derive(Debug, PartialEq)]
struct DelayedBackendMutatingStreamingCounterTask(usize);
#[derive(Default)]
struct MockMutatingBackend {
    msgs_recvd: usize,
}

#[derive(PartialEq, Debug, Clone)]
struct PushToVecHandler;
impl<T, Bkend, Md> TaskHandler<T, Vec<T>, Bkend, Md> for PushToVecHandler {
    fn handle(self, input: T) -> impl FrontendEffect<Vec<T>, Bkend, Md> {
        move |this: &mut Vec<_>| this.push(input)
    }
}

impl BackendTask<Arc<Mutex<MockMutatingBackend>>> for DelayedBackendMutatingRequest {
    type Output = String;
    type MetadataType = ();
    fn into_future(
        self,
        backend: &Arc<Mutex<MockMutatingBackend>>,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let backend = backend.clone();
        async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let mut lock = backend.lock().await;
            lock.msgs_recvd += 1;
            self.0
        }
        .boxed()
    }
}
impl<T: Send> BackendTask<T> for TextTask {
    type Output = String;
    type MetadataType = ();
    // Manual async function required due to bounds.
    #[allow(clippy::manual_async_fn)]
    fn into_future(self, _: &T) -> impl Future<Output = Self::Output> + Send + 'static {
        async { self.0 }
    }
}
impl<T> BackendStreamingTask<T> for StreamingCounterTask {
    type Output = usize;
    type MetadataType = ();
    fn into_stream(
        self,
        _: &T,
    ) -> impl futures::Stream<Item = Self::Output> + Send + Unpin + 'static {
        futures::stream::iter(0..self.0)
    }
}
impl<T> BackendStreamingTask<T> for PanickingStreamingCounterTask {
    type Output = usize;
    type MetadataType = ();
    fn into_stream(
        self,
        _: &T,
    ) -> impl futures::Stream<Item = Self::Output> + Send + Unpin + 'static {
        futures::stream::unfold(0, move |x| async move {
            if x == self.panic_on {
                panic!("hit target");
            }
            if x == self.count {
                return None;
            }
            Some((x, x + 1))
        })
        .boxed()
    }
}
impl BackendStreamingTask<Arc<Mutex<MockMutatingBackend>>>
    for DelayedBackendMutatingStreamingCounterTask
{
    type Output = usize;
    type MetadataType = ();
    fn into_stream(
        self,
        backend: &Arc<Mutex<MockMutatingBackend>>,
    ) -> impl futures::Stream<Item = Self::Output> + Send + Unpin + 'static {
        let backend = backend.clone();
        futures::stream::iter(0..self.0)
            .then(move |num| {
                let backend = backend.clone();
                async move {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    let backend = backend.clone();
                    let mut lock = backend.lock().await;
                    lock.msgs_recvd += 1;
                    num
                }
            })
            .boxed()
    }
}
#[derive(Debug, PartialEq)]
enum PanicType {
    Stream,
    Task,
}
async fn drain_manager<Frntend, Bkend, Md>(
    mut manager: AsyncCallbackManager<Frntend, Bkend, Md>,
    s: &mut Frntend,
    b: &Bkend,
) -> Result<(), PanicType>
where
    Md: PartialEq + 'static,
    Frntend: 'static,
    Bkend: Clone + 'static,
{
    loop {
        let Some(resp) = manager.get_next_response().await else {
            return Ok(());
        };
        match resp {
            async_callback_manager::TaskOutcome::StreamFinished { .. } => continue,
            async_callback_manager::TaskOutcome::MutationReceived { mutation, .. } => {
                manager.spawn_task(b, mutation(s))
            }
            async_callback_manager::TaskOutcome::TaskPanicked { .. } => {
                return Err(PanicType::Task);
            }
            async_callback_manager::TaskOutcome::StreamPanicked { .. } => {
                return Err(PanicType::Stream);
            }
        }
    }
}

#[tokio::test]
async fn test_mutate_once() {
    let mut state = String::new();
    let mut manager = AsyncCallbackManager::new();

    #[derive(Debug, PartialEq)]
    struct Handler;
    impl TaskHandler<String, String, (), ()> for Handler {
        fn handle(
            self,
            input: String,
        ) -> impl async_callback_manager::FrontendEffect<String, (), ()> {
            |this: &mut String| *this = input
        }
    }

    #[cfg(not(any(feature = "task-debug", feature = "task-equality")))]
    let handler = |state: &mut String, new| *state = new;

    #[cfg(any(feature = "task-debug", feature = "task-equality"))]
    let handler = Handler;

    let task = AsyncTask::new_future(TextTask("Hello from the future".to_string()), handler, None);
    manager.spawn_task(&(), task);
    drain_manager(manager, &mut state, &()).await.unwrap();
    assert_eq!(state, "Hello from the future".to_string());
}

#[tokio::test]
async fn test_mutate_twice() {
    let mut state = Vec::new();
    let mut manager = AsyncCallbackManager::new();

    #[cfg(not(any(feature = "task-debug", feature = "task-equality")))]
    let handler = |state: &mut Vec<_>, new| state.push(new);
    #[cfg(any(feature = "task-debug", feature = "task-equality"))]
    let handler = PushToVecHandler;

    let task = AsyncTask::new_future(TextTask("Message 1".to_string()), handler.clone(), None);
    manager.spawn_task(&(), task);

    let task = AsyncTask::new_future(TextTask("Message 2".to_string()), handler, None);
    manager.spawn_task(&(), task);
    drain_manager(manager, &mut state, &()).await.unwrap();
    assert_eq!(
        state,
        vec!["Message 1".to_string(), "Message 2".to_string()]
    );
}

#[tokio::test]
async fn test_mutate_stream() {
    let mut state = Vec::new();
    let mut manager = AsyncCallbackManager::new();

    #[cfg(not(any(feature = "task-debug", feature = "task-equality")))]
    let handler = |state: &mut Vec<_>, new| state.push(new);
    #[cfg(any(feature = "task-debug", feature = "task-equality"))]
    let handler = PushToVecHandler;

    let task = AsyncTask::new_stream(StreamingCounterTask(10), handler, None);
    manager.spawn_task(&(), task);
    drain_manager(manager, &mut state, &()).await.unwrap();
    assert_eq!(state, (0..10).collect::<Vec<_>>());
}

#[tokio::test]
async fn test_panicking_stream() {
    let mut state = Vec::new();
    let mut manager = AsyncCallbackManager::new();

    #[cfg(not(any(feature = "task-debug", feature = "task-equality")))]
    let handler = |state: &mut Vec<_>, new| state.push(new);
    #[cfg(any(feature = "task-debug", feature = "task-equality"))]
    let handler = PushToVecHandler;

    let task = AsyncTask::new_stream(
        PanickingStreamingCounterTask {
            count: 10,
            panic_on: 5,
        },
        handler,
        None,
    );
    manager.spawn_task(&(), task);
    let did_stream_panic = drain_manager(manager, &mut state, &()).await;
    assert_eq!(state, (0..5).collect::<Vec<_>>());
    assert_eq!(did_stream_panic, Err(PanicType::Stream))
}

#[tokio::test]
async fn test_mutate_stream_twice() {
    let backend = Arc::new(Mutex::new(MockMutatingBackend::default()));
    let mut state = Vec::new();
    let mut manager = AsyncCallbackManager::new();

    #[cfg(not(any(feature = "task-debug", feature = "task-equality")))]
    let handler = |state: &mut Vec<_>, new| state.push(new);
    #[cfg(any(feature = "task-debug", feature = "task-equality"))]
    let handler = PushToVecHandler;

    let task = AsyncTask::new_stream(
        DelayedBackendMutatingStreamingCounterTask(5),
        handler.clone(),
        None,
    );
    manager.spawn_task(&backend, task);
    let task = AsyncTask::new_stream(DelayedBackendMutatingStreamingCounterTask(5), handler, None);
    manager.spawn_task(&backend, task);
    drain_manager(manager, &mut state, &backend).await.unwrap();
    // Streams should be interleaved
    assert_ne!(state, vec![0, 1, 2, 3, 4, 0, 1, 2, 3, 4]);
    // And should contain all values
    state.sort();
    assert_eq!(state, vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4]);
}

#[tokio::test]
async fn test_block_constraint() {
    let backend = Arc::new(Mutex::new(MockMutatingBackend::default()));
    let mut state = vec![];
    let mut manager = AsyncCallbackManager::new();

    #[cfg(not(any(feature = "task-debug", feature = "task-equality")))]
    let handler = |state: &mut Vec<_>, new| state.push(new);
    #[cfg(any(feature = "task-debug", feature = "task-equality"))]
    let handler = PushToVecHandler;

    let task = AsyncTask::new_future(
        DelayedBackendMutatingRequest("This message should get blocked!".to_string()),
        handler.clone(),
        None,
    );
    manager.spawn_task(&backend, task);
    let task = AsyncTask::new_future(
        DelayedBackendMutatingRequest("Message 2".to_string()),
        handler,
        Some(Constraint::new_block_same_type()),
    );
    manager.spawn_task(&backend, task);
    drain_manager(manager, &mut state, &backend).await.unwrap();
    let backend_counter = backend.lock().await.msgs_recvd;
    assert_eq!(state, vec!["Message 2".to_string()]);
    assert_eq!(backend_counter, 2)
}

#[tokio::test]
async fn test_kill_constraint() {
    let mut state = vec![];
    let backend = Arc::new(Mutex::new(MockMutatingBackend::default()));
    let mut manager = AsyncCallbackManager::new();

    #[cfg(not(any(feature = "task-debug", feature = "task-equality")))]
    let handler = |state: &mut Vec<_>, new| state.push(new);
    #[cfg(any(feature = "task-debug", feature = "task-equality"))]
    let handler = PushToVecHandler;

    let task = AsyncTask::new_future(
        DelayedBackendMutatingRequest("This message should get killed!".to_string()),
        handler.clone(),
        None,
    );
    manager.spawn_task(&backend, task);
    let task = AsyncTask::new_future(
        DelayedBackendMutatingRequest("Message 2".to_string()),
        handler,
        Some(Constraint::new_kill_same_type()),
    );
    manager.spawn_task(&backend, task);
    drain_manager(manager, &mut state, &backend).await.unwrap();
    let backend_counter = backend.lock().await.msgs_recvd;
    assert_eq!(state, vec!["Message 2".to_string()]);
    assert_eq!(backend_counter, 1)
}

#[tokio::test]
async fn test_block_constraint_stream() {
    let backend = Arc::new(Mutex::new(MockMutatingBackend::default()));
    let mut state = vec![];
    let mut manager = AsyncCallbackManager::new();

    #[cfg(not(any(feature = "task-debug", feature = "task-equality")))]
    let handler = |state: &mut Vec<_>, new| state.push(new);
    #[cfg(any(feature = "task-debug", feature = "task-equality"))]
    let handler = PushToVecHandler;

    let task = AsyncTask::new_stream(
        DelayedBackendMutatingStreamingCounterTask(5),
        handler.clone(),
        None,
    );
    manager.spawn_task(&backend, task);
    let task = AsyncTask::new_stream(
        DelayedBackendMutatingStreamingCounterTask(5),
        handler,
        Some(Constraint::new_block_same_type()),
    );
    manager.spawn_task(&backend, task);
    drain_manager(manager, &mut state, &backend).await.unwrap();
    let backend_counter = backend.lock().await.msgs_recvd;
    assert_eq!(state, vec![0, 1, 2, 3, 4]);
    assert_eq!(backend_counter, 10)
}

#[tokio::test]
async fn test_kill_constraint_stream() {
    let backend = Arc::new(Mutex::new(MockMutatingBackend::default()));
    let mut state = vec![];
    let mut manager = AsyncCallbackManager::new();

    #[cfg(not(any(feature = "task-debug", feature = "task-equality")))]
    let handler = |state: &mut Vec<_>, new| state.push(new);
    #[cfg(any(feature = "task-debug", feature = "task-equality"))]
    let handler = PushToVecHandler;

    let task = AsyncTask::new_stream(
        DelayedBackendMutatingStreamingCounterTask(5),
        handler.clone(),
        None,
    );
    manager.spawn_task(&backend, task);
    let task = AsyncTask::new_stream(
        DelayedBackendMutatingStreamingCounterTask(5),
        handler,
        Some(Constraint::new_kill_same_type()),
    );
    manager.spawn_task(&backend, task);
    drain_manager(manager, &mut state, &backend).await.unwrap();
    let backend_counter = backend.lock().await.msgs_recvd;
    assert_eq!(state, vec![0, 1, 2, 3, 4]);
    assert_eq!(backend_counter, 5)
}

#[tokio::test]
async fn test_task_spawn_callback() {
    let task_received = Arc::new(std::sync::Mutex::new(false));
    let task_received_clone = task_received.clone();
    let mut manager = AsyncCallbackManager::new().with_on_task_spawn_callback(move |_| {
        *task_received_clone.lock().unwrap() = true;
    });
    #[derive(PartialEq, Debug)]
    struct EmptyHandler;
    impl TaskHandler<String, (), (), ()> for EmptyHandler {
        fn handle(self, _: String) -> impl FrontendEffect<(), (), ()> {
            |_: &mut ()| AsyncTask::new_no_op()
        }
    }

    #[cfg(not(any(feature = "task-debug", feature = "task-equality")))]
    let handler = |_: &mut (), _| ();
    #[cfg(any(feature = "task-debug", feature = "task-equality"))]
    let handler = EmptyHandler;

    let task = AsyncTask::new_future(TextTask("Hello from the future".to_string()), handler, None);
    manager.spawn_task(&(), task);
    assert!(*task_received.lock().unwrap());
}

#[tokio::test]
async fn test_task_spawns_task() {
    let mut state: Vec<String> = vec![];
    let mut manager = AsyncCallbackManager::new();

    #[derive(PartialEq, Debug)]
    struct ChainedHandler;
    impl TaskHandler<String, Vec<String>, (), ()> for ChainedHandler {
        fn handle(self, input: String) -> impl FrontendEffect<Vec<String>, (), ()> {
            |state: &mut Vec<_>| {
                state.push(input);
                AsyncTask::new_future(TextTask("World".to_string()), PushToVecHandler, None)
            }
        }
    }

    #[cfg(not(any(feature = "task-debug", feature = "task-equality")))]
    |state: &mut Vec<_>, output| {
        state.push(output);
        AsyncTask::new_future(
            TextTask("World".to_string()),
            |state: &mut Vec<String>, output| state.push(output),
            None,
        )
    };
    #[cfg(any(feature = "task-debug", feature = "task-equality"))]
    let handler = ChainedHandler;

    let task = AsyncTask::new_future(TextTask("Hello".to_string()), handler, None);
    manager.spawn_task(&(), task);
    drain_manager(manager, &mut state, &()).await.unwrap();
    assert_eq!(vec!["Hello".to_string(), "World".to_string()], state);
}
