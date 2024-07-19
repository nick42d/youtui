# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
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
