use crate::common::Song;

use self::cache_manager::{CacheManager, CachedSong};

mod cache_manager;
mod utils;
mod yt_dl_wrapper;

pub struct AudioManager {
    cache_manager: CacheManager,
}

impl AudioManager {
    pub fn new() -> Self {
        let audio_files_path = std::env::var("AUDIO_FILES_PATH").expect("AUDIO_FILES_PATH not set");
        AudioManager {
            cache_manager: CacheManager::new(audio_files_path.into()),
        }
    }

    pub async fn handle_link(&mut self, url: &str) -> Result<Vec<Box<dyn Song>>, String> {
        if let Some(songs) = self.cache_manager.get(url) {
            return Ok(songs.iter().map(|s| s.create()).collect());
        }

        if !utils::is_youtube_link(url) {
            return Err("Not a youtube link".into());
        }

        self.handle_youtube_link(url).await
    }

    pub fn load_songs(&self, queue: Vec<String>) -> Vec<Box<dyn Song>> {
        let mut res = Vec::new();
        for id in queue {
            let songs = self.cache_manager.get(&id);
            if songs.is_none() {
                continue;
            }
            let songs = songs.unwrap();
            for song in songs {
                res.push(song.create());
            }
        }
        res
    }

    async fn handle_youtube_link(&mut self, url: &str) -> Result<Vec<Box<dyn Song>>, String> {
        let songs =
            yt_dl_wrapper::download_audio_files(url, &self.cache_manager.get_yt_template()).await?;

        let cached_songs = {
            let mut cached_songs = Vec::new();
            for sv in songs {
                let ext = match sv.ext.clone() {
                    Some(e) => e,
                    None => continue,
                };
                let path = self.cache_manager.get_cached_song_path(&sv.id, &ext);
                let artist = match &sv.artist {
                    Some(a) => a.clone(),
                    None => "Unknown".to_string(),
                };
                let duration_value = match sv.duration.clone() {
                    Some(d) => d,
                    None => continue,
                };
                let duration = match duration_value.as_u64() {
                    Some(d) => d as u32,
                    None => continue,
                };
                let url = match &sv.url {
                    Some(u) => u.clone(),
                    None => continue,
                };
                let title = match &sv.title {
                    Some(t) => t.clone(),
                    None => continue,
                };
                let cs = CachedSong::new(title, artist, duration, path.into(), url);
                self.cache_manager.add(&cs.get_id(), vec![cs.clone()]);
                cached_songs.push(cs);
            }
            cached_songs
        };

        let res: Vec<Box<dyn Song>> = cached_songs.iter().map(|s| s.create()).collect();
        self.cache_manager.add(url, cached_songs);
        Ok(res)
    }
}
