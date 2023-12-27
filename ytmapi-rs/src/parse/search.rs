use super::{
    parse_item_text, Parse, ProcessedResult, SearchResult, SearchResultAlbum, SearchResultArtist,
    SearchResultSong,
};
use crate::common::{AlbumType, Explicit, SearchSuggestion, SuggestionType, TextRun, YoutubeID};
use crate::crawler::{JsonCrawler, JsonCrawlerBorrowed};
use crate::nav_consts::{
    BADGE_LABEL, NAVIGATION_BROWSE_ID, NAVIGATION_VIDEO_ID, SECTION_LIST, THUMBNAILS,
};
use crate::{query::*, ChannelID, Thumbnail, VideoID};
use crate::{Error, Result};
use const_format::concatcp;

// May be redundant due to encoding this in type system
#[derive(Debug, Clone)]
pub enum SearchResultType {
    Artist,
    Album(AlbumType), // Does albumtype matter here?
    Playlist,
    Song,
    Video,
    Station,
}
impl TryFrom<&String> for SearchResultType {
    type Error = crate::Error;
    fn try_from(value: &String) -> std::result::Result<Self, Self::Error> {
        match value.as_str() {
            // Dirty hack to get artist outputting
            "\"Artist\"" => Ok(Self::Artist),
            "artist" => Ok(Self::Artist),
            "album" => Ok(Self::Album(AlbumType::Album)),
            "ep" => Ok(Self::Album(AlbumType::EP)),
            "single" => Ok(Self::Album(AlbumType::Single)),
            "playlist" => Ok(Self::Playlist),
            "song" => Ok(Self::Song),
            "video" => Ok(Self::Video),
            "station" => Ok(Self::Station),
            // TODO: Better error
            _ => Err(Error::other(format!(
                "Unable to parse SearchResultType {value}"
            ))),
        }
    }
}

impl<'a> Parse for ProcessedResult<SearchQuery<'a, BasicSearch>> {
    type Output = Vec<super::SearchResult<'a>>;
    fn parse(self) -> Result<Self::Output> {
        let ProcessedResult {
            mut json_crawler, ..
        } = self;
        todo!();
    }
}
// TODO: Type safety
// TODO: Tests
fn parse_artist_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultArtist> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    // Will this find none and error? Note from previously.
    let artist = parse_item_text(&mut mrlir, 0, 0)?;
    let subscribers = parse_item_text(&mut mrlir, 1, 2)?;
    let browse_id = mrlir.take_value_pointer(NAVIGATION_BROWSE_ID).ok();
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultArtist {
        artist,
        subscribers,
        thumbnails,
        browse_id,
    })
}
// TODO: Type safety
// TODO: Tests
fn parse_album_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultAlbum> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    // Will this find none and error? Note from previously.
    let artist = parse_item_text(&mut mrlir, 0, 0)?;
    let album_type = parse_item_text(&mut mrlir, 1, 0).and_then(|a| AlbumType::try_from_str(a))?;
    let title = parse_item_text(&mut mrlir, 1, 2)?;
    let year = parse_item_text(&mut mrlir, 1, 4)?;
    let explicit = if mrlir.path_exists(BADGE_LABEL) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    let browse_id = mrlir.take_value_pointer(NAVIGATION_BROWSE_ID).ok();
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultAlbum {
        artist,
        thumbnails,
        browse_id,
        title,
        year,
        album_type,
        explicit,
    })
}
// TODO: Type safety
// TODO: Tests
fn parse_song_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultSong> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    // Will this find none and error? Note from previously.
    let title = parse_item_text(&mut mrlir, 0, 0)?;
    let artist = parse_item_text(&mut mrlir, 1, 0)?;
    let album = parse_item_text(&mut mrlir, 1, 2)?;
    let duration = parse_item_text(&mut mrlir, 1, 4)?;
    let plays = parse_item_text(&mut mrlir, 2, 0)?;
    let explicit = if mrlir.path_exists(BADGE_LABEL) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    let video_id = mrlir.take_value_pointer(NAVIGATION_VIDEO_ID).ok();
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultSong {
        artist,
        thumbnails,
        title,
        explicit,
        plays,
        album,
        video_id,
        duration,
    })
}
// TODO: Rename FilteredSearchSectionContents
struct SectionContentsCrawler(JsonCrawler);
// In this case, we've searched and had no results found.
// We are being quite explicit here to avoid a false positive.
// See tests for an example.
// TODO: Test this function.
fn section_contents_is_empty(section_contents: &SectionContentsCrawler) -> bool {
    section_contents
        .0
        .path_exists("/itemSectionRenderer/contents/0/didYouMeanRenderer")
}
impl<'a, F: FilteredSearchType> TryFrom<ProcessedResult<SearchQuery<'a, FilteredSearch<F>>>>
    for SectionContentsCrawler
{
    type Error = Error;
    fn try_from(value: ProcessedResult<SearchQuery<'a, FilteredSearch<F>>>) -> Result<Self> {
        let ProcessedResult {
            mut json_crawler, ..
        } = value;
        let section_contents = json_crawler.navigate_pointer(concatcp!(
            "/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content",
            SECTION_LIST,
            "/0"
        ))?;
        Ok(SectionContentsCrawler(section_contents))
    }
}
// XXX: Should this also contain query type?
struct FilteredSearchMSRContents(JsonCrawler);
impl TryFrom<SectionContentsCrawler> for FilteredSearchMSRContents {
    type Error = Error;
    fn try_from(value: SectionContentsCrawler) -> std::prelude::v1::Result<Self, Self::Error> {
        Ok(FilteredSearchMSRContents(
            value.0.navigate_pointer("/musicShelfRenderer/contents")?,
        ))
    }
}
impl TryFrom<FilteredSearchMSRContents> for Vec<SearchResultAlbum> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMSRContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .as_array_iter_mut()?
            .map(|a| parse_album_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMSRContents> for Vec<SearchResultArtist> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMSRContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .as_array_iter_mut()?
            .map(|a| parse_artist_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMSRContents> for Vec<SearchResultSong> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMSRContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .as_array_iter_mut()?
            .map(|a| parse_song_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<ArtistsFilter>>> {
    type Output = Vec<SearchResultArtist>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<AlbumsFilter>>> {
    type Output = Vec<SearchResultAlbum>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<SongsFilter>>> {
    type Output = Vec<SearchResultSong>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        FilteredSearchMSRContents::try_from(section_contents)?.try_into()
    }
}

impl<'a> Parse for ProcessedResult<GetSearchSuggestionsQuery<'a>> {
    type Output = Vec<SearchSuggestion>;
    fn parse(self) -> Result<Self::Output> {
        let ProcessedResult { json_crawler, .. } = self;
        let mut suggestions = json_crawler
            .navigate_pointer("/contents/0/searchSuggestionsSectionRenderer/contents")?;
        let mut results = Vec::new();
        for mut s in suggestions.as_array_iter_mut()? {
            let mut runs = Vec::new();
            if let Ok(search_suggestion) =
                s.borrow_pointer("/searchSuggestionRenderer/suggestion/runs")
            {
                for mut r in search_suggestion.into_array_iter_mut()? {
                    if let Ok(true) = r.take_value_pointer("/bold") {
                        runs.push(r.take_value_pointer("/text").map(|s| TextRun::Bold(s))?)
                    } else {
                        runs.push(r.take_value_pointer("/text").map(|s| TextRun::Normal(s))?)
                    }
                }
                results.push(SearchSuggestion::new(SuggestionType::Prediction, runs))
            } else {
                for mut r in s
                    .borrow_pointer("/historySuggestionRenderer/suggestion/runs")?
                    .into_array_iter_mut()?
                {
                    if let Ok(true) = r.take_value_pointer("/bold") {
                        runs.push(r.take_value_pointer("/text").map(|s| TextRun::Bold(s))?)
                    } else {
                        runs.push(r.take_value_pointer("/text").map(|s| TextRun::Normal(s))?)
                    }
                }
                results.push(SearchSuggestion::new(SuggestionType::History, runs))
            }
        }
        Ok(results)
    }
}

// Continuation functions for future use
fn get_continuations(res: &SearchResult) {}

fn get_reloadable_continuation_params(json: &mut JsonCrawlerBorrowed) -> Result<String> {
    let ctoken = json.take_value_pointer("/continuations/0/reloadContinuationData/continuation")?;
    Ok(get_continuation_string(ctoken))
}

fn get_continuation_params(
    json: &mut JsonCrawlerBorrowed,
    ctoken_path: Option<&str>,
) -> Result<String> {
    let ctoken = if let Some(ctoken_path) = ctoken_path {
        let key = format!("/continuations/0/next{ctoken_path}/ContinuationData/continuation");
        json.take_value_pointer(key)?
    } else {
        json.take_value_pointer("/continuations/0/next/ContinuationData/continuation")?
    };
    Ok(get_continuation_string(ctoken))
}

fn get_continuation_string(ctoken: String) -> String {
    format!("&ctoken={0}&continuation={0}", ctoken)
}

#[cfg(test)]
mod tests {
    use chrono::expect;

    use crate::{
        crawler::JsonCrawler,
        parse::{Parse, ProcessedResult},
        process::JsonCloner,
        query::{AlbumsFilter, ArtistsFilter, SearchQuery, SongsFilter},
    };
    use std::path::Path;

    #[tokio::test]
    async fn test_search_artists() {
        let source_path = Path::new("./test_json/search_artists_20231226.json");
        let expected_path = Path::new("./test_json/search_artists_20231226_output.txt");
        let source = tokio::fs::read_to_string(source_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = tokio::fs::read_to_string(expected_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = expected.trim();
        let json_clone = JsonCloner::from_string(source).unwrap();
        // Blank query has no bearing on function
        let query = SearchQuery::new("").with_filter(ArtistsFilter);
        let output = ProcessedResult::from_raw(JsonCrawler::from_json_cloner(json_clone), query)
            .parse()
            .unwrap();
        let output = format!("{:#?}", output);
        assert_eq!(output, expected);
    }
    #[tokio::test]
    async fn test_search_artists_empty() {
        let source_path = Path::new("./test_json/search_artists_no_results_20231226.json");
        let source = tokio::fs::read_to_string(source_path)
            .await
            .expect("Expect file read to pass during tests");
        let json_clone = JsonCloner::from_string(source).unwrap();
        // Blank query has no bearing on function
        let query = SearchQuery::new("").with_filter(ArtistsFilter);
        let output = ProcessedResult::from_raw(JsonCrawler::from_json_cloner(json_clone), query)
            .parse()
            .unwrap();
        assert_eq!(output, Vec::new());
    }
    #[tokio::test]
    async fn test_search_albums() {
        let source_path = Path::new("./test_json/search_albums_20231226.json");
        let expected_path = Path::new("./test_json/search_albums_20231226_output.txt");
        let source = tokio::fs::read_to_string(source_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = tokio::fs::read_to_string(expected_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = expected.trim();
        let json_clone = JsonCloner::from_string(source).unwrap();
        // Blank query has no bearing on function
        let query = SearchQuery::new("").with_filter(AlbumsFilter);
        let output = ProcessedResult::from_raw(JsonCrawler::from_json_cloner(json_clone), query)
            .parse()
            .unwrap();
        let output = format!("{:#?}", output);
        assert_eq!(output, expected);
    }
    #[tokio::test]
    async fn test_search_songs() {
        let source_path = Path::new("./test_json/search_songs_20231226.json");
        let expected_path = Path::new("./test_json/search_songs_20231226_output.txt");
        let source = tokio::fs::read_to_string(source_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = tokio::fs::read_to_string(expected_path)
            .await
            .expect("Expect file read to pass during tests");
        let expected = expected.trim();
        let json_clone = JsonCloner::from_string(source).unwrap();
        // Blank query has no bearing on function
        let query = SearchQuery::new("").with_filter(SongsFilter);
        let output = ProcessedResult::from_raw(JsonCrawler::from_json_cloner(json_clone), query)
            .parse()
            .unwrap();
        let output = format!("{:#?}", output);
        assert_eq!(output, expected);
    }
}
