pub use album::*;
pub use artist::*;
mod artist;
pub use search::*;
use std::borrow::Cow;
mod search;
use crate::common::BrowseID;

pub trait Query {
    // XXX: Consider if this should just return a tuple, Header seems overkill.
    // e.g fn header(&self) -> (Cow<str>, Cow<str>);
    fn header(&self) -> Header;
    fn params(&self) -> Option<Cow<str>>;
    fn path(&self) -> &str;
}

// Does this really need to be a struct? Could be a tuple?
pub struct Header<'a> {
    pub key: Cow<'a, str>,
    pub value: Cow<'a, str>,
}

pub mod album {
    use crate::common::{AlbumID, YoutubeID};

    use super::{BrowseID, Header, Query};
    use std::borrow::Cow;

    pub struct GetAlbumQuery<'a> {
        browse_id: AlbumID<'a>,
    }
    impl<'a> Query for GetAlbumQuery<'a> {
        fn header(&self) -> Header {
            Header {
                key: "browseId".into(),
                value: Cow::Borrowed(self.browse_id.get_raw()),
            }
        }
        fn path(&self) -> &str {
            "browse"
        }
        fn params(&self) -> Option<Cow<str>> {
            None
        }
    }
    impl<'a> GetAlbumQuery<'_> {
        pub fn new(browse_id: AlbumID<'a>) -> GetAlbumQuery<'a> {
            GetAlbumQuery { browse_id }
        }
    }
}

pub mod continuations {
    use std::borrow::Cow;

    use crate::common::AlbumID;

    use super::{FilteredSearch, Header, Query, SearchQuery};

    pub struct GetContinuationsQuery<Q: Query> {
        c_params: String,
        query: Q,
    }
    impl<'a> Query for GetContinuationsQuery<SearchQuery<'a, FilteredSearch>> {
        fn header(&self) -> Header {
            self.query.header()
        }
        fn path(&self) -> &str {
            self.query.path()
        }
        fn params(&self) -> Option<Cow<str>> {
            Some(Cow::Borrowed(&self.c_params))
        }
    }
    impl<Q: Query> GetContinuationsQuery<Q> {
        pub fn new(c_params: String, query: Q) -> GetContinuationsQuery<Q> {
            GetContinuationsQuery { c_params, query }
        }
    }
}
