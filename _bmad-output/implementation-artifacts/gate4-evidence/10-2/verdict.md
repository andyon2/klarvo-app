# GATE-4 Evidence — Story 10-2 (Native Preview Overlay)

Conductor: claude-opus-4-8 · Date: 2026-06-27 · Branch: conductor/epic-10 · Range: b658320..0348c38
Commits: f7e1c65 (story+GATE-1) · 2b2fbae (dev) · d23db1c (review fixes) · 0348c38 (review close-out)

## Self-verification (WSL-observable — done by conductor, objective results)

| Check | Result |
|---|---|
| Linux `cargo test --lib` (full suite) | **630 passed, 0 failed** |
| Win32 surface compile `cargo check --target x86_64-pc-windows-gnu` | **0 errors** (24 pre-existing `#[must_use]` BOOL warnings) |
| Frontend `npm run build` (tsc + vite) | **0 errors**, 78 modules |
| Removal surface (no live refs to deleted `PreviewPanel`/`create_preview_window`/`set_preview_shape`/`transcribe_live_preview`/`WEBVIEW2_BROWSER_ARGS`/`create_bar_window`) | **clean** — only one stale doc-comment in `src/types.ts:96` (references deleted `set_preview_shape`; harmless, noted) |
| Code review (3-reviewer adversarial: Blind/Edge/Auditor, Opus) | 5 Patch **fixed + re-verified** at d23db1c · 7 Defer (residual) · 9 Dismiss |

The 5 review fixes (all in `native_preview.rs`): overflow text anchor (newest now at bottom),
card-color double-premultiply (teal border no longer ~invisible), `rebuild_dibs` use-after-free on
the DIB-failure path, reposition-while-hidden stale DIB, discarded text alpha (0.88 now honored).

## Residual for Andi — REAL-TARGET WINDOWS GATE (not machine-claimed)

These are genuinely Windows-only / user-reachable; the conductor cannot observe them from WSL
(the preview only renders during a live recording on the real build). Run on a real Windows
release build via `scripts/sync-and-build.ps1`:

1. **Occlusion harness (machine, AC-4):** start a recording so the preview shows, then run
   `scripts/preview-occlusion-proof.ps1`. Expected: PASS — content pixels survive a maximized
   foreground window + 3 s dwell. Evidence lands in this dir (`structure-*.txt` / captures).
2. **Visual smoke (AC-2/AC-3, NFR-2):** preview card looks right — dark card, **teal hairline
   border visible** (was the double-premult bug), text in **Segoe UI**, **newest text anchored at
   the bottom** with the oldest fading at the top (was the overflow bug), anchored above the pill,
   **click-through** (clicking the card hits the app behind it), and it **repositions when you drag
   the pill**. Hidden when not recording.
3. **Standby smoke (AC-6, NFR-2):** record once (preview appears) → real sleep/Modern-Standby/
   lock-resume → record again → **preview reappears** (the present-loss class that broke the pill
   on 2026-06-27; user-reachable, so it is your gate not a machine claim).

GREEN on all three → conductor flips both status fields to `done` + Change-Log close-out.
Any FAILED → re-opens into the gated dev flow (fresh dev worker), never a bare-loop hot-patch.

## Deferred residuals (reported, not fixed this story — see story `### Review Findings`)

Geometry degenerate-case guards (h_max=0 / clamp inversion); multi-monitor + per-monitor-DPI
(mirrored substrate limit, shared with the pill); `parse_css_rgba` rgb/rgba-only (verify Settings
color-picker output format); `bar-moved` (0,0) defensive guard; rare CreateWindowExW/append-chunk
leaks; `line-height:1.5`/`letter-spacing` not reproduced (GDI cost — flag in visual smoke);
occlusion-harness `$EvidenceDir` cosmetic typo + PASS-criterion validation at run time.
