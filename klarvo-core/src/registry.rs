use std::collections::HashMap;
use std::sync::Arc;

use crate::traits::{CleanupStyle, SttProvider};

/// Registry of plugin instances keyed by their string IDs.
///
/// # Arc vs Box
///
/// The Registry uses `Arc<dyn …>` (not `Box<dyn …>`) for all plugin slots. This enables
/// multi-session instance re-use without cloning the underlying plugin state
/// (ADR-0005-§4 Lifetime-Mitigation). The `1A.5` Object-Safety compile-test uses
/// `Box<dyn PipelineStage>` as a surface-guarantee for object-safety — this is intentional
/// and consistent: Arc/Box are both object-safe thin-pointers; the Arc/Box asymmetry between
/// Registry and the 1A.5 test is documented here to prevent confusion.
#[derive(Default)]
pub struct PluginRegistry {
    stt: HashMap<String, Arc<dyn SttProvider>>,
    cleanup: HashMap<String, Arc<dyn CleanupStyle>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an [`SttProvider`] under `id`. Panics on duplicate ID.
    pub fn register_stt(&mut self, id: impl Into<String>, plugin: Arc<dyn SttProvider>) {
        let id = id.into();
        if self.stt.contains_key(&id) {
            panic!("duplicate stt plugin id: {id}");
        }
        self.stt.insert(id, plugin);
    }

    /// Look up a registered [`SttProvider`] by `id`. Returns `None` if not registered.
    pub fn stt(&self, id: &str) -> Option<Arc<dyn SttProvider>> {
        self.stt.get(id).cloned()
    }

    /// Register a [`CleanupStyle`] under `id`. Panics on duplicate ID.
    pub fn register_cleanup(&mut self, id: impl Into<String>, plugin: Arc<dyn CleanupStyle>) {
        let id = id.into();
        if self.cleanup.contains_key(&id) {
            panic!("duplicate cleanup plugin id: {id}");
        }
        self.cleanup.insert(id, plugin);
    }

    /// Look up a registered [`CleanupStyle`] by `id`. Returns `None` if not registered.
    pub fn cleanup(&self, id: &str) -> Option<Arc<dyn CleanupStyle>> {
        self.cleanup.get(id).cloned()
    }
}

pub fn bootstrap() -> PluginRegistry {
    PluginRegistry::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::error::AppError;
    use crate::pipeline::PipelineStage;
    use crate::traits::{AudioBuffer, CleanupInput, CleanupStyle, SttProvider};

    struct FakeCleanup;

    #[async_trait]
    impl PipelineStage for FakeCleanup {
        type Input = CleanupInput;
        type Output = String;

        async fn process(&self, input: CleanupInput) -> Result<String, AppError> {
            Ok(input.raw)
        }

        fn stage_type(&self) -> &'static str {
            "cleanup"
        }
    }

    #[async_trait]
    impl CleanupStyle for FakeCleanup {}

    struct FakeStt;

    #[async_trait]
    impl PipelineStage for FakeStt {
        type Input = AudioBuffer;
        type Output = String;

        async fn process(&self, _input: AudioBuffer) -> Result<String, AppError> {
            Ok("fake-transcription".to_string())
        }

        fn stage_type(&self) -> &'static str {
            "stt"
        }
    }

    #[async_trait]
    impl SttProvider for FakeStt {}

    #[test]
    fn register_and_lookup_cleanup_by_id() {
        let mut reg = PluginRegistry::new();
        reg.register_cleanup("fake", Arc::new(FakeCleanup));
        assert!(reg.cleanup("fake").is_some());
    }

    #[test]
    fn lookup_unknown_cleanup_id_returns_none() {
        let reg = PluginRegistry::new();
        assert!(reg.cleanup("nonexistent").is_none());
    }

    #[test]
    #[should_panic(expected = "duplicate cleanup plugin id")]
    fn duplicate_cleanup_register_panics() {
        let mut reg = PluginRegistry::new();
        reg.register_cleanup("fake", Arc::new(FakeCleanup));
        reg.register_cleanup("fake", Arc::new(FakeCleanup));
    }

    #[test]
    fn register_and_lookup_stt_by_id() {
        let mut reg = PluginRegistry::new();
        reg.register_stt("fake-stt", Arc::new(FakeStt));
        assert!(reg.stt("fake-stt").is_some());
    }

    #[test]
    fn lookup_unknown_stt_id_returns_none() {
        let reg = PluginRegistry::new();
        assert!(reg.stt("nonexistent").is_none());
    }

    #[test]
    #[should_panic(expected = "duplicate stt plugin id")]
    fn duplicate_stt_register_panics() {
        let mut reg = PluginRegistry::new();
        reg.register_stt("fake-stt", Arc::new(FakeStt));
        reg.register_stt("fake-stt", Arc::new(FakeStt));
    }
}
