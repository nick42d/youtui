# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]


## [0.0.10](https://github.com/nick42d/youtui/compare/json-crawler/v0.0.9...json-crawler/v0.0.10) - 2025-10-10

### Added
- [**breaking**] Additional queries - playlists ([#260](https://github.com/nick42d/youtui/pull/260))
- _Stream/Query split for GetPlaylist - into GetPlaylistTracks and GetPlaylistDetails. Same applies to GetWatchPlaylist - split to GetWatchPlaylist and GetLyricsID. lyrics module is removed and it's children moved to song module. watch_playlist module is removed and it's children went to song and playlist modules._ 
- *(ytmapi_rs)* [**breaking**] Add continuations for GetLibraryUpload queries ([#258](https://github.com/nick42d/youtui/pull/258))

- _UploadAlbum modified to reflect optional artist and year fields. TableListUploadSong modified to reflect optional album field. This also contains a breaking change to JsonCrawler - Narrowing of trait iterator types to JsonCrawlerIterator._ 

### Other
- fix release ([#246](https://github.com/nick42d/youtui/pull/246))
- Revert "chore: release ([#236](https://github.com/nick42d/youtui/pull/236))" ([#245](https://github.com/nick42d/youtui/pull/245))



## [0.0.9](https://github.com/nick42d/youtui/compare/json-crawler/v0.0.8...json-crawler/v0.0.9) - 2025-06-11

### Added
- feat: Add borrow_value and borrow_value_pointer methods ([#227](https://github.com/nick42d/youtui/pull/227))

## [0.0.8](https://github.com/nick42d/youtui/compare/json-crawler/v0.0.7...json-crawler/v0.0.8) - 2025-06-02

### Other
- Tidy imports for auto group and granularity ([#234](https://github.com/nick42d/youtui/pull/234))

## [0.0.7](https://github.com/nick42d/youtui/compare/json-crawler/v0.0.6...json-crawler/v0.0.7) - 2025-04-22

### Other
- Small lint fix ([#220](https://github.com/nick42d/youtui/pull/220))

## [0.0.6](https://github.com/nick42d/youtui/compare/json-crawler/v0.0.5...json-crawler/v0.0.6) - 2025-02-17

### Fixed
### Added
- [**breaking**] Relax API for take_value_pointers

### Other
- Update deps (#203)

## [0.0.5](https://github.com/nick42d/youtui/compare/json-crawler/v0.0.4...json-crawler/v0.0.5) - 2024-12-15

### Other
- Clippy lint fix (#185)
- Update dependencies (#182)

## [0.0.4](https://github.com/nick42d/youtui/compare/json-crawler/v0.0.3...json-crawler/v0.0.4) - 2024-11-18

### Added
- Added additional project lints only ([#178](https://github.com/nick42d/youtui/pull/178))

## [0.0.3](https://github.com/nick42d/youtui/compare/json-crawler/v0.0.2...json-crawler/v0.0.3) - 2024-10-26

### Fixed
- [**breaking**] Make album optional for songs search. Closes [#174](https://github.com/nick42d/youtui/pull/174) ([#176](https://github.com/nick42d/youtui/pull/176))
- _album field on SearchResultSong is now optional, removed public ways to create custom error from json_crawler (CrawlerError::array_size_from_context, JsonCrawlerIterator::get_context and JsonCrawlerArrayIterContext struct)_ 

## [0.0.2](https://github.com/nick42d/youtui/compare/json-crawler/v0.0.1...json-crawler/v0.0.2) - 2024-09-04

### Other
- Update dependencies ([#155](https://github.com/nick42d/youtui/pull/155))

## [0.0.1](https://github.com/nick42d/youtui/releases/tag/json-crawler/v0.0.1) - 2024-08-12

### Other
- Initial commit
