# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]


## [0.3.1](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.3.0...ytmapi-rs/v0.3.1) - 2026-02-09

### Fixed
- Add gzip feature to reqwest ([#348](https://github.com/nick42d/youtui/pull/348))




## [0.3.0](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.2.4...ytmapi-rs/v0.3.0) - 2026-02-07

### Added
- [**breaking**] Better granularity for LibraryPlaylist fields ([#343](https://github.com/nick42d/youtui/pull/343))

### Fixed
- [**breaking**] handle optional artists/album fields for uploaded songs in a playlist ([#346](https://github.com/nick42d/youtui/pull/346))

### Other
- [**breaking**] Update reqwest ([#327](https://github.com/nick42d/youtui/pull/327))
- _ytmapi_rs now uses rustls by default (just like reqwest)_ 

## [0.2.4](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.2.3...ytmapi-rs/v0.2.4) - 2026-01-13

### Other
- Update non-reqwest deps ([#328](https://github.com/nick42d/youtui/pull/328))




## [0.2.3](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.2.2...ytmapi-rs/v0.2.3) - 2025-12-22

### Fixed
- Handle playlist items that are not available / greyed out ([#315](https://github.com/nick42d/youtui/pull/315))
- Handle MUSIC_VIDEO_TYPE_OFFICIAL_SOURCE_MUSIC in playlist results ([#314](https://github.com/nick42d/youtui/pull/314))




## [0.2.2](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.2.1...ytmapi-rs/v0.2.2) - 2025-12-10

### Other
- Update edition ([#298](https://github.com/nick42d/youtui/pull/298))




## [0.2.1](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.2.0...ytmapi-rs/v0.2.1) - 2025-11-20

### Other
- Add clippy and rustfmt, fix cd trigger ([#274](https://github.com/nick42d/youtui/pull/274))

## [0.2.0](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.1.0...ytmapi-rs/v0.2.0) - 2025-10-10

### Added
- [**breaking**] Add GetUser queries ([#265](https://github.com/nick42d/youtui/pull/265))
- _ProfileID is renamed UserChannelID_ 
- [**breaking**] Add subscribe/unsubscribe artists queries ([#264](https://github.com/nick42d/youtui/pull/264))
- _ArtistParams struct renamed GetArtist in line with most of the other result sructs. Some of its returned field types have been updated to reflect reality._ 
- [**breaking**] Additional queries - playlists ([#260](https://github.com/nick42d/youtui/pull/260))
- _Stream/Query split for GetPlaylist - into GetPlaylistTracks and GetPlaylistDetails. Same applies to GetWatchPlaylist - split to GetWatchPlaylist and GetLyricsID. lyrics module is removed and it's children moved to song module. watch_playlist module is removed and it's children went to song and playlist modules._ 
- *(ytmapi_rs)* Allow custom authtokens to be provided ([#262](https://github.com/nick42d/youtui/pull/262))
- *(ytmapi_rs)* [**breaking**] Add GetLibraryPodcasts and GetLibraryChannels queries ([#259](https://github.com/nick42d/youtui/pull/259))
- _Parse and Query modules have been refactored - this changes the fully qualified path of some of the output types._ 
- *(ytmapi_rs)* [**breaking**] Add continuations for GetLibraryUpload queries ([#258](https://github.com/nick42d/youtui/pull/258))
- _UploadAlbum modified to reflect optional artist and year fields. TableListUploadSong modified to reflect optional album field. This also contains a breaking change to JsonCrawler - Narrowing of trait iterator types to JsonCrawlerIterator._ 
- *(ytmapi_rs)* Add continuations for search queries ([#257](https://github.com/nick42d/youtui/pull/257))


### Fixed
- handle new add/remove library icons that broke multiple queries ([#272](https://github.com/nick42d/youtui/pull/272))

### Other
- Add doc comment for AuthToken ([#256](https://github.com/nick42d/youtui/pull/256))
- [**breaking**] Refactor continuations ([#255](https://github.com/nick42d/youtui/pull/255))
- _Continuable queries no longer return their ContinuationParams by default, and simplification of public client API. Continuable trait replaced with new ParseFromContinuable trait._ 
- AuthToken no longer needs to be sealed, and fix changelog ([#254](https://github.com/nick42d/youtui/pull/254))



## [0.1.0](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.23...ytmapi-rs/v0.1.0) - 2025-06-15

### Added
- [**breaking**] Implement upload song query ([#239](https://github.com/nick42d/youtui/pull/239))
- _This includes a breaking refactor to AuthToken - now implementors just need to define headers and client_version instead of all the query types._

### Other
- Small fix to docs and remove unused UploadUrl type ([#252](https://github.com/nick42d/youtui/pull/252))

## [0.0.23](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.22...ytmapi-rs/v0.0.23) - 2025-06-11

### Added
- [**breaking**] Let queries take iterators as params ([#238](https://github.com/nick42d/youtui/pull/238))
- _This is mostly done in a non-breaking way, however if you were using explicit type parameters for iterators on some simplified queries they have been removed_ 
- [**breaking**]: Allow queries to be run without authentication ([#227](https://github.com/nick42d/youtui/pull/227))
- _This is a breaking change, as some queries are now restricted to only run when authenticated_
- _This also includes a refactor of query, reducing the number of re-exports, meaning some query parameters like SpellingMode now need to be imported more explicitely_
- _In addition, type of PodcastChannelTopResult has changed whilst fixing tests_
- _In addition, ErrorKind::BrowserAuthenticationFailed has been removed, as when fixing tests it was realised it isn't reliably detected_

### Fixed
- Get-album shouldnt hard error when not signed in ([#243](https://github.com/nick42d/youtui/pull/243))
- [**breaking**] Update oauth to latest method ([#241](https://github.com/nick42d/youtui/pull/241))
- _Methods used to create oauth tokens have been updated to reflect the need for Client ID and Client Secret._ 

### Other
- small fix to ytmapi-rs::Client docs ([#235](https://github.com/nick42d/youtui/pull/235))

## [0.0.22](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.21...ytmapi-rs/v0.0.22) - 2025-06-02

### Added
- Derive Eq & Hash for Youtuve IDs, add feature gated ability to construct using a reqwest::Client, add artist_thumbnails to GetAlbum (and fix thumbnails incorrectly showing artist thumbnails) ([#232](https://github.com/nick42d/youtui/pull/232))

### Other
- Tidy imports for auto group and granularity ([#234](https://github.com/nick42d/youtui/pull/234))

## [0.0.21](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.20...ytmapi-rs/v0.0.21) - 2025-04-22

### Added
- [**breaking**] SearchResultSong.album is now a ParsedSongAlbum instead of just album title. Also a lint fix. ([#220](https://github.com/nick42d/youtui/pull/220))

### Other
- *(deps)* bump tokio in the cargo group across 1 directory ([#216](https://github.com/nick42d/youtui/pull/216))

## [0.0.19](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.18...ytmapi-rs/v0.0.19) - 2025-02-17

### Fixed
- error running GetAlbumQuery (a/b test) - resolves #205 (#206)

### Other
- Update deps (#203)

## [0.0.18](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.17...ytmapi-rs/v0.0.18) - 2025-02-04

### Fixed 
- Use latest ytmapi-rs - for release (#199) - resolves error unknown variant KEEP #193

## [0.0.17](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.16...ytmapi-rs/v0.0.17) - 2024-12-15

### Added
- Clippy linting fix (#185)
- Update dependencies (#182)

## [0.0.16](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.15...ytmapi-rs/v0.0.16) - 2024-11-18

### Added
- Small fixes to alternative TLS builders ([#178](https://github.com/nick42d/youtui/pull/178))

## [0.0.15](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.14...ytmapi-rs/v0.0.15) - 2024-10-26

### Fixed
- [**breaking**] Make album optional for songs search. Closes [#174](https://github.com/nick42d/youtui/pull/174) ([#176](https://github.com/nick42d/youtui/pull/176))
- _album field on SearchResultSong is now optional_

## [0.0.14](https://github.com/nick42d/youtui/compare/ytmapi-rs/v0.0.13...ytmapi-rs/v0.0.14) - 2024-10-24

### Added
- [**breaking**] Implement continuations for GetLibraryXX Queries ([#165](https://github.com/nick42d/youtui/pull/165))
- _Client::post_query method has been improved to allow params to be passed to add to URL. Return types for GetLibraryXX queries have been changed to add continuation params - please consider this API still unstable is I'm not yet sure it that's the ideal form. Pre-existing continuations module and query have been refactored to new modules._ 

### Fixed
- Resolve duration for songs search and years field for artists search when multiple artists exist. Closes [#171](https://github.com/nick42d/youtui/pull/171) ([#173](https://github.com/nick42d/youtui/pull/173))

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
