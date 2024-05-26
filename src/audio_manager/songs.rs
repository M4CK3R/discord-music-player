use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use reqwest::Client;
use songbird::input::Input;
use tokio::sync::RwLock;
use youtube_dl::{SingleVideo, YoutubeDl};

use crate::{
    cache_manager::{
        cache_saver::CacheSaver, CacheManager, CacheableSong, CachedEntity, CachedSong,
    },
    common::{Song, SongId},
};

pub enum YtResult<CS>
where
    CS: CacheSaver + Clone,
{
    Song(YtSong<CS>),
    Playlist(Vec<YtSong<CS>>),
}

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
    output_template: String,
    base_path: PathBuf,
}

#[derive(Debug)]
pub enum YtSongError {
    YoutubeDlError(youtube_dl::Error),
    UnknownError,
    LinkNotFound,
}

impl From<youtube_dl::Error> for YtSongError {
    fn from(e: youtube_dl::Error) -> Self {
        YtSongError::YoutubeDlError(e)
    }
}

impl<CS> YtSong<CS>
where
    CS: CacheSaver + Clone,
{
    pub async fn new(
        link: &str,
        client: reqwest::Client,
        cache_manager: Arc<RwLock<CacheManager<CS>>>,
        output_template: String,
        base_path: PathBuf,
    ) -> Result<YtResult<CS>, YtSongError> {
        let yt_result = YoutubeDl::new(link.to_string())
            .flat_playlist(true)
            .run_async()
            .await?;

        if let Some(sv) = yt_result.clone().into_single_video() {
            return Self::from_sv(sv, client, cache_manager, output_template, base_path);
        }

        if let Some(pl) = yt_result.into_playlist() {
            return Self::from_pl(pl, client, cache_manager, output_template, base_path);
        }

        return Err(YtSongError::UnknownError);
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
        sv: SingleVideo,
        client: Client,
        cache_manager: Arc<RwLock<CacheManager<CS>>>,
        output_template: String,
        base_path: PathBuf,
    ) -> Result<YtResult<CS>, YtSongError> {
        let value = sv;
        Ok(YtResult::Song(YtSong {
            id: get_link(&value)?,
            title: get_title(&value),
            artist: get_artist(&value),
            duration: get_duration(&value),
            extension: get_extension(&value),
            yt_id: value.id,
            client,
            cache_manager: cache_manager,
            base_path,
            output_template,
        }))
    }

    fn from_pl(
        pl: youtube_dl::Playlist,
        client: Client,
        cache_manager: Arc<RwLock<CacheManager<CS>>>,
        output_template: String,
        base_path: PathBuf,
    ) -> Result<YtResult<CS>, YtSongError> {
        let entries = pl.entries.ok_or(YtSongError::UnknownError)?;
        let songs: Vec<_> = entries
            .into_iter()
            .filter_map(|sv| {
                Self::from_sv(
                    sv,
                    client.clone(),
                    cache_manager.clone(),
                    output_template.clone(),
                    base_path.clone(),
                )
                .ok()
                .map(|s| {
                    if let YtResult::Song(s) = s {
                        s
                    } else {
                        unreachable!()
                    }
                })
            })
            .collect();

        Ok(YtResult::Playlist(songs))
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
        if let Some(CachedEntity::Song(song)) = self.cache_manager.read().await.get_entry(&self.id)
        {
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
fn get_link(value: &SingleVideo) -> Result<String, YtSongError> {
    match &value.url {
        Some(url) => return Ok(url.clone()),
        None => (),
    }
    match &value.webpage_url {
        Some(url) => return Ok(url.clone()),
        None => (),
    }
    Err(YtSongError::LinkNotFound)
}
fn get_extension(value: &SingleVideo) -> Option<String> {
    value.ext.clone()
}

#[async_trait]
impl<CS> CacheableSong for YtSong<CS>
where
    CS: CacheSaver + Clone + Send + Sync + 'static,
{
    type E = String;
    fn get_path(&self) -> PathBuf {
        let e = match &self.extension {
            Some(e) => e.clone(),
            None => "mp3".to_string(),
        };
        self.base_path.join(format!("{}.{}", self.yt_id, e))
    }

    async fn cache_song(&self) -> Result<CachedSong, Self::E> {
        YoutubeDl::new(&self.id)
            .output_template(&self.output_template)
            .format("ba")
            .download_to_async("./")
            .await
            .map_err(|e| e.to_string())?;
        let e = match &self.extension {
            Some(e) => e.clone(),
            None => self
                .find_cached_song_extension(&self.base_path)
                .await
                .ok_or("No extension")?,
        };
        Ok(CachedSong {
            id: self.id.clone(),
            title: self.title.clone(),
            artist: self.artist.clone(),
            duration: self.duration.clone(),
            path: self.base_path.join(format!("{}.{}", self.yt_id, e)),
        })
    }
}
