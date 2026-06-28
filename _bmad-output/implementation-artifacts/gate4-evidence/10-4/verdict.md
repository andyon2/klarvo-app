# GATE-4 Verdict — Story 10-4 (Native Overlay DPI Scaling + Appearance-Wiring)

Status: **review** — machine side GREEN, real-machine visual smoke = Andi's residual.

## Self-verification (machine side, WSL) — DONE

- `cargo check --target x86_64-pc-windows-gnu` (win-gnu surface harness, recipe `gate4-evidence/10-1/win32-surface-check.md`): **0 errors** (pre-existing warnings only, same class as 10-1/10-3). Win32_UI_HiDpi feature + GetDpiForMonitor imports compile clean.
- `cargo test` (Linux): **green** (no functional logic on the Linux path changed; only the Windows `scale` source).
- Code-review: 3 adversarial reviewers (Blind / Edge-Case / Acceptance), **CLEAN**. One confirmed AC-5 finding (stale saved-coordinate re-scaling) fixed via work-area clamp (`795d5b3`); preview already clamps; multi-monitor mixed-DPI findings deferred (single-monitor setup → N/A).

## Why the rest is NOT WSL-certifiable (residual for Andi)

The absolute DPI scale, font-preset legibility, and visual appearance render only on a real high-DPI Windows display. WSL cannot run the Windows binary or observe native layered-overlay pixels. Per project-context's rendering-oracle rule + the story's Verification-Symmetry section, these are genuine real-target judgments. Andi CAN produce the test state (his monitor is at 125/150 %; the AC-1 log is observable in `Klarvo.log`).

## Andi's GATE-4 smoke checklist (real Windows machine, 125/150 % DPI)

Build: `scripts/sync-and-build.ps1` (then `rsign` per signing gotcha).

1. **AC-1** — Open `Klarvo.log` (Settings → About, or `%APPDATA%\com.klarvo.voice\klarvo.log`). On overlay creation, expect two lines like:
   `[native_pill] DPI: GetDpiForMonitor=144 GetDeviceCaps(legacy)=96 scale_real=1.500 scale_was=1.000`
   `[native_preview] DPI: ...` — confirms the legacy call was returning 96 (scale 1.0) and the real scale is now ~1.5.
2. **AC-2** — Trigger recording. Pill (logo + waveform + red cancel button) and preview card should now look the **same apparent size as before Epic 10** (no longer tiny).
3. **AC-3** — Change an appearance setting (e.g. preview font size = large, or a border color), save, start next recording → the change is visibly reflected in the preview.
4. **AC-4** — Compare small / medium / large preview font: the three should be **perceptibly distinct**. (Per your decision: if after the DPI fix they still feel too close, that's a separate calibration step — give the target sizes and we adjust the 11/13/15 mapping; do not expect it to be pre-tuned.)
5. **AC-5** — Pill drag + position persistence still work; the pill is **on-screen** at startup (the clamp guarantees this even if your saved position was stale); recording/transcription/cleanup/done/error states + standby recreate-on-start unchanged.

GREEN on all → conductor flips both status fields to `done`. Any FAILED → re-opens via a fresh dev worker (never bare-loop hot-patch).
