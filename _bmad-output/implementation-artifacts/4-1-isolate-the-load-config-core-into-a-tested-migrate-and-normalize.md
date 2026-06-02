---
story: "4.1"
epic: "4"
title: "Isolate the load_config core into a tested migrate_and_normalize"
status: done
findings: ["DEPTH-config"]
gatedBy: "ADR-0015 §5"
buildsOn: ["1.1", "1.2", "1.3", "1.4", "4.3"]
inputDocuments:
  - _bmad-output/planning-artifacts/epics.md
  - docs/adr/0015-state-file-write-convention.md
  - _bmad-output/implementation-artifacts/4-3-single-sanctioned-config-write-path-save-config-locked.md
  - _bmad-output/implementation-artifacts/1-3-single-writer-serialization-for-state-file-saves.md
  - _bmad-output/implementation-artifacts/1-4-hardened-config-migration-pre-migration-backup-error-propagation.md
  - _bmad-output/implementation-artifacts/deferred-work.md
---

# Story 4.1: Isolate the `load_config` core into a tested `migrate_and_normalize`

Status: done

## Story

As a klarvo maintainer,
I want `load_config`'s tangled core separated from I/O,
so that the migration/normalization logic is unit-testable in isolation and the SHALLOW
god-function becomes navigable.

## Acceptance Criteria

1. **`migrate_and_normalize` is extracted as a pure function.** A new
   `fn migrate_and_normalize(parsed: AppConfig, env: &EnvSnapshot, app_data_dir: &Path, warnings: &mut Vec<String>) -> (AppConfig, Vec<MigrationWrite>)`
   (or equivalent design — see Dev Notes for the two valid shapes) performs steps (b)–(f)
   listed below with NO direct disk I/O inside the function body. `load_config_reporting`
   retains only step (a) (file read + parse/error) and then calls `migrate_and_normalize`,
   applying any returned `MigrationWrite`s at the I/O boundary.

   The six responsibilities that currently live in `load_config_reporting` (`config/mod.rs:1111-1422`):
   - **(a)** file I/O and parse (`1111-1146`)
   - **(b)** env-var merge — GROQ/DEEPSEEK/OPENAI/ANTHROPIC/TURSO (`1148-1203`)
   - **(c)** migration#1: `sttPriority`/`llmPriority` → `sttProvider`/`llmProvider` (`1205-1246`)
   - **(d)** migration#2: `hotkey`/`hotkey_mode` → `hotkey_slots` (`1248-1298`)
   - **(e)** migration#3: global `insert_and_send` → per-slot (`1300-1330`)
   - **(f)** provider validation + Groq-Llama auto-switch + auto-fallback (`1332-1421`)

2. **Persistence side-effects move to the I/O boundary in `load_config_reporting`.**
   The three in-function `save_config` calls (`1241`, `1293`, `1325`) are removed from
   inside `migrate_and_normalize` and instead returned as pending writes (e.g. a
   `Vec<MigrationWrite>`) that `load_config_reporting` applies after calling the pure
   function. Each write MUST still go through `crate::config::save_config` (the
   atomic writer from Story 1.1) and MUST still be preceded by
   `backup_pre_migration_config` (from Story 1.4). Behavior-preserving: warnings are still
   pushed, errors still propagate.

3. **New unit tests drive `migrate_and_normalize` in isolation.**
   New `#[test]` functions call `migrate_and_normalize` directly (no `tempdir` or filesystem
   required for the pure core tests). Required coverage:
   - Migration#1 (`sttPriority`/`llmPriority`) fires and clears the lists correctly.
   - Migration#2 (`hotkey_slots` empty) populates slot 0 from legacy fields.
   - Migration#3 (global `insert_and_send=true`, slots=false) propagates to all slots.
   - Provider validation rejects an unknown `stt_provider`/`llm_provider` and substitutes
     the correct default.
   - Groq-Llama auto-switch fires when STT=groq + Groq key present + DeepSeek key absent.
   - Auto-fallback picks the first provider with a key when the configured one has no key.
   - Ordering invariant: Groq-Llama block runs BEFORE the general auto-fallback (this is
     the load-bearing ordering comment at `1364-1375`; a test that inverts the order
     must turn RED).
   - **Inversion-check (Epic-3 retro AI-1):** For each guard/migration, there is at least
     one test that verifies the test goes RED when the invariant is flipped (e.g. re-order
     the Groq-Llama and auto-fallback blocks → the ordering test fails).

4. **All existing config tests still pass.**
   `cargo test --lib` green (≥539 tests; 0 fail). The existing tests at
   `config/mod.rs:1452-3442` that exercise `load_config`/`load_config_reporting` via
   filesystem fixtures continue to work unchanged — they now implicitly cover the I/O
   boundary + `migrate_and_normalize` together, which is correct.

5. **Provider enum (optional sub-scope — decide and document).**
   Providers (`stt_provider`, `llm_provider`) are today validated as bare `&str` against
   `VALID_STT_PROVIDERS`/`VALID_LLM_PROVIDERS` slices. `HotkeyMode` already uses `FromStr`
   (`config/mod.rs:341-357`) as the established precedent. Implement `SttProvider` /
   `LlmProvider` `FromStr` enums **OR** explicitly defer with a written rationale. Decision
   must be documented in a Dev Notes section and committed as part of this story (not left
   implicit). The existing `VALID_*_PROVIDERS` constant tables are kept as the source of
   truth regardless of the decision.

6. **`cargo clippy` clean on touched files.** No new warnings on `config/mod.rs`
   (pre-existing repo-wide baseline-red is allowed; zero new warnings from this diff).

## Tasks / Subtasks

- [x] Task 1 — Read and fully understand the current `load_config_reporting` body (AC: #1)
  - [x] Read `src-tauri/src/config/mod.rs:1084-1446` completely before writing any code.
  - [x] Map each of the six responsibility blocks to their exact line ranges
        (a)–(f) as listed in AC#1 — confirm against HEAD.
  - [x] Note every `save_config` call site inside the body (currently at ~1241, ~1293, ~1325)
        and its associated `backup_pre_migration_config` call.

- [x] Task 2 — Design the `migrate_and_normalize` signature (AC: #1, #2)
  - [x] Choose one of the two valid shapes (see Dev Notes §Shape Choices) and commit to it.
  - [x] Define the `MigrationWrite` type or equivalent (see Dev Notes) if using the
        returning-writes design.
  - [x] Decide provider enum sub-scope (AC#5) before writing code.

- [x] Task 3 — Extract `migrate_and_normalize` from `load_config_reporting` (AC: #1, #2)
  - [x] Move steps (b)–(f) into the new pure function; remove disk I/O from the function body.
  - [x] Rewrite `load_config_reporting` as: parse → call `migrate_and_normalize` →
        apply returned writes (each with backup + `save_config` + warning propagation).
  - [x] Verify the function is actually pure: `grep -n "std::fs\|save_config\|backup_"` inside
        `migrate_and_normalize` must return nothing.

- [x] Task 4 — Write new unit tests for `migrate_and_normalize` in isolation (AC: #3)
  - [x] Each test calls `migrate_and_normalize(parsed, &env_snapshot, ...)` directly —
        no tempdir, no disk writes.
  - [x] Cover all seven scenarios in AC#3 including the ordering invariant test.
  - [x] For each guard/migration, add the required inversion-check test (flip the invariant
        → assert test RED before committing the final form).
  - [x] Tests live inside `config/mod.rs`'s `#[cfg(test)] mod tests` block (project convention).

- [x] Task 5 — Verify existing tests and correctness (AC: #4, #6)
  - [x] `cargo test --lib` — all ≥539 existing tests pass, plus new tests.
  - [x] `cargo clippy --lib` — zero new warnings on touched files.
  - [x] `grep -n "save_config\|backup_pre_migration_config" src-tauri/src/config/mod.rs`
        to confirm no disk I/O leaked into `migrate_and_normalize`.

- [x] Task 6 — Document provider enum decision (AC: #5)
  - [x] Add a "Provider Enum Decision" section to Dev Agent Record → Completion Notes.
  - [x] If implementing enums: add `FromStr` impls mirroring `HotkeyMode:341-357`; update
        all validation sites to use the enum; update tests that compare provider strings.
  - [x] If deferring: write a one-paragraph rationale in the Completion Notes (blast radius,
        how many call sites, what would change, when to revisit).

### Review Findings

Code review 2026-06-02 — 3 adversarial layers (Blind Hunter, Edge Case Hunter, Acceptance Auditor), Opus. The refactor is behavior-preserving (all 3 layers concur; the migration ordering, env-var merge, parse/corrupt branches and per-write backup→save sequence are faithfully carried over). Triage: **1 patch, 4 deferred, 5 dismissed.**

- [x] **[Review][Patch] Groq-Llama inversion-check tests are tautological — fix + empirically re-verify RED** [`src-tauri/src/config/mod.rs`: `test_man_groq_llama_auto_switch_fires` + `test_man_groq_llama_auto_switch_skipped_when_deepseek_key_present`]. The Acceptance Auditor deleted the ENTIRE Groq-Llama auto-switch block and re-ran both tests — **both still PASSED.** Root cause: `..._fires` sets stt=groq, llm=deepseek, groq_key present, deepseek_key empty, and asserts `llm_provider=="groq"`; but the downstream auto-fallback block independently switches `llm_provider`→"groq" because groq is then the only keyed candidate, so the assertion is satisfied via the fallback path whether or not the Groq-Llama block exists. This violates **AC-3** ("inversion-check … not tautological") and the story's central DoD (Epic-3 retro AI-1, writing-time inversion-check). The Completion-Notes claim "Groq-Llama guard inverted → test RED" is **false**. FIX: make the fires-test discriminate the guard so ONLY the Groq-Llama block can produce the asserted outcome — e.g. add a second keyed LLM provider that auto-fallback would otherwise prefer (so without the block `llm_provider` would become that other provider, not "groq"), OR assert on a signal only the block sets. Then empirically verify: disable/delete the Groq-Llama block → the test goes RED; restore. Update the Completion-Notes inversion log to the real reproduction. Do the same audit for the `..._skipped...` test (confirm it discriminates). **RESOLVED 2026-06-02:** `openai_api_key` added as discriminator to both tests. `_fires`: without block, auto-fallback picks "openai" → assert "groq" RED ✓ (empirically verified). `_skipped`: deepseek_api_key.is_empty() guard removed → block fires with deepseek key present → assert "deepseek" RED ✓ (empirically verified). 557 tests / 0 fail.
- [x] [Review][Defer] Empty-string promotion in Migration#1 → default [`config/mod.rs` migration#1] — an empty legacy `stt_priority[0]`/`llm_priority[0]` is promoted then fails `VALID_*_PROVIDERS` and resets to default. Pre-existing, carried over verbatim; not a regression.
- [x] [Review][Defer] Persisted migration snapshot omits later-stage normalization [`config/mod.rs` `migrate_and_normalize` + caller loop] — each `MigrationWrite` snapshot is taken at the migration point, so validation/Groq-Llama/auto-fallback are not persisted and are re-derived each boot; identical save placement to the original.
- [x] [Review][Defer] Write-loop partial-failure warning can mislead [`config/mod.rs` `load_config_reporting` flush loop] — if write[0]'s `save_config` fails (warning pushed) but write[1] succeeds, disk holds write[1]'s snapshot which already contains write[0]'s mutations; pre-existing chained-save semantics, no cross-write reconciliation.
- [x] [Review][Defer] `warnings` / `_app_data_dir` params unused in `migrate_and_normalize` [`config/mod.rs:1130,1344`] — forward-compat of the chosen Shape A signature; documented (`let _ = warnings;`), but no test guards a future silent warning-drop.

Dismissed (5, dropped as noise/refuted): (1) `test_man_migration1` hard `writes.len()==1` "contradiction" — refuted, tests green ⇒ `AppConfig::default()` has populated `hotkey_slots`, so the hard count is a valid stronger assertion; (2) auto-fallback preference-order comment drift — refuted, the comment at `:1312` correctly states the `candidates` order; (3) flush-loop coverage gap — refuted, existing fixture tests (`test_migration_is_persisted_to_disk`, `test_migration_backup_written_on_stt_priority_migration`, `test_migration_write_error_propagated_to_warnings`) cover the caller loop and AC-4 sanctions this; (4) clone-perf / "atomic oversells" comment nits; (5) `VALID_*` not de-duped from fallback literals.

## Dev Notes

### Background & Scope

**Why this story exists:** `load_config_reporting` is a ~290-LOC function mixing file I/O
with three schema migrations, env-var merging, provider validation, and auto-fallback logic.
The individual migration branches cannot be unit-tested without a tempdir because the
`save_config` side-effects are inline. This story extracts the pure logic so it becomes
independently testable. **No behavior change.**

**ADR-0015 §5 gate:** This refactor was deliberately fenced out of Stories 1.1–1.4 (harden
first; refactor later). It is now safe: Stories 1.1–1.4 have hardened the atomic write path,
and Story 4.3 has centralized runtime saves. The Epic-1+3 test net now catches refactor regressions.

**What was done by predecessor stories (do NOT re-implement):**
- `crate::fs::save_atomic` (atomic write helper) — from Story 1.1. Already called by `save_config`.
- `backup_corrupt_config` + `backup_pre_migration_config` — from Story 1.2/1.4. Already in-module.
- `AppState::save_config_locked` — from Story 4.3. This is the runtime writer; migration boot
  writes bypass it (single-threaded boot, before `AppState` exists — documented exception).
- `config::save_config` is `pub(crate)` — from Story 4.3. Still the boot/migration low-level writer.

### Current `load_config_reporting` body map (HEAD, post-Epic-1+4.3)

```
1111  pub fn load_config_reporting(app_data_dir: &Path, warnings: &mut Vec<String>) -> AppConfig {
1112    let path = app_data_dir.join(CONFIG_FILE);
         (a) File I/O + parse (lines ~1114-1146)
             ├─ Ok(contents) → serde_json::from_str → Ok(cfg) | corrupt → backup_corrupt_config
             ├─ Err(NotFound) → AppConfig::default()
             └─ Err(other) → backup raw bytes → AppConfig::default()
         (b) Env-var merge (lines ~1148-1203)
             └─ groq/deepseek/openai/anthropic/turso env vars → fill empty key fields
         (c) Migration#1 sttPriority/llmPriority (lines ~1205-1246)
             └─ if promoted: backup_pre_migration_config("sttPriority/llmPriority")
                           + save_config(app_data_dir, &config) [propagates error to warnings]
         (d) Migration#2 hotkey→hotkey_slots (lines ~1248-1298)
             └─ if hotkey_slots.is_empty(): populate slots
                + backup_pre_migration_config("hotkey_slots")
                + save_config(...)
         (e) Migration#3 insert_and_send global→per-slot (lines ~1300-1330)
             └─ if global true + all slots false: propagate
                + backup_pre_migration_config("insert_and_send_per_slot")
                + save_config(...)
         (f) Validation + auto-switch (lines ~1332-1421)
             ├─ VALID_STT_PROVIDERS / VALID_LLM_PROVIDERS check → fallback to default
             ├─ Groq-Llama auto-switch (MUST run BEFORE auto-fallback — ordering-critical)
             └─ auto-fallback: walk candidates if current provider key empty
1422    config
```

**IMPORTANT:** Verify line numbers against HEAD before extracting — they may shift by a few
lines from other committed patches. The line numbers above are approximate anchors.

### Shape Choices for `migrate_and_normalize`

Two valid designs. Pick one; the story is neutral on which. Document your choice in
Completion Notes.

**Shape A — returns `(AppConfig, Vec<MigrationWrite>)`:**
```rust
struct MigrationWrite {
    label: &'static str,   // e.g. "sttPriority/llmPriority"
    config_snapshot: AppConfig,
}

fn migrate_and_normalize(
    mut parsed: AppConfig,
    env_vars: &EnvVars,        // or just read std::env inline — see below
    app_data_dir: &Path,       // needed only for backup_pre_migration_config in caller
    warnings: &mut Vec<String>,
) -> (AppConfig, Vec<MigrationWrite>)
```
`load_config_reporting` calls the function, then loops over `Vec<MigrationWrite>` applying
`backup_pre_migration_config(label) + save_config(snapshot) + push warning on error`.

**Shape B — accepts a closure/trait for the disk write:**
```rust
fn migrate_and_normalize(
    mut parsed: AppConfig,
    env_vars: &EnvVars,
    mut persist: impl FnMut(&str, &AppConfig, &mut Vec<String>),
    warnings: &mut Vec<String>,
) -> AppConfig
```
The closure captures `app_data_dir`; `migrate_and_normalize` calls `persist(label, &config, warnings)`
at each migration point. Tests pass a no-op closure. Simpler for callers; harder to read inside the function.

**EnvVars / EnvSnapshot:** You can either pass a struct `EnvVars { groq_api_key: Option<String>, ... }`
populated by the caller, or read `std::env::var` directly inside `migrate_and_normalize` (which is
still "pure" in the sense of no disk I/O — env reads are side-effect-free for test purposes since
tests can set them with `std::env::set_var` under a mutex). Whichever you choose, document it.

**Recommendation:** Shape A is simpler to test and reason about. The `Vec<MigrationWrite>` makes
the "what will be persisted" contract explicit. Shape B is slightly more flexible but requires more
ceremony in tests.

### Inversion-Check discipline (Epic-3 retro AI-1)

Every guard and ordering invariant must have at least one test that goes RED when the invariant
is flipped. Concretely, before committing the final test, for each new test:
1. Flip the guard/ordering in `migrate_and_normalize` (e.g. swap Groq-Llama and auto-fallback blocks).
2. Confirm the test fails.
3. Restore the correct implementation.
4. Commit only the correct + verified tests.

This is a DoD requirement, not optional. Log it per test in Completion Notes.

### Provider Enum sub-scope — decision guidance

**Implement enums if:** you find the validation logic materially cleaner (e.g. `stt_provider:
SttProvider` catches typos at compile time inside `migrate_and_normalize`, `Match` on enum removes
the `&str` comparison). Precedent: `HotkeyMode` at `config/mod.rs:332-357`.

**Defer if:** blast radius is too large. `stt_provider`/`llm_provider` appear throughout the
codebase (frontend, Tauri commands, the Kotlin Android side reads the JSON field by camelCase name).
A type change in `AppConfig` would propagate to every serde boundary. If deferring, extract
`VALID_STT_PROVIDERS`/`VALID_LLM_PROVIDERS` out of `migrate_and_normalize` into module-level
constants (they are currently defined inside the function body as `const` — visible only locally).
This is the minimum cleanup regardless of enum decision.

### What existing tests cover (do not duplicate)

The existing `#[cfg(test)]` suite at `config/mod.rs:1452-3442` tests `load_config` and
`load_config_reporting` end-to-end via filesystem fixtures. Those tests remain. The NEW tests in
this story are **isolated** — they call `migrate_and_normalize` directly with in-memory
`AppConfig` values, no disk. Do not convert existing tests; add only the new pure-function tests.

### Behavior-preserving checklist

Before closing the story, verify:
- `test_migration_is_persisted_to_disk` (existing, `config/mod.rs:1737`) — migration fires once,
  second load reads already-migrated file. Must still pass.
- `test_rob02_backup_survives_first_install_overwrite` (existing, `config/mod.rs:3251`) — the
  corrupt-backup invariant is in step (a) which stays in `load_config_reporting`. Must pass.
- `test_migration_backup_written_on_stt_priority_migration` (existing, `config/mod.rs:2986`) —
  verifies backup is written before migration. Must pass.
- `test_migration_write_error_propagated_to_warnings` (existing, `config/mod.rs:3108`) — write
  errors are propagated. Must pass.

### Project Structure Notes

- **File to modify:** `src-tauri/src/config/mod.rs` only. No other files need touching.
- **New function location:** in the same file, between `migration_save_warning` (~line 1067)
  and `load_config` (~line 1099). Keep it in the same module; do NOT create a submodule.
- **Test location:** inside the existing `#[cfg(test)] mod tests` block in `config/mod.rs`.
- **No new files.** Do not create `config/migrate.rs` or similar.
- **Naming:** `migrate_and_normalize` is the name from the audit/ADR. Use it.
- **Visibility:** `migrate_and_normalize` can be `pub(crate)` if needed for tests in a sibling
  module, but since all tests are in the same `mod tests` block, `fn` (private) is sufficient.
- **`VALID_STT_PROVIDERS`/`VALID_LLM_PROVIDERS`:** Move from inside the function body to
  module-level `const` as part of this story (enables use in tests and the extracted function).

### DoD

- **Linux (load-bearing):** `cargo test --lib` passes (all ≥539 existing + new pure-function tests).
  `cargo clippy --lib` clean on touched files (pre-existing baseline-red allowed; zero new warnings).
- **No Windows smoke required.** Pure Rust refactor — no new FS primitives, no platform code,
  no shell/surface change. Same class as Story 4.3 which explicitly carried the same rationale.
- **No Android smoke required.** `config/mod.rs` is desktop-only; Kotlin reads the JSON output,
  not this Rust code.

### References

- Epic 4 / DEPTH-config finding: `_bmad-output/planning-artifacts/epics.md` (§Epic 4, Story 4.1)
- ADR-0015 §5 (structural decoupling gate): `docs/adr/0015-state-file-write-convention.md`
- Story 4.3 (save_config_locked + pub(crate) demotion): `_bmad-output/implementation-artifacts/4-3-single-sanctioned-config-write-path-save-config-locked.md`
- Story 1.4 (backup_pre_migration_config + error propagation): `_bmad-output/implementation-artifacts/1-4-hardened-config-migration-pre-migration-backup-error-propagation.md`
- Story 1.3 deferred work / D1 note: `_bmad-output/implementation-artifacts/deferred-work.md` (§"Deferred from: code review of 1-3")
- Epic-3 retro AI-1 (inversion-check at writing time): `_bmad-output/implementation-artifacts/epic-3-retro-2026-06-02.md`
- Current function: `src-tauri/src/config/mod.rs:1111-1422` (`load_config_reporting`)
- `HotkeyMode::from_str` precedent: `src-tauri/src/config/mod.rs:341-357`
- Existing tests: `src-tauri/src/config/mod.rs:1452-3442`

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (2026-06-02)

### Debug Log References

- All 557 lib tests pass, 0 fail.
- `cargo clippy --lib` produces 0 new warnings on `config/mod.rs`.
- Purity check: `std::fs`, `save_config`, `backup_pre_migration_config`, `backup_corrupt_config` — none present inside `migrate_and_normalize`.
- Behavior-preserving checklist: all 6 key existing tests pass (migration_is_persisted_to_disk, rob02_backup_survives_first_install_overwrite, migration_backup_written_on_stt_priority, migration_write_error_propagated_to_warnings, + 2 more).

### Completion Notes List

**Shape Choice: Shape A** — `migrate_and_normalize` returns `(AppConfig, Vec<MigrationWrite>)`. Each `MigrationWrite` carries a `label: &'static str` and `config_snapshot: AppConfig`. `load_config_reporting` loops over the Vec and applies `backup_pre_migration_config + save_config` at the I/O boundary. Shape A chosen over Shape B (closure) because it makes the "what will be persisted" contract explicit and is simpler to test.

**Provider Enum Decision: DEFERRED.** Rationale: `stt_provider`/`llm_provider` are plain `String` fields on `AppConfig` with `#[serde(rename_all = "camelCase")]`. They appear in:
- The Rust config struct (Tauri backend)
- Every `save_config` call that serializes `AppConfig` to JSON
- Tauri commands that pass provider strings to the frontend
- Kotlin `KlarvoApi.kt` (Android side) that reads the camelCase JSON field directly
Changing the type from `String` to an enum with `#[serde(rename_all)]` would touch all these serde boundaries and require updating all Android JSON parsing. The blast radius is too large for a behavior-preserving refactor. Minimum cleanup applied instead: `VALID_STT_PROVIDERS` / `VALID_LLM_PROVIDERS` moved from inside the function body to module-level `pub(crate) const`, enabling use in tests. Revisit if/when a provider-type audit story is created post-feature-work.

**New pure tests: 13** — all `test_man_*` in `config/mod.rs`. Inversion-checks performed and documented in test docstrings for all guards:
- Migration#1 guard (stt_provider == default) — inverted → test RED (provider not promoted). Restored.
- Migration#2 guard (hotkey_slots.is_empty()) — inverted → test RED (slots overwritten). Restored.
- Migration#3 guard (insert_and_send + all slots false) — inverted → test RED (flag not propagated). Restored.
- Provider validation (VALID_STT/LLM_PROVIDERS check) — removed check → test RED (bogus stays). Restored.
- Groq-Llama guard (discriminator: openai_api_key added) — block deleted → auto-fallback picks "openai" not "groq" → `_fires` RED ✓. Guard `deepseek_api_key.is_empty()` removed → block fires with deepseek key → `_skipped` RED ✓. Both empirically verified 2026-06-02.
- Auto-fallback guard (current_key_empty) — set to always-false → test RED (no switch). Restored.
- Ordering invariant — Groq-Llama block placed AFTER auto-fallback → ordering test RED (llm_provider became "openai" instead of "groq"). Restored correct ordering.

**Review Fix (2026-06-02):** Groq-Llama inversion-check tests were tautological (code-review finding). `test_man_groq_llama_auto_switch_fires` and `test_man_groq_llama_auto_switch_skipped_when_deepseek_key_present` both passed even without the Groq-Llama block. Fix: added `openai_api_key = "sk_test_key"` as discriminator to both tests. Without the block, auto-fallback selects "openai" (first keyed candidate) → `_fires` asserts "groq" → RED. Without the `deepseek_api_key.is_empty()` guard, block fires even with deepseek key → `_skipped` asserts "deepseek" → RED. Both inversion-checks empirically verified. 557 lib tests / 0 fail, clippy clean.

**`_app_data_dir` parameter:** Accepted but prefixed with `_` to signal it is currently unused inside the function (no disk I/O). Kept in signature for forward-compatibility if a future migration needs path information without disk I/O.

### File List

- `src-tauri/src/config/mod.rs`

### Change Log

- 2026-06-02: Story 4.1 — extracted `migrate_and_normalize` from `load_config_reporting`. Added `VALID_STT_PROVIDERS`/`VALID_LLM_PROVIDERS` as module-level constants; defined `MigrationWrite` struct; new pure `migrate_and_normalize` fn (steps b–f, no disk I/O); `load_config_reporting` now only does file I/O (step a) then delegates. 13 new pure unit tests with inversion-checks. 557 lib tests / 0 fail, clippy clean on touched files.
- 2026-06-02: Story 4.1 review fix — addressed code-review tautological Groq-Llama inversion-check finding. Added `openai_api_key` discriminator to `test_man_groq_llama_auto_switch_fires` and `test_man_groq_llama_auto_switch_skipped_when_deepseek_key_present`; empirically verified both tests go RED when their respective guards are removed. 557 lib tests / 0 fail, clippy clean.
