use songbird::input::Input;
use async_trait::async_trait;


#[async_trait]
pub trait Song: Send + Sync {
    fn title(&self) -> String;
    fn artist(&self) -> String;
    fn duration(&self) -> Option<u32>;
    async fn get_source(&self) -> Input;
    fn create(&self) -> Box<dyn Song>;
    fn get_id(&self) -> String;
}