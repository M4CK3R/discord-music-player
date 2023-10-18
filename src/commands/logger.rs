use std::sync::Arc;

use async_trait::async_trait;
use serenity::{http::Http, model::prelude::Message};

use self::messages::SENDING_MESSAGE;

pub(crate) mod messages {
    pub const SENDING_MESSAGE: &str = "Sending message";
    pub const GETTING_GUILD: &str = "Getting guild";
    pub const GETTING_CHANNEL: &str = "Getting channel";
    // pub const GETTING_CALL: &str = "Getting call";
    pub const JOINING_CHANNEL: &str = "Joining channel";
    pub const LEAVING_CHANNEL: &str = "Leaving channel";

    pub const PLAYING_SONG: &str = "Playing song";
    pub const STOPPING_SONG: &str = "Stopping song";
    pub const SKIPPING_SONG: &str = "Skipping song";
    pub const RESUMING_SONG: &str = "Resuming song";
    pub const SETTING_LOOP_MODE: &str = "Setting loop mode";
    pub const SHUFFLING_QUEUE: &str = "Shuffling queue";
    pub const PAUSING_SONG: &str = "Pausing song";
    pub const REMOVING_SONG: &str = "Removing song";
    pub const MOVING_SONG: &str = "Moving song";
    pub const ADDING_SONG: &str = "Adding song";

    pub const GETTING_URL: &str = "Getting url";
    pub const GETTING_INDEX: &str = "Getting index";
    pub const GETTING_NAME: &str = "Getting name";
}

#[async_trait]
pub(crate) trait Logger {
    fn log(self, message: Option<&str>) -> Self;
    fn log_message(self) -> Self;
    async fn log_with_message(self, message: &str, msg: &Message, http: &Arc<Http>) -> Self;
}

#[async_trait]
impl<S, E> Logger for Result<S, E>
where
    S: Send + Sync,
    E: std::fmt::Debug + Send + Sync,
{
    fn log(self, message: Option<&str>) -> Self {
        match &self {
            Ok(_) => {
                if let Some(msg) = message {
                    println!("SUCCESS: {}", msg);
                }
            }
            Err(e) => {
                let error_message = match message {
                    Some(msg) => format!("ERROR: {} - {:?}", msg, e),
                    None => format!("ERROR: {:?}", e),
                };
                eprintln!("ERROR: {:?}", error_message);
            }
        }
        self
    }

    fn log_message(self) -> Self {
        self.log(Some(SENDING_MESSAGE))
    }

    async fn log_with_message(self, message: &str, msg: &Message, http: &Arc<Http>) -> Self {
        let r = self.log(Some(message));
        if let Err(_e) = &r {
            let _ = msg
                .channel_id
                .say(http, format!("Error {}", message.to_lowercase()))
                .await
                .log_message();
        }
        r
    }
}
