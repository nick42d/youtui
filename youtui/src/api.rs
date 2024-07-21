//! Module to allow dynamic use of the generic 'YtMusic' struct at runtime.

use ytmapi_rs::{
    auth::{AuthToken, BrowserToken, OAuthToken},
    query::Query,
    YtMusic,
};

#[derive(Debug, Clone)]
pub enum DynamicYtMusic {
    Browser(YtMusic<BrowserToken>),
    OAuth(YtMusic<OAuthToken>),
}

// impl DynamicYtMusic {
//     pub async fn query<Q: Query<A>, A: AuthToken>(&self, query: Q) ->
// ytmapi_rs::Result<Q::Output> {         let res = match self {
//             DynamicYtMusic::Browser(yt) => yt.query(query).await,
//             DynamicYtMusic::OAuth(yt) => todo!(), // yt.query(query).await,
//         };
//         res
//     }
// }
