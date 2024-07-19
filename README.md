# About
Youtui - a simple TUI YouTube Music player written in Rust aiming to implement an Artist->Albums workflow for searching for music, and using discoverability principles for navigation. Inspired by https://github.com/ccgauche/ytermusic/ and cmus.

Ytmapi-rs - an asynchronous API for YouTube Music using Google's internal API, Tokio and Reqwest. Inspired by https://github.com/sigma67/ytmusicapi/.

This project is not supported or endorsed by Google.
# Features
- Quickly and easily display entire artist's discography
- Buffer upcoming songs
- Search suggestions
- Sorting and filtering
# Demo
[![asciicast](https://asciinema.org/a/qP9t8RKLNnja9LmqEuNIGWMCJ.svg)](https://asciinema.org/a/qP9t8RKLNnja9LmqEuNIGWMCJ)
# How to install and run
1. The easiest way to install is using crates.io by running `cargo install youtui`.
    1. Alternatively, youtui is now available in the AUR, install using your favourite helper (e.g `paru -S youtui`).
1. Give the application an authorisation header:
    1. Open YouTube Music in your browser - ensure you are logged in.
    1. Open web developer tools (F12).
    1. Open Network tab and locate a POST request to `music.youtube.com`.
    1. Copy the `Cookie` into a text file named `cookie.txt` into your local youtui config directory (e.g ~/.config/youtui/ on Linux). Note you will need to create the directory if it does not exist.
1. To run the TUI application, execute `youtui` with no arguments.
1. To use the API in command-line mode, execute `youtui --help` to see available commands.
## Cookie extraction examples
Firefox example (Right click and Copy Value):
![image](https://github.com/nick42d/youtui/assets/133559267/c7fda32c-10bc-4ebe-b18e-ee17c13f6bd0)
Chrome example (Select manually and paste):
![image](https://github.com/nick42d/youtui/assets/133559267/bd2ec37b-1a78-490f-b313-694145bb4854)
# Dependencies note
## General
- A font that can render FontAwesome symbols is required.
## Linux specific
- Youtui uses the Rodio library for playback which relies on Cpal https://github.com/rustaudio/cpal for ALSA support. The cpal readme mentions the that the ALSA development files are required which can be found in the following packages:
  - `libasound2-dev` (Debian / Ubuntu)
  - `alsa-lib-devel` (Fedora)
# Limitations
- Github version number and tags are only relevant for the binary component of the application. For the latest release version of ytmapi-rs, please refer to crates.io.
- This project is under heavy development, and interfaces could change at any time. The project will use semantic versioning to indicate when interfaces have stabilised.
- The Rodio library used for playback does not currently support seeking or checking progress although there are PRs in progress for both. Progress updates are currently emulated with a ticker and may be slightly out, and seeking is not yet implemented.
# Roadmap
## Application
- [x] Windows support (target for 0.0.1)
- [x] Configuration folder support (target for 0.0.1)
- [x] Implement improved download speed
- [x] Filtering (target for 0.0.3)
- [ ] Logging to a file
- [x] Release to AUR (target for 0.0.4)
- [x] Remove reliance on rust nightly (target for 0.0.4)
- [ ] Dbus support for media keys
- [ ] Seeking
- [ ] Mouse support
- [ ] Offline cache
- [ ] Streaming of buffered tracks
- [ ] OAuth authentication including automatic refresh of tokens
- [ ] Display lyrics and album cover (pixel art)
- [ ] Theming
- [ ] Configurable key bindings
## API
- [x] Document public API
- [ ] Automatically update User Agent using a library
- [ ] Implement endpoint continuations
- [ ] Implement all endpoints
- [x] OAuth authentication
- [ ] i18n

|Endpoint | Implemented: Query | Implemented: Continuations |
|--- | --- | --- |
|GetArtist | [x] ||
|GetAlbum | [x]* ||
|GetArtistAlbums | [x] ||
|Search | [x] |[ ]|
|GetSearchSuggestions|[x]||
|GetHome|[ ]||
|GetAlbumBrowseId|[ ]||
|GetUser|[ ]||
|GetUserPlaylists|[ ]||
|GetSong|[ ]||
|GetSongRelated|[ ]||
|GetLyrics|[x]||
|GetTasteProfile|[ ]||
|SetTasteProfile|[ ]||
|GetMoodCategories|[ ]||
|GetMoodPlaylists|[ ]||
|GetCharts|[ ]||
|GetWatchPlaylist|[x]\*|[ ]|
|GetLibraryPlaylists|[x]|[ ]|
|GetLibrarySongs|[x]|[ ]|
|GetLibraryAlbums|[x]|[ ]|
|GetLibraryArtists|[x]|[ ]|
|GetLibrarySubscriptions|[x]|[ ]|
|GetLikedSongs|[ ]||
|GetHistory|[x]*||
|AddHistoryItem|[ ]||
|RemoveHistoryItem|[x]||
|RateSong|[x]||
|EditSongLibraryStatus|[x]||
|RatePlaylist|[x]||
|SubscribeArtists|[ ]||
|UnsubscribeArtists|[ ]||
|GetPlaylist|[x]|[ ]|
|CreatePlaylist|[x]||
|EditPlaylist|[x]||
|DeletePlaylist|[x]||
|AddPlaylistItems|[x]||
|RemovePlaylistItems|[x]||
|GetLibraryUploadSongs|[x]|[ ]|
|GetLibraryUploadArtists|[x]|[ ]|
|GetLibraryUploadAlbums|[x]|[ ]|
|GetLibraryUploadArtist|[x]|[ ]|
|GetLibraryUploadAlbum|[x]||
|UploadAlbum|[ ]||
|DeleteUploadEntity|[x]||

\* get watch playlist is partially implemented only
- only returns playlist and lyrics ids

\* get artist is partially implemented only
- only returns albums and songs

\* get history is partially implemented only
- does not return a date, and remove from history feedback items are not generated.

# Additional information
See the wiki for additional information
https://github.com/nick42d/youtui/wiki
