use std::sync::Mutex;

use async_trait::async_trait;

use klarvo_core::{AppError, output::OutputTarget};

/// Placed in `klarvo-test-fixtures` per ADR-0008 Amendment-1 explicit instruction. Factor-in
/// justified by near-certain second-consumer: Story 2.6 FR29-Retry-Path
/// (OutputTarget-delivery-after-retry), Epic-3 Shell-Integration-Tests. See
/// `memory/feedback_premature_abstraction_guard` for guard policy. Persistence of delivered
/// texts for assertion access is a narrow, documented exception to the PII-log-discipline rule
/// (cf. `OutputTarget::deliver` Contract-Clause).
#[derive(Default)]
pub struct InMemoryOutputTarget {
    delivered: Mutex<Vec<String>>,
}

impl InMemoryOutputTarget {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the most recently delivered text, or `None` if deliver() was never called.
    pub fn last_delivered(&self) -> Option<String> {
        self.delivered.lock().unwrap().last().cloned()
    }

    /// Returns all delivered texts in call order.
    pub fn all_delivered(&self) -> Vec<String> {
        self.delivered.lock().unwrap().clone()
    }
}

#[async_trait]
impl OutputTarget for InMemoryOutputTarget {
    async fn deliver(&self, text: &str) -> Result<(), AppError> {
        self.delivered.lock().unwrap().push(text.to_owned());
        Ok(())
    }
}
