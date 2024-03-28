mod audio_manager;
mod cache_manager;
mod commands;
mod common;
mod event_handler;
mod queue_manager;

use audio_manager::StandardLinkHandler;

use common::{DiscordAudioManager, DiscordQueueManager};
use dotenv::dotenv;
use poise::PrefixFrameworkOptions;
use tracing::event;

use std::{collections::HashMap, env, sync::Arc};
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::RwLock,
};

use songbird::SerenityInit;

use serenity::{client::Client, prelude::GatewayIntents};

use crate::{
    common::{CommandError, Config, Data, DiscordCacheManager, DiscordCacheSaver},
    event_handler::Handler,
};

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let prefix = env::var("DISCORD_PREFIX").unwrap_or_else(|_e| "!".to_string());
    let cache_dir = env::var("DISCORD_CACHE_DIR").unwrap_or_else(|_e| "./cache".to_string());

    let framework: poise::Framework<Data, CommandError> = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::ping(),
                commands::join(),
                commands::leave(),
                commands::pause(),
                commands::resume(),
                commands::skip(),
                commands::set_loop(),
                commands::shuffle(),
                commands::show(),
                commands::queue(),
                commands::add(),
                commands::remove(),
                commands::clear(),
                commands::move_song(),
                commands::save(),
                commands::saved(),
                commands::load(),
                commands::remove_saved(),
                commands::help(),
            ],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(prefix.clone()),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands)
                    .await
                    .map_err(|e| CommandError::SerenityError(e))?;
                let create_commands =
                    poise::builtins::create_application_commands(&framework.options().commands);

                serenity::all::Command::set_global_commands(ctx, create_commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<Config>(Config {
            prefix: prefix.clone(),
            token: token.clone(),
            cache_dir: cache_dir.clone(),
            saved_queues_path: cache_dir.clone(),
        });
        data.insert::<DiscordQueueManager>(Arc::new(RwLock::new(HashMap::new())));
        let mut cache_manager = DiscordCacheManager::new(DiscordCacheSaver::new(cache_dir.clone()));
        cache_manager.load_cache();
        let arc_cache_manager = Arc::new(RwLock::new(cache_manager));
        data.insert::<DiscordCacheManager>(arc_cache_manager.clone());
        data.insert::<DiscordAudioManager>(Arc::new(RwLock::new(DiscordAudioManager::new(
            arc_cache_manager.clone(),
            StandardLinkHandler::new(cache_dir, arc_cache_manager.clone()),
        ))));
    }
    let data = client.data.clone();
    tokio::spawn(async move {
        event!(tracing::Level::INFO, "Starting client");
        let _ = client
            .start()
            .await
            .map_err(|why| println!("Client ended: {:?}", why));
    });

    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to create SIGINT signal");
    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to create SIGTERM signal");
    tokio::select! {
        _ = sigint.recv() => {
            event!(tracing::Level::INFO, "Received SIGINT");
        }
        _ = sigterm.recv() => {
            event!(tracing::Level::INFO, "Received SIGTERM");
        }
    }
    let data = data.read().await;
    {
        let cache_manager = data
            .get::<DiscordCacheManager>()
            .expect("Cache manager not found");
        event!(tracing::Level::INFO, "Saving cache");
        let mut cache_manager = cache_manager.write().await;
        cache_manager.save_cache();
    }
    {
        let queue_managers = data
            .get::<DiscordQueueManager>()
            .expect("Queue manager not found");
        event!(tracing::Level::INFO, "Saving queues");
        let queue_managers = queue_managers.read().await;
        for (guild_id, queue_manager) in queue_managers.iter() {
            event!(tracing::Level::INFO, "Saving queue for guild {}", guild_id);
            let queue_manager = queue_manager.read().await;
            queue_manager.save_queues();
        }
    }
    event!(tracing::Level::INFO, "Received Ctrl-C, shutting down.");
}
