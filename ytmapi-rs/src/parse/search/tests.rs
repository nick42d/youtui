use crate::{
    common::Explicit,
    crawler::JsonCrawler,
    parse::{Parse, ProcessedResult},
    process::JsonCloner,
    query::{
        AlbumsFilter, ArtistsFilter, CommunityPlaylistsFilter, EpisodesFilter,
        FeaturedPlaylistsFilter, PodcastsFilter, ProfilesFilter, SearchQuery, SongsFilter,
        VideosFilter,
    },
};
use pretty_assertions::assert_eq;
use std::path::Path;

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
#[tokio::test]
async fn test_search_songs() {
    let source_path = Path::new("./test_json/search_songs_20231226.json");
    let expected_path = Path::new("./test_json/search_songs_20231226_output.txt");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    let expected = tokio::fs::read_to_string(expected_path)
        .await
        .expect("Expect file read to pass during tests");
    let expected = expected.trim();
    let json_clone = JsonCloner::from_string(source).unwrap();
    // Blank query has no bearing on function
    let query = SearchQuery::new("").with_filter(SongsFilter);
    let output = ProcessedResult::from_raw(JsonCrawler::from_json_cloner(json_clone), query)
        .parse()
        .unwrap();
    let output = format!("{:#?}", output);
    assert_eq!(output, expected);
}
#[tokio::test]
async fn test_search_videos() {
    let source_path = Path::new("./test_json/search_videos_20231226.json");
    let expected_path = Path::new("./test_json/search_videos_20231226_output.txt");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    let expected = tokio::fs::read_to_string(expected_path)
        .await
        .expect("Expect file read to pass during tests");
    let expected = expected.trim();
    let json_clone = JsonCloner::from_string(source).unwrap();
    // Blank query has no bearing on function
    let query = SearchQuery::new("").with_filter(VideosFilter);
    let output = ProcessedResult::from_raw(JsonCrawler::from_json_cloner(json_clone), query)
        .parse()
        .unwrap();
    let output = format!("{:#?}", output);
    assert_eq!(output, expected);
}
#[tokio::test]
async fn test_search_featured_playlists() {
    let source_path = Path::new("./test_json/search_featured_playlists_20231226.json");
    let expected_path = Path::new("./test_json/search_featured_playlists_20231226_output.txt");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    let expected = tokio::fs::read_to_string(expected_path)
        .await
        .expect("Expect file read to pass during tests");
    let expected = expected.trim();
    let json_clone = JsonCloner::from_string(source).unwrap();
    // Blank query has no bearing on function
    let query = SearchQuery::new("").with_filter(FeaturedPlaylistsFilter);
    let output = ProcessedResult::from_raw(JsonCrawler::from_json_cloner(json_clone), query)
        .parse()
        .unwrap();
    let output = format!("{:#?}", output);
    assert_eq!(output, expected);
}
#[tokio::test]
async fn test_search_community_playlists() {
    let source_path = Path::new("./test_json/search_community_playlists_20231226.json");
    let expected_path = Path::new("./test_json/search_community_playlists_20231226_output.txt");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    let expected = tokio::fs::read_to_string(expected_path)
        .await
        .expect("Expect file read to pass during tests");
    let expected = expected.trim();
    let json_clone = JsonCloner::from_string(source).unwrap();
    // Blank query has no bearing on function
    let query = SearchQuery::new("").with_filter(CommunityPlaylistsFilter);
    let output = ProcessedResult::from_raw(JsonCrawler::from_json_cloner(json_clone), query)
        .parse()
        .unwrap();
    let output = format!("{:#?}", output);
    assert_eq!(output, expected);
}
#[tokio::test]
async fn test_search_episodes() {
    let source_path = Path::new("./test_json/search_episodes_20231226.json");
    let expected_path = Path::new("./test_json/search_episodes_20231226_output.txt");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    let expected = tokio::fs::read_to_string(expected_path)
        .await
        .expect("Expect file read to pass during tests");
    let expected = expected.trim();
    let json_clone = JsonCloner::from_string(source).unwrap();
    // Blank query has no bearing on function
    let query = SearchQuery::new("").with_filter(EpisodesFilter);
    let output = ProcessedResult::from_raw(JsonCrawler::from_json_cloner(json_clone), query)
        .parse()
        .unwrap();
    let output = format!("{:#?}", output);
    assert_eq!(output, expected);
}
#[tokio::test]
async fn test_search_podcasts() {
    let source_path = Path::new("./test_json/search_podcasts_20231226.json");
    let expected_path = Path::new("./test_json/search_podcasts_20231226_output.txt");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    let expected = tokio::fs::read_to_string(expected_path)
        .await
        .expect("Expect file read to pass during tests");
    let expected = expected.trim();
    let json_clone = JsonCloner::from_string(source).unwrap();
    // Blank query has no bearing on function
    let query = SearchQuery::new("").with_filter(PodcastsFilter);
    let output = ProcessedResult::from_raw(JsonCrawler::from_json_cloner(json_clone), query)
        .parse()
        .unwrap();
    let output = format!("{:#?}", output);
    assert_eq!(output, expected);
}
#[tokio::test]
async fn test_search_profiles() {
    let source_path = Path::new("./test_json/search_profiles_20231226.json");
    let expected_path = Path::new("./test_json/search_profiles_20231226_output.txt");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    let expected = tokio::fs::read_to_string(expected_path)
        .await
        .expect("Expect file read to pass during tests");
    let expected = expected.trim();
    let json_clone = JsonCloner::from_string(source).unwrap();
    // Blank query has no bearing on function
    let query = SearchQuery::new("").with_filter(ProfilesFilter);
    let output = ProcessedResult::from_raw(JsonCrawler::from_json_cloner(json_clone), query)
        .parse()
        .unwrap();
    let output = format!("{:#?}", output);
    assert_eq!(output, expected);
}
