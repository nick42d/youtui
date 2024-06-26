use crate::Result;
use std::{path::PathBuf, sync::Arc};

const _MUSIC_DIR: &str = "music/";

pub struct _MusicCache {
    songs: Vec<PathBuf>,
}

impl _MusicCache {
    fn _cache_song(&mut self, song: Arc<Vec<u8>>, path: PathBuf) -> Result<()> {
        let mut p = PathBuf::new();
        p.push(_MUSIC_DIR);
        p.push(&path);
        self.songs.push(path);
        std::fs::write(p, &*song)?;
        Ok(())
    }
    fn _retrieve_song(
        &self,
        path: PathBuf,
    ) -> std::result::Result<Option<Vec<u8>>, std::io::Error> {
        if self.songs.contains(&path) {
            let mut p = PathBuf::new();
            p.push(_MUSIC_DIR);
            p.push(&path);
            return std::fs::read(p).map(|v| Some(v));
        }
        Ok(None)
    }
}
