use super::ParseFrom;
use crate::{common::ContinuationParams, query::GetContinuationsQuery, ProcessedResult, Result};
use std::fmt::Debug;

/// This trait represents a result that can be streamed to get more results.
/// It will contain continuation params, and a parsing function for its
/// continuations.
// TODO: Implementation example, and consider moving to its own module.
pub trait Continuable<Q>: Sized {
    fn take_continuation_params(&mut self) -> Option<ContinuationParams<'static>>;
    fn parse_continuation(p: ProcessedResult<GetContinuationsQuery<'_, Q>>) -> Result<Self>;
}

// Implementing Continuable<Q> for T implies ParseFrom<GetContinuationsQuery<Q>
// for T.
impl<'a, T, Q> ParseFrom<GetContinuationsQuery<'a, Q>> for T
where
    T: Continuable<Q>,
    T: Debug,
{
    fn parse_from(p: ProcessedResult<GetContinuationsQuery<'a, Q>>) -> Result<Self> {
        T::parse_continuation(p)
    }
}
