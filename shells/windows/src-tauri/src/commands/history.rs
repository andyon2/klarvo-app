//! History Tauri-Command surface (Story 9.2 AC-7).
//!
//! Exposes get_history, delete_history_entry, clear_history as Tauri commands.
//! HistoryEntryDto is tauri-specta-exported for Story 9.3 (History-Panel).

use std::sync::Arc;

use specta::Type;
use tauri::State;

use klarvo_core::error::AppError;
use klarvo_core::history::{HistoryBackend, HistoryEntry};

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
    store: State<'_, Arc<dyn HistoryBackend>>,
    limit: Option<u32>,
) -> Result<Vec<HistoryEntryDto>, AppError> {
    let entries = store.list(limit.unwrap_or(100)).await?;
    Ok(entries.into_iter().map(HistoryEntryDto::from).collect())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_history_entry(
    store: State<'_, Arc<dyn HistoryBackend>>,
    id: i64,
) -> Result<(), AppError> {
    store.delete(id).await
}

#[tauri::command]
#[specta::specta]
pub async fn clear_history(
    store: State<'_, Arc<dyn HistoryBackend>>,
) -> Result<(), AppError> {
    store.clear().await
}
