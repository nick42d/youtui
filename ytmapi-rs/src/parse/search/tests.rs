use crate::{
    auth::BrowserToken,
    parse::SearchResults,
    process_json,
    query::{
        AlbumsFilter, ArtistsFilter, CommunityPlaylistsFilter, EpisodesFilter,
        FeaturedPlaylistsFilter, PodcastsFilter, ProfilesFilter, SearchQuery, SongsFilter,
        VideosFilter,
    },
};
use pretty_assertions::assert_eq;
use std::path::Path;

#[tokio::test]
async fn test_search_basic_top_result_no_type() {
    // Case where topmost result doesn't contain a type.
    parse_test!(
        "./test_json/search_basic_top_result_no_type_20240720.json",
        "./test_json/search_basic_top_result_no_type_20240720_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_basic_top_result_card() {
    // Case where there is only a 'card' top result, with no children.
    parse_test!(
        "./test_json/search_basic_top_result_card_20240721.json",
        "./test_json/search_basic_top_result_card_20240721_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}
#[tokio::test]
async fn test_basic_search_no_results_suggestions() {
    // Case where there are no results, but there are 'Did You Mean' suggestions.
    parse_test_value!(
        "./test_json/search_basic_no_results_suggestions_20240104.json",
        SearchResults::default(),
        SearchQuery::new(""),
        BrowserToken
    );
}

#[tokio::test]
async fn test_search_basic_no_results() {
    // Case where there are no results, and there are not 'Did You Mean'
    // suggestions.
    parse_test!(
        "./test_json/search_basic_no_results_20240721.json",
        "./test_json/search_basic_no_results_20240721_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}

#[tokio::test]
async fn test_search_artists_empty() {
    let source_path = Path::new("./test_json/search_artists_no_results_20231226.json");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    // Blank query has no bearing on function
    let query = SearchQuery::new("").with_filter(ArtistsFilter);
    let output = process_json::<_, BrowserToken>(source, query).unwrap();
    assert_eq!(output, Vec::new());
}
#[tokio::test]
// Test results appear for the correct categories.
async fn test_basic_search_has_simple_top_result() {
    let source_path = Path::new("./test_json/search_basic_top_result_20231228.json");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    // Blank query has no bearing on function
    let query = SearchQuery::new("");
    let output = process_json::<_, BrowserToken>(source, query).unwrap();
    assert!(!output.top_results.is_empty());
}
#[tokio::test]
// Test results appear for the correct categories.
async fn test_basic_search_has_card_top_result() {
    let source_path = Path::new("./test_json/search_highlighted_top_result_20240107.json");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    // Blank query has no bearing on function
    let query = SearchQuery::new("");
    let output = process_json::<_, BrowserToken>(source, query).unwrap();
    assert!(!output.top_results.is_empty());
}
#[tokio::test]
// Test results appear for the correct categories.
async fn test_basic_search_no_top_results_has_results() {
    let source_path = Path::new("./test_json/search_basic_no_top_result_20231228.json");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    // Blank query has no bearing on function
    let query = SearchQuery::new("");
    let output = process_json::<_, BrowserToken>(source, query).unwrap();
    assert!(!output.songs.is_empty());
    assert!(!output.featured_playlists.is_empty());
    assert!(!output.videos.is_empty());
    assert!(!output.community_playlists.is_empty());
    assert!(!output.episodes.is_empty());
    assert!(!output.artists.is_empty());
    assert!(!output.podcasts.is_empty());
    assert!(!output.profiles.is_empty());
    assert!(output.top_results.is_empty());
}

#[tokio::test]
async fn test_basic_search() {
    parse_test!(
        "./test_json/search_highlighted_top_result_20240107.json",
        "./test_json/search_highlighted_top_result_20240107_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}
#[tokio::test]
async fn test_basic_search_with_vodcasts_type_not_specified() {
    parse_test!(
        "./test_json/search_basic_with_vodcasts_type_not_specified_20240612.json",
        "./test_json/search_basic_with_vodcasts_type_not_specified_20240612_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}
#[tokio::test]
async fn test_basic_search_with_vodcasts_type_specified() {
    parse_test!(
        "./test_json/search_basic_with_vodcasts_type_specified_20240612.json",
        "./test_json/search_basic_with_vodcasts_type_specified_20240612_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_artists() {
    parse_test!(
        "./test_json/search_artists_20231226.json",
        "./test_json/search_artists_20231226_output.txt",
        SearchQuery::new("").with_filter(ArtistsFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_albums() {
    parse_test!(
        "./test_json/search_albums_20231226.json",
        "./test_json/search_albums_20231226_output.txt",
        SearchQuery::new("").with_filter(AlbumsFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_songs() {
    parse_test!(
        "./test_json/search_songs_20231226.json",
        "./test_json/search_songs_20231226_output.txt",
        SearchQuery::new("").with_filter(SongsFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_videos() {
    parse_test!(
        "./test_json/search_videos_20231226.json",
        "./test_json/search_videos_20231226_output.txt",
        SearchQuery::new("").with_filter(VideosFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_videos_2024() {
    // Vodcasts were added for this version
    parse_test!(
        "./test_json/search_videos_20240612.json",
        "./test_json/search_videos_20240612_output.txt",
        SearchQuery::new("").with_filter(VideosFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_featured_playlists() {
    parse_test!(
        "./test_json/search_featured_playlists_20231226.json",
        "./test_json/search_featured_playlists_20231226_output.txt",
        SearchQuery::new("").with_filter(FeaturedPlaylistsFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_community_playlists() {
    parse_test!(
        "./test_json/search_community_playlists_20231226.json",
        "./test_json/search_community_playlists_20231226_output.txt",
        SearchQuery::new("").with_filter(CommunityPlaylistsFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_episodes() {
    parse_test!(
        "./test_json/search_episodes_20231226.json",
        "./test_json/search_episodes_20231226_output.txt",
        SearchQuery::new("").with_filter(EpisodesFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_podcasts() {
    parse_test!(
        "./test_json/search_podcasts_20231226.json",
        "./test_json/search_podcasts_20231226_output.txt",
        SearchQuery::new("").with_filter(PodcastsFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_profiles() {
    parse_test!(
        "./test_json/search_profiles_20231226.json",
        "./test_json/search_profiles_20231226_output.txt",
        SearchQuery::new("").with_filter(ProfilesFilter),
        BrowserToken
    );
}
