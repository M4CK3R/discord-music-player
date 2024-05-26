pub mod cache_saver;
mod cached_song;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::common::{Song, SongId};

use self::cache_saver::CacheSaver;

pub use cached_song::{CachedSong, CacheableSong};

#[derive(Clone, Serialize, Deserialize)]
pub enum CachedEntity {
    Song(CachedSong),
    Playlist(Vec<SongId>),
}

pub struct CacheManager<CS>
where
    CS: CacheSaver,
{
    cache_saver: CS,
    cache: HashMap<SongId, CachedEntity>,
}

impl<CS> CacheManager<CS>
where
    CS: CacheSaver,
{
    pub fn new(cache_saver: CS) -> CacheManager<CS> {
        CacheManager {
            cache: HashMap::new(),
            cache_saver,
        }
    }
    pub fn load_cache(&mut self) {
        self.cache = match self.cache_saver.load_cache() {
            Ok(cache) => cache,
            Err(e) => {
                event!(Level::ERROR, "Failed to load cache: {:?}", e);
                HashMap::new()
            }
        };
    }
    pub fn save_cache(&mut self) {
        let res = self.cache_saver.save_cache(&self.cache);
        if let Err(e) = res {
            event!(Level::ERROR, "Failed to save cache: {:?}", e);
        }
    }
    pub fn get_entry(&self, id: &str) -> Option<&CachedEntity> {
        self.cache.get(id)
    }
    pub fn add_entry(&mut self, id: SongId, entity: CachedEntity) {
        self.cache.insert(id, entity);
    }
    pub fn _remove_song(&mut self, id: impl ToString) {
        self.cache.remove(&id.to_string());
    }
    pub fn _clear_cache(&mut self) {
        self.cache.clear();
    }
    pub fn _get_cache(&self) -> Vec<(String, Box<dyn Song>)> {
        let mut res = vec![];
        for (id, entity) in &self.cache {
            match entity {
                CachedEntity::Song(song) => res.push((id.clone(), song.clone_song())),
                CachedEntity::Playlist(songs) => {
                    for song in songs {
                        if let Some(CachedEntity::Song(song)) = self.cache.get(song) {
                            res.push((song.id.clone(), song.clone_song()));
                        }
                    }
                }
            }
        }
        res
    }
    pub fn _is_cached(&self, id: &SongId) -> bool {
        self.cache.contains_key(id)
    }
}
