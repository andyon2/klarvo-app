use async_trait::async_trait;

#[async_trait]
pub trait OutputTarget: Send + Sync {}
