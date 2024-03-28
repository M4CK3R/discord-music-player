mod player;
mod queue_saver;

use std::{
    collections::{HashMap, VecDeque},
    ops::Deref,
    sync::Arc,
};

use async_trait::async_trait;
use rand::seq::SliceRandom;
use songbird::{tracks::ControlError, Call, Event, EventContext, EventHandler, TrackEvent};
use tokio::sync::{Mutex, RwLock};
use tracing::{event, Level};

use crate::common::{Song, SongId};

pub use self::player::LoopMode;
use self::player::{CurrentSong, Player};
pub use self::queue_saver::{FileQueueSaver, QueueSaver};

type Queue = Arc<RwLock<VecDeque<Box<dyn Song>>>>;
pub struct QueueManager<QS>
where
    QS: QueueSaver + Send + Sync,
{
    queue: Queue,
    saved_queues: HashMap<String, Vec<SongId>>,
    queue_saver: QS,
    player: Arc<RwLock<Player>>,
}
impl<QS> QueueManager<QS>
where
    QS: QueueSaver + Send + Sync,
{
    pub fn new(queue_saver: QS) -> QueueManager<QS> {
        let mut qm = QueueManager {
            queue: Arc::new(RwLock::new(VecDeque::new())),
            saved_queues: HashMap::new(),
            player: Arc::new(RwLock::new(Player::new())),
            queue_saver,
        };
        match qm.queue_saver.load_queues() {
            Ok(queues) => qm.saved_queues = queues,
            Err(e) => {
                event!(Level::ERROR, "Failed to load queues: {}", e);
            }
        }
        qm
    }
    pub async fn add_saved_queue(&mut self, name: impl ToString) -> Result<(), String> {
        let queue_read = self.queue.read().await;
        if queue_read.is_empty() {
            return Err("Queue is empty".to_string());
        }
        let mut queue: Vec<String> = queue_read.iter().map(|s| s.get_id().clone()).collect();
        match self.player.read().await.get_current_song() {
            Some(current_song) => queue.push(current_song.song.get_id().clone()),
            None => (),
        };
        self.saved_queues.insert(name.to_string(), queue);
        Ok(())
    }
    pub fn get_saved_queue(&self, name: impl ToString) -> Option<Vec<SongId>> {
        self.saved_queues.get(&name.to_string()).cloned()
    }
    pub fn remove_saved_queue(&mut self, name: impl ToString) {
        self.saved_queues.remove(&name.to_string());
    }
    pub fn list_saved_queues(&self) -> Vec<SongId> {
        self.saved_queues.keys().cloned().collect()
    }
    pub async fn add_to_queue(&self, songs: Vec<Box<dyn Song>>) -> Result<(), ControlError> {
        self.queue.write().await.append(&mut songs.into());
        let player = self.player.read().await;
        if let (Some(_c), None) = (player.get_call(), player.get_current_song()) {
            drop(player);
            self.play_next().await?;
        }
        Ok(())
    }
    pub async fn _remove_from_queue(&self, songs: Vec<SongId>) {
        self.queue
            .write()
            .await
            .retain(|s| !songs.contains(s.get_id()));
    }
    pub async fn remove_from_queue_by_index(&self, index: usize) -> Option<Box<dyn Song>> {
        self.queue.write().await.remove(index)
    }
    pub async fn get_queue(&self) -> Vec<Box<dyn Song>> {
        self.queue
            .read()
            .await
            .iter()
            .map(|s| s.clone_song())
            .collect()
    }
    pub async fn clear_queue(&mut self) {
        self.queue.write().await.clear();
        let _ = self.skip().await;
    }
    pub async fn swap(&self, index1: usize, index2: usize) -> Result<(), usize> {
        let queue_len = self.queue.read().await.len();
        let index_invalid = |i: usize| i >= queue_len;

        if index_invalid(index1) {
            return Err(index1);
        }
        if index_invalid(index2) {
            return Err(index2);
        }

        self.queue.write().await.swap(index1, index2);
        Ok(())
    }
    pub fn save_queues(&self) {
        match self.queue_saver.save_queues(self.saved_queues.clone()) {
            Ok(_) => (),
            Err(e) => {
                event!(Level::ERROR, "Failed to save queues: {}", e);
            }
        }
    }
    pub async fn call_joined(
        this: QueueEventHandler<QS>,
        driver: Arc<Mutex<Call>>,
    ) -> Result<(), ControlError> {
        let t = this.0.write().await;
        let call = driver.clone();
        t.player.write().await.call_joined(driver);
        t.play_next().await?;
        drop(t);
        call.lock()
            .await
            .add_global_event(Event::Track(TrackEvent::End), this);
        Ok(())
    }
    pub async fn call_left(&mut self) -> Option<Arc<Mutex<Call>>> {
        self.player.write().await.call_left()
    }
    pub async fn _get_call(&self) -> Option<Arc<Mutex<Call>>> {
        self.player.read().await.get_call()
    }
    pub async fn pause(&self) -> Result<(), ControlError> {
        self.player.write().await.pause()
    }
    pub async fn resume(&self) -> Result<(), ControlError> {
        self.player.write().await.resume()
    }
    pub async fn skip(&self) -> Result<Box<dyn Song>, ControlError> {
        self.remove_current_song(true).await
    }
    pub async fn set_loop(&self, loop_mode: LoopMode) {
        self.player.write().await.loop_mode = loop_mode;
    }
    pub async fn shuffle(&self) {
        let mut queue = self.queue.write().await;
        let mut rng = rand::thread_rng();
        queue.make_contiguous().shuffle(&mut rng);
    }
    pub async fn get_current_song(&self) -> Option<CurrentSong> {
        self.player.read().await.get_current_song()
    }

    async fn remove_current_song(&self, song_skipped: bool) -> Result<Box<dyn Song>, ControlError> {
        let mut pw = self.player.write().await;
        let current_song = match pw.take_current_song() {
            Ok(song) => song,
            Err(e) => {
                event!(Level::ERROR, "Failed to remove current song: {}", e);
                return Err(e);
            }
        };
        let loop_mode = &pw.loop_mode;
        match loop_mode {
            LoopMode::Song => {
                if !song_skipped {
                    self.queue
                        .write()
                        .await
                        .push_front(current_song.clone_song())
                }
            }
            LoopMode::Queue => {
                self.queue
                    .write()
                    .await
                    .push_back(current_song.clone_song());
            }
            LoopMode::None => {}
        }
        return Ok(current_song);
    }
    async fn play_next(&self) -> Result<(), ControlError> {
        let song = match self.queue.write().await.pop_front() {
            Some(song) => song,
            None => return Ok(()),
        };
        self.player.write().await.play(song).await
    }
}

pub struct QueueEventHandler<QS>(Arc<RwLock<QueueManager<QS>>>)
where
    QS: QueueSaver + Send + Sync;

impl<QS> Deref for QueueEventHandler<QS>
where
    QS: QueueSaver + Send + Sync,
{
    type Target = Arc<RwLock<QueueManager<QS>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<QS> From<Arc<RwLock<QueueManager<QS>>>> for QueueEventHandler<QS>
where
    QS: QueueSaver + Send + Sync,
{
    fn from(queue_manager: Arc<RwLock<QueueManager<QS>>>) -> Self {
        QueueEventHandler(queue_manager)
    }
}

#[async_trait]
impl<QS> EventHandler for QueueEventHandler<QS>
where
    QS: QueueSaver + Send + Sync,
{
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let write = self.write().await;
        // let _cs = write.remove_current_song(false).await;
        match write.play_next().await {
            Ok(_) => (),
            Err(e) => {
                event!(Level::ERROR, "Failed to play next song: {}", e);
            }
        };
        None
    }
}
