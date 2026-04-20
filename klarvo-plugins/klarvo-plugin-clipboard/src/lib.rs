use std::sync::Arc;

use async_trait::async_trait;

use klarvo_core::PluginRegistry;
use klarvo_core::error::{AppError, AppErrorKind};
use klarvo_core::i18n;
use klarvo_core::output::{OutputTarget, keys};

/// Test coverage: CI-coverage for `OutputTarget`-delivery semantics is provided by
/// `InMemoryOutputTarget` integration tests in `klarvo-plugin-groq`. Real-arboard
/// smoke-validation is done manually via the Phase-1 dogfooding hotkey flow (arboard requires a
/// running OS clipboard service, unavailable in headless CI). Note: `arboard::Clipboard::new()`
/// and `set_text()` are synchronous; calling them inside `async fn deliver()` is
/// Phase-1-acceptable (sub-millisecond OS-call); `tokio::task::spawn_blocking` wrapping is a
/// Phase-2-refinement if benchmarks indicate contention. Phase-2 extension:
/// `klarvo-plugin-keystroke` (architecture.md:1036) implements `OutputTarget` via
/// Win32-SendInput-Direct-Keystroke-Injection, bypassing the clipboard.
pub struct ClipboardOutputTarget;

#[async_trait]
impl OutputTarget for ClipboardOutputTarget {
    async fn deliver(&self, text: &str) -> Result<(), AppError> {
        let result = arboard::Clipboard::new()
            .and_then(|mut cb| cb.set_text(text.to_owned()));
        match result {
            Ok(()) => Ok(()),
            Err(e) => {
                debug_assert!(i18n::is_key(keys::CLIPBOARD_UNAVAILABLE));
                Err(AppError {
                    kind: AppErrorKind::Io,
                    message: format!("clipboard: arboard error: {e}"),
                    user_message: Some(keys::CLIPBOARD_UNAVAILABLE.to_string()),
                    retryable: false,
                })
            }
        }
    }
}

pub fn register(registry: &mut PluginRegistry) {
    registry.register_output("clipboard", Arc::new(ClipboardOutputTarget));
}
