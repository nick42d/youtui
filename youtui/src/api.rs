//! Module to allow dynamic use of the generic 'YtMusic' struct at runtime.
use crate::{config::AuthType, error::Error, Result};
use ytmapi_rs::{
    auth::{BrowserToken, OAuthToken},
    query::Query,
    YtMusic,
};

#[derive(Debug, Clone)]
pub enum DynamicYtMusic {
    Browser(YtMusic<BrowserToken>),
    OAuth(YtMusic<OAuthToken>),
}

impl DynamicYtMusic {
    pub async fn query<Q, O>(&self, query: Q) -> Result<O>
    where
        Q: Query<BrowserToken, Output = O>,
        Q: Query<OAuthToken, Output = O>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(yt) => yt.query(query).await?,
            DynamicYtMusic::OAuth(yt) => yt.query(query).await?,
        })
    }
    pub async fn browser_query<Q>(&self, query: Q) -> Result<Q::Output>
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
    pub async fn oauth_query<Q>(&self, query: Q) -> Result<Q::Output>
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
    pub async fn query_source<Q, O>(&self, query: Q) -> Result<String>
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
    pub async fn browser_query_source<Q>(&self, query: Q) -> Result<String>
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
    pub async fn oauth_query_source<Q>(&self, query: Q) -> Result<String>
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
