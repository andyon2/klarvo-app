# Cross-platform parity fixtures

These JSON files are the shared golden vectors that keep the Rust (desktop) and Kotlin (Android)
implementations from silently drifting apart: one fixture, read by a test on **both** sides.

**There is no CI in this repo — nothing runs these automatically.** They are manual gates, and two
commands run the whole net: `scripts/android-smoke.sh` covers the Kotlin half (it syncs
`android/kotlin-src/` + `android/kotlin-test/` into the generated project and runs
`./gradlew :app:testUniversalDebugUnitTest` as a hard gate before it will install anything — note it
needs a connected device or emulator to reach that gate, and that gradle treats the test task as
up-to-date when only a *fixture* file changed, so add `--rerun-tasks` after a fixture-only edit or
you will read a stale green), and `cargo test --lib` in `src-tauri/` covers the Rust half.

Fixtures currently in the net:

| Fixture | Rust reader | Kotlin reader |
|---|---|---|
| `chunking-cleanup-vectors.json` | `llm/mod.rs` (`spec_chunking_vectors_*`) | `ChunkingVectorsTest` |
| `wav-rms-vectors.json` | `pipeline.rs` | `VadGateRmsFixtureTest` |
| `vad-gate-golden-vectors-7-2.json` | `vad/mod.rs` (`spec_hangover_fires_on_the_frame_after_the_required_count`, `spec_predictor_runs_on_a_sub_floor_frame` — the two rows story 13-2 added; the `energy-floor` and `stop-latency` rows stay Kotlin-only, they pin Kotlin-side formulas) | `VadGateGoldenVectorsTest` |
| `twin-constants-vectors.json` | `llm/mod.rs` (`spec_twin_constants_*`) | `TwinConstantsVectorsTest` (asserts 9 of 10 entries — `TWIN-CLEANUP-MODEL-ANTHROPIC-001` is Desktop-only, drift row H5, and is skipped by id) |
| `m12-dictionary-scope-vectors.json` | `llm/mod.rs` (`spec_m12_*`) | — (Android column is a written record: the Kotlin prompt builders are private) |
| `test-provider-scenario-vectors.json` | `llm/mod.rs` (`spec_test_llm_*`, `spec_test_provider_option_lists_*`) + `stt/mod.rs` (`spec_test_stt_*`) | `TestProviderScenarioTest` (asserts the 7 `surface: "llm"` entries; the 7 `surface: "stt"` entries are n/a on Android by construction — STT is shared Rust core per ADR-0017 — and are skipped by surface, as are the 2 `surface: "provider-options"` entries, which the Rust reader checks against `VALID_TEST_PROVIDER_LLM` / `VALID_TEST_PROVIDER_STT` and the throwaway desktop proxy harness checks against the rendered DOM). `SttSentinelClassificationTest` additionally reads `TEST-STT-EMPTY-001`'s sentinel name and retry budget |
| `guard-chain-vectors.json` | `pipeline.rs` (`spec_guard_*`, `spec_stt_prompt_join_is_explicit`, `spec_stt_hint_override_selection_falls_through_to_auto`, `spec_jni_guard_wrapper_agrees_with_the_desktop_chain`) | `GuardChainBridgeTest` (the guard VERDICTS are n/a on Android by construction — ADR-0017 makes them shared Rust core, reached over `GroqSttBridge`; the Kotlin half asserts the DELEGATION and drives the one vector that is a real twin, `STT-HINT-SELECT-*`) |
| `offline-rule-vectors.json` | `pipeline.rs` (`spec_offline_rule_matches_the_fixture_matrix`, `spec_offline_rule_platform_constants_match_the_fixture`) | `OfflineRuleVectorsTest` |
