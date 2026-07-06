# GATE-4 Evidence — Story 8-2 (Settings Studio-Dark re-port)

**Date:** 2026-07-06 · **Branch:** feat/epic-8-studio-dark-port · **HEAD:** 4896801
**Method:** Desktop surface — Contract smoke is Android-only, so degraded to the proven
`epic-8-fidelity-audit.md` method: built React rendered via `npm run dev` (preview-mode, no
Tauri backend) in headless Chromium (Playwright), driven Settings→sub-pages, screenshots +
structural DOM assertions. Real-device Windows aesthetic + config round-trip = Andi's residual.

## Self-verification (what I ran + objective results) — GREEN at observable layers
- **Build:** `npm run build` (tsc+vite) green, 0 TS errors, at HEAD 4896801 (independent re-run).
- **Task-6 gates:** 0 alias tokens; 0 native `<select>/range/switch` in consumers (FormControls excepted). Hold.
- **Settings Home:** renders — 9 category rows, icon-badges (teal/amber/grey), title+subtitle, chevrons. Matches mockup layout. Status-dots correctly ABSENT (deferred → 8-7). → ist-settings-home.png
- **AI & Providers sub-page:** provider status-dots, mono masked keys, TRIAL badges, Presets, App-Profiles. Matches mockup. → ist-ai-providers.png
- **KSelect portal fix (the HIGH review finding) — CONFIRMED:** dropdown portals to document.body, `position:fixed`, renders UNCLIPPED over the panel (mic select: System Default/Default Microphone/USB Headset visible past panel edge). → ist-kselect-open.png
- **KSegmented / KSlider:** render with correct active-state (Cloud/Offline segmented teal-active). → ist-recording-audio.png
- **Console:** only a preview-mode-only Tauri `listen()` artifact (SettingsPanel.tsx:198, pre-existing wiring, NOT 8-2; non-fatal — panel rendered). No 8-2-caused errors.

## Observed, but NOT 8-2 defects (recorded, not blocking)
- **Always-on focus ring (PRE-EXISTING, from 8-1):** `.focus-klarvo` (styles.css:253) is a bare class
  (no `:focus-visible`) → applies a permanent teal ring to every input/select. Mockup shows the ring on
  `:focus` only. Byte-identical to the approved reference build; introduced by 8-1's Task-5.3, not 8-2.
  → backlog item (own story: it touches the shared token layer / all surfaces).
- **Reference-vs-mockup fidelity deltas** (depth/status-dots/etc.): the deferred 8-7 fidelity pass.

## Residual for Andi (provably NOT observable from WSL preview — the real gate)
1. **Final aesthetic on the real Windows release build** (`sync-and-build.ps1`) — the pixel/aesthetic verdict.
2. **Live config round-trip:** change a setting (e.g. a KSelect, a KToggle, a KSlider) → Save → confirm it
   persists to `config.json` with correct camelCase key (preview has no Tauri backend, cannot persist).
