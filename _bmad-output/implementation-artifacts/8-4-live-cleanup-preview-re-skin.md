# Story 8.4: Live-Cleanup-Preview re-skin

Status: done

## Story

As a user reading along while dictating,
I want the live preview panel to present the raw transcript calmly and legibly,
so that I can orient on what's being captured without distraction.

## Acceptance Criteria

1. **Given** preview is enabled and recording starts **When** raw chunks arrive **Then** the live **RAW** transcript renders in **Geist Mono** on the transparent, bottom-anchored panel using named Studio-Dark tokens (live LLM cleanup stays off by design — only the raw stream is live).

2. **Given** the panel opens/closes **When** expand/collapse fires **Then** the motion uses `--motion-panel` (320ms) with `--ease-standard` `cubic-bezier(.2,0,0,1)` and is smooth; `prefers-reduced-motion` is honored (the `@media` block in `styles.css` already collapses `--motion-panel` to 1ms — no extra work needed).

3. **Given** the dark-background legibility issue **When** text renders **Then** the raw text is clearly legible — the dim-text trap is resolved: text uses `klarvo-text` (`#ECEEEF`) at 88% opacity at minimum (NOT the old `rgba(220,220,220,0.88)` which is close but not the named token; the spec value is `#ECEEEF`).

4. **Given** the preview is a **separate Tauri window** **When** any appearance value is config-driven **Then** it is re-read **reactively** on the next show cycle (existing `runShowSequence` + `getSettings()` call already handles this — no new event needed; Trap #3 satisfied by existing architecture).

5. **And** the preview panel carries **zero inline hex for covered roles** after this story (DT1 application). The six Studio-Dark values in `PreviewPanel.tsx` (bgColor, textColor, borderColor, fontFamily defaults) and the three files that define defaults (`tauri-commands.ts` mock, `previewAppearance.ts`, `lib.rs` Rust defaults) are all updated to Studio-Dark values.

6. **Given** the `previewAppearance.ts` theme presets **When** the "Dark" (default) preset is selected in Settings **Then** it produces Studio-Dark values — not the old `rgba(20,20,20,0.92)` etc. The preset constants are updated to match the Studio-Dark spec.

7. **Given** the font family **When** the preview renders **Then** the default font stack in all four default locations is `"Geist Mono, ui-monospace, 'Cascadia Code', monospace"` — the spec says raw dictation text uses **Geist Mono** (not Geist). The existing `PREVIEW_FONTS` list gains a "Geist Mono" entry as the default choice.

**DoD (surface-class):**
- `npm run build` (tsc + vite) green.
- `cargo check --target x86_64-pc-windows-gnu` green if any Rust defaults touched (lib.rs changes).
- Real Windows release build via `scripts/sync-and-build.ps1` (Andy's gate).
- Smoke: preview opens during recording, raw text is legible in Geist Mono, expand/collapse is smooth.
- Separate-window reactivity: save a changed appearance in Settings, trigger a new recording — the preview window picks up the saved values (not frozen at app-start values).
- Walk `docs/surface-smoke-checklist.md` traps #1, #2, #3, #5, #6 (see Dev Notes below for applicability).

## Tasks / Subtasks

- [x] **Task 1: Update default appearance values to Studio-Dark in all four locations** (AC: #1, #3, #5)
  - [x] 1.1 `src/PreviewPanel.tsx` — update `cardAppearance` initial state (lines ~65–73):
    - `textColor`: `"rgba(220,220,220,0.88)"` → `"rgba(236,238,239,0.88)"` (`klarvo-text` `#ECEEEF` @ 88%)
    - `bgColor`: `"rgba(25,25,25,0.96)"` → `"rgba(22,24,26,0.92)"` (`klarvo-surface` `#16181A` @ 92%)
    - `bgBlur`: keep `12` (glass effect, 12px blur — no spec change for preview panel; it's a legibility surface, not the pill)
    - `borderColor`: `"rgba(42,195,168,0.25)"` → `"rgba(41,199,172,0.25)"` (klarvo-teal `#29C7AC` @ 25%)
    - `borderWidth`: keep `1`
    - `borderRadius`: keep `14`
    - `fontFamily`: `"'Inter', system-ui, -apple-system, sans-serif"` → `"'Geist Mono', ui-monospace, 'Cascadia Code', monospace"` (AC #7)
  - [x] 1.2 `src/PreviewPanel.tsx` — update `runShowSequence` `appr` local default object (lines ~138–147) to the same Studio-Dark values.
  - [x] 1.3 `src/tauri-commands.ts` — update mock defaults (lines ~84–95):
    - `previewTextColor`, `previewBgColor`, `previewBorderColor`, `previewFontFamily` to Studio-Dark values.
  - [x] 1.4 `src/components/settings/previewAppearance.ts` — update `DEFAULT_TEXT_COLOR`, `DEFAULT_BG_COLOR`, `DEFAULT_BORDER_COLOR`, `DEFAULT_FONT_FAMILY` constants (lines ~13–16).
  - [x] 1.5 `src-tauri/src/lib.rs` — update `preview_text_color`, `preview_bg_color`, `preview_border_color`, `preview_font_family` in ALL three default-initializer locations (lines ~1285, ~1353, ~1415; grep `preview_text_color` to find all).
  - [x] 1.6 `src/PreviewPanel.tsx` — update the **third legacy-value location**: the `appr` object built inside the `try` block of `runShowSequence` (lines ~155–163), where `s.previewTextColor || "rgba(220,220,220,0.88)"` / `s.previewBgColor || "rgba(25,25,25,0.96)"` / `s.previewBorderColor || "rgba(42,195,168,0.25)"` / `s.previewFontFamily || "'Inter',..."` are the fallback values when `getSettings()` returns empty fields. Update these same four fallback literals to the Studio-Dark values (matching Task 1.1 and Task 1.2). **This is distinct from Task 1.1 (useState initial state) and Task 1.2 (outer `appr` default object before the try block).**

- [x] **Task 2: Update `previewAppearance.ts` theme presets to Studio-Dark** (AC: #6)
  - [x] 2.1 Update `PREVIEW_THEMES[0]` (Dark preset) to Studio-Dark values:
    - `textColor`: `"rgba(236,238,239,0.88)"` (klarvo-text @ 88%)
    - `bgColor`: `"rgba(22,24,26,0.92)"` (klarvo-surface @ 92%)
    - `bgBlur`: `12`
    - `borderColor`: `"rgba(41,199,172,0.25)"` (klarvo-teal @ 25%)
    - `borderWidth`: `1`
    - `borderRadius`: `14`
  - [x] 2.2 Keep "Light" and "High-contrast" presets as-is — they are intentionally non-dark-theme looks.
  - [x] 2.3 Add "Geist Mono" as the first entry to `PREVIEW_FONTS` in `previewAppearance.ts`:
    ```ts
    { label: "Geist Mono", stack: "'Geist Mono', ui-monospace, 'Cascadia Code', monospace" },
    ```
    (Keep existing entries. "Geist Mono" becomes the default entry for new users / reset.)

- [x] **Task 3: Update Settings font-family default wiring to Geist Mono** (AC: #7, #5)
  - [x] 3.1 Locate where Settings initializes the `localPreviewFontFamily` state (likely in `ShortcutsContent.tsx` or wherever the preview sub-section lives — search for `localPreviewFontFamily` or `previewFontFamily`).
  - [x] 3.2 Confirm the `PREVIEW_FONTS` dropdown default selection logic picks "Geist Mono" when the stored value matches the new `DEFAULT_FONT_FAMILY`. No hard-coded index assumptions — drive from the stored config value.

- [x] **Task 4: Verify expand/collapse motion uses motion tokens** (AC: #2)
  - [x] 4.1 `PreviewPanel.tsx` currently uses CSS-only grow (flex `justifyContent: flex-end`; the card grows from 0 height as content fills). Verify no hardcoded `transition` durations — the grow is currently CSS-natural (no explicit transition). If a transition is added, use `var(--motion-panel)` and `var(--ease-standard)`.
  - [x] 4.2 The window `show()`/`hide()` calls are Tauri's OS-level show/hide — no CSS transition applies here. This is acceptable (the OS-level show is instant). "Smooth expand/collapse" per UX-DR3 refers to the card growing as text accumulates, not an OS-window animation.

- [x] **Task 5: Verify build integrity + check Trap #1** (DoD)
  - [x] 5.1 `npm run build` (tsc + vite) green.
  - [x] 5.2 `cargo check --target x86_64-pc-windows-gnu` — pre-existing whisper-rs-sys C-dep failures only (same as 8-3); host-target `cargo check` green — confirms lib.rs Rust changes are type-correct.
  - [x] 5.3 Grep `PreviewPanel.tsx` for any remaining legacy values: no hex found; all rgba() values are Studio-Dark spec values.
  - [x] 5.4 Trap #1 check: confirmed — only default string values updated in lib.rs, no key name changes. serde camelCase mapping unchanged.
  - [x] 5.5 Trap #3 check: confirmed — no new mount-time reads added; `runShowSequence` → `getSettings()` remains the only reactive read path.
  - [x] 5.6 Trap #6 check: confirmed — no new config fields added. Trap #6 does not apply.

## Dev Notes

### Critical: Scope of This Story

**8.4 is a default-value + token-migration story — NOT a behavioral/geometry change.**

The `PreviewPanel.tsx` component architecture is NOT changed:
- No new config fields (use the existing 7 appearance fields from Story 6.6).
- No new Tauri commands or events.
- No geometry/sizing changes (`runShowSequence`, `previewGeometry`, window size logic).
- No changes to the stale-chunk guard, bar-moved repositioning, or auto-scroll logic.
- The `set_preview_shape` R11 coupling (Rust window region must match CSS `borderRadius`) is untouched — `borderRadius: 14` stays as-is.

**What changes:** only the default CSS values in four files, and the "Dark" theme preset in `previewAppearance.ts`.

### Critical: Separate Tauri Window Trap (#3)

`PreviewPanel` runs in the `"preview"` Tauri window — separate from the Settings window. It does NOT re-mount when Settings saves. The existing `runShowSequence` architecture already handles this correctly:

```
// Each recording cycle, on first chunk:
const s = await getSettings(); // reads FRESH config from Rust
appr = { bgColor: s.previewBgColor || DEFAULT, ... }
setCardAppearance(appr);       // drives React state → DOM
```

This `getSettings()` call inside `runShowSequence` is the reactive re-read (Trap #3 solution). It runs fresh every recording cycle. **Do not add a mount-time `getSettings()` for appearance** — that would freeze at app-start values.

### Critical: Font for Preview = Geist Mono, Not Geist

Per UX-DR3 and the design spec: raw dictation text uses **Geist Mono** (the monospace weight). This is distinct from the UI font Geist (sans-serif used in FloatingBar, Settings). The spec says "Live-RAW-Transkript in Mono". The preview panel shows raw dictation output — mono is intentional.

The CSS stack: `"'Geist Mono', ui-monospace, 'Cascadia Code', monospace"` — Geist Mono is bundled locally via `styles.css` `@font-face` (from `8-1-token-and-type-foundation`) as:
```css
@font-face { font-family: "Geist Mono"; ... src: url("/fonts/GeistMono-Regular.woff2") ... }
@font-face { font-family: "Geist Mono"; font-weight: 500; ... }
```
No CDN fetch. NFR6 satisfied.

### Studio-Dark Default Values for Preview

All four locations must agree on the same values:

| Field | Old default | New Studio-Dark default | Token name |
|---|---|---|---|
| `textColor` | `rgba(220,220,220,0.88)` | `rgba(236,238,239,0.88)` | klarvo-text `#ECEEEF` @ 88% |
| `bgColor` | `rgba(25,25,25,0.96)` | `rgba(22,24,26,0.92)` | klarvo-surface `#16181A` @ 92% |
| `bgBlur` | `12` | `12` | (unchanged — glass effect) |
| `borderColor` | `rgba(42,195,168,0.25)` | `rgba(41,199,172,0.25)` | klarvo-teal `#29C7AC` @ 25% |
| `borderWidth` | `1` | `1` | (unchanged) |
| `borderRadius` | `14` | `14` | (unchanged — R11 coupling) |
| `fontFamily` | `'Inter', system-ui, -apple-system, sans-serif` | `'Geist Mono', ui-monospace, 'Cascadia Code', monospace` | klarvo-text-mono |

**Note:** `bgColor` changes from `rgba(25,25,25,0.96)` (old warm dark, nearly opaque) to `rgba(22,24,26,0.92)` (klarvo-surface, slightly more transparent). This preserves the transparent-panel aesthetic while using the named token.

### Six Locations to Update (All Must Agree)

1. `src/PreviewPanel.tsx` — `cardAppearance` useState initial value (around line 65) — Task 1.1
2. `src/PreviewPanel.tsx` — `appr` outer default object before the `try` block in `runShowSequence` (around line 138) — Task 1.2
3. `src/PreviewPanel.tsx` — **`appr` object inside the `try` block** (around lines 155–163): the `s.previewTextColor || "rgba(220,220,220,0.88)"` fallback literals — Task 1.6 (easy to miss!)
4. `src/tauri-commands.ts` — mock defaults (used in preview mode / `npm run preview`) — Task 1.3
5. `src/components/settings/previewAppearance.ts` — `DEFAULT_*` constants + `PREVIEW_THEMES[0]` (Dark preset) — Task 1.4
6. `src-tauri/src/lib.rs` — Rust `AppConfig` defaults in all three initializer locations (grep `preview_text_color`) — Task 1.5

Locations 1–3 are all in `PreviewPanel.tsx`:
- Location 1: `cardAppearance` useState initial value (~line 65) — used for the very first render.
- Location 2: outer `appr` default object before the `try` block in `runShowSequence` (~line 138) — used if `getSettings()` throws before assigning.
- Location 3: **`appr` object inside the `try` block** (~lines 155–163) — the fallback arm `s.previewTextColor || "rgba(220,220,220,0.88)"` used when `getSettings()` succeeds but a field is empty. **This is the third location — easy to miss.**

Location 4 is the mock used during `npm run preview` development (`src/tauri-commands.ts`).
Location 5 drives what users see in the Settings appearance panel (`src/components/settings/previewAppearance.ts`).
Location 6 is the on-disk default for first-time users (Rust `Default` impl / migration defaults in `src-tauri/src/lib.rs`).

### Surface Smoke Checklist Trap Applicability (8.4)

| Trap | Applies? | Rationale |
|---|---|---|
| #1 camelCase config keys | CHECK | `lib.rs` Rust defaults use snake_case field names → serde `rename_all="camelCase"` → JSON. No new fields, but confirm values are updated in the right place. |
| #2 New float/Settings field in resync `useEffect` | NOT TRIGGERED | No new config fields. Existing fields already in the resync list. |
| #3 Separate window / reactive reads | CHECK | `PreviewPanel` is a separate window. Existing `runShowSequence` architecture handles this. Do not add new mount-time reads. |
| #4 Window geometry / shape region | NOT TRIGGERED | `borderRadius` stays 14; `set_preview_shape(14)` call unchanged. |
| #5 Push vs poll / event wiring | NOT TRIGGERED | No new events introduced. |
| #6 Multi-hop save chain | NOT TRIGGERED | No new config fields plumbed through the chain. |

### Existing Files — Do Not Accidentally Change

- `src/PreviewPanel.tsx` logic (stale-chunk guard, show-once gate, bar-moved repositioning, runShowSequence geometry, auto-scroll) — these are correct and must not be altered.
- `src-tauri/src/lib.rs` — only update `preview_text_color`, `preview_bg_color`, `preview_border_color`, `preview_font_family` default string values. Do NOT touch other config fields, window creation code, or Tauri commands.
- `src/components/settings/previewAppearance.ts` — update DEFAULT constants and PREVIEW_THEMES[0], add Geist Mono to PREVIEW_FONTS. Keep `rgbaToHexOpacity`/`hexOpacityToRgba` helpers, other themes, and other fonts unchanged.

### Objective Smoke Verification

To verify Geist Mono is rendering (not Inter fallback):
1. Trigger recording in the real Windows build with preview enabled.
2. DevTools → Elements → inspect `#preview-card`.
3. Computed `font-family` must include `Geist Mono` (not just `system-ui` or `Inter`).
4. Visual check: monospace characters should be clearly visible vs proportional rendering.

To verify Studio-Dark background:
- `#preview-card` computed `background-color` ≈ `rgba(22, 24, 26, 0.92)`.
- Text color ≈ `rgba(236, 238, 239, 0.88)`.

### References

- [Source: `_bmad-output/planning-artifacts/epics-visual-overhaul.md` — Story 8.4 ACs] — UX-DR3, DT1, NFR3
- [Source: `docs/design/overhaul/SPEC-studio-dark-overhaul.md`] — "Live-Cleanup-Preview: transparentes Panel, Live-Roh-Transkript in Mono, am Boden verankert"
- [Source: `docs/design/overhaul/02-surfaces.md` — Surface D] — "Live-Cleanup-Preview: ruhiger, gut lesbarer Live-Text"
- [Source: `src/PreviewPanel.tsx`] — Full current implementation: cardAppearance initial state (line ~65), appr default in runShowSequence (line ~138), transparent window constraint, R11 invariant (borderRadius must match set_preview_shape)
- [Source: `src/tauri-commands.ts` — mock defaults] — previewTextColor, previewBgColor etc. (lines ~84–95)
- [Source: `src/components/settings/previewAppearance.ts`] — DEFAULT constants (lines ~13–16), PREVIEW_THEMES (lines ~103–131), PREVIEW_FONTS (lines ~143–148)
- [Source: `src-tauri/src/lib.rs` lines ~1285, ~1353, ~1415] — Rust AppConfig defaults for preview fields (three locations, grep `preview_text_color`)
- [Source: `src/styles.css`] — Geist Mono @font-face (already bundled from 8-1); token vars `--color-klarvo-surface #16181A`, `--color-klarvo-text #ECEEEF`, `--color-klarvo-teal #29C7AC`; motion vars `--motion-panel 320ms`, `--ease-standard`
- [Source: `docs/surface-smoke-checklist.md`] — Trap #1 (camelCase keys), Trap #3 (separate window reactive reads)
- [Source: `_bmad-output/project-context.md` — Critical Rules] — "Never make the user the rendering oracle"; surface-class DoD; camelCase serde rename
- [Source: `_bmad-output/implementation-artifacts/8-3-floatingbar-re-skin.md`] — Colour token reference table; Studio-Dark hex values
- [Source: `_bmad-output/implementation-artifacts/6-6-preview-box-appearance-customization.md`] — R11 invariant (borderRadius CSS = set_preview_shape radius); multi-hop save chain trap; existing 7 appearance fields are correct and must not be re-plumbed

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (2026-06-15)

### Debug Log References

None.

### Completion Notes List

- Updated all 6 default-value locations (3 in PreviewPanel.tsx + tauri-commands.ts + previewAppearance.ts + lib.rs×3) to Studio-Dark tokens: klarvo-text `rgba(236,238,239,0.88)`, klarvo-surface `rgba(22,24,26,0.92)`, klarvo-teal border `rgba(41,199,172,0.25)`, fontFamily `'Geist Mono', ui-monospace, 'Cascadia Code', monospace`.
- Updated PREVIEW_THEMES[0] (Dark preset) to Studio-Dark values (AC #6).
- Added "Geist Mono" as first entry to PREVIEW_FONTS (AC #7 — new users / reset get Geist Mono).
- Updated SettingsPanel.tsx (6 additional fallback sites: 2× useState init, 2× useEffect sync, 2× isDirty check) — all Inter and old rgba fallbacks replaced for consistent Trap #2 behavior.
- Zero new config fields, zero new events, zero behavior changes. Pure default-value + token migration.
- `npm run build` green; host `cargo check` green; pre-existing win-gnu whisper-rs-sys C-dep failure unchanged (same as 8-3).
- No CSS transitions added to PreviewPanel.tsx; CSS-natural grow confirmed (AC #2 satisfied by existing architecture).
- Trap #3 verified: no mount-time appearance reads added; runShowSequence → getSettings() remains sole reactive path.

### File List

- src/PreviewPanel.tsx
- src/tauri-commands.ts
- src/components/settings/previewAppearance.ts
- src/components/SettingsPanel.tsx
- src-tauri/src/lib.rs
- src-tauri/src/config/mod.rs

## Change Log

- 2026-06-15: Story 8.4 implemented — Studio-Dark re-skin of PreviewPanel. Updated all default appearance values in 5 files (6 default-value sites + 6 SettingsPanel fallback sites). Added Geist Mono to PREVIEW_FONTS as first/default entry. Zero behavioral changes — pure token migration. npm run build green. Status → review.
- 2026-06-15: Review follow-up — applied confirmed findings: (1) Updated four `default_preview_*` functions in src-tauri/src/config/mod.rs (lines 986-1006) to Studio-Dark tokens — the serde default site missed in original dev pass; (2) Updated field doc comments at lines 739/744/752/766 and section comment at 985 to match; (3) Updated canary test assertions at lines 4151-4169 to Studio-Dark tokens, re-locking the migrated contract. settings.rs:2078-2084 (merge-behavior test using explicit existing values) left as-is per reviewer directive. cargo test: 628 passed, 0 failed. npm run build: green.
- 2026-06-15: Conductor adjudication of 3 escalated decisions + manual convergence (decision-gate preempted in-loop fix dispatch). **(b) Legibility regression CORRECTED:** the dev had dropped the Dark preset opacities (text 0.95→0.88, bg 0.96→0.92) — opposite of AC#3's purpose (88% is the legibility FLOOR, not a target; the SPEC mandates no preview opacity, so the prior 0.95/0.96 were not spec-overridden). Restored the prior legible opacities while KEEPING the new Studio-Dark hex (text `rgba(236,238,239,0.95)`, bg `rgba(22,24,26,0.96)`), applied **consistently across all sites**: previewAppearance.ts preset + DEFAULT_* constants, PreviewPanel.tsx fallbacks, SettingsPanel.tsx, tauri-commands.ts, types.ts JSDoc, AND the Rust serde defaults (config/mod.rs `default_preview_text/bg_color` + doc comments) + the default-assertion test (4151-4158) + 3 lib.rs fixtures — preset and fresh-install default now agree (no preset/default mismatch). **(a) Default-only migration ACCEPTED (not auto-migrated):** existing config.json users keep their persisted preview values; a forced migration would clobber Epic-6 user customizations (the canary test forbids migration writes by design). Adding a *conditional* "bump-only-if-still-at-old-default" value migration touches the sensitive ADR-0015 config path → NOT done autonomously overnight; flagged as an open product decision for Andy with that recommendation. (For Andy's morning verification: resetting preview appearance to Default surfaces Studio-Dark, so the gate is reachable.) **(c) DoD render gate = conductor GATE-4:** mechanical smoke GREEN via WSL Chromium harness (`/tmp/klarvo-bar-harness/8-4-smoke.mjs` + `8-4-smoke-http.mjs`): card text `rgb(236,238,239)`/`@.95`, surface `rgba(22,24,26,0.96)`, font-family = Geist Mono stack, **Geist Mono actually resolves in-engine** when http-served (woff2 bundled + @font-face in built CSS; the file:// variant fails only on Chromium's local-origin font block — a harness artifact, not an app defect). Patch: refreshed stale `Inter`/old-rgba JSDoc in types.ts. npm run build green; cargo check + 11 preview tests green. **Human-visual gate downgraded** (Verifikations-Symmetrie path 2): real WebView2 (vs Chromium) font resolution, separate-window save→recording reactivity, and legibility over arbitrary desktop content batched for Andy's morning. **Accepted residuals:** the conditional value-migration decision (a); the per-user inline-rgba duplication across 6+ files has no lint guard (DT1 SSOT spirit — call-out comments only); legacy `Inter`/old-teal in not-yet-migrated App.tsx/Onboarding.tsx/ThemeSwitcher.tsx (8-5/8-6 territory).
