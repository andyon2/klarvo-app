# Story 9.1: ADR — Android Bubble Rendering Tech (Precursor Gate)

Status: review

## Story

As an architect,
I want a decision on whether to extend the existing View+Canvas overlay or introduce a ComposeView,
so that all bubble stories build on a settled substrate and the Epic 9 effort estimate is honest.

## Acceptance Criteria

**AC1 — Decision recorded in the ADR:**
Given the current overlay (`FloatingBubbleView.kt` View+Canvas, `KlarvoOverlayService.kt`)
When the ADR is written
Then it records the decision (extend View+Canvas vs introduce ComposeView inside the `SYSTEM_ALERT_WINDOW` overlay) with rationale covering:
- Motion needs (state sequence + spring animations for idle→recording→transcribing→done)
- The listening-panel composition (grab handle, amber live-dot, RMS waveform, timer, red stop)
- RMS-waveform rendering approach
- The risk of mixing Compose into an overlay service (`SYSTEM_ALERT_WINDOW` context)
- The effort delta between the two substrates

**AC2 — ADR file location and format:**
Given the decision
When recorded
Then it lands as `docs/adr/0018-android-bubble-rendering-tech.md` following the ADR convention (Status: Accepted, Date, Context, Decision, Consequences) per `docs/adr/README.md`
And the `docs/adr/README.md` index table is updated with the new ADR row.

**AC3 — Verifiability-symmetry implication named:**
Given the chosen substrate
When stated in the ADR
Then the ADR explicitly names how the chosen substrate supports (or constrains) the Story 9.4 bubble state harness — i.e., how the dev-only path to drive the bubble through all four states deterministically will be implemented.

**AC4 — No code, ADR only:**
Given this is a precursor gate
When the story is done
Then the only deliverable is the committed ADR file — no code changes, no UI changes, no config changes.

**Inversion (must-fail gate):** An ADR that omits the effort-delta comparison between the two substrates, or that does not explicitly name the verifiability-symmetry implication for 9.4, must not pass review.

**DoD:** ADR committed in its own commit per ADR convention (`docs/adr/0018-android-bubble-rendering-tech.md` + index update). No code.

## Tasks / Subtasks

- [x] Task 1: Analyze current Android overlay codebase (AC: 1)
  - [x] Read `FloatingBubbleView.kt` (~478 LOC) — understand current state machine, rendering paths, animation setup
  - [x] Read `KlarvoOverlayService.kt` (~1482 LOC) — understand how it creates the overlay, attaches `FloatingBubbleView`, handles touch, manages lifecycle
  - [x] Check `src-tauri/gen/android/app/build.gradle.kts` — confirm **no Jetpack Compose dependency exists** (it doesn't — confirmed during story creation)
  - [x] Identify all animation paths currently used (ValueAnimator, OvershootInterpolator, LinearInterpolator)
  - [x] Map the current state machine: `IDLE / RECORDING / RECORDING_PTT / PROCESSING`

- [x] Task 2: Evaluate Option A — Extend View+Canvas (AC: 1)
  - [x] List what the new Epic 9 state sequence needs that current Canvas-based code already supports
  - [x] Identify what new rendering primitives are needed (glass ring 4dp, teal K letter, listening panel layout, waveform bars from RMS input, amber dot, timer text, grab handle)
  - [x] Assess spring animation support via `ValueAnimator` + `OvershootInterpolator` (currently used for PTT scale-up — same mechanism is available for spring-enter)
  - [x] Assess effort: what has to be added vs what can be reused
  - [x] Identify risks: View+Canvas layout complexity for the listening panel (a taller, multi-element panel is harder to lay out in `onDraw` than in a Compose/XML hierarchy)

- [x] Task 3: Evaluate Option B — Introduce ComposeView (AC: 1)
  - [x] Assess Compose-in-`SYSTEM_ALERT_WINDOW` compatibility: is `ComposeView` supported in an overlay `WindowManager` context on API 24+?
  - [x] If ComposeView works in overlay: assess effort to add Compose dependencies to `build.gradle.kts` and the blast radius on the build
  - [x] Assess whether Compose's `spring()` animation API covers the required motion (micro 120ms, state 180ms, enter 240ms spring `cubic-bezier(.34,1.56,.64,1)`)
  - [x] Assess how the 9.4 state harness would be implemented under Compose vs Canvas (Compose state is more easily injectable; Canvas requires imperative calls to `setState()`)
  - [x] Identify risks: new dependency, unknown overlay-service/Compose interaction, wider blast radius

- [x] Task 4: Write ADR-0018 (AC: 1, 2, 3)
  - [x] Create `docs/adr/0018-android-bubble-rendering-tech.md` using the project ADR format
  - [x] Context section: describe the current overlay (FloatingBubbleView View+Canvas 478 LOC + KlarvoOverlayService 1482 LOC), the Epic 9 requirements (new state sequence, listening panel, RMS waveform, spring motion), and the open question
  - [x] Decision section: state the chosen option clearly; include rejected alternative with rationale; list the numbered sub-decisions (e.g., how the state machine remaps, what canvas helpers are added/how Compose is scoped)
  - [x] Consequences section: positives, negatives, mitigations; **explicitly state how the 9.4 harness will work under the chosen substrate** (AC3)
  - [x] Include the effort-delta comparison between the two options (AC inversion guard)

- [x] Task 5: Update ADR index (AC: 2)
  - [x] Add row for ADR-0018 to the index table in `docs/adr/README.md`

- [x] Task 6: Commit per ADR convention (AC: 4)
  - [x] Stage only `docs/adr/0018-android-bubble-rendering-tech.md` and `docs/adr/README.md`
  - [x] Commit message: `docs(adr): 0018 Android bubble rendering tech — View+Canvas vs ComposeView`
  - [x] Verify no code files are staged

## Dev Notes

### Current Android Overlay Substrate — What Exists

**FloatingBubbleView.kt** (`android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt`, 478 LOC):
- Pure `View` subclass, all rendering via `Canvas.onDraw()`.
- Current state enum: `IDLE / RECORDING / RECORDING_PTT / PROCESSING`
- Animation: `ValueAnimator` for waveform bars (5 bars, phase offsets), rotation arc spinner, PTT scale-up (`OvershootInterpolator(2.0f)` → scale 1.0→1.3) and scale-down.
- Hardcoded colors: `#F5F5F5` (idle bg), `#EF4444` (recording red), `#F59E0B` (amber processing), `#22C55E` (confirm green) — all to be replaced in 9.2+.
- Bar mode (`RECORDING`) inflates width to `BAR_WIDTH_DP = 220`; circle stays at `bubbleSizeDp = 56`.
- Touch zone helpers `isTouchInCancelZone()` / `isTouchInConfirmZone()` read by `KlarvoOverlayService`.

**KlarvoOverlayService.kt** (`android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt`, 1482 LOC):
- `Service` + `WindowManager` overlay (`SYSTEM_ALERT_WINDOW`).
- Creates `FloatingBubbleView` and adds it via `windowManager.addView(bubbleView, bubbleParams)`.
- Touch handling, drag, long-press timer (500ms → PTT), mode switching, config loading.
- Recording modes: `HOLD / TOGGLE / AUTOSTOP / AUTO` (these map 1:1 to Epic 9's FR5 gesture modes).
- Long-press currently triggers push-to-talk — **Epic 9.8 remaps this to the popover menu**.

**No Compose in codebase:** `build.gradle.kts` has zero Compose dependencies. `minSdk = 24`.

### The Epic 9 Requirements That Drive the ADR Question

New state sequence from the epics spec (FR1-FR4):
- `idle` → one form, teal K + 4dp glass ring, responsive size clamp `clamp(36dp, 0.11×min(screenW,screenH)dp, 44dp)`
- `recording` → listening panel rises: grab handle, K + amber live-dot, reactive RMS waveform, timer, red stop. Live raw transcript multiline in-panel.
- `transcribing` → same panel, teal spinner + "Cleaning…", raw text dimmed.
- `done` → panel collapses, keyboard returns, brief check → idle.

The key question: the **listening panel** is a multi-element, vertically-stacked layout. In Canvas it requires manual layout math in `onDraw`. In Compose it is a `Column`/`Row` composable. This is the primary effort-delta factor.

### Motion Requirements (from SPEC)

- Spring enter: 240ms `cubic-bezier(.34,1.56,.64,1)` — achievable with `OvershootInterpolator` in ValueAnimator (already used for PTT scale-up, same tactile pattern)
- Micro: 120ms, state: 180ms, panel: 320ms — all achievable with ValueAnimator

### Compose-in-Overlay Risk to Assess

`ComposeView` requires a `LifecycleOwner` and `ViewModelStoreOwner` attached to it to work correctly. Overlay windows created via `windowManager.addView()` in a `Service` do NOT have a standard `LifecycleOwner` (Services don't implement `LifecycleOwner` by default). This is a **real risk that must be addressed in the ADR analysis**: either the ADR resolves it (e.g., by attaching a custom `LifecycleOwner`) or it is a decisive factor against Compose.

### ADR Format (from `docs/adr/README.md`)

```
# ADR-NNNN: Title

**Status:** Accepted
**Date:** YYYY-MM-DD
## Context    (problem + constraints)
## Decision   (short + direct; numbered sub-decisions + rejected alternatives)
## Consequences (positive / negative / mitigations)
```

ADR stubs with `Status: Proposed` are allowed initially, but must be `Accepted` before the story is done.

Next ADR number: **0018** (current last: 0017).

### Verifiability Symmetry — AC3 Requirement

Per the project-level principle: the 9.4 state harness must let **Andi** (not just the agent) drive the bubble through all four states on-device. The ADR must name how the chosen substrate enables this. For Canvas: it means exposing a public method (or debug broadcast) that calls `bubbleView.state = FloatingBubbleView.State.RECORDING_PTT` etc. + injects synthetic RMS/transcript values. For Compose: it means injecting a state holder with synthetic test data. Either way, the ADR should state the harness mechanism clearly enough that Story 9.4 has a clear substrate to build on.

### Project Structure Notes

- ADR files: `docs/adr/NNNN-short-title.md`
- ADR index: `docs/adr/README.md` (index table — add one row)
- No code paths are touched in this story; no `src/`, `src-tauri/`, or `android/kotlin-src/` files are modified.
- Commit convention: ADR gets its own dedicated commit.

### Commit Scope Rule

**Never `git add .`** — stage only the two ADR files:
1. `docs/adr/0018-android-bubble-rendering-tech.md`
2. `docs/adr/README.md`

### References

- [Source: epics-visual-overhaul.md, Story 9.1] — Story ACs and DoD
- [Source: epics-visual-overhaul.md, AR1] — ADR requirement: "load-bearing gate, not a formality"
- [Source: epics-visual-overhaul.md, AR3/NFR4] — 9.4 state harness, verifiability symmetry
- [Source: epics-visual-overhaul.md, AR5] — Hard IME constraints for subsequent stories
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt] — Current Canvas substrate
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt] — Overlay service lifecycle
- [Source: src-tauri/gen/android/app/build.gradle.kts] — No Compose dep; minSdk 24
- [Source: docs/adr/README.md] — ADR format + numbering convention (0018 is next)
- [Source: sprint-change-proposal-2026-06-13.md, Section 2 Artifact Conflicts] — "load-bearing open decision"
- [Source: project-context.md, Framework-Specific Rules] — Android bypasses Tauri IPC (~85%), `jni 0.21` pinned, NOT 0.22
- [Source: project-context.md, Critical Don't-Miss Rules] — BYOK, no telemetry
- [Source: _bmad-output/planning-artifacts/epics-visual-overhaul.md, Epic 9 dependency flow] — 9.1 (gate) → 9.2 → 9.3 → 9.4 → 9.5; this ADR gates all subsequent bubble stories

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (story-context pass, 2026-06-14)

### Debug Log References

(none — pure ADR story, no code changes)

### Completion Notes List

- Analyzed FloatingBubbleView.kt (478 LOC): pure View+Canvas substrate, state enum IDLE/RECORDING/RECORDING_PTT/PROCESSING, ValueAnimator+OvershootInterpolator(2.0f) already used for PTT scale-up (same pattern available for spring-enter).
- Analyzed KlarvoOverlayService.kt (1482 LOC): Service+WindowManager overlay, no LifecycleOwner, no Compose dependency anywhere in build.gradle.kts (minSdk=24).
- Evaluated Option B (ComposeView): blocked by LifecycleOwner requirement in a Service context — requires manual LifecycleRegistry+ViewModelStoreOwner infrastructure plus significant Gradle blast-radius; advantage (declarative layout) does not outweigh cost.
- Evaluated Option A (View+Canvas): spring animations achievable via OvershootInterpolator (already proven for PTT); waveform bars and drawWaveformBarsInZone() fully reusable; listening-panel layout is one-time coordinate math, bounded scope.
- **Decision: Option A — extend View+Canvas.** Written as ADR-0018, Status: Accepted.
- 9.4 harness mechanism named in ADR Consequences: debug broadcast receiver (BuildConfig.DEBUG) accepting state/rms/transcript extras, callable via `adb shell am broadcast`. Satisfies verifiability-symmetry (Andi can drive all four states on-device without live audio/network).
- AC inversion check: ADR includes a 7-row effort-delta table comparing both substrates explicitly — cannot pass review without it (inversion guard).
- Committed: `docs(adr): 0018 Android bubble rendering tech — View+Canvas vs ComposeView` (only two ADR files staged, zero code changes).

### File List

- docs/adr/0018-android-bubble-rendering-tech.md (new)
- docs/adr/README.md (modified — added ADR-0018 index row)

## Change Log

- 2026-06-14: ADR-0018 written and committed. Decision: extend View+Canvas (Option A). Rationale: ComposeView requires LifecycleOwner infrastructure not present in overlay Service context; blast-radius unacceptable. View+Canvas: OvershootInterpolator already covers spring motion; waveform bars reusable; one-time coordinate math for listening panel is bounded. 9.4 harness: debug broadcast via adb. (claude-sonnet-4-6)
