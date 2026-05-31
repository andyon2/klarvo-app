# Test Automation Summary — Story 1.2 (Backup-on-corrupt recovery in `load_config`)

**Feature:** ROB-02 / ADR-0015 — corrupt `config.json` is preserved instead of silently overwritten.
**Date:** 2026-05-31 · **Project:** klarvo · **Workflow:** `bmad-qa-generate-e2e-tests` (gap-discovery + auto-apply).

## Framework

- **Rust `cargo test`**, in-module `#[cfg(test)] mod tests` in `src-tauri/src/config/mod.rs` (the project's established pattern; the module already holds ~90 specs). No JS/browser E2E framework applies — this is a backend data-integrity feature with no UI surface to drive (the D1 boot-toast delivery is an explicitly deferred follow-up and is *not* Linux-testable; see story §D1).
- Private fns (`backup_corrupt_config`) are reachable from the in-module test via `use super::*`.

## Method

Story 1.2 shipped with 6 specs (a–f). This run did **gap discovery**, not duplication: a 4-lens fan-out (AC-completeness · branch/error-injection coverage · ROB-02 invariants · test-quality/non-tautology) surfaced 17 candidate gaps, each then **adversarially verified** (skeptical default: `already-covered` / `tautological` / `not-feasible` / `real`). 12 candidates resolved to 4 distinct real gaps after dedup; 5 were rejected (recorded below).

## Generated Tests (auto-applied)

All added to the `config::tests` module in `src-tauri/src/config/mod.rs`:

- [x] **(g)** `test_truly_unreadable_config_no_backup_no_warning` — **AC#4 inner branch.** When `config.json` cannot be read even as raw bytes (trigger: it is a *directory* → both `read_to_string` and `std::fs::read` fail with kind ≠ `NotFound`), `load_config` falls back to defaults with **no backup, no warning, no panic**. Pins the `Err(read_err)` arm (`config/mod.rs:1062-1064`) that spec (d) — raw re-read *succeeds* — never reaches.
- [x] **(h)** `test_backup_write_failure_records_degraded_warning` — **AC#5 failure path.** When `save_atomic` itself fails (trigger: backup target's parent dir absent), `backup_corrupt_config` returns normally (infallible), pushes exactly one **degraded** warning (`"could not be backed up"`), and writes no backup file. Pins the `save_atomic` Err arm (`config/mod.rs:992-1002`) untouched by (a)–(f).
- [x] **(i)** `test_corrupt_backup_warning_names_recovery_file` — **AC#1 / D1 content contract.** On a successful corrupt-backup the recorded warning **names the actual backup file** and carries the `config.json.corrupt-` prefix, so the user knows where to recover from. Tightens the prior `!warnings.is_empty()` assertions, which would pass on a content-free message; also locks the string-built name against a `with_extension` regression.
- [x] **(j)** `test_valid_config_records_no_backup_or_warning` — **AC#1 inverse / false-positive guard.** A valid `config.json` produces **no** backup and **no** warning, and its contents round-trip (not silently defaulted). No prior spec asserted the happy path stays clean.

### Robustness note on triggers

Both error-path triggers are **root-independent** by design: a directory-as-file (g) and a missing parent dir (h). The verifiers explicitly rejected the alternative `chmod 0o000` / `0o555` triggers because CI/containers frequently run as **root**, where permission bits are bypassed and such tests silently misfire.

## Coverage

| Acceptance Criterion | Before (a–f) | After (+g–j) |
|---|---|---|
| AC#1 parse-error backup + warning | (a)(b) | (a)(b) **+ (i) warning content** |
| AC#2 ROB-02 irreversibility | (e) | (e) |
| AC#3 NotFound → no backup/warning | (c) | (c) |
| AC#4 read-error → backup (raw read **succeeds**) | (d) | (d) |
| AC#4 read-error → **raw read also fails** (truly unreadable) | ✗ none | **(g)** |
| AC#5 no stray temp / non-destructive | (f) | (f) |
| AC#5 backup-write **failure** → degraded warning, infallible | ✗ none | **(h)** |
| AC#1 false-positive guard (valid config stays clean) | ✗ none | **(j)** |

- **config branch coverage** of `load_config_reporting` + `backup_corrupt_config`: all reachable arms now have a dedicated, deterministic, non-tautological spec.
- **config::tests:** 88 → **92 passing**. **Full lib suite:** 524 → **528 passing, 0 failed**, no regressions.

## Rejected candidates (verified non-actionable — recorded, not applied)

- **"Never overwrite a prior `.corrupt-<ts>`" across repeat boots** → *not-feasible.* The timestamp is **unix-seconds** granularity; two sequential corrupt loads in one test collide on the same `ts`, so the non-overwrite property cannot be pinned deterministically without timing control. A pre-seeded far-past fixture would pass trivially (tautological). **Documented coverage limitation**, not an applied test.
- **`with_extension` filename-trap** (×2) → *already-covered.* The `corrupt_backups` helper globs `config.json.corrupt-`; a regression to `with_extension` (yielding `config.corrupt`) makes spec (a)'s `assert_eq!(backups.len(), 1)` fail. Now additionally reinforced by (i).

## Validation against `checklist.md`

- [x] Tests cover happy path ((j)) + critical error cases ((g) unreadable, (h) backup-write failure)
- [x] Standard framework APIs (`#[test]`, `cargo test`); proper helpers (`temp_dir`, `corrupt_backups`)
- [x] All generated tests run successfully (92/92 config, 528/528 lib)
- [x] Clear descriptive doc-comments + assert messages; **no hardcoded waits/sleeps**
- [x] Tests independent (each owns its own `TempDir`, no shared/order-dependent state)
- [x] **Non-tautological** — each assertion names the regression it would catch (adversarially verified)

## Next Steps

- Windows smoke (NFR-W) for Story 1.2 remains the only outstanding DoD item — manual E1 handoff on Andy's Windows release build (unchanged by this QA run; these specs are Linux-validated `std::fs` logic).
- CI runs `cargo test --lib` (these specs are root-independent, so they hold under containerized CI).
