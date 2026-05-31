---
story: "1.3"
epic: "1"
title: "Single-writer serialization for state-file saves"
status: done
findings: ["ROB-04"]
gatedBy: ADR-0015
buildsOn: ["1.1", "1.2"]
inputDocuments:
  - _bmad-output/planning-artifacts/epics.md
  - docs/adr/0015-state-file-write-convention.md
  - docs/robustness-audit-2026-05-30.md
  - _bmad-output/implementation-artifacts/1-1-atomic-state-file-writes-via-a-shared-save-atomic-helper.md
  - _bmad-output/implementation-artifacts/1-2-backup-on-corrupt-recovery-in-load-config.md
---

# Story 1.3: Single-writer serialization for state-file saves

Status: done

## Story

As a klarvo user,
I want concurrent settings saves serialized,
so that a background bar-drag save can't clobber the whole config file and silently erase an API key I just saved.

## Acceptance Criteria

1. **Disk-write lock exists in AppState.** `AppState` gains a `config_disk_write: Mutex<()>`
   field (std::sync::Mutex). This is the single serialization point for all config disk I/O.

2. **All `save_config` callers hold the disk-write lock across the read-modify-write cycle.**
   Every production call to `save_config` in `src-tauri/src/commands/` (settings.rs, misc.rs,
   license.rs, voice_command.rs) acquires `config_disk_write` **before** reading the in-memory
   config and holds it **through** the disk write. Callers in test code are excluded.

3. **`config: Mutex<AppConfig>` is NOT held during disk I/O.**
   Every caller drops the in-memory config guard before calling `save_config`. The disk-write
   lock serializes the writes; the config lock is only held during the in-memory
   read-modify-clone step (not I/O). This is the opposite of `save_advanced_settings`'s
   current (incorrect) pattern of holding the config Mutex across disk I/O.

4. **`save_advanced_settings` fixed.** `settings.rs:609-627` currently holds
   `config: Mutex<AppConfig>` across the `save_config` call (blocks all readers during I/O).
   After the fix: acquires `config_disk_write` first, then acquires and drops `config`, then
   calls `save_config`, matching the canonical pattern.

5. **Race scenario: bar-drag doesn't erase an API key.** Given concurrent execution of
   `save_bar_position` (which takes a clone of config) and `save_settings` (which persists a
   new API key): if `save_settings` acquires `config_disk_write` first, its key write completes
   atomically before `save_bar_position` even reads the config. If `save_bar_position` acquires
   first, it completes and saves bar position; `save_settings` then reads config (with bar
   position already updated), merges the new key, and saves. Either way, the final on-disk
   config contains **both** the new API key and the new bar position.

6. **`cargo test` green; `cargo clippy` clean on touched files.**
   All existing config tests stay green. New specs (see tasks) cover the concurrent-save race
   and the `save_advanced_settings` lock discipline fix.

## DoD

- **Linux (load-bearing):** `cargo test` passes including new concurrent-save specs. `cargo
  clippy` clean on touched files. (Repo-wide clippy is pre-existing red on unrelated files —
  `v1-ship` baseline from before this epic. Touched-files-clean is the bar — escalation rule A6.)
- **No Windows smoke required for this story.** This story adds a pure-Rust `Mutex<()>` and
  rewires call sites. There are no new file-system primitives, no platform-specific paths, no
  changes to the write mechanism itself (`save_atomic` was already verified on Windows in Story
  1.1, commit `c1ffa79`). The Linux test is sufficient and authoritative.

## Dev Notes

Grounded against HEAD (`v1-ship`, 2026-05-31). Builds on Story 1.1's `crate::fs::save_atomic`
(`src-tauri/src/fs.rs`) and Story 1.2's backup-on-corrupt changes (`config/mod.rs`). Rule codes
(A*/V*/E*) reference `docs/bmad-autopilot-escalation-contract.md`.

---

### The bug (ROB-04) in concrete terms

Every `save_config` caller in commands/ follows the same pattern: acquire the in-memory
`config: Mutex<AppConfig>` lock, modify, clone, **drop the lock**, then call `save_config` (disk
write). There is no disk-write serialization. Two concurrent callers can therefore interleave:

```
Thread A (save_settings):   [lock config] → clone → [drop config]
Thread B (save_bar_position):              [lock config] → clone → [drop config] → save_config(B-clone)
Thread A (save_settings):                                                           → save_config(A-clone)
```

If B writes a stale clone (read before A updated memory) and then A writes the correct clone,
no data is lost. But if A writes first and then B writes its stale clone last, B's whole-file
overwrite erases A's API key change — the UI reports "saved" for both, the key is gone.

The exception is `save_advanced_settings` (`settings.rs:609-627`), which holds the config
Mutex across the disk write. This prevents *some* races but is an anti-pattern: it blocks
all config readers (including the recording pipeline) for the duration of I/O. It also doesn't
help against callers that read before `save_advanced_settings` acquires the lock.

---

### The fix: `config_disk_write: Mutex<()>` in `AppState`

Add a new `std::sync::Mutex<()>` field to `AppState`. All `save_config` callers must hold this
lock for the **entire** read-modify-write cycle — not just the write.

**Canonical pattern (every caller must converge on this):**

```rust
let _disk_guard = crate::lock!(inner.config_disk_write)?; // 1. acquire disk-write lock
let cfg_clone = {
    let mut cfg = crate::lock!(inner.config)?;             // 2. acquire config lock
    // modify cfg here
    cfg.clone()
};                                                          // 3. config lock drops
save_config(&inner.app_data_dir, &cfg_clone)               // 4. disk write (no config lock held)
    .map_err(|e| format!("...: {e}"))?;
// _disk_guard drops here
```

For `save_settings` (which does pre-write validation before reading config):

```rust
// (validation work before the lock — hotkey parsing, license checks — unchanged)
let _disk_guard = crate::lock!(inner.config_disk_write)?;  // acquire AFTER validation
let existing = crate::lock!(inner.config)?.clone();        // read inside disk-write lock
let new_cfg = merge_settings(existing, patch);
save_config(&inner.app_data_dir, &new_cfg)...?;
*crate::lock!(inner.config)? = new_cfg;                    // update memory (still inside disk-write lock)
// providers and hotkey re-registration follow AFTER disk-write lock drops
```

**Why std::sync::Mutex, not tokio::sync::Mutex?**
- All existing AppState locks are `std::sync::Mutex`.
- `save_config` is synchronous (no `.await`). Holding a std Mutex across a sync call from an
  async Tauri command is correct — no executor thread is blocked on an await.
- Consistency with existing lock macros (`crate::lock!`).

**Why the read must be inside the disk-write lock:**
The lock must cover the entire read-modify-write cycle, not just the write. If Thread B reads
*after* Thread A completes its write, B's clone is fresh and includes A's changes. If B reads
*before* A writes, B's clone is stale — and B's write (which comes after A's, since the lock
serializes writes) would overwrite A's changes. The lock on the read prevents this.

---

### AppState changes (`src-tauri/src/lib.rs`)

**Add to struct `AppState` (~line 222, after `config: Mutex<AppConfig>`):**
```rust
pub config_disk_write: Mutex<()>,
```

**Add to `AppState::new` (~line 311, after `config: Mutex::new(cfg)`):**
```rust
config_disk_write: Mutex::new(()),
```

No other change needed in lib.rs.

---

### All production call sites to update

Every entry below is a production `save_config` call that needs the canonical pattern applied.
Test-only call sites (inside `#[cfg(test)]` / `#[test]` blocks) do NOT need the disk-write lock
(tests run single-threaded or control concurrency explicitly).

**`src-tauri/src/commands/settings.rs`**

| Line (approx.) | Function | Current lock discipline | Fix needed |
|---|---|---|---|
| 495 | `save_settings` (async) | config dropped before write ✓ | Wrap read+write in `config_disk_write` |
| 625 | `save_advanced_settings` (sync) | **config held across write ✗** | Acquire `config_disk_write` first; drop config before write |
| 673 | `update_api_keys` (async) | config dropped before write ✓ | Wrap read+write in `config_disk_write` |
| 701 | `set_language` (sync) | config dropped before write ✓ | Wrap in `config_disk_write` |
| 713 | `set_cleanup_style` (sync) | config dropped before write ✓ | Wrap in `config_disk_write` |
| 728 | `set_output_language` (sync) | config dropped before write ✓ | Wrap in `config_disk_write` |
| 785 | `set_hotkey_slot` (sync) | config dropped before write ✓ | Wrap in `config_disk_write` |
| 883 | (check line) | config dropped before write ✓ | Wrap in `config_disk_write` |
| 971 | (check line) | config dropped before write ✓ | Wrap in `config_disk_write` |

**`src-tauri/src/commands/misc.rs`**

| Line (approx.) | Function | Current lock discipline | Fix needed |
|---|---|---|---|
| 41 | `save_profiles` (sync) | explicit `drop(cfg)` before write ✓ | Wrap in `config_disk_write` |
| 76 | `save_snippets` (sync) | explicit `drop(cfg)` before write ✓ | Wrap in `config_disk_write` |
| 185 | `save_bar_position` (sync) | explicit `drop(cfg)` before write ✓ | Wrap in `config_disk_write` |

**`src-tauri/src/commands/license.rs`**

| Lines (approx.) | Function | Fix needed |
|---|---|---|
| 78, 100, 149, 197 | license validation/activation | Wrap each read+write in `config_disk_write` |

**`src-tauri/src/commands/voice_command.rs`**

| Lines (approx.) | Function | Fix needed |
|---|---|---|
| 55, 97 | voice command saves | Wrap each in `config_disk_write` |

**Verify line numbers before editing** — grounded at 2026-05-31 HEAD but exact line numbers can
shift with other commits. `grep -n "save_config(" <file>` to confirm before each edit.

---

### `config/mod.rs` — no changes needed

`save_config` itself (`config/mod.rs:~1340-1355`) uses `crate::fs::save_atomic` (from Story 1.1)
and is correct. The serialization is in the callers, not in `save_config`. Do NOT add a lock
inside `save_config` — that would require `AppState` to be accessible from the config module
(wrong layering).

---

### `dictionary/mod.rs` — out of scope

`dictionary/mod.rs:152` calls `crate::fs::save_atomic` directly (not via `save_config`), and
the dictionary has its own `Mutex<Dictionary>` in AppState. The audit did not flag dictionary
writes as a concurrent-clobber risk. Do NOT add dictionary writes to `config_disk_write` — that
would be scope creep. If a dictionary analog is needed, it warrants its own story.

---

### Test specs to add

Add to `src-tauri/src/commands/settings.rs` (inside the existing `#[cfg(test)]` block):

**Spec A — Concurrent save race: API key survives bar-drag**

```rust
// Scenario: save_settings (new key) and save_bar_position (stale-ish clone) run concurrently.
// With config_disk_write, the full read-modify-write of each is atomic w.r.t. the other.
// After both complete, the final on-disk config must contain the API key.
#[test]
fn test_concurrent_save_settings_and_bar_position_key_survives() {
    // Set up AppState with a known groq_api_key = "".
    // Thread A: acquire disk-write lock, set groq_api_key to "sk-test", save.
    // Thread B: (started before A releases lock) waits for disk-write lock,
    //           reads config (now has "sk-test"), sets bar_x/bar_y, saves.
    // After both: load from disk, assert groq_api_key == "sk-test" AND bar_x == <value>.
    //
    // Implementation hint: use std::sync::Barrier to synchronize thread starts,
    // then call the command implementations directly (bypass Tauri invoke path).
    // Both threads share the same Arc<AppState>.
}
```

**Spec B — `save_advanced_settings` no longer holds config across write**

```rust
// Confirms save_advanced_settings drops the config lock before save_config returns.
// Approach: in a separate thread, try to lock `inner.config` while save_advanced_settings
// is in its disk-write phase. With the fix, the try_lock must succeed (config is free).
// Without the fix (config held across write), try_lock returns WouldBlock.
//
// Simpler approach: just confirm that a concurrent reader of `inner.config` does not
// deadlock when save_advanced_settings is running. Use a short timeout on the lock attempt.
#[test]
fn test_save_advanced_settings_config_not_held_during_write() { ... }
```

**Scope note:** testing real async concurrency for Tauri commands (which need AppHandle) is
complex. If the full async harness is not feasible, implement the serialization logic as a
pure helper function (`save_config_with_lock(state: &AppState, cfg: &AppConfig)`) that CAN be
tested synchronously, then have all Tauri command wrappers delegate to it.

---

### No migration, no UX change

This story adds a Mutex field to AppState (initialized in `AppState::new`) and rewires call
sites. There are no schema changes, no new config fields, no user-visible behavior changes.
The user's only visible change: concurrent saves no longer silently clobber each other.

---

### Dev Agent Record

#### Agent Model Used

claude-sonnet-4-6

#### Debug Log References

#### Completion Notes List

- Added `config_disk_write: Mutex<()>` to `AppState` struct and initializer in `lib.rs`.
- Updated all 17 production `save_config` call sites in `commands/` (settings.rs ×9, misc.rs ×3, license.rs ×4, voice_command.rs ×2) to hold `_disk_guard` across the read-modify-write cycle.
- Fixed `save_advanced_settings` to drop the `config` lock before disk I/O (was incorrectly holding it across the write).
- Fixed `voice_command.rs` stop-path to drop `config` lock before disk I/O (same anti-pattern).
- Added 2 new specs: Spec A (concurrent race — bar-drag cannot erase API key) and Spec B (config lock free during I/O). Both pass.
- `cargo test --lib`: 530 passed, 0 failed (528 pre-existing + 2 new). No new Clippy warnings.

#### File List

- `src-tauri/src/lib.rs`
- `src-tauri/src/commands/settings.rs`
- `src-tauri/src/commands/misc.rs`
- `src-tauri/src/commands/license.rs`
- `src-tauri/src/commands/voice_command.rs`

---

### Review Findings

_Code review 2026-05-31 (bmad-code-review; 3 adversarial layers — Blind Hunter / Edge Case
Hunter / Acceptance Auditor). **Verified clean:** all 6 ACs satisfied; 17 guarded production
call sites confirmed (settings ×9, misc ×3, license ×4, voice_command ×2); lock-ordering
globally consistent (`config_disk_write` → `config`, no inversion anywhere); no re-entrant
self-deadlock; production config.json write-site coverage is complete. Findings below are
hardening, not AC failures._

- [x] [Review][Decision] New specs characterize the lock *pattern* inline, not the real call
  sites — `test_disk_write_lock_serializes_concurrent_saves` and
  `test_advanced_settings_config_not_held_during_disk_write` hand-roll
  `config_disk_write → config → clone → save_config` in the test threads and never invoke
  `save_settings` / `save_bar_position` / `save_advanced_settings`. A future refactor that drops
  a guard from a real command would NOT fail these tests. Also: Spec A pins the schedule
  (Thread A locks before the barrier) so it has no negative control and exercises only one
  ordering; Spec B's `try_lock` assertion is tautological on the barrier sequencing (the reader
  `try_lock`s strictly after the writer's config-drop, which is sequenced-before the writer's
  `barrier.wait()`) and never observes the disk lock. Resolution options: (a) accept
  pattern-characterization tests as-is — invoking the real `#[tauri::command]` async fns needs
  an AppHandle/State harness; (b) extract a shared `save_config_locked(state, |cfg| …)` helper
  that all 17 sites delegate to and that the tests bind to — also closes the "`Mutex<()>`
  invariant is not compile-enforced" gap; (c) minimal — harden Spec B's assertion (observe the
  disk lock, not `config`) + add a symmetric-ordering/negative case, leave the binding gap.
  [blind+edge+auditor]
- [x] [Review][Patch] `save_settings` holds `config_disk_write` across `register_hotkey` +
  `apply_autostart` (OS-registry I/O) [src-tauri/src/commands/settings.rs:448-519] — the
  `_disk_guard` is function-scoped and only drops at return, so the global save-serialization
  mutex stays held across global-shortcut re-registration and the Windows startup-entry write.
  Every other config saver (incl. high-frequency bar-drag — the exact concurrency this story
  addresses) blocks behind that I/O. Fix: block-scope the guard to drop right after the
  in-memory update (line 500), mirroring `set_hotkey_slot` (settings.rs:770-797). No
  correctness/deadlock impact (`register_hotkey` does not take the disk lock); contention only.
  [edge]
- [x] [Review][Patch] `toggle_voice_command_mode` stop-path silently skips the "disabled"
  persist on lock poison [src-tauri/src/commands/voice_command.rs:53] — uses
  `if let Ok(_disk_guard) = inner.config_disk_write.lock()`, so on a poisoned lock the
  safety-net write (`voice_command_enabled = false`) the comment says must always happen is
  dropped with no log. The other 16 sites use `crate::lock!(…)?`. Keep the path best-effort
  (pre-existing fail-soft) but add a `log::warn!` on the skip so it is not silent. Low —
  current blast radius is small (voice-command autostart is dead code behind `if false`).
  [blind+edge+auditor]
- [x] [Review][Defer] Migration writes in `load_config` are unguarded by `config_disk_write`
  [src-tauri/src/config/mod.rs:1162,1211,1240] — deferred, pre-existing. Safe today (boot-only,
  single-threaded, runs before `AppState`/the mutex exists), but the "config.json is only
  written under `config_disk_write`" invariant is undocumented and would break if `load_config`
  ever becomes a runtime "reload" command. Out of this story's scope (config/mod.rs
  intentionally untouched).

**Resolution (2026-05-31, Andi's call — all applied, `cargo test --lib` 531 passed / 0 failed,
clippy clean on touched files):**

- **D1 → Option 3 applied + Option 2 deferred.** Spec B de-tautologized: a two-barrier sync now
  pins the observation window so the reader asserts BOTH `config.try_lock().is_ok()` AND
  `config_disk_write.try_lock().is_err()` while the writer holds the disk guard with `config`
  dropped (it now *observes the disk lock*, not just `config`). Added
  `test_disk_write_lock_serializes_concurrent_saves_bar_first` — the symmetric/negative-control
  ordering (bar-save wins the lock first). **Option 2 (shared `save_config_locked` helper so the
  17 sites bind structurally + the invariant becomes compile-enforced) deferred to a follow-up
  story** — see `deferred-work.md` (Epic 4 candidate; to be formalized via `bmad-create-story`,
  not improvised). Residual until then: a future edit dropping a guard from a real command would
  still not be caught by these tests (production is verified correct *now*).
- **P1 applied.** `save_settings` now `drop(_disk_guard)` immediately after the in-memory update
  (settings.rs:~500), releasing the global save-serialization lock before `register_hotkey` +
  `apply_autostart`. Mirrors `set_hotkey_slot`. No deadlock/correctness change; removes the
  bar-drag-blocks-behind-OS-I/O contention.
- **P2 applied.** voice_command stop-path converted to `match` with `log::warn!` on a poisoned
  `config_disk_write` (and on a poisoned `config`); path stays best-effort/fail-soft but is no
  longer silent.

**Dismissed (1):** Blind Hunter flagged a High "clone-then-drop memory/disk divergence" in
`save_settings`/`clear_api_key` (it had no project access and self-marked it unverifiable).
Refuted — both write the new config back into in-memory `config` **inside** the disk guard
(`settings.rs:500` and `settings.rs:987`). No divergence.
