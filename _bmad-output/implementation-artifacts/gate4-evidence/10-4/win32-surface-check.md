# Story 10-4 — Win32 Surface Compile Gate

**Date:** 2026-06-29
**Target:** `x86_64-pc-windows-gnu`
**Result:** ✅ 0 errors (55 warnings, all pre-existing `#[must_use]` BOOL/Result patterns — same class as 10-1/10-2/10-3)

---

## New feature added

`"Win32_UI_HiDpi"` added to `windows = "0.61"` features in `src-tauri/Cargo.toml`.
New import in both files: `use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};`

## Changes type-checked

### native_pill.rs
- Replaced 4-line `GetDC`/`GetDeviceCaps`/`ReleaseDC`/`scale` block with:
  - `SPI_GETWORKAREA` call for physical work-area coords
  - `candidate_pt` from saved_x/saved_y or center-bottom of work area
  - `MonitorFromPoint(candidate_pt, MONITOR_DEFAULTTONEAREST)` → `HMONITOR`
  - `GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y)` → real scale
  - AC-1 log line: `[native_pill] DPI: GetDpiForMonitor={dpi_x} GetDeviceCaps(legacy)={legacy} scale_real=... scale_was=...`
  - Fallback to `GetDeviceCaps` on error

### native_preview.rs
- Symmetric fix (same new DPI block using `pill_x`/`pill_y` as candidate point)
- Redundant second `SystemParametersInfoW(SPI_GETWORKAREA)` call eliminated (reuses `work_area` from the DPI block)
- AC-1 log line: `[native_preview] DPI: GetDpiForMonitor={dpi_x} GetDeviceCaps(legacy)={legacy} scale_real=... scale_was=...`

## Harness recipe

Same scratch-harness approach as 10-1/10-2. Harness located at scratchpad/win32-check-10-4/ (ephemeral).
Cargo.toml: same features as src-tauri/Cargo.toml windows block (including new Win32_UI_HiDpi).
Two patches: remove `#![cfg(target_os="windows")]`, replace `use tauri::` with `use crate::fake_tauri::` (native_pill.rs only).

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.25s
warning: `win32-check-10-4` (lib) generated 55 warnings (all #[must_use] BOOL/Result, pre-existing)
```

## Linux cargo test

```
test result: ok. 630 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
```

---

## GATE-4 residual for Andi — REAL-TARGET WINDOWS GATE

Run on real Windows machine at 125/150 % DPI via `scripts/sync-and-build.ps1`:

1. **AC-1 log verification:** In `Klarvo.log` (Settings → About or `%APPDATA%\com.klarvo.voice\klarvo.log`),
   look for lines like:
   ```
   [native_pill] DPI: GetDpiForMonitor=144 GetDeviceCaps(legacy)=96 scale_real=1.500 scale_was=1.000
   [native_preview] DPI: GetDpiForMonitor=144 GetDeviceCaps(legacy)=96 scale_real=1.500 scale_was=1.000
   ```
   (exact numbers depend on your display DPI; the `scale_was=1.000` confirms the old defect was present)

2. **AC-2 absolute size:** Pill (logo + waveform + cancel button) and preview card appear the same
   apparent size as before Epic 10's native migration. Reference: Andi's own memory + git history before 10-1.

3. **AC-3 appearance settings:** Change a preview setting in Settings (e.g. font size = large), save,
   start a new recording → preview card reflects the changed setting visibly.

4. **AC-4 preset legibility:** Small/medium/large font presets produce visibly different text sizes in
   the preview. If steps are adequate → pass. If inadequate → escalate to Andi for new target numbers
   before any code change.

5. **AC-5 no regression:** All prior 10-1/10-2/10-3 smoke behaviors intact (pill states, waveform,
   drag+position persistence, preview click-through+anchoring, standby recreate-on-start).

GREEN on all five → update story to `done`, flip sprint-status.yaml to `done`.
Any FAILED → re-open into gated dev flow.
