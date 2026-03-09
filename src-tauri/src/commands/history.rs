//! Tauri commands for dictation history and usage statistics.

use tauri::State;

use crate::history::{self, UsageSummary};
use crate::license::LicensedFeature;
use crate::require_license;
use crate::AppState;

/// Maximum number of history entries visible in the free tier.
const FREE_TIER_HISTORY_LIMIT: u32 = 50;

/// Returns the most recent history entries.
///
/// Free-tier users are limited to the most recent 50 entries.
/// Licensed users can request any limit (or use the caller-supplied limit).
#[tauri::command]
pub fn get_history(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<history::HistoryEntry>, String> {
    // Check license status to determine the effective limit.
    let effective_limit = {
        let status = state
            .inner()
            .license_status
            .lock()
            .map_err(|_| "license lock error".to_string())?;
        if crate::license::is_feature_allowed(&status, LicensedFeature::UnlimitedHistory) {
            // Licensed: honour the caller-supplied limit (default 50).
            limit.unwrap_or(50)
        } else {
            // Unlicensed: cap at the free-tier limit regardless of what was requested.
            FREE_TIER_HISTORY_LIMIT
        }
    };

    let db = crate::lock!(state.inner().history_db)?;
    history::get_entries(&db, effective_limit)
        .map_err(|e| format!("Failed to load history: {e}"))
}

/// Searches history entries by text content and/or app name.
///
/// Requires a paid license (full-text search is a paid feature).
#[tauri::command]
pub fn search_history(
    state: State<'_, AppState>,
    text_query: Option<String>,
    app_query: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<history::HistoryEntry>, String> {
    require_license!(state, LicensedFeature::UnlimitedHistory);
    let db = crate::lock!(state.inner().history_db)?;
    history::search_entries(
        &db,
        text_query.as_deref(),
        app_query.as_deref(),
        limit.unwrap_or(50),
    )
    .map_err(|e| format!("Failed to search history: {e}"))
}

/// Deletes a single history entry.
#[tauri::command]
pub fn delete_history_entry(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = crate::lock!(state.inner().history_db)?;
    history::delete_entry(&db, id)
        .map_err(|e| format!("Failed to delete history entry: {e}"))?;
    Ok(())
}

/// Deletes all history entries.
#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> Result<u64, String> {
    let db = crate::lock!(state.inner().history_db)?;
    history::clear_history(&db).map_err(|e| format!("Failed to clear history: {e}"))
}

/// Saves a dictation result to history (used by the frontend manual flow).
#[tauri::command]
pub fn add_history_entry(
    state: State<'_, AppState>,
    text: String,
    raw_text: Option<String>,
    style: String,
    language: String,
) -> Result<i64, String> {
    let inner = state.inner();
    let db = crate::lock!(inner.history_db)?;
    let app_name = inner.prev_window_title.lock().ok().and_then(|t| t.clone());
    let device_id = crate::lock!(inner.config)
        .ok()
        .map(|c| c.device_id.clone());
    history::add_entry(
        &db,
        &text,
        raw_text.as_deref(),
        &style,
        &language,
        false,
        app_name.as_deref(),
        None,
        device_id.as_deref(),
    )
    .map_err(|e| format!("Failed to save history entry: {e}"))
}

/// Returns aggregated usage statistics (cost tracker + dictation stats).
///
/// Requires a paid license (cost tracking is a paid feature).
#[tauri::command]
pub fn get_usage_stats(state: State<'_, AppState>) -> Result<UsageSummary, String> {
    require_license!(state, LicensedFeature::CostTracking);
    let db = crate::lock!(state.inner().history_db)?;
    history::get_usage_summary(&db).map_err(|e| format!("Failed to get usage stats: {e}"))
}

/// Returns filler word statistics from raw transcripts in history.
///
/// Requires a paid license (filler analysis is a paid feature).
#[tauri::command]
pub fn get_filler_stats(
    state: State<'_, AppState>,
) -> Result<Vec<history::FillerStat>, String> {
    require_license!(state, LicensedFeature::FillerAnalysis);
    let db = crate::lock!(state.inner().history_db)?;
    history::get_filler_stats(&db).map_err(|e| format!("Failed to get filler stats: {e}"))
}

/// Returns the most recent voice notes.
///
/// Requires a paid license (Voice Notes is a paid feature).
#[tauri::command]
pub fn get_notes(
    state: State<'_, AppState>,
    limit: u32,
) -> Result<Vec<history::HistoryEntry>, String> {
    require_license!(state, LicensedFeature::VoiceNotes);
    let db = crate::lock!(state.inner().history_db)?;
    history::get_notes(&db, limit).map_err(|e| format!("Failed to get notes: {e}"))
}

/// Saves a dictation result as a voice note (not pasted).
///
/// Requires a paid license (Voice Notes is a paid feature).
#[tauri::command]
pub fn save_note(
    state: State<'_, AppState>,
    text: String,
    raw_text: String,
    style: String,
) -> Result<i64, String> {
    require_license!(state, LicensedFeature::VoiceNotes);
    let inner = state.inner();
    let db = crate::lock!(inner.history_db)?;
    let cfg = crate::lock!(inner.config)?;
    let language = cfg.language.clone();
    let device_id = cfg.device_id.clone();
    drop(cfg);
    history::add_entry(
        &db,
        &text,
        Some(&raw_text),
        &style,
        &language,
        true,
        None,
        None,
        Some(&device_id),
    )
    .map_err(|e| format!("Failed to save note: {e}"))
}
