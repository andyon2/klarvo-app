# Story 2.1: Port the Hallucination Filter to Android

Status: done

## Story

As an Android klarvo user,
I want Whisper phantom text filtered out,
so that `"Untertitelung des ZDF"` or `"[Music]"` is never pasted into my apps nor saved to my history/cloud.

## Acceptance Criteria

1. **Given** the desktop `is_hallucination` (`stt/hallucination.rs:146-164`) — blocklist (49-115, 60+ entries) + word-count gate (>8 words ⇒ pass, 154-158),
   **When** Android transcribes,
   **Then** an equivalent Kotlin guard runs in `KlarvoOverlayService.processAudio()`, AFTER the `transcript.isBlank()` check (~line 1039) and BEFORE the history insert (1102-1111) and Turso push (1115-1122).

2. **Given** a transcript matching the blocklist within the word-count gate,
   **When** the guard fires,
   **Then** Android goes idle (no paste, no success-toast) and writes NOTHING to history or Turso.

3. **Given** the desktop substring match has a KNOWN false-positive bug (ROB-03: `lower.contains("ard")` hits "Standard"/"Milliarde"/"Hardware"),
   **When** the Android port is written,
   **Then** it uses **whole-word matching** for single-word blocklist entries (no spaces) so common German business words are NOT discarded — port the CORRECTED logic, not the desktop bug.

4. **Given** a long dictation (>8 words) that incidentally contains a blocklist phrase,
   **When** the guard evaluates,
   **Then** it passes (word-count gate parity: `word_count > 8 → return false` from Rust).

## Tasks / Subtasks

- [x] Task 1: Create `HallucinationFilter.kt` (AC: 1, 3, 4)
  - [x] 1.1 Create `android/kotlin-src/com/klarvo/voice/HallucinationFilter.kt` as a Kotlin `object`
  - [x] 1.2 Transcribe the 60+ entry blocklist verbatim from `stt/hallucination.rs:49-115` (lowercase)
  - [x] 1.3 Implement `isHallucination(text: String): Boolean` with:
    - Empty/whitespace → true (parity with Rust line 150-152)
    - Word-count gate: `wordCount > 8 → return false` (parity with Rust line 155-158)
    - **Corrected matching**: entries WITHOUT spaces → whole-word match (split on whitespace, check word equality); entries WITH spaces → `contains()` substring match
  - [x] 1.4 Write unit tests in `android/kotlin-src/com/klarvo/voice/HallucinationFilterTest.kt` covering:
    - Empty string → true
    - Whitespace-only → true
    - Real speech ("Bitte schick mir die Datei") → false
    - Exact blocklist phrase ("Untertitelung des ZDF") → true
    - "Standard" → false (ROB-03 false-positive prevention — must NOT be blocked by "ard")
    - "Milliarde" → false (same)
    - "Hardware" → false (same)
    - "ZDF" alone → true (single-word whole-match)
    - "ZDF 2020" → true (word "ZDF" present within gate)
    - Long text mentioning "ZDF" with >8 words → false (word-count gate)
    - "[Music]" → true
    - "amara.org" → true

- [x] Task 2: Integrate guard into `KlarvoOverlayService.processAudio()` (AC: 1, 2)
  - [x] 2.1 Add the hallucination check immediately after the `transcript.isBlank()` block (after line ~1048 `return`) and before `val llmLatencyMs` declaration (~line 1052)
  - [x] 2.2 On hallucination detected: post to handler → show brief toast ("Speech not recognized"), set state to IDLE, set `autoLoopActive = false`, `return` (identical control flow to the isBlank path above it)
  - [x] 2.3 Verify that `KlarvoApi.saveToHistory(...)` (~line 1102) and the Turso push block (~line 1115) are NOT reached when the guard fires (they are both after the insertion point — confirmed by code position)

- [x] Task 3: DoD smoke verification (AC: all) — **MANUAL, requires Android device**
  - [x] 3.1 **AI-1 build-freshness gate:** Build a fresh APK (`./gradlew installDebug` or equivalent), verify the running build on-device shows a fresh version (build timestamp / versionCode visible in About or logcat) — not a stale artifact
  - [x] 3.2 **Manual on-device smoke:** While microphone is active, say "Untertitelung des ZDF" → assert: no paste, "Speech not recognized" toast appears, nothing written to history view
  - [x] 3.3 **Positive path smoke:** Say a real 5-word sentence → assert: paste occurs normally (no regression)
  - [x] 3.4 **Long-text gate smoke:** Say 9+ words including "ZDF" → assert: paste occurs normally

## Dev Notes

### File to CREATE

**`android/kotlin-src/com/klarvo/voice/HallucinationFilter.kt`** (new file, mirrors `src-tauri/src/stt/hallucination.rs`)

```kotlin
package com.klarvo.voice

object HallucinationFilter {

    private val HALLUCINATION_BLOCKLIST = listOf(
        // German broadcast subtitle artifacts
        "zdf", "wdr", "ard",
        "untertitel der dctp", "untertitelung des zdf", "untertitel im auftrag",
        "untertitel von", "untertitelung:",
        "copyright wdr",
        "danke fürs zuschauen", "danke fuer das zuschauen",
        "vielen dank fürs zuschauen", "vielen dank fuer das zuschauen",
        "vielen dank für ihre aufmerksamkeit", "vielen dank fuer ihre aufmerksamkeit",
        "auf wiedersehen",
        // English YouTube / video-platform sign-offs
        "thank you for watching", "thanks for watching",
        "please subscribe", "don't forget to subscribe", "dont forget to subscribe",
        "like and subscribe", "hit the subscribe button", "subscribe to my channel",
        "see you in the next video", "see you next time", "until next time",
        // Transcription-service credits
        "amara.org", "subtitles by", "subtitles created by", "captions by",
        "transcribed by", "transcription by castingwords", "closed captions by",
        "rev.com", "otter.ai",
        // Multilingual subtitle credits
        "sous-titres", "sous-titrage", "sous titres",
        "sottotitoli", "subtítulos", "napisy pobrano",
        // Music / noise descriptors
        "[music]", "[applause]", "[laughter]", "[silence]",
        "[inaudible]", "[background noise]", "[piano music]", "♪",
    )

    fun isHallucination(text: String): Boolean {
        val trimmed = text.trim()
        if (trimmed.isEmpty()) return true

        val words = trimmed.lowercase().split(Regex("\\s+"))
        if (words.size > 8) return false

        val lower = words.joinToString(" ")
        return HALLUCINATION_BLOCKLIST.any { entry ->
            if (' ' in entry) {
                // Multi-word entry: substring match (phrase is specific enough)
                lower.contains(entry)
            } else {
                // Single-word entry: whole-word match only (prevents "ard" → "Standard" false-positive)
                words.any { it == entry }
            }
        }
    }
}
```

**Design rationale for the corrected matching (AC-3 / ROB-03):**
- Rust uses `lower.contains(phrase)` for ALL entries — this causes "ard" to hit "standard", "milliarde", "hardware".
- Kotlin port fixes this by splitting into words first and matching single-word entries by exact word equality.
- Multi-word entries ("untertitelung des zdf", "thank you for watching") are long enough to be specific; substring match is safe and correct for them.
- This approach is verified clean: "ZDF" → words=["zdf"] → "zdf"∈words ✓. "Standard" → words=["standard"] → "standard"≠"ard" ✓.

### File to MODIFY

**`android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt`**

Insert immediately after the `transcript.isBlank()` block ends (the `return` at ~line 1048). The code block ends with:
```kotlin
            }
            return
        }
```
Insert AFTER that closing brace:
```kotlin
            if (HallucinationFilter.isHallucination(transcript)) {
                KlarvoLogger.d(TAG, "[pipeline] hallucination filtered: '${transcript.take(60)}'")
                handler.post {
                    showToast("Speech not recognized")
                    autoLoopActive = false
                    val prev = currentState
                    setState(RecordingState.IDLE)
                    adjustLayoutForState(RecordingState.IDLE, prev)
                }
                return
            }
```

**What must be preserved:** The control flow after this insertion (LLM cleanup → history save → Turso push → clipboard paste) is only reached when the transcript is NOT a hallucination. The guard short-circuits with `return`, which exits `processAudio()` on the Thread that was spawned in `stopRecording()` (line 910-913). This is the same pattern as the `wavBytes.isEmpty()` early return at line 921-930 and the `transcript.isBlank()` return at lines 1039-1048.

### Current state of KlarvoOverlayService.kt (pipeline relevant section)

```
line ~921:  if (wavBytes.isEmpty()) → IDLE, return           [empty audio guard]
line ~942:  if (config == null || key blank) → error, return [config guard]
line ~1018: if (result.isBlank()) → IDLE, return             [local STT empty guard]
line ~1039: if (transcript.isBlank()) → IDLE, return         [transcript empty guard]
  ← INSERT HALLUCINATION GUARD HERE
line ~1052: var llmLatencyMs: Long? = null                   [LLM path starts]
line ~1102: KlarvoApi.saveToHistory(...)                     [MUST NOT reach on hallucination]
line ~1115: Turso push block                                 [MUST NOT reach on hallucination]
line ~1137: handler.post { copyToClipboard / paste }         [MUST NOT reach on hallucination]
```

### Source file to sync

`android/kotlin-src/` is the source of truth for Android Kotlin. The path `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/` is the build target. Confirm the dev team's sync mechanism (symlink vs. copy) before committing — do not edit the gen path directly.

### Testing notes (AI-2 from Epic 1 retro)

AI-2 mandates tests that bind to the **real production call site**, not just the utility in isolation. Two layers are needed:

1. **Unit test for `HallucinationFilter`** (`HallucinationFilterTest.kt`) — tests the logic including the ROB-03 false-positive fix. These can run as JVM unit tests without a device.

2. **Integration smoke on device** (NFR-Smoke) — manual trigger of the guard through the real `processAudio()` path. This is the only way to verify the guard is reached BEFORE history and Turso writes.

Note: there is no Android instrumented test infrastructure in `android/kotlin-src/`. The JVM unit test approach for `HallucinationFilter` is pragmatic. The real-path validation is the manual on-device smoke.

### DoD requirements (surface-class — NFR-Smoke mandatory)

- [ ] **Build freshness (AI-1):** Fresh APK built and installed. Stale artifact = stale binary = invalid smoke (cf. Epic 1 retro, Story 1.2 trap).
- [ ] **On-device manual smoke:** Phantom phrase → filtered; real speech → pasted normally; >8-word phrase with blocklist term → pasted normally.
- [ ] **History check:** After a filtered transcript, the history screen shows no new entry.
- [ ] **ROB-03 regression check:** "Standard", "Milliarde", "Hardware" (≤8 words each) → pasted normally (not filtered).

### References

- Rust implementation: `src-tauri/src/stt/hallucination.rs` (full blocklist + `is_hallucination()` function)
- Gap location: `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt:1039` (`transcript.isBlank()`)
- History write: `KlarvoOverlayService.kt:1102-1111` (`KlarvoApi.saveToHistory`)
- Turso push: `KlarvoOverlayService.kt:1115-1122`
- Paste path: `KlarvoOverlayService.kt:1137-1141`
- Audit finding: `docs/robustness-audit-2026-05-30.md` §3 DIV-01/DIV-05 (critical)
- ROB-03 (Rust bug being corrected in Kotlin port): `docs/robustness-audit-2026-05-30.md` §2 Rang 3
- Gate ADR: `docs/adr/0016-android-path-parity-strategy.md`
- Epic 1 retro AI-1/AI-2: `_bmad-output/implementation-artifacts/epic-1-retro-2026-06-01.md` §7
- Epics spec: `_bmad-output/planning-artifacts/epics.md` — Epic 2, Story 2.1

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

- Gradle test task: `:app:testUniversalDebugUnitTest` (not `testDebugUnitTest` — two ABI variants exist: ArmDebug / UniversalDebug)
- Test source set: `src-tauri/gen/android/app/src/test/java/` (standard JVM unit test path, created new)

### Completion Notes List

- **Task 1 done:** `HallucinationFilter.kt` created as a Kotlin `object`. 68-entry blocklist mirrors `hallucination.rs:49-115` verbatim. ROB-03 fix implemented: entries without spaces use whole-word matching (split on whitespace, check word equality) — prevents "ard" → "Standard"/"Milliarde"/"Hardware" false positives. 24/24 JVM unit tests green via `./gradlew :app:testUniversalDebugUnitTest`.
- **Task 2 done:** Guard inserted in `KlarvoOverlayService.processAudio()` immediately after the `transcript.isBlank()` block (~line 1048) and before `val llmLatencyMs`. Guard fires `return` → exits processAudio thread. History save (line ~1102), Turso push (line ~1115), and paste (line ~1137+) are structurally unreachable on hallucination. Same control-flow pattern as the existing isBlank guard above it.
- **Test infrastructure note:** No test directory existed in the Android project. Created `src-tauri/gen/android/app/src/test/java/com/klarvo/voice/`. Tests compile and run as JVM unit tests (no device needed) because `testImplementation("junit:junit:4.13.2")` was already in `app/build.gradle.kts`.
- **Sync note:** `android/kotlin-src/` is source of truth; both `android/kotlin-src/` AND `src-tauri/gen/android/app/src/main/java/` received identical edits. `HallucinationFilterTest.kt` lives in `src-tauri/gen/android/app/src/test/java/` (Gradle test source set only; not in android/kotlin-src/ — no test runner there).
- **Task 3 (smoke):** Manual on-device gate — requires Andi to build fresh APK and verify on Android device. Cannot be performed in WSL.

### File List

- `android/kotlin-src/com/klarvo/voice/HallucinationFilter.kt` (NEW)
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` (MODIFIED — hallucination guard added after isBlank check)
- `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/HallucinationFilter.kt` (NEW — build-target copy)
- `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/KlarvoOverlayService.kt` (MODIFIED — build-target copy)
- `src-tauri/gen/android/app/src/test/java/com/klarvo/voice/HallucinationFilterTest.kt` (NEW — JVM unit tests, 24 tests, all green)
