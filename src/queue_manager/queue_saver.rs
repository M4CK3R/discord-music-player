use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use crate::common::SongId;

const SAVED_QUEUES_FILE_NAME: &str = "saved_queues.json";

pub trait QueueSaver: Send + Sync + 'static {
    fn save_queues(&self, queues: HashMap<String, Vec<SongId>>) -> Result<(), String>;
    fn load_queues(&self) -> Result<HashMap<String, Vec<SongId>>, String>;
}



pub struct FileQueueSaver {
    saved_queues_path: PathBuf,
}

impl FileQueueSaver {
    pub fn new(saved_queues_path: impl AsRef<OsStr>) -> FileQueueSaver {
        FileQueueSaver {
            saved_queues_path: Path::new(&saved_queues_path).join(SAVED_QUEUES_FILE_NAME),
        }
    }
}

impl QueueSaver for FileQueueSaver {
    fn save_queues(&self, queues: HashMap<String, Vec<SongId>>) -> Result<(), String> {
        let file = std::fs::File::create(&self.saved_queues_path).map_err(|e| e.to_string())?;
        serde_json::to_writer(file, &queues).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn load_queues(&self) -> Result<HashMap<String, Vec<SongId>>, String> {
        let file = std::fs::File::open(&self.saved_queues_path).map_err(|e| e.to_string())?;
        serde_json::from_reader(file).map_err(|e| e.to_string())
    }
}

pub struct NullQueueSaver {}

impl NullQueueSaver {
    pub fn _new() -> NullQueueSaver {
        NullQueueSaver {}
    }
}

impl QueueSaver for NullQueueSaver {
    fn save_queues(&self, _: HashMap<String, Vec<SongId>>) -> Result<(), String> {
        Ok(())
    }

    fn load_queues(&self) -> Result<HashMap<String, Vec<SongId>>, String> {
        Ok(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use super::*;

    #[test]
    fn test_file_queue_saver_save_and_load_queues() {
        let tempdir = temp_dir().join("test_file_queue_saver_save_and_load_queues");
        std::fs::create_dir_all(&tempdir).expect("Failed to create temp dir");
        let saver = FileQueueSaver::new(&tempdir);
        let mut queues = HashMap::new();
        queues.insert("test".to_string(), vec!["test".to_string()]);
        saver.save_queues(queues.clone()).expect("Failed to save queues");
        let res = saver.load_queues().expect("Failed to load queues");
        assert_eq!(res, queues);
        std::fs::remove_dir_all(&tempdir).expect("Failed to remove temp dir");
    }
}
