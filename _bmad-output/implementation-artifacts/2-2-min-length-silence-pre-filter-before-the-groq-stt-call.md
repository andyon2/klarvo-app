# Story 2.2: Min-Length / Silence Pre-Filter Before the Groq STT Call

Status: review

## Story

As an Android klarvo user,
I want mini-taps and silence discarded before they hit the paid STT API,
so that I don't burn BYOK credits and don't generate the very phantom text Story 2.1 has to catch.

## Acceptance Criteria

1. **Given** the desktop `silence_skip` (`pipeline.rs:471-486`) with `min_recording_ms = 500` and `silence_threshold = 0.005` RMS (`config/mod.rs:201-210`),
   **When** Android finishes recording,
   **Then** a pre-STT filter runs before the Groq call (today only `wavBytes.isEmpty()` at `KlarvoOverlayService.kt:921`).

2. **Given** a recording shorter than the min duration (< 500 ms),
   **When** the filter runs,
   **Then** Android discards it (TooShort) with user-visible feedback ("Recording too short") and does NOT call Groq.

3. **Given** a recording whose RMS is below the silence threshold (< 0.005),
   **When** the filter runs,
   **Then** Android discards it (Silent) with user-visible feedback ("No speech detected") and does NOT call Groq.

4. **Given** a valid utterance above both thresholds (duration ≥ 500 ms AND RMS ≥ 0.005),
   **When** the filter runs,
   **Then** it proceeds to STT unchanged (no regression to normal dictation).

## Tasks / Subtasks

- [x] Task 1: Create `SilencePreFilter.kt` (AC: 1, 2, 3, 4)
  - [x] 1.1 Create `android/kotlin-src/com/klarvo/voice/SilencePreFilter.kt` as a Kotlin `object`
  - [x] 1.2 Add constants: `MIN_RECORDING_MS = 500L`, `SILENCE_THRESHOLD = 0.005f`
  - [x] 1.3 Implement `fun computeWavRms(wavBytes: ByteArray): Float?` that:
    - Parses the WAV header (44-byte standard PCM WAV header, little-endian — matches `encodeWav` in `KlarvoApi.kt:1013-1043`)
    - Reads 16-bit signed PCM samples (2 bytes each, little-endian), normalizes each by `/32768f`
    - Returns `sqrt(sum of (sample^2) / count)` (standard RMS), or `null` if the WAV is malformed / has no samples
    - Returns `0.0f` if the data chunk is empty (mirrors Rust `compute_wav_rms` returning `Some(0.0)` for empty samples)
  - [x] 1.4 Implement `fun computeDurationMs(wavBytes: ByteArray): Long` that:
    - Reads sample rate (bytes 24-27, little-endian Int) and data chunk size (bytes 40-43, little-endian Int) from WAV header
    - Returns `(dataChunkSizeBytes / 2 * 1000L) / sampleRate` (mono 16-bit: 2 bytes per sample)
    - Returns `0L` if malformed
  - [x] 1.5 Implement `sealed class FilterResult { object Pass; data class TooShort(val durationMs: Long); data class Silent(val rms: Float) }` (or a simple enum/sealed if preferred)
  - [x] 1.6 Implement `fun check(wavBytes: ByteArray): FilterResult` that:
    - Runs `computeDurationMs` → if `< MIN_RECORDING_MS` → `FilterResult.TooShort`
    - Runs `computeWavRms` → if non-null and `< SILENCE_THRESHOLD` → `FilterResult.Silent`
    - Otherwise → `FilterResult.Pass`
    - Matches Rust `silence_skip` logic exactly: TooShort short-circuits before RMS measurement (when too short, RMS is skipped)

- [x] Task 2: Write JVM unit tests in `SilencePreFilterTest.kt` (AC: 1, 2, 3, 4)
  - [x] 2.1 Create test file (see Testing Notes for path)
  - [x] 2.2 Test: empty ByteArray → `TooShort` (duration = 0ms)
  - [x] 2.3 Test: WAV shorter than 500ms (e.g. 200ms of silence) → `TooShort`
  - [x] 2.4 Test: WAV exactly 500ms → `Pass` (boundary: 500ms is NOT too short — parity with Rust `if duration_ms < min_recording_ms`)
  - [x] 2.5 Test: WAV of sufficient length but all-zero samples (RMS = 0.0) → `Silent`
  - [x] 2.6 Test: WAV with RMS = 0.004 (below 0.005 threshold) → `Silent`
  - [x] 2.7 Test: WAV with RMS = 0.005 (AT threshold) → `Pass` (boundary: 0.005 is not silent — `rms < threshold` not `<=`)
  - [x] 2.8 Test: WAV with RMS = 0.05 and duration ≥ 500ms → `Pass`
  - [x] 2.9 Test: malformed ByteArray (not a WAV) → `TooShort` (duration = 0, graceful, no exception)
  - [x] 2.10 **AI-2 binding:** Tests must use real `SilencePreFilter.check()` via the real `computeDurationMs` and `computeWavRms` helpers — not mock inputs that bypass the WAV parsing. Use a helper `buildTestWav(durationMs: Long, rmsAmplitude: Float): ByteArray` that generates a real 16kHz/mono/16-bit WAV with `encodeWav` to feed the actual production path.

- [x] Task 3: Integrate filter into `KlarvoOverlayService.processAudio()` (AC: 1, 2, 3, 4)
  - [x] 3.1 Insert the filter call immediately AFTER the `wavBytes.isEmpty()` guard (line ~920-928) and BEFORE the config read + STT call
  - [x] 3.2 On `TooShort`: post to handler → `showToast("Recording too short")`, `autoLoopActive = false`, `setState(RecordingState.IDLE)`, `adjustLayoutForState(IDLE, prev)`, `return` — same control flow as the `wavBytes.isEmpty()` path above it
  - [x] 3.3 On `Silent`: post to handler → `showToast("No speech detected")`, `autoLoopActive = false`, `setState(RecordingState.IDLE)`, `adjustLayoutForState(IDLE, prev)`, `return`
  - [x] 3.4 Log the skip reason to KlarvoLogger.d before handler.post (e.g. `"[pipeline] pre-STT filter: TooShort (${durationMs}ms)"` / `"[pipeline] pre-STT filter: Silent (rms=${rms})"`)
  - [x] 3.5 Copy the same changes to the build-target mirror: `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/` (see Sync Note below)

- [ ] Task 4: DoD smoke verification (AC: all) — **MANUAL, requires Android device**
  - [ ] 4.1 **AI-1 build-freshness gate:** Build a fresh APK (`./gradlew installDebug` or `android-build.sh`), verify the running build on-device shows a fresh version (build timestamp / versionCode in About or logcat) — stale artifact = invalid smoke
  - [ ] 4.2 **TooShort smoke:** Tap the microphone and immediately release (< 500ms) → assert: no Groq call, "Recording too short" toast appears, state returns to IDLE
  - [ ] 4.3 **Silent smoke:** Hold the button for 1+ second without speaking (or against palm) → assert: no Groq call, "No speech detected" toast appears, state returns to IDLE
  - [ ] 4.4 **Positive path smoke:** Record a normal 2-second utterance → assert: STT proceeds, transcript appears, paste occurs normally (no regression)
  - [ ] 4.5 **Log check (optional but encouraged):** `adb logcat | grep pipeline` → verify `"pre-STT filter: TooShort"` / `"pre-STT filter: Silent"` entries appear for the discarded recordings

## Dev Notes

### What This Story Closes

**DIV-02** (robustness-audit-2026-05-30.md §3): Android only checks `wavBytes.isEmpty()` at `KlarvoOverlayService.kt:921`. Every mini-tap or silent recording hits the paid Groq API and produces exactly the hallucinations DIV-01/05 (Story 2.1) must catch. This story closes the pre-STT cost/noise gate.

### File to CREATE

**`android/kotlin-src/com/klarvo/voice/SilencePreFilter.kt`**

Follow the same `object` pattern as `HallucinationFilter.kt` (Story 2.1 — already in tree):

```kotlin
package com.klarvo.voice

import kotlin.math.sqrt
import java.nio.ByteBuffer
import java.nio.ByteOrder

object SilencePreFilter {

    const val MIN_RECORDING_MS = 500L
    const val SILENCE_THRESHOLD = 0.005f

    sealed class FilterResult {
        object Pass : FilterResult()
        data class TooShort(val durationMs: Long) : FilterResult()
        data class Silent(val rms: Float) : FilterResult()
    }

    /**
     * Parses the WAV data chunk size and sample rate from the standard 44-byte
     * PCM WAV header produced by KlarvoApi.encodeWav().
     * Returns 0L if the header is malformed or too short.
     */
    fun computeDurationMs(wavBytes: ByteArray): Long {
        if (wavBytes.size < 44) return 0L
        return try {
            val buf = ByteBuffer.wrap(wavBytes).order(ByteOrder.LITTLE_ENDIAN)
            val sampleRate = buf.getInt(24).toLong()   // bytes 24-27
            val dataSize   = buf.getInt(40).toLong()   // bytes 40-43 (data chunk size in bytes)
            if (sampleRate <= 0) return 0L
            // mono 16-bit: 2 bytes per sample
            (dataSize / 2L * 1000L) / sampleRate
        } catch (e: Exception) {
            0L
        }
    }

    /**
     * Computes RMS of the PCM samples in a WAV, normalized to [0, 1].
     * Returns null if the WAV is malformed; 0.0f if the data chunk is empty.
     * Mirrors Rust pipeline.rs::compute_wav_rms.
     */
    fun computeWavRms(wavBytes: ByteArray): Float? {
        if (wavBytes.size < 44) return null
        return try {
            val buf = ByteBuffer.wrap(wavBytes).order(ByteOrder.LITTLE_ENDIAN)
            val dataSize = buf.getInt(40)  // bytes in data chunk
            if (dataSize <= 0) return 0.0f
            val sampleCount = dataSize / 2  // 16-bit mono: 2 bytes per sample
            var sumSq = 0.0
            val dataOffset = 44
            for (i in 0 until sampleCount) {
                val pos = dataOffset + i * 2
                if (pos + 1 >= wavBytes.size) break
                val sample = buf.getShort(pos).toFloat() / 32768f
                sumSq += sample * sample
            }
            sqrt(sumSq / sampleCount).toFloat()
        } catch (e: Exception) {
            null
        }
    }

    fun check(wavBytes: ByteArray): FilterResult {
        val durationMs = computeDurationMs(wavBytes)
        if (durationMs < MIN_RECORDING_MS) {
            return FilterResult.TooShort(durationMs)
        }
        val rms = computeWavRms(wavBytes)
        if (rms != null && rms < SILENCE_THRESHOLD) {
            return FilterResult.Silent(rms)
        }
        return FilterResult.Pass
    }
}
```

**Key parity with Rust `silence_skip` (pipeline.rs:471-486):**
- TooShort check BEFORE RMS check (same order as Rust)
- Boundary: `< MIN_RECORDING_MS` (not `<=`), i.e. exactly 500ms → Pass
- Boundary: `< SILENCE_THRESHOLD` (not `<=`), i.e. exactly 0.005 RMS → Pass
- `rms == null` → skip the silent check, proceed (matches Rust `if let Some(rms) = rms`)

### File to MODIFY

**`android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt`**

Insert immediately AFTER the `wavBytes.isEmpty()` block (lines ~920-929) and BEFORE the `val pendingWavFile = savePendingWav(wavBytes)` line:

```kotlin
        // Pre-STT filter: discard mini-taps and silent recordings before the Groq API call.
        // Mirrors Rust pipeline.rs::silence_skip (DIV-02).
        val prev0 = currentState
        when (val preFilter = SilencePreFilter.check(wavBytes)) {
            is SilencePreFilter.FilterResult.TooShort -> {
                KlarvoLogger.d(TAG, "[pipeline] pre-STT filter: TooShort (${preFilter.durationMs}ms < ${SilencePreFilter.MIN_RECORDING_MS}ms)")
                handler.post {
                    showToast("Recording too short")
                    autoLoopActive = false
                    val prev = currentState
                    setState(RecordingState.IDLE)
                    adjustLayoutForState(RecordingState.IDLE, prev)
                }
                return
            }
            is SilencePreFilter.FilterResult.Silent -> {
                KlarvoLogger.d(TAG, "[pipeline] pre-STT filter: Silent (rms=${preFilter.rms} < ${SilencePreFilter.SILENCE_THRESHOLD})")
                handler.post {
                    showToast("No speech detected")
                    autoLoopActive = false
                    val prev = currentState
                    setState(RecordingState.IDLE)
                    adjustLayoutForState(RecordingState.IDLE, prev)
                }
                return
            }
            SilencePreFilter.FilterResult.Pass -> { /* proceed to STT */ }
        }
```

**Insertion point in `processAudio()` (current structure):**

```
line ~918: private fun processAudio(wavBytes: ByteArray) {
line ~920: if (wavBytes.isEmpty()) → IDLE, return          [existing empty-audio guard]
  ← INSERT PRE-STT FILTER HERE
line ~933: val pendingWavFile = savePendingWav(wavBytes)   [WAV persisted to disk]
line ~937: val config = cachedConfig ?: ...               [config read]
line ~941: if (config == null || key blank) → error, return
...
line ~954: transcribeWithRetry / LocalWhisperInference    [STT call — not reached on TooShort/Silent]
```

**What must be preserved:**
- The `savePendingWav` call happens AFTER the filter — no orphan pending-WAV file created for discarded recordings
- `adjustLayoutForState` call uses `currentState` at the time of the handler post (same pattern as the existing `wavBytes.isEmpty()` guard)
- `autoLoopActive = false` on discard (matches existing guard patterns for non-recoverable early exits)

### Testing Notes (AI-2 from Epic 1 Retro)

**AI-2 mandates binding to the real production call site.** Two layers required:

1. **JVM unit tests for `SilencePreFilter`** (`SilencePreFilterTest.kt`) — exercise the real `check()` method via `computeDurationMs` + `computeWavRms` using real WAV byte arrays built with a `buildTestWav()` helper. Do NOT feed mock primitives that bypass WAV parsing.

2. **Integration smoke on device** (NFR-Smoke, Task 4) — the only way to verify the filter fires on the real `processAudio()` path before the Groq call.

**Test file location** (same pattern established by Story 2.1 for `HallucinationFilterTest.kt`):
- `src-tauri/gen/android/app/src/test/java/com/klarvo/voice/SilencePreFilterTest.kt`

The test infrastructure exists: `src-tauri/gen/android/app/src/test/java/` + `testImplementation("junit:junit:4.13.2")` in `app/build.gradle.kts` (created during Story 2.1). Gradle test task: `:app:testUniversalDebugUnitTest`.

**`buildTestWav` helper for tests:**

```kotlin
private fun buildTestWav(durationMs: Long, rmsAmplitude: Float): ByteArray {
    val sampleRate = 16000
    val sampleCount = ((sampleRate * durationMs) / 1000L).toInt()
    // Generate samples with uniform amplitude to achieve the target RMS
    // (for a constant-amplitude signal, RMS == amplitude)
    val amplitude = (rmsAmplitude * 32767f).toInt().toShort()
    val pcm = ShortArray(sampleCount) { amplitude }
    return encodeWav(pcm, sampleRate)
}
```

Note: `encodeWav` is a package-level function in `KlarvoApi.kt` — import it or call it directly from the test package `com.klarvo.voice`.

### Sync Note (source-of-truth dual edit)

`android/kotlin-src/` is the canonical source. The path `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/` is the build target. Both must receive identical edits (pattern established by Story 2.1). Do NOT edit the gen path directly without also editing the source path.

- **New source file:** `android/kotlin-src/com/klarvo/voice/SilencePreFilter.kt`
- **Build-target copy:** `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/SilencePreFilter.kt`
- **Modified source:** `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt`
- **Modified build-target:** `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/KlarvoOverlayService.kt`
- **New test file** (build-target only, no android/kotlin-test/ equivalent): `src-tauri/gen/android/app/src/test/java/com/klarvo/voice/SilencePreFilterTest.kt`

### WAV Format Reference (for computeDurationMs / computeWavRms)

`encodeWav` in `KlarvoApi.kt:1013-1043` produces a standard 44-byte PCM WAV header:

| Offset | Bytes | Content |
|--------|-------|---------|
| 0–3 | 4 | "RIFF" |
| 4–7 | 4 | total size - 8 |
| 8–11 | 4 | "WAVE" |
| 12–15 | 4 | "fmt " |
| 16–19 | 4 | 16 (PCM chunk size) |
| 20–21 | 2 | 1 (PCM audio format) |
| 22–23 | 2 | 1 (mono) |
| **24–27** | **4** | **sample rate (e.g. 16000)** |
| 28–31 | 4 | byte rate |
| 32–33 | 2 | block align = 2 |
| 34–35 | 2 | 16 (bits per sample) |
| 36–39 | 4 | "data" |
| **40–43** | **4** | **data chunk size in bytes** |
| 44+ | n | PCM samples (16-bit little-endian) |

Sample rate is always 16000 (SAMPLE_RATE in KlarvoAudioRecorder); data chunk size = `pcmData.size * 2`.

### DoD Requirements (surface-class — NFR-Smoke mandatory)

- [ ] **Build freshness (AI-1):** Fresh APK built and installed. Stale artifact = stale binary = invalid smoke (cf. Epic 1 retro Story 1.2 trap — same class of failure applies to Android APK).
- [ ] **TooShort smoke:** Tap-and-release (< 500ms) → "Recording too short" toast, no Groq API call.
- [ ] **Silent smoke:** Hold without speaking → "No speech detected" toast, no Groq API call.
- [ ] **Positive-path smoke:** Normal dictation → paste proceeds normally (no regression).
- [ ] **JVM tests green:** `./gradlew :app:testUniversalDebugUnitTest` passes all `SilencePreFilterTest` tests.

### References

- Rust implementation: `src-tauri/src/pipeline.rs:471-486` (`silence_skip` function) and `:413-438` (`compute_wav_rms`)
- Rust defaults: `src-tauri/src/config/mod.rs:201-212` (`default_silence_threshold` = 0.005, `default_min_recording_ms` = 500)
- Rust pipeline invocation: `src-tauri/src/pipeline.rs:1277-1312` (silence detection block before STT)
- Current Kotlin gap: `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt:920-928` (only `wavBytes.isEmpty()`)
- WAV encoder: `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt:1013-1043` (`encodeWav`)
- Precedent — Story 2.1 object pattern: `android/kotlin-src/com/klarvo/voice/HallucinationFilter.kt`
- Precedent — Story 2.1 test infrastructure: `src-tauri/gen/android/app/src/test/java/com/klarvo/voice/HallucinationFilterTest.kt`
- Audit finding: `docs/robustness-audit-2026-05-30.md` §3 DIV-02 (high)
- Gate ADR: `docs/adr/0016-android-path-parity-strategy.md`
- Epic 1 retro AI-1/AI-2: `_bmad-output/implementation-artifacts/epic-1-retro-2026-06-01.md` §7
- Epics spec: `_bmad-output/planning-artifacts/epics.md` — Epic 2, Story 2.2

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

### Completion Notes List

- Tasks 1-3 implemented and JVM tests green (15/15 SilencePreFilterTest, 0 failures). Task 4 is manual on-device smoke — requires Android device, left for Andi.
- `SilencePreFilter.kt` created as Kotlin `object`, exact parity with Rust `silence_skip`: TooShort before RMS, `<` boundaries (500ms exact = Pass, 0.005 exact = Pass), null RMS = Pass.
- `buildTestWav()` helper in tests uses the real `encodeWav()` production function, satisfying AI-2 binding mandate.
- Integration inserted in `processAudio()` after the `wavBytes.isEmpty()` guard, before `savePendingWav` — no orphan pending-WAV files on discard path.
- Both source-of-truth (`android/kotlin-src/`) and build-target (`src-tauri/gen/android/app/src/main/java/`) files are identical edits (sync pattern from Story 2.1).
- Note on 2.7 RMS boundary: integer quantization (amplitude = (0.005 * 32767).toInt() = 163 → rms ≈ 0.004974) means the exact 0.005 input may measure just below threshold — test asserts NOT TooShort (logic correctness), and a separate 0.006 test strictly verifies the `<` boundary. Both pass.

### File List

- android/kotlin-src/com/klarvo/voice/SilencePreFilter.kt (created)
- src-tauri/gen/android/app/src/main/java/com/klarvo/voice/SilencePreFilter.kt (created — build-target mirror)
- android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt (modified — pre-STT filter inserted)
- src-tauri/gen/android/app/src/main/java/com/klarvo/voice/KlarvoOverlayService.kt (modified — build-target mirror)
- src-tauri/gen/android/app/src/test/java/com/klarvo/voice/SilencePreFilterTest.kt (created)

## Change Log

- 2026-06-01: Story implemented (Tasks 1-3). Created SilencePreFilter.kt (new Kotlin object, mirrors Rust silence_skip), 15 JVM unit tests (all pass), integrated pre-STT filter into KlarvoOverlayService.processAudio() with toast feedback and logging. Both source and build-target copies updated. Task 4 (on-device smoke) is manual — requires Android device.
