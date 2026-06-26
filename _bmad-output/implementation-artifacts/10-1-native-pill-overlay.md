# Story 10.1: Native Pill (FloatingBar) Overlay

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a Klarvo user dictating into another app,
I want the recording pill to stay visible when that app covers its spot,
so that I can always tell whether recording is active — permanently, not until the next restart.

## Context & Why

The pill overlay (`bar`, 200×36) goes blank whenever a foreground window covers its screen region — i.e. exactly while you dictate *into* the app you're typing in. Root cause (measured, [ADR-0021](../../docs/adr/0021-native-desktop-overlays.md)): the occlusion-present halt lives **inside the WebView2/Chromium compositor** — it stops delivering the swapchain to DWM while it considers the window occluded. Four transient fixes (topmost re-assert, identical browser args, `CalculateNativeWinOcclusion` off, bundled-runtime pin) all faked a fix and returned. A native `WS_EX_LAYERED | WS_EX_TOPMOST` window stays fully composited when occluded — **proven 7600/7600 + 3 s dwell** before this work was committed.

**Decision:** render the pill as a native Win32 layered, always-on-top window drawn from Rust — not a Tauri/WebView2 webview. This is the **proof slice** of Epic 10: all the hard primitives (layered-window substrate, per-pixel alpha, RMS waveform, state rendering, drag) live here and validate the whole substrate before Story 10-2 (preview) reuses it.

**This is a technology migration, NOT a re-skin.** The native pill reproduces the *current* `FloatingBar.tsx` look 1:1. The parked Epic 8 "Studio-Dark" re-skin is explicitly **out of scope**.

## Acceptance Criteria

**AC-1 — Native layered window replaces the WebView2 `bar`:**
Given the app starts on Windows
When the pill is created (where `create_bar_window` is called today — `lib.rs:985` setup, `commands/misc.rs:244` ensure-recovery)
Then a native Win32 top-level window with `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` is created at the saved/default pill position, sized 200×36 (logical), its content presented via `UpdateLayeredWindow(ULW_ALPHA)` from a top-down 32bpp premultiplied-BGRA DIB section
And the WebView2 `"bar"` window, its `main.tsx` `label === "bar"` route (`main.tsx:27-29`), and `src/FloatingBar.tsx` are removed
And the change is gated `#[cfg(target_os = "windows")]`

**AC-2 — All pill states render natively, matching the current look 1:1:**
Given the recording pipeline drives the pill state
When the state is one of idle / recording / transcribing / cleaning / done / error (the `PipelineState` variants, `hotkey/mod.rs:23-42`)
Then the native pill renders the **current** `FloatingBar.tsx` appearance for each state:
- **idle** → hidden
- **recording** → rounded pill (radius = height/stadium) + 5-bar teal (`#2AC3A8` / waveform fill `rgba(42,195,168,0.85)`) waveform + red stop affordance + mode badge
- **transcribing** → amber (`#FFA344`) spinner + "Transcribing…"
- **cleaning** → amber (`#FFA344`) spinner + "Cleaning up…"
- **done (normal)** → green (`#4ADE80`) check + "Done"; **done (clipboard-only)** → amber clipboard 📋 + "In Clipboard"
- **error** → red (`#FF7369`) "Error"
With the pill shape and `rgba(25,25,25,0.96)` ~96%-opaque dark fill, 24×24 teal (`#14B8A6`) Klarvo "K" logo at left
And **no** Studio-Dark re-skin is introduced (colors/shape mirror today exactly)

**AC-3 — Waveform is RMS-driven in-process:**
Given recording is active
When RMS amplitude updates arrive on the existing `set_level_callback` path (`audio/mod.rs:334`; ~15 Hz, `samples_per_tick = native_sample_rate / 15`)
Then the native pill's waveform updates directly in-process (no `klarvo://audio-level` JS round-trip required for the native pill), using the **same mapping as today**: `boosted = level <= 0.006 ? 0 : pow(min(1, level*10), 0.4)`, a 20-sample rolling buffer, 5 bars sampled at `round((i/4)*19)`, bar height `max(3, amplitude*19)` with a 12% (`0.12`) floor — so the visual response matches the current pill

**AC-4 — Drag-to-move + position persistence (parity):**
Given the native pill is visible
When the user drags it (mouse-down anywhere except the stop affordance)
Then it follows the cursor, and the new position persists via `config.bar_x/bar_y` (`save_bar_position`, `commands/misc.rs:172`) and is restored on next start (`get_bar_position` → `create_bar_window(saved_x, saved_y)`) — behavioural parity with the WebView2 pill
And it continues to emit `klarvo://bar-moved` `{ x, y }` (logical px, throttled during drag + final on release) so the **still-WebView2 preview window stays anchored** (cross-story dependency — see Dev Notes)

**AC-5 — Occlusion-survival, machine-verified (the whole point):**
Given the native pill is visible during recording
When a foreground app is maximized over its screen region, and again after a 3 s dwell
Then the pill stays fully painted (content pixels > 0, ≈100% of the region) — verified by the ADR-0021 occlusion harness; this is the exact scenario where the WebView2 pill measured `screenTeal=0`

**AC-6 — No pipeline / main-window regression:**
Given the native pill has replaced the WebView2 bar
When recording, transcription, cleanup, and paste run, and the settings window is opened
Then the pipeline, hotkeys, paste, tray tooltip (`update_tray_tooltip` still fires from `emit_pipeline_state`), and the `main` window behave exactly as before

## Tasks / Subtasks

- [ ] **Task 1 — Native layered-window substrate** (AC: 1, 5)
  - [ ] Add a `src-tauri/src/overlay/` (or `native_pill.rs`) module, `#[cfg(target_os = "windows")]`
  - [ ] Extend `windows` 0.61 features in `Cargo.toml` as needed (`Win32_Foundation` for RECT/POINT/SIZE; confirm `WS_EX_LAYERED`/`UpdateLayeredWindow`/`CreateDIBSection`/`BitBlt` are reachable under `Win32_UI_WindowsAndMessaging` + `Win32_Graphics_Gdi` — mirror existing cfg-gates, never an unconditional dep)
  - [ ] Register a window class; `CreateWindowEx` with `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`, undecorated, skip-taskbar, non-focusable
  - [ ] Build a top-down 32bpp premultiplied-BGRA DIB section; present via `UpdateLayeredWindow(ULW_ALPHA)` (no GPU/swapchain/DirectComposition)
  - [ ] Size 200×36 logical → physical via the window's DPI; position from saved `bar_x/bar_y` or the existing center-bottom default (reuse the SPI_GETWORKAREA logic from `create_bar_window`, `lib.rs:706-749`)
- [ ] **Task 2 — CPU rasterizer for all pill states** (AC: 2)
  - [ ] Pill/stadium shape with `rgba(25,25,25,0.96)` fill (anti-aliased rounded ends)
  - [ ] Klarvo "K" logo (24×24, `#14B8A6`, radius 6, white K)
  - [ ] State content: recording (stop box + waveform + mode badge), transcribing/cleaning (spinner + label), done (check/clipboard + label), error (label). Colors per AC-2 table
  - [ ] Anti-aliased text + the rotating spinner + the check glyph are the non-trivial bits (ADR-0021 negative consequence) — pick a small CPU raster approach (e.g. a tiny text/AA path); keep it module-local, factor out only on a 2nd consumer
  - [ ] Drop backdrop blur (ADR-0021 sub-decision 3 — near-opaque fill makes it negligible)
- [ ] **Task 3 — In-process state + RMS drive** (AC: 2, 3)
  - [ ] Drive the native pill from the Rust pipeline state directly (the `emit_pipeline_state` path, `lib.rs:505`) — render on each `PipelineState` change; idle → hide
  - [ ] Feed the waveform from `set_level_callback` in-process (`audio/mod.rs:334`), applying the exact AC-3 mapping; ~15 Hz re-render of the 200×36 bitmap
  - [ ] The existing `klarvo://audio-level` / `klarvo://state-changed` emitters MAY remain for other consumers (preview), but the native pill must not depend on the JS round-trip
- [ ] **Task 4 — Drag + persistence + preview anchoring** (AC: 4)
  - [ ] Manual drag via `WM_NCHITTEST`/`WM_LBUTTONDOWN`+`WM_MOUSEMOVE` (or `WM_NCLBUTTONDOWN` HTCAPTION); exclude the stop-affordance hit region
  - [ ] On release, persist via the existing `save_bar_position` (logical px, both coords together); restore on next start
  - [ ] Emit `klarvo://bar-moved` `{ x, y }` (throttled during drag, final on release) — **preview still reads this** (see Dev Notes cross-story dep)
- [ ] **Task 5 — Remove the WebView2 bar surface** (AC: 1, 6)
  - [ ] Delete `main.tsx:27-29` `label === "bar"` branch; remove `src/FloatingBar.tsx`
  - [ ] Replace the `create_bar_window` call at `lib.rs:985` (and `ensure_bar_window`, `commands/misc.rs:244`) with the native-pill create/ensure; update the liveness check (`IsWindow`) instead of `get_webview_window("bar")`
  - [ ] Update the close handler `lib.rs:1072` (`label == "bar" || "preview"`) to only special-case `"preview"`
  - [ ] Keep `save_bar_position`/`get_bar_position` commands (still the drag persistence path) and the tray-tooltip coupling
  - [ ] Verify `tsc` / `npm run build` green after FloatingBar removal
- [ ] **Task 6 — Occlusion harness + verification** (AC: 5, 6)
  - [ ] Bring the occlusion harness into the repo (currently only in a Temp dir) as `scripts/desktop-occlusion-proof.ps1` (template = ADR-0021 `native-proof2.ps1`): fill/observe region → maximize Notepad over it + `SetForegroundWindow` → `CopyFromScreen` → count content pixels → re-sample after 3 s dwell. PASS = pixels > 0 incl. after dwell
  - [ ] `cargo check --target x86_64-pc-windows-gnu` green; Linux `cargo test` green
  - [ ] Run `docs/surface-smoke-checklist.md` applicable traps (esp. window-geometry/region clip)

## Dev Notes

### This is a technology migration, anchored to the CURRENT render
The binding appearance SOLL is the **current** `src/FloatingBar.tsx` (642 lines) — reproduce it 1:1, do **not** apply Epic 8 Studio-Dark. ADR-0021 is the binding architecture decision (substrate, sub-decisions, proof).

### Current appearance — exact values to reproduce (from `src/FloatingBar.tsx`)
- **Pill:** 200×36 logical; `borderRadius: 9999` (stadium); fill `rgba(25,25,25,0.96)`; current backdrop `blur(12px)` → **dropped** in native (sub-decision 3). [FloatingBar.tsx:530-533]
- **Logo:** 24×24, `#14B8A6`, radius 6, white "K" 14px bold. [FloatingBar.tsx:70-90]
- **Waveform:** 5 bars, 3px gap, `borderRadius:9999`, fill `rgba(42,195,168,0.85)`, height `max(3, amplitude*19)`, no transition (instant). [FloatingBar.tsx:105-118]
- **Stop affordance:** 14×14 red `rgba(248,113,113,0.9)` square, 8×8 inner box, radius 2; excluded from drag (`data-stop-btn`). [FloatingBar.tsx:168-194]
- **Spinner:** 13×13 rotating arc, 0.9s spin. [FloatingBar.tsx:128-148]  **Check:** 11×11 polyline, 3px stroke. [FloatingBar.tsx:151-165]
- **State accent/label/border:** recording teal `#2AC3A8`; transcribing/cleaning amber `#FFA344`; done green `#4ADE80` (clipboard-only amber `#FFA344`, "In Clipboard"); error red `#FF7369`. [FloatingBar.tsx:501-511]
- **Show/hide animation** (bar-expand 220ms / collapse 180ms / done-pop 280ms) is a nicety — reproduce if cheap, but the occlusion-survival + state fidelity are the gate, not the easing curve. [FloatingBar.tsx:47-61]

### Waveform RMS path (preserve the mapping exactly — calibrated)
Backend computes RMS per ~66ms chunk and today emits `klarvo://audio-level {level:f32}` at 15 Hz [`lib.rs:543`, `audio/mod.rs:879`]. The native pill should consume the **same RMS** via `set_level_callback` in-process [`audio/mod.rs:334`]. Frontend mapping to replicate [FloatingBar.tsx:385-388, 105-108]:
```
NOISE_FLOOR = 0.006
boosted = level <= NOISE_FLOOR ? 0 : pow(min(1, level*10), 0.4)   // clamp [0,1]
rolling buffer length 20; bar i in 0..4 → levels[round((i/4)*19)]
amplitude = max(0.12, sample); heightPx = max(3, amplitude*19)
```
Changing the noise floor / `*10` / `^0.4` breaks visual responsiveness — keep verbatim.

### `create_bar_window` today (the thing being replaced) — `lib.rs:645-778`
Tauri `WebviewWindowBuilder` "bar": `inner_size(200,36)`, `decorations(false)`, `transparent(true)`, `always_on_top(true)`, `resizable(false)`, `skip_taskbar(true)`, `focused(false)`, identical `WEBVIEW2_BROWSER_ARGS`, `shadow(false)`; then `set_window_region_pill` (`CreateRoundRectRgn`). Position priority: saved `bar_x/bar_y` → `SPI_GETWORKAREA` center-bottom (`y = bottom - 36 - 8`) → monitor fallback → `(400,10)`. **Reuse the positioning logic** for the native window. Called: setup `lib.rs:985` (Windows-only), recovery `ensure_bar_window` `commands/misc.rs:244`.

### Existing Win32 FFI to build on
`windows` 0.61 already gated `cfg(windows)` with `Win32_UI_WindowsAndMessaging`, `Win32_Graphics_Gdi`, `Win32_UI_Input_KeyboardAndMouse`, `Win32_System_Threading` [`Cargo.toml:51-59`]. Region helpers `CreateRoundRectRgn`/`SetWindowRgn` exist [`lib.rs:549-597`]; paste module already does HWND/`SetForegroundWindow`/`SendInput` patterns [`paste/mod.rs:176-387`] — reuse the HWND-cast + `unsafe`-block + log-on-failure idiom. Likely add `Win32_Foundation`; confirm `WS_EX_LAYERED`/`UpdateLayeredWindow`/`CreateDIBSection` reachability.

### State drive — `PipelineState` variants the pill renders [`hotkey/mod.rs:23-72`]
`Idle, Recording, Transcribing, Cleaning, Done, Error, Warning`. `Done` carries `clipboardOnly` (true → amber "In Clipboard"). `Warning` does NOT change the pill (transient toast elsewhere). `emit_pipeline_state` [`lib.rs:505`] also updates the tray tooltip — **keep that coupling**. Done flashes ~1.5–4s then idle; error ~2.5s then idle [FloatingBar.tsx:337-367].

### ⚠️ Cross-story technical dependency (E2 — expected, flagged now)
The **preview window stays WebView2 until Story 10-2**. It anchors itself to the pill by listening to `klarvo://bar-moved` `{x,y}` [FloatingBar.tsx:450-452, 476-478]. **The native pill MUST emit `klarvo://bar-moved` identically** (logical px, on drag-throttled + final on release) or the still-WebView2 preview goes misaligned/invisible. When 10-2 makes the preview native too, this event contract may be revisited via `bmad-correct-course`; for 10-1 it is a hard parity requirement.

### Removal surface [agent digest §6]
Delete `main.tsx:27-29` (`label==="bar"`) + `FloatingBar.tsx`; `tauri.conf.json` needs no change (bar was programmatic). Update close handler `lib.rs:1072`. Keep `save_bar_position`/`get_bar_position`, `installConsoleBridge` (preview still needs it), tray tooltip.

### Project Structure Notes
- Module placement: native pill code under `src-tauri/src/` behind `#[cfg(target_os = "windows")]`, mirroring existing platform gates (`whisper-rs`/`arboard`/`windows` are all cfg-gated — never break the Android/Linux build).
- Errors structured `Result`/`AppError`, never `panic!`/`todo!`/`unimplemented!` (fail-soft). No `debug_assert!` with side-effects.
- Code + comments English; commits small + scoped, never `git add .`.

### Testing standards
- **DoD is surface-class** (`project-context.md`): Linux `cargo test` + lint do NOT satisfy it. Hard gate = real Windows release build via `scripts/sync-and-build.ps1` + manual smoke.
- **Occlusion = machine-verified** (AC-5) via the harness — recorded as evidence to `_bmad-output/implementation-artifacts/gate4-evidence/10-1/` before the human gate.
- **Visual fidelity = Andi's smoke** on a real Windows build (NFR2) — never claimed from machine output. Pill looks right across all states; drag works; survives occlusion in real use.
- Never make the user the rendering oracle: get any visual/geometry defect into something **you** can observe (PrintWindow/CopyFromScreen of the transparent overlay, instrumented geometry) and name the cause before changing app code; change once. A failed smoke re-enters the gated dev flow, never a bare-loop hot-patch.
- Tests are inline `#[cfg(test)]` modules; bind tests to real code paths.

### References
- [Source: docs/adr/0021-native-desktop-overlays.md] — binding decision, sub-decisions 1-7, occlusion proof + harness
- [Source: _bmad-output/planning-artifacts/epics-native-overlays.md] — Epic 10 inventory (AR/VR/IR/NFR), Story 10-1 ACs
- [Source: src/FloatingBar.tsx:26-638] — appearance SOLL (1:1 target)
- [Source: src-tauri/src/lib.rs:645-778] — `create_bar_window` (replaced), :505-510 `emit_pipeline_state`, :543/600-610 audio-level emitter, :549-597 region helpers, :985 setup call, :1072 close handler
- [Source: src-tauri/src/audio/mod.rs:334, 879, 1156] — `set_level_callback`, samples_per_tick, RMS drain
- [Source: src-tauri/src/hotkey/mod.rs:23-176] — `PipelineState` / `PipelineEvent` / `EVENT_STATE_CHANGED`
- [Source: src-tauri/src/commands/misc.rs:172-202, 217-254] — `save_bar_position`/`get_bar_position`/`ensure_bar_window`
- [Source: src/main.tsx:27-29] — `label === "bar"` route (removed)
- [Source: src-tauri/Cargo.toml:51-59] — `windows` 0.61 features
- [Source: _bmad-output/project-context.md] — code rules (platform gates, surface DoD, no-rendering-oracle)
- [Source: docs/surface-smoke-checklist.md] — surface trap ledger

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List
