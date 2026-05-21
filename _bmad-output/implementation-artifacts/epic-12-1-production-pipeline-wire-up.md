---
story: 12.1
status: done
epic: 12
inputDocuments:
  - memory/project_pipeline_wireup_drift.md
  - _bmad-output/implementation-artifacts/production-pipeline-wireup-DRAFT.md
  - pipeline-manifest.toml
  - klarvo-plugins/klarvo-plugin-groq/src/lib.rs
  - klarvo-plugins/klarvo-plugin-verbatim/src/provider.rs
  - klarvo-core/src/manifest.rs
  - klarvo-core/src/registry.rs
  - klarvo-core/src/pipeline/executor.rs
  - shells/windows/src-tauri/src/main.rs (lines 49-100, 348-367)
  - shells/windows/src-tauri/Cargo.toml
---

# Story 12.1: Production Pipeline Wire-Up — Manifest + Plugin Registry

Status: **done**

## Story

As a Klarvo user,
I want that after Hotkey-Press + speaking + Hotkey-Release my audio is actually transcribed and the text lands via Ctrl+V in the active window,
so that Klarvo fulfills its actual purpose (voice-to-text dictation).

As a Klarvo developer,
I want the embedded `pipeline-manifest.toml` filled with real production stages and all referenced `plugin_id`s registered in `build_plugin_registry`,
so that committed story code (Epic 2 Groq, Verbatim Cleanup) is actually active at runtime, not just green in unit tests.

As a BMad maintainer,
I want a test gate that validates every `plugin_id` referenced in the embedded manifest is present in the plugin registry at boot,
so that future manifest changes cannot re-introduce this class of integration drift.

## Context

The app records audio and the pipeline executes — but the pipeline is mathematically a no-op (passthrough only). `klarvo-plugin-groq::register()` doesn't exist; `klarvo-plugin-groq` isn't even a dependency of the Windows shell crate. Story 11.5's `CleanupDone` event depends on a cleanup stage delivering text — that prerequisite doesn't exist yet.

**Pipeline topology selected (O1, decided 2026-05-21):** Variante A — STT=groq + Cleanup=verbatim.
- Polished is excluded (memory: `feedback_polished_designschwaeche` — "in v2 neu bauen")
- Whisper-local: conditional (already wired for model_path present, no manifest change needed)
- Groq's Epic-2 implementation is complete including retry, error classification, and auth-leak safety

**Delivery is NOT a pipeline stage (O2, resolved by code inspection):** `klarvo-core/src/pipeline/executor.rs` returns `StageData::Text`; the orchestrator in `klarvo-shell-orchestrator/src/session.rs` calls `target.deliver(&text)` + `paste_backend.paste()` after `run_pipeline` returns. No manifest change needed for delivery.

**No new `PipelineStageType` variant needed (O5, resolved):** `PipelineStageType::Cleanup { plugin_id }` already exists and handles `plugin_id = "verbatim"`. No ADR needed.

## Acceptance Criteria

**AC-1 — Manifest updated to production stages:**
`pipeline-manifest.toml` content after this story:
```toml
schema_version = 1

[[pipeline.stages]]
type = "stt"
plugin_id = "groq"

[[pipeline.stages]]
type = "cleanup"
plugin_id = "verbatim"
```
The old `passthrough` stage is removed.

**AC-2 — Groq plugin exposes `register()` and `ID`:**
`klarvo-plugins/klarvo-plugin-groq/src/lib.rs` exports a public `register(registry: &mut PluginRegistry, key_store: Arc<dyn KeyStore>)` function that calls `registry.register_stt(ID, Arc::new(Groq::new(key_store)))`. `ID = "groq"` is already defined in the file.

**AC-3 — Shell Cargo.toml includes `klarvo-plugin-groq` dependency:**
`shells/windows/src-tauri/Cargo.toml` lists `klarvo-plugin-groq = { path = "../../../klarvo-plugins/klarvo-plugin-groq" }` under `[dependencies]`.

**AC-4 — `build_plugin_registry` calls `klarvo_plugin_groq::register`:**
In `shells/windows/src-tauri/src/main.rs`, `build_plugin_registry` accepts a `keystore: Arc<dyn KeyStore>` parameter (in addition to existing `settings` and `output_language`), and calls `klarvo_plugin_groq::register(&mut registry, keystore)` after `klarvo_plugin_verbatim::register(&mut registry)`. The Whisper-local conditional block stays unchanged. The call site at Step 8 passes `Arc::clone(&keystore)`.

**AC-5 — Integration test: manifest plugin IDs match registered plugins (Forcing Sentinel):**
A new test file `klarvo-plugins/klarvo-plugin-groq/tests/wireup_consistency.rs` contains a `#[test]` that:
1. Calls `klarvo_core::manifest::parse_embedded()` and asserts it succeeds
2. For every `PipelineStageType::Stt { plugin_id }` stage in the manifest: builds a fresh `PluginRegistry`, calls `klarvo_plugin_groq::register(&mut registry, Arc::new(InMemoryKeyStore::empty()))`, and asserts `registry.stt(plugin_id).is_some()`
3. For every `PipelineStageType::Cleanup { plugin_id }` stage: builds a fresh registry, calls `klarvo_plugin_verbatim::register(&mut registry)`, and asserts `registry.cleanup(plugin_id).is_some()`
4. Does NOT reference `Passthrough` or `_` wildcards in the match (exhaustive, no wildcard, per `stage.rs` invariant)

This test fails at `cargo test` if the manifest is updated without updating the registry wire-up — the Forcing Sentinel contract.

**AC-6 — No changes to `klarvo-shell-orchestrator/src/session.rs`:**
The `CleanupDone` emission code at session.rs:385-393 is untouched. The pipeline now delivers text; the emission fires naturally through the existing code path.

**AC-7 — `cargo check --target x86_64-pc-windows-gnu` clean:**
No new compiler warnings or errors on the Windows cross-compile target (per `feedback_windows_cross_compile_verify`).

## Tasks / Subtasks

- [x] T1: Update `pipeline-manifest.toml` (AC-1)
  - [x] T1.1: Replace passthrough stage with stt + cleanup stages exactly as specified in AC-1
  - [x] T1.2: Verify existing `manifest.rs::tests::parse_embedded_smoke_test` still passes (`!m.pipeline.stages.is_empty()` — it will, 2 stages) ✓ 8/8 manifest tests pass
  - [x] T1.3: Verify that the compile-time `include_str!` in `klarvo-core/src/manifest.rs:44` picks up the new content (no path changes needed — it's `../../pipeline-manifest.toml` relative to klarvo-core/src) ✓ confirmed via test run
  - Note: `klarvo-core/tests/pipeline_end_to_end.rs::embedded_passthrough_manifest_preserves_input` was renamed to `passthrough_manifest_preserves_input` and decoupled from `parse_embedded()` to use `parse_from_str` with explicit passthrough TOML. The old test was testing passthrough behavior via the embedded manifest — now that the embedded manifest is production (stt+cleanup), the test is crate-agnostic.

- [x] T2: Add `register()` to `klarvo-plugin-groq` (AC-2)
  - [x] T2.1: Added `pub fn register(registry: &mut klarvo_core::registry::PluginRegistry, key_store: std::sync::Arc<dyn klarvo_core::keystore::KeyStore>)` to `klarvo-plugins/klarvo-plugin-groq/src/lib.rs` at the end of the file.
  - [x] T2.2: `ID = "groq"` const confirmed at line 68 of lib.rs
  - [x] T2.3: All 13 groq tests pass (6 e2e + 6 external_contract + 1 ignored)

- [x] T3: Add `klarvo-plugin-groq` to shell Cargo.toml (AC-3)
  - [x] T3.1: Added `klarvo-plugin-groq = { path = "../../../klarvo-plugins/klarvo-plugin-groq" }` adjacent to `klarvo-plugin-verbatim`

- [x] T4: Update `build_plugin_registry` in main.rs (AC-4)
  - [x] T4.1: Signature changed to include `keystore: std::sync::Arc<dyn KeyStore>` (KeyStore already imported on line 28)
  - [x] T4.2: `klarvo_plugin_groq::register(&mut registry, keystore);` added after verbatim::register, before whisper-local conditional
  - [x] T4.3: Uses fully-qualified crate path `klarvo_plugin_groq::register(...)` — consistent with existing `klarvo_plugin_verbatim::register(...)` pattern, no `use` needed
  - [x] T4.4: Call site at Step 8 updated to `build_plugin_registry(&settings, &output_language, Arc::clone(&keystore))`

- [x] T5: Create Forcing-Sentinel integration test (AC-5)
  - [x] T5.1: Both `klarvo-plugin-verbatim` and `klarvo-test-fixtures` were already in dev-dependencies — no change needed
  - [x] T5.2: Created `klarvo-plugins/klarvo-plugin-groq/tests/wireup_consistency.rs` with 2 tests. Note: The `#[cfg(feature = "stage-stt")]` guards were NOT used (those are klarvo-core features, not klarvo-plugin-groq features; using them would silently skip the test body). Instead, `PipelineStageType::Stt` and `PipelineStageType::Cleanup` are referenced directly — they're always available since klarvo-plugin-groq depends on klarvo-core with default features.
  - [x] T5.3: Both forcing-sentinel tests pass: `embedded_manifest_stt_plugin_ids_are_registered` + `embedded_manifest_cleanup_plugin_ids_are_registered`

- [x] T6: Cross-compile gate (AC-7)
  - [x] T6.1: `cargo check --target x86_64-pc-windows-gnu -p klarvo-plugin-groq` — clean. Full `klarvo-windows-shell` check fails on pre-existing `whisper-rs-sys` layout-size issue (MinGW bindings, unrelated to this story's changes). Verified groq plugin + klarvo-core compile cleanly for Windows target.

## Dev Notes

### Critical Codebase State (read before touching any file)

**`klarvo-plugin-groq` has NO `register()` function today.**
The lib.rs ends at line 319 with the WAV-encoding helper and error-mapping functions. There is no `register()` and no `pub use` re-export of one. The module-level doc at lines 10-13 explicitly says: "Registry-registration and Manifest-driven instantiation are Epic 3 scope" — this story IS that deferred scope.

**`klarvo-plugin-groq` is NOT a dependency of the Windows shell crate.**
`shells/windows/src-tauri/Cargo.toml` has `klarvo-plugin-verbatim` (line 42) and `klarvo-plugin-whisper-local` (line 43) — Groq is absent. Adding the dep is required for `main.rs` to compile after T4.

**`build_plugin_registry` is a nested function inside `fn main()`.**
It is defined at lines 49-100 of main.rs, inside the `#[cfg(target_os = "windows")] fn main()` body. It currently accepts `(settings: &Settings, output_language: &str)`. The `keystore: Arc<dyn KeyStore>` is available in scope at line 281 (outer fn main body) — `Arc::clone(&keystore)` can be passed to the function.

**Verbatim's `register()` pattern (reference for Groq's `register()`):**
```rust
// klarvo-plugins/klarvo-plugin-verbatim/src/provider.rs
pub fn register(registry: &mut PluginRegistry) {
    registry.register_cleanup(ID, Arc::new(Verbatim::new()));
}
```
Groq's signature differs: needs a `key_store: Arc<dyn KeyStore>` argument because `Groq::new(key_store)` takes it.

**Groq `ID` const is already defined (line 68):**
```rust
pub const ID: &str = "groq";
```
No change needed for the ID — just reference it in `register()`.

**Executor boot-check ordering (load-bearing, must not break):**
`executor.rs` runs two checks before any stage dispatch:
1. Type-Chaining-Check: Stt expects `"audio"` input, Cleanup expects `"text"`. The new manifest (audio → stt → text → cleanup → text) passes this check naturally.
2. Plugin-Registry-Lookup: Both `registry.stt("groq")` and `registry.cleanup("verbatim")` must return `Some`. This fails at runtime if AC-2/AC-4 are incomplete — the app hard-fails at boot with `PipelineValidation`.

**Manifest `include_str!` path:**
`klarvo-core/src/manifest.rs:44` has `include_str!("../../pipeline-manifest.toml")`. The workspace root `pipeline-manifest.toml` is embedded at compile-time. Changing the TOML content is all that's needed — no code change in manifest.rs.

**Existing `parse_embedded_smoke_test` (manifest.rs:275):**
```rust
fn parse_embedded_smoke_test() {
    let m = parse_embedded().expect("embedded manifest must parse");
    assert_eq!(m.schema_version, 1);
    assert!(!m.pipeline.stages.is_empty());
}
```
This continues to pass with 2 stages (stt + cleanup). No test modification needed.

**Existing manifest tests for stt and cleanup stages (manifest.rs:178-208):**
These are already gated on `#[cfg(feature = "stage-stt")]` and `#[cfg(feature = "stage-cleanup")]` and test `parse_from_str` with the exact TOML syntax this story produces. They confirm the parser already handles the new manifest format.

**Forcing-Sentinel test location rationale (AC-5):**
The test lives in `klarvo-plugin-groq/tests/` rather than the shell because:
- `klarvo-core` cannot depend on plugin crates (would create circular dependency)
- Shell's `build_plugin_registry` is a nested function inside `fn main()`, inaccessible from external test code without refactoring
- `klarvo-plugin-groq` already depends on `klarvo-core` (for traits, manifest, registry) — no new dependency arc created by the test
- `klarvo-plugin-verbatim` added as a `dev-dependency` of `klarvo-plugin-groq` only for this test (both are sibling crates, no circular issue)

**NFR5 PII-protection (no log leakage):**
`Groq::new()` and `register()` do not touch the API key — the key is fetched lazily in `transcribe()` via `key_store.get(GROQ_API_KEY_ID)`. No new logging added in this story. The existing auth-leak test in `klarvo-plugin-groq/tests/external_contract.rs` continues to cover this invariant.

**Session.rs unchanged (AC-6):**
`CleanupDone` emission at session.rs:385-393 fires when `text_to_deliver` is `Some` after the cleanup stage. The new pipeline delivers text through cleanup → the orchestrator sees `StageData::Text` → emits `CleanupDone`. No code change needed in session.rs.

### Architecture Compliance

- **Hard-fail-no-warn-skip invariant** (memory: `feedback_manifest_compile_contract`): The executor already enforces this. Groq registration failure is a boot-time hard-fail — correct behavior.
- **Cargo-Feature gates for stage types** (stage.rs): `PipelineStageType::Stt` is already gated on `feature = "stage-stt"`, `Cleanup` on `feature = "stage-cleanup"`. Both features are in `klarvo-core`'s default feature set. No feature changes needed.
- **Registry API**: `register_stt` panics on duplicate ID. `build_plugin_registry` is only called once at boot (Step 8) — no duplicate risk.
- **Windows cross-compile requirement** (memory: `feedback_windows_cross_compile_verify`): All changes in this story are in the Windows shell crate and in platform-agnostic plugin/core crates. Run `cargo check --target x86_64-pc-windows-gnu -p klarvo-windows-shell` after all changes.

### Out of Scope (do NOT implement in this story)

- DeepSeek / Chat / Keystroke / VAD-Silero plugin wire-up — own stories
- Automated E2E test framework (Tauri-WebDriver, audio mock, hotkey simulation) — own story
- Hotkey-Boot-DB-Bypass fix (memory: `project_hotkey_boot_db_bypass`) — own story
- Polished plugin wire-up (memory: `feedback_polished_designschwaeche` — "in v2 neu bauen") — own story
- Manual smoke-test execution or documentation — out of story scope (Andy's task post-impl)
- Moving `build_plugin_registry` to a separate module — not required by any AC; do not do it unless a compiler constraint forces it

### References

- [Source: klarvo-plugins/klarvo-plugin-groq/src/lib.rs#1-13] — module-level doc: "Registry-registration and Manifest-driven instantiation are Epic 3 scope"
- [Source: klarvo-plugins/klarvo-plugin-groq/src/lib.rs#68] — `pub const ID: &str = "groq";`
- [Source: klarvo-plugins/klarvo-plugin-groq/src/lib.rs#97-114] — `Groq::new(key_store: Arc<dyn KeyStore>)`
- [Source: klarvo-plugins/klarvo-plugin-verbatim/src/provider.rs#10-14] — verbatim `register()` pattern
- [Source: klarvo-core/src/registry.rs#29-35] — `register_stt(id, Arc<dyn SttProvider>)` signature
- [Source: klarvo-core/src/pipeline/executor.rs#85-200] — boot-check ordering + registry lookup hard-fail
- [Source: klarvo-core/src/manifest.rs#44] — `include_str!("../../pipeline-manifest.toml")`
- [Source: shells/windows/src-tauri/src/main.rs#49-100] — `build_plugin_registry` current signature + body
- [Source: shells/windows/src-tauri/src/main.rs#348-367] — Step 8: manifest parse + registry build + call site
- [Source: shells/windows/src-tauri/Cargo.toml#42-43] — current plugin deps (groq absent)
- [Source: klarvo-shell-orchestrator/src/session.rs#385-393] — CleanupDone emission (do not touch)
- [Source: klarvo-test-fixtures/src/keystore.rs#15-30] — InMemoryKeyStore::empty() for test fixture
- [memory: feedback_manifest_compile_contract] — hard-fail-no-warn-skip invariant
- [memory: feedback_polished_designschwaeche] — Polished excluded from this story
- [memory: feedback_windows_cross_compile_verify] — cross-compile gate requirement
- [memory: project_pipeline_wireup_drift] — diagnosis that motivated this story

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (analysis + story-spec + implementation)

### Debug Log References

### Completion Notes List

- AC-1: `pipeline-manifest.toml` updated from passthrough-only to stt(groq)+cleanup(verbatim). The `parse_embedded_smoke_test` verifies !stages.is_empty() — still passes (2 stages).
- AC-1 side effect: `klarvo-core/tests/pipeline_end_to_end.rs` test `embedded_passthrough_manifest_preserves_input` was decoupled from the embedded manifest → renamed to `passthrough_manifest_preserves_input` and now uses `parse_from_str` with explicit passthrough TOML. The test still covers the passthrough executor arm.
- AC-2: `pub fn register(registry, key_store)` added to `klarvo-plugin-groq/src/lib.rs`. Uses fully-qualified `std::sync::Arc` paths to match existing style in the file. `ID = "groq"` was already exported.
- AC-3: `klarvo-plugin-groq` dep added to shell Cargo.toml, adjacent to verbatim.
- AC-4: `build_plugin_registry` signature extended with `keystore: std::sync::Arc<dyn KeyStore>`. `klarvo_plugin_groq::register(&mut registry, keystore)` called immediately after verbatim, before whisper-local conditional. Call site passes `Arc::clone(&keystore)`.
- AC-5: `wireup_consistency.rs` created. Avoided `#[cfg(feature = "stage-stt")]` gates — those are klarvo-core features, not klarvo-plugin-groq features; using them silently skipped the test bodies. Direct variant usage works because klarvo-plugin-groq depends on klarvo-core with default features.
- AC-6: session.rs untouched — verified no changes.
- AC-7: `klarvo-plugin-groq` cross-compiles clean for `x86_64-pc-windows-gnu`. Full shell-crate check fails on pre-existing `whisper-rs-sys` MinGW layout-mismatch (unrelated to this story).
- All workspace tests (excl. jni + shell) pass: 122 klarvo-core + 14 klarvo-plugin-groq + pipeline e2e + fixtures.

### File List

- `pipeline-manifest.toml` — updated (passthrough → stt+cleanup)
- `klarvo-core/tests/pipeline_end_to_end.rs` — test decoupled from embedded manifest
- `klarvo-plugins/klarvo-plugin-groq/src/lib.rs` — `register()` function added
- `klarvo-plugins/klarvo-plugin-groq/tests/wireup_consistency.rs` — new (forcing-sentinel)
- `shells/windows/src-tauri/Cargo.toml` — `klarvo-plugin-groq` dep added
- `shells/windows/src-tauri/src/main.rs` — `build_plugin_registry` signature + groq call + call site

### Review Findings

Code-Review-Pass 2026-05-21 (Blind Hunter + Edge Case Hunter + Acceptance Auditor).

- [x] [Review][Decision→Accept] **12.1-D1 — Sentinel-Scope: per-plugin vs. shell-wireup** — `wireup_consistency.rs` ruft `klarvo_plugin_groq::register()` direkt im Test auf, prüft NICHT shell-side `build_plugin_registry`. Regression "main.rs droppt `klarvo_plugin_groq::register(...)` Call" wird vom Test nicht gefangen — nur Runtime hard-failed beim Boot via `executor.rs` Plugin-Registry-Lookup. **Decision 2026-05-21: Accept-as-is per Spec-Rationale.** Runtime-Boot-Hard-Fail (Executor `PipelineValidation`) ist Safety-Net; Refactor von `build_plugin_registry` aus `fn main()` raus wäre Premature-Abstraction für einen Test (memory `feedback_premature_abstraction_guard`).
- [x] [Review][Patch] **12.1-P1 — Vacuous-Pass im Forcing-Sentinel** [`klarvo-plugins/klarvo-plugin-groq/tests/wireup_consistency.rs:21-50`] — Wenn das Manifest auf passthrough-only zurückregrediert (= GENAU der Drift-Bug-Class den die Story verhindern soll), enthält `manifest.pipeline.stages` keine `Stt`/`Cleanup`-Variants — beide Tests laufen die `for`-Schleife durch ohne den `assert!()` zu treffen und passen grün. Fix: pre-condition `assert!(stt_count > 0, ...)` bzw. `assert!(cleanup_count > 0, ...)` vor der Loop, damit das Verschwinden eines Stage-Types einen Test-Failure erzeugt. **Applied 2026-05-21:** stt_count/cleanup_count pre-condition asserts hinzugefügt, beide Tests grün.
- [x] [Review][Defer] **12.1-DF1 — Kein behavioral E2E-Test der stt→cleanup-Chain** [`klarvo-plugins/klarvo-plugin-groq/tests/`] — Wire-Up + Forcing-Sentinel beweisen Registration, nicht Daten-Fluss durch zwei Stages. Defer-Reason: out-of-scope per Spec "Automated E2E test framework — own story"; pre-existing.
- [x] [Review][Defer] **12.1-DF2 — AC-7 nur teilweise satisfied (`whisper-rs-sys` MinGW-Layout-Mismatch)** [`shells/windows/src-tauri/`] — Voller `klarvo-windows-shell` cross-compile-check failed auf pre-existing MinGW-bindings-Issue. `klarvo-plugin-groq` + `klarvo-core` compilen sauber für x86_64-pc-windows-gnu. Defer-Reason: pre-existing, dokumentiert in Spec-Completion-Notes, unrelated zu Story-12.1-Changes (cf. memory `feedback_release_build_blind_spot`).
