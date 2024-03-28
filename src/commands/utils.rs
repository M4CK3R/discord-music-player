use std::sync::Arc;

use serenity::all::GuildId;
use tokio::sync::RwLock;

use crate::common::{
    CommandError, Context, DataRegistryError, DiscordAudioManager, DiscordQueueManager,
};

static _PROGRESS_BAR_LENGTH: usize = 20;
static _PROGRESS_BAR_FILL: &str = "▮";
static _PROGRESS_BAR_EMPTY: &str = "▯";

pub async fn get_queue_manager(
    ctx: Context<'_>,
    guild_id: &GuildId,
) -> Result<Arc<RwLock<DiscordQueueManager>>, CommandError> {
    let context = ctx.serenity_context();
    let data = context.data.read().await;
    let map = data
        .get::<DiscordQueueManager>()
        .ok_or(CommandError::DataRegistry(
            DataRegistryError::QueueManagerNotRegistered,
        ))?
        .read()
        .await;
    map.get(guild_id)
        .ok_or(CommandError::DataRegistry(
            DataRegistryError::QueueManagerNotRegistered,
        ))
        .cloned()
}

pub async fn get_audio_manager(
    ctx: Context<'_>,
) -> Result<Arc<RwLock<DiscordAudioManager>>, CommandError> {
    let context = ctx.serenity_context();
    let data = context.data.read().await;
    data.get::<DiscordAudioManager>()
        .ok_or(CommandError::DataRegistry(
            DataRegistryError::AudioManagerNotRegistered,
        ))
        .cloned()
}
