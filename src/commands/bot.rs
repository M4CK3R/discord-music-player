use std::sync::Arc;

use serenity::
    all::{ChannelId, GuildId}
;
use songbird::Songbird;
use tokio::sync::RwLock;

use crate::common::{CommandError, DiscordQueueManager};

pub async fn join(
    channel_id: ChannelId,
    guild_id: GuildId,
    manager: Arc<Songbird>,
    queue_manager: Arc<RwLock<DiscordQueueManager>>,
) -> Result<(), CommandError> {
    let call = manager
        .join(guild_id, channel_id)
        .await
        .map_err(|e| CommandError::SongbirdError(e.into()))?;
    match DiscordQueueManager::call_joined(queue_manager.into(), call).await {
        Ok(_) => Ok(()),
        Err(e) => Err(CommandError::SongbirdError(e.into())),
    }
}

pub async fn leave(
    queue_manager: Arc<RwLock<DiscordQueueManager>>,
) -> Result<(), CommandError> {
    let call = queue_manager.write().await.call_left().await;
    match call {
        Some(call) => {
            call.lock()
                .await
                .leave()
                .await
                .map_err(|e| CommandError::SongbirdError(e.into()))?;
            Ok(())
        }
        None => Err(CommandError::NotInVoiceChannel),
    }
}

