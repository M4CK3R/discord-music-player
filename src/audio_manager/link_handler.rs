use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use regex::Regex;

use reqwest::Client;
use tokio::sync::RwLock;

use crate::{
    cache_manager::{self, cache_saver::CacheSaver, CacheManager, CacheableSong, CachedEntity},
    common::{Song, SongId},
};

use super::songs::{YtResult, YtSong};
static YOUTUBE_REGEX: &str = r"(https?:\/\/)?(www\.)?(m\.)?(music\.)?((youtube)|(youtu\.be)).*";

pub enum LinkHandlerResult {
    Song(Box<dyn CacheableSong<E = String>>),
    Playlist(Vec<Box<dyn CacheableSong<E = String>>>),
}

impl<CS> From<YtResult<CS>> for LinkHandlerResult
where
    CS: CacheSaver + Clone + Send + Sync + 'static,
{
    fn from(result: YtResult<CS>) -> Self {
        match result {
            YtResult::Song(song) => LinkHandlerResult::Song(Box::new(song)),
            YtResult::Playlist(songs) => {
                let mut res = vec![];
                for song in songs {
                    res.push(Box::new(song) as Box<dyn CacheableSong<E = String>>);
                }
                LinkHandlerResult::Playlist(res)
            }
        }
    }
}

#[async_trait]
pub trait LinkHandling {
    async fn handle_link(&self, link: &str) -> Result<LinkHandlerResult, String>;
}

pub struct StandardLinkHandler<CS>
where
    CS: CacheSaver + Clone,
{
    path: PathBuf,
    yt_template: String,
    client: Client,
    cache_manager: Arc<RwLock<CacheManager<CS>>>,
}

impl<CS> StandardLinkHandler<CS>
where
    CS: CacheSaver + Clone,
{
    pub fn new(path: impl ToString, cache_manager: Arc<RwLock<CacheManager<CS>>>) -> Self {
        let p = PathBuf::from(path.to_string());
        Self {
            path: p,
            yt_template: format!("{}/%(id)s.%(ext)s", path.to_string()),
            client: Client::new(),
            cache_manager,
        }
    }
}

#[async_trait]
impl<CS> LinkHandling for StandardLinkHandler<CS>
where
    CS: CacheSaver + Clone + 'static + Send + Sync,
{
    async fn handle_link(&self, link: &str) -> Result<LinkHandlerResult, String> {
        if !is_yt_link(link) {
            return Err("Not a youtube link".to_string());
        }
        let s = YtSong::new(
            link,
            self.client.clone(),
            self.cache_manager.clone(),
            self.yt_template.clone(),
            self.path.clone(),
        )
        .await
        .map_err(|e| format!("Error creating song: {:?}", e))?;

        return Ok(s.into());
    }
}

fn is_yt_link(link: &str) -> bool {
    Regex::new(YOUTUBE_REGEX)
        .expect("Pattern was invalid")
        .is_match(link)
}

pub struct NullLinkHandler {}
#[async_trait]
impl LinkHandling for NullLinkHandler {
    async fn handle_link(&self, _link: &str) -> Result<LinkHandlerResult, String> {
        Ok(LinkHandlerResult::Playlist(Vec::new()))
    }
}
