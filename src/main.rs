mod audio_manager;
mod cache_manager;
mod commands;
mod common;
mod event_handler;
mod groups;
mod queue_manager;

use audio_manager::StandardLinkHandler;

use common::{DiscordAudioManager, DiscordQueueManager};
use dotenv::dotenv;
use tracing::event;

use std::{
    collections::{HashMap, HashSet},
    env,
    sync::Arc,
};
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::RwLock,
};

use songbird::SerenityInit;

use serenity::{
    builder::CreateMessage,
    client::{Client, Context},
    framework::{
        standard::{
            help_commands, macros::help, Args, CommandGroup, CommandResult, Configuration,
            HelpOptions,
        },
        StandardFramework,
    },
    model::{channel::Message, id::UserId, Color},
    prelude::{GatewayIntents, TypeMapKey},
};

use crate::{
    common::{DiscordCacheManager, DiscordCacheSaver},
    event_handler::Handler,
    groups::{CACHE_GROUP, GENERAL_GROUP, PLAYER_GROUP, QUEUE_GROUP},
};

impl TypeMapKey for DiscordQueueManager {
    type Value = Arc<RwLock<HashMap<String, Arc<RwLock<DiscordQueueManager>>>>>;
}
impl TypeMapKey for DiscordAudioManager {
    type Value = Arc<RwLock<DiscordAudioManager>>;
}
// pub type DiscordCacheManager = CacheManager<DiscordCacheSaver>;
impl TypeMapKey for DiscordCacheManager {
    type Value = Arc<RwLock<DiscordCacheManager>>;
}

struct Config {
    _prefix: String,
    _token: String,
    _cache_dir: String,
    _saved_queues_path: String,
}

impl TypeMapKey for Config {
    type Value = Config;
}

fn help_embed(title: &str, fields: &[(&[&str], &str)]) -> serenity::builder::CreateEmbed {
    let flds = fields.iter().map(|&(names, desc)| {
        let primary_name = names.first().unwrap();
        let aliases = names.iter().skip(1).map(|n| *n).collect::<Vec<_>>();
        let title = if aliases.is_empty() {
            format!("{}", primary_name)
        } else {
            format!("{} ({})", primary_name, aliases.join(", "))
        };
        (title, desc, true)
    });
    let embed = serenity::builder::CreateEmbed::default()
        .title(title)
        .fields(flds)
        .color(Color::DARK_GREEN);
    embed
}

#[help("help", "h", "?", "co", "nani")]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    if !args.is_empty() {
        let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
        return Ok(());
    }
    let g = groups
        .iter()
        .map(|g| {
            (
                g.name,
                g.options
                    .commands
                    .iter()
                    .map(|c| (c.options.names, c.options.desc.unwrap_or("")))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    let embeds = g
        .iter()
        .map(|(group, commands)| help_embed(&group, commands))
        .collect::<Vec<_>>();
    let _ = msg
        .channel_id
        .send_message(&context.http, CreateMessage::new().add_embeds(embeds))
        .await;
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let prefix = env::var("DISCORD_PREFIX").unwrap_or_else(|_e| "!".to_string());
    let cache_dir = env::var("DISCORD_CACHE_DIR").unwrap_or_else(|_e| "./cache".to_string());

    let framework = StandardFramework::new()
        .help(&MY_HELP)
        .unrecognised_command(commands::bot::unrecognised_command)
        .group(&GENERAL_GROUP)
        .group(&CACHE_GROUP)
        .group(&PLAYER_GROUP)
        .group(&QUEUE_GROUP);
    framework.configure(Configuration::new().prefix(&prefix));

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
            _prefix: prefix.clone(),
            _token: token.clone(),
            _cache_dir: cache_dir.clone(),
            _saved_queues_path: format!("{}/saved_queues.json", cache_dir),
        });
        data.insert::<DiscordQueueManager>(Arc::new(RwLock::new(HashMap::new())));
        let mut cache_manager = DiscordCacheManager::new(DiscordCacheSaver::new(cache_dir.clone()));
        cache_manager.load_cache();
        let arc_cache_manager = Arc::new(RwLock::new(cache_manager));
        data.insert::<DiscordAudioManager>(Arc::new(RwLock::new(DiscordAudioManager::new(
            arc_cache_manager.clone(),
            StandardLinkHandler::new(cache_dir, arc_cache_manager.clone()),
        ))));
        data.insert::<DiscordCacheManager>(arc_cache_manager.clone());
    }
    let data = client.data.clone();
    tokio::spawn(async move {
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
        let mut cache_manager = cache_manager.write().await;
        cache_manager.save_cache();
    }
    {
        let queue_managers = data
            .get::<DiscordQueueManager>()
            .expect("Queue manager not found");
        let queue_managers = queue_managers.read().await;
        for (_, queue_manager) in queue_managers.iter() {
            let queue_manager = queue_manager.read().await;
            queue_manager.save_queues();
        }
    }
    event!(tracing::Level::INFO, "Received Ctrl-C, shutting down.");
}
