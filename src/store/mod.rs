pub mod sqlx_store;

use async_trait::async_trait;

use crate::domain::{Event, NewEvent};
use crate::error::AppResult;

pub use sqlx_store::SqlxStore;

#[async_trait]
pub trait EventStore: Send + Sync {
    async fn create(&self, event: NewEvent) -> AppResult<Event>;

    async fn get(&self, id: &str) -> AppResult<Option<Event>>;

    async fn cancel(&self, id: &str) -> AppResult<()>;
}