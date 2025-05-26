use rusty_ytdl::reqwest;
use ytmapi_rs::common::Thumbnail;

/// Representation of a song in memory - an array of bytes.
/// Newtype pattern is used to provide a cleaner Debug display.
#[derive(PartialEq)]
pub struct InMemAlbumArt(pub Vec<u8>);
// Custom derive - otherwise will be displaying 3MB array of bytes...
impl std::fmt::Debug for InMemAlbumArt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("InMemSong").field(&"Vec<..>").finish()
    }
}

pub struct AlbumArtDownloader {
    client: reqwest::Client,
}

impl AlbumArtDownloader {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
    pub fn download_album_art(&self, thumbs: Thumbnail) -> () {
        ()
    }
}

fn download_album_art(thumbs: Thumbnail) -> () {}
