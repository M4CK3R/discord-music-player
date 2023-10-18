use serde::{Deserialize, Serialize};

use crate::common::Song;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SavedQueue {
    pub name: String,
    ids: Vec<String>,
}
impl SavedQueue {
    pub(crate) fn new(name: String, queue: Vec<Box<dyn Song>>) -> SavedQueue {
        SavedQueue {
            name,
            ids: queue.iter().map(|s| s.get_id()).collect(),
        }
    }

    pub(crate) fn get_ids(&self) -> Vec<String> {
        self.ids.clone()
    }
}
