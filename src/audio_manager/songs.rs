use std::path::PathBuf;

use async_trait::async_trait;
use reqwest::Client;
use songbird::input::Input;
use tracing::event;
use youtube_dl::{Error, SingleVideo, YoutubeDl};

use crate::{
    cache_manager::CachedSong,
    common::{Song, SongId},
};

#[derive(Clone)]
pub struct YtSong {
    id: String,
    title: String,
    artist: String,
    duration: Option<u64>,
    // link: String,
    extension: String,
    yt_id: String,
    client: reqwest::Client,
}

impl YtSong {
    #[cfg(test)]
    pub fn create(id: &str, title: &str, artist: &str, duration: Option<u64>) -> YtSong {
        YtSong {
            id: id.to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            duration,
            extension: "mp3".to_string(),
            yt_id: "test".to_string(),
            client: reqwest::Client::new(),
        }
    }

    // TODO change to Result
    pub async fn new(link: impl ToString, client: Client) -> Vec<YtSong> {
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
            return vec![YtSong::from_sv(sv, client)];
        }
        if let Some(pl) = yt_result.into_playlist() {
            let entries = match pl.entries {
                Some(entries) => entries,
                None => return vec![],
            };

            return entries
                .into_iter()
                .map(|sv| YtSong::from_sv(sv, client.clone()))
                .collect();
        }
        vec![]
    }

    pub async fn cache_song(
        self,
        output_template: String,
        base_path: PathBuf,
    ) -> Result<CachedSong, Error> {
        YoutubeDl::new(&self.id)
            .output_template(output_template)
            .format("ba")
            .download_to_async("./")
            .await?;
        Ok(CachedSong {
            id: self.id.clone(),
            title: self.title.clone(),
            artist: self.artist.clone(),
            duration: self.duration.clone(),
            path: base_path.join(format!("{}.{}", self.yt_id, self.extension)),
        })
    }

    fn from_sv(value: SingleVideo, client: Client) -> YtSong {
        YtSong {
            id: get_link(&value),
            title: get_title(&value),
            artist: get_artist(&value),
            duration: get_duration(&value),
            // link: get_link(&value),
            extension: get_extension(&value),
            yt_id: value.id,
            client,
        }
    }
}

#[async_trait]
impl Song for YtSong {
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
        let p = self.id.clone();
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
fn get_link(value: &SingleVideo) -> String {
    match &value.url {
        Some(url) => return url.clone(),
        None => (),
    }
    match &value.webpage_url {
        Some(url) => return url.clone(),
        None => (),
    }
    "Unknown".to_string()
}
fn get_extension(value: &SingleVideo) -> String {
    match &value.ext {
        Some(ext) => return ext.clone(),
        None => (),
    }
    "mp3".to_string()
}
