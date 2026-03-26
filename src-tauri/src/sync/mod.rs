//! Cross-device history sync via Turso HTTP API.
//!
//! Uses Turso's HTTP pipeline endpoint to push local entries and pull remote
//! entries from other devices. Sync is append-only — history entries are never
//! modified after creation, only new entries are exchanged.
//!
//! Architecture: DB reads and writes happen synchronously (non-async) so that
//! `rusqlite::Connection` (which is not `Send`) is never held across an await.
//! The async HTTP calls operate on owned data extracted first.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Turso API error: {0}")]
    TursoApi(String),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

// ---------------------------------------------------------------------------
// Sync entry (owned, Send-safe)
// ---------------------------------------------------------------------------

/// An owned history entry for sync transport. `Send + Sync` safe.
pub struct SyncEntry {
    pub uuid: String,
    pub text: String,
    pub raw_text: Option<String>,
    pub style: String,
    pub language: String,
    pub is_note: i32,
    pub app_name: Option<String>,
    pub device_id: Option<String>,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Turso HTTP API types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct PipelineRequest {
    requests: Vec<PipelineItem>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum PipelineItem {
    #[serde(rename = "execute")]
    Execute { stmt: Statement },
    #[serde(rename = "close")]
    Close,
}

#[derive(Serialize)]
struct Statement {
    sql: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    args: Vec<TursoValue>,
}

#[derive(Serialize, Clone)]
#[serde(tag = "type", content = "value")]
enum TursoValue {
    #[serde(rename = "text")]
    Text(String),
    #[serde(rename = "integer")]
    Integer(String),
    #[serde(rename = "null")]
    Null,
}

#[derive(Deserialize, Debug)]
struct PipelineResponse {
    results: Vec<PipelineResult>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum PipelineResult {
    #[serde(rename = "ok")]
    Ok { response: ResponseBody },
    #[serde(rename = "error")]
    Error { error: ApiError },
}

#[derive(Deserialize, Debug)]
struct ApiError {
    message: String,
}

#[derive(Deserialize, Debug)]
struct ResponseBody {
    #[serde(rename = "type")]
    _type: String,
    #[serde(default)]
    result: Option<QueryResult>,
}

#[derive(Deserialize, Debug)]
struct QueryResult {
    #[serde(default)]
    #[allow(dead_code)]
    cols: Vec<ColInfo>,
    #[serde(default)]
    rows: Vec<Vec<ResultValue>>,
    #[serde(default)]
    #[allow(dead_code)]
    affected_row_count: u64,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct ColInfo {
    name: String,
    #[serde(default)]
    decltype: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", content = "value")]
enum ResultValue {
    #[serde(rename = "text")]
    Text(String),
    #[serde(rename = "integer")]
    Integer(String),
    #[serde(rename = "null")]
    Null,
}

impl ResultValue {
    fn as_str(&self) -> Option<&str> {
        match self {
            ResultValue::Text(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            ResultValue::Integer(s) => s.parse().ok(),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// URL conversion
// ---------------------------------------------------------------------------

fn turso_http_url(libsql_url: &str) -> Result<String, SyncError> {
    if libsql_url.starts_with("libsql://") {
        Ok(libsql_url.replace("libsql://", "https://"))
    } else if libsql_url.starts_with("https://") {
        Ok(libsql_url.to_string())
    } else {
        Err(SyncError::InvalidUrl(format!(
            "Expected libsql:// or https:// URL, got: {libsql_url}"
        )))
    }
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

async fn execute_pipeline(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    requests: Vec<PipelineItem>,
) -> Result<Vec<PipelineResult>, SyncError> {
    let url = format!("{base_url}/v2/pipeline");
    let body = PipelineRequest { requests };

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(SyncError::TursoApi(format!("HTTP {status}: {text}")));
    }

    let text = resp.text().await?;
    let pipeline_resp: PipelineResponse = serde_json::from_str(&text)
        .map_err(|e| SyncError::TursoApi(format!("Failed to parse response: {e}\nBody: {text}")))?;
    Ok(pipeline_resp.results)
}

async fn execute_sql(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    sql: &str,
    args: Vec<TursoValue>,
) -> Result<QueryResult, SyncError> {
    let results = execute_pipeline(
        client,
        base_url,
        token,
        vec![
            PipelineItem::Execute {
                stmt: Statement {
                    sql: sql.to_string(),
                    args,
                },
            },
            PipelineItem::Close,
        ],
    )
    .await?;

    match results.into_iter().next() {
        Some(PipelineResult::Ok { response }) => response
            .result
            .ok_or_else(|| SyncError::TursoApi("Response has no result field".to_string())),
        Some(PipelineResult::Error { error }) => Err(SyncError::TursoApi(error.message)),
        None => Err(SyncError::TursoApi("Empty response".to_string())),
    }
}

// ---------------------------------------------------------------------------
// Public API -- synchronous DB operations
// ---------------------------------------------------------------------------

/// Reads unsynced entries from the local DB. Call this with the DB lock held,
/// then release the lock before doing any async work.
pub fn read_unsynced_entries(conn: &Connection) -> Result<Vec<SyncEntry>, SyncError> {
    let mut stmt = conn.prepare(
        "SELECT uuid, text, raw_text, style, language, is_note, app_name, device_id, created_at
         FROM history WHERE synced = 0 AND uuid IS NOT NULL",
    )?;
    let entries = stmt
        .query_map([], |row| {
            Ok(SyncEntry {
                uuid: row.get(0)?,
                text: row.get(1)?,
                raw_text: row.get(2)?,
                style: row.get(3)?,
                language: row.get(4)?,
                is_note: row.get(5)?,
                app_name: row.get(6)?,
                device_id: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(entries)
}

/// Marks entries as synced by UUID. Call with the DB lock held.
pub fn mark_entries_synced(conn: &Connection, uuids: &[String]) -> Result<(), SyncError> {
    for uuid in uuids {
        conn.execute("UPDATE history SET synced = 1 WHERE uuid = ?1", params![uuid])?;
    }
    Ok(())
}

/// Inserts remote entries into the local DB, skipping any that already exist.
/// Call with the DB lock held.
pub fn insert_pulled_entries(conn: &Connection, entries: &[SyncEntry]) -> Result<u32, SyncError> {
    let mut inserted = 0u32;
    for e in entries {
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) FROM history WHERE uuid = ?1",
            params![e.uuid],
            |r| r.get::<_, i64>(0),
        )? > 0;

        if exists {
            continue;
        }

        conn.execute(
            "INSERT OR IGNORE INTO history (text, raw_text, style, language, is_note, app_name, uuid, device_id, created_at, synced)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1)",
            params![
                e.text,
                e.raw_text,
                e.style,
                e.language,
                e.is_note,
                e.app_name,
                e.uuid,
                e.device_id,
                e.created_at,
            ],
        )?;
        inserted += 1;
    }
    Ok(inserted)
}

// ---------------------------------------------------------------------------
// Public API -- async HTTP operations
// ---------------------------------------------------------------------------

/// Ensures the remote table exists and pushes entries to Turso.
/// Returns `(pushed_count, uuids_to_mark)`.
pub async fn ensure_and_push(
    url: &str,
    token: &str,
    entries: Vec<SyncEntry>,
) -> Result<(u32, Vec<String>), SyncError> {
    let base_url = turso_http_url(url)?;
    let client = reqwest::Client::new();

    // Ensure remote table exists.
    execute_sql(
        &client,
        &base_url,
        token,
        "CREATE TABLE IF NOT EXISTS history (
            uuid       TEXT PRIMARY KEY,
            text       TEXT NOT NULL,
            raw_text   TEXT,
            style      TEXT NOT NULL DEFAULT 'polished',
            language   TEXT NOT NULL DEFAULT '',
            is_note    INTEGER NOT NULL DEFAULT 0,
            app_name   TEXT,
            device_id  TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        vec![],
    )
    .await?;

    if entries.is_empty() {
        return Ok((0, vec![]));
    }

    let count = entries.len() as u32;
    let uuids: Vec<String> = entries.iter().map(|e| e.uuid.clone()).collect();

    // Build a pipeline with all INSERT statements.
    let mut requests: Vec<PipelineItem> = entries
        .iter()
        .map(|e| PipelineItem::Execute {
            stmt: Statement {
                sql: "INSERT OR IGNORE INTO history (uuid, text, raw_text, style, language, is_note, app_name, device_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)".to_string(),
                args: vec![
                    TursoValue::Text(e.uuid.clone()),
                    TursoValue::Text(e.text.clone()),
                    match &e.raw_text {
                        Some(s) => TursoValue::Text(s.clone()),
                        None => TursoValue::Null,
                    },
                    TursoValue::Text(e.style.clone()),
                    TursoValue::Text(e.language.clone()),
                    TursoValue::Integer(e.is_note.to_string()),
                    match &e.app_name {
                        Some(s) => TursoValue::Text(s.clone()),
                        None => TursoValue::Null,
                    },
                    match &e.device_id {
                        Some(s) => TursoValue::Text(s.clone()),
                        None => TursoValue::Null,
                    },
                    TursoValue::Text(e.created_at.clone()),
                ],
            },
        })
        .collect();
    requests.push(PipelineItem::Close);

    let results = execute_pipeline(&client, &base_url, token, requests).await?;

    for result in &results {
        if let PipelineResult::Error { error } = result {
            log::warn!("[sync] Push error: {}", error.message);
        }
    }

    log::info!("[sync] Pushed {count} entries to Turso");
    Ok((count, uuids))
}

/// Pushes a single entry to Turso. Unlike `ensure_and_push`, this does NOT
/// run `CREATE TABLE IF NOT EXISTS` -- it assumes the remote table already
/// exists from a previous full sync. Intended for fire-and-forget auto-sync
/// after each dictation.
///
/// Returns the UUID of the pushed entry on success so the caller can mark it
/// as synced in the local DB.
pub async fn push_single_entry(
    url: &str,
    token: &str,
    entry: SyncEntry,
) -> Result<String, SyncError> {
    let base_url = turso_http_url(url)?;
    let client = reqwest::Client::new();

    let uuid = entry.uuid.clone();

    execute_sql(
        &client,
        &base_url,
        token,
        "INSERT OR IGNORE INTO history (uuid, text, raw_text, style, language, is_note, app_name, device_id, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        vec![
            TursoValue::Text(entry.uuid),
            TursoValue::Text(entry.text),
            match entry.raw_text {
                Some(s) => TursoValue::Text(s),
                None => TursoValue::Null,
            },
            TursoValue::Text(entry.style),
            TursoValue::Text(entry.language),
            TursoValue::Integer(entry.is_note.to_string()),
            match entry.app_name {
                Some(s) => TursoValue::Text(s),
                None => TursoValue::Null,
            },
            match entry.device_id {
                Some(s) => TursoValue::Text(s),
                None => TursoValue::Null,
            },
            TursoValue::Text(entry.created_at),
        ],
    )
    .await?;

    log::info!("[sync] Auto-pushed entry {uuid} to Turso");
    Ok(uuid)
}

/// Pulls history entries from Turso that belong to other devices.
pub async fn pull_remote_entries(
    url: &str,
    token: &str,
    device_id: &str,
) -> Result<Vec<SyncEntry>, SyncError> {
    let base_url = turso_http_url(url)?;
    let client = reqwest::Client::new();

    let result = execute_sql(
        &client,
        &base_url,
        token,
        "SELECT uuid, text, raw_text, style, language, is_note, app_name, device_id, created_at
         FROM history WHERE device_id != ? OR device_id IS NULL",
        vec![TursoValue::Text(device_id.to_string())],
    )
    .await?;

    let entries: Vec<SyncEntry> = result
        .rows
        .iter()
        .filter(|row| row.len() >= 9)
        .filter_map(|row| {
            Some(SyncEntry {
                uuid: row[0].as_str()?.to_string(),
                text: row[1].as_str().unwrap_or("").to_string(),
                raw_text: row[2].as_str().map(|s| s.to_string()),
                style: row[3].as_str().unwrap_or("polished").to_string(),
                language: row[4].as_str().unwrap_or("").to_string(),
                is_note: row[5].as_i64().unwrap_or(0) as i32,
                app_name: row[6].as_str().map(|s| s.to_string()),
                device_id: row[7].as_str().map(|s| s.to_string()),
                created_at: row[8].as_str().unwrap_or("").to_string(),
            })
        })
        .collect();

    Ok(entries)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turso_http_url_libsql() {
        let url = turso_http_url("libsql://my-db.turso.io").unwrap();
        assert_eq!(url, "https://my-db.turso.io");
    }

    #[test]
    fn test_turso_http_url_https() {
        let url = turso_http_url("https://my-db.turso.io").unwrap();
        assert_eq!(url, "https://my-db.turso.io");
    }

    #[test]
    fn test_turso_http_url_invalid() {
        assert!(turso_http_url("http://my-db.turso.io").is_err());
    }

    #[test]
    fn test_turso_value_serialization() {
        let text = serde_json::to_string(&TursoValue::Text("hello".to_string())).unwrap();
        assert!(text.contains("\"type\":\"text\""));
        assert!(text.contains("\"value\":\"hello\""));

        let null = serde_json::to_string(&TursoValue::Null).unwrap();
        assert!(null.contains("\"type\":\"null\""));

        let int = serde_json::to_string(&TursoValue::Integer("42".to_string())).unwrap();
        assert!(int.contains("\"type\":\"integer\""));
        assert!(int.contains("\"value\":\"42\""));
    }

    #[test]
    fn test_read_unsynced_empty_db() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                raw_text TEXT,
                style TEXT NOT NULL DEFAULT 'polished',
                language TEXT NOT NULL DEFAULT '',
                is_note INTEGER NOT NULL DEFAULT 0,
                app_name TEXT,
                uuid TEXT,
                device_id TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                synced INTEGER NOT NULL DEFAULT 0
            )",
        )
        .unwrap();

        let entries = read_unsynced_entries(&conn).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_insert_and_read_unsynced() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                raw_text TEXT,
                style TEXT NOT NULL DEFAULT 'polished',
                language TEXT NOT NULL DEFAULT '',
                is_note INTEGER NOT NULL DEFAULT 0,
                app_name TEXT,
                uuid TEXT UNIQUE,
                device_id TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                synced INTEGER NOT NULL DEFAULT 0
            )",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO history (text, uuid, device_id) VALUES ('Hello', 'uuid-1', 'dev-1')",
            [],
        )
        .unwrap();

        let entries = read_unsynced_entries(&conn).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uuid, "uuid-1");
        assert_eq!(entries[0].text, "Hello");

        // Mark as synced
        mark_entries_synced(&conn, &["uuid-1".to_string()]).unwrap();

        let entries = read_unsynced_entries(&conn).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_insert_pulled_entries() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                raw_text TEXT,
                style TEXT NOT NULL DEFAULT 'polished',
                language TEXT NOT NULL DEFAULT '',
                is_note INTEGER NOT NULL DEFAULT 0,
                app_name TEXT,
                uuid TEXT UNIQUE,
                device_id TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                synced INTEGER NOT NULL DEFAULT 0
            )",
        )
        .unwrap();

        let remote = vec![
            SyncEntry {
                uuid: "remote-1".to_string(),
                text: "From Android".to_string(),
                raw_text: None,
                style: "polished".to_string(),
                language: "de".to_string(),
                is_note: 0,
                app_name: Some("WhatsApp".to_string()),
                device_id: Some("android-phone".to_string()),
                created_at: "2026-03-08T12:00:00".to_string(),
            },
        ];

        let pulled = insert_pulled_entries(&conn, &remote).unwrap();
        assert_eq!(pulled, 1);

        // Inserting again should skip (duplicate UUID)
        let pulled2 = insert_pulled_entries(&conn, &remote).unwrap();
        assert_eq!(pulled2, 0);
    }
}
