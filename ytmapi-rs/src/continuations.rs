//! This module contains the `Continuable` trait, allowing streaming of results
//! that contain continuations.
use crate::{
    auth::AuthToken,
    common::ContinuationParams,
    parse::ParseFrom,
    query::{GetContinuationsQuery, PostMethod, PostQuery, Query, QueryMethod},
    ProcessedResult, Result,
};
use futures::Stream;
use std::fmt::Debug;

/// This trait represents a result that can be streamed to get more results.
/// It will contain continuation params, and a parsing function for its
/// continuations.
// TODO: Implementation example.
// TODO: Documement _why_ we need to take_continuation_params and we can't just
// use a reference.
pub trait Continuable<Q>: Sized {
    fn take_continuation_params(&mut self) -> Option<ContinuationParams<'static>>;
    fn parse_continuation(p: ProcessedResult<GetContinuationsQuery<'_, Q>>) -> Result<Self>;
}

// Implementing Continuable<Q> for T implies ParseFrom<GetContinuationsQuery<Q>
// for T.
// TODO: Consider if this lives here, or in parse module.
impl<'a, T, Q> ParseFrom<GetContinuationsQuery<'a, Q>> for T
where
    T: Continuable<Q>,
    T: Debug,
{
    fn parse_from(p: ProcessedResult<GetContinuationsQuery<'a, Q>>) -> Result<Self> {
        T::parse_continuation(p)
    }
}

/// Stream a query that can be streamed.
/// This function has quite complicated trait bounds. To step through them;
/// - query must meet the standard trait bounds for a query - Q: Query<A:
///   AuthToken>.
/// - only PostQuery queries can be streamed - therefore we add the trait bound
///   Q: PostQuery - this simplifies code within this function.
/// - a query can only be streamed if the output is Continuable - therefore we
///   specify Q::Output: Continuable<Q>.
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
    Q::Output: Continuable<Q>,
{
    futures::stream::unfold(
        (false, None::<GetContinuationsQuery<Q>>),
        move |(first, maybe_next_query)| async move {
            if !first {
                let first_res: Result<Q::Output> = Q::Method::call(query, client, tok)
                    .await
                    .and_then(|res| res.process())
                    .and_then(|res| res.parse_into());
                match first_res {
                    Ok(mut first) => {
                        let maybe_next_query = GetContinuationsQuery::<Q>::new(&mut first, query);
                        return Some((Ok(first), (true, maybe_next_query)));
                    }
                    Err(e) => return Some((Err(e), (true, None))),
                }
            }
            if let Some(next_query) = maybe_next_query {
                let next = PostMethod::call(&next_query, client, tok)
                    .await
                    .and_then(|res| res.process())
                    .and_then(|res| res.parse_into());

                match next {
                    Ok(mut next) => {
                        let maybe_next_query = GetContinuationsQuery::<Q>::new(&mut next, query);
                        return Some((Ok(next), (true, maybe_next_query)));
                    }
                    Err(e) => return Some((Err(e), (true, None))),
                }
            }
            None
        },
    )
}
