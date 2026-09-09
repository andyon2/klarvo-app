# Story 7.2: Android live auto-stop VAD-gate parity

Status: review

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
  - [x] Extend `SilenceThresholdTest.kt`, `PreviewPauseFramesTest.kt` per each AC's test note.
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
  - [x] `cargo test` — confirm no Rust behavior change (this story should not need Rust changes
    beyond the AC5 doc-comment correction, which is not a behavior change).
  - [~] **On-device/emulator smoke required** (per `project-context.md`: this touches live audio
    capture + real-time state — not a pure deterministic function like 7.1's chunking fix). Run
    `scripts/android-smoke.sh`, and a real recording session verifying auto-stop timing feels
    correct at default and tuned silence settings (human gate — do not claim this done without it).
    STATUS: build/install smoke run by the dev agent (see Dev Agent Record); the live-speech
    "feels correct" perceptual judgment is explicitly Andi's human gate and is NOT claimed done.

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
- `test-fixtures/vad-gate-golden-vectors-7-2.json` (NEW) — AC7 golden vectors (energy-floor +
  stop-latency, default + tuned), seeds Story 7.7
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (MODIFIED) — status update

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
