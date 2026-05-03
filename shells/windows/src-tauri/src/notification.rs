use std::sync::{
    Arc,
    RwLock,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::broadcast;
use tauri_plugin_notification::NotificationExt;
use klarvo_core::event::Event;

use crate::i18n::I18nTable;

/// Subscribes to the [`EventBus`] and emits native OS toast notifications
/// for dictation-lifecycle events.
///
/// Triggers:
/// - [`Event::RecordingDelivered`] — always: "Dictation pasted: {60-char preview}"
/// - [`Event::ErrorEmitted`] — only during an active recording session
///   (`in_session` guard: set by `RecordingStarted`, cleared by `RecordingCompleted`).
///   Boot-time errors (config parse, keystore) are suppressed because they arrive
///   before `RecordingStarted` fires.
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
            let in_session = Arc::new(AtomicBool::new(false));
            loop {
                match rx.recv().await {
                    Ok(event) => self.handle(&event, &in_session),
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "NotificationService lagged; skipped events");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    fn handle(&self, event: &Event, in_session: &Arc<AtomicBool>) {
        match event {
            Event::RecordingStarted { .. } => {
                in_session.store(true, Ordering::Relaxed);
            }
            Event::RecordingCompleted { .. } => {
                in_session.store(false, Ordering::Relaxed);
            }
            Event::RecordingDelivered { text, .. } => {
                let label = self.t("notification.dictation.delivered");
                let char_count = text.chars().count();
                let preview: String = text.chars().take(60).collect();
                let suffix = if char_count > 60 { "…" } else { "" };
                let body = format!("{label}: {preview}{suffix}");
                self.show(&body);
            }
            Event::ErrorEmitted { error_key, .. } => {
                if in_session.load(Ordering::Relaxed) {
                    let body = self.t(error_key);
                    self.show(&body);
                }
            }
            _ => {}
        }
    }

    fn t(&self, key: &str) -> String {
        self.i18n
            .read()
            .ok()
            .and_then(|table| table.get(key).cloned())
            .unwrap_or_else(|| key.to_string())
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
