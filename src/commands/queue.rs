use std::num::ParseIntError;

use chrono::{Duration, Utc};
use serenity::{
    builder::{CreateEmbed, CreateMessage},
    framework::standard::{macros::command, ArgError, Args, CommandResult},
    model::{channel::Message, Color},
    prelude::Context,
};
use tracing::{event, Level};

use crate::{
    commands::{logger::Logger, utils},
    common::Song,
};

static MAX_EMBED_FIELD_COUNT: usize = 25;
static PROGRESS_BAR_LENGTH: usize = 20;
static PROGRESS_BAR_FILL: &str = "▮";
static PROGRESS_BAR_EMPTY: &str = "▯";

#[command]
#[only_in(guilds)]
#[description = "Shuffles the queue"]
#[aliases("sh")]
#[usage = ""]
pub async fn shuffle(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager(guild.id.to_string(), &data, None, None).await?;
    queue_manager.write().await.shuffle().await;
    msg.reply(ctx, "Shuffled the queue").await.log_message()?;
    Ok(())
}

#[command]
#[only_in(guilds)]
#[description = "Shows the current song"]
#[aliases("s")]
#[usage = ""]
pub async fn show(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager(guild.id.to_string(), &data, None, None).await?;
    let queue_manager = queue_manager.read().await;
    let current_song = match queue_manager.get_current_song().await {
        Some(song) => song,
        None => {
            msg.reply(&ctx.http, "No song is playing")
                .await
                .log_message()?;
            return Ok(());
        }
    };
    let duration = Utc::now().signed_duration_since(current_song.started_at);
    let embed = utils::build_current_song_embed(
        current_song.song,
        duration.num_seconds(),
        PROGRESS_BAR_LENGTH,
        PROGRESS_BAR_FILL,
        PROGRESS_BAR_EMPTY,
    );

    msg.channel_id
        .send_message(&ctx.http, CreateMessage::new().add_embed(embed))
        .await
        .log_message()?;
    Ok(())
}

fn map_song((i, song): (usize, &Box<dyn Song>)) -> (String, String, bool) {
    let d = match song.duration() {
        Some(d) => utils::format_duration(&Duration::seconds(d as i64)),
        None => "".to_string(),
    };
    (
        format!("{}. {}", i + 1, song.title()),
        format!("{} {}", song.artist(), d),
        false,
    )
}

#[command]
#[only_in(guilds)]
#[description = "Shows the queue"]
#[aliases("q")]
#[usage = ""]
pub async fn queue(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let data = ctx.data.read().await;
    let queue_manager =
        utils::get_queue_manager(guild.id.to_string(), &data, Some(&msg), Some(&ctx.http)).await?;
    let queue_manager = queue_manager.read().await;
    let queue = queue_manager.get_queue().await;
    let fields = queue
        .iter()
        .enumerate()
        .map(map_song)
        .collect::<Vec<(String, String, bool)>>();
    let embeds = if fields.len() <= MAX_EMBED_FIELD_COUNT {
        vec![CreateEmbed::default()
            .title("Queue")
            .color(Color::DARK_GREEN)
            .fields(fields)
            .to_owned()]
    } else {
        fields
            .chunks(MAX_EMBED_FIELD_COUNT)
            .enumerate()
            .map(|(i, fields_chunk)| {
                CreateEmbed::default()
                    .title(format!("Queue {}", i + 1))
                    .color(Color::DARK_GREEN)
                    .fields(fields_chunk.to_vec())
                    .to_owned()
            })
            .collect::<Vec<CreateEmbed>>()
    };
    for embed in embeds {
        msg.channel_id
            .send_message(&ctx.http, CreateMessage::new().add_embed(embed))
            .await
            .log_message()?;
    }
    Ok(())
}

#[command]
#[only_in(guilds)]
#[description = "Adds a song to the queue"]
#[aliases("a", "play")]
#[usage = "<link>"]
#[example = "https://www.youtube.com/watch?v=6n3pFFPSlW4"]
pub async fn add(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let link = match args.single::<String>() {
        Ok(link) => link,
        Err(_) => {
            msg.reply(ctx, "You need to specify a link")
                .await
                .log_message()?;
            return Ok(());
        }
    };
    let data = ctx.data.read().await;
    let songs = {
        let audio_manager = utils::get_audio_manager(&data, Some(&msg), Some(&ctx.http)).await?;
        let mut audio_manager = audio_manager.write().await;
        match audio_manager.handle_link(&link).await {
            Ok(songs) => songs,
            Err(e) => {
                event!(Level::ERROR, "Failed to handle link {}", e);
                msg.reply(ctx, "Invalid link").await.log_message()?;
                return Ok(());
            }
        }
    };
    let n = songs.len();
    {
        let queue_manager =
            utils::get_queue_manager(guild.id.to_string(), &data, Some(&msg), Some(&ctx.http))
                .await?;
        let queue_manager = queue_manager.write().await;
        match queue_manager.add_to_queue(songs).await {
            Ok(_) => (),
            Err(e) => {
                event!(Level::ERROR, "Failed to play song {}", e);
                msg.reply(ctx, "Failed when trying to play song")
                    .await
                    .log_message()?;
                return Ok(());
            }
        }
    }

    msg.reply(ctx, format!("Added {} songs to the queue", n))
        .await
        .log_message()?;
    Ok(())
}

#[command]
#[only_in(guilds)]
#[description = "Removes a song from the queue"]
#[aliases("rm")]
#[usage = "<index>"]
#[example = "1"]
pub async fn remove(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let index = match sanitize_index(args.single::<usize>()) {
        Ok(index) => index,
        Err(e) => {
            msg.reply(ctx, e).await.log_message()?;
            return Ok(());
        }
    };
    let data = ctx.data.read().await;
    let queue_manager =
        utils::get_queue_manager(guild.id.to_string(), &data, Some(&msg), Some(&ctx.http)).await?;
    let queue_manager = queue_manager.write().await;
    let song = match queue_manager.remove_from_queue_by_index(index).await {
        Some(song) => song,
        None => {
            msg.reply(ctx, "Invalid index").await.log_message()?;
            return Ok(());
        }
    };
    msg.reply(ctx, format!("Removed {}", song.title()))
        .await
        .log_message()?;
    Ok(())
}

#[command]
#[only_in(guilds)]
#[description = "Clears the queue"]
#[aliases("c")]
#[usage = ""]
pub async fn clear(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let data = ctx.data.read().await;
    let queue_manager =
        utils::get_queue_manager(guild.id.to_string(), &data, Some(&msg), Some(&ctx.http)).await?;
    let mut queue_manager = queue_manager.write().await;
    queue_manager.clear_queue().await;
    msg.reply(ctx, "Cleared the queue").await.log_message()?;
    Ok(())
}

#[command("move")]
#[only_in(guilds)]
#[description = "Moves a song in the queue"]
#[aliases("mv")]
#[usage = "<index1> <index2>"]
#[example = "1 3"]
pub async fn move_song(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let index1 = match sanitize_index(args.single::<usize>()) {
        Ok(index) => index,
        Err(e) => {
            msg.reply(ctx, e).await.log_message()?;
            return Ok(());
        }
    };
    let index2 = match sanitize_index(args.single::<usize>()) {
        Ok(index) => index,
        Err(e) => {
            msg.reply(ctx, e).await.log_message()?;
            return Ok(());
        }
    };
    let data = ctx.data.read().await;
    let queue_manager =
        utils::get_queue_manager(guild.id.to_string(), &data, Some(&msg), Some(&ctx.http)).await?;
    let queue_manager = queue_manager.write().await;
    queue_manager.swap(index1, index2).await?;
    msg.reply(ctx, "Swapped the songs").await.log_message()?;
    Ok(())
}

fn sanitize_index(index: Result<usize, ArgError<ParseIntError>>) -> Result<usize, String> {
    let index = index.map_err(|_e| "You need to specify the first index".to_string())?;
    if index <= 0 {
        return Err("Index must be greater than 0".to_string());
    }
    Ok(index - 1)
}

#[command("save")]
#[only_in(guilds)]
#[description = "Saves the queue"]
#[aliases("sv")]
#[usage = "<name>"]
#[example = "my_queue"]
pub async fn save(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let name = match args.single::<String>() {
        Ok(name) => name,
        Err(_) => {
            msg.reply(ctx, "You need to specify a name")
                .await
                .log_message()?;
            return Ok(());
        }
    };
    let data = ctx.data.read().await;
    let queue_manager =
        utils::get_queue_manager(guild.id.to_string(), &data, Some(&msg), Some(&ctx.http)).await?;
    let mut queue_manager = queue_manager.write().await;
    match queue_manager.add_saved_queue(name).await {
        Ok(_) => msg.reply(ctx, "Saved the queue").await.log_message()?,
        Err(_e) => msg
            .reply(ctx, "Cannot save empty queue")
            .await
            .log_message()?,
    };
    Ok(())
}

#[command("saved")]
#[only_in(guilds)]
#[description = "Shows the saved queues"]
#[aliases("svd")]
#[usage = ""]
pub async fn saved(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let data = ctx.data.read().await;
    let queue_manager =
        utils::get_queue_manager(guild.id.to_string(), &data, Some(&msg), Some(&ctx.http)).await?;
    let queue_manager = queue_manager.read().await;
    let saved_queues = queue_manager.list_saved_queues();

    let fields = saved_queues
        .iter()
        .map(|s| (s.to_string(), "".to_string(), false))
        .collect::<Vec<_>>();
    let embeds = if fields.len() <= MAX_EMBED_FIELD_COUNT {
        vec![CreateEmbed::default()
            .title("Saved queues")
            .color(Color::BLITZ_BLUE)
            .fields(fields)
            .to_owned()]
    } else {
        fields
            .chunks(MAX_EMBED_FIELD_COUNT)
            .enumerate()
            .map(|(i, fields_chunk)| {
                CreateEmbed::default()
                    .title(format!("Saved queues {}", i + 1))
                    .color(Color::BLITZ_BLUE)
                    .fields(fields_chunk.to_vec())
                    .to_owned()
            })
            .collect::<Vec<CreateEmbed>>()
    };
    for embed in embeds {
        msg.channel_id
            .send_message(&ctx.http, CreateMessage::new().add_embed(embed))
            .await
            .log_message()?;
    }
    Ok(())
}

#[command("load")]
#[only_in(guilds)]
#[description = "Loads a saved queue"]
#[aliases("ld")]
#[usage = "<name>"]
#[example = "my_queue"]
pub async fn load(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let name = match args.single::<String>() {
        Ok(name) => name,
        Err(_) => {
            msg.reply(ctx, "You need to specify a name")
                .await
                .log_message()?;
            return Ok(());
        }
    };
    let data = ctx.data.read().await;
    let queue_manager =
        utils::get_queue_manager(guild.id.to_string(), &data, Some(&msg), Some(&ctx.http)).await?;
    let queue = {
        let queue_manager = queue_manager.read().await;

        match queue_manager.get_saved_queue(name) {
            Some(queue) => queue,
            None => {
                msg.reply(ctx, "Queue not found").await.log_message()?;
                return Ok(());
            }
        }
    };
    let songs = {
        let audio_manager = utils::get_audio_manager(&data, Some(&msg), Some(&ctx.http)).await?;
        let audio_manager = audio_manager.read().await;
        audio_manager.get_cached_songs(&queue).await
    };
    let n = songs.len();
    if n == 0 {
        msg.reply(ctx, "Queue is empty").await.log_message()?;
        return Ok(());
    }
    let queue_manager = queue_manager.write().await;
    match queue_manager.add_to_queue(songs).await {
        Ok(_) => (),
        Err(e) => {
            event!(Level::ERROR, "Failed to play song {}", e);
            msg.reply(ctx, "Failed when trying to play song")
                .await
                .log_message()?;
            return Ok(());
        }
    };
    msg.reply(ctx, format!("Added {} songs to the queue", n))
        .await
        .log_message()?;

    Ok(())
}

#[command("remove_saved")]
#[only_in(guilds)]
#[aliases("rms")]
#[description = "Removes a saved queue"]
#[usage = "<name>"]
#[example = "my_queue"]
pub async fn remove_saved(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let name = match args.single::<String>() {
        Ok(name) => name,
        Err(_) => {
            msg.reply(ctx, "You need to specify a name")
                .await
                .log_message()?;
            return Ok(());
        }
    };
    let data = ctx.data.read().await;
    let queue_manager =
        utils::get_queue_manager(guild.id.to_string(), &data, Some(&msg), Some(&ctx.http)).await?;
    let mut queue_manager = queue_manager.write().await;
    queue_manager.remove_saved_queue(name);
    msg.reply(ctx, "Removed the saved queue")
        .await
        .log_message()?;
    Ok(())
}
