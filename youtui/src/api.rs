//! Module to allow dynamic use of the generic 'YtMusic' struct at runtime.
use crate::config::{ApiKey, AuthType};
use anyhow::bail;
use futures::{StreamExt, TryStreamExt};
use std::borrow::Borrow;
use ytmapi_rs::{
    auth::{BrowserToken, OAuthToken},
    continuations::Continuable,
    error::ErrorKind,
    query::{PostQuery, Query},
    YtMusic, YtMusicBuilder,
};

// OK, this is a rabbit hole.
// 1. We want to be able store the Result of API creation in a shared cell
//    (needs to be Clone)
// 2. We can't return ytmapi_rs::Error as it is not Clone as it can contain
//    std::io::Error.
// 3. anyhow::Error is also not Clone
// 4. We can't just wrap the error in Arc<anyhow::Error> - can't be converted
//    back to anyhow::Error.
// 5. Therefore, we use this error type which is Clone - converting non-Clone
//    variants to Strign for type erasure.
// 6. The only variant we need to know more than the String representation is
//    the OAuthTokenExpired error, since it's used for retries.
type Result<T> = std::result::Result<T, ApiCreationError>;
#[derive(Clone, Debug)]
pub enum ApiCreationError {
    OAuthTokenExpired {
        token_hash: u64,
    },
    WrongAuthToken {
        current_authtype: AuthType,
        query_name_string: &'static str,
    },
    Other(String),
}

impl ApiCreationError {
    fn new_wrong_auth_token<Q>(current_authtype: AuthType) -> Self {
        ApiCreationError::WrongAuthToken {
            current_authtype,
            query_name_string: std::any::type_name::<Q>(),
        }
    }
}
impl std::error::Error for ApiCreationError {}
impl std::fmt::Display for ApiCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiCreationError::OAuthTokenExpired { token_hash: _ } => {
                write!(f, "OAuth token has expired")
            }
            ApiCreationError::Other(msg) => write!(f, "{msg}"),
            ApiCreationError::WrongAuthToken {
                current_authtype,
                query_name_string,
            } => {
                let expected_authtype = match current_authtype {
                    AuthType::Browser => AuthType::OAuth,
                    AuthType::OAuth => AuthType::Browser,
                };
                write!(
                    f,
                    "Query <{}> not supported on auth type {:?}. Expected auth type: {:?}",
                    query_name_string, current_authtype, expected_authtype
                )
            }
        }
    }
}
impl From<ytmapi_rs::Error> for ApiCreationError {
    fn from(value: ytmapi_rs::Error) -> Self {
        match value.into_kind() {
            ErrorKind::OAuthTokenExpired { token_hash } => {
                ApiCreationError::OAuthTokenExpired { token_hash }
            }
            other => ApiCreationError::Other(other.to_string()),
        }
    }
}

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
    pub async fn stream<Q, O>(&self, query: impl Borrow<Q>, max_pages: usize) -> Result<Vec<O>>
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
        })
    }
    pub async fn _browser_query<Q>(&self, query: impl Borrow<Q>) -> Result<Q::Output>
    where
        Q: Query<BrowserToken>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(yt) => yt.query(query).await?,
            DynamicYtMusic::OAuth(_) => bail!(wrong_auth_token_error_message::<Q>(AuthType::OAuth)),
        })
    }
    pub async fn _oauth_query<Q>(&self, query: impl Borrow<Q>) -> Result<Q::Output>
    where
        Q: Query<OAuthToken>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(_) => {
                bail!(wrong_auth_token_error_message::<Q>(AuthType::Browser))
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
    pub async fn stream_source<Q, O>(&self, _query: &Q, _max_pages: usize) -> Result<String>
    where
        Q: Query<BrowserToken, Output = O>,
        Q: Query<OAuthToken, Output = O>,
        Q: PostQuery,
        O: Continuable<Q>,
    {
        bail!("It's not currently possible to get source files for each result of a stream, since the source files get consumed to obtain continuation params".to_string())
    }
    pub async fn _browser_query_source<Q>(&self, query: &Q) -> Result<String>
    where
        Q: Query<BrowserToken>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(yt) => {
                yt.raw_query(query).await.map(|r| r.destructure_json())?
            }
            DynamicYtMusic::OAuth(_) => bail!(wrong_auth_token_error_message::<Q>(AuthType::OAuth)),
        })
    }
    pub async fn _oauth_query_source<Q>(&self, query: &Q) -> Result<String>
    where
        Q: Query<OAuthToken>,
    {
        Ok(match self {
            DynamicYtMusic::Browser(_) => {
                bail!(wrong_auth_token_error_message::<Q>(AuthType::Browser))
            }
            DynamicYtMusic::OAuth(yt) => yt.raw_query(query).await.map(|r| r.destructure_json())?,
        })
    }
}
