//! Module to allow dynamic use of the generic 'YtMusic' struct at runtime.

use ytmapi_rs::{
    auth::{AuthToken, BrowserToken, OAuthToken},
    parse::ParseFrom,
    query::{self, GetLibraryAlbumsQuery, Query},
    YtMusic,
};

#[derive(Debug, Clone)]
pub enum DynamicYtMusic {
    Browser(YtMusic<BrowserToken>),
    OAuth(YtMusic<OAuthToken>),
}

impl DynamicYtMusic {
    pub async fn query<Q, O>(&self, query: Q) -> ytmapi_rs::Result<O>
    where
        Q: Query<BrowserToken, Output = O>,
        Q: Query<OAuthToken, Output = O>,
    {
        match self {
            DynamicYtMusic::Browser(yt) => yt.query(query).await,
            DynamicYtMusic::OAuth(yt) => yt.query(query).await,
        }
    }
    pub async fn browser_query<Q>(&self, query: Q) -> ytmapi_rs::Result<Q::Output>
    where
        Q: Query<BrowserToken>,
    {
        match self {
            DynamicYtMusic::Browser(yt) => yt.query(query).await,
            DynamicYtMusic::OAuth(_) => panic!("Should return an error"),
        }
    }
    pub async fn oauth_query<Q>(&self, query: Q) -> ytmapi_rs::Result<Q::Output>
    where
        Q: Query<OAuthToken>,
    {
        match self {
            DynamicYtMusic::Browser(_) => panic!("Should return an error"),
            DynamicYtMusic::OAuth(yt) => yt.query(query).await,
        }
    }
    pub async fn query_source<Q, O>(&self, query: Q) -> ytmapi_rs::Result<String>
    where
        Q: Query<BrowserToken, Output = O>,
        Q: Query<OAuthToken, Output = O>,
    {
        match self {
            DynamicYtMusic::Browser(yt) => yt.raw_query(query).await.map(|r| r.destructure_json()),
            DynamicYtMusic::OAuth(yt) => yt.raw_query(query).await.map(|r| r.destructure_json()),
        }
    }
    pub async fn browser_query_source<Q>(&self, query: Q) -> ytmapi_rs::Result<String>
    where
        Q: Query<BrowserToken>,
    {
        match self {
            DynamicYtMusic::Browser(yt) => yt.raw_query(query).await.map(|r| r.destructure_json()),
            DynamicYtMusic::OAuth(_) => panic!("Should return an error"),
        }
    }
    pub async fn oauth_query_source<Q>(&self, query: Q) -> ytmapi_rs::Result<String>
    where
        Q: Query<OAuthToken>,
    {
        match self {
            DynamicYtMusic::Browser(_) => panic!("Should return an error"),
            DynamicYtMusic::OAuth(yt) => yt.raw_query(query).await.map(|r| r.destructure_json()),
        }
    }
}
