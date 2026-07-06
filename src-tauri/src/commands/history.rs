//! Tauri commands for dictation history and usage statistics.

use tauri::State;

use crate::history::{self, UsageSummary};
use crate::license::LicensedFeature;
use crate::llm::chunked_cleanup;
use crate::require_license;
use crate::stt::build_stt_prompt;
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
/// Available to all users (free and paid) — cost transparency is a core differentiator.
#[tauri::command]
pub fn get_usage_stats(state: State<'_, AppState>) -> Result<UsageSummary, String> {
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

/// Returns `true` if the given tip has already been shown to the user.
///
/// `tip_id` is a stable string identifier defined by the frontend (e.g.
/// `"onboarding_hotkey"`). This lets the frontend persist per-tip state
/// without adding new config fields.
#[tauri::command]
pub fn is_tip_shown(state: State<'_, AppState>, tip_id: String) -> Result<bool, String> {
    let db = crate::lock!(state.inner().history_db)?;
    history::is_tip_shown(&db, &tip_id).map_err(|e| format!("Failed to check tip: {e}"))
}

/// Marks a tip as shown. Idempotent -- calling multiple times is safe.
#[tauri::command]
pub fn mark_tip_shown(state: State<'_, AppState>, tip_id: String) -> Result<(), String> {
    let db = crate::lock!(state.inner().history_db)?;
    history::mark_tip_shown(&db, &tip_id).map_err(|e| format!("Failed to mark tip shown: {e}"))
}

/// Re-processes a `pending` history entry (Story 12-2, AC5 — "Erneut
/// verarbeiten"): re-runs STT + cleanup on the WAV preserved from the
/// original terminal failure. On success the entry is promoted to `done`
/// (text/raw_text filled in) and the stored WAV is deleted. On any failure
/// (missing file, STT error, cleanup error) the entry is left untouched —
/// still `pending`, WAV still on disk — and the error is returned for the
/// frontend to show inline.
#[tauri::command]
pub async fn reprocess_pending_entry(
    state: State<'_, AppState>,
    id: i64,
) -> Result<history::HistoryEntry, String> {
    let inner = state.inner();

    let entry = {
        let db = crate::lock!(inner.history_db)?;
        history::get_entry_by_id(&db, id)
            .map_err(|e| format!("Failed to load entry: {e}"))?
            .ok_or_else(|| "History entry not found".to_string())?
    };
    if entry.status != "pending" {
        return Err("Entry is not pending".to_string());
    }
    let audio_path = entry
        .audio_path
        .clone()
        .ok_or_else(|| "Pending entry has no stored audio".to_string())?;

    let wav_bytes = std::fs::read(&audio_path)
        .map_err(|e| format!("Failed to read stored audio: {e}"))?;

    // Dictionary + STT prompt hint, same shape as transcribe_audio.
    let dict_prompt = {
        let guard = crate::lock!(inner.dictionary)?;
        let terms = guard.terms_as_prompt();
        let terms_opt = if terms.is_empty() { None } else { Some(terms) };
        build_stt_prompt(terms_opt.as_deref(), &entry.language)
    };

    let stt_provider = crate::read_lock!(inner.stt_provider)?.clone();
    let raw_text = stt_provider
        .transcribe(&wav_bytes, &entry.language, dict_prompt.as_deref())
        .await
        .map_err(|e| format!("Re-transcription failed: {e}"))?;

    let cfg = crate::lock!(inner.config)?.clone();
    let cleanup_provider = crate::read_lock!(inner.cleanup_provider)?.clone();
    let dict_list = {
        let guard = crate::lock!(inner.dictionary)?;
        let list = guard.terms_as_list();
        if list.is_empty() { None } else { Some(list) }
    };
    let custom_prompt = if cfg.custom_prompt.is_empty() {
        None
    } else {
        Some(cfg.custom_prompt.clone())
    };
    let output_lang = if cfg.output_language.is_empty() {
        None
    } else {
        Some(cfg.output_language.clone())
    };

    let cleanup_result = chunked_cleanup(
        &*cleanup_provider,
        &raw_text,
        cfg.cleanup_style,
        dict_list.as_deref(),
        custom_prompt.as_deref(),
        output_lang.as_deref(),
    )
    .await
    .map_err(|e| format!("Cleanup failed: {e}"))?;

    let db = crate::lock!(inner.history_db)?;
    let promoted =
        history::promote_pending_to_done(&db, id, &cleanup_result.text, &raw_text)
            .map_err(|e| format!("Failed to update history entry: {e}"))?;
    if !promoted {
        return Err("Entry was already processed or discarded".to_string());
    }
    // AC5: audio retention is transient — delete the WAV now that it has been
    // promoted. Best-effort: the entry is already correctly `done` regardless.
    if let Err(e) = std::fs::remove_file(&audio_path) {
        log::warn!("[history] failed to delete promoted WAV {audio_path}: {e}");
    }

    history::get_entry_by_id(&db, id)
        .map_err(|e| format!("Failed to reload entry: {e}"))?
        .ok_or_else(|| "Entry disappeared after promotion".to_string())
}

/// Discards a `pending` history entry (Story 12-2, AC6 — "Verwerfen"):
/// deletes both the history row and its stored WAV. Tolerates an
/// already-missing WAV file without error.
#[tauri::command]
pub fn discard_pending_entry(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    discard_pending_entry_inner(state.inner(), id)
}

/// Testable body of [`discard_pending_entry`] — takes a plain `&AppState` so
/// it can be exercised without a Tauri runtime.
fn discard_pending_entry_inner(inner: &AppState, id: i64) -> Result<(), String> {
    let audio_path = {
        let db = crate::lock!(inner.history_db)?;
        let entry = history::get_entry_by_id(&db, id)
            .map_err(|e| format!("Failed to load entry: {e}"))?
            .ok_or_else(|| "History entry not found".to_string())?;
        if entry.status != "pending" {
            return Err("Entry is not pending".to_string());
        }
        entry.audio_path
    };

    let db = crate::lock!(inner.history_db)?;
    history::delete_entry(&db, id).map_err(|e| format!("Failed to delete entry: {e}"))?;
    drop(db);

    if let Some(path) = &audio_path {
        if let Err(e) = std::fs::remove_file(path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!("[history] failed to delete discarded WAV {path}: {e}");
            }
        }
    }

    Ok(())
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::test_helpers::temp_dir;
    use std::fs;

    /// Builds an [`AppState`] backed by a real on-disk history schema
    /// (via [`history::open_db`]) rather than the bare in-memory connection
    /// `test_helpers::make_state` uses — the pending-entry tests below need
    /// the actual `history` table + columns to exist.
    fn make_history_state(dir: &tempfile::TempDir) -> AppState {
        let db = history::open_db(dir.path()).expect("history db must open");
        AppState::new(
            AppConfig::default(),
            crate::dictionary::Dictionary::new(),
            dir.path().to_path_buf(),
            db,
        )
    }

    /// Inserts a `pending` history row whose `audio_path` points at a real
    /// (freshly-written) WAV file in `dir`, returning `(id, path)`.
    fn make_pending_entry_with_wav(state: &AppState, dir: &std::path::Path) -> (i64, std::path::PathBuf) {
        let wav_path = dir.join("pending.wav");
        fs::write(&wav_path, b"RIFF....fake wav bytes").expect("failed to write fake wav");
        let db = state.history_db.lock().unwrap();
        let id = history::add_pending_entry(&db, wav_path.to_str().unwrap(), "en", None, None)
            .expect("failed to insert pending entry");
        (id, wav_path)
    }

    // Story 12-2 review fix (Finding 4, G-A): discard_pending_entry must
    // actually delete both the history row and the on-disk WAV.
    #[test]
    fn test_discard_pending_entry_deletes_row_and_wav() {
        let dir = temp_dir();
        let state = make_history_state(&dir);
        let (id, wav_path) = make_pending_entry_with_wav(&state, dir.path());

        discard_pending_entry_inner(&state, id).expect("discard must succeed");

        let db = state.history_db.lock().unwrap();
        assert!(
            history::get_entry_by_id(&db, id).unwrap().is_none(),
            "history row must be gone after discard"
        );
        drop(db);
        assert!(!wav_path.exists(), "WAV file must be deleted after discard");
    }

    // Story 12-2 review fix (Finding 4, AC6): discarding an entry whose WAV
    // is already missing on disk must still remove the row, without error.
    #[test]
    fn test_discard_pending_entry_tolerates_missing_wav() {
        let dir = temp_dir();
        let state = make_history_state(&dir);
        let (id, wav_path) = make_pending_entry_with_wav(&state, dir.path());
        fs::remove_file(&wav_path).expect("failed to pre-delete wav for test setup");

        let result = discard_pending_entry_inner(&state, id);
        assert!(result.is_ok(), "missing WAV must not error: {result:?}");

        let db = state.history_db.lock().unwrap();
        assert!(
            history::get_entry_by_id(&db, id).unwrap().is_none(),
            "history row must still be removed when the WAV was already gone"
        );
    }

    // Story 12-2 review fix (Finding 6): discard must refuse a non-pending
    // entry rather than silently deleting an already-`done` transcription.
    #[test]
    fn test_discard_pending_entry_rejects_non_pending_status() {
        let dir = temp_dir();
        let state = make_history_state(&dir);
        let db = state.history_db.lock().unwrap();
        let id = history::add_entry(&db, "Already done", None, "polished", "en", false, None, None, None)
            .expect("failed to insert done entry");
        drop(db);

        let result = discard_pending_entry_inner(&state, id);
        assert!(result.is_err(), "discard must reject a non-pending entry");

        let db = state.history_db.lock().unwrap();
        assert!(
            history::get_entry_by_id(&db, id).unwrap().is_some(),
            "a done entry must not be deleted by discard"
        );
    }

    // NOTE (test boundary): `reprocess_pending_entry`'s success path drives a
    // live STT + cleanup network call (Groq/DeepSeek), which this test suite
    // deliberately does not mock (per project-context.md testing rules — no
    // network-mock framework). Its state-machine guard (non-pending id is
    // rejected) and its downstream promote/WAV-delete step are covered above
    // and by `history::promote_pending_to_done`'s own tests
    // (`history/mod.rs`); the STT round-trip itself remains a manual/surface
    // verification item.
}
