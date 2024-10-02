//! Example to demonstrate a network stream interrupted by another network
//! stream.
use async_callback_manager::{
    AsyncCallbackManager, BackendStreamingTask, CallbackSender, Constraint,
};
use futures::StreamExt;
use reqwest::Client;
use std::{sync::Arc, time::Duration, vec::IntoIter};

#[derive(Clone)]
struct ArcClient(Arc<Client>);

// impl ArcClient {
//     fn mimic_progress_update_stream(&self) -> impl Stream<Item = usize> {
//         let (tx, rx) = mpsc::channel(50);
//         tokio::spawn(async {});
//     }
// }

struct State {
    text_1: String,
    text_2: String,
    sender: CallbackSender<ArcClient, Self>,
}

struct StreamUrl(&'static str, usize);

enum StreamUrlState {
    Init,
    Gen(IntoIter<String>),
}

impl BackendStreamingTask<ArcClient> for StreamUrl {
    type Output = String;
    fn into_stream(
        self,
        backend: ArcClient,
    ) -> impl futures::Stream<Item = Self::Output> + Send + Unpin {
        let url = self.0.to_owned();
        let id = self.1;
        futures::stream::unfold(StreamUrlState::Init, move |state| {
            let url = url.clone();
            let backend = backend.clone();
            async move {
                match state {
                    StreamUrlState::Init => {
                        let mut lines = backend
                            .0
                            .get(url)
                            .send()
                            .await
                            .unwrap()
                            .text()
                            .await
                            .unwrap()
                            .lines()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .into_iter();
                        Some((
                            lines.next().unwrap().chars().take(30).collect(),
                            StreamUrlState::Gen(lines),
                        ))
                    }
                    StreamUrlState::Gen(mut lines) => {
                        tokio::time::sleep(Duration::from_millis(150)).await;
                        Some((
                            lines.next().unwrap().chars().take(30).collect(),
                            StreamUrlState::Gen(lines),
                        ))
                    }
                }
            }
        })
        .inspect(move |s| println!("text {s}, id: {}", id))
        .boxed()
    }
}

#[tokio::main]
async fn main() {
    let mut mgr = AsyncCallbackManager::new(50);
    let mut state = State {
        text_1: String::new(),
        text_2: String::new(),
        sender: mgr.new_sender(50),
    };
    state
        .sender
        .add_stream_callback(
            StreamUrl("https://www.rust-lang.org", 1),
            |state, strng| state.text_1 = strng,
            None,
        )
        .await
        .unwrap();
    let bkend = ArcClient(Arc::new(reqwest::Client::new()));
    for i in 1..50 {
        mgr.process_messages(bkend.clone());
        if i == 20 {
            state
                .sender
                .add_stream_callback(
                    StreamUrl("https://www.google.com", 2),
                    |state, strng| state.text_2 = strng,
                    Some(Constraint::new_block_same_type()),
                )
                .await
                .unwrap();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
        for action in state.sender.get_messages().await {
            action(&mut state)
        }
        println!("str1: {}, str2: {}", state.text_1, state.text_2)
    }
}
