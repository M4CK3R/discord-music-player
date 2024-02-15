use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use regex::Regex;

use reqwest::Client;
use tokio::sync::RwLock;

use crate::{
    cache_manager::{cache_saver::CacheSaver, CacheManager},
    common::Song,
};

use super::songs::YtSong;
static YOUTUBE_REGEX: &str = r"(https?:\/\/)?(www\.)?(m\.)?(music\.)?((youtube)|(youtu\.be)).*";
#[async_trait]
pub trait LinkHandling {
    async fn handle_link(&self, link: &str) -> Result<Vec<Box<dyn Song>>, String>;
}

pub struct StandardLinkHandler<CS>
where
    CS: CacheSaver,
{
    path: PathBuf,
    yt_template: String,
    cache_manager: Arc<RwLock<CacheManager<CS>>>,
    client: Client
}

impl<CS> StandardLinkHandler<CS>
where
    CS: CacheSaver,
{
    pub fn new(
        // audio_files_path: impl Into<PathBuf>,
        path: impl ToString,
        cache_manager: Arc<RwLock<CacheManager<CS>>>,
    ) -> StandardLinkHandler<CS> {
        let p = PathBuf::from(path.to_string());
        StandardLinkHandler {
            path: p,
            yt_template: format!("{}/%(id)s.%(ext)s", path.to_string()),
            cache_manager: cache_manager,
            client: Client::new()
        }
    }
    fn is_yt_link(link: &str) -> bool {
        Regex::new(YOUTUBE_REGEX)
            .expect("Pattern was invalid")
            .is_match(link)
    }
}

#[async_trait]
impl<CS> LinkHandling for StandardLinkHandler<CS>
where
    CS: CacheSaver + Send + Sync + 'static,
{
    async fn handle_link(&self, link: &str) -> Result<Vec<Box<dyn Song>>, String> {
        if !Self::is_yt_link(link) {
            return Err("Link not supported".to_string());
        }
        let songs = YtSong::new(link, self.client.clone()).await;
        if songs.is_empty() {
            return Err("Could not get song".to_string());
        }
        for song in &songs {
            let _handle = tokio::task::spawn(cache_song_and_save(
                song.clone(),
                self.yt_template.clone(),
                self.path.clone(),
                self.cache_manager.clone(),
            ));
        }
        Ok(songs.iter().map(|s| s.clone_song()).collect())
    }
}

async fn cache_song_and_save<CS>(
    song: YtSong,
    yt_template: String,
    path: PathBuf,
    cache_manager: Arc<RwLock<CacheManager<CS>>>,
) where
    CS: CacheSaver + Send + Sync,
{
    let cached_song = song.cache_song(yt_template.clone(), path.clone()).await;
    let cached_song = match cached_song {
        Ok(s) => s,
        Err(_e) => {
            return;
        }
    };
    let mut cache_manager = cache_manager.write().await;
    cache_manager.add_song(cached_song);
}

#[cfg(test)]
#[tokio::test]
async fn test_cache_song_and_save() {
    use std::env::temp_dir;

    let cache_manager = Arc::new(RwLock::new(CacheManager::new(
        crate::cache_manager::cache_saver::MemoryCacheSaver::new(),
    )));
    let song = YtSong::create("test_id", "test_title", "test_artist", Some(0));
    let tmp = temp_dir();
    let yt_template = format!("{}/%(id)s.%(ext)s", tmp.to_str().unwrap());
    cache_song_and_save(song, yt_template, tmp, cache_manager.clone()).await;
    assert!(cache_manager.read().await.is_cached(&"test_id".to_string()));
}

pub struct NullLinkHandler {}
#[async_trait]
impl LinkHandling for NullLinkHandler {
    async fn handle_link(&self, _link: &str) -> Result<Vec<Box<dyn Song>>, String> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use tokio::sync::RwLock;

    use crate::{
        audio_manager::link_handler::LinkHandling,
        cache_manager::{cache_saver::MemoryCacheSaver, CacheManager},
    };

    use super::StandardLinkHandler;

    type TestLinkHandler = StandardLinkHandler<MemoryCacheSaver>;
    type TestCacheManager = Arc<RwLock<CacheManager<MemoryCacheSaver>>>;

    static YT_LINK: &str = "https://www.youtube.com/watch?v=6n3pFFPSlW4";
    static INVALID_YT_LINK: &str = "https://www.youtube.com/watch?v=00000";
    static NON_YT_LINK: &str = "https://www.google.com";

    fn create_link_handler(path: &str) -> (TestLinkHandler, TestCacheManager) {
        let cache_manager = Arc::new(RwLock::new(CacheManager::new(MemoryCacheSaver::new())));
        (
            StandardLinkHandler::new(path, cache_manager.clone()),
            cache_manager,
        )
    }

    #[tokio::test]
    async fn test_link_handler_new() {
        static PATH: &str = "test";
        let (link_handler, _) = create_link_handler(PATH);
        let p = PathBuf::from(PATH);
        assert_eq!(link_handler.path, p);
    }

    #[tokio::test]
    async fn test_link_handler_is_yt_link() {
        assert!(TestLinkHandler::is_yt_link(YT_LINK));
        assert!(!TestLinkHandler::is_yt_link(NON_YT_LINK));
    }

    #[tokio::test]
    async fn test_link_handler_handle_link() {
        let (link_handler, _) = create_link_handler("test");
        let songs = link_handler
            .handle_link(YT_LINK)
            .await
            .expect("Could not get song");
        assert_eq!(songs.len(), 1);
        let songs = link_handler.handle_link(NON_YT_LINK).await;
        assert!(songs.is_err());
        let songs = link_handler.handle_link(INVALID_YT_LINK).await;
        assert!(songs.is_err());
    }
}
