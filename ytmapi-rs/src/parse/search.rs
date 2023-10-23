use const_format::concatcp;

use super::{parse_search_result, parse_search_results, ProcessedResult, SearchResult};
use crate::common::{AlbumType, TextRun};
use crate::crawler::JsonCrawlerBorrowed;
use crate::nav_consts::SECTION_LIST;
use crate::query::*;
use crate::{Error, Result};

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
impl<'a, S: SearchType> ProcessedResult<SearchQuery<'a, S>> {
    // TODO: Take the search suggestions param.
    // TODO: Handle errors
    pub fn parse(self) -> Result<Vec<super::SearchResult<'a>>> {
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

impl<'a> ProcessedResult<GetSearchSuggestionsQuery<'a>> {
    pub fn parse(self) -> Result<Vec<Vec<TextRun>>> {
        let ProcessedResult { json_crawler, .. } = self;
        let mut suggestions = json_crawler
            .navigate_pointer("/contents/0/searchSuggestionsSectionRenderer/contents")?;
        let mut results = Vec::new();
        for s in suggestions.as_array_iter_mut()? {
            let mut result = Vec::new();
            for mut r in s
                .navigate_pointer("/searchSuggestionRenderer/suggestion/runs")?
                .into_array_iter_mut()?
            {
                if let Ok(true) = r.take_value_pointer("/bold") {
                    result.push(r.take_value_pointer("/text").map(|s| TextRun::Bold(s))?)
                } else {
                    result.push(r.take_value_pointer("/text").map(|s| TextRun::Normal(s))?)
                }
            }
            results.push(result)
        }
        Ok(results)
    }
}

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
