use super::{PostMethod, PostQuery, Query};
use crate::{
    auth::AuthToken,
    common::{ContinuationParams, YoutubeID},
    continuations::Continuable,
    parse::ParseFrom,
};
use std::{borrow::Cow, vec::Vec};

pub struct GetContinuationsQuery<'a, Q> {
    query: &'a Q,
    continuation_params: ContinuationParams<'static>,
}

impl<'a, Q> GetContinuationsQuery<'a, Q> {
    pub fn new<T: Continuable<Q>>(
        res: &'_ mut T,
        query: &'a Q,
    ) -> Option<GetContinuationsQuery<'a, Q>> {
        let continuation_params = res.take_continuation_params()?;
        Some(GetContinuationsQuery {
            continuation_params,
            query,
        })
    }
    /// Create a GetContinuationsQuery with dummy continuation params - for
    /// testing purposes.
    pub fn new_mock_unchecked(query: &'a Q) -> GetContinuationsQuery<'a, Q> {
        GetContinuationsQuery {
            query,
            continuation_params: ContinuationParams::from_raw(""),
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
