use std::{path::PathBuf, sync::Arc};

const MUSIC_DIR: &str = "music/";

pub struct MusicCache {
    songs: Vec<PathBuf>,
}

impl MusicCache {
    fn cache_song(&mut self, song: Arc<Vec<u8>>, path: PathBuf) {
        let mut p = PathBuf::new();
        p.push(MUSIC_DIR);
        p.push(&path);
        self.songs.push(path);
        std::fs::write(p, &*song);
    }
    fn retrieve_song(&self, path: PathBuf) -> std::result::Result<Option<Vec<u8>>, std::io::Error> {
        if self.songs.contains(&path) {
            let mut p = PathBuf::new();
            p.push(MUSIC_DIR);
            p.push(&path);
            return std::fs::read(p).map(|v| Some(v));
        }
        Ok(None)
    }
}
