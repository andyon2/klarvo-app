//! Dictation history module.
//!
//! Stores completed dictations in a local SQLite database so the user can
//! review, search, and re-copy past results.
//!
//! The database file is `{app_data_dir}/history.db`.

use std::path::Path;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum HistoryError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single dictation history entry.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    /// Cleaned text that was pasted.
    pub text: String,
    /// Raw transcript before LLM cleanup (if available).
    pub raw_text: Option<String>,
    /// Cleanup style used (polished, verbatim, chat).
    pub style: String,
    /// Language setting at time of dictation.
    pub language: String,
    /// Whether this entry is a voice note (saved, not pasted).
    #[serde(default)]
    pub is_note: bool,
    /// Window title of the app the user was dictating into (if captured).
    pub app_name: Option<String>,
    /// ISO 8601 timestamp.
    pub created_at: String,
    /// Stable UUID for cross-device sync deduplication.
    pub uuid: Option<String>,
    /// ID of the device that created this entry (set during sync).
    pub device_id: Option<String>,
    /// Lifecycle state: `"done"` (normal, complete), `"pending"` (terminal STT
    /// failure — audio preserved, awaiting manual re-process), or `"failed"`
    /// (reserved for future use). Story 12-2.
    pub status: String,
    /// Path to the preserved raw WAV on disk, set only for `pending` entries.
    /// Deleted (and this field cleared) on a successful re-process or an
    /// explicit discard. Story 12-2 (transient/"primitive A" audio retention).
    pub audio_path: Option<String>,
}

/// A single API usage entry for cost tracking.
#[allow(dead_code)] // constructed in tests; kept for future usage-list Tauri command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageEntry {
    pub id: i64,
    /// Service identifier, e.g. `"groq_stt"` or `"deepseek_cleanup"`.
    pub service: String,
    /// Duration of the recorded audio in milliseconds (STT only).
    pub audio_duration_ms: Option<i64>,
    /// LLM prompt tokens consumed.
    pub prompt_tokens: Option<i64>,
    /// LLM completion tokens consumed.
    pub completion_tokens: Option<i64>,
    /// Estimated cost in USD.
    pub estimated_cost_usd: f64,
    /// ISO 8601 timestamp.
    pub created_at: String,
}

/// Aggregated usage statistics returned by `get_usage_summary`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    /// Total number of completed dictations (rows in history table).
    pub total_dictations: i64,
    /// Approximate total word count across all dictations.
    pub total_words: i64,
    /// Total estimated cost across all services (USD).
    pub total_cost_usd: f64,
    /// Total audio recorded in seconds.
    pub total_audio_seconds: f64,
    /// Total cost for STT calls (USD).
    pub total_stt_cost_usd: f64,
    /// Total cost for LLM cleanup calls (USD).
    pub total_llm_cost_usd: f64,
    /// Number of dictations completed today.
    pub dictations_today: i64,
    /// Total cost incurred today (USD).
    pub cost_today_usd: f64,
}

// ---------------------------------------------------------------------------
// Database setup
// ---------------------------------------------------------------------------

const DB_FILE: &str = "history.db";

/// Opens (or creates) the history database and runs migrations.
pub fn open_db(app_data_dir: &Path) -> Result<Connection, HistoryError> {
    let path = app_data_dir.join(DB_FILE);
    let conn = Connection::open(path)?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS history (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            text       TEXT NOT NULL,
            raw_text   TEXT,
            style      TEXT NOT NULL DEFAULT 'polished',
            language   TEXT NOT NULL DEFAULT '',
            is_note    INTEGER NOT NULL DEFAULT 0,
            app_name   TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            status     TEXT NOT NULL DEFAULT 'done',
            audio_path TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_history_created_at ON history(created_at DESC);

        CREATE TABLE IF NOT EXISTS usage (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            service             TEXT NOT NULL,
            audio_duration_ms   INTEGER,
            prompt_tokens       INTEGER,
            completion_tokens   INTEGER,
            estimated_cost_usd  REAL NOT NULL DEFAULT 0,
            created_at          TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_usage_created_at ON usage(created_at DESC);

        CREATE TABLE IF NOT EXISTS tips_shown (
            tip_id   TEXT PRIMARY KEY,
            shown_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    // Migration: add is_note column for existing databases.
    let has_is_note: bool = conn.prepare("SELECT is_note FROM history LIMIT 0").is_ok();
    if !has_is_note {
        conn.execute_batch("ALTER TABLE history ADD COLUMN is_note INTEGER NOT NULL DEFAULT 0")?;
    }

    // Migration: add app_name column for existing databases.
    let has_app_name: bool = conn.prepare("SELECT app_name FROM history LIMIT 0").is_ok();
    if !has_app_name {
        conn.execute_batch("ALTER TABLE history ADD COLUMN app_name TEXT")?;
    }

    // Migration: add uuid column for sync deduplication.
    let has_uuid: bool = conn.prepare("SELECT uuid FROM history LIMIT 0").is_ok();
    if !has_uuid {
        conn.execute_batch("ALTER TABLE history ADD COLUMN uuid TEXT")?;
        // Backfill existing entries with generated UUIDs.
        let mut stmt = conn.prepare("SELECT id FROM history WHERE uuid IS NULL")?;
        let ids: Vec<i64> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        for id in ids {
            let uuid = Uuid::new_v4().to_string();
            conn.execute("UPDATE history SET uuid = ?1 WHERE id = ?2", params![uuid, id])?;
        }
        conn.execute_batch(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_history_uuid ON history(uuid)",
        )?;
    }

    // Migration: add device_id column.
    let has_device_id: bool = conn.prepare("SELECT device_id FROM history LIMIT 0").is_ok();
    if !has_device_id {
        conn.execute_batch("ALTER TABLE history ADD COLUMN device_id TEXT")?;
    }

    // Migration: add synced flag (0 = not yet pushed to remote).
    let has_synced: bool = conn.prepare("SELECT synced FROM history LIMIT 0").is_ok();
    if !has_synced {
        conn.execute_batch(
            "ALTER TABLE history ADD COLUMN synced INTEGER NOT NULL DEFAULT 0",
        )?;
    }

    // Migration: add status + audio_path columns (Story 12-2 — B-capable
    // schema for terminal-failure audio-retry history entries). Additive,
    // idempotent, mirrors the is_note migration above: every pre-existing row
    // reads back as status='done', audio_path=NULL.
    let has_status: bool = conn.prepare("SELECT status FROM history LIMIT 0").is_ok();
    if !has_status {
        conn.execute_batch("ALTER TABLE history ADD COLUMN status TEXT NOT NULL DEFAULT 'done'")?;
    }
    let has_audio_path: bool = conn.prepare("SELECT audio_path FROM history LIMIT 0").is_ok();
    if !has_audio_path {
        conn.execute_batch("ALTER TABLE history ADD COLUMN audio_path TEXT")?;
    }

    Ok(conn)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Inserts a new dictation into the history.
///
/// - `uuid`: stable identifier for cross-device sync. If `None`, a new v4 UUID
///   is generated automatically.
/// - `device_id`: ID of the originating device. Pass `None` for local entries.
#[allow(clippy::too_many_arguments)]
pub fn add_entry(
    conn: &Connection,
    text: &str,
    raw_text: Option<&str>,
    style: &str,
    language: &str,
    is_note: bool,
    app_name: Option<&str>,
    uuid: Option<&str>,
    device_id: Option<&str>,
) -> Result<i64, HistoryError> {
    let entry_uuid = uuid
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    conn.execute(
        "INSERT INTO history (text, raw_text, style, language, is_note, app_name, uuid, device_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![text, raw_text, style, language, is_note as i32, app_name, entry_uuid, device_id],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Reads a `HistoryEntry` from a row.
///
/// Expected column order: id, text, raw_text, style, language, is_note,
/// app_name, created_at, uuid, device_id, status, audio_path.
fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<HistoryEntry> {
    Ok(HistoryEntry {
        id: row.get(0)?,
        text: row.get(1)?,
        raw_text: row.get(2)?,
        style: row.get(3)?,
        language: row.get(4)?,
        is_note: row.get::<_, i32>(5)? != 0,
        app_name: row.get(6)?,
        created_at: row.get(7)?,
        uuid: row.get(8)?,
        device_id: row.get(9)?,
        status: row.get(10)?,
        audio_path: row.get(11)?,
    })
}

const SELECT_COLUMNS: &str =
    "id, text, raw_text, style, language, is_note, app_name, created_at, uuid, device_id, status, audio_path";

/// Returns the most recent history entries (newest first), excluding notes.
pub fn get_entries(conn: &Connection, limit: u32) -> Result<Vec<HistoryEntry>, HistoryError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM history WHERE is_note = 0 ORDER BY created_at DESC, id DESC LIMIT ?1",
    ))?;
    let entries = stmt.query_map(params![limit], row_to_entry)?.collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

/// Returns the most recent voice notes (newest first).
pub fn get_notes(conn: &Connection, limit: u32) -> Result<Vec<HistoryEntry>, HistoryError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM history WHERE is_note = 1 ORDER BY created_at DESC, id DESC LIMIT ?1",
    ))?;
    let entries = stmt.query_map(params![limit], row_to_entry)?.collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

/// Returns a single history entry by ID, or `None` if it doesn't exist.
pub fn get_entry_by_id(conn: &Connection, id: i64) -> Result<Option<HistoryEntry>, HistoryError> {
    let mut stmt = conn.prepare(&format!("SELECT {SELECT_COLUMNS} FROM history WHERE id = ?1"))?;
    let mut rows = stmt.query_map(params![id], row_to_entry)?;
    match rows.next() {
        Some(row) => Ok(Some(row?)),
        None => Ok(None),
    }
}

/// Inserts a `pending` history entry for a terminal STT failure whose raw
/// audio was preserved to disk (AC2/AC3). `text`/`raw_text` are left empty —
/// the frontend renders a placeholder for `pending` entries instead.
pub fn add_pending_entry(
    conn: &Connection,
    audio_path: &str,
    language: &str,
    app_name: Option<&str>,
    device_id: Option<&str>,
) -> Result<i64, HistoryError> {
    let entry_uuid = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO history (text, style, language, is_note, app_name, uuid, device_id, status, audio_path) \
         VALUES ('', 'verbatim', ?1, 0, ?2, ?3, ?4, 'pending', ?5)",
        params![language, app_name, entry_uuid, device_id, audio_path],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Promotes a `pending` entry to `done` after a successful re-process (AC5):
/// fills in the produced text/raw_text and clears `audio_path` (the caller is
/// responsible for deleting the WAV file itself). Only affects rows that are
/// still `pending` — a no-op (returns `false`) if the entry was already
/// promoted/discarded concurrently or doesn't exist.
pub fn promote_pending_to_done(
    conn: &Connection,
    id: i64,
    text: &str,
    raw_text: &str,
) -> Result<bool, HistoryError> {
    let affected = conn.execute(
        "UPDATE history SET text = ?1, raw_text = ?2, status = 'done', audio_path = NULL \
         WHERE id = ?3 AND status = 'pending'",
        params![text, raw_text, id],
    )?;
    Ok(affected > 0)
}

/// Searches history entries by text content and/or app name (case-insensitive).
///
/// - Both `Some`: entries must match text AND app name.
/// - Only `text_query`: matches text content only.
/// - Only `app_query`: matches app name only.
/// - Both `None`: returns recent entries (same as `get_entries`).
pub fn search_entries(
    conn: &Connection,
    text_query: Option<&str>,
    app_query: Option<&str>,
    limit: u32,
) -> Result<Vec<HistoryEntry>, HistoryError> {
    match (text_query, app_query) {
        (Some(tq), Some(aq)) => {
            let tp = format!("%{tq}%");
            let ap = format!("%{aq}%");
            let mut stmt = conn.prepare(&format!(
                "SELECT {SELECT_COLUMNS} FROM history WHERE text LIKE ?1 AND app_name LIKE ?2
                 ORDER BY created_at DESC, id DESC LIMIT ?3",
            ))?;
            let entries = stmt.query_map(params![tp, ap, limit], row_to_entry)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(entries)
        }
        (Some(tq), None) => {
            let tp = format!("%{tq}%");
            let mut stmt = conn.prepare(&format!(
                "SELECT {SELECT_COLUMNS} FROM history WHERE text LIKE ?1
                 ORDER BY created_at DESC, id DESC LIMIT ?2",
            ))?;
            let entries = stmt.query_map(params![tp, limit], row_to_entry)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(entries)
        }
        (None, Some(aq)) => {
            let ap = format!("%{aq}%");
            let mut stmt = conn.prepare(&format!(
                "SELECT {SELECT_COLUMNS} FROM history WHERE app_name LIKE ?1
                 ORDER BY created_at DESC, id DESC LIMIT ?2",
            ))?;
            let entries = stmt.query_map(params![ap, limit], row_to_entry)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(entries)
        }
        (None, None) => get_entries(conn, limit),
    }
}

/// Deletes a single history entry by ID.
pub fn delete_entry(conn: &Connection, id: i64) -> Result<bool, HistoryError> {
    let affected = conn.execute("DELETE FROM history WHERE id = ?1", params![id])?;
    Ok(affected > 0)
}

/// Deletes all history entries.
pub fn clear_history(conn: &Connection) -> Result<u64, HistoryError> {
    let affected = conn.execute("DELETE FROM history", [])?;
    Ok(affected as u64)
}

/// Records an API usage event for cost tracking.
///
/// - `service`: identifier string, e.g. `"groq_stt"` or `"deepseek_cleanup"`.
/// - `audio_duration_ms`: audio length in ms (STT only; pass `None` for LLM calls).
/// - `prompt_tokens`: LLM prompt tokens (pass `None` for STT calls).
/// - `completion_tokens`: LLM completion tokens (pass `None` for STT calls).
/// - `estimated_cost_usd`: pre-computed cost in USD.
pub fn record_usage(
    conn: &Connection,
    service: &str,
    audio_duration_ms: Option<i64>,
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    estimated_cost_usd: f64,
) -> Result<i64, HistoryError> {
    conn.execute(
        "INSERT INTO usage (service, audio_duration_ms, prompt_tokens, completion_tokens, estimated_cost_usd)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            service,
            audio_duration_ms,
            prompt_tokens.map(|v| v as i64),
            completion_tokens.map(|v| v as i64),
            estimated_cost_usd,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Returns aggregated usage statistics across all recorded sessions.
///
/// Word count is approximated using character counting:
/// `length(text) - length(replace(text, ' ', '')) + 1`.
pub fn get_usage_summary(conn: &Connection) -> Result<UsageSummary, HistoryError> {
    // Total dictations and word count from history table.
    let (total_dictations, total_words): (i64, i64) = conn.query_row(
        "SELECT
            COUNT(*),
            COALESCE(SUM(length(text) - length(replace(text, ' ', '')) + 1), 0)
         FROM history",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    // Total audio seconds (from STT usage rows).
    let total_audio_ms: i64 = conn.query_row(
        "SELECT COALESCE(SUM(audio_duration_ms), 0) FROM usage WHERE audio_duration_ms IS NOT NULL",
        [],
        |row| row.get(0),
    )?;

    // Total cost by service category.
    let total_stt_cost_usd: f64 = conn.query_row(
        "SELECT COALESCE(SUM(estimated_cost_usd), 0.0) FROM usage WHERE service LIKE '%stt%'",
        [],
        |row| row.get(0),
    )?;

    let total_llm_cost_usd: f64 = conn.query_row(
        "SELECT COALESCE(SUM(estimated_cost_usd), 0.0) FROM usage WHERE service NOT LIKE '%stt%'",
        [],
        |row| row.get(0),
    )?;

    // Today's stats.
    let (dictations_today, cost_today_usd): (i64, f64) = conn.query_row(
        "SELECT
            (SELECT COUNT(*) FROM history WHERE date(created_at) = date('now')),
            (SELECT COALESCE(SUM(estimated_cost_usd), 0.0) FROM usage WHERE date(created_at) = date('now'))",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    Ok(UsageSummary {
        total_dictations,
        total_words,
        total_cost_usd: total_stt_cost_usd + total_llm_cost_usd,
        total_audio_seconds: total_audio_ms as f64 / 1000.0,
        total_stt_cost_usd,
        total_llm_cost_usd,
        dictations_today,
        cost_today_usd,
    })
}

// ---------------------------------------------------------------------------
// Tips tracking
// ---------------------------------------------------------------------------

/// Returns `true` if the given tip has already been shown to the user.
pub fn is_tip_shown(conn: &Connection, tip_id: &str) -> Result<bool, HistoryError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tips_shown WHERE tip_id = ?1",
        params![tip_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Records that a tip has been shown. Idempotent -- calling twice is safe.
pub fn mark_tip_shown(conn: &Connection, tip_id: &str) -> Result<(), HistoryError> {
    conn.execute(
        "INSERT OR IGNORE INTO tips_shown (tip_id) VALUES (?1)",
        params![tip_id],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Filler word statistics
// ---------------------------------------------------------------------------

/// A single filler word with its occurrence count.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FillerStat {
    pub word: String,
    pub count: i64,
}

/// Known filler words to track (German + English).
const FILLER_WORDS: &[&str] = &[
    // German
    "äh", "ähm", "also", "sozusagen", "quasi", "halt", "irgendwie",
    "eigentlich", "praktisch", "gewissermaßen", "na ja", "genau", "tja",
    // English
    "uh", "um", "like", "you know", "basically", "actually", "literally",
    "I mean", "kind of", "sort of",
];

/// Analyzes all raw transcripts in the history for filler word occurrences.
///
/// Returns a list sorted by count (most frequent first). Only fillers with
/// count > 0 are included.
pub fn get_filler_stats(conn: &Connection) -> Result<Vec<FillerStat>, HistoryError> {
    let mut stmt = conn.prepare(
        "SELECT raw_text FROM history WHERE raw_text IS NOT NULL AND raw_text != ''"
    )?;

    let raw_texts: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();

    let mut counts: Vec<(String, i64)> = FILLER_WORDS
        .iter()
        .map(|&word| {
            let lower_word = word.to_lowercase();
            let word_bytes = lower_word.len();
            let count: i64 = raw_texts.iter().map(|text| {
                let lower_text = text.to_lowercase();
                let text_bytes = lower_text.as_bytes();
                let word_b = lower_word.as_bytes();
                let mut n = 0i64;
                let mut start = 0usize;
                while start + word_bytes <= text_bytes.len() {
                    if let Some(pos) = lower_text[start..].find(&lower_word) {
                        let abs_pos = start + pos;
                        let end_pos = abs_pos + word_bytes;
                        // Check word boundaries using chars
                        let before_ok = abs_pos == 0 || {
                            let before = &lower_text[..abs_pos];
                            !before.chars().next_back().unwrap_or(' ').is_alphanumeric()
                        };
                        let after_ok = end_pos >= lower_text.len() || {
                            let after = &lower_text[end_pos..];
                            !after.chars().next().unwrap_or(' ').is_alphanumeric()
                        };
                        if before_ok && after_ok {
                            n += 1;
                        }
                        // Advance past this match (at least 1 byte, staying on char boundary)
                        start = end_pos;
                    } else {
                        break;
                    }
                }
                let _ = word_b; // suppress unused
                n
            }).sum();
            (word.to_string(), count)
        })
        .filter(|(_, count)| *count > 0)
        .collect();

    counts.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(counts.into_iter().map(|(word, count)| FillerStat { word, count }).collect())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                text       TEXT NOT NULL,
                raw_text   TEXT,
                style      TEXT NOT NULL DEFAULT 'polished',
                language   TEXT NOT NULL DEFAULT '',
                is_note    INTEGER NOT NULL DEFAULT 0,
                app_name   TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                uuid       TEXT,
                device_id  TEXT,
                synced     INTEGER NOT NULL DEFAULT 0,
                status     TEXT NOT NULL DEFAULT 'done',
                audio_path TEXT
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_history_uuid ON history(uuid);
            CREATE TABLE IF NOT EXISTS usage (
                id                  INTEGER PRIMARY KEY AUTOINCREMENT,
                service             TEXT NOT NULL,
                audio_duration_ms   INTEGER,
                prompt_tokens       INTEGER,
                completion_tokens   INTEGER,
                estimated_cost_usd  REAL NOT NULL DEFAULT 0,
                created_at          TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS tips_shown (
                tip_id   TEXT PRIMARY KEY,
                shown_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_add_and_get_entry() {
        let conn = mem_db();
        let id = add_entry(&conn, "Hello world", Some("hello world"), "polished", "en", false, None, None, None).unwrap();
        assert!(id > 0);

        let entries = get_entries(&conn, 10).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "Hello world");
        assert_eq!(entries[0].raw_text.as_deref(), Some("hello world"));
        assert_eq!(entries[0].style, "polished");
    }

    #[test]
    fn test_get_entries_ordered_newest_first() {
        let conn = mem_db();
        add_entry(&conn, "First", None, "polished", "de", false, None, None, None).unwrap();
        add_entry(&conn, "Second", None, "polished", "de", false, None, None, None).unwrap();
        add_entry(&conn, "Third", None, "polished", "de", false, None, None, None).unwrap();

        let entries = get_entries(&conn, 10).unwrap();
        assert_eq!(entries.len(), 3);
        // IDs should be descending (newest first)
        assert!(entries[0].id > entries[1].id);
        assert!(entries[1].id > entries[2].id);
    }

    #[test]
    fn test_get_entries_limit() {
        let conn = mem_db();
        for i in 0..10 {
            add_entry(&conn, &format!("Entry {i}"), None, "polished", "", false, None, None, None).unwrap();
        }
        let entries = get_entries(&conn, 3).unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_search_entries_text_only() {
        let conn = mem_db();
        add_entry(&conn, "Kubernetes deployment", None, "polished", "en", false, None, None, None).unwrap();
        add_entry(&conn, "Hello world", None, "polished", "en", false, None, None, None).unwrap();
        add_entry(&conn, "Kubernetes service", None, "polished", "en", false, None, None, None).unwrap();

        let results = search_entries(&conn, Some("kubernetes"), None, 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_entries_app_only() {
        let conn = mem_db();
        add_entry(&conn, "Hello from Slack", None, "polished", "en", false, Some("Slack - #general"), None, None).unwrap();
        add_entry(&conn, "Hello from VS Code", None, "polished", "en", false, Some("Visual Studio Code"), None, None).unwrap();
        add_entry(&conn, "No app context", None, "polished", "en", false, None, None, None).unwrap();

        let results = search_entries(&conn, None, Some("Slack"), 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].app_name.as_deref(), Some("Slack - #general"));
    }

    #[test]
    fn test_search_entries_text_and_app() {
        let conn = mem_db();
        add_entry(&conn, "Deploy k8s", None, "polished", "en", false, Some("Terminal"), None, None).unwrap();
        add_entry(&conn, "Deploy k8s", None, "polished", "en", false, Some("Slack"), None, None).unwrap();
        add_entry(&conn, "Hello world", None, "polished", "en", false, Some("Terminal"), None, None).unwrap();

        let results = search_entries(&conn, Some("Deploy"), Some("Terminal"), 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].app_name.as_deref(), Some("Terminal"));
    }

    #[test]
    fn test_search_entries_none_returns_all() {
        let conn = mem_db();
        add_entry(&conn, "A", None, "polished", "en", false, None, None, None).unwrap();
        add_entry(&conn, "B", None, "polished", "en", false, None, None, None).unwrap();

        let results = search_entries(&conn, None, None, 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_delete_entry() {
        let conn = mem_db();
        let id = add_entry(&conn, "To delete", None, "polished", "", false, None, None, None).unwrap();
        assert!(delete_entry(&conn, id).unwrap());
        assert!(!delete_entry(&conn, id).unwrap()); // already deleted

        let entries = get_entries(&conn, 10).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_clear_history() {
        let conn = mem_db();
        add_entry(&conn, "A", None, "polished", "", false, None, None, None).unwrap();
        add_entry(&conn, "B", None, "chat", "", false, None, None, None).unwrap();

        let deleted = clear_history(&conn).unwrap();
        assert_eq!(deleted, 2);

        let entries = get_entries(&conn, 10).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_entry_serializes_camel_case() {
        let entry = HistoryEntry {
            id: 1,
            text: "test".to_string(),
            raw_text: Some("raw".to_string()),
            style: "polished".to_string(),
            language: "de".to_string(),
            is_note: false,
            app_name: Some("Slack".to_string()),
            created_at: "2026-03-07T12:00:00".to_string(),
            uuid: Some("test-uuid".to_string()),
            device_id: None,
            status: "done".to_string(),
            audio_path: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("rawText"));
        assert!(json.contains("createdAt"));
        assert!(json.contains("appName"));
        assert!(json.contains("\"status\""));
        assert!(json.contains("audioPath"));
    }

    // --- Usage tracking ---

    #[test]
    fn test_record_usage_stt() {
        let conn = mem_db();
        let id = record_usage(&conn, "groq_stt", Some(3000), None, None, 0.000033).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn test_record_usage_llm() {
        let conn = mem_db();
        let id = record_usage(&conn, "deepseek_cleanup", None, Some(100), Some(50), 0.000082).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn test_get_usage_summary_empty() {
        let conn = mem_db();
        let summary = get_usage_summary(&conn).unwrap();
        assert_eq!(summary.total_dictations, 0);
        assert_eq!(summary.total_words, 0);
        assert_eq!(summary.total_cost_usd, 0.0);
        assert_eq!(summary.total_audio_seconds, 0.0);
        assert_eq!(summary.dictations_today, 0);
        assert_eq!(summary.cost_today_usd, 0.0);
    }

    #[test]
    fn test_get_usage_summary_with_data() {
        let conn = mem_db();

        // Two history entries.
        add_entry(&conn, "Hello world", None, "polished", "en", false, None, None, None).unwrap();
        add_entry(&conn, "Kubernetes deployment works", None, "polished", "en", false, None, None, None).unwrap();

        // STT usage: 5000ms audio, cost = 5000/3600000 * 0.04 ≈ 0.0000556
        record_usage(&conn, "groq_stt", Some(5000), None, None, 0.0000556).unwrap();
        // LLM usage: 80 prompt + 40 completion tokens
        record_usage(&conn, "deepseek_cleanup", None, Some(80), Some(40), 0.000066).unwrap();

        let summary = get_usage_summary(&conn).unwrap();

        assert_eq!(summary.total_dictations, 2);
        // "Hello world" = 2 words, "Kubernetes deployment works" = 3 words => 5 total
        assert_eq!(summary.total_words, 5);
        assert!((summary.total_audio_seconds - 5.0).abs() < 0.001);
        assert!(summary.total_stt_cost_usd > 0.0);
        assert!(summary.total_llm_cost_usd > 0.0);
        assert!(summary.total_cost_usd > 0.0);
        // Today's entries were just inserted, so dictations_today >= 2
        assert!(summary.dictations_today >= 2);
        assert!(summary.cost_today_usd > 0.0);
    }

    #[test]
    fn test_usage_summary_splits_stt_and_llm_costs() {
        let conn = mem_db();
        record_usage(&conn, "groq_stt", Some(1000), None, None, 0.1).unwrap();
        record_usage(&conn, "deepseek_cleanup", None, Some(100), Some(50), 0.2).unwrap();

        let summary = get_usage_summary(&conn).unwrap();
        assert!((summary.total_stt_cost_usd - 0.1).abs() < 1e-9);
        assert!((summary.total_llm_cost_usd - 0.2).abs() < 1e-9);
        assert!((summary.total_cost_usd - 0.3).abs() < 1e-9);
    }

    #[test]
    fn test_usage_entry_serializes_camel_case() {
        let entry = UsageEntry {
            id: 1,
            service: "groq_stt".to_string(),
            audio_duration_ms: Some(3000),
            prompt_tokens: None,
            completion_tokens: None,
            estimated_cost_usd: 0.000033,
            created_at: "2026-03-07T12:00:00".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("audioDurationMs"));
        assert!(json.contains("promptTokens"));
        assert!(json.contains("completionTokens"));
        assert!(json.contains("estimatedCostUsd"));
        assert!(json.contains("createdAt"));
    }

    #[test]
    fn test_usage_summary_serializes_camel_case() {
        let summary = UsageSummary {
            total_dictations: 10,
            total_words: 100,
            total_cost_usd: 0.5,
            total_audio_seconds: 30.0,
            total_stt_cost_usd: 0.2,
            total_llm_cost_usd: 0.3,
            dictations_today: 3,
            cost_today_usd: 0.05,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("totalDictations"));
        assert!(json.contains("totalWords"));
        assert!(json.contains("totalCostUsd"));
        assert!(json.contains("totalAudioSeconds"));
        assert!(json.contains("totalSttCostUsd"));
        assert!(json.contains("totalLlmCostUsd"));
        assert!(json.contains("dictationsToday"));
        assert!(json.contains("costTodayUsd"));
    }

    // --- Filler stats ---

    #[test]
    fn test_filler_stats_empty_db() {
        let conn = mem_db();
        let stats = get_filler_stats(&conn).unwrap();
        assert!(stats.is_empty());
    }

    #[test]
    fn test_filler_stats_counts_fillers() {
        let conn = mem_db();
        add_entry(&conn, "cleaned", Some("also äh ich meine also halt"), "polished", "de", false, None, None, None).unwrap();
        add_entry(&conn, "cleaned", Some("basically like you know"), "polished", "en", false, None, None, None).unwrap();

        let stats = get_filler_stats(&conn).unwrap();
        assert!(!stats.is_empty());

        let also_count = stats.iter().find(|s| s.word == "also").map(|s| s.count).unwrap_or(0);
        assert_eq!(also_count, 2);

        let basically_count = stats.iter().find(|s| s.word == "basically").map(|s| s.count).unwrap_or(0);
        assert_eq!(basically_count, 1);
    }

    #[test]
    fn test_filler_stats_sorted_by_count() {
        let conn = mem_db();
        add_entry(&conn, "cleaned", Some("äh äh äh also halt"), "polished", "de", false, None, None, None).unwrap();

        let stats = get_filler_stats(&conn).unwrap();
        assert!(stats.len() >= 2);
        assert!(stats[0].count >= stats[1].count, "Should be sorted by count descending");
    }

    #[test]
    fn test_filler_stat_serializes_camel_case() {
        let stat = FillerStat { word: "äh".to_string(), count: 5 };
        let json = serde_json::to_string(&stat).unwrap();
        assert!(json.contains("\"word\""));
        assert!(json.contains("\"count\""));
    }

    // --- Tips tracking ---

    #[test]
    fn test_is_tip_shown_new_tip_returns_false() {
        let conn = mem_db();
        let shown = is_tip_shown(&conn, "onboarding_hotkey").unwrap();
        assert!(!shown, "a brand-new tip must not be shown yet");
    }

    #[test]
    fn test_mark_tip_shown_then_is_tip_shown_returns_true() {
        let conn = mem_db();
        mark_tip_shown(&conn, "onboarding_hotkey").unwrap();
        let shown = is_tip_shown(&conn, "onboarding_hotkey").unwrap();
        assert!(shown, "tip must be shown after mark_tip_shown");
    }

    #[test]
    fn test_mark_tip_shown_idempotent() {
        let conn = mem_db();
        // Calling twice must not produce an error.
        mark_tip_shown(&conn, "onboarding_hotkey").unwrap();
        mark_tip_shown(&conn, "onboarding_hotkey").unwrap();
        let shown = is_tip_shown(&conn, "onboarding_hotkey").unwrap();
        assert!(shown);
    }

    // --- Migration ladder regression tests ---

    /// OLD pre-migration `history` schema — the state before ANY of open_db's five
    /// ALTER migrations (is_note, app_name, uuid, device_id, synced) ran.
    /// Shared verbatim by both ladder tests so the precondition cannot drift apart.
    const OLD_SCHEMA_DDL: &str = "CREATE TABLE history (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            text       TEXT NOT NULL,
            raw_text   TEXT,
            style      TEXT NOT NULL DEFAULT 'polished',
            language   TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX idx_history_created_at ON history(created_at DESC);";

    #[test]
    fn test_open_db_runs_migration_ladder() {
        // Verify that open_db() correctly upgrades an OLD pre-migration schema.
        // The OLD schema lacks is_note, app_name, uuid, device_id, synced columns —
        // all five must be present after open_db() runs, and the pre-existing row
        // must be UUID-backfilled (with its data intact) and a UNIQUE index created.
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("history.db");

        let row_id: i64;
        {
            // Build the OLD schema in a real file (open_db requires a file, not in-memory).
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(OLD_SCHEMA_DDL).unwrap();
            // Precondition guard, coupled by construction to the DB we actually migrate:
            // the OLD schema must genuinely lack uuid BEFORE open_db() runs, otherwise the
            // post-migration assertions below would be tautological.
            assert!(
                conn.prepare("SELECT uuid FROM history LIMIT 0").is_err(),
                "precondition: OLD schema must not have uuid before migration"
            );
            // Insert with bare SQL — add_entry() requires uuid/device_id columns.
            conn.execute(
                "INSERT INTO history (text, style, language) VALUES (?1, ?2, ?3)",
                params!["Migration test entry", "verbatim", "en"],
            )
            .unwrap();
            row_id = conn.last_insert_rowid();
            // conn drops here, releasing the file lock before open_db() is called.
        }

        // Call the REAL open_db() migration ladder (AC-1).
        let conn = open_db(dir.path()).unwrap();

        // AC-2: All migrated columns must exist after migration.
        for col in ["is_note", "app_name", "uuid", "device_id", "synced", "status", "audio_path"] {
            let sql = format!("SELECT {} FROM history LIMIT 0", col);
            assert!(
                conn.prepare(&sql).is_ok(),
                "{} column must exist after open_db migration",
                col
            );
        }

        // Data survival: the pre-existing row must be preserved verbatim and neither
        // dropped nor duplicated by a stray table-recreate. This is precisely the
        // "silently corrupting an existing user's history" failure the story guards against.
        let row_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            row_count, 1,
            "migration must preserve exactly the pre-existing row"
        );
        let (text, style, language): (String, String, String) = conn
            .query_row(
                "SELECT text, style, language FROM history WHERE id = ?1",
                params![row_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(text, "Migration test entry", "row text must survive migration");
        assert_eq!(style, "verbatim", "row style must survive migration");
        assert_eq!(language, "en", "row language must survive migration");

        // AC1: the pre-existing row must default to status='done', audio_path=NULL.
        let (status, audio_path): (String, Option<String>) = conn
            .query_row(
                "SELECT status, audio_path FROM history WHERE id = ?1",
                params![row_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "done", "pre-existing row must default to status='done'");
        assert_eq!(audio_path, None, "pre-existing row must default to audio_path=NULL");

        // AC-3: The pre-existing row must have been UUID-backfilled.
        let uuid_val: Option<String> = conn
            .query_row(
                "SELECT uuid FROM history WHERE id = ?1",
                params![row_id],
                |r| r.get(0),
            )
            .unwrap();
        let uuid_str = uuid_val.expect("uuid must not be NULL after backfill");
        assert_eq!(uuid_str.len(), 36, "uuid must be 36-char v4 format");
        assert_eq!(&uuid_str[8..9], "-", "uuid hyphen at position 8");
        assert_eq!(&uuid_str[13..14], "-", "uuid hyphen at position 13");
        assert_eq!(&uuid_str[18..19], "-", "uuid hyphen at position 18");
        assert_eq!(&uuid_str[23..24], "-", "uuid hyphen at position 23");

        // AC-4: The index on uuid must exist AND be UNIQUE. PRAGMA index_list returns
        // the `unique` flag in column 2 — assert it, not just the index name, so a
        // regression to a non-unique index (the cross-device sync dedup contract) is caught.
        let mut stmt = conn.prepare("PRAGMA index_list(history)").unwrap();
        let uuid_index_unique: Option<bool> = stmt
            .query_map([], |row| Ok((row.get::<_, String>(1)?, row.get::<_, i64>(2)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .find(|(name, _)| name == "idx_history_uuid")
            .map(|(_, unique)| unique == 1);
        assert_eq!(
            uuid_index_unique,
            Some(true),
            "idx_history_uuid must exist and be UNIQUE after open_db migration"
        );
    }

    #[test]
    fn test_open_db_migration_ladder_is_non_tautological() {
        // Pre-condition guard: confirms the OLD schema truly lacks uuid BEFORE any migration.
        // This test does NOT call open_db — it proves the pre-condition is genuine so the
        // main migration test is not accidentally asserting on an already-migrated schema.
        // Uses the SAME OLD_SCHEMA_DDL as the main test so the two cannot drift apart.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(OLD_SCHEMA_DDL).unwrap();
        let has_uuid = conn.prepare("SELECT uuid FROM history LIMIT 0").is_ok();
        assert!(
            !has_uuid,
            "old schema must not have uuid column — pre-condition guard for migration ladder test"
        );
    }

    // --- Story 12-2: pending/audio-retry entries ---

    #[test]
    fn test_add_pending_entry_creates_pending_row_with_audio_path() {
        let conn = mem_db();
        let id = add_pending_entry(&conn, "/tmp/pending/123.wav", "de", Some("Slack"), None).unwrap();
        assert!(id > 0);

        let entry = get_entry_by_id(&conn, id).unwrap().expect("entry must exist");
        assert_eq!(entry.status, "pending");
        assert_eq!(entry.audio_path.as_deref(), Some("/tmp/pending/123.wav"));
        assert_eq!(entry.is_note, false);
        assert_eq!(entry.language, "de");
        assert_eq!(entry.app_name.as_deref(), Some("Slack"));
    }

    #[test]
    fn test_pending_entry_appears_in_get_entries() {
        // Pending entries are failed dictations (is_note=0) — they must be
        // discoverable in the same dictation-history list as normal entries
        // (Dev Notes: "belong in the dictation-history view").
        let conn = mem_db();
        add_pending_entry(&conn, "/tmp/pending/1.wav", "en", None, None).unwrap();
        let entries = get_entries(&conn, 10).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].status, "pending");
    }

    #[test]
    fn test_get_entry_by_id_missing_returns_none() {
        let conn = mem_db();
        assert!(get_entry_by_id(&conn, 999).unwrap().is_none());
    }

    #[test]
    fn test_promote_pending_to_done_success() {
        let conn = mem_db();
        let id = add_pending_entry(&conn, "/tmp/pending/1.wav", "en", None, None).unwrap();

        let promoted = promote_pending_to_done(&conn, id, "Cleaned text", "raw text").unwrap();
        assert!(promoted);

        let entry = get_entry_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(entry.status, "done");
        assert_eq!(entry.text, "Cleaned text");
        assert_eq!(entry.raw_text.as_deref(), Some("raw text"));
        assert_eq!(entry.audio_path, None, "audio_path must be cleared on promotion");
    }

    #[test]
    fn test_promote_pending_to_done_is_noop_for_already_done_entry() {
        // Guards against double-promotion (e.g. a concurrent re-process click):
        // once an entry is 'done' it must not be re-promotable.
        let conn = mem_db();
        let id = add_entry(&conn, "Already done", None, "polished", "en", false, None, None, None).unwrap();

        let promoted = promote_pending_to_done(&conn, id, "New text", "new raw").unwrap();
        assert!(!promoted);

        let entry = get_entry_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(entry.text, "Already done", "a non-pending row must not be overwritten");
    }

    #[test]
    fn test_promote_pending_to_done_missing_id_returns_false() {
        let conn = mem_db();
        assert!(!promote_pending_to_done(&conn, 999, "x", "y").unwrap());
    }

    #[test]
    fn test_discard_pending_entry_via_delete_entry() {
        // Verwerfen (AC6): delete_entry already removes the row regardless of
        // status; the WAV-file deletion itself is the command layer's job
        // (history has no filesystem access), covered by the pipeline/command
        // integration tests.
        let conn = mem_db();
        let id = add_pending_entry(&conn, "/tmp/pending/1.wav", "en", None, None).unwrap();
        assert!(delete_entry(&conn, id).unwrap());
        assert!(get_entry_by_id(&conn, id).unwrap().is_none());
    }

    #[test]
    fn test_happy_path_add_entry_defaults_to_done_no_audio_path() {
        // AC7: normal add_entry() calls must be byte-identical to today —
        // status defaults to 'done' and audio_path stays NULL via the column
        // defaults, without add_entry's signature changing at all.
        let conn = mem_db();
        let id = add_entry(&conn, "Normal dictation", Some("raw"), "polished", "en", false, None, None, None).unwrap();
        let entry = get_entry_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(entry.status, "done");
        assert_eq!(entry.audio_path, None);
    }
}
