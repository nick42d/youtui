use super::spawn_run_or_kill;
use super::KillableTask;
use crate::app::taskmanager::TaskID;
use crate::config::ApiKey;
use crate::error::Error;
use crate::Result;
use futures::FutureExt;
use futures::TryFutureExt;
use std::future::Future;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Notify;
use tokio::sync::OnceCell;
use tokio::sync::RwLock;
use tracing::{error, info};
use ytmapi_rs::auth::BrowserToken;
use ytmapi_rs::auth::OAuthToken;
use ytmapi_rs::common::AlbumID;
use ytmapi_rs::common::SearchSuggestion;
use ytmapi_rs::error::ErrorKind;
use ytmapi_rs::parse::AlbumSong;
use ytmapi_rs::parse::GetArtistAlbums;
use ytmapi_rs::query::GetAlbumQuery;
use ytmapi_rs::query::GetArtistAlbumsQuery;
use ytmapi_rs::ChannelID;
use ytmapi_rs::YtMusic;
use ytmapi_rs::YtMusicBuilder;

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
        song_list: Vec<AlbumSong>,
        album: String,
        year: String,
        artist: String,
        id: TaskID,
    },
    ApiError(Error),
}
pub struct Api {
    // Do I want to keep track of tasks here in a joinhandle?
    api: Arc<OnceCell<Result<DynamicApi>>>,
    notify: Arc<Notify>,
    _api_init: tokio::task::JoinHandle<()>,
    response_tx: mpsc::Sender<super::Response>,
}
#[derive(Clone, Debug)]
pub enum DynamicApi {
    // Arc is there to allow clone. Could potentially be removed if Clone can be removed.
    OAuth(Arc<RwLock<YtMusic<OAuthToken>>>),
    Browser(YtMusic<BrowserToken>),
}
impl DynamicApi {
    pub async fn new_from_cookie(cookie: String) -> Result<Self> {
        Ok(DynamicApi::Browser(
            YtMusicBuilder::new_rustls_tls()
                .with_browser_token_cookie(cookie)
                .build()
                .await?,
        ))
    }
    pub fn new_from_oauth_token(token: OAuthToken) -> Result<Self> {
        Ok(DynamicApi::OAuth(Arc::new(RwLock::new(
            YtMusicBuilder::new_rustls_tls()
                .with_oauth_token(token)
                .build()?,
        ))))
    }
    /// Run a query. If the oauth token is expired, take the lock and refresh
    /// it (single retry only).
    // NOTE: Determine how to handle if multiple queries in progress when we lock.
    // TODO: Refresh the oauth file also.
    pub async fn query<Q, O>(&self, query: Q) -> ytmapi_rs::Result<O>
    where
        Q: ytmapi_rs::query::Query<BrowserToken, Output = O>,
        Q: ytmapi_rs::query::Query<OAuthToken, Output = O>,
        Q: Clone,
    {
        match self {
            DynamicApi::Browser(yt) => Ok(yt.query(query).await?),
            DynamicApi::OAuth(yt) => {
                // TODO: Remove clone. It's required currently to allow retry of the query.
                let result = yt.read().await.query(query.clone()).await;
                match result {
                    Ok(r) => Ok(r),
                    Err(e) => match e.into_kind() {
                        ErrorKind::OAuthTokenExpired { token_hash } => {
                            // First check to see if the token_hash hasn't changed since calling the
                            // query. If it has, that means another query must have already
                            // refreshed the token.
                            if yt.read().await.get_token_hash() == token_hash {
                                info!("Refreshing oauth token");
                                yt.write().await.refresh_token().await?;
                            }
                            Ok(yt.read().await.query(query).await?)
                        }
                        other => Err(other.into()),
                    },
                }
            }
        }
    }
}

impl Api {
    pub fn new(api_key: ApiKey, response_tx: mpsc::Sender<super::Response>) -> Self {
        let api = Arc::new(OnceCell::new());
        let notify = Arc::new(Notify::new());
        let api_clone = api.clone();
        let notify_clone = notify.clone();
        let _api_init = tokio::spawn(async move {
            info!("Initialising API");
            // TODO: Error handling
            let api_gen = match api_key {
                ApiKey::BrowserToken(c) => DynamicApi::new_from_cookie(c).await,
                ApiKey::OAuthToken(t) => DynamicApi::new_from_oauth_token(t),
            }
            .map_err(Into::into);
            api_clone
                .set(api_gen)
                .expect("First time initializing api should always succeed");
            notify_clone.notify_one();
            info!("API initialised");
        });
        Self {
            api,
            response_tx,
            notify,
            _api_init,
        }
    }
    async fn get_api(&self) -> Result<&DynamicApi> {
        // Consider returning an error, instead of looping.
        loop {
            match self.api.get() {
                // Wait for initialisation to complete if it hasn't already.
                None => self.notify.notified().await,
                // TODO: Better error - hack to turn &E into E
                Some(api) => return api.as_ref().map_err(|_| Error::UnknownAPIError),
            }
        }
    }
    pub async fn handle_request(&self, request: Request) -> Result<()> {
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
    async fn handle_get_search_suggestions(&self, text: String, task: KillableTask) -> Result<()> {
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
                return Ok(());
            }
        }
        .clone();
        let _ = spawn_run_or_kill(
            async move {
                tracing::info!("Getting search suggestions for {text}");
                let query = ytmapi_rs::query::GetSearchSuggestionsQuery::new(&text);
                let search_suggestions = match api.query(query).await {
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

    async fn handle_new_artist_search(&self, artist: String, task: KillableTask) -> Result<()> {
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
                return Ok(());
            }
        }
        .clone();
        let _ = spawn_run_or_kill(
            async move {
                //            let api = crate::app::api::APIHandler::new();
                //            let search_res = api.search_artists(&self.search_contents, 20);
                tracing::info!("Running search query");
                let search_res = match api
                    .query(
                        ytmapi_rs::query::SearchQuery::new(artist)
                            .with_filter(ytmapi_rs::query::ArtistsFilter)
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
                let artist_list = search_res.into_iter().collect();
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
        &self,
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
                return Ok(());
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
                // Should this actually take ChannelID::try_from(BrowseID::Artist) ->
                // ChannelID::Artist?
                let query = ytmapi_rs::query::GetArtistQuery::new(browse_id);
                let artist = api.query(query).await;
                let artist = match artist {
                    Ok(a) => a,
                    Err(e) => {
                        let Some((json, key)) = e.get_json_and_key() else {
                            error!("API error received <{e}>");
                            info!("Telling caller no songs found (error)");
                            let _ = tx
                                .send(super::Response::Api(Response::NoSongsFound(id)))
                                .await;
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
                let browse_id_list: Vec<AlbumID> = if artist_albums_browse_id.is_none()
                    && artist_albums_params.is_none()
                    && !artist_albums_results.is_empty()
                {
                    // Assume we already got all the albums from the search.
                    artist_albums_results
                        .into_iter()
                        .map(|r| r.album_id)
                        .collect()
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
                    let query = GetArtistAlbumsQuery::new(temp_browse_id, temp_params);
                    let albums = match api.query(query).await {
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
                        let query = GetAlbumQuery::new(&b_id);
                        let album = match api.query(query).await {
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
