use crate::api::{DynamicApiError, DynamicYtMusic};
use crate::app::CALLBACK_CHANNEL_SIZE;
use crate::async_rodio_sink::send_or_error;
use crate::config::ApiKey;
use crate::{OAUTH_FILENAME, get_config_dir};
use anyhow::{Error, Result};
use async_callback_manager::PanickingReceiverStream;
use async_cell::sync::AsyncCell;
use futures::stream::FuturesOrdered;
use futures::{Stream, StreamExt};
use rusty_ytdl::reqwest;
use std::borrow::Borrow;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tracing::{error, info};
use ytmapi_rs::auth::{BrowserToken, OAuthToken};
use ytmapi_rs::common::{
    AlbumID, ArtistChannelID, PlaylistID, SearchSuggestion, Thumbnail, VideoID,
};
use ytmapi_rs::parse::{
    AlbumSong, GetAlbum, GetArtistAlbums, ParsedSongAlbum, ParsedSongArtist, PlaylistItem,
    SearchResultArtist, SearchResultPlaylist, SearchResultSong, WatchPlaylistTrack,
};
use ytmapi_rs::query::{GetAlbumQuery, GetArtistAlbumsQuery, GetWatchPlaylistQuery};

#[derive(Clone)]
/// # Note
/// Since the underlying API is wrapped in an Arc, it's cheap to clone this
/// type.
pub struct Api {
    api: Arc<AsyncCell<Result<ConcurrentApi, DynamicApiError>>>,
}
pub type ConcurrentApi = Arc<RwLock<DynamicYtMusic>>;

impl Api {
    pub fn new(api_key: ApiKey, client: reqwest::Client) -> Api {
        let api = AsyncCell::new().into_shared();
        let api_clone = api.clone();
        tokio::spawn(async move {
            let api = DynamicYtMusic::new(api_key, client)
                .await
                .map(|api| Arc::new(RwLock::new(api)));
            api_clone.set(api)
        });
        Api { api }
    }
    // NOTE: Situation where user has tried to create API from an expired OAuth
    // token is not currently handled.
    pub async fn get_api(&self) -> Result<ConcurrentApi, DynamicApiError> {
        // Note that the error, if it exists, is cloned here.
        self.api.get().await
    }
    pub async fn get_search_suggestions(
        &self,
        text: String,
    ) -> Result<(Vec<SearchSuggestion>, String)> {
        get_search_suggestions(self.get_api().await?, text).await
    }
    pub async fn search_playlists(&self, text: String) -> Result<Vec<SearchResultPlaylist>> {
        search_playlists(self.get_api().await?, text).await
    }
    pub async fn search_artists(&self, text: String) -> Result<Vec<SearchResultArtist>> {
        search_artists(self.get_api().await?, text).await
    }
    pub async fn search_songs(&self, text: String) -> Result<Vec<SearchResultSong>> {
        search_songs(self.get_api().await?, text).await
    }
    pub fn get_playlist_songs(
        &self,
        playlist_id: PlaylistID<'static>,
        max_results: usize,
    ) -> impl Stream<Item = GetPlaylistSongsProgressUpdate> + 'static + use<> {
        let api = self.api.clone();
        get_playlist_songs(api, playlist_id, max_results)
    }
    pub fn get_artist_songs(
        &self,
        browse_id: ArtistChannelID<'static>,
    ) -> impl Stream<Item = GetArtistSongsProgressUpdate> + 'static + use<> {
        let api = self.api.clone();
        get_artist_songs(api, browse_id)
    }
    pub async fn get_watch_playlist(
        &self,
        video_id: VideoID<'static>,
    ) -> Result<Vec<WatchPlaylistTrack>> {
        get_watch_playlist(self.get_api().await?, video_id).await
    }
}

/// Update the local oauth token file.
async fn update_oauth_token_file(token: OAuthToken) -> Result<()> {
    let mut file_path = get_config_dir()?;
    file_path.push(OAUTH_FILENAME);
    let mut tmpfile_path = file_path.clone();
    tmpfile_path.set_extension("json.tmp");
    let out = serde_json::to_string_pretty(&token)?;
    info!("Updating oauth token at: {:?}", &file_path);
    let mut file = tokio::fs::File::create_new(&tmpfile_path).await?;
    file.write_all(out.as_bytes()).await?;
    tokio::fs::rename(tmpfile_path, &file_path).await?;
    info!("Updated oauth token at: {:?}", file_path);
    Ok(())
}

/// Run a query. If the oauth token is expired, take the lock and refresh
/// it (single retry only). If another error occurs, try a single retry too.
pub async fn query_api_with_retry<Q, O>(api: &ConcurrentApi, query: impl Borrow<Q>) -> Result<O>
where
    Q: ytmapi_rs::query::Query<BrowserToken, Output = O>,
    Q: ytmapi_rs::query::Query<OAuthToken, Output = O>,
{
    let res = api
        .read()
        .await
        .query_browser_or_oauth::<Q, O>(query.borrow())
        .await;
    match res {
        Ok(r) => Ok(r),
        Err(e) => {
            info!("Got error {e} from api");
            match e.downcast::<ytmapi_rs::Error>().map(|e| e.into_kind()) {
                Ok(ytmapi_rs::error::ErrorKind::OAuthTokenExpired { token_hash }) => {
                    // Take a clone to re-use later.
                    let api_clone = api.to_owned();
                    // First take an exclusive lock - prevent others from doing the same.
                    let api_owned = api_clone.clone();
                    let mut api_locked = api_owned.write_owned().await;
                    // Then check to see if the token_hash hasn't changed since calling the
                    // query. If it hasn't, we were the first one and are responsible for
                    // refreshing. If it has, that means another query must have
                    // already refreshed the token, and we don't need to do
                    // anything.
                    let api_token_hash = api_locked.get_token_hash()?;
                    if api_token_hash == Some(token_hash) {
                        // A task is spawned to refresh the token, to ensure that it still
                        // refreshes even if this task is
                        // cancelled.
                        tokio::spawn(async {
                            info!("Refreshing oauth token");
                            let tok = api_locked.refresh_token().await?.expect("Expected to be able to refresh token if I got an OAuthTokenExpired error");
                            info!("Oauth token refreshed");
                            if let Err(e) = update_oauth_token_file(tok).await {
                                error!("Error updating locally saved oauth token: <{e}>")
                            }
                            Ok::<_,anyhow::Error>(api_locked)
                        }).await??;
                    }
                    Ok(api_clone
                        .read_owned()
                        .await
                        .query_browser_or_oauth(query)
                        .await?)
                }
                // Regular retry without token refresh, if token isn't expired.
                Ok(_) => {
                    info!("Retrying once");
                    Ok(api.read().await.query_browser_or_oauth(query).await?)
                }
                // If the DynamicApi didn't return a ytmapi_rs::Error, the error must be
                // non-retryable.
                Err(e) => Err(e),
            }
        }
    }
}

async fn search_playlists(api: ConcurrentApi, text: String) -> Result<Vec<SearchResultPlaylist>> {
    tracing::info!("Searching playlists for {text}");
    let query = ytmapi_rs::query::SearchQuery::new(text)
        .with_filter(ytmapi_rs::query::search::PlaylistsFilter)
        .with_spelling_mode(ytmapi_rs::query::search::SpellingMode::ExactMatch);
    query_api_with_retry(&api, query).await
}

async fn search_artists(api: ConcurrentApi, text: String) -> Result<Vec<SearchResultArtist>> {
    tracing::info!("Searching artists for {text}");
    let query = ytmapi_rs::query::SearchQuery::new(text)
        .with_filter(ytmapi_rs::query::search::ArtistsFilter)
        .with_spelling_mode(ytmapi_rs::query::search::SpellingMode::ExactMatch);
    query_api_with_retry(&api, query).await
}

async fn search_songs(api: ConcurrentApi, text: String) -> Result<Vec<SearchResultSong>> {
    tracing::info!("Searching songs for {text}");
    let query = ytmapi_rs::query::SearchQuery::new(text)
        .with_filter(ytmapi_rs::query::search::SongsFilter)
        .with_spelling_mode(ytmapi_rs::query::search::SpellingMode::ExactMatch);
    query_api_with_retry(&api, query).await
}

pub async fn get_search_suggestions(
    api: ConcurrentApi,
    text: String,
) -> Result<(Vec<SearchSuggestion>, String)> {
    tracing::info!("Getting search suggestions for {text}");
    let query = ytmapi_rs::query::GetSearchSuggestionsQuery::new(&text);
    let results = query_api_with_retry(&api, query).await?;
    Ok((results, text))
}

pub enum GetArtistSongsProgressUpdate {
    Loading,
    // Caller should know the ArtistChannelID already, as they provided it.
    // Stream closes here.
    GetArtistAlbumsError(Error),
    // Stream doesn't close here - maybe some of the other albums were succesfully able to send
    // songs.
    GetAlbumsSongsError {
        album_id: AlbumID<'static>,
        error: Error,
    },
    SongsFound,
    Songs {
        song_list: Vec<AlbumSong>,
        album: ParsedSongAlbum,
        year: String,
        artists: Vec<ParsedSongArtist>,
        thumbnails: Vec<Thumbnail>,
    },
    // Stream closes here.
    AllSongsSent,
    // Stream closes here.
    NoSongsFound,
}

fn get_artist_songs(
    api: Arc<AsyncCell<Result<ConcurrentApi, DynamicApiError>>>,
    browse_id: ArtistChannelID<'static>,
) -> impl Stream<Item = GetArtistSongsProgressUpdate> + 'static {
    let (tx, rx) = tokio::sync::mpsc::channel(CALLBACK_CHANNEL_SIZE);
    let handle = tokio::spawn(async move {
        tracing::info!("Running songs query");
        send_or_error(&tx, GetArtistSongsProgressUpdate::Loading).await;
        let api = match api.get().await {
            Err(e) => {
                error!("Error getting API");
                send_or_error(
                    tx,
                    GetArtistSongsProgressUpdate::GetArtistAlbumsError(e.into()),
                )
                .await;
                return;
            }
            Ok(api) => api,
        };
        let query = ytmapi_rs::query::GetArtistQuery::new(&browse_id);
        let artist = query_api_with_retry(&api, query).await;
        let artist = match artist {
            Ok(a) => a,
            Err(e) => {
                error!("Error with GetArtistQuery");
                send_or_error(tx, GetArtistSongsProgressUpdate::GetArtistAlbumsError(e)).await;
                return;
            }
        };
        let Some(albums) = artist.top_releases.albums else {
            tracing::info!("Telling caller no songs found (no params)");
            send_or_error(tx, GetArtistSongsProgressUpdate::NoSongsFound).await;
            return;
        };

        let GetArtistAlbums {
            browse_id: artist_albums_browse_id,
            params: artist_albums_params,
            results: artist_albums_results,
            ..
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
            send_or_error(&tx, GetArtistSongsProgressUpdate::NoSongsFound).await;
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
            let albums = match query_api_with_retry(&api, query).await {
                Ok(r) => r,
                Err(e) => {
                    error!("Received error on get_artist_albums query \"{}\"", e);
                    send_or_error(tx, GetArtistSongsProgressUpdate::GetArtistAlbumsError(e)).await;
                    return;
                }
            };
            albums.into_iter().map(|a| a.browse_id).collect()
        };
        send_or_error(&tx, GetArtistSongsProgressUpdate::SongsFound).await;
        // Request all albums, concurrently but retaining order.
        // Future improvement: instead of using a FuturesOrdered, we could send
        // willy-nilly but with an index, so the caller can insert songs in place.
        let mut stream = browse_id_list
            .into_iter()
            .inspect(|a_id| {
                tracing::info!("Spawning request for caller tracks for album ID {:?}", a_id,)
            })
            .map(|a_id| {
                let api = api.clone();
                async move {
                    let query = GetAlbumQuery::new(&a_id);
                    (query_api_with_retry(&api, query).await, a_id)
                }
            })
            .collect::<FuturesOrdered<_>>();
        while let Some((maybe_album, album_id)) = stream.next().await {
            let album = match maybe_album {
                Ok(album) => album,
                Err(e) => {
                    error!("Error with GetAlbumQuery, album {:?}", album_id);
                    send_or_error(
                        &tx,
                        GetArtistSongsProgressUpdate::GetAlbumsSongsError { album_id, error: e },
                    )
                    .await;
                    continue;
                }
            };
            let GetAlbum {
                title,
                artists,
                year,
                tracks,
                thumbnails,
                ..
            } = album;
            tracing::info!("Sending caller tracks for artist {:?}", browse_id);
            send_or_error(
                &tx,
                GetArtistSongsProgressUpdate::Songs {
                    song_list: tracks,
                    album: ParsedSongAlbum {
                        name: title,
                        id: album_id,
                    },
                    year,
                    artists,
                    thumbnails,
                },
            )
            .await;
        }
        send_or_error(tx, GetArtistSongsProgressUpdate::AllSongsSent).await;
    });
    PanickingReceiverStream::new(rx, handle)
}

pub enum GetPlaylistSongsProgressUpdate {
    Loading,
    Songs(Vec<PlaylistItem>),
    // PlaylistID is returned to allow caller to reuse allocation if required.
    // May occur before or after sending some songs, ie api could fail straight away or stream
    // some songs then fail. Stream closes here.
    GetPlaylistSongsError {
        playlist_id: PlaylistID<'static>,
        error: Error,
    },
    // Stream closes here.
    AllSongsSent,
}

fn get_playlist_songs(
    api: Arc<AsyncCell<Result<ConcurrentApi, DynamicApiError>>>,
    playlist_id: PlaylistID<'static>,
    _max_results: usize,
) -> impl Stream<Item = GetPlaylistSongsProgressUpdate> + 'static {
    let (tx, rx) = tokio::sync::mpsc::channel(CALLBACK_CHANNEL_SIZE);
    let handle = tokio::spawn(async move {
        tracing::info!("Running songs query");
        send_or_error(&tx, GetPlaylistSongsProgressUpdate::Loading).await;
        let api = match api.get().await {
            Err(e) => {
                error!("Error getting API");
                send_or_error(
                    tx,
                    GetPlaylistSongsProgressUpdate::GetPlaylistSongsError {
                        playlist_id,
                        error: e.into(),
                    },
                )
                .await;
                return;
            }
            Ok(api) => api,
        };
        let query = ytmapi_rs::query::GetPlaylistTracksQuery::new((&playlist_id).into());
        // TODO: Streaming
        let first_tracks = query_api_with_retry(&api, query).await;
        match first_tracks {
            Ok(t) => {
                info!("Sending caller tracks for {:?}", playlist_id);
                send_or_error(&tx, GetPlaylistSongsProgressUpdate::Songs(t)).await;
            }
            Err(error) => {
                error!("Error with GetPlaylistTracksQuery");
                send_or_error(
                    &tx,
                    GetPlaylistSongsProgressUpdate::GetPlaylistSongsError { playlist_id, error },
                )
                .await;
                return;
            }
        }
        send_or_error(tx, GetPlaylistSongsProgressUpdate::AllSongsSent).await;
    });
    PanickingReceiverStream::new(rx, handle)
}

async fn get_watch_playlist(
    api: ConcurrentApi,
    video_id: VideoID<'static>,
) -> Result<Vec<WatchPlaylistTrack>> {
    tracing::info!("Getting watch playlist for {:?}", video_id);
    let query = GetWatchPlaylistQuery::new_from_video_id(video_id);
    query_api_with_retry(&api, query).await
}
