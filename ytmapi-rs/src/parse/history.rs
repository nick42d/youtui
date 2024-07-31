use super::{parse_table_list_item, TableListItem, TryParseFrom, MUSIC_SHELF};
use crate::{
    common::ApiOutcome,
    crawler::JsonCrawler,
    nav_consts::{SECTION_LIST, SINGLE_COLUMN_TAB},
    query::{AddHistoryItemQuery, GetHistoryQuery, RemoveHistoryItemsQuery},
    utils, Result,
};
use const_format::concatcp;

impl TryParseFrom<GetHistoryQuery> for Vec<TableListItem> {
    fn parse_from(p: super::ProcessedResult<GetHistoryQuery>) -> Result<Self> {
        let json_crawler = JsonCrawler::from(p);
        let contents = json_crawler.navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST))?;
        let nested_iter = contents
            .into_array_into_iter()?
            .map(|c| -> crate::Result<_> {
                let iter = c
                    .navigate_pointer(concatcp!(MUSIC_SHELF, "/contents"))?
                    .into_array_into_iter()?
                    .filter_map(|item| parse_table_list_item(item).transpose());
                Ok(iter)
            });
        utils::process_results::process_results(nested_iter, |i| {
            i.flatten().collect::<Result<Vec<TableListItem>>>()
        })?
    }
}
impl<'a> TryParseFrom<RemoveHistoryItemsQuery<'a>> for Vec<ApiOutcome> {
    fn parse_from(p: super::ProcessedResult<RemoveHistoryItemsQuery>) -> Result<Self> {
        let json_crawler = JsonCrawler::from(p);
        json_crawler
            .navigate_pointer("/feedbackResponses")?
            .into_array_into_iter()?
            .map(|mut response| {
                response
                    .take_value_pointer::<bool>("/isProcessed")
                    .map(|p| {
                        if p {
                            return ApiOutcome::Success;
                        }
                        // Better handled in another way...
                        ApiOutcome::Failure
                    })
            })
            .rev()
            .collect()
    }
}
impl<'a> TryParseFrom<AddHistoryItemQuery<'a>> for () {
    fn parse_from(_: crate::parse::ProcessedResult<AddHistoryItemQuery>) -> crate::Result<Self> {
        // Api only returns an empty string, no way of validating if correct or not.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        auth::BrowserToken,
        common::{SongTrackingUrl, YoutubeID},
        query::AddHistoryItemQuery,
    };

    #[tokio::test]
    async fn test_add_history_item_query() {
        let source = String::new();
        crate::process_json::<_, BrowserToken>(
            source,
            AddHistoryItemQuery::new(SongTrackingUrl::from_raw("")),
        )
        .unwrap();
    }
    #[tokio::test]
    async fn test_get_history() {
        parse_test!(
            "./test_json/get_history_20240701.json",
            "./test_json/get_history_20240701_output.txt",
            crate::query::GetHistoryQuery,
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_get_history_with_upload_song() {
        parse_test!(
            "./test_json/get_history_20240713.json",
            "./test_json/get_history_20240713_output.txt",
            crate::query::GetHistoryQuery,
            BrowserToken
        );
    }
    #[tokio::test]
    async fn test_remove_history_items() {
        parse_test!(
            "./test_json/remove_history_items_20240704.json",
            "./test_json/remove_history_items_20240704_output.txt",
            crate::query::RemoveHistoryItemsQuery::new(Vec::new()),
            BrowserToken
        );
    }
}
