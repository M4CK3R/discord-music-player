use std::{fmt::Display, sync::Arc};

use songbird::{
    tracks::{ControlError, TrackHandle},
    Call,
};
use tokio::sync::Mutex;
use tracing::{event, Level};

use crate::common::Song;

pub struct CurrentSong {
    pub song: Box<dyn Song>,
    pub track_handle: TrackHandle,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

impl Clone for CurrentSong {
    fn clone(&self) -> Self {
        CurrentSong {
            song: self.song.clone_song(),
            track_handle: self.track_handle.clone(),
            started_at: self.started_at,
        }
    }
}

#[derive(Clone, Debug)]
pub enum LoopMode {
    Song,
    Queue,
    Repeat(usize),
    None,
}

impl Display for LoopMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoopMode::Song => write!(f, "Song"),
            LoopMode::Queue => write!(f, "Queue"),
            LoopMode::Repeat(n) => write!(f, "Repeat {}", n),
            LoopMode::None => write!(f, "None"),
        }
    }
}

pub struct Player {
    call: Option<Arc<Mutex<Call>>>,
    current_song: Option<CurrentSong>,
    pub loop_mode: LoopMode,
}

impl Player {
    pub fn new() -> Player {
        Player {
            call: None,
            current_song: None,
            loop_mode: LoopMode::None,
        }
    }
    pub fn call_joined(&mut self, driver: Arc<Mutex<Call>>) {
        self.call = Some(driver);
    }
    pub fn call_left(&mut self) -> Option<Arc<Mutex<Call>>> {
        self.call.take()
    }
    pub fn get_call(&self) -> Option<Arc<Mutex<Call>>> {
        self.call.clone()
    }
    pub fn get_current_song(&self) -> Option<CurrentSong> {
        self.current_song.clone()
    }
    pub fn take_current_song(&mut self) -> Result<Box<dyn Song>, ControlError> {
        if let Some(current_song) = self.current_song.take() {
            let res = current_song.track_handle.stop();
            if let Err(e) = res {
                event!(Level::ERROR, "Failed to stop song: {}", e);
            }
            Ok(current_song.song)
        } else {
            Err(ControlError::InvalidTrackEvent)
        }
    }
    pub fn pause(&mut self) -> Result<(), ControlError> {
        if let Some(current_song) = &self.current_song {
            current_song.track_handle.pause()?;
        }
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), ControlError> {
        if let Some(current_song) = &self.current_song {
            current_song.track_handle.play()?;
        }
        Ok(())
    }
    pub async fn play(&mut self, song: Box<dyn Song>) -> Result<(), ControlError> {
        let cs = self.current_song.take();
        if let Some(cs) = cs {
            cs.track_handle.stop()?;
        }
        let call = match &self.call {
            Some(c) => c,
            None => return Err(ControlError::InvalidTrackEvent),
        };
        let t = call.lock().await.play_only_input(song.get_input().await);
        self.current_song = Some(CurrentSong {
            song,
            track_handle: t,
            started_at: chrono::Utc::now(),
        });
        Ok(())
    }
}
