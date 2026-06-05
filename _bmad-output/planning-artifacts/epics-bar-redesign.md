---
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics", "step-03-create-stories"]
status: stories-defined
inputDocuments:
  - docs/bar-redesign-spec.md          # Soll-Spec + Foundation Design (the codeable contract)
  - docs/deep-dive-bar-subsystem.md    # Ist-Zustand + the 13-item race table this retires
  - _bmad-output/project-context.md
  - docs/surface-smoke-checklist.md     # surface-class DoD control
trackType: brownfield-feature
featureEpic: 6
note: >
  Re-architecture epic, not a new feature. Separate planning artifact: epics.md is the CLOSED
  remediation breakdown (Epics 1-4); epics-live-preview.md is Epic 5 (the live-preview feature).
  Epic 6 RE-ARCHITECTS the bar so the geometry race class cannot exist by construction. The
  Epic-5 retro's "no Epic 6" predates this need and is explicitly reversed (four failed
  single-window geometry fixes are the trigger). No PRD/Architecture/UX doc — the requirements
  source is docs/bar-redesign-spec.md (foundation design, grounded in the deep-dive audit).
  Shares the sprint-status.yaml ledger. Per-story full context via bmad-create-story per session.
  Supersedes the carried-forward Story 5-7 grow-upward clip blocker (parked → folded into 6-5).
---

# klarvo - Epic Breakdown (Floating Bar Re-Architecture · Epic 6)

## Overview

Four geometry fix-attempts on the single-window FloatingBar failed (the last made it worse and was
reverted). Root cause, confirmed by the deep-dive (`docs/deep-dive-bar-subsystem.md`): the bar is
**one window** that re-measures → resizes → reshapes → repositions itself on **every preview chunk**
via independent async IPC. The races **R3/R4/R5/R6/R10/R11** are six faces of that single design
choice; point-guards cannot make per-chunk async geometry atomic.

This epic **re-designs** the surface (it does not refactor it):

1. The **pill becomes fully static** — fixed size, never resizes; the width-preset no longer affects
   it.
2. The **live preview moves into its own transparent, click-through `"preview"` window** above the
   pill, **centered**, growing **upward**.
3. The preview window is created **once at its full limit height**; the dark card grows **via pure
   CSS** inside it and scrolls past the cap — so there is **no per-chunk window IPC at all** (NFR1).
   This eliminates the entire geometry race class **by construction**.
4. Preview presentation is driven by a **single scale factor** `k = fontPx / 11`: width presets and
   the height limit are defined at the small font and scale with the chosen font size.

**FRs covered:** FR1–FR10 · **NFRs:** NFR1–NFR6 · **AR:** AR1–AR5 · **UX:** UX-DR1

## Requirements Inventory

Brownfield re-architecture: requirements are extracted from `docs/bar-redesign-spec.md`. IDs are
native to this epic.

### Functional Requirements

- **FR1** — The pill (`"bar"` window) is **static**: one fixed size, it and its elements never resize;
  the width-preset (Compact/Comfortable/Wide) no longer affects it.
- **FR2** — Live preview renders in a **separate `"preview"` window** above the pill: transparent,
  click-through, always-on-top, decorationless, skip-taskbar.
- **FR3** — The preview window is **horizontally centered over the pill** and stretches **upward**.
- **FR4** — The preview card **grows with the text up to a fixed height limit**, then **scrolls**
  inside (top-fade). The limit is independent of the width preset.
- **FR5** — The **width preset** (Compact/Comfortable/Wide) affects **only** the preview width.
- **FR6** — A **new font-size setting** (Small/Medium/Large, default Small) affects **only** the
  preview; persisted as `preview_font_size`.
- **FR7** — Width, height limit, and font **scale together** by `k = fontPx / 11` (the scale-factor
  model). Widths 260/320/400 and height limit 600 are defined at the small font.
- **FR8** — The preview window **appears only while recording** when preview text is present and
  **hides when recording ends**.
- **FR9** — The preview **follows the pill on drag** (via `klarvo://bar-moved`); only the **pill**
  position is persisted, the preview position is always derived.
- **FR10** — **Recovery for both windows** (`ensure_bar_window` + new `ensure_preview_window`).

### NonFunctional Requirements

- **NFR1** *(the race-class killer)* — **No per-chunk window IPC.** Window size, position, and region
  are set **once per show / once per drag**, never per preview chunk. Inversion: any per-chunk
  `setSize`/`setPosition`/region call re-introduces R3/R4/R5/R10.
- **NFR2** — **Behavior-preserving** for the pill state machine, waveform, mode badge, recovery, and
  the **final pasted output** (Variant B carried over — preview never feeds output). Only geometry
  and preview presentation change.
- **NFR3** — **Retain the legitimate guards**, do not regress them: R1 stale-chunk (frontend), R2
  backpressure (backend), R7 done→idle, R8/R9 backend offline/leak guards.
- **NFR4** — Tauri event names use the **colon form**; the new event is `klarvo://bar-moved`.
- **NFR5** — Config key **camelCase** (`previewFontSize`); writes via the sanctioned single-writer
  atomic path (`save_config_locked`, ADR-0015); missing-field default = `small` (no migration write).
- **NFR6** — **Windows-only surface.** Linux is near-zero signal: a real Windows release build +
  manual press-to-paste smoke is the hard gate; walk `docs/surface-smoke-checklist.md`.

### Additional Requirements (from the deep-dive audit)

- **AR1** — A **third window label `"preview"`** + Rust `create_preview_window` + `main.tsx` routing
  (`main` → App, `bar` → FloatingBar, `preview` → new `PreviewPanel`).
- **AR2** — The preview window is **fixed at the clamped max height**; the card is **bottom-aligned
  and grows via CSS**, scrolls past the cap.
- **AR3** — Max-height clamp = `min(BASE_MAX_HEIGHT × k, (bar_y − GAP) − (screenTop + 12))`, computed
  at **show-time and on drag** (not per chunk).
- **AR4** — The pill **loses all geometry/measure logic** (`panelHeight`, `panelScrolls`, `geomTick`,
  `measureRef`, `previewPanelRef`, the measure `useLayoutEffect`, the per-chunk grow/resize effect,
  the panel render).
- **AR5** — Remove `set_bar_shape("panel")` and the per-preset `screenCap` remnants; the pill region
  is set **once** at creation.

### UX Design Requirements

- **UX-DR1** — Preview grows upward, centered above the pill; transparent empty area above the card;
  top-fade when scrolling; click-through (display-only, no controls).

### FR Coverage Map

| FR/NFR/AR | Story |
|---|---|
| FR2 (shell), FR10, AR1, NFR6 | 6.1 |
| FR2, FR3, FR4, FR5, FR8, FR7(width), NFR1, NFR2, NFR3(R1), AR2, AR3, UX-DR1 | 6.2 |
| FR6, FR7(full), NFR5 | 6.3 |
| FR9, NFR4 | 6.4 |
| FR1, NFR2, AR4, AR5 | 6.5 |

## Epic List

### Epic 6: Floating Bar Re-Architecture

The user dictates with the bar exactly as today, but the **pill never moves or resizes**, and the
**live preview is a separate window above it** that grows upward and caps+scrolls. The win is
structural: the geometry is **static per recording**, so the race class that broke four fix-attempts
**cannot exist**. Standalone: builds only on existing v1 recording/pipeline/FloatingBar surfaces;
enables no future epic but retires the bar's geometry debt and unblocks the parked 5-7.

**FRs covered:** FR1–FR10 · **NFRs:** NFR1–NFR6 · **AR:** AR1–AR5 · **UX:** UX-DR1

**Planned story decomposition** (full ACs below):

- **6.1 — Scaffold the standalone `"preview"` window** *(Wave 1, foundation)*. Rust
  `create_preview_window` + handler registration + `ensure_preview_window` recovery; `main.tsx`
  routes `"preview"` → empty `PreviewPanel`; transparent/click-through/always-on-top/hidden at
  startup. Covers FR2(shell), FR10, AR1, NFR6.
- **6.2 — Move live preview into the window (CSS-grow, scale geometry)** *(depends on 6.1)*. The big
  one. `PreviewPanel` subscribes to `state-changed` + `live-preview-chunk` (R1 guard retained);
  card grows upward via CSS inside the fixed-max window, caps+scrolls; geometry via `previewGeometry`
  (Small font + width presets); centered above the pill + screen clamp; show-once / hide-on-end. The
  pill's in-window preview is **disabled here** → exactly one preview. Covers FR2, FR3, FR4, FR5, FR8,
  FR7(width), NFR1, NFR2, NFR3(R1), AR2, AR3, UX-DR1.
- **6.3 — Font-size axis** *(depends on 6.2)*. New `preview_font_size` config + 3-way Settings picker;
  width + height + font scale by `k`. Covers FR6, FR7(full), NFR5.
- **6.4 — Couple the preview to pill drag** *(depends on 6.2)*. Pill emits `klarvo://bar-moved`;
  preview re-centers; only pill position persisted. Covers FR9, NFR4.
- **6.5 — Pill fully static + cleanup** *(depends on 6.2 + 6.4)*. Delete the dead grow code; pill one
  fixed size, region once; clipboard-done re-laid-out to fit the fixed width; remove
  `set_bar_shape("panel")` + screenCap remnants; reconcile/park 5-7. Covers FR1, NFR2, AR4, AR5.

**Dependency flow:** 6.1 → 6.2; **6.3 and 6.4 parallel after 6.2**; **6.5 after 6.2 + 6.4**. No story
depends on a later story. After **6.2** the geometry race class (R3/R4/R5/R6/R10/R11) is already gone.

**Working decisions (defaults from the spec, confirmable at story time):** pill stays at width 200
with the clipboard-done state re-laid-out to fit (not widening to 220); the preview window is created
hidden at startup; drag-follow is live via throttled `bar-moved` (fallback: snap on drag-end).

## Epic 6: Floating Bar Re-Architecture

The pill behaves as today but is fully static; the live preview is a separate transparent window
above the pill that grows upward and caps+scrolls — with the geometry set once per recording so the
race class cannot exist.

### Story 6.1: Scaffold the standalone "preview" window

As a developer re-architecting the bar,
I want a standalone transparent, click-through `"preview"` window created, routed, and recoverable,
So that live preview can render in its own window fully decoupled from the pill.

**Acceptance Criteria:**

**Given** the app starts on Windows
**When** the `setup` closure runs
**Then** a second WebView window labeled `"preview"` is created **hidden**, transparent, decorationless,
always-on-top, skip-taskbar, no-shadow, and **click-through** (ignores cursor events), mirroring the
bar's creation flags via a new `create_preview_window` helper.

**Given** both the `"bar"` and `"preview"` windows exist
**When** `main.tsx` evaluates the current window label
**Then** `"preview"` routes to a new `PreviewPanel` component (alongside `"main"` → App, `"bar"` →
FloatingBar); `PreviewPanel` renders only `RESET_CSS` for now (no content).

**Given** the preview window vanished or is unresponsive
**When** `ensure_preview_window` is invoked
**Then** it probes `get_webview_window("preview").is_visible()` and recreates via
`create_preview_window` when missing/unresponsive (mirrors `ensure_bar_window`), returning `true` when
recreated; desktop-only, registered in the `invoke_handler`.

**Given** the scaffolded window
**When** it is shown at a fixed test size/position and then hidden
**Then** it shows, positions, and hides with **no white-line / shape artifact** (verified in the
Windows smoke).

**DoD:** Real Windows release build + manual smoke (show/position/hide, no artifact).
`cargo check --target x86_64-pc-windows-gnu` green. Linux `cargo test` for compile + command
registration. Walk `docs/surface-smoke-checklist.md` (window-geometry/region, event push-wiring).

### Story 6.2: Move live preview into the window (CSS-grow, scale geometry)

As a user dictating with preview enabled,
I want the live text to appear in a window above the pill that grows upward and caps + scrolls,
So that I can read along while the pill never moves or resizes.

**Acceptance Criteria:**

**Given** preview is enabled and a recording is active in Toggle/Hold
**When** the **first** `klarvo://live-preview-chunk` of the cycle arrives
**Then** `PreviewPanel` computes `previewGeometry(widthPreset, "small")` with the screen clamp (AR3),
sets the window **size + rounded-rect region (radius 14) + position** (centered over the pill, bottom
edge `GAP` above the pill top) **once**, and shows the window.

**Given** preview chunks continue to arrive
**When** each chunk is appended
**Then** the dark card grows **upward via CSS** within the static window (bottom-aligned) and **no**
`setSize`/`setPosition`/region call is issued per chunk (NFR1).
**And** inversion: re-introducing a per-chunk window resize brings back the cold-expansion / pre-measure
clip (R3/R4) — proving the static-window invariant is load-bearing.

**Given** the accumulated text exceeds the window's max height
**When** further chunks arrive
**Then** the **inner** text area scrolls to the newest line with a **top-fade**; the window itself does
not grow further.

**Given** a chunk arrives after the recording ended (stale/out-of-cycle)
**When** the listener fires
**Then** the `isRecordingRef` guard (R1) drops it — no text bleeds into the next cycle.
**And** inversion: removing the guard lets a stale post-done chunk repopulate the panel.

**Given** the recording ends (`done`/`idle`/`error`)
**When** `klarvo://state-changed` fires
**Then** `PreviewPanel` clears the accumulated text and hides the preview window.

**Given** the width preset is Compact/Comfortable/Wide
**When** the preview opens
**Then** only the **preview width** changes (260/320/400 at the small font); the **pill is unaffected**
(FR5).

**Given** the pill is in any active state
**When** preview is active
**Then** the pill window is **not resized** — the old in-pill grow path is disabled in this story, so
exactly one preview surface exists (the new window). (Dead pill code is deleted in 6.5.)

**DoD:** Real Windows release build + manual smoke — text grows upward, caps + scrolls, centered above
a **non-resizing** pill, no top-clip on the first chunk. `tsc`/`vite` + `cargo check` win-target green.
Walk `docs/surface-smoke-checklist.md` (separate-window reactivity, geometry/region clip, event wiring).

### Story 6.3: Font-size axis (preview_font_size + Settings picker + k-scaling)

As a user,
I want to choose among three preview font sizes in Settings,
So that the preview is readable at my preferred size with the whole box scaling proportionally.

**Acceptance Criteria:**

**Given** a fresh config (field absent)
**When** the schema is loaded
**Then** `preview_font_size` reads its serde default `"small"` (camelCase key `previewFontSize`) with
**no** migration write; round-trip + missing-field + camelCase tests assert this.

**Given** the Settings live-preview section
**When** the user picks Small / Medium / Large
**Then** the value persists via `save_config_locked` (ADR-0015) and the picker reflects the saved value.

**Given** a font size is chosen
**When** the preview next opens
**Then** font (11/13/15), width, and height limit all scale by `k = fontPx / 11` (1.0 / 1.18 / 1.36);
`PreviewPanel` reads the setting **reactively** (re-read on open / backend event — separate-window
rule), never frozen at app-start.
**And** inversion: hard-coding `k = 1` leaves Medium/Large visually identical to Small → RED.

**DoD:** Windows settings-smoke (pick a size → camelCase key in `config.json` → preview reflects it on
next open) + config tests. `tsc`/`vite` + `cargo check` win-target green.

### Story 6.4: Couple the preview to pill drag

As a user,
I want the preview to stay centered above the pill while I drag it,
So that the two always move together.

**Acceptance Criteria:**

**Given** a recording with the preview window open
**When** the user drags the pill
**Then** the pill emits `klarvo://bar-moved` `{x, y}` (throttled to an animation frame + once on
drag-end; colon form, NFR4).

**Given** the preview window is open
**When** `klarvo://bar-moved` fires
**Then** `PreviewPanel` re-centers via `setPosition` **only** (no resize), using the new pill anchor +
the screen clamp.

**Given** the drag ends
**When** the final position is computed
**Then** **only the pill** position is persisted (`save_bar_position`); the preview position is always
derived from the pill anchor.

**Given** preview is closed (not recording)
**When** the pill is dragged
**Then** no preview repositioning occurs (no preview window to move).

**DoD:** Real Windows release build + manual smoke — drag during recording keeps the preview centered
above the pill, no teleport, no resize. `tsc`/`vite` + `cargo check` win-target green.

### Story 6.5: Pill fully static + cleanup

As a developer,
I want the dead grow logic removed and the pill made truly static,
So that the codebase reflects the new foundation with no resize paths left and 5-7 is reconciled.

**Acceptance Criteria:**

**Given** `FloatingBar.tsx` after 6.2/6.4
**When** the dead grow code is removed
**Then** `livePreview`, `panelHeight`, `panelScrolls`, `geomTick`, `measureRef`, `previewPanelRef`,
`prevIsPanelOpenRef`, the measure `useLayoutEffect`, the per-chunk grow/resize effect, the panel render
block, and `setBarShape("panel")` are all deleted (AR4); the pill `show` effect is **show/hide +
position only** (no resize).

**Given** the pill window
**When** it is created
**Then** it is one fixed size (PILL_WIDTH × PILL_HEIGHT) with its pill region set **once** at creation;
it is **never resized** in any state (recording/processing/done/clipboard/error) (FR1).

**Given** the clipboard-done state ("In Clipboard")
**When** it is shown
**Then** it fits within the fixed pill width (200) — re-laid-out (icon + compact text) rather than
widening the window to 220 (FR1).

**Given** the cleanup is complete
**When** the build runs
**Then** the unused `set_bar_shape` "panel"/shape path and the per-preset `screenCap` remnants are
removed (AR5); lib tests green; `clippy` no-new; no dead code.

**Given** Story 5-7 (parked, `review`)
**When** Epic 6 lands
**Then** 5-7 is reconciled: its R1 stale-chunk guard and R2 backpressure live on in the new design;
5-7 is marked superseded/parked in `sprint-status.yaml` (it no longer needs the single-window smoke).

**DoD:** Real Windows release build + manual smoke — the pill never resizes in any state; preview still
works end-to-end. Lib tests green + `clippy` clean on touched files. `tsc`/`vite` + `cargo check`
win-target green.

---

_Epic 6 planning artifact. Codeable contract: `docs/bar-redesign-spec.md`. Ist-Zustand:
`docs/deep-dive-bar-subsystem.md`. Per-story full context via `bmad-create-story` per session._
