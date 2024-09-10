use super::{PostMethod, PostQuery, Query, QueryMethod};
use crate::{
    auth::AuthToken,
    common::{ContinuationParams, YoutubeID},
    parse::{Continuable, ParseFrom},
    Result,
};
use futures::Stream;
use std::{borrow::Cow, vec::Vec};

pub struct GetContinuationsQuery<'a, Q> {
    query: &'a Q,
    continuation_params: ContinuationParams<'static>,
}

impl<'a, Q> GetContinuationsQuery<'a, Q> {
    pub fn new<T: crate::parse::Continuable<Q>>(
        res: &'_ mut T,
        query: &'a Q,
    ) -> Option<GetContinuationsQuery<'a, Q>> {
        let continuation_params = res.take_continuation_params()?;
        Some(GetContinuationsQuery {
            continuation_params,
            query,
        })
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

/// Stream a query that can be streamed.
/// This function has quite complicated trait bounds. To step through them;
/// - query must meet the standard trait bounds for a query - Q: Query<A:
///   AuthToken>.
/// - only PostQuery queries can be streamed - therefore we add the trait bound
///   Q: PostQuery - this simplifies code within this function.
/// - since queries may capture a lifetime (e.g a RateSongQuery<'a>), we specify
///   the captured lifetime as Q: 'a - TBC
/// - a query can only be streamed if the output is Continuable - therefore we
///   specify Q::Output: Continuable<Q>.
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
                let next = next_query
                    .call_this(client, tok)
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
