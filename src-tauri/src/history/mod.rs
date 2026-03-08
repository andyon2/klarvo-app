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
}

/// A single API usage entry for cost tracking.
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
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
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
        CREATE INDEX IF NOT EXISTS idx_usage_created_at ON usage(created_at DESC);",
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
/// app_name, created_at, uuid, device_id.
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
    })
}

/// Returns the most recent history entries (newest first), excluding notes.
pub fn get_entries(conn: &Connection, limit: u32) -> Result<Vec<HistoryEntry>, HistoryError> {
    let mut stmt = conn.prepare(
        "SELECT id, text, raw_text, style, language, is_note, app_name, created_at, uuid, device_id
         FROM history WHERE is_note = 0 ORDER BY created_at DESC, id DESC LIMIT ?1",
    )?;
    let entries = stmt.query_map(params![limit], row_to_entry)?.collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
}

/// Returns the most recent voice notes (newest first).
pub fn get_notes(conn: &Connection, limit: u32) -> Result<Vec<HistoryEntry>, HistoryError> {
    let mut stmt = conn.prepare(
        "SELECT id, text, raw_text, style, language, is_note, app_name, created_at, uuid, device_id
         FROM history WHERE is_note = 1 ORDER BY created_at DESC, id DESC LIMIT ?1",
    )?;
    let entries = stmt.query_map(params![limit], row_to_entry)?.collect::<Result<Vec<_>, _>>()?;
    Ok(entries)
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
            let mut stmt = conn.prepare(
                "SELECT id, text, raw_text, style, language, is_note, app_name, created_at, uuid, device_id
                 FROM history WHERE text LIKE ?1 AND app_name LIKE ?2
                 ORDER BY created_at DESC, id DESC LIMIT ?3",
            )?;
            let entries = stmt.query_map(params![tp, ap, limit], row_to_entry)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(entries)
        }
        (Some(tq), None) => {
            let tp = format!("%{tq}%");
            let mut stmt = conn.prepare(
                "SELECT id, text, raw_text, style, language, is_note, app_name, created_at, uuid, device_id
                 FROM history WHERE text LIKE ?1
                 ORDER BY created_at DESC, id DESC LIMIT ?2",
            )?;
            let entries = stmt.query_map(params![tp, limit], row_to_entry)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(entries)
        }
        (None, Some(aq)) => {
            let ap = format!("%{aq}%");
            let mut stmt = conn.prepare(
                "SELECT id, text, raw_text, style, language, is_note, app_name, created_at, uuid, device_id
                 FROM history WHERE app_name LIKE ?1
                 ORDER BY created_at DESC, id DESC LIMIT ?2",
            )?;
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
                synced     INTEGER NOT NULL DEFAULT 0
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
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("rawText"));
        assert!(json.contains("createdAt"));
        assert!(json.contains("appName"));
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
}
