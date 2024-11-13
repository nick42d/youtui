use crate::{
    api::DynamicYtMusic, config::ApiKey, core::send_or_error, error::Error, get_config_dir, Result,
    OAUTH_FILENAME,
};
use async_cell::sync::AsyncCell;
use futures::{future::Shared, stream::FuturesOrdered, Future, FutureExt, TryFutureExt};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::{borrow::Borrow, sync::Arc};
use tokio::{
    io::AsyncWriteExt,
    sync::{mpsc::Sender, RwLock},
    task::JoinHandle,
};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info};
use ytmapi_rs::parse::{AlbumSong, SearchResultArtist};
use ytmapi_rs::{
    auth::{BrowserToken, OAuthToken},
    common::{AlbumID, ArtistChannelID, SearchSuggestion},
    error::ErrorKind,
    parse::{GetAlbum, GetArtistAlbums},
    query::{GetAlbumQuery, GetArtistAlbumsQuery},
};

pub struct Api {
    api: Arc<AsyncCell<std::result::Result<ConcurrentApi, String>>>,
}
pub type ConcurrentApi = Arc<RwLock<DynamicYtMusic>>;

const GET_ARTIST_SONGS_STREAM_SIZE: usize = 50;

impl Api {
    pub fn new(api_key: ApiKey) -> Api {
        let api = AsyncCell::new().into_shared();
        let api_clone = api.clone();
        tokio::spawn(async move {
            let api = DynamicYtMusic::new(api_key)
                .await
                .map(|api| Arc::new(RwLock::new(api)))
                // Hack to allow error to be cloneable.
                .map_err(|e| format!("{:?}", e));
            api_clone.set(api)
        });
        Api { api }
    }
    pub async fn get_api(&self) -> std::result::Result<ConcurrentApi, String> {
        // Note that the error, if it exists, is cloned here.
        self.api.get().await
    }
    pub async fn get_search_suggestions(
        &self,
        text: String,
    ) -> Result<(Vec<SearchSuggestion>, String)> {
        get_search_suggestions(
            self.get_api().await.map_err(Error::new_api_error_string)?,
            text,
        )
        .await
    }
    pub async fn search_artists(&self, text: String) -> Result<Vec<SearchResultArtist>> {
        search_artists(
            self.get_api().await.map_err(Error::new_api_error_string)?,
            text,
        )
        .await
    }
    pub fn get_artist_songs(
        &self,
        browse_id: ArtistChannelID<'static>,
    ) -> impl Stream<Item = GetArtistSongsProgressUpdate> + 'static {
        let api = async { self.get_api().await.map_err(Error::new_api_error_string) };
        get_artist_songs(api, browse_id)
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
// NOTE: Determine how to handle if multiple queries in progress when we lock.
// TODO: Refresh the oauth file also. (send message to server - filemanager -
// component)
pub async fn query_api_with_retry<Q, O>(
    api: &ConcurrentApi,
    query: impl Borrow<Q>,
) -> crate::Result<O>
where
    Q: ytmapi_rs::query::Query<BrowserToken, Output = O>,
    Q: ytmapi_rs::query::Query<OAuthToken, Output = O>,
{
    let res = api.read().await.query::<Q, O>(query.borrow()).await;
    match res {
        Ok(r) => Ok(r),
        Err(Error::Api(e)) => {
            info!("Got error {e} from api");
            match e.into_kind() {
                ErrorKind::OAuthTokenExpired { token_hash } => {
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
                        // A task is spawned to refresh the token, to ensure that it still refreshes
                        // even if this task is cancelled.
                        tokio::spawn(async {
                            info!("Refreshing oauth token");
                            let tok = api_locked.refresh_token().await?.expect("Expected to be able to refresh token if I got an OAuthTokenExpired error");
                            info!("Oauth token refreshed");
                            if let Err(e) = update_oauth_token_file(tok).await {
                                error!("Error updating locally saved oauth token: <{e}>")
                            }
                            Ok::<_,Error>(api_locked)
                        }).await??;
                    }
                    Ok(api_clone.read_owned().await.query(query).await?)
                }
                // Regular retry without token refresh, if token isn't expired.
                _ => {
                    info!("Retrying once");
                    Ok(api.read().await.query(query).await?)
                }
            }
        }
        Err(other_err) => Err(other_err),
    }
}

async fn search_artists(api: ConcurrentApi, text: String) -> Result<Vec<SearchResultArtist>> {
    tracing::info!("Getting artists for {text}");
    let query = ytmapi_rs::query::SearchQuery::new(text)
        .with_filter(ytmapi_rs::query::ArtistsFilter)
        .with_spelling_mode(ytmapi_rs::query::SpellingMode::ExactMatch);
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
    NoSongsFound,
    SearchArtistError,
    SongsFound,
    Songs {
        song_list: Vec<AlbumSong>,
        album: String,
        year: String,
        artist: String,
    },
    AllSongsSent,
}

fn get_artist_songs(
    api: impl Future<Output = Result<ConcurrentApi>> + Send + 'static,
    browse_id: ArtistChannelID<'static>,
) -> impl Stream<Item = GetArtistSongsProgressUpdate> + 'static {
    /// Bailout function that will log an error and send NoSongsFound if we get
    /// an unrecoverable error.
    async fn bailout(e: impl std::fmt::Display, tx: Sender<GetArtistSongsProgressUpdate>) {
        error!("API error received <{e}>");
        info!("Telling caller no songs found (error)");
        send_or_error(tx, GetArtistSongsProgressUpdate::NoSongsFound).await;
    }

    let (tx, rx) = tokio::sync::mpsc::channel(50);
    tokio::spawn(async move {
        tracing::info!("Running songs query");
        send_or_error(&tx, GetArtistSongsProgressUpdate::Loading).await;
        let api = match api.await {
            Err(e) => return bailout(e, tx).await,
            Ok(api) => api,
        };
        let query = ytmapi_rs::query::GetArtistQuery::new(&browse_id);
        let artist = query_api_with_retry(&api, query).await;
        let artist = match artist {
            Ok(a) => a,
            Err(e) => {
                let Error::Api(e) = e else {
                    return bailout(e, tx).await;
                };
                let e = e.into_kind();
                let ErrorKind::JsonParsing(e) = e else {
                    return bailout(e, tx).await;
                };
                let (json, key) = e.get_json_and_key();
                // TODO: Bring loggable json errors into their own function.
                error!("API error recieved at key {:?}", key);
                // compile_error!("Remove shitty logging");
                let path = std::path::Path::new("test.json");
                std::fs::write(path, json)
                    .unwrap_or_else(|e| error!("Error <{e}> writing json log"));
                info!("Wrote json to {:?}", path);
                tracing::info!("Telling caller no songs found (error)");
                send_or_error(tx, GetArtistSongsProgressUpdate::NoSongsFound).await;
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
                    // TODO: Better Error type
                    send_or_error(tx, GetArtistSongsProgressUpdate::SearchArtistError).await;
                    return;
                }
            };
            albums.into_iter().map(|a| a.browse_id).collect()
        };
        send_or_error(&tx, GetArtistSongsProgressUpdate::NoSongsFound).await;
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
                    query_api_with_retry(&api, query).await
                }
            })
            .collect::<FuturesOrdered<_>>();
        while let Some(maybe_album) = stream.next().await {
            let album = match maybe_album {
                Ok(album) => album,
                Err(e) => {
                    error!("Error <{e}> getting album");
                    return;
                }
            };
            let GetAlbum {
                title,
                artists,
                year,
                tracks,
                ..
            } = album;
            tracing::info!("Sending caller tracks for artist {:?}", browse_id);
            send_or_error(
                &tx,
                GetArtistSongsProgressUpdate::Songs {
                    song_list: tracks,
                    album: title,
                    year,
                    artist: artists
                        .into_iter()
                        .next()
                        .map(|a| a.name)
                        .unwrap_or_default(),
                },
            )
            .await;
        }
        send_or_error(tx, GetArtistSongsProgressUpdate::AllSongsSent).await;
    });
    ReceiverStream::new(rx)
}
