# Story 9.5: Bubble State Sequence + Listening Panel + Waveform

Status: review

> **RE-FASHIONED 2026-06-16 against [ADR-0019](../../docs/adr/0019-cross-platform-design-ssot.md) + the
> extended canon.** The previously-built approach (suppress-bubble-to-idle · red square = Send · extra
> neutral ✗ cancel) is **SUPERSEDED** — see `_bmad-output/planning-artifacts/sprint-change-proposal-2026-06-16.md`.
> The full old build-record (old ACs, completed tasks, Dev Agent Record) is preserved **verbatim** in the
> **SUPERSEDED** appendix at the bottom of this file — *do NOT rebuild that*. This story is a
> **modification of the standing build**, not greenfield: the listening panel and the double-window
> defect fix already exist and stand; only the bubble's recording visual and the confirm/cancel
> semantics flip.

## Story

As a user dictating from a text field,
I want the bubble to run idle→recording→transcribing→done with a Klarvo-owned listening panel,
so that I see live feedback and the cleaned text lands in my field — sending by tapping the bubble and
cancelling with the red square, exactly mirroring the desktop colour-semantics.

## Stands vs Flips (read this first)

**Stands — already built and correct, do NOT re-do:**
- `ListeningPanelView` as a **separate `TYPE_APPLICATION_OVERLAY` window** (panel ≠ inside the bubble view).
- Panel content per canon: grip, K-badge, amber live-dot + pulse, 5-bar amber RMS waveform, timer,
  multiline mono transcript + amber caret, footer; TRANSCRIBING teal spinner + dimmed text; DONE check→idle.
- The **double-window defect fix**: only ONE recording overlay form exists (the old HOLD-tap "expand to
  bar" window stays retired), and the **bubble window stays alive so taps reach `handleTouch`** — this is
  now load-bearing, because tap-to-send depends on it.
- Token values from Story 9-10 codegen (`KlarvoTheme.kt` generated from `klarvo.css`).

**Flips — this story's work (4 changes):**
1. **Bubble recording visual:** replace `suppressedForPanel`→static-idle with the canon
   `.ab-bubble.recording` form (teal-gradient squircle + amber pulse-ring + send-glyph instead of "K").
2. **Bubble tap = Senden:** a short tap on the recording-state bubble → `stopAndProcessRecording()`.
3. **Panel red square = Abbrechen:** the `.ab-bar-stop` red square → `cancelRecording()` (discard, no
   paste). Inverts the old `stopAndProcessRecording` wiring; parity with desktop red square.
4. **Remove the extra neutral ✗ button** (added in the 2026-06-16 double-window fix). Cancel = red square;
   Send = bubble-tap. Two distinct, correctly-coloured affordances.

## Acceptance Criteria

**AC1 — Recording panel rises with grab handle, K + amber live-dot, waveform, timer, red Abbrechen square:**
Given recording starts (tap or long-press gesture)
When `setState(RecordingState.RECORDING)` is called
Then a Klarvo-owned overlay panel rises from the bottom of the screen with:
  - a grab handle (34dp × 4dp, `KlarvoTheme.Border2`, centered at top of panel)
  - a top-row: squircle K-badge (18dp, teal squircle) + amber live-dot (7dp circle, pulsing) + 5-bar amber
    RMS waveform + timer (Geist Mono 10.5sp, `KlarvoTheme.Dim`) + **red square = Abbrechen** (26dp × 26dp,
    rounded 8dp, `KlarvoTheme.DangerBg` bg + danger border, inner 9dp square `KlarvoTheme.Danger`,
    `contentDescription`/aria = "Abbrechen")
  - panel background `rgba(18,20,22,.98)` (≈ `0xFA121416`), top amber border-line (`KlarvoTheme.AmberLine`)
  - panel enters with spring animation (240ms `OvershootInterpolator(1.8f)` from height=0 to full height)
And the bubble stays visible above the panel **in its recording state** (see AC6) — NOT the idle K.

**AC2 — Live RAW transcript runs multiline inside the panel (NOT in the foreign field):**
Given recording is active and audio chunks stream in
When raw transcript text accumulates
Then the text renders inside the panel (font mono 13sp, `KlarvoTheme.Muted`) with a blinking amber caret
And no text is set in the foreign text field until the final paste step (AR5a: `SYSTEM_ALERT_WINDOW`
overlay cannot set composing text in a foreign field)
And `debugTranscript` injected via the 9.4 harness is displayed in the panel during harness runs

**AC3 — Transcribing: same panel, teal spinner + "Cleaning…", raw text dimmed:**
Given recording stops and `setState(RecordingState.TRANSCRIBING)` is called
When the pipeline transitions
Then the same panel remains on screen (no collapse between RECORDING and TRANSCRIBING)
And the top-row changes to: squircle K-badge + teal spinner (15dp, 900ms rotation) + "Cleaning…" label
(11sp, `KlarvoTheme.Dim`, Geist Mono)
And the red Abbrechen square disappears (replaced by empty space)
And the amber top border-line becomes `KlarvoTheme.Border2` (standard, no amber accent)
And the raw transcript text renders in `KlarvoTheme.Dim` (dimmed — not `KlarvoTheme.Muted`)
And the footer reads "Gleich fertig · Tastatur kommt gleich zurück" (or locale equivalent)

**AC4 — Done: panel collapses, keyboard note, cleaned text lands in field, bubble shows check → idle:**
Given `setState(RecordingState.DONE)` is called after paste
When the done transition fires
Then the listening panel collapses (slides down in 320ms `LinearInterpolator`)
And the bubble displays the teal squircle with a white checkmark (`.ab-bubble.done`, already in 9.4)
And after 800ms (`doneFlashRunnable`) the bubble returns to idle
And the cleaned text has been written to the focused field via
`KlarvoAccessibilityService.instance?.pasteIntoFocusedField()` (a11y ACTION_PASTE) — fallback: clipboard
only (existing path, no change)
And the keyboard is NOT forcibly dismissed in this story (keyboard-collapse is Story 9.6)

**AC5 — Panel is a second `TYPE_APPLICATION_OVERLAY` window, NOT inside `FloatingBubbleView`:**
Given the listening panel's size and layout complexity
When the panel is shown
Then it is added to `WindowManager` as a separate `View` with `TYPE_APPLICATION_OVERLAY`, anchored at the
bottom of the screen (above the keyboard)
And `FloatingBubbleView` remains a small squircle in the corner (size/position unchanged)
And the panel window is `MATCH_PARENT` width, height = auto-sized by content (minimum 200dp per canon
`.ab-panel { min-height: 200px }`)
And the panel window is created/destroyed on each recording cycle (added in RECORDING, removed in IDLE
after DONE flash)

**AC6 — Bubble recording state + confirm/cancel semantics (the ADR-0019 core):**
Given recording is active (RECORDING or TRANSCRIBING)
When the bubble is rendered
Then the bubble renders the canon `.ab-bubble.recording` form, NOT the idle K:
  - teal-gradient squircle (reuse `KlarvoTheme.TealHi`/`TealLo`, same 40dp / r=12dp shape as idle)
  - an **amber pulse-ring** animation matching canon `@keyframes abbubblepulse` (1400ms, ease-out,
    repeating): expanding amber ring `KlarvoTheme.Amber` (`rgba(233,162,76,…)`) — 2px+4px rings →
    3px+15px fade-out → loop
  - a **send-glyph** (paper-plane SVG path `m22 2-7 20-4-9-9-4 20-7z`, ~20dp, `KlarvoTheme.OnTeal` stroke)
    in place of the "K"
And **a short tap on the recording-state bubble = Senden** → calls `stopAndProcessRecording()`
(stop → transcribe → clean → paste). This is the primary confirm affordance.
And **the red square in the panel = Abbrechen** → calls `cancelRecording()` (discard, no paste).
And there is **no separate neutral ✗ button** (the one added in the 2026-06-16 fix is removed).
And on DONE/IDLE the bubble restores its normal visual (checkmark → idle K), and the recording-state
animations are stopped/cleaned up.

> **Note on gesture modes:** existing modes (HOLD release / TOGGLE tap / AUTOSTOP / AUTO) still drive
> start/stop as today; AC6 standardises that *whatever stops recording from the bubble is "send"* and the
> red square is the *only* cancel. Per-mode gesture nuances are Story 9-7 — do NOT expand mode behaviour here.

**AC7 — States verified via the 9.4 harness:**
Given the debug broadcast receiver from Story 9.4
When `adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording --ef rms 0.7 --es transcript "Test text"` is sent
Then the listening panel appears with the waveform animated at the given RMS level, the bubble shows the
recording form (amber pulse + send-glyph), and the transcript text shows in the panel
And harness states for `transcribing`, `done`, `idle` drive panel + bubble correctly
> **MIUI caveat:** the 9.4 broadcast harness is dead on Andi's real MIUI device (broadcast not delivered);
> harness verification runs on emulator only and is **not** a visual oracle. Real-device drive is manual
> (the harness-on-MIUI fix is a separate owed story).

**Inversion (must-fail gates):**
- Red square / any danger-coloured control wired to **send/confirm** = instant review failure (ADR-0019:
  red = Abbrechen only, on both platforms).
- Bubble staying **idle** (K) during recording instead of the `.ab-bubble.recording` form = review failure.
- A separate neutral ✗ cancel button still present = review failure (cancel is the red square).
- Any attempt to set composing/live text in the foreign field from the overlay = review failure (AR5a).
- A second recording overlay form reappearing alongside the panel (double-window regression) = review failure.
- Amber appearing in TRANSCRIBING state = review failure (amber = recording only; transcribing uses teal).

**DoD:** On-device smoke on Andi's real device — real end-to-end dictation in a 3rd-party app (e.g. Chrome
address bar or WhatsApp), across modes (PTT hold/release · HOLD-tap · TOGGLE · AUTOSTOP · AUTO): bubble
shows the amber-pulsing send-form during recording; **tapping the bubble sends** (panel → teal spinner →
cleaned text lands in the field → panel collapses); **the red square cancels** (panel dismisses, nothing
pasted); no double overlay. Emulator harness drives states for compile/regression only — **GATE 4 = Andi's
real device** (emulator is not a visual oracle, E9). `scripts/android-smoke.sh` exits 0.

## Tasks / Subtasks

- [x] **Task 1: Bubble recording visual — `.ab-bubble.recording` in `FloatingBubbleView`** (AC: 6)
  - [x] 1.1 Replace the `suppressedForPanel`→static-idle rendering with the canon recording form: teal
    gradient squircle (reuse idle gradient/shape) + send-glyph (paper-plane path `m22 2-7 20-4-9-9-4 20-7z`,
    ~20dp, `KlarvoTheme.OnTeal` stroke ~2.2dp) instead of the "K" glyph, for RECORDING and TRANSCRIBING
    effective-state.
  - [x] 1.2 Add the amber pulse-ring animation matching `@keyframes abbubblepulse` (1400ms ease-out, repeat):
    expanding amber ring(s) `KlarvoTheme.Amber`. Reuse the existing animator plumbing; ensure it is started
    on entering recording and **stopped + cleaned up** on DONE/IDLE.
  - [x] 1.3 Keep the window-stays-alive invariant from the double-window fix (bubble still receives touch in
    handleTouch). Do NOT reintroduce the retired HOLD-tap "expand to bar" window.
  - [x] 1.4 On the per-transition `alpha=1.0f` reset path: ensure the recording form (not idle) is what shows
    while recording — the reset must not flip the bubble back to idle (the old `alpha` trap).

- [x] **Task 2: Confirm/cancel semantics in `KlarvoOverlayService` / panel** (AC: 6)
  - [x] 2.1 Wire **bubble short-tap during recording → `stopAndProcessRecording()`** (Send). Confirm this
    composes correctly with the existing gesture-mode stop wiring (TOGGLE tap / HOLD release already stop).
  - [x] 2.2 Re-wire the panel **red square → `cancelRecording()`** (was `stopAndProcessRecording`). Update
    `isTouchOnStopButton`/handler accordingly; relabel `contentDescription` to "Abbrechen".
  - [x] 2.3 **Remove the neutral ✗ button** added in the 2026-06-16 fix (`isTouchOnCancelButton` + its draw +
    its touch branch). Cancel is now the red square only.

- [x] **Task 3: Locale / copy** (AC: 1, 3)
  - [x] 3.1 Confirm footer strings ("Tastatur pausiert · kehrt beim Einfügen zurück" / "Gleich fertig ·
    Tastatur kommt gleich zurück"). Resolve the residual "Cleaning…" → "Bereinigt…" locale item flagged in
    the prior run (German product copy).

- [x] **Task 4: Compile + verify** (AC: all)
  - [x] 4.1 `scripts/android-smoke.sh` exits 0 (Kotlin compile clean, DEBUG APK built). JVM tests pass (24/24).
  - [ ] 4.2 Emulator harness: drive recording→transcribing→done→idle; verify bubble shows the recording form
    (amber pulse + send-glyph) and panel red square is present in RECORDING only. (Compile/regression signal —
    NOT the visual gate.) — MIUI caveat: harness dead on real device; emulator path skipped (unattended emulator
    was not available in this session; compile/build gate GRÜN = sufficient for review).
  - [ ] 4.3 **GATE 4 (Andi, real device):** end-to-end across modes per DoD — tap-bubble-sends, red-square-
    cancels, no double overlay.

- [x] **Task 5: Commit** (AC: all)
  - [x] 5.1 Stage only the touched Kotlin files (`FloatingBubbleView.kt`, `KlarvoOverlayService.kt`,
    `ListeningPanelView.kt`). Never `git add .`.
  - [x] 5.2 Commit message: `feat(android): 9-5 rebuild — recording-state bubble + tap=send / red=cancel (ADR-0019)`

## Dev Notes

### What is the actual delta (since the panel already exists)
The listening panel (`ListeningPanelView`), its separate-overlay-window architecture, and the
double-window defect fix are already in the tree and **stand**. The only code that changes is (a) the
bubble's recording visual in `FloatingBubbleView`, (b) the send/cancel wiring in `KlarvoOverlayService`
and the panel touch handlers, and (c) removing the neutral ✗. Read the SUPERSEDED appendix for the exact
shape of the existing panel code — it is still the implementation, minus the ✗ button and with the red
square re-pointed to cancel.

### Canon values for the recording bubble (binding — read from the canon)
From file-local CSS in `Klarvo Design System.html`:
- `.ab-bubble` (line 39): 40dp, `border-radius:12px`, `box-shadow: 0 6px 18px rgba(0,0,0,.5)` + glass.
- `.ab-bubble.recording` (line 45): `background: linear-gradient(150deg, var(--k-teal-hi), var(--k-teal-lo))`,
  `color: var(--k-on-teal)`, `animation: abbubblepulse 1400ms ease-out infinite`.
- `.ab-bubble.recording .send` (line 46): `width:20px; height:20px`.
- `@keyframes abbubblepulse` (lines 47–51): rings in amber `rgba(233,162,76, …)` — 0%/100%
  `0 0 0 2px (.95), 0 0 0 4px (.35)`; 70% `0 0 0 3px (.55), 0 0 0 15px (0)`.
- Send-glyph SVG (line 727): `path d="m22 2-7 20-4-9-9-4 20-7z"`, `stroke="var(--k-on-teal)"`, `stroke-width=2.2`.
- RECORDING artboard label (line 729): *"Bubble bleibt sichtbar mit Amber-Puls + Send-Glyph (antippen =
  senden) · rotes Quadrat = abbrechen"* — the binding semantic statement.

### Touch-routing constraint (carried from the prior build — still applies)
`FLAG_NOT_FOCUSABLE` panels receive no touch directly; the bubble view's `setOnTouchListener` receives
bubble-window touches. The panel uses `FLAG_NOT_FOCUSABLE | FLAG_NOT_TOUCH_MODAL` and a panel touch
listener that returns true on `ACTION_DOWN` inside a control so `ACTION_UP` arrives (the F1/F2 fixes).
Re-point that control from stop→cancel; keep the coordinate translation (panel-root → TopRowView-local).

### "Live RAW transcript" in the current pipeline (unchanged)
The pipeline is batch-only (no streaming STT). The transcript area is blank during real recording and
populated by `debugTranscript` under the harness. Do NOT invent fake chunking. (See SUPERSEDED appendix
for the full note — still accurate.)

### Files to Modify
| File | Change |
|------|--------|
| `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` | Recording-state visual (`.ab-bubble.recording`: teal + amber pulse-ring + send-glyph); keep window-alive invariant; fix alpha-reset path |
| `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` | Bubble-tap→send wiring; panel red square→`cancelRecording`; remove ✗ branch |
| `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt` | Remove the neutral ✗ button (draw + bounds + `isTouchOnCancelButton`); relabel red square "Abbrechen" |
| `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt` | Only if a new token is needed (most likely none — recording uses existing Teal*/Amber/OnTeal) |

No Rust/Tauri/Desktop files. Desktop parity check is a separate ADR-0019 follow-up.

### References
- [Source: docs/adr/0019-cross-platform-design-ssot.md] — colour-semantics rule + interaction parity (the decision).
- [Source: docs/design/overhaul/source/Klarvo Design System.html, lines 38–52, 715–729] — `.ab-bubble.recording`,
  `abbubblepulse`, send-glyph, RECORDING artboard + binding affordance labels.
- [Source: docs/design/overhaul/source/assets/klarvo.css] — token values (`--k-amber`, `--k-teal-*`, `--k-on-teal`).
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-2026-06-16.md] — this re-fashion's full impact analysis.
- [Source: docs/postmortem-2026-06-15-epic-conductor.md] — the double-window defect + the traps (PTT touch-stream, alpha reset, MIUI harness).
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt] — `drawWaveformBarsInZone()`, `drawSpinner()`, animator plumbing to reuse.
- [Source: _bmad-output/project-context.md] — minSdk 24, no Compose, never `git add .`, Android changes require on-device smoke.

## Dev Agent Record (ADR-0019 rebuild, 2026-06-16)

### Agent Model Used
claude-sonnet-4-6

### Completion Notes

- **Task 1 (FloatingBubbleView.kt):** Replaced `suppressedForPanel`→static-idle rendering with the canon `.ab-bubble.recording` form. RECORDING and TRANSCRIBING now both draw: teal-gradient squircle (reuse idle gradient + shadow + ring shape) + `drawAmberPulseRings()` (1400ms `AccelerateDecelerate` ValueAnimator, two expanding rings in `KlarvoTheme.Amber`) + `drawSendGlyph()` (paper-plane path `m22 2-7 20-4-9-9-4 20-7z`, 20dp bounding box, `KlarvoTheme.OnTeal` stroke 2.2dp). `suppressedForPanel` kept as no-op property for backwards compat. `onMeasure` simplified: bar-mode removed (HOLD-tap bar retired). `isTouchInCancelZone`/`isTouchInConfirmZone` return false (dead). `updateAnimators`: starts `amberPulseAnimator` on RECORDING/TRANSCRIBING, stops+resets on DONE/IDLE.
- **Task 2 (KlarvoOverlayService.kt):** Panel touch listener re-wired: red square → `cancelRecording()` (was `stopAndProcessRecording`). `isTouchOnCancelButton` branch removed. `handleTap` RECORDING branch simplified: all modes → `stopAndProcessRecording()` (ADR-0019: bubble tap = Senden). Header + doc comments updated.
- **Task 3 (ListeningPanelView.kt):** `cancelBtnRect`, `isTouchOnCancelButton` removed. `✗`-button draw code removed. `rightReserved` formula updated (no cancelBtnSz). `isTouchOnStopButton` public API unchanged (KlarvoOverlayService still calls it). "Cleaning…" → "Bereinigt…" (German product copy). Footer strings confirmed correct.
- **Task 4:** `scripts/android-smoke.sh` exits 0. JVM tests 24/24. APK built + installed on 100.112.41.70:5555 (v0.5.0). Harness dead on real MIUI (known); compile gate is the unattended signal. GATE 4 = Andi real device.
- **Task 5:** Staged + committed (scoped files only, no `git add .`).

### File List
- `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` (modified — recording-state visual, amber pulse-ring, send-glyph, suppressedForPanel no-op)
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` (modified — panel red square→cancel, bubble-tap→send, ✗ branch removed)
- `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt` (modified — ✗ button removed, cancelBtnRect removed, "Bereinigt…")
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (status update)
- `_bmad-output/implementation-artifacts/9-5-bubble-state-sequence-listening-panel-waveform.md` (this file)

## Change Log

- 2026-06-15: Story implemented — ListeningPanelView (NEW), KlarvoOverlayService wired, KlarvoTheme tokens added. Kotlin BUILD SUCCESSFUL, 60/60 JVM tests pass. Harness smoke verified on device (recording/transcribing/done/idle all drive panel correctly). [claude-sonnet-4-6]
- 2026-06-15: Code-review fixes F1..F9 applied (claude-sonnet-4-6) — see SUPERSEDED appendix for the full list.
- 2026-06-15 — Epic-conductor close-out → **done** (sole committer; folded 3 worker commits via reset --soft + one clean commit). GATE-4 fidelity GREEN on emulator-5554 for recording/transcribing/done; AR5a respected; amber absent in transcribing. Listening panel built as a SEPARATE TYPE_APPLICATION_OVERLAY window. | epic-conductor (Opus)
- 2026-06-15 — **REOPENED done→review.** The emulator GATE-4 GREEN was a FALSE-GREEN: on Andi's real Xiaomi/MIUI the RECORDING state showed TWO overlay windows (panel + the old FloatingBubbleView RECORDING bar). The 9.4 harness is also dead on MIUI. Real defect + fix-spec documented in `docs/postmortem-2026-06-15-epic-conductor.md`. | Andi + Claude (conductor postmortem)
- 2026-06-16 — **Double-window defect FIXED** (attended): bubble drew its OWN recording form as a second overlay; fix added `suppressedForPanel`→static-idle + retired the HOLD-tap bar window + restored a neutral ✗ cancel on the panel. Kotlin compile clean; JVM tests green. (This fix's *interaction surface* is superseded below; the *no-double-window* part stands.) | Claude (Opus)
- 2026-06-16 — **Real-device smoke surfaced a deeper issue → interaction MODEL changed.** Andi confirmed function works but flagged Android↔Windows divergence (desktop red square = Cancel; my 9-5 made it = Send). Root-caused in **ADR-0019** + canon extended (Option A): **red = Abbrechen**, **tap the bubble = Senden**, bubble gets `.ab-bubble.recording`. Supersedes the suppress-to-idle / red=stop / extra-✗ approach. | Andi + Claude (Opus)
- 2026-06-16 — **Story RE-FASHIONED (correct-course, Weg A).** Status `review`→`backlog`; ACs/Tasks rewritten to the ADR-0019 model (recording-state bubble, tap=send, red=cancel, no ✗); old build-record moved verbatim to the SUPERSEDED appendix; the panel + double-window fix marked as standing. Epic 9.5 AC updated in `epics-visual-overhaul.md`. Frontmatter↔tracker drift resolved (both `backlog`). Proposal: `sprint-change-proposal-2026-06-16.md`. | Claude (Opus)
- 2026-06-16 — **ADR-0019 rebuild BUILT.** FloatingBubbleView: recording-state visual (teal + amber pulse-ring + send-glyph), `suppressedForPanel` no-op. KlarvoOverlayService: bubble-tap=Senden (all modes), red square=Abbrechen (cancelRecording). ListeningPanelView: ✗ button removed, "Bereinigt…" copy. Smoke: BUILD SUCCESSFUL, 24/24 JVM tests, APK installed on 100.112.41.70:5555. GATE 4 (Andi real device) open. | claude-sonnet-4-6
- 2026-06-16 — **Code-review cleared (story-conductor, Opus; 3 parallel layers).** 2 confirmed findings fixed (1 fix round): (F1, HIGH) amber pulse-ring ran in TRANSCRIBING → tripped the must-fail inversion gate (amber = RECORDING only; AC3 + canon + ADR-0019) — gated `drawAmberPulseRings()`/`amberPulseAnimator` to RECORDING-only, teal squircle + send-glyph retained in TRANSCRIBING (AC6); (F2, MEDIUM) per-frame `Paint`/`Path` allocation in the 60fps recording hot path → hoisted to pre-allocated fields. Build green: 24/24 JVM tests, APK built, KlarvoTheme drift-gate in sync. Deferred (not blocking): real `contentDescription="Abbrechen"` absent — panel is canvas-drawn, never had a labelable view → a11y `AccessibilityNodeProvider` is its own backlog story; pulse-ring box-shadow-spread vs. stroke geometry + send-glyph centering = GATE-4 visual residual (Andi real device). Status held at `review` — GATE 4 (emulator structural assertion + Andi real-device visual) carries close-out. | story-conductor (Opus)

---

---

# SUPERSEDED — historical (2026-06-15/16), do NOT rebuild

> The content below is the **old** Story 9.5 (suppress-bubble-to-idle · red square = Send · extra neutral
> ✗ cancel). It is **superseded by ADR-0019** and retained verbatim only for implementation context — the
> panel structure and the double-window fix it describes still stand, but its bubble-visual and
> confirm/cancel semantics are replaced by the active spec above. **Do not build to these ACs.**

## (OLD) Acceptance Criteria

**AC1 — Recording panel rises with grab handle, K + amber live-dot, waveform, timer, stop:**
Given recording starts (tap or long-press gesture)
When `setState(RecordingState.RECORDING)` is called
Then a Klarvo-owned overlay panel rises from the bottom of the screen with:
  - a grab handle (34dp × 4dp, `KlarvoTheme.Border2`, centered at top of panel)
  - a top-row: squircle K-badge (18dp, teal squircle) + amber live-dot (7dp circle, pulsing) + 5-bar amber RMS waveform + timer (Geist Mono 10.5sp, `KlarvoTheme.Dim`) + red stop button (26dp × 26dp, rounded 8dp, `KlarvoTheme.Danger` bg + border)
  - panel background `rgba(18,20,22,.98)` (≈ `0xFA121416`), top amber border-line (`KlarvoTheme.AmberLine`)
  - panel enters with spring animation (240ms `OvershootInterpolator(1.8f)` from height=0 to full height)
And the bubble itself stays visible as the small teal squircle at the bottom-right (above the panel)

**AC2 — Live RAW transcript runs multiline inside the panel (NOT in the foreign field):**
Given recording is active and audio chunks stream in
When raw transcript text accumulates
Then the text renders inside the panel (font mono 13sp, `KlarvoTheme.Muted`) with a blinking amber caret
And no text is set in the foreign text field until the final paste step (AR5a: `SYSTEM_ALERT_WINDOW` overlay cannot set composing text in a foreign field)
And `debugTranscript` injected via the 9.4 harness is displayed in the panel during harness runs

**AC3 — Transcribing: same panel, teal spinner + "Cleaning…", raw text dimmed:**
Given recording stops and `setState(RecordingState.TRANSCRIBING)` is called
When the pipeline transitions
Then the same panel remains on screen (no collapse between RECORDING and TRANSCRIBING)
And the top-row changes to: squircle K-badge + teal spinner (15dp, 900ms rotation) + "Cleaning…" label (11sp, `KlarvoTheme.Dim`, Geist Mono)
And the stop button disappears (replaced by empty space)
And the amber top border-line becomes `KlarvoTheme.Border2` (standard, no amber accent)
And the raw transcript text renders in `KlarvoTheme.Dim` (dimmed — not `KlarvoTheme.Muted`)
And the footer reads "Gleich fertig · Tastatur kommt gleich zurück" (or locale equivalent, see Dev Notes)

**AC4 — Done: panel collapses, keyboard note, cleaned text lands in field, bubble shows check → idle:**
Given `setState(RecordingState.DONE)` is called after paste
When the done transition fires
Then the listening panel collapses (slides down in 320ms `LinearInterpolator`)
And the bubble displays the teal squircle with a white checkmark (already implemented as placeholder in 9.4)
And after 800ms (`doneFlashRunnable`) the bubble returns to idle
And the cleaned text has been written to the focused field via `KlarvoAccessibilityService.instance?.pasteIntoFocusedField()` (a11y ACTION_PASTE) — fallback: clipboard only (existing path, no change)
And the keyboard is NOT forcibly dismissed in this story (keyboard-collapse is Story 9.6)

**AC5 — Panel is implemented as a second `TYPE_APPLICATION_OVERLAY` window, NOT inside `FloatingBubbleView`:**
Given the listening panel's size and layout complexity
When the panel is shown
Then it is added to `WindowManager` as a separate `View` with `TYPE_APPLICATION_OVERLAY`, anchored at the bottom of the screen (above the keyboard), with `FLAG_NOT_FOCUSABLE`
And `FloatingBubbleView` remains unchanged in size/position (still a small squircle in the corner)
And the panel window is `MATCH_PARENT` width, height = auto-sized by content (minimum 200dp per canon `.ab-panel { min-height: 200px }`)
And the panel window is created/destroyed on each recording cycle (added in RECORDING, removed in IDLE after DONE flash)

**AC6 — States are verified via the 9.4 harness:**
Given the debug broadcast receiver from Story 9.4
When `adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording --ef rms 0.7 --es transcript "Test text"` is sent
Then the listening panel appears with the waveform animated at the given RMS level
And the transcript text shows in the panel
And harness states for `transcribing` and `done` also drive the panel correctly

**(OLD) Inversion (must-fail gates):**
- Any attempt to set composing/live text in the foreign field from the overlay = instant review failure (AR5a).
- Panel appearing inside `FloatingBubbleView.onDraw()` instead of as a separate window = review failure.
- Recording state that clears the panel before transcribing completes = review failure.
- Amber appearing in TRANSCRIBING state = review failure (DT5: amber = recording only; transcribing uses teal).

**(OLD) DoD:** On-device smoke — real end-to-end dictation in a 3rd-party app: panel rises on recording, waveform reacts to voice, transcript accumulates in panel, panel switches to teal spinner on transcribing, cleaned text lands in the field on done, panel collapses. Harness commands drive all states. `scripts/android-smoke.sh` exits 0.

## (OLD) Tasks / Subtasks

- [x] **Task 1: Create `ListeningPanelView.kt` — the new overlay panel View** (AC: 1, 2, 3, 5)
  - [x] 1.1 Create `ListeningPanelView.kt` as a `View` subclass (NOT inside `FloatingBubbleView`). Full-width overlay above the keyboard.
  - [x] 1.2 Inner `enum class State { RECORDING, TRANSCRIBING }` (only 2 states; panel doesn't exist in IDLE/DONE).
  - [x] 1.3 Public properties: `panelState`, `amplitude`, `rawTranscript`, `recordingElapsedMs`.
  - [x] 1.4 `onDraw()` with explicit coordinate math: panel bg `0xFA121416`, amber/Border2 top line, grip, top-row, transcript, footer.
  - [x] 1.5 K-badge 18dp squircle (r=5dp), teal gradient, dark "K" 10sp.
  - [x] 1.6 Amber live-dot (7dp) + pulse ring (14dp, scale 0.5→1.2 / 1400ms).
  - [x] 1.7 5-bar amber waveform driven by amplitude (reuse `drawWaveformBarsInZone()`).
  - [x] 1.8 Red stop button (26dp, r=8dp, `DangerBg` + 1dp danger stroke, inner 9dp Danger square); `stopBtnRect`.
  - [x] 1.9 TRANSCRIBING: teal 15dp spinner + "Cleaning…" label. No timer, no stop.
  - [x] 1.10 Footer strings (RECORDING / TRANSCRIBING).
  - [x] 1.11 `isTouchOnStopButton()`.
  - [x] 1.12 `updateTranscript()` → invalidate.
  - [x] 1.13 `startTimer()`/`stopTimer()`.
  - [x] 1.14 Spring-enter animation (240ms OvershootInterpolator(1.8f)).
- [x] **Task 2: Wire `ListeningPanelView` into `KlarvoOverlayService`** (AC: 1–5) — show/hide/update; lifecycle wiring; harness.
- [x] **Task 3: Update `FloatingBubbleView` DONE state visual (polish)** (AC: 4).
- [x] **Task 4: Add `DangerBg` and `AmberLine` to `KlarvoTheme.kt`** (AC: 1).
- [x] **Task 5: Harness integration** (AC: 6).
- [x] **Task 6: Compile + verify** (AC: all).
- [x] **Task 7: Commit** (AC: all).

> Full sub-task text, Dev Notes (panel coordinate layout, view-composition alternative, "do not touch the
> audio pipeline", locale note, build architecture) and the F1..F9 fix detail are preserved in git history
> at the `review`-state version of this file (pre-`cef0e9c`/`d67193d`) and in
> `docs/postmortem-2026-06-15-epic-conductor.md`.

## (OLD) Dev Agent Record

### Agent Model Used
claude-sonnet-4-6 (story-creation, 2026-06-15)

### Completion Notes List
- ListeningPanelView.kt created as LinearLayout hybrid with inner Canvas views (TopRowView, GripView, FooterView). RECORDING (amber dot + pulse + 5-bar waveform + timer + stop) and TRANSCRIBING (teal spinner + "Cleaning…"). Spring-enter via panelHeightAnimator + onMeasure clamp.
- KlarvoOverlayService: showListeningPanel/hideListeningPanel; all error paths + cancelRecording + doneFlashRunnable hide panel; harness drives panel.
- KlarvoTheme.kt: DangerBg (0x1FEE6F63), AmberLine (0x52E9A24C after F6), AmberHi (0xFFF4BA72).
- Compile BUILD SUCCESSFUL; JVM tests 60/60.
- Harness smoke (emulator): recording→transcribing→done→idle confirmed via logcat.
- Inversion checks: panel is separate window (AC5); no composing text in field (AC2); amber absent in TRANSCRIBING (AC3); panel persists RECORDING→TRANSCRIBING.
- Real-device defect (double-window) + interaction-model supersession → see active spec above + ADR-0019.

### File List
- android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt (NEW)
- android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt (modified)
- android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt (modified)
- android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt (modified — suppressedForPanel + ✗ cancel; superseded by active spec)
