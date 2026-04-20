use std::collections::HashMap;
use std::sync::Arc;

use crate::traits::CleanupStyle;

#[derive(Default)]
pub struct PluginRegistry {
    cleanup_styles: HashMap<String, Arc<dyn CleanupStyle>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_cleanup(&mut self, id: impl Into<String>, plugin: Arc<dyn CleanupStyle>) {
        let id = id.into();
        if self.cleanup_styles.contains_key(&id) {
            panic!("duplicate cleanup plugin id: {id}");
        }
        self.cleanup_styles.insert(id, plugin);
    }

    pub fn cleanup(&self, id: &str) -> Option<Arc<dyn CleanupStyle>> {
        self.cleanup_styles.get(id).cloned()
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
    use crate::traits::{CleanupInput, CleanupStyle};

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

    #[test]
    fn register_and_lookup_by_id() {
        let mut reg = PluginRegistry::new();
        reg.register_cleanup("fake", Arc::new(FakeCleanup));
        assert!(reg.cleanup("fake").is_some());
    }

    #[test]
    fn lookup_unknown_id_returns_none() {
        let reg = PluginRegistry::new();
        assert!(reg.cleanup("nonexistent").is_none());
    }

    #[test]
    #[should_panic(expected = "duplicate cleanup plugin id")]
    fn duplicate_register_panics() {
        let mut reg = PluginRegistry::new();
        reg.register_cleanup("fake", Arc::new(FakeCleanup));
        reg.register_cleanup("fake", Arc::new(FakeCleanup));
    }
}
