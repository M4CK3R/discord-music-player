use std::sync::Arc;

use chrono::{Duration, Utc};
use serenity::all::{Color, CreateEmbed};
use tokio::sync::RwLock;

use crate::common::{CommandError, DiscordAudioManager, DiscordQueueManager, Song};

pub async fn shuffle(queue_manager: Arc<RwLock<DiscordQueueManager>>) -> Result<(), CommandError> {
    let queue_manager = queue_manager.write().await;
    queue_manager.shuffle().await;
    Ok(())
}

static MAX_EMBED_FIELD_COUNT: usize = 25;
static PROGRESS_BAR_LENGTH: usize = 20;
static PROGRESS_BAR_FILL: &str = "▮";
static PROGRESS_BAR_EMPTY: &str = "▯";

fn get_progress_bar(
    song_duration: i64,
    duration_played: i64,
    progress_bar_length: usize,
    progress_bar_fill: &str,
    progress_bar_empty: &str,
) -> String {
    let fill_amount = (duration_played as f32 / song_duration as f32 * progress_bar_length as f32)
        .round() as usize;

    let empty_amount = if fill_amount > progress_bar_length {
        0
    } else {
        progress_bar_length - fill_amount
    };

    format!(
        "[{}{}]",
        progress_bar_fill.repeat(fill_amount),
        progress_bar_empty.repeat(empty_amount)
    )
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

pub async fn show(
    queue_manager: Arc<RwLock<DiscordQueueManager>>,
) -> Result<CreateEmbed, CommandError> {
    let queue_manager = queue_manager.read().await;
    let current_song = match queue_manager.get_current_song().await {
        Some(song) => song,
        None => return Err(CommandError::NoSongPlaying),
    };
    let elapsed = Utc::now()
        .signed_duration_since(current_song.started_at)
        .num_seconds() as u64;
    let song_duration = current_song.song.duration();
    let timestamp = match song_duration {
        Some(duration) => {
            let d = if elapsed > duration as u64 {
                Duration::seconds(0)
            } else {
                Duration::seconds((duration - elapsed) as i64)
            };
            Utc::now() + d
        }
        None => Utc::now(),
    };

    let progress_bar = get_progress_bar(
        song_duration.unwrap_or(0) as i64,
        elapsed as i64,
        PROGRESS_BAR_LENGTH,
        PROGRESS_BAR_FILL,
        PROGRESS_BAR_EMPTY,
    );

    let duration = match song_duration {
        Some(duration) => format_duration(&Duration::seconds(duration as i64)),
        None => "??".to_string(),
    };

    let embed = CreateEmbed::default()
        .title("Currently Playing")
        .color(Color::from_rgb(255, 0, 0))
        .timestamp(timestamp)
        .field(
            current_song.song.title(),
            format!(
                "{}\n{} {}/{}",
                progress_bar,
                current_song.song.artist(),
                duration,
                format_duration(&Duration::seconds(elapsed as i64))
            ),
            false,
        );
    Ok(embed)
}

fn map_song((i, song): (usize, &Box<dyn Song>)) -> (String, String, bool) {
    let d = match song.duration() {
        Some(d) => format_duration(&Duration::seconds(d as i64)),
        None => "".to_string(),
    };
    (
        format!("{}. {}", i + 1, song.title()),
        format!("{} {}", song.artist(), d),
        false,
    )
}

pub async fn queue(
    queue_manager: Arc<RwLock<DiscordQueueManager>>,
) -> Result<Vec<CreateEmbed>, CommandError> {
    let queue_manager = queue_manager.read().await;
    let queue = queue_manager.get_queue().await;

    Ok(list_songs(queue))
}

pub fn list_songs(queue: Vec<Box<dyn Song>>) -> Vec<CreateEmbed> {
    let fields = queue
        .iter()
        .enumerate()
        .map(map_song)
        .collect::<Vec<(String, String, bool)>>();
    let fields = fields.chunks(MAX_EMBED_FIELD_COUNT);
    let n = fields.len();
    let title = |i: usize| {
        if n > 1 {
            format!("Queue ({}/{})", i + 1, n)
        } else {
            "Queue".to_string()
        }
    };
    let embeds = fields
        .enumerate()
        .map(|(i, chunk)| {
            CreateEmbed::default()
                .title(title(i))
                .color(Color::from_rgb(255, 0, 0))
                .fields(chunk.to_vec())
        })
        .collect::<Vec<CreateEmbed>>();
    embeds
}

pub async fn add(
    queue_manager: Arc<RwLock<DiscordQueueManager>>,
    audio_manager: Arc<RwLock<DiscordAudioManager>>,
    link: String,
) -> Result<usize, CommandError> {
    let songs = {
        let mut audio_manager = audio_manager.write().await;
        audio_manager
            .handle_link(&link)
            .await
            .map_err(|_e| CommandError::LinkHandling("Some error".to_string()))?
    };
    let n = songs.len();
    let queue_manager = queue_manager.write().await;
    match queue_manager.add_to_queue(songs).await {
        Ok(_) => Ok(n),
        Err(e) => Err(e.into()),
    }
}

pub async fn remove(
    queue_manager: Arc<RwLock<DiscordQueueManager>>,
    index: usize,
) -> Result<Box<dyn Song>, CommandError> {
    let queue_manager = queue_manager.write().await;
    queue_manager
        .remove_from_queue_by_index(index)
        .await
        .ok_or(CommandError::InvalidIndex(index))
}

pub async fn clear(queue_manager: Arc<RwLock<DiscordQueueManager>>) -> Result<(), CommandError> {
    let mut queue_manager = queue_manager.write().await;
    queue_manager.clear_queue().await;
    Ok(())
}

pub async fn move_song(
    queue_manager: Arc<RwLock<DiscordQueueManager>>,
    from: usize,
    to: usize,
) -> Result<(), CommandError> {
    let queue_manager = queue_manager.write().await;
    queue_manager
        .swap(from, to)
        .await
        .map_err(|e| CommandError::InvalidIndex(e))?;
    Ok(())
}

pub async fn save(
    queue_manager: Arc<RwLock<DiscordQueueManager>>,
    name: &String,
) -> Result<(), CommandError> {
    let mut queue_manager = queue_manager.write().await;
    queue_manager
        .add_saved_queue(name)
        .await
        .map_err(|_e| CommandError::EmptyQueue)?;
    Ok(())
}

pub async fn saved(
    queue_manager: Arc<RwLock<DiscordQueueManager>>,
) -> Result<Vec<CreateEmbed>, CommandError> {
    let queue_manager = queue_manager.read().await;
    let fields = queue_manager.list_saved_queues();
    let fields = fields.chunks(MAX_EMBED_FIELD_COUNT).map(|chunk| {
        chunk
            .iter()
            .map(|name| (name.clone(), "", false))
            .collect::<Vec<_>>()
    });
    let n = fields.len();
    let title = |i: usize| {
        if n > 1 {
            format!("Saved queues ({}/{})", i + 1, n)
        } else {
            "Saved queues".to_string()
        }
    };
    let embeds = fields
        .enumerate()
        .map(|(i, chunk)| {
            CreateEmbed::default()
                .title(title(i))
                .color(Color::from_rgb(255, 0, 0))
                .fields(chunk)
        })
        .collect::<Vec<CreateEmbed>>();
    Ok(embeds)
}

pub async fn load(
    queue_manager: Arc<RwLock<DiscordQueueManager>>,
    audio_manager: Arc<RwLock<DiscordAudioManager>>,
    name: &String,
) -> Result<usize, CommandError> {
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
    let n = songs.len();
    let queue_manager = queue_manager.write().await;
    queue_manager.add_to_queue(songs).await?;
    Ok(n)
}

pub async fn remove_saved(
    queue_manager: Arc<RwLock<DiscordQueueManager>>,
    name: &String,
) -> Result<(), CommandError> {
    let mut queue_manager = queue_manager.write().await;
    queue_manager.remove_saved_queue(name);
    Ok(())
}
