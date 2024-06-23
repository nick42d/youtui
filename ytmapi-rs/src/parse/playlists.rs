use super::{ParseFrom, ProcessedResult};
use crate::{
    common::PlaylistID,
    crawler::JsonCrawler,
    nav_consts::{SECTION_LIST_ITEM, SINGLE_COLUMN_TAB},
    query::{
        AddPlaylistItemsQuery, DeletePlaylistQuery, EditPlaylistQuery, GetPlaylistQuery,
        RemovePlaylistItemsQuery,
    },
    Thumbnail,
};
use const_format::concatcp;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct GetPlaylist {
    id: PlaylistID<'static>,
    privacy: (),
    title: String,
    description: String,
    author: String,
    year: String,
    duration: String,
    track_count: usize,
    thumbnails: Vec<Thumbnail>,
    suggestions: Vec<()>,
    related: Vec<()>,
    tracks: Vec<()>,
}

impl<'a> ParseFrom<RemovePlaylistItemsQuery<'a>> for () {
    fn parse_from(
        p: ProcessedResult<RemovePlaylistItemsQuery<'a>>,
    ) -> crate::Result<<RemovePlaylistItemsQuery<'a> as crate::query::Query>::Output> {
        todo!()
    }
}
impl<'a> ParseFrom<AddPlaylistItemsQuery<'a>> for () {
    fn parse_from(
        p: ProcessedResult<AddPlaylistItemsQuery<'a>>,
    ) -> crate::Result<<AddPlaylistItemsQuery<'a> as crate::query::Query>::Output> {
        todo!()
    }
}
impl<'a> ParseFrom<EditPlaylistQuery<'a>> for () {
    fn parse_from(
        p: ProcessedResult<EditPlaylistQuery<'a>>,
    ) -> crate::Result<<EditPlaylistQuery<'a> as crate::query::Query>::Output> {
        todo!()
    }
}
impl<'a> ParseFrom<DeletePlaylistQuery<'a>> for () {
    fn parse_from(
        p: ProcessedResult<DeletePlaylistQuery<'a>>,
    ) -> crate::Result<<DeletePlaylistQuery<'a> as crate::query::Query>::Output> {
        todo!()
    }
}

impl<'a> ParseFrom<GetPlaylistQuery<'a>> for GetPlaylist {
    fn parse_from(
        p: ProcessedResult<GetPlaylistQuery<'a>>,
    ) -> crate::Result<<GetPlaylistQuery<'a> as crate::query::Query>::Output> {
        let json_crawler: JsonCrawler = p.into();
        let mut results = json_crawler.navigate_pointer(concatcp!(
            SINGLE_COLUMN_TAB,
            SECTION_LIST_ITEM,
            "/musicPlaylistShelfRenderer"
        ))?;

        let id = results.take_value_pointer("/playlistId")?;

        Ok(GetPlaylist {
            id,
            privacy: todo!(),
            title: todo!(),
            description: todo!(),
            author: todo!(),
            year: todo!(),
            duration: todo!(),
            track_count: todo!(),
            thumbnails: todo!(),
            suggestions: todo!(),
            related: todo!(),
            tracks: todo!(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        auth::BrowserToken,
        common::{PlaylistID, YoutubeID},
        query::GetPlaylistQuery,
        YtMusic,
    };
    use pretty_assertions::assert_eq;
    use std::path::Path;

    #[tokio::test]
    async fn test_get_playlist_query() {
        let source_path = Path::new("./test_json/get_playlist_20240617.json");
        let expected_path = Path::new("./test_json/get_playlist_20240617_output.txt");
        let source = tokio::fs::read_to_string(source_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = tokio::fs::read_to_string(expected_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = expected.trim();
        // Blank query has no bearing on function
        let query = GetPlaylistQuery::new(PlaylistID::from_raw(""));
        let output = YtMusic::<BrowserToken>::process_json(source, query).unwrap();
        let output = format!("{:#?}", output);
        assert_eq!(output, expected);
    }
}
