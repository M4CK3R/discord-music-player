use songbird::input::Input;
use async_trait::async_trait;

use crate::{cache_manager::{cache_saver::FileCacheSaver, CacheManager}, audio_manager::{StandardLinkHandler, AudioManager}, queue_manager::{FileQueueSaver, QueueManager}};

pub type SongId = String;

#[async_trait]
pub trait Song: Send + Sync {
    fn title(&self) -> &String;
    fn artist(&self) -> &String;
    fn duration(&self) -> Option<u64>;
    async fn get_input(&self) -> Input;
    fn clone_song(&self) -> Box<dyn Song>;
    fn get_id(&self) -> &SongId;
}

pub type DiscordCacheSaver = FileCacheSaver;
pub type DiscordLinkHandler = StandardLinkHandler<DiscordCacheSaver>;
pub type DiscordAudioManager = AudioManager<DiscordCacheSaver, DiscordLinkHandler>;
pub type DiscordCacheManager = CacheManager<DiscordCacheSaver>;
pub type DiscordQueueSaver = FileQueueSaver;
pub type DiscordQueueManager = QueueManager<DiscordQueueSaver>;