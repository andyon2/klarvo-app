//! Reads the v1 `history.db` (SQLite) — tables `history` and `usage`.
//!
//! v1 source: `src-tauri/src/history/mod.rs`. The `tips_shown` table is
//! intentionally not imported (UI state, not user data — see
//! `docs/migration/v1-to-v2.md`).

use std::path::Path;

use rusqlite::Connection;

use super::V1ImportWarning;

const HISTORY_DB: &str = "history.db";

/// A single row from v1 `history` table.
#[derive(Debug, Clone, PartialEq)]
pub struct V1HistoryEntry {
    pub id: i64,
    pub text: String,
    pub raw_text: Option<String>,
    pub style: String,
    pub language: String,
    pub is_note: bool,
    pub app_name: Option<String>,
    pub created_at: String,
    pub uuid: Option<String>,
    pub device_id: Option<String>,
}

/// A single row from v1 `usage` table (cost tracking).
#[derive(Debug, Clone, PartialEq)]
pub struct V1UsageEntry {
    pub id: i64,
    pub service: String,
    pub audio_duration_ms: Option<i64>,
    pub prompt_tokens: Option<i64>,
    pub completion_tokens: Option<i64>,
    pub estimated_cost_usd: f64,
    pub created_at: String,
}

/// Load history + usage tables from `<appdata>/history.db`.
///
/// Returns `(history, usage)` — each is `None` on missing file, unreadable
/// DB, or unreadable table. Malformed individual rows yield
/// `V1ImportWarning::RowSkipped` and are omitted from the returned Vec.
pub fn load(
    appdata: &Path,
    warnings: &mut Vec<V1ImportWarning>,
) -> (Option<Vec<V1HistoryEntry>>, Option<Vec<V1UsageEntry>>) {
    let db_path = appdata.join(HISTORY_DB);
    if !db_path.exists() {
        warnings.push(V1ImportWarning::FileMissing { file: HISTORY_DB });
        return (None, None);
    }

    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            warnings.push(V1ImportWarning::ParseError {
                file: HISTORY_DB,
                detail: e.to_string(),
            });
            return (None, None);
        }
    };

    let history = load_history_rows(&conn, warnings);
    let usage = load_usage_rows(&conn, warnings);
    (history, usage)
}

fn load_history_rows(
    conn: &Connection,
    warnings: &mut Vec<V1ImportWarning>,
) -> Option<Vec<V1HistoryEntry>> {
    // v1 added columns incrementally via ALTER TABLE; older installs may lack
    // uuid/device_id. COALESCE via SELECT of nullable types handles that.
    let mut stmt = match conn.prepare(
        "SELECT id, text, raw_text, style, language, is_note, app_name, created_at, uuid, device_id \
         FROM history",
    ) {
        Ok(s) => s,
        Err(e) => {
            warnings.push(V1ImportWarning::ParseError {
                file: HISTORY_DB,
                detail: format!("history table: {e}"),
            });
            return None;
        }
    };

    let iter = match stmt.query_map([], row_to_history) {
        Ok(i) => i,
        Err(e) => {
            warnings.push(V1ImportWarning::ParseError {
                file: HISTORY_DB,
                detail: format!("history query: {e}"),
            });
            return None;
        }
    };

    let mut rows = Vec::new();
    for (idx, row_result) in iter.enumerate() {
        match row_result {
            Ok(entry) => rows.push(entry),
            Err(e) => warnings.push(V1ImportWarning::RowSkipped {
                table: "history",
                detail: format!("row {idx}: {e}"),
            }),
        }
    }
    Some(rows)
}

fn load_usage_rows(
    conn: &Connection,
    warnings: &mut Vec<V1ImportWarning>,
) -> Option<Vec<V1UsageEntry>> {
    let mut stmt = match conn.prepare(
        "SELECT id, service, audio_duration_ms, prompt_tokens, completion_tokens, estimated_cost_usd, created_at \
         FROM usage",
    ) {
        Ok(s) => s,
        Err(e) => {
            warnings.push(V1ImportWarning::ParseError {
                file: HISTORY_DB,
                detail: format!("usage table: {e}"),
            });
            return None;
        }
    };

    let iter = match stmt.query_map([], row_to_usage) {
        Ok(i) => i,
        Err(e) => {
            warnings.push(V1ImportWarning::ParseError {
                file: HISTORY_DB,
                detail: format!("usage query: {e}"),
            });
            return None;
        }
    };

    let mut rows = Vec::new();
    for (idx, row_result) in iter.enumerate() {
        match row_result {
            Ok(entry) => rows.push(entry),
            Err(e) => warnings.push(V1ImportWarning::RowSkipped {
                table: "usage",
                detail: format!("row {idx}: {e}"),
            }),
        }
    }
    Some(rows)
}

fn row_to_history(row: &rusqlite::Row<'_>) -> rusqlite::Result<V1HistoryEntry> {
    Ok(V1HistoryEntry {
        id: row.get(0)?,
        text: row.get(1)?,
        raw_text: row.get(2)?,
        style: row.get(3)?,
        language: row.get(4)?,
        is_note: row.get::<_, i64>(5)? != 0,
        app_name: row.get(6)?,
        created_at: row.get(7)?,
        uuid: row.get(8)?,
        device_id: row.get(9)?,
    })
}

fn row_to_usage(row: &rusqlite::Row<'_>) -> rusqlite::Result<V1UsageEntry> {
    Ok(V1UsageEntry {
        id: row.get(0)?,
        service: row.get(1)?,
        audio_duration_ms: row.get(2)?,
        prompt_tokens: row.get(3)?,
        completion_tokens: row.get(4)?,
        estimated_cost_usd: row.get(5)?,
        created_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v1_import::test_util::tempdir;

    /// Write a minimal v1-shape history.db schema (matches `src-tauri/src/history/mod.rs`).
    fn init_v1_schema(conn: &Connection) {
        conn.execute_batch(
            "CREATE TABLE history (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                text       TEXT NOT NULL,
                raw_text   TEXT,
                style      TEXT NOT NULL DEFAULT 'polished',
                language   TEXT NOT NULL DEFAULT '',
                is_note    INTEGER NOT NULL DEFAULT 0,
                app_name   TEXT,
                created_at TEXT NOT NULL,
                uuid       TEXT,
                device_id  TEXT,
                synced     INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE usage (
                id                  INTEGER PRIMARY KEY AUTOINCREMENT,
                service             TEXT NOT NULL,
                audio_duration_ms   INTEGER,
                prompt_tokens       INTEGER,
                completion_tokens   INTEGER,
                estimated_cost_usd  REAL NOT NULL DEFAULT 0,
                created_at          TEXT NOT NULL
            );
            CREATE TABLE tips_shown (
                tip_id   TEXT PRIMARY KEY,
                shown_at TEXT NOT NULL
            );",
        )
        .unwrap();
    }

    #[test]
    fn missing_db_yields_file_missing_warning() {
        let tmp = tempdir();
        let mut warnings = Vec::new();
        let (h, u) = load(tmp.path(), &mut warnings);
        assert!(h.is_none());
        assert!(u.is_none());
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            &warnings[0],
            V1ImportWarning::FileMissing { file: "history.db" }
        ));
    }

    #[test]
    fn populated_db_returns_rows() {
        let tmp = tempdir();
        let db_path = tmp.path().join(HISTORY_DB);
        let conn = Connection::open(&db_path).unwrap();
        init_v1_schema(&conn);
        conn.execute(
            "INSERT INTO history (text, raw_text, style, language, is_note, app_name, created_at, uuid)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                "Hello world",
                "hallo welt",
                "polished",
                "de",
                0,
                "Notepad",
                "2026-04-18T10:00:00Z",
                "aaaa-bbbb-1111",
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO usage (service, audio_duration_ms, estimated_cost_usd, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["groq_stt", 3500, 0.000039, "2026-04-18T10:00:01Z"],
        )
        .unwrap();
        drop(conn);

        let mut warnings = Vec::new();
        let (history, usage) = load(tmp.path(), &mut warnings);
        assert!(warnings.is_empty());
        let h = history.unwrap();
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].text, "Hello world");
        assert_eq!(h[0].raw_text.as_deref(), Some("hallo welt"));
        assert_eq!(h[0].style, "polished");
        assert_eq!(h[0].language, "de");
        assert!(!h[0].is_note);
        assert_eq!(h[0].app_name.as_deref(), Some("Notepad"));
        assert_eq!(h[0].uuid.as_deref(), Some("aaaa-bbbb-1111"));
        assert!(h[0].device_id.is_none());

        let u = usage.unwrap();
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].service, "groq_stt");
        assert_eq!(u[0].audio_duration_ms, Some(3500));
        assert!((u[0].estimated_cost_usd - 0.000039).abs() < 1e-9);
    }

    #[test]
    fn corrupt_file_yields_parse_error() {
        let tmp = tempdir();
        let db_path = tmp.path().join(HISTORY_DB);
        // Write garbage that isn't a valid SQLite file.
        std::fs::write(&db_path, b"not a sqlite file, just some bytes").unwrap();

        let mut warnings = Vec::new();
        let (h, u) = load(tmp.path(), &mut warnings);
        assert!(h.is_none());
        assert!(u.is_none());
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, V1ImportWarning::ParseError { file: "history.db", .. }))
        );
    }

    #[test]
    fn missing_usage_table_still_imports_history() {
        let tmp = tempdir();
        let db_path = tmp.path().join(HISTORY_DB);
        let conn = Connection::open(&db_path).unwrap();
        // Only create history, skip usage.
        conn.execute_batch(
            "CREATE TABLE history (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                text       TEXT NOT NULL,
                raw_text   TEXT,
                style      TEXT NOT NULL DEFAULT 'polished',
                language   TEXT NOT NULL DEFAULT '',
                is_note    INTEGER NOT NULL DEFAULT 0,
                app_name   TEXT,
                created_at TEXT NOT NULL,
                uuid       TEXT,
                device_id  TEXT
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO history (text, style, language, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["x", "polished", "", "2026-04-18T10:00:00Z"],
        )
        .unwrap();
        drop(conn);

        let mut warnings = Vec::new();
        let (history, usage) = load(tmp.path(), &mut warnings);
        assert!(history.is_some());
        assert_eq!(history.unwrap().len(), 1);
        assert!(usage.is_none());
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, V1ImportWarning::ParseError { .. }))
        );
    }
}
