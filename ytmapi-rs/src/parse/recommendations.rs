use super::{
    ParseFrom, TASTE_ITEM_CONTENTS, TASTE_PROFILE_ARTIST, TASTE_PROFILE_IMPRESSION,
    TASTE_PROFILE_ITEMS, TASTE_PROFILE_SELECTION,
};
use crate::{
    common::recomendations::TasteToken,
    crawler::JsonCrawler,
    query::{GetTasteProfileQuery, SetTasteProfileQuery},
    utils::{self, process_results},
    Result,
};
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Debug, Clone, Deserialize, Serialize)]
pub struct TasteProfileArtist {
    pub artist: String,
    pub taste_tokens: TasteToken<'static>,
}

impl<'a, I> ParseFrom<SetTasteProfileQuery<'a, I>> for ()
where
    I: Iterator<Item = TasteToken<'a>> + Clone,
{
    fn parse_from(p: super::ProcessedResult<SetTasteProfileQuery<'a, I>>) -> Result<Self> {
        todo!()
    }
}

impl ParseFrom<GetTasteProfileQuery> for Vec<TasteProfileArtist> {
    fn parse_from(p: super::ProcessedResult<GetTasteProfileQuery>) -> Result<Self> {
        let crawler = JsonCrawler::from(p);
        // TODO: Neaten this
        let nested_iter = crawler
            .navigate_pointer(TASTE_PROFILE_ITEMS)?
            .into_array_into_iter()?
            .map(|item| {
                item.navigate_pointer(TASTE_ITEM_CONTENTS).and_then(|item| {
                    item.into_array_into_iter()
                        .map(|res| res.map(get_taste_profile_artist))
                })
            });
        utils::process_results::process_results(nested_iter, |i| {
            i.flatten().collect::<Result<Vec<TasteProfileArtist>>>()
        })?
    }
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
        query::{GetTasteProfileQuery, SetTasteProfileQuery},
    };

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
        let query = SetTasteProfileQuery::new(Vec::new());
        panic!()
    }
}
