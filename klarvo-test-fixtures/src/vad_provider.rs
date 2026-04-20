use std::collections::VecDeque;

use async_trait::async_trait;

use klarvo_core::audio::vad::{VadDecision, VadProvider};
use klarvo_core::error::PluginError;

/// Test fixture implementing `VadProvider`. Returns pre-programmed `VadDecision` values in
/// sequence. After exhaustion returns `Silence`. Use to test `run_capture_session` without
/// depending on RMS-threshold behavior.
pub struct MockVadProvider {
    decisions: VecDeque<VadDecision>,
}

impl MockVadProvider {
    /// Returns decisions[i] for the i-th process() call.
    /// After exhaustion, always returns VadDecision::Silence.
    pub fn with_decisions(decisions: Vec<VadDecision>) -> Self {
        Self { decisions: VecDeque::from(decisions) }
    }
}

#[async_trait]
impl VadProvider for MockVadProvider {
    async fn process(
        &mut self,
        _samples: &[f32],
        _ts_ms: u64,
    ) -> Result<VadDecision, PluginError> {
        Ok(self.decisions.pop_front().unwrap_or(VadDecision::Silence))
    }

    fn reset(&mut self) {
        // Reset resettiert VadProvider-internen-Zustand, nicht die Test-Sequenz.
        // Caller erstellt neue Instanz wenn Sequenz neu starten soll.
    }
}
