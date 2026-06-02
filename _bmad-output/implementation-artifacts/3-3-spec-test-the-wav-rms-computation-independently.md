# Story 3.3: Spec-Test the WAV-RMS Computation Independently

Status: done

## Story

As a klarvo maintainer,
I want `compute_wav_rms` covered by known-input→known-output specs AND those same vectors shared with the Kotlin `SilencePreFilter.computeWavRms`,
so that a quantization/normalization bug surfaces as a failing assertion instead of being cemented as an "expected" snapshot, and so that Rust↔Kotlin RMS divergence (the exact 2.2 divisor bug) is detectable by a cross-platform contract.

## Acceptance Criteria

1. **Given** `compute_wav_rms` (`src-tauri/src/pipeline.rs:413-438`) is pure/public and today partly covered by an `insta` golden-master snapshot (`pipeline.rs:3319-3338`),
   **When** this story lands,
   **Then** the computation is covered by independent parametric specs:
   - silence WAV (all-zero i16 samples) → `Some(0.0)`
   - full-scale 440 Hz sine at 1.0 amplitude → `Some(rms)` where `(rms - 1/√2).abs() < 1e-3`
   - a known constant-amplitude speech-level WAV (e.g. amplitude = 0.3) → `Some(rms)` where `rms ≈ 0.3` (within 1e-3)
   - invalid byte slice (not a WAV) → `None`
   - empty byte slice → `None`
   - WAV with empty data chunk (valid header, 0 samples) → `Some(0.0)`

2. **Given** the `insta` snapshot (`pipeline.rs:3319-3338`) pins the implementation rather than the spec,
   **When** this story lands,
   **Then** the snapshot assertion (`insta::assert_debug_snapshot!("compute_wav_rms_sine_tone", rms)`) is removed from `characterize_compute_wav_rms_sine_tone_snapshot` and replaced with a closed-form tolerance assertion.
   **And** the snapshot file `src-tauri/src/snapshots/klarvo_lib__pipeline__tests__compute_wav_rms_sine_tone.snap` is deleted.

3. **And** the specs cover both i16 and float32 WAV sample paths (the function normalizes int by `max_val = (1 << (bits_per_sample-1)) as f32`).

4. **Given** Epic-2-Retro AI-1: "author the vectors as a language-neutral shared fixture (e.g. JSON: WAV-input description → expected RMS), not Rust-inline constants, plus a thin Kotlin/JVM test that consumes the same shared file",
   **When** this story lands,
   **Then** a shared fixture file `test-fixtures/wav-rms-vectors.json` exists at the repo root with an array of test case objects, each specifying: `id`, `description`, `wav_encoding` (how to construct the WAV programmatically), `expected_rms` (float or `null` for None/invalid), and `tolerance`.
   **And** the Rust specs in `pipeline.rs` MUST read `test-fixtures/wav-rms-vectors.json` at test runtime using `serde_json` — hardcoding vector values in Rust (even with a comment) is not permitted; the JSON file is the single source of truth for both sides.
   **And** a thin Kotlin/JVM test `android/kotlin-test/com/klarvo/voice/WavRmsVectorsTest.kt` reads the same `test-fixtures/wav-rms-vectors.json` at test runtime and validates `SilencePreFilter.computeWavRms()` against each vector.

5. **Given** Story 4.2 (DEPTH-pipeline) will demote `compute_wav_rms` from `pub` to `pub(crate)`,
   **When** this story's in-module `#[cfg(test)]` tests land,
   **Then** they keep working after that demotion (in-module tests can access `pub(crate)` items in the same crate — no change needed here, just don't use `super::compute_wav_rms` from an external test).

## Tasks / Subtasks

- [x] Task 1: Create shared fixture file `test-fixtures/wav-rms-vectors.json` (AC: 4)
  - [x] 1.1 Create directory `test-fixtures/` at the repo root (sibling to `src-tauri/`, `android/`)
  - [x] 1.2 Create `test-fixtures/wav-rms-vectors.json` with the following test vector schema:
    ```json
    [
      {
        "id": "RMS-001",
        "description": "empty byte slice — not a WAV at all",
        "wav_encoding": {"type": "raw_bytes", "bytes": []},
        "expected_rms": null,
        "tolerance": null
      },
      {
        "id": "RMS-002",
        "description": "invalid bytes — not a WAV header",
        "wav_encoding": {"type": "raw_bytes", "bytes": [116, 104, 105, 115, 32, 105, 115, 32, 110, 111, 116, 32, 97, 32, 87, 65, 86]},
        "expected_rms": null,
        "tolerance": null
      },
      {
        "id": "RMS-003",
        "description": "silence WAV — 100ms, all-zero i16 samples, 16kHz mono",
        "wav_encoding": {"type": "synthetic", "sample_rate": 16000, "channels": 1, "bits_per_sample": 16, "duration_ms": 100, "amplitude": 0.0},
        "expected_rms": 0.0,
        "tolerance": 0.0
      },
      {
        "id": "RMS-004",
        "description": "full-scale 440Hz sine, 1.0 amplitude, 1s, 16kHz mono — RMS = 1/sqrt(2)",
        "wav_encoding": {"type": "sine", "sample_rate": 16000, "channels": 1, "bits_per_sample": 16, "duration_ms": 1000, "freq_hz": 440.0, "amplitude": 1.0},
        "expected_rms": 0.7071067811865476,
        "tolerance": 1e-3
      },
      {
        "id": "RMS-005",
        "description": "constant amplitude 0.3 (speech level), 200ms, 16kHz mono — RMS ≈ 0.3",
        "wav_encoding": {"type": "synthetic", "sample_rate": 16000, "channels": 1, "bits_per_sample": 16, "duration_ms": 200, "amplitude": 0.3},
        "expected_rms": 0.3,
        "tolerance": 1e-3
      },
      {
        "id": "RMS-006",
        "description": "WAV with valid header but zero data chunk (0 samples) — must return Some(0.0) not None",
        "wav_encoding": {"type": "synthetic", "sample_rate": 16000, "channels": 1, "bits_per_sample": 16, "duration_ms": 0, "amplitude": 0.0},
        "expected_rms": 0.0,
        "tolerance": 0.0
      },
      {
        "id": "RMS-007",
        "description": "float32 WAV, constant amplitude 0.5, 100ms, 16kHz mono — RMS ≈ 0.5",
        "wav_encoding": {"type": "synthetic", "sample_rate": 16000, "channels": 1, "bits_per_sample": 32, "sample_format": "float", "duration_ms": 100, "amplitude": 0.5},
        "expected_rms": 0.5,
        "tolerance": 1e-4
      }
    ]
    ```
    NOTE: The `wav_encoding` field describes the WAV construction algorithm — both Rust and Kotlin test helpers build the actual bytes from this spec; the JSON does NOT embed raw WAV bytes (except for RMS-001/RMS-002 which test invalid input).

- [x] Task 2: Replace insta snapshot test with closed-form specs driven by the shared JSON fixture in `pipeline.rs` (AC: 1, 2, 3, 4, 5)
  - [x] 2.1 In `src-tauri/src/pipeline.rs`, locate the `#[cfg(test)] mod tests` section starting around line 3273 ("Characterization tests for compute_wav_rms")
  - [x] 2.2 Remove the `insta::assert_debug_snapshot!` call from `characterize_compute_wav_rms_sine_tone_snapshot` (around line 3338)
  - [x] 2.3 Delete `src-tauri/src/snapshots/klarvo_lib__pipeline__tests__compute_wav_rms_sine_tone.snap`
  - [x] 2.4 Add a `#[cfg(test)]` helper `load_wav_rms_vectors() -> Vec<serde_json::Value>` that reads `test-fixtures/wav-rms-vectors.json` at runtime
  - [x] 2.5 Add a parametric test `spec_wav_rms_vectors_json` that calls `load_wav_rms_vectors()`, iterates each vector, builds the WAV bytes from `wav_encoding` using the existing `make_wav` / `make_sine_wav` helpers (see Dev Notes), calls `compute_wav_rms`, and asserts against `expected_rms` / `tolerance`. Fail with the vector `id` on mismatch.
  - [x] 2.6 Rename the remaining `characterize_*` tests to `spec_*` to reflect their new nature (the existing individual tests that already cover silence/invalid/speech-level should be kept as readable named specs — they can delegate to the JSON vector under the same id, or remain as named wrappers with an explicit note that the JSON is authoritative).
  - [x] 2.7 Add a float32 WAV path test (vector RMS-007): build a WAV using `hound::SampleFormat::Float` (constant 0.5 samples), call `compute_wav_rms`, assert `(rms - 0.5).abs() < 1e-4`. This MUST be driven from the JSON fixture vector via the parametric test in 2.5 (the JSON defines the expected value; the Rust test reads it from there).
  - [x] 2.8 Add an empty data chunk test (vector RMS-006): build a WAV with 0 samples (valid hound spec, 0 samples written), call `compute_wav_rms`, assert `rms == Some(0.0)`. Also driven from the JSON fixture in the parametric test.

- [x] Task 3: Write Kotlin WavRmsVectorsTest consuming the shared fixture (AC: 4)
  - [x] 3.1 Create `android/kotlin-test/com/klarvo/voice/WavRmsVectorsTest.kt`
  - [x] 3.2 The test reads `test-fixtures/wav-rms-vectors.json` from the repo root. Resolved via multi-fallback `System.getProperty("user.dir")` candidates; primary path is 4 levels up from Gradle :app module CWD.
  - [x] 3.3 Inline minimal recursive-descent JSON parser (Kotlin-only, no extra deps; `org.json` not available in JVM unit test classpath without Android stubs).
  - [x] 3.4 `buildVectorWav` helper implemented: raw_bytes / synthetic / sine / float32 (using `buildFloat32Wav` with audioFormat=3)
  - [x] 3.5 Assertions: null → assertNull; non-null → assertNotNull + tolerance check
  - [x] 3.6 `@Test fun vectors_matchExpectedRms()` parametric test iterates all 7 vectors
  - [x] 3.7 RMS-007 asserts null. NOTE: the implementation previously returned a garbled non-null value (0.348) because it did not validate audioFormat. Added a 1-line audioFormat guard to `SilencePreFilter.computeWavRms` (`if (audioFormat != 1) return null`) to make it correctly return null for float32 WAVs — this is the minimal fix required for the test to pass and for the implementation to correctly document the delta.

- [x] Task 4: Verify cargo test passes (AC: 1, 2, 3)
  - [x] 4.1 Run `cargo test -p klarvo -- pipeline::tests` in `src-tauri/` and verify 0 failures — 86 pipeline tests pass including all 7 spec_* tests
  - [x] 4.2 Confirm the `compute_wav_rms_sine_tone.snap` snapshot is no longer referenced — no `assert_debug_snapshot!` calls remain for this name
  - [x] 4.3 Confirm `cargo test` passes without `INSTA_UPDATE=unseen` env flag — 541 lib tests / 0 fail

- [x] Task 5: Verify Kotlin JVM tests pass (AC: 4)
  - [x] 5.1 Run the JVM unit tests via `./gradlew :app:testUniversalDebugUnitTest --rerun-tasks` in `src-tauri/gen/android/` — 59 JVM tests / 0 failures; `WavRmsVectorsTest.vectors_matchExpectedRms` passes all 7 vectors

## Dev Notes

### What This Story Closes

**TEST-04** (robustness-audit-2026-05-30.md §4): `compute_wav_rms` is covered ONLY by golden-master/insta snapshot; `silence_skip` consumes RMS as a given argument and never tests the computation. A quantization/computation bug would be cemented as "expected snapshot". This story replaces the snapshot with closed-form assertions and adds a language-neutral cross-platform contract.

**Epic-2-Retro AI-1** (epic-2-retro-2026-06-02.md §7): The 2.2 code-review caught that `computeWavRms` in Kotlin divided by the header-claimed `sampleCount` instead of the count actually read — a correctness bug that was invisible without a cross-platform contract. This story implements the "parity down-payment" explicitly requested in the retro.

### Rust JSON Fixture Reading — Key Detail for Task 2.4–2.5

`serde_json` is already a dependency in `src-tauri/Cargo.toml`. The test helper `load_wav_rms_vectors()` (Task 2.4) resolves the fixture path via `CARGO_MANIFEST_DIR` (always set during `cargo test`; points to `src-tauri/`). Parent = workspace root = where `test-fixtures/` lives.

For the parametric test (Task 2.5), the dev agent must build WAV bytes from each vector's `wav_encoding` object. Use these dispatch rules:
- `"type": "raw_bytes"` → convert `bytes` JSON array to `Vec<u8>` directly (covers RMS-001 empty, RMS-002 garbage)
- `"type": "synthetic"` with `amplitude` → generate `duration_ms * sample_rate / 1000` samples all at `amplitude`, pass to `make_wav` (16-bit int, existing helper) — covers RMS-003, RMS-005, RMS-006
- `"type": "sine"` → generate `duration_ms * sample_rate / 1000` samples as sine at `freq_hz`, pass to `make_wav` — covers RMS-004
- `"type": "synthetic"` with `"sample_format": "float"` → build 32-bit IEEE float WAV using `hound::SampleFormat::Float`; the existing `make_wav` only does 16-bit int, so the dev agent must add a `make_float_wav(samples: &[f32]) -> Vec<u8>` helper — covers RMS-007

The existing `make_wav` helper is at `pipeline.rs:3283`. There is no `make_float_wav` yet — add it in the same `#[cfg(test)]` module.

The speech-level test (`characterize_compute_wav_rms_speech_level_above_default_threshold`) currently only checks `rms > threshold` without asserting the exact value. When driven from the JSON fixture (RMS-005, `expected_rms: 0.3`, `tolerance: 1e-3`), the assertion becomes `(rms - 0.3).abs() < 1e-3`. Update the named spec test accordingly.

### Files to MODIFY

| File | Change |
|---|---|
| `src-tauri/src/pipeline.rs` | Replace insta snapshot test with closed-form specs; add float32 test; add empty-chunk test |
| `src-tauri/src/snapshots/klarvo_lib__pipeline__tests__compute_wav_rms_sine_tone.snap` | DELETE |

### Files to CREATE

| File | Purpose |
|---|---|
| `test-fixtures/wav-rms-vectors.json` | Language-neutral shared test vector fixture |
| `android/kotlin-test/com/klarvo/voice/WavRmsVectorsTest.kt` | Thin Kotlin consumer of the shared vectors |

### The Existing Code Being Modified

**`compute_wav_rms` in `pipeline.rs:413-438`** — already public, already pure. The function:
1. Opens a `hound::WavReader` on the byte slice — returns `None` if parse fails
2. If `SampleFormat::Float`: collects f32 samples directly
3. If `SampleFormat::Int`: reads i32 samples, normalizes by `(1_i64 << (bits_per_sample - 1)) as f32`
4. Returns `Some(0.0)` if samples is empty (after parse success)
5. Returns `Some(audio::compute_rms(&samples))` otherwise

**Existing tests in `pipeline.rs:3273-3370`** (the "Characterization tests for compute_wav_rms" block):
- `characterize_compute_wav_rms_silence_is_zero` — passes (already spec-level), rename to `spec_*`
- `characterize_compute_wav_rms_sine_tone_snapshot` — HAS the insta snapshot; REMOVE the snapshot call, keep the closed-form `(rms - expected).abs() < 1e-3`
- `characterize_compute_wav_rms_invalid_bytes_returns_none` — passes (already spec-level), rename
- `characterize_compute_wav_rms_empty_bytes_returns_none` — passes, rename
- `characterize_compute_wav_rms_speech_level_above_default_threshold` — passes; consider making the expected RMS explicit (not just "> threshold") if adding a constant-amplitude 0.3 case

**`make_wav` helper (pipeline.rs:3282-3302)** — already exists in the test module; the Kotlin side already has `buildTestWav` in `SilencePreFilterTest.kt` that does the same thing. Both encode 16kHz/mono/16-bit PCM. Reuse both helpers.

**The decision-matrix snapshot** (`klarvo_lib__pipeline__tests__decision_matrix_snapshot.snap`) — DO NOT touch this. It is a DIFFERENT snapshot pinning `silence_skip` decision output, not `compute_wav_rms`. It has an explicit comment: "review it deliberately, never blind-accept the snapshot." Leave it intact.

### Shared Fixture Path Handling (Kotlin)

The JVM unit test runner (called from `android/` or `android/kotlin-test/`) sets the working directory to the Android project root (`android/`). The repo root is one level up: `../test-fixtures/wav-rms-vectors.json`. If that doesn't resolve, use `../../test-fixtures/wav-rms-vectors.json` (relative to `android/kotlin-test/`). The safest approach: `File(System.getProperty("user.dir")).resolve("../test-fixtures/wav-rms-vectors.json")`. Print the resolved path in the test if you get a FileNotFound.

If the JVM test runner is invoked from the repo root (e.g. via `scripts/android-build.sh`), use `test-fixtures/wav-rms-vectors.json` directly.

**Fallback**: If path resolution is fragile in CI, copy the fixture into `android/kotlin-test/resources/wav-rms-vectors.json` and read it via `javaClass.classLoader.getResourceAsStream("wav-rms-vectors.json")`. Document whichever path you use.

### Float32 WAV path in Kotlin

`SilencePreFilter.computeWavRms` currently only handles 16-bit int samples. Vector RMS-007 uses `audioFormat=3` (IEEE float). The Kotlin code will encounter these bytes and likely return `null` (the WAV header bytes 20-21 will be 0x03 0x00 instead of 0x01 0x00, and the `getShort` parsing of PCM data will still run but with misinterpreted bytes). The test for this vector in Kotlin MUST assert `null` — it is documenting a known limitation, not a bug to fix in this story.

For the Rust float32 test (Task 2.5): build the WAV using hound's `SampleFormat::Float` spec. `hound::WavSpec { sample_format: SampleFormat::Float, bits_per_sample: 32, .. }` — then write f32 samples with `writer.write_sample(0.5f32)`. The `compute_wav_rms` Rust function handles this path via `SampleFormat::Float` arm.

### How to Run JVM Tests

```bash
cd /home/andyon2/workspace/products/klarvo/android
# Run all Kotlin unit tests
kotlinc -cp kotlin-src kotlin-test -include-runtime -d test.jar 2>&1
# OR if there's a test runner script:
./gradlew :app:testDebugUnitTest   # (if running inside the full Android Gradle project)
```

Check the existing `android/kotlin-test/` README or `android/build-tests.sh` if present. The existing tests (SilencePreFilterTest, HallucinationFilterTest, BankingGuardTest, SanitizePathsTest) already run; follow whatever invocation works for them.

### Story 4.2 Compatibility

Story 4.2 (DEPTH-pipeline) will demote `compute_wav_rms` from `pub` to `pub(crate)`. The tests in this story are all `#[cfg(test)] mod tests` INSIDE `pipeline.rs` — in-module tests can access `pub(crate)` items freely. No external test binary references `compute_wav_rms`. This story's tests are safe from that future demotion.

### AI-2 Lesson (from Epic-1-Retro and Epic-2-Retro)

All spec tests MUST bind to the real production call site, not mock inputs or indirect proxies:
- Rust: call `compute_wav_rms(&wav_bytes)` with real WAV bytes from the `make_wav` helper
- Kotlin: call `SilencePreFilter.computeWavRms(wavBytes)` with real WAV bytes from `buildTestWav`

Do NOT pass pre-computed RMS floats to `silence_skip` and claim it tests `compute_wav_rms`.

### References

- `src-tauri/src/pipeline.rs:413-438` — `compute_wav_rms` implementation [Source: pipeline.rs]
- `src-tauri/src/pipeline.rs:3273-3370` — existing characterization tests [Source: pipeline.rs]
- `src-tauri/src/snapshots/klarvo_lib__pipeline__tests__compute_wav_rms_sine_tone.snap` — snapshot to delete
- `android/kotlin-src/com/klarvo/voice/SilencePreFilter.kt:57-98` — Kotlin `computeWavRms` [Source: SilencePreFilter.kt]
- `android/kotlin-test/com/klarvo/voice/SilencePreFilterTest.kt` — existing Kotlin tests (pattern reference)
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt:1024` — `encodeWav` (production WAV encoder)
- `docs/robustness-audit-2026-05-30.md §4` — TEST-04 finding
- `_bmad-output/implementation-artifacts/epic-2-retro-2026-06-02.md §7` — AI-1 parity down-payment mandate
- `_bmad-output/planning-artifacts/epics.md` — Epic 3 Story 3.3 (p. 599-623)

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (2026-06-02)

### Debug Log References

- RMS-007 Kotlin assertion: initial run showed `computeWavRms` returned 0.3480291 (garbled) for float32 WAV instead of null. Root cause: no audioFormat validation in `SilencePreFilter.computeWavRms`. Added 1-line guard (`val audioFormat = buf.getShort(20).toInt(); if (audioFormat != 1) return null`). All 18 SilencePreFilterTest cases remain green after the change.
- `org.json.JSONArray/JSONObject` not available in JVM unit tests (Android stubs not loaded). Used inline minimal recursive-descent JSON parser instead.

### Completion Notes List

- Task 1: `test-fixtures/wav-rms-vectors.json` was pre-created (already present). Verified content matches story spec exactly (7 vectors RMS-001..RMS-007).
- Task 2: Replaced entire "Characterization tests for compute_wav_rms" block in `pipeline.rs`. Deleted snapshot file. Added `load_wav_rms_vectors()`, `build_vector_wav()`, `make_float_wav()` helpers. Added `spec_wav_rms_vectors_json` (parametric), 6 named `spec_*` wrappers. Old `characterize_*` names all renamed. 7 new spec tests + 541 total tests / 0 fail.
- Task 3: Created `WavRmsVectorsTest.kt` with inline JSON parser, multi-fallback path resolution, and `buildVectorWav` / `buildFloat32Wav` helpers. Added audioFormat guard to `SilencePreFilter.computeWavRms` (minimal fix to make null-assertion pass for RMS-007). 59 JVM tests / 0 fail.
- AC-5 (in-module test compatibility): all `spec_*` tests are inside `pipeline.rs` `#[cfg(test)] mod tests` — they access `compute_wav_rms` via `super::` scope and will survive any `pub` → `pub(crate)` demotion in Story 4.2.

### File List

- `test-fixtures/wav-rms-vectors.json` (pre-existing, verified)
- `src-tauri/src/pipeline.rs` (modified — replaced characterize block with spec tests + helpers)
- `src-tauri/src/snapshots/klarvo_lib__pipeline__tests__compute_wav_rms_sine_tone.snap` (DELETED)
- `android/kotlin-src/com/klarvo/voice/SilencePreFilter.kt` (modified — added audioFormat guard)
- `android/kotlin-test/com/klarvo/voice/WavRmsVectorsTest.kt` (created)

### Change Log

- 2026-06-02: Story 3.3 implemented. Replaced insta snapshot with closed-form JSON-driven parametric spec tests in Rust (7 spec_* tests). Created Kotlin WavRmsVectorsTest consuming the same JSON fixture. Added audioFormat=3 guard to SilencePreFilter.computeWavRms (null for float32 WAVs). Deleted snapshot file. 541 Rust lib tests + 59 JVM tests / all pass.
- 2026-06-02: Addressed 3 code review findings (F2, F1, F3). (F2) Added `expected_rms_kotlin: null` + `divergence_reason` to RMS-007 in wav-rms-vectors.json; WavRmsVectorsTest.kt now reads `expected_rms_kotlin` from fixture instead of hardcoded `if (id == "RMS-007")` branch — asymmetry is first-class contract data. (F1) Fixed stale/contradictory class-doc and method-doc in WavRmsVectorsTest.kt that described pre-fix world (audioFormat guard now exists, test passes). (F3) Corrected misleading Rust comment on named spec_* wrappers in pipeline.rs — they hardcode constants; only the parametric `spec_wav_rms_vectors_json` reads the JSON. 541 Rust tests + 59 JVM tests / 0 fail.

## Review Findings

_Code review 2026-06-02 (Opus 4.8, 3 adversarial layers: Blind Hunter / Edge Case Hunter / Acceptance Auditor). 1 decision-needed, 2 patch, 1 defer-bundle, rest dismissed. The GATE-2-ratified production guard (float32→null in SilencePreFilter) is NOT re-opened._

- [x] [Review][Patch] (resolved from Decision → **Option B**, Andi 2026-06-02) Lift the float32 Rust↔Kotlin asymmetry into the shared fixture: add a per-platform expectation field to RMS-007 in `test-fixtures/wav-rms-vectors.json` (e.g. `expected_rms_kotlin: null` + a short `divergence_reason`), have `WavRmsVectorsTest.kt` read that field instead of the hardcoded `if (id == "RMS-007")` branch, and remove the id special-case. The asymmetry becomes first-class contract data (per AI-1); the JSON stays the single source of truth for both sides. Rust continues to read `expected_rms` (0.5) [WavRmsVectorsTest.kt:254-267, test-fixtures/wav-rms-vectors.json].
- [x] [Review][Patch] Stale/contradictory comments in WavRmsVectorsTest.kt claim "the impl does NOT validate audioFormat … assertion WILL FAIL until audioFormat validation is added" — but that validation was added in this same diff; the test passes. Comments describe a pre-fix world [WavRmsVectorsTest.kt:20-25, 238-240, 254-267].
- [x] [Review][Patch] Rust named-wrapper tests hardcode constants (1/√2, 0.3, 0.5) while their comment claims each "delegates its authoritative value from the JSON fixture" — only the parametric `spec_wav_rms_vectors_json` actually reads the JSON (and it satisfies AC-4); reword the comment or make the wrappers read the fixture [pipeline.rs:3417-3418, 3463-3464].
- [x] [Review][Defer] Test-helper robustness + speculative future-fixture hardening (minimal Kotlin JSON parser escape/number/EOF handling; `bits==32` float-vs-int conflation; raw_bytes range validation; path-resolution first-match; `tested>=7` not asserting specific IDs; Kotlin amplitude no-clamp; guard's offset-20/canonical-44-byte assumption + no bits/channels check — pre-existing, Android `encodeWav` always emits canonical 16-bit mono PCM; `make_float_wav` doc "audioFormat=3" vs hound EXTENSIBLE claim, unverified, no behavioral impact) — deferred, all gated on a clean machine-written fixture and unreachable production paths.
