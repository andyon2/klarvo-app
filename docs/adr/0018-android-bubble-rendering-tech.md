# ADR-0018: Android Bubble Rendering Tech — View+Canvas vs ComposeView

**Status:** Accepted
**Date:** 2026-06-14

## Context

The Epic 9 Android visual overhaul introduces a fundamentally new bubble interaction: a state sequence
(`idle → recording → transcribing → done`) with a Klarvo-owned **listening panel** (grab handle,
amber live-dot, reactive RMS waveform, timer, red stop button, live raw transcript) plus spring
animations and a long-press popover menu.

**Current substrate:**

- `FloatingBubbleView.kt` (~478 LOC): pure `View` subclass; all rendering via `Canvas.onDraw()`;
  state enum `IDLE / RECORDING / RECORDING_PTT / PROCESSING`; animation via `ValueAnimator`
  (`OvershootInterpolator(2.0f)` for PTT scale-up, `LinearInterpolator` for waveform bars and
  rotation spinner; 5 animated bars with phase offsets).
- `KlarvoOverlayService.kt` (~1482 LOC): `Service` that adds the view to `WindowManager` via
  `TYPE_APPLICATION_OVERLAY` (`SYSTEM_ALERT_WINDOW`). The service owns the touch handler, drag
  logic, mode switching, config loading, and the full audio/STT/LLM pipeline.
- **No Jetpack Compose dependency exists anywhere in the build** (`build.gradle.kts`: zero
  `androidx.compose.*` entries). `minSdk = 24`.

**The open question:** The new Epic 9 bubble states require a multi-element, vertically-stacked
**listening panel** that must:
1. lay out a grab handle, K-logo + amber live-dot, a 5-bar RMS waveform, a timer, and a red stop
   button in a constrained vertical arrangement;
2. run spring-enter (240ms `cubic-bezier(.34,1.56,.64,1)`) and state-transition animations (120ms
   micro, 180ms state, 320ms panel);
3. render responsive to RMS amplitude values streamed from the audio thread.

Two substrate options were evaluated: (A) extend the existing View+Canvas, or (B) introduce a
`ComposeView` wrapper inside the overlay window.

## Decision

**Option A chosen: Extend the existing View+Canvas substrate.**

ComposeView in an overlay `Service` context is not viable without significant infrastructure work
that is disproportionate for this project. View+Canvas is sufficient for all Epic 9 requirements.

### Rationale

**Option B — ComposeView: risk outweighs benefit.**

`ComposeView` requires a `LifecycleOwner` and a `ViewModelStoreOwner` to be attached to the view's
context (via `ViewTreeLifecycleOwner.set()` / `ViewTreeViewModelStoreOwner.set()`) before Compose
can compose at all. An overlay `Service` created via `WindowManager.addView()` is **not** a
`LifecycleOwner` by default — it has no `Lifecycle` object. Without one, `ComposeView` throws at
runtime or produces a blank view.

The workaround — attaching a manually-managed `LifecycleRegistry` and a stub
`ViewModelStoreOwner` — is possible (used by some floating-window libraries), but it introduces:
- an entirely new lifecycle management layer not present anywhere else in this codebase;
- a new dependency (`androidx.compose.ui`, `androidx.compose.foundation`, `androidx.compose.material`,
  `androidx.lifecycle:lifecycle-viewmodel-compose` or similar) with a significant blast radius on the
  Android build (Compose requires Kotlin 1.9+, a specific AGP version range, `buildFeatures { compose
  = true }`, and a `composeOptions` block);
- an unknown interaction surface: `TYPE_APPLICATION_OVERLAY` windows live outside the normal view
  hierarchy, and overlay-service + Compose is an uncommon pairing with documented gotchas (IME
  interaction, focus handling, accessibility node propagation).

The **only concrete advantage** of Compose for Epic 9 is that `Column`/`Row` composables make the
multi-element listening panel layout more declarative than `onDraw()` coordinate math. That benefit
does not outweigh the blast radius, the LifecycleOwner infrastructure debt, and the unknown overlay
interaction surface.

**Option A — View+Canvas: sufficient, low-blast-radius.**

The required motion is fully achievable with `ValueAnimator` + `OvershootInterpolator`:
- The PTT scale-up already uses `OvershootInterpolator(2.0f)` at 200ms — the same mechanism covers
  the 240ms spring-enter for the listening panel (same interpolator, same animator pattern, different
  duration and targets).
- All required timing tiers (120ms micro, 180ms state, 320ms panel) map directly to `ValueAnimator`
  durations with `LinearInterpolator` or `OvershootInterpolator` depending on whether overshoot is
  desired.
- The waveform bars are already animated via 5 phase-offset `ValueAnimator` bars; the same mechanism
  renders RMS-driven bars by replacing the sinusoidal amplitude with the live `amplitude` field.

The listening panel's layout is the main complexity driver. In `onDraw()`, a multi-element vertical
layout requires explicit coordinate math for each element. The panel is however a **fixed
composition** (same elements in the same order, fixed aspect ratio within the bubble's bounding
box), not a dynamic flow layout — so the coordinate math is a one-time authoring cost, not ongoing
maintenance. The existing bar layout in `drawRecordingBar()` demonstrates the pattern works.

**Effort delta:**

| Concern | Option A (View+Canvas) | Option B (ComposeView) |
|---|---|---|
| New dependencies | None | Compose UI + Foundation + (lifecycle-viewmodel-compose); AGP/Kotlin version alignment |
| LifecycleOwner infra | None | Manual LifecycleRegistry + stub ViewModelStoreOwner required |
| Listening panel layout | Coordinate math in onDraw (~60–80 LOC) | Column/Row composable (~30–40 LOC but gated on infra above) |
| Animation | ValueAnimator (existing pattern) | Compose spring() / animate*AsState (new API surface) |
| Overlay-service compatibility | Proven (FloatingBubbleView works today) | Requires testing; documented gotchas with IME + focus |
| 9.4 harness (state injection) | Public setState() method or debug broadcast | Compose state holder injection |
| Build blast radius | Zero | Medium-high (composeOptions block, compilerExtension version pinning) |
| Risk of overlay-service regression | Negligible (no change to view attachment) | Unknown; must be validated |
| Estimated extra stories to handle infra | 0 | ~1 dedicated Compose-infra setup story |

**Numbered sub-decisions:**

1. The new Epic 9 state sequence (`idle / recording / transcribing / done`) replaces
   `IDLE / RECORDING / RECORDING_PTT / PROCESSING` as the canonical state enum in
   `FloatingBubbleView`. The old states are remapped: `PROCESSING` → `transcribing`; `RECORDING_PTT`
   is retired (the distinction between bar-mode and circular-mode recording is replaced by the unified
   listening panel).
2. The listening panel is rendered as a new `drawListeningPanel()` helper in `FloatingBubbleView`,
   called from `onDraw()` in the `recording` and `transcribing` states. Panel element positions are
   computed from the view's current width/height at draw time.
3. Spring-enter for the listening panel uses `ValueAnimator` + `OvershootInterpolator` (same pattern
   as the existing PTT scale-up animator). The interpolator overshoot coefficient is tuned to
   approximate `cubic-bezier(.34,1.56,.64,1)` tactilely.
4. RMS waveform in the listening panel reuses the existing 5-bar `drawWaveformBarsInZone()` helper,
   driven by the existing `amplitude` field on `FloatingBubbleView`.
5. Compose is explicitly **not introduced** in this epic. If a future epic requires a Compose-based
   settings UI or in-app screen (not an overlay), that decision is independent of this ADR and would
   apply only to non-overlay contexts.

## Consequences

### Positive

- **Zero build blast-radius**: no new Gradle dependencies, no `buildFeatures { compose = true }`,
  no compiler plugin, no AGP/Kotlin version constraints.
- **Proven overlay compatibility**: the View+Canvas substrate already works in `TYPE_APPLICATION_OVERLAY`
  — no unknown interaction surface.
- **Existing animation pattern reused**: `OvershootInterpolator` + `ValueAnimator` already covers
  the required spring motion; same mechanism extends to the panel enter/collapse.
- **Full RMS waveform reuse**: `drawWaveformBarsInZone()` and the `amplitude` field are already
  wired from the audio thread; no new data channel needed.
- **Incremental scope**: each Epic 9 story modifies `FloatingBubbleView` and `KlarvoOverlayService`
  only — the entire change surface is two known files.

### Negative

- **Listening panel layout is coordinate math**: laying out 6 elements (grab handle, K+amber-dot row,
  waveform, timer, stop button, transcript area) in `onDraw()` requires explicit position arithmetic.
  More verbose than a Compose `Column` but bounded in scope (fixed composition, one-time authoring).
- **No declarative layout primitives**: if the panel gains dynamic element counts in a future epic,
  the coordinate approach becomes harder to maintain. At that point, a Compose refactor would be
  justified (and the LifecycleOwner infrastructure cost would be spread over more value).
- **Spring approximation**: `OvershootInterpolator` approximates `cubic-bezier(.34,1.56,.64,1)` but
  does not reproduce it exactly. The perceptual difference is minor for the 240ms panel-enter
  duration; if pixel-exact spring easing is required in a future story, `SpringAnimation` from the
  `dynamicanimation` library would be the correct addition (zero Compose dependency).

### Mitigations

- The panel layout complexity is mitigated by adding a `drawListeningPanel()` helper method that
  encapsulates all panel coordinate logic, keeping `onDraw()` clean.
- The spring-approximation difference is mitigated by tuning the `OvershootInterpolator` coefficient
  empirically during 9.3 / 9.4 implementation.

### Verifiability Symmetry — Story 9.4 Harness (AC3)

Under View+Canvas, the bubble state is set via the public `state` property on `FloatingBubbleView`
(already public: `var state: State = State.IDLE`). The overlay service already routes all state
changes through `setState()` in `KlarvoOverlayService`.

The **Story 9.4 bubble state harness** will use this mechanism:

1. A debug broadcast receiver (`com.klarvo.voice.DEBUG_SET_STATE`) registered in
   `KlarvoOverlayService` only in debug builds (`BuildConfig.DEBUG`). It accepts an intent extra
   `"state"` (string: `"idle"`, `"recording"`, `"transcribing"`, `"done"`) and calls `setState()`
   directly.
2. A second extra `"rms"` (float 0.0–1.0) injects a synthetic `bubbleView.amplitude` value so the
   waveform is exercisable without live audio.
3. A third extra `"transcript"` (string) injects synthetic raw-transcript text into the listening
   panel for 9.4 / 9.5 development.

This makes **all four states reachable by Andi on-device** via `adb shell am broadcast` without
requiring live microphone input, a real STT response, or a network call — satisfying the
verifiability-symmetry principle: the same state space the agent can reach in tests is also
reachable by the human tester.

Example (run from WSL):
```sh
adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE \
  --es state recording --ef rms 0.6 \
  --es transcript "This is a synthetic dictation transcript."
```

The harness is gated by `BuildConfig.DEBUG` and the broadcast receiver is unregistered in release
builds — no user-facing surface, no telemetry.
