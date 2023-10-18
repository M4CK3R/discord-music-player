use std::{collections::VecDeque, sync::Arc};

use async_trait::async_trait;
use rand::seq::SliceRandom;
use songbird::{tracks::TrackHandle, Call, EventHandler, Event, EventContext, TrackEvent};
use tokio::sync::RwLock;

use crate::common::Song;

#[derive(Clone, Copy)]
pub enum LoopMode {
    None,
    Song,
    Queue,
    Repeat(i32),
}

pub(crate) struct Queue {
    current_song: Arc<RwLock<Option<(Box<dyn Song>, TrackHandle)>>>,
    songs: Arc<RwLock<VecDeque<Box<dyn Song>>>>,
    driver: Arc<RwLock<Option<Call>>>,
    loop_mode: Arc<RwLock<LoopMode>>,
}


impl Clone for Queue {
    fn clone(&self) -> Self {
        Self {
            songs: self.songs.clone(),
            current_song: self.current_song.clone(),
            driver: self.driver.clone(),
            loop_mode: self.loop_mode.clone(),
        }
    }
}

impl Queue {
    pub(crate) async fn add(&mut self, songs: Vec<Box<dyn Song>>) -> usize {
        let mut songs_lock = self.songs.write().await;
        for song in songs{
            songs_lock.push_back(song);
        }
        songs_lock.len()
    }

    pub(crate) fn new() -> Queue {
        Queue {
            songs: Arc::new(RwLock::new(VecDeque::new())),
            current_song: Arc::new(RwLock::new(None)),
            driver: Arc::new(RwLock::new(None)),
            loop_mode: Arc::new(RwLock::new(LoopMode::None)),
        }
    }

    pub(crate) async fn play(&self) -> Result<(), String> {
        {
            let current_song = self.current_song.read().await;
            if current_song.is_some() {
                return Ok(());
            }
        }
        let mut driver = self.driver.write().await;
        let driver = match driver.as_mut() {
            Some(driver) => driver,
            None => return Err("No driver".into()),
        };
        let mut songs = self.songs.write().await;
        let song = match songs.pop_front(){
            Some(song) => song,
            None => return Err("No songs in queue".into()),
        };

        let source = song.get_source().await;
            
        let handle = driver.play_only_source(source);        
        let mut current_song = self.current_song.write().await;
        current_song.replace((song, handle));
        Ok(())
    }

    pub(crate) async fn stop(&self) -> Result<(), String> {
        let mut current_song = self.current_song.write().await;
        if current_song.is_none() {
            return Err("No song is playing".into());
        }
        let (_song, track_handle) = current_song.as_mut().unwrap();
        let res = track_handle.stop();
        if res.is_err() {
            return Err("Failed to stop song".into());
        }
        current_song.take();
        Ok(())
    }

    pub(crate) async fn pause(&self) -> Result<(), String> {
        let mut current_song = self.current_song.write().await;
        if current_song.is_none() {
            return Err("No song is playing".into());
        }
        let (_song, track_handle) = current_song.as_mut().unwrap();
        let res = track_handle.pause();
        if res.is_err() {
            return Err("Failed to pause song".into());
        }
        Ok(())
    }

    pub(crate) async fn resume(&self) -> Result<(), String> {
        let mut current_song = self.current_song.write().await;
        if current_song.is_none() {
            return Err("No song is playing".into());
        }
        let (_song, track_handle) = current_song.as_mut().unwrap();
        let res = track_handle.play();
        if res.is_err() {
            return Err("Failed to resume song".into());
        }
        Ok(())
    }

    pub(crate) async fn skip(&self) -> Result<(), String> {
        let mut current_song = self.current_song.write().await;
        if current_song.is_none() {
            return Err("No song is playing".into());
        }
        let (_song, track_handle) = current_song.as_mut().unwrap();
        let res = track_handle.stop();
        if res.is_err() {
            return Err("Failed to skip song".into());
        }
        Ok(())
    }

    pub(crate) async fn set_loop(&mut self, lp: LoopMode) {
        let mut loop_mode = self.loop_mode.write().await;
        *loop_mode = lp;
    }

    pub(crate) async fn shuffle(&mut self) -> Result<(), String> {
        let mut songs = self.songs.write().await;
        songs
            .make_contiguous()
            .shuffle(&mut rand::thread_rng());
        Ok(())
    }

    pub(crate) async fn get_songs(&self) -> Vec<Box<dyn Song>> {
        let songs = self.songs.read().await;
        songs.iter().map(|s| s.create()).collect()
    }

    pub(crate) async fn get_current_song(&self) -> Option<(Box<dyn Song>, f32)> {
        let current_song = self.current_song.read().await;
        if current_song.is_none() {
            return None;
        }
        let (current_song, track_handle) = current_song.as_ref().unwrap();
        let duration_played = match track_handle.get_info().await{
            Ok(info) => info.position.as_secs_f32(),
            Err(_) => return None,
        };
        Some((current_song.create(), duration_played))
    }

    pub(crate) async fn remove(&mut self, index: usize) -> Result<Box<dyn Song>, String> {
        let mut songs = self.songs.write().await;
        match songs.remove(index){
            Some(song) => Ok(song),
            None => Err(format!("Index out of bounds: {index}")),
        }
    }

    pub(crate) async fn clear(&mut self) {
        let mut songs = self.songs.write().await;
        songs.clear();
    }

    pub(crate) async fn move_song(&mut self, from: usize, to: usize) -> Result<Box<dyn Song>, String> {
        let mut songs = self.songs.write().await;
        if from >= songs.len() || to >= songs.len() {
            return Err(format!("Index out of bounds: {from} - {to}"));
        }
        songs.swap(from, to);
        let song = songs.get(to).unwrap();
        Ok(song.create())
    }

    pub(crate) async fn set_driver(&self, mut call: Option<Call>) -> () {
        let mut driver = self.driver.write().await;
        if call.is_none() {
            driver.take();
            return;
        }
        let mut call = call.take().unwrap();
        call.add_global_event(Event::Track(TrackEvent::End), self.clone());
        driver.replace(call);
    }
}


#[async_trait]
impl EventHandler for Queue {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {{
        let mut loop_mode = self.loop_mode.write().await;
        let cs = self.current_song.write().await.take();
        if cs.is_some() {
            let cs = cs.unwrap();
            let mut songs = self.songs.write().await;
            match *loop_mode {
                LoopMode::Song => songs.push_front(cs.0),
                LoopMode::Queue => songs.push_back(cs.0),
                LoopMode::Repeat(n) => {
                    songs.push_front(cs.0);
                    *loop_mode = if n <= 1 {
                        LoopMode::None
                    } else {
                        LoopMode::Repeat(n - 1)
                    };
                }
                _ => {}
            };
        }
    }
    let _ = self.play().await;
    None
    }
}