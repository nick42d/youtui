//! Integration tests for async-callback-manager.

use async_callback_manager::{
    AsyncCallbackManager, AsyncTask, BackendStreamingTask, BackendTask, Constraint,
};
use futures::{FutureExt, StreamExt};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::Mutex;

#[derive(Debug)]
struct TextTask(String);
#[derive(Debug)]
struct DelayedBackendMutatingRequest(String);
#[derive(Debug)]
struct StreamingCounterTask(usize);
#[derive(Debug)]
struct DelayedBackendMutatingStreamingCounterTask(usize);
#[derive(Default)]
struct MockMutatingBackend {
    msgs_recvd: usize,
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

async fn drain_manager<Frntend, Bkend, Md>(
    mut manager: AsyncCallbackManager<Frntend, Bkend, Md>,
    s: &mut Frntend,
    b: &Bkend,
) where
    Md: PartialEq + 'static,
    Frntend: 'static,
    Bkend: Clone + 'static,
{
    loop {
        let Some(resp) = manager.get_next_response().await else {
            return;
        };
        match resp {
            async_callback_manager::TaskOutcome::StreamClosed => continue,
            async_callback_manager::TaskOutcome::MutationReceived { mutation, .. } => {
                manager.spawn_task(b, mutation(s))
            }
            async_callback_manager::TaskOutcome::TaskPanicked { .. } => panic!(),
        }
    }
}

#[tokio::test]
async fn test_mutate_once() {
    let mut state = String::new();
    let mut manager = AsyncCallbackManager::new();
    let task = AsyncTask::new_future(
        TextTask("Hello from the future".to_string()),
        |state, new| *state = new,
        None,
    );
    manager.spawn_task(&(), task);
    drain_manager(manager, &mut state, &()).await;
    assert_eq!(state, "Hello from the future".to_string());
}

#[tokio::test]
async fn test_mutate_twice() {
    let mut state = Vec::new();
    let mut manager = AsyncCallbackManager::new();
    let task = AsyncTask::new_future(
        TextTask("Message 1".to_string()),
        |state: &mut Vec<_>, new| state.push(new),
        None,
    );
    manager.spawn_task(&(), task);
    let task = AsyncTask::new_future(
        TextTask("Message 2".to_string()),
        |state: &mut Vec<_>, new| state.push(new),
        None,
    );
    manager.spawn_task(&(), task);
    drain_manager(manager, &mut state, &()).await;
    assert_eq!(
        state,
        vec!["Message 1".to_string(), "Message 2".to_string()]
    );
}

#[tokio::test]
async fn test_mutate_stream() {
    let mut state = Vec::new();
    let mut manager = AsyncCallbackManager::new();
    let task = AsyncTask::new_stream(
        StreamingCounterTask(10),
        |state: &mut Vec<_>, new| state.push(new),
        None,
    );
    manager.spawn_task(&(), task);
    drain_manager(manager, &mut state, &()).await;
    assert_eq!(state, (0..10).collect::<Vec<_>>());
}

#[tokio::test]
async fn test_mutate_stream_twice() {
    let backend = Arc::new(Mutex::new(MockMutatingBackend::default()));
    let mut state = Vec::new();
    let mut manager = AsyncCallbackManager::new();
    let task = AsyncTask::new_stream(
        DelayedBackendMutatingStreamingCounterTask(5),
        |state: &mut Vec<_>, new| state.push(new),
        None,
    );
    manager.spawn_task(&backend, task);
    let task = AsyncTask::new_stream(
        DelayedBackendMutatingStreamingCounterTask(5),
        |state: &mut Vec<_>, new| state.push(new),
        None,
    );
    manager.spawn_task(&backend, task);
    drain_manager(manager, &mut state, &backend).await;
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
    let task = AsyncTask::new_future(
        DelayedBackendMutatingRequest("This message should get blocked!".to_string()),
        |state: &mut Vec<_>, new| state.push(new),
        None,
    );
    manager.spawn_task(&backend, task);
    let task = AsyncTask::new_future(
        DelayedBackendMutatingRequest("Message 2".to_string()),
        |state: &mut Vec<_>, new| state.push(new),
        Some(Constraint::new_block_same_type()),
    );
    manager.spawn_task(&backend, task);
    drain_manager(manager, &mut state, &backend).await;
    let backend_counter = backend.lock().await.msgs_recvd;
    assert_eq!(state, vec!["Message 2".to_string()]);
    assert_eq!(backend_counter, 2)
}

#[tokio::test]
async fn test_kill_constraint() {
    let mut state = vec![];
    let backend = Arc::new(Mutex::new(MockMutatingBackend::default()));
    let mut manager = AsyncCallbackManager::new();
    let task = AsyncTask::new_future(
        DelayedBackendMutatingRequest("This message should get killed!".to_string()),
        |state: &mut Vec<_>, new| state.push(new),
        None,
    );
    manager.spawn_task(&backend, task);
    let task = AsyncTask::new_future(
        DelayedBackendMutatingRequest("Message 2".to_string()),
        |state: &mut Vec<_>, new| state.push(new),
        Some(Constraint::new_kill_same_type()),
    );
    manager.spawn_task(&backend, task);
    drain_manager(manager, &mut state, &backend).await;
    let backend_counter = backend.lock().await.msgs_recvd;
    assert_eq!(state, vec!["Message 2".to_string()]);
    assert_eq!(backend_counter, 1)
}

#[tokio::test]
async fn test_block_constraint_stream() {
    let backend = Arc::new(Mutex::new(MockMutatingBackend::default()));
    let mut state = vec![];
    let mut manager = AsyncCallbackManager::new();
    let task = AsyncTask::new_stream(
        DelayedBackendMutatingStreamingCounterTask(5),
        |state: &mut Vec<_>, new| state.push(new),
        None,
    );
    manager.spawn_task(&backend, task);
    let task = AsyncTask::new_stream(
        DelayedBackendMutatingStreamingCounterTask(5),
        |state: &mut Vec<_>, new| state.push(new),
        Some(Constraint::new_block_same_type()),
    );
    manager.spawn_task(&backend, task);
    drain_manager(manager, &mut state, &backend).await;
    let backend_counter = backend.lock().await.msgs_recvd;
    assert_eq!(state, vec![0, 1, 2, 3, 4]);
    assert_eq!(backend_counter, 10)
}

#[tokio::test]
async fn test_kill_constraint_stream() {
    let backend = Arc::new(Mutex::new(MockMutatingBackend::default()));
    let mut state = vec![];
    let mut manager = AsyncCallbackManager::new();
    let task = AsyncTask::new_stream(
        DelayedBackendMutatingStreamingCounterTask(5),
        |state: &mut Vec<_>, new| state.push(new),
        None,
    );
    manager.spawn_task(&backend, task);
    let task = AsyncTask::new_stream(
        DelayedBackendMutatingStreamingCounterTask(5),
        |state: &mut Vec<_>, new| state.push(new),
        Some(Constraint::new_kill_same_type()),
    );
    manager.spawn_task(&backend, task);
    drain_manager(manager, &mut state, &backend).await;
    let backend_counter = backend.lock().await.msgs_recvd;
    assert_eq!(state, vec![0, 1, 2, 3, 4]);
    assert_eq!(backend_counter, 5)
}

#[tokio::test]
async fn test_task_spawn_callback() {
    let task_received = Arc::new(std::sync::Mutex::new(false));
    let task_received_clone = task_received.clone();
    let mut manager = AsyncCallbackManager::new().with_on_task_spawn_callback(move |resp| {
        eprintln!("Response {:?} received", resp);
        *task_received_clone.lock().unwrap() = true;
    });
    let task = AsyncTask::new_future(
        TextTask("Hello from the future".to_string()),
        |_: &mut (), _| {},
        None,
    );
    manager.spawn_task(&(), task);
    assert!(*task_received.lock().unwrap());
}

#[tokio::test]
async fn test_task_spawns_task() {
    let mut state: Vec<String> = vec![];
    let mut manager = AsyncCallbackManager::new();
    let task = AsyncTask::new_future_chained(
        TextTask("Hello".to_string()),
        |state: &mut Vec<_>, output| {
            state.push(output);
            AsyncTask::new_future(
                TextTask("World".to_string()),
                |state: &mut Vec<String>, output| state.push(output),
                None,
            )
        },
        None,
    );
    manager.spawn_task(&(), task);
    drain_manager(manager, &mut state, &()).await;
    assert_eq!(vec!["Hello".to_string(), "World".to_string()], state);
}

#[tokio::test]
async fn test_recursive_map() {
    todo!()
}
