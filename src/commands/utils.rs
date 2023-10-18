use std::{sync::Arc, fmt::Display, error::Error, str::FromStr};

use chrono::{Duration, Utc};
use serenity::{
    model::prelude::{ChannelId, Guild, Message},
    prelude::{Context, TypeMap}, utils::Color, builder::CreateEmbed,
};
use songbird::Songbird;
use tokio::sync::RwLock;

use crate::{audio_manager::AudioManager, queue_manager::{QueueManager, queue::LoopMode}};
use crate::common::Song;

static PROGRESS_BAR_LENGTH: usize = 20;
static PROGRESS_BAR_FILL: &str = "▮";
static PROGRESS_BAR_EMPTY: &str = "▯";

pub(crate) fn get_audio_manager_lock(data: &TypeMap) -> Arc<RwLock<AudioManager>> {
    data.get::<AudioManager>()
        .expect("Expected AudioManager in TypeMap.")
        .clone()
}

pub(crate) fn get_queue_manager_lock(data: &TypeMap) -> Arc<RwLock<QueueManager>> {
    data.get::<QueueManager>()
        .expect("Expected QueueManager in TypeMap.")
        .clone()
}

pub(crate) fn get_guild(ctx: &Context, msg: &Message) -> Option<Guild> {
    msg.guild(&ctx.cache)
}

pub(crate) fn get_channel_id(guild: &Guild, msg: &Message) -> Option<ChannelId> {
    guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id)
}

pub(crate) async fn get_manager(ctx: &Context) -> Arc<Songbird> {
    songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone()
}

pub(crate) fn build_embed<S: ToString>(
    title: S,
    colour: Color,
    fields: Vec<(String, String, bool)>,
) -> CreateEmbed {
    CreateEmbed::default()
        .title(title)
        .colour(colour)
        .fields(fields)
        .to_owned()
}

pub(crate) fn build_current_song_embed(song: Box<dyn Song>, duration_played: f32) -> CreateEmbed {
    let song_duration = match song.duration(){
        Some(n) => n as f32,
        None => f32::MAX,
    }; // TODO handle this differently
    let timestamp = Utc::now() + Duration::seconds((song_duration - duration_played) as i64);
    let fill_amount = (duration_played as f32 / song_duration as f32
        * PROGRESS_BAR_LENGTH as f32)
        .round() as usize;
    let progress_bar = format!(
        "[{}{}]",
        PROGRESS_BAR_FILL.repeat(fill_amount),
        PROGRESS_BAR_EMPTY.repeat(PROGRESS_BAR_LENGTH - fill_amount)
    );

    let duration = Duration::seconds(duration_played as i64);
    let overall_duration = Duration::seconds(song_duration as i64);

    let duration_formatted = format!(
        "{}/{}",
        format_duration(&duration),
        format_duration(&overall_duration)
    );

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

#[derive(Debug)]
pub(crate) struct CommandError{
    msg: String,
}

impl Display for CommandError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl Error for CommandError {
    
}

impl From<String> for CommandError{
    fn from(msg: String) -> Self {
        CommandError{
            msg,
        }
    }
}

impl From<&str> for CommandError{
    fn from(msg: &str) -> Self {
        CommandError{
            msg: msg.to_string(),
        }
    }
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