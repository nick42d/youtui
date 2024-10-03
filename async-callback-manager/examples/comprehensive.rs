use async_callback_manager::{
    AsyncCallbackManager, BackendStreamingTask, BackendTask, CallbackSender, Constraint, Result,
};
use futures::{Future, Stream, StreamExt};
use std::{sync::Arc, time::Duration};
use tokio_stream::wrappers::ReceiverStream;

struct Controller {
    state: Root,
    server: Arc<Requester>,
    runner: AsyncCallbackManager<Arc<Requester>>,
}

struct Root {
    pl: Playlist,
    b: Browser,
}

struct Browser {
    cur_text: String,
    callback_handler: CallbackSender<Arc<Requester>, Self>,
}

struct Playlist {
    cur_song: String,
    cur_song_num: usize,
    cur_vol: usize,
    callback_handler: CallbackSender<Arc<Requester>, Self>,
}

#[tokio::main]
async fn main() {
    let mut runner = AsyncCallbackManager::new(50);
    let server = Arc::new(Requester::new());
    let state = Root::new(&mut runner);
    let mut controller = Controller {
        state,
        server,
        runner,
    };
    // r.pl.playback_updates().await;
    controller.state.pl.update_cur_song().await;
    controller.state.pl.stream_song_nums().await;
    controller.state.b.update_cur_text().await;
    for i in 1..100 {
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        if i < 30 {
            controller.state.pl.update_cur_vol(i, 50).await.unwrap();
        }
        if i == 40 {
            controller
                .state
                .pl
                .callback_handler
                .add_stream_callback(
                    StreamNumsRequest {
                        max_nums: 2000,
                        step: 10,
                        sleep_ms: 3,
                    },
                    |pl, num| pl.cur_song_num = num,
                    Some(Constraint::new_block_same_type()),
                )
                .await
                .unwrap();
        }
        println!("Iter {i}");
        println!(
            "..pl.cur_song: {}, ..pl.cur_song_num: {}, ..state.pl.cur_vol: {}",
            controller.state.pl.cur_song,
            controller.state.pl.cur_song_num,
            controller.state.pl.cur_vol,
        );
        println!("..b.cur_text: {}", controller.state.b.cur_text.trim_start());
        controller
            .runner
            .process_messages(controller.server.clone());
        controller.state.try_handle().await;
        println!("-------------------------------------------------------------");
    }
}

impl Root {
    fn new(runner: &mut AsyncCallbackManager<Arc<Requester>>) -> Self {
        let pl = Playlist::new(runner);
        let b = Browser::new(runner);
        Root { pl, b }
    }
    async fn try_handle(&mut self) {
        self.pl.try_handle().await;
        self.b.try_handle().await;
    }
}

impl Browser {
    async fn try_handle(&mut self) {
        self.callback_handler.get_messages().await.apply(self);
    }
    fn new(runner: &mut AsyncCallbackManager<Arc<Requester>>) -> Self {
        let callback_handler = runner.new_sender(50);
        Self {
            cur_text: String::new(),
            callback_handler,
        }
    }
    async fn update_cur_text(&mut self) {
        self.callback_handler
            .add_callback(
                BasicRequest(3),
                |browser, string| browser.cur_text = string,
                None,
            )
            .await
            .unwrap();
    }
}

impl Playlist {
    async fn try_handle(&mut self) {
        self.callback_handler.get_messages().await.apply(self);
    }
    fn new(runner: &mut AsyncCallbackManager<Arc<Requester>>) -> Self {
        let callback_handler = runner.new_sender(50);
        Self {
            cur_song: String::new(),
            cur_song_num: 0,
            cur_vol: 0,
            callback_handler,
        }
    }
    async fn update_cur_vol(&mut self, num: usize, delay_ms: u64) -> Result<()> {
        self.callback_handler
            .add_callback(
                ReturnNumAfterDelay { num, delay_ms },
                |pl, num| pl.cur_vol = num,
                Some(Constraint::new_block_same_type()),
            )
            .await
    }
    async fn update_cur_song(&mut self) {
        self.callback_handler
            .add_callback(BasicRequest(1), |pl, string| pl.cur_song = string, None)
            .await
            .unwrap();
    }
    async fn stream_song_nums(&mut self) {
        self.callback_handler
            .add_stream_callback(
                StreamNumsRequest {
                    max_nums: 1000,
                    sleep_ms: 3,
                    step: 1,
                },
                |pl, num| pl.cur_song_num = num,
                None,
            )
            .await
            .unwrap();
    }
}

pub struct BasicRequest(pub usize);

pub struct ReturnNumAfterDelay {
    pub num: usize,
    pub delay_ms: u64,
}

pub struct StreamNumsRequest {
    pub max_nums: usize,
    pub step: usize,
    pub sleep_ms: usize,
}

impl BackendStreamingTask<Arc<Requester>> for StreamNumsRequest {
    type Output = usize;
    fn into_stream(
        self,
        _server: Arc<Requester>,
    ) -> impl Stream<Item = Self::Output> + Send + Unpin {
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        tokio::task::spawn(async move {
            let mut i = 0;
            while i < self.max_nums {
                i += self.step;
                tokio::time::sleep(std::time::Duration::from_millis(self.sleep_ms as u64)).await;
                tx.send(i).await.unwrap();
            }
        });
        ReceiverStream::new(rx).boxed()
    }
}

impl BackendTask<Arc<Requester>> for ReturnNumAfterDelay {
    type Output = usize;
    async fn into_future(self, _: Arc<Requester>) -> usize {
        tokio::time::sleep(Duration::from_millis(self.delay_ms)).await;
        self.num
    }
}

impl BackendTask<Arc<Requester>> for BasicRequest {
    type Output = String;
    fn into_future(self, server: Arc<Requester>) -> impl Future<Output = Self::Output> + Send {
        server.basic_request(self.0)
    }
}

pub struct Requester {
    pub client: reqwest::Client,
}

impl Requester {
    pub fn new() -> Self {
        Requester {
            client: reqwest::Client::new(),
        }
    }
    pub async fn basic_request(self: Arc<Self>, n: usize) -> String {
        self.client
            .get("https://www.rust-lang.org")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
            .lines()
            .nth(n)
            .unwrap()
            .to_owned()
    }
}

impl Default for Requester {
    fn default() -> Self {
        Self::new()
    }
}
