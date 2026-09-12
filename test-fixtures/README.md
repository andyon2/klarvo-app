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
| `vad-gate-golden-vectors-7-2.json` | — (Kotlin-only) | `VadGateGoldenVectorsTest` |
| `twin-constants-vectors.json` | `llm/mod.rs` (`spec_twin_constants_*`) | `TwinConstantsVectorsTest` (asserts 9 of 10 entries — `TWIN-CLEANUP-MODEL-ANTHROPIC-001` is Desktop-only, drift row H5, and is skipped by id) |
| `m12-dictionary-scope-vectors.json` | `llm/mod.rs` (`spec_m12_*`) | — (Android column is a written record: the Kotlin prompt builders are private) |
