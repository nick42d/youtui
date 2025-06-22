use super::{PostMethod, PostQuery, Query};
use crate::auth::AuthToken;
use crate::common::{ContinuationParams, YoutubeID};
use crate::continuations::ParseFromContinuable;
use crate::ProcessedResult;
use std::borrow::Cow;
use std::vec::Vec;

/// Query that will get continuations for a query that returned paginated
/// results.
pub struct GetContinuationsQuery<'a, Q> {
    query: &'a Q,
    continuation_params: ContinuationParams<'static>,
}

impl<'a, Q> GetContinuationsQuery<'a, Q> {
    /// Create a GetContinuationsQuery with dummy continuation params - for
    /// testing purposes.
    pub fn new_mock_unchecked(query: &'a Q) -> GetContinuationsQuery<'a, Q> {
        GetContinuationsQuery {
            query,
            continuation_params: ContinuationParams::from_raw(""),
        }
    }
}
impl<'a, Q> GetContinuationsQuery<'a, Q> {
    pub fn from_first_result<T: ParseFromContinuable<Q>>(
        res: ProcessedResult<'a, Q>,
    ) -> crate::Result<(T, Option<GetContinuationsQuery<'a, Q>>)> {
        let query = res.query;
        let (res, continuation_params) = T::parse_from_continuable(res)?;
        let maybe_continuation_query =
            continuation_params.map(|continuation_params| GetContinuationsQuery {
                continuation_params,
                query,
            });
        Ok((res, maybe_continuation_query))
    }
    pub fn from_continuation<'b, T: ParseFromContinuable<Q>>(
        res: ProcessedResult<'a, GetContinuationsQuery<'b, Q>>,
    ) -> crate::Result<(T, Option<GetContinuationsQuery<'b, Q>>)> {
        let query = res.query.query;
        let (res, continuation_params) = T::parse_continuation(res)?;
        let maybe_continuation_query =
            continuation_params.map(|continuation_params| GetContinuationsQuery {
                continuation_params,
                query,
            });
        Ok((res, maybe_continuation_query))
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
        let ProcessedResult {
            query,
            source,
            json,
        } = p;
        let p = ProcessedResult {
            query: query.query,
            source,
            json,
        };
        T::parse_continuation(p)
    }
}

impl<Q: Query<A>, A: AuthToken> Query<A> for GetContinuationsQuery<'_, Q>
where
    Q: PostQuery,
    Q::Output: ParseFromContinuable<Self>,
{
    type Output = Q::Output;
    type Method = PostMethod;
}

impl<Q> PostQuery for GetContinuationsQuery<'_, Q>
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
