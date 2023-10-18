use chrono::Duration;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::Context,
    utils::{Color, Colour},
};

use crate::commands::{
    logger::{
        messages::{
            ADDING_SONG, GETTING_INDEX, GETTING_NAME, GETTING_URL, MOVING_SONG, REMOVING_SONG,
            SHUFFLING_QUEUE,
        },
        Logger,
    },
    utils,
};

use super::logger::messages::GETTING_GUILD;

static MAX_EMBED_FIELD_COUNT: usize = 25;

#[command]
#[only_in(guilds)]
pub async fn shuffle(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager.write().await;
    queue_manager
        .shuffle(guild.id)
        .await
        .log_with_message(SHUFFLING_QUEUE, msg, &ctx.http)
        .await?;

    msg.channel_id
        .say(&ctx.http, "Shuffled queue")
        .await
        .log_message()?;
    return Ok(());
}

#[command]
#[only_in(guilds)]
pub async fn show(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let queue_manager = queue_manager.read().await;
    let current_song_info = queue_manager
        .get_current_song(guild.id)
        .await
        .ok_or("Could not get current song")
        .log_with_message("No song is playing", msg, &ctx.http)
        .await?;

    let (current_song, duration_played) = current_song_info;

    let embed = utils::build_current_song_embed(current_song, duration_played);

    msg.channel_id
        .send_message(&ctx.http, |m| m.set_embed(embed))
        .await
        .log_message()?;

    return Ok(());
}

#[command]
#[only_in(guilds)]
pub async fn queue(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let queue_manager = queue_manager.read().await;
    let queue = queue_manager.get_song_list(guild.id);

    let fields = queue
        .await
        .iter()
        .enumerate()
        .map(|(i, song)| {
            let d = song.duration().unwrap_or(u32::MAX);
            let d = Duration::seconds(d as i64);
            (
                format!("{}. {}", i + 1, song.title()),
                format!("{} {}", song.artist(), utils::format_duration(&d)),
                false,
            )
        })
        .collect::<Vec<(String, String, bool)>>();

    if fields.len() <= MAX_EMBED_FIELD_COUNT {
        let embed = utils::build_embed("Queue", Color::DARK_GREEN, fields);
        msg.channel_id
            .send_message(&ctx.http, |m| m.set_embed(embed))
            .await
            .log_message()?;
        return Ok(());
    }

    let embeds = fields
        .chunks(MAX_EMBED_FIELD_COUNT)
        .enumerate()
        .map(|(i, fields_chunk)| {
            utils::build_embed(
                format!("Queue {}", i + 1),
                Color::DARK_GREEN,
                fields_chunk.to_vec(),
            )
        })
        .collect::<Vec<_>>();

    for embed in embeds {
        let _ = msg
            .channel_id
            .send_message(&ctx.http, |m| m.set_embed(embed))
            .await
            .log_message();
    }
    return Ok(());
}

#[command]
#[only_in(guilds)]
pub async fn add(ctx: &Context, msg: &Message, mut _args: Args) -> CommandResult {
    let url = match _args.single::<String>() {
        Ok(url) => Ok(url),
        Err(_) => Err("Url not found or was invalid"),
    }
    .log_with_message(GETTING_URL, msg, &ctx.http)
    .await?;

    let data = ctx.data.read().await;
    let audio_manager = utils::get_audio_manager_lock(&data);
    let mut audio_manager = audio_manager.write().await;
    let songs = audio_manager
        .handle_link(&url)
        .await
        .log_with_message(ADDING_SONG, msg, &ctx.http)
        .await?;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager.write().await;
    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;
    let songs_added = songs.len();
    let queue_size = queue_manager.add(guild.id, songs).await;
    msg.channel_id
        .say(
            &ctx.http,
            format!(
                "Added {} songs to queue\nTotal: {}",
                songs_added, queue_size
            ),
        )
        .await
        .log_message()?;
    let r = queue_manager.play(guild.id).await;
    if r.is_ok() {
        msg.channel_id
            .say(&ctx.http, "Playing song")
            .await
            .log_message()?;
    }
    return Ok(());
}

#[command]
#[only_in(guilds)]
pub async fn remove(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = match args.single::<usize>() {
        Ok(index) => Ok(index - 1),
        Err(_) => Err("Index not found or was invalid"),
    }
    .log_with_message(GETTING_INDEX, msg, &ctx.http)
    .await?;

    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager.write().await;
    let song = queue_manager
        .remove(guild.id, index)
        .await
        .log_with_message(REMOVING_SONG, msg, &ctx.http)
        .await?;
    msg.channel_id
        .say(&ctx.http, format!("Removed {}", song.title()))
        .await
        .log_message()?;

    return Ok(());
}

#[command]
#[only_in(guilds)]
pub async fn clear(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager.write().await;
    queue_manager.clear(guild.id).await;
    msg.channel_id
        .say(&ctx.http, "Cleared queue")
        .await
        .log_message()?;
    return Ok(());
}

#[command("move")]
#[only_in(guilds)]
pub async fn move_song(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let index1 = match args.single::<usize>() {
        Ok(index) => Ok(index - 1),
        Err(_) => Err("Index not found or was invalid"),
    }
    .log_with_message("Index 'from' was invalid", msg, &ctx.http)
    .await?;

    let index2 = match args.single::<usize>() {
        Ok(index) => Ok(index - 1),
        Err(_) => Err("Index not found or was invalid"),
    }
    .log_with_message("Index 'to' was invalid", msg, &ctx.http)
    .await?;

    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;
    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager.write().await;
    let song = queue_manager
        .move_song(guild.id, index1, index2)
        .await
        .log_with_message(MOVING_SONG, msg, &ctx.http)
        .await?;
    msg.channel_id
        .say(&ctx.http, format!("Moved {}", song.title()))
        .await
        .log_message()?;
    return Ok(());
}

#[command("save")]
#[only_in(guilds)]
pub async fn save(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let name = match args.single::<String>() {
        Ok(name) => Ok(name),
        Err(_) => Err("Name not found"),
    }
    .log_with_message(GETTING_NAME, msg, &ctx.http)
    .await?;

    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;
    let data = ctx.data.read().await;
    let queue_manager_lock = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager_lock.write().await;
    let mut queue = queue_manager.get_song_list(guild.id).await;
    if let Some((cs, _d)) = queue_manager.get_current_song(guild.id).await {
        queue.insert(0, cs);
    }
    queue_manager.save_queue(guild.id, name, queue);

    msg.channel_id
        .say(&ctx.http, "Saving queue")
        .await
        .log_message()?;

    return Ok(());
}

#[command("saved")]
#[only_in(guilds)]
pub async fn saved(ctx: &Context, msg: &Message, mut _args: Args) -> CommandResult {
    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let queue_manager = queue_manager.read().await;
    let queues = queue_manager
        .get_saved_queues(guild.id)
        .ok_or("No saved queues found")
        .log_with_message("No saved queues found", msg, &ctx.http)
        .await?;

    let fields = queues
        .iter()
        .map(|name| {
            let name = name.clone();
            let value = String::new();
            (name, value, false)
        })
        .collect::<Vec<_>>();

    if fields.len() <= MAX_EMBED_FIELD_COUNT {
        let embed = utils::build_embed("Saved queues", Colour::BLITZ_BLUE, fields);
        msg.channel_id
            .send_message(&ctx.http, |m| m.set_embed(embed))
            .await
            .log_message()?;
        return Ok(());
    }

    let embeds = fields
        .chunks(MAX_EMBED_FIELD_COUNT)
        .enumerate()
        .map(|(i, fields_chunk)| {
            utils::build_embed(
                format!("Saved queues {}", i + 1),
                Color::DARK_GREEN,
                fields_chunk.to_vec(),
            )
        })
        .collect::<Vec<_>>();

    for embed in embeds {
        msg.channel_id
            .send_message(&ctx.http, |m| m.set_embed(embed))
            .await
            .log_message()?;
    }
    Ok(())
}

#[command("load")]
#[only_in(guilds)]
pub async fn load(ctx: &Context, msg: &Message, mut _args: Args) -> CommandResult {
    let name = match _args.single::<String>() {
        Ok(name) => Ok(name),
        Err(_) => Err("Name not found"),
    }
    .log_with_message(GETTING_NAME, msg, &ctx.http)
    .await?;

    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let data = ctx.data.read().await;
    let queue_manager_lock = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager_lock.write().await;
    let queue = queue_manager
        .get_saved_queue(guild.id, &name)
        .ok_or("No saved queue found")
        .log_with_message("No saved queue found", msg, &ctx.http)
        .await?;
    let audio_manager_lock = utils::get_audio_manager_lock(&data);
    let audio_manager = audio_manager_lock.read().await;
    let songs = audio_manager.load_songs(queue);
    queue_manager.clear(guild.id).await;
    let s = queue_manager.add(guild.id, songs).await;

    msg.channel_id
        .say(&ctx.http, format!("Loaded queue ({name}:{s})"))
        .await
        .log_message()?;

    let r = queue_manager.play(guild.id).await;
    if r.is_ok() {
        msg.channel_id
            .say(&ctx.http, "Playing song")
            .await
            .log_message()?;
    }
    Ok(())
}


#[command("remove_saved")]
#[aliases("rms")]
#[only_in(guilds)]
pub async fn remove_saved(ctx: &Context, msg: &Message, mut _args: Args) -> CommandResult {
    let name = match _args.single::<String>() {
        Ok(name) => Ok(name),
        Err(_) => Err("Name not found"),
    }
    .log_with_message(GETTING_NAME, msg, &ctx.http)
    .await?;

    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let data = ctx.data.read().await;
    let queue_manager_lock = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager_lock.write().await;
    queue_manager
        .remove_saved_queue(guild.id, &name)
        .ok_or("No saved queue found")
        .log_with_message("No saved queue found", msg, &ctx.http)
        .await?;

    msg.channel_id
        .say(&ctx.http, "Removed saved queue")
        .await
        .log_message()?;
    Ok(())
}