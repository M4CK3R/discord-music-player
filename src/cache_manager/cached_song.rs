use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use songbird::input::Input;

use crate::common::{Song, SongId};

#[derive(Clone, Deserialize, Serialize)]
pub struct CachedSong {
    pub id: SongId,
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub duration: Option<u64>,
}

#[async_trait]
pub trait CacheableSong: Song {
    type E;
    fn get_path(&self) -> PathBuf;
    async fn cache_song(&self) -> Result<CachedSong, Self::E> {
        Ok(CachedSong {
            id: self.get_id().clone(),
            path: self.get_path().clone(),
            title: self.title().clone(),
            artist: self.artist().clone(),
            duration: self.duration(),
        })
    }
}

#[async_trait]
impl Song for CachedSong {
    fn title(&self) -> &String {
        &self.title
    }

    fn artist(&self) -> &String {
        &self.artist
    }

    fn duration(&self) -> Option<u64> {
        self.duration
    }

    fn clone_song(&self) -> Box<dyn Song> {
        Box::new(self.clone())
    }

    fn get_id(&self) -> &SongId {
        &self.id
    }

    async fn get_input(&self) -> Input {
        let p = self.path.clone();
        tracing::info!("Getting cached input for {} with path: {:?}", self.id, p);
        let r = songbird::input::File::new(p);
        r.into()
    }
}

impl CacheableSong for CachedSong {
    type E = ();
    fn get_path(&self) -> PathBuf {
        self.path.clone()
    }
}
