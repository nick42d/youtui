use super::{
    ParseFrom, CATEGORY_TITLE, GRID, RUN_TEXT, TASTE_ITEM_CONTENTS, TASTE_PROFILE_ARTIST,
    TASTE_PROFILE_IMPRESSION, TASTE_PROFILE_ITEMS, TASTE_PROFILE_SELECTION,
};
use crate::{
    common::{MoodCategoryParams, PlaylistID, TasteToken, Thumbnail},
    nav_consts::{
        CAROUSEL, CAROUSEL_TITLE, CATEGORY_PARAMS, MTRIR, NAVIGATION_BROWSE_ID, SECTION_LIST,
        SINGLE_COLUMN_TAB, SUBTITLE_RUNS, THUMBNAIL_RENDERER, TITLE_TEXT,
    },
    query::{
        GetMoodCategoriesQuery, GetMoodPlaylistsQuery, GetTasteProfileQuery, SetTasteProfileQuery,
    },
    Result,
};
use const_format::concatcp;
use itertools::Itertools;
use json_crawler::{
    CrawlerError, CrawlerResult, JsonCrawler, JsonCrawlerBorrowed, JsonCrawlerIterator,
    JsonCrawlerOwned,
};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct TasteProfileArtist {
    pub artist: String,
    pub taste_tokens: TasteToken<'static>,
}
#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct MoodCategorySection {
    pub section_name: String,
    pub mood_categories: Vec<MoodCategory>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct MoodCategory {
    pub title: String,
    pub params: MoodCategoryParams<'static>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct MoodPlaylistCategory {
    pub category_name: String,
    pub playlists: Vec<MoodPlaylist>,
}

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct MoodPlaylist {
    pub playlist_id: PlaylistID<'static>,
    pub title: String,
    pub thumbnails: Vec<Thumbnail>,
    pub author: String,
}

impl<'a, I> ParseFrom<SetTasteProfileQuery<'a, I>> for ()
where
    I: Iterator<Item = TasteToken<'a>> + Clone,
{
    fn parse_from(_: super::ProcessedResult<SetTasteProfileQuery<'a, I>>) -> Result<Self> {
        // Doesn't seem to be an identifier in the response to determine if success or
        // failure - so always assume success.
        Ok(())
    }
}

impl ParseFrom<GetTasteProfileQuery> for Vec<TasteProfileArtist> {
    fn parse_from(p: super::ProcessedResult<GetTasteProfileQuery>) -> Result<Self> {
        let crawler = JsonCrawlerOwned::from(p);
        // TODO: Neaten this
        crawler
            .navigate_pointer(TASTE_PROFILE_ITEMS)?
            .try_into_iter()?
            .map(|item| -> Result<_> {
                Ok(item
                    .navigate_pointer(TASTE_ITEM_CONTENTS)?
                    .try_into_iter()?
                    .map(get_taste_profile_artist))
            })
            .process_results(|iter| iter.flatten().collect::<Result<_>>())?
    }
}

impl ParseFrom<GetMoodCategoriesQuery> for Vec<MoodCategorySection> {
    fn parse_from(p: super::ProcessedResult<GetMoodCategoriesQuery>) -> crate::Result<Self> {
        let crawler = JsonCrawlerOwned::from(p);
        crawler
            .navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST))?
            .try_into_iter()?
            .map(parse_mood_category_sections)
            .collect()
    }
}
impl<'a> ParseFrom<GetMoodPlaylistsQuery<'a>> for Vec<MoodPlaylistCategory> {
    fn parse_from(p: super::ProcessedResult<GetMoodPlaylistsQuery<'a>>) -> Result<Self> {
        fn parse_mood_playlist_category(
            mut crawler: JsonCrawlerOwned,
        ) -> Result<MoodPlaylistCategory> {
            let array = vec![
                |s: &mut JsonCrawlerOwned| -> std::result::Result<_, json_crawler::CrawlerError> {
                    parse_mood_playlist_category_grid(s.borrow_pointer(GRID)?)
                },
                |s: &mut JsonCrawlerOwned| -> std::result::Result<_, json_crawler::CrawlerError> {
                    parse_mood_playlist_category_carousel(s.borrow_pointer(CAROUSEL)?)
                },
            ];
            crawler.try_functions(array).map_err(Into::into)
        }
        fn parse_mood_playlist_category_grid(
            mut crawler: JsonCrawlerBorrowed,
        ) -> json_crawler::CrawlerResult<MoodPlaylistCategory> {
            let category_name =
                crawler.take_value_pointer(concatcp!("/header/gridHeaderRenderer", TITLE_TEXT))?;
            let playlists = crawler
                .navigate_pointer("/items")?
                .try_iter_mut()?
                .map(parse_mood_playlist)
                .collect::<CrawlerResult<_>>()?;
            Ok(MoodPlaylistCategory {
                category_name,
                playlists,
            })
        }
        fn parse_mood_playlist_category_carousel(
            mut crawler: JsonCrawlerBorrowed,
        ) -> json_crawler::CrawlerResult<MoodPlaylistCategory> {
            let category_name = crawler.take_value_pointer(concatcp!(CAROUSEL_TITLE, "/text"))?;
            let playlists = crawler
                .navigate_pointer("/contents")?
                .try_iter_mut()?
                .map(parse_mood_playlist)
                .collect::<CrawlerResult<_>>()?;
            Ok(MoodPlaylistCategory {
                category_name,
                playlists,
            })
        }
        fn parse_mood_playlist(
            crawler: JsonCrawlerBorrowed,
        ) -> json_crawler::CrawlerResult<MoodPlaylist> {
            let mut item = crawler.navigate_pointer(MTRIR)?;
            let playlist_id = item.take_value_pointer(NAVIGATION_BROWSE_ID)?;
            let title = item.take_value_pointer(TITLE_TEXT)?;
            let thumbnails = item.take_value_pointer(THUMBNAIL_RENDERER)?;
            let subtitle_runs_iter = item.borrow_pointer(SUBTITLE_RUNS)?.try_into_iter()?;
            let subtitle_runs_iter_context = subtitle_runs_iter.get_context();
            let author = subtitle_runs_iter
                .take(3)
                .last()
                .map(|mut run| run.take_value_pointer("/text"))
                .ok_or_else(|| {
                    CrawlerError::array_size_from_context(subtitle_runs_iter_context, 1)
                })??;
            Ok(MoodPlaylist {
                playlist_id,
                title,
                thumbnails,
                author,
            })
        }
        let json_crawler: JsonCrawlerOwned = p.into();
        json_crawler
            .navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST))?
            .try_into_iter()?
            .map(parse_mood_playlist_category)
            .collect()
    }
}

fn parse_mood_category_sections(crawler: JsonCrawlerOwned) -> Result<MoodCategorySection> {
    let mut crawler = crawler.navigate_pointer(GRID)?;
    let section_name =
        crawler.take_value_pointer(concatcp!("/header/gridHeaderRenderer/title", RUN_TEXT))?;
    let mood_categories = crawler
        .navigate_pointer("/items")?
        .try_into_iter()?
        .map(parse_mood_categories)
        .collect::<Result<Vec<_>>>()?;
    Ok(MoodCategorySection {
        section_name,
        mood_categories,
    })
}
fn parse_mood_categories(crawler: JsonCrawlerOwned) -> Result<MoodCategory> {
    let mut crawler = crawler.navigate_pointer("/musicNavigationButtonRenderer")?;
    let title = crawler.take_value_pointer(concatcp!(CATEGORY_TITLE))?;
    let params = crawler.take_value_pointer(concatcp!(CATEGORY_PARAMS))?;
    Ok(MoodCategory { title, params })
}

fn get_taste_profile_artist(mut crawler: JsonCrawlerOwned) -> Result<TasteProfileArtist> {
    let artist = crawler.take_value_pointer(TASTE_PROFILE_ARTIST)?;
    let impression_value = crawler.take_value_pointer(TASTE_PROFILE_IMPRESSION)?;
    let selection_value = crawler.take_value_pointer(TASTE_PROFILE_SELECTION)?;
    let taste_tokens = TasteToken {
        impression_value,
        selection_value,
    };
    Ok(TasteProfileArtist {
        artist,
        taste_tokens,
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        auth::BrowserToken,
        common::{
            MoodCategoryParams, TasteToken, TasteTokenImpression, TasteTokenSelection, YoutubeID,
        },
        query::{
            GetMoodCategoriesQuery, GetMoodPlaylistsQuery, GetTasteProfileQuery,
            SetTasteProfileQuery,
        },
    };

    #[tokio::test]
    async fn test_get_mood_categories() {
        parse_test!(
            "./test_json/get_mood_categories_20240723.json",
            "./test_json/get_mood_categories_20240723_output.txt",
            GetMoodCategoriesQuery,
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_mood_playlists() {
        parse_test!(
            "./test_json/get_mood_playlists_20240723.json",
            "./test_json/get_mood_playlists_20240723_output.txt",
            GetMoodPlaylistsQuery::new(MoodCategoryParams::from_raw("")),
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_taste_profile() {
        parse_test!(
            "./test_json/get_taste_profile_20240722.json",
            "./test_json/get_taste_profile_20240722_output.txt",
            GetTasteProfileQuery,
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_set_taste_profile() {
        parse_test_value!(
            "./test_json/set_taste_profile_20240723.json",
            (),
            SetTasteProfileQuery::new([TasteToken {
                impression_value: TasteTokenImpression::from_raw(""),
                selection_value: TasteTokenSelection::from_raw("")
            }]),
            BrowserToken
        );
    }
}
