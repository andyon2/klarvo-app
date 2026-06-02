---
story: "4.2"
epic: "4"
title: "Tighten pipeline.rs contracts + demote test-only pub surface"
status: done
findings: ["DEPTH-pipeline"]
gatedBy: "Epic 3 test net (behavior-preserving refactor)"
buildsOn: ["3.1", "3.2", "3.3", "3.4"]
inputDocuments:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/implementation-artifacts/4-1-isolate-the-load-config-core-into-a-tested-migrate-and-normalize.md
  - _bmad-output/implementation-artifacts/deferred-work.md
---

# Story 4.2: Tighten `pipeline.rs` contracts + demote test-only `pub` surface

Status: done

## Story

As a klarvo maintainer,
I want `ProcessInput`/`ProcessOutcome`'s leaky contracts tightened and the test-only public surface demoted,
so that the pipeline's real interface is honest and consistency invariants are type-enforced rather than
doc-enforced.

## Acceptance Criteria

1. **`SttPromptPair` groups `dict_prompt` + `stt_hint_text` in `ProcessInput`.**
   `ProcessInput` (`pipeline.rs:905-927`, 17 fields) currently requires `dict_prompt` (`911`)
   and `stt_hint_text` (`913`) to stay consistent only by doc-comment. After this story, a new
   `SttPromptPair` substructure groups them so consistency is type-enforced — the caller cannot
   set one without the other. `ProcessInput` gains a single `stt_prompt: SttPromptPair` field
   replacing the two separate fields.

2. **`deliver_outcome` encapsulates the `ProcessOutcome` match arms.**
   `ProcessOutcome` (`pipeline.rs:932-953`) currently forces the caller
   (`stop_and_process_pipeline`, lines `1500-1544`) to hand-roll match-arm side-effects inline:
   error-metric increments, `consume_command_mode` calls, early returns on `Stopped`/`CommandFailed`,
   and value extraction from `Produced`.
   After this story, exactly those match-arm contents are pulled into a single
   `fn deliver_outcome(outcome, state, is_command_mode) -> Option<(...)>` (new — none exists today),
   so the consume semantics aren't re-implemented per caller.
   Usage recording, paste, history, and the done-event are NOT moved — they run after the match
   block and remain in `stop_and_process_pipeline` unchanged.

3. **Five pure decision-helpers demoted to `pub(crate)`.**
   The 5 helpers are currently `pub` but only used in-module + by in-module tests:
   - `compute_wav_rms` (line `413`)
   - `is_offline` (line `453`)
   - `silence_skip` (line `471`)
   - `post_stt_skip` (line `500`)
   - `select_llm_path` (line `523`)

   After this story they are `pub(crate)`. The in-module `#[cfg(test)]` tests keep working
   without any change (they are in the same crate). Verify no external call site exists
   before demoting.

4. **Behavior is unchanged: full hotkey→paste pipeline behaves identically.**
   All existing tests pass: `cargo test --lib` green (≥557 existing tests + any new ones added).
   The strengthened Epic-3 test net (`process_audio` specs, WAV-RMS specs, Silero VAD path,
   PI-security specs) catches any refactor regression.

5. **`cargo clippy` clean on touched files.** No new warnings on `pipeline.rs`
   (pre-existing repo-wide baseline-red allowed; zero new warnings from this diff).

6. **Inversion-check (Epic-3 retro AI-1 — DoD line).**
   For each new test added (if any), there is at least one test that goes RED when the guarded
   invariant is flipped. Verify + log in Completion Notes as done in Stories 3.1–3.4 and 4.1.

## Tasks / Subtasks

- [x] Task 1 — Read the current `ProcessInput`, `ProcessOutcome`, and the 5 helpers completely (AC: #1, #2, #3)
  - [x] Read `src-tauri/src/pipeline.rs:905-953` in full (the two structs/enums).
  - [x] Read the 5 helper functions: `compute_wav_rms` (~413), `is_offline` (~453),
        `silence_skip` (~471), `post_stt_skip` (~500), `select_llm_path` (~523).
  - [x] Read `stop_and_process_pipeline` match arms at ~1500-1544 completely — this is the
        code that `deliver_outcome` will encapsulate. Document what each arm does.
  - [x] Verify no external crate (outside `klarvo_lib`) calls the 5 helpers before demoting.
        Run `grep -rn "pipeline::compute_wav_rms\|pipeline::is_offline\|pipeline::silence_skip\|pipeline::post_stt_skip\|pipeline::select_llm_path" src-tauri/src/` to confirm zero external refs.

- [x] Task 2 — Introduce `SttPromptPair` and update `ProcessInput` (AC: #1)
  - [x] Define `pub(crate) struct SttPromptPair { pub dict_prompt: Option<String>, pub stt_hint_text: String }`.
        Place it near the `ProcessInput` definition in `pipeline.rs`.
  - [x] Replace the two fields in `ProcessInput` with `pub stt_prompt: SttPromptPair`.
  - [x] Update the single construction site in `stop_and_process_pipeline` (~1379-1390,
        where `dict_prompt` and `stt_hint_text` are built) to use `SttPromptPair { dict_prompt, stt_hint_text }`.
  - [x] Update the destructuring of `ProcessInput` inside `process_audio` (~976-992) to
        extract `dict_prompt` and `stt_hint_text` from `stt_prompt`.
  - [x] Update `make_input` in the test module (~2423-2441) to use the new field.
  - [x] Confirm all compilation errors are resolved and `cargo test --lib` is green.

- [x] Task 3 — Extract `deliver_outcome` from `stop_and_process_pipeline` (AC: #2)
  - [x] Read the full match block at ~1499-1544 one more time before extracting.
  - [x] Define `fn deliver_outcome(outcome: ProcessOutcome, state: &AppState, is_command_mode: bool) -> Option<(String, String, bool, u64, Option<u64>, Option<u32>, Option<u32>)>`.
        (Returns `None` on `Stopped`/`CommandFailed` — the caller returns early;
        returns `Some(cleaned_text, raw_text, is_command, stt_ms, llm_ms, prompt_tokens, completion_tokens)` on `Produced`.)
        The function is NOT async (no await points inside the match arms — only lock acquisitions and flag resets).
        It does NOT receive `handle`, `duration_ms`, or `pipeline_start` — those are only needed for
        usage/paste/history/done-event which remain in `stop_and_process_pipeline` after the call.
        Only the match-arm contents (error counter increments, `consume_command_mode` calls,
        early returns, and value extraction) move into `deliver_outcome`.
  - [x] Replace the match block in `stop_and_process_pipeline` with a `deliver_outcome(...)` call.
        The caller uses `let Some((cleaned_text, ...)) = deliver_outcome(...) else { return; };`
        and then continues with usage recording, paste, history, metrics, done-event as before (unchanged).
  - [x] Ensure `is_command_mode` is still available to `stop_and_process_pipeline` after the
        refactor (it is set before `process_input` is built, so it is in scope).
  - [x] Run `cargo test --lib` — all ≥557 tests must pass.

- [x] Task 4 — Demote 5 helpers to `pub(crate)` (AC: #3)
  - [x] Change `pub fn compute_wav_rms` → `pub(crate) fn compute_wav_rms`.
  - [x] Change `pub fn is_offline` → `pub(crate) fn is_offline`.
  - [x] Change `pub fn silence_skip` → `pub(crate) fn silence_skip`.
  - [x] Change `pub fn post_stt_skip` → `pub(crate) fn post_stt_skip`.
  - [x] Change `pub fn select_llm_path` → `pub(crate) fn select_llm_path`.
  - [x] Run `cargo build` (not just `cargo test --lib`) to catch any external-crate use that
        was missed by the grep. Zero compile errors expected.

- [x] Task 5 — Verify correctness and document (AC: #4, #5, #6)
  - [x] `cargo test --lib` — all ≥557 existing tests pass, plus any new ones. Zero failures.
  - [x] `cargo clippy --lib` — zero new warnings on `pipeline.rs` (pre-existing warnings allowed).
  - [x] If new tests were added (e.g. for `deliver_outcome` branching or `SttPromptPair`
        construction), verify inversion-check (AC-6): flip the invariant → test RED → restore.
        Log each check in Completion Notes.
  - [x] Write Completion Notes: which shape was chosen for `deliver_outcome`, whether new tests
        were added and their inversion-check status, and the `SttPromptPair` design decision.

## Dev Notes

### Background & Scope

**Why this story exists:** `pipeline.rs` (3570 LOC at HEAD) has three leaky-abstraction issues
identified in the robustness audit §5 (DEPTH-pipeline):

1. `ProcessInput` uses a doc-comment to enforce `dict_prompt`↔`stt_hint_text` consistency
   instead of the type system.
2. `ProcessOutcome` forces every caller to hand-roll its own deferred side-effect application.
   Today there is only one caller (`stop_and_process_pipeline`), but the pattern is fragile.
3. Five pure decision-helpers are `pub` only for tests; they should be `pub(crate)` so the
   nominal public API is honest.

**No behavior change.** This is a pure structural refactor. The Epic-1+3 test net (557 tests
at Story 4.1 close, including the new Silero VAD, WAV-RMS, PI-security, and feedback gate specs)
is specifically the safety net that makes this refactor safe to run now.

**Gating:** Run AFTER Epics 1–3. ADR-0015 §5 explicitly deferred structural refactors until
the persistence hardening was in place. Epic-3 specifically strengthened the test net. Both
conditions are now met.

**What was NOT done by predecessor stories (guard against re-doing):**
- Stories 1.1–1.4: config/state persistence hardening. Touch only `config/mod.rs`, `commands/`.
- Story 4.3: `AppState::save_config_locked` choke-point. Touch only `config/` and `commands/`.
- Story 4.1: `migrate_and_normalize` extraction from `load_config_reporting`. Touch only `config/mod.rs`.
- Stories 3.1–3.4: test-integrity fixes. Touch only `audio/mod.rs`, `commands/feedback.rs`,
  `pipeline.rs` (WAV-RMS tests), and `tests/pi_security/judge.rs`.

**This story modifies ONLY `src-tauri/src/pipeline.rs`.**

### Current `ProcessInput` body (HEAD, post-Epic-3)

```
905  pub struct ProcessInput {
906      pub wav_bytes: Vec<u8>,
907      pub language: String,
908      pub stt_provider: Arc<dyn SttProvider>,
909      pub cleanup_provider: Arc<dyn CleanupProvider>,
910      /// STT conditioning prompt (dictionary terms + hint), passed to `transcribe`.
911      pub dict_prompt: Option<String>,
912      /// Hint text alone (no dictionary terms), used by the hallucination guards.
913      pub stt_hint_text: String,
914      pub offline_mode: bool,
915      pub selected_text: Option<String>,
916      pub cleanup_style: CleanupStyle,
917      pub custom_prompt: Option<String>,
918      pub matched_profile_name: Option<String>,
919      pub dict_list: Option<String>,
920      pub output_lang: Option<String>,
921      pub llm_provider_name: String,
922      pub config_for_fallback: AppConfig,
923  }
```

Fields `dict_prompt` and `stt_hint_text` must move into `SttPromptPair`.

### Current 5 helpers being demoted

All are in `pipeline.rs`, between lines ~413 and ~531:

```
pub fn compute_wav_rms(wav_bytes: &[u8]) -> Option<f32>    // line ~413
pub fn is_offline(stt_provider: &str, llm_provider: &str) -> bool  // line ~453
pub fn silence_skip(duration_ms, min_recording_ms, rms, threshold) -> Option<SilenceSkip>  // line ~471
pub fn post_stt_skip(transcript: &str, stt_hint: &str) -> Option<PostSttSkip>  // line ~500
pub fn select_llm_path(offline: bool, has_selected_text: bool) -> LlmPath  // line ~523
```

All are called ONLY in-module (production call sites in `stop_and_process_pipeline` / `process_audio`)
and in the in-module `#[cfg(test)] mod tests` block (~2132). In-module tests keep working after
`pub` → `pub(crate)` because test code is in the same crate.

**Verify before changing:** `grep -rn "pipeline::compute_wav_rms\|pipeline::is_offline" src-tauri/src/`
should return zero results outside `pipeline.rs` itself. (If `main.rs` or any other module
calls them via the full path, that would be a compile error after demotion — fix it first.)

### Current `ProcessOutcome` match in `stop_and_process_pipeline`

The match block at ~1499-1544 (the code `deliver_outcome` will encapsulate):

```rust
let (cleaned_text, raw_text, is_command, stt_ms, llm_ms, prompt_tokens, completion_tokens) =
    match outcome {
        ProcessOutcome::Stopped { stt_error } => {
            if stt_error {
                if let Ok(mut m) = state.feedback_metrics.lock() {
                    m.stt_error_count = m.stt_error_count.saturating_add(1);
                }
            }
            return;
        }
        ProcessOutcome::CommandFailed => {
            if let Ok(mut m) = state.feedback_metrics.lock() {
                m.llm_error_count = m.llm_error_count.saturating_add(1);
            }
            consume_command_mode(&state);
            return;
        }
        ProcessOutcome::Produced {
            cleaned_text, raw_text, is_command, stt_ms, llm_ms,
            prompt_tokens, completion_tokens, llm_error,
        } => {
            if llm_error {
                if let Ok(mut m) = state.feedback_metrics.lock() {
                    m.llm_error_count = m.llm_error_count.saturating_add(1);
                }
            }
            if is_command_mode {
                consume_command_mode(&state);
            }
            (cleaned_text, raw_text, is_command, stt_ms, llm_ms, prompt_tokens, completion_tokens)
        }
    };
```

Everything AFTER this block (record usage, paste, history, metrics, done-event) stays in
`stop_and_process_pipeline`. `deliver_outcome` only moves the match arms above.

**Important design constraint:** `deliver_outcome` needs `is_command_mode` (the shell flag,
set before `process_input` is built), not just the `outcome`. Pass it as a parameter.
The function signature should be a regular `fn` (not async) since the match arms contain
no await points — only lock acquisitions and flag resets.

### `deliver_outcome` signature recommendation

```rust
fn deliver_outcome(
    outcome: ProcessOutcome,
    state: &AppState,
    is_command_mode: bool,
) -> Option<(String, String, bool, u64, Option<u64>, Option<u32>, Option<u32>)>
```

Returns `None` on `Stopped`/`CommandFailed` (early return in caller); `Some(...)` on `Produced`.

This avoids making `deliver_outcome` async (no await inside the match arms), avoids passing
`AppHandle` (only `AppState` is needed), and keeps the function testable without a real handle.

### `SttPromptPair` design

```rust
pub struct SttPromptPair {
    /// Combined STT conditioning prompt (dictionary terms + hint text), passed to `transcribe`.
    pub dict_prompt: Option<String>,
    /// Hint text alone (no dictionary terms), used by the hallucination guards.
    pub stt_hint_text: String,
}
```

Visibility: `pub` (same as `ProcessInput` fields) is acceptable. `pub(crate)` also fine.
Keep it in the same file as `ProcessInput` (do not create a new module or file).

### Call sites to update for `SttPromptPair`

1. **Construction site** (`stop_and_process_pipeline`, ~1379-1387):
   ```rust
   // BEFORE
   let dict_prompt = stt::build_stt_prompt_with_hint(...);
   let stt_hint_text = stt_hint.unwrap_or_else(...);
   // ...later in ProcessInput { ... dict_prompt, stt_hint_text, ... }

   // AFTER
   let stt_prompt = SttPromptPair {
       dict_prompt: stt::build_stt_prompt_with_hint(...),
       stt_hint_text: stt_hint.unwrap_or_else(...),
   };
   // ...later in ProcessInput { ... stt_prompt, ... }
   ```

2. **Destructuring site** (`process_audio`, ~976-992):
   ```rust
   // BEFORE
   let ProcessInput { ..., dict_prompt, stt_hint_text, ... } = input;

   // AFTER
   let ProcessInput { ..., stt_prompt: SttPromptPair { dict_prompt, stt_hint_text }, ... } = input;
   // or: let ProcessInput { ..., stt_prompt, ... } = input;
   //     let (dict_prompt, stt_hint_text) = (stt_prompt.dict_prompt, stt_prompt.stt_hint_text);
   ```

3. **Test construction site** (`make_input` in `#[cfg(test)] mod tests`, ~2423-2441):
   ```rust
   // BEFORE
   dict_prompt: None,
   stt_hint_text: TEST_STT_HINT.to_string(),

   // AFTER
   stt_prompt: SttPromptPair {
       dict_prompt: None,
       stt_hint_text: TEST_STT_HINT.to_string(),
   },
   ```

### Existing test coverage to preserve

All existing tests in `#[cfg(test)] mod tests` (~2132) must keep passing:

- `test_offline_flag_*` (5 tests, ~2144-2205) — call `is_offline` directly; still work after
  `pub` → `pub(crate)` (same module).
- `test_silence_skip_*` (6 tests, ~2207-2250) — call `silence_skip` directly; still work.
- `test_post_stt_skip_*` (3 tests, ~2253-2275) — call `post_stt_skip` directly; still work.
- `test_select_llm_path_*` (3 tests, ~2277-2293) — call `select_llm_path`; still work.
- `test_decision_matrix_snapshot` (~2296) — uses all 4 above; still works.
- `test_process_audio_*` (8 async tests, ~2454-2665) — call `process_audio` with `ProcessInput`;
  the `make_input` helper needs updating for `SttPromptPair` (Task 2).
- `spec_wav_rms_vectors_json` + named WAV-RMS wrappers (~3391-3505) — call `compute_wav_rms`;
  still work after demotion (same module).
- `sanitize_*` tests (~3512-3568) — call `sanitize_llm_output`; unaffected.
- `test_resolve_*` provider tests (~2667-2770) — unaffected.

### Inversion-Check Discipline (Epic-3 retro AI-1 — DoD)

This is a **behavioral-preserving refactor**: no new guards are being introduced, so there
may be no new tests required. However, if any new test is written (e.g. to characterize
`deliver_outcome` branching), apply the discipline:

1. Flip the guarded invariant (e.g. swap `Stopped` / `Produced` return values).
2. Confirm the test fails (RED).
3. Restore the correct code.
4. Commit only the correct + verified tests.

Log the result in Completion Notes even if the finding is "no new tests added."

### Behavior-Preserving Checklist

Before closing, run these specific existing tests and confirm green:

- `cargo test --lib pipeline` — all pipeline tests pass.
- `cargo test --lib test_process_audio_normal_cleanup` — core happy path.
- `cargo test --lib test_process_audio_stt_failure` — `Stopped { stt_error: true }` path.
- `cargo test --lib test_process_audio_command_failure` — `CommandFailed` path.
- `cargo test --lib spec_wav_rms_vectors_json` — WAV-RMS parity fixture.
- `cargo test --lib` — full suite (≥557 tests, 0 fail).

### DoD

- **Linux (load-bearing):** `cargo test --lib` passes (all ≥557 existing tests, 0 fail).
  `cargo clippy --lib` clean on touched files (zero new warnings on `pipeline.rs`).
- **No Windows smoke required.** Pure Rust structural refactor — no new FS primitives, no
  platform-gated code, no shell/surface change. Same class as Stories 4.1 and 4.3, which
  carried the identical rationale.
- **No Android smoke required.** `pipeline.rs` is desktop-only; Kotlin reads the JSON output,
  not this Rust code.

### Project Structure Notes

- **File to modify:** `src-tauri/src/pipeline.rs` ONLY. No other files need touching.
- **New types:** `SttPromptPair` and `deliver_outcome` are added to `pipeline.rs` in-module.
  Do NOT create a new file or submodule.
- **`SttPromptPair` location:** Place it immediately before or after the `ProcessInput` struct
  definition (~line 905). Keep it in the top-level module, not inside `mod tests`.
- **`deliver_outcome` location:** Place it immediately after `stop_and_process_pipeline`
  (or just before it), in the top-level module, NOT inside `mod tests`.
- **Test changes:** Only `make_input` in `mod tests` needs updating (Task 2). Do not touch
  any other test.

### References

- Epic 4 / DEPTH-pipeline finding: `_bmad-output/planning-artifacts/epics.md` (§Epic 4, Story 4.2)
- Story 4.1 (equivalent config refactor, same behavior-preserving class): `_bmad-output/implementation-artifacts/4-1-isolate-the-load-config-core-into-a-tested-migrate-and-normalize.md`
- Story 4.3 (save_config_locked choke-point precedent): `_bmad-output/implementation-artifacts/4-3-single-sanctioned-config-write-path-save-config-locked.md`
- Deferred work (pre-existing DEPTH-pipeline context): `_bmad-output/implementation-artifacts/deferred-work.md` (§"From Task 2.2")
- Epic-3 retro AI-1 (inversion-check at writing time): `_bmad-output/implementation-artifacts/epic-3-retro-2026-06-02.md`
- Current `ProcessInput`: `src-tauri/src/pipeline.rs:905-927`
- Current `ProcessOutcome`: `src-tauri/src/pipeline.rs:932-953`
- 5 helpers to demote: `src-tauri/src/pipeline.rs:413, 453, 471, 500, 523`
- Match arms to extract: `src-tauri/src/pipeline.rs:~1499-1544`
- Existing tests: `src-tauri/src/pipeline.rs:2132-3568` (`#[cfg(test)] mod tests`)
- project-context.md: `_bmad-output/project-context.md` (v1-ship rules, `pub(crate)` guidance)

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

- Baseline clippy on HEAD: 2 pre-existing warnings on `pipeline.rs` (lines 41 + 598).
- Post-change clippy: same 2 pre-existing warnings only. The new `deliver_outcome` return type
  triggered `clippy::type_complexity`; suppressed with `#[allow(clippy::type_complexity)]` and
  a rationale comment (tuple mirrors `ProcessOutcome::Produced` fields; named struct is a
  follow-up refactor).
- External-ref grep: zero results for all 5 helper names via `pipeline::` prefix — demotion safe.

### Completion Notes List

**SttPromptPair design:** Used `pub struct SttPromptPair` (same visibility as `ProcessInput`) with
`pub dict_prompt: Option<String>` and `pub stt_hint_text: String`. Placed immediately before
`ProcessInput`. Three sites updated: construction in `stop_and_process_pipeline` (grouped into
`SttPromptPair { dict_prompt, stt_hint_text }`), destructuring in `process_audio` (pattern
`stt_prompt: SttPromptPair { dict_prompt, stt_hint_text }`), and `make_input` in `mod tests`.

**`deliver_outcome` shape:** Regular `fn` (not async — no await points in the match arms).
Signature: `fn deliver_outcome(outcome: ProcessOutcome, state: &AppState, is_command_mode: bool) -> Option<(...)>`.
Placed after `stop_and_process_pipeline` (before `run_dictation_pipeline`). Exact match-arm
logic extracted verbatim — no semantic change. Caller replaced with
`let Some((...)) = deliver_outcome(outcome, &state, is_command_mode) else { return; };`.

**Inversion-check (AC-6):** No new tests were added — this is a behavior-preserving structural
extraction, and AC-6 mandates the flip discipline only *for new tests*. None were written, so no flip
was executed.

**Correction (code review 2026-06-02):** the original note here claimed `test_process_audio_stt_failure` /
`test_process_audio_command_failure` "exercise the `None` paths of `deliver_outcome`" and that flipping
`Stopped → Some(...)` would turn `test_process_audio_stt_failure` RED. **That is false.** Both tests call
`process_audio` (not `stop_and_process_pipeline`) and assert on the `ProcessOutcome` variant that
`process_audio` *produces* — they never reach `deliver_outcome`, so flipping `deliver_outcome`'s arms would
**not** make them go RED. `deliver_outcome` has **no direct unit test**. This is not a coverage regression:
the same match arms were previously inline in `stop_and_process_pipeline`, which has no unit test either
(it needs an `AppState`/`AppHandle`). Correctness of this refactor rests on (a) the extraction being
arm-for-arm verbatim from the original match — confirmed by all three review layers — and (b) the unchanged
557-test suite passing. A direct `deliver_outcome` test (needs an `AppState` fixture) is recorded in
deferred-work.md.

**5 helpers demoted to `pub(crate)`:** `compute_wav_rms`, `is_offline`, `silence_skip`,
`post_stt_skip`, `select_llm_path`. Grep confirmed zero external call sites before demotion.
`cargo build` confirmed zero compile errors after demotion.

**Test results:** 557 lib tests / 0 fail. All 4 specific behavior-preserving checklist tests
(`test_process_audio_normal_cleanup`, `test_process_audio_stt_failure`,
`test_process_audio_command_failure`, `spec_wav_rms_vectors_json`) pass. `cargo build` clean (0 errors).
`cargo clippy --lib` — zero new warnings on `pipeline.rs` vs. pre-existing baseline.

### File List

- `src-tauri/src/pipeline.rs`

### Change Log

- 2026-06-02: Introduced `SttPromptPair` struct; replaced two loose fields in `ProcessInput`;
  extracted `deliver_outcome` from `stop_and_process_pipeline`; demoted 5 helpers to `pub(crate)`.
  557 tests / 0 fail, clippy clean on `pipeline.rs`. (Story 4.2)

## Review Findings

_Code review 2026-06-02 (3 adversarial layers, Opus 4.8): Edge Case Hunter clean (empty), Acceptance
Auditor all 6 ACs PASS, Blind Hunter 12 raised — 10 dismissed as diff-only artifacts resolved by the
project-access layers (visibility unverifiable → 0 external callers + build clean; behavioral-equivalence
"unprovable from hunk" → arm-for-arm verbatim; borrow-form/truncation/naming → pre-existing or compile-proven)._

- [x] [Review][Patch] False AC-6 inversion-check claim in Completion Notes — APPLIED. The note claimed
  `test_process_audio_stt_failure`/`command_failure` exercise `deliver_outcome`'s `None` paths and would
  catch a `Stopped → Some` flip; they call `process_audio`, never reach `deliver_outcome`, so the flip would
  NOT turn them RED. Corrected to state the truth (no direct test; correctness from verbatim extraction +
  unchanged 557-suite). [story Completion Notes — Inversion-check (AC-6)]
- [x] [Review][Defer] `deliver_outcome` has no direct unit test [`pipeline.rs:~1786`] — deferred. Needs an
  `AppState` fixture; not a coverage regression (the arms were previously inline in the untested
  `stop_and_process_pipeline`). Corroborated by Blind Hunter + Acceptance Auditor.
- [x] [Review][Defer] `deliver_outcome` returns a 7-tuple instead of a named struct [`pipeline.rs:~1786`] —
  deferred. Sanctioned by premature-abstraction-guard; tuple field order verified to match
  `ProcessOutcome::Produced` exactly (Edge Case Hunter). `#[allow(clippy::type_complexity)]` carries the
  rationale.
