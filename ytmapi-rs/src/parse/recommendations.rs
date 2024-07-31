use std::sync::Arc;

use super::{
    ParseFrom, CATEGORY_TITLE, GRID, RUN_TEXT, TASTE_ITEM_CONTENTS, TASTE_PROFILE_ARTIST,
    TASTE_PROFILE_IMPRESSION, TASTE_PROFILE_ITEMS, TASTE_PROFILE_SELECTION,
};
use crate::{
    common::{recomendations::TasteToken, MoodCategoryParams, PlaylistID},
    crawler::{JsonCrawler, JsonCrawlerBorrowed},
    nav_consts::{
        CAROUSEL, CAROUSEL_TITLE, CATEGORY_PARAMS, MTRIR, NAVIGATION_BROWSE_ID, SECTION_LIST,
        SINGLE_COLUMN_TAB, SUBTITLE_RUNS, THUMBNAIL_RENDERER, TITLE_TEXT,
    },
    query::{
        GetMoodCategoriesQuery, GetMoodPlaylistsQuery, GetTasteProfileQuery, SetTasteProfileQuery,
    },
    utils, Error, Result, Thumbnail,
};
use const_format::concatcp;
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
        let crawler = JsonCrawler::from(p);
        // TODO: Neaten this
        let nested_iter = crawler
            .navigate_pointer(TASTE_PROFILE_ITEMS)?
            .into_array_into_iter()?
            .map(|item| -> Result<_> {
                Ok(item
                    .navigate_pointer(TASTE_ITEM_CONTENTS)?
                    .into_array_into_iter()?
                    .map(get_taste_profile_artist))
            });
        utils::process_results::process_results(nested_iter, |i| {
            i.flatten().collect::<Result<Vec<TasteProfileArtist>>>()
        })?
    }
}

impl ParseFrom<GetMoodCategoriesQuery> for Vec<MoodCategorySection> {
    fn parse_from(p: super::ProcessedResult<GetMoodCategoriesQuery>) -> crate::Result<Self> {
        let crawler = JsonCrawler::from(p);
        crawler
            .navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST))?
            .into_array_into_iter()?
            .map(parse_mood_category_sections)
            .collect()
    }
}
impl<'a> ParseFrom<GetMoodPlaylistsQuery<'a>> for Vec<MoodPlaylistCategory> {
    fn parse_from(p: super::ProcessedResult<GetMoodPlaylistsQuery<'a>>) -> crate::Result<Self> {
        fn parse_mood_playlist_category(mut crawler: JsonCrawler) -> Result<MoodPlaylistCategory> {
            if let Ok(grid) = crawler.borrow_pointer(GRID) {
                parse_mood_playlist_category_grid(grid)
            } else if let Ok(carousel) = crawler.borrow_pointer(CAROUSEL) {
                parse_mood_playlist_category_carousel(carousel)
            } else {
                return Err(crawler.generate_error_paths_not_found([GRID, CAROUSEL]));
            }
        }
        fn parse_mood_playlist_category_grid(
            mut crawler: JsonCrawlerBorrowed,
        ) -> Result<MoodPlaylistCategory> {
            let category_name =
                crawler.take_value_pointer(concatcp!("/header/gridHeaderRenderer", TITLE_TEXT))?;
            let playlists = crawler
                .navigate_pointer("/items")?
                .into_array_iter_mut()?
                .map(parse_mood_playlist)
                .collect::<Result<_>>()?;
            Ok(MoodPlaylistCategory {
                category_name,
                playlists,
            })
        }
        fn parse_mood_playlist_category_carousel(
            mut crawler: JsonCrawlerBorrowed,
        ) -> Result<MoodPlaylistCategory> {
            let category_name = crawler.take_value_pointer(concatcp!(CAROUSEL_TITLE, "/text"))?;
            let playlists = crawler
                .navigate_pointer("/contents")?
                .into_array_iter_mut()?
                .map(parse_mood_playlist)
                .collect::<Result<_>>()?;
            Ok(MoodPlaylistCategory {
                category_name,
                playlists,
            })
        }
        fn parse_mood_playlist(crawler: JsonCrawlerBorrowed) -> Result<MoodPlaylist> {
            let mut item = crawler.navigate_pointer(MTRIR)?;
            let playlist_id = item.take_value_pointer(NAVIGATION_BROWSE_ID)?;
            let title = item.take_value_pointer(TITLE_TEXT)?;
            let thumbnails = item.take_value_pointer(THUMBNAIL_RENDERER)?;
            let author = item
                .borrow_pointer(SUBTITLE_RUNS)?
                .into_array_iter_mut()?
                .take(3)
                .last()
                .map(|mut run| run.take_value_pointer("/text"))
                .ok_or_else(|| {
                    Error::array_size(
                        format!("{}{SUBTITLE_RUNS}/text", item.get_path()),
                        // TODO: Remove allocation
                        Arc::new(item.get_source().into()),
                        1,
                    )
                })??;
            Ok(MoodPlaylist {
                playlist_id,
                title,
                thumbnails,
                author,
            })
        }
        let json_crawler: JsonCrawler = p.into();
        json_crawler
            .navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST))?
            .into_array_into_iter()?
            .map(parse_mood_playlist_category)
            .collect()
    }
}

fn parse_mood_category_sections(crawler: JsonCrawler) -> Result<MoodCategorySection> {
    let mut crawler = crawler.navigate_pointer(GRID)?;
    let section_name =
        crawler.take_value_pointer(concatcp!("/header/gridHeaderRenderer/title", RUN_TEXT))?;
    let mood_categories = crawler
        .navigate_pointer("/items")?
        .into_array_into_iter()?
        .map(parse_mood_categories)
        .collect::<Result<Vec<_>>>()?;
    Ok(MoodCategorySection {
        section_name,
        mood_categories,
    })
}
fn parse_mood_categories(crawler: JsonCrawler) -> Result<MoodCategory> {
    let mut crawler = crawler.navigate_pointer("/musicNavigationButtonRenderer")?;
    let title = crawler.take_value_pointer(concatcp!(CATEGORY_TITLE))?;
    let params = crawler.take_value_pointer(concatcp!(CATEGORY_PARAMS))?;
    Ok(MoodCategory { title, params })
}

fn get_taste_profile_artist(mut crawler: JsonCrawler) -> Result<TasteProfileArtist> {
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
            recomendations::TasteToken, MoodCategoryParams, TasteTokenImpression,
            TasteTokenSelection, YoutubeID,
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
