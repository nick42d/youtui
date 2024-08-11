//! Module to allow dynamic use of the generic 'YtMusic' struct at runtime.
use crate::{
    config::{ApiKey, AuthType},
    error::Error,
    Result,
};
use std::borrow::Borrow;
use ytmapi_rs::{
    auth::{BrowserToken, OAuthToken},
    query::Query,
    YtMusic, YtMusicBuilder,
};

#[derive(Debug, Clone)]
pub enum DynamicYtMusic {
    Browser(YtMusic<BrowserToken>),
    OAuth(YtMusic<OAuthToken>),
}

impl DynamicYtMusic {
    pub async fn new(key: ApiKey) -> Result<Self> {
        match key {
            ApiKey::BrowserToken(cookie) => Ok(DynamicYtMusic::Browser(
                YtMusicBuilder::new_rustls_tls()
                    .with_browser_token_cookie(cookie)
                    .build()
                    .await?,
            )),
            ApiKey::OAuthToken(token) => Ok(DynamicYtMusic::OAuth(
                YtMusicBuilder::new_rustls_tls()
                    .with_oauth_token(token)
                    .build()?,
            )),
        }
    }
    // TO DETERMINE HOW TO HANDLE BROWSER CASE.
    pub async fn refresh_token(&mut self) -> Result<Option<OAuthToken>> {
        Ok(match self {
            DynamicYtMusic::Browser(_) => None,
            DynamicYtMusic::OAuth(yt) => Some(yt.refresh_token().await?),
        })
    }
    // TO DETERMINE HOW TO HANDLE BROWSER CASE.
    pub fn get_token_hash(&self) -> Result<Option<u64>> {
        Ok(match self {
            DynamicYtMusic::Browser(_) => None,
            DynamicYtMusic::OAuth(yt) => Some(yt.get_token_hash()),
        })
    }
    pub async fn query<Q, O>(&self, query: impl Borrow<Q>) -> Result<O>
    where
        Q: Query<BrowserToken, Output = O>,
        Q: Query<OAuthToken, Output = O>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(yt) => yt.query(query).await?,
            DynamicYtMusic::OAuth(yt) => yt.query(query).await?,
        })
    }
    pub async fn browser_query<Q>(&self, query: impl Borrow<Q>) -> Result<Q::Output>
    where
        Q: Query<BrowserToken>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(yt) => yt.query(query).await?,
            DynamicYtMusic::OAuth(_) => {
                return Err(Error::new_wrong_auth_token_error_browser(
                    query,
                    AuthType::OAuth,
                ))
            }
        })
    }
    pub async fn oauth_query<Q>(&self, query: impl Borrow<Q>) -> Result<Q::Output>
    where
        Q: Query<OAuthToken>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(_) => {
                return Err(Error::new_wrong_auth_token_error_oauth(
                    query,
                    AuthType::Browser,
                ))
            }
            DynamicYtMusic::OAuth(yt) => yt.query(query).await?,
        })
    }
    pub async fn query_source<Q, O>(&self, query: &Q) -> Result<String>
    where
        Q: Query<BrowserToken, Output = O>,
        Q: Query<OAuthToken, Output = O>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(yt) => {
                yt.raw_query(query).await.map(|r| r.destructure_json())?
            }
            DynamicYtMusic::OAuth(yt) => yt.raw_query(query).await.map(|r| r.destructure_json())?,
        })
    }
    pub async fn browser_query_source<Q>(&self, query: &Q) -> Result<String>
    where
        Q: Query<BrowserToken>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(yt) => {
                yt.raw_query(query).await.map(|r| r.destructure_json())?
            }
            DynamicYtMusic::OAuth(_) => {
                return Err(Error::new_wrong_auth_token_error_browser(
                    query,
                    AuthType::OAuth,
                ))
            }
        })
    }
    pub async fn oauth_query_source<Q>(&self, query: &Q) -> Result<String>
    where
        Q: Query<OAuthToken>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(_) => {
                return Err(Error::new_wrong_auth_token_error_oauth(
                    query,
                    AuthType::Browser,
                ))
            }
            DynamicYtMusic::OAuth(yt) => yt.raw_query(query).await.map(|r| r.destructure_json())?,
        })
    }
}
