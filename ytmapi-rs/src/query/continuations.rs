use super::{BasicSearch, GetQuery, PostMethod, PostQuery, Query, QueryMethod, SearchQuery};
use crate::{
    auth::{AuthToken, BrowserToken},
    common::{ContinuationParams, YoutubeID},
    parse::{self, ParseFrom, ProcessedResult},
    RawResult, Result,
};
use async_stream::{stream, try_stream};
use std::{borrow::Cow, fmt::Debug, future::Future, marker::PhantomData, pin::pin, vec::Vec};
use tokio::stream;
use tokio_stream::Stream;

pub trait Continuable {
    fn take_continuation_params(&mut self) -> Option<ContinuationParams<'static>>;
}

impl<'a, A: AuthToken> StreamingQuery<'a, A> for super::GetLibrarySongsQuery {}

// NOTE: StreamingQuery only implemented for Self: PostQuery - only post queries
// have continuations.
pub trait StreamingQuery<'a, A: AuthToken>: Query<A>
where
    Self: 'a,
    Self::Output: Continuable,
    Self: PostQuery,
    Self::Output: ParseFrom<GetContinuationsQuery<'a, Self>>,
{
    fn stream(
        &'a self,
        client: &'a crate::client::Client,
        tok: &'a A,
    ) -> impl Stream<Item = Result<Self::Output>> + 'a {
        try_stream! {
            let mut first_res: Self::Output = Self::Method::call(self, client, tok)
                .await?
                .process()?
                .parse_into()?;
            let mut maybe_next_query = GetContinuationsQuery::<Self>::new(&mut first_res, self);
            yield first_res;
            while let Some(next_query) = maybe_next_query {
                let mut next = next_query
                    .call_this(client, tok)
                    .await?
                    .process()?
                    .parse_into()?;
                maybe_next_query = GetContinuationsQuery::<Self>::new(&mut next, self);
                yield next;
            };
        }
    }
}

impl<'a, Q: Query<A>, A: AuthToken> Query<A> for GetContinuationsQuery<'a, Q>
where
    Q: PostQuery,
    Q::Output: ParseFrom<Self>,
{
    type Output = Q::Output;
    type Method = PostMethod;
}

impl<'a, Q> PostQuery for GetContinuationsQuery<'a, Q>
where
    Q: PostQuery,
{
    fn header(&self) -> serde_json::Map<String, serde_json::Value> {
        self.query.header()
    }
    fn params(&self) -> Vec<(&str, Cow<str>)> {
        let params = self.continuation_params.get_raw();
        vec![("ctoken", params.into()), ("continuation", params.into())]
    }
    fn path(&self) -> &str {
        self.query.path()
    }
}

// NOTE: StreamingQuery only implemented for Self: PostQuery - only post queries
// have continuations.
// HOW TO CREATE A STREAM WITHOUT async_stream CRATE
// pub trait StreamingQuery2<A: AuthToken>: Query<A>
// where
//     Self::Output: Continuable,
//     Self: PostQuery,
// {
//     fn stream<'a>(
//         &'a self,
//         client: &'a crate::client::Client,
//         tok: &'a A,
//     ) -> impl Stream<Item = Result<Self::Output>> + 'a {
//         futures::stream::unfold(
//             (false, None::<GetContinuationsQuery<Self>>),
//             |(first, maybe_next_query)| async move {
//                 if !first {
//                     let first_res: Self::Output = Self::Method::call(self,
// client, tok)                         .await
//                         .unwrap()
//                         .process()
//                         .unwrap()
//                         .parse_into()
//                         .unwrap();
//                     let mut maybe_next_query =
// GetContinuationsQuery::new(&first_res, self);                     return
// Some((first_res, (true, maybe_next_query)));                 }
//                 if let Some(next_query) = maybe_next_query {
//                     let next = next_query
//                         .call_this(client, tok)
//                         .await
//                         .unwrap()
//                         .process()
//                         .unwrap()
//                         .parse_into()
//                         .unwrap();
//                     maybe_next_query = GetContinuationsQuery::new(&next,
// self);                     return Some((next, (true, maybe_next_query)));
//                 }
//                 return None;
//             },
//         )
//     }
// }

pub struct GetContinuationsQuery<'a, Q> {
    query: &'a Q,
    continuation_params: ContinuationParams<'static>,
}

impl<'a, Q> GetContinuationsQuery<'a, Q> {
    pub fn new<I: ParseFrom<Q> + Continuable>(
        res: &'_ mut I,
        query: &'a Q,
    ) -> Option<GetContinuationsQuery<'a, Q>> {
        let continuation_params = res.take_continuation_params()?;
        Some(GetContinuationsQuery {
            continuation_params,
            query,
        })
    }
}
