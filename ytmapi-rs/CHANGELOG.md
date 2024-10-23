# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]


## [0.0.14](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.13...ytmapi-rs/v0.0.14) - 2024-10-23

### Added
- [**breaking**] Implement continuations for GetLibraryXX Queries ([#165](https://github.com/nick42d/youtui/pull/165))
- _Client::post_query method has been improved to allow params to be passed to add to URL. Return types for GetLibraryXX queries have been changed to add continuation params - please consider this API still unstable is I'm not yet sure it that's the ideal form. Pre-existing continuations module and query have been refactored to new modules._ 

### Other
- First cut of solution
- Utilise itertools for process_results
- Add itertools as dependency




## [0.0.13](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.12...ytmapi-rs/v0.0.13) - 2024-09-04

### Added
- [**breaking**] Minor improvements to public Query API. Closes [#103](https://github.com/nick42d/youtui/pull/103) ([#157](https://github.com/nick42d/youtui/pull/157))
- _generics removed from get_artist_query simplified query (changed to impl trait)_ 
- Refactor server, implement seek, reduce playback gaps, apply clippy suggestions, update ratatui, implement file logging, make song results order repeatable. ([#151](https://github.com/nick42d/youtui/pull/151))
- [**breaking**] Mark public structs non-exhaustive - Closes [#135](https://github.com/nick42d/youtui/pull/135) ([#145](https://github.com/nick42d/youtui/pull/145))
- _This is a significant breaking change, primarily due to marking many structs non_exhaustive. This breakage now will save breakage in the future. In addition, significant refactoring between modules was undertaken to better organise the project. Further to this, a small number of structs were renamed to better indicate their purpose._ 
- [**breaking**] Implement podcast queries ([#159](https://github.com/nick42d/youtui/pull/159))
- _ChannelID is renamed to ArtistChannelID - allows for new PodcastChannelID also. In additional, video_id field replaced with episode_id field on SearchResultVideo::VideoEpisode, PlaylistEpisode, SearchResultEpisode, HistoryItemEpisode_

### Fixed
- Handle case where top search result is a 'radio' playlist with only 1 subtitle. ([#163](https://github.com/nick42d/youtui/pull/163))
- Resolve panic from api search / improve panic handling ([#161](https://github.com/nick42d/youtui/pull/161))
- Make the channel/artist thumbnail on playlists/albums optional ([#147](https://github.com/nick42d/youtui/pull/147))

### Other
- Update dependencies ([#155](https://github.com/nick42d/youtui/pull/155))

## [0.0.12](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.11...ytmapi-rs/v0.0.12) - 2024-08-17

### Fixed
- [**breaking**] Make library management items optional for album songs ([#140](https://github.com/nick42d/youtui/pull/140))
    - _Changed type of AlbumSong to allow library options to be optional_ 
- fix! Make library management items optional for artist songs ([#139](https://github.com/nick42d/youtui/pull/139))
    - _Changed type of ArtistSong to allow library options to be optional_ 
- Json::into_inner() method should be pub ([#137](https://github.com/nick42d/youtui/pull/137))

## [0.0.11](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.10...ytmapi-rs/v0.0.11) - 2024-08-12

### Fixed
- Move to new rusty_ytdl version (reduces number of downloading 403 errors), and add new scheduled test for downloading ([#134](https://github.com/nick42d/youtui/pull/134))
- Account for search case where an about message exists, but results also exist. ([#131](https://github.com/nick42d/youtui/pull/131)) - Resolves [#128](https://github.com/nick42d/youtui/pull/128)
- [**breaking**] Allow for 'about' renderer in filtered search, and not having 'views' in playlists. ([#130](https://github.com/nick42d/youtui/pull/130))
    - _Added new PlaylistItem types: Episode and UploadSong._

### Other
- [**breaking**] Avoid leaking `serde_json::value` / move `JsonCrawler` to its own crate ([#127](https://github.com/nick42d/youtui/pull/127))
- _ErrorKind's ArraySize, PathNotFoundInArray, PathsNotFound, Parsing and Navigation consilidated into single ErrorKind. Removed parse_upload_song_artists/album functions that had accidentally been marked pub. Removed Error::get_json_and_key function - moved to the ErrorKind itself._



## [0.0.10](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.9...ytmapi-rs/v0.0.10) - 2024-08-05

### Added
- [**breaking**] Return dates and FeedbackTokenRemoveFromHistory from GetHistoryQuery. Closes [#109](https://github.com/nick42d/youtui/pull/109) ([#121](https://github.com/nick42d/youtui/pull/121))
- _Changed type returned from GetHistoryQuery, removed new unused types TableListItem, TableListVideo and TableListEpisode._

### Fixed
- fix! Correct feature gateing / docs for builder ([#111](https://github.com/nick42d/youtui/pull/111))
_Removes YtMusicBuilder::with_rustls_tls unless rustls-tls feature is selected._

## [0.0.9](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.8...ytmapi-rs/v0.0.9) - 2024-07-31

### Added
- [**breaking**] Implement Get method requests - specifically AddHistoryItemQuery. Resolves [#60](https://github.com/nick42d/youtui/pull/60) ([#107](https://github.com/nick42d/youtui/pull/107)), and includes fix for [#106](https://github.com/nick42d/youtui/pull/106).
_generate_xx functions now take Client parameter. Removal of complex YtMusic constructors (functionality moved to new YtMusicBuilder), removed some public functions from RawResult. Query and AuthToken traits modified to allow for specialising by Post / Get type._
- [**breaking**] Improve error messages ([#102](https://github.com/nick42d/youtui/pull/102))
_Removed TryFrom implementation for AlbumType, remove ErrorKind::Other, replaced ZST ApiSuccess with ApiOutcome enum._

## [0.0.8](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.7...ytmapi-rs/v0.0.8) - 2024-07-24

### Added
- Add commandline flag to change auth type. Resolves [#98](https://github.com/nick42d/youtui/pull/98) ([#99](https://github.com/nick42d/youtui/pull/99))
- Implement Taste Profiles and Moods - Resolves [#75](https://github.com/nick42d/youtui/pull/75) ([#97](https://github.com/nick42d/youtui/pull/97))
- feat! Add oauth option for CLI back in. Resolves [#89](https://github.com/nick42d/youtui/pull/89) ([#93](https://github.com/nick42d/youtui/pull/93))
- [**breaking**] Handle new formats for Top Results. Resolves [#87](https://github.com/nick42d/youtui/pull/87) ([#88](https://github.com/nick42d/youtui/pull/88))
New field 'message' added to ErrorKind::Parsing to improve error output.

### Fixed
- Fix not available songs in album failing to parse ([#100](https://github.com/nick42d/youtui/pull/100))
## [0.0.7](https://github.com/nick42d/youtui/compare/ytmapi-rs-v0.0.6...ytmapi-rs-v0.0.7) - 2024-07-19

### Added
- feat! Move convenience functions behind feature gate and add documentation. Resolves [#76](https://github.com/nick42d/youtui/pull/76) ([#81](https://github.com/nick42d/youtui/pull/81))
- feat: Implment mechanism to force use of tls selection - resolves [#30](https://github.com/nick42d/youtui/pull/30) ([#80](https://github.com/nick42d/youtui/pull/80))
- [**breaking**] Allow specialisation of queries depending on the Token ([#79](https://github.com/nick42d/youtui/pull/79))
- feat: Implement DeleteUploadEntity ([#73](https://github.com/nick42d/youtui/pull/73))
- [**breaking**] Implement get library upload queries - resolves [#66](https://github.com/nick42d/youtui/pull/66) ([#70](https://github.com/nick42d/youtui/pull/70))

## [0.0.6](https://github.com/nick42d/youtui/compare/ytmapi-rs-v0.0.5...ytmapi-rs-v0.0.6) - 2024-07-13

### Added
- Implement EditSongLibraryStatus (Resolves [#63](https://github.com/nick42d/youtui/pull/63)) ([#64](https://github.com/nick42d/youtui/pull/64))

### Other
- Seperate live integration tests from local tests - resolves [#61](https://github.com/nick42d/youtui/pull/61) ([#65](https://github.com/nick42d/youtui/pull/65))

## [0.0.5](https://github.com/nick42d/youtui/compare/ytmapi-rs-v0.0.4...ytmapi-rs-v0.0.5) - 2024-07-10

### Added
- [**breaking**] Implement History queries and refactor 'playlist' result types ([#59](https://github.com/nick42d/youtui/pull/59)) - Resolves [#58](https://github.com/nick42d/youtui/pull/58)
- feat(api)! Implement library queries - resolves [#56](https://github.com/nick42d/youtui/pull/56) ([#57](https://github.com/nick42d/youtui/pull/57))

### Other
- Fix reqest URL on ytmapi docs
- [**breaking**] API refactoring: LibraryArtistsSortOrder renamed GetLibrarySortOrder, AlbumParams other versions removed, AlbumParams like_status removed, replaced with new field library_status, AlbumLikeStatus renamed to InLikedSongs, ParseTarget for errors modified - only types now Array or Other(String), module YoutubeResult and usage of ResultCore and YoutubeResult trait removed, add error message to ErrorKind::OtherCodeInResponse, impl_youtube_id no longer public/

## [0.0.4](https://github.com/nick42d/youtui/compare/ytmapi-rs-v0.0.3...ytmapi-rs-v0.0.4) - 2024-06-27

### Added
- Added Playlist query functions to API
- Add documentation for TLS options
- breaking: AlbumParams track_count field renamed to track_count_text - no longer try to parse a number from google's string
- breaking: removed ProcessedResult::parse method
- breaking: LyricsID internals no longer public
- fix - breaking: Renamed deserialization method on AuthToken from 'serialize_json"
- breaking: removed Parse trait - replaced with ParseFrom
- fix: Implement new album / playlist format
- Remove OpenSSL dependency on Linux. Resolves [#42](https://github.com/nick42d/youtui/pull/42) ([#44](https://github.com/nick42d/youtui/pull/44))
# v0.0.3
- fix - breaking: Make error module public
- fix - breaking: Correctly parse video podcast results when searching - changes SearchRessult SearchResultVideo type
- Add support for rustls - thanks to @hungl68444
# v0.0.2
- Improved error handling.
- Provisioned for specialisation over Auth type.
- Added usage examples.
- Documented public API. 
- Removed nightly requirement.
- Implemented all Search cases.
# v0.0.1
- Initial release to github / crates.io.
