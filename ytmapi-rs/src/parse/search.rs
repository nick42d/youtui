use super::{
    parse_item_text, parse_search_result, parse_search_results, parse_thumbnails, Parse,
    ProcessedResult, SearchResult, SearchResultAlbum, SearchResultArtist,
};
use crate::common::{AlbumType, Explicit, SearchSuggestion, SuggestionType, TextRun, YoutubeID};
use crate::crawler::{JsonCrawler, JsonCrawlerBorrowed};
use crate::nav_consts::{BADGE_LABEL, NAVIGATION_BROWSE_ID, SECTION_LIST, THUMBNAILS};
use crate::{query::*, ChannelID};
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
        //TODO top_result
        //        let mut top_result = None;
        // May receive a tabbedSearchResultRenderer
        // TODO: tab index depends on scope or filter (currently just 0 in the pointer)
        let result = if let Ok(r) = json_crawler.borrow_pointer(concatcp!(
            "/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content",
            SECTION_LIST
        )) {
            r
        } else {
            json_crawler.borrow_pointer("/contents")?
        };
        //        if let Some(r) = json_data
        //            .get("contents")
        //            .expect("json format is known")
        //            .get("tabbedSearchResultsRenderer")
        //        {
        //            // TODO: tab index depends on scope or filter
        //            let tab_index = 0;
        //            result = &r["tabs"][tab_index]["tabRenderer"]["content"];
        //        } else {
        //            result = json_data.get_mut("contents").expect("json format is known");
        //        }
        //        let result =
        //            nav_utils::nav(result, &nav_utils::SECTION_LIST).expect("json format is known");
        // TODO: Return early if no results.
        // TODO: guard against not being an array.
        // let mut new_result = serde_json::Value::Null;
        // for mut r in result.into_array_iter_mut()? {
        //     // Sometimes is mcs, sometimes ms.
        //     if let Ok(mut mcs) = r.borrow_pointer("/musicCardShelfRenderer") {
        //         // TODO: Categories, more from youtube is missing sometimes.
        //         new_result = mcs._take_json_pointer("/contents")?;
        //         // TODO return top_result
        //         let _top_result = Some(super::parse_top_result(mcs));
        //     } else if let Ok(mut ms) = r.borrow_pointer("/musicShelfRenderer") {
        //         // TODO: Categories
        //         new_result = ms._take_json_pointer("/contents")?;
        //     } else {
        //         continue;
        //     }
        // }

        // New_result may exist in musicCardShelfRenderer or musicShelfRendererer.
        // If either of the renderers exist then contents must exist. Else error.
        // Note - not yet doing top_result - see code above.
        // XXX: Should break if contensts not found
        let search_results = result
            .into_array_iter_mut()?
            .find_map(|mut a| {
                if let Ok(ab) = a.borrow_pointer("/musicCardShelfRenderer") {
                    Ok(ab
                        .navigate_pointer("/contents")
                        .and_then(|ab| parse_search_results(ab)))
                } else {
                    a.borrow_pointer("/musicShelfRenderer").map(|a| {
                        a.navigate_pointer("/contents")
                            .and_then(|a| parse_search_results(a))
                    })
                }
                .ok()
            })
            .transpose()?
            .into_iter()
            .flatten()
            .collect();
        Ok(search_results)
    }
}
// TODO: Type safety
fn parse_artist_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultArtist> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    // Will this find none and error? Note from previously.
    let artist = parse_item_text(&mut mrlir, 0, 0)?;
    let browse_id = mrlir
        .take_value_pointer::<String, &str>(NAVIGATION_BROWSE_ID)
        .map(|s| ChannelID::from_raw(s))
        .ok();
    let thumbnails = mrlir
        .navigate_pointer(THUMBNAILS)
        .and_then(|mut t| parse_thumbnails(&mut t))?;
    Ok(SearchResultArtist {
        artist,
        thumbnails,
        browse_id,
    })
}
// TODO: Type safety
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
    let browse_id = mrlir
        .take_value_pointer::<String, &str>(NAVIGATION_BROWSE_ID)
        .map(|s| ChannelID::from_raw(s))
        .ok();
    let thumbnails = mrlir
        .navigate_pointer(THUMBNAILS)
        .and_then(|mut t| parse_thumbnails(&mut t))?;
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
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<ArtistsFilter>>> {
    type Output = Vec<SearchResultArtist>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        // TODO: Make this a From method.
        section_contents
            .0
            .navigate_pointer("/musicShelfRenderer/contents")?
            .as_array_iter_mut()?
            .map(|a| parse_artist_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl<'a> Parse for ProcessedResult<SearchQuery<'a, FilteredSearch<AlbumsFilter>>> {
    type Output = Vec<SearchResultAlbum>;
    fn parse(self) -> Result<Self::Output> {
        let section_contents = SectionContentsCrawler::try_from(self)?;
        if section_contents_is_empty(&section_contents) {
            return Ok(Vec::new());
        }
        // TODO: Make this a From method.
        section_contents
            .0
            .navigate_pointer("/musicShelfRenderer/contents")?
            .as_array_iter_mut()?
            .map(|a| parse_album_search_result_from_music_shelf_contents(a))
            .collect()
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
        query::{AlbumsFilter, ArtistsFilter, SearchQuery},
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
}
