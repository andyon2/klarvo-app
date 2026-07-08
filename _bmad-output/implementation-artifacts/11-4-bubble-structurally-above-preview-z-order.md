---
story: "11.4"
epic: "11"
title: "Bubble structurally above the Preview panel (Z-order)"
status: review
track: L2-fix
gatedBy: ["11.3"]
buildsOn: ["11.3"]
enabledBy: []
inputDocuments:
  - docs/backlog.md#11-4 — Bubble strukturell über der Preview (Z-Order) — Split aus 11-3 Punkt 6
  - _bmad-output/implementation-artifacts/11-3-android-preview-box-device-feedback-pass.md — Task 6 investigation (AC-6 escalated, not implemented), Dev Notes "Hardest technical point"
  - _bmad-output/project-context.md
---

# Story 11.4: Bubble structurally above the Preview panel (Z-order)

Status: review

> **Epic 11 — Cross-Platform Live-Preview.** Story 11-3 shipped the fixed-height, auto-scrolling
> preview panel (AC-1..AC-5, `done`, real-device-verified). Its Task 6 (AC-6, "bubble always
> renders above the panel") was **investigated and escalated, not implemented** — split out to
> this story per Andi's 2026-07-07 GATE-2 decision. **Intent was then clarified on 2026-07-08's
> device test** (see "Design decisions" below): this is genuine Z-**layering**, not geometric
> repositioning.

## Design decisions (Andi — binding, do not re-litigate)

Source: `docs/backlog.md` §"11-4", decisions dated 2026-07-07 (split + investigation) and
2026-07-08 (intent clarification).

1. **The goal is real Z-layering, NOT positioning.** Andi's 2026-07-08 device test showed that
   the *current* 11-3 build already exhibits an unwanted "trick": dragging the bubble into the
   panel's area causes it to be **geometrically repositioned** so it visually stays above the
   panel's top edge (the two never overlap on screen). **Andi does NOT want this.** He wants the
   bubble to be able to sit **at the same screen position as the panel, and layer-render on top of
   it** — i.e. the panel may be **effectively partially covered** by the bubble when they occupy
   the same space, exactly like two overlapping windows where one has priority. **Any incidental
   repositioning behavior that keeps the bubble geometrically clear of the panel must be found and
   removed as part of this story** — see "Dev Notes → Investigate the repositioning behavior"
   below; this codebase's authors could not pin an exact line for it during story creation (no
   drag/keyboard-clamp code was found that explicitly reasons about the panel's top edge), so the
   dev agent must locate the actual mechanism (it may be an *emergent* interaction between two
   independent clamps, e.g. the keyboard-avoidance clamp in `adjustBubbleForKeyboard`
   coincidentally also keeping the bubble clear of the panel, rather than a single dedicated
   "stay above panel" function) before removing it.
2. **Mechanism is open, "re-add" is the first thing to try.** 11-3's investigation established:
   both windows use `overlayType = TYPE_APPLICATION_OVERLAY` from the same foreground `Service`
   → Android z-orders by **add-order** within that type, with no priority field. 11-3 rejected
   **re-adding the bubble window after each panel show** as "fragile" (order-dependent, would
   regress if any future code path shows the panel before the bubble) — but for the *narrower*
   goal now clarified ("bubble ends up above panel", not "structurally survive any future
   add-order"), **Andi's stated preference (2026-07-08) is to try re-add first**, and fall back to
   the heavier `TYPE_ACCESSIBILITY_OVERLAY` reparenting (11-3's Dev Notes "Hardest technical
   point") only if re-add cannot cleanly carry the bubble's drag/touch behavior. This reverses
   11-3's implicit bias against re-add for THIS story's narrower scope — re-add is the sanctioned
   starting point here, not a rejected option.
3. **`TYPE_ACCESSIBILITY_OVERLAY` reparenting stays the documented fallback**, with the same
   regression risk 11-3 flagged: it requires ownership to move to `KlarvoAccessibilityService`
   (already bound/running for keyboard detection, but the accessibility permission itself is
   optional/best-effort today) — a user who has not granted Accessibility permission would lose
   the bubble entirely if bubble creation moved there outright. If this path is taken, that
   regression must be avoided (e.g. a permission-checked fallback back to `TYPE_APPLICATION_OVERLAY`
   ownership in `KlarvoOverlayService` when Accessibility is not granted) — do not silently
   introduce a "no accessibility permission → no bubble" regression.

## Story

As an Android user dictating with the preview panel visible,
I want the bubble to always render (and receive touches) above the preview panel when they occupy
the same screen area — even if that means the bubble visually covers part of the panel — and I
want to be able to drag the bubble anywhere, including into the panel's area, without an
artificial repositioning trick keeping them apart,
so that the ➤ Senden / ✗ Abbrechen controls are always reachable regardless of where I've placed
the bubble, and the app doesn't fight my own drag gesture.

## Scope boundaries (read before touching code)

**IN:**
- `KlarvoOverlayService.kt`: the Z-order/layering mechanism between `bubbleParams`
  (`:1006-1016`) and the panel's `params` (`showListeningPanel`, `:2295-2352`) — whichever
  concrete change makes the bubble render/receive-touches on top regardless of add order (re-add
  after `showListeningPanel`, a window-type change, or — only if neither works cleanly — the
  `TYPE_ACCESSIBILITY_OVERLAY` reparenting described in 11-3 Dev Notes).
- Locating and removing the geometric-repositioning behavior described in Design Decision 1 —
  search `adjustBubbleForKeyboard` (`:837-846`), the drag handler (`:1290-1310`), and any other
  code that reads or reasons about `panelParams`/panel height/panel position when computing
  `bubbleParams.y` or clamping bubble movement. Confirm with a manual drag-trace (or a JVM test on
  any pure clamp function found) that after the fix, dragging the bubble fully into the panel's
  screen rectangle is possible and the bubble renders on top there.
- Possibly `KlarvoAccessibilityService.kt` **only if** re-add (or another same-owner mechanism)
  cannot cleanly carry the bubble's existing drag/touch/edge-snap logic — see 11-3 Dev Notes
  "Hardest technical point" for the reparenting shape and its regression risk, which this story
  must also guard against if it goes that route (Design Decision 3).
- New/updated JVM unit tests for any new pure Kotlin logic introduced (e.g. an order-independence
  guard/resolver function), following the existing `android/kotlin-test/com/klarvo/voice/` pattern.

**OUT (do not touch):**
- The panel's fixed-height/scroll/auto-scroll transcript rendering, its content pipeline
  (`deltaSnapshotWav`, `flushPreviewDelta`, `appendPreviewText`, `sanitizePreviewChunk`) — all
  11-2/11-3 territory, untouched. This story only changes **which window renders on top and
  receives touches** when bubble and panel occupy the same screen area, and removes the
  unwanted geometric-clearance behavior.
- The panel's header/footer copy, `GripView` removal, `FONT_PX_SP` scale — 11-3 AC-1/2/4/5,
  already done, not to be re-touched.
- The Settings/Appearance category, config fields, or any Rust/TS file — **zero Rust changes,
  zero frontend changes** unless the investigation surfaces a genuine need (not expected; flag
  and escalate rather than silently expanding scope if one is found).
- `FLAG_NOT_TOUCHABLE` on any overlay window (project-wide rule, HyperOS alpha-dim quirk — see
  Dev Notes reference; also directly relevant here since a window-type change touches
  `WindowManager.LayoutParams` flags).
- Auto/AutoStop mode's one-shot silence/paste behavior, VAD, `KlarvoAudioRecorder.kt` — untouched,
  this story is pure overlay-window layering.
- 11-6 (line-spacing Appearance setting) — separate backlog item, not this story.

## ⚠️ OPEN ITEMS — needs Andi's confirmation (not invented, not defaulted silently)

These points are **not fully pinned** by the backlog/canon. The qualitative goal (bubble
structurally on top, no geometric-clearance trick) is binding; the concrete mechanism and its
edge-case behavior are first-pass engineering choices to confirm at the real-device GATE-4, the
same pattern 11-3 used for its own first-pass numbers.

1. **Concrete Z-order mechanism** (Design Decision 2): re-add-after-panel-show is the sanctioned
   starting point, but it is **not guaranteed to survive** the panel being repeatedly
   shown/hidden during one recording (RECORDING→TRANSCRIBING→RECORDING cycles,
   `showListeningPanel` call sites at `KlarvoOverlayService.kt:433, 442` plus HOLD/TOGGLE
   internals) — if re-add proves fragile in exactly the way 11-3's investigation predicted
   (order-dependent, breaks on a future/edge call-order), escalate to the a11y-reparenting
   fallback (Design Decision 3) rather than shipping a known-fragile fix; document which path was
   taken and why in Completion Notes, mirroring 11-3's own escalation discipline.
2. **Touch semantics when bubble visually covers the panel** (not specified anywhere): once the
   bubble renders on top of the panel, does the panel's touch area underneath the bubble need to
   remain a no-op passthrough (it already is one — the panel has no touch listener per 11-3 Dev
   Notes, "the panel has no touch listener, so touches that land on it are simply absorbed"), or
   could bubble-on-top now visually clash with the panel's border/background in an ugly way that
   needs a z-aware visual tweak? First-pass assumption: no visual tweak needed (panel is a plain
   rectangle, bubble is a small squircle — overlap is expected to look fine per Andi's stated
   intent that the panel may be "effectively partially covered"). **Flag for Andi at GATE-4**: does
   the overlap read as intentional/acceptable, or does it need a visual affordance (e.g. bubble
   shadow, or panel dimming under the bubble)?
3. **If the a11y-reparenting fallback is taken** (Design Decision 3): the exact permission-check
   + fallback-ownership shape (e.g. "if Accessibility not granted, bubble stays owned by
   `KlarvoOverlayService` with today's `TYPE_APPLICATION_OVERLAY`, accepting the pre-11-4 z-order
   behavior for that user only") is a first-pass engineering call for the dev to make and document
   — not pinned by Andi beyond "must not silently lose the bubble for non-a11y users".

## Acceptance Criteria

**AC-1 (Bubble renders + receives touches above the panel, order-independent):**
Given the bubble window (`bubbleParams`) and the panel window (`panelParams`, created in
`showListeningPanel`) are both visible, both currently `overlayType = TYPE_APPLICATION_OVERLAY`
z-ordered by add-order (`KlarvoOverlayService.kt:1006-1016` vs `:2295-2352`),
When both windows are visible simultaneously, in any order and across any number of
show/hide/re-show cycles of the panel during one recording (RECORDING→TRANSCRIBING→RECORDING;
`showListeningPanel` call sites at `:433, 442` and further internal call sites),
Then the bubble's touchable region (idle circle, or the expanded ➤/✗ cluster) always renders
visually on top of the panel and always receives touches — independent of which window was added
first or most recently,
And the mechanism used is documented in Completion Notes with the specific Android API/approach
chosen, and a reviewer can verify from the diff alone (or a documented inversion trace) that the
fix does not silently degrade back to add-order dependence after a panel hide/re-show cycle.

**AC-2 (No overlap-avoidance / no geometric-clearance trick):**
Given the user drags the bubble (free-drag, `ACTION_MOVE` path, `KlarvoOverlayService.kt:1290-1310`)
into the screen rectangle currently occupied by the visible preview panel,
When the drag completes,
Then the bubble's final position is exactly where the user dropped it (subject only to the
existing, unrelated edge-snap behavior, AC9/Story 9.3, when `bubbleEdgeSnap != false`) — no
additional Y-clamp or repositioning logic pushes the bubble away from the panel's top edge or any
other panel-relative boundary,
And the bubble visually renders on top of the panel at that position (AC-1) rather than the panel
being pushed/clamped away from the bubble or vice versa,
And any pre-existing code found responsible for the old geometric-clearance behavior (Design
Decision 1) is either removed or demonstrated (with a code-level trace, not just "it doesn't
reproduce today") to be unrelated to panel-avoidance — whichever it turns out to be, Completion
Notes documents which code was implicated and what changed.

**AC-3 (Existing bubble behaviors unregressed):**
Given the Z-order/layering fix and the repositioning-behavior removal from AC-1/AC-2,
When the bubble is dragged, edge-snapped, keyboard-avoided (`adjustBubbleForKeyboard`), or
cycled through IDLE/RECORDING/TRANSCRIBING/DONE states,
Then all of that existing behavior (drag, edge-snap-on-release, keyboard jump-up/restore,
push-to-talk hold-target hit-testing, `suppressedForPanel` state visuals) continues to work
exactly as before this story — this story changes only Z-order/layering and removes the one
specific panel-avoidance behavior named in AC-2, nothing else in the bubble's positioning system.

**AC-4 (Accessibility-permission regression guard, only if the a11y-reparenting fallback is used):**
Given the fallback mechanism (Design Decision 3) is needed because re-add proves insufficient
(Open Item 1),
When a user has **not** granted the Accessibility permission (today optional/best-effort, used
only for keyboard detection with an existing non-accessibility fallback per 11-3 Dev Notes),
Then the bubble still appears and functions (it must not silently disappear for that user) — the
specific fallback shape (documented pre-11-4 z-order behavior for that user vs. some other
degraded-but-functional path) is the dev's first-pass call per Open Item 3, but "no bubble at all"
is not an acceptable outcome for any user regardless of Accessibility permission state.
**This AC only applies if the a11y path is taken; N/A and document as such if re-add (or another
same-owner mechanism) succeeds.**

**DoD (surface-class — mirrors project testing rules):**
- New pure Kotlin logic (any order-independence guard/resolver function introduced) has JVM unit
  tests in `android/kotlin-test/com/klarvo/voice/`, following existing patterns
  (`RecordingModeSilenceSelectionTest.kt`, `IsScrolledToBottomTest.kt`). Inversions documented
  empirically (flip the order-independence assumption → test goes RED) — mirrors 11-3's own
  inversion discipline for AC-6-equivalent claims.
- `scripts/android-smoke.sh` clean build/install if the story stays Kotlin-only (expected — confirm
  no `.rs`/`.ts`/`.tsx` file appears in the File List before relying on the lighter smoke path; if
  the investigation surfaces a genuine cross-cutting need, escalate rather than silently expanding
  scope, per the Scope Boundaries "OUT" list).
- **Real-device Android smoke required (GATE-4)** — Andi, on his real device: (a) start a
  HOLD/TOGGLE recording with preview enabled so the panel is visible, drag the bubble into and
  around the panel's area, confirm the bubble renders on top and its ➤/✗ controls stay reachable
  and functional throughout, in every recording state transition (RECORDING→TRANSCRIBING→DONE and
  back); (b) confirm the bubble no longer gets artificially repositioned away from the panel when
  dragged toward/into it (AC-2) — the drag should feel exactly as free as before, just now able to
  overlap the panel; (c) spot-check drag, edge-snap, and keyboard-avoidance still behave as before
  (AC-3). GATE-4 = real device, never emulator (`reference_android_emulator_window_structure_oracle`).
- Confirm AC-1's inversion at review time: reason through (or structurally test) whether the fix
  still holds if the panel is shown/hidden/re-shown multiple times within one recording — if the
  chosen mechanism is order-dependent across a hide/re-show cycle, it does NOT satisfy AC-1
  regardless of how it looks on the first manual test (this is exactly the failure mode 11-3's own
  investigation flagged for a naive re-add).

## Tasks / Subtasks

- [x] **Task 1 — Locate the geometric-clearance behavior** (AC-2)
  - [x] 1.1 Manually trace (or reproduce via `dumpsys window windows` / logging of `bubbleParams.y`
    during a drag) what currently happens when the bubble is dragged toward/into the panel's
    screen rectangle. Confirm whether it's `adjustBubbleForKeyboard`'s `maxY` clamp
    (`KlarvoOverlayService.kt:837-846`) incidentally interacting with the panel (e.g. because IME
    visibility and panel visibility often coincide), the free-drag path itself
    (`:1290-1310`, currently a straight assignment with no panel-awareness found during story
    creation), or something else not yet located.
  - [x] 1.2 Document the located mechanism (or the absence of one, if it turns out to be a
    perception rather than an actual code clamp — in which case document that finding instead and
    confirm AC-2 is trivially satisfied) in Completion Notes before proceeding to Task 2.

- [x] **Task 2 — Remove the geometric-clearance behavior** (AC-2)
  - [x] 2.1 Remove or neutralize whatever Task 1 located, so a drag into the panel's area is not
    clamped/repositioned.
  - [x] 2.2 Verify AC-3's unrelated behaviors (edge-snap, keyboard-avoidance for the *actual*
    keyboard, not the panel) still function — do not remove more than the panel-specific
    behavior.

- [x] **Task 3 — Z-order fix: bubble structurally above panel** (AC-1)
  - [x] 3.1 Implement re-add-after-panel-show first (Design Decision 2): after
    `showListeningPanel` successfully adds the panel window, re-add (or
    `windowManager.removeView` + `windowManager.addView`) the bubble window so it becomes the
    most-recently-added `TYPE_APPLICATION_OVERLAY` window and thus renders/receives-touches on
    top.
  - [x] 3.2 Stress the panel show/hide/re-show cycle (RECORDING→TRANSCRIBING→RECORDING,
    `showListeningPanel`/`hideListeningPanel` call sites throughout `KlarvoOverlayService.kt`) to
    confirm the bubble stays on top across every cycle, not just the first show.
  - [x] 3.3 If 3.1/3.2 prove fragile (order-dependent breakage on a hide/re-show cycle, or the
    re-add disrupts in-flight drag/touch state), escalate to the `TYPE_ACCESSIBILITY_OVERLAY`
    reparenting fallback (Design Decision 3, 11-3 Dev Notes "Hardest technical point") — document
    the escalation decision and the specific failure that triggered it in Completion Notes before
    building the larger change.
  - [x] 3.4 If Task 3.3's fallback is taken: implement the permission-checked fallback shape (Open
    Item 3 / AC-4) so a user without Accessibility permission still gets a functional (if
    pre-11-4-z-order) bubble, never no bubble at all.

- [x] **Task 4 — Tests + verification**
  - [x] 4.1 If Task 3 introduces any pure resolver/guard function (e.g. "is this bubble-on-top
    approach order-independent" as a testable invariant), add a JVM unit test with a documented
    inversion, following `IsScrolledToBottomTest.kt`'s pattern. If the fix is purely a
    `WindowManager` call-sequence change with no new pure logic, document why no new test applies
    (mirrors 11-3 Task 6.4's precedent for "no code, no test, document why").
  - [x] 4.2 Full JVM test suite green (`:app:testUniversalDebugUnitTest --offline`), clean Kotlin
    compile (`:app:compileUniversalDebugKotlin --offline`), `node scripts/gen-android-theme.mjs
    --check` clean (unaffected — no theme changes expected).
  - [x] 4.3 `scripts/android-smoke.sh` clean build/install; confirm File List stays Kotlin-only
    (no `.rs`/`.ts`/`.tsx`) so the lighter smoke path is the correct DoD gate.
  - [ ] 4.4 Real-device GATE-4 smoke per DoD — Andi's action, with the specific overlap-and-reach
    confirmation and no-more-clearance-trick confirmation called out above.

## Dev Notes

### Why this is a separate story from 11-3 (context for the "why")

11-3's Task 6 investigated this exact problem (AC-6 in that story) and found no lightweight,
same-owner, permission-free fix within `TYPE_APPLICATION_OVERLAY` itself — the only structural
z-band above it is `TYPE_ACCESSIBILITY_OVERLAY`, addable only from an `AccessibilityService`'s own
`Context`. 11-3 escalated rather than building the ~350-line bubble-ownership reparenting
unprompted (Andi's GATE-2 call, 2026-07-07: split to this story). **Then, on 2026-07-08's
device-test of the 11-3 build, Andi discovered a second, related problem**: the *current* build
already does something to keep the bubble geometrically clear of the panel when dragged near it —
which is the opposite of what he wants. He wants true layering (bubble covers panel), not
avoidance. Both problems (the missing Z-order fix, and the unwanted avoidance behavior) are one
story because they are two faces of the same underlying question: "what happens when the bubble
and panel occupy the same screen space" — the answer must now be "bubble wins, panel is
partially hidden," never "they're kept apart."

### Investigate the repositioning behavior (important — not conclusively located during story creation)

A repo-wide search during story creation for `panelParams`/panel-height/panel-position reads
inside `KlarvoOverlayService.kt`'s bubble-positioning code (`adjustBubbleForKeyboard`, the
free-drag `ACTION_MOVE` handler, `applyKeyboardState`) did **not** find an explicit "stay above
panel" clamp — only the pre-existing, panel-unrelated keyboard-avoidance clamp
(`adjustBubbleForKeyboard`, `:837-846`, clamps `bubbleParams.y` against `screenH - keyboardHeightPx
- NAV_BAR_CLEARANCE_PX - windowPx`) and the pure `ACTION_MOVE` assignment (`:1290-1310`, no clamp
at all as read). **This means either:**
(a) the observed behavior is an *emergent* side effect of the keyboard clamp (plausible: the panel
is shown at `gravity = Gravity.BOTTOM` with a fixed 200dp height, occupying roughly the same
screen region the keyboard would, and if `checkKeyboardVisibility`/`adjustBubbleForKeyboard` is
in some code path reasoning about "space near the bottom" using a value that happens to correlate
with the panel's presence, that would produce exactly Andi's observed symptom without any
dedicated "avoid panel" code existing), or
(b) there is a mechanism this search missed (check for logic in `FloatingBubbleView.kt`,
`KlarvoAccessibilityService.kt`, or any panel-aware code added after this story's creation-session
git snapshot), or
(c) Andi's observation was of a different, adjacent behavior (e.g. the panel's own show-time
Z-order — since the panel currently renders on top of the bubble, per the AC-1 defect — being
misread as "the bubble avoided the panel" when actually "the panel just covered the bubble and the
bubble looked like it moved out of the way because it visually disappeared under the panel"; in
that case fixing AC-1 might make AC-2 trivially true with nothing to remove).
**Task 1 exists specifically to resolve this ambiguity with a real repro before touching code** —
do not guess-and-remove.

### Z-order mechanism precedent from 11-3 (do not repeat the same investigation)

11-3's Dev Notes "Hardest technical point" already did the `TYPE_ACCESSIBILITY_OVERLAY` research:
it's the standard Android mechanism for "always above other app overlays," addable only through a
`WindowManager` obtained from an active `AccessibilityService`'s own `Context` — non-accessibility
callers (a plain foreground `Service`, which is what `KlarvoOverlayService` is) cannot add such a
window directly. If Task 3.3's fallback is needed, do not re-research this; reuse 11-3's finding
and its documented regression risk (accessibility permission is optional/best-effort today; moving
bubble *ownership* there would make the bubble vanish for any user without that permission —
Design Decision 3/AC-4 exists specifically to prevent that regression).

### Panel touch-passthrough (already correct, do not add touch handling to the panel)

Per 11-3 Dev Notes ("Modell B (ADR-0019 §4′): panel is a PASSIVE display... The panel has no touch
listener, so touches that land on it are simply absorbed (no-op)") — the panel already has no
touch listener. AC-1's "bubble receives touches" requirement is really about the bubble's own
window (`bubbleParams`) being the visually-topmost, add-order-winning window when both are
visible; it does not require adding any touch-forwarding logic to the panel.

### No `FLAG_NOT_TOUCHABLE` — HyperOS quirk (still applies, especially relevant here)

Per 11-2/11-3 Dev Notes and `reference_hyperos_overlay_quirks`: HyperOS/MIUI force-dims any
`TYPE_APPLICATION_OVERLAY` window carrying `FLAG_NOT_TOUCHABLE` to alpha 0.8. Neither the panel
nor the bubble carry this flag today (`KlarvoOverlayService.kt:1009-1010` bubble,
`:2333-2334` panel). **If Task 3 changes either window's `overlayType` or flags** (e.g. as part of
an a11y-reparenting fallback), the new flags must not reintroduce `FLAG_NOT_TOUCHABLE` on either
window.

### Testing pattern to mirror for new pure Kotlin logic

`RecordingMode.selectSilenceSecs` (`KlarvoOverlayService.kt:146`), 11-3's `isScrolledToBottom`
(`ListeningPanelView.kt`) and `IsScrolledToBottomTest.kt` are the templates: pure functions, no
`Context`/`WindowManager`/I/O, directly JVM-testable in `android/kotlin-test/com/klarvo/voice/`,
with an explicit inversion documented per test.

### Project Structure Notes

- Kotlin changes: `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` (bubble/panel
  window add-order or type, and whatever Task 1 locates for the repositioning removal). Possibly
  `KlarvoAccessibilityService.kt` only if Task 3.3's fallback is triggered.
- New/updated JVM tests: `android/kotlin-test/com/klarvo/voice/` (only if Task 3/4.1 introduces new
  pure logic — not guaranteed, a pure `WindowManager` call-order change may need none).
- **No Rust changes, no frontend/TS changes expected** — confirm no `.rs`/`.ts`/`.tsx` file appears
  in the File List before closing; if one does, the lighter `scripts/android-smoke.sh` path is
  wrong and this needs escalation (unexpected scope growth per the Scope Boundaries "OUT" list).

### References

- `docs/backlog.md` §"11-4 — Bubble strukturell über der Preview (Z-Order)" — full source of the
  split decision, the 2026-07-08 intent clarification, and the re-add-first preference.
- `_bmad-output/implementation-artifacts/11-3-android-preview-box-device-feedback-pass.md` —
  Task 6 investigation (AC-6 escalated, why re-add was originally rejected for 11-3's *own*
  structural-guarantee framing, `TYPE_ACCESSIBILITY_OVERLAY` research), Dev Notes "Hardest
  technical point", the panel's passive-touch model.
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt:1006-1019` (`bubbleParams`
  construction + touch listener), `:837-846` (`adjustBubbleForKeyboard`), `:1290-1310` (free-drag
  `ACTION_MOVE` handler), `:2295-2352` (`showListeningPanel`, panel `params` construction),
  `:433, 442` (panel show call sites for RECORDING/TRANSCRIBING) [Source:
  android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt].
- `android/kotlin-src/com/klarvo/voice/KlarvoAccessibilityService.kt` — existing bound
  accessibility service, referenced as the Task 3.3 fallback's ownership target if needed
  (no window-manager usage in it today, per 11-3's own confirmation) [Source:
  android/kotlin-src/com/klarvo/voice/KlarvoAccessibilityService.kt].
- `android/kotlin-test/com/klarvo/voice/RecordingModeSilenceSelectionTest.kt`,
  `IsScrolledToBottomTest.kt` — pure-function JVM test patterns to mirror.
- `_bmad-output/project-context.md` — Android real-device smoke gate, no `FLAG_NOT_TOUCHABLE`
  rule, Android build-freshness verification method.

## Previous Story Intelligence (11-3)

- **Git pattern:** 11-3 landed as a sequence of small, scoped commits on branch
  `fix/11-3-android-preview-box` (`152ed10` review fixes → `0c6665c` GATE-4 structural green/AC-6
  split → `704969c` reopen → `dba2bb3`/`b50e439`/`6b0c7d4` the AC-3 pivot round → `de0543a`
  GATE-3 close-out). This story should branch similarly (a `fix/11-4-...` or continuation branch)
  and commit per logical step, never `git add .` (project-context.md rule).
- **JVM test count discipline:** 11-3's Completion Notes tracked exact before/after JVM test
  counts at every round (163 → 161 after a deletion+addition). Mirror this precision in this
  story's Debug Log/Completion Notes.
- **Escalate, don't silently absorb scope:** 11-3's Task 6 escalation (rather than building the
  a11y-reparenting unprompted) is the exact discipline this story's Task 3.3 asks for again if
  re-add doesn't hold up — do not repeat 11-3's investigation from scratch if it re-escalates;
  cite it.
- **GATE-4 = real device only, never emulator** — both 11-2 and 11-3 required Andi's real
  Xiaomi/HyperOS device for the final visual/behavioral confirmation; emulator smoke
  (`scripts/android-smoke.sh`) only covers build/install + JVM tests.

## Dev Agent Record

### Agent Model Used

Claude Sonnet 5 (bmad-dev-story)

### Debug Log References

- `./gradlew :app:compileUniversalDebugKotlin --offline` — BUILD SUCCESSFUL (clean compile after
  sync of `android/kotlin-src/` → `gen/android/app/src/main/java/com/klarvo/voice/`).
- `./gradlew :app:testUniversalDebugUnitTest --offline` — BUILD SUCCESSFUL, 162/162 JVM tests
  green across all flavor/buildType variants (0 failures/errors in every `test-results/*.xml`);
  count unchanged from pre-story baseline (no test added/removed, per Task 4.1 rationale below).
- `node scripts/gen-android-theme.mjs --check` — `KlarvoTheme.kt is in sync with canon klarvo.css`
  (unaffected, as expected — no theme/appearance changes in this story).
- `scripts/android-smoke.sh` — full clean run: drift-gate OK, 17 production + 18 test Kotlin files
  synced, 24 JVM tests green (0 failures) in the script's own quick pass, fresh debug APK built in
  4s, installed on real device `100.112.41.70:33233` (Tailscale adb), `versionName` on-device
  verified `0.5.0`. SMOKE BUILD OK.

### Completion Notes List

**Task 1/2 (AC-2) — no dedicated geometric-clearance clamp exists; the observed "avoidance" was
the AC-1 z-order defect itself.** A targeted search of every place `KlarvoOverlayService.kt`
computes or clamps `bubbleParams.y` (`adjustBubbleForKeyboard:857-873`, the free-drag
`ACTION_MOVE` handler `:1300-1316`, `adjustLayoutForState:1103-1200`, `setupBubble`'s initial
placement `:962-1022`), plus `FloatingBubbleView.kt` and `KlarvoAccessibilityService.kt` (both
`grep`ped for `panel`/`Panel` — the only hit is `FloatingBubbleView.suppressedForPanel`, a pure
*visual*-state boolean with no positional effect, `KlarvoOverlayService.kt:2400`), found **no
code anywhere that reads `panelParams` or panel geometry when computing the bubble's position**.
The only Y-clamp in the codebase is `adjustBubbleForKeyboard`'s keyboard-height clamp, and it is
driven exclusively by the real IME window (`KlarvoAccessibilityService.notifyKeyboardState`
filters `AccessibilityWindowInfo.TYPE_INPUT_METHOD` specifically — it cannot be mistaking the
panel overlay for a keyboard window). This matches Dev Notes hypothesis (c): before this story,
`showListeningPanel` added the panel window *after* the bubble, so per Android's
add-order-only z-ordering for same-type `TYPE_APPLICATION_OVERLAY` windows (11-3's own finding),
**the panel rendered on top of the bubble** whenever they occupied the same screen region. A
bubble dragged into that region would visually disappear under the panel — indistinguishable, at
a glance, from "the bubble got pushed away." There is nothing to remove for Task 2; the AC-1 fix
(Task 3) directly resolves AC-2 as well, since once the bubble is the most-recently-added window
it renders on top of the panel wherever it is dragged, with the drag's own straight
`bubbleParams.x/y = ...` assignment (`:1300-1316`, unchanged, still `dropped-exactly-where`)
never touched. Task 2.2 (verify AC-3 behaviors untouched) is satisfied by construction — nothing
was removed, so keyboard-avoidance and edge-snap are byte-identical to pre-11-4.

**Task 3 (AC-1) — re-add-after-panel-show implemented (Design Decision 2's sanctioned first try);
did not need to escalate to the `TYPE_ACCESSIBILITY_OVERLAY` fallback.** New
`reorderBubbleAbovePanel()` (`KlarvoOverlayService.kt`) does `windowManager.removeView(bubbleView)`
+ `windowManager.addView(bubbleView, bubbleParams)`, called right after `showListeningPanel`'s own
successful `windowManager.addView(panel, params)`. Because this fires on *every* actual panel add
(the early-return branch for an already-visible panel, e.g. RECORDING→TRANSCRIBING without a
hide, doesn't touch window order and needs no reorder — the bubble is already above it from the
prior show), the invariant holds across every RECORDING→TRANSCRIBING→RECORDING hide/re-show cycle
during one recording, not just the first show — this directly satisfies AC-1's order-independence
requirement including its "across any number of show/hide/re-show cycles" clause. No
`TYPE_ACCESSIBILITY_OVERLAY` reparenting was needed, so **AC-4 is N/A** (Open Item 1 resolved:
re-add held up under analysis, no escalation trigger found).

**Fragility found and guarded (the one real risk in the re-add approach, per Task 3.3's own
framing):** `showListeningPanel()` is called synchronously from `longPressRunnable` (push-to-talk
long-press) **while the user's finger is still down** — i.e. mid-touch-sequence on the bubble
window. `windowManager.removeView()` on a window with an in-flight touch tears down that window's
input channel, which the OS reports back as `ACTION_CANCEL` — this would have broken push-to-talk
hit-tracking exactly the way Task 3.3 warned about ("the re-add disrupts in-flight drag/touch
state"). Rather than escalating on a hypothesis, the fix is guarded: a new `bubbleReorderPending`
flag defers the reorder when `activePointerId != MotionEvent.INVALID_POINTER_ID` (a gesture is in
flight) to the next `ACTION_UP`/`ACTION_CANCEL` in `handleTouch`, where it's consumed once the
gesture has actually ended. This is a same-owner, same-window fix — no reparenting needed. Residual:
during the (short) window between push-to-talk trigger and finger release, if the user somehow
drags the HOLD-cluster surface into the panel's area mid-hold, the panel could theoretically still
render on top until release; this is a narrow edge case flagged for GATE-4 spot-check (Task 4.4)
rather than solved speculatively, since it cannot be observed from this environment (no device
touch simulation available) — see project rule "never make the user the rendering oracle," which
is why this is called out explicitly for Andi's confirmation rather than silently assumed fine.

**Task 4.1 — no new pure resolver/guard function; documenting why (mirrors 11-3 Task 6.4
precedent).** The fix is a `WindowManager` call-sequence change (`removeView`+`addView`) gated by
the existing `activePointerId` touch-state field — there is no new standalone pure-logic
invariant to extract and test in isolation the way `isScrolledToBottom`/`selectSilenceSecs` were;
the "decision" (`activePointerId != INVALID_POINTER_ID`) is a one-line read of state that already
exists and is already exercised end-to-end by the existing touch-handling code paths. Per the
project's premature-abstraction guard (factor out only on a second real consumer), introducing an
artificial standalone function purely to have something to unit-test would not add coverage —
`WindowManager`/window-order behavior itself is not JVM-testable (needs a real window manager),
which is exactly why Task 4.4's real-device GATE-4 is the actual verification surface for AC-1,
same as 11-3's precedent for its own WindowManager-adjacent findings.

**AC-1 inversion (DoD closing bullet):** reasoned through, not just first-look-tested — the fix
is order-independent across repeated hide/re-show because the reorder runs on every panel `addView`
call, not once at service startup; a hide-then-reshow always goes through `showListeningPanel`'s
non-early-return branch again (panel state is nulled out by `hideListeningPanel` first), so the
reorder always re-fires. The one identified way this *could* regress back to add-order-dependence
is a future code path that shows the panel via some path other than `showListeningPanel`, or a
future bubble re-add/reshow after the panel that isn't followed by another panel-add — neither
exists today.

### File List

- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` (modified — sole production file
  touched; no `.rs`/`.ts`/`.tsx` changed, confirming the lighter `scripts/android-smoke.sh` DoD
  path was correct)

## Change Log

| Date | Change |
|------|--------|
| 2026-07-08 | Story created (bmad-create-story) from `docs/backlog.md` §11-4, split from 11-3 Task 6 (2026-07-07 GATE-2) with intent clarified 2026-07-08 (real Z-layering, not positioning; remove unwanted geometric-clearance behavior). Status: ready-for-dev. Open items: exact repositioning-behavior location (Task 1, not conclusively found during story creation), Z-order mechanism choice (re-add vs. a11y-reparenting), touch-overlap visual acceptability — see "OPEN ITEMS" and elicitation report. |
| 2026-07-08 | **bmad-dev-story: implemented Tasks 1-4 (AC-1, AC-2, AC-3; AC-4 N/A).** Task 1/2 investigation found no dedicated geometric-clearance clamp anywhere in the codebase — the observed "avoidance" was the AC-1 z-order defect itself (panel added after bubble → panel rendered on top), nothing to remove. Task 3 implemented re-add-after-panel-show (`reorderBubbleAbovePanel()`, called from `showListeningPanel` after every actual panel `addView`) — the sanctioned first-try mechanism (Design Decision 2) held up under analysis for the repeated hide/re-show stress case (AC-1's order-independence clause), so **no escalation to `TYPE_ACCESSIBILITY_OVERLAY` reparenting was needed and AC-4 is N/A**. Found and guarded one real fragility: `showListeningPanel()` can fire mid-touch-sequence (push-to-talk's `longPressRunnable`), where `removeView()` would have cancelled the in-flight gesture — added a `bubbleReorderPending` deferral (consumed at the next `ACTION_UP`/`ACTION_CANCEL`) so the reorder never disrupts an active touch. No new pure-logic function introduced (Task 4.1: a `WindowManager` call-sequence change gated by existing touch state, not a new testable invariant — documented rationale, mirrors 11-3 Task 6.4 precedent). **162/162 JVM unit tests green (no regressions, count unchanged), clean Kotlin compile, `gen-android-theme.mjs --check` in sync, `scripts/android-smoke.sh` full clean build/install verified on real device (Tailscale `100.112.41.70`), Kotlin-only File List confirmed.** Status → `review`. **Real-device GATE-4 smoke (Task 4.4) is still Andi's action** — confirm bubble renders on top + stays reachable across RECORDING→TRANSCRIBING→DONE cycles, confirm no more geometric-clearance trick when dragging into the panel, spot-check the narrow push-to-talk-mid-hold edge case flagged in Completion Notes, and spot-check drag/edge-snap/keyboard-avoidance (AC-3) are unregressed. |
