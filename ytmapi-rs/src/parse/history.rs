use const_format::concatcp;

use super::{parse_table_list_item, ApiSuccess, ParseFrom, TableListItem, MUSIC_SHELF};
use crate::{
    crawler::JsonCrawler,
    nav_consts::{SECTION_LIST, SINGLE_COLUMN_TAB},
    query::{GetHistoryQuery, RemoveHistoryItemsQuery},
    utils, Error, Result,
};

impl ParseFrom<GetHistoryQuery> for Vec<TableListItem> {
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
impl<'a> ParseFrom<RemoveHistoryItemsQuery<'a>> for Vec<Result<ApiSuccess>> {
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
                            return Ok(ApiSuccess);
                        }
                        // Better handled in another way...
                        Err(Error::other("Recieved isProcessed false"))
                    })
            })
            .rev()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::BrowserToken;

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
