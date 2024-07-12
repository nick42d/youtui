use crate::query::{
    GetLibraryUploadAlbumQuery, GetLibraryUploadAlbumsQuery, GetLibraryUploadArtistQuery,
    GetLibraryUploadArtistsQuery, GetLibraryUploadSongsQuery,
};

use super::ParseFrom;

impl ParseFrom<GetLibraryUploadSongsQuery> for () {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadSongsQuery>,
    ) -> crate::Result<<GetLibraryUploadSongsQuery as crate::query::Query>::Output> {
        todo!()
    }
}
impl ParseFrom<GetLibraryUploadAlbumsQuery> for () {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadAlbumsQuery>,
    ) -> crate::Result<<GetLibraryUploadAlbumsQuery as crate::query::Query>::Output> {
        todo!()
    }
}
impl ParseFrom<GetLibraryUploadArtistsQuery> for () {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadArtistsQuery>,
    ) -> crate::Result<<GetLibraryUploadArtistsQuery as crate::query::Query>::Output> {
        todo!()
    }
}
impl<'a> ParseFrom<GetLibraryUploadAlbumQuery<'a>> for () {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadAlbumQuery>,
    ) -> crate::Result<<GetLibraryUploadAlbumQuery as crate::query::Query>::Output> {
        todo!()
    }
}
impl<'a> ParseFrom<GetLibraryUploadArtistQuery<'a>> for () {
    fn parse_from(
        p: super::ProcessedResult<GetLibraryUploadArtistQuery>,
    ) -> crate::Result<<GetLibraryUploadArtistQuery as crate::query::Query>::Output> {
        todo!()
    }
}
