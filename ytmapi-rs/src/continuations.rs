//! This module contains the `Continuable` trait, allowing streaming of results
//! that contain continuations.
use crate::auth::{AuthToken, RawResult};
use crate::common::ContinuationParams;
use crate::parse::ParseFrom;
use crate::query::{GetContinuationsQuery, PostMethod, PostQuery, Query, QueryMethod};
use crate::{ProcessedResult, Result};
use futures::Stream;
use std::fmt::Debug;

/// This trait represents a result that can be streamed to get more results.
/// It will contain continuation params, and a parsing function for its
/// continuations.
// TODO: Implementation example.
pub trait ParseFromContinuable<Q>: Debug + Sized {
    fn parse_from_continuable(
        p: ProcessedResult<Q>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)>;
    fn parse_continuation(
        p: ProcessedResult<GetContinuationsQuery<'_, Q>>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)>;
}

impl<T, Q> ParseFrom<Q> for T
where
    T: ParseFromContinuable<Q>,
{
    fn parse_from(p: ProcessedResult<Q>) -> crate::Result<Self> {
        T::parse_from_continuable(p).map(|t| t.0)
    }
}

impl<T, Q> ParseFromContinuable<GetContinuationsQuery<'_, Q>> for T
where
    T: std::fmt::Debug + Sized,
    T: ParseFromContinuable<Q>,
{
    fn parse_from_continuable(
        p: ProcessedResult<GetContinuationsQuery<Q>>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        T::parse_continuation(p)
    }
    fn parse_continuation(
        p: ProcessedResult<GetContinuationsQuery<'_, GetContinuationsQuery<Q>>>,
    ) -> crate::Result<(Self, Option<ContinuationParams<'static>>)> {
        todo!()
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
// TODO: It may be possible to remove the Q: PostQuery bound,
// instead calling QueryMethod<...>::Call directly.
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
/// output, by cloning the source before yielding it. This function has quite
/// complicated trait bounds. To step through them;
/// - query must meet the standard trait bounds for a query - Q: Query<A:
///   AuthToken>.
/// - only PostQuery queries can be streamed - therefore we add the trait bound
///   Q: PostQuery - this simplifies code within this function.
/// - a query can only be streamed if the output is Continuable - therefore we
///   specify Q::Output: ParseFromContinuable<Q>.
// TODO: It may be possible to remove the Q: PostQuery bound,
// instead calling QueryMethod<...>::Call directly.
pub(crate) fn stream_with_source<'a, Q, A>(
    query: &'a Q,
    client: &'a crate::client::Client,
    tok: &'a A,
) -> impl Stream<Item = Result<(String, Q::Output)>> + 'a
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
                    .and_then(|res| {
                        Ok((
                            res.source.clone(),
                            GetContinuationsQuery::from_first_result(res)?,
                        ))
                    });
                match first_res {
                    Ok((first_source, (first_output, next_query))) => {
                        return Some((Ok((first_source, first_output)), (true, next_query)));
                    }
                    Err(e) => return Some((Err(e), (true, None))),
                }
            }
            if let Some(ref next_query) = maybe_next_query {
                let next_res = PostMethod::call(next_query, client, tok)
                    .await
                    .and_then(|res| res.process());
                let next_res = next_res.and_then(|res| {
                    Ok((
                        res.source.clone(),
                        GetContinuationsQuery::from_continuation(res)?,
                    ))
                });
                match next_res {
                    Ok((this_source, (this_output, next_query))) => {
                        return Some((Ok((this_source, this_output)), (true, next_query)));
                    }
                    Err(e) => return Some((Err(e), (true, None))),
                }
            }
            None
        },
    )
}
