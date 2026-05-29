---
title: 'Provider resolve de-duplication (Task 2.1 residual, Phase 2)'
type: 'refactor'
created: '2026-05-29'
status: 'done'
baseline_commit: 'f0c3885'
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The STT+LLM provider resolve pair (`resolve_stt_provider` then `resolve_cleanup_provider`, called back-to-back) is duplicated across the two live config-mutating commands `save_settings` (settings.rs:492-493) and `clear_api_key` (settings.rs:969-970), plus boot (lib.rs:289-290). A third path, `update_api_keys` (653-690), hand-builds providers and diverges — but it is dead (no `src/` frontend or `android/` caller).

**Approach:** Extract one pure helper `resolve_providers(cfg, dir) -> (stt, cleanup)` in `pipeline.rs` and route the live sites through it. Behavior-preserving — both sites already make the identical adjacent calls. Leave `update_api_keys` untouched (dead, already doc-marked legacy). The settings field-merge (`merge_settings`) is already canonical (commit f0c3885) and is out of scope.

## Boundaries & Constraints

**Always:**
- Behavior-preserving. All 489 existing tests stay green (`cd src-tauri && cargo test`).
- Helper is a PURE function (no locks, no persistence, no `AppState`) so it is order-independent at each call site.
- Backward search before edit: confirm the resolve-pair call sites are exactly the three named.

**Ask First:**
- If a call site does NOT make the two `resolve_*` calls adjacently / with the same arguments (so a single pure helper would alter observable behavior) — HALT and report before forcing the extraction.

**Never:**
- Do NOT rename `merge_settings` → `apply_patch` (pure churn; merge is already canonical).
- Do NOT relocate the merge onto `AppConfig` (layering inversion: `SettingsPatch` lives in `commands/`).
- Do NOT touch `update_api_keys` or add an `id()`/`name()` seam to the provider traits (dead-code path; out of scope).
- No new crates, no module split, no change to provider selection / fallback logic.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Resolve from config | `&AppConfig` + `app_data_dir` | Returns `(Arc<dyn SttProvider>, Arc<dyn CleanupProvider>)` identical to the two inline calls today | N/A — `resolve_*` are infallible |
| Unknown provider id in cfg | cfg with `stt_provider`/`llm_provider` set to garbage | Same fallback as today (Groq for STT, DeepSeek for cleanup) | N/A |

</frozen-after-approval>

## Code Map

- `src-tauri/src/pipeline.rs` — `resolve_stt_provider:41`, `resolve_cleanup_provider:107`; NEW `resolve_providers` helper goes here, next to them.
- `src-tauri/src/commands/settings.rs` — `save_settings` (live, resolve pair at 492-493), `clear_api_key` (live, resolve pair at 969-970), `update_api_keys` (dead, 653-690; leave).
- `src-tauri/src/lib.rs:289-290` — boot-time resolve pair (third live consumer).

## Tasks & Acceptance

**Execution:**
- [x] `src-tauri/src/pipeline.rs` — add `pub fn resolve_providers(cfg: &AppConfig, app_data_dir: &Path) -> (Arc<dyn SttProvider>, Arc<dyn CleanupProvider>)` returning `(resolve_stt_provider(cfg, app_data_dir), resolve_cleanup_provider(cfg))`.
- [x] `src-tauri/src/commands/settings.rs` — replace the two-line resolve pair in `save_settings` (492-493) and `clear_api_key` (969-970) with one `resolve_providers(...)` call each; leave each command's persist / in-memory / lock-write ordering exactly as-is.
- [x] `src-tauri/src/lib.rs` — replace the boot resolve pair (289-290) with `resolve_providers(...)`.

**Acceptance Criteria:**
- Given the current config, when `save_settings` or `clear_api_key` runs, then `inner.stt_provider` and `inner.cleanup_provider` are repopulated from the post-mutation config exactly as today (no observable change).
- Given the full suite, when `cargo test` runs in `src-tauri`, then 489 tests pass (incl. A1–A5), 0 failed.
- Given `update_api_keys`, when reviewed, then its body is unchanged.

## Design Notes

`update_api_keys` investigated and deliberately excluded: no caller in `src/` (frontend) or `android/`; its doc already says "prefer save_settings". Its hand-rolled `GroqWhisper`/`DeepSeekCleanup` construction ignores `stt_provider`/`llm_provider` selection and `stt_model` — a genuine divergence, but unreachable. Correcting it would need an instance-introspection seam (`id()` on both provider traits, ~12 impls) plus a characterization net (Rule 10) for zero runtime value. Deferred; if the command is ever revived, that seam + net must come first.

`merge_settings` (the ~97-field merge) is NOT renamed/relocated: f0c3885 already made it the single canonical, atomic merge with one production caller. The briefing's "AppConfig::apply_patch" naming was aspirational; honoring it would invert module layering.

## Verification

**Commands:**
- `cd src-tauri && cargo test` — expected: `test result: ok. 489 passed; 0 failed` (incl. A1–A5).
- `cd src-tauri && cargo clippy --all-targets` — expected: no new warnings.

**Manual checks:**
- `git grep -n "resolve_stt_provider\|resolve_cleanup_provider" src-tauri/src` — expected: the two `resolve_*` definitions + tests only; the three former call sites now call `resolve_providers`.

## Suggested Review Order

- New pure helper — the design intent; both providers resolved in one call.
  [`pipeline.rs:63`](../../src-tauri/src/pipeline.rs#L63)

- Hot path: settings save routed through the helper (resolve→persist order kept).
  [`settings.rs:492`](../../src-tauri/src/commands/settings.rs#L492)

- API-key clear routed through the helper (same ordering preserved).
  [`settings.rs:968`](../../src-tauri/src/commands/settings.rs#L968)

- Boot: `AppState::new` routed through the helper (third consumer).
  [`lib.rs:289`](../../src-tauri/src/lib.rs#L289)
