use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use crate::common::SongId;

use super::CachedEntity;

#[derive(Debug, Clone)]
pub enum CacheSaverError {
    FailedToCreateFile,
    FailedToParseData,
    FailedToWriteToFile,
    FailedToReadFromFile,
}
pub trait CacheSaver {
    fn save_cache(&mut self, cache: &HashMap<SongId, CachedEntity>) -> Result<(), CacheSaverError>;
    fn load_cache(&self) -> Result<HashMap<SongId, CachedEntity>, CacheSaverError>;
}

#[derive(Debug, Clone)]
pub struct FileCacheSaver {
    cache_dir: PathBuf,
}

impl FileCacheSaver {
    pub fn new<P>(cache_dir: P) -> FileCacheSaver
    where
        P: Into<PathBuf>,
    {
        FileCacheSaver {
            cache_dir: cache_dir.into(),
        }
    }
}

impl CacheSaver for FileCacheSaver {
    fn save_cache(&mut self, cache: &HashMap<SongId, CachedEntity>) -> Result<(), CacheSaverError> {
        let mut file = match File::create(&self.cache_dir.join("cache.json")) {
            Ok(file) => file,
            Err(_) => return Err(CacheSaverError::FailedToCreateFile),
        };
        let data = match serde_json::to_string(cache) {
            Ok(data) => data,
            Err(_) => return Err(CacheSaverError::FailedToParseData),
        };
        match file.write_all(data.as_bytes()) {
            Ok(_) => Ok(()),
            Err(_) => Err(CacheSaverError::FailedToWriteToFile),
        }
    }

    fn load_cache(&self) -> Result<HashMap<SongId, CachedEntity>, CacheSaverError> {
        let file_data = fs::read_to_string(&self.cache_dir.join("cache.json"))
            .map_err(|_| CacheSaverError::FailedToReadFromFile)?;
        let data =
            serde_json::from_str(&file_data).map_err(|_| CacheSaverError::FailedToParseData)?;
        Ok(data)
    }
}

#[derive(Clone)]
pub struct MemoryCacheSaver {
    pub cache: HashMap<SongId, CachedEntity>,
}
impl MemoryCacheSaver {
    #[allow(dead_code)]
    pub fn new() -> MemoryCacheSaver {
        MemoryCacheSaver {
            cache: HashMap::new(),
        }
    }
}
impl CacheSaver for MemoryCacheSaver {
    fn save_cache(
        &mut self,
        cache: &HashMap<SongId, super::CachedEntity>,
    ) -> Result<(), CacheSaverError> {
        self.cache = cache.clone();
        Ok(())
    }

    fn load_cache(&self) -> Result<HashMap<SongId, super::CachedEntity>,CacheSaverError> {
        Ok(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, env::temp_dir, fs, path::PathBuf};

    use crate::cache_manager::cached_song::CachedSong;

    use super::{CacheSaver, FileCacheSaver};

    #[test]
    fn test_save_cache_empty() {
        let cache_dir = temp_dir().join("test_save_cache_empty");
        assert!(
            fs::create_dir(&cache_dir).is_ok(),
            "Failed to create cache directory"
        );
        let mut cache_saver = FileCacheSaver::new(cache_dir.clone());
        let res = cache_saver.save_cache(&HashMap::new());
        assert!(res.is_ok(), "Failed to save cache: {:?}", res);
        let data = match fs::read_to_string(cache_dir.join("cache.json")) {
            Ok(data) => data,
            Err(e) => panic!("Failed to read cache file: {}", e),
        };
        assert_eq!(data, "{}");
        let _ = fs::remove_dir_all(cache_dir);
    }

    #[test]
    fn test_save_cache() {
        let cache_dir = temp_dir().join("test_save_cache");
        assert!(
            fs::create_dir(&cache_dir).is_ok(),
            "Failed to create cache directory"
        );
        let mut cache = HashMap::new();
        cache.insert(
            "test".to_string(),
            crate::cache_manager::CachedEntity::Song(CachedSong {
                artist: "test".to_string(),
                title: "test".to_string(),
                id: "test".to_string(),
                duration: Some(0),
                path: PathBuf::from("test"),
            }),
        );
        let mut cache_saver = FileCacheSaver::new(cache_dir.clone());
        let res = cache_saver.save_cache(&cache);
        assert!(res.is_ok(), "Failed to save cache: {:?}", res);
        let data = match fs::read_to_string(cache_dir.join("cache.json")) {
            Ok(data) => data,
            Err(e) => panic!("Failed to read cache file: {}", e),
        };
        assert_eq!(
            data,
            serde_json::to_string(&cache).expect("Failed to serialize cache")
        );

        let _ = fs::remove_dir_all(cache_dir);
    }
}
