# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]


## [0.0.8](https://github.com/nick42d/youtui/compare/async-callback-manager/v0.0.7...async-callback-manager/v0.0.8) - 2025-07-07

### Added
- [**breaking**] Additional queries - playlists ([#260](https://github.com/nick42d/youtui/pull/260))
- _Stream/Query split for GetPlaylist - into GetPlaylistTracks and GetPlaylistDetails. Same applies to GetWatchPlaylist - split to GetWatchPlaylist and GetLyricsID. lyrics module is removed and it's children moved to song module. watch_playlist module is removed and it's children went to song and playlist modules._ 



## [0.0.7](https://github.com/nick42d/youtui/compare/async-callback-manager/v0.0.6...async-callback-manager/v0.0.7) - 2025-06-02

### Added
- Correctly export TaskInformation ([#232](https://github.com/nick42d/youtui/pull/232))

### Other
- Tidy imports for auto group and granularity ([#234](https://github.com/nick42d/youtui/pull/234))

## [0.0.6](https://github.com/nick42d/youtui/compare/async-callback-manager/v0.0.5...async-callback-manager/v0.0.6) - 2025-04-22

### Other
- Add AsyncTask::is_no_op() function ([#218](https://github.com/nick42d/youtui/pull/218))
- *(deps)* bump tokio in the cargo group across 1 directory ([#216](https://github.com/nick42d/youtui/pull/216))

## [0.0.5](https://github.com/nick42d/youtui/compare/async-callback-manager/v0.0.4...async-callback-manager/v0.0.5) - 2025-02-17

### Other
- Update deps (#203)


## [0.0.4](https://github.com/nick42d/youtui/compare/async-callback-manager/v0.0.3...async-callback-manager/v0.0.4) - 2025-02-04

### Other
- update Cargo.lock dependencies




## [0.0.3](https://github.com/nick42d/youtui/compare/async-callback-manager/v0.0.2...async-callback-manager/v0.0.3) - 2024-12-15

### Added
- Refactor to improve combinators (#185)
- Update dependencies (#182)

## [0.0.2](https://github.com/nick42d/youtui/compare/async-callback-manager/v0.0.1...async-callback-manager/v0.0.2) - 2024-11-18

### Fixed
- Fixed tests breaking as unable to drain manager [#179](https://github.com/nick42d/youtui/pull/179) ([#181](https://github.com/nick42d/youtui/pull/181))


