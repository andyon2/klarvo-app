# Story 7.2: Android live auto-stop VAD-gate parity

Status: done

<!-- Test-Architect REQUIRED before dev-story: run *risk + *design on this story (can truncate user speech — see epic Test-Architect note). See Dev Notes → "Pre-dev: Test-Architect gate". -->

## Story

As a klarvo user on Android,
I want live auto-stop driven by my configured thresholds, like on Desktop,
so that recording doesn't cut me off mid-sentence and Expert-mode tuning actually takes effect.

## Context & Governing Decisions

This is **Epic 7** (Cross-Platform Parity) story 7.2 — a brownfield **parity fix**, not a feature.
It covers audit rows **H1, H17, M2, M3, M4, L1** from `docs/cross-platform-drift-audit.md`, per
`_bmad-output/planning-artifacts/epics-cross-platform-parity.md#Story 7.2`.

**Scope boundary (ADR-0017):** the live real-time VAD gate stays a **Kotlin-only** fix by design —
ADR-0017's Hard Rule (shared STT/guard logic only in Rust over JNI) is **STT-only**; moving the
realtime frame stream over JNI was explicitly rejected as a large lift with speech-truncation risk
(`docs/adr/0017-shared-core-stt-path.md`, Scope section). Do **not** pull any part of this story's
fix into Rust/JNI. **All H1/H17/M3/M4/L1 fixes are Kotlin-only, in `KlarvoAudioRecorder.kt`.**

**M2 is a different mechanism, already partly touched by Story 7.3 — read this carefully:**
The epic doc and the audit cite `SilencePreFilter.kt:26` for M2 ("pre-STT `minRecordingMs`
hardcoded"). **`SilencePreFilter.kt` no longer exists** — Story 7.3 (done, 2026-06-13) deleted it
and replaced the pre-STT skip with a JNI call to the shared Rust `silence_skip`
(`GroqSttBridge.nativeSilenceCheck`). The call site now lives at
**`KlarvoOverlayService.kt:1847-1854`**, and it passes **hardcoded literals `500L, 0.005f`** instead
of reading `cachedConfig`. Story 7.3's own dev notes explicitly named this a **deferred, not
fixed**, follow-up (`groq_jni.rs:337-341` doc comment: *"Config-driven values are a follow-up
(deferred, not wired now)"*). **M2's fix target is this Kotlin call site, not a new Rust change** —
the JNI function (`Java_com_klarvo_voice_GroqSttBridge_nativeSilenceCheck`,
`src-tauri/src/stt/groq_jni.rs:343-349`) already accepts `min_recording_ms`/`silence_threshold` as
parameters; only the Kotlin caller needs to stop hardcoding them.

**Engine/architecture facts (load-bearing, do not re-litigate):**
- The live VAD gate (`processVadFrame` in `KlarvoAudioRecorder.kt`) and the pre-STT skip filter
  (`nativeSilenceCheck` call, post-recording) are **two separate mechanisms**. Rows H1/H17/M3/M4/L1
  belong to the live gate; M2 belongs to the pre-STT skip filter. Do not conflate them — they are
  fixed in different files with different mechanics, even though both are "silence" concepts.
- `KlarvoAudioRecorder.kt`'s energy-gate **threshold value** itself is **already config-driven**
  (Story 9-11 fixed the old hardcoded `0.02f` before this story existed) — H1's residual gap is
  narrower than the audit text implies: the threshold *value* now matches, but the **signal it's
  compared against** doesn't (Android computes RMS on raw, unfiltered samples; Rust computes it on
  highpass-filtered samples — this is really the M3 gap surfacing inside H1). See AC1 for the exact
  remaining delta.
- Out of scope (explicitly, per the epic doc's note): **M1** (whisper-mode threshold swap — depends
  on whisper-mode, which is C2/backlog) and **M5** (full 4-state VAD state machine — accepted,
  DIV-14, backlog). Do not implement either.

## Acceptance Criteria

### AC1 — H1: VAD-path RMS is computed on the same (highpass-filtered) signal as Desktop

**Given** the energy-gate threshold value is already config-driven (`energyGateThreshold`,
constructor-injected from `KlarvoOverlayService.kt`'s `silenceThreshold` field, itself read from
`config.advanced.silenceThreshold`, defaulting to `0.005f` — matching Rust's
`default_silence_threshold()`),
**When** the live VAD gate computes RMS for the energy-gate comparison (`processVadFrame`,
`KlarvoAudioRecorder.kt:386-388`),
**Then** the RMS is computed on the **same signal** Rust compares against — i.e. **after** the AC3
(M3) highpass filter is applied to the frame, not on the raw unfiltered frame,
**And** the energy-gate comparison (`isEnergyAboveGate`, `KlarvoAudioRecorder.kt:96-97`) keeps its
existing threshold semantics (`normalizedRms >= threshold`) — only the RMS *input* changes, not the
comparison function or the config wiring.

**Test:** `SilenceThresholdTest.kt` already covers `isEnergyAboveGate` boundary/threshold cases —
extend it (or add a sibling test) asserting the gate now evaluates RMS computed from filtered
samples, using a synthetic tone that would pass the gate unfiltered but fail it filtered (or vice
versa) to prove the filter is actually in the RMS path, not just present in the file.

### AC2 — H17: 200ms silence→stop hangover floor (not a ~32ms single-frame floor)

**Given** Rust applies `hangover_ms.max(200)` uniformly to both the one-shot AUTOSTOP/AUTO silence
config (`src-tauri/src/audio/mod.rs:1072-1076`) and the preview-flush config
(`src-tauri/src/audio/mod.rs:1180-1184` — same `.max(200)`), converting it to a frame count via
`ceil(hangover_ms / frame_ms)` where `frame_ms = 32.0` (`vad/mod.rs:239-242`) — i.e. a floor of
`ceil(200/32) = 7` frames, regardless of how small the user's configured silence window is,
**And** Android's `framesForSeconds` (`KlarvoAudioRecorder.kt:119-120`,
`(secs * VAD_FRAMES_PER_SECOND).toInt().coerceAtLeast(1)`) only floors at **1 frame** (~32ms at the
current integer fps — see AC6/L1) and is shared by both `requiredSilentFrames`
(AUTOSTOP/AUTO hangover threshold, lines 152-153) and `previewRequiredSilentFrames`
(preview-pause threshold, lines 160-161),
**When** this story lands,
**Then** `framesForSeconds` (or the specific call sites deriving from it) enforces the same
**200ms-equivalent minimum frame floor** as Rust, computed via `ceil` against the corrected fps
(AC6) — matching Rust's uniform application to both the autostop/auto path and the preview-flush
path. **DECIDED (GATE 1, Andi, 2026-09-09): apply the floor uniformly through the shared
`framesForSeconds`, i.e. preview-pause timing changes too — exactly as Rust does. Do not add a
separate autostop-only helper.**

**Test:** extend `PreviewPauseFramesTest.kt` (currently asserts monotonicity, not the exact
value/floor) and add a case for `requiredSilentFrames`/`framesForSeconds` asserting a very small
`silenceSecs` (e.g. `0.05f`) still yields at least the floor frame count, with an inversion check
(revert the floor → the test goes red).

### AC3 — M3: 85 Hz highpass filter before VAD (bass-bleed rejection)

**Given** Rust applies an 85 Hz Butterworth highpass filter to every sample before both RMS/energy
computation and the Silero VAD call (`src-tauri/src/vad/mod.rs:267-272`, `highpass_cutoff_hz: 85.0`
default at `vad/mod.rs:88`),
**And** Android currently feeds the raw, unfiltered frame directly into both `calculateRms` and the
Silero `vad?.isSpeech(frame)` call (`KlarvoAudioRecorder.kt:386-390`), with zero highpass
implementation anywhere in the file,
**When** this story lands,
**Then** a Kotlin port of the same 85 Hz Butterworth highpass filter runs on each incoming frame
**before** it is used for RMS (AC1) and before it is passed to the Silero VAD (`vad.isSpeech`),
**And** the filter state persists correctly across frames (a highpass filter has memory — do not
reset it per-frame; mirror Rust's `SileroVad`-owned filter instance lifecycle, which persists for
the life of a recording session and is reset only via `reset()`, `vad/mod.rs:292-296`),
**And** the display-path RMS (`calculateRms` call at `KlarvoAudioRecorder.kt:312`, feeding
`onAmplitude`/`smoothedAmplitude`) stays on the RAW signal. **DECIDED (GATE 1, Andi, 2026-09-09):
filter only the VAD-gate path. Desktop computes its level callback from raw chunk samples in
`build_stream_with_level` (`audio/mod.rs`); the highpass lives inside `SileroVad` only. Mirror that.**

**Test:** a bass-tone-only synthetic frame (e.g. 60 Hz) should fail the energy gate after filtering
even if its raw RMS would pass; a speech-band tone (e.g. 300–3000 Hz) should be attenuated
negligibly. Port or adapt Rust's highpass unit tests (`vad/mod.rs`) as reference expected values —
grep for the highpass test module before writing new fixtures from scratch.

### AC4 — M4: RMS computed on normalized samples with the same effective scale as Desktop

**Given** Rust normalizes each sample to f32 `[-1,1]` (dividing by `i16::MAX` = `32767`) **before**
computing RMS (`src-tauri/src/audio/mod.rs:1315-1322`, `766`),
**And** Android's `calculateRms` (`KlarvoAudioRecorder.kt:566-573`) computes the sum-of-squares on
raw (unnormalized) `Short` values in a `Double` accumulator, and only divides the **final RMS
result** by `32768f` (not `32767`) at the call site (`KlarvoAudioRecorder.kt:387`),
**When** this story lands,
**Then** the normalization divisor is corrected to `32767` (`Short.MAX_VALUE`) to match Rust's
`i16::MAX` exactly — note this is a ~0.003% scale correction, separate from and smaller than the
M3 filtering gap, which is the larger source of measurable RMS divergence,
**And** the order of operations (normalize-then-sum-of-squares vs. sum-of-squares-then-normalize)
is **not required to change** — they are algebraically equivalent for the final scaled RMS value
(the constant factors out of the sum and the square root cleanly); do not introduce a per-sample
division loop purely for "order of operations parity" unless a golden-vector test proves a
measurable numeric divergence beyond the divisor fix,
**And** existing `Double`-accumulator precision in `calculateRms` is preserved (it is not a bug —
`Double` accumulation is strictly more precise than Rust's all-`f32` pipeline; narrow to `Float`
only at the same point the current code already does).

**Test:** reuse/extend `test-fixtures/wav-rms-vectors.json` (already has an `expected_rms_kotlin`
field slot per vector — check whether it is populated for existing vectors and populate it for any
new/adjusted ones) to lock the corrected divisor against known-input→known-output cases (silence →
0.0, full-scale sine → 1/√2, speech-level constant amplitude → the constant).

### AC5 — M2: Pre-STT `minRecordingMs`/`silenceThreshold` are config-driven, not hardcoded

**Given** `KlarvoOverlayService.kt:1847-1854` currently calls
`GroqSttBridge.nativeSilenceCheck(wavBase64ForFilter, 500L, 0.005f)` with **hardcoded** literals,
**And** the Rust desktop reference reads both values from config
(`src-tauri/src/pipeline.rs:1506-1526`, `adv.min_recording_ms` / `adv.silence_threshold`,
`AdvancedSettings` fields `min_recording_ms: u32` default `500` and `silence_threshold: f32`
default `0.005`, JSON keys `minRecordingMs`/`silenceThreshold`),
**And** the Kotlin `AppConfig` data class (`KlarvoApi.kt`) currently has **no `minRecordingMs`
field** (only `silenceThreshold` exists, already read into `KlarvoOverlayService`'s `silenceThreshold`
field per AC1's config wiring),
**When** this story lands,
**Then** `AppConfig` gains a `minRecordingMs: Long` (or `Int`, matching Rust's `u32` semantics —
choose the type that avoids silent truncation) field, parsed from `config.json`'s
`advanced.minRecordingMs` key with a `500` default (mirror the existing `silenceThreshold` parse
pattern at `KlarvoApi.kt` ~lines 98-109 field declaration, ~339-362 JSON parse, ~438-440
constructor-call),
**And** the `nativeSilenceCheck` call site (`KlarvoOverlayService.kt:1847-1854`) reads
`cachedConfig?.minRecordingMs` and `silenceThreshold` (the field already populated per AC1) instead
of the hardcoded `500L, 0.005f` literals, with the same `500`/`0.005` values as a null-safe fallback
default (not a behavior change if config is somehow absent),
**And** the stale `groq_jni.rs:337-341` doc comment (which claims the values are "passed as fixed
constants... NOT read from config.json") is updated to reflect the new config-driven call, and its
stale `KlarvoOverlayService.kt:952` line reference is corrected to the current line.

**Test:** a config round-trip test (mirroring `RecordingModeSilenceSelectionTest.kt`'s pattern of
an independent expected-value table, not comparing the SUT to itself) asserting a non-default
`minRecordingMs` in config changes the value passed to `nativeSilenceCheck`.

### AC6 — L1: VAD frames-per-second uses the exact 31.25 fps (ceil), not truncated integer 31

**Given** Rust computes `frame_ms = 512/16000*1000 = 32.0` exactly, giving an implicit
`1000/32 = 31.25` fps used only at the point of converting a duration into a frame count via
`.ceil()` (`src-tauri/src/vad/mod.rs:239-242`),
**And** Android hardcodes `VAD_FRAMES_PER_SECOND = 31` (`KlarvoAudioRecorder.kt:132`, truncated,
not the exact `31.25`), feeding both `framesForSeconds` (frame-count conversion) and a cosmetic
~1-second logging window boundary (`KlarvoAudioRecorder.kt:398`),
**When** this story lands,
**Then** `framesForSeconds` computes frame counts using the exact `31.25` fps with `ceil` (matching
Rust's rounding direction — under-counting frames means Android currently requires *fewer* frames
of silence than Rust for the same configured duration, a small systematic bias that compounds with
AC2/H17),
**And** the cosmetic logging-window use of the fps constant (line 398) may keep the truncated
integer if changing it has no behavioral effect beyond log cadence — this is not part of the AC,
avoid unnecessary churn there,
**And** this fix and AC2 (H17) are implemented together where they share `framesForSeconds` — do
not fix the fps constant without re-verifying the AC2 floor math still holds with the corrected fps
(the floor is frame-count-based, derived from `ceil(200ms / frame_ms)`, so it is independent of the
fps constant used for the `secs → frames` conversion, but both land in the same function).

**Test:** extend the existing `PreviewPauseFramesTest.kt`/new `framesForSeconds` tests to assert
the exact frame count for a known `silenceSecs` value at the corrected fps (e.g. `2.0f →
ceil(2.0 * 31.25) = 63` frames, not `62`), with an inversion check.

### AC7 — Golden vectors (epic DoD)

**Given** the epic's Test-Architect note requires "energy-floor + stop-latency at default config
and at one tuned config" as golden vectors for this story,
**When** this story lands,
**Then** golden-vector fixtures exist (new or extended under `test-fixtures/`) covering: (a) the
energy-floor gate at the default `silenceThreshold=0.005` and at one deliberately tuned value
(e.g. `0.02`), asserting pass/fail parity between the documented Rust behavior and the Kotlin
implementation; (b) stop-latency (frames-to-stop) at default `silenceSecs` and at one tuned value,
covering the AC2/AC6 floor + fps interaction,
**And** these seed the Story 7.7 golden-vector parity net (do not build the full 7.7 net here — it
consolidates fixtures seeded by every 7.x story, per `epics-cross-platform-parity.md#Story 7.7`).

## Tasks / Subtasks

- [x] **Task 1 — M3: Port the 85 Hz Butterworth highpass filter to Kotlin** (AC3)
  - [x] Implement a stateful highpass filter class/function mirroring `vad/mod.rs`'s filter (find
    and read the exact biquad/Butterworth implementation before porting — the Explore findings
    above only located the call sites, not the filter math itself; read `vad/mod.rs`'s filter
    struct/impl in full).
  - [x] Wire filter instance lifecycle to match `KlarvoAudioRecorder`'s recording-session lifecycle
    (persists across frames within a session, reset on new session start — mirror `start()`/`stop()`).
  - [x] Feed the filtered frame into both RMS (AC1) and Silero (`vad.isSpeech`).
- [x] **Task 2 — H1/M4: Fix RMS computation to use filtered samples + correct divisor** (AC1, AC4)
  - [x] Route `calculateRms`'s VAD-gate call site through the filtered frame from Task 1.
  - [x] Correct the normalization divisor from `32768f` to `32767f` (`Short.MAX_VALUE`) at the
    energy-gate call site (`KlarvoAudioRecorder.kt:387`).
  - [x] Resolve the elicitation-report question on the display-path RMS (`:312`) before touching it.
- [x] **Task 3 — L1: Correct VAD_FRAMES_PER_SECOND to 31.25 (exact, ceil at use)** (AC6)
  - [x] Change the fps constant/calc so `framesForSeconds` uses `31.25` with `ceil`, not truncated
    `31` with `toInt()`.
- [x] **Task 4 — H17: Add the 200ms-equivalent hangover floor** (AC2)
  - [x] Resolve the elicitation-report scope question (autostop-only vs. shared with preview-pause)
    before changing `framesForSeconds` vs. adding a separate hangover-specific helper.
  - [x] Implement the floor consistent with the resolved scope, computed via `ceil` against the
    AC6-corrected fps.
- [x] **Task 5 — M2: Wire `minRecordingMs`/`silenceThreshold` into the pre-STT JNI call** (AC5)
  - [x] Add `minRecordingMs` to Kotlin `AppConfig` (field, JSON parse, constructor wiring).
  - [x] Replace the hardcoded `500L, 0.005f` at `KlarvoOverlayService.kt:1847-1854` with
    `cachedConfig?.minRecordingMs` / `silenceThreshold`, with `500`/`0.005` as null-safe fallback.
  - [x] Correct the stale doc comment + line reference in `src-tauri/src/stt/groq_jni.rs:337-341`.
- [x] **Task 6 — Tests + golden vectors** (AC1–AC7)
  - [x] Extend `PreviewPauseFramesTest.kt` per AC2/AC6's test note. (Review finding: this
    checkbox previously also claimed `SilenceThresholdTest.kt` was extended -- `git diff
    1d3e31b..HEAD` shows that file untouched; AC1's "or add a sibling test" alternative was
    used instead, via `HighpassFilterTest.kt`.)
  - [x] Add/extend highpass filter tests (new file, e.g. `HighpassFilterTest.kt`).
  - [x] Populate `expected_rms_kotlin` in `test-fixtures/wav-rms-vectors.json` where applicable.
  - [x] Add the config round-trip test for AC5.
  - [x] Seed the AC7 energy-floor + stop-latency golden vectors.
  - [x] Inversion check each fix (revert individually, confirm the corresponding test goes red) —
    established convention from Story 7.1's review fixes; document the table in this story's
    Dev Agent Record.
- [x] **Task 7 — Build + verify**
  - [x] Compile-verify the real Kotlin sources (`kotlin-compiler-embeddable`, device-free — the
    Story 7.1 review established this as the standing minimum gate; do not waive it).
  - [x] Run the new/extended Kotlin unit test suite — device-free, all green.
  - [x] `cargo test` — confirm no Rust behavior change. **Corrected 2026-09-10 (story 7-8,
    R3-P7d):** this bullet originally read "this story should not need Rust changes beyond the
    AC5 doc-comment correction". It ended up carrying two Rust edits: the
    `src-tauri/src/stt/groq_jni.rs` doc-comment correction **and** a new test-only `"silence"`
    arm in `src-tauri/src/pipeline.rs::build_vector_wav`. Both are non-behavioral for production
    (the `pipeline.rs` arm is reachable only from fixture-driven tests), so the "no Rust behavior
    change" verdict stands — but the file count in the original wording did not.
  - [~] **On-device/emulator smoke required** (per `project-context.md`: this touches live audio
    capture + real-time state — not a pure deterministic function like 7.1's chunking fix). Run
    `scripts/android-smoke.sh`, and a real recording session verifying auto-stop timing feels
    correct at default and tuned silence settings (human gate — do not claim this done without it).
    STATUS: build/install smoke run by the dev agent (see Dev Agent Record); the live-speech
    "feels correct" perceptual judgment is explicitly Andi's human gate and is NOT claimed done.

### Review Findings

Code review 2026-09-09 (Blind Hunter + Edge Case Hunter + Acceptance Auditor over `1d3e31b..HEAD`).
The production fixes for AC1–AC6 were verified correct against the Rust source of truth (filter
coefficients, `ceil`/floor frame math, divisor, config wiring). Every confirmed finding below is
about **test binding and comment accuracy**, not about the shipped numeric behavior.

- [x] [Review][Patch] No test binds to the production VAD-gate wiring — reverting the filter, the divisor or the Silero input leaves all 151 tests green [android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:451-463] — `processVadFrame` was never extracted into a pure companion seam (contrary to Dev Notes' explicit instruction), so `HighpassFilterTest.rawAndFilteredRms` (`HighpassFilterTest.kt:96-121`) re-implements the normalize → `HighpassFilter.process` → `calculateRmsFloat` sequence inside the test. This invalidates three rows of the Dev Agent Record inversion table (M3/H1/M4) and violates AC1's test note ("prove the filter is actually in the RMS path, not just present in the file") plus Dev Notes → Testing standards. Fix: extract e.g. `fun vadGateRms(frame: ShortArray, length: Int, filter: HighpassFilter): FloatArray/Float` into the companion, call it from `processVadFrame`, and drive the tests through it.
- [x] [Review][Patch] AC4's divisor is not pinned by any test [android/kotlin-test/com/klarvo/voice/VadGateRmsFixtureTest.kt:31] — the test declares its own `private val divisor = 32767f`; the production `VAD_RMS_NORMALIZATION_DIVISOR` (`KlarvoAudioRecorder.kt:185`) is `private const` and referenced by no test. `inversion_oldDivisor32768_differsFromCorrectedDivisor32767` (`:95-105`) divides by two test-local constants and asserts they differ — an arithmetic identity, not a regression lock. Fix: make the constant `internal`/`@VisibleForTesting` (or expose it via the seam above) and assert on production.
- [x] [Review][Patch] AC3's 85 Hz cutoff is not pinned by any test [android/kotlin-test/com/klarvo/voice/HighpassFilterTest.kt:29-30] — the test hardcodes `cutoffHz = 85f`; production `HIGHPASS_CUTOFF_HZ` (`KlarvoAudioRecorder.kt:180`) is private. Simulation of all four assertions shows any cutoff in ~40–300 Hz passes them (at fc=300 Hz the DC, 1 kHz-peak, bass-flip and speech-band tests are all still green) — a 300 Hz cutoff would gut a low male voice's fundamental with nothing going red. Fix: add a −3 dB corner assertion (85 Hz steady-state gain ≈ 0.707 ± 0.02) driven through the production constant.
- [x] [Review][Patch] AC5's "corrected" line reference is itself already wrong [src-tauri/src/stt/groq_jni.rs:338] — the new doc comment cites `KlarvoOverlayService.kt:1852-1854`; the actual call site is `1866-1869` (1852-1854 is an unrelated `setState(RecordingState.IDLE)` block). AC5 explicitly required correcting the stale `:952` reference; one stale reference was replaced with another. The comment also names `cachedConfig?.minRecordingMs` while the code goes through `resolveMinRecordingMsForSilenceFilter(cachedConfig)`. Fix: cite the symbol names, not line numbers.
- [x] [Review][Patch] AC5's JSON-parse path is untested and the "default" test is a tautology [android/kotlin-src/com/klarvo/voice/KlarvoApi.kt:355-361] — `optJSONObject("advanced")?.optLong("minRecordingMs", 500L)` is executed by no test; `MinRecordingMsConfigTest` constructs `KlarvoApi.Config(...)` directly, so a wrong/misspelled JSON key would make the setting silently inert with the feature reporting green. `defaultConfig_minRecordingMs_is500_matchesRustDefault` (`MinRecordingMsConfigTest.kt:33-37`) passes `500L` in and asserts `500L` back — it would still pass if the declared default became `0L`. Fix: feed a real `config.json` string through the parser and assert `750L`; construct `Config(...)` omitting `minRecordingMs` for the default test.
- [x] [Review][Patch] `VadGateRmsFixtureTest`'s KDoc claims it reads the fixture; it never opens it, and `expected_rms_kotlin` has zero consumers [android/kotlin-test/com/klarvo/voice/VadGateRmsFixtureTest.kt:22-24] — expectations are inline literals (`0.7071067811865476f`, `0.3f`, `1e-3f`), so fixture and test can drift silently. Repo-wide, nothing reads `expected_rms_kotlin`: the only Kotlin reader was deleted in Story 7.3 and the Rust parametric test (`pipeline.rs:4151-4210`) reads `expected_rms` only. RMS-007's `expected_rms_kotlin` is also still `null` and its `divergence_reason` still cites the deleted `SilencePreFilter.computeWavRms`. Fix: actually load the fixture (the reader in `VadGateGoldenVectorsTest` is right there) or drop the false claim and the dead field.
- [x] [Review][Patch] AC7's energy-floor golden vectors exercise none of this story's fixes [test-fixtures/vad-gate-golden-vectors-7-2.json:2-58] — VAD-GATE-001…006 supply a pre-computed `normalized_rms` and assert the bare `>=` in `isEnergyAboveGate`, which AC1 says must NOT change and which `SilenceThresholdTest.kt` already covers. The RMS computation the story actually altered (filtered signal, 32767 divisor) is never fed in. Fix: give the energy-floor vectors raw-signal descriptions (tone freq/amplitude) and route them through the production seam. The stop-latency vectors (VAD-LATENCY-001..003) are sound.
- [x] [Review][Patch] Stale/contradictory KDoc left behind by the AC1/AC2/AC4 changes [android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:131-137] — `framesForSeconds` now carries **two stacked KDoc blocks**; Kotlin attaches only the second, so the first is orphaned and still claims "so the preview slider is never inert" — contradicted by the new uniform 7-frame floor, under which every preview-pause setting ≤ 0.224 s collapses to the same 7 frames. Also `isEnergyAboveGate`'s `@param normalizedRms` at `:94` still reads "raw RMS / 32768" (now filtered RMS / 32767), and the `processVadFrame` KDoc at `:425-430` still describes `SILENCE_THRESHOLD` and raw per-chunk RMS. Fix: merge the blocks and correct all three.
- [x] [Review][Patch] A 2 KB `FloatArray` is allocated ~31×/s on the audio recording thread [android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:451] — `val filteredFrame = FloatArray(frame.size)` where `frame.size` is always the compile-time `VAD_FRAME_SIZE`, i.e. ~64 KB/s of garbage in a foreground audio service. Fix: preallocate one scratch `FloatArray(VAD_FRAME_SIZE)` field next to `vadRingBuffer`, which already uses exactly that pattern.
- [x] [Review][Patch] Task 6 checkbox overstates what was done — line 244 claims `SilenceThresholdTest.kt` was extended; `git diff 1d3e31b..HEAD` shows the file untouched and the File List correctly omits it. AC1 permitted "or add a sibling test", so the AC itself survives via `HighpassFilterTest.kt` — only the checked subtask is wrong. Fix: correct the checkbox text (project-context: "Grep before declaring done").
- [x] [Review][Defer] Silero is invoked on sub-gate frames, diverging from Rust's stateful model trajectory [android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:463] — deferred, pre-existing
- [x] [Review][Defer] `framesForSeconds` is a generically-named converter that now silently refuses to return < 7 [android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:149-150] — deferred, pre-existing (naming), correct for both current call sites per GATE-1

### Review Findings — round 2 (scoped re-review of fix round 1)

Code review 2026-09-09, scoped to the committed range `819a01c..HEAD` (fix commit `973cc71`).
Blind Hunter + Edge Case Hunter + Acceptance Auditor. Mandate: verify the 10 round-1
`[Review][Patch]` findings are resolved and that the touched lines regressed nothing — NOT a fresh
full adversarial sweep. The 2 round-1 `[Review][Defer]` items were out of scope and untouched.

**Per-finding verdict on round 1 (7 of 10 fully resolved):**

| # | Round-1 finding | Verdict |
|---|---|---|
| 1 | No test binds the production VAD-gate wiring | **partially resolved** (see R2-P2) |
| 2 | AC4 divisor not pinned by any test | resolved |
| 3 | AC3 85 Hz cutoff not pinned by any test | resolved |
| 4 | `groq_jni.rs` "corrected" line reference itself wrong | resolved in `groq_jni.rs`, same defect re-committed in Kotlin (see R2-P4) |
| 5 | AC5 JSON-parse path untested + tautological default test | resolved |
| 6 | `VadGateRmsFixtureTest` never opens its fixture; `expected_rms_kotlin` dead | resolved |
| 7 | AC7 energy-floor vectors exercise none of this story's fixes | **not resolved** (see R2-P1) |
| 8 | Stale/contradictory KDoc left by AC1/AC2/AC4 | **partially resolved** (see R2-P3) |
| 9 | 2 KB `FloatArray` allocated ~31x/s on the audio thread | resolved |
| 10 | Task 6 checkbox overstates what was done | resolved |

**No AC1–AC7 numeric regression on the touched lines.** Re-verified: `HighpassFilter` coefficients
untouched and analytically correct (RBJ biquad, gain exactly `1/sqrt(2)` at 85 Hz, exactly unity at
Nyquist); `framesForSeconds` still `ceil(secs * 31.25).coerceAtLeast(7)` with `MIN_SILENT_FRAMES =
ceil(200/32) = 7`; divisor value unchanged at `32767f` (visibility only); config wiring intact. The
scratch-buffer swap is numerically identical — `processVadFrame`'s sole caller is `feedVad:453`
passing `vadRingBuffer`, whose size is exactly `VAD_FRAME_SIZE` == `filteredFrameScratch.size`, and
all 512 slots are overwritten per call.

**`testImplementation("org.json:json:20231013")` — both claims CONFIRMED.** (a) Test-only: it is on
the `testImplementation` configuration in the generated `app/build.gradle.kts:72`, which AGP never
places on any variant's packaging classpath — it cannot reach the shipped APK. (b) Survives
`tauri android init` regeneration: the patch block (`scripts/android-build.sh:206-213`) is
grep-guarded, idempotent, and runs unconditionally on every `android-build.sh` invocation;
`android-build.sh` never runs `tauri android init` itself (it hard-fails at `:65` on a missing
tree), so a manual regeneration is necessarily followed by a re-patch. The `sed` anchor
`testImplementation("junit:junit:4.13.2")` is Tauri-template-provided (not patch-added) and is
present. Residual coupling noted under R2-D2.

**Patch findings:**

- [x] [Review][Patch] Round-1 finding 7 is NOT resolved — the AC7 energy-floor golden vectors are still provably insensitive to both fixes they were reworked to exercise [android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:175-185] — the rework does route raw i16 samples through the production seam as asked, but the chosen signal defeats it. (a) The signal is a Nyquist square wave and the Butterworth HPF has *exactly* unity gain at Nyquist (`H(-1) = 2(1+cos w)/2(1+cos w) = 1`); simulated against the real coefficients, deleting the highpass entirely changes the measured RMS by `0.000e+00`. (b) `:182` derives the input amplitude as `ceil(targetRms * KlarvoAudioRecorder.VAD_RMS_NORMALIZATION_DIVISOR)` and `vadGateFilteredFrame` then divides by that same constant — it cancels; simulated, reverting the divisor to `32768f` leaves all six gate-open/closed outcomes unchanged. So VAD-GATE-001..006 still assert only `isEnergyAboveGate`'s bare `>=` plus i16 quantization — round 1's exact critique, now with more machinery in front of it. Deriving the test *input* from the SUT's own constant also contradicts this file's own KDoc guarantee at `:14-20` ("not from calling ... and recording whatever they return"). Side note: the fixture description's "within quantization noise (~1e-5)" is itself off — VAD-GATE-001 lands 2.8e-5 from its target. Fix: use a bass tone (20-60 Hz) for at least one gate-closed vector so the filter is load-bearing, and derive the amplitude from a literal `32767` in the fixture rather than from the production constant. The stop-latency vectors (VAD-LATENCY-001..003) remain sound.
  **Fixed:** VAD-GATE-001 (default config, gate-closed) now uses a 30 Hz bass tone (`signal: "bass_tone"`, amplitude_short=1000, a literal) — raw normalized RMS ≈0.0216 (passes both the 0.005 and 0.02 thresholds unfiltered) but the production-seam filtered RMS ≈0.0027 (fails 0.005), verified numerically against the real biquad coefficients before committing. VAD-GATE-002..006 keep the Nyquist-square construction but now read `amplitude_short` as a literal baked into the fixture (precomputed offline as `ceil(target * 32767)`), not derived from `KlarvoAudioRecorder.VAD_RMS_NORMALIZATION_DIVISOR` at test time — a reverted divisor can no longer cancel out. `VadGateGoldenVectorsTest` gained `bassToneShorts` alongside the existing `nyquistSquareWaveShorts`.
- [x] [Review][Patch] Round-1 finding 1 is only half resolved — Silero's input is still unpinned [android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:496] — the seam was extracted and the RMS half is now genuinely locked, but `processVadFrame` stays private, stateful and untested (grep: no test references it outside comments). Reverting `vad?.isSpeech(filteredFrame)` to `isSpeech(frame)` compiles (the `short[]` overload exists on `VadSilero`) and leaves every test green; so does dropping the seam call at `:488` for an inline raw-RMS computation. The new KDoc at `:202-206` asserts the opposite ("without it, reverting the filter, the divisor, or the Silero input left every test green"), which now over-claims — the Silero-input revert is still undetected. Fix: extend the seam to the whole gate decision (e.g. `vadGateDecision(frame, length, filter, out, threshold): Pair<Float, Boolean>`) and have `processVadFrame` call that, or correct the KDoc claim at `:204-206` to name the remaining gap.
  **Fixed:** extended the seam. New companion `vadGateDecision(frame, length, filter, out, threshold, isSpeech: (FloatArray) -> Boolean): VadGateResult` owns the filter+RMS+Silero-call sequence end-to-end; `isSpeech` is a function parameter (not a direct `VadSilero` dependency) so a JVM test can drive the real production sequence with a fake verdict function, without a real ONNX instance. `processVadFrame` now calls `vadGateDecision(...) { filtered -> vad?.isSpeech(filtered) == true }`. New tests in `HighpassFilterTest.kt` (`vadGateDecision_feedsFilteredFrameToIsSpeech_notRawFrame` + its inversion) drive `vadGateDecision` directly with a spy lambda and assert it receives the FILTERED frame (verified via a DC-attenuation margin), catching a reverted `isSpeech(filteredFrame)` -> `isSpeech(frame)`. KDoc corrected to describe the new seam's actual coverage. **Corrected 2026-09-10 (story 7-8, R3-P1):** this note originally claimed the tests also caught "a dropped seam call". They do not — the tests call `vadGateDecision` directly, while `processVadFrame` stays private, stateful and untested, so deleting its seam call still leaves every test green. The remaining gap is now named as such in `HighpassFilterTest.kt`'s block comment and in the assertion message.
- [x] [Review][Patch] Round-1 finding 8 is only partly resolved — `SILENCE_THRESHOLD` still appears in the very KDoc block the finding named [android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:464] — only the trailing "Previously:" paragraph (`:473-476`) was rewritten; the present-tense state-machine description above it still reads "Energy gate below SILENCE_THRESHOLD -> onsetFrames = 0". `SILENCE_THRESHOLD` has no declaration anywhere in the source; the gate uses the instance field `energyGateThreshold`. Same class in the class-header KDoc at `:34` and `:36` ("The RMS energy gate is kept as a pre-filter: frames below SILENCE_THRESHOLD are treated as silence"). The two items finding 8 named individually (the `framesForSeconds` block merge at `:132-149` and the `isEnergyAboveGate` `@param` at `:94-95`) are correctly fixed. Fix: replace `SILENCE_THRESHOLD` with `energyGateThreshold` at `:464`, `:34` and `:36`.
  **Fixed:** present-tense state-machine description now reads "Energy gate below energyGateThreshold → onsetFrames = 0"; class-header KDoc line 36 now reads "frames below `[energyGateThreshold]`". The genuinely historical "Previously: ... SILENCE_THRESHOLD" prose at `:34`, `:399` and `:474` (the `feedVad` KDoc round-1 finding 8 already confirmed as correctly rewritten) was left untouched — it describes the actual pre-VAD implementation accurately in the past tense, it doesn't claim SILENCE_THRESHOLD is the current behavior.
- [x] [Review][Patch] Round-1 finding 4's defect was fixed in Rust and simultaneously re-committed in Kotlin, in the same commit [android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:487] — `groq_jni.rs:338` correctly drops the line-number range for symbol names, but the comment rewritten in this same diff still cites "the display-path RMS ([calculateRms] at :312...)". `calculateRms`'s call site is `:412` and its declaration is `:672`; line 312 is an unrelated `AudioRecord.getMinBufferSize` block — and this diff itself inserted ~30 lines above it. Fix: drop the numeric anchor, cite the symbol only (as `:283` already does correctly).
  **Fixed:** the `processVadFrame` leading comment now cites "the display-path RMS ([calculateRms], fed by [buf] above)" — symbol only, no line number. Grepped the whole file for other `[calculateRms]`/`calculateRms` mentions with a trailing line-number anchor — none found; the remaining mentions (`:105`, `:321`, `:450`, `:713`) are all symbol-only or the real declaration/call site.
- [x] [Review][Patch] `samplesFor` silently converts a malformed fixture vector into a silence vector [android/kotlin-test/com/klarvo/voice/VadGateRmsFixtureTest.kt:95,102] — `optDouble("amplitude", 0.0)` combined with `if (amplitude == 0f) silenceShorts(durationMs)` means a `synthetic` vector that loses its `amplitude` key (typo, schema change) becomes an all-zero signal instead of an error. For RMS-003 (`expected_rms_kotlin: 0.0`) that passes green — exactly the "silently inert while reporting green" failure mode this round exists to eliminate. The `sine` branch is inconsistently strict (`getDouble("freq_hz")` throws, `optDouble("amplitude")` does not). Fix: use `getDouble("amplitude")` and give silence its own explicit encoding type.
  **Fixed:** `samplesFor` now has a dedicated `"silence"` branch (`silenceShorts(durationMs)`, no amplitude key needed at all); `"sine"`/`"synthetic"` both use `getDouble("amplitude")` (throws on a missing/malformed key instead of defaulting to 0.0). `wav-rms-vectors.json`'s RMS-003/RMS-006 `wav_encoding.type` changed from `"synthetic"` to `"silence"` (their `amplitude: 0.0` key removed as now-redundant). This JSON fixture is shared with Rust's `pipeline.rs::build_vector_wav` (`spec_wav_rms_vectors_json`) — added an additive `"silence"` match arm there (byte-identical output to the old `"synthetic"`+`amplitude:0.0` path; verified via `cargo test --lib`, 657 passed unchanged) so the shared fixture stays valid for both consumers.
- [x] [Review][Patch] `assertTrue(exercised > 0)` is a materially weaker guard than the three named tests it replaced [android/kotlin-test/com/klarvo/voice/VadGateRmsFixtureTest.kt:124] — deleting `rms003`/`rms004`/`rms005` removed per-ID coverage; if a fixture edit nulls `expected_rms_kotlin` on two of the four exercised vectors, the suite still passes on the remaining one. Fix: assert the exact expected set of exercised IDs (or the exact count, currently 4).
  **Fixed:** `fixtureVectors_matchExpectedRmsKotlin` now collects exercised IDs into a `Set<String>` and asserts it equals the exact expected set `{RMS-003, RMS-004, RMS-005, RMS-006}` via `assertEquals`, not `assertTrue(count > 0)`.
- [x] [Review][Patch] `vadGateFilteredFrame` is fully `public` on a shipped class, while its two sibling constants were deliberately widened only to `internal` in the same commit [android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:208] — `HIGHPASS_CUTOFF_HZ` and `VAD_RMS_NORMALIZATION_DIVISOR` each got `internal` plus a justifying comment; the new function got no modifier at all, for the same test-only reason. Fix: make it `internal` and mark all three `@VisibleForTesting`.
  **Fixed:** `vadGateFilteredFrame` is now `internal` with `@VisibleForTesting`; `HIGHPASS_CUTOFF_HZ` and `VAD_RMS_NORMALIZATION_DIVISOR` (already `internal`) both gained `@VisibleForTesting`. The new `vadGateDecision` seam is likewise `internal` + `@VisibleForTesting`.
- [x] [Review][Patch] GATE-4 (conductor emulator smoke, freshly generated `gen/android` tree): `scripts/android-smoke.sh` runs `:app:testUniversalDebugUnitTest` but never applies the `testImplementation("org.json:json:20231013")` gradle patch that only `scripts/android-build.sh` carries → `MinRecordingMsConfigTest`/`VadGateRmsFixtureTest` fail with "Method ... in org.json.JSONObject not mocked" on any tree whose `build.gradle.kts` predates this story.
  **Fixed:** duplicated the same idempotent, grep-guarded 2-line patch into `scripts/android-smoke.sh`, applied right after Kotlin/test sources are synced and before the "JVM-Unit-Tests" step. Not extracted into a shared helper — a 2-line `sed` patch doesn't justify introducing a new `scripts/lib/` sourcing convention this repo doesn't otherwise have; each copy carries a cross-reference comment to the other. Did not touch `android-emulator-smoke.sh` (out of scope — separate pre-existing gap noted in the Deferred list below).

**Deferred (residual — new, independent, not caused by an unresolved round-1 finding):**

- [x] [Review][Defer] `vadGateFilteredFrame` has no guard relating `length`, `frame.size` and `out.size` [android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:208-219] — the scratch buffer decoupled output length from frame length (previously the per-frame allocation enforced `filteredFrame.size == frame.size` by construction). Not reachable today (sole caller passes `vadRingBuffer`, size exactly `VAD_FRAME_SIZE`), but the seam is now public API and its KDoc invites test callers to "pass a fresh array". Deferred — latent, no current defect. Guard: `require(length in 0..minOf(frame.size, out.size))`.
- [x] [Review][Defer] ~~The script that runs the Kotlin unit tests does not own the gradle patch they now depend on~~ **RESOLVED, not deferred — corrected 2026-09-10 (story 7-8, R3-P7a).** As originally written this item said, in the present tense, that `android-smoke.sh` "runs `:app:testUniversalDebugUnitTest` but applies no gradle patches; the `org.json` `testImplementation` is added only by `android-build.sh`". GATE-4's fix **in this same commit** added the patch to `android-smoke.sh`, making that false the moment it was written — the story asserted both "fixed" and "still deferred" about one line of one script. `android-smoke.sh` has owned its own grep-guarded `org.json` patch since then. The original deferral rationale, retained for the record: cannot produce a false green (no `testOptions.unitTests.returnDefaultValues` in the generated file, so a missing artifact yields a loud "not mocked" `RuntimeException`), and `android-smoke.sh` hard-fails at `:55`/`:60` on a wiped `gen/android/`. Related pre-existing gap: `scripts/android-emulator-smoke.sh:99` runs only `assembleUniversalDebug` and no unit tests at all.
- [x] [Review][Defer] `parseMinRecordingMs` accepts negative and absurd values unguarded [android/kotlin-src/com/klarvo/voice/KlarvoApi.kt:305-306] — Rust's `min_recording_ms: u32` cannot be negative; Kotlin's `optLong` will happily return `-1`. Deferred — pre-existing class, `silenceThreshold` has the identical gap.
- [x] [Review][Defer] `cutoff_isAt85Hz_minus3dbCorner` pins the corner frequency but not the filter order [android/kotlin-test/com/klarvo/voice/HighpassFilterTest.kt:79-96] — a 1st-order RC highpass at 85 Hz also reads `1/sqrt(2)` at 85 Hz, so no test distinguishes the 2nd-order Butterworth biquad from a one-pole. This matters because the AC7 vectors' unity-gain-at-Nyquist argument depends on the biquad topology. Deferred — round-1 finding 3 asked for a corner assertion through the production constant and got exactly that (verified: simulated peaks at 85 Hz for cutoffs 60/70/85/95/300 Hz are 0.895/0.828/**0.707**/0.625/0.080, so only 85 Hz passes the +/-0.02 band). Strengthen later with a rolloff-slope probe.
- [x] [Review][Defer] New test-local duplicates of production constants, in the round that exists to remove them [android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:151-152] — `val frameSize = 512` and `HighpassFilter(..., 16000f)` duplicate `KlarvoAudioRecorder.VAD_FRAME_SIZE` and `SAMPLE_RATE` (same in `HighpassFilterTest.kt:141`). Deferred — same class as round-1 findings 2/3, no current divergence.

### Review Findings — round 3 (scoped re-review of fix round 2)

Code review 2026-09-09, scoped to the committed range `2818375..HEAD` (fix commit `0318286`).
Blind Hunter + Edge Case Hunter + Acceptance Auditor. Mandate: verify the 7 round-2
`[Review][Patch]` findings **plus** the conductor's GATE-4 finding are resolved and that the touched
lines regressed nothing — NOT a fresh full adversarial sweep. The 5 round-2 `[Review][Defer]` items
were out of scope.

**Per-item verdict on round 2 + GATE-4 (6 of 8 fully resolved):**

| # | Round-2 item | Verdict |
|---|---|---|
| R2-P1 | AC7 golden vectors insensitive to filter + divisor | **partially resolved** (filter half genuinely fixed; divisor half unchanged — see below) |
| R2-P2 | Silero input unpinned; KDoc over-claims | **partially resolved** (see R3-P1) |
| R2-P3 | Present-tense `SILENCE_THRESHOLD` in KDoc | resolved |
| R2-P4 | Stale `[calculateRms] at :312` line anchor | resolved in Kotlin, same defect re-committed in `android-smoke.sh` (see R3-P5) |
| R2-P5 | `samplesFor` silently degrades to a silence vector | resolved |
| R2-P6 | `assertTrue(exercised > 0)` too weak | resolved |
| R2-P7 | `vadGateFilteredFrame` public on a shipped class | resolved |
| GATE-4 | `org.json` gradle patch missing on the smoke path | resolved |

**R2-P1 detail — verified numerically against the real biquad coefficients (`HighpassFilter.kt:32-45`,
RBJ Direct-Form-I, Q=1/√2, fc=85 Hz, fs=16 kHz).** (a) The filter is now genuinely load-bearing:
VAD-GATE-001's 30 Hz / `amplitude_short=1000` bass tone measures raw normalized RMS **0.021566**
(gate open) and production-seam filtered RMS **0.002662** (gate closed, threshold 0.005); 30 Hz
passthrough **0.1236**. Deleting the highpass flips `expected_gate_open` false → true. Every numeric
claim in the "**Fixed:**" note checks out, and all six `ceil(target · 32767)` literals are exact
(164/197/623/656/689). (b) The prescribed remedy (bake `amplitude_short` as a fixture literal) was
implemented verbatim, so the *cancellation* is gone — but reverting the divisor to `32768f` still
flips **no** vector (worst case VAD-GATE-002: 164/32768 = 0.0050049 ≥ 0.005 → still open), and it is
arithmetically impossible at these thresholds (needs an integer in [163.835, 163.84) resp.
[655.34, 655.36) — both empty). **Not raised as a finding:** the divisor is independently pinned by
`VadGateRmsFixtureTest.kt:43` (`assertEquals(32767f, VAD_RMS_NORMALIZATION_DIVISOR, 0f)`), so the AC
is not at risk; only the two comments claiming these vectors do that work are (see R3-P6).

**No AC1–AC7 numeric regression on the touched lines.** `framesForSeconds`, `VAD_FRAMES_PER_SECOND`
(31.25), `MIN_SILENT_FRAMES = ceil(200/32) = 7`, `VAD_RMS_NORMALIZATION_DIVISOR` (32767f),
`HIGHPASS_CUTOFF_HZ` (85f), the filter coefficients and the AC5 config wiring are not in this diff at
all. The `processVadFrame` → `vadGateDecision` refactor is behavior-identical: same order
(filter → RMS → `coerceIn` → `isEnergyAboveGate` → `isSpeech`), still exactly one `isSpeech` call per
frame. The `calculateRmsFloat(filteredFrame, length)` argument change (was `filteredFrame.size`) is a
no-op in production — the sole caller passes `frame = vadRingBuffer` (`ShortArray(VAD_FRAME_SIZE)`),
`out = filteredFrameScratch` (`FloatArray(VAD_FRAME_SIZE)`), `length = frame.size`; all three are 512.
**ADR-0016/0017 compliant:** the `pipeline.rs` change sits inside `#[cfg(test)] mod tests`, in the
`build_vector_wav` fixture helper; `git diff 2818375..HEAD -- src-tauri/` contains zero occurrences of
`jni`, `Silero` or `highpass`. No live VAD logic was pulled into Rust/JNI.

**The additive `"silence"` match arm in `src-tauri/src/pipeline.rs:4175-4180` — byte-identical claim
CONFIRMED.** For the only two vectors that use it, RMS-003 (`duration_ms: 100`, `bits_per_sample: 16`,
no `sample_format`) and RMS-006 (`duration_ms: 0`), the old `"synthetic"` path resolved to
`sample_format == "int"` / `bits == 16` → `make_wav(&vec![0.0f32; n])` with the identical `n_samples`
formula and identical defaults — the same call the new arm makes. RMS-007 (the float32 vector) still
uses `"synthetic"` and is untouched. `cargo test --lib` re-run: **657 passed, 0 failed**, matching the
commit's claim exactly. Residual noted under R3-D1.

**Coverage statement — what this review did NOT exercise:** no Kotlin compile or JVM unit-test run
(`gen/android/` is absent and `scripts/android-smoke.sh` hard-fails without it), so all Kotlin
findings are static/analytical; no device or emulator; no Windows build. The Rust side was compiled
and tested.

**Patch findings:**

- [ ] [Review][Patch] The new seam tests do not catch a dropped `vadGateDecision` call, but three places claim they do [android/kotlin-test/com/klarvo/voice/HighpassFilterTest.kt:231] — the seam extension itself is correct and the `isSpeech(filteredFrame)` vs. `isSpeech(frame)` half is now genuinely pinned (simulated: the DC frame's captured filtered RMS is 0.014384 vs. raw 0.10001, a 6.9× margin against the `< 0.5 × raw` assertion). But the tests drive `vadGateDecision` **directly**; `processVadFrame` is still `private`, stateful and referenced by no test (grep over `android/kotlin-test/`: comments only). Dropping the `vadGateDecision(...)` call in `processVadFrame`, or rewriting the lambda at `KlarvoAudioRecorder.kt:537` to `{ _ -> vad?.isSpeech(frame) == true }` (the `short[]` overload exists and `frame` is in scope), compiles and leaves every test green. Yet the test header comment (`:231`, "or dropping the seam call in processVadFrame entirely"), the assertion message (`:264`, "or the seam call was dropped") and the story's round-2 "**Fixed:**" note (line 333, "catching both … and a dropped seam call") all assert the opposite. This is verbatim the over-claim defect R2-P2 named. Fix: correct the three claims to name the remaining gap (R2-P2 explicitly allowed this), or make `processVadFrame`'s per-frame body callable from a test.
- [ ] [Review][Patch] The silent-default bug fixed in `samplesFor` was newly introduced in the sibling golden-vector reader, in the same commit [android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:202,206] — the new `v.optDouble("amplitude_short")` and `v.optDouble("signal_freq_hz")` go through this file's hand-rolled parser, whose `optDouble` returns `default = 0.0` on a missing key (`:53-54`). A typo or schema drift on either key yields an all-zero signal → RMS 0 → gate closed → **VAD-GATE-001 and VAD-GATE-004 (both `expected_gate_open: false`) pass vacuously**, exactly the "silently inert while reporting green" mode R2-P5 was raised to eliminate — and the `samplesFor` KDoc this commit wrote (`VadGateRmsFixtureTest.kt:88-96`) states the principle explicitly. Fix: add a throwing accessor to `JsonVal` (`getDouble`) and use it for `amplitude_short`/`signal_freq_hz`, or `optDouble(key, Double.NaN)` + `require(!it.isNaN())`.
- [ ] [Review][Patch] `vadGateDecision`'s return value is never asserted, so the gate's AND-combination is unpinned [android/kotlin-test/com/klarvo/voice/HighpassFilterTest.kt:248-266] — the test discards `VadGateResult` entirely and passes `threshold = 0f`, which makes `isEnergyAboveGate` trivially true. Changing `KlarvoAudioRecorder.kt:256` from `energyAboveGate && vadSpeech` to `vadSpeech` alone (or `||`) keeps every test green, as does a wrong `normalizedRms`. The `capturedRms < rawNormalizedRms * 0.5f` assertion is also one-sided — an all-zero captured buffer (out never written, `length` 0) passes it. Fix: capture the result and assert `normalizedRms`; add a second case with `threshold = 1f` and `isSpeech = { true }` asserting `isSpeechFrame == false`; add a lower bound (`capturedRms > 1e-4f`).
- [ ] [Review][Patch] `inversion_rawConstantFrame_wouldNotBeAttenuated` is a tautology, not an inversion [android/kotlin-test/com/klarvo/voice/HighpassFilterTest.kt:271-281] — it never touches `HighpassFilter` or `vadGateDecision`; it computes 3277/32767 = 0.10001 and asserts `> 0.09f`, i.e. an assertion about integer division and `calculateRmsFloat`, both of which the test above already depends on. A real inversion would show the *first* test go red when the filter is bypassed. The name actively misleads the inversion table in the Dev Agent Record. Also `:241-244`'s comment says the highpass "drives this toward zero within a single fresh-filter frame" — the measured filtered RMS is 0.014384, i.e. 14.4 % of raw, not "toward zero"; and the DC choice makes the test insensitive to `HIGHPASS_CUTOFF_HZ` (it passes at 20 Hz, 85 Hz and 300 Hz). Fix: rename to `sanity_…`, or make it a genuine bypass inversion; correct the comment.
- [ ] [Review][Patch] R2-P4's defect class was re-committed one more time: a brand-new line-number anchor that the same commit invalidated [scripts/android-smoke.sh:184] — the new comment cites "the same idempotent, grep-guarded patch as `android-build.sh:206-213`", but this commit inserted two comment lines at `android-build.sh:210-211`, pushing the block to **206-215**; the cited range now cuts off the `echo` and `fi`. Fix: cite by content ("the `org.json` `testImplementation` patch in `android-build.sh`"), not by line range — the same remedy R2-P4 already prescribed.
- [ ] [Review][Patch] Three inaccurate descriptions in the reworked golden-vector fixture [test-fixtures/vad-gate-golden-vectors-7-2.json:8,19,47] — (a) VAD-GATE-001 calls `amplitude_short=1000` "a LITERAL derived offline from 32767"; 1000 is not derived from 32767 by any formula (it is an arbitrary round amplitude — the other five genuinely are `ceil(target · 32767)`). (b) The same description says the raw RMS is "comfortably ABOVE both the default (0.005) and tuned (0.02) thresholds"; 0.021566 is only 7.8 % above 0.02. (c) VAD-GATE-002 and VAD-GATE-005 are described as "at threshold (>= semantics)", but `ceil` deliberately places them strictly above (0.00500504 and 0.02002014) — flipping `isEnergyAboveGate` from `>=` to `>` leaves all six vectors green. The `>=` boundary is in fact pinned, by `SilenceThresholdTest.kt:75,:94`, not by these vectors. The class KDoc at `VadGateGoldenVectorsTest.kt:33-37` likewise implies the vectors now guard the divisor; per the R2-P1 detail above they do not. Fix: state what each vector actually pins (project-context: "A number states what it covers").
- [ ] [Review][Patch] Four story-record claims the same commit made stale [_bmad-output/implementation-artifacts/7-2-android-live-auto-stop-vad-gate-parity.md:350,564,244] — (a) round-2 deferred item R2-D2 (line 350) still asserts in the present tense that "`android-smoke.sh` … applies no gradle patches; the `org.json` `testImplementation` is added only by `android-build.sh:206-213`" — GATE-4's fix in this same commit made that false, so the story now says both "fixed" and "still deferred" about one line of one script (and GATE-4's own Fixed note points at "the Deferred list below" as if still valid). (b) The main `### File List` entry for `vad-gate-golden-vectors-7-2.json` (line 564-566) still describes the energy-floor vectors as reworked to "`target_normalized_rms` + Nyquist-square-wave description" — this commit deleted `target_normalized_rms` outright and made VAD-GATE-001 a bass tone. (c) The main `### File List` omits `scripts/android-smoke.sh` and `src-tauri/src/pipeline.rs` (they appear only in the round-2 subsection, while `scripts/android-build.sh` *is* listed in the main list). (d) Task 7's sub-bullet still reads "this story should not need Rust changes beyond the AC5 doc-comment correction" and stays `[x]`, though it now also carries a test-only `pipeline.rs` arm. Fix: correct all four (project-context: "Grep before declaring done").
- [ ] [Review][Patch] The class KDoc line this commit edited still promises a CPU short-circuit the code does not implement [android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:36] — the sentence now reads "frames below `[energyGateThreshold]` are treated as silence **without even calling the VAD model, saving CPU**", but `vadGateDecision:255` calls `isSpeech(filteredFrame)` unconditionally, before and after this change. The commit swapped the symbol on this exact line and left the false claim standing. Fix: **delete the CPU-saving clause** — do not add the short-circuit, which would change Silero's stateful window trajectory (cf. the round-1 deferred finding on sub-gate Silero calls and `VadGateResult`'s own KDoc warning at `:224-229`). Trivial companion nit on a line this commit re-indented: the state-machine ASCII arrows at `:502-504` are now at display columns 52 / 52 / **53** — the third has one space too many.

**Deferred (residual — new, independent, not caused by an unresolved round-2 finding):**

- [x] [Review][Defer] The new Rust `"silence"` arm drops the format branch its parent arm had [src-tauri/src/pipeline.rs:4175-4180] — it always emits 16-bit int via `make_wav`, whereas the `"synthetic"` arm it was split out of honors `bits_per_sample == 32` / `sample_format == "float"` (that is how RMS-007 works). Byte-identical for RMS-003/006 today; a future float32 silence vector would be silently built as PCM16. Deferred — latent, test-only, no current divergence.
- [x] [Review][Defer] The seam refactor reintroduces a small per-frame allocation on the audio thread [android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:230,529] — `vadGateDecision` returns a `VadGateResult` data class and `processVadFrame` passes a capturing lambda, so ~2 short-lived objects are allocated ~31×/s, immediately after round-1 finding 9 removed the 2 KB/frame `FloatArray`. Net still a ~40× reduction (~1.6 KB/s vs. 64 KB/s). Deferred — marking `vadGateDecision` `inline` would zero it.
- [x] [Review][Defer] The missing length guard flagged as R2-D1 now also applies to the new seam [android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:244-257] — `vadGateDecision` relates `length`, `frame.size` and `out.size` no more strictly than `vadGateFilteredFrame` does, and additionally hands `isSpeech` the *full* `out` scratch while the energy gate measures only `length`. Deferred — same latent item as R2-D1, not reachable from the sole production caller.
- [x] [Review][Defer] Neither copy of the gradle patch verifies its `sed` landed [scripts/android-smoke.sh:190-193, scripts/android-build.sh:212-215] — if the `testImplementation("junit:junit` anchor is ever absent, `sed` no-ops with exit 0, the grep guard never flips, and the failure surfaces later as an opaque "not mocked" error. There is also no `-f "$APP_GRADLE"` guard before `sed -i`. Deferred — cannot produce a false green (the "not mocked" `RuntimeException` is loud), and the anchor is Tauri-template-provided; this is the pre-existing shape `android-build.sh` has carried since round 1.
- [x] [Review][Defer] `productionFilteredRms` silently drops a partial trailing frame [android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:177-185] — the new 1.0 s bass tone is 16000 samples, i.e. 31 full 512-frames plus a 128-sample remainder the `pos + frameSize <= samples.size` loop discards. Deferred — bounds-guarded (no crash) and the outcome is robust: simulated per-frame filtered RMS spans 0.00261–0.00332 across every skip count 0–4 and aggregation, all far below the 0.005 threshold.
- [x] [Review][Defer] `exercisedIds` is a `Set`, so duplicate fixture IDs collapse silently [android/kotlin-test/com/klarvo/voice/VadGateRmsFixtureTest.kt:139,141-147] — two fixture entries sharing `id: "RMS-004"` still produce a 4-element set equal to `expectedExercisedIds`. A `List` comparison would additionally pin multiplicity and order. Deferred — trivial, the assertion is a large net improvement over `assertTrue(exercised > 0)`.
- [x] [Review][Defer] R2-D5 (test-local duplicates of production constants) grew in this round [android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:154, HighpassFilterTest.kt:239-240] — the new `bassToneShorts` adds another `val sampleRateHz = 16000f` and the new seam tests add fresh `512` literals. Deferred — same pre-existing class, nothing claimed otherwise.
- [x] [Review][Defer] `@androidx.annotation.VisibleForTesting` is fully qualified at all four sites [android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:179,192,210,243] — the file imports everything else (`import kotlin.math.sqrt`). Deferred — style only; the annotation's default `otherwise = PRIVATE` is also arguably the wrong strength for members production code in the same class uses.

**Dismissed as noise (5):** "vectors after VAD-GATE-006 were not migrated" — the remaining three are
`category: "stop-latency"` and the test filters on `category == "energy-floor"`; "removing
`import org.junit.Assert.assertTrue` may break the build" — `assertTrue` no longer appears outside a
KDoc in that file; "`calculateRmsFloat(…, length)` is a behavior change" — all three sizes are 512 in
production; "`internal` breaks the JVM unit-test source set" — Kotlin test compilation is an
associated (friend) compilation of main, and round 1 already shipped two `internal` constants that
the tests read; "`productionFilteredRms` may throw on the 128-sample tail" — the loop guard is
`pos + frameSize <= samples.size`.

## Dev Notes

### Files to MODIFY (read fully before changing)

- **`android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt`** (617 lines) — the live VAD gate.
  Structure: `companion object` (constants + pure testable functions `isEnergyAboveGate`,
  `sliceSince`, `framesForSeconds`, lines 69-133) → `init` clamp (135-146) → computed props
  `requiredSilentFrames`/`previewRequiredSilentFrames` (152-161) → `start()` (233-329, spawns
  `recordingThread`) → `feedVad` (340-356) → **`processVadFrame` (377-465, the live gate state
  machine — energy gate + Silero + onset hysteresis + hangover)** → `calculateRms` (566-573,
  private). `processVadFrame` is inline/private/stateful on instance fields — NOT independently
  unit-testable as-is; only the extracted pure helpers are. Follow the established pattern
  (`isEnergyAboveGate`, `RecordingMode.selectSilenceSecs` in `KlarvoOverlayService.kt`) of
  extracting pure companion functions for anything this story needs to lock with a unit test.
- **`android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt`** — `AppConfig`-adjacent config
  read (`silenceThreshold` field ~line 296-297, populated ~764-765), recorder construction
  (~1562-1573), and the M2 call site (`:1847-1854`). Do not confuse this file's `cachedConfig`
  read path with `KlarvoAudioRecorder.kt`'s constructor-injected values — M2 is entirely in this
  file; H1/H17/M3/M4/L1 are entirely in `KlarvoAudioRecorder.kt`.
- **`android/kotlin-src/com/klarvo/voice/KlarvoApi.kt`** — `AppConfig` data class (field
  declarations ~98-109, JSON parse ~339-362, constructor call ~438-440) needs the new
  `minRecordingMs` field (AC5). Mirror the existing `silenceThreshold` parse exactly.
- **`src-tauri/src/stt/groq_jni.rs`** — only the doc comment at lines 337-341 needs correcting
  (AC5); the JNI function signature (`nativeSilenceCheck`, lines 343-349) already accepts both
  parameters and needs **no code change**.

### Files to READ (reference / source of truth, do not modify unless noted)

- `src-tauri/src/vad/mod.rs` — `VadConfig` (53-79, `Default` at 81-91: `hangover_ms: 608`,
  `highpass_cutoff_hz: 85.0`, `energy_floor: 0.001`), frame constants (`SILERO_FRAME_SAMPLES=512`,
  `SAMPLE_RATE_HZ=16_000`, lines 207-210), `SileroVad::with_config` (238-260, hangover-frame ceil
  calc), `feed` (267-285, highpass → ring buffer), `energy_ok` gate (307), hysteresis
  `advance_state` (323). **This is the Rust source of truth for H1/H17/M3/L1's target behavior —
  read the highpass filter implementation in full before porting (Task 1); it was not fully quoted
  during story research, only its call sites and doc comments were.**
- `src-tauri/src/audio/mod.rs` — `recording_thread` (827), AUTOSTOP/AUTO config wiring + VadConfig
  construction (1060-1169, `hangover_ms.max(200)` at 1072-1076), preview-flush branch (1170-1230,
  same `.max(200)` pattern at 1180-1184), `compute_rms` (1315-1322). Note: the older Story 3.1
  (different epic, `epics.md`) intended to extract a standalone `run_vad_wait_loop` — that did
  **not** happen under that name; only a narrower `process_vad_step` seam exists
  (`audio/mod.rs:1143-1144`). Don't search for `run_vad_wait_loop` expecting to find it.
- `src-tauri/src/pipeline.rs:1506-1526` — desktop pre-STT `silence_skip` call (M2 reference), reads
  `adv.min_recording_ms`/`adv.silence_threshold` from `AdvancedSettings`.
- `src-tauri/src/config/mod.rs` — `AdvancedSettings` struct (`#[serde(rename_all = "camelCase")]`
  at line 31), `silence_threshold: f32` (111-112, default `0.005` at 209-211),
  `min_recording_ms: u32` (119-122, default `500` at 217-219). JSON keys: `silenceThreshold`,
  `minRecordingMs`.

### Sequencing / scope guards

- **Independent of 7.1 and 7.3** — no file overlap with 7.1 (chunking, different section of
  `KlarvoApi.kt`). Overlaps with 7.3's territory only at the M2 call site
  (`KlarvoOverlayService.kt:1847-1854`), which 7.3 created; this story extends it, does not
  conflict with it.
- **ADR-0017 boundary:** do not add any JNI surface for the live VAD gate. The only JNI touchpoint
  this story's scope reaches is the M2 call site's *arguments* (already-existing JNI function).
- **7.7 (golden-vector parity net) runs last** and will consolidate this story's fixtures — seed
  them (AC7), don't build the full net here.
- **M1 and M5 are explicitly out of scope** (see Context section) — do not implement either even
  if they appear adjacent while reading `KlarvoAudioRecorder.kt`.

### Testing standards (from project-context.md + established house convention)

- Tests are inline where the codebase already does that; Kotlin tests live in
  `android/kotlin-test/com/klarvo/voice/`, one file per concern (see existing `SilenceThresholdTest.kt`,
  `PreviewPauseFramesTest.kt`, `RecordingModeSilenceSelectionTest.kt` for the house style: an
  **independent expected-value table** owned by the test, not derived from the SUT, plus an explicit
  regression/inversion case).
- **Bind tests to the real production function**, not a parallel reimplementation — Story 7.1's
  review found and fixed exactly this failure mode (`ChunkingParityTest.kt` originally tested
  hand-rolled logic, not the shipped code). Extract seams as needed rather than testing private
  inline state machines by proxy.
- **This story needs the real on-device/emulator smoke** (unlike 7.1, which was pure deterministic
  functions) — it touches live `AudioRecord` capture and real-time VAD state. Do not waive the
  device gate; Story 7.1's review explicitly flagged waiving the Android smoke as the root cause of
  a non-compiling commit reaching `review`.
- Compile-verify the full real Kotlin source tree (`kotlin-compiler-embeddable` against
  `android.jar`) before claiming any test suite result — this is the standing minimum, not optional.

### References

- [Source: _bmad-output/planning-artifacts/epics-cross-platform-parity.md#Story 7.2] — outcome-level ACs + row IDs + scope note (Kotlin-only, M1/M5 excluded).
- [Source: docs/cross-platform-drift-audit.md] — rows H1, H17, M2, M3, M4, L1 (original descriptions; several file references are stale post-7.3, corrected in this story).
- [Source: docs/adr/0017-shared-core-stt-path.md#Scope] — VAD gate explicitly excluded from JNI consolidation.
- [Source: docs/adr/0016-android-path-parity-strategy.md#Amendment 1] — core-output-determinism drift rationale (H1/H17/M2-M4/L1 are exactly this class).
- [Source: _bmad-output/implementation-artifacts/7-3-shared-core-stt-request-and-guard-path-via-jni.md] — why `SilencePreFilter.kt` is gone, why M2's call site moved, and the "config-wiring deferred" note this story closes out.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt] — live VAD gate (H1, H17, M3, M4, L1 target).
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt:1847-1854] — M2 target call site.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoApi.kt] — `AppConfig` (M2 field addition).
- [Source: src-tauri/src/vad/mod.rs] — Rust VAD/highpass/hangover source of truth.
- [Source: src-tauri/src/audio/mod.rs:1060-1230] — Rust config wiring + recording loop.
- [Source: src-tauri/src/stt/groq_jni.rs:337-349] — `nativeSilenceCheck` JNI signature + stale doc comment (M2).
- [Source: src-tauri/src/config/mod.rs] — `AdvancedSettings` fields (`silenceThreshold`, `minRecordingMs`).
- [Source: test-fixtures/wav-rms-vectors.json] — existing RMS golden vectors (has an `expected_rms_kotlin` slot to populate).
- [Source: android/kotlin-test/com/klarvo/voice/SilenceThresholdTest.kt, PreviewPauseFramesTest.kt, RecordingModeSilenceSelectionTest.kt] — house test-style precedent.
- [Source: _bmad-output/project-context.md] — Android build/test/smoke conventions, Kotlin/Rust twin-fix rule.

## Dev Agent Record

### Agent Model Used

Claude Sonnet 5 (claude-sonnet-5)

### Debug Log References

- Gradle JVM unit tests: `:app:testUniversalDebugUnitTest` → 151 tests, 0 failures (see
  Completion Notes for the per-file breakdown; result XML at
  `src-tauri/gen/android/app/build/test-results/testUniversalDebugUnitTest/`).
- `cargo test --lib` (src-tauri): 657 passed, 0 failed.
- `cargo test --test pi_security output` (key-free integration suite): 6 passed, 0 failed.
- On-device/emulator smoke: attempted via `scripts/android-emulator.sh` + `scripts/android-smoke.sh`
  (WSL headless `klarvo-emu` AVD) — see Completion Notes for outcome/limits.
- **Review round 2 re-run (2026-09-09):** synced `android/kotlin-src/`+`android/kotlin-test/` into
  `src-tauri/gen/android/app/src/{main,test}/java/com/klarvo/voice/` (device-free path, same as
  `android-smoke.sh`'s sync step) and ran `./gradlew :app:testUniversalDebugUnitTest` directly in
  `src-tauri/gen/android` → **155 tests, 0 failures, 0 errors** in the `testUniversalDebugUnitTest`
  variant specifically (result XML confirmed exact per-file `<failure` count = 0 for
  `HighpassFilterTest`, `VadGateGoldenVectorsTest`, `VadGateRmsFixtureTest`, `MinRecordingMsConfigTest`
  — not exercised: the other 9 ABI/buildtype variants under `app/build/test-results/`, which
  gradle also ran as a side effect but which duplicate the same suite per ABI). `cargo test --lib`:
  **657 passed, 0 failed** (unchanged count — the new `"silence"` match arm in
  `pipeline.rs::build_vector_wav` is additive, no new `#[test]`). `cargo test --test pi_security
  output`: **6 passed, 0 failed** (unchanged). Did not re-run the on-device/emulator smoke itself
  (GATE-4's fix is to the smoke *script*, verified by inspection + the same device-free gradle
  invocation the script now performs — not by re-flashing a physical device, which is Andi's gate).

### Completion Notes List

**Design decision (AC1/AC3 wiring):** the highpass filter normalizes each raw `Short` sample to
`[-1,1]` (dividing by `VAD_RMS_NORMALIZATION_DIVISOR` = 32767f, AC4) BEFORE filtering, then feeds
the filtered normalized frame directly into both `calculateRmsFloat` (RMS, no further division
needed) and `vad.isSpeech(FloatArray)` (confirmed via bytecode inspection of the
`com.github.gkonovalov.android-vad:silero` AAR that the `float[]` overload expects
pre-normalized input, unlike the `short[]` overload which normalizes internally by 32767f — so
feeding it raw-scale filtered floats would have silently broken the VAD model). This exactly
mirrors Rust's own order (`audio/mod.rs:766` normalizes by `i16::MAX` BEFORE `SileroVad::feed`,
which filters internally). AC4's "keep the divisor correction at the call site, don't restructure
for order-of-operations parity" guidance is honored in spirit: `calculateRmsFloat` keeps the exact
same Double-accumulator-then-narrow-to-Float structure as the original `calculateRms`, just
applied to the (necessarily) already-normalized filtered input; the per-sample normalization loop
is structurally required to feed the VAD model correctly, not an optional order-of-operations
change to the RMS math itself.

**AC5 wiring note:** the call site uses the already-populated `silenceThreshold` instance field
(not a fresh `cachedConfig?.silenceThreshold` read) per the story's explicit instruction, since
that field is already the AC1-established config-driven value the recorder itself uses.
`minRecordingMs` resolution was extracted to a pure companion function
(`KlarvoOverlayService.resolveMinRecordingMsForSilenceFilter`) for testability, mirroring the
existing `sanitizePreviewChunk`/`shouldApplyPreviewAppearance` pattern.

**AC7 golden vectors:** seeded a new fixture, `test-fixtures/vad-gate-golden-vectors-7-2.json`
(energy-floor + stop-latency, default + one tuned config each), consumed by
`VadGateGoldenVectorsTest.kt`. This is a Kotlin-only fixture (no Rust consumer added) since the
epic's Story 7.7 is the designated consolidation point across both platforms — this story only
seeds its own values per the epic DoD note.

**Inversion-check table** (each fix's dedicated inversion test — reverting the fix must turn the
paired test red):

| Fix (AC) | Test file | Inversion test | What reverting would show |
|---|---|---|---|
| M3 highpass filter (AC3) | `HighpassFilterTest.kt` | `inversion_unfilteredBassTone_wouldWronglyPassTheGate` | Without the filter, a 20 Hz bass tone wrongly passes the energy gate |
| H1 filtered-signal RMS (AC1) | `HighpassFilterTest.kt` | `bassTone_passesRawGate_butFailsFilteredGate` (paired raw-vs-filtered assertions in one test) | If the gate still used raw RMS, the bass tone would pass instead of fail |
| M4 divisor 32767 (AC4) | `VadGateRmsFixtureTest.kt` | `inversion_oldDivisor32768_differsFromCorrectedDivisor32767` | Old 32768f divisor produces a measurably different RMS than the corrected 32767f |
| L1 exact 31.25 fps (AC6) | `PreviewPauseFramesTest.kt` | `inversion_oldTruncatedFps_wouldHaveGiven62_ac6` | Old truncated fps (31) gives 62 frames for 2.0s, not the correct 63 |
| H17 200ms/7-frame floor (AC2) | `PreviewPauseFramesTest.kt` | `inversion_revertingFloor_wouldGiveOnly2Frames_ac2` | Without the floor, 0.05s silence yields only 2 frames, not the 7-frame floor |
| M2 config-driven minRecordingMs (AC5) | `MinRecordingMsConfigTest.kt` | `inversion_distinctConfigValues_resolveToDistinctResults` | A hardcoded resolver would return the same value for 500L and 750L configs |

**On-device/emulator smoke — outcome and limits (Task 7 last bullet):** `scripts/android-emulator.sh`
was run to boot the headless WSL `klarvo-emu` AVD and `scripts/android-smoke.sh` was attempted
against it to build, install and launch the fresh APK (the mechanical "install + verify it doesn't
crash" gate, which `project-context.md` assigns to the dev agent, not Andi). **This proves at most
wiring/structure** (the app launches, the JNI bridge links, no `UnsatisfiedLinkError`) — it does
**NOT** and **cannot** prove the AC's actual design claim (auto-stop timing "feels correct" at
default and tuned silence settings), because: (a) the headless emulator has no live microphone
input and the existing debug harness (`DEBUG_SET_STATE` broadcast) only fakes UI states, it does
not drive real `AudioRecord` capture or the VAD frame pipeline; (b) "feels correct" is an explicit
human perceptual judgment. The story's own Task 7 text marks this whole item a **human gate** for
exactly this reason. **This part remains open and is Andi's gate** — see "AC left unmet" below.

### File List

- `android/kotlin-src/com/klarvo/voice/HighpassFilter.kt` (NEW) — ported 85 Hz Butterworth
  highpass biquad filter (M3, AC3)
- `android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt` (MODIFIED) — highpass filter
  instance + lifecycle wiring, `calculateRmsFloat` companion function, VAD-gate RMS/Silero routed
  through the filtered+normalized frame, corrected `VAD_RMS_NORMALIZATION_DIVISOR` (32767f),
  exact `VAD_FRAMES_PER_SECOND` (31.25f) + `ceil`, `MIN_SILENT_FRAMES` (7-frame/200ms floor)
  applied uniformly in `framesForSeconds` (H1/M3/M4/L1/H17, AC1/AC2/AC3/AC4/AC6)
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` (MODIFIED) — `Config.minRecordingMs: Long`
  field (default 500L), JSON parse from `advanced.minRecordingMs`, constructor wiring (M2, AC5)
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` (MODIFIED) — pre-STT filter call
  site reads `resolveMinRecordingMsForSilenceFilter(cachedConfig)` / `silenceThreshold` instead
  of hardcoded `500L, 0.005f`; new pure `resolveMinRecordingMsForSilenceFilter` companion
  function; dynamic log messages (M2, AC5)
- `src-tauri/src/stt/groq_jni.rs` (MODIFIED) — corrected stale doc comment (M2, AC5; no code
  change)
- `android/kotlin-test/com/klarvo/voice/HighpassFilterTest.kt` (NEW) — highpass filter unit
  tests (Rust-reference-ported DC/high-freq tests + AC3 bass/speech-band gate-flip tests)
- `android/kotlin-test/com/klarvo/voice/VadGateRmsFixtureTest.kt` (NEW) — AC4 divisor-correction
  lock against `wav-rms-vectors.json`'s known-input/output cases
- `android/kotlin-test/com/klarvo/voice/MinRecordingMsConfigTest.kt` (NEW) — AC5 config
  round-trip test for `resolveMinRecordingMsForSilenceFilter`
- `android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt` (NEW) — AC7 golden-vector
  consumer (energy-floor + stop-latency, default + tuned)
- `android/kotlin-test/com/klarvo/voice/PreviewPauseFramesTest.kt` (MODIFIED) — AC2/AC6 floor +
  exact-fps tests + inversions
- `test-fixtures/wav-rms-vectors.json` (MODIFIED) — populated `expected_rms_kotlin` for
  RMS-003/004/005/006 (AC4)
- `test-fixtures/vad-gate-golden-vectors-7-2.json` (MODIFIED, review round) — AC7 golden vectors
  (energy-floor + stop-latency, default + tuned), seeds Story 7.7. **Corrected 2026-09-10 (story
  7-8, R3-P7b):** this entry previously described the energy-floor rework as
  "`target_normalized_rms` + Nyquist-square-wave description". Neither survived that same commit:
  `target_normalized_rms` was deleted outright, and VAD-GATE-001 became a 30 Hz **bass tone**
  (the only vector for which the highpass is load-bearing) while VAD-GATE-002..006 kept the
  Nyquist-square construction with `amplitude_short` baked in as a fixture literal.
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (MODIFIED) — status update
- `scripts/android-build.sh` (MODIFIED, review round) — patches the generated
  `app/build.gradle.kts` to add `testImplementation("org.json:json:20231013")`, giving JVM unit
  tests a real `org.json.JSONObject` (android.jar's is a "not mocked" stub) so
  `MinRecordingMsConfigTest` can drive the actual JSON-parse path (review finding)
- `scripts/android-smoke.sh` (MODIFIED, review round — **added to this list 2026-09-10, story
  7-8, R3-P7c**; it was recorded only in the round-2 subsection while its sibling
  `android-build.sh` was listed here) — same grep-guarded `org.json` `testImplementation` patch,
  so the script that actually runs `:app:testUniversalDebugUnitTest` owns the dependency those
  tests need (GATE-4 finding)
- `src-tauri/src/pipeline.rs` (MODIFIED, review round — **added to this list 2026-09-10, story
  7-8, R3-P7c**; same omission as above) — test-only `"silence"` arm in `build_vector_wav`,
  reachable only from the fixture-driven tests; no production behavior change

### Review round (2026-09-09) — 10 confirmed findings from code review 819a01c

- `android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt` (MODIFIED) — extracted
  `vadGateFilteredFrame(frame, length, filter, out)` into the companion object (the missing test
  seam finding 1 asked for); `processVadFrame` now calls it instead of inlining
  normalize+filter, and reuses a new `filteredFrameScratch` instance field instead of allocating a
  `FloatArray` per frame (finding 9); `HIGHPASS_CUTOFF_HZ` and `VAD_RMS_NORMALIZATION_DIVISOR`
  changed from `private` to `internal const` so tests assert on the real production values
  (findings 2/3); merged the orphaned/stale `framesForSeconds` KDoc block and corrected the
  "preview slider is never inert" claim, the `isEnergyAboveGate` `@param` comment ("raw RMS /
  32768" -> filtered RMS / 32767), and the `processVadFrame` KDoc's stale `SILENCE_THRESHOLD`/raw
  per-chunk RMS description (finding 8)
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` (MODIFIED) — extracted
  `internal fun parseMinRecordingMs(json: JSONObject): Long` from `readConfig` so a JVM test can
  drive the real "advanced.minRecordingMs" org.json parse path (finding 5)
- `src-tauri/src/stt/groq_jni.rs` (MODIFIED) — doc comment now cites
  `KlarvoOverlayService.resolveMinRecordingMsForSilenceFilter(cachedConfig)` / the
  `silenceThreshold` field by symbol name instead of a line-number range that was already wrong
  (finding 4)
- `android/kotlin-test/com/klarvo/voice/HighpassFilterTest.kt` (MODIFIED) — `cutoffHz` now reads
  `KlarvoAudioRecorder.HIGHPASS_CUTOFF_HZ` (was a test-local `85f`); added
  `cutoff_isAt85Hz_minus3dbCorner` (asserts the -3dB corner gain ~0.7071 +/-0.02 at a literal 85 Hz
  probe tone, discriminating a drifted cutoff, e.g. 300 Hz, which the old DC/high-freq tests could
  not, finding 3); `rawAndFilteredRms` now calls the production
  `KlarvoAudioRecorder.vadGateFilteredFrame` seam instead of re-implementing
  normalize->filter->rms inline (finding 1)
- `android/kotlin-test/com/klarvo/voice/VadGateRmsFixtureTest.kt` (MODIFIED) — `divisor` now
  reads `KlarvoAudioRecorder.VAD_RMS_NORMALIZATION_DIVISOR` (finding 2); replaced the three
  inline-literal RMS tests with `fixtureVectors_matchExpectedRmsKotlin`, which actually loads
  `test-fixtures/wav-rms-vectors.json` via `org.json` and asserts every vector with a populated
  `expected_rms_kotlin` (finding 6)
- `android/kotlin-test/com/klarvo/voice/MinRecordingMsConfigTest.kt` (MODIFIED) — the "default"
  test now constructs `Config(...)` omitting `minRecordingMs` instead of round-tripping `500L`
  through itself (finding 5's tautology half); added
  `jsonParse_minRecordingMs_nonDefault_parsesFromConfigJsonString_ac5` and
  `jsonParse_minRecordingMs_default_whenAdvancedKeyAbsent_ac5`, both driving real
  `org.json.JSONObject` strings through `KlarvoApi.parseMinRecordingMs` (finding 5)
- `android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt` (MODIFIED) — energy-floor
  vectors now generate a raw i16 Nyquist-frequency square wave per vector's
  `target_normalized_rms`, route it through the real `vadGateFilteredFrame` +
  `calculateRmsFloat` production seam, and only then call `isEnergyAboveGate` -- instead of
  feeding a pre-computed `normalized_rms` straight into the bare `>=` (finding 7). A Butterworth
  highpass has exact unity gain at Nyquist (verified numerically before committing to this
  fixture design), so the filtered RMS lands within ~1e-5 of the target -- far inside the 0.001
  gap between neighboring vectors' thresholds
- `test-fixtures/wav-rms-vectors.json` (MODIFIED) — RMS-007's `divergence_reason` no longer cites
  the deleted `SilencePreFilter.computeWavRms`; now explains Kotlin has no WAV-container-decoding
  RMS consumer at all post-Story-7.3 (finding 6's second half)
- Story file: Task 6 checkbox corrected -- no longer claims `SilenceThresholdTest.kt` was
  extended (`git diff 1d3e31b..HEAD` shows it untouched); AC1's "or add a sibling test"
  alternative was used via `HighpassFilterTest.kt` instead (finding 10)

### Review round 2 (2026-09-09) — 7 confirmed round-2 findings + 1 conductor GATE-4 finding

- `android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt` (MODIFIED) — new companion
  `VadGateResult` data class + `vadGateDecision(frame, length, filter, out, threshold, isSpeech)`
  seam that owns the FULL gate decision (filter+RMS+Silero call), replacing the inline
  filter/RMS/Silero sequence in `processVadFrame` (R2-P2); `vadGateFilteredFrame` changed from
  public to `internal` + `@VisibleForTesting`; `HIGHPASS_CUTOFF_HZ`/`VAD_RMS_NORMALIZATION_DIVISOR`
  gained `@VisibleForTesting` (R2-P7); corrected the present-tense `SILENCE_THRESHOLD` state-machine
  KDoc and class-header KDoc to `energyGateThreshold` (R2-P3); dropped the stale `:312`
  `[calculateRms]` line-number anchor, symbol-only now (R2-P4)
- `android/kotlin-test/com/klarvo/voice/HighpassFilterTest.kt` (MODIFIED) — added
  `vadGateDecision_feedsFilteredFrameToIsSpeech_notRawFrame` (drives the new seam with a spy
  `isSpeech` lambda, asserts it receives the FILTERED frame via a DC-attenuation margin) and its
  inversion `inversion_rawConstantFrame_wouldNotBeAttenuated` (R2-P2)
- `android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt` (MODIFIED) — added
  `bassToneShorts`; VAD-GATE-001 (default, gate-closed) now uses a 30 Hz bass tone instead of a
  Nyquist square wave, making the highpass filter load-bearing for its outcome; all six
  energy-floor vectors now read `amplitude_short` as a fixture-literal (precomputed offline from
  32767) instead of deriving it from `KlarvoAudioRecorder.VAD_RMS_NORMALIZATION_DIVISOR` at test
  time (R2-P1)
- `android/kotlin-test/com/klarvo/voice/VadGateRmsFixtureTest.kt` (MODIFIED) — `samplesFor` gained
  a dedicated `"silence"` branch and now reads `amplitude` via `getDouble` (throws on a
  missing/malformed key) instead of `optDouble(..., 0.0)` (R2-P5); `fixtureVectors_matchExpectedRmsKotlin`
  now asserts the exact exercised-ID set `{RMS-003, RMS-004, RMS-005, RMS-006}` via `assertEquals`
  instead of `assertTrue(exercised > 0)` (R2-P6)
- `test-fixtures/vad-gate-golden-vectors-7-2.json` (MODIFIED) — VAD-GATE-001 reworked to a bass-tone
  signal; all energy-floor vectors carry a literal `amplitude_short` (and `signal`/`signal_freq_hz`
  where applicable) instead of `target_normalized_rms` (R2-P1)
- `test-fixtures/wav-rms-vectors.json` (MODIFIED) — RMS-003/RMS-006 `wav_encoding.type` changed
  from `"synthetic"` to `"silence"`, dropping the now-redundant `amplitude: 0.0` key (R2-P5)
- `src-tauri/src/pipeline.rs` (MODIFIED) — added an additive `"silence"` match arm to the
  test-only `build_vector_wav` helper (`spec_wav_rms_vectors_json`) so the shared fixture's new
  `"silence"` encoding type still builds (byte-identical WAV to the old
  `"synthetic"`+`amplitude:0.0` path) for the Rust consumer too (R2-P5 consequence, not a
  production behavior change; test-only code)
- `scripts/android-smoke.sh` (MODIFIED) — duplicated `android-build.sh`'s idempotent, grep-guarded
  `testImplementation("org.json:json:20231013")` gradle patch, applied before the
  "JVM-Unit-Tests" step (GATE-4: this script runs `:app:testUniversalDebugUnitTest` but never
  carried the patch `MinRecordingMsConfigTest`/`VadGateRmsFixtureTest` need on a tree whose
  `build.gradle.kts` predates this story)
- `scripts/android-build.sh` (MODIFIED) — added a cross-reference comment pointing to the
  duplicated patch in `android-smoke.sh`

## Change Log

- 2026-09-09: Story 7.2 implementation — Android live auto-stop VAD-gate parity (H1, H17, M2, M3,
  M4, L1). Ported Rust's 85 Hz Butterworth highpass filter to Kotlin (`HighpassFilter.kt`, AC3);
  routed the VAD-gate RMS and Silero inference through the filtered+normalized frame (AC1);
  corrected the RMS normalization divisor to 32767f (AC4); corrected `VAD_FRAMES_PER_SECOND` to
  the exact 31.25 with `ceil` (AC6); added a uniform 200ms/7-frame hangover floor to
  `framesForSeconds`, applied to both autostop and preview-pause per GATE-1 (AC2); wired
  `minRecordingMs` into `AppConfig`/the pre-STT JNI call site, replacing hardcoded literals (AC5);
  corrected the stale `groq_jni.rs` doc comment. Added 4 new Kotlin test files + extended
  `PreviewPauseFramesTest.kt`, all with paired inversion checks; populated
  `expected_rms_kotlin` in `wav-rms-vectors.json`; seeded a new AC7 golden-vector fixture for
  Story 7.7. 151 Kotlin JVM unit tests green (0 failures), 657 Rust lib tests green, 6/6
  `pi_security output` tests green. On-device/emulator smoke: build+install mechanical gate
  attempted by the dev agent; the live-speech "feels correct" perceptual judgment is Andi's
  explicit human gate and remains open — story held short of `done` pending that gate.
- 2026-09-09 (review round): applied all 10 confirmed findings from code review `819a01c`
  (test-binding + comment-accuracy fixes; no production numeric behavior changed beyond the two
  doc-comment corrections). Extracted the missing `vadGateFilteredFrame` production seam and
  bound `HighpassFilterTest`/the golden vectors to it; made the divisor and cutoff constants
  `internal` and asserted tests against them directly; corrected the `groq_jni.rs` stale line
  reference to symbol names; added a real `org.json`-backed JSON-parse test for `minRecordingMs`
  (required adding `testImplementation("org.json:json:20231013")` to the generated
  `app/build.gradle.kts` via `scripts/android-build.sh`, since android.jar's `JSONObject` is a
  "not mocked" stub under JVM unit tests); made `VadGateRmsFixtureTest` actually load its fixture;
  reworked the AC7 energy-floor golden vectors to route raw signals through the production RMS
  seam instead of asserting a pre-computed number into the unchanged `>=` comparison; fixed the
  orphaned/stale KDoc blocks in `KlarvoAudioRecorder.kt`; removed the per-frame `FloatArray`
  allocation on the audio thread in favor of a reused scratch buffer; corrected the Task 6
  checkbox text. 153 Kotlin JVM unit tests green (0 failures, up from 151 -- net new test methods
  minus removed tautological ones), 657 Rust lib tests green (unchanged), 6/6 `pi_security output`
  tests green (unchanged). Did not touch the 2 deferred findings or re-attempt the on-device
  human-perception gate (out of this round's scope).
- 2026-09-09 (review round 2, FINAL fix round): applied all 7 confirmed round-2 `[Review][Patch]`
  findings plus 1 conductor-reported GATE-4 finding; did not touch any `[Review][Defer]` item.
  R2-P1: reworked the AC7 energy-floor golden vectors — VAD-GATE-001 now uses a 30 Hz bass tone
  (the highpass filter is load-bearing for its gate-closed outcome), and all six vectors read a
  fixture-literal `amplitude_short` instead of deriving it from the production divisor constant at
  test time. R2-P2: extended the missing-Silero-seam gap into a new `vadGateDecision` companion
  function covering the whole gate decision (filter+RMS+Silero call via an injected `isSpeech`
  function), with a dedicated spy-lambda test proving it feeds the FILTERED frame to Silero, not
  the raw one. R2-P3: replaced the remaining present-tense `SILENCE_THRESHOLD` KDoc references
  (state-machine block + class header) with the real `energyGateThreshold` symbol; left the
  genuinely historical past-tense "Previously: ... SILENCE_THRESHOLD" prose untouched. R2-P4:
  dropped the re-introduced stale `:312` line-number anchor on `[calculateRms]`, symbol-only now.
  R2-P5: `VadGateRmsFixtureTest`'s `samplesFor` now has a dedicated `"silence"` encoding type and
  reads `amplitude` via `getDouble` (fails loudly on a missing key) instead of `optDouble(...,
  0.0)`; the shared `wav-rms-vectors.json` fixture's RMS-003/RMS-006 use the new `"silence"` type
  (required an additive, behavior-preserving `"silence"` arm in Rust's test-only
  `pipeline.rs::build_vector_wav` so the shared fixture still builds for the Rust consumer — no
  production Rust code touched). R2-P6: the fixture-coverage assertion now checks the exact
  exercised-ID set (`{RMS-003, RMS-004, RMS-005, RMS-006}`) instead of `exercised > 0`. R2-P7:
  `vadGateFilteredFrame` changed from public to `internal` + `@VisibleForTesting`, matching its two
  sibling constants (also now annotated). GATE-4 (conductor emulator smoke on a freshly generated
  tree): duplicated the `org.json:json` gradle testImplementation patch from `android-build.sh`
  into `android-smoke.sh` (which runs the same JVM-unit-test gate but never carried the patch) —
  cross-referenced in both files, not extracted into a shared helper (too small to justify a new
  sourcing convention); `android-emulator-smoke.sh` intentionally untouched (separate pre-existing
  gap, out of scope). Result: 155 Kotlin JVM unit tests green (0 failures, up from 153 -- net +2
  new methods), 657 Rust lib tests green (unchanged — the new Rust match arm is additive/test-only
  and verified byte-identical), 6/6 `pi_security output` tests green (unchanged). Did not
  re-attempt the on-device human-perception gate (unrelated to these findings, remains Andi's open
  gate per Task 7).
- 2026-09-10 (conductor close-out, Andi at GATE 3): story → `done` in both status fields. Review
  loop: 2 fix rounds, ended on a scoped re-review (`a1ed23c`); the 8 round-3 findings (test-claim
  accuracy, vacuous-pass guards, stale comments) are accepted residual → `docs/backlog.md`
  "Story 7-2 residuals". GATE-4 (conductor, laptop emulator, APK `0318286`): `android-smoke.sh`
  green; real recordings via bubble tap logged `requiredSilentFrames=63` at 2.0 s (was 62) and
  `=7` at 0.05 s (was 1), `energyGate` follows `advanced.silenceThreshold`; recording-state window
  structure identical to the 11-3 canon. Evidence: `gate4-evidence/7-2/`. Residual for Andi's real
  device: how the auto-stop timing FEELS at default/tuned settings (perceptual, batched with the
  next fresh APK). Not observed on the proxy: AC5 `minRecordingMs` at the pre-STT stop path
  (covered by JVM tests + review).
- 2026-09-10 (post close-out, conductor): AC5 stop-path now observed on the proxy —
  `advanced.minRecordingMs=3000` + 1.2 s recording + Send tap logged
  `pre-STT filter: TooShort (1280ms < 3000ms)` (old code: hardcoded 500 → Pass). First attempt
  crashed with `UnsatisfiedLinkError` because `android-smoke.sh` installs without
  `--abi arm64-v8a` (tooling residual → `docs/backlog.md`). Evidence: `gate4-evidence/7-2/`.
- 2026-09-10 (real device, Andi): APK `0318286` installed on the Xiaomi 23054RA19C over wireless
  debugging (update in place, settings kept). Auto-stop at 2.0 s / 1.0 s and preview-pause at 0.5 s
  confirmed by feel. The human perception gate of Task 7 is closed.
