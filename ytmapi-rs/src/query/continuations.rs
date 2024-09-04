use super::{BasicSearch, GetQuery, PostMethod, PostQuery, Query, QueryMethod, SearchQuery};
use crate::{
    auth::{AuthToken, BrowserToken},
    parse::{ParseFrom, ProcessedResult},
    Result,
};
use async_stream::{stream, try_stream};
use std::{borrow::Cow, fmt::Debug};
use tokio_stream::Stream;

trait Continuable {
    fn get_continuation_params(&self) -> Option<String>;
}
// trait StreamingQuery<A: AuthToken>: Query<A>
// where
//     Self::Output: Continuable,
// {
//     fn stream(
//         &self,
//         client: &crate::client::Client,
//         tok: &A,
//     ) -> impl Stream<Item = Result<Self::Output>> {
//         try_stream! {
//             let first_res: Self::Output = Self::Method::call(self, client,
// tok).await?.process()?.parse_into()?;             let first_cont_pars =
// Continuable::get_continuation_params(&first_res);             yield
// first_res;             if let Some(first_cont_pars) = first_cont_pars {
//                 let query = GetContinuationsQuery {
//                     continuation_params: first_cont_pars,
//                     query: self
//                 };
//                 let next = <GetContinuationsQuery<'_, _> as
// Query<A>>::Method::call(&query, client, tok).await?.process()?.parse_into()?;
//             }
//         }
//     }
// }

fn stream<'a, Q: Query<BrowserToken>>(
    query: Q,
    client: &crate::client::Client,
    tok: &BrowserToken,
) -> impl Stream<Item = Result<Q::Output>>
where
    Q::Output: Continuable,
{
    try_stream! {
        let first_res: Q::Output = Q::Method::call(&query, client, tok).await?.process()?.parse_into()?;
        let first_cont_pars = first_res.get_continuation_params();
        yield first_res;
        if let Some(first_cont_pars) = first_cont_pars {
            let query = GetContinuationsQuery {
                continuation_params: first_cont_pars,
                query: &query
            };
            let next = <GetContinuationsQuery<'_, _> as Query<BrowserToken>>::Method::call(&query, client, tok).await?.process()?.parse_into()?;
        }
    }
}

impl<'a, Q, T: Debug> ParseFrom<GetContinuationsQuery<'a, Q>> for T {
    fn parse_from(p: ProcessedResult<GetContinuationsQuery<'a, Q>>) -> crate::Result<Self> {
        todo!()
    }
}

pub struct GetContinuationsQuery<'a, Q> {
    continuation_params: String,
    query: &'a Q,
}
// TODO: Output type
impl<'a, Q: Query<A>, A: AuthToken> Query<A> for GetContinuationsQuery<'a, Q>
// where
//     Q::Output: ParseFrom<Self>,
//     Q::Method: QueryMethod<Self, A, <Q as Query<A>>::Output>,
{
    type Output = Q::Output;
    type Method = Q::Method;
}
// impl<'a> PostQuery for GetContinuationsQuery<SearchQuery<'a, BasicSearch>>
// where
//     SearchQuery<'a, BasicSearch>: PostQuery,
// {
//     fn header(&self) -> serde_json::Map<String, serde_json::Value> {
//         self.query.header()
//     }
//     fn path(&self) -> &str {
//         self.query.path()
//     }
//     fn params(&self) -> Option<Cow<str>> {
//         Some(Cow::Borrowed(&self.continuation_params))
//     }
// }
impl<'a, Q> GetContinuationsQuery<'a, Q> {
    pub fn new(c_params: String, query: &'a Q) -> GetContinuationsQuery<'a, Q> {
        GetContinuationsQuery {
            continuation_params: c_params,
            query,
        }
    }
}
