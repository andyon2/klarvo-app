---
status: ready-for-dev
trackType: brownfield-architecture-migration
featureEpics: [10]
inputDocuments:
  - docs/adr/0021-native-desktop-overlays.md          # binding architecture decision + proof + sub-decisions
  - src/FloatingBar.tsx                                # appearance SOLL for the native pill (current approved look)
  - src/PreviewPanel.tsx                               # appearance SOLL for the native preview
  - docs/design/overhaul/SPEC-studio-dark-overhaul.md  # directional token source (NOT a re-skin mandate here)
  - docs/bar-redesign-spec.md                          # pill geometry / positioning math
  - docs/deep-dive-bar-subsystem.md                    # Ist-Zustand of the bar subsystem
  - _bmad-output/project-context.md                    # code rules (camelCase, platform gates, surface DoD)
  - docs/surface-smoke-checklist.md                    # surface-class DoD control
note: >
  Mini-epic: replace the two transparent always-on-top desktop overlays (pill + preview) — which
  go blank when occluded because the WebView2 compositor halts (4 transient fixes, see ADR-0021) —
  with native Win32 layered windows drawn from Rust. Architecture migration, NOT a re-skin: the
  native overlays reproduce the CURRENT approved look 1:1; the parked Epic 8 Studio-Dark re-skin is
  out of scope here. Proof-first: pill (Story 10-1) ships first and validates the substrate; preview
  (Story 10-2) reuses it. Branch feat/native-desktop-overlays off v1-ship. Per-story full context via
  bmad-create-story. DoD split: occlusion = machine-verified (harness in ADR-0021); appearance =
  Andi smoke on real Windows.
---

# klarvo — Epic 10: Native Desktop Overlays

## Overview

The pill (`bar`, 200×36) and live-preview (`preview`) overlays go blank whenever a foreground window
covers their screen region — the core "Pille unsichtbar in anderen Apps" blocker. Root cause
(measured, see [ADR-0021](../../docs/adr/0021-native-desktop-overlays.md)): the occlusion-present
halt lives inside the WebView2/Chromium compositor and is not fixable by any flag or runtime version
(four transient fixes confirmed it). A native layered topmost window stays fully composited when
occluded (proven 7600/7600 + dwell). Decision: render both overlays as native Win32 layered windows
from Rust.

Two stories, separated by risk and human-test surface:

- **Story 10-1 — Native pill** (the proof slice): all the hard primitives live here (layered-window
  substrate, per-pixel-alpha present, RMS waveform, state rendering, drag). Validates the whole
  approach before the preview reuses it.
- **Story 10-2 — Native preview** (reuses the substrate): scrollable text card, click-through,
  pill-anchored, grow-up.

This is an **architecture migration, not a re-skin** — each native overlay reproduces the *current*
`FloatingBar.tsx` / `PreviewPanel.tsx` appearance 1:1. The parked Epic 8 Studio-Dark re-skin is
explicitly out of scope.

## Requirements Inventory

Categories: **AR** = architecture/substrate, **VR** = visual fidelity (against the current render),
**IR** = interaction parity, **NFR** = non-functional / DoD.

- **AR1** — The pill and preview are native Win32 top-level windows
  (`WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`; preview adds
  `WS_EX_TRANSPARENT`), content presented via `UpdateLayeredWindow(ULW_ALPHA)` from a top-down 32bpp
  premultiplied-BGRA DIB, CPU-rasterized. No GPU/swapchain/DirectComposition. Windows-only.
- **AR2** — The native overlays are driven by the Rust pipeline state + RMS **in-process** (no
  dependency on `klarvo://` events for these two windows). The existing emitters may remain for other
  consumers.
- **AR3** — As each native overlay lands, its WebView2 window, its `main.tsx` label route, and its
  React entry point (`FloatingBar.tsx` / `PreviewPanel.tsx`) are removed. The `main` (settings)
  window stays WebView2.
- **AR4** — ADR-0020's runtime-pin machinery (bundled runtime + `sync-and-build.ps1` self-heal) is
  retired once **both** overlays are native (tracked in 10-2's DoD; do not remove early — the main
  window unaffected, but the overlay driver is what justified it).
- **VR1** — Each pill state renders 1:1 with the current `FloatingBar.tsx` look: idle (hidden),
  recording (pill + 5-bar teal `#2AC3A8` waveform + stop affordance), transcribing/cleaning (amber
  `#FFA344` spinner + label), done (green `#4ADE80` check / clipboard amber), error (red `#FF7369`).
  Rounded-pill shape, ~96%-opaque dark fill. **No** Studio-Dark re-skin.
- **VR2** — The preview renders 1:1 with the current `PreviewPanel.tsx` look: dark card, scrollable
  cleaned text, bottom-aligned grow-up, top-fade when scrolled, teal hairline border.
- **VR3** — Backdrop blur is dropped (ADR-0021 sub-decision 3); the near-opaque fill makes this
  negligible. If Andi's smoke flags it, real blur is a separate follow-up.
- **IR1** — Pill drag-to-move + position persistence via the existing `config.bar_x/bar_y`
  (`save_bar_position` / `get_bar_position`), restored on next start.
- **IR2** — Preview is click-through (`WS_EX_TRANSPARENT`), anchored to the pill, repositions when
  the pill is dragged.
- **NFR1** — Occlusion-survival is **machine-verified** per story via the ADR-0021 harness (content
  pixels remain while a foreground app is maximized over the region, incl. 3 s dwell).
- **NFR2** — Visual fidelity is **Andi's smoke** on a real Windows release build. Never claimed from
  machine output.
- **NFR3** — No regression to the recording pipeline, hotkeys, paste, or the `main` window.

---

## Story 10-1 — Native pill (FloatingBar) overlay

**As** a Klarvo user dictating into another app,
**I want** the recording pill to stay visible when that app covers its spot,
**so that** I can always tell whether recording is active — permanently, not until the next restart.

### Acceptance Criteria

**AC-1 — Native layered window replaces the WebView2 `bar`:**
Given the app starts on Windows
When the pill is created (where `create_bar_window` is called today)
Then a native Win32 top-level window with `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW |
WS_EX_NOACTIVATE` is created at the saved/default pill position, sized 200×36, its content presented
via `UpdateLayeredWindow(ULW_ALPHA)` from a premultiplied-BGRA DIB
And the WebView2 `"bar"` window, its `main.tsx` `"bar"` route, and `src/FloatingBar.tsx` are removed
And the change is gated `#[cfg(target_os = "windows")]`

**AC-2 — All pill states render natively, matching the current look 1:1:**
Given the recording pipeline drives the pill state
When the state is one of idle / recording / transcribing / cleaning / done / error
Then the native pill renders the **current** `FloatingBar.tsx` appearance for each state — idle
hidden; recording = rounded pill + 5-bar teal (`#2AC3A8`) waveform + stop affordance; transcribing &
cleaning = amber (`#FFA344`) spinner + label; done = green (`#4ADE80`) check (or amber clipboard when
paste failed); error = red (`#FF7369`) — with the rounded-pill shape and ~96%-opaque dark fill
And **no** Studio-Dark re-skin is introduced (this is a tech migration; colors/shape mirror today)

**AC-3 — Waveform is RMS-driven in-process:**
Given recording is active
When RMS amplitude updates arrive on the existing `set_level_callback` path (~15 Hz)
Then the native pill's waveform updates directly in-process (no `klarvo://audio-level` JS round-trip
required), using the same mapping as today (`pow(min(1, level*10), 0.4)`, noise floor `0.006`, 5 bars,
12% floor) so the visual response matches the current pill

**AC-4 — Drag-to-move + position persistence (parity):**
Given the native pill is visible
When the user drags it
Then it follows the cursor, and the new position persists via `config.bar_x/bar_y`
(`save_bar_position`) and is restored on next start — behavioural parity with the WebView2 pill

**AC-5 — Occlusion-survival, machine-verified (the whole point):**
Given the native pill is visible during recording
When a foreground app is maximized over its screen region, and again after a 3 s dwell
Then the pill stays fully painted (content pixels > 0, ≈100% of the region) — verified by the
ADR-0021 occlusion harness; this is the exact scenario where the WebView2 pill measured 0

**AC-6 — No pipeline / main-window regression:**
Given the native pill has replaced the WebView2 bar
When recording, transcription, cleanup, and paste run, and the settings window is opened
Then the pipeline, hotkeys, paste, and the `main` window behave exactly as before

### DoD (surface-class)

- Real Windows release build via `scripts/sync-and-build.ps1`.
- **Occlusion harness PASS** (machine, agent-run, AC-5) — content pixels survive foreground occlusion
  + 3 s dwell; recorded as evidence before the human gate.
- **Andi smoke on real Windows** (NFR2): pill looks right across all states; drag works; survives
  occlusion in real use.
- `cargo check --target x86_64-pc-windows-gnu` green; Linux `cargo test` green; `tsc` / `npm run
  build` green after FloatingBar removal.
- Surface-smoke-checklist traps reviewed (esp. region/geometry; event-wiring N/A since in-process).
- Code-review inversion (reviewer-verified, not self-attested) per project rules.

---

## Story 10-2 — Native preview overlay

**As** a Klarvo user,
**I want** the live-preview card to stay visible when an app covers it,
**so that** I can read the transcript-so-far while dictating into that app.

### Acceptance Criteria

**AC-1 — Native layered window replaces the WebView2 `preview`:**
Given Story 10-1 established the native layered-window substrate
When the preview is created
Then a native Win32 window (`WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE |
WS_EX_TRANSPARENT` for click-through) renders the preview, and the WebView2 `"preview"` window, its
`main.tsx` route, and `src/PreviewPanel.tsx` are removed

**AC-2 — Renders the current preview look 1:1:**
Given live-preview chunks arrive
Then the native preview shows the current `PreviewPanel.tsx` appearance — dark card, scrollable
cleaned text bottom-aligned and growing up, top-fade when scrolled, teal hairline border — driven by
the existing live-preview-chunk flow (in-process or via the existing event)

**AC-3 — Click-through + pill-anchored positioning (parity):**
Given the preview is visible
Then cursor events pass through it, it is anchored above the pill, and it repositions when the pill
is dragged — parity with today

**AC-4 — Occlusion-survival, machine-verified:**
Given the preview is visible
When a foreground app is maximized over its region (+ 3 s dwell)
Then it stays fully painted — verified by the occlusion harness

**AC-5 — Retire ADR-0020 machinery:**
Given both overlays are now native
Then the bundled-runtime pin + `sync-and-build.ps1` self-heal (ADR-0020) are removed, and ADR-0020 is
marked Superseded in the index (already noted; confirm clean removal)

### DoD (surface-class)

- Same shape as 10-1: Windows release build; occlusion harness PASS (machine); Andi smoke
  (appearance + click-through + anchoring); `cargo check`/`cargo test`/`tsc` green; review inversion.
