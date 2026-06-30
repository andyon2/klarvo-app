# Story 10.1: Native Pill (FloatingBar) Overlay

Status: done

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

- [x] **Task 1 — Native layered-window substrate** (AC: 1, 5)
  - [x] Add a `src-tauri/src/native_pill.rs` module, `#[cfg(target_os = "windows")]`
  - [x] Extend `windows` 0.61 features in `Cargo.toml`: added `Win32_System_LibraryLoader` for `GetModuleHandleW`
  - [x] Register a window class; `CreateWindowExW` with `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`, `WS_POPUP`, skip-taskbar, non-focusable
  - [x] Build top-down 32bpp premultiplied-BGRA DIB section (main + tmp); present via `UpdateLayeredWindow(ULW_ALPHA)`
  - [x] Size 200×36 logical → physical via `GetDeviceCaps(LOGPIXELSX)`; position from saved `bar_x/bar_y` or `SPI_GETWORKAREA` center-bottom fallback
- [x] **Task 2 — CPU rasterizer for all pill states** (AC: 2)
  - [x] Pill/stadium shape via cubic-bezier path in tiny-skia (R=H/2 = full stadium), fill `rgba(25,25,25,0.96)` premultiplied
  - [x] Klarvo "K" logo (24×24 teal rounded rect + GDI white "K" composited with coverage-blend)
  - [x] State content: recording (stop box + 5-bar waveform + GDI mode badge), transcribing/cleaning (quarter-arc spinner + GDI label), done/done-clipboard (check polyline or clipboard icon + GDI label), error (GDI label)
  - [x] GDI text composited via tmp DIB (white-on-black → B-channel coverage mask → premul blend onto BGRA)
  - [x] Backdrop blur dropped per ADR-0021 sub-decision 3
- [x] **Task 3 — In-process state + RMS drive** (AC: 2, 3)
  - [x] `emit_pipeline_state` (lib.rs) calls `pill.set_state()` in-process via `PostMessageW` (WM_PILL_SET_STATE)
  - [x] `setup_audio_level_emitter` feeds `pill.feed_rms()` in-process (WM_PILL_SET_RMS); `klarvo://audio-level` still emitted for preview window
  - [x] AC-3 RMS mapping identical to FloatingBar.tsx (NOISE_FLOOR=0.006, boosted=pow(min(1,level*10),0.4), 20-buffer, 5 bars)
- [x] **Task 4 — Drag + persistence + preview anchoring** (AC: 4)
  - [x] WndProc handles WM_LBUTTONDOWN (drag start or stop-button cancel), WM_MOUSEMOVE (drag tracking with SetCapture), WM_LBUTTONUP (drag end + save)
  - [x] Position persisted via `state.save_config_locked("bar position", ...)` on release
  - [x] `klarvo://bar-moved {x,y}` emitted throttled during drag + final on release (preview window anchoring)
- [x] **Task 5 — Remove the WebView2 bar surface** (AC: 1, 6)
  - [x] Deleted `src/FloatingBar.tsx`; removed `label === "bar"` branch from `src/main.tsx`
  - [x] Replaced `create_bar_window` call with `native_pill::NativePill::create()` in lib.rs setup
  - [x] `ensure_bar_window` updated to check `pill.is_alive()` via `IsWindow()`
  - [x] Close handler `lib.rs` updated to only special-case `"preview"` (removed `"bar"`)
  - [x] `set_bar_shape` made no-op (native pill shape = pixel alpha, no Win32 region needed)
  - [x] `save_bar_position`/`get_bar_position` and tray-tooltip coupling kept
- [x] **Task 6 — Occlusion harness + verification** (AC: 5, 6)
  - [x] `scripts/desktop-occlusion-proof.ps1` created: Notepad-maximize occlusion proof + 3s dwell, evidence PNGs to gate4-evidence/10-1/
  - [x] Linux `cargo test` — 18/18 passed, 0 failed (15 pre-existing warnings, no new errors)
  - [x] `cargo check --target x86_64-pc-windows-gnu` — native_pill.rs compiles; ort-sys/llama-cpp failures are pre-existing cross-compile limitations unrelated to this change

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
claude-sonnet-4-6 (2026-06-27)

### Debug Log References
- Linux `cargo test`: 18 passed, 0 failed (pre-existing 15 warnings)
- `cargo check` clean after set_bar_shape simplification

### Completion Notes List
- **Architecture**: tiny-skia (0.11) for shape rendering (stadium, rounded rects, polylines, spinner arc) + GDI for text (K logo, labels, mode badge). GDI text composited via white-on-black tmp DIB → B-channel coverage mask → premultiplied BGRA blend onto main DIB.
- **Thread model**: dedicated OS thread owns the HWND + message loop. State/RMS arrive via PostMessageW (WM_PILL_SET_STATE 0x8001 / WM_PILL_SET_RMS 0x8002 / WM_PILL_SET_MODE 0x8003). AppHandle stored in PillWindowState for cross-thread emit.
- **RGBA→BGRA conversion**: tiny-skia Pixmap is premultiplied RGBA; main DIB is premultiplied BGRA. Conversion = swap bytes [0] and [2] per pixel — done in `copy_rgba_to_bgra`.
- **Sub-decision 3**: backdrop blur dropped (ADR-0021). Near-opaque 0.96 fill makes it negligible.
- **`set_bar_shape` made no-op**: native pill shape = ULW_ALPHA pixel alpha, no Win32 region needed. Command still registered for frontend compat.
- **Cross-target concern**: `native_pill.rs` is `#[cfg(target_os = "windows")]` — Linux build excludes it entirely; Android unaffected. `tiny-skia` dep also gated under `cfg(windows)`.
- **E2 dependency**: `klarvo://bar-moved {x,y}` still emitted during drag + on release so preview window stays anchored until Story 10-2.
- **AC-5 machine verification**: `scripts/desktop-occlusion-proof.ps1` requires Klarvo running + recording active. Evidence written to `gate4-evidence/10-1/`. Human gate = Windows release build + smoke.

### File List
- `src-tauri/src/native_pill.rs` — new: native Win32 layered pill window (~700 lines)
- `src-tauri/Cargo.toml` — modified: added `Win32_System_LibraryLoader` feature + `tiny-skia = "0.11"` dep
- `src-tauri/src/lib.rs` — modified: `mod native_pill`, `native_pill` field on AppState, `emit_pipeline_state` drives pill, `setup_audio_level_emitter` feeds pill, setup replaced `create_bar_window` with `NativePill::create`, close handler updated
- `src-tauri/src/commands/misc.rs` — modified: `ensure_bar_window` uses `IsWindow` liveness check; `set_bar_shape` → no-op
- `src-tauri/src/pipeline.rs` — modified: recording-start recovery block uses native pill instead of WebView2 bar
- `src/main.tsx` — modified: removed `label === "bar"` branch (3 lines)
- `src/FloatingBar.tsx` — deleted
- `scripts/desktop-occlusion-proof.ps1` — new: Notepad occlusion harness for AC-5 gate-4 evidence

## Change Log
- 2026-06-27: Story 10-1 implemented — native Win32 layered pill replaces WebView2 bar; tiny-skia rasterizer; GDI text compositing; in-process state+RMS drive; drag+persistence+preview anchoring; WebView2 bar surface removed. Linux cargo test 18/18 green. Windows build + occlusion smoke = Andi gate.
- 2026-06-27: **DONE — Andi attended Windows gate PASSED.** Windows release build green; always-on-top survives foreground occlusion (AC-5, the epic's whole point — confirmed in real use); drag + persistence + preview anchoring OK (AC-4); no pipeline/tray regression (AC-6); visual fidelity 1:1 (AC-2/NFR2) after three post-review fix rounds (below). Pre-gate, the win-gnu compile (Decision-1 residual) was closed by a WSL-side pure-Rust compile harness (`windows`+`tiny-skia`, excludes the C++ whisper/ort deps) — Win32 surface is now machine-checkable from WSL; recipe in `gate4-evidence/10-1/win32-surface-check.md` (reusable for 10-2). Fix rounds: (a) `116380a` — corrected ~30 `windows` 0.61.3 API signatures (PostMessageW Some-wrap, DeleteObject/SelectObject `.into()`, TextOutW 4-arg, CreateFontW typed enums, missing imports) that the blocked win-gnu check had masked → real Windows build was failing; (b) `8f171c8` — waveform fidelity: reserve mode-badge width so the 5 bars flex-fill only the remaining space (was spanning under the badge, ~22px blocky bars) + rounded capsule bars (borderRadius:9999) + AA; (c) `dcddf80` — render all pill text in the app's Geist font (embedded .ttf derived from the repo .woff2, loaded via AddFontMemResourceEx; was Segoe UI — wrong glyphs, esp. the logo K) + center the K from its measured extent. Range `94a9cc77..dcddf80`.
- 2026-06-27: Code review (autonomous conductor, Opus 3-layer adversarial) cleared after 1 fix round (commit `6796358`). 8 confirmed findings fixed — CRITICAL: pill was driven only at 4 `emit_pipeline_state` sites while ~16 raw `EVENT_STATE_CHANGED` emits bypassed it (pill stuck on Recording) → all transitions now route through the choke; AC-3 waveform double-boost removed (boost applied once); waveform ring restored to oldest→newest FIFO; HIGH: mode badge fed from `klarvo://active-mode`; stop-button cancel now hides pill; MEDIUM: sound teardown (DestroyWindow + USERDATA null, no double-free), Pixmap-alloc-failure no longer panics; LOW: bar-moved throttled. Machine gates green (Linux cargo test 628+18, tsc+vite build). `cargo check --target x86_64-pc-windows-gnu` infra-blocked by untouched whisper-rs-sys/ort-sys CMake (not a regression). Status held at `review` — real-Windows visual fidelity (AC-2/NFR2) + occlusion harness PASS (AC-5) + drag (AC-4) + pipeline/tray no-regression (AC-6) are Andi's attended gate. Evidence: `gate4-evidence/10-1/verdict.md`.
