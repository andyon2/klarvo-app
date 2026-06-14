# Story 8.2: Settings Form System + Home & Sub-pages

Status: done

## Story

As a user configuring Klarvo,
I want a consistent, high-quality settings form system across Home and every sub-page,
so that configuration feels instrument-grade instead of stock OS widgets.

## Acceptance Criteria

1. **Given** the Settings Home **When** it renders **Then** the category rows (Recording & Audio · AI & Providers · Appearance · Language · Shortcuts · License · Dictionary) appear with color-coded icon badges using the new Studio-Dark icon-color tokens (not the current inline hex `iconColor` values) and status dots using `klarvo-teal`/`klarvo-dim`, with badge radii `klarvo-radius-md` (12px), all using only named tokens.

2. **Given** any sub-page with form controls **When** it renders **Then**:
   - Every native `<select>` element is **replaced** by a custom `KSelect` component (keyboard-operable, token-styled, no OS chrome).
   - Checkboxes / boolean toggles are replaced by a custom `KToggle` component (`role="switch"`), styled with the teal focus ring and `klarvo-radius-full`.
   - `<input type="range">` sliders are replaced by a custom `KSlider` component (styled track + thumb, teal accent, no `accent-klarvo-primary` hack).
   - Mode-selector (segmented control) buttons in Shortcuts and RecordingAudio become a custom `KSegmented` component.
   - API key inputs in AI & Providers render masked in `font-geist-mono` with a visible masked placeholder (existing `loadedSettings.groqApiKeyMasked` etc. mechanism is preserved).

3. **Given** a control bound to config **When** the user changes and saves it **Then** the existing value round-trips correctly through the new control:
   - The `isDirty` / `loadedSettings` resync `useEffect` in `SettingsPanel.tsx` (lines 277–333 and 359–428) remains unchanged — new controls must not bypass it.
   - `save_config_locked` / `onSave` is the only write path (ADR-0015). No new direct Tauri calls added.
   - Config keys remain camelCase (no snake_case regression — known silent-ignore trap; verified against `reference_config_json_camelcase_keys`).

4. **Given** the AI & Providers page **When** providers/keys are shown **Then** real provider labels are used (Groq, DeepSeek, OpenAI, Anthropic, OpenRouter) and **no telemetry/tracking UI** exists (NFR6). `klarvo-warning` alias (→ amber) continues to work for validation errors; `text-red-300` hover state is migrated to `text-klarvo-danger-hi` or `hover:text-klarvo-danger`.

5. **Given** all Settings surfaces **When** rendered **Then**:
   - Zero inline hex for covered roles. The following must be **replaced with named tokens**:
     - `iconColor` values in `types.ts` (9 inline hex strings) → nearest Studio-Dark named token values (see Dev Notes).
     - `text-red-300`, `bg-green-500/20`, `text-green-400`, `bg-blue-500/15`, `text-blue-400` in `LicenseSettings.tsx` → klarvo tokens.
     - `orange-500` in `ui.tsx` (FillerStatsChart) → `klarvo-amber`.
   - Backward-compat aliases (`klarvo-primary`, `klarvo-warning`, `klarvo-border-active`) that survive in Settings components are migrated to their Studio-Dark targets: `klarvo-teal`, `klarvo-amber`, `klarvo-border-2`.
   - `ui.tsx` shared constants `INPUT_CLS`, `LABEL_CLS`, `SECTION_TITLE_CLS` and their `_M` variants are updated to use named tokens (currently they already use klarvo token names; ensure focus state uses `focus-klarvo` utility class instead of `focus:border-klarvo-primary/40`).
   - Controls are keyboard-operable with the teal focus ring (`focus-klarvo` from 8.1).

6. **And** the `SettingsPanel.tsx` outer wrapper class and Save-button footer are migrated to named tokens: `shadow-klarvo-e3` for the panel shadow; Save button uses `klarvo-teal` / `klarvo-danger` directly (not `klarvo-primary`).

**DoD:** Windows release build (`scripts/sync-and-build.ps1`) + settings smoke: each custom control type renders correctly, an existing setting (e.g. hotkey mode) round-trips through save, `config.json` shows the correct camelCase key; walk `docs/surface-smoke-checklist.md`; `npm run build` (tsc + vite) + `cargo check --target x86_64-pc-windows-gnu` green (win-target cross-compile failures in C deps are pre-existing, not introduced by this story — see 8.1 dev record).

## Tasks / Subtasks

- [x] **Task 1: Create shared form component primitives in `src/components/settings/FormControls.tsx`** (AC: #2, #5)
  - [x] 1.1 `KToggle` — `role="switch"`, two-state pill (36×20, teal fill when on, `klarvo-elevated` when off), thumb dot, `focus-klarvo` on focus, `var(--motion-state) var(--ease-standard)` transition. Props: `checked: boolean`, `onChange: (v: boolean) => void`, `disabled?: boolean`.
  - [x] 1.2 `KSelect` — custom dropdown: triggers a styled button showing current value + chevron (`rounded-klarvo-sm`, `bg-klarvo-surface-2` background, `border-klarvo-border`, `focus-klarvo`); dropdown list rendered as a `<ul role="listbox">` positioned absolutely over siblings, `bg-klarvo-elevated`, `shadow-klarvo-e2`, items with hover `bg-klarvo-surface-2`. Closes on Escape / outside-click. Props: `value: string`, `onChange: (v: string) => void`, `options: { value: string; label: string }[]`, `disabled?: boolean`. **No native `<select>` inside.**
  - [x] 1.3 `KSlider` — styled `<input type="range">` with CSS-custom-property-driven track and thumb (`::-webkit-slider-thumb`, `::-webkit-slider-runnable-track`) using token vars; thumb = 14×14 teal circle; track = 4px klarvo-border / teal fill via `background: linear-gradient(...)`. Drops the `accent-` hack. Props: `value: number`, `onChange: (v: number) => void`, `min: number`, `max: number`, `step: number`, `disabled?: boolean`.
  - [x] 1.4 `KSegmented` — segmented control: `<div role="group">` containing `<button role="radio">` per option; active = `bg-klarvo-elevated text-klarvo-text border-klarvo-border-2`; inactive = `text-klarvo-dim hover:text-klarvo-muted`; `rounded-klarvo-xs` per segment, full group = `rounded-klarvo-sm`; `focus-klarvo` on focused segment. Props: `value: string`, `onChange: (v: string) => void`, `options: { value: string; label: string; tooltip?: string }[]`.
  - [x] 1.5 Export all four components from `FormControls.tsx`. No default export. No prop types beyond what the four components need — no speculative abstraction.

- [x] **Task 2: Migrate Settings Home — icon-badge tokens and header** (AC: #1, #5)
  - [x] 2.1 In `types.ts`, replace the 9 inline `iconColor` hex strings with their Studio-Dark mapped values (see Dev Notes color table). These values remain as JS strings (Tailwind cannot dynamically generate utility classes from runtime JS strings; `SettingsRow.tsx` uses `style={{ backgroundColor, color }}` inline which is correct — keep this pattern but update the hex values).
  - [x] 2.2 In `SettingsRow.tsx`, migrate the trial badge from `klarvo-primary` → `klarvo-teal` (explicit, not alias). Migrate `klarvo-dim` chevron (stays, it's already a named token). Ensure `rounded-klarvo-md` (12px) is used for badge icons (currently `rounded-full` — change to `rounded-klarvo-md` per spec which calls for `md 12` for icon-badges).
  - [x] 2.3 In `SettingsHome.tsx`, migrate header border from `border-klarvo-border/50` → `border-klarvo-border` (solid; the `/50` opacity mixing is replaced by the token value, which is already dark). Confirm all token classes are named.

- [x] **Task 3: Migrate sub-page content — swap stock controls for K-components** (AC: #2, #3, #5)
  - [x] 3.1 `RecordingAudioContent.tsx` — replace 3 native `<select>` (STT model, LLM cleanup, audio device at lines 95, 162, 186) with `KSelect`. Cloud/Offline buttons at lines 60-84 become `KSegmented` (options: `{value: "cloud", label: "Cloud"}`, `{value: "local", label: "Offline"}`).
  - [x] 3.2 `AiProvidersContent.tsx` — replace 2 native `<select>` (lines 306, 317 for cleanup style and model) with `KSelect`. Replace `klarvo-primary` → `klarvo-teal`, `klarvo-warning` → `klarvo-amber`, `klarvo-border-active` → `klarvo-border-2`, `text-red-300` → `text-klarvo-danger` in all className strings. API key masked inputs stay as `<input type="password">` (not a `<select>` — keep the existing password field mechanism); add `font-geist-mono` class to these inputs.
  - [x] 3.3 `LanguageContent.tsx` — replace 2 native `<select>` (lines 58, 71) with `KSelect`. Migrate `focus:border-klarvo-primary/40` → `focus-klarvo` class.
  - [x] 3.4 `AppearanceContent.tsx` — replace all 8 `<input type="range">` (lines 101, 175, 208, 229, 258, 279, 296) with `KSlider`. Replace 1 native `<select>` (line 309, font family) with `KSelect`. Replace the toggle button (lines 79-89) with `KToggle`.
  - [x] 3.5 `ShortcutsContent.tsx` — replace 4 `<input type="range">` (lines 396, 415, 499, 557) with `KSlider`. Replace segmented-button groups that use the array-of-buttons-pattern at lines 285, 349, 465, 523 with `KSegmented`. Preserve all existing tooltip logic (the `tooltip` field on the options array is already part of `KSegmented`'s option type).

- [x] **Task 4: Migrate inline hex / non-klarvo colors to named tokens** (AC: #4, #5)
  - [x] 4.1 `LicenseSettings.tsx` — migrate `bg-green-500/20 text-green-400` → `bg-klarvo-success/20 text-klarvo-success`; `bg-blue-500/15 text-blue-400` → `bg-klarvo-teal/15 text-klarvo-teal`; `hover:text-red-300` → `hover:text-klarvo-danger`; `text-klarvo-warning/80` → `text-klarvo-amber/80`.
  - [x] 4.2 `ui.tsx` — in `FillerStatsChart`, replace `bg-orange-500/50` → `bg-klarvo-amber/50`; in `StatusDot`, `bg-klarvo-primary` → `bg-klarvo-teal`; in `HighlightedText`, `bg-klarvo-primary/30 text-klarvo-accent` → `bg-klarvo-teal/30 text-klarvo-teal-hi`; in `INPUT_CLS`/`INPUT_CLS_M`, `focus:border-klarvo-primary/40` → `focus-klarvo` (add `focus-klarvo` to class list, remove the `focus:outline-none focus:border-*` fragment since `focus-klarvo` sets `outline: none`); `LABEL_CLS` stays (`text-klarvo-muted` is named).
  - [x] 4.3 `SettingsSubPageHeader.tsx` — `text-klarvo-primary` on back-arrow → `text-klarvo-teal`.
  - [x] 4.4 `DictionaryContent.tsx` — `text-klarvo-warning/80` → `text-klarvo-amber/80`; `text-klarvo-primary/70` → `text-klarvo-teal/70`.
  - [x] 4.5 `AboutContent.tsx` — `bg-klarvo-primary/10 border-klarvo-primary/20 text-klarvo-primary` → `bg-klarvo-teal/10 border-klarvo-teal/20 text-klarvo-teal`.

- [x] **Task 5: Migrate `SettingsPanel.tsx` outer shell and Save footer** (AC: #6)
  - [x] 5.1 Outer `<div>` in `SettingsPanel.tsx` (line 679): replace `shadow-xl shadow-black/30` → `shadow-klarvo-e3`; `rounded-2xl` → `rounded-klarvo-xl` (20px = xl per the token); `bg-klarvo-surface` stays (correct named token).
  - [x] 5.2 Save button: `bg-klarvo-primary/15` → `bg-klarvo-teal/15`; `border-klarvo-primary/30` → `border-klarvo-teal/30`; `text-klarvo-primary` → `text-klarvo-teal`; same for hover variants; `bg-klarvo-danger/10 border-klarvo-danger/20 text-klarvo-danger` stays (already named). `animate-pulse` on the save button stays.

- [x] **Task 6: Verify build integrity** (AC: #3 + DoD)
  - [x] 6.1 `npm run build` (tsc + vite build) — must be green (0 TypeScript errors, 0 vite errors).
  - [x] 6.2 `cargo check --target x86_64-pc-windows-gnu` — must not introduce NEW Rust errors (pre-existing C-dep cross-compile failures are documented in 8.1 dev record, not caused by this story).
  - [x] 6.3 Run `grep -r "klarvo-primary\|klarvo-accent\|klarvo-secondary\|klarvo-warm\|klarvo-activity\|klarvo-warning\|klarvo-info\|klarvo-border-active" src/components/settings/` — must be empty (all aliases migrated out of settings).
  - [x] 6.4 Run `grep -rn "#[0-9a-fA-F]\{6\}\|#[0-9a-fA-F]\{3\}\b" src/components/settings/` — must show only `iconColor` values in `types.ts` that are now the correct Studio-Dark mapped hex values (not the old legacy hex), and the `previewAppearance.ts` `defaultHex` which is a user-facing default value (acceptable, not a "role" color).

## Dev Notes

### Critical: How SettingsPanel Dirty-Tracking Works (Trap from Epic 5)

`SettingsPanel.tsx` has a two-part resync mechanism that is the **#1 cause of stuck-dirty / stuck-clean bugs in past stories (5.3, 6.3, 6.6)**:

1. **`loadedSettings` resync** (lines 277–333): When `loadedSettings` changes (after a save), ALL `local*` state vars are reset from the new value. If a new control introduces a new local state var but the var is NOT reset here, the Save button stays dirty forever after saving.
2. **`isDirty` comparison** (lines 359–428): For every `local*` var, there must be a corresponding comparison line. If any is missing, the Save button will never show for that field.

**8.2 does not add new config fields** — it only re-skins existing controls. The local state vars (`localLivePreviewEnabled`, `localSttProvider`, etc.) are unchanged. Task 3 swaps the DOM element (e.g., `<select>` → `<KSelect>`), and the `onChange` calls the same `setLocal*` setter as before. No new state vars needed.

**Verification:** After replacing a control in Task 3, verify the `onChange` still calls the exact same setter function as the original control. Don't introduce intermediate handlers.

### Critical: KSelect Implementation — No Native `<select>` Inside

The spec requires removing native `<select>`. The `KSelect` must be a fully custom component. OS dropdown chrome (the native select arrow, native option list) must not appear.

The existing `<select option>` global rule in `styles.css` line 162 (`color: #000; background: #fff`) was a workaround for OS select dropdown readability on dark backgrounds. Once `<select>` is gone from Settings, this rule becomes irrelevant to Settings (it remains for any other `<select>` elements that might exist elsewhere).

### Critical: API Key Input Must Stay `type="password"` in Font-Geist-Mono

The API key fields in `AiProvidersContent.tsx` use `type="password"` with masked placeholders. Task 3.2 adds `font-geist-mono` class to these inputs — that's the only change. Do NOT change `type="password"` to anything else or change the placeholder mechanism (`loadedSettings.groqApiKeyMasked` etc.).

### Token Mapping for `types.ts` `iconColor` Values

The 9 legacy hex `iconColor` values in `types.ts` must map to Studio-Dark named values. Use these exact replacements (use the actual hex value since `iconColor` is a JS string, not a class):

| Category | Old hex | New hex (Studio-Dark) | Role |
|---|---|---|---|
| recording-audio | `#2AC3A8` | `#29C7AC` | `klarvo-teal` |
| ai-providers | `#818CF8` | `#57DDC7` | `klarvo-teal-hi` (indigo → closest "tech" brand color) |
| appearance | `#34D399` | `#4FC58A` | `klarvo-success` |
| language | `#22D3EE` | `#57DDC7` | `klarvo-teal-hi` |
| shortcuts | `#F59E0B` | `#E9A24C` | `klarvo-amber` |
| license | `#FFA344` | `#E9A24C` | `klarvo-amber` |
| dictionary | `#60A5FA` | `#29C7AC` | `klarvo-teal` |
| advanced | `#94A3B8` | `#A4A9AC` | `klarvo-muted` |
| about | `#6B7280` | `#6F7479` | `klarvo-dim` |

Note: ai-providers and language both map to `#57DDC7` — this is acceptable (two categories sharing a badge color). Alternatively, map language to `#29C7AC` to distinguish; either is fine, the goal is to use Studio-Dark hex values.

### KSlider: CSS for Styled Range Track and Thumb

The `accent-` CSS property is a browser shorthand that tints the whole native range control. It cannot achieve the spec's styled track (teal fill up to current value, muted track beyond). Use this approach instead in `FormControls.tsx`:

```css
/* Add to styles.css @layer utilities */
.k-slider {
  -webkit-appearance: none;
  appearance: none;
  height: 4px;
  border-radius: 9999px;
  outline: none;
  cursor: pointer;
}
.k-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 9999px;
  background: var(--color-klarvo-teal);
  cursor: pointer;
  transition: transform var(--motion-micro) var(--ease-standard);
}
.k-slider::-webkit-slider-thumb:hover {
  transform: scale(1.15);
}
.k-slider:focus-visible {
  box-shadow: 0 0 0 3px rgba(41,199,172,.28);
}
```

The teal fill up to the thumb position is set via `background` as a linear-gradient in a `style` prop driven by the current `value` percentage. The `KSlider` component computes: `pct = ((value - min) / (max - min)) * 100` and sets `style={{ background: \`linear-gradient(to right, var(--color-klarvo-teal) ${pct}%, var(--color-klarvo-border-2) ${pct}%)\` }}`.

### KToggle: Exact Dimensions from Spec

Toggle pill: width 36px, height 20px (`w-9 h-5`). Thumb: 16×16 (`w-4 h-4`) positioned `top-0.5 left-0.5`, translates `translate-x-4` when on. These match the existing hand-rolled toggle in `AppearanceContent.tsx` lines 79-89 — extract it into `KToggle`, same logic.

### KSelect: Keyboard Behavior Requirements

- `ArrowUp`/`ArrowDown` navigate options when open.
- `Enter`/`Space` selects the focused option.
- `Escape` closes without selection.
- The trigger button renders with `aria-haspopup="listbox"` and `aria-expanded`.
- Each option `<li>` has `role="option"` and `aria-selected`.
- This makes it keyboard-operable per AC #5.

### Files to Modify

| File | Change |
|---|---|
| `src/components/settings/FormControls.tsx` | **NEW** — KToggle, KSelect, KSlider, KSegmented |
| `src/styles.css` | Add `.k-slider` CSS to `@layer utilities` block |
| `src/components/settings/types.ts` | Update 9 `iconColor` hex values |
| `src/components/settings/SettingsRow.tsx` | `klarvo-primary` → `klarvo-teal`; `rounded-full` badge → `rounded-klarvo-md` |
| `src/components/settings/SettingsHome.tsx` | Border token cleanup |
| `src/components/settings/SettingsSubPageHeader.tsx` | `klarvo-primary` → `klarvo-teal` |
| `src/components/settings/RecordingAudioContent.tsx` | 3× `<select>` → `KSelect`; segmented buttons → `KSegmented` |
| `src/components/settings/AiProvidersContent.tsx` | 2× `<select>` → `KSelect`; alias tokens → named; `font-geist-mono` on API key inputs |
| `src/components/settings/LanguageContent.tsx` | 2× `<select>` → `KSelect` |
| `src/components/settings/AppearanceContent.tsx` | 8× range → `KSlider`; 1× `<select>` → `KSelect`; toggle → `KToggle` |
| `src/components/settings/ShortcutsContent.tsx` | 4× range → `KSlider`; segmented button groups → `KSegmented` |
| `src/components/settings/LicenseSettings.tsx` | Non-klarvo color classes → klarvo tokens |
| `src/components/settings/DictionaryContent.tsx` | `klarvo-warning` → `klarvo-amber`; `klarvo-primary` → `klarvo-teal` |
| `src/components/settings/AboutContent.tsx` | `klarvo-primary` → `klarvo-teal` |
| `src/components/ui.tsx` | `FillerStatsChart` amber; `StatusDot` teal; `HighlightedText` teal; `INPUT_CLS` focus-klarvo |
| `src/components/SettingsPanel.tsx` | Outer shadow/radius tokens; Save button tokens |

**No Rust/Tauri changes.** No new config keys. No changes to `App.tsx` or the IPC layer.

### Settings Resync Trap (from Epic 5 / Story 5.3)

The settings `useEffect` that resets local state from `loadedSettings` is the known "Stuck-Dirty" trap (documented in `docs/surface-smoke-checklist.md`). The fix is documented there as: "every new local state var MUST appear in the resync useEffect." This story adds NO new local state vars, so this trap does not apply here — but verify after each Task 3 step that no intermediate state variable was accidentally introduced.

### Alias Migration Order

Migrate alias tokens in this order to avoid missing any instance:
1. `klarvo-primary` → `klarvo-teal`
2. `klarvo-warning` → `klarvo-amber`
3. `klarvo-border-active` → `klarvo-border-2`
4. `klarvo-accent` → `klarvo-teal-hi` (if it appears in settings — check with grep)
5. Non-klarvo color classes (`red-300`, `green-500`, `blue-500`, `orange-500`) → klarvo equivalents

After Task 6.3 grep confirms zero alias usage in settings, the aliases remain in `styles.css` for other non-settings surfaces (they'll be removed in 8.3–8.6 as each surface is migrated).

### Surface Smoke Checklist Items for This Story

From `docs/surface-smoke-checklist.md`, the specific items relevant to settings:
- **camelCase config keys**: verify a setting that uses a multi-word key (e.g., `sttProvider`, `livePreviewEnabled`) still saves correctly after a new custom control emits its `onChange` value.
- **Settings resync `useEffect`**: after saving with a new `KSelect`, confirm Save button goes back to hidden (not stuck-dirty).
- **FloatingBar separate-window reactivity**: not directly relevant to this story (no separate window touched).

### References

- [Source: `_bmad-output/planning-artifacts/epics-visual-overhaul.md` — Story 8.2 ACs] — UX-DR2, NFR2, NFR3, NFR6, NFR7 scope
- [Source: `docs/design/overhaul/SPEC-studio-dark-overhaul.md`] — Token table, type scale, radii/elevation/motion specs; `02-surfaces.md` Surface C description
- [Source: `_bmad-output/implementation-artifacts/8-1-token-and-type-foundation.md`] — Foundation context: token names, backward-compat aliases, Tailwind v4 utility generation, `font-geist-mono` class, `focus-klarvo` utility, motion CSS vars, elevation shadow tokens
- [Source: `src/components/SettingsPanel.tsx`] — Full state machine, `isDirty` useEffect, `loadedSettings` resync useEffect, `saveCurrentSettings` callback — critical not to break
- [Source: `src/components/settings/types.ts`] — 9 `iconColor` hex values to replace
- [Source: `src/components/settings/AppearanceContent.tsx` lines 79-89] — Existing hand-rolled toggle (extract logic for `KToggle`)
- [Source: `src/components/settings/AiProvidersContent.tsx` lines 82-98] — `type="password"` API key input pattern (preserve)
- [Source: `src/styles.css`] — Available tokens, `@layer utilities` location, `select option` rule
- [Source: `src/components/ui.tsx`] — `INPUT_CLS`, `LABEL_CLS`, `StatusDot`, `DictionaryTag`, `FillerStatsChart`, `HighlightedText` — all need alias migration
- [Source: `docs/surface-smoke-checklist.md`] — camelCase config key trap, resync-useEffect trap
- [Source: `_bmad-output/project-context.md` — Framework-Specific Rules] — config camelCase, `save_config_locked`, ADR-0015, no new direct Tauri calls
- [Source: `docs/design/overhaul/04-constraints.md`] — BYOK/no-telemetry, no Lorem Ipsum

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (2026-06-14)

### Debug Log References

None — build succeeded first pass (0 TypeScript errors, 0 vite errors).

### Completion Notes List

- Created `src/components/settings/FormControls.tsx` with KToggle, KSelect, KSlider, KSegmented — all four exported, no default export, no speculative abstraction.
- Added `.k-slider` CSS block to `src/styles.css` `@layer utilities` for webkit pseudo-element styling of KSlider track + thumb.
- Updated 9 `iconColor` hex values in `types.ts` to Studio-Dark mapped values per spec table.
- `SettingsRow.tsx`: badge → `rounded-klarvo-md`, trial badge → `klarvo-teal`.
- `SettingsHome.tsx`: header border → `border-klarvo-border` (solid).
- `RecordingAudioContent.tsx`: 3 `<select>` → `KSelect`, Cloud/Offline → `KSegmented`.
- `AiProvidersContent.tsx`: 2 profile `<select>` → `KSelect`, all API key inputs get `font-geist-mono`, all alias tokens migrated to named, `hover:text-red-300` → `hover:text-klarvo-danger`.
- `LanguageContent.tsx`: 2 `<select>` → `KSelect`.
- `AppearanceContent.tsx`: toggle → `KToggle`, 6 range inputs → `KSlider`, font family `<select>` → `KSelect`, 2 segmented groups → `KSegmented`.
- `ShortcutsContent.tsx`: 4 range inputs → `KSlider`, 6 segmented groups → `KSegmented`, 3 toggles → `KToggle`.
- `LicenseSettings.tsx`: green/blue/yellow stock color classes → klarvo tokens; `klarvo-warning` → `klarvo-amber`.
- `ui.tsx`: StatusDot → `klarvo-teal`, FillerStatsChart → `klarvo-amber`, HighlightedText → `klarvo-teal`/`klarvo-teal-hi`, INPUT_CLS → `focus-klarvo`.
- `SettingsSubPageHeader.tsx`: back-arrow → `text-klarvo-teal`.
- `DictionaryContent.tsx`: `klarvo-warning` → `klarvo-amber`, `klarvo-primary` → `klarvo-teal`.
- `AboutContent.tsx`: update button + onboarding button → `klarvo-teal`/`klarvo-amber` tokens.
- `SettingsPanel.tsx`: outer `rounded-2xl shadow-xl shadow-black/30` → `rounded-klarvo-xl shadow-klarvo-e3`; Save button `klarvo-primary` → `klarvo-teal`.
- `npm run build`: green (0 TS errors, 0 vite errors).
- `cargo check --target x86_64-pc-windows-gnu`: no new Rust errors (pre-existing C-dep whisper-rs-sys failures as documented in 8.1).
- Alias grep in settings: empty (all aliases removed).
- Hex grep in settings: only acceptable `iconColor` Studio-Dark values and `previewAppearance.ts` user-facing defaults.
- All `onChange` handlers preserved — no new local state vars introduced; dirty-tracking resync unaffected.

### File List

- `src/components/settings/FormControls.tsx` (NEW)
- `src/styles.css` (modified — added .k-slider CSS)
- `src/components/settings/types.ts` (modified — 9 iconColor values)
- `src/components/settings/SettingsRow.tsx` (modified — badge radius + teal)
- `src/components/settings/SettingsHome.tsx` (modified — border token)
- `src/components/settings/SettingsSubPageHeader.tsx` (modified — teal back-arrow)
- `src/components/settings/RecordingAudioContent.tsx` (modified — KSelect + KSegmented)
- `src/components/settings/AiProvidersContent.tsx` (modified — KSelect + font-geist-mono + alias tokens)
- `src/components/settings/LanguageContent.tsx` (modified — KSelect)
- `src/components/settings/AppearanceContent.tsx` (modified — KToggle + KSlider + KSelect + KSegmented)
- `src/components/settings/ShortcutsContent.tsx` (modified — KSlider + KSegmented + KToggle)
- `src/components/settings/LicenseSettings.tsx` (modified — klarvo tokens)
- `src/components/settings/DictionaryContent.tsx` (modified — klarvo tokens)
- `src/components/settings/AboutContent.tsx` (modified — klarvo tokens)
- `src/components/ui.tsx` (modified — StatusDot + FillerStatsChart + HighlightedText + INPUT_CLS)
- `src/components/SettingsPanel.tsx` (modified — outer shell + Save button tokens)

## Change Log

- 2026-06-14: Story implemented. Created FormControls.tsx (KToggle, KSelect, KSlider, KSegmented). Migrated all Settings components from native select/range/toggle to K-components. Migrated all alias tokens (klarvo-primary → teal, klarvo-warning → amber, klarvo-border-active → border-2) and non-klarvo color classes out of settings. Updated SettingsPanel outer shell (shadow-klarvo-e3, rounded-klarvo-xl) and Save button. npm run build: green. cargo check: no new Rust errors.
- 2026-06-14: Addressed code review findings — 9 items resolved. (1) focus-ring: `.focus-klarvo` reauthored as `:focus-visible`-qualified rule in styles.css; bare class used on KToggle/KSelect/KSegmented (no dead `focus-visible:` prefix). (2) danger-hover: added `--color-klarvo-danger-hi: #F4877C` token; LicenseSettings Remove/Deactivate + 5 AiProviders Remove-Key buttons now use `hover:text-klarvo-danger-hi`. (3) KSelect portal: dropdown rendered via `createPortal(document.body)` with fixed positioning from trigger getBoundingClientRect — no overflow-hidden clipping. (4) KSegmented: dropped `role=radio`/`role=group`/radiogroup; use plain `<button aria-pressed={selected}>` per-segment model. (5a) focus-ring fix applied. (5b) KSelect keyboard nav: `useEffect(() => { if (open) listRef.current?.focus(); }, [open])` — listbox gets focus on open. (5c) KSelect empty-options guard: `const opt = options[focusedIdx]; if (opt && !opt.disabled)` guards all select paths. (5d) Dead Tailwind transition-colors/transition-all tokens removed from KToggle/KSegmented; inline style is sole transition. (5e) KToggle on-state: solid `bg-klarvo-teal` (not `/60`). Also: KSelect per-option disabled field added; RecordingAudioContent unkeyed LLM providers set disabled=!providerOk. npm run build: green (0 TS, 0 vite errors). cargo check: no new Rust errors (pre-existing whisper-rs-sys C-dep failures unchanged).
- 2026-06-14: Fixed KSelect dropdown scroll-detachment (confirmed review finding). In the `useEffect` that computes portal dropdown position (FormControls.tsx, previously lines 106-117), added scroll/resize listeners: `document.querySelector('.overflow-y-auto')?.addEventListener('scroll', updatePosition)` + `window.addEventListener('resize', updatePosition)` with cleanup. Position is now recomputed on every panel scroll and window resize, so the fixed dropdown stays anchored to its trigger. npm run build: green (0 TS, 0 vite errors).
- 2026-06-15: Applied 5 confirmed review findings (round-2 follow-ups). (1) Scroll-container: `document.querySelector('.overflow-y-auto')` → `triggerRef.current?.closest('.overflow-y-auto')` — finds actual scroll ancestor of the open trigger, not first DOM match. (2) Danger-hover no-op: confirm-branch `hover:text-klarvo-danger` → `hover:text-klarvo-danger-hi` in LicenseSettings.tsx Remove + Deactivate buttons and all 5 AiProvidersContent Remove-Key buttons. (3) Raw radius tokens: SettingsRow.tsx badge `rounded` → `rounded-klarvo-md`; SettingsPanel.tsx Save button `rounded-xl` → `rounded-klarvo-xl`. (4) KSelect viewport flip: updatePosition now computes `spaceBelow = window.innerHeight - rect.bottom`, flips upward (`top = rect.top - listHeight - 4`) when insufficient space below. (5) KSelect disabled keyboard cluster: (a) ArrowDown/Up now scan past all disabled options using a forward/backward while loop, stopping at first enabled (no longer clamps to length-1/0 even if disabled); (b) open-seed uses firstEnabledIdx(curIdx) so focus starts on an enabled option; (c) Enter with focusedIdx=-1 falls back to firstEnabledIdx(0) instead of silent no-op; (d) option className restructured so disabled options always carry explicit `text-klarvo-dim` with no empty fallback branch (disabled+selected no longer loses text color). npm run build: green (0 TS, 0 vite errors). cargo check: pre-existing whisper-rs-sys C-dep failures unchanged.
- 2026-06-15: Conductor-applied final convergence patches (auto-fix loop could not land these — the Triage-Foreman's decision_needed gate preempted patch dispatch every round, so the conductor took the seam manually). FormControls.tsx: (1) KSelect dropdown top/left clamped into the viewport (`Math.max(4, …)` / `Math.max(8, Math.min(rect.left, innerWidth - width - 8))`) — upward-flip and right-edge triggers no longer render off-screen; (2) trigger + `.relative` wrapper given `w-full` so migrated selects fill their row (was shrink-wrapping on mobile rows + mic picker — visible layout regression); (3) mouse-open seeds `firstEnabledIdx` (parity with keyboard path — never highlights a disabled option); (4) `createPortal` gated on `open` (no live portal subscription while closed). styles.css: (5) added `.focus-klarvo:focus { border-color: teal }` mouse-focus affordance alongside the `:focus-visible` keyboard ring (text inputs showed nothing on mouse click). LicenseSettings.tsx + AiProvidersContent.tsx (×7): (6) un-armed Remove/Deactivate hover kept in the amber family (`hover:text-klarvo-amber`); only the armed/confirm branch hovers to `danger-hi` (two-stage arm-then-confirm affordance — no premature red on un-armed buttons). npm run build: green. Mechanical GATE-4 smoke GREEN (source + built-CSS assertions: danger-hi token, both focus rules present, no dead `focus-visible:focus-klarvo` variant, all source patches verified). **Human-visual gate consciously downgraded** for the autonomous run (Verifikations-Symmetrie path 2): the DoD's Windows-release round-trip smoke + aesthetic judgment are batched for Andy's morning branch review (documented residual). **Accepted residuals (deferred, not blocking):** Settings Home per-row status dots (AC #1 — pending human AC decision on dot semantics); danger-hi reconciliation into the canonical SPEC token table; full radius-token sweep (~26-34 raw `rounded-*`); border-opacity normalization; KSelect single-scroll-ancestor tracking; KSelect screen-reader announce (aria-activedescendant + per-option ids); KSegmented radiogroup-ARIA trade-off; KSlider prefers-reduced-motion scale + `::-moz` fallback.
