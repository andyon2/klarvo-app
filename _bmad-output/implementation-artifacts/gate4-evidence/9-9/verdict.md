# GATE-4 evidence — Story 9-9 (in-app recording state re-skin)

Date: 2026-06-21 · Conductor: bmad-story-conductor (interactive) · Commit range: c2452e38..3b2d7ee

## Surface class

9-9 is a **WebView in-app surface** — the React `RecordButton` (+ status label + raw-text area)
in `src/App.tsx`, rendered inside `TauriActivity`'s WebView. It is **not** a native overlay window.
Therefore the contract's `visual_oracle.structural_method` (`dumpsys window windows` counting
APPLICATION_OVERLAY windows) — built for the bubble overlay stories 9-3..9-5 — **does not apply**.
The strongest unattended machine assertion for a token re-skin of a WebView surface is
**artifact-level token-chain correctness**: does the built web bundle apply the canon token values?

## Machine verification (conductor, GREEN)

1. **Source** (`git show 3b2d7ee:src/App.tsx`, lines 51–92 + status label + raw-text):
   - RecordButton recording state → `bg-klarvo-warning/20 text-klarvo-warning` + glow `rgba(233,162,76,0.3)` (amber)
   - busy glow → `rgba(233,162,76,0.2)` (amber, FIX 1); idle glow → `rgba(41,199,172,…)` (teal, FIX 2)
   - pulse-ring → `border-klarvo-warning opacity-40 animate-ping` (full-alpha, FIX 3)
   - status-label recording → `text-klarvo-warning`; error stays `text-klarvo-danger`
   - raw-text → `bg-klarvo-bg-deep`
   - Only ONE `RecordButton` definition; it contains **no** `danger` / `red-400` / old-red-glow residual.

2. **Built CSS** (`dist/assets/index-B09_KOLh.css`, 01:45:49 = fix build):
   - Canon tokens defined: `--color-klarvo-amber:#e9a24c`, `--color-klarvo-teal:#29c7ac`,
     `--color-klarvo-bg-deep:#0a0b0c`, `--color-klarvo-warning→amber`, `--color-klarvo-primary→teal`.
   - Fixed shadow utilities emitted: `rgba(233,162,76,0.3)`, `rgba(233,162,76,0.2)`, `rgba(41,199,172,0.2)`.
   - Token utilities emitted: `bg-klarvo-warning/20`, `text-klarvo-warning`, `border-klarvo-warning`, `bg-klarvo-bg-deep`.

3. **Shipped App JS bundle** (`dist/assets/index-C6_kJ92b.js`, 01:45:49):
   - Applies all fixed RecordButton classes (positive controls = 1+).
   - Negative-control hits (`bg-klarvo-danger/20`, `border-red-400 ping`, `rgba(255,115,105)`×16)
     all trace to **other** components — `Onboarding.tsx` (a record-button mock, separate surface/story),
     `FloatingBar.tsx`, `ThemeSwitcher.tsx` — **not** the in-app RecordButton.

4. **Build / smoke**: `npm run build` PASS (82 modules, tsc clean); `scripts/android-smoke.sh` EXIT 0
   (24 JVM tests green, KlarvoTheme drift-gate OK, APK built v0.5.0 + installed on device).

## Residual for Andi (real-device PIXEL/aesthetic gate — the only un-machine-observable part)

The emulator is **not** a pixel oracle (contract `visual_oracle.pixel=false`), and the in-app recording
states need the WebView driven (tap mic). The APK is already installed on the device (dev install).
Open Klarvo → main screen and confirm:

1. **Idle** → RecordButton mic = teal.
2. **Recording** (tap mic) → button + pulse-ring = **amber** (was red). This is the GATE-1 decision.
3. **Processing** (tap again) → amber spinner.
4. **"Show original"** expanded → raw-text background = deepest dark, consistent with surroundings.

## Observation (out of 9-9 scope, for the onboarding re-skin)

`src/Onboarding.tsx:1093/1104` still renders a red record-button mock (`bg-klarvo-danger/20`,
`border-red-400 animate-ping`). That is the onboarding surface, a separate story — note for whoever
does the onboarding re-skin so its mock matches the new amber live-indicator.
