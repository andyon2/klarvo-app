use async_trait::async_trait;

#[async_trait]
pub trait SttProvider: Send + Sync {}
