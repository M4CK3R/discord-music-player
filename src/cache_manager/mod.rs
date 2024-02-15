pub mod cache_saver;
mod cached_song;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::{event, Level};

use crate::common::{Song, SongId};

use self::cache_saver::CacheSaver;

pub use cached_song::{CachedSong, CashableSong};

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
    pub fn get_song(&self, id: &str) -> Option<Vec<&CachedSong>> {
        let id = id.to_string();
        match self.cache.get(&id) {
            Some(CachedEntity::Song(song)) => Some(vec![song]),
            Some(CachedEntity::Playlist(songs)) => Some(
                songs
                    .iter()
                    .map(|s| self.get_song(s))
                    .filter_map(|s| s)
                    .flatten()
                    .collect(),
            ),
            None => None,
        }
    }
    pub fn _add_songs(&mut self, song_id: SongId, mut songs: Vec<impl CashableSong>) {
        if songs.len() == 1 {
            let song = match songs.pop() {
                Some(song) => song,
                None => return,
            };
            self.add_song(song.into());
        } else {
            let p = CachedEntity::Playlist(songs.iter().map(|s| s.get_id().clone()).collect());
            self.cache.insert(song_id, p);
            for song in songs {
                self.add_song(song.into());
            }
        };
    }
    pub fn add_song(&mut self, song: impl CashableSong) {
        self.cache
            .insert(song.get_id().clone(), CachedEntity::Song(song.into()));
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
    pub fn is_cached(&self, id: &SongId) -> bool {
        self.cache.contains_key(id)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        fs::{self},
    };

    use async_trait::async_trait;

    use crate::{
        cache_manager::cache_saver::{FileCacheSaver, MemoryCacheSaver},
        common::{Song, SongId},
    };

    use super::{
        cached_song::{CachedSong, CashableSong},
        CacheManager,
    };

    #[derive(Clone)]
    struct MockSong {
        id: SongId,
        title: String,
        artist: String,
        duration: Option<u64>,
    }

    #[async_trait]
    impl Song for MockSong {
        fn get_id(&self) -> &SongId {
            &self.id
        }

        fn clone_song(&self) -> Box<dyn Song> {
            Box::new(MockSong {
                id: self.id.clone(),
                title: self.title.clone(),
                artist: self.artist.clone(),
                duration: self.duration.clone(),
            })
        }

        fn title(&self) -> &String {
            &self.title
        }

        fn artist(&self) -> &String {
            &self.artist
        }

        fn duration(&self) -> Option<u64> {
            self.duration.clone()
        }

        async fn get_input(&self) -> songbird::input::Input {
            unimplemented!("MockSong does not implement get_source")
        }
    }

    impl CashableSong for MockSong {
        fn get_path(&self) -> &std::path::PathBuf {
            unimplemented!("MockSong does not implement get_path")
        }
    }

    impl Into<CachedSong> for MockSong {
        fn into(self) -> CachedSong {
            CachedSong {
                id: self.id,
                title: self.title,
                artist: self.artist,
                duration: self.duration,
                path: Default::default(),
            }
        }
    }

    #[test]
    fn add_songs() {
        let mut cache_manager = CacheManager::new(MemoryCacheSaver::new());

        let song1 = MockSong {
            id: "song1".to_string(),
            title: "Song 1".to_string(),
            artist: "Artist 1".to_string(),
            duration: Some(100),
        };
        let song2 = MockSong {
            id: "song2".to_string(),
            title: "Song 2".to_string(),
            artist: "Artist 2".to_string(),
            duration: Some(200),
        };

        cache_manager.add_song(song1.clone());
        cache_manager.add_song(song2.clone());

        let cached_song1 = cache_manager
            .get_song(&"song1".to_string())
            .expect("Song 1 not cached");
        assert_eq!(cached_song1[0].get_id(), song1.get_id());

        let cached_song2 = cache_manager
            .get_song(&"song2".to_string())
            .expect("Song 2 not cached");
        assert_eq!(cached_song2[0].get_id(), song2.get_id());
    }

    #[test]
    fn remove_songs() {
        let mut cache_manager = CacheManager::new(MemoryCacheSaver::new());

        let song1 = MockSong {
            id: "song1".to_string(),
            title: "Song 1".to_string(),
            artist: "Artist 1".to_string(),
            duration: Some(100),
        };
        let song2 = MockSong {
            id: "song2".to_string(),
            title: "Song 2".to_string(),
            artist: "Artist 2".to_string(),
            duration: Some(200),
        };

        cache_manager.add_song(song1.clone());
        cache_manager.add_song(song2.clone());

        let cached_song1 = cache_manager.get_song("song1").expect("Song 1 not cached");
        assert_eq!(cached_song1[0].get_id(), song1.get_id());

        let cached_song2 = cache_manager.get_song("song2").expect("Song 2 not cached");
        assert_eq!(cached_song2[0].get_id(), song2.get_id());

        cache_manager._remove_song("song1");

        let cached_song1 = cache_manager.get_song("song1");
        assert!(cached_song1.is_none());

        let cached_song2 = cache_manager.get_song("song2").expect("Song 2 not cached");
        assert_eq!(cached_song2[0].get_id(), song2.get_id());
    }

    #[test]
    fn clear_cache() {
        let mut cache_manager = CacheManager::new(MemoryCacheSaver::new());

        let song1 = MockSong {
            id: "song1".to_string(),
            title: "Song 1".to_string(),
            artist: "Artist 1".to_string(),
            duration: Some(100),
        };
        let song2 = MockSong {
            id: "song2".to_string(),
            title: "Song 2".to_string(),
            artist: "Artist 2".to_string(),
            duration: Some(200),
        };

        cache_manager.add_song(song1.clone());
        cache_manager.add_song(song2.clone());

        let cached_song1 = cache_manager.get_song("song1").expect("Song 1 not cached");
        assert_eq!(cached_song1[0].get_id(), song1.get_id());

        let cached_song2 = cache_manager.get_song("song2").expect("Song 2 not cached");
        assert_eq!(cached_song2[0].get_id(), song2.get_id());

        cache_manager._clear_cache();

        let cached_song1 = cache_manager.get_song("song1");
        assert!(cached_song1.is_none());

        let cached_song2 = cache_manager.get_song("song2");
        assert!(cached_song2.is_none());
    }

    #[test]
    fn get_cache() {
        let mut cache_manager = CacheManager::new(MemoryCacheSaver::new());

        let song1 = MockSong {
            id: "song1".to_string(),
            title: "Song 1".to_string(),
            artist: "Artist 1".to_string(),
            duration: Some(100),
        };
        let song2 = MockSong {
            id: "song2".to_string(),
            title: "Song 2".to_string(),
            artist: "Artist 2".to_string(),
            duration: Some(200),
        };

        cache_manager.add_song(song1.clone());
        cache_manager.add_song(song2.clone());

        let cached_songs = cache_manager._get_cache();

        assert_eq!(cached_songs.len(), 2);
        assert!(cached_songs.iter().any(|(id, _)| id == song1.get_id()));
        assert!(cached_songs.iter().any(|(id, _)| id == song2.get_id()));
    }

    #[test]
    fn save_cache() {
        let tmp_dir = temp_dir();
        let mut cache_manager = CacheManager::new(FileCacheSaver::new(&tmp_dir));

        let song1 = MockSong {
            id: "song1".to_string(),
            title: "Song 1".to_string(),
            artist: "Artist 1".to_string(),
            duration: Some(100),
        };
        let song2 = MockSong {
            id: "song2".to_string(),
            title: "Song 2".to_string(),
            artist: "Artist 2".to_string(),
            duration: Some(200),
        };

        cache_manager.add_song(song1.clone());
        cache_manager.add_song(song2.clone());

        cache_manager.save_cache();

        let mut cache_manager = CacheManager::new(FileCacheSaver::new(&tmp_dir));
        cache_manager.load_cache();

        let cached_songs = cache_manager._get_cache();

        assert_eq!(cached_songs.len(), 2);
        assert!(cached_songs.iter().any(|(id, _)| id == song1.get_id()));
        assert!(cached_songs.iter().any(|(id, _)| id == song2.get_id()));
        let _ = fs::remove_dir(tmp_dir);
    }
}
