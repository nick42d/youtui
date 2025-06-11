# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]


## [0.0.25](https://github.com/nick42d/youtui/compare/youtui/v0.0.24...youtui/v0.0.25) - 2025-06-11

### Added
- [**breaking**] Let queries take iterators as params ([#238](https://github.com/nick42d/youtui/pull/238))
- _Let queries take iterators as params ([#238](https://github.com/nick42d/youtui/pull/238))_ 
- feat!(ytmapi_rs): Allow queries to be run without authentication ([#227](https://github.com/nick42d/youtui/pull/227))

### Fixed
- Logging shouldnt fail when error messages occur in parallel ([#244](https://github.com/nick42d/youtui/pull/244))
- [**breaking**] Update oauth to latest method ([#241](https://github.com/nick42d/youtui/pull/241))
- _Methods used to create oauth tokens have been updated to reflect the need for Client ID and Client Secret._ 



## [0.0.24](https://github.com/nick42d/youtui/compare/youtui/v0.0.23...youtui/v0.0.24) - 2025-06-02

### Added
- Show album art ([#232](https://github.com/nick42d/youtui/pull/232))
- Add shell completions with generate-completions cmdline option ([#233](https://github.com/nick42d/youtui/pull/233)) (Closes #47)

### Other
- Tidy imports for auto group and granularity ([#234](https://github.com/nick42d/youtui/pull/234))
- [**breaking**] Revert unneeded part of #221 ([#230](https://github.com/nick42d/youtui/pull/230))
- _Removes ability to set media keys as keybinds. Instead, you should use platform media controls (and media keys are likely routed through these already)._

## [0.0.23](https://github.com/nick42d/youtui/compare/youtui/v0.0.22...youtui/v0.0.23) - 2025-05-25

### Added
- Use platform media controls ([#223](https://github.com/nick42d/youtui/pull/223))
- Add ability to use media keys as keybinds ([#221](https://github.com/nick42d/youtui/pull/221))
- _Note - currently disabled_ 

### Other
- Update README.md ([#228](https://github.com/nick42d/youtui/pull/228))

## [0.0.22](https://github.com/nick42d/youtui/compare/youtui/v0.0.21...youtui/v0.0.22) - 2025-04-22

### Added
- [**breaking**] Ability to search for songs instead of just artists - closes 153 ([#220](https://github.com/nick42d/youtui/pull/220))
- _Some keybind actions have been renamed - see config.toml for more details. In addition, default keybinds for filter and sort have changed slightly to accommodate new F6 default keybind for changing search type. In addition, search entry form starts open when you open youtui._ 
### Other
- Add some more unit tests - playlist ([#218](https://github.com/nick42d/youtui/pull/218))
- *(deps)* bump tokio in the cargo group across 1 directory ([#216](https://github.com/nick42d/youtui/pull/216))
- Unit tests for playlist ([#214](https://github.com/nick42d/youtui/pull/214))

## [0.0.20](https://github.com/nick42d/youtui/compare/youtui/v0.0.19...youtui/v0.0.20) - 2025-02-17

### Fixed
- Not able to move up/down on sort menu (Resolves #201) (#204)
- Error running GetAlbumQuery (a/b test) - resolves #205 (#206)

### Other
- Update deps (#203)

## [0.0.19](https://github.com/nick42d/youtui/compare/youtui/v0.0.18...youtui/v0.0.19) - 2025-02-04

### Added
- Improve logging (#192) - resolves #129

### Fixed
- Bump rusty_ytdl version (#198) - closes video source empty #196
- Use latest ytmapi-rs - for release (#199) - resolves error unknown variant KEEP #193

### Other
- Use anyhow for youtui - closes #187 (#190)

## [0.0.18](https://github.com/nick42d/youtui/compare/youtui/v0.0.17...youtui/v0.0.18) - 2024-12-15

### Added
- Configurable keyboard shortcuts - see #10 or README for docs (#185)
- Implement ability to move cursor within text box - closes #154 (#182)

### Fixed
- Use a unique identifier to add albums instead of the album name. Closes #12. (#183)

## [0.0.17](https://github.com/nick42d/youtui/compare/youtui/v0.0.16...youtui/v0.0.17) - 2024-11-18

### Added
- refactor task management and improve responsiveness ([#178](https://github.com/nick42d/youtui/pull/178))

### Fixed
- [**breaking**] Update default auth method - closes [#179](https://github.com/nick42d/youtui/pull/179) ([#181](https://github.com/nick42d/youtui/pull/181))
- _Youtui default auth method has changed from OAuth to Browser._ 

## [0.0.16](https://github.com/nick42d/youtui/compare/youtui/v0.0.15...youtui/v0.0.16) - 2024-10-26

### Fixed
- Make album optional for songs search. Closes [#174](https://github.com/nick42d/youtui/pull/174) ([#176](https://github.com/nick42d/youtui/pull/176))
- _album field on SearchResultSong is now optional, removed public ways to create custom error from json_crawler (CrawlerError::array_size_from_context, JsonCrawlerIterator::get_context and JsonCrawlerArrayIterContext struct)_ 

## [0.0.15](https://github.com/nick42d/youtui/compare/youtui/v0.0.14...youtui/v0.0.15) - 2024-10-24

### Fixed
- Add way to supply potoken ([#170](https://github.com/nick42d/youtui/pull/170))

## [0.0.14](https://github.com/nick42d/youtui/compare/youtui/v0.0.13...youtui/v0.0.14) - 2024-09-04

### Added
- Highlight now playing song in playlist ([#156](https://github.com/nick42d/youtui/pull/156))
- Refactor server, implement seek, reduce playback gaps, apply clippy suggestions, update ratatui, implement file logging, make song results order repeatable. ([#151](https://github.com/nick42d/youtui/pull/151))

### Fixed
- Resolve panic from api search / improve panic handling ([#161](https://github.com/nick42d/youtui/pull/161))
- Choose 'Highest' audio quality by default ([#150](https://github.com/nick42d/youtui/pull/150)) - resolves [#143](https://github.com/nick42d/youtui/pull/143)

### Other
- Update dependencies ([#155](https://github.com/nick42d/youtui/pull/155))

## [0.0.13](https://github.com/nick42d/youtui/compare/youtui/v0.0.12...youtui/v0.0.13) - 2024-08-17

### Other
- updated the following local packages: ytmapi-rs (resolves some crashes when searching for songs - [#138]).

## [0.0.12](https://github.com/nick42d/youtui/compare/youtui/v0.0.11...youtui/v0.0.12) - 2024-08-12

### Fixed
- Move to new rusty_ytdl version (reduces number of downloading 403 errors), and add new scheduled test for downloading ([#134](https://github.com/nick42d/youtui/pull/134))
- Oauth refresh can no longer get cancelled ([#124](https://github.com/nick42d/youtui/pull/124))
### Other
- Update readme ([#122](https://github.com/nick42d/youtui/pull/122))


## [0.0.11](https://github.com/nick42d/youtui/compare/youtui/v0.0.10...youtui/v0.0.11) - 2024-08-05

### Added
- [**breaking**] Return dates and FeedbackTokenRemoveFromHistory from GetHistoryQuery. Closes [#109](https://github.com/nick42d/youtui/pull/109) ([#121](https://github.com/nick42d/youtui/pull/121))
- _Changed type returned from GetHistoryQuery, removed new unused types TableListItem, TableListVideo and TableListEpisode._
- [**breaking**] Implement Oauth option for TUI. Resolves [#92](https://github.com/nick42d/youtui/pull/92) ([#104](https://github.com/nick42d/youtui/pull/104))
- _Changed default authentication to Oauth - you will receive an error on first startup. See README.md for more details._

### Fixed
- Fix unable to play songs that had a retried download ([#120](https://github.com/nick42d/youtui/pull/120))
- Resolve cant skip if current track is in error ([#119](https://github.com/nick42d/youtui/pull/119)) - Resolves [#118](https://github.com/nick42d/youtui/pull/118)

## [0.0.10](https://github.com/nick42d/youtui/compare/youtui/v0.0.9...youtui/v0.0.10) - 2024-08-03

### Fixed
- Fix all songs causing crash with 'UnrecognisedFormat', and subset of songs causing crash with 'End of stream'. Downloads will now retry up to 5 times. (Resolves [#113](https://github.com/nick42d/youtui/pull/113), [#95](https://github.com/nick42d/youtui/pull/95)) ([#115](https://github.com/nick42d/youtui/pull/115))

## [0.0.9](https://github.com/nick42d/youtui/compare/youtui/v0.0.8...youtui/v0.0.9) - 2024-07-31

### Added
- [**breaking**] Implement Get method requests - specifically AddHistoryItemQuery. Resolves [#60](https://github.com/nick42d/youtui/pull/60) ([#107](https://github.com/nick42d/youtui/pull/107)), and includes fix for [#106](https://github.com/nick42d/youtui/pull/106).
_generate_xx functions now take Client parameter. Removal of complex YtMusic constructors (functionality moved to new YtMusicBuilder), removed some public functions from RawResult. Query and AuthToken traits modified to allow for specialising by Post / Get type._
### Other
- Update README.md

## [0.0.8](https://github.com/nick42d/youtui/compare/youtui/v0.0.7...youtui/v0.0.8) - 2024-07-24

### Added
- Add commandline flag to change auth type. Resolves [#98](https://github.com/nick42d/youtui/pull/98) ([#99](https://github.com/nick42d/youtui/pull/99))
- Implement Taste Profiles and Moods - Resolves [#75](https://github.com/nick42d/youtui/pull/75) ([#97](https://github.com/nick42d/youtui/pull/97))
- feat! Add oauth option for CLI back in. Resolves [#89](https://github.com/nick42d/youtui/pull/89) ([#93](https://github.com/nick42d/youtui/pull/93))
- [**breaking**] Handle new formats for Top Results. Resolves [#87](https://github.com/nick42d/youtui/pull/87) ([#88](https://github.com/nick42d/youtui/pull/88))
New field 'message' added to ErrorKind::Parsing to improve error output.

### Other
- Improve README.md ([#91](https://github.com/nick42d/youtui/pull/91)) by @yonas - Closes [#90](https://github.com/nick42d/youtui/pull/90)- Update README.md

## [0.0.7](https://github.com/nick42d/youtui/compare/youtui/v0.0.6...youtui/v0.0.7) - 2024-07-19

### Added
- feat: More reliable use of rustls-tls
- feat: Implement DeleteUploadEntity ([#73](https://github.com/nick42d/youtui/pull/73))
- [**breaking**] Implement get library upload queries - resolves [#66](https://github.com/nick42d/youtui/pull/66) ([#70](https://github.com/nick42d/youtui/pull/70))
New variant UploadSong added to TableListItem - this can occur when parsing History where you have recently played an uploaded song.
### Fixed
- youtui: Correctly use rustls over openssl ([#78](https://github.com/nick42d/youtui/pull/78))
### Other
- Update README.md- release ([#71](https://github.com/nick42d/youtui/pull/71))
- Update to latest library version

## [0.0.6](https://github.com/nick42d/youtui/compare/youtui-v0.0.5...youtui-v0.0.6) - 2024-07-13

### Other
- Resolved application crash due to unrecognised format ([#68](https://github.com/nick42d/youtui/pull/68))
- Fix table on README
- Update to latest library version

## [0.0.5](https://github.com/nick42d/youtui/compare/youtui-v0.0.4...youtui-v0.0.5) - 2024-06-27

### Added
- [**breaking**] Added Playlist query functions to API

### Fixed
- Resolve visual glitch with table heading [#52](https://github.com/nick42d/youtui/pull/52) ([#53](https://github.com/nick42d/youtui/pull/53))

### Other
- Update dependencies ([#51](https://github.com/nick42d/youtui/pull/51)) - resolves [#43](https://github.com/nick42d/youtui/pull/43)

## [0.0.4]
### Added
- Removed nightly requirement.
- Updated to latest version of API and bumped some other dependencies.
- Added pkgbuild for AUR.
### Fixed
- Resolved #16.
## [0.0.3]
### Added
- Added filtering for browser.
- Keybinds for modal dialogs like search and filter now shown on top bar.
- Reduced number of help commands shown (e.g for list methods like Up / Down).
- Help commands now scrollable if they don't fit on the screen.
- Now able to exit app with Ctrl-C
### Fixed
- Resolved #6 and #5. Thanks @SeseMueller for the reports!
## [0.0.2]
### Added
- Added instructions to README to install with nightly rust. 
## [0.0.1]
### Other
- Initial release to github / crates.io.
