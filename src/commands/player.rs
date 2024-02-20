use std::str::FromStr;

use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::Context,
};
use tracing::{event, Level};

use crate::{
    commands::{
        logger::{messages::SETTING_LOOP_MODE, Logger},
        utils,
    },
    queue_manager::LoopMode,
};

#[command]
#[only_in(guilds)]
#[description = "Pauses the current song"]
#[aliases("p")]
pub async fn pause(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager(guild.id.to_string(), &data, None, None).await?;
    let queue_manager = queue_manager.write().await;
    match queue_manager.pause().await {
        Ok(_) => {
            msg.reply(ctx, "Song paused").await.log_message()?;
        }
        Err(e) => {
            event!(Level::ERROR, "Failed to pause song {}", e);
            msg.reply(ctx, "No song is playing").await.log_message()?;
        }
    };
    Ok(())
}

#[command]
#[only_in(guilds)]
#[description = "Resumes the current song"]
#[aliases("r")]
pub async fn resume(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager(guild.id.to_string(), &data, None, None).await?;
    let queue_manager = queue_manager.write().await;
    match queue_manager.resume().await {
        Ok(_) => {
            msg.reply(ctx, "Song resumed").await.log_message()?;
        }
        Err(e) => {
            event!(Level::ERROR, "Failed to resume song {}", e);
            msg.reply(ctx, "No song is paused").await.log_message()?;
        }
    }
    Ok(())
}

#[command]
#[only_in(guilds)]
#[description = "Skips the current song"]
#[aliases("sk")]
pub async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = utils::get_guild(ctx, msg).await?;
    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager(guild.id.to_string(), &data, None, None).await?;
    queue_manager.write().await.skip().await;
    msg.reply(ctx, "Song skipped").await.log_message()?;
    Ok(())
}

#[command("loop")]
#[description = "Sets the loop mode"]
#[usage = "<mode>"]
#[example = "none"]
#[example = "song"]
#[example = "queue"]
#[example = "5 <-- Repeat the song 5 times"]
pub async fn set_loop(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let loop_mode_string = match args.single::<String>() {
        Ok(loop_mode_string) => loop_mode_string,
        Err(_) => {
            msg.reply(ctx, "You need to specify a loop mode")
                .await
                .log_message()?;
            return Ok(());
        }
    };
    let loop_mode = LoopMode::from_str(&loop_mode_string)
        .log_with_message(SETTING_LOOP_MODE, msg, &ctx.http)
        .await?;
    let guild = utils::get_guild(ctx, msg).await?;
    let data = ctx.data.read().await;
    let queue_manager = utils::get_queue_manager(guild.id.to_string(), &data, None, None).await?;
    queue_manager
        .write()
        .await
        .set_loop(loop_mode.clone())
        .await;
    msg.reply(ctx, format!("Loop mode set to {}", loop_mode))
        .await
        .log_message()?;
    Ok(())
}
