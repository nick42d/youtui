[![dependency status](https://deps.rs/repo/github/nick42d/youtui/status.svg)](https://deps.rs/repo/github/nick42d/youtui)
![build](https://github.com/nick42d/youtui/actions/workflows/release-plz.yml/badge.svg)

## About
Youtui - a simple TUI YouTube Music player written in Rust aiming to implement an Artist->Albums workflow for searching for music, and using discoverability principles for navigation. Inspired by https://github.com/ccgauche/ytermusic/ and cmus.

Ytmapi-rs - an asynchronous API for YouTube Music using Google's internal API, Tokio and Reqwest. Inspired by https://github.com/sigma67/ytmusicapi/.

This project is not supported or endorsed by Google.

## Features
- Quickly and easily display entire artist's discography
- Buffer upcoming songs
- Search suggestions
- Sorting and filtering

## Demo
[![asciicast](https://asciinema.org/a/k6oz2Tx2NXVdaYWSsnHFi0Dwg.svg)](https://asciinema.org/a/k6oz2Tx2NXVdaYWSsnHFi0Dwg)

## Installing youtui
[![Packaging status](https://repology.org/badge/vertical-allrepos/youtui.svg)](https://repology.org/project/youtui/versions)

### Arch Linux
`paru -S youtui`

### FreeBSD
`pkg install youtui`

### Cargo
`cargo install youtui`

## Running youtui
The default option is to use browser authentication, To change this to oauth authentication, a config.toml file can be added to the local youtui config directory (e.g ~/.config/youtui/ on Linux), with the value auth_type = "OAuth". Please note however that config file format is currently unstable and could change in the future.
### Commands
1. To run the TUI application, execute `youtui` with no arguments.
1. To use the API in command-line mode, execute `youtui --help` to see available commands.
### Browser Auth Setup Steps
1. Open YouTube Music in your browser - ensure you are logged in.
1. Open web developer tools (F12).
1. Open Network tab and locate a POST request to `music.youtube.com`.
1. Copy the `Cookie` into a text file named `cookie.txt` into your local youtui config directory. Note you will need to create the directory if it does not exist.
Firefox example (Right click and Copy Value):
![image](https://github.com/nick42d/youtui/assets/133559267/c7fda32c-10bc-4ebe-b18e-ee17c13f6bd0)
Chrome example (Select manually and paste):
![image](https://github.com/nick42d/youtui/assets/133559267/bd2ec37b-1a78-490f-b313-694145bb4854)
### OAuth Setup Steps
1. Prerequisite: A Google Cloud Console account and project are required.
1. Following the [YouTube Data API docs](https://developers.google.com/youtube/registering_an_application), create a new `OAuth client ID` for the application type `TVs and Limited Input devices`. 
1. Run `youtui setup-oauth` with your Client ID and Client Secret and following the instructions. This will create a new `oauth.json` file in the default configuration directory.
### Other Setup
1. If music downloads always return an error, you are able to supply a PO Token by saving it to the file `po_token.txt` into your local youtui config directory. For more information on PO Tokens and how to obtain them, see [here](https://github.com/yt-dlp/yt-dlp/wiki/Extractors#po-token-guide).
1. Configurable keybinds can be supplied as part of your `config.toml`. Example `config.toml`s have been provided in the `./youtui/config/` directory. Please note, the config file format is currently unstable and could break between releases.

## Dependencies note
### General
- A font that can render FontAwesome symbols is required.
### Linux specific
- Youtui uses the Rodio library for playback which relies on Cpal https://github.com/rustaudio/cpal for ALSA support. The cpal readme mentions the that the ALSA development files are required which can be found in the following packages:
  - `libasound2-dev` (Debian / Ubuntu)
  - `alsa-lib-devel` (Fedora)

## Limitations
- This project is under heavy development, and interfaces could change at any time. The project will use semantic versioning to indicate when interfaces have stabilised.

## Roadmap
### Application
- [x] Windows support (target for 0.0.1)
- [x] Configuration folder support (target for 0.0.1)
- [x] Implement improved download speed
- [x] Filtering (target for 0.0.3)
- [x] Release to AUR (target for 0.0.4)
- [x] Remove reliance on rust nightly (target for 0.0.4)
- [x] OAuth authentication including automatic refresh of tokens
- [x] Seeking
- [x] Configurable key bindings
- [x] Logging to a file
- [x] Dbus support for media keys
- [x] Display album cover
- [ ] Gapless playback (blocked - requires symphonia AAC gapless support)
- [ ] Mouse support
- [ ] Offline cache
- [ ] Streaming of buffered tracks
- [ ] Display lyrics
- [ ] Theming
### API
- [x] Document public API
- [x] OAuth authentication
- [x] Implement endpoint continuations
- [ ] Implement all endpoints
- [ ] Automatically update User Agent using a library
- [ ] i18n

Feature parity with `ytmusicapi`
|Endpoint | Implemented: Query | Implemented: Continuations |
|--- | --- | --- |
|GetArtist |[x]||
|GetAlbum |[x]*||
|GetArtistAlbums |[x]||
|Search |[x]|[x]|
|GetSearchSuggestions|[x]||
|GetHome|Not Planned*||
|GetAlbumBrowseId|[ ]||
|GetUser|[ ]||
|GetUserPlaylists|[ ]||
|GetUserVideos|[ ]||
|GetSong|[ ]*||
|GetSongRelated|[ ]*||
|GetLyrics|[x]||
|GetTasteProfile|[x]||
|SetTasteProfile|[x]||
|GetMoodCategories|[x]||
|GetMoodPlaylists|[x]||
|GetCharts|Not Planned*||
|GetWatchPlaylist|[x]|[x]|
|GetLibraryPlaylists|[x]|[x]|
|GetLibrarySongs|[x]|[x]|
|GetLibraryAlbums|[x]|[x]|
|GetLibraryArtists|[x]|[x]|
|GetLibraryArtistSubscriptions|[x]|[x]|
|GetLibraryPodcasts|[x]|[x]|
|GetLibraryChannels|[x]|[x]|
|GetLikedSongs|[ ]|[ ]|
|GetSavedEpisodes|[ ]|[ ]|
|GetAccountInfo|[ ]||
|GetHistory|[x]||
|AddHistoryItem|[x]||
|RemoveHistoryItem|[x]||
|RateSong|[x]||
|EditSongLibraryStatus|[x]||
|RatePlaylist|[x]||
|SubscribeArtists|[ ]||
|UnsubscribeArtists|[ ]||
|GetPlaylist|[x]|[x]|
|CreatePlaylist|[x]||
|EditPlaylist|[x]||
|DeletePlaylist|[x]||
|AddPlaylistItems|[x]||
|RemovePlaylistItems|[x]||
|GetChannel|[*]||
|GetChannelEpisodes|[*]||
|GetPodcast|[*]|[ ]|
|GetEpisode|[*]||
|GetEpisodesPlaylist|Not Planned*||
|Original: GetNewEpisodes|[*]||
|GetLibraryUploadSongs|[x]|[x]|
|GetLibraryUploadArtists|[x]|[x]|
|GetLibraryUploadAlbums|[x]|[x]|
|GetLibraryUploadArtist|[x]|[x]|
|GetLibraryUploadAlbum|[x]||
|UploadSong|[x]||
|DeleteUploadEntity|[x]||

\* GetArtist is partially implemented only
- only returns albums and songs

\* Only the tracking url from GetSong is implemented - as GetSongTrackingUrl. Any additional features for GetSong are not currently planned - recommend taking a look at `rusty_ytdl` library for these features.

\* Note, significantly dynamic pages, such as GetHome and GetSongRelated are not currently planned.

\* GetEpisodesPlaylist is not implemented - it seems the only use case is to get the New Episodes playlist, which has been implemented instead as GetNewEpisodes.

## Developer notes
See the wiki for additional information
https://github.com/nick42d/youtui/wiki
