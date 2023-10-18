use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::common::Song;

static CACHE_FILE_NAME: &str = "cache.json";
pub struct CacheManager {
    cache: HashMap<String, Vec<CachedSong>>,
    cache_dir: String,
    yt_template: String,
}

impl CacheManager {
    pub fn new(cache_dir: String) -> Self {
        let mut m = CacheManager {
            cache_dir: cache_dir.clone(),
            cache: HashMap::new(),
            yt_template: format!("{}/%(id)s.%(ext)s", cache_dir),
        };
        m.load();
        m
    }

    fn load(&mut self) {
        let path = PathBuf::from(&self.cache_dir).join(CACHE_FILE_NAME);
        let file_data = match fs::read_to_string(path) {
            Ok(data) => data,
            Err(_) => "{}".to_string(),
        };
        let data: HashMap<String, Vec<CachedSong>> = serde_json::from_str(&file_data).unwrap();
        self.cache = data;
    }

    pub fn save(&self) {
        let path = PathBuf::from(&self.cache_dir).join(CACHE_FILE_NAME);
        let mut file = File::create(path).unwrap();
        let data = serde_json::to_string(&self.cache).unwrap();
        file.write_all(data.as_bytes()).unwrap();
    }

    pub fn add(&mut self, id: &str, songs: Vec<CachedSong>) {
        if songs.is_empty() {
            return;
        }
        if self.cache.contains_key(id) {
            self.cache.remove(id);
        }
        self.cache.insert(id.to_string(), songs);
    }

    pub fn get(&self, id: &str) -> Option<&Vec<CachedSong>> {
        self.cache.get(id)
    }

    pub fn get_yt_template(&self) -> String {
        self.yt_template.clone()
    }

    pub fn get_cached_song_path(&self, id: &str, ext: &str) -> String {
        format!("{}/{}.{}", self.cache_dir, id, ext)
    }
}

impl Drop for CacheManager {
    fn drop(&mut self) {
        self.save();
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct CachedSong {
    title: String,
    artist: String,
    duration: u32,
    path: PathBuf,
    id: String,
}
impl CachedSong {
    pub(crate) fn new(title: String, artist: String, duration: u32, path: PathBuf, id: String) -> CachedSong {
        CachedSong {
            title,
            artist,
            duration,
            path,
            id
        }
    }
}

#[async_trait]
impl Song for CachedSong {
    fn title(&self) -> String {
        self.title.clone()
    }

    fn artist(&self) -> String {
        self.artist.clone()
    }

    fn duration(&self) -> Option<u32> {
        Some(self.duration)
    }

    async fn get_source(&self) -> songbird::input::Input {
        let p = self.path.clone();
        songbird::ffmpeg(p)
            .await
            .expect("Could not create source from file")
    }

    fn create(&self) -> Box<dyn Song> {
        Box::new(CachedSong {
            title: self.title.clone(),
            artist: self.artist.clone(),
            duration: self.duration,
            path: self.path.clone(),
            id: self.id.clone(),
        })
    }

    fn get_id(&self) -> String {
        self.id.clone()
    }
}
