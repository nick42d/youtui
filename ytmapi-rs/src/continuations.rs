//! This module contains the `ParseFromContinuable` trait, allowing streaming of
//! results that contain continuations.
//! # Implementation example - Incomplete
//! ```no_run
//! # struct GetDateQuery;
//! use serde::Deserialize;
//! use ytmapi_rs::common::ContinuationParams;
//! use ytmapi_rs::query::GetContinuationsQuery;
//!
//! #[derive(Debug, Deserialize)]
//! struct Date {
//!     date_string: String,
//!     date_timestamp: usize,
//! }
//! impl ytmapi_rs::continuations::ParseFromContinuable<GetDateQuery> for () {
//!     fn parse_from_continuable(
//!         p: ytmapi_rs::ProcessedResult<GetDateQuery>,
//!     ) -> ytmapi_rs::Result<(Self, Option<ContinuationParams<'static>>)> {
//!         todo!();
//!     }
//!     fn parse_continuation(
//!         p: ytmapi_rs::ProcessedResult<GetContinuationsQuery<'_, GetDateQuery>>,
//!     ) -> ytmapi_rs::Result<(Self, Option<ContinuationParams<'static>>)> {
//!         todo!();
//!     }
//! }
//! ```
//! # Alternative implementation
//! An alternative to working directly with [`crate::json::Json`] is to add
//! `json-crawler` as a dependency and use the provided
//! `From<ProcessedResult> for JsonCrawlerOwned` implementation.
use crate::auth::AuthToken;
use crate::common::ContinuationParams;
use crate::parse::ParseFrom;
use crate::query::{GetContinuationsQuery, PostMethod, PostQuery, Query, QueryMethod};
use crate::{ProcessedResult, Result};
use futures::Stream;
use std::fmt::Debug;

/// This trait represents a result that can be streamed to get more results.
/// It will contain continuation params, and a parsing function for its
/// continuations.
pub trait ParseFromContinuable<Q>: Debug + Sized {
    fn parse_from_continuable(
        p: ProcessedResult<Q>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)>;
    fn parse_continuation(
        p: ProcessedResult<GetContinuationsQuery<'_, Q>>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)>;
}

/// Blanket implementation of ParseFrom where T implements ParseFromContinuable
/// so that caller can write T::ParseFrom.
impl<T, Q> ParseFrom<Q> for T
where
    T: ParseFromContinuable<Q>,
{
    fn parse_from(p: ProcessedResult<Q>) -> crate::Result<Self> {
        T::parse_from_continuable(p).map(|t| t.0)
    }
}

/// Stream a query that can be streamed.
/// This function has quite complicated trait bounds. To step through them;
/// - query must meet the standard trait bounds for a query - Q: Query<A:
///   AuthToken>.
/// - only PostQuery queries can be streamed - therefore we add the trait bound
///   Q: PostQuery - this simplifies code within this function.
/// - a query can only be streamed if the output is Continuable - therefore we
///   specify Q::Output: ParseFromContinuable<Q>.
pub(crate) fn stream<'a, Q, A>(
    query: &'a Q,
    client: &'a crate::client::Client,
    tok: &'a A,
) -> impl Stream<Item = Result<Q::Output>> + 'a
where
    A: AuthToken,
    Q: Query<A>,
    Q: PostQuery,
    Q::Output: ParseFromContinuable<Q>,
{
    futures::stream::unfold(
        // Initial state for unfold
        // The first component is that the first query hasn't been run.
        // The second component of state represents if there are continuations
        // (this is ignored on first run)
        (false, None::<GetContinuationsQuery<'a, Q>>),
        move |(first_query_run, maybe_next_query)| async move {
            if !first_query_run {
                let first_res = Q::Method::call(query, client, tok)
                    .await
                    .and_then(|res| res.process())
                    .and_then(|res| GetContinuationsQuery::from_first_result(res));
                match first_res {
                    Ok((first, next)) => {
                        return Some((Ok(first), (true, next)));
                    }
                    Err(e) => return Some((Err(e), (true, None))),
                }
            }
            if let Some(ref next_query) = maybe_next_query {
                let next_res = PostMethod::call(next_query, client, tok)
                    .await
                    .and_then(|res| res.process());
                let next_res =
                    next_res.and_then(|res| GetContinuationsQuery::from_continuation(res));
                match next_res {
                    Ok((this, next)) => {
                        return Some((Ok(this), (true, next)));
                    }
                    Err(e) => return Some((Err(e), (true, None))),
                }
            }
            None
        },
    )
}

/// Stream a query that can be streamed, returning the source as well as the
/// output, by cloning the source before yielding it.
/// Note that the stream will stop if an error is detected (after returning
/// the source string that produced the error).
/// This function has quite complicated trait bounds. To step through them;
/// - query must meet the standard trait bounds for a query - Q: Query<A:
///   AuthToken>.
/// - only PostQuery queries can be streamed - therefore we add the trait bound
///   Q: PostQuery - this simplifies code within this function.
/// - a query can only be streamed if the output is Continuable - therefore we
///   specify Q::Output: ParseFromContinuable<Q>.
pub(crate) fn raw_json_stream<'a, Q, A>(
    query: &'a Q,
    client: &'a crate::client::Client,
    tok: &'a A,
) -> impl Stream<Item = Result<String>> + 'a
where
    A: AuthToken,
    Q: Query<A>,
    Q: PostQuery,
    Q::Output: ParseFromContinuable<Q>,
{
    futures::stream::unfold(
        // Initial state for unfold
        // The first component is that the first query hasn't been run.
        // The second component of state represents if there are continuations
        // (this is ignored on first run)
        (false, None::<GetContinuationsQuery<'a, Q>>),
        move |(first_query_run, maybe_next_query)| async move {
            if !first_query_run {
                let first_raw_res = Q::Method::call(query, client, tok).await;
                match first_raw_res {
                    Ok(first_raw_res) => {
                        let first_source = first_raw_res.json.clone();
                        let next_query = first_raw_res
                            .process()
                            .and_then(GetContinuationsQuery::from_first_result::<Q::Output>)
                            .ok()
                            .and_then(|(_, q)| q);
                        return Some((Ok(first_source), (true, next_query)));
                    }
                    Err(e) => return Some((Err(e), (true, None))),
                }
            }
            if let Some(ref next_query) = maybe_next_query {
                let next_raw_res =
                    <GetContinuationsQuery<Q> as Query<A>>::Method::call(next_query, client, tok)
                        .await;
                match next_raw_res {
                    Ok(next_raw_res) => {
                        let next_source = next_raw_res.json.clone();
                        let next_query = next_raw_res
                            .process()
                            .and_then(GetContinuationsQuery::from_continuation::<Q::Output>)
                            .ok()
                            .and_then(|(_, q)| q);
                        return Some((Ok(next_source), (true, next_query)));
                    }
                    Err(e) => return Some((Err(e), (true, None))),
                }
            }
            None
        },
    )
}
