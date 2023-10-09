# About
Youtui - a simple TUI YouTube Music player written in Rust. Inspired by https://github.com/ccgauche/ytermusic/.

Ytmapi-rs - an asynchronous API for youtube music - using Google's internal API. Inspired by https://github.com/sigma67/ytmusicapi/.

This project is not supported by Google.
## How to install and run
- Clone the repository
- Build - note nightly rust required
- Give the application an authorisation header:
  1. Open the https://music.youtube.com website in your browser
  2. Open the developer tools (F12)
  3. Go to the Network tab
  4. Find the request to the `music.youtube.com` document / page
  5. Copy the `Cookie` header from the associated response request
  6. Create a file in the same directory as the binary called `headers.txt`
  7. Create an entry like this `Cookie: <cookie>`
  8. Add a valid "User Agent" below the cookie, like `User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/110.0.0.0 Safari/537.36`
- The following libraries are required for sound on linux (note debian/ubuntu package names):
- - `alsa-tools` `libasound2-dev` `libdbus-1-dev` `pkg-config`
### Limitations
- Song files will be downloaded in the ./music directory from where the binary is located until a better caching system is built.
## Coding constraints
App has been designed for me to learn Rust, and therefore I have implemented the following constraints to learn some features. I am aware these may not be the most efficient ways to code.
1. Avoid shared mutable state: 
The app will avoid shared mutable state primitives such as Mutex and RefCell and instead communicate via messaging.
1. Concurrency over parralelism: 
Where possible, the app will use use an asynchronous mode of operation (such as futures::join! and tokio::select) over parallel equivalents such as tokio::spawn and thread::spawn.
1. Avoid cloning: Where possible, the app will avoid cloning as a method to beat the borrow checker. Instead, we will try to safely borrow.
1. Encode state into the type system: Where possible use the type system to represent actions that are not possible in the current state. This will improve developer ergonomics.
## Design principles
I am aiming to follow the following design principles
1. Discoverability
The app should limit the cognitive load required to memorise commands and should instead provide mechanisms for the user to discover non-obvious commands. E.g commands that require multiple keypresses should display context menus for the subsequent presses like Kakoune or Helix.
## Roadmap
### Application
- [ ] Offline cache
- [ ] Implement improved download speed
- [ ] Real time streaming
- [ ] Theming
### API
- [ ] Implement all endpoints
- [ ] OAuth authentication
- [ ] i18n

|Endpoint | Implemented |
|--- | --- |
|GetArtist | [x] |
|GetAlbum | [x] |
|GetArtistAlbums | [x] |
|Search | [ ]* |
|GetSearchSuggestions|[ ]|
|GetHome|[ ]|
|GetAlbumBrowseId|[ ]|
|GetUser|[ ]|
|GetUserPlaylists|[ ]|
|GetSong|[ ]|
|GetSongRelated|[ ]|
|GetLyrics|[ ]|
|GetTasteProfile|[ ]|
|SetTasteProfile|[ ]|
|GetMoodCategories|[ ]|
|GetMoodPlaylists|[ ]|
|GetCharts|[ ]|
|GetWatchPlaylist|[ ]|
|GetLibraryPlaylists|[ ]|
|GetLibrarySongs|[ ]|
|GetLibraryAlbums|[ ]|
|GetLibraryArtists|[ ]|
|GetLibrarySubscriptions|[ ]|
|GetLikedSongs|[ ]|
|GetHistory|[ ]|
|AddHistoryItem|[ ]|
|RemoveHistoryItem|[ ]|
|RateSong|[ ]|
|EditSongLibraryStatus|[ ]|
|RatePlaylist|[ ]|
|SubscribeArtists|[ ]|
|UnsubscribeArtists|[ ]|
|GetPlaylist|[ ]|
|CreatePlaylist|[ ]|
|EditPlaylist|[ ]|
|DeletePlaylist|[ ]|
|AddPlaylistItems|[ ]|
|RemovePlaylistItems|[ ]|
|GetLibraryUploadSongs|[ ]|
|GetLibraryUploadArtists|[ ]|
|GetLibraryUploadAlbums|[ ]|
|GetLibraryUploadArtist|[ ]|
|GetLibraryUploadAlbum|[ ]|
|UploadAlbum|[ ]|
|DeleteUploadEntity|[ ]|
* search is partially implemented only 
- only returns artists
- does not implement continuations - only first x results returned.
