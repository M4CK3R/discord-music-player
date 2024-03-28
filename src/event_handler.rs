use std::sync::Arc;

use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{gateway::Ready, guild::Guild},
};
use tokio::sync::RwLock;
use tracing::Level;

use crate::{
    common::{DiscordQueueManager, DiscordQueueSaver},
    Config,
};

pub(crate) struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
    async fn guild_create(&self, ctx: Context, guild: Guild, _is_new: Option<bool>) {
        let guild_id = guild.id;
        let data = ctx.data.read().await;
        let queue_manager_map = data
            .get::<DiscordQueueManager>()
            .expect("Queue manager not found"); //.write().await;
        if !queue_manager_map.read().await.contains_key(&guild_id) {
            tracing::event!(Level::INFO, "Creating queue manager for guild {}", guild_id);
            let config = data.get::<Config>().expect("Config not found");

            let p = format!("{}/{}", &config.saved_queues_path, &guild_id);
            match std::fs::create_dir_all(&p) {
                Ok(_) => (),
                Err(e) => {
                    tracing::event!(Level::ERROR, "Failed to create dir for saved queues: {}", e);
                    return;
                }
            };
            let queue_saver = DiscordQueueSaver::new(&p);
            queue_manager_map.write().await.insert(
                guild_id,
                Arc::new(RwLock::new(DiscordQueueManager::new(queue_saver))),
            );
        }
    }
}
