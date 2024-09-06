use super::{BasicSearch, GetQuery, PostMethod, PostQuery, Query, QueryMethod, SearchQuery};
use crate::{
    auth::{AuthToken, BrowserToken},
    parse::{self, ParseFrom, ProcessedResult},
    RawResult, Result,
};
use async_stream::{stream, try_stream};
use std::{borrow::Cow, fmt::Debug, future::Future};
use tokio_stream::Stream;

pub trait Continuable {
    fn get_continuation_params(&self) -> Option<String>;
}

impl<A: AuthToken> StreamingQuery<A> for super::GetLibraryUploadSongsQuery {}

impl Continuable for Vec<parse::TableListUploadSong> {
    fn get_continuation_params(&self) -> Option<String> {
        todo!()
    }
}

trait StreamingQuery<A: AuthToken>: Query<A>
where
    Self::Output: Continuable,
{
    async fn stream(
        &self,
        client: &crate::client::Client,
        tok: &A,
    ) -> impl Stream<Item = Result<Self::Output>> {
        try_stream! {
            let first_res: Self::Output = Self::Method::call(self, client, tok)
                .await?
                .process()?
                .parse_into()?;
            let mut maybe_next_query = GetContinuationsQuery::new(&first_res, self);
            yield first_res;
            while let Some(next_query) = maybe_next_query {
                let next = next_query
                    .call_this(client, tok)
                    .await?
                    .process()?
                    .parse_into()?;
                maybe_next_query = GetContinuationsQuery::new(&next, self);
                yield next;
            };
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

impl<'a, Q: Query<A>, A: AuthToken> Query<A> for GetContinuationsQuery<'a, Q> {
    type Output = Q::Output;
    type Method = SpecialMethod;
}

pub struct SpecialMethod;

impl super::private::Sealed for SpecialMethod {}

impl<Q, A, O> QueryMethod<Q, A, O> for SpecialMethod
where
    Q: Query<A, Output = O>,
    A: AuthToken,
{
    async fn call<'a>(
        query: &'a Q,
        client: &crate::client::Client,
        tok: &A,
    ) -> Result<RawResult<'a, Q, A>> {
        todo!()
    }
}
impl<'a, Q> GetContinuationsQuery<'a, Q> {
    pub fn new<I: ParseFrom<Q> + Continuable>(
        res: &'_ I,
        query: &'a Q,
    ) -> Option<GetContinuationsQuery<'a, Q>> {
        let continuation_params = res.get_continuation_params()?;
        Some(GetContinuationsQuery {
            continuation_params,
            query,
        })
    }
}
