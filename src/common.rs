use std::{fmt::Display, sync::Arc, collections::HashMap};

use async_trait::async_trait;
use serenity::all::GuildId;
use songbird::{error::{ControlError, JoinError}, input::Input, typemap::TypeMapKey};
use tokio::sync::RwLock;

use crate::{
    audio_manager::{AudioManager, StandardLinkHandler},
    cache_manager::{cache_saver::FileCacheSaver, CacheManager},
    queue_manager::{FileQueueSaver, QueueManager},
};

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

pub struct Config {
    pub prefix: String,
    pub token: String,
    pub cache_dir: String,
    pub saved_queues_path: String,
}

impl TypeMapKey for Config {
    type Value = Config;
}

#[derive(Clone, Debug)]
pub struct Data {}
impl Display for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Data")
    }
}

pub type Error = CommandError;
pub type Context<'a> = poise::Context<'a, Data, Error>;

impl TypeMapKey for DiscordQueueManager {
    type Value = Arc<RwLock<HashMap<GuildId, Arc<RwLock<DiscordQueueManager>>>>>;
}
impl TypeMapKey for DiscordAudioManager {
    type Value = Arc<RwLock<DiscordAudioManager>>;
}

impl TypeMapKey for DiscordCacheManager {
    type Value = Arc<RwLock<DiscordCacheManager>>;
}


#[derive(Debug)]
pub enum CommandError {
    SerenityError(serenity::Error),
    SongbirdError(SongbirdError),
    NotInVoiceChannel,
    NoSongPlaying,
    LinkHandling(String),
    InvalidIndex(usize),
    EmptyQueue,
    NotInGuild,
    DataRegistry(DataRegistryError)
}

impl Display for CommandError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::SerenityError(e) => write!(f, "Serenity error: {}", e),
            CommandError::SongbirdError(e) => write!(f, "Songbird error: {}", e),
            CommandError::NotInVoiceChannel => write!(f, "Not in a voice channel"),
            CommandError::NoSongPlaying => write!(f, "No song is currently playing"),
            CommandError::LinkHandling(l) => write!(f, "Link handling error: {}", l),
            CommandError::InvalidIndex(i) => write!(f, "Invalid index: {}", i),
            CommandError::EmptyQueue =>  write!(f, "Queue is empty"),
            CommandError::NotInGuild => write!(f, "Not in a guild"),
            CommandError::DataRegistry(e) => write!(f, "Data registry error: {}", e),
        }
    }
}

impl From<serenity::Error> for CommandError {
    fn from(error: serenity::Error) -> Self {
        CommandError::SerenityError(error)
    }
}

impl From<ControlError> for CommandError {
    fn from(error: ControlError) -> Self {
        CommandError::SongbirdError(error.into())
    }
}

#[derive(Debug)]
pub enum SongbirdError {
    JoinError(JoinError),
    ControlError(ControlError),
}

impl Display for SongbirdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SongbirdError::JoinError(e) => write!(f, "Join error: {}", e),
            SongbirdError::ControlError(e) => write!(f, "Control error: {}", e)
        }
    }
}

impl From<JoinError> for SongbirdError {
    fn from(error: JoinError) -> Self {
        SongbirdError::JoinError(error)
    }
}

impl From<ControlError> for SongbirdError {
    fn from(error: ControlError) -> Self {
        SongbirdError::ControlError(error)
    }
}

#[derive(Debug)]
pub enum DataRegistryError{
    QueueManagerNotRegistered,
    SongbirdNotRegistered,
    AudioManagerNotRegistered,
}

impl Display for DataRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataRegistryError::QueueManagerNotRegistered => write!(f, "Queue manager not registered"),
            DataRegistryError::SongbirdNotRegistered => write!(f, "Songbird not registered"),
            DataRegistryError::AudioManagerNotRegistered => write!(f, "Audio manager not registered"),
        }
    }
}