use std::sync::Arc;

use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{gateway::Ready, guild::Guild},
};
use tokio::sync::RwLock;

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
        let guild_id = guild.id.to_string();
        let data = ctx.data.read().await;
        let queue_manager_map = data.get::<DiscordQueueManager>().expect("Queue manager not found");//.write().await;
        if !queue_manager_map.read().await.contains_key(&guild_id) {
            let config = data.get::<Config>().expect("Config not found");
            let queue_saver = DiscordQueueSaver::new(&config._saved_queues_path);
            queue_manager_map.write().await.insert(
                guild_id,
                Arc::new(RwLock::new(DiscordQueueManager::new(queue_saver))),
            );
        }
    }
}
