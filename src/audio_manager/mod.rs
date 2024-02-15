mod link_handler;
mod songs;

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{
    cache_manager::{cache_saver::CacheSaver, CacheManager},
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
    CS: CacheSaver + Send + Sync,
    LH: LinkHandling,
{
    pub fn new(
        cache_manager_instance: Arc<RwLock<CacheManager<CS>>>,
        link_handler_instance: LH,
    ) -> AudioManager<CS, LH> {
        AudioManager {
            cache_manager_instance,
            link_handler: link_handler_instance,
        }
    }

    pub async fn _is_cached(&self, id: &SongId) -> bool {
        let cache_manager_read = self.cache_manager_instance.read().await;
        cache_manager_read.is_cached(id)
    }

    pub async fn handle_link(&mut self, link: &String) -> Result<Vec<Box<dyn Song>>, String> {
        if self._is_cached(link).await{
            let cache_manager_read = self.cache_manager_instance.read().await;
            let songs = cache_manager_read.get_song(link);
            match songs {
                Some(songs) => return Ok(songs.iter().map(|s| s.clone_song()).collect()),
                None => (),
            }
        }
        self.link_handler.handle_link(link).await
    }

    pub async fn get_cached_songs(&self, ids: &Vec<SongId>) -> Vec<Box<dyn Song>> {
        let cache_manager_read = self.cache_manager_instance.read().await;
        ids.
        iter()
        .filter_map(|id| cache_manager_read.get_song(id))
        .flatten()
        .map(|s| s.clone_song())
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::RwLock;

    use crate::{
        audio_manager::link_handler::NullLinkHandler,
        cache_manager::{cache_saver::MemoryCacheSaver, CacheManager},
    };

    use super::AudioManager;

    #[tokio::test]
    async fn test_is_cached_false() {
        let cache_manager = Arc::new(RwLock::new(CacheManager::new(MemoryCacheSaver::new())));

        let audio_manager = AudioManager::new(cache_manager, NullLinkHandler {});
        let id = "test".to_string();
        assert_eq!(audio_manager._is_cached(&id).await, false);
    }
}
