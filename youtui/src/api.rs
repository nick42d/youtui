//! Module to allow dynamic use of the generic 'YtMusic' struct at runtime.
use crate::config::{ApiKey, AuthType};
use anyhow::{bail, Result};
use error::wrong_auth_token_error_message;
pub use error::*;
use futures::{StreamExt, StreamExt, TryStreamExt, TryStreamExt};
use rusty_ytdl::reqwest;
use std::borrow::Borrow;
use ytmapi_rs::auth::noauth::NoAuthToken;
use ytmapi_rs::auth::{BrowserToken, BrowserToken, OAuthToken, OAuthToken};
use ytmapi_rs::continuations::Continuable;
use ytmapi_rs::query::{PostQuery, PostQuery, Query, Query};
use ytmapi_rs::{YtMusic, YtMusic, YtMusicBuilder, YtMusicBuilder};
mod error;

#[derive(Debug, Clone)]
pub enum DynamicYtMusic {
    Browser(YtMusic<BrowserToken>),
    OAuth(YtMusic<OAuthToken>),
    NoAuth(YtMusic<NoAuthToken>),
}

impl DynamicYtMusic {
    pub async fn new(key: ApiKey, client: reqwest::Client) -> Result<Self, error::DynamicApiError> {
        match key {
            ApiKey::BrowserToken(cookie) => Ok(DynamicYtMusic::Browser(
                YtMusicBuilder::new_with_client(ytmapi_rs::Client::new_from_reqwest_client(client))
                    .with_browser_token_cookie(cookie)
                    .build()
                    .await?,
            )),
            ApiKey::OAuthToken(token) => Ok(DynamicYtMusic::OAuth(
                YtMusicBuilder::new_rustls_tls()
                    .with_oauth_token(token)
                    .build()?,
            )),
            ApiKey::None => Ok(DynamicYtMusic::NoAuth(
                YtMusicBuilder::new_rustls_tls().build().await?,
            )),
        }
    }
    // TO DETERMINE HOW TO HANDLE BROWSER/NOAUTH CASE.
    pub async fn refresh_token(&mut self) -> Result<Option<OAuthToken>> {
        Ok(match self {
            DynamicYtMusic::Browser(_) | DynamicYtMusic::NoAuth(_) => None,
            DynamicYtMusic::OAuth(yt) => Some(yt.refresh_token().await?),
        })
    }
    // TO DETERMINE HOW TO HANDLE BROWSER/NOAUTH CASE.
    pub fn get_token_hash(&self) -> Result<Option<u64>> {
        Ok(match self {
            DynamicYtMusic::Browser(_) | DynamicYtMusic::NoAuth(_) => None,
            DynamicYtMusic::OAuth(yt) => Some(yt.get_token_hash()),
        })
    }
    pub async fn query<Q, O>(&self, query: impl Borrow<Q>) -> Result<O>
    where
        Q: Query<BrowserToken, Output = O>,
        Q: Query<OAuthToken, Output = O>,
        Q: Query<NoAuthToken, Output = O>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(yt) => yt.query(query).await?,
            DynamicYtMusic::OAuth(yt) => yt.query(query).await?,
            DynamicYtMusic::NoAuth(yt) => yt.query(query).await?,
        })
    }
    pub async fn query_browser_or_oauth<Q, O>(&self, query: impl Borrow<Q>) -> Result<O>
    where
        Q: Query<BrowserToken, Output = O>,
        Q: Query<OAuthToken, Output = O>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(yt) => yt.query(query).await?,
            DynamicYtMusic::OAuth(yt) => yt.query(query).await?,
            DynamicYtMusic::NoAuth(_) => bail!(wrong_auth_token_error_message::<Q>(
                AuthType::Unauthenticated,
                &[AuthType::Browser, AuthType::OAuth]
            )),
        })
    }
    pub async fn _stream<Q, O>(&self, query: impl Borrow<Q>, max_pages: usize) -> Result<Vec<O>>
    where
        Q: Query<BrowserToken, Output = O>,
        Q: Query<OAuthToken, Output = O>,
        Q: Query<NoAuthToken, Output = O>,
        O: Continuable<Q>,
        Q: PostQuery,
    {
        Ok(match self {
            DynamicYtMusic::Browser(yt) => {
                yt.stream(query.borrow())
                    .take(max_pages)
                    .try_collect()
                    .await?
            }
            DynamicYtMusic::OAuth(yt) => {
                yt.stream(query.borrow())
                    .take(max_pages)
                    .try_collect()
                    .await?
            }
            DynamicYtMusic::NoAuth(yt) => {
                yt.stream(query.borrow())
                    .take(max_pages)
                    .try_collect()
                    .await?
            }
        })
    }
    pub async fn stream_browser_or_oauth<Q, O>(
        &self,
        query: impl Borrow<Q>,
        max_pages: usize,
    ) -> Result<Vec<O>>
    where
        Q: Query<BrowserToken, Output = O>,
        Q: Query<OAuthToken, Output = O>,
        O: Continuable<Q>,
        Q: PostQuery,
    {
        Ok(match self {
            DynamicYtMusic::Browser(yt) => {
                yt.stream(query.borrow())
                    .take(max_pages)
                    .try_collect()
                    .await?
            }
            DynamicYtMusic::OAuth(yt) => {
                yt.stream(query.borrow())
                    .take(max_pages)
                    .try_collect()
                    .await?
            }
            DynamicYtMusic::NoAuth(_) => bail!(wrong_auth_token_error_message::<Q>(
                AuthType::Unauthenticated,
                &[AuthType::Browser, AuthType::OAuth]
            )),
        })
    }
    pub async fn query_source<Q, O>(&self, query: &Q) -> Result<String>
    where
        Q: Query<BrowserToken, Output = O>,
        Q: Query<OAuthToken, Output = O>,
        Q: Query<NoAuthToken, Output = O>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(yt) => {
                yt.raw_query(query).await.map(|r| r.destructure_json())?
            }
            DynamicYtMusic::OAuth(yt) => yt.raw_query(query).await.map(|r| r.destructure_json())?,
            DynamicYtMusic::NoAuth(yt) => {
                yt.raw_query(query).await.map(|r| r.destructure_json())?
            }
        })
    }
    pub async fn query_source_browser_or_oauth<Q, O>(&self, query: &Q) -> Result<String>
    where
        Q: Query<BrowserToken, Output = O>,
        Q: Query<OAuthToken, Output = O>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(yt) => {
                yt.raw_query(query).await.map(|r| r.destructure_json())?
            }
            DynamicYtMusic::OAuth(yt) => yt.raw_query(query).await.map(|r| r.destructure_json())?,
            DynamicYtMusic::NoAuth(_) => bail!(wrong_auth_token_error_message::<Q>(
                AuthType::Unauthenticated,
                &[AuthType::Browser, AuthType::OAuth]
            )),
        })
    }
    pub async fn _stream_source<Q, O>(&self, _query: &Q, _max_pages: usize) -> Result<String>
    where
        Q: Query<BrowserToken, Output = O>,
        Q: Query<OAuthToken, Output = O>,
        Q: Query<NoAuthToken, Output = O>,
        Q: PostQuery,
        O: Continuable<Q>,
    {
        bail!("It's not currently possible to get source files for each result of a stream, since the source files get consumed to obtain continuation params");
    }
    pub async fn stream_source_browser_or_oauth<Q, O>(
        &self,
        _query: &Q,
        _max_pages: usize,
    ) -> Result<String>
    where
        Q: Query<BrowserToken, Output = O>,
        Q: Query<OAuthToken, Output = O>,
        Q: PostQuery,
        O: Continuable<Q>,
    {
        bail!("It's not currently possible to get source files for each result of a stream, since the source files get consumed to obtain continuation params");
    }
}
