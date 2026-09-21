# Story 13-2 — inversion evidence (code)

**48 inversions, all RED**, in four batches. Each batch names the tree it was
measured at: the first pass presented one total over "the shipped tree", but R1-R8
had been measured mid-build, before the last two Rust tests existed (their evidence
lines say `767 filtered out` where the later ones say `768`). That framing was a
review finding; the Rust batch below is a **re-run against the current tree**, so no
row here is older than the code it claims to pin.

Every row is a real edit to production source followed by a real test run, then a
revert. Both gates are green after the last revert (`cargo test --lib` **774 passed**;
JVM gate **29 suites / 268 tests**, 0 failures) and `git status` carries no stray edit.

## Rust — the shared core (`cd src-tauri && cargo test --lib <filter>`)

*Measured at: current tree (cargo test --lib: 774 passed) — re-run 2026-09-21 after the review round*

| # | Deliberate break | File | Filter | RED | Evidence |
|---|---|---|---|---|---|
| R1 | guard_transcript runs the ghost strip BEFORE the fragment strip | `src-tauri/src/pipeline.rs` | `spec_guard_order_strips_fragments_before_the_verdict` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 773 filtered out; finished in 0.00s` |
| R2 | guard_transcript loses the pre-guard ghost strip (the pre-13-2 Desktop chain) | `src-tauri/src/pipeline.rs` | `spec_guard_strips_a_raw_ghost_before_the_blocklist` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 773 filtered out; finished in 0.00s` |
| R3 | guard_transcript_for_jni hands a dropped transcript back as text instead of None | `src-tauri/src/stt/groq_jni.rs` | `spec_jni_guard_wrapper_agrees_with_the_desktop_chain` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 773 filtered out; finished in 0.00s` |
| R4 | build_stt_prompt_with_hint goes back to the separator-less join | `src-tauri/src/stt/mod.rs` | `spec_stt_prompt_join_is_explicit` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 773 filtered out; finished in 0.00s` |
| R5 | select_stt_hint_override loses the auto fall-through (the obvious-reading twin) | `src-tauri/src/stt/mod.rs` | `spec_stt_hint_override_selection_falls_through_to_auto` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 773 filtered out; finished in 0.00s` |
| R6 | terminal_degrade_cause narrows back to `Some(_) if paste_failed` | `src-tauri/src/pipeline.rs` | `spec_clipboard_failure_on_a_clean_run_names_the_shipped_cause` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 773 filtered out; finished in 0.00s` |
| R7 | offline_rule_with drops the platform-availability clause (D-M21 returns) | `src-tauri/src/pipeline.rs` | `spec_offline_rule_matches_the_fixture_matrix` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 773 filtered out; finished in 0.00s` |
| R8 | offline_rule_with drops the local-STT clause (G2a returns) | `src-tauri/src/pipeline.rs` | `spec_offline_rule_matches_the_fixture_matrix` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 773 filtered out; finished in 0.00s` |
| R9 | frame_decision restores the `if energy_ok` guard around the predictor call | `src-tauri/src/vad/mod.rs` | `spec_predictor_runs_on_a_sub_floor_frame` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 773 filtered out; finished in 0.00s` |
| R10 | `guard_transcript_for_jni` hands a DROPPED transcript back as text instead of `None` | `src-tauri/src/stt/groq_jni.rs` | `spec_jni_guard_wrapper_agrees_with_the_desktop_chain` | **RED** | `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 776 filtered out; finished in 0.00s` — **re-run 2026-09-21 by the follow-up review.** The row previously read "the same break [as R9], measured against the wrapper-agreement test" and carried `767 filtered out`, i.e. a 768-test tree, under a batch header claiming every row had been re-measured against the shipped tree. Two defects in one row: the description named a VAD break that cannot make this test fail, and the evidence was older than the code. Both corrected by actually performing the break the test can witness and recording what came out. |

## Kotlin — the Android twins (device-free JVM gate, `--rerun-tasks --tests <t>`)

*Measured at: tree at commit a995ca7 (JVM 28 suites / 253 tests). Behaviour unchanged by the two later rounds; the tests these rows name were not edited.*

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

*Measured at: tree at commit 967609f (JVM 29 suites / 260 tests)*

A Matrix Test Audit found four I/O & Edge-Case Matrix rows — **D11**, **B1-Android**,
**D6-Android** and the flush-time half of **E1** — whose only evidence was "read the
diff". `OverlayServiceSourceContractTest` covers them.

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

## Review round — the decisions that were revertible with every gate green

*Measured at: current tree (cargo test --lib: 774 passed; JVM 29 suites / 268 tests)*

A review found decisions that no gate compiled or no test read, so reverting them left
every gate green — the same class of defect this story is about, one level up. The
remedy each time was to move the decision into a plain-Rust helper or a pure companion
seam and assert it, never to add prose.

| # | Deliberate break | File | Expected-red test | RED |
|---|---|---|---|---|
| V1 | guard_hint_for_jni ignores customPrompt (B4's Android half goes inert) | `src-tauri/src/stt/groq_jni.rs` | `spec_jni_guard_hint_is_the_hint_and_not_the_built_prompt` | **RED** |
| V2 | stt_error_sentinel moves the ResponseFormat arm BELOW the catch-all (D-M5 returns) | `src-tauri/src/stt/groq_jni.rs` | `spec_jni_response_format_sentinel_precedes_the_network_catch_all` | **RED** |
| V3 | the Rust emptiness check drops .trim() (whitespace-only diverges from Kotlin again) | `src-tauri/src/llm/mod.rs` | `spec_whitespace_only_answer_is_rejected_like_an_empty_one` | **RED** |
| V4 | process_frame re-inlines the energy guard around the predictor call (D-L19 undone one level up) | `src-tauri/src/vad/mod.rs` | `spec_feed_calls_the_predictor_on_every_sub_floor_frame` | **RED** |
| V5 | offline_passthrough reintroduces a SECOND rule inside cleanup_text (D-M20 verbatim) | `src-tauri/src/commands/recording.rs` | `spec_in_app_button_offline_returns_the_raw_text_unchanged` | **RED** |
| V6 | parseSttPrompt reads the key from the config ROOT instead of `advanced` (B4 inert) | `KlarvoApi.kt` | `sttPromptKeysAreReadFromTheRealAdvancedObject` | **RED** |
| V7 | guardedClipboardWrite narrows back to catch (Exception) — an Error reaches the looper | `KlarvoOverlayService.kt` | `clipboardWriteAlsoCatchesAnError` | **RED** |
| V8 | the cleanup-ladder gate gets its `e is IOException &&` pre-filter back (D10 returns) | `KlarvoOverlayService.kt` | `cleanupLadderGateHasNoTypePreFilter` | **RED** |
| V9 | one G2a guard goes back to its own "local" literal | `KlarvoOverlayService.kt` | `theThreeOfflineGuardsShareOneProviderIdLiteral` | **RED** |
| V10 | mapCleanupResponse stops trimming (whitespace-only answers are delivered again) | `KlarvoApi.kt` | `whitespaceOnlyAnswerIsRejectedLikeAnEmptyOne` | **RED** |
| V11 | the ghost-strip bridge call loses its guard (a stale .so kills the worker thread) | `KlarvoOverlayService.kt` | `theGhostStripBridgeCallDegradesInsteadOfKillingTheRun` | **RED** |
| V12 | the delivery block reads KlarvoAccessibilityService.instance twice again | `KlarvoOverlayService.kt` | `theLiveAccessibilityReferenceIsReadOnce` | **RED** |

## Four inversions came back GREEN first — that is what they are for

Each exposed a test defect, fixed before the tables above were recorded.

1. **`guard_transcript_for_jni` returned a dropped transcript as text** and the
   wrapper-agreement test stayed green: every guard vector asserted SURVIVAL, so the skip
   arm was never taken. Fixed with two DROP vectors, a new test, and a count assertion so
   the wrapper test cannot agree vacuously.
2. **The dictation call site was fed `config.customPrompt` again** and the D-H4 tripwire
   stayed green, because the live-preview call site still satisfied its `contains`. The
   assertion now counts `transcribeWithRetry` call sites against hint arguments and
   rejects any `config.customPrompt` line that is not the LLM's `customInstructions`.
3. **The ghost-strip bridge call lost its guard** and stayed green: the test accepted any
   earlier `try {` in the file, and `processAudio` has several. It now requires the call
   to be the FIRST statement of its own `try`.
4. **The delivery block read `KlarvoAccessibilityService.instance` twice again** and
   stayed green: nothing asserted the single read at all. `theLiveAccessibilityReferenceIsReadOnce`
   now does.

**Two further rows first went RED by COMPILE ERROR, which proves nothing about the test**
(A3: moving the history block ahead of `handler.post` left the `captured*` locals out of
scope; A4: `run { … }.start()` does not type-check). Both were rewritten until the
assertion itself failed, and only the recompiled forms are recorded.

## What is still NOT inverted, and why

- **A REAL `setPrimaryClip` failure.** Not producible on a JVM host, and the test provider
  cannot inject it; ADR-0016 Amendment 4 records D6 as agent-verified only. The catch
  SEMANTICS are executed through `guardedClipboardWrite` — including an `Error`, since the
  row promises "never an uncaught main-thread exception" — and the counter wiring is
  tripwired.
- **The `extern "system"` bodies themselves.** After the review round they contain only
  unmarshalling, the Tokio runtime and the return; every DECISION they used to hold is now
  a plain-Rust helper with its own test (`select_stt_provider`, `guard_hint_for_jni`,
  `stt_error_sentinel`, `guard_transcript_for_jni`).
- **That the device behaves this way.** A source tripwire proves the code says the right
  thing, never that HyperOS does it. The device half of B1-Android, D4, D6 and E1 is on
  Andi's list in `verdict.md`.
