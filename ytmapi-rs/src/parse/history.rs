use const_format::concatcp;

use super::{parse_table_list_items, ApiSuccess, ParseFrom, TableListItem, MUSIC_SHELF};
use crate::{
    crawler::JsonCrawler,
    nav_consts::{SECTION_LIST, SINGLE_COLUMN_TAB},
    query::{GetHistoryQuery, RemoveHistoryItemsQuery},
    Error,
};

impl ParseFrom<GetHistoryQuery> for Vec<TableListItem> {
    fn parse_from(p: super::ProcessedResult<GetHistoryQuery>) -> crate::Result<Self> {
        let json_crawler = JsonCrawler::from(p);
        let contents = json_crawler.navigate_pointer(concatcp!(SINGLE_COLUMN_TAB, SECTION_LIST))?;
        // TODO: Reduce allocations.
        // If parse_playlist_items returns Vec<Result<SongResult>> or
        // parse_playlist_item function created, we could call potentiall call
        // flatten().collect() directly
        // May require itertools::flatten_ok() or itertools::process_results for this.
        let nested_res: crate::Result<Vec<Vec<TableListItem>>> = contents
            .into_array_into_iter()?
            .map(|c| {
                parse_table_list_items(c.navigate_pointer(concatcp!(MUSIC_SHELF, "/contents"))?)
            })
            .collect();
        Ok(nested_res?.into_iter().flatten().collect())
    }
}
impl<'a> ParseFrom<RemoveHistoryItemsQuery<'a>> for Vec<crate::Result<ApiSuccess>> {
    fn parse_from(p: super::ProcessedResult<RemoveHistoryItemsQuery>) -> crate::Result<Self> {
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
