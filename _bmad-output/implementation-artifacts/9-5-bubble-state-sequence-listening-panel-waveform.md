# Story 9.5: Bubble State Sequence + Listening Panel + Waveform

Status: done

> **RE-FASHIONED (2nd time) 2026-06-17 against [ADR-0019](../../docs/adr/0019-cross-platform-design-ssot.md)
> Amendment §4′ + §4′-Addendum (Modell B).** The previously-built approach (recording-state bubble with
> amber pulse-ring + send-glyph · **tap the bubble = Send** · red square **in the panel** = Cancel) is now
> **SUPERSEDED** — Andi's real-use review (2026-06-17) found two competing moving elements + an asymmetric
> control split. Modell B replaces it with **one control cluster at the bubble's spot**. The full prior
> build-record (its ACs, tasks, Dev Agent Record) is preserved verbatim in the **SUPERSEDED — Layer 2**
> appendix below; the still-older first build is **Layer 3**. *Do NOT build to either appendix.*
>
> This is a **modification of the standing build**, not greenfield. The separate-overlay listening panel and
> the no-double-window fix already exist and **stand**; this story re-shapes the recording interaction
> (cluster), makes the panel passive, and changes done→green.

## Story

As a user dictating from a text field,
I want the bubble to become a single control cluster while recording — **➤ Send** (teal), a live **amber
waveform**, and **✗ Cancel** (red) in one place — with the preview panel staying passive, and a clear
green "done",
so that there is exactly one place to finish or discard a dictation, the colour-semantics match desktop
(red = cancel only), and nothing competes for my eye.

## Stands vs Flips (read this first)

**Stands — already built and correct, do NOT re-do:**
- `ListeningPanelView` as a **separate `TYPE_APPLICATION_OVERLAY` window** (panel ≠ inside the bubble view).
- The **no-double-window** invariant: only ONE recording overlay form (the retired HOLD-tap "expand to bar"
  window stays retired), and the **bubble window stays alive so taps reach `handleTouch`** — now load-bearing
  because the cluster's buttons live in that window.
- Panel infrastructure: grip, K-badge, multiline mono transcript + amber caret, footer; the
  RECORDING→TRANSCRIBING persistence (no collapse between them); DONE→IDLE flash plumbing.
- Token values from Story 9-10 codegen (`KlarvoTheme.kt` generated from `klarvo.css`).
- AR5a (no composing/live text into the foreign field); batch-only pipeline (no fake streaming).

**Flips — this story's work (6 changes, delta from the standing §4 build → Modell B):**
1. **Recording bubble → control cluster.** Remove the single recording-bubble (teal squircle + amber
   **pulse-ring** + send-glyph). In RECORDING the bubble window grows into a **cluster** at the dock spot:
   `[➤ send] [amber waveform] [✗ cancel]` on a soft semi-transparent backdrop with a static amber ring.
2. **Send affordance: bubble-tap → dedicated ➤ teal Send button.** Tap-the-bubble-to-send is **removed**;
   send is the teal paper-plane button. **No checkmark here** (checkmark = done only).
3. **Cancel affordance: panel red square → ✗ red button in the cluster.** The red cancel **leaves the panel**
   and joins the cluster. The panel no longer has any red control.
4. **Panel becomes passive.** Remove from the panel: the 5-bar amber waveform, the amber live-dot + pulse,
   the red square. Keep: K-badge, "Aufnahme" label, timer, live transcript + amber caret, footer. (The amber
   top border-line on the recording panel **stays** per canon.)
5. **Waveform relocates into the cluster** (amber, **between** ➤ and ✗) — the **only** moving element during
   recording.
6. **done → success green.** The done bubble flips from teal squircle to the canon **`.ab-bubble.done`**
   (success-green gradient + dark check), then **collapses back to idle** (Andi: "wichtig, dass danach das
   normale idle zurückkehrt").
   Plus: **transcribing keeps the dock spot occupied** (Variante B, Andi-approved) — the cluster collapses to
   one teal **processing bubble** (`.ab-bubble.proc`, spinner) while the panel shows its teal spinner.

## Acceptance Criteria

All visual/geometry values are **binding from the canon source** (`docs/design/overhaul/source/Klarvo Design
System.html` file-local CSS + `assets/klarvo.css` tokens), fingerprint `efe726c6…`. Read values there, never
from this prose. dp = the canon's px values (1:1).

**AC1 — RECORDING: the bubble window becomes a control cluster at the dock spot.**
Given recording starts (any gesture mode) and `setState(RecordingState.RECORDING)` is called
When the bubble is rendered
Then the corner bubble window **grows** into a horizontal cluster (`.ab-cluster`) anchored at the dock
(right ≈ 8dp, raised to sit above the panel — canon `bottom:228px` equivalent) containing, left→right:
  - a **➤ Send button** (`.ab-cbtn.send`): 40×40dp, radius 12dp, teal-gradient fill
    (`KlarvoTheme.TealHi`→`TealLo`, 150°), shadow `0 6px 18px rgba(0,0,0,.5)` + glass inset; paper-plane glyph
    (`M22 2 11 13` + `M22 2 15 22l-4-9-9-4 20-7z`), 19dp, stroke 2.2dp, `KlarvoTheme.OnTeal`
  - a **live amber waveform** (`.hwave`) **between** the buttons (see AC4)
  - a **✗ Cancel button** (`.ab-cbtn.cancel`): 40×40dp, radius 12dp, fill `KlarvoTheme.DangerBg`
    (`0x1FEE6F63`), 1dp border `0x66EE6F63`, ✗ glyph (`M18 6 6 18` + `M6 6l12 12`), 19dp, stroke 2.4dp,
    `KlarvoTheme.Danger`; `contentDescription`/aria = "Abbrechen"
  - cluster backdrop: rounded rect radius 18dp, fill `rgba(20,22,24,.55)` (`0x8C141618`), gap 9dp between
    items, padding 6dp, **static** soft amber ring `0 0 0 1.5px KlarvoTheme.AmberLine` (NOT animated)
And the touch target of each button is ≥ 48dp (transparent padding if the visual is 40dp).
And the **idle "K" bubble does NOT show** during recording (it is replaced by the cluster).

**AC2 — ➤ Send button = finish/send (the primary, non-red affordance).**
Given recording is active and the cluster is shown
When the **➤ Send button** is tapped
Then `stopAndProcessRecording()` is called (stop → transcribe → clean → paste) — exactly as the gesture-mode
stop does today
And there is **no checkmark** on the send control (a checkmark belongs only to `done`, AC7)
And **tapping the bubble/cluster background is NOT a send** (the standing "tap-the-bubble = send" wiring is
removed; only the ➤ button sends).

**AC3 — ✗ Cancel button = discard, and it lives in the cluster (not the panel).**
Given recording is active and the cluster is shown
When the **✗ Cancel button** is tapped
Then `cancelRecording()` is called (discard, no paste, panel + cluster dismissed → IDLE)
And the **panel contains no red/danger control** at all (the old red square is gone from the panel).

**AC4 — Live amber waveform sits between ➤ and ✗; it is the only motion during recording.**
Given recording is active
When audio amplitude streams in
Then a 5-bar amber waveform renders **between** the Send and Cancel buttons: container height 18dp, bar width
3dp, gap 3dp, colour `KlarvoTheme.Amber`, bars driven by RMS amplitude (reuse `drawWaveformBarsInZone()`),
canon idle motion = `abwv` 850ms ease scaleY .4↔1 staggered (0/120/240/80/300ms) when amplitude is flat
And the waveform is **non-interactive** (no hit zone)
And it is the **only** animating element on screen during RECORDING (no pulse-ring, no panel waveform, no
panel live-dot).

**AC5 — RECORDING panel is passive: text + time only.**
Given recording is active and `setState(RecordingState.RECORDING)`
When the panel is shown
Then the panel (`.ab-panel.rec`, separate overlay window) contains ONLY: grip (34×4dp `Border2`); a top-row
of **K-badge** (18dp teal squircle, r5) + **"Aufnahme"** label (Geist Mono, `Dim`) + **timer** (Geist Mono
13sp, `Muted`, right-aligned); the **live raw transcript** (mono 13sp `Muted`) + blinking **amber caret**;
the **footer** "Tastatur pausiert · kehrt beim Einfügen zurück" (locale)
And the panel has **no waveform, no amber live-dot/pulse, no red square** (all removed/relocated)
And the panel keeps its **amber top border-line** (`.ab-panel.rec` → `KlarvoTheme.AmberLine`), background
`rgba(18,20,22,.98)` (`0xFA121416`), min-height 200dp
And no text is written to the foreign field during recording (AR5a).

**AC6 — TRANSCRIBING (Variante B): cluster collapses to one teal processing bubble; dock spot stays occupied.**
Given recording stops (via ➤ or a gesture stop) and `setState(RecordingState.TRANSCRIBING)` is called
When the pipeline transitions
Then the cluster collapses to **one teal processing bubble** (`.ab-bubble.proc`) at the same dock spot
(40dp teal-gradient squircle + a rotating spinner glyph, 20dp, `KlarvoTheme.OnTeal` stroke, `spin` 900ms) —
the place where ➤/✗ were stays occupied (continuity; no empty corner)
And the **same panel stays on screen** (no collapse between RECORDING and TRANSCRIBING); its top-row changes
to K-badge + **teal spinner** (15dp) + **"Bereinigt…"** label; the raw transcript dims to `KlarvoTheme.Dim`;
the amber top border-line reverts to `KlarvoTheme.Border2` (no amber); footer reads "Gleich fertig ·
Tastatur kommt gleich zurück"
And **no amber appears in TRANSCRIBING** (amber = recording/live only — both the processing bubble and the
panel spinner are teal).

**AC7 — DONE: success-green bubble + check, then back to idle; cleaned text lands in field.**
Given `setState(RecordingState.DONE)` is called after paste
When the done transition fires
Then the panel collapses (slides down) and the dock shows the **success-green done bubble**
(`.ab-bubble.done`: `linear-gradient(150°, KlarvoTheme.SuccessHi #62E0A4, KlarvoTheme.Success #4FC58A)`,
dark check polyline 20dp, stroke 3dp `KlarvoTheme.OnTeal`) — visibly distinct from the teal idle bubble
And after the done flash the bubble **returns to the normal idle "K"** (teal squircle) — the corner returns to
its resting idle state (Andi: das normale idle muss zurückkehren)
And the cleaned text has been written to the focused field via
`KlarvoAccessibilityService.instance?.pasteIntoFocusedField()` (a11y ACTION_PASTE) — fallback clipboard only
And the keyboard is NOT forcibly dismissed in this story (keyboard-collapse is Story 9.6).

**AC8 — Window geometry transitions are clean (the structural invariant).**
Given the recording lifecycle idle → recording → transcribing → done → idle
When each state is entered
Then the bubble overlay window resizes correctly per state: idle = single 40dp squircle; recording = cluster
(~ send 40 + gap 9 + waveform + gap 9 + cancel 40 + padding 12 ≈ 150dp wide × ~52dp); transcribing = single
40dp proc bubble; done = single 40dp done bubble; back to idle = single 40dp squircle
And there is **never a second recording overlay form** alongside the panel (no double-window regression)
And the bubble window stays alive throughout so the cluster buttons receive touch (`handleTouch`); the retired
HOLD-tap bar window is NOT reintroduced.

**AC9 — Token source for success-green (ADR-0019 §2: no hand-typed hex).**
Given the done bubble needs a two-stop green gradient and only `--k-success` exists as a token today
When the done visual is implemented
Then `--k-success-hi: #62E0A4` is added to `klarvo.css` (formalises the literal already used by the canon's
`.ab-bubble.done`), `KlarvoTheme.kt` is **re-generated** via the 9-10 codegen so `KlarvoTheme.SuccessHi`
exists, and the done gradient uses `SuccessHi`→`Success` — **no hand-typed hex** in `FloatingBubbleView.kt`
And the canon fingerprint is re-stamped: update `docs/design/overhaul/source/MANIFEST.md` (`sourceFingerprint`
+ a provenance row "added `--k-success-hi` token") since `klarvo.css` changed.

**AC10 — States verified via the 9.4 harness (machine signal) + GATE-4 (real device).**
Given the debug broadcast receiver from Story 9.4
When `adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording --ef rms 0.7 --es transcript "Test"` is sent (and `transcribing`/`done`/`idle`)
Then on the **emulator** the overlay **window structure** matches AC8 (recording = panel + cluster window of
cluster size; transcribing = panel + single proc window; done/idle = single bubble window; never a double
recording form) — this is the machine-checkable oracle (E9: emulator = window-structure oracle, NOT a pixel
oracle)
And **GATE-4 (Andi, real device)** is the visual/interaction gate (see DoD).

**Inversion (must-fail gates):**
- Any **red/danger** control wired to **send/confirm** = instant review failure (ADR-0019 §3: red = Abbrechen
  only, both platforms).
- A **checkmark** shown on the recording **Send** control = review failure (check = done only).
- **Tapping the bubble/cluster background** triggering send (standing wiring not removed) = review failure.
- A **red square still in the panel** = review failure (cancel is the cluster ✗).
- **Waveform or amber** appearing **in the panel** during recording, OR **amber in TRANSCRIBING** = review
  failure (waveform lives in the cluster; amber = recording only).
- The **idle "K"** shown during recording instead of the cluster = review failure.
- A **second recording overlay form** alongside the panel (double-window regress) = review failure.
- Composing/live text set in the foreign field from the overlay = review failure (AR5a).
- Hand-typed green hex in `FloatingBubbleView.kt` instead of generated `KlarvoTheme.SuccessHi` = review failure
  (ADR-0019 §2).

**DoD:** On-device smoke on **Andi's real device** — real end-to-end dictation in a 3rd-party app (e.g.
Chrome address bar, WhatsApp), across modes (PTT hold/release · HOLD-tap · TOGGLE · AUTOSTOP · AUTO):
recording shows the **cluster** (➤ teal · amber waveform · ✗ red) at the bubble spot with the **passive
panel** (text + time, no waveform/red); **➤ sends** (→ teal proc bubble + panel spinner → cleaned text lands
in the field → panel collapses → **green done bubble → back to idle**); **✗ cancels** (panel + cluster
dismiss, nothing pasted); no double overlay; waveform is the only motion while recording. Emulator harness
drives the window-structure assertion (AC10) for compile/regression only — **GATE 4 = Andi's real device**
(emulator is not a visual oracle, E9). `scripts/android-smoke.sh` exits 0.

## Tasks / Subtasks

- [x] **Task 1: Token — add success-hi + regen** (AC: 9)
  - [x] 1.1 Add `--k-success-hi: #62E0A4` to `docs/design/overhaul/source/assets/klarvo.css` (next to `--k-success`).
  - [x] 1.2 Re-run the 9-10 token codegen → `KlarvoTheme.kt` gains `SuccessHi`. Verify the KlarvoTheme drift-gate is green.
  - [x] 1.3 Re-stamp `docs/design/overhaul/source/MANIFEST.md`: new `sourceFingerprint` (`cat HTML CSS | md5sum`) + a provenance row "added `--k-success-hi` token (9-5 Modell B done-green)".

- [x] **Task 2: Recording cluster in `FloatingBubbleView` + window resize** (AC: 1, 2, 3, 4, 8)
  - [x] 2.1 Replace the recording-bubble rendering (teal squircle + amber **pulse-ring** + send-glyph) with the
    **cluster**: backdrop rounded-rect (r18, `0x8C141618`, static amber ring 1.5dp `AmberLine`), then
    `[➤ send]`, `[waveform]`, `[✗ cancel]` (geometry per AC1). Remove `drawAmberPulseRings()`/`amberPulseAnimator`.
  - [x] 2.2 Grow the bubble overlay window to cluster size on RECORDING (WindowManager LayoutParams width/x),
    shrink back to 40dp on TRANSCRIBING/DONE/IDLE (AC8). Keep the window-alive invariant (no retired bar window).
  - [x] 2.3 Draw the ➤ send glyph (paper-plane, OnTeal) and ✗ cancel glyph (Danger) per AC1; draw the cluster
    backdrop + amber ring.
  - [x] 2.4 Draw the amber waveform **between** the buttons (reuse `drawWaveformBarsInZone()`, RMS-driven), 18dp
    zone; ensure it is the only animation in RECORDING.
  - [x] 2.5 Hit-test **two zones** (send / cancel) with ≥48dp targets; waveform zone non-interactive. Fix the
    alpha-reset path so the cluster (not idle) shows while recording.

- [x] **Task 3: Confirm/cancel wiring in `KlarvoOverlayService`** (AC: 2, 3)
  - [x] 3.1 Wire the cluster **➤ send** zone → `stopAndProcessRecording()`. **Remove** the standing
    "tap-the-bubble = send" wiring (bubble-background tap no longer sends).
  - [x] 3.2 Wire the cluster **✗ cancel** zone → `cancelRecording()`.
  - [x] 3.3 Confirm composition with gesture modes (TOGGLE tap / HOLD release / AUTOSTOP / AUTO) — they still
    start/stop; whatever stops = send; cluster ✗ = the only explicit cancel. (Per-mode nuance is 9-7 — do not expand.)

- [x] **Task 4: Panel → passive in `ListeningPanelView`** (AC: 5, 6)
  - [x] 4.1 Remove from the RECORDING panel: the 5-bar waveform, the amber live-dot + pulse, the red square
    (`stopBtnRect`/`isTouchOnStopButton` and its draw). Keep grip, K-badge, "Aufnahme", timer, transcript +
    amber caret, footer.
  - [x] 4.2 TRANSCRIBING panel unchanged in spirit (teal spinner 15dp + "Bereinigt…", dimmed text, Border2 top
    line) — verify it still matches the canon after the recording-panel edits.

- [x] **Task 5: Transcribing proc bubble + done-green + return-to-idle in `FloatingBubbleView`** (AC: 6, 7)
  - [x] 5.1 TRANSCRIBING: render the single **teal proc bubble** (`.ab-bubble.proc`: teal squircle + 20dp OnTeal
    spinner, `spin` 900ms) at the dock spot; window shrinks to 40dp.
  - [x] 5.2 DONE: render `.ab-bubble.done` (gradient `SuccessHi`→`Success`, dark check polyline 20dp).
  - [x] 5.3 After the done flash, **restore the idle "K"** bubble (return-to-idle). Stop/clean up all
    recording/transcribing animators.

- [x] **Task 6: Locale / copy** (AC: 5, 6) — confirm "Aufnahme", "Bereinigt…", footers ("Tastatur pausiert ·
  kehrt beim Einfügen zurück" / "Gleich fertig · Tastatur kommt gleich zurück") match canon German copy.

- [x] **Task 7: Compile + verify** (AC: all)
  - [x] 7.1 `scripts/android-smoke.sh` exits 0 (Kotlin compile clean, DEBUG APK built); JVM tests pass; KlarvoTheme drift-gate green.
  - [x] 7.2 Emulator window-structure assertion (AC10/AC8): idle/recording/transcribing/done/idle window counts
    + cluster window geometry; no double-window. (Machine signal — NOT the visual gate.) Capture evidence under `gate4-evidence/9-5/`.
  - [ ] 7.3 **GATE 4 (Andi, real device):** end-to-end across modes per DoD.

- [x] **Task 8: Commit** (AC: all) — stage only touched files (`klarvo.css`, `KlarvoTheme.kt`, `FloatingBubbleView.kt`,
  `KlarvoOverlayService.kt`, `ListeningPanelView.kt`, `MANIFEST.md`, this story). Never `git add .`.

## Dev Notes

### What is the actual delta (the tree currently holds the §4 tap-to-send build)
The standing build (SUPERSEDED Layer 2 below) has: a recording-state **bubble** (teal squircle + amber
pulse-ring + send-glyph), **bubble-tap = send**, and a **red square in the panel = cancel**. Modell B replaces
that interaction. The **listening panel as a separate overlay window** and the **no-double-window fix** stand.
Read Layer 2 for the exact current shapes of `FloatingBubbleView`/`KlarvoOverlayService`/`ListeningPanelView`
— this story edits those, it does not start from zero.

### Canon values (binding — read from the canon, fingerprint `efe726c6…`)
File-local CSS in `Klarvo Design System.html`:
- `.ab-cluster` (l.47): right 8, gap 9, padding 6, radius 18, `box-shadow 0 0 0 1.5px var(--k-amber-line)`,
  `background rgba(20,22,24,.55)`. Recording artboard places it at `bottom:228px`.
- `.ab-cbtn` (l.48): 40×40, radius 12, `box-shadow 0 6px 18px rgba(0,0,0,.5)` + glass. `.send` = teal gradient
  + OnTeal. `.cancel` = `var(--k-danger-bg)` + `1px solid rgba(238,111,99,.4)` + Danger. `svg` 19px.
- `.hwave` (l.67–74): height 18, 5 bars width 3 gap 3 amber; `@keyframes abwv` scaleY .4↔1 850ms staggered.
- `.ab-panel` / `.ab-panel.rec` (l.77–83): bg `rgba(18,20,22,.98)`, min-height 200; `.rec` keeps
  `border-top-color var(--k-amber-line)`. Passive recording panel = grip + (K + "Aufnahme" + timer) + text +
  caret + foot (no waveform/dot/red — those are the removals).
- `.ab-bubble.proc` (l.46–47, in-repo Variante-B extension): teal gradient squircle + `.spinner` 20px; the
  transcribing artboard places it at `bottom:228px`.
- `.ab-bubble.done` (l.43): `linear-gradient(150deg, #62E0A4, var(--k-success))`, color `#05201B`; check
  polyline 20px stroke OnTeal width 3.
- Send glyph (l.726): `M22 2 11 13 / M22 2 15 22l-4-9-9-4 20-7z` stroke 2.2; cancel glyph (l.728):
  `M18 6 6 18 / M6 6l12 12` stroke 2.4.
`assets/klarvo.css`: `--k-success #4FC58A` exists; `--k-success-hi #62E0A4` to be added (AC9).

### Touch-routing constraint (carried — still applies)
`FLAG_NOT_FOCUSABLE` panels receive no touch directly; the **bubble window** view's touch listener handles the
cluster. Two hit zones now (send/cancel). Keep the F1/F2 pattern (return true on `ACTION_DOWN` inside a control
so `ACTION_UP` arrives) and the coordinate translation. The cluster grows the bubble window — make sure the
window x/width follow so the right edge stays anchored and touch math uses the new window size.

### "Live RAW transcript" (unchanged)
Pipeline is batch-only (no streaming STT). The transcript area is blank during real recording, populated by
`debugTranscript` under the harness. Do NOT invent fake chunking.

### Files to Modify
| File | Change |
|------|--------|
| `docs/design/overhaul/source/assets/klarvo.css` | add `--k-success-hi: #62E0A4` (AC9) |
| `docs/design/overhaul/source/MANIFEST.md` | re-stamp fingerprint + provenance row (AC9) |
| `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt` | regen — gains `SuccessHi` (codegen output; do not hand-edit) |
| `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` | recording cluster (backdrop+ring, ➤/✗ buttons, waveform between), window resize, hit-zones; transcribing proc bubble; done-green; return-to-idle; remove pulse-ring |
| `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` | ➤ zone→send, ✗ zone→cancel; remove bubble-tap=send; window LayoutParams per state |
| `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt` | strip panel waveform + live-dot + red square (passive) |

No Rust/Tauri/Desktop files. Desktop already has red=cancel (the parity target); no desktop change here.

### References
- [Source: docs/adr/0019-cross-platform-design-ssot.md] — §3 colour-semantics, §4′ Modell B, §4′-Addendum (transcribing Variante B).
- [Source: docs/design/overhaul/source/Klarvo Design System.html, l.43–52, 67–83, 715–776] — cluster, cbtn, hwave, panel, proc, done, send/cancel glyphs.
- [Source: docs/design/overhaul/source/assets/klarvo.css] — token values (`--k-success`, teal/amber/danger).
- [Source: docs/design/overhaul/source/MANIFEST.md] — fingerprint `efe726c6…`; in-repo extensions table.
- [Source: docs/design/overhaul/mockup-9-5-transcribing-done.html] — Andi's approved transcribing-B + done-G1 (experiential approval).
- [Source: docs/postmortem-2026-06-15-epic-conductor.md] — double-window defect + traps (PTT touch-stream, alpha reset, MIUI harness dead).
- [Source: _bmad-output/project-context.md] — minSdk 24, no Compose, never `git add .`, Android changes require on-device smoke.

## Dev Agent Record (Modell B build, 2026-06-18) — claude-opus-4-8

**Task 1 — Token:** Added `--k-success-hi:#62E0A4` to `klarvo.css` after `--k-success`. Re-ran `gen-android-theme.mjs`; `KlarvoTheme.kt` regenerated (27 tokens, `SuccessHi = 0xFF62E0A4`). Drift-gate `--check` PASSED. `MANIFEST.md` re-stamped (fingerprint `717d5d3879090a58db4d732f5c35208f`, provenance row added).

**Task 2 — FloatingBubbleView cluster:** Full rewrite of RECORDING rendering. Cluster constants: `CLUSTER_VISUAL_W_DP=150`, `CLUSTER_VISUAL_H_DP=52`, `CLUSTER_SHADOW_PAD_DP=8`, btn=40dp r12, gap=9dp, pad=6dp, backdrop r18 `0x8C141618`. Static amber ring `1.5dp AmberLine`. Two canvas sub-paths for send-glyph (M22,2→11,13 diagonal + M22,2→15,22→11,13→2,9→close body polygon). Touch zones tracked as `clusterSendZoneEnd`/`clusterCancelZoneStart` (set during `onDraw`). `suppressedForPanel` made no-op; `barAnimator` only in RECORDING (no scale pop, no pulse ring). Removed unused `appIconDrawable`/`ContextCompat`/`Drawable` imports.

**Task 3 — KlarvoOverlayService wiring:** Added `preclusterBubbleX: Int?` to save/restore x position. `adjustLayoutForState(RECORDING)` expands to `clusterW×clusterH`, right-anchors at saved dock spot (`x = savedX + touchTargetPx - clusterW`); other states restore saved x + idle square. `handleTap(RECORDING)` routes `isTouchInConfirmZone` → `stopAndProcessRecording()`, `isTouchInCancelZone` → `cancelRecording()`, backdrop = no-op (AC2). `showListeningPanel()` adds `FLAG_NOT_TOUCHABLE` + removed touch listener (panel passive).

**Task 4 — ListeningPanelView passive:** `applyAnimatorsForState(RECORDING)` pauses all animators. `TopRowView.onDraw(RECORDING)` stripped to K-badge + "Aufnahme" label + timer (no waveform, no livedot, no red square). TRANSCRIBING path unchanged. Locale strings confirmed: "Aufnahme" / "Bereinigt…" / "Tastatur pausiert · kehrt beim Einfügen zurück" / "Gleich fertig · Tastatur kommt gleich zurück".

**Task 5 — Transcribing proc bubble + done-green:** TRANSCRIBING draws teal squircle + rotating spinner (`rotationAnimator`). DONE draws `LinearGradient(SuccessHi→Success)` + dark check polyline (`drawCheckMark`, 3dp stroke, `OnTeal`). `doneFlashRunnable` restores idle state (K-badge, teal gradient, idle animators reset).

**Task 7 — Smoke + structural gate:**
- `scripts/android-smoke.sh` → `SMOKE BUILD OK v0.5.0`, 24/24 JVM tests PASSED, APK installed on real device `100.112.41.70:5555` (from prior session; HyperOS harness dead after force-stop — known issue).
- Emulator structural gate (`emulator-5554`, `klarvo-emu` AVD, density ~2.625):
  - IDLE: `(897,1200)(162×162)` — 1 bubble ✓
  - RECORDING: `(0,0)(fillxwrap) gr=BOTTOM alpha=0.8` (panel) + `(351,1200)(435×178)` (cluster ≈ 166×68dp) ✓
  - TRANSCRIBING: panel stays + `(897,1200)(162×162)` proc bubble ✓ (no collapse between states)
  - DONE: panel collapsed, `(897,1200)(162×162)` done bubble ✓
  - Post-flash (back to IDLE): `(897,1200)(162×162)` idle ✓
  - Evidence: `gate4-evidence/9-5/window-structure.txt`
- GATE-4 visual (Andi real device): PENDING (see Task 7.3 — emulator structural GREEN is the machine gate; real-device aesthetic + interaction = Andi's gate)

## File List

- `docs/design/overhaul/source/assets/klarvo.css` (added `--k-success-hi` token)
- `docs/design/overhaul/source/MANIFEST.md` (fingerprint `717d5d38…`, provenance row)
- `scripts/gen-android-theme.mjs` (added `'SuccessHi'` to `CONSUMED_IDENTIFIERS`)
- `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt` (regenerated — 27 tokens, `SuccessHi`)
- `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` (Modell B full rewrite)
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` (cluster window resize + touch routing)
- `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt` (panel passive in RECORDING)
- `gate4-evidence/9-5/window-structure.txt` (emulator structural evidence)

## Change Log

- 2026-06-19 — **GATE-4 real-device GREEN → Status `done` (Andi).** Real end-to-end dictation on the real
  device passed: cluster (➤ teal · amber waveform · ✗ red) at the bubble spot, passive panel, **➤ sends**
  → teal proc bubble + panel spinner → cleaned text lands in the field → green done bubble → idle; **✗
  cancels** (nothing pasted); no double overlay. Build `Klarvo-v0.5.0-20260617-2105` (commit `8d597c9`,
  debug-keystore-signed, installed without data loss). **4 follow-ups raised by Andi — NOT 9-5 gaps,
  deferred to fresh sessions, homed in `docs/backlog.md` ("Story 9-5 GATE-4 green — Modell B interaction
  follow-ups"):** (1) cluster waveform must be RMS-reactive like desktop (AC4 intended RMS-driven — looks
  static); (2) swap ➤/✗ so ➤ Send sits at the idle-bubble position (thumb habit); (3) Android cleaned
  live-preview still unbuilt (confirms existing backlog item); (4) HOLD mode needs different bubble
  behavior (release already sends → separate ➤/✗ redundant + ✗ unusable). | Claude (Opus 4.8)
- 2026-06-18 (PM) — **Real-device debug + AC reality-reconcile (Opus, commits `ec21774`/`064142f`).** Andi's
  real-device smoke surfaced 3 issues; resolved by mechanical verification (DEBUG_SET_STATE harness works on the
  real device → force state + `screencap` + `dumpsys window`):
  (1) **Panel transparency root cause = HyperOS force-dims `FLAG_NOT_TOUCHABLE` overlays to window alpha 0.8**
  (overrides params/view alpha; PixelFormat + bg-color were red herrings). Fix: drop NOT_TOUCHABLE from the panel
  window (now touchable, absorbs its own touches — harmless, the cluster is a separate window). Opaque verified
  across RECORDING/TRANSCRIBING/DONE. See memory `reference_hyperos_overlay_quirks`.
  (2) **AC5 "live raw transcript" is NOT achievable on Android** — Groq STT is batch-only (no partials), so the
  transcript area stays empty during recording; only the blinking amber caret indicates "listening" (verified
  visible). **DESCOPE: AC5's live-text requirement → its own new story ("Android Live-Preview" via on-device
  Whisper). For 9-5, caret-only is the accepted RECORDING preview.**
  (3) **Footer "Tastatur pausiert …" was false** — an overlay cannot dismiss another app's IME (NOT_FOCUSABLE);
  real keyboard collapse needs the a11y service (**Story 9-6, revived**). Footer reworded to honest "🎙 Ich höre
  zu …" / "🎙 Wird verarbeitet …". **AC5/AC6 footer wording is now superseded by the honest text.**
  **To close 9-5:** Andi finger-smoke (mic→STT→paste; already worked in logs) + accept the AC5 caret-only /
  honest-footer reconcile above, then Status → done in this file AND sprint-status.yaml. | Claude (Opus 4.8)
- 2026-06-18 — **Modell B built (Tasks 1–8 complete, GATE-4 machine side GRÜN).** Cluster (➤ teal / amber waveform / ✗ red) at bubble spot; panel passive; transcribing = proc bubble + panel spinner; done = success-green → idle. Emulator structural window assertion GRÜN (all 5 states verified). Pending: GATE-4 real-device visual smoke (Task 7.3, Andi). | Claude (Opus 4.8)
- 2026-06-17 — **Story RE-FASHIONED (2nd) to Modell B (§4′ + transcribing Variante B).** Status `done`→`ready-for-dev`.
  ACs/Tasks rewritten: recording = control cluster (➤ teal send · amber waveform · ✗ red cancel) at the bubble
  spot; panel passive; transcribing = teal proc bubble (Variante B, Andi-approved); done = success-green → back
  to idle. Canon extended in-repo (`.ab-bubble.proc`), fingerprint `b95f86f9…`→`efe726c6…`, ADR-0019 §4′-Addendum
  + MANIFEST row added. Prior §4 tap-to-send build moved verbatim to SUPERSEDED Layer 2. Picks via
  `mockup-9-5-transcribing-done.html` (transcribing=B, done=G1). | Claude (Opus) + Andi (mockup gate)
- 2026-06-16 — **GATE 4 GREEN → done** (the now-superseded §4 build). Emulator structural assertion GREEN; Andi
  real-device smoke approved function + interaction ("erstmal abgesegnet"); Andi then flagged the two-moving-
  elements / asymmetric-control issue that triggered Modell B. See Layer 2 for the full §4 record.

---

---

# SUPERSEDED — Layer 2 (ADR-0019 §4 original: recording-bubble · tap=send · red square in panel)

> The content below is the **§4 tap-to-send** Story 9.5 — **built and briefly `done` (2026-06-16)**, then
> superseded by Modell B (§4′) above. The **separate-overlay panel** and the **no-double-window fix** it
> describes still stand; its **bubble-visual (pulse-ring + send-glyph), tap-to-send, and panel-red-square**
> are replaced by the cluster above. **Do not build to these ACs.**

## (L2) Story
As a user dictating from a text field, I want the bubble to run idle→recording→transcribing→done with a
Klarvo-owned listening panel, so that I see live feedback and the cleaned text lands in my field — sending by
tapping the bubble and cancelling with the red square, exactly mirroring the desktop colour-semantics.

## (L2) Acceptance Criteria (superseded)

**AC1 — Recording panel rises with grab handle, K + amber live-dot, waveform, timer, red Abbrechen square**
(panel grip 34×4 Border2; top-row K-badge 18dp + amber live-dot 7dp pulsing + 5-bar amber waveform + timer +
**red square = Abbrechen** 26×26 r8 DangerBg+border+inner 9dp Danger; panel bg `0xFA121416`, amber top line;
spring enter 240ms OvershootInterpolator(1.8f)). Bubble stays in its recording state above the panel.

**AC2 — Live RAW transcript runs multiline inside the panel (NOT in the foreign field)** (mono 13sp Muted +
amber caret; AR5a no composing text in field; `debugTranscript` shown under harness).

**AC3 — Transcribing: same panel, teal spinner + "Bereinigt…", raw text dimmed** (no collapse RECORDING→
TRANSCRIBING; K-badge + teal spinner 15dp 900ms + "Bereinigt…"; red square gone; amber line → Border2; text Dim;
footer "Gleich fertig · Tastatur kommt gleich zurück").

**AC4 — Done: panel collapses, cleaned text lands in field, bubble check → idle** (320ms slide; teal squircle +
white check; 800ms `doneFlashRunnable` → idle; a11y `pasteIntoFocusedField()`, clipboard fallback; keyboard not
dismissed — that's 9.6).

**AC5 — Panel is a second `TYPE_APPLICATION_OVERLAY` window, NOT inside `FloatingBubbleView`** (MATCH_PARENT
width, content-auto height min 200dp; created/destroyed per cycle; `FloatingBubbleView` stays a corner squircle).

**AC6 — Bubble recording state + confirm/cancel semantics (the §4 core, NOW SUPERSEDED)** — recording bubble =
teal squircle + **amber pulse-ring** (`abbubblepulse` 1400ms) + **send-glyph**; **short tap on the bubble =
Senden** (`stopAndProcessRecording()`); **red square in panel = Abbrechen** (`cancelRecording()`); no separate
neutral ✗; on DONE/IDLE restore normal visual. *(Modell B replaces tap-to-send with the ➤ button and moves
cancel into the cluster.)*

**AC7 — States verified via the 9.4 harness** (DEBUG_SET_STATE drives panel + bubble; MIUI caveat: broadcast
dead on Andi's real device → emulator-only machine signal, not a visual oracle).

**(L2) Inversions:** red/danger wired to send = fail; bubble staying idle during recording = fail; separate
neutral ✗ present = fail; composing text in foreign field = fail; second recording overlay form = fail; amber in
transcribing = fail.

**(L2) DoD:** real-device smoke across modes — bubble shows amber-pulse send-form; **tap sends**; **red square
cancels**; no double overlay; `scripts/android-smoke.sh` exits 0. GATE 4 = Andi real device.

## (L2) Dev Agent Record (ADR-0019 §4 rebuild, 2026-06-16) — claude-sonnet-4-6
- Task1 `FloatingBubbleView.kt`: recording form = teal squircle + `drawAmberPulseRings()` (1400ms, two amber
  rings) + `drawSendGlyph()` (paper-plane, OnTeal 2.2); `suppressedForPanel` → no-op; `onMeasure` simplified
  (bar-mode removed); `updateAnimators` starts amberPulse on RECORDING, stops on DONE/IDLE.
- Task2 `KlarvoOverlayService.kt`: panel red square → `cancelRecording()`; `handleTap` RECORDING → all modes
  `stopAndProcessRecording()` (tap=send).
- Task3 `ListeningPanelView.kt`: removed `cancelBtnRect`/`isTouchOnCancelButton`/✗ draw; "Cleaning…" → "Bereinigt…".
- Task4: smoke exits 0; JVM 24/24; APK installed on 100.112.41.70:5555; MIUI harness dead → compile gate.
- Code-review (story-conductor, Opus, 3 layers): F1 (HIGH) amber pulse-ring leaked into TRANSCRIBING → gated to
  RECORDING-only; F2 (MEDIUM) per-frame Paint/Path alloc → hoisted. Deferred: real `contentDescription` (canvas-
  drawn → a11y NodeProvider own story); pulse-ring/send-glyph centring = GATE-4 residual.
- GATE-4: emulator structural GREEN (idle 1 / recording 2 = panel 1080×525 + bubble 162×162 / transcribing 2 /
  done 1; evidence `gate4-evidence/9-5/`); Andi real-device "erstmal abgesegnet" → then Modell B.

### (L2) File List
- `FloatingBubbleView.kt` (recording visual, amber pulse-ring, send-glyph, suppressedForPanel no-op)
- `KlarvoOverlayService.kt` (panel red square→cancel, bubble-tap→send, ✗ removed)
- `ListeningPanelView.kt` (✗ removed, "Bereinigt…")

---

---

# SUPERSEDED — Layer 3 (first build, 2026-06-15: suppress-to-idle · red square = Send · neutral ✗)

> The original Story 9.5. Superseded twice. The **panel structure + double-window fix** it introduced stand;
> its bubble-visual and confirm/cancel semantics are replaced. **Do not build to these ACs.** Full sub-task
> text, Dev Notes, and the F1..F9 fix detail live in git history (pre-`cef0e9c`/`d67193d`) and in
> `docs/postmortem-2026-06-15-epic-conductor.md`.

**(L3) gist:** Created `ListeningPanelView.kt` as the separate overlay window (grip, K-badge, amber dot+pulse,
5-bar waveform, timer, **red stop = Send**, teal spinner + "Cleaning…" in transcribing). Wired into
`KlarvoOverlayService` (show/hide/update, harness). `KlarvoTheme.kt`: `DangerBg 0x1FEE6F63`, `AmberLine
0x52E9A24C` (after F6), `AmberHi 0xFFF4BA72`. BUILD SUCCESSFUL, JVM 60/60. Real-device review found the
double-window defect + the red=Send semantic inversion → Layer 2, then Modell B.
