use rusty_ytdl::DownloadOptions;
use rusty_ytdl::Video;
use rusty_ytdl::VideoOptions;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
mod structures;
use anyhow::Result;
use std::path::Path;
use std::path::PathBuf;
use tracing::{error, info};
use ytmapi_rs::common::AlbumID;
use ytmapi_rs::common::YoutubeID;
use ytmapi_rs::parse::GetArtistAlbums;
use ytmapi_rs::parse::SongResult;

use ytmapi_rs::ChannelID;
use ytmapi_rs::VideoID;

use crate::core::send_or_error;

use super::ui::structures::ListSongID;
use super::ui::taskregister::TaskID;

const TEMP_MUSIC_DIR: &str = "./music";

pub struct KillRequest;

pub struct KillableTask {
    pub id: TaskID,
    pub kill_rx: oneshot::Receiver<KillRequest>,
}

pub enum Request {
    // TaskID, KillRequest is starting to look like a pattern.
    GetSearchSuggestions(String, KillableTask),
    NewArtistSearch(String, TaskID, oneshot::Receiver<KillRequest>),
    SearchSelectedArtist(ChannelID<'static>, TaskID, oneshot::Receiver<KillRequest>),
    DownloadSong(
        VideoID<'static>,
        ListSongID,
        TaskID,
        oneshot::Receiver<KillRequest>,
    ),
}
pub enum Response {
    ReplaceArtistList(Vec<ytmapi_rs::parse::SearchResultArtist>, TaskID),
    SearchArtistError(TaskID),
    ReplaceSearchSuggestions(Vec<String>, TaskID),
    SongListLoading(TaskID),
    SongListLoaded(TaskID),
    NoSongsFound(TaskID),
    SongsFound(TaskID),
    AppendSongList(Vec<SongResult>, String, String, TaskID),
    SongProgressUpdate(SongProgressUpdateType, ListSongID, TaskID),
}

#[derive(Debug)]
pub enum SongProgressUpdateType {
    Started,
    Downloading(u8), // Percentage as integer
    Completed(PathBuf),
    Error,
}

pub struct Server {
    // Do I want to keep track of tasks here in a joinhandle?
    api: Option<ytmapi_rs::YtMusic>,
    api_init: Option<tokio::task::JoinHandle<ytmapi_rs::YtMusic>>,
    response_tx: mpsc::Sender<Response>,
    request_rx: mpsc::Receiver<Request>,
}

impl Server {
    pub fn new(response_tx: mpsc::Sender<Response>, request_rx: mpsc::Receiver<Request>) -> Self {
        let api_init = Some(tokio::spawn(async move {
            info!("Initialising API");
            let api = ytmapi_rs::YtMusic::from_header_file(std::path::Path::new("headers.txt"))
                .await
                .unwrap();
            info!("API initialised");
            api
        }));
        Self {
            api: None,
            api_init,
            request_rx,
            response_tx,
        }
    }
    async fn get_api(&mut self) -> Result<&ytmapi_rs::YtMusic> {
        if self.api_init.is_some() {
            let handle = self.api_init.take();
            let api = handle.unwrap().await?;
            self.api = Some(api);
        }
        return Ok(self
            .api
            .as_ref()
            .expect("Should have put the API into the option above"));
    }
    pub async fn run(&mut self) {
        // Could be a while let
        loop {
            match self.request_rx.recv().await {
                Some(Request::DownloadSong(video_id, playlist_id, id, kill)) => {
                    self.handle_download_song(video_id, playlist_id, id, kill)
                        .await
                }
                Some(Request::NewArtistSearch(a, id, kill)) => {
                    self.handle_new_artist_search(a, id, kill).await
                }
                Some(Request::GetSearchSuggestions(text, task)) => {
                    self.handle_get_search_suggestions(text, task).await
                }
                Some(Request::SearchSelectedArtist(browse_id, id, kill)) => {
                    self.handle_search_selected_artist(browse_id, id, kill)
                        .await
                }
                None => (),
            }
        }
    }
    async fn handle_download_song(
        &mut self,
        song_video_id: VideoID<'static>,
        playlist_id: ListSongID,
        id: TaskID,
        kill: oneshot::Receiver<KillRequest>,
    ) {
        let tx = self.response_tx.clone();
        let _ = spawn_run_or_kill(
            async move {
                tracing::info!("Running download");
                send_or_error(
                    &tx,
                    Response::SongProgressUpdate(SongProgressUpdateType::Started, playlist_id, id),
                )
                .await;
                let options = VideoOptions {
                    quality: rusty_ytdl::VideoQuality::LowestAudio,
                    filter: rusty_ytdl::VideoSearchOptions::Audio,
                    // Options for changing chunk size.
                    // download_options: DownloadOptions {
                    //     dl_chunk_size: Some(2),
                    // },
                    ..Default::default()
                };
                let Ok(video) = Video::new_with_options(song_video_id.get_raw(), options) else {
                    error!("Error received finding song");
                    return;
                };
                let path_string = format!("music/{}.mp4", song_video_id.get_raw());
                let path = Path::new(&path_string);
                // Test of in-memory download with callback.
                // Works correctly.
                // let stream = video.clone().stream().await.unwrap();
                // let mut i = 0;
                // let mut chunks = Vec::new();
                // while let Some(mut chunk) = stream.chunk().await.unwrap() {
                //     i += 1;
                //     info!("got chunk {i}");
                //     chunks.append(&mut chunk)
                // }
                // info!("total chunks length{}", chunks.len());
                match video.download(path).await {
                    Ok(_) => {
                        send_or_error(
                            &tx,
                            Response::SongProgressUpdate(
                                SongProgressUpdateType::Completed(path.into()),
                                playlist_id,
                                id,
                            ),
                        )
                        .await
                    }
                    Err(_) => {
                        send_or_error(
                            &tx,
                            Response::SongProgressUpdate(
                                SongProgressUpdateType::Error,
                                playlist_id,
                                id,
                            ),
                        )
                        .await
                    }
                };
            },
            kill,
        )
        .await;
    }
    async fn handle_get_search_suggestions(&mut self, text: String, task: KillableTask) {
        let KillableTask { id, kill_rx } = task;
        // Give the task a clone of the API. Not ideal but works.
        // The largest part of the API is Reqwest::Client which contains an Arc
        // internally and so I believe clones efficiently.
        // Possible alternative: https://stackoverflow.com/questions/51044467/how-can-i-perform-parallel-asynchronous-http-get-requests-with-reqwest
        // Create a stream of tasks, map with a reference to API.
        let api = self.get_api().await.unwrap().clone();
        let tx = self.response_tx.clone();
        let _ = spawn_run_or_kill(
            async move {
                tracing::info!("Getting search suggestions for {text}");
                let search_suggestions = match api.get_search_suggestions(text).await {
                    Ok(t) => t,
                    Err(e) => {
                        error!("Received error on search suggestions query \"{}\"", e);
                        return;
                    }
                };
                tracing::info!("Requesting caller to replace search suggestions");
                let _ = tx
                    .send(Response::ReplaceSearchSuggestions(search_suggestions, id))
                    .await;
            },
            kill_rx,
        )
        .await;
    }
    async fn handle_new_artist_search(
        &mut self,
        artist: String,
        id: TaskID,
        kill: oneshot::Receiver<KillRequest>,
    ) {
        // Give the task a clone of the API. Not ideal but works.
        // The largest part of the API is Reqwest::Client which contains an Arc
        // internally and so I believe clones efficiently.
        // Possible alternative: https://stackoverflow.com/questions/51044467/how-can-i-perform-parallel-asynchronous-http-get-requests-with-reqwest
        // Create a stream of tasks, map with a reference to API.
        let api = self.get_api().await.unwrap().clone();
        let tx = self.response_tx.clone();
        let _ = spawn_run_or_kill(
            async move {
                //            let api = crate::app::api::APIHandler::new();
                //            let search_res = api.search_artists(&self.search_contents, 20);
                tracing::info!("Running search query");
                let search_res = match api
                    .search(
                        ytmapi_rs::query::SearchQuery::new(artist)
                            .set_filter(ytmapi_rs::query::Filter::Artists)
                            .set_spelling_mode(ytmapi_rs::query::SpellingMode::ExactMatch),
                    )
                    .await
                {
                    Ok(t) => t,
                    Err(e) => {
                        error!("Received error on search artist query \"{}\"", e);
                        tx.send(Response::SearchArtistError(id))
                            .await
                            .unwrap_or_else(|_| error!("Error sending response"));
                        return;
                    }
                };
                let artist_list = search_res
                    .into_iter()
                    .map(|r| match r {
                        ytmapi_rs::parse::SearchResult::Artist(a) => a,
                        _ => unimplemented!(),
                    })
                    .collect();
                tracing::info!("Requesting caller to replace artist list");
                let _ = tx.send(Response::ReplaceArtistList(artist_list, id)).await;
            },
            kill,
        )
        .await;
    }
    async fn handle_search_selected_artist(
        &mut self,
        browse_id: ChannelID<'static>,
        id: TaskID,
        kill: oneshot::Receiver<KillRequest>,
    ) {
        // See above note
        let api = self.get_api().await.unwrap().clone();
        let tx = self.response_tx.clone();
        let _ = spawn_run_or_kill(
            async move {
                let tx = tx.clone();
                let _ = tx.send(Response::SongListLoading(id)).await;
                tracing::info!("Running songs query");
                // Should this be a ChannelID or BrowseID? Should take a trait?.
                // Should this actually take ChannelID::try_from(BrowseID::Artist) -> ChannelID::Artist?
                let artist = api
                    .get_artist(ytmapi_rs::query::GetArtistQuery::new(
                        ytmapi_rs::ChannelID::from_raw(browse_id.get_raw()),
                    ))
                    .await;
                let artist = match artist {
                    Ok(a) => a,
                    Err(e) => {
                        let Some((json, key)) = e.get_json_and_key() else {
                            return;
                        };
                        // TODO: Bring loggable json errors into their own function.
                        error!("API error recieved at key {:?}", key);
                        let path = std::path::Path::new("test.json");
                        std::fs::write(path, json)
                            .unwrap_or_else(|e| error!("Error <{e}> writing json log"));
                        info!("Wrote json to {:?}", path);
                        tracing::info!("Telling caller no songs found (error)");
                        let _ = tx.send(Response::NoSongsFound(id)).await;
                        return;
                    }
                };
                let Some(albums) = artist.top_releases.albums else {
                    tracing::info!("Telling caller no songs found (params)");
                    let _ = tx.send(Response::NoSongsFound(id)).await;
                    return;
                };
                let GetArtistAlbums {
                    browse_id: Some(browse_id),
                    params: Some(params),
                    ..
                } = albums
                else {
                    tracing::info!("Telling caller no songs found (params)");
                    let _ = tx.send(Response::NoSongsFound(id)).await;
                    return;
                };
                let albums = match api
                    .get_artist_albums(ytmapi_rs::query::GetArtistAlbumsQuery::new(
                        ytmapi_rs::ChannelID::from_raw(browse_id.get_raw()),
                        params,
                    ))
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        error!("Received error on get_artist_albums query \"{}\"", e);

                        // TODO: Better Error type
                        tx.send(Response::SearchArtistError(id))
                            .await
                            .unwrap_or_else(|_| error!("Error sending response"));
                        return;
                    }
                };
                let _ = tx.send(Response::SongsFound(id)).await;
                // Concurrently request all albums.
                let mut browse_id_list = Vec::new();
                for album in albums {
                    // XXX: This is a hack to return the album with the resuls, could be a better way to do this.
                    browse_id_list.push((album.browse_id, album.title));
                }
                let futures = browse_id_list.into_iter().map(|b_id| {
                    let api = &api;
                    let tx = tx.clone();
                    async move {
                        tracing::info!(
                            "Spawning request for caller tracks for request ID {:?}",
                            id
                        );
                        let album = match api
                            .get_album(ytmapi_rs::query::GetAlbumQuery::new(AlbumID::from_raw(
                                &b_id.0,
                            )))
                            .await
                        {
                            Ok(album) => album,
                            Err(e) => {
                                error!("Error getting album {} {} :{e}", b_id.1, b_id.0);
                                return;
                            }
                        };
                        tracing::info!("Sending caller tracks for request ID {:?}", id);
                        let _ = tx
                            .send(Response::AppendSongList(
                                album.tracks,
                                b_id.1,
                                album.year, // alternative way to get album information.
                                id,
                            ))
                            .await;
                    }
                });
                let _ = futures::future::join_all(futures).await;
                let _ = tx.send(Response::SongListLoaded(id)).await;
            },
            kill,
        )
        .await;
    }
}
// Consider using this instead of macro above.
async fn run_or_kill(
    future: impl futures::Future<Output = ()>,
    kill_rx: oneshot::Receiver<KillRequest>,
) {
    tokio::select! {
        _ = future => (),
        _ = kill_rx => info!("Task killed by caller"), // Is there a better way to do this?
    }
}

async fn spawn_run_or_kill(
    future: impl futures::Future<Output = ()> + Send + 'static,
    kill_rx: oneshot::Receiver<KillRequest>,
) {
    tokio::spawn(run_or_kill(future, kill_rx));
}
