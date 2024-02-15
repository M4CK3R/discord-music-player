use std::sync::Arc;

use async_trait::async_trait;
use serenity::{http::Http, model::prelude::Message};
use tracing::{event, Level};

use self::messages::SENDING_MESSAGE;

pub(crate) mod messages {
    pub const SENDING_MESSAGE: &str = "Sending message";
    pub const GETTING_GUILD: &str = "Getting guild";
    pub const GETTING_CHANNEL: &str = "Getting channel";
    pub const GETTING_VOICE_MANAGER: &str = "Getting voice manager";
    // pub const GETTING_CALL: &str = "Getting call";
    pub const JOINING_CHANNEL: &str = "Joining channel";
    pub const GETTING_QUEUE_MANAGER: &str = "Getting queue manager";
    pub const GETTING_QUEUE_MANAGER_MAP: &str = "Getting queue manager";
    pub const _LEAVING_CHANNEL: &str = "Leaving channel";

    pub const _PLAYING_SONG: &str = "Playing song";
    pub const _STOPPING_SONG: &str = "Stopping song";
    pub const _SKIPPING_SONG: &str = "Skipping song";
    pub const _RESUMING_SONG: &str = "Resuming song";
    pub const SETTING_LOOP_MODE: &str = "Setting loop mode";
    pub const _SHUFFLING_QUEUE: &str = "Shuffling queue";
    pub const _PAUSING_SONG: &str = "Pausing song";
    pub const _REMOVING_SONG: &str = "Removing song";
    pub const _MOVING_SONG: &str = "Moving song";
    pub const _ADDING_SONG: &str = "Adding song";

    pub const _GETTING_URL: &str = "Getting url";
    pub const _GETTING_INDEX: &str = "Getting index";
    pub const _GETTING_NAME: &str = "Getting name";
}

#[async_trait]
pub(crate) trait Logger {
    fn log(self, message: Option<&str>) -> Self;
    fn log_message(self) -> Self;
    async fn log_with_message(self, message: &str, msg: &Message, http: &Arc<Http>) -> Self;
    async fn try_log_with_message(
        self,
        message: &str,
        msg: Option<&Message>,
        http: Option<&Arc<Http>>,
    ) -> Self;
}

#[async_trait]
impl<S, E> Logger for Result<S, E>
where
    S: Send,
    E: std::fmt::Debug + Send + Sync,
{
    fn log(self, message: Option<&str>) -> Self {
        match &self {
            Ok(_) => {
                if let Some(msg) = message {
                    event!(Level::INFO, "{}", msg);
                }
            }
            Err(e) => {
                let error_message = match message {
                    Some(msg) => format!("ERROR: {} - {:?}", msg, e),
                    None => format!("ERROR: {:?}", e),
                };
                event!(Level::ERROR, "{:?}", error_message);
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

    async fn try_log_with_message(
        self,
        message: &str,
        msg: Option<&Message>,
        http: Option<&Arc<Http>>,
    ) -> Self {
        if let (Some(msg), Some(http)) = (msg, http) {
            return self.log_with_message(message, msg, http).await;
        }
        let r = self.log(Some(message));
        r
    }
}
