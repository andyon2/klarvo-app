---
story: "1.4"
epic: "1"
title: "Hardened config migration — pre-migration backup + error propagation"
status: done
findings: ["ROB-05"]
gatedBy: ADR-0015
buildsOn: ["1.1", "1.2", "1.3", "4.3"]
inputDocuments:
  - _bmad-output/planning-artifacts/epics.md
  - docs/adr/0015-state-file-write-convention.md
  - docs/robustness-audit-2026-05-30.md
  - _bmad-output/implementation-artifacts/1-1-atomic-state-file-writes-via-a-shared-save-atomic-helper.md
  - _bmad-output/implementation-artifacts/1-2-backup-on-corrupt-recovery-in-load-config.md
  - _bmad-output/implementation-artifacts/1-3-single-writer-serialization-for-state-file-saves.md
  - _bmad-output/implementation-artifacts/4-3-single-sanctioned-config-write-path-save-config-locked.md
---

# Story 1.4: Hardened config migration — pre-migration backup + error propagation

Status: done

## Story

As a klarvo user upgrading to a new version,
I want my config migration protected,
so that a write failure mid-migration on first upgrade-boot can't lose my keys and license at the
worst possible moment.

## Acceptance Criteria

1. **Pre-migration backup written before each migration save.**
   When any of the three config migrations runs and is about to persist, a backup of the
   current on-disk `config.json` is written to `config.json.pre-migration-<unix_ts>` in the same
   directory **before** `save_config` is called for that migration. The backup uses
   `crate::fs::save_atomic` (same path as `backup_corrupt_config`). Infallible by contract: a
   failed backup write is logged (`log::warn!`) but never blocks migration from proceeding.

2. **Migration write errors propagated via `warnings`, not swallowed.**
   The three `if let Err(e) = save_config(...) { log::warn!(...) }` blocks (lines ~1162, ~1211,
   ~1240 in `config/mod.rs`) are replaced with `if let Err(e) = ... { warnings.push(msg); log::warn!(...) }`.
   The error is both logged AND appended to the `warnings: &mut Vec<String>` already passed into
   `load_config_reporting`. The warning message includes the migration name and instructs the user
   that their keys/license are intact in the pre-migration backup.

3. **Atomic write inherited automatically.**
   Since `save_config` already routes through `crate::fs::save_atomic` (Story 1.1, `config/mod.rs:1349-1358`),
   no additional change is needed for atomicity. The migrations inherit it.

4. **Keys + license NOT lost on migration write failure.**
   Given: a migration triggers + `save_config` fails. Then: the pre-migration backup exists on disk
   with all keys/license intact. The in-memory `config` struct is correct (migration applied). Only
   the on-disk file is stale (pre-migration state) — but that is the safe state.

5. **No backup written when no migration runs.**
   `load_config_reporting` on a current (already-migrated) config writes zero new files to disk.
   The backup is only written when a migration is actually about to execute its `save_config` call.

6. **`cargo test` green; `cargo clippy` clean on touched files.**
   All existing migration tests stay green. New specs cover: backup file created (one per
   migration that fires), backup survives a migration write failure, no backup when no migration.

## DoD

- **Linux (load-bearing):** `cargo test` passes including new migration-backup specs.
  `cargo clippy` clean on touched files. (Repo-wide clippy is pre-existing red on unrelated
  files — touched-files-clean is the bar, escalation rule A6.)
- **Windows cross-compile:** `cargo check --target x86_64-pc-windows-gnu` on touched files.
  Ensures no Windows-incompatible API is introduced. Full Windows smoke **not required** for
  this story: `save_atomic` Windows rename-semantics were verified in Story 1.1 (c1ffa79),
  and the same atomic-backup pattern was verified in Story 1.2 (ae4068f). This story
  applies the same primitives to new backup file paths — no new Windows-specific risk.

## Tasks / Subtasks

- [x] Add `backup_pre_migration_config` function adjacent to `backup_corrupt_config` (AC: 1, 4)
- [x] Update Migration 1 (sttPriority/llmPriority): backup + propagate error to `warnings` (AC: 1, 2)
- [x] Update Migration 2 (hotkey_slots): backup + propagate error to `warnings` (AC: 1, 2)
- [x] Update Migration 3 (insert_and_send_per_slot): backup + propagate error to `warnings` (AC: 1, 2)
- [x] Add `pre_migration_backups` helper + Specs A–E in `#[cfg(test)]` (AC: 5, 6)

## Dev Notes

Grounded against HEAD (`v1-ship`, post-commit 9e017d0). Builds on Story 1.1's
`crate::fs::save_atomic` (`src-tauri/src/fs.rs:13`), Story 1.2's `backup_corrupt_config`
pattern (`config/mod.rs:972-1004`), and Story 1.3/4.3's `save_config` + `save_config_locked`
distinction. Rule codes (A*/V*/E*) reference `docs/bmad-autopilot-escalation-contract.md`.

---

### The bug (ROB-05) in concrete terms

The three migrations in `load_config_reporting` (`config/mod.rs`) all save with a bare `if let
Err(e) = save_config(...) { log::warn!(...) }`. Two problems:

1. **No pre-migration backup**: if the atomic rename partially fails or a later-boot
   `load_config` sees unexpected state, the user has no snapshot of their pre-migration config.
2. **Errors swallowed**: `warnings` is right there as a parameter but migration errors never
   touch it. The caller (`lib.rs`) routes `warnings` to the user-visible error event. Without
   propagation, the user has no idea their migration failed.

The worst-case scenario: user upgrades, migration runs, `save_config` fails (e.g., disk full,
permissions, NTFS lock), and the old config.json is still on disk — but the user sees nothing.
Next boot: migration runs again (since the old config still has the legacy fields) → same failure.
Keys/license survive because the old file is still there, but the state is stuck.

With this story's fix:
- Pre-migration backup exists → user has a known-good snapshot regardless of what happens.
- Warning is surfaced → user sees a toast / error banner and can report it.

---

### Implementation: new `backup_pre_migration_config` function

Add a private function adjacent to `backup_corrupt_config` in `config/mod.rs`:

```rust
/// Writes a best-effort backup of the current on-disk `config.json` to
/// `config.json.pre-migration-<unix_ts>` before a schema migration mutates
/// and re-persists the config (ROB-05 / ADR-0015 §4).
///
/// Infallible by contract: a failed backup is logged as a warning but never
/// blocks the migration. The caller must invoke this BEFORE calling
/// `save_config` for the migration write.
fn backup_pre_migration_config(app_data_dir: &Path, migration_name: &str) {
    let path = app_data_dir.join(CONFIG_FILE);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup_name = format!("{CONFIG_FILE}.pre-migration-{ts}");
    let backup_path = match path.parent() {
        Some(dir) => dir.join(&backup_name),
        None => std::path::PathBuf::from(&backup_name),
    };
    match std::fs::read(&path) {
        Ok(raw) => match crate::fs::save_atomic(&backup_path, &raw) {
            Ok(()) => log::info!(
                "[config] Pre-migration backup written to {} (migration: {migration_name})",
                backup_path.display()
            ),
            Err(e) => log::warn!(
                "[config] Failed to write pre-migration backup to {} ({e}); \
                 continuing with migration (migration: {migration_name})",
                backup_path.display()
            ),
        },
        Err(e) => log::warn!(
            "[config] Could not read config.json for pre-migration backup ({e}); \
             continuing with migration (migration: {migration_name})"
        ),
    }
}
```

Key differences from `backup_corrupt_config`:
- Reads the file itself (`std::fs::read(path)`) rather than receiving raw bytes — migration
  runs after parsing, so `contents` is no longer in scope.
- Takes `migration_name: &str` for log context.
- Uses `pre-migration-` prefix (not `corrupt-`) in the backup filename.
- Does NOT push to `warnings` on failure — backup failures are internal telemetry, not
  user-actionable. Only migration SAVE failures get pushed to warnings (AC2).

---

### Implementation: update the three migration save blocks

**Migration 1 — sttPriority/llmPriority → provider fields** (~lines 1157-1165):

```rust
// BEFORE:
if migrated {
    log::info!("[config] Migrated legacy sttPriority/llmPriority to provider fields");
    config.stt_priority.clear();
    config.llm_priority.clear();
    if let Err(e) = save_config(app_data_dir, &config) {
        log::warn!("[config] Failed to persist migrated config: {e}");
    }
}

// AFTER:
if migrated {
    log::info!("[config] Migrated legacy sttPriority/llmPriority to provider fields");
    config.stt_priority.clear();
    config.llm_priority.clear();
    backup_pre_migration_config(app_data_dir, "sttPriority/llmPriority");
    if let Err(e) = save_config(app_data_dir, &config) {
        let msg = format!(
            "Config migration (sttPriority/llmPriority) could not be saved: {e}. \
             Your original config was backed up to config.json.pre-migration-<ts> — \
             your keys and license are intact."
        );
        log::warn!("[config] {msg}");
        warnings.push(msg);
    }
}
```

**Migration 2 — hotkey/hotkey_mode → hotkey_slots** (~lines 1210-1213):

```rust
// BEFORE:
if let Err(e) = save_config(app_data_dir, &config) {
    log::warn!("[config] Failed to persist hotkey_slots migration: {e}");
}

// AFTER:
backup_pre_migration_config(app_data_dir, "hotkey_slots");
if let Err(e) = save_config(app_data_dir, &config) {
    let msg = format!(
        "Config migration (hotkey_slots) could not be saved: {e}. \
         Your original config was backed up to config.json.pre-migration-<ts> — \
         your keys and license are intact."
    );
    log::warn!("[config] {msg}");
    warnings.push(msg);
}
```

**Migration 3 — global insert_and_send → per-slot** (~lines 1239-1242):

```rust
// BEFORE:
if let Err(e) = save_config(app_data_dir, &config) {
    log::warn!("[config] Failed to persist insert_and_send slot migration: {e}");
}

// AFTER:
backup_pre_migration_config(app_data_dir, "insert_and_send_per_slot");
if let Err(e) = save_config(app_data_dir, &config) {
    let msg = format!(
        "Config migration (insert_and_send per-slot) could not be saved: {e}. \
         Your original config was backed up to config.json.pre-migration-<ts> — \
         your keys and license are intact."
    );
    log::warn!("[config] {msg}");
    warnings.push(msg);
}
```

---

### Critical: `load_config_reporting` already has `warnings: &mut Vec<String>`

The `warnings` parameter is already threaded through `load_config_reporting` (see line 1033).
No signature change needed. The `backup_corrupt_config` function (line 972) already uses it.
This story simply extends the same mechanism to migration saves.

### Critical: `load_config` wrapper does NOT need `warnings`

`load_config` (line 1021) is a test-only wrapper that calls `load_config_reporting` with a
throwaway `Vec<String>`. Migration error warnings will be silently dropped there — that is
correct and expected. Tests that want to verify warning propagation must call
`load_config_reporting` directly with a `&mut warnings` they inspect.

### Critical: `save_config` vs `save_config_locked`

Migration runs inside `load_config_reporting`, which is called at boot time BEFORE `AppState`
is created (`lib.rs` setup sequence). Therefore migrations CORRECTLY call `save_config` directly
(the `pub(crate)` boot-time path). `save_config_locked` (AppState method, Story 4.3) is for
runtime saves only. Do NOT change migrations to use `save_config_locked`.

---

### Files to modify

- **`src-tauri/src/config/mod.rs`** — only file changed:
  1. Add `backup_pre_migration_config` function (adjacent to `backup_corrupt_config`)
  2. Update three migration save blocks (lines ~1162, ~1211, ~1240)
  3. Add ~6 new test functions in the `#[cfg(test)] mod tests` block at the bottom

No other files are touched. No signature changes, no public API changes.

---

### Tests to write

All tests go in `config/mod.rs` under `#[cfg(test)] mod tests`. Follow the existing pattern
(use `temp_dir()` helper, `std::fs::write` for setup, `load_config` or `load_config_reporting`
to trigger).

**Spec A — backup created on stt/llm priority migration:**
```rust
#[test]
fn test_migration_backup_written_on_stt_priority_migration() {
    let dir = temp_dir();
    let legacy = r#"{"sttPriority": ["openai"], "llmPriority": ["anthropic"]}"#;
    std::fs::write(dir.path().join("config.json"), legacy.as_bytes()).unwrap();

    let _ = load_config(dir.path());

    let backups: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("config.json.pre-migration-")
        })
        .collect();
    assert_eq!(backups.len(), 1, "exactly one pre-migration backup should be written");
    // Backup must be valid JSON and contain the pre-migration sttPriority field.
    let backup_content =
        std::fs::read_to_string(backups[0].path()).expect("backup must be readable");
    assert!(
        backup_content.contains("sttPriority") || backup_content.contains("sttPriority"),
        "backup should contain the pre-migration field"
    );
}
```

**Spec B — backup created on hotkey_slots migration:**
```rust
#[test]
fn test_migration_backup_written_on_hotkey_slots_migration() {
    let dir = temp_dir();
    let legacy = r#"{"hotkey": "ctrl+alt+r", "hotkeyMode": "toggle"}"#;
    std::fs::write(dir.path().join("config.json"), legacy.as_bytes()).unwrap();

    let _ = load_config(dir.path());

    let backups: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("config.json.pre-migration-")
        })
        .collect();
    assert!(!backups.is_empty(), "pre-migration backup must be written for hotkey_slots migration");
}
```

**Spec C — no backup when no migration:**
```rust
#[test]
fn test_no_migration_backup_when_no_migration_runs() {
    let dir = temp_dir();
    // Already-migrated config: has hotkey_slots, no legacy sttPriority.
    let modern = r#"{
        "hotkeySlots": [
            {"hotkey": "ctrl+alt+r", "mode": "hold"},
            {"hotkey": "", "mode": "hold"}
        ]
    }"#;
    std::fs::write(dir.path().join("config.json"), modern.as_bytes()).unwrap();

    let _ = load_config(dir.path());

    let backups: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("config.json.pre-migration-")
        })
        .collect();
    assert!(backups.is_empty(), "no pre-migration backup when no migration runs");
}
```

**Spec D — migration write error propagated to warnings:**
```rust
#[test]
fn test_migration_write_error_propagated_to_warnings() {
    let dir = temp_dir();
    let legacy = r#"{"hotkey": "ctrl+alt+r", "hotkeyMode": "toggle"}"#;
    std::fs::write(dir.path().join("config.json"), legacy.as_bytes()).unwrap();

    // Make the dir read-only so save_config fails (Linux only — skip on Windows).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();

        let mut warnings = Vec::new();
        let _ = load_config_reporting(dir.path(), &mut warnings);

        // Restore permissions so TempDir can clean up.
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();

        assert!(
            warnings.iter().any(|w| w.contains("migration") && w.contains("could not be saved")),
            "migration write failure must be propagated to warnings; got: {warnings:?}"
        );
    }
}
```

> **Note for Spec D:** The `#[cfg(unix)]` guard is required — read-only dir semantics on
> Windows (NTFS) differ. The test is Linux-only; that is acceptable because the Linux test is
> the load-bearing DoD gate and the cross-compile check covers the Windows path.

**Spec E — backup content is valid JSON (belt-and-suspenders):**
```rust
#[test]
fn test_migration_backup_is_valid_json() {
    let dir = temp_dir();
    let legacy = r#"{"sttPriority": ["openai"]}"#;
    std::fs::write(dir.path().join("config.json"), legacy.as_bytes()).unwrap();

    let _ = load_config(dir.path());

    let backup = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("config.json.pre-migration-")
        })
        .expect("backup must exist");

    let content = std::fs::read_to_string(backup.path()).unwrap();
    serde_json::from_str::<serde_json::Value>(&content)
        .expect("backup must be valid JSON");
}
```

---

### Anti-patterns to avoid

- **Do NOT push to `warnings` when backup write fails.** Backup failure is an internal
  telemetry warning, not user-actionable. Only migration SAVE failures get pushed to `warnings`.
- **Do NOT share one backup across all three migrations.** Each migration that fires writes
  its own backup (they run sequentially, each could fail independently).
- **Do NOT change `save_config` call sites in test code.** Only production migration sites.
- **Do NOT call `save_config_locked`.** Migrations run before `AppState` exists.
- **Do NOT read the file before parsing** for the backup. Read it fresh inside
  `backup_pre_migration_config` using `std::fs::read(path)` — the original parse already
  consumed `contents` via `read_to_string` and it's out of scope.

### Project Structure Notes

- All changes are in `src-tauri/src/config/mod.rs` only.
- `backup_pre_migration_config` follows the exact same structure as `backup_corrupt_config`
  (line 972). Place it immediately after `backup_corrupt_config` in the file (before the public
  API section at line 950+).
- New function is `fn backup_pre_migration_config(...)` (private, no `pub`).

### References

- ROB-05: `docs/robustness-audit-2026-05-30.md` §2
- ADR-0015 §4: `docs/adr/0015-state-file-write-convention.md`
- `backup_corrupt_config`: `src-tauri/src/config/mod.rs:972-1004`
- `load_config_reporting`: `src-tauri/src/config/mod.rs:1033`
- `save_config`: `src-tauri/src/config/mod.rs:1349-1358`
- `save_atomic`: `src-tauri/src/fs.rs:13`
- Story 1.1 (atomic write): `_bmad-output/implementation-artifacts/1-1-*.md`
- Story 1.2 (backup-on-corrupt): `_bmad-output/implementation-artifacts/1-2-*.md`
- Story 4.3 (save_config_locked): `_bmad-output/implementation-artifacts/4-3-*.md`

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (create-story + dev-story, 2026-05-31)

### Debug Log References

### Completion Notes List

- Added `backup_pre_migration_config(app_data_dir, migration_name)` at `config/mod.rs` adjacent to `backup_corrupt_config`. Uses `std::fs::read` + `crate::fs::save_atomic`. Infallible; logs warn on failure, never blocks migration.
- Three migration save blocks updated: `backup_pre_migration_config` called before each `save_config`; write errors now push to `warnings` (in addition to `log::warn!`).
- Atomicity inherited automatically (AC3): `save_config` already routes through `save_atomic` since Story 1.1.
- 6 new test functions added: `test_migration_backup_written_on_stt_priority_migration`, `test_migration_backup_written_on_hotkey_slots_migration`, `test_migration_backup_written_on_insert_and_send_migration`, `test_no_migration_backup_when_no_migration_runs`, `test_migration_backup_is_valid_json`, `test_migration_write_error_propagated_to_warnings` (Linux-only `#[cfg(unix)]`).
- `pre_migration_backups(dir)` helper function added alongside `corrupt_backups`.
- 537 tests pass (up from 531; +6 new). 0 failures. Clippy clean on touched files.
- Windows cross-compile target not installed in this WSL env (pre-existing toolchain gap); new code uses only cross-platform APIs (`std::fs::read`, `std::time::SystemTime`, `crate::fs::save_atomic`).

### File List

- `src-tauri/src/config/mod.rs` — modified

### Review Findings

- [x] [Review][Decision] AC-6 gap — "backup succeeds but save_config fails" scenario untested — dismissed: backup-success is proven by Spec A–C; save_config failure propagation is proven by Spec D; combined scenario is untestable without mocking because both share the same directory (read-only blocks both). Structural guarantee holds.
- [x] [Review][Patch] `backups.len() == 1` assertion fails in stt test — FIXED: fixture now includes `hotkeySlots` to suppress hotkey_slots migration; only sttPriority migration fires → exactly 1 backup [src-tauri/src/config/mod.rs:test_migration_backup_written_on_stt_priority_migration]
- [x] [Review][Patch] Timestamp collision silently overwrites earlier backup when multiple migrations fire within the same second — FIXED: migration name embedded in backup filename (`pre-migration-{ts}-{safe_name}`); each migration produces a unique filename regardless of timestamp granularity [src-tauri/src/config/mod.rs:backup_pre_migration_config]
- [x] [Review][Patch] User-facing warning contains literal `<ts>` placeholder instead of actual backup path — FIXED: function now returns `Option<String>` (backup filename on success); call sites embed actual filename in message or fall back to generic "look for config.json.pre-migration-* files" [src-tauri/src/config/mod.rs:~1203,~1258,~1295]
- [x] [Review][Patch] `backup_path` None-arm uses bare CWD path instead of `app_data_dir` — FIXED: `None => app_data_dir.join(&backup_name)` [src-tauri/src/config/mod.rs:backup_pre_migration_config]
- [x] [Review][Patch] Duplicate stale comment in `test_migration_write_error_propagated_to_warnings` — dismissed: false positive; only one `// Restore permissions` comment exists in the actual file

#### Independent re-review — Opus 4.8, 3 adversarial layers (2026-05-31)

Re-run requested because the original review (Sonnet 4.6) was distrusted. Verified independently: 98 config-module tests pass / 0 fail, `config/mod.rs` is clippy-clean (all 19 repo warnings are in unrelated files), Windows cross-compile target genuinely not installed. The 4 Sonnet patches are sound; the following were **additionally** surfaced.

- [x] [Review][Decision] RESOLVED (Opus, Option 2 — reword): Chained-migration backups capture post-previous-migration state, not the true original — for migrations 2 & 3 the "Your original config was backed up to …" wording overstates what the file contains. `backup_pre_migration_config` re-reads `config.json` from disk on each call (`config/mod.rs:1017,1029`), but migration 1 already wrote its `save_config` (`:1214`) before migration 2's backup reads disk (`:1275`), and likewise 2→3. On a first-upgrade boot where ≥2 migrations fire, the `…-hotkey_slots` / `…-insert_and_send_per_slot` backups hold the *post-prior-migration* config, and the literal pre-upgrade byte-image survives only in the `…-sttPriority-llmPriority` backup. **No data loss**: keys/license live in none of the migrated fields, so every backup preserves them (AC-4's safety goal holds). The defect is (a) the "original config" wording is inaccurate for chained migrations and (b) a true byte-for-byte rollback is only possible from backup A. Options: snapshot the on-disk bytes once before any migration / reword the message to drop "original" / accept as-is. [config/mod.rs:1017-1052,1213-1333]
- [x] [Review][Patch] APPLIED: AC-4's core safety claim ("keys + license NOT lost on migration write failure") had zero direct test coverage. Spec D (`test_migration_write_error_propagated_to_warnings`) makes the dir read-only (`0o555`), which fails BOTH the backup `save_atomic` AND `save_config` — so it exercises the generic `.unwrap_or_else` location branch and never asserts that the on-disk `config.json` still holds the keys/license afterward. Add an assertion to Spec D: write a legacy config containing an API key, trigger the failed migration, then read `config.json` back and assert the key survived. (The `Some(filename)` warning branch — backup succeeds, save fails — is genuinely mock-dependent on Linux since both writes share the dir; that sub-case is acceptably deferred, which partly vindicates the line-480 dismissal, but its stated reasoning was imprecise.) [config/mod.rs:test_migration_write_error_propagated_to_warnings ~3066]
- [x] [Review][Patch] APPLIED (folded into the decision patch): Three near-identical ~12-line error blocks (location-resolve + `format!` + `log::warn!` + `warnings.push`) were copy-pasted across the three migration sites with only the migration-name literal differing — quality-only, 3 proven consumers justify extracting `fn migration_save_warning(label, e, backup_file) -> String`. [config/mod.rs:1214-1228,1276-1290,1318-1332]
- [x] [Review][Defer] Unbounded pre-migration-backup accumulation across repeated boots with a config.json-specific persistent save failure (no GC/retention; migration re-fires every boot because `save_config` never persists the migrated form). Mirrors the existing `backup_corrupt_config` no-cleanup property; a retention policy spanning both backup kinds is a follow-up. [config/mod.rs:1017-1052] — deferred, design decision
- [x] [Review][Defer] DoD Windows cross-compile gate (`cargo check --target x86_64-pc-windows-gnu`) not executed — target not installed in this WSL env (confirmed). New code uses only portable APIs (`std::fs::read`, `SystemTime`, `str::replace`, `save_atomic`); risk low but the gate is technically unmet. [DoD] — deferred, verify on next Windows build
- Dismissed as noise (5): same-migration same-second cross-boot overwrite (overwritten backup is byte-identical since the failed save left disk unchanged); `unwrap_or(0)` clock-skew filename (cosmetic, unrealistic, matches existing pattern); `save_atomic` orphan-temp (pre-existing accepted `save_atomic` contract, not introduced here); dead `path.parent()` None-arm (correct-but-unreachable, strictly safer than `backup_corrupt_config`); boot-time bypass of the Story 1.3 `config_disk_write` lock (by design — migrations run single-threaded before `AppState` exists, spec-sanctioned in Dev Notes; already tracked as a Story 1.3 deferral).
