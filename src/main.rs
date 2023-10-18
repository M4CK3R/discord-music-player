mod event_handler;
mod commands;
mod groups;
mod audio_manager;
mod queue_manager;
mod common;

use std::{env, sync::Arc};
use audio_manager::AudioManager;
use dotenv::dotenv;
use event_handler::Handler;

use queue_manager::QueueManager;
// This trait adds the `register_songbird` and `register_songbird_with` methods
// to the client builder below, making it easy to install this voice client.
// The voice client can be retrieved in any command using `songbird::get(ctx).await`.
use songbird::SerenityInit;

use serenity::{
    client::Client,
    framework::StandardFramework,
    prelude::{GatewayIntents, TypeMapKey},
};
use tokio::sync::RwLock;

use crate::groups::GENERAL_GROUP;

// TODO rewrite this as HashMap<String, Arc<RwLock<AudioManager>>>
impl TypeMapKey for AudioManager {
    type Value = Arc<RwLock<AudioManager>>;
}

// TODO rewrite this as HashMap<String, Arc<RwLock<QueueManager>>>
impl TypeMapKey for QueueManager {
    type Value = Arc<RwLock<QueueManager>>;
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();
    
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    let framework = StandardFramework::new()
        .configure(|c| c
                   .prefix("!"))
        .group(&GENERAL_GROUP);

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<AudioManager>(Arc::new(RwLock::new(AudioManager::new())));
        data.insert::<QueueManager>(Arc::new(RwLock::new(QueueManager::new())));
    }

    tokio::spawn(async move {
        let _ = client.start().await.map_err(|why| println!("Client ended: {:?}", why));
    });
    
    tokio::signal::ctrl_c().await.expect("Ups");
    println!("Received Ctrl-C, shutting down.");
}