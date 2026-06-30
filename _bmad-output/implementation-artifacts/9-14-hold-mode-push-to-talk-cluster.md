# Story 9.14: HOLD-Modus (Push-to-Talk) — vereinfacht (ein Abbrechen-Button)

Status: ready-for-dev

> **⚠️ NEU GEFASST 2026-07-01 — SUPERSEDET die Zwei-Ziel-B-Sprache-Fassung.** Die Zwei-Ziel-Variante
> (Sperren + Abbrechen, commits `ce20bb0`/`c431ba5`) fiel an Andis echtem Gerät durch: alles zu groß,
> Anker-K springt vs. Idle-Bubble, und die Design-Erkenntnis „Loslassen = senden braucht keinen eigenen
> Button" macht die zwei gleichberechtigten Ziele überflüssig. Diese Fassung baut das **vereinfachte**
> Modell (ADR-0019 Amendment 2026-07-01): **ein** Abbrechen-Button, Senden = Loslassen, kein Sperren.
> Tasks/Subtasks + Dev Notes werden von `bmad-create-story` gegen den neuen Canon neu generiert.

## Story

As a user dictating on Android with the **Hold** gesture,
I want to hold to record, **let go to send**, and drag to a single clear **Cancel** button to discard,
so that sending is effortless (just release) and only the destructive action needs a deliberate target — with thumb-sized, on-screen-fitting controls.

## Scope (locked — vereinfachtes HOLD, nur wenn `longPressMode == RecordingMode.HOLD`)

- **Anker-Bubble** (teal-Gradient-Squircle, „K", amber Halte-Ring) am Dock, wo der Finger hält — **Größe = Idle-Bubble-Größe** (`bubbleSizeDp`, responsive ~44dp). Kein eigener Größen-Parameter; entkoppelt vom Button-Regler. Kein Größen-/Orts-Sprung gegenüber der Idle-Bubble.
- **Ein** runder **Abbrechen-Button** (✗, dunkle blickdichte Ruhe-Fläche + roter Ring) — wächst weg von Daumen/Dock-Kante. **Größe am `recordingButtonSizeDp`-Regler.**
- **Grow-on-target:** sobald der Finger den Button erreicht, **wächst er + leuchtet rot**, Label → „loslassen = abbrechen".
- **Release-to-commit:** **Loslassen auf dem Button = `cancelRecording()`**. **Loslassen überall sonst = senden (`stopAndProcessRecording()`)**. **Zurückziehen** vom Button vor dem Loslassen = nichts (Undo).
- **Dynamik (bauen):** Ghost-Bubble folgt dem Finger · Origin-Bubble faded auf ~.32 beim Ziehen · Caption wechselt auf „Finger auf Abbrechen · loslassen löst aus".
- **Dock-adaptiv:** Button-Position spiegelt je Andock-Seite; wächst nie unter den Daumen.
- **Regler erweitern:** `recordingButtonSizeDp`-Auswahl auf **{52, 60, 72, 84, 96}** setzen (Andi-entschieden 2026-07-01; ersetzt {60,72,88}; 72 bleibt Default). Gilt für TAP-Surface (9-15) **und** HOLD-Abbrechen-Button. JVM-Floor-Test (≥48) bleibt erfüllt; Default 72 unverändert.

**Hard scope boundaries:**
- **Kein Sperren / kein Lock→TAP** in HOLD — die gesamte Sperren-Mechanik (`lockHoldToCluster`, Lock-Ziel, hochziehen-sperren) entfällt für HOLD.
- **Kein** Senden-Button (Senden = Loslassen).
- **Nur** HOLD-Modus. Tap/Toggle/Auto-Stop/Auto = 9-15-TAP-Surface (nicht hier anfassen).
- **Keine** RMS/Waveform-Änderung (Story 9-12). **Keine** Token-Änderung (`KlarvoTheme.kt`). Farb-Semantik: **teal=Anker · amber=live · rot=Abbrechen**.
- 9-7 (Gesten-Modus-Erkennung) **nicht** still erweitern.
- `FLAG_NOT_TOUCHABLE` nie.

## Acceptance Criteria

**AC1 — Vereinfachte HOLD-Surface.** Given `pushToTalkActive` und Aufnahme startet, When gezeichnet wird, Then erscheinen: kleine Anker-Bubble (= Idle-Größe) am Dock + **ein** Abbrechen-Ruhe-Button (✗, rot-Ring) + ruhiger Waveform-Chip + Caption „Aufnahme · loslassen = senden" — und **kein** Sperren-Ziel, **kein** Senden-Button, keine alten `.ab-holddock`-Surfaces. Werte aus `mockup-mobile-hold-simple.html` Frame `sRest`.

**AC2 — Anker = Idle-Größe, kein Sprung.** Die Anker-Bubble hat dieselbe Größe wie die Idle-Bubble (`bubbleSizeDp`) und erscheint an derselben On-Screen-Position, an der die Idle-Bubble war (Daumen ist physisch dort) — kein sicht­barer Größen- oder Orts-Sprung beim Übergang idle→hold.

**AC3 — Abbrechen am Regler.** Der Abbrechen-Button skaliert mit `recordingButtonSizeDp`. Der Regler bietet die Stufen **{52, 60, 72, 84, 96}** (ersetzt {60,72,88}; Default 72), als Segmented-Control in `ShortcutsContent.tsx`.

**AC4 — Grow-on-target.** When der Finger den Abbrechen-Button erreicht, Then wächst er + leuchtet rot + Label „loslassen = abbrechen". Render: `sHit`.

**AC5 — Release-to-commit + Senden-by-default.** Loslassen **auf** dem aktiven Abbrechen-Button → `cancelRecording()`. Loslassen **irgendwo sonst** → `stopAndProcessRecording()` (senden). Zurückziehen vom Button vor dem Loslassen → kein Auslösen.

**AC6 — Dynamik.** Beim Ziehen: Ghost-Bubble folgt dem Finger, Origin-Bubble faded, Caption wechselt auf „Finger auf Abbrechen · loslassen löst aus" (Render `sHit`).

**AC7 — Dock-adaptiv.** Button-Position spiegelt je Andock-Seite; wächst weg von Dock-Kante/Daumen.

**AC8 — Andere Modi/Zustände unberührt.** Tap/Toggle/Auto = 9-15-Surface; IDLE/TRANSCRIBING/DONE unverändert.

**Inversion (must-fail gates):**
- Sperren-Ziel oder ein Senden-Button im HOLD-Zustand sichtbar = review failure.
- Anker-Bubble größer als / versetzt zur Idle-Bubble = review failure (AC2).
- Auslösung beim Schwellwert *während der Bewegung* statt beim **Loslassen** = review failure (AC5).
- Loslassen außerhalb des Abbrechen-Buttons bricht ab statt zu senden = review failure (AC5).
- Farben getauscht = review failure.

## Tasks / Subtasks

- [ ] Task 1: Collapse the two-target data model to CANCEL-only (`FloatingBubbleView.kt`)
  - [ ] 1.1 `HoldTarget` enum (line 57): `{ NONE, LOCK, CANCEL }` → `{ NONE, CANCEL }`; update its KDoc and the file-level class doc (lines 10-51, 23-31, 38-42) to drop all "Sperren"/two-target language
  - [ ] 1.2 Remove fixed-bubble constants `HOLD_BUBBLE_DP`/`HOLD_BUBBLE_R_DP` (lines 255-256) — the anchor bubble now reads `bubbleSizeDp` via the existing `getBubbleSizeDp()` (line 633), same value `drawIdleBubble()` uses (line 685), per AC2
  - [ ] 1.3 Collapse `HOLD_TARGET_REST_DP`/`HOLD_TARGET_ACTIVE_DP` (lines 257-258, fixed px) into: REST diameter = `recordingButtonSizeDp` dp (mirrors the TAP surface's circle-diameter convention — `drawTapSurface`'s `scale = recordingButtonSizeDp / 132f`, radius literally `recordingButtonSizeDp*dp/2`, lines ~738-741); ACTIVE = REST × a scale constant `HOLD_CANCEL_ACTIVE_SCALE = 1.25f` (ratio derived from canon `.zone.rest{width:96px}`→`.zone.active{width:120px}`, `mockup-mobile-hold-simple.html` lines 99-100 — 120/96=1.25)
  - [ ] 1.4 Remove `HOLD_LOCK_OFFSET_ABOVE_DP` (line 272); replace `HOLD_CANCEL_OFFSET_BELOW_DP` with a 2D offset pair `HOLD_CANCEL_OFFSET_X_DP`/`HOLD_CANCEL_OFFSET_Y_DP` (see Dev Notes "Cancel-button offset geometry" for derivation + exact reference values — bubble-center → cancel-center is now diagonal, not purely vertical)
  - [ ] 1.5 Remove `HOLD_BUBBLE_EDGE_GAP_DP`/`HOLD_BUBBLE_TARGET_GAP_DP`/`HOLD_TARGET_FAR_INSET_DP` (lines 268-270) — the bubble no longer has its own HOLD-specific edge inset; it keeps the idle window's existing position (Task 6)
  - [ ] 1.6 Replace the fixed `HOLD_VISUAL_W_DP`/`HOLD_VISUAL_H_DP` constants (lines 279-284) with `@JvmStatic` functions `holdVisualWidthDp(buttonSizeDp)`/`holdVisualHeightDp(buttonSizeDp)` (mirrors `tapVisualWidthDp`/`tapVisualHeightDp`, lines 202-213) — window bounding box is now `bubbleSizeDp`(fixed) + offset + scaled-cancel-target(ACTIVE radius), a function of `recordingButtonSizeDp`, not a constant
  - [ ] 1.7 Keep `HOLD_CHIP_H_DP`, `HOLD_CHIP_BUBBLE_GAP_DP`, `HOLD_SHADOW_PAD_DP` unchanged (chip geometry unaffected by Lock removal)
  - [ ] 1.8 Update `recordingButtonSizeDp`'s KDoc (lines 161-166) — it now also drives the HOLD Cancel button, not just the TAP surface

- [ ] Task 2: Simplify the pure geometry functions (`FloatingBubbleView.kt` companion object)
  - [ ] 2.1 `holdBubbleCenter()` (lines 324-342): its Y-pinning currently derives from `lockOffsetAbove`/`targetActiveRadius` (line 340), which has no input left once LOCK is gone. Re-derive Y from the idle-anchor mechanism instead (see Task 6.2) — judgment call whether the function survives in a simplified form or is inlined/removed; do not keep dead LOCK-shaped parameters for their own sake (Code-Simplicity: obvious over clever)
  - [ ] 2.2 Replace `holdTargetCenters()` (lines 355-378, returns `Pair<HoldPoint,HoldPoint>` for lock+cancel) with a single-target equivalent, e.g. `holdCancelCenter(dockSide, bubbleCenter, offsetXDp, offsetYDp): HoldPoint` — `@JvmStatic`, pure, dock-side-mirrored (negate Δx for left dock, Δy always upward), returns one `HoldPoint`
  - [ ] 2.3 `isInsideCircle()` (lines 291-296) — unchanged, reused as-is

- [ ] Task 3: Rewrite `drawHoldTargets()` + helpers for the single-button surface (`FloatingBubbleView.kt` lines 1236-1497)
  - [ ] 3.1 Factor a shared private squircle-draw helper out of `drawIdleBubble()`'s gradient/shadow/corner-radius block (`side*0.30f` corner formula) — it now has 2 real consumers (idle bubble + HOLD anchor bubble), justifying extraction per the project's "factor out only on proven duplication" rule. The HOLD anchor calls it at `bubbleSizeDp` + adds the amber holding-ring overlay on top — this makes AC2 ("kein Größen-/Orts-Sprung") true by construction, not by copy-pasted-and-hopefully-matching constants
  - [ ] 3.2 Draw ONE cancel target via `drawHoldZone()` (lines 1330-1418) — strip the `isLock` parameter and all lock-branches (color/icon/label selection); keep the REST/ACTIVE radial-gradient + glow-ring + label rendering for CANCEL only
  - [ ] 3.3 `drawHoldChip()` (lines 1425-1473) — unchanged, still hugs the bubble
  - [ ] 3.4 `drawHoldCaption()` (lines 1476-1497) — add a hit-state parameter; render "Aufnahme · loslassen = senden" at rest, "Finger auf Abbrechen · loslassen löst aus" when `holdTargetHit == CANCEL` (AC6, canon `sHit` `.reccap` text)
  - [ ] 3.5 NEW: ghost-bubble + origin-fade dynamics (AC6) — once the finger crosses a small dead-zone away from the bubble (reuse the ~10dp `dragThresholdPx` convention already in `KlarvoOverlayService.kt`, see Task 4), draw the origin/anchor bubble at ~0.32 alpha (canon `opacity:.32`) and draw a ghost squircle (same shared helper from 3.1, ~0.92× the anchor's size per canon `.ghost`/`.heldbub` ratio 44/48) centered at the LIVE finger position forwarded from `KlarvoOverlayService` — the canon's ghost position is wherever the finger actually is, not a derived/interpolated point
  - [ ] 3.6 Delete `drawLockIcon()` (lines 1504-1522) and its pre-allocated fields `lockBodyRect`/`lockShackleRect`/`lockBodyFillPaint`/`lockShacklePaint` (lines 457-463) — dead once the Lock target is gone
  - [ ] 3.7 Delete `holdLockGradient*` cached-shader fields (lines 486-489); keep `holdCancelGradient*` (lines 490-493)
  - [ ] 3.8 Update `drawHoldTargets()`'s block comment (lines 1228-1234) for the single-button model

- [ ] Task 4: Forward live finger position for the ghost bubble (`KlarvoOverlayService.kt`)
  - [ ] 4.1 In the `pushToTalkActive` branch of `ACTION_MOVE` (lines 1188-1212), alongside the existing `holdTargetHit` hit-tracking, forward `(touchX, touchY)` to a new `bubbleView` property (e.g. `holdFingerX`/`holdFingerY: Float` + a `holdDragging: Boolean` dead-zone flag) so `drawHoldTargets()` can render the ghost at the actual finger position
  - [ ] 4.2 Reset the new ghost/drag state on `ACTION_UP`/`ACTION_CANCEL` (mirrors the existing `holdTargetHit = HoldTarget.NONE` resets at lines 1283/1302) and in `cancelRecording()`/`stopAndProcessRecording()` (lines 1530-1531/1563-1564) — same "transient HOLD flag must never leak into the next recording" lesson already documented at both call sites

- [ ] Task 5: Simplify release-to-commit + delete the Lock transition (`KlarvoOverlayService.kt`)
  - [ ] 5.1 In the `pushToTalkActive` release dispatch (lines 1265-1284), delete the `HoldTarget.LOCK ->` branch entirely; `CANCEL -> { holdDockActive=false; cancelRecording() }` and `NONE -> stopAndProcessRecording()` already match the new spec verbatim — no change needed to those two lines
  - [ ] 5.2 Delete `lockHoldToCluster()` whole (lines 1116-1139) — its only caller is removed in 5.1
  - [ ] 5.3 Delete `setLockedModeOnPanel()` (lines 448-452); in `ListeningPanelView.kt` delete the `isLockedMode` field (line 66), its label branch (line 433, "Aufnahme · 🔒 gesperrt") and its footer-override block (lines 592-598, "Finger losgelassen · weiter über die Knöpfe") — all dead once the Lock target is gone. `isHoldMode`/`setHoldModeOnPanel()` stay (still describes the live hold)
  - [ ] 5.4 Reword the "finding 2" comments at `KlarvoOverlayService.kt:1529`/`1562` that reference `isLockedMode` by name — the lesson (reset transient HOLD flags on stop/cancel) still applies, the flag is gone

- [ ] Task 6: Window sizing — `adjustLayoutForState()` HOLD branch (`KlarvoOverlayService.kt` lines 1041-1114)
  - [ ] 6.1 Recompute `holdW`/`holdH` (lines 1053-1054) from the new `holdVisualWidthDp(recordingButtonSizeDp)`/`holdVisualHeightDp(recordingButtonSizeDp)` (Task 1.6) instead of the fixed `HOLD_VISUAL_W_DP`/`HOLD_VISUAL_H_DP`
  - [ ] 6.2 Y-anchor (lines 1062-1071, currently calls `holdBubbleCenter()` with the now-gone Lock offset): since AC2 requires the bubble to sit at the EXACT idle on-screen position, derive the window's placement from `preclusterBubbleY`/`idleCenterY` (the mechanism already present, lines 1066/1072) directly, sized so the bubble's drawn center coincides with the idle center and the window extends however far the cancel target's diagonal offset (Task 1.4) requires in whichever direction(s) it grows — the new layout is diagonal, not purely vertical, so the old symmetric-vertical window-budget assumption no longer holds; re-derive from scratch rather than patching the old formula
  - [ ] 6.3 Preserve the bottom-edge clamp pattern (lines 1073-1079, `maxHoldY = screenH - NAV_BAR_CLEARANCE_PX - holdH`) — re-derive its `holdH` input, keep the clamp logic itself; add the equivalent clamp for whichever horizontal direction the cancel target now extends into (new — the old purely-vertical layout never needed a side clamp)
  - [ ] 6.4 Leave the still-open "first `ACTION_MOVE` reads stale idle-square-width" race (`docs/backlog.md:423`, carried forward per `:436`) as-is — not in this story's AC scope, do not silently fix or silently regress it further
  - [ ] 6.5 `getDockSide()` (lines 1020-1026) — no change expected (uses `preclusterBubbleX`/idle window px, not HOLD-specific)

- [ ] Task 7: Extend the `recordingButtonSizeDp` Settings control (touches 9-15's UI — story scope line "Regler erweitern: mehr + kleinere Stufen")
  - [ ] 7.1 Lower `TAP_BUTTON_SIZE_MIN` (`FloatingBubbleView.kt:193`, currently 60) — bounded below by the JVM test floor `recording_button_size_min_is_at_least_48dp` (`TapSurfaceTouchZoneTest.kt:192-197`, asserts `>= 48`) unless that test is intentionally updated too. **Exact new step values are an open design question (see Elicitation Report) — implement a provisional set bounded by [48,88], do not present it as final.**
  - [ ] 7.2 Extend the step array in `ShortcutsContent.tsx:657` (`[60, 72, 88] as const`) to match 7.1; reword the sub-label at line 654 ("TAP-surface Send/Cancel circle diameter.") since it now also governs the HOLD Cancel button
  - [ ] 7.3 Verify `KlarvoApi.kt:293`'s `.coerceIn(FloatingBubbleView.TAP_BUTTON_SIZE_MIN, TAP_BUTTON_SIZE_MAX)` picks up the new MIN automatically (no separate edit expected beyond 7.1)

- [ ] Task 8: JVM unit tests
  - [ ] 8.1 Replace `HoldTargetTouchZoneTest.kt` (186 lines, all 13 tests reference LOCK directly or via Lock-vs-Cancel relative positioning — full rewrite, not a patch) with a CANCEL-only equivalent: dock-mirroring (Δx sign flips by dock side, Δy unchanged), hit-resolution at REST radius, edge-of-radius hit/miss, ACTIVE-growth-doesn't-expand-hitzone, miss-cases (bubble position, window origin) — same no-Robolectric pure-function convention, against the new `holdCancelCenter`-equivalent from Task 2.2
  - [ ] 8.2 Add tests pinning `HOLD_CANCEL_ACTIVE_SCALE` and the new `holdVisualWidthDp/HeightDp(buttonSizeDp)` functions (mirrors `TapSurfaceTouchZoneTest.visual_width_scales_proportionally_with_button_size`)
  - [ ] 8.3 If Task 7.1 changes `TAP_BUTTON_SIZE_MIN`, update/add a test pinning the new floor (parallel to `recording_button_size_min_is_at_least_48dp`)
  - [ ] 8.4 `./gradlew :app:testUniversalDebugUnitTest --rerun-tasks` — must be green, no regression in `TapSurfaceTouchZoneTest`

- [ ] Task 9: Drift/compile gates
  - [ ] 9.1 `node scripts/gen-android-theme.mjs --check` — must stay `[ok]` (no `KlarvoTheme.kt` edits; all HOLD-local colors stay local `private const val`, matching the existing `HOLD_DANGER_HI`/`HOLD_DANGER_LINE`/`HOLD_ZONE_REST_BG` convention, lines 226-228)
  - [ ] 9.2 `cargo check --target x86_64-pc-windows-gnu` — only if Rust files touched (expected N/A unless Task 7 needs Rust-side bounds changes, mirrors 9-15 Task 6.3)

- [ ] Task 10: Structural smoke + GATE-4 evidence
  - [ ] 10.1 Emulator structural smoke via the `DEBUG_SET_STATE` broadcast pattern already used for 9-14's prior GATE-4 evidence (`adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording --ez hold_mode true --ef rms 0.6 -p com.klarvo.voice`, see `_bmad-output/implementation-artifacts/gate4-evidence/9-14/structure-recording-hold.txt` for the prior pattern) + `dumpsys window windows` capture confirming: ONE overlay window (no separate Lock-target surface), no `FLAG_NOT_TOUCHABLE`, window size reflects the new (smaller, single-target) geometry — not the prior build's `939x1076`
  - [ ] 10.2 Real-device GATE-4 (Andi's batched gate) — see story DoD below (unmodified, preserved verbatim)

## Dev Notes

- **Reuse, don't reinvent — `recordingButtonSizeDp` infrastructure already exists (Story 9-15).** The Cancel button's scaling, the Settings round-trip (FE↔`config.json`↔Android), and the proportional-scaling pattern (`scale = recordingButtonSizeDp / 132f`, `tapVisualWidthDp`/`tapVisualHeightDp`) are all already built and battle-tested in `FloatingBubbleView.kt`/`KlarvoOverlayService.kt`/`KlarvoApi.kt`/`ShortcutsContent.tsx`. This story extends/reuses that infrastructure for the HOLD Cancel button — it does not duplicate it. Full file chain for the config key: `src/components/settings/ShortcutsContent.tsx` → `src/components/SettingsPanel.tsx` → `src/types.ts` / `src/tauri-commands.ts` → `src-tauri/src/config/mod.rs` / `src-tauri/src/commands/settings.rs` (camelCase `recordingButtonSizeDp` in `config.json`, per `reference_config_json_camelcase_keys`) → `KlarvoApi.kt:96/292-293` → `KlarvoOverlayService.kt:2082` (`reloadBubbleAppearance()`) → `FloatingBubbleView.recordingButtonSizeDp` setter (coerces to `[TAP_BUTTON_SIZE_MIN, TAP_BUTTON_SIZE_MAX]`).

- **Anchor bubble = idle bubble, literally, not a lookalike.** `bubbleSizeDp` (private field, `FloatingBubbleView.kt:175`, default 56 but runtime value comes from the responsive formula `clamp(36, 0.11×min(screenW,screenH), 44)` in `KlarvoOverlayService.computeVisualSizeDp()`, lines 985-997 — confirms the ADR's "~44dp responsive") is read via the existing public `getBubbleSizeDp()`. AC2's "kein Sprung" is best satisfied by sharing the exact same squircle-draw code path between `drawIdleBubble()` and the HOLD anchor (Task 3.1), not by copying the gradient/corner-radius formula a second time with hand-matched constants — two copies WILL drift eventually.

- **Cancel-button offset geometry — derivation + exact reference values.** Extracted directly from `docs/design/overhaul/mockup-mobile-hold-simple.html`'s `sRest`/`sHit` frames (the "SIMPLIFIED HOLD CANON" override block, lines 93-106, applies on top of the inherited B-refined markup positions):
  - REST (`sRest`): bubble `right:26px;top:436px` at 48×48 (post-override) → center ≈ (343, 460) in the 393×895 viewport. Cancel zone `left:117px;top:252px` at 96×96 → center ≈ (165, 300).
  - ACTIVE (`sHit`): cancel zone `left:105px;top:240px` at 120×120 → center ≈ (165, 300) — **identical center to REST**, confirming center-fixed growth (only the drawn radius changes, same convention the old two-target code already used for hit-zone-stays-REST-size, Task 8.1's "growth doesn't expand hitzone" tests).
  - Ghost (`sHit`): `right:130px;top:372px` — this is illustrative ("wherever the finger is" in this one frame), not a formula input; render the ghost at the LIVE touch position, not at this fixed point.
  - Origin/faded bubble (`sHit`): `right:26px;top:436px;opacity:.32` — same position as REST, confirms the anchor never moves, only fades.
  - Derived offset (cancel-center − bubble-center) for a right-docked bubble: **Δx ≈ −178dp, Δy ≈ −160dp** (cancel sits up-and-toward-screen-center). Mirror Δx's sign for a left-docked bubble; Δy is always upward. The 393×895 viewport is rendered 1:1 as dp (confirmed: 1080/393 ≈ 2460/895 ≈ 2.75 = the device's density, same convention the original `HOLD_BUBBLE_DP`/`HOLD_LOCK_OFFSET_ABOVE_DP` etc. already used reading the prior B-refined mockup).
  - **Calibration caveat (read before treating these as gospel):** this story's own history is two prior GATE-4 failures caused by exactly this kind of mockup-px-as-dp value turning out too large at real device scale (9-15's first build, and this same 9-14 story's prior two-target build). The ADR records this specific mockup as "Andi-approved 2026-07-01" at device scale, which is stronger provenance than either prior failure had — but per the codebase's own established framing of these constants ("Not exact-pixel law — keep proportions, GATE-4 visual fidelity is Andi's real-device call"), treat 178/160 as the right ballpark, not a pixel-perfect contract. The gap between the (fixed) bubble and the (scaling) Cancel button is a fixed dp offset, not itself scaled by `recordingButtonSizeDp` — no canon text addresses whether it should scale; fixed-offset matches the old code's existing convention (gaps were always fixed dp, never user-scaled) and is the lower-risk default.

- **Window geometry is now diagonal, not vertical.** The old two-target HOLD window was a simple vertical column (Lock above, Cancel below, same X) cropped tightly around the bubble. The new window must accommodate the bubble pinned to its idle on-screen position (AC2) plus ONE target offset diagonally up-and-toward-center (or up-and-away-from-dock-edge, mirrored) from it — this is a real reshape of `adjustLayoutForState`'s HOLD branch (Task 6), not a parameter tweak. Reuse the existing `preclusterBubbleX`/`preclusterBubbleY` idle-anchor mechanism (already present for exactly this "remember where the idle bubble was" purpose) rather than re-deriving bubble position from the old Lock-offset-based `holdBubbleCenter()` Y-pinning formula.

- **Ghost-bubble/origin-fade dead-zone — reuse, don't invent a new threshold.** `KlarvoOverlayService.kt` already has a `dragThresholdPx = 10f * density` (line 491, declared 208) used for the free-drag-the-idle-bubble gesture. No equivalent threshold currently exists for HOLD (the old code's hit-tracking ran unconditionally from the first `ACTION_MOVE`). Reusing the same ~10dp convention for "has the finger left the bubble enough to start showing the ghost" is the lowest-risk default — it's an existing, already-tuned constant in this exact file, not a new magic number.

- **Color semantics — no new tokens.** `KlarvoTheme.kt` is generated from canon CSS (`node scripts/gen-android-theme.mjs --check` gate) and must NOT be hand-edited. All HOLD-local colors stay local `private const val` Canvas constants, following the existing `HOLD_DANGER_HI = 0xFFF4897E` / `HOLD_DANGER_LINE = 0x73EE6F63` / `HOLD_ZONE_REST_BG = 0xEB121416` convention (`FloatingBubbleView.kt:226-228`) — these survive unchanged (Cancel's colors don't change, only Lock's disappear). Binding: **teal=Senden(N/A here, no Send button)·amber=live·rot=Abbrechen** — never swapped.

- **`FLAG_NOT_TOUCHABLE` — never.** HyperOS dims such overlays to alpha 0.8 (`reference_hyperos_overlay_quirks`). Not used anywhere in this surface today; do not introduce it.

- **No structural smoke script exists for HOLD today.** `scripts/android-smoke.sh` has no HOLD-specific assertions (grep confirms). The prior story's GATE-4 structural evidence was captured manually via the `DEBUG_SET_STATE` broadcast + `dumpsys window windows` (see Task 10.1) — follow that same manual pattern again; there is no automated assertion to update.

- **Residual/out-of-scope (do not fix here, do not silently regress further):** the "first `ACTION_MOVE` reads stale idle-square-width" 1-frame race (`docs/backlog.md:423`, reconfirmed open at `:436`) is pre-existing, transient, self-correcting, and explicitly not in this story's AC — leave it. The `.heldbub .finger` indicator + inner amber ring (cosmetic, `docs/backlog.md:426`) was never pulled into the 07-01 re-scope's "now build it" list (only ghost/fade/caption were) — do not build it speculatively.

### Project Structure Notes

- All Kotlin changes live in `android/kotlin-src/com/klarvo/voice/{FloatingBubbleView,KlarvoOverlayService,ListeningPanelView}.kt` — this is the source of truth; `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/*.kt` is a 1:1 mirror, do not hand-edit it directly (it's regenerated/synced).
- JVM tests live in `android/kotlin-test/com/klarvo/voice/` (no `tests/` tree, no Robolectric — pure-function-under-test convention established by 9-13/9-15/9-14).
- Settings UI: `src/components/settings/ShortcutsContent.tsx` (control) + `src/components/SettingsPanel.tsx` (state/wiring) — no new files needed, this is an edit to existing 9-15 infrastructure.
- No new ADR needed — ADR-0019 Amendment 2026-07-01 (`docs/adr/0019-cross-platform-design-ssot.md` lines 179-194) already covers this story's decision.

### References

- [Source: docs/adr/0019-cross-platform-design-ssot.md#Amendment 2026-07-01 — HOLD vereinfacht]
- [Source: docs/design/overhaul/mockup-mobile-hold-simple.html — frames `sRest`/`sHit`, fingerprint `7e2829a5625c224fb2227cff53cefa70`]
- [Source: _bmad-output/planning-artifacts/epics-visual-overhaul.md#Story 9.14 / Story 9.15 — epic-level scope/anchors, now superseded in detail by this story file and the 07-01 amendment]
- [Source: _bmad-output/implementation-artifacts/9-15-mobile-tap-recording-surface-reskin.md — `recordingButtonSizeDp` infrastructure this story reuses]
- [Source: docs/backlog.md#Story 9-14 HOLD — Code-Review-Defers (2026-06-30) and #Story 9-14 — Re-Scope auf vereinfachtes HOLD (2026-07-01)]
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt — HOLD section lines 57-66, 119-145, 250-378, 1228-1524]
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt — HOLD touch/layout lines 1020-1139, 1143-1309, 1517-1588]
- [Source: android/kotlin-test/com/klarvo/voice/HoldTargetTouchZoneTest.kt — full file to be replaced]

## Anchors (binding design source)

- **ADR-0019 Amendment 2026-07-01** (`docs/adr/0019-cross-platform-design-ssot.md`) — vereinfachtes HOLD.
- **Bindendes Render (SOLL):** `docs/design/overhaul/mockup-mobile-hold-simple.html` (Frames `sRest` + `sHit`), Fingerprint `7e2829a5625c224fb2227cff53cefa70`. **Supersedet** `mockup-mobile-hold-B-refined.html`.
- Baut auf der Vorgänger-Implementierung `c431ba5` auf (entfernt deren Lock-Ziel + zweites Ziel; entkoppelt Anker-Größe; koppelt Abbrechen an Regler).
- Touch/Canvas: `reference_android_bubble_canvas_and_install.md`. Verifikations-Lehre: `feedback_gate4_smoke_needs_behavioral_delta.md`.

## DoD (surface-class)

DEBUG APK builds (via `tauri android build`, Node 20); JVM-Tests grün; Emulator strukturelle Smoke grün (Fenster-Struktur via `dumpsys window` unter `BMAD_CONDUCTOR=1`). **GATE-4 Bewegung/Touch/Visual = echtes Gerät (Andis Batch-Gate):** halten/loslassen=senden/auf-Abbrechen-ziehen, grow-on-target, Ghost-Folgen, Anker = Idle-Größe ohne Sprung, Regler-Stufen — nur am echten Gerät verifizierbar. `FLAG_NOT_TOUCHABLE` nie.

## Dependency

Nutzt 9-15-TAP-Surface-Infrastruktur (Geometrie-Pattern, `recordingButtonSizeDp`) — aber **nicht** den Gesperrt-Zustand (kein Lock mehr). Regler-Erweiterung berührt 9-15-Settings-UI (`ShortcutsContent.tsx`).

## Dev Agent Record

> _Wird beim Build neu befüllt._

## Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-06-26 | (frühere Fassung) Slide-Spur-HOLD gebaut (`c389c88`/`e92f4f3`), GATE-4 Real-Device FAILED (Andi). | conductor |
| 2026-06-26 | NEU GEFASST in B-Sprache (Zwei-Ziel: Sperren + Abbrechen) gegen `mockup-mobile-hold-B-refined.html`. | claude-opus-4-8 |
| 2026-06-30 | Implementiert (`ce20bb0`) + Code-Review-Fixes (`c431ba5`, A/B/C) + GATE-4 Struktur-Smoke grün. Status `review`. | claude-sonnet-5 / conductor |
| 2026-06-30 | **GATE-4 Real-Device FAILED (Andi):** zu groß · Anker-K springt vs. Idle · Design-Erkenntnis „Senden=Loslassen braucht keinen Button". | claude-opus-4-8 (conductor) |
| 2026-07-01 | **NEU GEFASST — vereinfacht** (ADR-0019 Amendment 2026-07-01): ein Abbrechen-Button · Senden=Loslassen · kein Sperren · Anker=Idle-Größe · Abbrechen am erweiterten Regler · Dynamik (Ghost/Fade/Caption) gebaut. Canon `mockup-mobile-hold-simple.html` (`7e2829a5…`). Zwei-Ziel-Scope superseded. `ready-for-dev`. | claude-opus-4-8 (conductor) |
| 2026-07-01 | Tasks/Subtasks + Dev Notes generated by `bmad-create-story` against the 07-01 canon + current code (`HoldTarget`/`drawHoldTargets`/`adjustLayoutForState`/`lockHoldToCluster` in `FloatingBubbleView.kt`/`KlarvoOverlayService.kt`, plus `ListeningPanelView.kt`'s `isLockedMode`). Open design questions (exact `recordingButtonSizeDp` step values; ghost/fade/caption trigger = any-drag vs on-target-only) flagged for Andi, not defaulted silently. | claude-sonnet-5 |
