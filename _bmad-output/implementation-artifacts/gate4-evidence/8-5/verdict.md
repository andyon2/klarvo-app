# GATE-4 evidence — Story 8-5 (Main-Window / History re-skin)

Date: 2026-08-18 · Range `7cb5f6f..c1a84d8` · Branch `feat/8-5-history-reskin`

## Method

The desktop surface is React in a webview, so the unattended proxy is the real
Vite dev server rendered in real Chromium (Playwright, cached chromium-1228),
running in **preview mode** — `window.__TAURI_INTERNALS__` is absent, so
`tauri-commands.ts` serves its mock data and no Tauri runtime is needed. This is
the same method Story 8-2 used at its GATE 4.

Two states are unreachable from the shipped mock data. Both were produced by a
**throwaway edit to `src/tauri-commands.ts` that was reverted afterwards** — the
working tree is clean and no probe code is committed:
- empty history → `MOCK_HISTORY` set to `[]`
- pending entry (Story 12-2 state) → one `status: "pending"` entry appended

## Measured (structural — this IS the unattended gate)

| AC | Claim | Measured | Verdict |
|---|---|---|---|
| #1 | timestamps in Geist Mono | `font-family: "Geist Mono", ui-monospace, "Cascadia Code", monospace` | PASS |
| #1 | app tag amber | `color rgb(233,162,76)` = `#E9A24C` = canon `--k-amber` | PASS |
| #1 | app tag border | `rgba(233,162,76,0.32)` = canon `--k-amber-line` | PASS |
| #1 | app tag is a pill | `border-radius` fully rounded | PASS |
| #1 | card density `p-3.5` | `padding: 14px` on all normal cards | PASS |
| #1 | card hover lift | bg and border both change on hover | PASS |
| #2 | designed empty-state | "No dictations yet" + clock icon renders | PASS |
| #2 | distinct no-results | "No results / No dictations match your search" | PASS |
| #3 | search fill `surface-2` | `rgb(27,30,32)` = `#1B1E20` = `--color-klarvo-surface-2` | PASS |
| #3 | teal focus affordance | focus sets teal border + 1px teal ring (not `.focus-klarvo`) | PASS |
| #7 | no Inter override | `main` = `Geist, ui-sans-serif, system-ui, sans-serif` | PASS |
| P1 | pill is `inline-flex` | `display: inline-flex; align-items: center` at render | PASS |
| P2 | decorative SVG hidden | clock svg carries `aria-hidden="true"` at render | PASS |
| D2 | hotkey line desktop-only | renders on desktop viewport (mobile path not exercised) | PARTIAL |
| D3 | pending card aligned | `padding: 14px`, same as normal cards | PASS |
| D3 | amber survives hover | base amber α0.1/border α0.4 → hover α0.2/border α0.6, hue identical | PASS |

Both GATE-1 design decisions are confirmed at render, not merely in source.

## Not decided here

- **Pixel / aesthetic verdict on the real target.** Chromium is a close relative
  of WebView2 but not identical: font rasterisation, DPI scaling and the Windows
  text-scale-factor drift (see `reference-windows-text-scale-factor-webview-gdi`)
  are not reproduced here. The look on the real Windows release build stays
  Andi's gate.
- **D2 on Android.** The `isDesktop` gate was verified only in its desktop
  branch. The mobile branch was not rendered.
- **`max-h-[calc(100vh-250px)]` against the taller empty-state**, sticky hover on
  touch, and the widened meta row — recorded as deferred, device-observable only.

## Artefacts

`ist-populated.png` · `ist-no-results.png` · `ist-empty.png` ·
`ist-cards-normal.png` · `ist-cards-pending.png` ·
`measurements-populated.json` · `measurements-empty.json` ·
`measurements-cards-normal.json` · `measurements-cards-pending.json`
