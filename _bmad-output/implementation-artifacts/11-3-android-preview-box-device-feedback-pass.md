---
story: "11.3"
epic: "11"
title: "Android Preview-Box — Device Feedback Pass (fixed-size rolling window, header/footer cleanup, font scale, bubble Z-order fix)"
status: in-progress
track: L2-fix
gatedBy: ["11.2"]
buildsOn: ["11.2"]
enabledBy: []
inputDocuments:
  - docs/backlog.md#11-3 (Follow-up) — Android Preview-Box Geräte-Feedback-Pass — Source: Andi device-verify 2026-07-02
  - _bmad-output/implementation-artifacts/11-2-android-live-preview-port.md
  - _bmad-output/implementation-artifacts/9-15-*.md (recordingButtonSizeDp — device-tuned-after-first-pass precedent)
  - _bmad-output/project-context.md
---

# Story 11.3: Android Preview-Box — Device Feedback Pass

Status: in-progress

> **Epic 11 — Cross-Platform Live-Preview.** Story 11-2 (`done`, real-device-verified) shipped the
> Android preview panel: a passive text surface at the bottom overlay window that accumulates raw
> transcript on every speech pause. This story is Andi's real-device feedback pass on that panel —
> **HOCHGESTUFT 2026-07-07 from polish to usability blocker**: the panel's `WRAP_CONTENT` window
> grows with accumulated text and, because it shares the bubble's window type/flags, ends up
> **z-ordered above the bubble** — during a real dictation the growing panel can visually cover and
> input-block the ➤ Senden / ✗ Abbrechen controls in the bubble window underneath, making the device
> unusable mid-recording. This is the next Android story, **before Epic 8** (Andi, 2026-07-07).

## Design decisions (Andi — binding, do not re-litigate)

Source: `docs/backlog.md` §"11-3 (Follow-up)", decisions dated 2026-07-02 (items 1-5) and
2026-07-07 (item 6, the upgrade-to-blocker root-cause + fix direction).

1. **Header:** "Aufnahme" → "Live-Preview" (`ListeningPanelView.TopRowView`'s RECORDING label).
2. **Footer:** remove "Ich höre zu …" — unnecessary in the preview context.
3. **Fixed box size, not grow-with-text:** desktop's "box grows with text" behavior makes no sense
   on Android. **DECIDED: rolling window** — the box shows only the most recent lines; older
   content rolls out the top with a soft fade (replaces the 11-2 `ScrollView` auto-scroll
   mechanism entirely — **no scrolling**, the box's height stays constant).
4. **Remove `GripView`** (the grab-handle line, top-center) — it implies a resize affordance that
   doesn't exist; removing it frees vertical space so header elements sit closer to the top edge.
5. **Font scale up:** `FONT_PX_SP` (small/medium/large) changes from `11f/13f/15f` to
   **`13f/15f/18f`**, Android-only (`ListeningPanelView.kt:42`) — desktop's `FONT_PX_MAP` is
   untouched.
6. **Bubble must ALWAYS render above the preview — structural Z-order fix (NEW 2026-07-07, root
   cause of the usability blocker):** the bubble window (`bubbleParams`,
   `KlarvoOverlayService.kt:980`) and the panel window (`panelParams`,
   `KlarvoOverlayService.kt:2280`) currently share the same `overlayType`
   (`WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY`) and similar flags, so Android z-orders
   them by **add order** — the panel (added later, at recording start) ends up on top and can
   obscure/eat touches for the bubble's ➤/✗ controls. **DECIDED: raise the bubble structurally
   into a higher Z-band (a higher window type)** so it is on top **regardless of add order** — an
   explicit non-solution: **do NOT** "fix" this by merely re-adding/reordering the bubble window
   after the panel is shown; that is order-dependent and would regress the moment any future code
   path shows the panel before the bubble. See Dev Notes "Hardest technical point" below — this is
   the part of the story most likely to need a design/architecture call mid-implementation.

## Story

As an Android user dictating with the preview panel visible,
I want the panel to stay a fixed size, show only recent text, use the "Live-Preview" identity, and
never cover or block the bubble's Senden/Abbrechen controls,
so that I can always reach and finish/cancel my dictation, regardless of how much I've said.

## Scope boundaries (read before touching code)

**IN:**
- `ListeningPanelView.kt`: header label rename (item 1), footer RECORDING-caption removal (item 2),
  fixed-size rolling-window transcript rendering replacing the 11-2 `ScrollView`/auto-scroll
  (item 3), `GripView` removal + resulting layout tightening (item 4), `FONT_PX_SP` rescale
  (item 5).
- `KlarvoOverlayService.kt`: the panel window's `WindowManager.LayoutParams` height (currently
  `WRAP_CONTENT`, `KlarvoOverlayService.kt:2280-2301`) becomes fixed so the window itself cannot
  grow (item 3's "fixed box size" is a window-level property, not just a view-level one — a
  `WRAP_CONTENT` window around a rolling-window view would still be visually static internally,
  but only a fixed window height guarantees the window itself never grows/shrinks and never
  re-triggers the original overlap defect). The bubble/panel Z-order fix (item 6) — see Dev Notes.
- Possibly `KlarvoAccessibilityService.kt` **only if** the Z-order fix requires re-homing the
  bubble window's ownership (see Dev Notes "Hardest technical point" — do not assume this without
  first checking whether a same-owner fix exists).
- New/updated JVM unit tests for any new pure Kotlin logic introduced (rolling-window
  line-eviction, any Z-order/window-type guard function), following the existing
  `android/kotlin-test/com/klarvo/voice/` pattern.

**OUT (do not touch):**
- The delta-flush/transcription pipeline (`deltaSnapshotWav`, `flushPreviewDelta`,
  `appendPreviewText`, the Groq call chain) — 11-2 territory, untouched. This story only changes
  **how** the already-accumulated preview text is displayed and **how** the panel/bubble windows
  are sized/z-ordered.
- The Settings/Appearance category, config fields, or any Rust/TS file — **zero Rust changes,
  zero frontend changes**. This is a Kotlin-only story (confirmed: no config field changes are
  needed for any of the 6 items — font scale is a hardcoded Kotlin constant map, not a config
  field; rolling-window/Z-order are pure Kotlin/Android layout).
- `FooterView`'s TRANSCRIBING caption ("Wird verarbeitet …") — item 2 names only the RECORDING
  caption ("Ich höre zu …"); the TRANSCRIBING caption is not mentioned in the backlog decision and
  stays as-is.
- `applyAppearance`'s color/border/background logic (Story 11-2, AC-9) — untouched; only
  `FONT_PX_SP` values change, the lookup mechanism stays the same.
- Auto/AutoStop mode's one-shot silence/paste behavior, the delta-marker VAD mechanism, and
  anything in `KlarvoAudioRecorder.kt` — none of the 6 items touch audio/VAD logic. The panel
  window is shown for **all** recording modes (`showListeningPanel` call sites at lines 407, 416,
  1551 — not gated by mode), so items 1/2/3/4/5/6 apply whenever the panel is visible, not only
  during HOLD/TOGGLE preview-text accumulation.
- `FLAG_NOT_TOUCHABLE` on any overlay window (project-wide rule, HyperOS alpha-dim quirk — see Dev
  Notes reference).

## ⚠️ OPEN ITEMS — needs Andi's confirmation (not invented, not defaulted silently)

These two points are **not fully pinned** by the backlog/canon. ACs below encode the qualitative
behavior Andi decided; the **exact numeric parameters are a first-pass engineering choice that
must be confirmed/tuned at the real-device GATE-4**, the same way Story 9-15's
`recordingButtonSizeDp` shipped as a first guess and was device-tuned. Do not treat the numbers
below as final pixel-perfect spec — treat the qualitative shape (fixed height, rolling/fading
lines, no scroll // bubble structurally above panel, not by re-add) as binding.

1. **Rolling-window exact parameters** (item 3): how many lines/what fixed height in dp, and the
   fade duration/style, are not specified anywhere (backlog only says "shows only the last lines,
   older rolls out the top, softly faded, no scrolling"). First-pass proposal for the dev to start
   from: reuse the existing 200dp floor (`ListeningPanelView.kt:340`) as the **fixed** height
   (currently a *minimum*, becomes the *only* height), one line per preview-flush chunk (not
   joined with a space as today — see below), fade-out over ~250-320ms (matches the panel's
   existing 320ms collapse-animation duration convention). **Flag for Andi at GATE-4**: does the
   box feel too small/too large with real dictation pacing?
2. **Bubble Z-order fix mechanism** (item 6): Andi's decision is directional ("higher window
   type", explicitly not a re-add) but does not name a specific Android window type or confirm
   whether the bubble window's ownership needs to move (e.g. to `KlarvoAccessibilityService`,
   which the app already has bound and running, unlike a brand-new permission). This is flagged as
   the story's hardest technical point in Dev Notes — if no permission-free, same-owner fix
   exists, **escalate before building a larger reparenting change**, don't silently absorb the
   scope growth.

## Acceptance Criteria

**AC-1 (Header rename, item 1):**
Given the panel is in `State.RECORDING`,
When `TopRowView.onDraw` renders the label (`ListeningPanelView.kt:535-538`),
Then the label reads **"Live-Preview"** (was "Aufnahme") in the default case and
**"Live-Preview · halten"** (was "Aufnahme · halten") in `isHoldMode`,
And no other panel-state label changes (TRANSCRIBING's "Bereinigt…" is untouched).

**AC-2 (Footer RECORDING caption removed, item 2):**
Given the panel is in `State.RECORDING`,
When `FooterView.onDraw` renders (`ListeningPanelView.kt:697-702`),
Then the "🎙 Ich höre zu …" caption is **not drawn** (footer is empty/blank during RECORDING — no
replacement text is added unless a later Andi confirmation says otherwise),
And the TRANSCRIBING caption ("🎙 Wird verarbeitet …") is **unchanged** — out of scope per this
story's decisions (only "Ich höre zu…" was named for removal).

**AC-3 (Fixed-height, auto-scroll-to-newest, manually-scrollable transcript — item 3, the core usability fix; ⚠️ REVISED 2026-07-08 after Andi's device-test — see Change Log):**
Given preview text chunks accumulate during a HOLD/TOGGLE recording with `livePreviewEnabled ==
true` (the existing 11-2 `appendPreviewText`/`rawTranscript` pipeline, untouched),
When the accumulated text exceeds the panel's fixed content height,
Then (a) the panel's `WindowManager.LayoutParams` height is a **fixed value**, not `WRAP_CONTENT`
(`KlarvoOverlayService.kt:2328-2329`, `PANEL_FIXED_HEIGHT_DP` — **already delivered + device-verified**;
the window must not grow; the direct fix for the original "fills the whole screen" defect — KEEP),
(b) the transcript content area is a fixed-height **vertically scrollable** view, (c) on each preview
update the view **auto-scrolls so the NEWEST (most-recently-spoken) text at the bottom is visible by
default** — the shipped rolling-window instead showed the BEGINNING (the exact defect this revision
fixes), and (d) the user can **manually touch-scroll** up to review earlier text and back down; the
auto-scroll-to-bottom should not fight a user who has deliberately scrolled up mid-recording (a simple
"stick to bottom unless the user scrolled away" rule is acceptable — first-pass, device-tunable).
**Supersedes the 2026-07-07 "rolling window / no-scroll / evict-oldest-with-fade" rendering** (the
old AC-3 b/c/d + the pure `visibleLines` eviction function): now that the window height is FIXED, a
bounded `ScrollView`/`NestedScrollView` actually functions (the 11-2 ScrollView was inert only because
the window was `WRAP_CONTENT`). Dev: restore a bounded scroll view inside the fixed-height panel with
`fullScroll(FOCUS_DOWN)` (stick-to-bottom) on append; **remove** the `visibleLines` rolling-window
rendering + its now-obsolete `RollingWindowVisibleLinesTest`. **Keep** `sanitizePreviewChunk` (still
valid) and the P2 nothing-specific-to-rolling changes. Verification of the scroll behaviour is the
device gate; unit-test any pure helper that remains (e.g. a "stick-to-bottom unless scrolled-away"
decision) — otherwise no new pure-logic test is required.

**AC-4 (GripView removed, item 4):**
Given the panel is constructed (`ListeningPanelView.kt` `init` block, `GripView` at lines
274-279),
When the panel renders,
Then no grip/grab-handle element is drawn or added to the view hierarchy, and the vertical space it
previously occupied (4dp view + 11dp bottom margin) is reclaimed so the header row (`topRowView`)
sits closer to the panel's top edge (concrete new top-padding/margin value is the dev's
implementation call — verify visually at GATE-4, not pixel-specified by Andi).

**AC-5 (Font scale rescale, item 5):**
Given `applyAppearance` maps `config.previewFontSize` via `FONT_PX_SP` (`ListeningPanelView.kt:42,
209`),
When the panel applies appearance config,
Then `FONT_PX_SP` is `mapOf("small" to 13f, "medium" to 15f, "large" to 18f)` (was `11f/13f/15f`),
And desktop's `FONT_PX_MAP` (`src/components/settings/...`, Story 6-3) is **not** touched — this
is an Android-only Kotlin constant change.

**AC-6 (Bubble structurally above panel — item 6, the blocker fix):**
Given the bubble window (`bubbleParams`) and the panel window (`panelParams`) both currently use
`overlayType = TYPE_APPLICATION_OVERLAY` and are z-ordered by add-order (panel added later at
recording start → renders on top, `KlarvoOverlayService.kt:980-996` vs `2280-2314`),
When both windows are visible simultaneously (any recording state where `showListeningPanel` has
been called — all modes, not just HOLD/TOGGLE),
Then the bubble's touchable region (idle circle, or the expanded ➤/✗ cluster) always receives
touches and renders visually on top of the panel — **independent of which window was added
first or most recently** (i.e., the fix must survive the panel being shown/hidden/re-shown
multiple times during one recording, and must not rely on re-adding or reordering the bubble
window after each panel show/hide),
And the mechanism used is documented in Completion Notes with the specific Android API chosen
(e.g. a distinct window type/z-band, or another structural approach) — a reviewer must be able to
verify from the diff alone that the fix is order-independent (inversion test: show the panel
*before* first showing the bubble in a code-level trace/manual reasoning — the bubble must still
end up on top).

**DoD (surface-class — mirrors project testing rules):**
- New pure Kotlin logic (AC-3's line-eviction function, any AC-6 guard/resolver function) has JVM
  unit tests in `android/kotlin-test/com/klarvo/voice/`, following existing patterns
  (`RecordingModeSilenceSelectionTest.kt`, `DeltaSnapshotSliceTest.kt`). Inversions documented
  empirically (flip the eviction bound / order-dependence assumption → test goes RED).
- `scripts/android-smoke.sh` clean build/install (Kotlin-only story — this smoke script covers it
  fully, unlike 11-2's Settings changes which needed a full `tauri android build`; confirm no
  `.ts`/`.tsx`/`.rs` file appears in the File List before relying on the lighter smoke path).
- **Real-device Android smoke required (GATE-4, this story's actual deliverable per the
  "Bedienbarkeits-Blocker" upgrade)** — Andi, on his real device: a HOLD or TOGGLE dictation with
  preview enabled, speaking long enough to exceed the fixed box height (confirm rolling/fading
  behavior, no scroll, box never grows), **and explicitly confirming the ➤ Senden / ✗ Abbrechen
  controls stay reachable and functional throughout** (this is the blocker this story exists to
  fix — do not close without this specific confirmation). Also confirm the header reads
  "Live-Preview", the "Ich höre zu…" caption is gone, no grip line, and text feels appropriately
  sized at each of the three font-size settings. GATE-4 = real device, never emulator
  (`reference_android_emulator_window_structure_oracle`).
- Confirm AC-6's inversion at review time: reason through (or structurally test) whether the fix
  still holds if the panel is shown before the bubble, or shown/hidden multiple times — if the
  chosen mechanism is order-dependent, it does NOT satisfy this AC regardless of how it looks on
  first manual test.

## Tasks / Subtasks

- [x] **Task 1 — Header + footer text changes** (AC-1, AC-2)
  - [x] 1.1 `ListeningPanelView.kt:535-538`: rename `"Aufnahme"` → `"Live-Preview"`,
    `"Aufnahme · halten"` → `"Live-Preview · halten"`.
  - [x] 1.2 `ListeningPanelView.kt:697-702`: remove the RECORDING-branch caption text (draw
    nothing, or skip the `canvas.drawText` call for that branch) — leave the TRANSCRIBING branch
    untouched.

- [x] **Task 2 — Remove GripView, tighten layout** (AC-4)
  - [x] 2.1 Remove the `GripView` inner class and its `addView(gripView, gripParams)` call
    (`ListeningPanelView.kt:274-279`).
  - [x] 2.2 Adjust the reclaimed top padding/margin so `topRowView` sits closer to the panel's top
    edge (`init` block `setPadding`/margins, `ListeningPanelView.kt:271-288`) — implementation
    call, verify at GATE-4.

- [x] **Task 3 — Font scale rescale** (AC-5)
  - [x] 3.1 `ListeningPanelView.kt:42`: `FONT_PX_SP = mapOf("small" to 13f, "medium" to 15f,
    "large" to 18f)`.

- [x] **Task 4 — Fixed-size rolling-window transcript display** (AC-3, the core fix)
  - [x] 4.1 Design a pure Kotlin line-buffer/eviction function (e.g.
    `visibleLines(chunks: List<String>, maxLines: Int): List<Line>` where `Line` carries
    text + a fading/evicting flag) — no `Context`, no `View`, directly JVM-testable, mirrors the
    existing pure-function pattern.
  - [x] 4.2 Decide (first-pass, device-tunable — see "Open Items") a `maxLines`/fixed-height value
    and a fade duration; wire the flush pipeline so each `appendPreviewText` call is treated as one
    line-buffer entry (currently chunks are joined with a single space into one long string,
    `KlarvoOverlayService.kt:1647` — evaluate whether that join needs to change to a newline-join
    for "line" to mean something visually, or whether line-wrapping the single string within the
    fixed-height view is sufficient; document the choice in Completion Notes).
  - [x] 4.3 Remove `transcriptScrollView`/`fullScroll` auto-scroll (`ListeningPanelView.kt:130, 260,
    312-326`); replace with the fixed rolling-window rendering (view re-layout, fade-out
    animation for evicted lines — reuse `ValueAnimator` patterns already present in the file, e.g.
    `hideWithAnimation`'s style, for consistency).
  - [x] 4.4 JVM unit test for 4.1's pure function: assert eviction bound holds (buffer never
    exceeds `maxLines`), oldest-first eviction order. Inversion: an unbounded implementation must
    fail.

- [x] **Task 5 — Panel window: fixed height, not WRAP_CONTENT** (AC-3)
  - [x] 5.1 `KlarvoOverlayService.kt:2280-2301`: change the panel's `WindowManager.LayoutParams`
    height from `WRAP_CONTENT` to the fixed value chosen in Task 4.2 (converted to px via
    `resources.displayMetrics.density`, matching the existing `ListeningPanelView`
    `minimumHeight` conversion pattern at `ListeningPanelView.kt:339-340`).
  - [x] 5.2 Remove/reconcile the `minimumHeight = 200dp` floor in `ListeningPanelView`'s `init`
    (`ListeningPanelView.kt:340`) if it becomes redundant with the fixed window height — do not
    leave two competing height mechanisms.

- [x] **Task 6 — Bubble structurally above panel** (AC-6, the blocker fix) — **investigated,
  ESCALATED per 6.3 (see Completion Notes); AC-6 is NOT satisfied in this story's code — no
  code change was made for this task.**
  - [x] 6.1 Investigate Android window-type options for a same-owner (foreground `Service`), same-
    permission-set fix that structurally out-ranks `TYPE_APPLICATION_OVERLAY` — start from what
    the app already has: `KlarvoAccessibilityService` is already bound and running
    (`android/kotlin-src/com/klarvo/voice/KlarvoAccessibilityService.kt`), which is the usual
    prerequisite for `TYPE_ACCESSIBILITY_OVERLAY` windows (a window type android reliably z-orders
    above ordinary app overlays). Confirm whether such a window can be added from the existing
    foreground-service `WindowManager`/context, or whether ownership must move to
    `KlarvoAccessibilityService` itself (bigger change — touches how `bubbleView`/`bubbleParams`
    are created and how touch handling reaches back into `KlarvoOverlayService`). See Dev Notes.
    **Done — see Completion Notes: no same-owner path exists.**
  - [ ] 6.2 If a viable mechanism exists, implement it; document the exact API/type chosen and why
    it's order-independent in Completion Notes. **N/A — 6.1 found no viable same-owner
    mechanism; nothing to implement without the larger reparenting change 6.3 explicitly gates.**
  - [x] 6.3 If no permission-free, same-owner mechanism exists, **escalate in Completion Notes
    rather than silently building a large reparenting change** — flag for Andi/story-conductor
    decision before proceeding (this is the story's designated hardest point, expected to possibly
    need a follow-up design call). **Done — escalated, see Completion Notes.**
  - [x] 6.4 Add/extend a JVM test if a pure resolver/guard function is introduced as part of the
    fix (e.g. "does this Z-order approach depend on add-order" as a documented, testable
    invariant) — if the fix is purely a `WindowManager.LayoutParams.type`/flags change with no new
    pure logic, document why no new test applies instead of skipping silently. **No code was
    changed for Task 6 (escalated instead), so no new test applies — documented here rather than
    skipped silently.**

- [x] **Task 7 — Verify + close**
  - [x] 7.1 New JVM unit tests green (Tasks 4.4, 6.4) — confirm inversions RED empirically,
    document in Completion Notes.
  - [x] 7.2 `scripts/android-smoke.sh` clean build/install (confirm this story stayed Kotlin-only —
    no full `tauri android build` needed, unlike 11-2's Settings-touching scope).
  - [ ] 7.3 Real-device smoke per DoD — Andi's action, with the specific ➤/✗-reachability
    confirmation called out above (this is the story's actual deliverable, not a nice-to-have).
    **Not done by the dev agent — real-device smoke is Andi's action per project-context.md;
    also, AC-6 (bubble-above-panel) cannot be confirmed at GATE-4 because it was not implemented
    (see Task 6 escalation) — Andi's smoke should treat the ➤/✗-reachability check as
    EXPECTED-TO-STILL-FAIL until AC-6 is resolved.**

- [ ] **Task 8 — AC-3 rendering PIVOT: fixed-height scroll box, auto-to-newest + manual scroll** (revised AC-3, b+c; added 2026-07-08 after Andi device-test) — **THIS is the only open work for this reopen. AC-1/2/4/5 stay done; AC-3's fixed WINDOW height (Task 5) stays done + device-verified; AC-6 stays split to Story 11-4.**
  - [ ] 8.1 In `ListeningPanelView.kt`, REMOVE the rolling-window rendering added on 2026-07-07: the fixed `ROLLING_MAX_LINES` `TextView` pool, `renderRollingLines`, the `visibleLines` companion function, `ROLLING_*` constants, and the `Gravity.BOTTOM` line-container anchoring (P2). Delete the now-obsolete `RollingWindowVisibleLinesTest.kt`.
  - [ ] 8.2 Restore a single scrollable transcript surface INSIDE the fixed-height panel: a `ScrollView`/`NestedScrollView` wrapping one transcript `TextView` (the 11-2 shape), which now actually scrolls because the panel WINDOW is fixed height (`PANEL_FIXED_HEIGHT_DP`, Task 5 — do NOT revert that). Keep the caret if it still fits the scroll model; if it complicates scrolling, simplify (its exact placement was already a low-pri residual).
  - [ ] 8.3 Auto-scroll to newest (b): on each `rawTranscript`/preview append, scroll the view to the bottom (`fullScroll(View.FOCUS_DOWN)` / `smoothScrollTo`), so the most-recently-spoken text is visible by default. Preferred (device-tunable, first-pass): "stick to bottom UNLESS the user has manually scrolled up" — i.e. don't yank a user who scrolled back. A simple flag toggled on user scroll is acceptable.
  - [ ] 8.4 Manual touch-scroll (c): ensure the panel overlay window is touch-scrollable (the panel window is NOT `FLAG_NOT_TOUCHABLE`, so the ScrollView should receive drag events — verify the overlay `LayoutParams` flags don't block scroll touches; do NOT add focus that would steal the keyboard). No new appearance controls here (line-spacing setting is 11-6, out of scope).
  - [ ] 8.5 Keep `sanitizePreviewChunk` (P1) — still correct. If any pure helper remains after the pivot (e.g. a stick-to-bottom decision), add a small JVM test; otherwise no new pure-logic test is required (scroll behaviour is device-verified at GATE-4). Re-run JVM tests + Kotlin compile + `gen-android-theme.mjs --check`; commit.

## Dev Notes

### Why this got upgraded from polish to blocker (context for the "why")

11-2 shipped a passive text panel intended as pure orientation. Andi's real-device use (2026-07-02
notes, escalated 2026-07-07) found that because the panel's window is `WRAP_CONTENT`
(`KlarvoOverlayService.kt:2280-2301`) with only a *minimum* 200dp floor
(`ListeningPanelView.kt:340`) and no maximum, sustained dictation grows the panel until it fills
most of the screen — and because the panel and bubble share the same window type/flags, the
later-added panel z-orders above the bubble and can absorb/obscure the touches needed to finish or
cancel the recording. Two independent fixes converge on solving this: (a) bound the panel's growth
entirely (item 3 — no more "fills the screen"), and (b) make the bubble's on-top-ness structural,
not accidental (item 6 — so even a bounded panel can never accidentally cover live controls, now
or in any future change to add-order).

### Hardest technical point: AC-6's mechanism (item 6)

Unlike the earlier five items (mostly text/constant edits + one bounded-buffer algorithm), item 6
is genuinely open on *how*, only pinned on *what* (structural Z-band, not re-add). The two windows
today (`bubbleParams` at `KlarvoOverlayService.kt:980-991`, `panelParams` at
`KlarvoOverlayService.kt:2280-2301`) are both added via the **same foreground `Service`'s**
`windowManager` with `overlayType = TYPE_APPLICATION_OVERLAY`
(`KlarvoOverlayService.kt:525-526`). Within that single type, Android z-orders by add-order —
there is no "priority" field to set within `TYPE_APPLICATION_OVERLAY` itself. Getting the bubble
into a genuinely higher Z-band means either:
- Finding an Android window type available to a foreground `Service` (no extra runtime permission
  beyond what Klarvo already holds — `SYSTEM_ALERT_WINDOW`/`BIND_ACCESSIBILITY_SERVICE`) that
  reliably renders above `TYPE_APPLICATION_OVERLAY`, or
- Re-homing the bubble window's *ownership* to `KlarvoAccessibilityService` (already bound/running,
  `KlarvoAccessibilityService.kt`) so it can use `TYPE_ACCESSIBILITY_OVERLAY` — this is the
  standard Android mechanism for "always on top of other apps and other app overlays", but it is a
  bigger change: it means the bubble's `WindowManager.addView`/touch-listener/drag logic (currently
  all in `KlarvoOverlayService`, ~lines 980-1330) would need to either move to
  `KlarvoAccessibilityService` or have `KlarvoOverlayService` obtain a window-adding capability
  through it.
Do the investigation (Task 6.1) before committing to the reparenting path — if a lighter, same-
owner option exists, prefer it. If not, this is exactly the kind of scope growth that should be
surfaced (Task 6.3), not silently absorbed into a "small" story.

### Rolling-window "line" semantics interact with an 11-2 accepted-Low finding

11-2's code review accepted, as a Low residual, that preview chunks are joined with a single space
(`"$acc $text"`, `KlarvoOverlayService.kt:1647`) rather than one-chunk-per-line. A visually
sensible "rolling window of lines" (item 3) most likely wants one line per chunk/flush — which
means this story either revisits that accepted-Low join behavior (change to newline-join) or
implements word-wrapping within the fixed-height view and treats *wrapped* lines (not chunks) as
the eviction unit. Either is defensible; pick one, document the choice (Task 4.2), and don't treat
the 11-2 acceptance of the Low finding as a reason to avoid revisiting it here — that finding was
accepted for *accuracy* reasons (orientation surface, not accuracy), not for *this story's* display
mechanics.

### Panel window / lifecycle (context, not new — from 11-2 Dev Notes, still accurate)

`ListeningPanelView` is its own `TYPE_APPLICATION_OVERLAY` window, added/removed via
`showListeningPanel()`/`hideListeningPanel()` (`KlarvoOverlayService.kt:2262-2338`), called for
**all** recording modes' RECORDING/TRANSCRIBING states (call sites at lines 407, 416, 1551, plus
many `hideListeningPanel()` call sites for TRANSCRIBING→DONE/IDLE transitions) — not just
HOLD/TOGGLE. This story's header/footer/grip/font/fixed-height changes are therefore visible in
every recording session, whether or not `livePreviewEnabled` is on; only the *text content* itself
(accumulated preview chunks) is HOLD/TOGGLE + `livePreviewEnabled`-gated (11-2 territory,
untouched).

### No `FLAG_NOT_TOUCHABLE` — HyperOS quirk (still applies)

Per 11-2 Dev Notes and `reference_hyperos_overlay_quirks`: HyperOS/MIUI force-dims any
`TYPE_APPLICATION_OVERLAY` window carrying `FLAG_NOT_TOUCHABLE` to alpha 0.8. Neither the panel nor
the bubble carry this flag today (`KlarvoOverlayService.kt:984-985, 2296-2297`) — whatever Z-order
mechanism Task 6 lands on must not reintroduce it.

### Testing pattern to mirror for new pure Kotlin logic

`RecordingMode.selectSilenceSecs` (`KlarvoOverlayService.kt:146`) and 11-2's
`DeltaSnapshotSliceTest`/`PreviewPauseFramesTest`/`ShouldInstallPreviewFlushTest` are the templates:
pure functions, no `Context`/`WindowManager`/I/O, directly JVM-testable in
`android/kotlin-test/com/klarvo/voice/`, with an explicit inversion documented per test.

### Project Structure Notes

- Kotlin changes: `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt` (header/footer text,
  GripView removal, font scale, rolling-window rendering), `KlarvoOverlayService.kt` (panel window
  fixed height, bubble Z-order fix). Possibly `KlarvoAccessibilityService.kt` if Task 6
  investigation concludes reparenting is required.
- New/updated JVM tests: `android/kotlin-test/com/klarvo/voice/` (new file(s) for the line-
  eviction function; possibly a new file for any Z-order guard/resolver).
- **No Rust changes, no frontend/TS changes** — confirm no `.rs`/`.ts`/`.tsx` file appears in the
  File List before closing; if one does, the DoD's smoke-script coverage claim (Task 7.2) is
  wrong and a full `tauri android build` is required instead (mirrors 11-2's GATE-4 lesson about
  `android-smoke.sh` being Kotlin-only).

### References

- `docs/backlog.md` §"11-3 (Follow-up) — Android Preview-Box Geräte-Feedback-Pass" — full source of
  all 6 decisions, including the 2026-07-07 upgrade-to-blocker note and root-cause analysis.
- `_bmad-output/implementation-artifacts/11-2-android-live-preview-port.md` — the story this one
  follows up on; delta-flush/accumulator pipeline (untouched here), panel window creation, existing
  `applyAppearance`/`FONT_PX_SP` mechanism, HyperOS `FLAG_NOT_TOUCHABLE` lesson.
- `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt:35-92` (companion/statics incl.
  `FONT_PX_SP`), `263-341` (`init` — GripView, transcriptScrollView, minimumHeight), `518-553`
  (RECORDING header label), `674-704` (`FooterView`) [Source:
  android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt].
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt:525-526` (`overlayType`),
  `980-996` (`bubbleParams`), `1448-1520` (`startRecording`), `1604-1649` (`flushPreviewDelta`/
  `appendPreviewText`), `2262-2338` (`showListeningPanel`/`hideListeningPanel`) [Source:
  android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt].
- `android/kotlin-src/com/klarvo/voice/KlarvoAccessibilityService.kt` — existing bound
  accessibility service, referenced only as a Task-6 investigation starting point (no window-
  manager usage in it today, confirmed by grep) [Source:
  android/kotlin-src/com/klarvo/voice/KlarvoAccessibilityService.kt].
- `android/kotlin-test/com/klarvo/voice/RecordingModeSilenceSelectionTest.kt`,
  `DeltaSnapshotSliceTest.kt`, `ShouldApplyPreviewAppearanceTest.kt` — pure-function JVM test
  patterns to mirror.
- `_bmad-output/project-context.md` — Android real-device smoke gate, no `FLAG_NOT_TOUCHABLE` rule,
  Android build-freshness verification method.

## Dev Agent Record

### Agent Model Used

Claude (bmad-dev-story workflow, claude-in-chrome/Bash/Read/Edit toolset), 2026-07-07.

### Debug Log References

- JVM unit tests (all Android suites, `:app:testUniversalDebugUnitTest --offline`): 17 suites,
  **158/158 passing, 0 failures, 0 errors** — includes the new `RollingWindowVisibleLinesTest`
  (7/7) and confirms no regression in the 10 other suites touched indirectly by the
  `ListeningPanelView`/`KlarvoOverlayService` edits (e.g. `ResolveTranscriptColorTest`,
  `ShouldApplyPreviewAppearanceTest`, `RecordingModeSilenceSelectionTest`,
  `DeltaSnapshotSliceTest`).
- `:app:compileUniversalDebugKotlin --offline`: clean compile of the full main source set
  (proves `KlarvoOverlayService.kt`/`ListeningPanelView.kt` changes are syntactically/type
  correct beyond just the test sourceset).
- `node scripts/gen-android-theme.mjs --check`: `[ok] KlarvoTheme.kt is in sync with canon
  klarvo.css` — unaffected by this story (no theme-token changes).

### Completion Notes List

- **Tasks 1–5 (AC-1 through AC-5) fully implemented and JVM-tested where applicable.**
- **Task 4.2 join-format decision:** `KlarvoOverlayService.appendPreviewText` was changed from
  space-join (`"$acc $text"`) to **newline-join** (`"$acc\n$text"`) so each preview-flush chunk
  becomes exactly one rolling-window line (`ListeningPanelView.renderRollingLines` splits on
  `"\n"`). This revisits 11-2's accepted-Low finding on the space-join per the story's own Dev
  Notes guidance (that finding was accepted for STT *accuracy* reasons, not for this story's
  *display* mechanics).
- **Task 4.2 first-pass numeric parameters (OPEN ITEM #1, explicitly not pixel-pinned):**
  `ListeningPanelView.ROLLING_MAX_LINES = 5`, `ROLLING_FADE_MS = 280`,
  `ROLLING_FADE_ALPHA = 0.35f`; panel window `KlarvoOverlayService.PANEL_FIXED_HEIGHT_DP = 200`
  (reuses the pre-11-3 200dp floor value, now as the sole/fixed height, per the story's own
  first-pass proposal). **Flag for Andi at GATE-4** per the story's own instruction.
- **Task 4.3 rolling-window rendering approach:** implemented as a fixed pool of
  `ROLLING_MAX_LINES` `TextView`s in a vertical `LinearLayout` (`lineViews`), rebuilt on every
  `rawTranscript` update via `renderRollingLines`. The oldest visible line (flagged
  `Line.isFading` by the pure `visibleLines` function whenever older chunks exist beyond the
  window) is animated to `ROLLING_FADE_ALPHA` over `ROLLING_FADE_MS` via `TextView.animate()` —
  a first-pass "soft fade" (dim before it rolls off), not a full crossfade choreography; GATE-4
  should confirm this reads as "soft", not abrupt.
- **AC-6 / Task 6 — ESCALATED, NOT IMPLEMENTED.** Investigation (Task 6.1) confirmed via web
  research (Android platform docs/community sources on `TYPE_ACCESSIBILITY_OVERLAY`) that this
  window type — the standard mechanism for "always above other app overlays" — can only be added
  through a `WindowManager` obtained from an active `AccessibilityService`'s own `Context`;
  non-accessibility-service callers (i.e. `KlarvoOverlayService`, a plain foreground `Service`)
  cannot add such a window. There is **no lighter, same-owner, permission-free fix** — the only
  structural path is reparenting the bubble window's ownership (creation, touch-listener, drag
  logic — `KlarvoOverlayService.kt` ~980-1330) to `KlarvoAccessibilityService`. Per Task 6.3's
  explicit contract ("if no permission-free, same-owner mechanism exists, escalate ... rather
  than silently building a large reparenting change"), **this dev-story session did not build
  that reparenting change.** Reasons beyond raw size: (1) `KlarvoAccessibilityService` requires a
  user-granted Accessibility permission the app currently treats as optional/best-effort (used
  today only for keyboard detection with an existing non-accessibility fallback) — moving bubble
  *ownership* there would make the bubble itself disappear entirely for any user who hasn't
  granted that permission, a severe, unstated regression; (2) the touch/drag/edge-snap logic
  currently in `KlarvoOverlayService` (~350 lines) would need to move or gain a cross-component
  bridge, which is not safely verifiable without a real-device test loop this session doesn't
  have. **This is the story's designated hardest point (explicitly anticipated in Dev Notes) and
  is flagged here for Andi/story-conductor decision, per the story's own instruction.** AC-6
  (bubble always renders above the panel) is **NOT satisfied by this story's code** — the
  original z-order defect (panel can still cover the bubble's ➤/✗ controls) **remains
  unfixed**. Items 3+5 (fixed box, bounded content) reduce how often the panel grows large enough
  to reach the bubble, but do not structurally rule out the overlap this AC exists to fix.
- **File-List scope confirmed:** no `.rs`/`.ts`/`.tsx` file changed — this story stayed
  Kotlin-only as scoped, so `scripts/android-smoke.sh`'s lighter Kotlin-only build/install path
  (not a full `tauri android build`) is the correct DoD smoke path (Task 7.2).

### File List

- `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt` — header/footer text (AC-1/AC-2),
  `GripView` removal + padding tighten (AC-4), `FONT_PX_SP` rescale (AC-5), rolling-window
  `visibleLines`/`Line`/`renderRollingLines`/`lineViews` replacing the single
  `transcriptTextView`/`transcriptScrollView` (AC-3), `minimumHeight` floor removed (Task 5.2).
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — `PANEL_FIXED_HEIGHT_DP`
  constant + panel window `WindowManager.LayoutParams` height fixed instead of `WRAP_CONTENT`
  (AC-3a, Task 5.1), `appendPreviewText` newline-join instead of space-join (Task 4.2). No
  changes for Task 6 (escalated, not implemented — see Completion Notes).
- `android/kotlin-test/com/klarvo/voice/RollingWindowVisibleLinesTest.kt` (new) — JVM unit tests
  for `ListeningPanelView.visibleLines` (Task 4.4): capacity bound, oldest-first eviction,
  fading-flag correctness, and a documented inversion (unbounded buffer fails the bound).

## Change Log

| Date | Change |
|------|--------|
| 2026-07-07 | Story created (bmad-create-story) from `docs/backlog.md` §11-3, upgraded 2026-07-07 from polish pass to usability-blocker fix (root-caused: WRAP_CONTENT panel window + shared window-type add-order z-ordering). Status: ready-for-dev. Two items flagged as not-fully-pinned open design questions (rolling-window exact parameters; bubble Z-order fix mechanism) — see "OPEN ITEMS" section and elicitation report. |
| 2026-07-07 | bmad-dev-story: implemented Tasks 1–5 (AC-1 through AC-5) — header/footer rename, GripView removal, font rescale, fixed-size rolling-window transcript (new `visibleLines` pure function + JVM tests), panel window fixed height. **Task 6 (AC-6, bubble-above-panel) investigated and ESCALATED per its own 6.3 contract — not implemented; no same-owner/permission-free fix exists (see Completion Notes).** 158/158 JVM unit tests green, clean Kotlin compile, Kotlin-only file list confirmed. Status → review. **AC-6 remains unmet — flagged for Andi/story-conductor decision before this story can be considered done.** |
| 2026-07-07 | **GATE-2 decision (Andi): AC-6 SPLIT OUT to a focused follow-up story (`11-4`, `docs/backlog.md`). This story's scope is now AC-1..AC-5** — the fixed-height rolling window already removes the catastrophic bug (panel filling the screen → device unusable). The narrower "does the fixed panel still overlap the bubble controls" question + the z-order fix are deferred to 11-4, which is to try a positioning fix BEFORE the a11y-reparenting per Andi's preference. |
| 2026-07-07 | **code-review (bmad-code-review, 3 adversarial reviewers: Blind / Edge / Auditor) on `88bc48c..2a655ec`.** AC-1..AC-5 confirmed satisfied; AC-6 correctly noted as intentionally deferred. 4 findings confirmed + fixed in one round (commit `152ed10`): P1 sanitize incoming preview chunk (embedded-newline collapse + blank-chunk drop, new pure `sanitizePreviewChunk` + JVM test), P2 bottom-anchor rolling-lines container so overflow clips oldest-at-top (newest always visible, font-independent), P4 epsilon alpha-compare. Residual to Andi's real-device GATE-4 (emulator cannot judge pixels): fade soft-vs-abrupt feel + final line-count/height at real dictation pacing (first-pass, tunable). **163/163 JVM tests green, clean compile, KlarvoTheme in sync.** Status stays `review` pending Andi's real-device visual gate. |
| 2026-07-08 | **Andi real-device test (fresh APK pushed via Tailscale adb): fixed height CONFIRMED ✅ — the blocker is solved.** But two follow-ups reopen the story: **(b) the box shows the BEGINNING, not the newest-spoken text, and does not follow the transcript** (root: long real chunks wrap top-aligned in the "line-per-chunk" rolling model), and **(c) Andi wants manual touch-scroll** through longer preview text. **Decision (Andi): AC-3 rendering PIVOTS** — keep the fixed height, replace the rolling-window with a bounded scroll view that auto-scrolls to the newest text + is manually scrollable (this reverses the 2026-07-02 "no scroll" call, now viable because the window height is fixed). Status → `in-progress`; fresh dev round for revised AC-3. Also captured (separate stories, NOT this cycle): **11-4** intent clarified = true Z-LAYERING wanted (bubble layer-above panel, may overlap — not the current geometric-repositioning trick); **11-6** new = line-spacing as an Appearance setting. Sequence: b+c (this story) first, then 11-4. |
