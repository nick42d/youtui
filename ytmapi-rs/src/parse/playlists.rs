use super::{ParseFrom, ProcessedResult};
use crate::{
    common::PlaylistID,
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
    async fn parse_from<A: crate::auth::AuthToken>(
        q: RemovePlaylistItemsQuery<'a>,
        yt: &crate::YtMusic<A>,
    ) -> crate::Result<<RemovePlaylistItemsQuery<'a> as crate::query::Query>::Output> {
        todo!()
    }
}
impl<'a> ParseFrom<AddPlaylistItemsQuery<'a>> for () {
    async fn parse_from<A: crate::auth::AuthToken>(
        q: AddPlaylistItemsQuery<'a>,
        yt: &crate::YtMusic<A>,
    ) -> crate::Result<<AddPlaylistItemsQuery<'a> as crate::query::Query>::Output> {
        todo!()
    }
}
impl<'a> ParseFrom<EditPlaylistQuery<'a>> for () {
    async fn parse_from<A: crate::auth::AuthToken>(
        q: EditPlaylistQuery<'a>,
        yt: &crate::YtMusic<A>,
    ) -> crate::Result<<EditPlaylistQuery<'a> as crate::query::Query>::Output> {
        todo!()
    }
}
impl<'a> ParseFrom<DeletePlaylistQuery<'a>> for () {
    async fn parse_from<A: crate::auth::AuthToken>(
        q: DeletePlaylistQuery<'a>,
        yt: &crate::YtMusic<A>,
    ) -> crate::Result<<DeletePlaylistQuery<'a> as crate::query::Query>::Output> {
        todo!()
    }
}

impl<'a> ParseFrom<GetPlaylistQuery<'a>> for GetPlaylist {
    async fn parse_from<A: crate::auth::AuthToken>(
        q: GetPlaylistQuery<'a>,
        yt: &crate::YtMusic<A>,
    ) -> crate::Result<<GetPlaylistQuery<'a> as crate::query::Query>::Output> {
        let ProcessedResult { json_crawler, .. } = yt.processed_query(q).await?;
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
    use std::path::Path;

    use crate::{
        common::{browsing::Lyrics, LyricsID, PlaylistID, YoutubeID},
        crawler::JsonCrawler,
        parse::ProcessedResult,
        process::JsonCloner,
        query::{lyrics::GetLyricsQuery, GetPlaylistQuery},
    };
    use pretty_assertions::assert_eq;

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
        let json_clone = JsonCloner::from_string(source).unwrap();
        // Blank query has no bearing on function
        let query = GetPlaylistQuery::new(PlaylistID::from_raw(""));
        let output = ProcessedResult::from_raw(JsonCrawler::from_json_cloner(json_clone), query)
            .parse()
            .unwrap();
        let output = format!("{:#?}", output);
        assert_eq!(output, expected);
    }
}
