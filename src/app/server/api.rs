use super::spawn_run_or_kill;
use super::KillableTask;
use crate::app::taskmanager::TaskID;
use crate::config::ApiKey;
use crate::error::Error;
use crate::Result;
use tokio::sync::mpsc;
use tracing::{error, info};
use ytmapi_rs::auth::BrowserToken;
use ytmapi_rs::common::youtuberesult::YoutubeResult;
use ytmapi_rs::common::AlbumID;
use ytmapi_rs::common::SearchSuggestion;
use ytmapi_rs::common::YoutubeID;
use ytmapi_rs::parse::GetArtistAlbums;
use ytmapi_rs::parse::SongResult;
use ytmapi_rs::ChannelID;

pub enum Request {
    GetSearchSuggestions(String, KillableTask),
    NewArtistSearch(String, KillableTask),
    SearchSelectedArtist(ChannelID<'static>, KillableTask),
}
#[derive(Debug)]
pub enum Response {
    ReplaceArtistList(Vec<ytmapi_rs::parse::SearchResultArtist>, TaskID),
    SearchArtistError(TaskID),
    ReplaceSearchSuggestions(Vec<SearchSuggestion>, TaskID, String),
    SongListLoading(TaskID),
    SongListLoaded(TaskID),
    NoSongsFound(TaskID),
    SongsFound(TaskID),
    AppendSongList {
        song_list: Vec<SongResult>,
        album: String,
        year: String,
        artist: String,
        id: TaskID,
    },
    ApiError(Error),
}
pub struct Api {
    // Do I want to keep track of tasks here in a joinhandle?
    api: Option<ytmapi_rs::YtMusic<BrowserToken>>,
    api_init: Option<tokio::task::JoinHandle<Result<ytmapi_rs::YtMusic<BrowserToken>>>>,
    response_tx: mpsc::Sender<super::Response>,
}

impl Api {
    pub fn new(api_key: ApiKey, response_tx: mpsc::Sender<super::Response>) -> Self {
        let api_init = Some(tokio::spawn(async move {
            info!("Initialising API");
            // TODO: Error handling
            let api = match api_key {
                ApiKey::BrowserToken(c) => ytmapi_rs::YtMusic::from_cookie(c).await?,
                ApiKey::OAuthToken(t) =>
                // TODO: Add OAuth
                {
                    unimplemented!()
                }
                // ytmapi_rs::YtMusic::from_oauth_token(t),
            };
            info!("API initialised");
            Ok(api)
        }));
        Self {
            api: None,
            api_init,
            response_tx,
        }
    }
    async fn get_api(&mut self) -> Result<&ytmapi_rs::YtMusic<BrowserToken>> {
        // NOTE: This function returns a different type of error if not called before, due to difficulties
        // I'm having in saving Result<T,E> but returning Result<&T, E>.
        if let Some(handle) = self.api_init.take() {
            let api = handle.await??;
            self.api = Some(api);
        }
        if let Some(api) = self.api.as_ref() {
            Ok(api)
        } else {
            // Rough guard against the case of sending an unkown api error.
            // TODO: Better handling for this edge case.
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            Err(Error::UnknownAPIError)
        }
    }
    pub async fn handle_request(&mut self, request: Request) -> Result<()> {
        match request {
            Request::NewArtistSearch(a, task) => self.handle_new_artist_search(a, task).await,
            Request::GetSearchSuggestions(text, task) => {
                self.handle_get_search_suggestions(text, task).await
            }
            Request::SearchSelectedArtist(browse_id, task) => {
                self.handle_search_selected_artist(browse_id, task).await
            }
        }
    }
    async fn handle_get_search_suggestions(
        &mut self,
        text: String,
        task: KillableTask,
    ) -> Result<()> {
        let KillableTask { id, kill_rx } = task;
        // Give the task a clone of the API. Not ideal but works.
        // The largest part of the API is Reqwest::Client which contains an Arc
        // internally and so I believe clones efficiently.
        // Possible alternative: https://stackoverflow.com/questions/51044467/how-can-i-perform-parallel-asynchronous-http-get-requests-with-reqwest
        // Create a stream of tasks, map with a reference to API.
        let tx = self.response_tx.clone();
        let api = match self.get_api().await {
            Ok(api) => api,
            Err(e) => {
                error!("Error {e} connecting to API");
                tx.send(crate::app::server::Response::Api(Response::ApiError(e)))
                    .await?;
                // Rough guard against the case of sending an unkown api error.
                // TODO: Better handling for this edge case.
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                return Err(Error::UnknownAPIError);
            }
        }
        .clone();
        let _ = spawn_run_or_kill(
            async move {
                tracing::info!("Getting search suggestions for {text}");
                let search_suggestions = match api.get_search_suggestions(&text).await {
                    Ok(t) => t,
                    Err(e) => {
                        error!("Received error on search suggestions query \"{}\"", e);
                        return;
                    }
                };
                tracing::info!("Requesting caller to replace search suggestions");
                let _ = tx
                    .send(super::Response::Api(Response::ReplaceSearchSuggestions(
                        search_suggestions,
                        id,
                        text,
                    )))
                    .await;
            },
            kill_rx,
        )
        .await;
        Ok(())
    }

    async fn handle_new_artist_search(&mut self, artist: String, task: KillableTask) -> Result<()> {
        let KillableTask { id, kill_rx } = task;
        // Give the task a clone of the API. Not ideal but works.
        // The largest part of the API is Reqwest::Client which contains an Arc
        // internally and so I believe clones efficiently.
        // Possible alternative: https://stackoverflow.com/questions/51044467/how-can-i-perform-parallel-asynchronous-http-get-requests-with-reqwest
        // Create a stream of tasks, map with a reference to API.
        let tx = self.response_tx.clone();
        let api = match self.get_api().await {
            Ok(api) => api,
            Err(e) => {
                error!("Error {e} connecting to API");
                tx.send(crate::app::server::Response::Api(Response::ApiError(e)))
                    .await?;
                // Rough guard against the case of sending an unkown api error.
                // TODO: Better handling for this edge case.
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                return Err(Error::UnknownAPIError);
            }
        }
        .clone();
        let _ = spawn_run_or_kill(
            async move {
                //            let api = crate::app::api::APIHandler::new();
                //            let search_res = api.search_artists(&self.search_contents, 20);
                tracing::info!("Running search query");
                let search_res = match api
                    .search(
                        ytmapi_rs::query::SearchQuery::new(artist)
                            .with_filter(ytmapi_rs::query::Filter::Artists)
                            .with_spelling_mode(ytmapi_rs::query::SpellingMode::ExactMatch),
                    )
                    .await
                {
                    Ok(t) => t,
                    Err(e) => {
                        error!("Received error on search artist query \"{}\"", e);
                        tx.send(super::Response::Api(Response::SearchArtistError(id)))
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
                let _ = tx
                    .send(super::Response::Api(Response::ReplaceArtistList(
                        artist_list,
                        id,
                    )))
                    .await;
            },
            kill_rx,
        )
        .await;
        Ok(())
    }
    async fn handle_search_selected_artist(
        &mut self,
        browse_id: ChannelID<'static>,
        task: KillableTask,
    ) -> Result<()> {
        let KillableTask { id, kill_rx } = task;
        // See above note
        let tx = self.response_tx.clone();
        let api = match self.get_api().await {
            Ok(api) => api,
            Err(e) => {
                error!("Error {e} connecting to API");
                tx.send(crate::app::server::Response::Api(Response::ApiError(e)))
                    .await?;
                // Rough guard against the case of sending an unkown api error.
                // TODO: Better handling for this edge case.
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                return Err(Error::UnknownAPIError);
            }
        }
        .clone();
        let _ = spawn_run_or_kill(
            async move {
                let tx = tx.clone();
                let _ = tx
                    .send(super::Response::Api(Response::SongListLoading(id)))
                    .await;
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
                        let _ = tx
                            .send(super::Response::Api(Response::NoSongsFound(id)))
                            .await;
                        return;
                    }
                };
                let Some(albums) = artist.top_releases.albums else {
                    tracing::info!("Telling caller no songs found (no params)");
                    let _ = tx
                        .send(super::Response::Api(Response::NoSongsFound(id)))
                        .await;
                    return;
                };

                let GetArtistAlbums {
                    browse_id: artist_albums_browse_id,
                    params: artist_albums_params,
                    results: artist_albums_results,
                } = albums;
                let browse_id_list = if artist_albums_browse_id.is_none()
                    && artist_albums_params.is_none()
                    && !artist_albums_results.is_empty()
                {
                    // Assume we already got all the albums from the search.
                    let browse_id_list: Option<Vec<_>> = artist_albums_results
                        .iter()
                        .map(|r| {
                            r.get_channel_id()
                                .as_ref()
                                .map(|c_id| AlbumID::from_raw(c_id.get_raw()))
                        })
                        .collect();
                    if let Some(browse_id_list) = browse_id_list {
                        browse_id_list
                    } else {
                        tracing::info!(
                            "Telling caller no songs found (some albums missing browse id)"
                        );
                        let _ = tx
                            .send(super::Response::Api(Response::NoSongsFound(id)))
                            .await;
                        return;
                    }
                } else if artist_albums_params.is_none() || artist_albums_browse_id.is_none() {
                    tracing::info!("Telling caller no songs found (no params or browse_id)");
                    let _ = tx
                        .send(super::Response::Api(Response::NoSongsFound(id)))
                        .await;
                    return;
                } else {
                    // Must have params and browse_id
                    let Some(temp_browse_id) = artist_albums_browse_id else {
                        unreachable!("Checked not none above")
                    };
                    let Some(temp_params) = artist_albums_params else {
                        unreachable!("Checked not none above")
                    };

                    let albums = match api
                        .get_artist_albums(ytmapi_rs::query::GetArtistAlbumsQuery::new(
                            ytmapi_rs::ChannelID::from_raw(temp_browse_id.get_raw()),
                            temp_params,
                        ))
                        .await
                    {
                        Ok(r) => r,
                        Err(e) => {
                            error!("Received error on get_artist_albums query \"{}\"", e);

                            // TODO: Better Error type
                            tx.send(super::Response::Api(Response::SearchArtistError(id)))
                                .await
                                .unwrap_or_else(|_| error!("Error sending response"));
                            return;
                        }
                    };
                    albums.into_iter().map(|a| a.browse_id).collect()
                };
                let _ = tx
                    .send(super::Response::Api(Response::SongsFound(id)))
                    .await;
                // Concurrently request all albums.
                let futures = browse_id_list.into_iter().map(|b_id| {
                    let api = &api;
                    let tx = tx.clone();
                    // TODO: remove allocation
                    let artist_name = artist.name.clone();
                    async move {
                        tracing::info!(
                            "Spawning request for caller tracks for request ID {:?}",
                            id
                        );
                        let album = match api
                            .get_album(ytmapi_rs::query::GetAlbumQuery::new(&b_id))
                            .await
                        {
                            Ok(album) => album,
                            Err(e) => {
                                error!("Error <{e}> getting album {:?}", b_id);
                                return;
                            }
                        };
                        tracing::info!("Sending caller tracks for request ID {:?}", id);
                        let _ = tx
                            .send(super::Response::Api(Response::AppendSongList {
                                song_list: album.tracks,
                                album: album.title,
                                year: album.year,
                                artist: artist_name,
                                id,
                            }))
                            .await;
                    }
                });
                let _ = futures::future::join_all(futures).await;
                let _ = tx
                    .send(super::Response::Api(Response::SongListLoaded(id)))
                    .await;
            },
            kill_rx,
        )
        .await;
        Ok(())
    }
}
