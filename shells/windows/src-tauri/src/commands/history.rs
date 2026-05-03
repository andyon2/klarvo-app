//! History Tauri-Command surface (Story 9.2 AC-7).
//!
//! Exposes get_history, delete_history_entry, clear_history as Tauri commands.
//! HistoryEntryDto is tauri-specta-exported for Story 9.3 (History-Panel).

use std::sync::Arc;

use specta::Type;
use tauri::State;

use klarvo_core::error::AppError;
use klarvo_core::history::{HistoryBackend, HistoryEntry};

/// Upper bound on the number of history entries a single `get_history` call may
/// return. Also the default when the caller does not pass `limit`.
///
/// Why a clamp: Tauri serialises the result over the IPC channel and the
/// renderer materialises every row. Without an upper bound, a frontend bug
/// passing `u32::MAX` (or a power-user with `history.max_entries=1_000_000`)
/// could OOM the renderer or freeze the IPC pipe.
pub const MAX_LIST_LIMIT: u32 = 1000;

/// Tauri-managed state newtype around `Arc<dyn HistoryBackend>`.
///
/// Tauri's state map is keyed by concrete type. Registering the trait-object Arc
/// directly (`app.manage(Arc<dyn HistoryBackend>)`) collides at the type-key with
/// any other `app.manage(Arc<...>)` that resolves to the same vtable layout. The
/// newtype gives Tauri a stable, distinct identity-type and is also a single
/// place to extend later (e.g. carry `max_entries` next to the backend).
pub struct HistoryStoreState(pub Arc<dyn HistoryBackend>);

/// Tauri-serializable projection of HistoryEntry.
/// `specta::Type` export generates a TypeScript interface for Story 9.3.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntryDto {
    pub id: i64,
    pub text: String,
    pub style: String,
    pub language: String,
    pub created_at: String,
    pub plugin_id: Option<String>,
    pub output_language: Option<String>,
}

impl From<HistoryEntry> for HistoryEntryDto {
    fn from(e: HistoryEntry) -> Self {
        Self {
            id: e.id,
            text: e.text,
            style: e.style,
            language: e.language,
            created_at: e.created_at,
            plugin_id: e.plugin_id,
            output_language: e.output_language,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn get_history(
    store: State<'_, HistoryStoreState>,
    limit: Option<u32>,
) -> Result<Vec<HistoryEntryDto>, AppError> {
    // None = "give me everything up to MAX_LIST_LIMIT" — the store further bounds
    // by `max_entries` (default 500). Explicit limit is clamped to MAX_LIST_LIMIT.
    let effective_limit = limit.unwrap_or(MAX_LIST_LIMIT).min(MAX_LIST_LIMIT);
    let entries = store.0.list(effective_limit).await?;
    Ok(entries.into_iter().map(HistoryEntryDto::from).collect())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_history_entry(
    store: State<'_, HistoryStoreState>,
    id: i64,
) -> Result<(), AppError> {
    store.0.delete(id).await
}

#[tauri::command]
#[specta::specta]
pub async fn clear_history(
    store: State<'_, HistoryStoreState>,
) -> Result<(), AppError> {
    store.0.clear().await
}
