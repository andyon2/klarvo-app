//! Settings-Service: typed SQLite-backed user-setting accessor layer.
//!
//! Architecture:
//! - `SettingsEmitter` trait (core-portable, no Tauri import) per ADR-0009 Hybrid-C analog.
//! - `Settings::open()` / `Settings::in_memory()` initialise the DB and apply schema migrations
//!   via `rusqlite_migration`. Migration is idempotent — safe to call on every boot.
//! - One-shot TOML→SQLite migration: `migrate_from_toml_if_needed()` writes 5 Phase-1 fields
//!   into SQLite on first boot when the settings table is empty (AC-2).
//! - Typed Core-Accessors (AC-4): `ui_language()`, `set_ui_language()`, etc.
//! - Plugin-Setting API with Core-Namespace-Guard (AC-7).

pub mod defaults;
mod migrations;

use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::error::{AppError, AppErrorKind};
use crate::recording::RecordingMode;
use defaults::*;

/// Core namespace key-prefixes that plugin-setting writes/reads must not access.
const CORE_PREFIXES: &[&str] = &["app.", "hotkey.", "ui.", "audio.", "license.", "history."];

/// Maximum byte-length of any individual setting value. Guards against IPC-DOS
/// (oversized strings holding the SQLite mutex during write).
const MAX_VALUE_LEN: usize = 4096;

/// Maximum byte-length of a plugin_id. Combined with `validate_plugin_id`'s
/// charset restriction this caps `plugins.<id>.<key>` total length.
const MAX_PLUGIN_ID_LEN: usize = 64;

/// AC-2 detect-condition: only the 5 core-namespace migration keys decide
/// whether the one-shot TOML→SQLite migration has already run. Plugin-rows
/// must not block the core-migration.
const MIGRATION_SENTINEL_KEYS: &[&str] = &[
    "hotkey.slot1.combo",
    "app.output_target_id",
    "ui.language",
    "app.dictionary_language",
    "app.output_language",
];

// ---------------------------------------------------------------------------
// SettingsEmitter trait (AC-5)
// ---------------------------------------------------------------------------

/// Shell-implemented event emitter. Lives in `klarvo-core` without a Tauri import.
///
/// ADR-0009 Hybrid-C analog: Core holds `Arc<dyn SettingsEmitter>`;
/// `TauriSettingsEmitter` in the Windows shell calls `app.emit()`.
pub trait SettingsEmitter: Send + Sync {
    fn emit_settings_changed(&self, key: &str, new_value: &str);
}

/// No-op emitter for tests and contexts without a Tauri runtime.
pub struct NoopSettingsEmitter;

impl SettingsEmitter for NoopSettingsEmitter {
    fn emit_settings_changed(&self, _key: &str, _new_value: &str) {}
}

// ---------------------------------------------------------------------------
// TomlMigrationSource (AC-2)
// ---------------------------------------------------------------------------

/// Phase-1 user-settings extracted from `config.toml` by the Shell and passed
/// to `Settings::migrate_from_toml_if_needed()` (AC-2 Ast A).
///
/// All fields use Phase-1 defaults when the TOML field is missing or malformed
/// (fail-soft per `feedback_scaffold_fail_soft_pattern`). The Shell is responsible
/// for the soft-parse — this struct always contains valid string values.
pub struct TomlMigrationSource {
    pub hotkey_slot1_combo: String,
    pub output_target_id: String,
    pub ui_language: String,
    pub dictionary_language: String,
    pub output_language: String,
}

// ---------------------------------------------------------------------------
// Settings struct
// ---------------------------------------------------------------------------

pub struct Settings {
    conn: Mutex<Connection>,
    emitter: Arc<dyn SettingsEmitter>,
}

impl Settings {
    /// Open (or create) a SQLite file at `path` and apply schema migrations.
    pub fn open(path: impl AsRef<Path>, emitter: Arc<dyn SettingsEmitter>) -> Result<Self, AppError> {
        let mut conn = Connection::open(path.as_ref()).map_err(|e| db_err(format!("open: {e}")))?;
        migrations::apply(&mut conn)?;
        Ok(Self { conn: Mutex::new(conn), emitter })
    }

    /// In-memory SQLite — for tests; schema migrations applied identically.
    pub fn in_memory(emitter: Arc<dyn SettingsEmitter>) -> Result<Self, AppError> {
        let mut conn =
            Connection::open_in_memory().map_err(|e| db_err(format!("in_memory: {e}")))?;
        migrations::apply(&mut conn)?;
        Ok(Self { conn: Mutex::new(conn), emitter })
    }

    // -----------------------------------------------------------------------
    // One-shot TOML→SQLite migration (AC-2 / AC-3)
    // -----------------------------------------------------------------------

    /// Write Phase-1 TOML settings into SQLite on the very first boot.
    ///
    /// Detect-condition: `settings` table is empty (AC-3: non-empty → skip).
    ///
    /// - `Some(src)` → AC-2 Ast A: migrate 5 fields inside an EXCLUSIVE transaction.
    ///   On any write error the transaction rolls back fully (no partial state).
    /// - `None` → AC-2 Ast B: Fresh-Install; leave table empty; return Ok(()).
    pub fn migrate_from_toml_if_needed(
        &self,
        source: Option<&TomlMigrationSource>,
    ) -> Result<(), AppError> {
        // Detect-condition (AC-3): only the 5 core sentinel keys count;
        // plugin-rows that may have been written first must not block this.
        // Restrict to type='string' so a future writer using a different type-tag
        // for one of these keys does not block migration AND leave a row that
        // typed accessors cannot read.
        let placeholders = vec!["?"; MIGRATION_SENTINEL_KEYS.len()].join(",");
        let count_sql = format!(
            "SELECT COUNT(*) FROM settings WHERE type = 'string' AND key IN ({placeholders})"
        );

        let mut guard = lock_conn(&self.conn);

        let count: i64 = guard
            .query_row(
                &count_sql,
                rusqlite::params_from_iter(MIGRATION_SENTINEL_KEYS.iter()),
                |row| row.get(0),
            )
            .map_err(|e| db_err(format!("count: {e}")))?;

        if count > 0 {
            return Ok(()); // AC-3: already migrated
        }

        let Some(src) = source else {
            return Ok(()); // AC-2 Ast B: fresh install
        };

        // AC-2 Ast A: exclusive transaction for atomicity
        let tx = guard
            .transaction_with_behavior(rusqlite::TransactionBehavior::Exclusive)
            .map_err(|e| db_err(format!("begin exclusive: {e}")))?;

        let writes = [
            ("hotkey.slot1.combo", src.hotkey_slot1_combo.as_str()),
            ("app.output_target_id", src.output_target_id.as_str()),
            ("ui.language", src.ui_language.as_str()),
            ("app.dictionary_language", src.dictionary_language.as_str()),
            ("app.output_language", src.output_language.as_str()),
        ];

        for (key, value) in writes {
            // Defensive validation: even though the Shell soft-parses the TOML
            // before constructing TomlMigrationSource, a buggy soft-parser could
            // hand us empty/control-char/oversized values. Re-validate so that
            // the migration cannot persist values the typed setters would reject
            // (read/write asymmetry).
            validate_setting_value(key, value)?;
            tx.execute(
                "INSERT OR REPLACE INTO settings (key, value, type) VALUES (?1, ?2, 'string')",
                rusqlite::params![key, value],
            )
            .map_err(|e| db_err(format!("write {key}: {e}")))?;
        }

        tx.commit().map_err(|e| db_err(format!("commit: {e}")))?;

        // Drop the lock before invoking the emitter so listeners can re-enter
        // the Settings API (e.g. read accessors) without deadlocking.
        drop(guard);

        // Each emit is wrapped in catch_unwind so a panicking emitter does not
        // skip the remaining 4 emits (and does not propagate up the
        // command boundary as a transaction-failure illusion — the DB is
        // already committed).
        for (key, value) in writes {
            emit_or_warn(&*self.emitter, key, value);
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal raw get / set
    // -----------------------------------------------------------------------

    fn get_raw(&self, key: &str) -> Result<Option<String>, AppError> {
        let guard = lock_conn(&self.conn);
        let result = guard.query_row(
            "SELECT value, type FROM settings WHERE key = ?1",
            [key],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        );
        match result {
            Ok((value, type_str)) => match type_str.as_str() {
                // Treat empty stored value as "missing" so typed accessors
                // fall back to defaults instead of returning Some("") — guards
                // against a write path that bypassed validate_setting_value
                // and persisted an empty NOT-NULL row.
                "string" | "i64" | "bool" | "json" if value.is_empty() => Ok(None),
                "string" | "i64" | "bool" | "json" => Ok(Some(value)),
                unknown => Err(AppError {
                    kind: AppErrorKind::Internal,
                    message: format!("unknown settings type '{unknown}' for key '{key}'"),
                    user_message: None,
                    retryable: false,
                }),
            },
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(db_err(format!("get '{key}': {e}"))),
        }
    }

    /// Persist `value` under `key`, then call `emitter.emit_settings_changed`.
    /// On persist error: return Err without emitting (AC-5). On a panicking
    /// emitter: log a warning but return Ok — the DB write is already committed
    /// and surfacing the panic as a command-error would prompt the user to
    /// retry, double-writing.
    fn set_raw(&self, key: &str, value: &str, type_str: &str) -> Result<(), AppError> {
        {
            let guard = lock_conn(&self.conn);
            guard
                .execute(
                    "INSERT OR REPLACE INTO settings (key, value, type) VALUES (?1, ?2, ?3)",
                    rusqlite::params![key, value, type_str],
                )
                .map_err(|e| db_err(format!("set '{key}': {e}")))?;
        }
        emit_or_warn(&*self.emitter, key, value);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Typed Core-Accessors (AC-4)
    // -----------------------------------------------------------------------

    pub fn ui_language(&self) -> Result<String, AppError> {
        Ok(self.get_raw("ui.language")?.unwrap_or_else(|| DEFAULT_UI_LANGUAGE.to_string()))
    }

    pub fn set_ui_language(&self, val: &str) -> Result<(), AppError> {
        validate_setting_value("ui.language", val)?;
        self.set_raw("ui.language", val, "string")
    }

    pub fn output_language(&self) -> Result<String, AppError> {
        Ok(self
            .get_raw("app.output_language")?
            .unwrap_or_else(|| DEFAULT_OUTPUT_LANGUAGE.to_string()))
    }

    pub fn set_output_language(&self, val: &str) -> Result<(), AppError> {
        validate_setting_value("app.output_language", val)?;
        self.set_raw("app.output_language", val, "string")
    }

    pub fn dictionary_language(&self) -> Result<String, AppError> {
        Ok(self
            .get_raw("app.dictionary_language")?
            .unwrap_or_else(|| DEFAULT_DICTIONARY_LANGUAGE.to_string()))
    }

    pub fn set_dictionary_language(&self, val: &str) -> Result<(), AppError> {
        validate_setting_value("app.dictionary_language", val)?;
        self.set_raw("app.dictionary_language", val, "string")
    }

    pub fn hotkey_slot1_combo(&self) -> Result<String, AppError> {
        Ok(self
            .get_raw("hotkey.slot1.combo")?
            .unwrap_or_else(|| DEFAULT_HOTKEY_SLOT1_COMBO.to_string()))
    }

    pub fn set_hotkey_slot1_combo(&self, val: &str) -> Result<(), AppError> {
        validate_setting_value("hotkey.slot1.combo", val)?;
        self.set_raw("hotkey.slot1.combo", val, "string")
    }

    pub fn output_target_id(&self) -> Result<String, AppError> {
        Ok(self
            .get_raw("app.output_target_id")?
            .unwrap_or_else(|| DEFAULT_OUTPUT_TARGET_ID.to_string()))
    }

    pub fn set_output_target_id(&self, val: &str) -> Result<(), AppError> {
        validate_setting_value("app.output_target_id", val)?;
        self.set_raw("app.output_target_id", val, "string")
    }

    pub fn recording_mode_slot1(&self) -> Result<RecordingMode, AppError> {
        match self.get_raw("hotkey.slot1.mode")? {
            Some(s) => RecordingMode::from_str(&s),
            None => Ok(RecordingMode::Hold),
        }
    }

    pub fn set_recording_mode_slot1(&self, mode: RecordingMode) -> Result<(), AppError> {
        self.set_raw("hotkey.slot1.mode", &mode.to_string(), "string")
    }

    // -----------------------------------------------------------------------
    // Plugin-Setting API (AC-7)
    // -----------------------------------------------------------------------

    /// Write a plugin-scoped setting under `plugins.<plugin_id>.<key>`.
    /// Returns `AppError::Validation` when `key` starts with a core-namespace prefix
    /// or is otherwise mis-shaped.
    pub fn set_plugin_setting(
        &self,
        plugin_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), AppError> {
        validate_plugin_id(plugin_id)?;
        validate_plugin_key(key)?;
        check_core_namespace(key)?;
        validate_setting_value(key, value)?;
        let full_key = format!("plugins.{plugin_id}.{key}");
        self.set_raw(&full_key, value, "string")
    }

    /// Read a plugin-scoped setting. Returns `None` when the key does not exist.
    /// Returns `AppError::Validation` when `key` starts with a core-namespace prefix
    /// or is otherwise mis-shaped.
    pub fn get_plugin_setting(
        &self,
        plugin_id: &str,
        key: &str,
    ) -> Result<Option<String>, AppError> {
        validate_plugin_id(plugin_id)?;
        validate_plugin_key(key)?;
        check_core_namespace(key)?;
        let full_key = format!("plugins.{plugin_id}.{key}");
        self.get_raw(&full_key)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn db_err(msg: String) -> AppError {
    AppError {
        kind: AppErrorKind::Io,
        message: format!("settings db: {msg}"),
        user_message: None,
        retryable: false,
    }
}

fn validation_err(msg: String) -> AppError {
    AppError {
        kind: AppErrorKind::Validation,
        message: msg,
        user_message: Some("error.settings.validation".into()),
        retryable: false,
    }
}

fn check_core_namespace(key: &str) -> Result<(), AppError> {
    if CORE_PREFIXES.iter().any(|p| key.starts_with(p)) {
        return Err(validation_err(format!(
            "plugin setting key '{key}' violates core namespace (reserved prefixes: {})",
            CORE_PREFIXES.join(", ")
        )));
    }
    Ok(())
}

/// Validate a setting value: non-empty, length-capped, no control characters.
///
/// Reject any character classified as a control character by `char::is_control`
/// (covers U+0000..=U+001F, U+007F DEL, and the C1 range U+0080..=U+009F) —
/// any user-supplied hotkey-combo / locale / output-target-id is plain
/// printable text. Control chars in those slots cause silent late failures
/// (locale-loader miss, hotkey register fail) which the user only notices on
/// next boot.
fn validate_setting_value(key: &str, val: &str) -> Result<(), AppError> {
    if val.is_empty() {
        return Err(validation_err(format!(
            "settings value for '{key}' must not be empty"
        )));
    }
    if val.len() > MAX_VALUE_LEN {
        return Err(validation_err(format!(
            "settings value for '{key}' byte-length {} exceeds maximum {MAX_VALUE_LEN}",
            val.len()
        )));
    }
    if val.chars().any(|c| c.is_control()) {
        return Err(validation_err(format!(
            "settings value for '{key}' contains control characters"
        )));
    }
    Ok(())
}

/// Validate a plugin-setting `key`: same shape rules as `validate_setting_value`
/// plus a hard reject of the recursive `plugins.` prefix (which would shadow
/// the core's plugin-namespace primary keys, e.g. `plugins.<id>.plugins.x`).
fn validate_plugin_key(key: &str) -> Result<(), AppError> {
    if key.is_empty() {
        return Err(validation_err(
            "plugin setting key must not be empty".to_string(),
        ));
    }
    if key.len() > MAX_VALUE_LEN {
        return Err(validation_err(format!(
            "plugin setting key byte-length {} exceeds maximum {MAX_VALUE_LEN}",
            key.len()
        )));
    }
    if key.chars().any(|c| c.is_control()) {
        return Err(validation_err(format!(
            "plugin setting key '{key}' contains control characters"
        )));
    }
    if key.starts_with("plugins.") {
        return Err(validation_err(format!(
            "plugin setting key '{key}' must not start with 'plugins.' (recursive)"
        )));
    }
    Ok(())
}

/// Run the emitter inside `catch_unwind` so a panicking custom `SettingsEmitter`
/// implementation does not propagate the panic across the FFI boundary nor
/// abort a multi-write loop. Logs a warning instead.
fn emit_or_warn(emitter: &dyn SettingsEmitter, key: &str, value: &str) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        emitter.emit_settings_changed(key, value);
    }));
    if result.is_err() {
        tracing::warn!(
            setting_key = key,
            "settings emitter panicked; DB write already committed, continuing"
        );
    }
}

/// Validate a `plugin_id`: non-empty, length-capped, charset `[a-z0-9_-]+`.
///
/// Strict identifier shape prevents shadowing (`plugin_id="groq.config"`),
/// path-trickery (`".."`), and key-collision via dots in the id.
fn validate_plugin_id(plugin_id: &str) -> Result<(), AppError> {
    if plugin_id.is_empty() {
        return Err(validation_err("plugin_id must not be empty".to_string()));
    }
    if plugin_id.len() > MAX_PLUGIN_ID_LEN {
        return Err(validation_err(format!(
            "plugin_id exceeds {MAX_PLUGIN_ID_LEN} bytes"
        )));
    }
    if !plugin_id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err(validation_err(format!(
            "plugin_id '{plugin_id}' contains characters outside [a-z0-9_-]"
        )));
    }
    Ok(())
}

/// Acquire the SQLite connection mutex. Recovers from poisoning rather than
/// panic-cascading: if a previous holder panicked mid-write the data is still
/// consistent because rusqlite uses `INSERT OR REPLACE` (no in-memory state).
/// Per `feedback_scaffold_fail_soft_pattern` — never panic in
/// app-lifetime-bound services.
///
/// On poison-recovery: log a warning and best-effort `ROLLBACK` to clear any
/// dangling transaction the panicking holder might have left open. SQLite
/// silently no-ops `ROLLBACK` outside an active transaction, so this is safe
/// to invoke unconditionally.
fn lock_conn(m: &Mutex<Connection>) -> std::sync::MutexGuard<'_, Connection> {
    match m.lock() {
        Ok(g) => g,
        Err(poison) => {
            tracing::warn!("settings mutex poisoned by prior panic, recovering");
            let g = poison.into_inner();
            let _ = g.execute("ROLLBACK", []);
            g
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn noop() -> Arc<dyn SettingsEmitter> {
        Arc::new(NoopSettingsEmitter)
    }

    struct RecordingEmitter(Mutex<Vec<(String, String)>>);

    impl SettingsEmitter for RecordingEmitter {
        fn emit_settings_changed(&self, key: &str, value: &str) {
            self.0
                .lock()
                .unwrap()
                .push((key.to_string(), value.to_string()));
        }
    }

    fn recording_emitter() -> Arc<RecordingEmitter> {
        Arc::new(RecordingEmitter(Mutex::new(Vec::new())))
    }

    // -----------------------------------------------------------------------
    // AC-1: schema created + idempotent via rusqlite_migration
    // -----------------------------------------------------------------------

    #[test]
    fn settings_table_created_after_open() {
        let s = Settings::in_memory(noop()).unwrap();
        let guard = s.conn.lock().unwrap();
        let exists: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='settings'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(exists, 1, "settings table must exist after open()");
    }

    #[test]
    fn type_column_accepts_valid_types() {
        let s = Settings::in_memory(noop()).unwrap();
        // insert all 4 valid types
        {
            let g = s.conn.lock().unwrap();
            for (k, t) in [("k1", "string"), ("k2", "i64"), ("k3", "bool"), ("k4", "json")] {
                g.execute(
                    "INSERT INTO settings (key, value, type) VALUES (?1, 'v', ?2)",
                    rusqlite::params![k, t],
                )
                .unwrap();
            }
        }
        // reads succeed
        for key in ["k1", "k2", "k3", "k4"] {
            s.get_raw(key).expect("valid type must not error");
        }
    }

    #[test]
    fn unknown_type_returns_internal_error() {
        let s = Settings::in_memory(noop()).unwrap();
        {
            let g = s.conn.lock().unwrap();
            g.execute(
                "INSERT INTO settings (key, value, type) VALUES ('x', 'v', 'unknown')",
                [],
            )
            .unwrap();
        }
        let err = s.get_raw("x").unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Internal));
    }

    // -----------------------------------------------------------------------
    // AC-2 Ast A: TOML migration writes 5 fields transactionally
    // -----------------------------------------------------------------------

    fn sample_source() -> TomlMigrationSource {
        TomlMigrationSource {
            hotkey_slot1_combo: "Ctrl+Shift+R".to_string(),
            output_target_id: "clipboard".to_string(),
            ui_language: "de".to_string(),
            dictionary_language: "en".to_string(),
            output_language: "de".to_string(),
        }
    }

    #[test]
    fn migrate_from_toml_writes_all_5_fields() {
        let s = Settings::in_memory(noop()).unwrap();
        let src = sample_source();
        s.migrate_from_toml_if_needed(Some(&src)).unwrap();

        assert_eq!(s.hotkey_slot1_combo().unwrap(), "Ctrl+Shift+R");
        assert_eq!(s.output_target_id().unwrap(), "clipboard");
        assert_eq!(s.ui_language().unwrap(), "de");
        assert_eq!(s.dictionary_language().unwrap(), "en");
        assert_eq!(s.output_language().unwrap(), "de");
    }

    #[test]
    fn migrate_from_toml_emits_5_settings_changed_events() {
        // AC-2 + AC-5 alignment: downstream consumers (A8-Sub/C2/C3) listening on
        // `settings.changed` must see the migration writes. Direct SQL inside the
        // transaction does not call `set_raw`, so the emit happens explicitly after
        // commit.
        let emitter = recording_emitter();
        let s = Settings::in_memory(Arc::clone(&emitter) as Arc<dyn SettingsEmitter>).unwrap();
        s.migrate_from_toml_if_needed(Some(&sample_source())).unwrap();

        let calls = emitter.0.lock().unwrap();
        assert_eq!(calls.len(), 5, "migration must emit one event per core field");
        let keys: Vec<&str> = calls.iter().map(|(k, _)| k.as_str()).collect();
        for sentinel in MIGRATION_SENTINEL_KEYS {
            assert!(keys.contains(sentinel), "missing emit for '{sentinel}'");
        }
    }

    #[test]
    fn migration_does_not_block_when_only_plugin_rows_exist() {
        // AC-3 sentinel-key detect-condition (was: total-row-count). A plugin
        // that wrote a setting before first boot must not block the core
        // TOML→SQLite migration.
        let s = Settings::in_memory(noop()).unwrap();
        s.set_plugin_setting("groq", "model", "llama-3-70b").unwrap();

        s.migrate_from_toml_if_needed(Some(&sample_source())).unwrap();

        // Core fields migrated despite the pre-existing plugin row.
        assert_eq!(s.ui_language().unwrap(), "de");
        assert_eq!(s.hotkey_slot1_combo().unwrap(), "Ctrl+Shift+R");
    }

    #[test]
    fn migrate_from_toml_all_rows_have_string_type() {
        let s = Settings::in_memory(noop()).unwrap();
        s.migrate_from_toml_if_needed(Some(&sample_source())).unwrap();
        let guard = s.conn.lock().unwrap();
        let count: i64 = guard
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE type != 'string'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "all migrated rows must have type='string'");
    }

    // -----------------------------------------------------------------------
    // AC-2 Ast B: Fresh-Install — no TOML → table stays empty
    // -----------------------------------------------------------------------

    #[test]
    fn fresh_install_leaves_table_empty() {
        let s = Settings::in_memory(noop()).unwrap();
        s.migrate_from_toml_if_needed(None).unwrap();
        let guard = s.conn.lock().unwrap();
        let count: i64 = guard
            .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0, "fresh-install must not write any rows");
    }

    #[test]
    fn fresh_install_accessors_return_defaults() {
        let s = Settings::in_memory(noop()).unwrap();
        s.migrate_from_toml_if_needed(None).unwrap();
        assert_eq!(s.ui_language().unwrap(), DEFAULT_UI_LANGUAGE);
        assert_eq!(s.output_language().unwrap(), DEFAULT_OUTPUT_LANGUAGE);
        assert_eq!(s.dictionary_language().unwrap(), DEFAULT_DICTIONARY_LANGUAGE);
        assert_eq!(s.hotkey_slot1_combo().unwrap(), DEFAULT_HOTKEY_SLOT1_COMBO);
        assert_eq!(s.output_target_id().unwrap(), DEFAULT_OUTPUT_TARGET_ID);
    }

    // -----------------------------------------------------------------------
    // AC-3: migration idempotent (non-empty table → skip)
    // -----------------------------------------------------------------------

    #[test]
    fn migrate_from_toml_idempotent_on_second_run() {
        let s = Settings::in_memory(noop()).unwrap();
        let src = sample_source();
        s.migrate_from_toml_if_needed(Some(&src)).unwrap();

        // manually overwrite one key to detect if re-migration fires
        s.set_ui_language("fr").unwrap();

        // second run must not overwrite
        s.migrate_from_toml_if_needed(Some(&src)).unwrap();
        assert_eq!(s.ui_language().unwrap(), "fr", "second migration must not overwrite");
    }

    // -----------------------------------------------------------------------
    // AC-4 / AC-5: typed accessors + emitter
    // -----------------------------------------------------------------------

    #[test]
    fn set_ui_language_persists_and_emitter_receives_event() {
        let emitter = recording_emitter();
        let s = Settings::in_memory(Arc::clone(&emitter) as Arc<dyn SettingsEmitter>).unwrap();
        s.set_ui_language("de").unwrap();
        assert_eq!(s.ui_language().unwrap(), "de");
        let calls = emitter.0.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], ("ui.language".to_string(), "de".to_string()));
    }

    #[test]
    fn validation_error_on_empty_value_does_not_emit() {
        // AC-5 mandate: "Bei Persistenz-Fehler: kein Emit". Validation rejects
        // an empty value before the DB-write — the emitter must stay silent.
        // Covers the error→no-emit branch that the success-path test cannot.
        let emitter = recording_emitter();
        let s = Settings::in_memory(Arc::clone(&emitter) as Arc<dyn SettingsEmitter>).unwrap();
        let err = s.set_output_language("").unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Validation));
        assert_eq!(emitter.0.lock().unwrap().len(), 0, "no emit on validation error");
    }

    #[test]
    fn all_5_typed_accessors_roundtrip() {
        let s = Settings::in_memory(noop()).unwrap();
        s.set_hotkey_slot1_combo("Alt+F4").unwrap();
        s.set_output_target_id("keystroke").unwrap();
        s.set_ui_language("de").unwrap();
        s.set_dictionary_language("de").unwrap();
        s.set_output_language("de").unwrap();

        assert_eq!(s.hotkey_slot1_combo().unwrap(), "Alt+F4");
        assert_eq!(s.output_target_id().unwrap(), "keystroke");
        assert_eq!(s.ui_language().unwrap(), "de");
        assert_eq!(s.dictionary_language().unwrap(), "de");
        assert_eq!(s.output_language().unwrap(), "de");
    }

    // -----------------------------------------------------------------------
    // AC-7: plugin namespace guard
    // -----------------------------------------------------------------------

    #[test]
    fn plugin_set_rejects_core_namespace_prefixes() {
        let s = Settings::in_memory(noop()).unwrap();
        for forbidden in [
            "app.output_target_id",
            "hotkey.slot1.combo",
            "ui.language",
            "audio.sample_rate",
            "license.key",
            "history.max_entries",
        ] {
            let err = s.set_plugin_setting("groq", forbidden, "x").unwrap_err();
            assert!(
                matches!(err.kind, AppErrorKind::Validation),
                "key '{forbidden}' must be rejected"
            );
        }
    }

    #[test]
    fn plugin_get_rejects_core_namespace_prefixes() {
        let s = Settings::in_memory(noop()).unwrap();
        let err = s.get_plugin_setting("groq", "app.something").unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Validation));
    }

    #[test]
    fn plugin_set_get_symmetric_roundtrip() {
        let s = Settings::in_memory(noop()).unwrap();
        s.set_plugin_setting("groq", "model", "llama-3-70b").unwrap();
        let val = s.get_plugin_setting("groq", "model").unwrap();
        assert_eq!(val, Some("llama-3-70b".to_string()));
    }

    #[test]
    fn plugin_get_returns_none_for_missing_key() {
        let s = Settings::in_memory(noop()).unwrap();
        let val = s.get_plugin_setting("groq", "nonexistent").unwrap();
        assert_eq!(val, None);
    }

    // -----------------------------------------------------------------------
    // Input validation (P6 + P9 from code-review 2026-04-29)
    // -----------------------------------------------------------------------

    #[test]
    fn set_rejects_empty_value() {
        let s = Settings::in_memory(noop()).unwrap();
        let err = s.set_ui_language("").unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Validation));
    }

    #[test]
    fn set_rejects_value_with_control_chars() {
        let s = Settings::in_memory(noop()).unwrap();
        for bad in ["a\nb", "tab\there", "null\0byte"] {
            let err = s.set_hotkey_slot1_combo(bad).unwrap_err();
            assert!(matches!(err.kind, AppErrorKind::Validation), "must reject '{bad:?}'");
        }
    }

    #[test]
    fn set_rejects_oversized_value() {
        let s = Settings::in_memory(noop()).unwrap();
        let huge = "a".repeat(MAX_VALUE_LEN + 1);
        let err = s.set_output_target_id(&huge).unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Validation));
    }

    #[test]
    fn plugin_set_rejects_invalid_plugin_id() {
        let s = Settings::in_memory(noop()).unwrap();
        for bad in ["", "Groq", "groq.config", "groq/x", "../etc", "groq plugin"] {
            let err = s.set_plugin_setting(bad, "model", "v").unwrap_err();
            assert!(
                matches!(err.kind, AppErrorKind::Validation),
                "plugin_id '{bad}' must be rejected"
            );
        }
    }

    #[test]
    fn plugin_set_accepts_valid_plugin_id() {
        let s = Settings::in_memory(noop()).unwrap();
        for ok in ["groq", "deep-gram", "stt_v2", "x", "abc123"] {
            s.set_plugin_setting(ok, "k", "v").expect(ok);
        }
    }

    #[test]
    fn plugin_settings_are_namespace_prefixed_in_db() {
        let s = Settings::in_memory(noop()).unwrap();
        s.set_plugin_setting("groq", "model", "test-model").unwrap();
        let guard = s.conn.lock().unwrap();
        let key: String = guard
            .query_row(
                "SELECT key FROM settings WHERE key LIKE 'plugins.%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(key, "plugins.groq.model");
    }

    // -----------------------------------------------------------------------
    // AC-2: recording_mode_slot1 accessor
    // -----------------------------------------------------------------------

    #[test]
    fn recording_mode_slot1_default_fallback_returns_hold() {
        let s = Settings::in_memory(noop()).unwrap();
        let mode = s.recording_mode_slot1().unwrap();
        assert_eq!(mode, crate::recording::RecordingMode::Hold);
    }

    #[test]
    fn recording_mode_slot1_roundtrip() {
        let s = Settings::in_memory(noop()).unwrap();
        for mode in [
            crate::recording::RecordingMode::Hold,
            crate::recording::RecordingMode::Toggle,
            crate::recording::RecordingMode::AutoStop,
            crate::recording::RecordingMode::WaitAndType,
        ] {
            s.set_recording_mode_slot1(mode.clone()).unwrap();
            let got = s.recording_mode_slot1().unwrap();
            assert_eq!(got, mode);
        }
    }

    #[test]
    fn recording_mode_slot1_invalid_stored_value_returns_validation_error() {
        let s = Settings::in_memory(noop()).unwrap();
        // Directly insert a bad value, bypassing the typed accessor
        {
            let g = s.conn.lock().unwrap();
            g.execute(
                "INSERT OR REPLACE INTO settings (key, value, type) VALUES ('hotkey.slot1.mode', 'bad_mode', 'string')",
                [],
            ).unwrap();
        }
        let err = s.recording_mode_slot1().unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Validation));
    }
}
