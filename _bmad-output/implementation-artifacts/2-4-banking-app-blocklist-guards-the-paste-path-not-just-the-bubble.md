# Story 2.4: Banking-App Blocklist Guards the Paste Path, Not Just the Bubble

Status: review

## Story

As an Android klarvo user,
I want the banking-app protection to actually stop the paste,
so that a pipeline that started before I switched to my banking app doesn't paste my dictation into it.

## Acceptance Criteria

1. **Given** `bankingAppActive` today gates ONLY bubble visibility (`KlarvoOverlayService.kt:466` and `471`) and the paste path (`handler.post` block at `~1190-1197`: `copyToClipboard` + `pasteIntoFocusedField`) has NO check,
   **When** a transcript is ready and `bankingAppActive` is true at the moment the paste lambda executes on the main thread,
   **Then** the paste path skips BOTH the clipboard write (`copyToClipboard`) and the accessibility paste (`pasteIntoFocusedField`).

2. **Given** a recording that STARTED before an app-switch into a banking app,
   **When** the pipeline completes (STT + optional LLM cleanup) while the banking app is still focused,
   **Then** nothing is written to the clipboard and nothing is pasted into it.

3. **Given** the user is NOT in a banking app (`bankingAppActive == false`),
   **When** a transcript is ready,
   **Then** paste proceeds normally — clipboard write and accessibility paste are unchanged (no regression).

4. **Given** paste is blocked by the banking guard,
   **When** the guard fires,
   **Then** the user receives a toast message informing them that the paste was blocked (not a silent no-op that looks like a pipeline failure).

## Tasks / Subtasks

- [x] Task 1: Add banking guard to the paste lambda in `KlarvoOverlayService.kt` (AC: 1, 2, 3, 4)
  - [x] 1.1 In `KlarvoOverlayService.kt`, inside the `handler.post { ... }` block that starts at line ~1190, add a `bankingAppActive` check at the TOP of the lambda body, BEFORE the `copyToClipboard(finalText)` call.
  - [x] 1.2 When the guard fires (`bankingAppActive == true`): show a toast — e.g., `showToast("Paste blocked — banking app active.")` — then `return@post` (skip clipboard write + accessibility paste). The `setState(RecordingState.IDLE)` and `adjustLayoutForState(...)` calls that follow the paste block should still execute so the UI returns to idle cleanly.
  - [x] 1.3 Apply the identical change to the build-target mirror: `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/KlarvoOverlayService.kt` — byte-identical diff.

- [x] Task 2: Write JVM unit tests in `BankingGuardTest.kt` (AC: 1, 2, 3, 4)
  - [x] 2.1 Create canonical test file at `android/kotlin-test/com/klarvo/voice/BankingGuardTest.kt` (AI-2 binding: see Dev Notes — this story requires a focused test approach)
  - [x] 2.2 Create byte-identical mirror at `src-tauri/gen/android/app/src/test/java/com/klarvo/voice/BankingGuardTest.kt`
  - [x] 2.3 Test: `bankingAppActive = false` → paste proceeds (AC-3 — positive regression path)
  - [x] 2.4 Test: `bankingAppActive = true` → paste blocked (AC-1 — the main finding)
  - [x] 2.5 Test: state transitions correctly to IDLE even when paste is blocked (AC-4 side effect — idle cleanup not skipped; NOTE: IDLE transition tested via shouldBlockPaste contract; toast + state machine covered by on-device smoke as documented in Dev Notes AI-2 binding)
  - [x] 2.6 Run `./gradlew :app:testUniversalDebugUnitTest` — all tests green, 0 failures

- [ ] Task 3: DoD smoke verification (AC: all) — **MANUAL, requires Android device**
  - [ ] 3.1 **AI-1 build-freshness gate:** Build via `scripts/android-build.sh` (produces a timestamped APK under `releases/v0.5.0/`). The freshly-built APK must be installed; freshness is proven by the build+install act + the script's timestamp gate. Cross-check via `adb shell dumpsys package com.klarvo.voice` (`lastUpdateTime`).
  - [ ] 3.2 **Normal dictation (positive path):** Normal utterance pasted normally into a non-banking app — AC-3 regression confirmed.
  - [ ] 3.3 **Banking guard smoke:** Start a recording, switch to a banking app (e.g., N26 or any app in the blocklist) before the pipeline completes OR while it is processing. Verify: (a) nothing is pasted into the banking app, (b) a toast appears indicating paste was blocked, (c) app returns to idle state correctly.

## Dev Notes

### What This Story Closes

**DIV-04** (`docs/robustness-audit-2026-05-30.md` §3, row 4, severity: high): `bankingAppActive` controls ONLY bubble visibility (`KlarvoOverlayService.kt:461/466`); the paste path has no check (`:1137-1141` in audit — lines ~1190-1197 in current HEAD after Story 2.3 changes). A pipeline that started before the user switched to a banking app continues and pastes into it — the "non-disableable protection" protects only bubble visibility, not the actual paste.

### Current Code State (HEAD after Stories 2.1–2.3)

The paste sequence in `KlarvoOverlayService.kt` lives inside a `handler.post { ... }` block:

```kotlin
// Line ~1190 (inside processResult() pipeline completion, after LLM cleanup + history save)
handler.post {
    copyToClipboard(finalText)                                   // ← no banking guard
    val pasted = KlarvoAccessibilityService.instance != null
    KlarvoAccessibilityService.instance?.pasteIntoFocusedField() // ← no banking guard
    val preview = if (finalText.length > 50) finalText.take(50) + "..." else finalText
    if (pasted) showToast("Inserted: $preview") else showToast("Copied: $preview")
    // ... feedback metrics, setState(IDLE), adjustLayoutForState, auto-send, AUTO loop
}
```

`bankingAppActive` is a field on `KlarvoOverlayService`:
```kotlin
private var bankingAppActive = false   // line 132
```

It is written only inside `handler.post { ... }` blocks (see `onBankingAppStateChanged` at line ~389), which means reads inside this `handler.post` paste block are also on the same main-looper thread — **no synchronization needed**. The read of `bankingAppActive` in the paste lambda is safe without locks.

The existing bubble guards are at:
- `showBubble()` line ~466: `if (bankingAppActive) return` — skips WindowManager addView
- `showBubble()` line ~471: `if (bankingAppActive) return@Runnable` — re-check after debounce delay

**The fix is a single `if (bankingAppActive) { ... return@post }` guard inside the paste `handler.post` block.** No new fields, no new classes, no threading change.

### Exact Fix Location

Inside `KlarvoOverlayService.kt`, the `handler.post` at line ~1190. Add at the TOP of the lambda, before `copyToClipboard`:

```kotlin
handler.post {
    // DIV-04 fix: abort paste if a banking app is focused at paste time.
    // The pipeline may have started before the app-switch; this guard ensures
    // nothing reaches the clipboard or accessibility paste path.
    if (bankingAppActive) {
        showToast("Paste blocked — banking app active.")
        setState(RecordingState.IDLE)
        adjustLayoutForState(RecordingState.IDLE, currentState)  // keep UI consistent
        return@post
    }

    copyToClipboard(finalText)
    // ... rest unchanged
```

**Important:** `setState(RecordingState.IDLE)` and `adjustLayoutForState(...)` MUST still run when the guard fires. They appear later in the original block (lines ~1215-1217). Moving them before `return@post` ensures the UI transitions back to idle even when the paste is skipped. Use `currentState` (the service field) since `prev = currentState` assignment hasn't happened yet — capture it before the early return, or just call `setState(RecordingState.IDLE)` directly (it handles the prev-state internally).

**Do NOT:**
- Move the guard OUTSIDE the `handler.post` block (thread-safety: `bankingAppActive` writes and reads must all be on main looper)
- Check only `copyToClipboard` but leave `pasteIntoFocusedField` unguarded — both must be blocked (AC-1: "skip BOTH")
- Skip the toast — AC-4 requires user feedback
- Leave the state in a non-IDLE condition after the guard fires — user would need to restart to resume

### AI-2 Binding (Epic 1 Retro) — Test Approach for Service Logic

**Challenge:** `KlarvoOverlayService` is an Android `Service` and cannot be instantiated in a JVM unit test (it requires a running Android context, `WindowManager`, `Handler` backed by `Looper`, etc.). The existing test pattern (Story 2.1 `HallucinationFilter`, Story 2.2 `SilencePreFilter`, Story 2.3 `KlarvoApi.sanitizeLlmOutput`) tests standalone pure Kotlin objects — not Android service internals.

**This story's fix is in the service body, not a standalone helper.** The AI-2 mandate ("bind tests to real production call sites, not inline pattern copies") applies, but the vehicle is different:

- **Preferred approach:** Extract the paste-decision logic into a small, testable pure function. For example:
  ```kotlin
  // In KlarvoOverlayService (or BankingGuard.kt companion)
  internal fun shouldBlockPaste(bankingActive: Boolean): Boolean = bankingActive
  ```
  Then test `BankingGuard.shouldBlockPaste(true)` and `shouldBlockPaste(false)` directly — binding to the real decision function, not a copy.

- **Minimal approach (acceptable):** If extraction feels like over-engineering for a one-liner check, the test can assert the publicly observable contract (e.g., the decision function result) and document why the integration behavior is covered by the on-device smoke. In this case, make the decision function `internal` so the test package can reach it.

- **Do NOT** write a test that duplicates the `if (bankingAppActive)` logic inline in the test body — that is exactly the "testing a re-declared copy" anti-pattern AI-2 forbids.

The test in `BankingGuardTest.kt` should call the real extraction point (whether it's a standalone function or an `internal` helper on the service). The on-device smoke (Task 3) covers the full integration path.

### Files to Modify (No New Files Beyond Tests)

| File | Change |
|---|---|
| `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` | Add `bankingAppActive` guard at top of paste `handler.post` lambda; call `setState`+`adjustLayoutForState` + `return@post` when blocked; show toast |
| `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/KlarvoOverlayService.kt` | byte-identical build-target mirror |

### Files to CREATE

| File | Purpose |
|---|---|
| `android/kotlin-test/com/klarvo/voice/BankingGuardTest.kt` | canonical tracked test (AI-2 binding) |
| `src-tauri/gen/android/app/src/test/java/com/klarvo/voice/BankingGuardTest.kt` | gitignored build-target mirror |

### Sync Note (Source-of-Truth Dual Edit)

`android/kotlin-src/` is the canonical source. `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/` is the build target (gitignored). Both must receive identical edits. **Pattern established by Stories 2.1–2.3 — never edit only one side.**

### Test Infrastructure (Established by Stories 2.1–2.3)

- Canonical test dir: `android/kotlin-test/com/klarvo/voice/` (tracked in git)
- Build-target test dir: `src-tauri/gen/android/app/src/test/java/com/klarvo/voice/` (gitignored)
- Gradle test task: `./gradlew :app:testUniversalDebugUnitTest` (NOT `testDebugUnitTest` — two ABI variants)
- JUnit dependency already in `app/build.gradle.kts`: `testImplementation("junit:junit:4.13.2")`
- Existing tests: `HallucinationFilterTest.kt`, `SilencePreFilterTest.kt`, `SanitizePathsTest.kt` — use same package declaration and structure

### DoD Requirements (Surface-Class — NFR-Smoke Mandatory)

- **Build freshness (AI-1):** Fresh APK built and installed via `scripts/android-build.sh`. Stale artifact = invalid smoke (Story 1.2 trap, Epic 1 retro AI-1). Verify via `adb shell dumpsys package com.klarvo.voice` (`lastUpdateTime`).
- **Positive-path regression:** Normal dictation → paste proceeds normally in a non-banking app (AC-3).
- **Banking guard smoke:** Paste blocked with toast when banking app is active at pipeline completion (AC-4). The trigger scenario (recording starts before app-switch) is the primary attack vector — test it.
- **JVM tests green:** `./gradlew :app:testUniversalDebugUnitTest` passes all `BankingGuardTest` tests alongside the existing 54 tests.

**Note on in-person smoke trigger:** The banking guard fires on the CURRENTLY focused app at paste time (inside `handler.post`), not at recording start. The real DIV-04 scenario is: start dictating in app A, switch to banking app before the pipeline finishes, observe blocked paste. The smoke verifier can also test the simpler case: start a recording while the banking app is already open (if the bubble doesn't show, confirm the paste is also blocked when dictation completes).

### Thread-Safety Reminder

`bankingAppActive` is a plain `var` field (no `volatile`, no `AtomicBoolean`). It is written in `onBankingAppStateChanged` via `handler.post { ... }` (main looper). The paste lambda is ALSO on the main looper (`handler.post`). Android's single-threaded main looper serializes all `handler.post` callbacks — the read in the paste lambda is already safe. Do NOT add `@Volatile` or `AtomicBoolean` wrapping; that would be over-engineering and inconsistent with the existing usage at lines 466/471.

### Rust Parity Reference

The Rust desktop pipeline does not have an equivalent banking-app guard — this is an Android-only feature (`bankingAppActive` has no desktop counterpart). ADR-0016 explicitly closes DIV-06..14 as "accepted asymmetry." This fix is NOT cross-platform; it is Android-only by design.

### References

- Audit finding: `docs/robustness-audit-2026-05-30.md` §3 DIV-04 (high)
- Gate ADR: `docs/adr/0016-android-path-parity-strategy.md`
- Banking guard (bubble side): `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` lines 132, 389-407, 466, 471
- Paste path (unguarded): `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` lines ~1190-1197
- Handler is main looper: `KlarvoOverlayService.kt:105` (`Handler(Looper.getMainLooper())`)
- `onBankingAppStateChanged`: `KlarvoOverlayService.kt:389-407`
- Epics spec: `_bmad-output/planning-artifacts/epics.md` — Epic 2, Story 2.4
- Epic 1 retro AI-1/AI-2: `_bmad-output/implementation-artifacts/epic-1-retro-2026-06-01.md` §7
- Precedent — object pattern: `android/kotlin-src/com/klarvo/voice/HallucinationFilter.kt` (Story 2.1)
- Precedent — dual-edit sync: Stories 2.1, 2.2, 2.3 (canonical `android/kotlin-src/` + gen mirror)
- Build freshness gate: `scripts/android-build.sh`

## Review Findings

_Code review 2026-06-01 (3 adversarial layers: Blind Hunter / Edge Case Hunter / Acceptance Auditor, Opus 4.8). Verdict: **PASS** — all 4 ACs satisfied, gen-mirror byte-identical, AI-2 binding correct, 58 JVM tests green. 1 parity patch recommended, 2 deferred, rest dismissed as refuted/accepted._

### Patch

- [x] [Review][Patch] Banking-block path does not reset `autoLoopActive` — parity gap. Every other terminal path (937/952/963/986/1025/1043/1063/1083/1095/1265) sets `autoLoopActive = false`; the new block (1194-1200) is the sole exception. The AUTO loop DOES halt (the `return@post` precedes the restart at :1248), so no functional break — but the flag is left stale until the next IDLE/PROCESSING tap. Add `autoLoopActive = false` inside the guard for consistency with all siblings. [KlarvoOverlayService.kt:1194-1200]

### Deferred (pre-existing, out of scope — logged to deferred-work.md)

- [x] [Review][Defer] Pending auto-send Enter not cancelled on block — `handler.postDelayed({ performEnter() }, 150)` from a prior AUTO segment is not cancelled by the guard; its Enter keystroke can land in the focused app. Narrow (~150ms window) and pre-existing to auto-send, orthogonal to the paste-path guard. [KlarvoOverlayService.kt:1238-1240]
- [x] [Review][Defer] Banking-app detection latency / defaults-open — the guard acts correctly on `bankingAppActive`, but the upstream detection that sets it can lag the real foreground switch; the guard cannot close that gap. Upstream/pre-existing (same field drives the existing bubble guard). [KlarvoOverlayService.kt:389-407]

### Dismissed (refuted / accepted)

- **"Data loss when blocked"** (Blind, High) — REFUTED: `KlarvoApi.saveToHistory` runs at :1155, before the paste block; the transcript is persisted, not lost. Not pasting into the banking app is the intended behavior.
- **In-lambda TOCTOU / unsynchronized `bankingAppActive` / force-reset-to-IDLE / layout double-apply / `internal` visibility** (Blind, High×3 + Med×2) — REFUTED by the code-reading layers + verified: the write site at :393 is also `handler.post` (main looper), the block mirrors the sibling pattern at 1226-1228, 58 JVM tests compile & pass, gen-mirror byte-identical.
- **Hardcoded English toast / i18n** (Blind, Med) — matches the existing Android toast convention ("Inserted:", "Copied:"); the Android shell has no locale layer (Rust desktop only).
- **`shouldBlockPaste` pure pass-through + idempotency tests** (Blind, Med/Low) — ACCEPTED tradeoff: the AI-2-sanctioned extraction; real integration coverage is the on-device smoke (Task 3). Discussed and approved at GATE 1.
- **Orphaned delegation comment** at ~:1379 — harmless documentation.

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

_No blocking issues encountered._

### Completion Notes List

- **Task 1:** Extracted `BankingGuard` as a standalone `internal object` in new file `BankingGuard.kt` (AI-2 pattern — testable without Android context). The `handler.post` paste lambda now calls `BankingGuard.shouldBlockPaste(bankingAppActive)` at the TOP, before `copyToClipboard`. When guard fires: toast shown, `setState(IDLE)` + `adjustLayoutForState` called, then `return@post` — both clipboard write and accessibility paste are skipped. `setState`/`adjustLayoutForState` captured via `val prev = currentState` before early return to keep UI consistent.
- **Task 1.3:** `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/KlarvoOverlayService.kt` and `BankingGuard.kt` both byte-identical to canonical sources (verified via `diff`).
- **Task 2:** `BankingGuardTest.kt` with 4 tests calls `BankingGuard.shouldBlockPaste()` directly (AI-2 binding). No Android context needed. The AC-4 IDLE-transition + toast side effect requires Android runtime and is covered by on-device smoke (Task 3) as documented in Dev Notes.
- **Task 2.6:** `./gradlew :app:testUniversalDebugUnitTest` — **58 tests total, 0 failures, 0 errors** (BankingGuardTest: 4, HallucinationFilterTest: 24, SilencePreFilterTest: 18, SanitizePathsTest: 12).
- **Task 3:** MANUAL on-device smoke — not yet performed. Story is in `review` status pending on-device verification (surface-class hard-gate per Epic-1-Retro AI-1).
- **Review Patch applied 2026-06-01:** Added `autoLoopActive = false` inside banking guard block in `handler.post` paste lambda — parity with all other terminal paths (937/952/963/986/1025/1043/1063/1083/1095/1265). Both canonical and gen-mirror updated, byte-identical. 58 JVM tests, 0 failures.

### File List

- `android/kotlin-src/com/klarvo/voice/BankingGuard.kt` (new — internal object, shouldBlockPaste decision function)
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` (modified — DIV-04 guard in paste handler.post lambda)
- `android/kotlin-test/com/klarvo/voice/BankingGuardTest.kt` (new — 4 JVM unit tests, AI-2 binding)
- `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/BankingGuard.kt` (new — byte-identical mirror)
- `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/KlarvoOverlayService.kt` (modified — byte-identical mirror)
- `src-tauri/gen/android/app/src/test/java/com/klarvo/voice/BankingGuardTest.kt` (new — byte-identical mirror)
- `_bmad-output/implementation-artifacts/2-4-banking-app-blocklist-guards-the-paste-path-not-just-the-bubble.md` (story file — status + tasks updated)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (story 2-4 set to review)

## Change Log

- 2026-06-01: DIV-04 fix — extracted `BankingGuard` object, added paste-path guard to `handler.post` lambda in `KlarvoOverlayService`, wrote 4 JVM unit tests; 58 JVM tests green (0 failures). On-device smoke (Task 3) remains for Andi to perform before marking done.
- 2026-06-01: Review patch — added `autoLoopActive = false` to banking guard block for parity with all other terminal paths; both canonical + gen-mirror updated, byte-identical; 58 JVM tests green.
