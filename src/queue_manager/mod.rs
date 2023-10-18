use std::{collections::HashMap, path::PathBuf, fs::{self, File}, io::Write};

use serenity::model::prelude::GuildId;
use songbird::Call;

use self::{queue::Queue, saved_queue::SavedQueue};

use crate::common::Song;

pub mod queue;
mod saved_queue;

const SAVE_FILE_NAME: &str = "saved_queues.json";

pub struct QueueManager {
    queues: HashMap<GuildId, Queue>,
    saved_queues: HashMap<GuildId, HashMap<String, SavedQueue>>,
    save_file_path: String,
}

impl QueueManager {
    pub fn new() -> Self {
        let path = std::env::var("SAVED_QUEUES_PATH").expect("SAVED_QUEUES_PATH not set");
        let mut q = QueueManager {
            queues: HashMap::new(),
            saved_queues: HashMap::new(),
            save_file_path: path,
        };
        q.load();
        q
    }

    pub async fn add(&mut self, guild_id: GuildId, songs: Vec<Box<dyn Song>>) -> usize {
        let queue = self.get_queue_mut(guild_id);
        queue.add(songs).await
    }

    pub async fn play(&mut self, guild_id: GuildId) -> Result<(), String> {
        let queue = self.get_queue_mut(guild_id);
        queue.play().await
    }

    pub async fn set_driver(&mut self, guild_id: GuildId, driver: Option<&mut Call>) {
        let queue = self.get_queue_mut(guild_id);
        match driver {
            Some(driver) => queue.set_driver(Some(driver.clone())).await,
            None => queue.set_driver(None).await,
        }
    }

    pub async fn stop(&mut self, guild_id: GuildId) -> Result<(), String> {
        let queue = self.get_queue_mut(guild_id);
        queue.stop().await
    }

    pub async fn pause(&mut self, guild_id: GuildId) -> Result<(), String> {
        let queue = self.get_queue_mut(guild_id);
        queue.pause().await
    }

    pub async fn resume(&mut self, guild_id: GuildId) -> Result<(), String> {
        let queue = self.get_queue_mut(guild_id);
        queue.resume().await
    }

    pub async fn skip(&mut self, guild_id: GuildId) -> Result<(), String> {
        let queue = self.get_queue_mut(guild_id);
        queue.skip().await
    }

    pub async fn set_loop(&mut self, guild_id: GuildId, loop_mode: queue::LoopMode) {
        let queue = self.get_queue_mut(guild_id);
        queue.set_loop(loop_mode).await;
    }

    pub async fn shuffle(&mut self, guild_id: GuildId) -> Result<(), String> {
        let queue = self.get_queue_mut(guild_id);
        queue.shuffle().await
    }

    pub async fn get_song_list(&self, guild_id: GuildId) -> Vec<Box<dyn Song>> {
        let queue = match self.get_queue(guild_id) {
          Ok(q) => q,
            Err(_) => return Vec::new(),  
        };
        queue.get_songs().await
    }

    pub async fn get_current_song(&self, guild_id: GuildId) -> Option<(Box<dyn Song>, f32)> {
        let queue = match self.get_queue(guild_id){
            Ok(q) => q,
            Err(_) => return None,
        };
        queue.get_current_song().await
    }

    pub async fn remove(&mut self, guild_id: GuildId, index: usize) -> Result<Box<dyn Song>, String> {
        let queue = self.get_queue_mut(guild_id);
        queue.remove(index).await
    }

    pub async fn clear(&mut self, guild_id: GuildId) {
        let queue = self.get_queue_mut(guild_id);
        queue.clear().await;
    }

    pub async fn move_song(&mut self, guild_id: GuildId, from: usize, to: usize) -> Result<Box<dyn Song>, String> {
        let queue = self.get_queue_mut(guild_id);
        queue.move_song(from, to).await
    }

    fn get_queue_mut(&mut self, guild_id: GuildId) -> &mut Queue {
        self.queues.entry(guild_id).or_insert_with(|| Queue::new())
    }

    fn get_queue(&self, guild_id: GuildId) -> Result<&Queue, String> {
        self.queues.get(&guild_id).ok_or_else(|| "Queue not found".to_string())
    }

    pub fn save_queue(&mut self, guild_id: GuildId, name: String, queue: Vec<Box<dyn Song>>) {
        let saved_queue = SavedQueue::new(name, queue);
        let guild_queues = self.saved_queues.entry(guild_id).or_insert_with(|| HashMap::new());
        guild_queues.insert(saved_queue.name.clone(), saved_queue);
    }

    pub fn get_saved_queue(&self, guild_id: GuildId, name: &str) -> Option<Vec<String>> {
        let guild_queues = self.saved_queues.get(&guild_id)?;
        let saved_queue = guild_queues.get(name)?;
        Some(saved_queue.get_ids())
    }

    pub fn get_saved_queues(&self, guild_id: GuildId) -> Option<Vec<String>> {
        let guild_queues = self.saved_queues.get(&guild_id)?;
        Some(guild_queues.keys().cloned().collect())
    }

    pub fn remove_saved_queue(&mut self, guild_id: GuildId, name: &str) -> Option<()> {
        let guild_queues = self.saved_queues.get_mut(&guild_id)?;
        guild_queues.remove(name)?;
        Some(())
    }

    fn load(&mut self) {
        let path = PathBuf::from(&self.save_file_path).join(SAVE_FILE_NAME);
        let file_data = match fs::read_to_string(path) {
            Ok(data) => data,
            Err(_) => "{}".to_string(),
        };
        let data: HashMap<GuildId, HashMap<String, SavedQueue>> = serde_json::from_str(&file_data).unwrap();
        self.saved_queues = data;
    }

    pub fn save(&self) {
        let path = PathBuf::from(&self.save_file_path).join(SAVE_FILE_NAME);
        let mut file = File::create(path).unwrap();
        let data = serde_json::to_string(&self.saved_queues).unwrap();
        file.write_all(data.as_bytes()).unwrap();
    }
}


impl Drop for QueueManager {
    fn drop(&mut self) {
        self.save();
    }
}