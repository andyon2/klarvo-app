# Story 9.4: Bubble State Harness (Verifiability Precursor — Before 9.5)

Status: review

## Story

As a developer (and as Andi the human tester),
I want a dev-only way to drive the bubble through all four Epic-9 states on demand,
so that the upcoming state UI (Story 9.5) is verifiable without live audio/network — built BEFORE the states.

## Acceptance Criteria

**AC1 — Debug broadcast receiver drives all four states on-device:**
Given the app is installed as a DEBUG build and `KlarvoOverlayService` is running
When `adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state <token>` is sent
Then the bubble transitions to the requested state immediately on the main thread
And `<token>` maps to: `"idle"` → `IDLE`, `"recording"` → `RECORDING`, `"transcribing"` → `TRANSCRIBING`, `"done"` → `DONE`
And the four new enum members (`IDLE / RECORDING / TRANSCRIBING / DONE`) replace the old `IDLE / RECORDING / RECORDING_PTT / PROCESSING` enum in both `FloatingBubbleView.State` and the internal `RecordingState` in `KlarvoOverlayService`

**AC2 — Synthetic RMS + transcript injection via harness:**
Given the `DEBUG_SET_STATE` broadcast
When it also carries `--ef rms <0.0–1.0>` and/or `--es transcript "<text>"`
Then `bubbleView.amplitude` is set to the RMS float (drives waveform bars in recording/transcribing states)
And the transcript string is stored for use by the listening-panel render in Story 9.5
And these extras are silently ignored in `idle` and `done` states (no crash)

**AC3 — Andi can reproduce each state himself (verifiability symmetry):**
Given the adb commands documented in Dev Notes
When Andi runs them from the Windows machine (WSL terminal pointing at the Tailscale-pinned device)
Then each of the four states is reachable within ≤2 commands (start service + send broadcast)
And no live microphone, Groq API key, or network call is required

**AC4 — Harness is dev-only / gated out in release builds:**
Given `BuildConfig.DEBUG == false` (release APK, i.e. the `android-build.sh` output)
When the service starts
Then the debug broadcast receiver is NOT registered (no `registerReceiver` call for `DEBUG_SET_STATE`)
And no harness-related `Intent` action string is exported in a release manifest
And the smoke APK (`android-smoke.sh`, which builds a DEBUG APK) DOES register the receiver

**AC5 — State-machine migration: old enum fully replaced by new enum:**
Given the enum migration
When the new `IDLE / RECORDING / TRANSCRIBING / DONE` enum is in place
Then every exhaustive `when` over the enum in both files is updated (no `RecordingState.RECORDING_PTT` or `RecordingState.PROCESSING` reference remains)
And the `setState()` dispatch in `KlarvoOverlayService` maps new states to `FloatingBubbleView.State` correctly
And the tap handler, silence guard, and mode→state mapping are rewritten to use the new enum (see Dev Notes for the exact call sites)

**AC6 — Existing live recording paths remain functional:**
Given the real recording flow (tap → startRecording())
When it runs on-device after the migration
Then HOLD mode still expands to the bar; TOGGLE/AUTOSTOP/AUTO still show the circular form (now `RECORDING` instead of `RECORDING_PTT`)
And the silence-detection guard fires correctly
And the old `RECORDING_PTT` push-to-talk path is collapsed into `RECORDING` (no behavior loss — PTT still works)
And `PROCESSING` is renamed `TRANSCRIBING` with no behavior change (pipeline still calls processAudio → setState(TRANSCRIBING) → setState(IDLE))

**AC7 — On-device demonstration: all four states reachable via harness:**
Given the harness is built and the DEBUG APK is installed
When each `adb shell am broadcast` command from Dev Notes is run
Then all four states are observable on-device, including waveform animation with a non-zero RMS value
And APK freshness verified via `scripts/android-smoke.sh` timestamp gate
And no crash or ANR occurs during any state transition

**Inversion (must-fail gate):** A submission that registers the debug receiver unconditionally (not gated by `BuildConfig.DEBUG`) must not pass review. A submission where `RECORDING_PTT` or `PROCESSING` still appear as live code references (not just comments) must not pass. A submission where the harness requires a live microphone, Groq API key, or network connection to operate must not pass.

**DoD:** On-device demonstration that all four states (idle/recording/transcribing/done) plus waveform/transcript can be triggered via the harness. All adb commands work from Andi's WSL terminal. No live audio/network required.

## Tasks / Subtasks

- [x] **Task 1: Migrate state enum — replace RECORDING_PTT/PROCESSING with TRANSCRIBING/DONE** (AC: 1, 5, 6)
  - [x] 1.1 In `FloatingBubbleView.kt` — change `enum class State { IDLE, RECORDING, RECORDING_PTT, PROCESSING }` to `enum class State { IDLE, RECORDING, TRANSCRIBING, DONE }`. Update `updateAnimators()`: `RECORDING_PTT ->` branch removed; `PROCESSING ->` becomes `TRANSCRIBING ->` (same spinner logic); new `DONE ->` arm (same as IDLE for now — scale-reset + no animators). Update `onDraw()`: rename `State.RECORDING_PTT` arm to `State.RECORDING` (keeping circular red form; Story 9.5 will replace it entirely). Rename `State.PROCESSING` arm to `State.TRANSCRIBING` (same teal squircle + spinner; Story 9.5 redesigns). Add `State.DONE` arm (show teal squircle with a checkmark drawn via `drawCheckMark()`). No new visual design required for 9.4 — DONE is a placeholder.
  - [x] 1.2 In `KlarvoOverlayService.kt` — change `private enum class RecordingState { IDLE, RECORDING, RECORDING_PTT, PROCESSING }` to `private enum class RecordingState { IDLE, RECORDING, TRANSCRIBING, DONE }`. Update `setState()` dispatch at ~line 1451 to map to new `FloatingBubbleView.State` values: `RECORDING_PTT -> FloatingBubbleView.State.RECORDING` (removed), `PROCESSING -> FloatingBubbleView.State.TRANSCRIBING` (renamed), `DONE -> FloatingBubbleView.State.DONE` (new).
  - [x] 1.3 Update the tap handler (`handleTouch`, the `when (currentState)` block at ~line 871): `RecordingState.RECORDING_PTT ->` becomes `RecordingState.RECORDING ->` (merge: keep the per-mode stop logic from the old PTT branch; the HOLD bar confirm/cancel path stays in a `bubbleView.isTouchInConfirmZone` sub-check within the same `RECORDING` branch). Remove the old separate `RecordingState.RECORDING ->` branch and fold it into the new unified `RECORDING` handler (see Dev Notes for merge strategy).
  - [x] 1.4 Update silence-detection guard at ~line 1022: `if (currentState != RecordingState.RECORDING && currentState != RecordingState.RECORDING_PTT) return` → `if (currentState != RecordingState.RECORDING) return`
  - [x] 1.5 Update `startRecording()` at ~line 1000–1014: remove the three-way split between PTT/HOLD/else. New logic: HOLD mode → `setState(RecordingState.RECORDING)` + `adjustLayoutForState(RECORDING, IDLE)` (bar expand); all other modes (TOGGLE/AUTOSTOP/AUTO/PTT) → `setState(RecordingState.RECORDING)` with NO `adjustLayoutForState` (circular, no bar). The `pushToTalkActive` flag still exists for PTT detection; see Dev Notes.
  - [x] 1.6 Update `processAudio()` and its completion callbacks: rename all `RecordingState.PROCESSING` references to `RecordingState.TRANSCRIBING`. Add `RecordingState.DONE` transition just before returning to `IDLE` (briefly set state to DONE then delay 800ms before `setState(IDLE)` + `adjustLayoutForState(IDLE, prev)`) — this is the placeholder DONE flash Story 9.5 will animate.
  - [x] 1.7 Update `adjustLayoutForState()` at ~line 705: rename `RecordingState.RECORDING_PTT` → `RecordingState.RECORDING` in the `else ->` branch. Add `RecordingState.DONE ->` mapped to the same touch-target dimensions as `RecordingState.IDLE`.
  - [x] 1.8 Update `cancelRecording()` at ~line 1047 and `onSilenceTriggered()` at ~line 1017: rename references from `RECORDING_PTT`/`PROCESSING` to `RECORDING`/`TRANSCRIBING` as appropriate.
  - [x] 1.9 Verify no remaining references to `RECORDING_PTT` or `PROCESSING` in live code (comments are OK); run `grep -n "RECORDING_PTT\|\.PROCESSING" android/kotlin-src/` — must return zero live code hits.

- [x] **Task 2: Add debug broadcast receiver (DEBUG-only, harness)** (AC: 1, 2, 4)
  - [x] 2.1 In `KlarvoOverlayService.kt` companion object, add constants (gated to debug intent action pattern):
    ```kotlin
    // Debug harness — registered only in debug builds (BuildConfig.DEBUG)
    private const val ACTION_DEBUG_SET_STATE = "com.klarvo.voice.DEBUG_SET_STATE"
    private const val EXTRA_STATE     = "state"      // "idle"|"recording"|"transcribing"|"done"
    private const val EXTRA_RMS       = "rms"        // Float 0.0–1.0 (synthetic amplitude)
    private const val EXTRA_TRANSCRIPT = "transcript" // String (synthetic raw text)
    ```
  - [x] 2.2 Add `private var debugTranscript: String = ""` field to `KlarvoOverlayService` (stores the injected transcript for Story 9.5's panel render to consume; treated as empty string in 9.4 since the panel doesn't exist yet).
  - [x] 2.3 Add the debug receiver field:
    ```kotlin
    private val debugStateReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action != ACTION_DEBUG_SET_STATE) return
            val stateToken = intent.getStringExtra(EXTRA_STATE) ?: return
            val rms = intent.getFloatExtra(EXTRA_RMS, -1f)
            val transcript = intent.getStringExtra(EXTRA_TRANSCRIPT)
            handler.post {
                val newState = when (stateToken.lowercase()) {
                    "idle"         -> RecordingState.IDLE
                    "recording"    -> RecordingState.RECORDING
                    "transcribing" -> RecordingState.TRANSCRIBING
                    "done"         -> RecordingState.DONE
                    else -> { KlarvoLogger.w(TAG, "DEBUG_SET_STATE: unknown token '$stateToken'"); return@post }
                }
                if (rms >= 0f) bubbleView.amplitude = rms.coerceIn(0f, 1f)
                if (transcript != null) debugTranscript = transcript
                setState(newState)
                KlarvoLogger.d(TAG, "[harness] state → $newState (rms=$rms, transcript=${transcript?.take(30)})")
            }
        }
    }
    ```
  - [x] 2.4 In `onCreate()`, after registering `notificationActionReceiver`, add a **debug-only** registration block:
    ```kotlin
    if (BuildConfig.DEBUG) {
        val debugFilter = IntentFilter(ACTION_DEBUG_SET_STATE)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(debugStateReceiver, debugFilter, RECEIVER_NOT_EXPORTED)
        } else {
            registerReceiver(debugStateReceiver, debugFilter)
        }
        KlarvoLogger.d(TAG, "[harness] debug broadcast receiver registered")
    }
    ```
  - [x] 2.5 In `onDestroy()`, add symmetrical debug-only unregister:
    ```kotlin
    if (BuildConfig.DEBUG) {
        try { unregisterReceiver(debugStateReceiver) } catch (e: IllegalArgumentException) {
            KlarvoLogger.w(TAG, "[harness] debugStateReceiver already unregistered", e)
        }
    }
    ```
  - [x] 2.6 Confirm `BuildConfig` import resolves — it lives in the generated `com.klarvo.voice` package (Tauri codegen injects it; already used by `Logger.kt` at `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/generated/Logger.kt:86`). No new import needed if the file is already in the same package.

- [x] **Task 3: Wire up DONE state visual (placeholder checkmark)** (AC: 1, 7)
  - [x] 3.1 In `FloatingBubbleView.onDraw()` `State.DONE` arm: draw the same teal squircle as IDLE, then overlay a white checkmark using the existing `drawCheckMark()` helper at center with `arm = visualRadius * 0.35f`. This is a placeholder — Story 9.5 will replace it with an animated transition.
  - [x] 3.2 In `KlarvoOverlayService.processAudio()` success path (after paste and before returning to IDLE), add the DONE flash:
    ```kotlin
    val prev = currentState
    setState(RecordingState.DONE)
    adjustLayoutForState(RecordingState.DONE, prev)
    handler.postDelayed({
        setState(RecordingState.IDLE)
        adjustLayoutForState(RecordingState.IDLE, RecordingState.DONE)
    }, 800L)
    ```
    (Replace the direct `setState(IDLE)` call in the success completion path with this pattern.)

- [x] **Task 4: Compile + verify** (AC: all)
  - [x] 4.1 Run `scripts/android-smoke.sh` — must exit 0 (Kotlin compile clean, DEBUG APK built and installed). **Result: JVM unit tests 60/60 PASS, Debug APK built (119 MB, 13s). adb install blocked by Xiaomi USER_RESTRICTED — requires on-device Andi gate (known constraint from `reference_android_bubble_canvas_and_install.md`). Build compile step PASS.**
  - [x] 4.2 Run `grep -n "RECORDING_PTT\|RecordingState\.PROCESSING\|FloatingBubbleView\.State\.PROCESSING" android/kotlin-src/com/klarvo/voice/*.kt` — must return zero live code hits (comments excluded). **Result: PASS — zero hits.**
  - [ ] 4.3 Run each harness broadcast from WSL and confirm the visual state on-device: **PENDING — requires Andi's on-device smoke (Xiaomi install restriction must be cleared first).** Commands ready:
    ```sh
    adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state idle
    adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording --ef rms 0.7 --es transcript "Hello world"
    adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state transcribing --ef rms 0.3 --es transcript "Hello world"
    adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state done
    ```
  - [ ] 4.4 Verify live dictation still works: tap bubble → records → processes → returns to idle (no regression from the enum migration). **PENDING — on-device smoke by Andi.**

- [x] **Task 5: Commit** (AC: all)
  - [x] 5.1 Stage only: `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt`, `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt`. Never `git add .`
  - [x] 5.2 Commit message: `feat(android): 9-4 bubble state harness — enum migration + debug broadcast`

## Dev Notes

### What This Story Does (Exactly)

Story 9.4 is a **precursor infrastructure story**. It does two things that are both required before 9.5:
1. **Migrates the state enum** from `IDLE/RECORDING/RECORDING_PTT/PROCESSING` to `IDLE/RECORDING/TRANSCRIBING/DONE` — the new four-state sequence Story 9.5 will implement visually.
2. **Adds the debug broadcast harness** so Andi can drive any state on-device without live audio. This satisfies verifiability symmetry (the states must be *reachable for test* before they are *built*, else 9.5 ships states no one can verify).

The visual rendering in 9.4 is minimal. `RECORDING` and `TRANSCRIBING` keep the existing circular form (now drawing with `KlarvoTheme.Danger` and `KlarvoTheme.Teal` respectively — same as old PTT/PROCESSING). `DONE` gets a placeholder teal squircle + checkmark. Story 9.5 replaces all visual rendering with the listening panel, waveform, and correct state transitions.

### State Machine Migration — Merge Strategy (Task 1)

The current tap handler has **two separate `when` branches** for `RECORDING` (bar tap logic) and `RECORDING_PTT` (circular stop logic). After migration, both collapse into a single `RecordingState.RECORDING ->` branch. The logic inside must handle both bar and non-bar modes:

```kotlin
RecordingState.RECORDING -> {
    when {
        // Bar mode (HOLD): check zone first
        bubbleView.state == FloatingBubbleView.State.RECORDING &&
        bubbleView.isTouchInCancelZone(touchX) -> cancelRecording()
        bubbleView.state == FloatingBubbleView.State.RECORDING &&
        bubbleView.isTouchInConfirmZone(touchX) -> stopRecordingAndProcess()
        // Circular mode (TOGGLE/AUTOSTOP/AUTO/PTT): tap stops recording
        else -> stopRecordingAndProcess()
    }
}
```

The `pushToTalkActive` flag is still needed to distinguish PTT (long-press driven) from tap modes, but it only affects `startRecording()` (PTT → no bar expand) and `handleLongPressRelease()` (PTT → stop on finger lift). The state enum does NOT need to distinguish PTT from non-PTT.

**Key call sites to update (with approximate line numbers based on 9.3 baseline):**

| Call site | File | Current ref | New ref |
|-----------|------|-------------|---------|
| Tap handler (RECORDING branch) | KlarvoOverlayService ~line 871 | `RECORDING ->` | merge with `RECORDING_PTT ->` |
| Tap handler (RECORDING_PTT branch) | KlarvoOverlayService ~line 898 | `RECORDING_PTT ->` | remove; fold into RECORDING |
| Silence guard | KlarvoOverlayService ~line 1022 | `!= RECORDING && != RECORDING_PTT` | `!= RECORDING` |
| startRecording() PTT branch | KlarvoOverlayService ~line 1003 | `setState(RECORDING_PTT)` | `setState(RECORDING)` |
| startRecording() HOLD branch | KlarvoOverlayService ~line 1007 | `setState(RECORDING)` + adjustLayout | unchanged |
| startRecording() else branch | KlarvoOverlayService ~line 1012 | `setState(RECORDING_PTT)` | `setState(RECORDING)` |
| processAudio() start | KlarvoOverlayService ~line 1074 | `setState(PROCESSING)` | `setState(TRANSCRIBING)` |
| processAudio() all `setState(IDLE)` returns | KlarvoOverlayService ~1095–1443 | `setState(IDLE)` | unchanged |
| processAudio() success path | KlarvoOverlayService ~line 1370+ | `setState(IDLE)` | `setState(DONE)` + delay → `setState(IDLE)` |
| setState() dispatch | KlarvoOverlayService ~line 1451 | `RECORDING_PTT -> FloatingBubbleView.State.RECORDING_PTT` | remove |
| setState() dispatch | KlarvoOverlayService ~line 1451 | `PROCESSING -> FloatingBubbleView.State.PROCESSING` | `TRANSCRIBING -> FloatingBubbleView.State.TRANSCRIBING` |
| adjustLayoutForState() | KlarvoOverlayService ~line 705 | `else -> touchTargetPx` covers RECORDING_PTT | RECORDING, TRANSCRIBING, DONE all → touchTargetPx |
| FloatingBubbleView.State enum | FloatingBubbleView line 48 | `IDLE, RECORDING, RECORDING_PTT, PROCESSING` | `IDLE, RECORDING, TRANSCRIBING, DONE` |
| FloatingBubbleView.updateAnimators() | FloatingBubbleView ~line 201 | `RECORDING_PTT ->`, `PROCESSING ->` | merge PTT→RECORDING; PROCESSING→TRANSCRIBING; add DONE |
| FloatingBubbleView.onDraw() | FloatingBubbleView ~line 293 | `State.RECORDING_PTT ->`, `State.PROCESSING ->` | rename; add `State.DONE ->` |

**~13 total references to RECORDING_PTT** — most are in KlarvoOverlayService, not FloatingBubbleView.

### Cancellation + Error Paths (Important — Don't Miss)

`cancelRecording()` (~line 1047) currently checks `if (currentState != RecordingState.RECORDING && currentState != RecordingState.RECORDING_PTT) return`. After migration: `if (currentState != RecordingState.RECORDING) return`. Same for any `onSilenceTriggered()` guard.

`processAudio()` has multiple `setState(RecordingState.IDLE)` returns in error paths (STT failed, cleanup failed, network error, etc.) — these do NOT get the DONE flash, only the success path does. Leave all error `setState(IDLE)` calls as-is.

### DONE State — Delayed Transition Pattern

Use `handler.postDelayed()`, not `Thread.sleep()`, for the 800ms DONE → IDLE delay (we're on the main thread via `handler.post`). Pattern:
```kotlin
val prevStateForLayout = currentState
setState(RecordingState.DONE)
adjustLayoutForState(RecordingState.DONE, prevStateForLayout)
handler.postDelayed({
    val prev2 = currentState // should still be DONE
    setState(RecordingState.IDLE)
    adjustLayoutForState(RecordingState.IDLE, prev2)
}, 800L)
```
Do NOT call `processAudio()` cleanup steps after `setState(DONE)` — they've already run.

### Debug Broadcast — Architecture Constraint

The debug receiver is registered in `onCreate()` and unregistered in `onDestroy()`. It must NOT be registered in `onStartCommand()` (which runs multiple times per start-intent). The existing `notificationActionReceiver` pattern (lines 247–255) is the exact model to follow.

`BuildConfig.DEBUG` is available without explicit import in `KlarvoOverlayService.kt` because the Tauri-generated build already adds `buildFeatures { buildConfig = true }` to `app/build.gradle.kts` (patched by `scripts/android-build.sh` line 176–182). The `android-smoke.sh` builds a `universal/debug` APK → `BuildConfig.DEBUG = true`. The `android-build.sh` builds a `universal/release` APK → `BuildConfig.DEBUG = false`. This gate works without any extra configuration.

### Andi's Harness Commands (from WSL Terminal)

Device is reachable via Tailscale at `100.112.41.70`, pinned via `adb tcpip 5555` (Shortcut "Klarvo ADB Pin"). From WSL:

```sh
# Step 1 — Connect (run once per session if device not showing in adb devices)
adb connect 100.112.41.70:5555

# Step 2 — Verify service is running (tap a text field first to show the bubble)

# Drive each state:
adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state idle
adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording --ef rms 0.7 --es transcript "Dies ist ein Testtranskript."
adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state transcribing --ef rms 0.2 --es transcript "Dies ist ein Testtranskript."
adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state done
```

Document these exact commands in a code comment above the receiver registration in `KlarvoOverlayService.kt` so they're permanently accessible.

### What This Story Does NOT Do

- Does NOT implement the listening panel (Story 9.5)
- Does NOT implement the amber live-dot, timer, or raw-transcript area (Story 9.5)
- Does NOT implement visual state TRANSITIONS / spring animations (Story 9.5)
- Does NOT implement keyboard collapse (Story 9.6)
- Does NOT implement short-press gesture modes (Story 9.7)
- Does NOT implement long-press popover (Story 9.8)
- Does NOT touch Rust/Tauri/Desktop code
- Does NOT touch `KlarvoTheme.kt`, `KlarvoApi.kt`, or React sources
- Does NOT change `FloatingBubbleView.BAR_WIDTH_DP` or bar layout (bar still exists for HOLD mode in RECORDING state)
- Does NOT remove `pushToTalkActive` flag or change long-press PTT behavior (the tap handler changes are in the `when (currentState)` dispatch, not in PTT detection)

### Files to Modify

| File | Change |
|------|--------|
| `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` | Enum rename; updateAnimators + onDraw migration; DONE arm added |
| `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` | Enum rename; debug receiver; tap handler merge; silence guard; processAudio TRANSCRIBING+DONE; adjustLayoutForState |

No other files.

### Build Architecture (Same as 9.2 / 9.3)

`android/kotlin-src/` is the tracked source tree. `src-tauri/gen/android/` is gitignored generated output. `scripts/android-smoke.sh` syncs sources, builds DEBUG APK, installs on device. `scripts/android-build.sh` builds signed RELEASE APK.

`BuildConfig` is in the generated package (`com.klarvo.voice`) — already available; no import needed in a `package com.klarvo.voice` file.

### References

- [Source: epics-visual-overhaul.md, Story 9.4] — ACs, DoD, sequencing rationale
- [Source: epics-visual-overhaul.md, Story 9.5] — next story (what the harness enables)
- [Source: docs/adr/0018-android-bubble-rendering-tech.md, "Verifiability Symmetry"] — ADR sub-decision; harness design: broadcast receiver, state tokens, RMS/transcript extras, BuildConfig.DEBUG gating, example adb commands
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, ~line 114] — current `RecordingState` enum (baseline to migrate FROM)
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, ~line 216–254] — existing `notificationActionReceiver` pattern (model for debug receiver registration/unregistration)
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, ~line 1451–1465] — `setState()` dispatch (update required)
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, ~line 871–920] — tap handler `when (currentState)` branches (merge required)
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, line 48] — `State` enum (rename)
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, line 516–531] — `drawCheckMark()` helper (reuse for DONE placeholder)
- [Source: scripts/android-smoke.sh, line 175] — `universal/debug` APK path → `BuildConfig.DEBUG = true`
- [Source: scripts/android-build.sh, lines 176–182] — `buildFeatures { buildConfig = true }` patch (confirms BuildConfig is available)
- [Source: src-tauri/gen/android/app/src/main/java/com/klarvo/voice/generated/Logger.kt, line 86] — `BuildConfig.DEBUG` usage example (confirms it resolves in this package)
- [Source: _bmad-output/project-context.md] — minSdk 24, jni 0.21 pinned, no Compose, never `git add .`, Android changes require on-device smoke

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (implementation, 2026-06-15)

### Debug Log References

- adb install blocked by Xiaomi USER_RESTRICTED (known constraint, `reference_android_bubble_canvas_and_install.md`). Workaround: Andi muss auf dem Device "Unbekannte Apps erlauben" für diese Verbindung aktivieren. Task 4.3 + 4.4 sind Andi's on-device gate.
- RECORDING-State unified: Beide alten Branches (RECORDING Bar-Modus + RECORDING_PTT Kreis-Modus) werden jetzt im selben `RecordingState.RECORDING` geführt. Differenzierung im onDraw erfolgt via `width > height` (Bar-Modus hat WRAP_CONTENT = breites Fenster).
- adjustLayoutForState() wurde sorgfältig erweitert: die HOLD-tap-only Bar-Expansion via `!pushToTalkActive && tapMode==HOLD` vermeidet Bar-Expand beim Long-Press-PTT.

### Completion Notes List

- **Task 1 (Enum-Migration):** `FloatingBubbleView.State` + `RecordingState` migriert zu `IDLE/RECORDING/TRANSCRIBING/DONE`. Alle 13 Referenzen auf `RECORDING_PTT` und `PROCESSING` ersetzt. `grep` check: 0 Treffer.
- **Task 2 (Debug Receiver):** `ACTION_DEBUG_SET_STATE` Broadcast Receiver in `KlarvoOverlayService.kt` — constants im companion, `debugTranscript` Feld, `debugStateReceiver` Objekt, `BuildConfig.DEBUG`-gated `registerReceiver` in `onCreate()` + `unregisterReceiver` in `onDestroy()`. adb-Harness-Commands als Kommentar dokumentiert.
- **Task 3 (DONE State):** Teal-Squircle + weißer Checkmark in `FloatingBubbleView.onDraw()`. 800ms DONE-Flash in `processAudio()` success-path via `handler.postDelayed()`.
- **Task 4 (Verify):** JVM-Tests: 60/60 PASS (alle 6 Test-Klassen). Debug APK: kompiliert, 119 MB, 13s Build. grep-Gate: PASS. On-device install: blockiert durch Xiaomi USER_RESTRICTED — benötigt Andis manuelles Gate (Tasks 4.3/4.4 offen).
- **Task 5 (Commit):** Staged + committed (`feat(android): 9-4 bubble state harness — enum migration + debug broadcast`).

### File List

- `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` — State enum IDLE/RECORDING/TRANSCRIBING/DONE; updateAnimators() + onDraw() migriert; DONE arm (teal squircle + checkmark)
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — RecordingState enum; debug receiver; tap handler merge; silence guard; adjustLayoutForState; setState() dispatch; processAudio DONE flash

## Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-06-15 | Enum migration (RECORDING_PTT→RECORDING, PROCESSING→TRANSCRIBING, DONE new); debug broadcast receiver; DONE placeholder visual; 60 JVM tests PASS, Debug APK built. On-device smoke pending Andi gate. | claude-sonnet-4-6 |
