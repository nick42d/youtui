use super::{
    ApiSuccess, ParseFrom, CATEGORY_TITLE, GRID, GRID_ITEMS, HEADER_DETAIL, RUN_TEXT,
    TASTE_ITEM_CONTENTS, TASTE_PROFILE_ARTIST, TASTE_PROFILE_IMPRESSION, TASTE_PROFILE_ITEMS,
    TASTE_PROFILE_SELECTION,
};
use crate::{
    common::{recomendations::TasteToken, MoodCategoryParams},
    crawler::{self, JsonCrawler},
    nav_consts::{CATEGORY_PARAMS, NAVIGATION_BROWSE, SECTION_LIST, SINGLE_COLUMN_TAB},
    query::{
        GetMoodCategoriesQuery, GetMoodPlaylistsQuery, GetTasteProfileQuery, SetTasteProfileQuery,
    },
    utils, Result,
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

impl<'a, I> ParseFrom<SetTasteProfileQuery<'a, I>> for ApiSuccess
where
    I: Iterator<Item = TasteToken<'a>> + Clone,
{
    fn parse_from(p: super::ProcessedResult<SetTasteProfileQuery<'a, I>>) -> Result<Self> {
        // Doesn't seem to be an identifier in the response to determine if success or
        // failure - so always assume success.
        Ok(ApiSuccess)
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
impl<'a> ParseFrom<GetMoodPlaylistsQuery<'a>> for () {
    fn parse_from(p: super::ProcessedResult<GetMoodPlaylistsQuery<'a>>) -> crate::Result<Self> {
        todo!()
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
        parse::ApiSuccess,
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
            ApiSuccess,
            SetTasteProfileQuery::new([TasteToken {
                impression_value: TasteTokenImpression::from_raw(""),
                selection_value: TasteTokenSelection::from_raw("")
            }]),
            BrowserToken
        );
    }
}
