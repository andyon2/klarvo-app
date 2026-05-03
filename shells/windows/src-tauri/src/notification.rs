use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tauri_plugin_notification::NotificationExt;
use klarvo_core::event::Event;

use crate::i18n::I18nTable;

/// Subscribes to the [`EventBus`] and emits a native OS toast notification
/// for [`Event::RecordingDelivered`] — fired only in WaitAndType-Mode where
/// the pipeline produces a transcription that has NOT been auto-pasted.
///
/// Toast body: `"{label}: {preview}"` where `label` resolves the i18n key
/// `notification.dictation.delivered` and `preview` is the first 60 chars of
/// the transcription with a `…` suffix when truncated.
///
/// Hold/Toggle/AutoStop modes auto-paste and do not emit `RecordingDelivered`,
/// so they receive no toast in this story (deferred to follow-up: needs a
/// `RecordingPasted` event or equivalent emitted from the Hold-success path).
///
/// Error toasts are deferred to a follow-up story — `Event::ErrorEmitted` is
/// not currently published to the EventBus (`TauriErrorEmitter::emit_error`
/// emits `app.error` directly to the frontend per ADR-0009 §SD-1, bypassing
/// the bus). A subscriber here would observe nothing in production.
///
/// Generic over `R: tauri::Runtime` — same pattern as `EventMirror` and
/// `TauriErrorEmitter`, enabling `MockRuntime` in tests.
pub struct NotificationService<R: tauri::Runtime> {
    app_handle: tauri::AppHandle<R>,
    i18n: Arc<RwLock<I18nTable>>,
}

impl<R: tauri::Runtime> NotificationService<R> {
    pub fn new(handle: tauri::AppHandle<R>, i18n: Arc<RwLock<I18nTable>>) -> Self {
        Self { app_handle: handle, i18n }
    }

    /// Spawn a background task that drains `rx` and triggers native OS notifications.
    /// Returns immediately; the task runs until the channel is closed.
    ///
    /// Uses `tauri::async_runtime::spawn` per `memory/project_shell_runtime_model`
    /// (single tokio-Runtime in shell scope).
    pub fn start(self, mut rx: broadcast::Receiver<Event>) {
        tauri::async_runtime::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => self.handle(&event),
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "NotificationService lagged; skipped events");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    fn handle(&self, event: &Event) {
        if let Event::RecordingDelivered { text, .. } = event {
            let label = self.t("notification.dictation.delivered");
            let char_count = text.chars().count();
            let preview: String = text.chars().take(60).collect();
            let suffix = if char_count > 60 { "…" } else { "" };
            let body = format!("{label}: {preview}{suffix}");
            self.show(&body);
        }
    }

    fn t(&self, key: &str) -> String {
        match self.i18n.read() {
            Ok(table) => table.get(key).cloned().unwrap_or_else(|| key.to_string()),
            Err(e) => {
                tracing::warn!(error = %e, key = key, "i18n RwLock poisoned; falling back to raw key");
                key.to_string()
            }
        }
    }

    fn show(&self, body: &str) {
        if let Err(e) = self
            .app_handle
            .notification()
            .builder()
            .title("Klarvo")
            .body(body)
            .show()
        {
            tracing::warn!(error = %e, "NotificationService: OS notification failed (fail-soft)");
        }
    }
}
