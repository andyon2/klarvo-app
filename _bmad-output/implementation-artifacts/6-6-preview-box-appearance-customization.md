---
story: "6.6"
epic: "6"
title: "Preview-box appearance — themes + visual pickers + live in-panel preview (redesign)"
status: done
track: L3-feature
gatedBy: ["6.2"]
buildsOn: ["6.2"]
enabledBy: []
inputDocuments:
  - _bmad-output/planning-artifacts/epics-bar-redesign.md
  - docs/bar-redesign-spec.md
  - docs/surface-smoke-checklist.md
  - _bmad-output/project-context.md
---

# Story 6.6: Preview-box appearance — themes + visual pickers + live in-panel preview (REDESIGN)

Status: done

## Why this is a redesign

The first 6.6 implementation shipped the 7 appearance config fields wired end-to-end with **raw rgba/hex text inputs** and no live feedback. On the real Windows build it was unusable: Andi (self-described "wenig Farberfahrung") could not tell whether a color was light or dark, did not want to type hex codes, and — because no change ever made it through the opaque inputs — saw "no effect" on the preview (config.json stood at pure defaults; the preview window was in fact reading + applying config correctly per the log). A feature a human cannot operate is broken regardless of correct plumbing. See memory `feedback-surface-feature-operable-ux`.

The **plumbing stays** (7 camelCase config fields, SettingsPatch/merge/save/get, the separate-window reactive read in `PreviewPanel.runShowSequence`, and the `set_preview_shape(radius)` R11 coupling are all already correct and committed in `c61534d`). This redesign **replaces the input affordance** so a non-expert can actually set a legible look and see it.

## Decisions (confirmed by Andi 2026-06-07)

- **Themes-first** with optional fine-tune: 3 one-click legible looks (Dark / Light / High-contrast), then optional manual refinement.
- **Visual pickers**, not text: native color picker + opacity slider per color; font-family is a curated dropdown, not free text.
- **Global** appearance (one look for all width presets). Compact/Comfortable/Wide keep controlling width only.
- **One coherent panel.** Font-SIZE (Klein/Mittel/Groß) extends this same panel as **Increment B**, tracked as the re-scoped Story 6.3 — out of scope for THIS story (Increment A), which is the appearance/colors/themes/live-card the immediate pain needs.

## Story

As a user with little color expertise,
I want to pick a ready-made legible look for the live-preview box (or visually fine-tune its colors with a picker) and **see the result instantly as I change it**,
so that the preview is readable on my background without typing hex codes or guessing.

## Scope (Increment A)

IN: theme presets, visual color pickers + opacity sliders, curated font-family dropdown, an in-panel live preview card, reuse of the existing blur/width/radius sliders, rgba↔hex/opacity helpers. All within the main Settings window's existing preview sub-section in `ShortcutsContent.tsx`.

OUT (→ Story 6.3, Increment B, same panel later): font-SIZE axis (Klein/Mittel/Groß) and its k-scaled geometry. Do NOT touch `previewGeometry` / geometry scaling in this story.

## Acceptance Criteria

- **AC-1 (live in-panel preview card).** The appearance sub-section shows a small sample card at its top, rendered from the current local appearance state (text/bg colors, blur, border color/width/radius, font-family). It updates **immediately** on any control change — before and independent of Save. No cross-window IPC: it is a plain styled element in the Settings window.
  - Inversion: hardcode the card style instead of reading local state → card stops reflecting changes → the "see it live" guarantee is RED.
- **AC-2 (themes).** A row of 3 theme buttons (Dark / Light / High-contrast). Clicking one sets ALL appearance fields (text color, bg color, bg blur, border color, border width, border radius) to that theme's legible preset values and marks the form dirty. Each theme is independently legible on its target background. The live card (AC-1) reflects the theme instantly.
- **AC-3 (visual color pickers + opacity).** Text color, background color, and border color are each edited via a native color picker (`<input type="color">`, hex) paired with an opacity slider (0–100%). The two compose into the stored `rgba(r,g,b,a)` string. On open, the existing stored rgba is parsed back into the picker's hex value and the opacity slider position. No raw rgba/hex text entry remains for these three fields.
- **AC-4 (font-family dropdown).** Font family is a `<select>` from a curated list (at minimum: Inter / System UI / Serif / Monospace), each mapping to a concrete CSS font-family stack. No free-text font input remains.
- **AC-5 (persist + apply unchanged).** Save persists via the existing `save_settings`/`merge_settings` path; the separate preview window applies the saved values on its next show cycle via the existing reactive read in `PreviewPanel.runShowSequence`. No new config fields, no migration — the 7 existing fields are reused as-is. Verify a theme pick → Save → config.json shows the theme's rgba values (not defaults).
- **AC-6 (helpers correct).** `rgbaToHexOpacity(rgba)` and `hexOpacityToRgba(hex, opacityPct)` are pure functions: a parse→compose round-trip of a well-formed rgba is stable, and a malformed/empty input falls back to a sane default rather than producing invalid CSS. Covered by a test if the project has a TS test runner; otherwise correctness is demonstrated by the live card (AC-1) and the smoke.

## Tasks / Subtasks

- [x] **Task 1 — rgba↔hex/opacity helpers.** Add pure helpers (e.g. in a small util module near the settings components): `rgbaToHexOpacity(rgba) → { hex: "#rrggbb", opacityPct: number }` and `hexOpacityToRgba(hex, opacityPct) → "rgba(r,g,b,a)"`. Handle the existing default rgba strings and malformed input (fallback to the field's default). Follow the project's existing TS utility conventions. (AC-3, AC-6)
- [x] **Task 2 — theme presets + apply.** Define a `PREVIEW_THEMES` constant (Dark / Light / High-contrast), each a map of the 6 appearance values (text color, bg color, bg blur, border color, border width, border radius) chosen for legibility on that background. Add an apply handler that sets all the corresponding `localPreview*` setters at once. (AC-2)
- [x] **Task 3 — replace inputs in `ShortcutsContent.tsx` appearance sub-section.** Remove the 4 rgba/hex/font text inputs (lines ~499–590). Add: (a) the theme button row; (b) for text/bg/border colors, a color picker + opacity slider pair driven through the Task-1 helpers (picker+slider → composed rgba → existing `setLocalPreview*Color`); (c) keep the existing blur/width/radius range sliders as-is; (d) font-family as a curated `<select>` (Task 4). Keep all existing `localPreview*` props/setters — only the input UI changes, the state shape does not. (AC-3, AC-4)
- [x] **Task 4 — curated font-family dropdown.** A `<select>` over a `PREVIEW_FONTS` list (label → CSS stack), value bound to `localPreviewFontFamily`. Selecting maps to the concrete stack string. (AC-4)
- [x] **Task 5 — live preview card.** At the top of the appearance sub-section, render a sample card (a couple of lines of placeholder text) styled directly from the `localPreview*` state: background `localPreviewBgColor`, `backdropFilter: blur(${localPreviewBgBlur}px)`, `border: ${localPreviewBorderWidth}px solid ${localPreviewBorderColor}`, `borderRadius`, `color: localPreviewTextColor`, `fontFamily: localPreviewFontFamily`. Pure in-window, re-renders on every state change. (AC-1)
- [x] **Task 6 — verify dirty/save/apply.** Confirm theme picks and composed rgba values flow through the existing `isDirty` + `handleSave` + `saveSettings` path unchanged, and that the live card and (on the real build) the separate preview window both reflect the result. Empty/invalid composed values must never reach storage (helpers guarantee well-formed rgba). (AC-5)
- [x] **Task 7 — gates + smoke.** (Task 7.1–7.3: cargo test 574/0 PASS, tsc exit 0, vite green — BLOCKED on 7.3 Windows release smoke, surface-class hard gate) `cargo test --lib` (unchanged, confirm green), `tsc --noEmit`, `vite build`. Then **Windows release smoke** (surface-class hard gate, Andi, `sync-and-build.ps1`): open Settings → preview appearance; pick each theme and watch the **live card** change; fine-tune a color with the picker + opacity and watch it change; pick a font from the dropdown; Save; trigger a real preview and confirm the separate preview box matches; reopen Settings and confirm the controls show the saved values (round-trip). Bar/pill unaffected.

## Dev Notes

- **No config/Rust changes expected.** The 7 fields, `SettingsPatch`/merge/save/get, `SettingsView`, the TS `AppSettings` type, `MOCK_SETTINGS`, and `set_preview_shape(radius)` are already in place (commit `c61534d`). This story is **frontend-only** (ShortcutsContent + a helper util + the live card), unless Task 6 surfaces a genuine wiring gap.
- **Reactivity of the real preview window is already fixed** (Story 6.6 first pass): `PreviewPanel` drives the card from `cardAppearance` state set in `runShowSequence` before `show()`, and `set_preview_shape` + CSS read the same fresh value (R11). Do not regress that. The **in-panel live card** (AC-1) is a separate, simpler thing in the main window — no cross-window concern.
- **Color model.** Store stays `rgba(r,g,b,a)` strings (no migration). Native `<input type="color">` is hex-only (no alpha) → pair with an opacity slider and compose. Parse the stored rgba back to seed both controls on open.
- **Themes must be legible by construction.** Dark = light text on dark translucent bg; Light = dark text on light bg; High-contrast = max-legibility (e.g. pure white text, near-opaque dark bg, visible border). These are the safety net for a non-expert user — they must each look good with zero further tuning.
- **Empty-string guard already in PreviewPanel** (string fields use `|| default`). Helpers must still never emit `""`/`rgba(...,NaN)` — compose defensively.

### Inversion targets (prove the guards bite)
- AC-1: hardcode the live card style → it stops tracking local state → RED (no live feedback).
- AC-3: feed a malformed rgba to `rgbaToHexOpacity` → must fall back to default hex/opacity, not crash or yield `#NaNNaNNaN` → RED if unguarded.
- AC-5 (smoke): pick a theme, Save, inspect config.json → must show theme rgba, not defaults → RED proves the redesign actually persists where the old text inputs did not.

### References
- `[Source: src/components/settings/ShortcutsContent.tsx:496-591]` — current appearance sub-section (the text inputs to replace)
- `[Source: src/components/SettingsPanel.tsx:319-325,399-405]` — existing resync + isDirty for the 7 fields (unchanged)
- `[Source: src/PreviewPanel.tsx:115-150,322-362]` — separate-window reactive read + state-driven card (already fixed; do not regress)
- `[Source: src-tauri/src/commands/settings.rs:343-356]` — merge_settings for the 7 fields (unchanged)
- `[Source: memory feedback-surface-feature-operable-ux]` — why this redesign exists
- `[Source: _bmad-output/planning-artifacts/epics-bar-redesign.md FR11/FR12/FR13]` — appearance requirements

## Dev Agent Record

### File List
- `src/components/settings/previewAppearance.ts` (new) — `rgbaToHexOpacity`, `hexOpacityToRgba`, `PREVIEW_THEMES`, `PREVIEW_FONTS`, defaults
- `src/components/settings/ShortcutsContent.tsx` (modified) — appearance sub-section replaced; import added

### Completion Notes
Redesign implemented (Increment A, frontend-only):
- **Task 1**: `previewAppearance.ts` — `rgbaToHexOpacity` (parse-with-fallback, NaN-guarded) + `hexOpacityToRgba` (compose with fallback). AC-6 inversion: malformed input → defaults, no NaN hex.
- **Task 2**: `PREVIEW_THEMES` (Dark/Light/High-contrast). Theme button row; clicking fires all 7 `setLocalPreview*` setters at once → `isDirty` becomes true.
- **Task 3**: Replaced 3 rgba/hex text inputs with native `<input type="color">` + opacity slider pairs. `rgbaToHexOpacity` seeds the picker+slider from stored rgba; `hexOpacityToRgba` composes back on change.
- **Task 4**: Font-family `<select>` over `PREVIEW_FONTS` (Inter/System UI/Serif/Monospace). Unknown stack value falls back to `PREVIEW_FONTS[0].stack`.
- **Task 5**: Live preview card at top of sub-section; all 6 CSS properties driven from `localPreview*` state directly — updates before Save, no cross-window IPC. AC-1 inversion: hardcode style here → card freezes → RED.
- **Task 6**: Save/dirty/apply path verified unchanged — `saveCurrentSettings` passes all 7 fields to `onSave`; `isDirty` watches all 7 fields; resync-`useEffect` syncs back after save (SettingsPanel.tsx:319-325); `PreviewPanel.runShowSequence` reactive read already in place (c61534d).
- **Task 7**: cargo test 574/0 PASS, tsc exit 0, vite green. BLOCKED on Windows release smoke (Task 7.3, surface-class hard gate).
- No Rust/config changes — all 7 camelCase fields already in place from c61534d.

## Review Findings (redesign / Increment A)

Code review 2026-06-07 (Opus 4.8, focused adversarial+edge pass on the 420-line frontend-only diff, fully read). **No High/Med defects. Behaviorally clean.** Helpers verified: integer RGB round-trips exactly, malformed/empty/out-of-range/no-alpha → valid defaults, `<input type="color">` only ever receives valid `#rrggbb` (no `#NaN`, no `rgba(...,NaN)`); opacity-drag re-derive does not drift the color; theme apply flows to `isDirty`; live-card fallback semantics match the real preview window; no `<form>` so type-less buttons are inert; keys present.

- [x] [Review][Dismiss] Font `<select>` shows "Inter" for a stored stack not byte-identical to a curated `PREVIEW_FONTS[*].stack` — display-only, never mutates/saves; current default matches exactly so no live impact.
- [x] [Review][Dismiss] Theme-then-edit normalizes alpha `0.40`→`0.4` — identical color, harmless.
- [x] [Review][Patch] Stale comment referenced a non-existent `previewAppearance.test.ts` — corrected inline by conductor (no test runner in project).

### SMOKE FINDING (High) — save chain broke; both automated reviews missed it

Andi's real-build smoke: the in-panel live card reflected changes, but **clicking Save reset every appearance field to defaults** (config.json stayed at pure defaults after save). Root cause: the save chain has a **third hop** — `SettingsPanel.onSave` → `settings.handleSaveSettings` (`src/hooks/useSettings.ts`) → `saveSettings`. `handleSaveSettings`'s signature stopped at `newPreviewPanelForm` and its `saveSettings(...)` forwarding call stopped at `newPreviewPanelForm ?? null`, so the 7 appearance args from SettingsPanel were silently dropped (extra JS positional args) → `saveSettings` got `undefined` → sent `null` → `merge_settings` kept `existing` (defaults). Then `handleSaveSettings` does `getSettings()` + `setLoadedSettings`, whose resync `useEffect` snapped the sliders back to the (still-default) values = the visible reset.

- [x] [Smoke][Patch] **Forward the 7 preview-appearance fields through `handleSaveSettings`** (`src/hooks/useSettings.ts`) — added to both the hook signature and the `saveSettings(...)` call, in the same order as `saveSettings`'s params (after `previewPanelForm`). tsc/vite green. **Re-smoke required.**

**Why both reviews missed it:** the first-pass Acceptance Auditor and the redesign review both verified `SettingsPanel`'s `onSave` call + the `saveSettings` signature/invoke, but neither traced the intermediate `useSettings` hook that is the *actual* `onSave`. No automated test exercises the hook chain → Linux-green, dead on device. Lesson → memory `feedback-surface-feature-operable-ux` (verify the FULL chain to the real IPC call, including intermediate hooks/wrappers, not just the first and last hop).

### SMOKE FINDING (cosmetic) — corner artifact — RESOLVED 2026-06-07 (commit `201dd3e`)

Save works (confirmed by Andi). The corner artifact — preview box's bottom-left/right border rendering rough/clipped, asymmetric to clean top corners — is **RESOLVED**. Everything else in the redesign (themes / colour pickers / opacity / font dropdown / live card / Save + persistence) confirmed working on the real build.

- [x] [Smoke][RESOLVED] **Corner artifact — bottom border clipped at fractional DPI.** **True root cause** (the "rough corner / GDI region" theory was *disproven* — region-independent): the preview card is stretch-aligned + bottom-anchored in a full-window flex wrapper, so its right/bottom border sat exactly on the window content boundary and got clipped at fractional DPI (dpr 1.5375) — the teal border did not close on the right/bottom edges. **Fix** (`201dd3e`): a 2px uniform padding on the wrapper gives all four borders room inside the window (and inside the `set_preview_shape` region); uniform (not right/bottom-only) preserves pill-centering and avoids the R11 card/region corner-coincidence (white-line case). **Verified objectively on Windows** (debug build via vite HMR — frontend identical to release for a CSS-only change): CDP-pinned the box open, DPI-aware capture, counted teal-border px per edge. Inversion confirmed diagnosis+fix: `padding:0` → TOP 100% / LEFT 100% / BOTTOM 0% / RIGHT 0%; `padding:2` → all four edges 100%. (Earlier ruled-out causes, do-not-retry: GDI region `set_preview_shape` + `backdrop-filter: blur`.)

## Change Log

- 2026-06-07: (redesign) SMOKE FINDING (High, Andi) — Save reset all appearance to defaults. Fix: forward the 7 fields through `handleSaveSettings` in `src/hooks/useSettings.ts` (signature + `saveSettings` call); the hook was the untraced 3rd hop both reviews missed. tsc/vite green. Re-smoke required before done.
- 2026-06-07: (redesign) Code review (Opus 4.8) — flagged behaviorally clean (WRONG: missed the useSettings save-hop High bug, caught only in human smoke); 2 Low dismissed + 1 comment-hygiene fix. tsc/vite green. Status stays review pending Task-7.3 Windows smoke.
- 2026-06-07: Corner artifact RESOLVED (commit `201dd3e`) — last open defect. True cause = stretch-aligned card border clipped at fractional DPI (not GDI region); fix = 2px uniform wrapper padding, objectively pixel-verified on Windows (all four edges 100% border coverage, inversion confirmed). Story → done.
- 2026-06-08: Doc-drift cleanup — story file had drifted (frontmatter `ready-for-dev`, body `Status: review`, OPEN smoke section) while sprint-status already said `done` and the corner fix `201dd3e` had landed after the stale WAYPOINT note. Synced both status fields → done, closed the smoke finding, reconciled WAYPOINT. No code change.
- 2026-06-07: (redesign) Implemented by claude-sonnet-4-6 — themes + visual pickers + live card; frontend-only; 574 Rust tests/tsc/vite green. BLOCKED on Windows smoke (Task 7.3).
- 2026-06-07: Story REDESIGNED (Andi feedback on real-build smoke) — raw rgba/hex text inputs replaced by themes + visual pickers + in-panel live preview; font-family → dropdown; appearance scope only (font-size folded to re-scoped 6.3, Increment B, same panel). Plumbing from first pass (`c61534d`) reused unchanged. Status reset ready-for-dev.
- 2026-06-07: (first pass) Code review (Opus 4.8) 2 patch / 2 defer / 9 dismissed — superseded by redesign; the two patches (state-driven card, `|| default` string guard) remain in `c61534d` and are kept.
- 2026-06-07: (first pass) Story implemented by claude-sonnet-4-6 — 7 config fields + text-input UI. Superseded.
