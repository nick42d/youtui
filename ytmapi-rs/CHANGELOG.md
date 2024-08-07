# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
