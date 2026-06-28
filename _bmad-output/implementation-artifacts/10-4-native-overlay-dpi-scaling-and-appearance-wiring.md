# Story 10.4: Native Overlay DPI Scaling + Appearance-Wiring Audit

Status: in-progress

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a Klarvo user on a high-DPI (125/150 %) display,
I want both native overlays (pill + preview) to render at the same size as the old WebView2
overlays, with the appearance/size settings visibly effective,
so that the overlays are legible and the font-size presets actually differ.

## Context & Why

Since Epic 10's native rebuild (10-1 pill, 10-2 preview), both overlays render **too small** vs. the
old WebView2 overlays — pill (logo + cancel button too small), preview card, AND font-size presets feel
far too weak (large ≈ tiny, small ≈ unusable). Andi confirmed this in real-device smoke after 10-2
(2026-06-28). Full scope + read-only diagnosis: `docs/backlog.md` (section "Epic 10 —
Native-Overlay-Skalierung zu klein + Appearance-Wiring-Audit").

**Root cause (leading hypothesis — NOT yet verified on Windows, must confirm via AC-1 logging):**
Both files compute:
```rust
// native_pill.rs:1259   AND   native_preview.rs:942 (identical pattern)
let screen_dc = GetDC(None);
let dpi = GetDeviceCaps(Some(screen_dc), LOGPIXELSX);
ReleaseDC(None, screen_dc);
let scale = dpi as f64 / 96.0;
```
Under **per-monitor-v2 DPI awareness** (which tao/Tauri sets via the embedded manifest),
`GetDeviceCaps(desktop_dc, LOGPIXELSX)` returns **96** regardless of the monitor's real DPI →
`scale = 1.0` even on a 150 % display. The old WebView2 windows were correctly DPI-scaled by Tauri.
Correct API: `GetDpiForMonitor(MonitorFromPoint(...), MDT_EFFECTIVE_DPI)`.

**Settings-wiring diagnosis (read-only, already done):** The `previewFontSize` → `font_px` mapping
and the rest of the appearance chain are correctly wired. This is **not** the defect. AC-3 is a
smoke-verification, not a code-fix story.

### ⚠️ HYPOTHESIS REFUTED at GATE-4 smoke (2026-06-28) — story re-scoped

Andi's real-machine AC-1 log: `GetDpiForMonitor=120 GetDeviceCaps(legacy)=120 scale_real=1.250
scale_was=1.250`. **Both APIs return 120 → `scale_was == scale_real == 1.25`.** On Andi's single
primary monitor `GetDeviceCaps` was returning the correct DPI all along; the "scale=1.0 bug" never
existed there and the DPI fix (a0334bf) is a **no-op**. AC-2 failed precisely because there was nothing
to enlarge.

**Measurement (conductor, from Andi's full-width screenshot):** pill = **249 px** wide on a 1918 px
(≈ physical 1920) screen. Designed width = 200 logical × 1.25 = **250 px**. → The native pill renders
**1:1 at its designed size**, identical to the old WebView2 pill (both 200×36 logical; all internal
constants — logo 24, stop 14/8, waveform 20, spinner 13, check 11, bg rgba(25,25,25,.96), fonts 14/10 —
verified equal via git). **No DWM virtualization, no scaling regression.**

**Conclusion:** "too small" is not a bug — the overlays are *designed* small (pill 200×36, preview fonts
11/13/15) and render correctly. Enlarging is a **deliberate design/taste decision** (Andi, 2026-06-28),
not a defect fix. The DPI fix + clamp (a0334bf, 795d5b3) are kept (harmless, more-correct for
multi-monitor, AC-1 log useful); the F2/F3 multi-monitor findings stay in backlog (N/A single-monitor).
New real work = AC-6, a user-tunable overlay size factor.

## Acceptance Criteria

**AC-1 — Root-cause confirmed via device log:**
Given the app starts on a real Windows machine at 125 % or 150 % DPI
When the native pill and preview are created
Then both overlay threads log a line comparing `GetDeviceCaps(legacy)` vs `GetDpiForMonitor(real)`,
showing the discrepancy (e.g. `GetDeviceCaps=96 GetDpiForMonitor=144 scale_was=1.000
scale_real=1.500`) so the absolute-scale defect is proven in the log, not assumed
And the log is observable by Andi (via `Klarvo.log`) before / during the smoke

**AC-2 — Both overlays render 1:1 with the old WebView2 size at the real monitor DPI:**
Given the DPI fix is applied to both `native_pill.rs` and `native_preview.rs`
When Andi starts the app on his 125/150 % monitor and triggers recording
Then the pill (logo + waveform + cancel button) and the preview card are the same apparent size as
before Epic 10's native migration (reference: git history before 10-1 landed; Andi's own memory)
And both overlays use `GetDpiForMonitor(MonitorFromPoint(candidate_pt, MONITOR_DEFAULTTONEAREST),
MDT_EFFECTIVE_DPI)` in place of `GetDeviceCaps(screen_dc, LOGPIXELSX)` — one shared mechanism,
both files

**AC-3 — Appearance settings are visibly effective in the live preview:**
Given Andi changes any `previewXxx` setting in Settings (font size, panel form, bg color, border
color/width/radius, font family, text color) and saves
When he starts the next recording (the preview is recreated per recording start with
`PreviewConfig::from_app_config`)
Then the live-preview card reflects the changed setting visibly — color, size, font, or border
change is observable on screen
And specifically `previewFontSize` small/medium/large produce visibly different text sizes in the
preview (after the DPI fix the physical sizes are approximately 11×scale / 13×scale / 15×scale px)

**AC-4 — Size presets are perceptibly distinct after the DPI fix:**
Given the DPI fix is in place and the real scale is applied
When the preview renders at small / medium / large font sizes (11 / 13 / 15 logical px mapped to
real physical px via the corrected scale)
Then small/medium/large are visually distinguishable from each other — the steps are not nearly
identical
And **if** the corrected sizes still feel inadequate to Andi, the exact numerical targets are a
human/taste call and must be elicited from Andi before any change to the `font_px` mapping —
**do NOT adjust the 11/13/15 values without explicit sign-off**; this AC is pass/fail after the DPI
fix, not a pre-decided calibration

**AC-5 — No regression to existing overlay behaviour:**
Given the DPI fix is in place
When recording, transcription, cleanup, done, and error states drive the pill and preview
Then all Story 10-1, 10-2, 10-3 ACs remain intact: pill renders all states 1:1, RMS waveform
updates in-process, drag + position persistence work, preview is click-through + pill-anchored,
standby recreate-on-start is preserved, occlusion survival is unchanged
And `cargo check --target x86_64-pc-windows-gnu` is green

**AC-6 — User-tunable overlay size factor (BOTH overlays, default = no change):**
Given a new `overlayScale` value in `config.json` (default `1.0`)
When the app starts (overlays are created reading config)
Then both the native pill and the native preview render their dimensions, fonts, and elements scaled
by `dpi_scale × overlayScale` — at `overlayScale = 1.0` the result is pixel-identical to today
(measured 249 px pill), and at e.g. `1.3` both overlays are uniformly ~30 % larger
And Andi can edit `overlayScale` in `config.json`, restart the app, and immediately see the new size —
without a rebuild per tuning step (Verifikations-Symmetrie: he produces the test state himself)
And changing the factor may reposition a pill that has a saved drag position (stored logical was under
the prior factor); one drag re-settles it and it persists at the new factor — acceptable for a tuning
knob, documented in the smoke note

## Tasks / Subtasks

- [x] Task 1: Add `Win32_UI_HiDpi` feature to Cargo.toml (AC: 2)
  - [x] In `src-tauri/Cargo.toml` under `[target.'cfg(windows)'.dependencies]` → `windows = { ...,
    features = [...] }`, append `"Win32_UI_HiDpi"` to the features list (alongside the existing
    Win32_UI_WindowsAndMessaging, Win32_Graphics_Gdi, etc.)

- [x] Task 2: Fix DPI scale in `native_pill.rs` (AC: 1, 2)
  - [x] Replace lines 1257–1261 (the `GetDC` / `GetDeviceCaps` / `ReleaseDC` / `scale` block) with
    the `MonitorFromPoint` + `GetDpiForMonitor` approach (see Dev Notes for exact pattern)
  - [x] Add `use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};` import
  - [x] Keep `MonitorFromPoint` and `MONITOR_DEFAULTTONEAREST` — they are already available via the
    existing `use windows::Win32::Graphics::Gdi::*;` wildcard import
  - [x] Include the AC-1 log line in the `Ok` branch before the fallback path

- [x] Task 3: Fix DPI scale in `native_preview.rs` (AC: 1, 2)
  - [x] Replace lines 940–944 (same `GetDC` / `GetDeviceCaps` block) with the same
    `MonitorFromPoint` + `GetDpiForMonitor` approach
  - [x] Use `pill_x` / `pill_y` (the logical coordinates passed to `preview_thread`) as the
    candidate point when available; fall back to center of work area
  - [x] Same imports as Task 2
  - [x] Same AC-1 log line

- [x] Task 4: Verify appearance-settings end-to-end (AC: 3, 4)
  - [x] Read `native_preview.rs:91–143` (`PreviewConfig::from_app_config`) and confirm all 8
    appearance fields (bg/text/border color, border width/radius, font family, font size, panel form)
    are correctly mapped — no code change expected, this is a read-and-confirm step
  - [x] Read the save chain (`settings.rs:merge_settings`, `useSettings.ts:handleSaveSettings`,
    `SettingsPanel.tsx:onSave`) and confirm no hop drops an appearance field (trap #6 precedent)
  - [x] Note: the chain was verified in this story's create-story pass — all three hops correctly
    forward all appearance fields (see Dev Notes). Smoke-verify on real Windows; only change code if
    the smoke reveals a gap

- [x] Task 5: compile-verify and DoD checks (AC: 5)
  - [x] `cargo check --target x86_64-pc-windows-gnu` (use the 10-1/10-3 recipe:
    `gate4-evidence/10-1/win32-surface-check.md`) — scratch harness: 0 errors, 55 pre-existing warnings
  - [x] Linux `cargo test` green (no functional logic changed) — 630 passed, 0 failed
  - [ ] Build real Windows release via `scripts/sync-and-build.ps1`
  - [ ] Andi smoke on real Windows (AC-1 log + AC-2 absolute size + AC-3 appearance settings +
    AC-4 preset legibility + AC-5 no regression)

- [ ] Task 6: User-tunable overlay size factor (AC: 6) — the re-scoped real work
  - [ ] `src-tauri/src/config/mod.rs`: add field `overlay_scale: f64` to `AppConfig` with
    `#[serde(default = "default_overlay_scale")]` (serde `rename_all = "camelCase"` → JSON key
    `overlayScale`); add `fn default_overlay_scale() -> f64 { 1.0 }`. Mirror the existing `bar_x`/
    appearance-field pattern.
  - [ ] `src-tauri/src/native_pill.rs`: thread the factor into `pill_thread`. Add an `overlay_scale: f64`
    parameter to `NativePill::create` + `pill_thread`. After the DPI block computes the dpi `scale`,
    set the render scale: `let scale = dpi_scale * overlay_scale;` (rename the existing post-DPI
    `scale` binding to `dpi_scale`, then derive `scale` once). Everything downstream
    (`phys_w`/`phys_h`, `fh()` fonts, `s.scale` used by the draw path, `compute_initial_pos`,
    the WM_LBUTTONUP persist `win_x / s.scale`) keeps using `scale` unchanged — coupled model, so the
    saved position round-trips for any fixed factor. Do NOT decouple position; the one-drag re-settle
    is acceptable (AC-6).
  - [ ] `src-tauri/src/lib.rs` (~705-755): read `cfg.overlay_scale` next to `cfg.bar_x`/`bar_y` and
    pass it to `NativePill::create(...)`.
  - [ ] `src-tauri/src/native_preview.rs`: add `overlay_scale: f64` to `PreviewConfig`; in
    `from_app_config` set `overlay_scale: cfg.overlay_scale`. In `preview_thread`, after the DPI block,
    `let scale = dpi_scale * config.overlay_scale;` (same rename-to-`dpi_scale` pattern). All preview
    dimensions/fonts already derive from `scale` → uniform enlargement, position round-trips.
  - [ ] Verify: `cargo check --target x86_64-pc-windows-gnu` green; Linux `cargo test` green. At
    `overlayScale=1.0` the render must be byte-identical to before (the multiply is ×1.0).

## Dev Notes

### DPI Fix — Exact Implementation Pattern

**The chicken-and-egg:** `MonitorFromPoint` needs a physical screen point; the physical position
needs the scale; the scale needs the monitor DPI. Resolution: derive a candidate physical point from
the known-before-scale context, use it to pick the monitor, then compute the true scale.

**For `native_pill.rs` — replace lines 1257–1261 with:**

```rust
// --- Determine DPI via the monitor at the expected window position ---
// SPI_GETWORKAREA returns physical-pixel coordinates regardless of DPI awareness.
// Under per-monitor-v2 (Tauri's embedded manifest), GetDeviceCaps(desktop_dc, LOGPIXELSX)
// always returns 96 → scale=1.0, wrong on high-DPI monitors. Correct: GetDpiForMonitor.
let mut work_area = RECT::default();
let _ = SystemParametersInfoW(
    SPI_GETWORKAREA, 0,
    Some(&raw mut work_area as *mut _),
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
);
// Candidate point for monitor selection:
// - saved_x/saved_y are logical px; under the previously broken scale=1.0 they equal
//   physical px, so they select the correct monitor in the common case.
// - Default: center-bottom of work area (physical coords from SPI_GETWORKAREA).
let candidate_pt = match (saved_x, saved_y) {
    (Some(lx), Some(ly)) => POINT { x: lx as i32, y: ly as i32 },
    _ => POINT {
        x: work_area.left + (work_area.right - work_area.left) / 2,
        y: work_area.bottom.saturating_sub(5),
    },
};
let hmon = MonitorFromPoint(candidate_pt, MONITOR_DEFAULTTONEAREST);
let mut dpi_x = 0u32;
let mut dpi_y = 0u32;
let scale = if GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_ok() {
    let screen_dc = GetDC(None);
    let legacy = GetDeviceCaps(Some(screen_dc), LOGPIXELSX) as u32;
    ReleaseDC(None, screen_dc);
    log::info!(
        "[native_pill] DPI: GetDpiForMonitor={dpi_x} GetDeviceCaps(legacy)={legacy} \
         scale_real={:.3} scale_was={:.3}",
        dpi_x as f64 / 96.0, legacy as f64 / 96.0
    );
    dpi_x as f64 / 96.0
} else {
    log::warn!("[native_pill] GetDpiForMonitor failed — falling back to GetDeviceCaps");
    let screen_dc = GetDC(None);
    let d = GetDeviceCaps(Some(screen_dc), LOGPIXELSX);
    ReleaseDC(None, screen_dc);
    d as f64 / 96.0
};
// phys_w, phys_h, compute_initial_pos follow unchanged — they already use `scale`
```

Note: `SPI_GETWORKAREA` is already called inside `compute_initial_pos` (for the default-position
case). The extra call here is a minimal redundancy (2 OS calls) — acceptable. Do NOT refactor
`compute_initial_pos` to avoid it; scope creep.

Note: `MONITOR_DEFAULTTONEAREST` is available via the existing
`use windows::Win32::UI::WindowsAndMessaging::*;` wildcard import (the constant lives in
`WinUser.h` / `Win32::UI::WindowsAndMessaging` in the windows crate). If the compiler can't find it
there, also check `windows::Win32::Graphics::Gdi::MONITOR_DEFAULTTONEAREST`.
`MonitorFromPoint` is in `windows::Win32::Graphics::Gdi` (already wildcard-imported).

**For `native_preview.rs` — replace lines 940–944 with the symmetric pattern:**

```rust
// --- DPI + scale (per-monitor, not desktop DC) ---
let mut work_area = RECT::default();
let _ = SystemParametersInfoW(
    SPI_GETWORKAREA, 0,
    Some(&raw mut work_area as *mut _),
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
);
// pill_x/pill_y are logical px (≈ physical under old broken scale=1.0).
// Use the pill position to pick the monitor the preview will appear on.
let candidate_pt = match (pill_x, pill_y) {
    (Some(px), Some(py)) => POINT { x: px as i32, y: py as i32 },
    _ => POINT {
        x: work_area.left + (work_area.right - work_area.left) / 2,
        y: work_area.bottom.saturating_sub(5),
    },
};
let hmon = MonitorFromPoint(candidate_pt, MONITOR_DEFAULTTONEAREST);
let mut dpi_x = 0u32;
let mut dpi_y = 0u32;
let scale = if GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_ok() {
    let screen_dc = GetDC(None);
    let legacy = GetDeviceCaps(Some(screen_dc), LOGPIXELSX) as u32;
    ReleaseDC(None, screen_dc);
    log::info!(
        "[native_preview] DPI: GetDpiForMonitor={dpi_x} GetDeviceCaps(legacy)={legacy} \
         scale_real={:.3} scale_was={:.3}",
        dpi_x as f64 / 96.0, legacy as f64 / 96.0
    );
    dpi_x as f64 / 96.0
} else {
    log::warn!("[native_preview] GetDpiForMonitor failed — falling back to GetDeviceCaps");
    let screen_dc = GetDC(None);
    let d = GetDeviceCaps(Some(screen_dc), LOGPIXELSX);
    ReleaseDC(None, screen_dc);
    d as f64 / 96.0
};
// The existing work_area.left/right/top reads below continue unchanged:
// let work_left = work_area.left;
// (etc.)
```

In `native_preview.rs`, the existing code queries `work_area` again right after the DPI block (lines
946–956). Reuse the `work_area` struct populated above — remove the redundant
`SystemParametersInfoW(SPI_GETWORKAREA)` call that follows in the original code.

### Required Import Additions

Both files need:
```rust
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
```
Add this to the existing `use windows::...` block in each file.

`Cargo.toml` (`src-tauri/Cargo.toml`): add `"Win32_UI_HiDpi"` to the `windows` features list under
`[target.'cfg(windows)'.dependencies]`.

### Appearance-Settings Chain (read-verify, no code change expected)

The full settings-to-preview chain (verified in create-story pass):

1. **Settings UI** (`SettingsPanel.tsx:527–547`): all 8 `previewXxx` values passed to `onSave`
2. **useSettings hook** (`useSettings.ts:88–113`): all 8 forwarded to `saveSettings` (tauri-commands.ts)
3. **Tauri command** (`commands/settings.rs:502–558`): all 8 in `SettingsPatch`, merged via
   `merge_settings` (`settings.rs:339–360`), saved via `save_config_locked`
4. **Recording start** (`lib.rs:651`): `PreviewConfig::from_app_config(&config_guard)` reads the
   updated config → `native_preview.rs:91–143` maps all 8 fields correctly
5. **Renderer** (`native_preview.rs`): all 8 fields applied to the GDI rasterizer

The chain has no known gaps (compare with Epic 6, Story 6-6 where `useSettings` dropped all 7
appearance args — that was the trap; the trap has been fixed). The smoke is the verification.

### Existing Code State — What Must Be Preserved

- `native_pill.rs:1262–1263`: `phys_w`/`phys_h` computed from `scale` — unchanged; the scale fix
  cascades automatically
- `native_pill.rs:1266`: `compute_initial_pos(saved_x, saved_y, scale, phys_w, phys_h)` —
  unchanged; scale is now the corrected value
- `native_pill.rs:1299`: `let fh = |logical: i32| -> i32 { (logical as f64 * scale) as i32 };` —
  unchanged; now correctly scales fonts
- `native_preview.rs:414`: `let k = config.font_px as f64 / BASE_FONT_PX;` → `w_logical` →
  `phys_w = (w_logical * scale) as i32` — unchanged; cascades correctly
- All waveform, state-machine, drag, RMS, recreate-on-start (10-3 AC-1) logic is untouched

### AC-4 Open — Do NOT Auto-Decide Font Calibration

The `font_px` values (11 / 13 / 15 logical px for small/medium/large) were designed to match the
old WebView2 appearance. After the DPI fix at 150 % display, they become ≈16.5 / 19.5 / 22.5
physical px — matching the old WebView2 sizes. Whether that is "perceptibly distinct enough" is
Andi's call. If he says the steps are adequate after the smoke, AC-4 is done. If he says they still
feel too close, the new target numbers must come from him before the dev changes anything. Do NOT
self-decide.

### Platform Gates

Both files are already under `#[cfg(target_os = "windows")]` / Windows-only modules. The new
`GetDpiForMonitor` call must remain inside the same gates — no global platform guard needed.

### Cross-Compile Verification

`cargo check --target x86_64-pc-windows-gnu` is the machine-side gate. Recipe:
`gate4-evidence/10-1/win32-surface-check.md`. Run this before declaring code done.

### Verification Symmetry

Andi CAN reach the test state: his monitor is at real 125/150 % DPI. The log line (AC-1) is
observable in `Klarvo.log` (Settings → About or via `%APPDATA%\com.klarvo.voice\klarvo.log`). The
absolute scale and preset legibility are genuine aesthetic judgments that only the real hardware
reveals — not WSL-certifiable.

### Surface-Smoke Checklist Traps (relevant to this story)

- **Trap #1 (camelCase keys):** not applicable — no new config fields
- **Trap #2 (resync useEffect):** not applicable — no new settings fields
- **Trap #3 (separate-window reload):** not applicable — native windows, no React
- **Trap #6 (multi-hop chain):** verified-correct in create-story pass (see above); smoke confirms

### Project Structure Notes

- Files modified: `src-tauri/Cargo.toml`, `src-tauri/src/native_pill.rs`,
  `src-tauri/src/native_preview.rs` — all Windows-only, all in the existing Epic 10 surface
- No new files, no new config fields, no DB changes, no frontend changes
- Active branch: `conductor/epic-10` (the branch Epic 10 has lived on throughout)
- The `windows` crate version is **0.61** (pinned — do NOT change the version, only add a feature)

### References

- Epic 10 story definition: `_bmad-output/planning-artifacts/epics-native-overlays.md` (Story 10-4
  section)
- Full backlog diagnosis: `docs/backlog.md` (section "Epic 10 — Native-Overlay-Skalierung…")
- Native pill DPI block: `src-tauri/src/native_pill.rs:1257–1261`
- Native preview DPI block: `src-tauri/src/native_preview.rs:940–944`
- Appearance mapping: `src-tauri/src/native_preview.rs:91–143` (`PreviewConfig::from_app_config`)
- Geometry cascades from scale: `native_pill.rs:1262–1266`, `native_preview.rs:414–431`
- Settings chain: `src/components/SettingsPanel.tsx:527–547` → `src/hooks/useSettings.ts:88–114` →
  `src-tauri/src/commands/settings.rs:502–558`
- Previous story (10-3) dev notes + review findings: `10-3-native-pill-standby-resilience.md`
- Cross-compile recipe: `gate4-evidence/10-1/win32-surface-check.md`
- Surface smoke checklist: `docs/surface-smoke-checklist.md`
- ADR-0021: `docs/adr/0021-native-desktop-overlays.md`

## DoD (surface-class)

- Real Windows release build via `scripts/sync-and-build.ps1`.
- `cargo check --target x86_64-pc-windows-gnu` green (cross-compile gate).
- Linux `cargo test` green.
- **GATE 4 is Andi's real Windows machine at real monitor DPI (125/150 %):**
  - AC-1: log shows `GetDeviceCaps=96` vs `GetDpiForMonitor=144` (or equivalent for 150 %)
  - AC-2: pill and preview appear the same size as before Epic 10
  - AC-3: a non-default appearance setting (e.g. font size = large) is visibly effective in the
    preview after save + next recording
  - AC-4: small/medium/large presets are perceptibly distinct (pass or escalate per AC-4 rule)
  - AC-5: all prior 10-1/10-2/10-3 smoke behaviours intact
- Code-review inversion (reviewer-verified, not self-attested) per project rules.

## Change Log

| Date | Change |
|---|---|
| 2026-06-28 | Story authored from Andi real-device smoke (post-10-2) + read-only diagnosis in `docs/backlog.md`. DPI root-cause identified but not yet verified on Windows (leading hypothesis). |
| 2026-06-29 | DPI fix implemented in native_pill.rs + native_preview.rs (GetDpiForMonitor replaces GetDeviceCaps). Win32_UI_HiDpi feature added to Cargo.toml. Appearance chain read-and-confirmed (no code gaps). Win32 cross-compile: 0 errors. Linux tests: 630/0. GATE-4 Windows smoke pending Andi's real device. |
| 2026-06-28 | GATE-4 smoke FAILED → hypothesis REFUTED. Andi's AC-1 log: `GetDeviceCaps=120 GetDpiForMonitor=120 scale_was=scale_real=1.25` — no scale=1.0 bug existed; DPI fix was a no-op. Conductor measured pill = 249 px (= designed 250 px) from full-width screenshot → renders 1:1 with old WebView2, no virtualization. "Too small" = designed-small, not a bug. Re-scoped: DPI fix + clamp kept (harmless); real work = AC-6 user-tunable `overlayScale` factor (Andi chose self-tunable knob, 2026-06-28). Status → in-progress. |
| 2026-06-28 | Code-review (3 adversarial reviewers) CLEAN. One confirmed AC-5 finding fixed (795d5b3): `compute_initial_pos` now clamps the scaled saved pill position to the work area — guards a stale pre-DPI-fix coordinate from placing the pill off-screen (also closes backlog robustness gap "off-screen drag not clamped"). Preview already clamps. Multi-monitor mixed-DPI findings (monitor-selection coord-space, primary-only work_area) deferred to backlog — N/A on Andi's single-monitor setup. Status held at `review` pending Andi's real-machine GATE-4 smoke. Evidence: gate4-evidence/10-4/. |

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

- Win32 surface compile: scratch harness at scratchpad/win32-check-10-4/, 0 errors, 55 pre-existing `#[must_use]` BOOL/Result warnings (same class as 10-1/10-2/10-3). Features: `windows = "0.61"` + `Win32_UI_HiDpi` added.
- Linux `cargo test --lib`: 630 passed, 0 failed.

### Completion Notes List

- Task 1: Added `"Win32_UI_HiDpi"` to `src-tauri/Cargo.toml` windows features (pinned version 0.61 unchanged).
- Task 2: `native_pill.rs` — replaced 4-line `GetDeviceCaps` DPI block (lines 1257–1261) with a 35-line `MonitorFromPoint`+`GetDpiForMonitor` block per Dev Notes exact pattern. Candidate point from saved_x/saved_y or center-bottom of SPI_GETWORKAREA. AC-1 log line in Ok branch compares `GetDeviceCaps(legacy)` vs `GetDpiForMonitor(real)` values. Import added: `use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI}`.
- Task 3: `native_preview.rs` — symmetric fix replacing 4+8 lines (DPI block + separate work_area block) with combined 35-line block. Reuses `work_area` struct (eliminates redundant `SystemParametersInfoW` call). Candidate point from pill_x/pill_y. Same AC-1 log line. Same import.
- Task 4: Appearance chain read-and-confirmed — all 8 fields (bg/text/border color, border width/radius, font family, font size, panel form) correctly wired through SettingsPanel.tsx:527–547 → useSettings.ts:88–113 → settings.rs:SettingsPatch/merge_settings → PreviewConfig::from_app_config:91–143 → renderer. No code gap found, chain is intact.
- Task 5: Machine gates green (cross-compile 0 errors, Linux tests 630/0). GATE-4 Windows smoke (AC-1 log verification + AC-2 size + AC-3 appearance + AC-4 presets + AC-5 no regression) is Andi's real-device gate — requires Windows build via sync-and-build.ps1.

### File List

- `src-tauri/Cargo.toml` — added `"Win32_UI_HiDpi"` to windows features
- `src-tauri/src/native_pill.rs` — DPI fix: GetDpiForMonitor replaces GetDeviceCaps; import added
- `src-tauri/src/native_preview.rs` — DPI fix: GetDpiForMonitor replaces GetDeviceCaps; import added; redundant SPI_GETWORKAREA call eliminated
