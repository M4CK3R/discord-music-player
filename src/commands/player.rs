use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{common::{CommandError, DiscordQueueManager, Song}, queue_manager::LoopMode};


pub async fn pause(queue_manager: Arc<RwLock<DiscordQueueManager>>) -> Result<(), CommandError> {
    let queue_manager = queue_manager.write().await;
    match queue_manager.pause().await {
        Ok(_) => Ok(()),
        Err(e) => Err(CommandError::SongbirdError(e.into())),
    }
}


pub async fn resume(queue_manager: Arc<RwLock<DiscordQueueManager>>) -> Result<(), CommandError> {
    let queue_manager = queue_manager.write().await;
    match queue_manager.resume().await {
        Ok(_) => Ok(()),
        Err(e) => Err(CommandError::SongbirdError(e.into())),
    }
}

pub async fn skip(queue_manager: Arc<RwLock<DiscordQueueManager>>) -> Result<Box<dyn Song>, CommandError> {
    let queue_manager = queue_manager.write().await;
    let cs = queue_manager.skip().await;
    cs.map_err(|e| CommandError::SongbirdError(e.into()))
}

pub async fn set_loop(
    queue_manager: Arc<RwLock<DiscordQueueManager>>,
    loop_mode: LoopMode,
) -> Result<(), CommandError> {
    let queue_manager = queue_manager.write().await;
    queue_manager.set_loop(loop_mode).await;
    Ok(())
}