use async_trait::async_trait;

#[async_trait]
pub trait LlmProvider: Send + Sync {}
