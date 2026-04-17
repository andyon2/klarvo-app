use async_trait::async_trait;

#[async_trait]
pub trait CleanupStyle: Send + Sync {}
