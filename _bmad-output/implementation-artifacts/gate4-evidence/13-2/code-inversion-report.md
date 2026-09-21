# Story 13-2 — inversion evidence (code)

**36 inversions, all RED**: 10 Rust, 18 Kotlin, 8 Kotlin matrix-audit follow-up.

Every new or reshaped guard, sentinel, predicate and terminal decision was broken
deliberately, run, shown RED, and reverted. Each row is a real edit to production
source followed by a real test run, not a reading. The tree was re-confirmed green
after the last revert (`cargo test --lib` 768 passed; JVM gate 29 suites / 260 tests,
0 failures) and `git status` carries no stray edit.

Driver scripts: `invert_rust.py` / `invert_kotlin.py` (scratchpad, not committed —
they are mechanical apply/run/revert loops over the tables below).

## Rust (`cd src-tauri && cargo test --lib <filter>`)

| # | Deliberate break | File | Filter | RED | Evidence |
|---|---|---|---|---|---|
| R1 | guard_transcript runs the ghost strip BEFORE the fragment strip | `src-tauri/src/pipeline.rs` | `spec_guard_order_strips_fragments_before_the_verdict` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 766 filtered out; finished in 0.00s` |
| R2 | guard_transcript loses the pre-guard ghost strip (the pre-13-2 Desktop chain) | `src-tauri/src/pipeline.rs` | `spec_guard_strips_a_raw_ghost_before_the_blocklist` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 766 filtered out; finished in 0.00s` |
| R3 | build_stt_prompt_with_hint goes back to the separator-less join | `src-tauri/src/stt/mod.rs` | `spec_stt_prompt_join_is_explicit` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 766 filtered out; finished in 0.00s` |
| R4 | select_stt_hint_override loses the auto fall-through (the obvious-reading twin) | `src-tauri/src/stt/mod.rs` | `spec_stt_hint_override_selection_falls_through_to_auto` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 766 filtered out; finished in 0.00s` |
| R5 | terminal_degrade_cause narrows back to `Some(_) if paste_failed` | `src-tauri/src/pipeline.rs` | `spec_clipboard_failure_on_a_clean_run_names_the_shipped_cause` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 766 filtered out; finished in 0.00s` |
| R6 | offline_rule_with drops the platform-availability clause (D-M21 returns) | `src-tauri/src/pipeline.rs` | `spec_offline_rule_matches_the_fixture_matrix` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 766 filtered out; finished in 0.00s` |
| R7 | offline_rule_with drops the local-STT clause (G2a returns) | `src-tauri/src/pipeline.rs` | `spec_offline_rule_matches_the_fixture_matrix` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 766 filtered out; finished in 0.00s` |
| R8 | frame_decision restores the `if energy_ok` guard around the predictor call | `src-tauri/src/vad/mod.rs` | `spec_predictor_runs_on_a_sub_floor_frame` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 766 filtered out; finished in 0.00s` |
| R9 | guard_transcript_for_jni hands a dropped transcript back as text instead of None | `src-tauri/src/stt/groq_jni.rs` | `spec_guard_still_drops_an_echo_and_a_hallucination` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 767 filtered out; finished in 0.00s` |
| R10 | the same break, measured against the wrapper-agreement test | `src-tauri/src/stt/groq_jni.rs` | `spec_jni_guard_wrapper_agrees_with_the_desktop_chain` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 767 filtered out; finished in 0.00s` |

## Kotlin (device-free JVM gate, `./gradlew :app:testUniversalDebugUnitTest --rerun-tasks --tests <t>`)

| # | Deliberate break | File | Expected-red test | RED |
|---|---|---|---|---|
| K1 | mapCleanupResponse stops rejecting an empty answer | `KlarvoApi.kt` | `emptyAnswerIsRejectedAsNonRetryable` | **RED** |
| K2 | mapCleanupResponse stops inspecting finish_reason | `KlarvoApi.kt` | `truncatedAnswerIsRejectedAsNonRetryable` | **RED** |
| K3 | isRetryableCleanupFailure drops the JSONException arm (D-M2 returns) | `KlarvoOverlayService.kt` | `malformedAnswerThrowsAJsonException_andNowFiresTheLadder` | **RED** |
| K4 | isRetryableCleanupFailure drops the two named non-retryable arms | `KlarvoOverlayService.kt` | `emptyAnswerIsRejectedAsNonRetryable` | **RED** |
| K5 | classifySttSentinel loses the __ERROR_FORMAT: arm (falls to the catch-all) | `KlarvoOverlayService.kt` | `responseFormatSentinelIsNonRetryable` | **RED** |
| K6 | transcribeWithRetry goes back to the 3-attempt / 2s+5s budget | `KlarvoOverlayService.kt` | `retryBudgetIsOneAttempt` | **RED** |
| K7 | decideDelivery ignores the paste outcome (decides on `instance != null` again) | `KlarvoOverlayService.kt` | `pasteAttemptedButNoFocusedField_showsClipboardToastAndNoCheck` | **RED** |
| K8 | decideDelivery lets a cleanup failure count as success (the DONE flash returns) | `KlarvoOverlayService.kt` | `cleanupFailure_getsNoDoneFlash` | **RED** |
| K9 | decideDelivery ignores a failed clipboard write | `KlarvoOverlayService.kt` | `clipboardWriteFailed_showsNothingAndClaimsNothing` | **RED** |
| K10 | shouldInstallPreviewFlush drops the sttProvider clause (E1 returns) | `KlarvoOverlayService.kt` | `ShouldInstallPreviewFlushTest` | **RED** |
| K11 | hangoverFired goes back to `>=` (fires on the N-th frame) | `KlarvoAudioRecorder.kt` | `hangoverEdgeVector_matchesHangoverFired` | **RED** |
| K12 | vadGateDecision skips the predictor below the energy gate (Android adopts the old Desktop shape) | `KlarvoAudioRecorder.kt` | `engineCallVector_predictorRunsOnSubFloorFrame` | **RED** |
| K13 | skipsCloudCleanup drops the platform-availability clause (D-M21 returns) | `KlarvoOverlayService.kt` | `offlineRuleMatchesTheFixtureMatrix` | **RED** |
| K14 | skipsCloudCleanup drops the local-STT clause (D-H10 returns) | `KlarvoOverlayService.kt` | `offlineRuleMatchesTheFixtureMatrix` | **RED** |
| K15 | the post-cleanup ghost strip call is removed (B3's Android half returns) | `KlarvoOverlayService.kt` | `postCleanupGhostStripGoesThroughTheJniBridge` | **RED** |
| K16 | selectSttHintOverride loses the auto fall-through (the obvious-reading twin) | `KlarvoApi.kt` | `sttHintOverrideSelectionMatchesTheRustTwin` | **RED** |
| K17 | the dictation call site is fed config.customPrompt again (D-H4 returns) | `KlarvoOverlayService.kt` | `theJniGetsTheSttHintAndNotTheCleanupInstruction` | **RED** |
| K18 | only the LIVE-PREVIEW call site is fed config.customPrompt again | `KlarvoOverlayService.kt` | `theJniGetsTheSttHintAndNotTheCleanupInstruction` | **RED** |

## Kotlin — Matrix Test Audit follow-up (four rows that had no covering test)

A Matrix Test Audit found four I/O & Edge-Case Matrix rows — **D11**, **B1-Android**,
**D6-Android** and the flush-time half of **E1** — whose only evidence in the first pass
was "read the diff". `OverlayServiceSourceContractTest` covers them; D6's catch semantics
run through a real seam (`KlarvoOverlayService.guardedClipboardWrite`, the `vadGateDecision`
parameter pattern), the other three are order-anchored source tripwires.

| # | Deliberate break | File | Expected-red test | RED |
|---|---|---|---|---|
| A1 | D11: "No speech detected" is toasted again (a recognition result speaks) | `KlarvoOverlayService.kt` | `nothingRecognizedIsSilent_butCaptureAndConfigFaultsStillSpeak` | **RED** |
| A2 | D11 other half: "No audio recorded" is removed too (the capture fault goes silent) | `KlarvoOverlayService.kt` | `nothingRecognizedIsSilent_butCaptureAndConfigFaultsStillSpeak` | **RED** |
| A3 | B1-Android: the history/Turso worker moves back ahead of the banking verdict (Steps 3/3b) | `KlarvoOverlayService.kt` | `bankingVerdictPrecedesTheHistoryAndTursoWrites` | **RED** |
| A4 | B1-Android: the writes run inline on the main looper instead of a worker thread | `KlarvoOverlayService.kt` | `theWritesAreStillOffTheMainLooper` | **RED** |
| A5 | D6: guardedClipboardWrite rethrows instead of catching (the uncaught main-thread throw returns) | `KlarvoOverlayService.kt` | `clipboardWriteIsCaughtCountedAndNeverPropagates` | **RED** |
| A6 | D6: copyToClipboard's failure handler stops incrementing pasteErrorCount | `KlarvoOverlayService.kt` | `copyToClipboardRoutesThroughTheSeamAndCountsTheFailure` | **RED** |
| A7 | E1: the flush-time re-check moves BELOW the delta snapshot | `KlarvoOverlayService.kt` | `previewFlushRechecksTheStoredSttProviderBeforeItTouchesAudio` | **RED** |
| A8 | E1: the flush-time re-check only logs and no longer returns | `KlarvoOverlayService.kt` | `previewFlushRechecksTheStoredSttProviderBeforeItTouchesAudio` | **RED** |

**Two of these first went RED by COMPILE ERROR, which proves nothing about the test**, and
were rewritten until the assertion itself failed: moving the history/Turso block ahead of
`handler.post` left the `captured*` locals out of scope (the pre-13-2 shape uses the local
names, so the inversion now substitutes them), and turning `Thread { … }.start()` into
`run { … }.start()` does not type-check (the inversion now drops `.start()` too). Both are
recorded above in their recompiled, assertion-RED form.

Vacuity discipline for this batch: every assertion is either an ORDER assertion — which
needs both needles to exist and cannot be satisfied by a second occurrence elsewhere, the
failure mode that made two earlier tripwires pass — or two-sided (A1 vs A2: the toasts that
must be gone AND the ones that must remain).

## Two inversions that came back GREEN first, and what they changed

An inversion that passes is the point of running them. Two did, and both were
test defects, fixed before the table above was recorded:

1. **`guard_transcript_for_jni` handed a dropped transcript back as text** and
   `spec_jni_guard_wrapper_agrees_with_the_desktop_chain` stayed green. Cause: every
   guard vector in the fixture asserted SURVIVAL — the direction this story widened —
   so the wrapper's skip arm was never taken. Fix: two DROP vectors
   (`GUARD-ECHO-DROP-001`, `GUARD-BLOCKLIST-DROP-001`), a new test
   `spec_guard_still_drops_an_echo_and_a_hallucination`, and a count assertion in the
   wrapper test so it can no longer agree vacuously. Both now go RED (R9, R10).

2. **The dictation call site was fed `config.customPrompt` again** and
   `theJniGetsTheSttHintAndNotTheCleanupInstruction` stayed green. Cause: the tripwire
   asked `src.contains("sttHintFor(config)")`, and the live-preview call site still
   contained it. Fix: the assertion now counts `transcribeWithRetry` call sites and
   requires the same number of hint arguments, and rejects any `config.customPrompt`
   line that is not the LLM's `customInstructions`. Both call sites now go RED
   separately (K17, K18).

## What was NOT inverted, and why

- **`stt/groq_jni.rs`'s `__ERROR_FORMAT:` arm and the `nativeTranscribe` hint feed.**
  Everything inside `nativeTranscribe` is `#[cfg(target_os = "android")]`; no Linux test
  compiles it, so breaking it changes nothing a gate can see. What IS inverted is the
  half that decides behaviour: the Kotlin classifier (K5) and the call-site tripwires
  (K17, K18). The Rust arm's placement before the catch-all is a reading of the match,
  recorded here as such.
- **A REAL `setPrimaryClip` failure.** Not producible on a JVM host, and the test
  provider cannot inject it; ADR-0016 Amendment 4 records D6 as agent-verified only. The
  catch SEMANTICS are executed through `guardedClipboardWrite` (A5) and the counter
  wiring is tripwired (A6), so what remains unverified is the device, not the code.
- **That the device really behaves this way.** A source tripwire proves the code says the
  right thing, never that HyperOS does it. The device half of B1-Android, D4, D6 and E1
  is on Andi's list in `verdict.md`.

*(This list was longer after the first pass. The Matrix Test Audit rows A1-A8 above closed
the "diff-verified only" entries for D11, B1-Android, D6-Android and E1's flush-time
re-check.)*

