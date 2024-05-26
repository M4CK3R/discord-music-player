mod link_handler;
mod songs;

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{
    audio_manager::link_handler::LinkHandlerResult,
    cache_manager::{cache_saver::CacheSaver, CacheManager, CacheableSong, CachedEntity},
    common::{Song, SongId},
};

use self::link_handler::LinkHandling;

pub use self::link_handler::StandardLinkHandler;
pub struct AudioManager<CS, LH>
where
    CS: CacheSaver + Send + Sync,
    LH: LinkHandling,
{
    pub cache_manager_instance: Arc<RwLock<CacheManager<CS>>>,
    pub link_handler: LH,
}

impl<CS, LH> AudioManager<CS, LH>
where
    CS: CacheSaver + Send + Sync + 'static,
    LH: LinkHandling,
{
    pub fn new(cache_manager: Arc<RwLock<CacheManager<CS>>>, link_handler: LH) -> Self {
        Self {
            cache_manager_instance: cache_manager,
            link_handler,
        }
    }
    pub async fn handle_link(&mut self, link: &str) -> Result<Vec<Box<dyn Song>>, String> {
        // Read from cache
        let cache_entry = {
            let read = self.cache_manager_instance.read().await;
            read.get_entry(link).cloned()
        };
        if let Some(cached) = cache_entry {
            return self.handle_cached(cached).await;
        }

        let lh_result = self.link_handler.handle_link(link).await?;
        match lh_result {
            LinkHandlerResult::Song(song) => {
                let res = song.clone_song();
                Self::cache_song(self.cache_manager_instance.clone(), link.to_string(), song);
                Ok(vec![res])
            }
            LinkHandlerResult::Playlist(songs) => {
                let mut res = vec![];
                let ids = songs.iter().map(|s| s.get_id().to_string()).collect();
                self.cache_manager_instance
                    .write()
                    .await
                    .add_entry(link.to_string(), CachedEntity::Playlist(ids));
                for song in songs {
                    let s = song.clone_song();
                    Self::cache_song(
                        self.cache_manager_instance.clone(),
                        song.get_id().to_string(),
                        song,
                    );
                    res.push(s);
                }
                Ok(res)
            }
        }
    }

    async fn handle_cached(&self, cached: CachedEntity) -> Result<Vec<Box<dyn Song>>, String> {
        match cached {
            CachedEntity::Song(song) => return Ok(vec![song.clone_song()]),
            CachedEntity::Playlist(song_ids) => {
                let mut res = vec![];
                for id in song_ids {
                    if let Ok(song) = self.handle_song(&id).await {
                        res.push(song);
                    }
                }
                return Ok(res);
            }
        }
    }

    async fn handle_song(&self, song: &SongId) -> Result<Box<dyn Song>, String> {
        if let Some(CachedEntity::Song(song)) =
            self.cache_manager_instance.read().await.get_entry(song)
        {
            return Ok(song.clone_song());
        }
        match self.link_handler.handle_link(song).await? {
            LinkHandlerResult::Song(song) => Ok(song.clone_song()),
            LinkHandlerResult::Playlist(_songs) => {
                unreachable!("Playlist should be handled in handle_song")
            }
        }
    }

    fn cache_song(
        cache_manager_instance: Arc<RwLock<CacheManager<CS>>>,
        id: SongId,
        song: Box<dyn CacheableSong<E = String>>,
    ) {
        // Spawn a thead to cache the song
        tokio::spawn(async move {
            let Ok(cached) = song.cache_song().await else {
                return;
            };

            cache_manager_instance
                .write()
                .await
                .add_entry(id, CachedEntity::Song(cached));
        });
    }
}
