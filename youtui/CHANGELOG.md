# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.6](https://github.com/nick42d/youtui/compare/youtui-v0.0.5...youtui-v0.0.6) - 2024-07-13

### Added
- Implement EditSongLibraryStatus (Resolves [#63](https://github.com/nick42d/youtui/pull/63)) ([#64](https://github.com/nick42d/youtui/pull/64))
- [**breaking**] Implement History queries and refactor 'playlist' result types ([#59](https://github.com/nick42d/youtui/pull/59)) - Resolves [#58](https://github.com/nick42d/youtui/pull/58)
- feat(api)! Implement library queries - resolves [#56](https://github.com/nick42d/youtui/pull/56) ([#57](https://github.com/nick42d/youtui/pull/57))

### Other
- Resolved application crash due to unrecognised format ([#68](https://github.com/nick42d/youtui/pull/68))
- Seperate live integration tests from local tests - resolves [#61](https://github.com/nick42d/youtui/pull/61) ([#65](https://github.com/nick42d/youtui/pull/65))
- release ([#54](https://github.com/nick42d/youtui/pull/54))
- Fix table on README

### Other
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
