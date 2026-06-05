# Floating Bar Subsystem — Deep Dive Documentation

**Generated:** 2026-06-05
**Scope:** `src/FloatingBar.tsx` + its Tauri coupling (frontend bindings, Rust window/event/command sides)
**Files Analyzed:** 9 (2 owned, 7 coupling surfaces)
**Lines of Code (core owned):** 895 (`FloatingBar.tsx`) + 33 (`main.tsx`)
**Workflow Mode:** Exhaustive Deep-Dive
**Status:** Fact-basis (Ist-Zustand). No recommendations are prescriptive — this is the map, not the plan.

---

## Overview

The **floating bar** is Klarvo's always-on-top dictation HUD: a small pill that lives above the
Windows taskbar, shows recording state (waveform / spinner / done-pop / error), and — when live
preview is enabled — grows into an upward-expanding card that streams partial transcript text.

**It is a SEPARATE Tauri WebView window** (label `"bar"`), not a React subtree of the main app.
A single React bundle (`index.html`) is loaded into both windows; `main.tsx` branches on
`getCurrentWindow().label === "bar"` to mount `FloatingBar` instead of `App`. The two windows
share **nothing** at the JS level — no React context, no shared state, no props. All
communication between them happens through the **Rust backend** (Tauri events + commands).

**Purpose:** Give the user immediate, glanceable feedback for the global-hotkey dictation
pipeline without stealing focus from whatever app they are dictating into.

**Key Responsibilities (the bar does all of these in one 895-line component):**

1. **Window lifecycle** — show/hide/resize/reshape/reposition its own OS window via direct Tauri
   window API calls (`setSize`, `setPosition`, `show`, `hide`) plus a Rust command for the OS
   region mask (`set_bar_shape`).
2. **State rendering** — a 7-state machine (`idle | recording | transcribing | cleaning | done |
   error | warning`) driven by `klarvo://state-changed` events.
3. **Audio waveform** — a 5-bar visualizer fed by a 20-sample ring buffer from `klarvo://audio-level`
   (~15 Hz).
4. **Live preview panel** — accumulates `klarvo://live-preview-chunk` text, measures it via a hidden
   probe, and grows the window upward to fit, capping at the room above the pill.
5. **Mode badge** — shows the active hotkey mode (`Hold`/`Toggle`/`Auto Stop`/`Auto`), kept fresh via
   `klarvo://active-mode`.
6. **Drag + position persistence** — manual mouse-driven drag (Tauri's native drag is unreliable on
   transparent decorationless WebView2) with debounced save to `config.json` (`save_bar_position`).
7. **Self-healing** — `ensureBarWindow()` recovery when `show()` fails or before each recording.

**Integration Summary:** 6 Tauri commands invoked, 4 Tauri events subscribed (0 emitted),
6 direct Tauri window-API calls, 2 config fields owned (`bar_x`/`bar_y`), 2 config fields read
(`hotkeyMode`, `previewPanelForm`). The bar is a pure **consumer** of backend events — it never
emits its own.

> **Platform reality:** The bar is **effectively Windows-only.** Its startup creation
> (`lib.rs:869`) is gated `#[cfg(target_os = "windows")]`, and `set_bar_shape`'s body is
> `#[cfg(target_os = "windows")]`. On Linux/macOS the `"bar"` window is never created at boot, so
> `main.tsx` never routes to `FloatingBar` there. Android has no bar concept at all (native overlay
> bubble instead). **Linux `cargo test` exercises none of this** — see Race & Verification sections.

---

## Complete File Inventory

### `src/FloatingBar.tsx` — THE subsystem

**Purpose:** The entire floating-bar UI, window-control logic, event wiring, and race guards in one
default-exported function component.
**Lines of Code:** 895
**File Type:** React 19 function component (TSX, inline-style only — no Tailwind classes, no CSS modules)

**What Future Contributors Must Know:**
- This file is a **separate-window mount**. It does NOT re-render when the Settings panel
  opens/closes (that is a view-toggle inside the *main* window). Therefore **every settings value
  the bar reads must be refreshed reactively** (on a state transition or panel-open edge), because a
  mount-only `getSettings()` freezes the value at app-start. This is why `previewPanelForm` is
  re-read on the closed→open panel edge (line 365) and `hotkeyMode` is refreshed via the
  `klarvo://active-mode` event (line 302).
- The bar **drives its own OS window geometry** from React effects via async Tauri IPC. Because each
  resize/position/shape is an independent async round-trip, **ordering and lateness of these IPC
  calls is the dominant source of bugs** (top-clip, white-line, teleport). Most of the effect
  complexity exists to defend against IPC races, not to render UI.
- There are **no automated tests** for this file (no `*.test.tsx` anywhere in `src/`). Every guard is
  validated only by manual Windows smoke. The inline `// Inversion:` comments document how to prove
  each guard is load-bearing by hand.

**Exports:**
- `default function FloatingBar(): JSX.Element` — the whole component. No named exports.

**Module-internal (not exported):**
- `KlarvoLogo()`, `Waveform({levels})`, `Spinner({color})`, `CheckIcon({color})`,
  `StopButton({onClick})` — sub-components.
- `hotkeyModeLabel(mode): string` — enum→label map.
- `getFormAppearance(form): {width, screenCap}` — preset lookup (compact/comfortable/wide).

**Constants (cross-boundary magic numbers — see Tech Debt):**
- `BAR_COUNT = 5`, `PILL_WIDTH = 200`, `PILL_WIDTH_CLIPBOARD = 220`, `PILL_HEIGHT = 36`,
  `SCREEN_TOP_MARGIN = 12`.
- `FORM_APPEARANCES = { compact:{260,240}, comfortable:{320,320}, wide:{400,400} }`,
  `DEFAULT_FORM = "comfortable"`.

**Dependencies (imports):**
- `react` — `useState, useEffect, useLayoutEffect, useRef`.
- `@tauri-apps/api/event` — `listen` (used directly for 3 of the 4 subscribed events).
- `@tauri-apps/api/webviewWindow` — `getCurrentWebviewWindow` (the bar's own window handle).
- `@tauri-apps/api/dpi` — `LogicalSize, LogicalPosition`.
- `./types` — `RecordingState, HotkeyMode`.
- `./tauri-commands` — `onStateChanged, setBarShape, cancelRecording, saveBarPosition,
  getBarPosition, getSettings, ensureBarWindow`.

**Used By:**
- `src/main.tsx` (dynamic `import("./FloatingBar")` when `label === "bar"`).

**State (11 `useState` + 12 `useRef`):**

| `useState` | initial | role |
|---|---|---|
| `state` | `"idle"` | the 7-state machine |
| `levels` | `Array(20).fill(0)` | audio ring buffer |
| `showDone` | `false` | done-pop visibility |
| `clipboardOnly` | `false` | focus-restore-failed flag |
| `livePreview` | `""` | accumulated preview text (push sink) |
| `panelHeight` | `0` | measured preview-box height (logical px) |
| `panelScrolls` | `false` | true once capped → scroll + top-fade |
| `collapsing` | `false` | collapse-animation gate |
| `hotkeyMode` | `"hold"` | active mode for badge |
| `previewPanelForm` | `DEFAULT_FORM` | display preset |
| `geomTick` | `0` | forces geometry re-assert after cold open |

| `useRef` | mirrors | why a ref (not state) |
|---|---|---|
| `doneTimerRef` | — | done-flash timeout handle |
| `collapseTimerRef` | — | collapse→hide timeout handle |
| `previewPanelRef` | — | the scrollable preview DOM node |
| `measureRef` | — | hidden probe DOM node (height measurement) |
| `panelHeightRef` | `panelHeight` | live value for mount-once drag handlers |
| `isPanelOpenRef` | `isPanelOpen` | live value for mount-once drag handlers |
| `barX` / `barY` | — | persisted pill top-left anchor (logical px) |
| `isRecordingRef` | `isRecording` | **stale-chunk guard** (5.7 AC-2) |
| `prevIsPanelOpenRef` | prev `isPanelOpen` | closed→open edge detection |
| `dragRef` | — | active-drag descriptor (null = not dragging) |
| `prevIsPillVisible` | prev `isPillVisible` | visible→idle edge detection |

**Derived values (computed each render, lines 253–265):**
- `isRecording`, `isProcessing`, `isActive`, `isError`, `isPillVisible`, `isIdle`, `isDone`.
- `isPanelOpen = isRecording && livePreview.length > 0` — the single switch that toggles pill↔card.
- `pillWidth`, `activePillWidth`, `activePillHeight`, `PANEL_WIDTH`, `PANEL_ABS_MAX` (lines 377–418).

**The hooks — exhaustive (15 active: 14 `useEffect` + 1 `useLayoutEffect`; +1 disabled):**

> The brief said "~12 useEffects." The actual count is **15 active effect hooks.** Enumerated here
> because the effect soup IS the subsystem's complexity, and most exist purely as race guards.

| # | Line | Deps | Cleanup | Responsibility / guarded race |
|---|------|------|---------|-------------------------------|
| 1 | 268 | `[]` | — | **Load stored position on mount.** `getBarPosition()`; falls back to `outerPosition()/scaleFactor`. Sets `barX/barY`. |
| 2 | 290 | `[]` | — | **Load settings on mount.** `getSettings()` → seeds `hotkeyMode` + `previewPanelForm`. (Frozen-at-startup; refreshed by #3 and #7.) |
| 3 | 302 | `[]` | unlisten | **`klarvo://active-mode` listener.** Keeps the mode badge correct when Hotkey-2 fires with a different mode than Hotkey-1. |
| 4 | 318 | `[isRecording]` | — | **`isRecordingRef` live mirror.** Feeds the stale-chunk guard in #5. |
| 5 | 326 | `[]` | unlisten | **`klarvo://live-preview-chunk` push listener.** Guard 1: `if (!isRecordingRef.current) return` (stale-chunk, AC-2). Guard 2: skip empty/whitespace chunks (AC-7). Appends to `livePreview`. |
| 6 | 352 | `[livePreview, panelScrolls]` | — | **Auto-scroll to newest text — ONLY when `panelScrolls`.** Scrolling while uncapped caused the grow-upward top-clip (WebView2 viewport-reflow lag). |
| 7 | 365 | `[isPanelOpen]` | — | **Re-read `previewPanelForm` on closed→open edge.** Defeats the frozen-at-startup problem for a preset saved in Settings without an app restart. |
| 8 | 390 | `[livePreview, isPanelOpen, PANEL_ABS_MAX, PANEL_WIDTH]` | — | **`useLayoutEffect`: measure text height via hidden probe**, cap to `min(PANEL_ABS_MAX, room above pill)`, set `panelHeight` + `panelScrolls`. Resets both to 0/false when closed. |
| 9 | 409 | none (runs every render) | — | **Mirror `panelHeight`/`isPanelOpen` into refs** so the mount-once drag handlers can convert window-top-left ↔ pill-anchor. |
| 10 | 433 | `[isPanelOpen]` | cancel raf + timeout | **Bump `geomTick`** on next frame + ~120 ms after open → re-runs #11 to fix cold first-expansion. Idempotent. |
| 11 | 440 | `[isPillVisible, activePillWidth, activePillHeight, geomTick]` | — | **THE window driver.** Order: `setSize → setBarShape → setPosition → show`. Guards: skip pre-measure transient (`isPanelOpen && panelHeight===0`), skip `setPosition` mid-drag (`dragRef.current == null`), `ensureBarWindow()` recovery if `show()` throws. |
| 12 | 498 | none (runs every render) | — | **Collapse-then-hide.** On visible→idle edge: play `bar-collapse`, then `win.hide()` after 200 ms. |
| 13 | 521 | `[]` | unlisten | **`onStateChanged` (`klarvo://state-changed`) handler.** The state machine's engine: clears preview on `recording`, schedules done-flash + done→idle on `done`, resets on `error`/`idle`, drops transient `warning`. Pre-recording `ensureBarWindow()` safety net. |
| 14 | 567 | `[]` | unlisten | **`klarvo://audio-level` handler.** RMS → `min(1, level*10)` → `pow(0.4)` → push into 20-sample ring. |
| 15 | 618 | `[]` | removeEventListener | **Global drag handlers** (`mousemove`/`mouseup` on `window`). On move: `setPosition`. On up: read final pos, convert to pill anchor, `saveBarPosition`. |
| — | 583 | `[isRecording]` | (disabled) | **Commented-out** legacy live-preview *polling* — disabled because it caused 10–20× Groq quota use. The push model (#5) replaced it. |

**Side Effects (this component is unusually effect-heavy):**
- **OS window mutation** — `setSize`, `setPosition`, `show`, `hide` (direct Tauri window API).
- **Tauri IPC commands** — `getBarPosition`, `getSettings`, `setBarShape`, `cancelRecording`,
  `saveBarPosition`, `ensureBarWindow`.
- **Tauri event subscriptions** — 4 (`state-changed`, `audio-level`, `live-preview-chunk`,
  `active-mode`).
- **Global DOM listeners** — `window` mousemove/mouseup for drag.
- **Timers** — done-flash (`1500`/`4000` ms), collapse→hide (`200` ms), error→idle (`2500` ms),
  geom-late (`120` ms).
- **Console logging** — `[bar] ...` info/warn/error throughout (no remote telemetry — BYOK).

**Error Handling:** Fully fail-soft. Every IPC call is `.catch()`-logged and non-fatal; position
load failure leaves the bar wherever Tauri placed it; `show()` failure triggers `ensureBarWindow()`
recovery. No throw escapes a handler.

**Testing:** **None.** No frontend test runner is wired for this file. Guards are documented via
inline `// Inversion:` comments and validated by manual Windows release smoke.

**Comments/TODOs:** No `TODO`/`FIXME`. Extensive *rationale* comments (the file is heavily
self-documenting about WHY each guard exists — these are the most valuable artifact in the file).

---

### `src/main.tsx` — the window router

**Purpose:** Single React entry point for BOTH windows; routes by Tauri window label.
**Lines of Code:** 33
**File Type:** React DOM bootstrap (TSX, top-level `await`).

**What Future Contributors Must Know:** `label === "bar" && !isPreviewMode` is the ONLY thing that
mounts `FloatingBar`. In `isPreviewMode` (plain browser, `__TAURI_INTERNALS__` absent) it always
mounts `App` — so the bar is **uninspectable in `npm run preview`.** `FloatingBar` is dynamically
imported so the Tauri window API stays off the module-eval path in a plain browser.

**Used By:** Vite entry (`index.html`). **Depends on:** `App`, `./styles.css`, `isPreviewMode`,
dynamic `./FloatingBar`, dynamic `@tauri-apps/api/window`.

---

### `src/tauri-commands.ts` — frontend IPC bindings (coupling surface; bar-relevant subset)

**Purpose:** Thin typed wrappers over `tauri::invoke` / `listen`, with preview-mode mocks.
**Lines of Code:** 1037 total (bar uses ~8 functions).
**File Type:** TS module.

**Bar-relevant exports:**
- `isPreviewMode: boolean` (line 20) — `__TAURI_INTERNALS__` absence ⇒ browser/mock mode.
- `onStateChanged(cb): Promise<()=>void>` (382) → `listen("klarvo://state-changed")`.
- `getSettings(): Promise<AppSettings>` (224) → `invoke("get_settings")` — returns the **entire**
  settings object even though the bar reads only 2 fields.
- `setBarShape("idle"|"pill"|"panel"): Promise<void>` (517) → `invoke("set_bar_shape", {shape})`.
- `cancelRecording(): Promise<void>` (194) → `invoke("cancel_recording")`.
- `saveBarPosition(x,y): Promise<void>` (859) → `invoke("save_bar_position", {x,y})`.
- `getBarPosition(): Promise<{x,y}|null>` (868) → `invoke("get_bar_position")`.
- `ensureBarWindow(): Promise<boolean>` (1033) → `invoke("ensure_bar_window")` (returns `false` in
  preview/Android).

**What Future Contributors Must Know:** Param keys are **snake_case to match Rust struct fields**;
`live-preview-chunk` and `audio-level` and `active-mode` are subscribed *directly* via
`@tauri-apps/api/event` inside `FloatingBar`, NOT through wrappers here — so the bar's event surface
is split across two files. Every wrapper short-circuits in `isPreviewMode`.

---

### `src/types.ts` — shared contracts (coupling surface)

**Bar-relevant types:** `RecordingState` (the 7 states), `HotkeyMode` (`toggle|hold|autostop|auto`),
`StateChangedPayload` (the `state-changed` payload shape, incl. `clipboardOnly`), `AppSettings`
(read for `hotkeyMode`, `previewPanelForm`). **LOC:** 234. The `StateChangedPayload` TS interface is
the hand-maintained mirror of the Rust `PipelineEvent` struct — they must stay in sync manually
(no codegen).

---

### `src-tauri/src/lib.rs` — window creation, emitters, command registration (Rust coupling)

**Bar-relevant surface:**
- `create_bar_window<M: Manager<Wry>>(app, saved_x, saved_y)` (603) — builds the `"bar"` WebView:
  `inner_size(80,10)`, `decorations(false)`, `transparent(true)`, `always_on_top(true)`,
  `resizable(false)`, `skip_taskbar(true)`, `focused(false)`, `shadow(false)` (Windows). Sets initial
  pill region, then positions: **(1)** saved config pos → **(2)** Win32 `SPI_GETWORKAREA` →
  **(3)** `monitor.size()` w/ 60 px taskbar estimate → **(4)** hard-coded `(400,10)`.
- **Startup call site** (869): `#[cfg(target_os = "windows")]` — **Windows-only.**
- `emit_pipeline_state(handle, PipelineEvent)` (487) — single call site for `klarvo://state-changed`
  + tray-tooltip sync. THE producer the bar's `onStateChanged` consumes.
- `setup_audio_level_emitter(handle)` (583) — installs the recorder level callback that emits
  `klarvo://audio-level` (`EVENT_AUDIO_LEVEL`, line 525). Called from setup, recording commands, and
  pipeline (3 sites).
- `set_window_region_ellipse / _pill / _round_rect` (530/545/565) — Win32 `SetWindowRgn` helpers the
  bar's shape command uses. `_round_rect` carries the white-line warning (region radius MUST equal
  CSS `borderRadius`).
- **Close handling** (947): the `"bar"` window's `CloseRequested` is always `prevent_close()`d +
  hidden — the bar must always exist.
- **`invoke_handler`** (966) registers `set_bar_shape`, `save_bar_position`, `get_bar_position`,
  `ensure_bar_window` (the last `#[cfg(desktop)]`).
- **Setup** (821): `_saved_bar_x = cfg.bar_x`, `_saved_bar_y = cfg.bar_y` loaded for create.

**What Future Contributors Must Know:** `create_bar_window` is `#[cfg(desktop)]` but its boot call is
`#[cfg(target_os = "windows")]` — so on Linux/macOS the window is never made. The bar's existence is
a Windows fact.

---

### `src-tauri/src/commands/misc.rs` — the four bar commands (Rust coupling)

- `save_bar_position(state, x, y)` (172) — writes `cfg.bar_x/bar_y` via `save_config_locked` (atomic,
  single-writer — ADR-0015). Both coords always written together.
- `get_bar_position(state)` (186) — returns `Some((x,y))` only when both are set, else `None`.
- `ensure_bar_window(app, state)` (208, `#[cfg(desktop)]`, async) — probes
  `get_webview_window("bar").is_visible()`; if missing/unresponsive, re-reads saved pos and calls
  `create_bar_window`. Returns `true` if recreated.
- `set_bar_shape(handle, shape)` (385) — `#[cfg(windows)]` body. `"idle"`→pill region 80×10;
  `"panel"`→`round_rect` using **live `inner_size()`** (dynamic) at radius `14*scale`;
  else (`"pill"`)→pill region 200×36. Non-Windows: no-op.

**What Future Contributors Must Know:** `set_bar_shape "panel"` reads the window's *actual*
`inner_size()` — so the frontend MUST `await win.setSize(...)` BEFORE calling `setBarShape("panel")`
(the bar does, in effect #11). The pill dimensions (200×36, 80×10) are **duplicated** here from
`FloatingBar.tsx` constants — no shared source.

---

### `src-tauri/src/hotkey/mod.rs` — the event payload contract (Rust coupling)

- `EVENT_STATE_CHANGED = "klarvo://state-changed"` (176).
- `enum PipelineState` (26, `#[serde(rename_all="lowercase")]`) — the 7 states.
- `struct PipelineEvent` (47) — `state`, `text?`, `rawText?` (renamed), `error?`, `warning?`,
  `clipboardOnly?` (renamed). Constructors: `idle/recording/transcribing/cleaning/done/
  done_with_clipboard_only/error/warn`. This struct IS `StateChangedPayload` on the Rust side; the
  `#[serde(rename)]` attrs produce the camelCase keys the TS expects.

---

### `src-tauri/src/pipeline.rs` — event producers + the live-preview race machinery (Rust coupling)

- `flush_preview_delta(handle)` (1935, async) — takes a delta WAV snapshot, re-reads provider
  (offline recheck, AC-7), transcribes (raw, no LLM), emits `klarvo://live-preview-chunk` (1978).
  Fully fail-soft.
- `maybe_install_preview_flush(handle)` (2007, `#[cfg(desktop)]`) — installs the repeatable
  silence-flush callback IFF `live_preview_enabled && stt_provider != "local"` AND not already
  recording. Creates a per-cycle `AtomicU8` in-flight counter and a closure that acquires a slot via
  `try_acquire_preview_slot` before spawning `flush_preview_delta`. **Called only for Toggle (2095)
  and Hold (2265)** — never Auto/AutoStop (AC-6).
- `preview_flush_should_install(enabled, provider) -> bool` (1867) — the testable predicate.
- `MAX_PREVIEW_IN_FLIGHT = 1` (1878), `struct PreviewSlotGuard` (1890, RAII `Drop` → `fetch_sub`),
  `try_acquire_preview_slot(counter)` (1915, lock-free CAS 0→1). **This is the backpressure cap.**
- `klarvo://active-mode` emit (2241) — fired in the hotkey handler the moment a slot's mode is
  resolved, so the bar badge reflects the firing slot.
- `create_bar_window` recovery call (606) inside the recording start path.

**What Future Contributors Must Know:** The in-flight CAS runs **on the cpal real-time audio callback
thread**, where `Mutex::lock` is forbidden — that is *why* it is a lock-free atomic, not a mutex.
A skipped flush does NOT advance the delta marker, so skipped audio is folded into the next flush
(NFR1). MAX=1 serializes Groq calls (default 2.0 s pause ≫ sub-1 s Groq latency).

---

### `src-tauri/src/config/mod.rs` — owned + read config fields (Rust coupling)

- `bar_x: Option<f64>` (726), `bar_y: Option<f64>` (732) — owned by the bar; `None` on first run;
  camelCase on disk; covered by round-trip + missing-field + camelCase tests (2686–2736).
- `live_preview_enabled: bool` (707, default `false`), `preview_pause_silence_secs: f32`
  (712, default 2.0), `preview_panel_form: String` (720, default `"comfortable"`) — read by the
  bar/pipeline. Missing-field defaults are inversion-guarded (3914–3958).

---

## Contributor Checklist

- **Risks & Gotchas:**
  - The bar is a **separate window that never re-mounts on Settings save** — any settings value it
    shows must be refreshed reactively, or it freezes at app-start.
  - **All geometry is async IPC**; the dominant bug class is stale/late/out-of-order
    `setSize`/`setPosition`/`setBarShape` landings (top-clip, white-line, teleport).
  - **Cross-boundary magic numbers** (pill 200×36, idle 80×10, corner radius 14, preset widths,
    `"comfortable"`) are duplicated between `FloatingBar.tsx` and Rust with no shared source — drift
    here re-introduces the white-line / clip artifacts.
  - **No frontend tests.** Linux `cargo test` covers only the Rust predicate/guard seams
    (`preview_flush_should_install`, `try_acquire_preview_slot`, config round-trips), NOT the
    window/effect behavior.
  - The bar is **Windows-only** in practice; do not assume it exists on Linux/macOS.
- **Pre-change Verification Steps:**
  1. `cargo check --target x86_64-pc-windows-gnu` when touching any Rust coupling file.
  2. `cargo test` (Linux) for the preview predicate, in-flight cap, and config-field guards.
  3. Build via `scripts/sync-and-build.ps1` → manual Windows release smoke (Linux is near-zero
     signal for this subsystem).
  4. Walk `docs/surface-smoke-checklist.md` items: camelCase config keys, Settings resync-useEffect,
     **FloatingBar separate-window reactivity**, window-geometry/region clip, event push-wiring.
- **Suggested Tests Before PR:** Manual smoke matrix — for each of {Hold, Toggle} × {comfortable,
  wide}: start recording → confirm pill→panel grows upward without top-clip on the FIRST chunk;
  drag mid-recording → confirm no teleport when a chunk lands; finish → confirm done-pop clears
  preview and no stale text bleeds into the next cycle.

---

## Architecture & Design Patterns

### Code Organization

One monolithic component file. Sub-components (logo, waveform, spinner, icons, stop button) are
module-local function components above the default export. There is **no** state-management library,
no reducer, no context — just `useState`/`useRef` and a wall of effects. The split of concerns is by
*effect*, not by module: each effect owns one responsibility + its race guard.

### Design Patterns

- **Separate-window + event-bus coupling** — the bar and main app are isolated WebViews that
  communicate only through Rust (Tauri events down, commands up). No shared JS state.
- **Push sink** — `livePreview` is an append-only accumulator fed by events, cleared on state edges.
- **Live-ref mirror** — refs (`isRecordingRef`, `panelHeightRef`, `isPanelOpenRef`) shadow state so
  that **mount-once (`[]`-dep) listeners and global drag handlers read live values** instead of stale
  closure captures. This is the central idiom and the source of the effect count.
- **Edge detection via prev-ref** — `prevIsPillVisible`, `prevIsPanelOpenRef` detect transitions
  inside dep-less effects.
- **Hidden-probe measurement** — an off-screen `aria-hidden` clone measures text height at the final
  width so sizing never depends on the live (lagging) window width — avoids a resize feedback loop.
- **RAII slot guard (Rust)** — `PreviewSlotGuard` releases the in-flight cap on drop, panic-safe.
- **Fail-soft everywhere** — no error path throws; structured logs only.
- **Idempotent re-assert** — `geomTick` re-runs the geometry effect to converge a cold first open.

### State Management Strategy

Local component state only. The "source of truth" for the dictation state lives in the **Rust
backend**; the bar is a projection of `klarvo://state-changed`. Settings truth lives in
`config.json` (read via `getSettings`/`getBarPosition`). The bar persists exactly one thing:
`bar_x`/`bar_y` via `saveBarPosition` (atomic single-writer, ADR-0015).

### Error Handling Philosophy

Fail-soft + self-heal. IPC errors are caught and logged; `show()` failure escalates to
`ensureBarWindow()` recreation; a pre-recording `ensureBarWindow()` is a safety net for a window that
silently vanished after hours in the background.

### Testing Strategy

Rust-side seams are unit-tested on Linux (`preview_flush_should_install`, `try_acquire_preview_slot`/
`PreviewSlotGuard`, config field round-trips + missing-field defaults, `PipelineEvent` clipboard
flag). The **frontend has no tests**; the contract is "inversion comments + Windows manual smoke."

---

## Data Flow

```
                         ┌─────────────────────── RUST BACKEND ───────────────────────┐
  global hotkey ──▶ hotkey handler (pipeline.rs)                                       │
        │                  │  emit klarvo://active-mode (mode)                         │
        │                  ▼                                                           │
        │           emit_pipeline_state ──▶ klarvo://state-changed (PipelineEvent)     │
        │                  │                                                           │
        │           recorder level cb ───▶ klarvo://audio-level ({level})  (~15 Hz)    │
        │                  │                                                           │
        │   (Toggle/Hold)  ▼                                                           │
        │   maybe_install_preview_flush ─ silence cb ─▶ try_acquire_preview_slot       │
        │                  │ (slot free)                                               │
        │                  ▼                                                           │
        │           flush_preview_delta ─▶ klarvo://live-preview-chunk (raw text)      │
        └──────────────────┼───────────────────────────────────────────────────────┘
                           │  (4 events, one-way down)
                           ▼
                ┌──────── "bar" WebView (FloatingBar.tsx) ────────┐
                │  active-mode ─▶ hotkeyMode (badge)              │
                │  state-changed ─▶ state machine ─▶ show/hide,   │
                │       clear/keep livePreview, done-flash        │
                │  audio-level ─▶ levels ring ─▶ Waveform         │
                │  live-preview-chunk ─▶ [stale guard] ─▶         │
                │       livePreview ─▶ measure(probe) ─▶          │
                │       panelHeight ─▶ setSize/Shape/Position     │
                │                                                 │
                │  user drag ─▶ setPosition ─▶ (up) saveBarPosition  ──┐ (commands, up)
                │  stop click ─▶ cancelRecording                       │
                │  show() fail / pre-record ─▶ ensureBarWindow         ▼
                └─────────────────────────────────────────────── invoke ▶ Rust
```

### Data Entry Points (into the bar)
- **`klarvo://state-changed`** — the master signal; everything visible keys off it.
- **`klarvo://audio-level`** — waveform feed.
- **`klarvo://live-preview-chunk`** — preview text feed (gated by the stale-chunk guard).
- **`klarvo://active-mode`** — badge feed.
- **`getSettings()` / `getBarPosition()`** — mount-time + panel-open reads.

### Data Transformations
- **RMS → bar height:** `min(1, level*10)` then `pow(0.4)` (compresses quiet speech up).
- **chunk → panel text:** trim, drop empty, space-join append.
- **text → window height:** hidden probe `offsetHeight` → cap to `min(PANEL_ABS_MAX, barY − 12)` →
  `panelHeight` → `PILL_HEIGHT + panelHeight`.
- **window top-left ↔ pill anchor:** when panel open, window top = `barY − panelHeight`; drag-end
  stores the pill anchor (`ly + panelHeight`).

### Data Exit Points (out of the bar)
- **`saveBarPosition(x, y)`** → `config.json` `bar_x`/`bar_y`.
- **`cancelRecording()`** → backend pipeline cancel.
- **`setBarShape(shape)`** → OS window region mask.
- **`ensureBarWindow()`** → window recreation.
- Direct window API: `setSize`/`setPosition`/`show`/`hide`.

---

## Integration Points

### Commands Invoked (bar → Rust, 6)
| TS wrapper | Rust command | Purpose |
|---|---|---|
| `getBarPosition` | `get_bar_position` | restore position on mount |
| `getSettings` | `get_settings` | seed/refresh hotkeyMode + previewPanelForm |
| `setBarShape` | `set_bar_shape` | OS region mask (idle/pill/panel) |
| `cancelRecording` | `cancel_recording` | stop button |
| `saveBarPosition` | `save_bar_position` | persist drag-end position |
| `ensureBarWindow` | `ensure_bar_window` | recovery / pre-record safety net |

### Direct Tauri Window API (bar → its own window, not commands)
`getCurrentWebviewWindow()` → `setSize`, `setPosition`, `show`, `hide`, `outerPosition`,
`scaleFactor`.

### Events Subscribed (Rust → bar, 4; **0 emitted by the bar**)
| Event | Payload | Producer |
|---|---|---|
| `klarvo://state-changed` | `PipelineEvent` (`StateChangedPayload`) | `emit_pipeline_state` (lib.rs) |
| `klarvo://audio-level` | `{ level: number }` | `setup_audio_level_emitter` (lib.rs) |
| `klarvo://live-preview-chunk` | `string` (raw segment) | `flush_preview_delta` (pipeline.rs) |
| `klarvo://active-mode` | `HotkeyMode` | hotkey handler (pipeline.rs) |

> **Naming rule:** all four use the **colon** form (`klarvo://...`). Tauri reserves `.` in event
> strings — never use dots here.

### Shared State (cross-window, via backend)
| State | Where it lives | Accessed by |
|---|---|---|
| recording/pipeline state | Rust pipeline → events | bar (read), main App (read) |
| `bar_x` / `bar_y` | `config.json` | bar (RW via commands) |
| `live_preview_enabled`, `preview_pause_silence_secs`, `preview_panel_form` | `config.json` | Settings (write), pipeline + bar (read) |

### Database Access
**None.** The bar touches neither `history.db` nor any SQLite. Its only persistence is the two
config coords.

---

## Dependency Graph

```
main.tsx ──(dynamic, label==="bar")──▶ FloatingBar.tsx
FloatingBar.tsx ──▶ tauri-commands.ts ──▶ @tauri-apps/api/core (invoke)
                ├──▶ @tauri-apps/api/event (listen)        ──▶ [Rust events]
                ├──▶ @tauri-apps/api/webviewWindow         ──▶ [own OS window]
                ├──▶ @tauri-apps/api/dpi
                └──▶ types.ts

Rust producers (no JS import — coupled only by event/command name strings):
  lib.rs (emit_pipeline_state, audio-level, create_bar_window, region helpers, handler reg)
  pipeline.rs (live-preview-chunk, active-mode, preview-flush race machinery)
  commands/misc.rs (set_bar_shape, save/get_bar_position, ensure_bar_window)
  hotkey/mod.rs (PipelineEvent / EVENT_STATE_CHANGED contract)
  config/mod.rs (bar_x/bar_y + preview fields)
```

### Entry Points (not imported by others in scope)
- `main.tsx` (the window bootstrap).

### Leaf Nodes (don't import others in scope)
- `types.ts`.

### Circular Dependencies
✓ None in the JS scope. The frontend↔Rust coupling is by **string contract** (event/command names),
not import — which is exactly why drift is silent and untyped across the boundary.

---

## The Race Class (explicit — the brief's focus)

The bar defends against a **family of timing hazards**, almost all caused by async Tauri IPC and the
separate-window model. Each is listed with its trigger, symptom, and the guard.

| # | Race | Trigger | Symptom if unguarded | Guard (where) |
|---|------|---------|----------------------|---------------|
| R1 | **Stale chunk** | chunk listener is mount-once; closure captured a stale `isRecording` | post-`done` chunk repopulates `livePreview` → text bleeds into idle/next cycle | `isRecordingRef` live mirror + `if (!isRecordingRef.current) return` (FB #4/#5, 5.7 AC-2) |
| R2 | **Preview backpressure** | two pause boundaries fire before the first Groq flush returns | overlapping flushes, 10–20× quota, out-of-order text | `try_acquire_preview_slot` CAS, `MAX_PREVIEW_IN_FLIGHT=1`, RAII `PreviewSlotGuard` (pipeline.rs, 5.7 AC-4); lock-free because it runs on the cpal RT thread |
| R3 | **Cold first-expansion** | first pill→panel `setSize` under-applies height (biggest on Wide) | window too short → flex-end + overflow:hidden clips the panel **top** until next chunk | `geomTick` raf + 120 ms re-assert (FB #10/#11, 5.5 r2); idempotent |
| R4 | **Pre-measure transient** | geometry effect runs with `panelHeight===0` before `useLayoutEffect` measures | stale pill-height resize lands AFTER the correct one → top-clip | `if (isPanelOpen && panelHeight===0) return` (FB #11, 5.5) |
| R5 | **Drag vs chunk reposition** | a preview chunk arrives mid-drag and the geometry effect fires `setPosition` | window teleports to stale `barX/barY` mid-drag | `dragRef.current == null` check before `setPosition` (FB #11, AC-4) |
| R6 | **Grow-upward viewport lag** | auto-scroll to bottom while uncapped during WebView2 viewport-reflow lag | oldest line pushed up and out the top under overflow:hidden, doesn't heal | gate scroll on `panelScrolls` (scroll only when capped) (FB #6, 5.7 geometry) |
| R7 | **Done→idle in Auto-Loop** | done timer fires `setState("idle")` but the next cycle already set `"recording"` | the new recording state is clobbered back to idle | `setState(prev => prev === "done" ? "idle" : prev)` (FB #13) |
| R8 | **Preview-config leak** | `set_preview_flush_config` written while already recording | stale config consumed by the NEXT `start_recording` on any path | `is_recording()` guard in `maybe_install_preview_flush` (pipeline.rs, 5.7) |
| R9 | **Mid-record provider swap** | user switches STT to "local" during a recording | flush tries a local-model transcription it shouldn't | flush-time re-read of `stt_provider` in `flush_preview_delta` (AC-7), not just install-time |
| R10 | **Show-before-shape** | `show()` before the OS region mask is applied | "white line" flash of unmasked window | strict order `setSize → setBarShape → setPosition → show` (FB #11) |
| R11 | **Region/CSS radius mismatch** | OS round-rect radius ≠ CSS `borderRadius` | persistent white-line at the corner gap | both pinned to **14** (FB wrapper + `set_window_region_round_rect`) |
| R12 | **Vanished window** | bar window silently dies after hours / `show()` throws | recording with no visible feedback | `ensureBarWindow()` on `show()` failure + pre-recording safety net (FB #11/#13) |
| R13 | **Boot warning lost** | config warning emitted before the frontend listener mounts | toast never shown | acknowledged-lost; durable surface is the `config.json.corrupt-<ts>` backup file, not the event (D1, lib.rs) |

**Root cause shared by R1, R3–R7, R10:** the bar mutates its own OS window through **independent
async IPC calls fired from React effects**, so any two of {measure, resize, reshape, reposition,
show} can land out of order relative to a competing event (chunk arrival, drag, cold start). The
guards are point-fixes; there is no single serialization layer.

---

## Testing Analysis

### Coverage Summary
- **Frontend (`FloatingBar.tsx`, `main.tsx`):** 0% — no test runner wired.
- **Rust seams:** unit-tested (predicate, in-flight cap, config round-trips, event flag). These cover
  the *logic* the window behavior depends on, not the window behavior itself.

### Test Files
- `src-tauri/src/pipeline.rs` `#[cfg(test)]` — `try_acquire_preview_slot` cap, install predicate,
  Auto/AutoStop-never-install (AC-6).
- `src-tauri/src/config/mod.rs` `#[cfg(test)]` — `bar_x`/`bar_y` round-trip, missing-field, camelCase
  key; preview-field defaults inversion guards.
- `src-tauri/src/hotkey/mod.rs` `#[cfg(test)]` — `clipboardOnly` serialization.

### Testing Gaps
- The entire effect chain (measure → resize → shape → position → show) is untested in any automated
  form.
- All 13 races are validated only by manual Windows smoke + inline inversion comments.
- No regression harness for the cross-boundary magic numbers (200/36/80/10/14/preset widths).
- Preview-mode (`npm run preview`) cannot exercise the bar at all (it mounts `App`).

---

## Related Code & Reuse Opportunities

### Similar Features Elsewhere
- **Android overlay bubble** (`android/.../com/klarvo/voice`, native Kotlin) — the platform analog of
  the bar (`TYPE_APPLICATION_OVERLAY`). It is a *separate* implementation, not shared code (Android
  bypasses Tauri IPC ~85%). Behavioral parity must be hand-mirrored (ADR-0016).
- **Main `App.tsx` Settings panel** — the *writer* of `previewPanelForm` / `live_preview_enabled`
  that the bar reads; the canonical example of the "separate window must refresh" coupling.

### Reusable Utilities Available
- `tauri-commands.ts` wrappers + `isPreviewMode` mock pattern — the established IPC seam; any new bar
  command should be added here, snake_case args, with a preview mock.
- `set_window_region_*` helpers (lib.rs) — the only sanctioned way to mask the OS window region.
- `save_config_locked` / ADR-0015 atomic single-writer — the only sanctioned way to persist new bar
  config fields.

### Patterns to Follow
- **New event:** colon name, add producer in Rust, subscribe in `FloatingBar` (or wrap in
  `tauri-commands.ts` if non-trivial), mirror payload type in `types.ts`.
- **New settings field the bar shows:** add reactive refresh (state edge or event), never rely on the
  mount-only `getSettings()`.

---

## Implementation Notes

### Code Quality Observations
- Exceptional *rationale* comments — the WHY behind each guard is documented inline; this is the
  file's biggest asset and should be preserved through any refactor.
- The component conflates ~7 responsibilities (window lifecycle, state machine, waveform, preview,
  badge, drag, recovery). High cohesion within each effect, low separation across them.
- Inline styles only — no design-token reuse; colors/dims are literals.

### TODOs and Future Work
- No `TODO`/`FIXME` in the bar files themselves. (A `TODO(ROB-04)` exists in `lib.rs` setup but is
  unrelated to the bar — voice-command config write path.)

### Known Issues / Tech Debt
- **Cross-boundary magic-number duplication** (pill 200×36, idle 80×10, radius 14, preset widths,
  `"comfortable"`) — no shared source between TS and Rust; drift re-introduces clip/white-line bugs.
- **No frontend tests** — every UI/window guard is smoke-only.
- **No serialization layer for window IPC** — 13 point-guards instead of one ordered apply.
- **Heavy `getSettings()`** returns the entire settings object to read 2 fields (twice: mount +
  panel-open).
- **Split event surface** — 3 events subscribed inline in `FloatingBar`, 1 (`state-changed`) via a
  wrapper; the bar's full contract is spread across `FloatingBar.tsx` + `tauri-commands.ts`.
- **Windows-only** with `#[cfg]` asymmetry (`create_bar_window` is `desktop`, its boot call is
  `windows`) — easy to misread as cross-platform.

### Optimization Opportunities (observations, not directives)
- A dedicated lightweight bar-settings command (return only `hotkeyMode` + `previewPanelForm`) would
  shrink the two `getSettings()` round-trips.
- A single `applyGeometry()` async serializer (queue/coalesce setSize/shape/position/show) could
  retire R3, R4, R5, R10 as a class instead of individually.
- Extract shared dimension constants into one source consumed by both TS and Rust (codegen or a
  generated constants file) to retire the duplication-drift risk.

---

## Modification Guidance

### To Add New Functionality
- **New visual state:** extend `RecordingState`/`PipelineState` (both sides), add a constructor in
  `PipelineEvent`, handle it in effect #13, render a branch in the pill row.
- **New bar→backend action:** add a `#[tauri::command]` (likely in `commands/misc.rs`), register it
  in `lib.rs` `invoke_handler`, add a `tauri-commands.ts` wrapper with a preview mock.

### To Modify Existing Functionality
- **Geometry/sizing:** any change to PANEL widths, radius, or pill dims MUST be mirrored in
  `set_bar_shape` (Rust) AND the CSS `borderRadius` — they are coupled by R11. Re-smoke the cold
  first-expansion (R3) on the Wide preset.
- **Preview behavior:** changes to flush cadence/backpressure live in `maybe_install_preview_flush` /
  `try_acquire_preview_slot`; keep the lock-free invariant (cpal RT thread).

### To Remove/Deprecate
- The bar window's `CloseRequested` is force-prevented; removing the bar means unwiring that, the
  startup `create_bar_window`, the 4 producers, and the 6 commands. The Android bubble is independent.

### Testing Checklist for Changes
- [ ] `cargo test` (Linux) green — predicate, in-flight cap, config fields.
- [ ] `cargo check --target x86_64-pc-windows-gnu` green if any Rust file touched.
- [ ] Windows release build via `scripts/sync-and-build.ps1`.
- [ ] Manual smoke: Hold + Toggle × comfortable + wide — no first-chunk top-clip (R3/R4).
- [ ] Drag mid-recording while chunks arrive — no teleport (R5).
- [ ] Finish recording — done-pop clears preview, no stale bleed into next cycle (R1/R7).
- [ ] Corner has no white line at the panel radius (R10/R11).
- [ ] `docs/surface-smoke-checklist.md` relevant items walked.

---

_Generated by `document-project` workflow (deep-dive mode)_
_Base Documentation: docs/index.md_
_Scan Date: 2026-06-05_
_Analysis Mode: Exhaustive (literal full-file review of owned files; bar-relevant regions of coupling files)_
