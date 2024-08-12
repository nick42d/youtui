# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]


## [0.0.1](https://github.com/nick42d/youtui/releases/tag/json-crawler/v0.0.1) - 2024-08-12

### Other
- [**breaking**] Avoid leaking `serde_json::value` / move `JsonCrawler` to its own crate ([#127](https://github.com/nick42d/youtui/pull/127))
- _ErrorKind's ArraySize, PathNotFoundInArray, PathsNotFound, Parsing and Navigation consilidated into single ErrorKind. Removed parse_upload_song_artists/album functions that had accidentally been marked pub. Removed Error::get_json_and_key function - moved to the ErrorKind itself._


