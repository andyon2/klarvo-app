//! Schema migration engine for the `settings` SQLite database.
//!
//! Uses `PRAGMA user_version` to track applied migrations — the same mechanism
//! used by `rusqlite_migration` and similar libraries. Migrations are applied
//! in order; each is idempotent when re-run (handled by the version guard).
//!
//! Schema-ownership principle (AC-1): no `CREATE TABLE IF NOT EXISTS` appears
//! in application code — the migration list here is the sole schema authority.

use rusqlite::Connection;

use crate::error::{AppError, AppErrorKind};

struct Migration {
    version: u32,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: "CREATE TABLE settings (
              key   TEXT PRIMARY KEY NOT NULL,
              value TEXT NOT NULL,
              type  TEXT NOT NULL
          )",
}];

/// Apply all pending schema migrations to `conn`. Idempotent on re-run.
///
/// Refuses to operate on a DB whose `user_version` is ahead of the highest
/// known migration in this binary — that means a newer Klarvo version wrote
/// the file and the user downgraded. Continuing would surface as cryptic
/// query failures later; we fail boot-time with a clear error instead.
pub(super) fn apply(conn: &mut Connection) -> Result<(), AppError> {
    let current: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| migration_err(format!("read user_version: {e}")))?;

    let max_known = MIGRATIONS.iter().map(|m| m.version).max().unwrap_or(0);
    if current > max_known {
        return Err(migration_err(format!(
            "settings db at user_version {current} is ahead of binary's max known migration {max_known} (downgrade?)"
        )));
    }

    for m in MIGRATIONS.iter().filter(|m| m.version > current) {
        let tx = conn
            .transaction()
            .map_err(|e| migration_err(format!("begin tx for v{}: {e}", m.version)))?;

        tx.execute_batch(m.sql)
            .map_err(|e| migration_err(format!("execute migration v{}: {e}", m.version)))?;

        tx.pragma_update(None, "user_version", m.version)
            .map_err(|e| migration_err(format!("update user_version to {}: {e}", m.version)))?;

        tx.commit()
            .map_err(|e| migration_err(format!("commit migration v{}: {e}", m.version)))?;
    }

    Ok(())
}

fn migration_err(msg: String) -> AppError {
    AppError {
        kind: AppErrorKind::Io,
        message: format!("settings schema migration: {msg}"),
        user_message: None,
        retryable: false,
    }
}
