# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
