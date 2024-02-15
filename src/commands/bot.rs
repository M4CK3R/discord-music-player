use serenity::{
    framework::standard::{
        macros::{command, hook},
        CommandResult,
    },
    model::channel::Message,
    prelude::Context,
};
use tracing::{event, Level};

use crate::{
    commands::{
        logger::{messages::JOINING_CHANNEL, Logger},
        utils,
    },
    common::DiscordQueueManager,
};

#[command]
#[only_in(guilds)]
#[description = "Pings the bot"]
#[aliases("pong")]
pub async fn ping(_ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(&_ctx.http, "Pong!").await.log_message()?;
    Ok(())
}

#[command]
#[only_in(guilds)]
#[description = "Joins the voice channel you are in"]
#[aliases("connect", "j")]
pub async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let channel_id = utils::get_channel_id(&guild, msg, &ctx.http).await?;
    let manager = utils::get_manager(ctx, msg).await?;
    let call = manager
        .join(guild.id, channel_id)
        .await
        .log_with_message(JOINING_CHANNEL, msg, &ctx.http)
        .await?;
    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager(guild.id.to_string(), &data, None, None).await?;
    // queue_manager.write().await.call_joined(call).await;
    match DiscordQueueManager::call_joined(queue_manager.into(), call).await {
        Ok(_) => (),
        Err(e) => {
            event!(Level::ERROR, "Failed to join voice channel: {}", e);
            let _ = msg
                .reply(&ctx.http, format!("Failed to join voice channel: {}", e))
                .await
                .log_message();
        }
    };
    Ok(())
}

#[command]
#[only_in(guilds)]
#[description = "Leaves the voice channel"]
#[aliases("disconnect", "l", "papa", "dobranoc")]
pub async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager(guild.id.to_string(), &data, None, None).await?;
    let call = queue_manager.write().await.call_left().await;

    match call {
        Some(call) => {
            call.lock().await.leave().await?;
            msg.reply(&ctx.http, "Left the voice channel")
                .await
                .log_message()?;
        }
        None => {
            msg.reply(&ctx.http, "Not in a voice channel")
                .await
                .log_message()?;
        }
    }
    Ok(())
}

#[hook]
pub async fn unrecognised_command(ctx: &Context, msg: &Message, unrecognised_command_name: &str) {
    let _ = msg
        .reply(
            &ctx,
            format!("Unrecognised command: {}", unrecognised_command_name),
        )
        .await
        .log_message();
}
