# Story 13-1b — inversion evidence for the Rust and Kotlin guards

Every guard and vector this story added or reshaped was **executed RED** before its
green run was believed. Each row below is a real edit to the tree, a real test run,
and a revert — driven mechanically, not asserted in prose. The tree was re-confirmed
green after the last revert (`cargo test --lib` 749 passed; JVM gate 25 suites / 217
tests / 0 failures).

The harness's own four inversion groups are in `inversion-report.md`.

## Rust (`cd src-tauri && cargo test --lib <filter>`)

| # | Inversion (the edit) | File | Test that must go RED | Result |
|---|---|---|---|---|
| 1 | config step (f) drops the LLM value allowlist check | `src-tauri/src/config/mod.rs` | `spec_unknown_test_provider_value_normalizes_to_off` | **RED** ✓ |
| 2 | "debug" is put back on VALID_LLM_PROVIDERS / VALID_STT_PROVIDERS | `src-tauri/src/config/mod.rs` | `spec_old_shape_13_1_config_lands_on_a_real_provider_and_an_off_test_provider` | **RED** ✓ |
| 3 | testProviderLlm loses its camelCase serde name | `src-tauri/src/config/mod.rs` | `spec_test_provider_keys_survive_a_real_config_file_round_trip` | **RED** ✓ |
| 4 | truncated is added to the STT value allowlist | `src-tauri/src/config/mod.rs` | `spec_truncated_is_llm_only_and_is_normalized_away_on_the_stt_chain` | **RED** ✓ |
| 5 | resolve_cleanup_provider loses its early return | `src-tauri/src/pipeline.rs` | `spec_test_provider_llm_key_selects_the_test_provider` | **RED** ✓ |
| 6 | resolve_stt_provider loses its early return | `src-tauri/src/pipeline.rs` | `spec_test_provider_stt_key_selects_the_test_provider` | **RED** ✓ |
| 7 | `stt_provider_reload_needed` is forced to return `false` | `src-tauri/src/commands/settings.rs` | `spec_test_provider_stt_change_rebuilds_the_stt_provider` | **RED** ✓ |
| 7b | the `hot_reload_stt_provider(` CALL is deleted from `save_advanced_settings` | `src-tauri/src/commands/settings.rs` | `spec_save_advanced_settings_rebuilds_both_runtime_slots` | **RED** ✓ |
| 8 | select_stt_provider's comparison is flipped | `src-tauri/src/stt/groq_jni.rs` | `spec_android_select_stt_provider` | **RED** ✓ |
| 9 | `off` is removed from the React TEST_PROVIDER_LLM_OPTIONS array | `src/components/AdvancedSettingsPanel.tsx` | `spec_react_test_provider_arrays_are_pinned_to_rust` | **RED** ✓ |
| 10 | the feedback FAB is switched back on | `src/App.tsx` | `spec_react_feedback_fab_is_off_and_its_modal_stays_mounted` | **RED** ✓ |

Evidence per row (the `test result:` line from the inverted run):

```
spec_unknown_test_provider_value_normalizes_to_off: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 747 filtered out; finished in 0.00s
spec_old_shape_13_1_config_lands_on_a_real_provider_and_an_off_test_provider: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 747 filtered out; finished in 0.00s
spec_test_provider_keys_survive_a_real_config_file_round_trip: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 747 filtered out; finished in 0.00s
spec_truncated_is_llm_only_and_is_normalized_away_on_the_stt_chain: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 747 filtered out; finished in 0.00s
spec_test_provider_llm_key_selects_the_test_provider: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 747 filtered out; finished in 0.00s
spec_test_provider_stt_key_selects_the_test_provider: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 747 filtered out; finished in 0.06s
spec_test_provider_stt_change_rebuilds_the_stt_provider: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 747 filtered out; finished in 0.00s
spec_android_select_stt_provider: test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 745 filtered out; finished in 0.00s
spec_react_test_provider_arrays_are_pinned_to_rust: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 747 filtered out; finished in 0.00s
spec_react_feedback_fab_is_off_and_its_modal_stays_mounted: test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 747 filtered out; finished in 0.00s
```

## Kotlin (device-free JVM gate, `:app:testUniversalDebugUnitTest --rerun-tasks`)

| # | Inversion (the edit) | File | Test that must go RED | Result | Tests that actually failed |
|---|---|---|---|---|---|
| 1 | the license gate stops forcing testProviderLlm off | `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` | `licenseGate_forcesBothTestKeysOffWhenUnlicensed` | **RED** ✓ | licenseGate_forcesBothTestKeysOffWhenUnlicensed |
| 2 | resolveLlmProvider's early-return condition is flipped | `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` | `resolveLlmProvider_testProviderWinsWithoutAnyApiKey` | **RED** ✓ | resolveLlmProvider_testProviderWinsWithoutAnyApiKey, resolveLlmProvider_oldShapeDebugNameResolvesToDeepSeekNotADeadProvider |
| 3 | the test branch moves BELOW val url = URL(provider.url) | `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` | `testProviderBranchIsTakenBeforeTheProviderUrlIsBuilt` | **RED** ✓ | testProviderBranchIsTakenBeforeTheProviderUrlIsBuilt |

## Correction, 2026-09-21 (review)

Row 7 previously read **"save_advanced_settings stops rebuilding the STT slot"**.
That is not what was inverted, and the distinction matters:
`spec_test_provider_stt_change_rebuilds_the_stt_provider` drives
`hot_reload_stt_provider` **directly**, so deleting the CALL to it from the command
leaves the whole suite green (measured after the review: 748 passed, 0 failed) — the
helper stays referenced by the test module's `use super::{…}`, so not even a dead-code
warning fires. The one wire carrying this story's headline AC was recorded as covered
while nothing executed it.

Row 7 now records what was actually edited (`stt_provider_reload_needed` forced to
`false`), and row **7b** is the new source-text tripwire
`spec_save_advanced_settings_rebuilds_both_runtime_slots`, which reads the function's
own body and asserts BOTH `hot_reload_*_provider(` calls are present. It was inverted by
deleting the call and went RED with the expected message. The call-site gap itself (an
executing test needs a Tauri `AppState`) is filed in `docs/backlog.md`.


## Round 2 — the guards added by the 2026-09-21 review fixes

Same method: real edit, real run, real revert.

| # | Inversion (the edit) | File | Test that must go RED | Result |
|---|---|---|---|---|
| R1 | `effective_llm_provider_name` reverts to the CONFIGURED provider name | `src-tauri/src/pipeline.rs` | `spec_test_provider_is_never_a_cleanup_fallback_candidate` | **RED** ✓ |
| R2 | the test-provider clause moves BACK below the Windows local-model guard | `src-tauri/src/commands/settings.rs` | `spec_save_advanced_settings_rebuilds_both_runtime_slots` | **RED** ✓ |
| R3 | `sticky bottom-0` is dropped from the embedded Advanced footer | `src/components/AdvancedSettingsPanel.tsx` | `spec_react_test_provider_arrays_are_pinned_to_rust` | **RED** ✓ |
| R4 | the two test-provider rows are CROSSED (each writes the other's key) | `src/components/AdvancedSettingsPanel.tsx` | `spec_react_test_provider_arrays_are_pinned_to_rust` | **RED** ✓ |
| R5 | the `hot_reload_stt_provider(` CALL is deleted from `save_advanced_settings` | `src-tauri/src/commands/settings.rs` | `spec_save_advanced_settings_rebuilds_both_runtime_slots` | **RED** ✓ |
| R6 | the FAB block is moved BELOW the guard's closing `)}` | `src/App.tsx` | `spec_react_feedback_fab_is_off_and_its_modal_stays_mounted` | **RED** ✓ |
| K1 | `parseTestProvider` drops the value allowlist | `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` | `jsonParse_testProvider_normalizesOutOfSetValuesToOffPerChain` | **RED** ✓ |
| K2 | the Android local-STT branch stops consulting testProviderStt | `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` | `localSttBranchIsTakenOnlyWhileTheTestProviderIsOff` | **RED** ✓ |
| K3 | the Kotlin STT allowlist wrongly keeps `truncated` | `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` | `valueAllowlistsMatchTheFixture` | **RED** ✓ |

⚠️ **One of these could not be inverted on this host and is pinned as source text instead.**
The `cleanup_provider_reload_needed` ordering defect is WINDOWS-ONLY: on Linux the
`cfg!(target_os = "windows")` early return never fires, so every runtime assertion about
the predicate passes with the clause on either side of it. The ordering is therefore
pinned inside `spec_save_advanced_settings_rebuilds_both_runtime_slots` by reading the
function body, and THAT is what row R2 inverts.

## What these inversions do NOT cover

- Anything that only a real device or a real Windows build can show. No APK was built,
  no `.so` was linked, nothing ran on the Xiaomi or on Windows.
- The JNI arity itself: no gate loads `libklarvo_lib.so`, so the 8-parameter Kotlin
  declaration and the 8-parameter Rust entry point are checked by nothing that executes
  (pre-existing gap, `docs/backlog.md`).
- Persistence through the real Tauri command: `save_advanced_settings` needs an
  `AppState`. Its two reload predicates and both hot-reload swaps are driven directly;
  the two CALLS inside the command are pinned only as source text (row 7b), not executed.
