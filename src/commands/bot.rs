use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
    prelude::Context,
    utils::Colour,
};

use crate::commands::{
    logger::{
        messages::{
            GETTING_CHANNEL, GETTING_GUILD, JOINING_CHANNEL, LEAVING_CHANNEL,
        },
        Logger,
    },
    utils,
};

#[command]
#[only_in(guilds)]
pub async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.channel_id.say(&ctx.http, "Pong!").await.log_message()?;
    Ok(())
}

#[command]
#[only_in(guilds)]
pub async fn help(ctx: &Context, msg: &Message) -> CommandResult {
    let commands = vec![
        ("join", "Join the voice channel", "!join"),
        ("leave", "Leave the voice channel", "!leave"),
        ("play", "Start playing", "!play"),
        ("pause", "Pause the current song", "!pause"),
        ("resume", "Resume the current song", "!resume"),
        ("skip", "Skip the current song", "!skip"),
        ("stop", "Stop playing", "!stop"),
        ("queue", "Show the current queue", "!queue"),
        ("add", "Add a song to the queue", "!add <url>"),
        ("remove", "Remove a song from the queue", "!remove <index>"),
        ("clear", "Clear the queue", "!clear"),
        ("move", "Move a song in the queue", "!move_song <from> <to>"),
        (
            "loop",
            "Set the loop mode",
            "!set_loop [queue|song|number|none]",
        ),
        ("shuffle", "Shuffle the queue", "!shuffle"),
        ("show", "Show the current song", "!show"),
        ("ping", "Ping the bot", "!ping"),
        ("help", "Show this message", "!help"),
        (
            "todo",
            "NOT A COMMAND\n1.Messages when an error occurs",
            "-",
        ),
    ];
    let c = commands
        .iter()
        .map(|command| {
            (
                command.0.to_string(),
                format!("!{}\nUsage: `{}`", command.1, command.2),
                true,
            )
        })
        .collect();
    let embed = utils::build_embed("Help", Colour::DARK_GREEN, c);
    msg.channel_id
        .send_message(&ctx.http, |m| m.set_embed(embed))
        .await
        .log_message()?;
    Ok(())
}

#[command]
#[only_in(guilds)]
pub async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let channel_id = utils::get_channel_id(&guild, msg)
        .ok_or("Channel not found")
        .log_with_message(GETTING_CHANNEL, msg, &ctx.http)
        .await?;

    let manager = utils::get_manager(ctx).await;

    let (call, res) = manager.join(guild.id, channel_id).await;

    res.log_with_message(JOINING_CHANNEL, msg, &ctx.http)
        .await?;

    let data = ctx.data.write().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager.write().await;
    let mut c = call.lock().await;
    queue_manager.set_driver(guild.id, Some(&mut c)).await;

    msg.channel_id
        .say(&ctx.http, "Joined channel")
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

#[command]
#[only_in(guilds)]
pub async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let manager = utils::get_manager(ctx).await;

    manager
        .leave(guild.id)
        .await
        .log_with_message(LEAVING_CHANNEL, msg, &ctx.http)
        .await?;

    let data = ctx.data.write().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager.write().await;
    let _ = queue_manager.stop(guild.id).await;
    let _ = queue_manager.set_driver(guild.id, None).await;
    Ok(())
}
