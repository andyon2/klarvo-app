---
story: "4.3"
epic: "4"
title: "Single sanctioned config-write path (save_config_locked choke-point)"
status: done
findings: ["code-review-1.3-D1"]
gatedBy: ADR-0015
buildsOn: ["1.3"]
scopeFenceException: true
inputDocuments:
  - _bmad-output/planning-artifacts/epics.md
  - docs/adr/0015-state-file-write-convention.md
  - _bmad-output/implementation-artifacts/1-3-single-writer-serialization-for-state-file-saves.md
  - _bmad-output/implementation-artifacts/deferred-work.md
---

# Story 4.3: Single sanctioned config-write path (save_config_locked choke-point)

Status: done

## ⚠️ Scope-fence exception (decided 2026-05-31, Andi)

Epic 4 is deliberately fenced "last, after Epics 1+3" with the explicit rule *"Do NOT
implement any of this epic under remediation time-pressure ahead of the hardening work"*
(epics.md, Epic 4 scope fence). **This story is a conscious, one-off exception to that fence.**
It is pulled forward — ahead of Stories 1.4/1.5 — because it is tightly coupled to Story 1.3:
1.3 just rewired 18 config-save call sites with an identical hand-written lock pattern, so the
context is hot and the sites are uniform. Extracting the choke-point now is far cheaper than
re-loading all 17 sites later. Epic 4's other stories (4.1 DEPTH-config, 4.2 DEPTH-pipeline)
remain deferred per the fence. This exception applies to 4.3 only.

Source of the work: **code review of Story 1.3, decision D1, Option 2** (deferred there, now
pulled forward). See `1-3-…md` Review Findings and `deferred-work.md`.

## Story

As a klarvo maintainer,
I want a single sanctioned function that is the only runtime path to persist `config.json`,
so that the ROB-04 disk-write serialization invariant is enforced by code structure (one
choke-point, the lock impossible to get wrong at a call site) instead of by reviewer vigilance
across 17 hand-written copies — and so the concurrency specs bind to the real production path
instead of characterizing a hand-rolled copy of it.

## Acceptance Criteria

1. **`AppState::save_config_locked` exists** and performs the full ROB-04 cycle in one place:
   acquire `config_disk_write` → acquire `config` → apply caller's `mutate` closure → clone
   snapshot → drop `config` → write snapshot to disk under the still-held disk-write lock →
   return the snapshot. The `config` lock is never held across disk I/O. Signature:
   `pub fn save_config_locked(&self, context: &str, mutate: impl FnOnce(&mut AppConfig)) -> Result<AppConfig, String>`.

2. **All 18 production config-save call sites route through `save_config_locked`.** Every
   production `save_config` call in `commands/` (settings ×9, misc ×3, license ×4,
   voice_command ×2 = **18**) is replaced by a `save_config_locked` call. No production command
   holds `config_disk_write`/`config` by hand for a config save anymore.
   (Note: Story 1.3's notes said "17" — an arithmetic slip; 9+3+4+2 = 18. The work was correct,
   only the prose count was off. Corrected here.)

3. **`config::save_config` is demoted to `pub(crate)`** and documented as the boot-only
   low-level writer. The only direct callers left are the single-threaded boot/migration sites
   (`config/mod.rs` migrations, `lib.rs` first-install) and test fixtures — all in-crate.

4. **No behavior change.** Same fields persisted, same in-memory updates, same provider/hotkey
   re-registration ordering, same early-return validation in `clear_api_key`, same best-effort
   fail-soft + warn in the voice_command stop-path. Error-message wording is unified to
   `"Failed to persist {context}: {e}"` (cosmetic; documented).

5. **The concurrency specs bind to the real helper.** The Story-1.3 specs (which hand-rolled
   the lock dance inline) are rewritten to call `state.save_config_locked(...)` directly, so a
   future edit that drops the lock from the helper makes them fail. Coverage:
   (a) concurrent key-save vs bar-save → both survive (forward),
   (b) symmetric ordering (reverse),
   (c) `save_config_locked` updates in-memory config AND disk coherently to the same snapshot.

6. **`cargo test` green; `cargo clippy` clean on touched files.** Now-unused `save_config`
   imports removed from `misc.rs`/`license.rs`/`voice_command.rs`.

## DoD

- **Linux (load-bearing):** `cargo test --lib` passes (incl. the rebound concurrency specs).
  `cargo clippy` clean on touched files (repo-wide clippy pre-existing red — touched-files-clean
  is the bar, escalation rule A6).
- **No Windows smoke required.** Pure-Rust refactor: a closure-based helper + call-site
  rewiring + a visibility narrowing. No new FS primitives, no platform paths, no change to the
  write mechanism (`save_atomic`, verified on Windows in Story 1.1). Same class as Story 1.3,
  which carried the same no-smoke rationale.

## Dev Notes

Grounded against HEAD (`v1-ship`, after Story 1.3 commit `21e3ec4`). Rule codes reference
`docs/bmad-autopilot-escalation-contract.md`.

### The helper (`src-tauri/src/lib.rs`, top of `impl AppState`, before `new`)

```rust
pub fn save_config_locked(
    &self,
    context: &str,
    mutate: impl FnOnce(&mut AppConfig),
) -> Result<AppConfig, String> {
    let _disk_guard = crate::lock!(self.config_disk_write)?;
    let snapshot = {
        let mut cfg = crate::lock!(self.config)?;
        mutate(&mut cfg);          // deref-coerces MutexGuard<AppConfig> -> &mut AppConfig
        cfg.clone()
    };                              // config lock dropped; disk guard still held
    crate::config::save_config(&self.app_data_dir, &snapshot)
        .map_err(|e| format!("Failed to persist {context}: {e}"))?;
    Ok(snapshot)
}
```

Returns the persisted snapshot so callers that re-resolve providers from the new config
(`save_settings`, `update_api_keys`, `clear_api_key`) use the return value instead of re-locking.

### Why this also subsumes the Story-1.3 P1 fix

`save_settings` previously held the disk guard across `register_hotkey` + `apply_autostart`
(the 1.3 review P1, fixed there with an explicit `drop(_disk_guard)`). With the helper, the
lock lives *only* inside `save_config_locked`, so it is released before any post-save
provider/hotkey/autostart work **by construction** — the explicit drop is no longer needed.

### Irregular sites

- **`save_settings`**: validation stays before the call; closure does `*cfg = merge_settings(cfg.clone(), patch)`; providers resolved from the returned snapshot; hotkey + autostart unchanged, now outside the lock automatically.
- **`clear_api_key`**: provider name validated up front (returns the original error before any lock); closure clears the matched field; providers hot-reloaded from the returned snapshot.
- **voice_command stop-path**: `if let Err(e) = inner.save_config_locked(…) { log::warn!(…) }` — keeps the best-effort fail-soft + the 1.3 P2 warn-on-poison, now in one line.

### Enforcement: helper + `pub(crate)`, NOT a token witness (assessed, rejected)

A type-level token (`save_config(dir, cfg, &DiskWriteGuard)`) would make a missing lock a
*compile* error — strictly stronger. **Rejected as disproportionate:** `save_config` has 57
call sites (≈34 in tests/fixtures + 5 boot/migration sites that run before `AppState` exists).
A token param would churn all 57 and force a boot-only `unchecked` escape hatch. The
helper-plus-`pub(crate)` approach centralizes the discipline, makes the correct path the
obvious one, makes any direct `save_config` call trivially greppable in review, and binds the
tests to the real path — at a fraction of the blast radius. If a token is ever wanted, it is a
clean follow-up on top of this choke-point.

### Out of scope

- `dictionary/mod.rs` writes (own mutex; Story 1.3 scoped out; unchanged).
- The boot/migration `save_config` calls in `config/mod.rs` + `lib.rs` stay direct (the
  documented single-threaded boot exception — see deferred W1 from the 1.3 review). Routing
  those through the helper is impossible (no `AppState` yet) and unnecessary (no concurrency).

### Dev Agent Record

#### Agent Model Used

claude-opus-4-8 (1M)

#### Completion Notes List

- Added `AppState::save_config_locked(context, mutate)` (lib.rs) — the single runtime config-write
  path; returns the persisted snapshot. Demoted `config::save_config` to `pub(crate)` with a
  boot-only doc-contract.
- Routed all **18** production sites through it (settings ×9, misc ×3, license ×4, voice_command ×2).
  Corrected the Story-1.3 "17" miscount (9+3+4+2 = 18). Irregular sites handled as designed:
  `save_settings` (validation before, providers/hotkey/autostart after — the helper auto-subsumes
  the 1.3 P1 drop), `clear_api_key` (provider validated before the lock), voice_command stop-path
  (one-line `if let Err(e) { warn }`, keeps the 1.3 P2 fail-soft warn).
- Removed now-unused `save_config` imports from misc.rs/license.rs/voice_command.rs; settings.rs
  keeps it for test fixtures.
- Rebound the concurrency specs to the real helper: `test_save_config_locked_serializes_concurrent_saves`
  (forward), `..._three_way` (16-round, 3-racer negative control), `..._updates_memory_and_disk_coherently`
  (helper contract). The old hand-rolled specs (and the tautological Spec B) are gone.
- `cargo test --lib`: **531 passed, 0 failed**. `cargo clippy --lib`: clean on all touched files.
  Reroute verified complete (0 production `save_config` left in `commands/`; only test fixtures +
  boot/migration call it directly).
- Error-message wording unified to `"Failed to persist {context}: {e}"` (cosmetic; AC4).

#### File List

- `src-tauri/src/lib.rs`
- `src-tauri/src/config/mod.rs`
- `src-tauri/src/commands/settings.rs`
- `src-tauri/src/commands/misc.rs`
- `src-tauri/src/commands/license.rs`
- `src-tauri/src/commands/voice_command.rs`

---

### Review Findings

_Code review 2026-05-31 (same-session, 3 adversarial subagent layers — Blind Hunter / Edge Case
Hunter / Acceptance Auditor — on the 4.3 refactor diff vs commit `21e3ec4`). **Verified clean:**
helper correct; all 18 sites semantics-preserving; no reentrancy / self-deadlock; lock-ordering
globally consistent (`config_disk_write` → `config`, no inversion); no runtime reroute miss;
boot/migration bypass legitimate; AC1–AC5 satisfied; count = 18 confirmed; all three Dev-Notes
claims (token rejection, P1-subsumption, fence-exception) hold._

- [x] [Review][Patch] Unused `lock` import introduced by the reroute [src-tauri/src/commands/voice_command.rs:10]
  — APPLIED. The file's last `lock!` callers were replaced by `save_config_locked`, leaving
  `use crate::{AppState, lock}` with a dead `lock`. Narrowed to `use crate::AppState;`. (AC6 — my
  earlier clippy grep missed it: clippy prints the filename on a separate line from `warning:`, so
  a single-line filter dropped it. Caught by Edge Case Hunter + Acceptance Auditor.)
- [x] [Review][Patch] Dead `if false` voice-command-autostart block would bypass ROB-04 if re-enabled
  [src-tauri/src/lib.rs:~862] — APPLIED. Added a `TODO(ROB-04)` noting that a re-enable must route
  the reset through `save_config_locked` instead of the hand-written `config`-held-across-`save_config`
  pattern. (Edge Case Hunter latent note.)
- [x] [Review][Patch] 3-way concurrency test docstring overclaimed determinism
  [src-tauri/src/commands/settings.rs] — APPLIED. Reworded to state it is a *probabilistic*
  negative control (the `config` mutex serializes the in-memory mutations, so a single round can
  pass even without `config_disk_write`; 16 rounds make a regression *likely*, not *certain*, to
  surface). (Blind Hunter Low.)
- [x] [Review][Won't-fix, documented] Lock-discipline properties ("config not held across disk I/O";
  "disk lock held across the write") no longer have a dedicated behavioral assertion (Blind Hunter,
  Medium). **Rationale:** with the single 8-line `save_config_locked`, this property is *structural*
  — `config` lives in an inner block that ends before `save_config` runs, auditable at a glance.
  The old Story-1.3 Spec B asserted it against a **hand-rolled copy** of the lock dance, not the
  production path, so its "coverage" of production was illusory — re-adding that style would
  re-introduce exactly the non-binding-copy anti-pattern this story (D1) set out to eliminate. A
  behavioral test of "config free *during* the disk write" requires hooking/​slowing the write,
  which is not feasible without instrumenting production. Net production coverage is unchanged; the
  property moved from "tested on a copy" to "structural + reviewed." Accepted as a conscious trade.

**Dismissed / pre-existing (not introduced by 4.3, left per escalation rule A6):** `unused import:
LicenseStatus` (license.rs:21, lib.rs:71) and `map_or can be simplified` (misc.rs:299) are
pre-existing baseline-red on touched files — confirmed outside the 4.3 diff. 4.3 introduces **zero**
new clippy warnings after the `lock`-import fix. Informational deltas confirmed benign by all
layers: `resolve_providers` now runs just outside the disk lock (pure, infallible, no race); the
voice_command poison-log collapsed two messages into one (still warns — Acceptance Auditor noted it
now *also* surfaces disk-write failures, a net improvement).
