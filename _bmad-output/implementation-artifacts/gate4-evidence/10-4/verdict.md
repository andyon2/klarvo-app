# GATE-4 Verdict — Story 10-4 (re-scoped: user-tunable overlayScale)

Status: **review** — machine side GREEN; the real-machine step is now Andi's **size tuning**.

## What the GATE-4 smoke proved (2026-06-28) — original hypothesis REFUTED

- AC-1 log on Andi's machine: `GetDpiForMonitor=120 GetDeviceCaps(legacy)=120 scale_was=scale_real=1.250`.
  Both APIs return 120 → the DPI scale was **always correct (1.25)** on his single primary monitor. The
  "scale=1.0 bug" never existed there; the DPI fix (a0334bf) was a **no-op**.
- Conductor measured the pill from Andi's full-width screenshot: **249 px** wide on a 1918 px (≈ physical
  1920) screen. Designed = 200 logical × 1.25 = **250 px**. → Native pill renders **1:1 at designed size**,
  identical to the old WebView2 pill. No DWM virtualization, no scaling regression.
- Conclusion: "too small" is **by design**, not a defect. Enlarging = a taste decision. Andi chose a
  **self-tunable size knob** (Verifikations-Symmetrie: he produces the test state himself, no rebuild per step).

## Self-verification (machine side, WSL) — DONE for the overlayScale change

- Linux `cargo check` (full crate): clean (exit 0).
- Golden-master roundtrip test `test_appconfig_golden_master_full_field_roundtrip`: PASS — confirms
  `overlayScale` persists through save/load (it is set to a non-default 1.3 in that test).
- win-gnu scratch harness (native_pill + native_preview, refreshed from current source, clean recompile):
  **0 errors**, 55 pre-existing warnings.
- `NativePill::create` signature (+`overlay_scale: f64`) ↔ `lib.rs:756` call-site: consistent;
  extraction `_overlay_scale` is `_`-prefixed so the Linux build stays warning-free.
- Review (direct, Opus): coupled model `scale = dpi_scale × overlayScale` in both threads; at 1.0 the
  render is pixel-identical to today (the multiply is ×1.0). CLEAN.

## Andi's tuning-smoke (real Windows machine)

1. **Build** the new version: `scripts/sync-and-build.ps1` (+ `rsign`). This build contains the
   `overlayScale` reader; the old build does not.
2. **First run:** overlays look exactly as now (overlayScale defaults to 1.0).
3. **Tune:** edit `config.json` at
   `%APPDATA%\com.klarvo.voice\config.json` → add/set `"overlayScale": 1.3` (1.3 = ~30 % bigger; try
   1.2–1.5). **Restart the app** → both pill and preview render at the new size.
4. Repeat step 3, changing the number, until the size feels right. Tell the conductor the value you
   settled on (it just lives in your config; no code change needed unless you want a different *default*).
5. Note: if your pill has a saved drag position, changing the factor may shift it once — drag it back, it
   re-persists at the new factor. Expected for a tuning knob.

GREEN (you found a comfortable value) → conductor flips both status fields to `done`. If `overlayScale`
has **no visible effect** or something regresses → re-opens via a fresh dev worker.
