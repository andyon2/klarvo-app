---
story: "1.1"
epic: "1"
title: "Atomic state-file writes via a shared save_atomic helper"
status: review
findings: ["ROB-01"]
gatedBy: ADR-0015
inputDocuments:
  - _bmad-output/planning-artifacts/epics.md
  - docs/adr/0015-state-file-write-convention.md
  - docs/robustness-audit-2026-05-30.md
---

# Story 1.1: Atomic state-file writes via a shared `save_atomic` helper

As a klarvo user, I want my config and dictionary written atomically, so that a crash or power loss
mid-write can never leave me with an empty/truncated `config.json` and the loss of all my API keys
and license.

## Acceptance Criteria

1. **`save_atomic` helper** — writes to a temp file in the SAME directory as the target, fsyncs it
   (`sync_all`), and atomically renames/replaces it over the target. Sync variant (`save_config`/
   `save_dictionary` are sync callers).
2. **`save_config`** (`config/mod.rs:1261-1271`, today bare `std::fs::write`) routes through
   `save_atomic`. **`save_dictionary`** (`dictionary/mod.rs:146-160`, same gap) also routes through it.
3. **Crash-safety** — killed between temp-write and rename: previous file intact; orphan temp never
   read as live config.
4. **No swallowing** — the helper returns its write error; callers propagate it.

## DoD

- Linux: `cargo test` (new specs below) + `cargo clippy` clean.
- **NFR-W (hard gate, MANUAL):** rename-over-existing-target atomicity verified on a REAL Windows
  release build. Linux cannot validate this — see escalation E1.

## Dev notes — forks resolved by the conductor (do not re-litigate)

Grounded against HEAD (v1-ship, 2026-05-30). See `docs/bmad-autopilot-escalation-contract.md` for the
rule each resolution maps to.

- **Approach = `tempfile::NamedTempFile`** (A1; ADR-0015 §1 sanctions `persist`/`ReplaceFileW`).
  Verified (V1): `persist` *atomically replaces* an existing file cross-platform and does **not**
  sync. So: `NamedTempFile::new_in(parent_dir)` → `write_all(bytes)` → `as_file().sync_all()` →
  `persist(target)`. No naive `std::fs::rename` (breaks on Windows when target exists). No unsafe FFI.
- **Promote `tempfile` from `[dev-dependencies]` to `[dependencies]`** in `src-tauri/Cargo.toml`
  (it is currently dev-only). Sanctioned by ADR-0015 → not an E2 stop.
- **Home = new `src-tauri/src/fs.rs`**, declared `mod fs;` in `lib.rs` (A2). `config` and `dictionary`
  share no parent module; ADR-0015 says all state files inherit this helper.
- **Signature** `pub fn save_atomic(path: &Path, bytes: &[u8]) -> anyhow::Result<()>` (A3, matches
  callers). `save_config`/`save_dictionary` keep their `create_dir_all`, serialize as today, then call
  `save_atomic(&path, contents.as_bytes())`.
- **fsync the temp handle** even though the "ref impl" `llm_model.rs:249-258` only `flush()`es
  (A4 — the AC requires `sync_all`; the anchor is illustrative and under-implements).
- **Parent-dir fsync deferred** (A5 — documented limitation; residual risk "durable but stale-old
  file" ⊂ the truncation risk being fixed; the AC's "never truncated/empty" guarantee is fully met).

## Tasks

- [x] Add `src-tauri/src/fs.rs` with `save_atomic`; declare `mod fs;` in `lib.rs` (private `mod`, matches siblings).
- [x] Promote `tempfile` to `[dependencies]` in `src-tauri/Cargo.toml` (removed from dev-deps; resolves 3.26.0).
- [x] Route `save_config` (`config/mod.rs:1267`) and `save_dictionary` (`dictionary/mod.rs:152`) through `save_atomic` — single-line each, behavior byte-identical otherwise.
- [x] Linux specs (a)-(d): 4 tests, all pass. Plus 82 config + 25 dictionary tests still green.
- [x] `cargo test --lib` green; `cargo clippy` clean for touched files (see known-issue below re: repo-wide gate).
- [ ] **MANUAL / ESCALATE (E1, gates →done):** Windows release-build atomicity check (NFR-W) — see checklist below.

## Review outcome (adversarial, fresh-context Opus)

All six invariants CONFIRMED (atomicity, fsync-before-rename, no-temp-leak-on-error, error propagation,
call-site behavior preservation, dependency hygiene). Strictly more durable than the cited "ref impl"
(real `sync_all` vs. mere `flush`). **No blockers, no should-fix.** Two test-quality nits, deferred
(documented, not churned — conductor call):

- Nit 1: test (d) asserts only `is_err()`, doesn't pin the error source. Could be tightened to assert the
  failure comes from `new_in` (NotFound), not the `.parent()`-None guard.
- Nit 2: the `.parent()`-is-`None` guard is unexercised by any test (only reachable for a bare root path;
  harmless since callers always pass `app_data_dir.join(FILE)`).

## NFR-W Windows smoke-DoD — exact checks to run on a real Windows release build

(Linux cannot validate any of these — `MoveFileExW`+`MOVEFILE_REPLACE_EXISTING` path.)

1. Normal cycle: press-to-paste → settings-save → kill the process → reopen → exactly ONE valid
   `config.json`, no leftover temp, keys/license intact.
2. Locked target: a save while the file is held open by another process/AV scanner must return `Err`
   (surfaced, not a silent partial write).
3. Read-only target: confirm a read-only existing `config.json` doesn't block or corrupt the replace
   (or that the error is surfaced).

## Known repo issue surfaced during this story (NOT this story's defect → quick-dev)

`cargo clippy --lib -- -D warnings` is already RED on `v1-ship`: 19 errors in untouched files
(`llm/mod.rs`, `audio/mod.rs`, `commands/misc.rs`, `license/ls_client.rs`, `commands/feedback.rs`,
`commands/license.rs`, `pipeline.rs`) + unused imports at `lib.rs:71`, `pipeline.rs:41`. The repo-wide
clippy gate predates this change. Route to `bmad-quick-dev`.
