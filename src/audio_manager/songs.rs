use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use reqwest::Client;
use songbird::input::Input;
use tokio::sync::RwLock;
use tracing::event;
use youtube_dl::{SingleVideo, YoutubeDl};

use crate::{
    cache_manager::{cache_saver::CacheSaver, CacheManager, CachedEntity, CachedSong},
    common::{Song, SongId},
};

#[derive(Clone)]
pub struct YtSong<CS>
where
    CS: CacheSaver + Clone,
{
    id: String,
    title: String,
    artist: String,
    duration: Option<u64>,
    extension: Option<String>,
    yt_id: String,
    client: reqwest::Client,
    cache_manager: Arc<RwLock<CacheManager<CS>>>,
}

impl<CS> YtSong<CS>
where
    CS: CacheSaver + Clone,
{
    #[cfg(test)]
    pub fn create(
        id: &str,
        title: &str,
        artist: &str,
        duration: Option<u64>,
        cache_manager: Arc<RwLock<CacheManager<CS>>>,
    ) -> YtSong<CS> {
        YtSong {
            id: id.to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            duration,
            extension: Some("mp3".to_string()),
            yt_id: "test".to_string(),
            client: reqwest::Client::new(),
            cache_manager: cache_manager,
        }
    }

    // TODO change to Result
    pub async fn new(
        link: impl ToString,
        client: Client,
        cache_manager: Arc<RwLock<CacheManager<CS>>>,
    ) -> Vec<YtSong<CS>> {
        let yt_result = YoutubeDl::new(link.to_string())
            .flat_playlist(true)
            .run_async()
            .await;
        let yt_result = match yt_result {
            Ok(yt_result) => yt_result,
            Err(e) => {
                event!(tracing::Level::ERROR, "Error getting song: {}", e);
                return vec![];
            }
        };
        if let Some(sv) = yt_result.clone().into_single_video() {
            let yt_song = YtSong::from_sv(sv, client, cache_manager);
            match yt_song {
                Ok(s) => return vec![s],
                Err(e) => {
                    event!(tracing::Level::ERROR, "Error getting song: {}", e);
                    return vec![];
                }
            }
        }
        if let Some(pl) = yt_result.into_playlist() {
            let entries = match pl.entries {
                Some(entries) => entries,
                None => return vec![],
            };

            return entries
                .into_iter()
                .map(|sv| YtSong::from_sv(sv, client.clone(), cache_manager.clone()))
                .filter_map(|s| match s {
                    Ok(s) => Some(s),
                    Err(e) => {
                        event!(tracing::Level::ERROR, "Error getting song: {}", e);
                        None
                    }
                })
                .collect();
        }
        vec![]
    }

    pub async fn cache_song(
        self,
        output_template: String,
        base_path: PathBuf,
    ) -> Result<CachedSong, String> {
        YoutubeDl::new(&self.id)
            .output_template(output_template)
            .format("ba")
            .download_to_async("./")
            .await
            .map_err(|e| e.to_string())?;
        let e = match &self.extension {
            Some(e) => e.clone(),
            None => self
                .find_cached_song_extension(&base_path)
                .await
                .ok_or("No extension")?,
        };
        Ok(CachedSong {
            id: self.id.clone(),
            title: self.title.clone(),
            artist: self.artist.clone(),
            duration: self.duration.clone(),
            path: base_path.join(format!("{}.{}", self.yt_id, e)),
        })
    }

    async fn find_cached_song_extension(&self, base_path: &PathBuf) -> Option<String> {
        let id = self.yt_id.clone();
        let mut dir_contents = match tokio::fs::read_dir(base_path).await {
            Ok(dir_contents) => dir_contents,
            Err(_e) => return None,
        };
        let os_id = id.as_str();
        while let Ok(Some(entry)) = dir_contents.next_entry().await {
            let p = entry.path();
            let file_name = match p.file_stem() {
                Some(file_name) => file_name,
                None => {
                    continue;
                }
            };
            if file_name != os_id {
                continue;
            }
            let ext = match p.extension() {
                Some(ext) => ext,
                None => return None,
            };
            return ext.to_os_string().into_string().ok();
        }
        None
    }

    fn from_sv(
        value: SingleVideo,
        client: Client,
        cache_manager: Arc<RwLock<CacheManager<CS>>>,
    ) -> Result<YtSong<CS>, String> {
        Ok(YtSong {
            id: get_link(&value).ok_or("No link found")?,
            title: get_title(&value),
            artist: get_artist(&value),
            duration: get_duration(&value),
            // link: get_link(&value),
            extension: get_extension(&value),
            yt_id: value.id,
            client,
            cache_manager: cache_manager,
        })
    }
}

#[async_trait]
impl<CS> Song for YtSong<CS>
where
    CS: CacheSaver + Send + Sync + Clone + 'static,
{
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
        if let Some(CachedEntity::Song(song)) = self.cache_manager.read().await.get_song(&self.id) {
            return song.get_input().await;
        }
        let p = self.id.clone();
        tracing::info!("Getting YT input for {}", p);
        songbird::input::YoutubeDl::new(self.client.clone(), p).into()
    }
}

fn get_title(sv: &SingleVideo) -> String {
    match sv.title.clone() {
        Some(title) => return title,
        None => (),
    }
    match sv.alt_title.clone() {
        Some(title) => return title,
        None => (),
    }
    "Unknown".to_string()
}
fn get_artist(sv: &SingleVideo) -> String {
    match sv.artist.clone() {
        Some(artist) => return artist,
        None => (),
    }
    match sv.uploader.clone() {
        Some(artist) => return artist,
        None => (),
    }
    "Unknown".to_string()
}
fn get_duration(sv: &SingleVideo) -> Option<u64> {
    match &sv.duration {
        Some(duration) => duration.as_u64(),
        None => None,
    }
}
fn get_link(value: &SingleVideo) -> Option<String> {
    match &value.url {
        Some(url) => return Some(url.clone()),
        None => (),
    }
    match &value.webpage_url {
        Some(url) => return Some(url.clone()),
        None => (),
    }
    None
}
fn get_extension(value: &SingleVideo) -> Option<String> {
    value.ext.clone()
}
