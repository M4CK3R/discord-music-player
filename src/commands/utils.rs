use std::{str::FromStr, sync::Arc};

use chrono::{Duration, Utc};
use serenity::{
    builder::CreateEmbed,
    client::Context,
    http::Http,
    model::{channel::Message, guild::Guild, id::ChannelId, Color},
};
use songbird::{typemap::TypeMap, Songbird};
use tokio::sync::RwLock;

use crate::{
    commands::logger::messages::GETTING_QUEUE_MANAGER,
    common::{DiscordAudioManager, DiscordQueueManager, Song},
    queue_manager::LoopMode,
};

use super::logger::{
    messages::{GETTING_CHANNEL, GETTING_GUILD, GETTING_QUEUE_MANAGER_MAP, GETTING_VOICE_MANAGER},
    Logger,
};

static _PROGRESS_BAR_LENGTH: usize = 20;
static _PROGRESS_BAR_FILL: &str = "▮";
static _PROGRESS_BAR_EMPTY: &str = "▯";

pub(crate) async fn get_guild(ctx: &Context, msg: &Message) -> Result<Guild, String> {
    msg.guild(&ctx.cache)
        .ok_or("Guild not found".to_string())
        .map(|g| g.to_owned()) // TODO: remove clone
        .log_with_message(GETTING_GUILD, msg, &ctx.http)
        .await
}

pub(crate) async fn get_channel_id(
    guild: &Guild,
    msg: &Message,
    http: &Arc<Http>,
) -> Result<ChannelId, String> {
    guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id)
        .ok_or("Channel not found".to_string())
        .log_with_message(GETTING_CHANNEL, msg, http)
        .await
}

pub(crate) async fn get_manager(ctx: &Context, msg: &Message) -> Result<Arc<Songbird>, String> {
    songbird::get(ctx)
        .await
        .ok_or("Songbird not found".to_string())
        .log_with_message(GETTING_VOICE_MANAGER, msg, &ctx.http)
        .await
}

pub(crate) async fn get_queue_manager(
    guild_id: String,
    data: &TypeMap,
    msg: Option<&Message>,
    http: Option<&Arc<Http>>,
) -> Result<Arc<RwLock<DiscordQueueManager>>, String> {
    let queue_manager_map = data
        .get::<DiscordQueueManager>()
        .ok_or("Queue manager not found".to_string())
        .try_log_with_message(GETTING_QUEUE_MANAGER_MAP, msg, http)
        .await?;
    let queue_manager = queue_manager_map.read().await;
    let result = queue_manager
        .get(&guild_id)
        .ok_or("Queue manager not found".to_string())
        .try_log_with_message(GETTING_QUEUE_MANAGER, msg, http)
        .await?
        .clone();
    Ok(result)
}

pub(crate) async fn get_audio_manager(
    data: &TypeMap,
    msg: Option<&Message>,
    http: Option<&Arc<Http>>,
) -> Result<Arc<RwLock<DiscordAudioManager>>, String> {
    let audio_manager = data
        .get::<DiscordAudioManager>()
        .ok_or("Audio manager not found".to_string())
        .try_log_with_message(GETTING_QUEUE_MANAGER, msg, http)
        .await?;
    Ok(audio_manager.to_owned())
}

impl FromStr for LoopMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        match s.as_str() {
            "none" => Ok(LoopMode::None),
            "song" => Ok(LoopMode::Song),
            "queue" => Ok(LoopMode::Queue),
            x => {
                if let Ok(n) = x.parse() {
                    return Ok(LoopMode::Repeat(n));
                }
                Err("Invalid loop mode")
            }
        }
    }
}

fn get_progress_bar(
    song_duration: i64,
    duration_played: i64,
    progress_bar_length: usize,
    progress_bar_fill: &str,
    progress_bar_empty: &str,
) -> String {
    let fill_amount = (duration_played as f32 / song_duration as f32 * progress_bar_length as f32)
        .round() as usize;

    format!(
        "[{}{}]",
        progress_bar_fill.repeat(fill_amount),
        progress_bar_empty.repeat(progress_bar_length - fill_amount)
    )
}

pub(crate) fn build_current_song_embed(
    song: Box<dyn Song>,
    duration_played: i64,
    progress_bar_length: usize,
    progress_bar_fill: &str,
    progress_bar_empty: &str,
) -> CreateEmbed {
    let (overall_duration, timestamp, progress_bar) = match song.duration() {
        Some(song_duration) => {
            let song_duration_i = song_duration as i64;
            let overall_duration = format_duration(&Duration::seconds(song_duration_i));
            let timestamp = Utc::now() + Duration::seconds(song_duration_i - duration_played);
            let progress_bar = get_progress_bar(
                song_duration_i,
                duration_played,
                progress_bar_length,
                progress_bar_fill,
                progress_bar_empty,
            );
            (overall_duration, timestamp, progress_bar)
        }
        None => ("∞".to_string(), Utc::now(), "∞".to_string()),
    };

    let duration = Duration::seconds(duration_played as i64);

    let duration_formatted = format!("{}/{}", format_duration(&duration), &overall_duration);

    return CreateEmbed::default()
        .title("Currently playing")
        .color(Color::from_rgb(255, 0, 0))
        .timestamp(timestamp)
        .field(
            song.title(),
            format!("{}\n{progress_bar} {duration_formatted}", song.artist()),
            false,
        )
        .to_owned();
}

pub(crate) fn format_duration(d: &Duration) -> String {
    let h_string = if d.num_hours() > 0 {
        format!("{:02}:", d.num_hours())
    } else {
        "".to_string()
    };
    format!(
        "{h_string}{:02}:{:02}",
        d.num_minutes() % 60,
        d.num_seconds() % 60
    )
}
