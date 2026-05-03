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
        return Err(AppError {
            kind: AppErrorKind::Configuration,
            message: format!(
                "history db at user_version {current} is ahead of binary's max known \
                 migration {max_known} (downgrade?)"
            ),
            user_message: Some(keys::DOWNGRADE_DETECTED.to_string()),
            retryable: false,
        });
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

/// Apply connection-level pragmas: WAL journal mode, normal sync, foreign keys, busy timeout.
/// Run once after `Connection::open` to harden against concurrent reads (Story 9.3 panel)
/// and slow-disk fsync stalls.
fn setup_pragmas(conn: &Connection) -> Result<(), AppError> {
    // WAL allows a concurrent reader without blocking writers — keeps Story 9.3 list()
    // unblocked while session.rs runs an append.
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| hist_err(format!("pragma journal_mode=WAL: {e}")))?;
    // NORMAL sync is the recommended companion to WAL: durable across app crashes,
    // not across full power-loss — acceptable for a history log.
    conn.pragma_update(None, "synchronous", "NORMAL")
        .map_err(|e| hist_err(format!("pragma synchronous=NORMAL: {e}")))?;
    conn.pragma_update(None, "foreign_keys", true)
        .map_err(|e| hist_err(format!("pragma foreign_keys=ON: {e}")))?;
    // 5s busy timeout: covers transient lock contention without surfacing SQLITE_BUSY
    // to the orchestrator's fail-soft path.
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| hist_err(format!("busy_timeout: {e}")))?;
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
    /// Creates parent directories if missing (first-run on clean install).
    pub fn open(path: &std::path::Path, max_entries: u32) -> Result<Self, AppError> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .map_err(|e| hist_err(format!("create parent dir {}: {e}", parent.display())))?;
        }
        let mut conn =
            Connection::open(path).map_err(|e| hist_err(format!("open: {e}")))?;
        setup_pragmas(&conn)?;
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
        // foreign_keys + busy_timeout still meaningful for in-memory; WAL is no-op there.
        setup_pragmas(&conn)?;
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
            .map_err(|e| append_err(format!("append tx: {e}")))?;

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
        .map_err(|e| append_err(format!("insert: {e}")))?;

        let pending_id = tx.last_insert_rowid();

        // Retention: max_entries == 0 means "disabled" (unbounded growth — caller
        // contract; settings clamps to >=1 at boot, but explicit here for safety).
        // Otherwise prune so the table holds at most `max_entries` rows.
        // Bind as i64 to avoid signed/unsigned cast surprises in SQLite arithmetic.
        if max_entries > 0 {
            let cap = max_entries as i64;
            tx.execute(
                "DELETE FROM history WHERE id IN (
                     SELECT id FROM history ORDER BY id ASC
                     LIMIT MAX(0, (SELECT COUNT(*) FROM history) - ?1)
                 )",
                [cap],
            )
            .map_err(|e| append_err(format!("prune: {e}")))?;
        }

        // Capture the row id only after commit succeeds — otherwise on a failed
        // commit the caller would receive a valid-looking id for a row that does
        // not exist (subsequent delete would silently no-op).
        tx.commit()
            .map_err(|e| append_err(format!("commit append: {e}")))?;

        Ok(pending_id)
    }

    async fn list(&self, limit: u32) -> Result<Vec<HistoryEntry>, AppError> {
        let guard = self.conn.lock().await;
        let mut stmt = guard
            .prepare(
                "SELECT id, text, raw_text, style, language, app_name, created_at,
                        uuid, device_id, plugin_id, manifest_version, output_language
                 FROM history ORDER BY id DESC LIMIT ?1",
            )
            .map_err(|e| list_err(format!("prepare list: {e}")))?;

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
            .map_err(|e| list_err(format!("query list: {e}")))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| list_err(format!("row: {e}")))?);
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
            .map_err(|e| list_err(format!("count: {e}")))?;
        Ok(n)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generic history error without a user-facing key — used for setup/migration paths
/// where the user_message is set explicitly (e.g. downgrade detection) or where
/// the failure is fatal-at-boot and surfaces via the boot fail-soft fallback.
fn hist_err(msg: String) -> AppError {
    AppError {
        kind: AppErrorKind::Io,
        message: format!("history: {msg}"),
        user_message: None,
        retryable: false,
    }
}

/// Append-path error with localized user_message (`error.history.append_failed`).
/// Surfaces in session.rs fail-soft warn-log; user-facing toast resolves the key.
fn append_err(msg: String) -> AppError {
    AppError {
        kind: AppErrorKind::Io,
        message: format!("history: {msg}"),
        user_message: Some(keys::APPEND_FAILED.to_string()),
        retryable: false,
    }
}

/// Read-path error with localized user_message (`error.history.list_failed`).
/// Used by `list()` and `count()` so Tauri-Command failures surface a toast key.
fn list_err(msg: String) -> AppError {
    AppError {
        kind: AppErrorKind::Io,
        message: format!("history: {msg}"),
        user_message: Some(keys::LIST_FAILED.to_string()),
        retryable: false,
    }
}

/// Format a UNIX-epoch second count as an ISO-8601 UTC string: `"YYYY-MM-DDTHH:MM:SSZ"`.
pub(crate) fn format_utc_datetime(secs: u64) -> String {
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
///
/// On a misconfigured system clock (set before 1970), `duration_since` errors and
/// we fall back to epoch zero (`1970-01-01T00:00:00Z`). The fallback is logged at
/// `warn` level so the resulting epoch-stamped entries are diagnosable post-hoc.
pub fn wall_clock_iso8601() -> String {
    let secs = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(e) => {
            tracing::warn!(
                error = %e,
                "system clock is before UNIX epoch; history entry timestamped 1970-01-01"
            );
            0
        }
    };
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
            // Match the format-test anchor below (1746266400 epoch seconds).
            created_at: "2025-05-03T10:00:00Z".to_string(),
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
        assert_eq!(entries[0].created_at, "2025-05-03T10:00:00Z");
    }

    #[tokio::test]
    async fn max_entries_pruning() {
        let n = 5u32;
        let store = SqliteHistoryStore::in_memory(n).unwrap();
        for i in 0..=n {
            store.append(&new_entry(&format!("entry {i}"))).await.unwrap();
        }
        assert_eq!(store.count().await.unwrap(), n);

        let entries = store.list(100).await.unwrap();
        let texts: Vec<&str> = entries.iter().map(|e| e.text.as_str()).collect();
        let ids: Vec<i64> = entries.iter().map(|e| e.id).collect();

        // Survivors must be exactly entry 1..=n (oldest pruned, newest retained).
        assert!(!texts.contains(&"entry 0"), "oldest entry must be pruned");
        for i in 1..=n {
            let label = format!("entry {i}");
            assert!(
                texts.contains(&label.as_str()),
                "expected {label:?} in survivors, got {texts:?}"
            );
        }

        // Newest-first ordering by id (strictly descending).
        for w in ids.windows(2) {
            assert!(w[0] > w[1], "ids must be strictly descending: {ids:?}");
        }

        // Atomicity hint: the highest surviving id must be the last-inserted one (n+1)
        // — a regression that pruned the newest entry instead of the oldest would fail here.
        assert_eq!(*ids.first().unwrap(), (n as i64) + 1, "newest must survive");
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

        // Insert real entries first so a buggy DELETE without WHERE-id (or with an
        // inverted predicate) would visibly destroy them.
        let id1 = store.append(&new_entry("survivor 1")).await.unwrap();
        let id2 = store.append(&new_entry("survivor 2")).await.unwrap();

        // Delete a non-existent id — must be Ok and must not touch other rows.
        store.delete(9999).await.unwrap();

        let entries = store.list(10).await.unwrap();
        assert_eq!(entries.len(), 2, "non-target rows must be untouched");
        let ids: Vec<i64> = entries.iter().map(|e| e.id).collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
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
        // Epoch.
        assert_eq!(format_utc_datetime(0), "1970-01-01T00:00:00Z");
        // One second past epoch — covers seconds field.
        assert_eq!(format_utc_datetime(1), "1970-01-01T00:00:01Z");
        // 2025-05-03T10:00:00Z.
        assert_eq!(format_utc_datetime(1746266400), "2025-05-03T10:00:00Z");
        // 2000-01-01T00:00:00Z — Y2K boundary, era division.
        assert_eq!(format_utc_datetime(946684800), "2000-01-01T00:00:00Z");
    }

    /// January / February dates exercise the `mo <= 2 → y + 1` correction branch
    /// in the Gregorian decomposition (months 1/2 are year-1 in the algorithm's
    /// internal representation). Without these the algorithm is silent.
    #[test]
    fn format_utc_datetime_jan_feb_year_correction() {
        // 2025-01-15T00:00:00Z.
        assert_eq!(format_utc_datetime(1736899200), "2025-01-15T00:00:00Z");
        // 2025-02-28T23:59:59Z — last second of Feb in non-leap year.
        assert_eq!(format_utc_datetime(1740787199), "2025-02-28T23:59:59Z");
        // 2025-03-01T00:00:00Z — first second of March, post-correction branch.
        assert_eq!(format_utc_datetime(1740787200), "2025-03-01T00:00:00Z");
    }

    /// Leap-year February 29 — a date that only exists in years divisible by 4
    /// (and the Gregorian century rule). Regressions in `doy` calculation surface here.
    #[test]
    fn format_utc_datetime_leap_day() {
        // 2024-02-29T12:34:56Z.
        assert_eq!(format_utc_datetime(1709210096), "2024-02-29T12:34:56Z");
        // 2000-02-29T00:00:00Z — century leap year (divisible by 400).
        assert_eq!(format_utc_datetime(951782400), "2000-02-29T00:00:00Z");
    }

    /// Year-boundary transitions — ensures days-since-epoch arithmetic carries
    /// correctly across December → January.
    #[test]
    fn format_utc_datetime_year_boundary() {
        // 2024-12-31T23:59:59Z — last second of year.
        assert_eq!(format_utc_datetime(1735689599), "2024-12-31T23:59:59Z");
        // 2025-01-01T00:00:00Z — first second of next year.
        assert_eq!(format_utc_datetime(1735689600), "2025-01-01T00:00:00Z");
    }
}
