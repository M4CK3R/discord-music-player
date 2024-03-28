use poise::CreateReply;

use crate::{
    common::{CommandError, Context, DataRegistryError, Error},
    queue_manager::LoopMode,
};

mod bot;
mod player;
mod queue;
mod utils;

/// Ping the bot!
#[poise::command(slash_command, prefix_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

/// Join the voice channel the user is in
#[poise::command(slash_command, prefix_command)]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx.guild().ok_or(CommandError::NotInGuild)?.to_owned();
    let author_id = ctx.author().id;
    let channel_id = guild
        .voice_states
        .get(&author_id)
        .and_then(|voice_state| voice_state.channel_id)
        .ok_or(CommandError::NotInVoiceChannel)?;
    let guild_id = guild.id;
    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or(CommandError::DataRegistry(
            DataRegistryError::SongbirdNotRegistered,
        ))?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    bot::join(channel_id, guild_id, manager, queue_manager).await?;
    let reply = CreateReply::default()
        .content("Joined the voice channel")
        .reply(true)
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

/// Leave the voice channel
#[poise::command(slash_command, prefix_command)]
pub async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    bot::leave(queue_manager).await?;
    let reply = CreateReply::default()
        .content("Left the voice channel")
        .reply(true)
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

/// Pause the current song
#[poise::command(slash_command, prefix_command)]
pub async fn pause(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    player::pause(queue_manager).await?;
    let reply = CreateReply::default()
        .content("Paused the song")
        .reply(true)
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

/// Resume the current song
#[poise::command(slash_command, prefix_command)]
pub async fn resume(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    player::resume(queue_manager).await?;
    let reply = CreateReply::default()
        .content("Resumed the song")
        .reply(true)
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

/// Skip the current song
#[poise::command(slash_command, prefix_command)]
pub async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    let skipped = player::skip(queue_manager).await?;
    let reply = CreateReply::default()
        .content(format!("Skipped {}", skipped.title()))
        .reply(true)
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

/// Set the loop mode
/// available modes: none, song, queue
#[poise::command(slash_command, prefix_command, rename = "loop")]
pub async fn set_loop(
    ctx: Context<'_>,
    #[description = "Loop mode to set"] loop_mode: LoopMode,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    player::set_loop(queue_manager, loop_mode.clone()).await?;
    let reply = CreateReply::default()
        .content(format!("Loop mode set to {:?}", loop_mode))
        .reply(true)
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

/// Shuffle the queue
#[poise::command(slash_command, prefix_command)]
pub async fn shuffle(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    queue::shuffle(queue_manager).await?;
    let reply = CreateReply::default()
        .content("Shuffled the queue")
        .reply(true)
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

/// Show the current song
/// if saved_queue_name is provided, show the contents of the saved queue
#[poise::command(slash_command, prefix_command)]
pub async fn show(ctx: Context<'_>, saved_queue_name: Option<String>) -> Result<(), Error> {
    if let Some(name) = saved_queue_name {
        return list_saved(ctx, name).await;
    }
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    let e = queue::show(queue_manager).await?;
    let reply = CreateReply::default().embed(e).reply(true).ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

pub async fn list_saved(ctx: Context<'_>, name: String) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    let audio_manager = utils::get_audio_manager(ctx).await?;
    let saved_queue = {
        let queue_manager = queue_manager.read().await;
        queue_manager
            .get_saved_queue(name)
            .ok_or(CommandError::EmptyQueue)?
    };
    let songs = {
        let mut audio_manager = audio_manager.write().await;
        let mut res = vec![];
        for s in saved_queue {
            let song = audio_manager.handle_link(&s).await;
            if let Ok(mut song) = song {
                res.append(&mut song);
            }
        }
        res
    };
    let embeds = queue::list_songs(songs);
    if embeds.is_empty() {
        let reply = CreateReply::default()
            .content("The queue is empty")
            .reply(true)
            .ephemeral(true);
        ctx.send(reply).await?;
    }
    for e in embeds {
        let reply = CreateReply::default().reply(true).ephemeral(true).embed(e);
        ctx.send(reply).await?;
    }
    Ok(())
}

/// Show the queue
#[poise::command(slash_command, prefix_command)]
pub async fn queue(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    let embeds = queue::queue(queue_manager).await?;
    if embeds.is_empty() {
        let reply = CreateReply::default()
            .content("The queue is empty")
            .reply(true)
            .ephemeral(true);
        ctx.send(reply).await?;
    }
    for e in embeds {
        let reply = CreateReply::default().reply(true).ephemeral(true).embed(e);
        ctx.send(reply).await?;
    }
    Ok(())
}

/// Add a song to the queue
#[poise::command(slash_command, prefix_command)]
pub async fn add(ctx: Context<'_>, url: String) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    let audio_manager = utils::get_audio_manager(ctx).await?;
    let reply = CreateReply::default()
        .content("Adding song to the queue (it may take a while)")
        .reply(true)
        .ephemeral(true);
    let r = ctx.send(reply).await?;
    let n = queue::add(queue_manager, audio_manager, url).await?;
    let reply = CreateReply::default()
        .content(format!("Added {} song(s) to the queue", n))
        .reply(true)
        .ephemeral(true);
    r.edit(ctx, reply).await?;
    Ok(())
}

/// Remove a song from the queue
#[poise::command(slash_command, prefix_command)]
pub async fn remove(ctx: Context<'_>, index: usize) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    let song = queue::remove(queue_manager, index).await?;
    let reply = CreateReply::default()
        .content(format!("Removed song: {index}. {}", song.title()))
        .reply(true)
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

/// Clear the queue
#[poise::command(slash_command, prefix_command)]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    queue::clear(queue_manager).await?;
    let reply = CreateReply::default()
        .content("Cleared the queue")
        .reply(true)
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

/// Move a song in the queue from one index to another index (1-based)
#[poise::command(slash_command, prefix_command, rename = "move")]
pub async fn move_song(
    ctx: Context<'_>,
    #[min = 1] from: usize,
    #[min = 1] to: usize,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    queue::move_song(queue_manager, from - 1, to - 1).await?;
    let reply = CreateReply::default()
        .content(format!("Moved song from {from} to {to}"))
        .reply(true)
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

/// Save the queue
#[poise::command(slash_command, prefix_command)]
pub async fn save(ctx: Context<'_>, name: String) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    queue::save(queue_manager, &name).await?;
    let reply = CreateReply::default()
        .content(format!("Saved the queue as {name}"))
        .reply(true)
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

/// List the saved queues
#[poise::command(slash_command, prefix_command)]
pub async fn saved(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    let embeds = queue::saved(queue_manager).await?;
    if embeds.is_empty() {
        let reply = CreateReply::default()
            .content("There are no saved queues")
            .reply(true)
            .ephemeral(true);
        ctx.send(reply).await?;
    }
    for e in embeds {
        let reply = CreateReply::default().reply(true).ephemeral(true).embed(e);
        ctx.send(reply).await?;
    }
    Ok(())
}

/// Load a saved queue
#[poise::command(slash_command, prefix_command)]
pub async fn load(ctx: Context<'_>, name: String) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    let audio_manager = utils::get_audio_manager(ctx).await?;
    let n = queue::load(queue_manager, audio_manager, &name).await?;
    let reply = CreateReply::default()
        .content(format!("Loaded {n} song(s) from {name}"))
        .reply(true)
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

/// Remove a saved queue
#[poise::command(slash_command, prefix_command)]
pub async fn remove_saved(ctx: Context<'_>, name: String) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or(CommandError::NotInGuild)?;
    let queue_manager = utils::get_queue_manager(ctx, &guild_id).await?;
    queue::remove_saved(queue_manager, &name).await?;
    let reply = CreateReply::default()
        .content(format!("Removed saved queue {name}"))
        .reply(true)
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}

// TODO: make it prettier
/// Help command
#[poise::command(slash_command, prefix_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Command to get help for"] command: Option<String>,
) -> Result<(), Error> {
    if command.is_some() {
        let cfg = poise::samples::HelpConfiguration {
            ephemeral: true,
            show_subcommands: true,
            show_context_menu_commands: false,
            include_description: true,
            ..Default::default()
        };
        poise::builtins::help(ctx, command.as_deref(), cfg).await?;
        return Ok(());
    }
    let command_list = &ctx.framework().options().commands;
    let commands = command_list
        .iter()
        .map(|c| c.name.clone())
        .collect::<Vec<_>>();
    let reply = CreateReply::default()
        .content(format!("Available commands: {}", commands.join(", ")))
        .reply(true)
        .ephemeral(true);
    ctx.send(reply).await?;
    Ok(())
}
