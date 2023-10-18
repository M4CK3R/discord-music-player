use std::str::FromStr;

use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::Context,
};

use crate::{
    commands::{
        logger::{
            messages::{
                GETTING_GUILD, PAUSING_SONG, PLAYING_SONG, RESUMING_SONG, SETTING_LOOP_MODE,
                SKIPPING_SONG, STOPPING_SONG,
            },
            Logger,
        },
        utils,
    },
    queue_manager::queue::LoopMode,
};

#[command]
#[only_in(guilds)]
pub async fn play(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager.write().await;

    queue_manager
        .play(guild.id)
        .await
        .log_with_message(PLAYING_SONG, msg, &ctx.http)
        .await?;

    msg.channel_id
        .say(&ctx.http, "Playing song")
        .await
        .log_message()?;
    Ok(())
}

#[command]
#[only_in(guilds)]
pub async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager.write().await;
    queue_manager
        .stop(guild.id)
        .await
        .log_with_message(STOPPING_SONG, msg, &ctx.http)
        .await?;

    msg.channel_id
        .say(&ctx.http, "Stopped song")
        .await
        .log_message()?;
    Ok(())
}

#[command]
#[only_in(guilds)]
pub async fn pause(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager.write().await;
    queue_manager
        .pause(guild.id)
        .await
        .log_with_message(PAUSING_SONG, msg, &ctx.http)
        .await?;

    msg.channel_id
        .say(&ctx.http, "Paused song")
        .await
        .log_message()?;
    Ok(())
}

#[command]
#[only_in(guilds)]
pub async fn resume(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager.write().await;
    queue_manager
        .resume(guild.id)
        .await
        .log_with_message(RESUMING_SONG, msg, &ctx.http)
        .await?;

    msg.channel_id
        .say(&ctx.http, "Resumed song")
        .await
        .log_message()?;
    Ok(())
}

#[command]
#[only_in(guilds)]
pub async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager.write().await;
    queue_manager
        .skip(guild.id)
        .await
        .log_with_message(SKIPPING_SONG, msg, &ctx.http)
        .await?;

    msg.channel_id
        .say(&ctx.http, "Skipped song")
        .await
        .log_message()?;
    Ok(())
}

#[command("loop")]
pub async fn set_loop(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild = utils::get_guild(ctx, msg)
        .ok_or("Guild not found")
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await?;

    let loop_mode_string = args.single::<String>().unwrap();
    let loop_mode = LoopMode::from_str(&loop_mode_string)
        .log_with_message(SETTING_LOOP_MODE, msg, &ctx.http)
        .await?;

    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager_lock(&data);
    let mut queue_manager = queue_manager.write().await;
    queue_manager.set_loop(guild.id, loop_mode).await;

    msg.channel_id
        .say(&ctx.http, format!("Loop mode set to {}", loop_mode_string))
        .await
        .log_message()?;
    Ok(())
}
