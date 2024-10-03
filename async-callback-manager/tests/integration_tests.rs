//! Integration tests for async-callback-manager.

use async_callback_manager::{BackendStreamingTask, BackendTask, Constraint};
use futures::FutureExt;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

const DEFAULT_CHANNEL_SIZE: usize = 10;

struct TextTask(String);
struct DelayedBackendMutatingRequest(String);
struct StreamingCounterTask(usize);
#[derive(Default)]
struct MockBackend {
    msgs_recvd: usize,
}

impl BackendTask<Arc<Mutex<MockBackend>>> for DelayedBackendMutatingRequest {
    type Output = String;
    async fn into_future(self, backend: Arc<Mutex<MockBackend>>) -> Self::Output {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let mut lock = backend.lock().await;
        lock.msgs_recvd += 1;
        self.0
    }
}
impl<T: Send> BackendTask<T> for TextTask {
    type Output = String;
    async fn into_future(self, _: T) -> Self::Output {
        self.0
    }
}
impl<T> BackendStreamingTask<T> for StreamingCounterTask {
    type Output = usize;
    fn into_stream(self, _: T) -> impl futures::Stream<Item = Self::Output> + Send + Unpin {
        futures::stream::iter(0..self.0)
    }
}

#[tokio::test]
async fn test_mutate_once() {
    let mut state = String::new();
    let mut manager = async_callback_manager::AsyncCallbackManager::new(DEFAULT_CHANNEL_SIZE);
    let mut state_receiver = manager.new_sender(DEFAULT_CHANNEL_SIZE);
    state_receiver
        .add_callback(
            TextTask("Hello from the future".to_string()),
            |state, new| *state = new,
            None,
        )
        .await
        .unwrap();
    manager.drain(()).await;
    state_receiver.get_messages().await.apply(&mut state);
    assert_eq!(state, "Hello from the future".to_string());
}

#[tokio::test]
async fn test_mutate_twice() {
    let mut state = Vec::new();
    let mut manager = async_callback_manager::AsyncCallbackManager::new(DEFAULT_CHANNEL_SIZE);
    let mut state_receiver = manager.new_sender::<Vec<_>>(DEFAULT_CHANNEL_SIZE);
    state_receiver
        .add_callback(
            TextTask("Message 1".to_string()),
            |state, new| state.push(new),
            None,
        )
        .await
        .unwrap();
    state_receiver
        .add_callback(
            TextTask("Message 2".to_string()),
            |state, new| state.push(new),
            None,
        )
        .await
        .unwrap();
    manager.drain(()).await;
    state_receiver.get_messages().await.apply(&mut state);
    assert_eq!(
        state,
        vec!["Message 1".to_string(), "Message 2".to_string()]
    );
}

#[tokio::test]
async fn test_mutate_stream() {
    let mut state = Vec::new();
    let mut manager = async_callback_manager::AsyncCallbackManager::<()>::new(DEFAULT_CHANNEL_SIZE);
    let mut state_receiver = manager.new_sender::<Vec<usize>>(DEFAULT_CHANNEL_SIZE);
    state_receiver
        .add_stream_callback(StreamingCounterTask(10), |state, new| state.push(new), None)
        .await
        .unwrap();
    manager.drain(()).await;
    state_receiver.get_messages().await.apply(&mut state);
    assert_eq!(state, (0..10).collect::<Vec<_>>());
}

#[tokio::test]
async fn test_block_constraint() {
    let backend = Arc::new(Mutex::new(MockBackend::default()));
    let mut state = vec![];
    let mut manager = async_callback_manager::AsyncCallbackManager::new(DEFAULT_CHANNEL_SIZE);
    let mut state_receiver = manager.new_sender::<Vec<_>>(DEFAULT_CHANNEL_SIZE);
    state_receiver
        .add_callback(
            DelayedBackendMutatingRequest("This message should get blocked!".to_string()),
            |state, new| state.push(new),
            None,
        )
        .await
        .unwrap();
    state_receiver
        .add_callback(
            DelayedBackendMutatingRequest("Message 2".to_string()),
            |state, new| state.push(new),
            Some(Constraint::new_block_same_type()),
        )
        .await
        .unwrap();
    manager.drain(backend.clone()).await;
    state_receiver.get_messages().await.apply(&mut state);
    let backend_counter = backend.lock().await.msgs_recvd;
    assert_eq!(state, vec!["Message 2".to_string()]);
    assert_eq!(backend_counter, 2)
}

#[tokio::test]
async fn test_kill_constraint() {
    let mut state = vec![];
    let backend = Arc::new(Mutex::new(MockBackend::default()));
    let mut manager = async_callback_manager::AsyncCallbackManager::new(DEFAULT_CHANNEL_SIZE);
    let mut state_receiver = manager.new_sender::<Vec<_>>(DEFAULT_CHANNEL_SIZE);
    state_receiver
        .add_callback(
            DelayedBackendMutatingRequest("This message should get killed!".to_string()),
            |state, new| state.push(new),
            None,
        )
        .await
        .unwrap();
    state_receiver
        .add_callback(
            DelayedBackendMutatingRequest("Message 2".to_string()),
            |state, new| state.push(new),
            Some(Constraint::new_kill_same_type()),
        )
        .await
        .unwrap();
    manager.drain(backend.clone()).await;
    state_receiver.get_messages().await.apply(&mut state);
    let backend_counter = backend.lock().await.msgs_recvd;
    assert_eq!(state, vec!["Message 2".to_string()]);
    assert_eq!(backend_counter, 1)
}
