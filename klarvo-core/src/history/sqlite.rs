//! SQLite-backed history store.
//!
//! Schema migration uses `PRAGMA user_version` — identical pattern to
//! `settings/migrations.rs`. All DB calls run under a `tokio::sync::Mutex`
//! because `rusqlite::Connection` is `Send` but not `Sync`.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use crate::error::{AppError, AppErrorKind};
use crate::history::{HistoryBackend, HistoryEntry, NewHistoryEntry};

use super::keys;

// ---------------------------------------------------------------------------
// Schema migration
// ---------------------------------------------------------------------------

struct Migration {
    version: u32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: "CREATE TABLE history (
              id               INTEGER PRIMARY KEY AUTOINCREMENT,
              text             TEXT    NOT NULL,
              raw_text         TEXT,
              style            TEXT    NOT NULL DEFAULT 'verbatim',
              language         TEXT    NOT NULL DEFAULT '',
              app_name         TEXT,
              created_at       TEXT    NOT NULL,
              uuid             TEXT,
              device_id        TEXT,
              plugin_id        TEXT,
              manifest_version TEXT,
              output_language  TEXT
          )",
}];

fn apply_migrations(conn: &mut Connection) -> Result<(), AppError> {
    let current: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| hist_err(format!("read user_version: {e}")))?;

    let max_known = MIGRATIONS.iter().map(|m| m.version).max().unwrap_or(0);
    if current > max_known {
        return Err(hist_err(format!(
            "history db at user_version {current} is ahead of binary's max known \
             migration {max_known} (downgrade?)"
        )));
    }

    for m in MIGRATIONS.iter().filter(|m| m.version > current) {
        let tx = conn
            .transaction()
            .map_err(|e| hist_err(format!("begin tx for v{}: {e}", m.version)))?;
        tx.execute_batch(m.sql)
            .map_err(|e| hist_err(format!("execute migration v{}: {e}", m.version)))?;
        tx.pragma_update(None, "user_version", m.version)
            .map_err(|e| hist_err(format!("update user_version to {}: {e}", m.version)))?;
        tx.commit()
            .map_err(|e| hist_err(format!("commit migration v{}: {e}", m.version)))?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// SqliteHistoryStore
// ---------------------------------------------------------------------------

pub struct SqliteHistoryStore {
    conn: Arc<tokio::sync::Mutex<Connection>>,
    max_entries: u32,
}

impl SqliteHistoryStore {
    /// Open (or create) history.db at `path` and apply schema migrations.
    pub fn open(path: &std::path::Path, max_entries: u32) -> Result<Self, AppError> {
        let mut conn =
            Connection::open(path).map_err(|e| hist_err(format!("open: {e}")))?;
        apply_migrations(&mut conn)?;
        Ok(Self {
            conn: Arc::new(tokio::sync::Mutex::new(conn)),
            max_entries,
        })
    }

    /// In-memory SQLite for tests — schema migrations applied identically.
    pub fn in_memory(max_entries: u32) -> Result<Self, AppError> {
        let mut conn = Connection::open_in_memory()
            .map_err(|e| hist_err(format!("in_memory: {e}")))?;
        apply_migrations(&mut conn)?;
        Ok(Self {
            conn: Arc::new(tokio::sync::Mutex::new(conn)),
            max_entries,
        })
    }
}

#[async_trait::async_trait]
impl HistoryBackend for SqliteHistoryStore {
    async fn append(&self, entry: &NewHistoryEntry) -> Result<i64, AppError> {
        let mut guard = self.conn.lock().await;
        let max_entries = self.max_entries;

        let conn = &mut *guard;
        let tx = conn
            .transaction()
            .map_err(|e| hist_err(format!("append tx: {e}")))?;

        tx.execute(
            "INSERT INTO history (text, raw_text, style, language, app_name, created_at,
                                  uuid, device_id, plugin_id, manifest_version, output_language)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                entry.text,
                entry.raw_text,
                entry.style,
                entry.language,
                entry.app_name,
                entry.created_at,
                entry.uuid,
                entry.device_id,
                entry.plugin_id,
                entry.manifest_version,
                entry.output_language,
            ],
        )
        .map_err(|e| hist_err(format!("insert: {e}")))?;

        let new_id = tx.last_insert_rowid();

        // Prune oldest entries if over cap.
        if max_entries > 0 {
            tx.execute(
                "DELETE FROM history WHERE id IN (
                     SELECT id FROM history ORDER BY id ASC
                     LIMIT MAX(0, (SELECT COUNT(*) FROM history) - ?1)
                 )",
                [max_entries],
            )
            .map_err(|e| hist_err(format!("prune: {e}")))?;
        }

        tx.commit().map_err(|e| hist_err(format!("commit append: {e}")))?;

        Ok(new_id)
    }

    async fn list(&self, limit: u32) -> Result<Vec<HistoryEntry>, AppError> {
        let guard = self.conn.lock().await;
        let mut stmt = guard
            .prepare(
                "SELECT id, text, raw_text, style, language, app_name, created_at,
                        uuid, device_id, plugin_id, manifest_version, output_language
                 FROM history ORDER BY id DESC LIMIT ?1",
            )
            .map_err(|e| hist_err(format!("prepare list: {e}")))?;

        let rows = stmt
            .query_map([limit], |row| {
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    raw_text: row.get(2)?,
                    style: row.get(3)?,
                    language: row.get(4)?,
                    app_name: row.get(5)?,
                    created_at: row.get(6)?,
                    uuid: row.get(7)?,
                    device_id: row.get(8)?,
                    plugin_id: row.get(9)?,
                    manifest_version: row.get(10)?,
                    output_language: row.get(11)?,
                })
            })
            .map_err(|e| hist_err(format!("query list: {e}")))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| hist_err(format!("row: {e}")))?);
        }
        Ok(entries)
    }

    async fn delete(&self, id: i64) -> Result<(), AppError> {
        let guard = self.conn.lock().await;
        guard
            .execute("DELETE FROM history WHERE id = ?1", [id])
            .map_err(|e| AppError {
                kind: AppErrorKind::Io,
                message: format!("history delete: {e}"),
                user_message: Some(keys::DELETE_FAILED.to_string()),
                retryable: false,
            })?;
        // Idempotent: 0 rows affected is OK.
        Ok(())
    }

    async fn clear(&self) -> Result<(), AppError> {
        let guard = self.conn.lock().await;
        guard
            .execute("DELETE FROM history", [])
            .map_err(|e| AppError {
                kind: AppErrorKind::Io,
                message: format!("history clear: {e}"),
                user_message: Some(keys::CLEAR_FAILED.to_string()),
                retryable: false,
            })?;
        Ok(())
    }

    async fn count(&self) -> Result<u32, AppError> {
        let guard = self.conn.lock().await;
        let n: u32 = guard
            .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
            .map_err(|e| hist_err(format!("count: {e}")))?;
        Ok(n)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn hist_err(msg: String) -> AppError {
    AppError {
        kind: AppErrorKind::Io,
        message: format!("history: {msg}"),
        user_message: None,
        retryable: false,
    }
}

/// Format a UNIX-epoch second count as an ISO-8601 UTC string: `"YYYY-MM-DDTHH:MM:SSZ"`.
pub fn format_utc_datetime(secs: u64) -> String {
    // Manual decomposition — avoids a `chrono` or `time` crate dependency.
    // Gregorian calendar math following the algorithm from "days since epoch".
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400; // days since 1970-01-01

    // Gregorian calendar decomposition.
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, d, h, m, s)
}

/// Current wall-clock time as an ISO-8601 UTC string.
pub fn wall_clock_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_utc_datetime(secs)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    fn new_entry(text: &str) -> NewHistoryEntry {
        NewHistoryEntry {
            text: text.to_string(),
            raw_text: None,
            style: "verbatim".to_string(),
            language: "en".to_string(),
            app_name: None,
            created_at: "2026-05-03T10:00:00Z".to_string(),
            uuid: None,
            device_id: None,
            plugin_id: None,
            manifest_version: None,
            output_language: None,
        }
    }

    #[tokio::test]
    async fn in_memory_applies_schema() {
        let store = SqliteHistoryStore::in_memory(10).unwrap();
        // All operations must run without schema error.
        assert_eq!(store.count().await.unwrap(), 0);
        let id = store.append(&new_entry("test")).await.unwrap();
        assert!(id > 0);
        let entries = store.list(10).await.unwrap();
        assert_eq!(entries.len(), 1);
        store.delete(id).await.unwrap();
        assert_eq!(store.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn append_stores_entry_and_list_returns_it() {
        let store = SqliteHistoryStore::in_memory(100).unwrap();
        store.append(&new_entry("hello world")).await.unwrap();
        let entries = store.list(10).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "hello world");
        assert_eq!(entries[0].style, "verbatim");
        assert_eq!(entries[0].created_at, "2026-05-03T10:00:00Z");
    }

    #[tokio::test]
    async fn max_entries_pruning() {
        let n = 5u32;
        let store = SqliteHistoryStore::in_memory(n).unwrap();
        for i in 0..=n {
            store.append(&new_entry(&format!("entry {i}"))).await.unwrap();
        }
        assert_eq!(store.count().await.unwrap(), n);
        // Oldest entry (id=1) must have been pruned.
        let entries = store.list(100).await.unwrap();
        // Entries are ordered newest-first; the last in the list is the oldest survivor.
        let texts: Vec<_> = entries.iter().map(|e| e.text.as_str()).collect();
        assert!(!texts.contains(&"entry 0"), "oldest entry must be pruned");
    }

    #[tokio::test]
    async fn delete_removes_specific_entry() {
        let store = SqliteHistoryStore::in_memory(100).unwrap();
        let id1 = store.append(&new_entry("first")).await.unwrap();
        let id2 = store.append(&new_entry("second")).await.unwrap();
        let id3 = store.append(&new_entry("third")).await.unwrap();

        store.delete(id2).await.unwrap();

        let entries = store.list(10).await.unwrap();
        assert_eq!(entries.len(), 2);
        let ids: Vec<i64> = entries.iter().map(|e| e.id).collect();
        assert!(!ids.contains(&id2));
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id3));
    }

    #[tokio::test]
    async fn delete_on_missing_id_is_noop() {
        let store = SqliteHistoryStore::in_memory(100).unwrap();
        // Should return Ok even if id doesn't exist.
        store.delete(9999).await.unwrap();
    }

    #[tokio::test]
    async fn clear_removes_all_entries() {
        let store = SqliteHistoryStore::in_memory(100).unwrap();
        store.append(&new_entry("a")).await.unwrap();
        store.append(&new_entry("b")).await.unwrap();
        store.append(&new_entry("c")).await.unwrap();
        store.clear().await.unwrap();
        assert_eq!(store.count().await.unwrap(), 0);
    }

    #[test]
    fn format_utc_datetime_known_value() {
        // 2025-05-03T10:00:00Z = 1746266400 seconds since epoch.
        assert_eq!(format_utc_datetime(1746266400), "2025-05-03T10:00:00Z");
        // Epoch itself
        assert_eq!(format_utc_datetime(0), "1970-01-01T00:00:00Z");
    }
}
