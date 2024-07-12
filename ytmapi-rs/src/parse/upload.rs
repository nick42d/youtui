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
#[cfg(test)]
mod tests {
    use crate::{
        auth::BrowserToken,
        common::{UploadAlbumID, UploadArtistID, YoutubeID},
    };
    #[tokio::test]
    async fn test_get_library_upload_songs() {
        parse_test!(
            "./test_json/get_library_upload_songs_20240712.json",
            "./test_json/get_library_upload_songs_20240712_output.txt",
            crate::query::GetLibraryUploadSongsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_upload_albums() {
        parse_test!(
            "./test_json/get_library_upload_albums_20240712.json",
            "./test_json/get_library_upload_albums_20240712_output.txt",
            crate::query::GetLibraryUploadAlbumsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_upload_artists() {
        parse_test!(
            "./test_json/get_library_upload_artists_20240712.json",
            "./test_json/get_library_upload_artists_20240712_output.txt",
            crate::query::GetLibraryUploadArtistsQuery::default(),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_upload_artist() {
        parse_test!(
            "./test_json/get_library_upload_artists_20240712.json",
            "./test_json/get_library_upload_artists_20240712_output.txt",
            crate::query::GetLibraryUploadArtistQuery::new(UploadArtistID::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_library_upload_album() {
        parse_test!(
            "./test_json/get_library_upload_album_20240712.json",
            "./test_json/get_library_upload_album_20240712_output.txt",
            crate::query::GetLibraryUploadAlbumQuery::new(UploadAlbumID::from_raw("")),
            BrowserToken
        );
    }
}
