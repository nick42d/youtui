//! Integration tests for async-callback-manager.

use async_callback_manager::{
    AsyncCallbackManager, AsyncCallbackSender, BackendStreamingTask, BackendTask, Constraint,
};
use futures::{FutureExt, StreamExt};
use std::{future::Future, ops::Deref, sync::Arc, time::Duration};
use tokio::sync::Mutex;

const DEFAULT_CHANNEL_SIZE: usize = 10;

struct TextTask(String);
struct DelayedBackendMutatingRequest(String);
struct StreamingCounterTask(usize);
struct DelayedBackendMutatingStreamingCounterTask(usize);
#[derive(Default)]
struct MockMutatingBackend {
    msgs_recvd: usize,
}

impl BackendTask<Arc<Mutex<MockMutatingBackend>>> for DelayedBackendMutatingRequest {
    type Output = String;
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
    // Manual async function required due to bounds.
    #[allow(clippy::manual_async_fn)]
    fn into_future(self, _: &T) -> impl Future<Output = Self::Output> + Send + 'static {
        async { self.0 }
    }
}
impl<T> BackendStreamingTask<T> for StreamingCounterTask {
    type Output = usize;
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

fn init<Bkend: Clone, Frntend>() -> (
    AsyncCallbackManager<Bkend>,
    AsyncCallbackSender<Bkend, Frntend>,
) {
    let mut manager = async_callback_manager::AsyncCallbackManager::new(DEFAULT_CHANNEL_SIZE);
    let sender = manager.new_sender(DEFAULT_CHANNEL_SIZE);
    (manager, sender)
}

async fn drain_manager<Bkend: Clone>(mut manager: AsyncCallbackManager<Bkend>, _: Bkend) {
    loop {
        if manager.process_next_response().await.is_none() {
            return;
        }
    }
}

#[tokio::test]
async fn test_mutate_once() {
    let mut state = String::new();
    let (mut manager, mut state_receiver) = init();
    state_receiver
        .add_callback(
            TextTask("Hello from the future".to_string()),
            |state, new| *state = new,
            None,
        )
        .await
        .unwrap();
    manager.spawn_next_task(()).await;
    drain_manager(manager, ()).await;
    state_receiver
        .get_next_mutations(50)
        .await
        .apply(&mut state);
    assert_eq!(state, "Hello from the future".to_string());
}

#[tokio::test]
async fn test_mutate_twice() {
    let mut state = Vec::new();
    let (mut manager, mut state_receiver) = init();
    state_receiver
        .add_callback(
            TextTask("Message 1".to_string()),
            |state: &mut Vec<_>, new| state.push(new),
            None,
        )
        .await
        .unwrap();
    manager.spawn_next_task(()).await;
    state_receiver
        .add_callback(
            TextTask("Message 2".to_string()),
            |state, new| state.push(new),
            None,
        )
        .await
        .unwrap();
    manager.spawn_next_task(()).await;
    drain_manager(manager, ()).await;
    state_receiver
        .get_next_mutations(50)
        .await
        .apply(&mut state);
    assert_eq!(
        state,
        vec!["Message 1".to_string(), "Message 2".to_string()]
    );
}

#[tokio::test]
async fn test_mutate_stream() {
    let mut state = Vec::new();
    let (mut manager, mut state_receiver) = init();
    state_receiver
        .add_stream_callback(
            StreamingCounterTask(10),
            |state: &mut Vec<_>, new| state.push(new),
            None,
        )
        .await
        .unwrap();
    manager.spawn_next_task(()).await;
    drain_manager(manager, ()).await;
    state_receiver
        .get_next_mutations(50)
        .await
        .apply(&mut state);
    assert_eq!(state, (0..10).collect::<Vec<_>>());
}

#[tokio::test]
async fn test_mutate_stream_twice() {
    let backend = Arc::new(Mutex::new(MockMutatingBackend::default()));
    let mut state = Vec::new();
    let (mut manager, mut state_receiver) = init();
    state_receiver
        .add_stream_callback(
            DelayedBackendMutatingStreamingCounterTask(5),
            |state: &mut Vec<_>, new| state.push(new),
            None,
        )
        .await
        .unwrap();
    manager.spawn_next_task(backend.clone()).await;
    state_receiver
        .add_stream_callback(
            DelayedBackendMutatingStreamingCounterTask(5),
            |state: &mut Vec<_>, new| state.push(new),
            None,
        )
        .await
        .unwrap();
    manager.spawn_next_task(backend.clone()).await;
    drain_manager(manager, backend).await;
    state_receiver
        .get_next_mutations(50)
        .await
        .apply(&mut state);
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
    let (mut manager, mut state_receiver) = init::<_, Vec<_>>();
    state_receiver
        .add_callback(
            DelayedBackendMutatingRequest("This message should get blocked!".to_string()),
            |state, new| state.push(new),
            None,
        )
        .await
        .unwrap();
    manager.spawn_next_task(backend.clone()).await;
    state_receiver
        .add_callback(
            DelayedBackendMutatingRequest("Message 2".to_string()),
            |state, new| state.push(new),
            Some(Constraint::new_block_same_type()),
        )
        .await
        .unwrap();
    manager.spawn_next_task(backend.clone()).await;
    drain_manager(manager, backend.clone()).await;
    state_receiver
        .get_next_mutations(50)
        .await
        .apply(&mut state);
    let backend_counter = backend.lock().await.msgs_recvd;
    assert_eq!(state, vec!["Message 2".to_string()]);
    assert_eq!(backend_counter, 2)
}

#[tokio::test]
async fn test_kill_constraint() {
    let mut state = vec![];
    let backend = Arc::new(Mutex::new(MockMutatingBackend::default()));
    let (mut manager, mut state_receiver) = init::<_, Vec<_>>();
    state_receiver
        .add_callback(
            DelayedBackendMutatingRequest("This message should get killed!".to_string()),
            |state, new| state.push(new),
            None,
        )
        .await
        .unwrap();
    manager.spawn_next_task(backend.clone()).await;
    state_receiver
        .add_callback(
            DelayedBackendMutatingRequest("Message 2".to_string()),
            |state, new| state.push(new),
            Some(Constraint::new_kill_same_type()),
        )
        .await
        .unwrap();
    manager.spawn_next_task(backend.clone()).await;
    drain_manager(manager, backend.clone()).await;
    state_receiver
        .get_next_mutations(50)
        .await
        .apply(&mut state);
    let backend_counter = backend.lock().await.msgs_recvd;
    assert_eq!(state, vec!["Message 2".to_string()]);
    assert_eq!(backend_counter, 1)
}

#[tokio::test]
async fn test_block_constraint_stream() {
    let backend = Arc::new(Mutex::new(MockMutatingBackend::default()));
    let mut state = vec![];
    let (mut manager, mut state_receiver) = init::<_, Vec<_>>();
    state_receiver
        .add_stream_callback(
            DelayedBackendMutatingStreamingCounterTask(5),
            |state, new| state.push(new),
            None,
        )
        .await
        .unwrap();
    manager.spawn_next_task(backend.clone()).await;
    state_receiver
        .add_stream_callback(
            DelayedBackendMutatingStreamingCounterTask(5),
            |state, new| state.push(new),
            Some(Constraint::new_block_same_type()),
        )
        .await
        .unwrap();
    manager.spawn_next_task(backend.clone()).await;
    drain_manager(manager, backend.clone()).await;
    state_receiver
        .get_next_mutations(50)
        .await
        .apply(&mut state);
    let backend_counter = backend.lock().await.msgs_recvd;
    assert_eq!(state, vec![0, 1, 2, 3, 4]);
    assert_eq!(backend_counter, 10)
}

#[tokio::test]
async fn test_kill_constraint_stream() {
    let backend = Arc::new(Mutex::new(MockMutatingBackend::default()));
    let mut state = vec![];
    let (mut manager, mut state_receiver) = init::<_, Vec<_>>();
    state_receiver
        .add_stream_callback(
            DelayedBackendMutatingStreamingCounterTask(5),
            |state, new| state.push(new),
            None,
        )
        .await
        .unwrap();
    manager.spawn_next_task(backend.clone()).await;
    state_receiver
        .add_stream_callback(
            DelayedBackendMutatingStreamingCounterTask(5),
            |state, new| state.push(new),
            Some(Constraint::new_kill_same_type()),
        )
        .await
        .unwrap();
    manager.spawn_next_task(backend.clone()).await;
    drain_manager(manager, backend.clone()).await;
    state_receiver
        .get_next_mutations(50)
        .await
        .apply(&mut state);
    let backend_counter = backend.lock().await.msgs_recvd;
    assert_eq!(state, vec![0, 1, 2, 3, 4]);
    assert_eq!(backend_counter, 5)
}

#[tokio::test]
async fn test_task_received_callback() {
    let (manager, state_receiver) = init::<(), ()>();
    let task_received = Arc::new(std::sync::Mutex::new(false));
    let task_received_clone = task_received.clone();
    let mut manager = manager.with_on_task_received_callback(move |resp| {
        eprintln!("Response {:?} received", resp);
        *task_received_clone.lock().unwrap() = true;
    });
    state_receiver
        .add_callback(
            TextTask("Hello from the future".to_string()),
            |_, _| {},
            None,
        )
        .await
        .unwrap();
    manager.manage_next_event(()).await.unwrap();
    assert!(*task_received.lock().unwrap());
}

#[tokio::test]
async fn test_response_received_callback() {
    let (manager, state_receiver) = init::<(), ()>();
    let response_received = Arc::new(std::sync::Mutex::new(false));
    let response_received_clone = response_received.clone();
    let task_is_now_finished = Arc::new(std::sync::Mutex::new(false));
    let task_is_now_finished_clone = task_is_now_finished.clone();
    let mut manager = manager.with_on_response_received_callback(move |resp| {
        eprintln!("Response {:?} received", resp);
        *response_received_clone.lock().unwrap() = true;
        *task_is_now_finished_clone.lock().unwrap() = resp.task_is_now_finished;
    });
    state_receiver
        .add_callback(
            TextTask("Hello from the future".to_string()),
            |_, _| {},
            None,
        )
        .await
        .unwrap();
    manager.manage_next_event(()).await.unwrap();
    manager.manage_next_event(()).await.unwrap();
    assert!(*response_received.lock().unwrap());
    assert!(*task_is_now_finished.lock().unwrap());
}
