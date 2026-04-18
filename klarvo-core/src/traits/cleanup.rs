use async_trait::async_trait;

use crate::error::PluginError;

#[async_trait]
pub trait CleanupStyle: Send + Sync {
    async fn apply(&self, input: &str) -> Result<String, PluginError>;
}
