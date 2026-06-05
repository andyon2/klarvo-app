# Floating Bar Re-Architecture — Spec & Foundation Design

**Date:** 2026-06-05
**Status:** Soll-Spec + Foundation Design — ready for epic/story breakdown
**Ist-Zustand reference:** `docs/deep-dive-bar-subsystem.md` (the 13-item race table this design retires)
**Supersedes:** carried-forward Story 5-7 grow-upward clip blocker (parked → folded here)

> **Why this exists:** Four geometry fix-attempts on the single-window bar failed (the last made it
> worse and was reverted). Root cause: the bar is ONE window that re-measures → resizes → reshapes →
> repositions itself on **every preview chunk** via independent async IPC. The races R3/R4/R5/R6/R10
> are five faces of that one design choice; point-guards cannot make per-chunk async geometry atomic.
> This is a **re-design**, not a refactor: separate the preview into its own window and make the
> geometry **static per recording** so the race class cannot exist by construction.

---

## 1. Capabilities (Soll-Zustand)

### Pill bar (`"bar"` window) — becomes fully static
- Centered above the taskbar, as today. **Fixed size — never resizes, neither it nor its elements.**
- The width-preset setting (Compact/Comfortable/Wide) **no longer affects the pill.**
- Shows the same states: idle (hidden) · recording (waveform + stop + mode badge) · processing
  (spinner) · done (+ clipboard variant) · error.
- Draggable; its position is the single persisted anchor (`bar_x`/`bar_y`).

### Preview (`"preview"` window) — NEW, separate window above the pill
- Its own always-on-top, transparent, **click-through** (display-only) window.
- Horizontally **centered over the pill**, stretches **upward**.
- Appears only while recording when preview text is present; hides when recording ends.
- The dark card **grows upward with the text** up to a **fixed height limit**, then **scrolls**
  inside (top-fade) — the limit is independent of the width preset.
- Follows the pill when the pill is dragged.

### Settings — preview-only, two axes
- **Width preset** (existing `preview_panel_form`): Compact / Comfortable / Wide.
- **Font size** (NEW `preview_font_size`): Small / Medium / Large.
- Both affect **only the preview**, never the pill.

### Non-goals (unchanged)
- Preview is **orientation, not accuracy** — it never feeds the pasted output (Variant B).
- No telemetry. Windows-desktop surface (the bar concept is Windows-only; Android uses the native
  overlay bubble).

---

## 2. The scale-factor geometry model (the clever unification)

The **font size is the single scalar.** Width presets and the height limit are defined once at the
small font and scale by `k = fontPx / 11`. One formula drives all three axes.

```ts
// The ONLY tuned source: 3 font sizes. small = today's look.
const FONT_PX = { small: 11, medium: 13, large: 15 };
const k = (size) => FONT_PX[size] / FONT_PX.small;        // 1.0 / 1.18 / 1.36

// Defined at the small font — scale by k:
const BASE_WIDTH      = { compact: 260, comfortable: 320, wide: 400 };
const BASE_MAX_HEIGHT = 600;

function previewGeometry(widthPreset, fontSize) {
  const f = k(fontSize);
  return {
    fontPx:    FONT_PX[fontSize],
    width:     Math.round(BASE_WIDTH[widthPreset] * f),
    maxHeight: Math.round(BASE_MAX_HEIGHT      * f),   // clamped to screen at show-time (§4)
  };
}
```

**Confirmed values:** font Small 11 (= today) / Medium 13 / Large 15, default **Small**; widths @small
260 / 320 / 400; height limit @small **600** (→ 600 / 709 / 818 at the three steps).

**Why it's clean:** width, height, and font all scale by the same `k`, so the box is **proportionally
identical** at every font size (no stubby lines at large font; max visible line-count ≈ constant ~36
lines at every step). You tune one number, not nine combinations.

---

## 3. Foundation design — why the race class cannot exist

### 3.1 The key trick: a static window, a CSS-growing card

> The preview window is created **once** at its **full limit height** (`maxHeight`), **transparent**,
> with its **bottom edge fixed just above the pill**. The dark rounded card is **bottom-aligned** and
> grows **upward inside** this static window via **pure CSS** as text accumulates. Past `maxHeight`
> the inner text area **scrolls**.

Consequence: **zero per-chunk window IPC.** No measure→resize feedback loop, no per-chunk
setSize/setPosition/setRegion. Per recording the only window calls are: **show once** (sized +
positioned + region applied with final values) and **hide once**. Everything between is CSS.

### 3.2 Window topology

| Window | Lifecycle | Size | Shape/region | Interaction |
|---|---|---|---|---|
| `"bar"` (pill) | created hidden at startup (Windows); shown on activity, hidden on idle | **fixed** PILL_WIDTH × PILL_HEIGHT; **never resized** | pill region set **once** at creation | draggable |
| `"preview"` (NEW) | created hidden at startup; shown on first chunk during recording, hidden on end | set **once per show** = `width × clampedMaxHeight` | rounded-rect region (radius 14) set **once per show** | **click-through** |

### 3.3 Positioning math (centered above pill, grows up)

```
pillCenterX = bar_x + PILL_WIDTH / 2
W           = previewGeometry.width
H           = clampedMaxHeight                      // see clamp below
GAP         = 8                                     // px between preview bottom and pill top

preview.left = clamp(pillCenterX - W/2, screenLeft + 12, screenRight - W - 12)
preview.top  = bar_y - GAP - H
// screen clamp so it never runs off the top:
clampedMaxHeight = min(previewGeometry.maxHeight, (bar_y - GAP) - (screenTop + 12))
```

The clamp reads the screen/pill geometry **at show-time and on drag** — once, not per chunk → still
atomic, still race-free.

### 3.4 State & event model (fully decoupled windows)

Both windows are separate React mounts that talk only to the backend (no shared JS). The **preview
window subscribes to backend events directly** — it does not depend on the pill:

- `klarvo://state-changed` → on `recording`: clear text, arm; on `done`/`idle`/`error`: hide + clear.
- `klarvo://live-preview-chunk` → **keep the stale-chunk guard** (`isRecordingRef`, R1): append; on
  the **first** chunk of a cycle, compute geometry (read `get_bar_position` + screen), `setSize` +
  region + `setPosition` + `show` — once.
- `klarvo://bar-moved {x,y}` (NEW) → re-center (`setPosition` only) so the preview follows a drag.

The **pill** keeps its state machine, waveform, mode badge, drag, show/hide, recovery — and emits
`klarvo://bar-moved` on drag (throttled to a frame + on drag-end). It **loses all geometry/measure
logic.**

### 3.5 Drag coupling

Drag stays on the pill (manual mouse drag — Tauri native drag is unreliable on transparent WebView2).
On drag the pill emits `klarvo://bar-moved`; the preview re-centers via `setPosition`. Only the **pill**
position is persisted (`save_bar_position`); the preview position is always derived.

### 3.6 What is removed from `FloatingBar.tsx`

Deleted: `livePreview`, `panelHeight`, `panelScrolls`, `geomTick`, `measureRef`, `previewPanelRef`,
`prevIsPanelOpenRef`, the measure `useLayoutEffect`, the per-chunk grow/resize geometry effect, the
panel render block, `setBarShape("panel")`, the panel-form re-read. The pill `show` effect collapses
to **show/hide + position only** (no resize). The 80×10 "idle thin" shape is obsolete (idle = hidden).

### 3.7 Race-class disposition

| Race (from deep-dive) | New status | How |
|---|---|---|
| R3 cold first-expansion | **eliminated** | pill never resizes; preview sized once per show |
| R4 pre-measure transient | **eliminated** | no measure→resize loop; card grows via CSS |
| R5 drag-vs-chunk reposition | **eliminated** | no per-chunk setPosition; preview moves only on show + bar-moved |
| R6 grow-upward viewport lag | **eliminated** | fixed window; CSS card + inner scroll, no window-resize-vs-viewport race |
| R10 show-before-shape | **eliminated as a class** | the show sequence runs **once** with final values, not racing chunks |
| R11 region/CSS radius mismatch | **trivial** | region radius 14 set once, matches static CSS card |
| R1 stale chunk | **retained** (still valid) | `isRecordingRef` guard kept in the preview window |
| R2 preview backpressure | **retained** | backend `try_acquire_preview_slot` unchanged |
| R7 done→idle Auto-Loop | **retained** | pill state machine unchanged |
| R8/R9 config-leak / provider-swap | **retained** | backend guards unchanged |
| R12 vanished window | **retained, extended** | recovery for BOTH windows (`ensure_bar_window` + new `ensure_preview_window`) |
| R13 boot warning lost | **unchanged** | unrelated to the bar geometry |

The **entire geometry race class (R3/R4/R5/R6/R10/R11) is gone by construction.** The retained guards
are legitimate and orthogonal.

---

## 4. Config & Settings changes

- **NEW** `AppConfig.preview_font_size: String` — serde `#[serde(default = ...)]` → `"small"`,
  camelCase key `previewFontSize` (⚠️ camelCase trap). Round-trip + missing-field + camelCase tests.
- **Existing** `preview_panel_form` (width preset) — unchanged on disk; **consumers change** (pill
  stops reading it; preview reads both it and `preview_font_size`, reactively per separate-window
  rule).
- **Settings UI:** add a 3-way Font-Size picker (Small/Medium/Large) in the live-preview section,
  alongside the existing width-preset picker. Writes via sanctioned `save_config_locked` (ADR-0015).
- **Possibly removable:** `set_bar_shape` (pill shape now set once at creation) and the per-preset
  `screenCap` remnants — cleaned up in the final story.

---

## 5. Story breakdown — Epic 6: Floating Bar Re-Architecture

> Placement: a **new Epic 6** (the Epic-5 retro's "no Epic 6" predates this need — explicitly
> reversed). Each story leaves a working app. Surface-class → real **Windows release build + manual
> smoke** in every DoD (Linux is near-zero signal here). Walk `docs/surface-smoke-checklist.md`.

| Story | Title | Outcome | Depends on |
|---|---|---|---|
| **6-1** | Scaffold the standalone `"preview"` window | Rust `create_preview_window` + handler registration + `ensure_preview_window` recovery; `main.tsx` routes label `"preview"` → new `PreviewPanel` stub; window is transparent, click-through, always-on-top, skip-taskbar, no-shadow, created hidden at startup. Renders nothing yet. **DoD:** Windows smoke — window can be shown/positioned/hidden at a fixed spot. | — |
| **6-2** | Move live preview into the window (CSS-grow, scale geometry) | `PreviewPanel` subscribes to `state-changed` + `live-preview-chunk` (with the R1 stale-chunk guard); accumulates text; renders the bottom-aligned card that grows upward via CSS inside the fixed `maxHeight` window, scrolls + top-fade past the cap; geometry from `previewGeometry` (Small font, width presets); centered above the pill (reads `get_bar_position` + screen clamp); show-once on first chunk, hide on end. **Pill's in-window preview is disabled here** (pill stops growing) so there is exactly one preview. **DoD:** Windows smoke — text grows upward, caps + scrolls, centered, no clip; pill no longer resizes for preview. | 6-1 |
| **6-3** | Font-size axis: `previewFontSize` config + Settings picker + k-scaling | New config key + 3-way picker; preview reads it reactively; width + height limit + font all scale by `k`. **DoD:** config round-trip/camelCase tests + settings smoke; preview reflects a font change on next open (separate-window reactivity). | 6-2 |
| **6-4** | Couple the preview to pill drag | Pill emits `klarvo://bar-moved` (throttled + on drag-end); preview re-centers via `setPosition`; only pill position persisted. **DoD:** Windows smoke — dragging during recording keeps the preview centered above the pill. | 6-2 |
| **6-5** | Make the pill fully static + cleanup | Delete the dead grow code from `FloatingBar.tsx`; pill created at one fixed size, region set once, never resized; the clipboard-done state re-laid-out to fit the single fixed width; remove `set_bar_shape("panel")` / unused shape command + per-preset screenCap remnants; reconcile/park 5-7 (its R1/R2 guards live on in the new design). **DoD:** Windows smoke — pill never resizes in any state; tests green; no dead code. | 6-2, 6-4 |

**Sequence:** 6-1 → 6-2 → { 6-3, 6-4 in parallel } → 6-5.

---

## 6. Open / minor decisions (defaults chosen — flag if you disagree)

1. **Pill fixed width incl. clipboard-done.** Today the "In Clipboard" done state widens the pill to
   220. To make the pill truly static, default = **keep pill at 200** and re-lay-out the clipboard
   state to fit (icon + compact text). Alternative: pill always 220.
2. **Preview window creation:** created **hidden at startup** (mirrors the bar, simplest recovery) vs.
   lazily on first use. Default = startup-hidden.
3. **Drag-follow smoothness:** preview follows via throttled `bar-moved`. If per-frame feels heavy,
   fall back to "re-center on drag-end only" (preview lags then snaps).
4. **Epic placement:** new **Epic 6** vs. continuing Epic 5. Default = Epic 6 (re-architecture, not a
   feature addition).

---

_Foundation design for the bar re-architecture. Ist-Zustand: `docs/deep-dive-bar-subsystem.md`._
