use async_trait::async_trait;

use crate::error::PluginError;

#[async_trait]
pub trait VadProvider: Send + Sync {
    async fn process(&mut self, samples: &[f32]) -> Result<VadDecision, PluginError>;

    fn reset(&mut self);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VadDecision {
    Silence,
    SpeechStart { ts_ms: u64 },
    Speech,
    SpeechEnd { ts_ms: u64, duration_ms: u64 },
}
