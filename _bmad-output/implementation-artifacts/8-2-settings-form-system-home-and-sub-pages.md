# Story 8.2: Settings Form System + Home & Sub-pages

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a user configuring Klarvo,
I want a consistent, high-quality settings form system across Home and every sub-page,
so that configuration feels instrument-grade instead of stock OS widgets.

## Context (why this story exists now)

Epic 8 originally built its Studio-Dark re-skin entirely on `conductor/epic-8` (a branch that has
since drifted 238 commits behind `v1-ship` and will **not** be merged — see the re-scope commit
`fe4f0a0`, 2026-07-06). That branch's finished 8.2 work serves as a **reference template only**.
This story re-ports the same intent against the *current* `v1-ship` Settings code, which has moved
on since (new Appearance preview-color-picker section, more toggles/segmented groups in Shortcuts,
IA restructure in `dbe3be6`). Verified during story creation: `src/components/settings/types.ts`'s
9 `iconColor` hex values are **byte-identical** to the reference branch's "old hex" column, so the
reference's color-mapping table below still applies unchanged. Everything else (line numbers,
control counts) was re-counted against the current tree and is called out where it differs.

**Depends on 8.1 (done)** — the Studio-Dark token/type/motion foundation and backward-compat aliases
already exist in `src/styles.css`. This story is the first to actually migrate a surface off the
aliases (`klarvo-primary` etc.) onto the named tokens.

## Acceptance Criteria

1. **Given** the Settings Home **When** it renders **Then** the category rows (Recording & Audio ·
   AI & Providers · Appearance · Language · Shortcuts · License · Dictionary) appear with
   color-coded icon badges using the new Studio-Dark icon-color tokens (not the current inline hex
   `iconColor` values), badge radii `klarvo-radius-md` (12px), using only named tokens.
   **Scope note (see Dev Notes "Excluded from this story"):** per-category status dots on the Home
   list are explicitly **out of scope** — deferred to Story 8-7 (fidelity pass).

2. **Given** any sub-page with form controls **When** it renders **Then**:
   - Every native `<select>` element is **replaced** by a custom `KSelect` component (keyboard-operable, token-styled, no OS chrome).
   - Checkboxes / boolean toggles are replaced by a custom `KToggle` component (`role="switch"`), styled with the teal focus ring and `klarvo-radius-full`.
   - `<input type="range">` sliders are replaced by a custom `KSlider` component (styled track + thumb, teal accent, no `accent-klarvo-primary` hack).
   - Segmented button-groups (the hand-rolled "flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5" pattern) in Shortcuts, RecordingAudio, and Appearance become a custom `KSegmented` component.
   - API key inputs in AI & Providers render masked in `font-geist-mono` with a visible masked placeholder (existing `loadedSettings.groqApiKeyMasked` etc. mechanism preserved).
   - **Out of scope:** the 3 native `<input type="color">` swatches in `AppearanceContent.tsx` (text/bg/border color pickers) are not one of the four control types named in this AC — leave them as native color inputs; only the opacity `<input type="range">` next to each stays in scope for `KSlider`.

3. **Given** a control bound to config **When** the user changes and saves it **Then** the existing value round-trips correctly through the new control:
   - The `isDirty` / `loadedSettings` resync `useEffect` in `SettingsPanel.tsx` remains unchanged — new controls must not bypass it.
   - `save_config_locked` / `onSave` is the only write path (ADR-0015). No new direct Tauri calls added.
   - Config keys remain camelCase (no snake_case regression — known silent-ignore trap).
   - No new config fields are introduced by this story; every `onChange` on a swapped control must call the **exact same** existing `setLocal*` setter as the control it replaced.

4. **Given** the AI & Providers page **When** providers/keys are shown **Then** real provider labels are used (Groq, DeepSeek, OpenAI, Anthropic, OpenRouter — already the case) and **no telemetry/tracking UI** exists (NFR6, unchanged by this story).

5. **Given** all Settings surfaces **When** rendered **Then**:
   - Zero inline hex for covered roles. Must be replaced with named tokens:
     - The 9 `iconColor` values in `types.ts` → Studio-Dark mapped values (table below).
     - `LicenseSettings.tsx`: `bg-green-500/20 text-green-400`, `bg-blue-500/15 text-blue-400` (×2), `hover:text-red-300`, `text-klarvo-warning/80` → klarvo tokens.
     - `ui.tsx` `FillerStatsChart`: `bg-orange-500/50` → `bg-klarvo-amber/50`.
   - All backward-compat aliases that survive in Settings/`ui.tsx` (`klarvo-primary`, `klarvo-warning`, `klarvo-accent`, `klarvo-border-active`) are migrated to their Studio-Dark targets (`klarvo-teal`, `klarvo-amber`, `klarvo-teal-hi`, `klarvo-border-2`).
   - `ui.tsx` shared constants `INPUT_CLS`/`INPUT_CLS_M` use the `focus-klarvo` utility instead of `focus:outline-none focus:border-klarvo-primary/40`.
   - Controls are keyboard-operable with the teal focus ring (`focus-klarvo`, from 8.1).

6. **And** the `SettingsPanel.tsx` outer wrapper and Save-button footer are migrated to named tokens: `shadow-klarvo-e3` for the panel shadow (replacing `shadow-xl shadow-black/30`), `rounded-klarvo-xl` (replacing `rounded-2xl`); Save button uses `klarvo-teal`/`klarvo-danger` directly (not `klarvo-primary`).

**DoD:** Windows release build (`scripts/sync-and-build.ps1`) + settings smoke: each custom control
type renders correctly, an existing setting (e.g. hotkey mode) round-trips through save,
`config.json` shows the correct camelCase key; walk `docs/surface-smoke-checklist.md`; `npm run
build` (tsc + vite) green; `cargo check --target x86_64-pc-windows-gnu` shows no *new* errors
(pre-existing C cross-compile failures in ggml/ort/whisper are documented in 8.1's dev record, not
caused by this story — no Rust files are touched here).

## Tasks / Subtasks

- [x] **Task 1: Create shared form component primitives in `src/components/settings/FormControls.tsx`** (AC: #2, #5)
  - [x] 1.1 `KToggle` — `role="switch"`, pill 36×20 (`w-9 h-5`), thumb 16×16 (`w-4 h-4`) at `top-0.5 left-0.5`, `translate-x-4` when on, teal fill (`bg-klarvo-teal/40` or similar — match current visual weight) when on / `bg-klarvo-elevated` when off, `focus-klarvo` on focus, `var(--motion-state) var(--ease-standard)` transition. Extract logic from the existing hand-rolled toggle in `AppearanceContent.tsx` (the "Live Preview" switch) — same behavior, now shared. Props: `checked: boolean`, `onChange: (v: boolean) => void`, `disabled?: boolean`.
  - [x] 1.2 `KSelect` — custom dropdown: styled trigger button (current value + chevron, `rounded-klarvo-sm`, `bg-klarvo-surface-2`, `border-klarvo-border`, `focus-klarvo`), dropdown list as `<ul role="listbox">` positioned absolutely, `bg-klarvo-elevated`, `shadow-klarvo-e2`, hover `bg-klarvo-surface-2`. Closes on Escape / outside-click. `ArrowUp`/`ArrowDown` navigate, `Enter`/`Space` select. `aria-haspopup="listbox"`, `aria-expanded`; options `role="option"` + `aria-selected`. Props: `value: string`, `onChange: (v: string) => void`, `options: { value: string; label: string }[]`, `disabled?: boolean`. **No native `<select>` inside.**
  - [x] 1.3 `KSlider` — styled `<input type="range">`: `.k-slider` CSS class (add to `src/styles.css` `@layer utilities`, see Dev Notes for the exact CSS) drives `::-webkit-slider-thumb`/`::-webkit-slider-runnable-track`; thumb 14×14 teal circle; track fill via inline `style={{ background: linear-gradient(...) }}` computed from `(value-min)/(max-min)`. Drops the `accent-klarvo-primary` hack. Props: `value: number`, `onChange: (v: number) => void`, `min: number`, `max: number`, `step: number`, `disabled?: boolean`.
  - [x] 1.4 `KSegmented` — `<div role="group">` of `<button role="radio">` per option; active = `bg-klarvo-elevated text-klarvo-text border-klarvo-border-2` (or the existing `bg-klarvo-teal/15 text-klarvo-teal` active-pattern already used across these files — keep the existing visual language, don't invent a new one); inactive = `text-klarvo-dim hover:text-klarvo-muted`; `rounded-klarvo-xs` per segment, group `rounded-klarvo-sm`; `focus-klarvo` on the focused segment. Props: `value: string`, `onChange: (v: string) => void`, `options: { value: string; label: string; tooltip?: string }[]`.
  - [x] 1.5 Export all four, no default export, no speculative props beyond what's used.

- [x] **Task 2: Migrate Settings Home — icon-badge tokens** (AC: #1, #5)
  - [x] 2.1 `types.ts`: replace the 9 inline `iconColor` hex strings with the Studio-Dark mapped values (table in Dev Notes). Keep as JS string values (Tailwind can't generate classes from runtime strings — `SettingsRow.tsx`'s `style={{ backgroundColor, color }}` inline pattern is correct, keep it).
  - [x] 2.2 `SettingsRow.tsx`: `klarvo-primary` (TRIAL badge) → `klarvo-teal`. Badge icon shape: currently `rounded-full` — change to `rounded-klarvo-md` (12px) per AC #1.
  - [x] 2.3 `SettingsHome.tsx`: header border `border-klarvo-border/50` → `border-klarvo-border` (solid).

- [x] **Task 3: Migrate sub-page content — swap stock controls for K-components** (AC: #2, #3, #5)
  - [x] 3.1 `RecordingAudioContent.tsx` — 3 native `<select>` (STT model, LLM cleanup, audio device) → `KSelect`. Cloud/Offline button-pair → `KSegmented` (`{value:"cloud",label:"Cloud"}`, `{value:"local",label:"Offline"}`).
  - [x] 3.2 `AiProvidersContent.tsx` — 2 native `<select>` (cleanup style, model) → `KSelect`. Migrate `klarvo-primary`→`klarvo-teal`, `klarvo-warning`→`klarvo-amber`, `klarvo-border-active`→`klarvo-border-2`, `hover:text-red-300`→`hover:text-klarvo-danger` throughout. Add `font-geist-mono` to the 5 API-key `<input type="password">` fields — keep `type="password"` and the masked-placeholder mechanism unchanged.
  - [x] 3.3 `LanguageContent.tsx` — 2 native `<select>` → `KSelect`. `focus:border-klarvo-primary/40` → `focus-klarvo`.
  - [x] 3.4 `AppearanceContent.tsx` — 1 hand-rolled toggle ("Live Preview") → `KToggle`. 7 `<input type="range">` → `KSlider` (this includes the 3 opacity-percentage sliders paired with native `<input type="color">` swatches — leave the color-swatch inputs untouched). 1 native `<select>` (font family) → `KSelect`. 2 segmented button-groups (font-size Klein/Mittel/Groß; Darstellung Compact/Comfortable/Wide) → `KSegmented`.
  - [x] 3.5 `ShortcutsContent.tsx` — 5 `<input type="range">` → `KSlider`. 6 segmented button-groups (hotkey-tab, bubble-tab, and per-tab mode pickers) → `KSegmented` (preserve existing tooltip data where present — pass through the `tooltip` option field). 5 hand-rolled `role="switch"` toggles (bubble-size, edge-snap, auto-paste, insert-and-send, auto-capitalize) → `KToggle`.
  - [x] 3.6 **Verification pass:** for every control swapped in 3.1–3.5, confirm its `onChange`/`onClick` still calls the identical pre-existing setter (`setLocal*`) with the identical value shape it received before — no new intermediate handler, no new state variable (AC #3).

- [x] **Task 4: Migrate remaining inline hex / non-klarvo colors to named tokens** (AC: #4, #5)
  - [x] 4.1 `LicenseSettings.tsx` — `bg-green-500/20 text-green-400` → `bg-klarvo-success/20 text-klarvo-success`; `bg-blue-500/15 text-blue-400` (both occurrences) → `bg-klarvo-teal/15 text-klarvo-teal`; `hover:text-red-300` → `hover:text-klarvo-danger`; `text-klarvo-warning/80` → `text-klarvo-amber/80`; `klarvo-primary` → `klarvo-teal`.
  - [x] 4.2 `ui.tsx` — `FillerStatsChart`: `bg-orange-500/50` → `bg-klarvo-amber/50`. `StatusDot`: `bg-klarvo-primary` → `bg-klarvo-teal` (this component is used live on the AI & Providers sub-page — do not remove it, only re-token it; it stays out of scope for Settings-Home per AC #1's scope note). `HighlightedText`: `bg-klarvo-primary/30 text-klarvo-accent` → `bg-klarvo-teal/30 text-klarvo-teal-hi`. `INPUT_CLS`/`INPUT_CLS_M`: replace `focus:outline-none focus:border-klarvo-primary/40` with `focus-klarvo` (from 8.1).
  - [x] 4.3 `SettingsSubPageHeader.tsx` — `text-klarvo-primary` (back-arrow) → `text-klarvo-teal`.
  - [x] 4.4 `DictionaryContent.tsx` — `text-klarvo-warning/80` → `text-klarvo-amber/80`; `text-klarvo-primary/70` → `text-klarvo-teal/70`.
  - [x] 4.5 `AboutContent.tsx` — `bg-klarvo-primary/10 border-klarvo-primary/20 text-klarvo-primary` → `bg-klarvo-teal/10 border-klarvo-teal/20 text-klarvo-teal`.

- [x] **Task 5: Migrate `SettingsPanel.tsx` outer shell and Save footer** (AC: #6)
  - [x] 5.1 Outer wrapper `<div>`: `shadow-xl shadow-black/30` → `shadow-klarvo-e3`; `rounded-2xl` → `rounded-klarvo-xl`; `bg-klarvo-surface border-klarvo-border/60` stay (already named).
  - [x] 5.2 Save button and its states: `bg-klarvo-primary/15 border-klarvo-primary/30 text-klarvo-primary` (saved state) and `bg-klarvo-primary/10 border-klarvo-primary/30 text-klarvo-primary hover:bg-klarvo-primary/15 hover:border-klarvo-primary/40` (dirty/pulsing state) → `klarvo-teal` equivalents. `animate-pulse` stays.

- [x] **Task 6: Verify build integrity + zero-alias gate** (AC: #3 + DoD)
  - [x] 6.1 `npm run build` (tsc + vite build) — 0 errors.
  - [x] 6.2 `cargo check --target x86_64-pc-windows-gnu` — no *new* errors vs. the pre-existing baseline from 8.1's dev record.
  - [x] 6.3 `grep -rn "klarvo-primary\|klarvo-accent\|klarvo-secondary\|klarvo-warm\|klarvo-activity\|klarvo-warning\|klarvo-info\|klarvo-border-active" src/components/settings/ src/components/ui.tsx src/components/SettingsPanel.tsx` → must be empty.
  - [x] 6.4 `grep -rn "red-300\|green-500\|green-400\|blue-500\|blue-400\|orange-500" src/components/settings/ src/components/ui.tsx` → must be empty.
  - [x] 6.5 `grep -rn "#[0-9a-fA-F]\{6\}\|#[0-9a-fA-F]\{3\}\b" src/components/settings/` → must show only the 9 new Studio-Dark `iconColor` values in `types.ts`, the native `<input type="color">` default-hex constants in `AppearanceContent.tsx`/`previewAppearance.ts` (user-facing default values, not a role color — acceptable), and nothing else.
  - [x] 6.6 `grep -c '<select\|type="range"\|role="switch"' src/components/settings/*.tsx --exclude=FormControls.tsx` → all zero (every native control replaced; `FormControls.tsx` is excluded because it legitimately *defines* the `KSelect`/`KSlider`/`KToggle` primitives this AC's native-control ban targets, not a regression).

## Dev Notes

### Excluded from this story (do not build, do not ask — already decided)

- **Settings-Home per-category status dots.** `epic-8-fidelity-audit.md` line 34 names this as a
  confirmed gap ("Keine Status-Dots auf Kategorie-Rows... Status pro Kategorie ableiten") and the
  epic's re-scope note (`sprint-status.yaml`, epic-8 comment block) explicitly assigns "Status-Dots"
  to **Story 8-7 (fidelity pass, deferred)**, not to the "erstmal so" base re-port this story
  delivers. AC #1 above only asks for icon-badge tokens + radius, not new status indicators.
- **History date-format (US vs. DE-compact)** and **FloatingBar/Live-Preview gaps** — different
  stories (8-5 and superseded 8-3/8-4), not touched here.
- **Native `<input type="color">` swatches** in `AppearanceContent.tsx` — not one of the four
  control types this story's AC names (select/toggle/slider/segmented); left as-is.

### Critical: How SettingsPanel Dirty-Tracking Works (recurring trap — Epics 5/6)

`SettingsPanel.tsx` resyncs all `local*` state from `loadedSettings` after every save, and separately
computes `isDirty` by comparing every `local*` var against its saved counterpart. **This story adds
no new config fields** — Task 3 only swaps the DOM element (e.g. `<select>` → `<KSelect>`); the
`onChange` must call the *exact same* `setLocal*` setter the original control called. If a swap
introduces an intermediate handler or a new local var, the Save button either stays dirty forever
after saving, or never shows dirty at all. Verify this explicitly per control (Task 3.6).

### KSelect — No Native `<select>` Inside

Fully custom, no OS dropdown chrome. The `select option { color:#000; background:#fff }` rule in
`styles.css` was a workaround for OS-select readability on dark backgrounds; once `<select>` is gone
from Settings this rule becomes irrelevant there (it may still apply to `<select>` elsewhere in the
app — do not remove it).

### API Key Inputs Stay `type="password"`

Only add `font-geist-mono`. Do not touch the `type="password"` attribute or the
`loadedSettings.groqApiKeyMasked` etc. masked-placeholder mechanism.

### Token Mapping for `types.ts` `iconColor` (verified identical to reference branch)

| Category | Current hex | New hex (Studio-Dark) | Role |
|---|---|---|---|
| recording-audio | `#2AC3A8` | `#29C7AC` | `klarvo-teal` |
| ai-providers | `#818CF8` | `#57DDC7` | `klarvo-teal-hi` |
| appearance | `#34D399` | `#4FC58A` | `klarvo-success` |
| language | `#22D3EE` | `#57DDC7` | `klarvo-teal-hi` |
| shortcuts | `#F59E0B` | `#E9A24C` | `klarvo-amber` |
| license | `#FFA344` | `#E9A24C` | `klarvo-amber` |
| dictionary | `#60A5FA` | `#29C7AC` | `klarvo-teal` |
| advanced | `#94A3B8` | `#A4A9AC` | `klarvo-muted` |
| about | `#6B7280` | `#6F7479` | `klarvo-dim` |

(`ai-providers` and `language` sharing `#57DDC7` is acceptable — two categories sharing a badge
color; this was already the accepted resolution on the reference branch.)

### KSlider CSS (add to `src/styles.css` `@layer utilities`)

```css
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
.k-slider::-webkit-slider-thumb:hover { transform: scale(1.15); }
.k-slider:focus-visible { box-shadow: 0 0 0 3px rgba(41,199,172,.28); }
```
Track fill: `KSlider` computes `pct = ((value-min)/(max-min))*100` and sets
`style={{ background: \`linear-gradient(to right, var(--color-klarvo-teal) ${pct}%, var(--color-klarvo-border-2) ${pct}%)\` }}`.

### KToggle — Exact Dimensions (match existing hand-rolled toggles)

Pill `w-9 h-5` (36×20), thumb `w-4 h-4` (16×16) at `top-0.5 left-0.5`, `translate-x-4` when on. This
is exactly the pattern already hand-rolled 6 times across `AppearanceContent.tsx` (1×) and
`ShortcutsContent.tsx` (5×) — `KToggle` extracts it into one shared component, behavior unchanged.

### Current Control Inventory (re-counted against today's tree, story creation time)

- `<select>`: `RecordingAudioContent.tsx` ×3, `AiProvidersContent.tsx` ×2, `LanguageContent.tsx` ×2, `AppearanceContent.tsx` ×1 — **8 total**.
- `<input type="range">`: `AppearanceContent.tsx` ×7, `ShortcutsContent.tsx` ×5 — **12 total**.
- Hand-rolled `role="switch"` toggle: `AppearanceContent.tsx` ×1, `ShortcutsContent.tsx` ×5 — **6 total**.
- Hand-rolled segmented groups (`flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 ...`): `RecordingAudioContent.tsx` ×1 (Cloud/Offline), `AppearanceContent.tsx` ×2 (font-size, Darstellung), `ShortcutsContent.tsx` ×6 (hotkey-tab, bubble-tab, and per-tab pickers) — **9 total**.

These counts are higher than the reference branch's story (built 2026-06-14) because
`AppearanceContent.tsx` grew a preview-color-picker section and `ShortcutsContent.tsx` grew bubble
settings since then (`dbe3be6`, `565dc0b`, `21ae533`, `af1c0bd`). Re-verify counts with the Task 6
greps before calling any sub-task done — do not rely on this table alone if the tree has moved again.

### Files to Modify

| File | Change |
|---|---|
| `src/components/settings/FormControls.tsx` | **NEW** — KToggle, KSelect, KSlider, KSegmented |
| `src/styles.css` | Add `.k-slider` CSS to `@layer utilities` |
| `src/components/settings/types.ts` | 9 `iconColor` hex values |
| `src/components/settings/SettingsRow.tsx` | `klarvo-primary`→`klarvo-teal`; badge `rounded-full`→`rounded-klarvo-md` |
| `src/components/settings/SettingsHome.tsx` | Header border token |
| `src/components/settings/RecordingAudioContent.tsx` | 3×`<select>`→KSelect; Cloud/Offline→KSegmented |
| `src/components/settings/AiProvidersContent.tsx` | 2×`<select>`→KSelect; alias tokens→named; `font-geist-mono` on key inputs |
| `src/components/settings/LanguageContent.tsx` | 2×`<select>`→KSelect |
| `src/components/settings/AppearanceContent.tsx` | 7×range→KSlider; 1×`<select>`→KSelect; toggle→KToggle; 2×segmented groups→KSegmented |
| `src/components/settings/ShortcutsContent.tsx` | 5×range→KSlider; 6×segmented groups→KSegmented; 5×toggle→KToggle |
| `src/components/settings/LicenseSettings.tsx` | Non-klarvo color classes → klarvo tokens |
| `src/components/settings/DictionaryContent.tsx` | Alias tokens → named |
| `src/components/settings/AboutContent.tsx` | Alias tokens → named |
| `src/components/settings/SettingsSubPageHeader.tsx` | Alias token → named |
| `src/components/ui.tsx` | `FillerStatsChart`, `StatusDot`, `HighlightedText`, `INPUT_CLS`/`INPUT_CLS_M` |
| `src/components/SettingsPanel.tsx` | Outer shadow/radius tokens; Save button tokens |

**No Rust/Tauri changes. No new config keys. No changes to `App.tsx` or the IPC layer.**

### Surface-Smoke-Checklist Items Relevant Here

From `docs/surface-smoke-checklist.md`: **trap #1** (camelCase config keys — verify a multi-word
key like `sttProvider` still round-trips after a `KSelect`/`KSlider` emits its value) and **trap #2**
(resync `useEffect` — after saving through a new control, confirm the Save button returns to
hidden, not stuck-dirty). Trap #3 (separate-window reactivity) is not relevant — no separate window
touched by this story.

### References

- [Source: `_bmad-output/planning-artifacts/epics-visual-overhaul.md` — Story 8.2 ACs] — UX-DR2, NFR2, NFR3, NFR6, NFR7 scope
- [Source: `docs/design/overhaul/SPEC-studio-dark-overhaul.md`] — token/type/radii/elevation/motion values (already landed in 8.1)
- [Source: `docs/design/overhaul/02-surfaces.md` §C] — Settings surface redesign goal
- [Source: `_bmad-output/implementation-artifacts/8-1-token-and-type-foundation.md`] — foundation tokens, aliases, `focus-klarvo`, motion vars, `font-geist-mono`
- [Source: `_bmad-output/implementation-artifacts/epic-8-fidelity-audit.md` line 33-35] — confirms Settings-Home status-dot gap is a known, separately-tracked item (8-7), not this story's scope
- [Source: reference branch `conductor/epic-8`, `_bmad-output/implementation-artifacts/8-2-settings-form-system-home-and-sub-pages.md` (done, 2026-06-14) + `src/components/settings/FormControls.tsx`] — template for the four K-components and the color-mapping table; **not mergeable** (238 commits behind), used as a pattern reference only, re-verified against today's tree
- [Source: `src/components/SettingsPanel.tsx`] — `isDirty`/`loadedSettings` resync mechanism, Save button, outer wrapper
- [Source: `src/components/settings/types.ts`, `SettingsRow.tsx`, `AppearanceContent.tsx`, `ShortcutsContent.tsx`, `RecordingAudioContent.tsx`, `AiProvidersContent.tsx`, `LanguageContent.tsx`, `LicenseSettings.tsx`, `DictionaryContent.tsx`, `AboutContent.tsx`, `SettingsSubPageHeader.tsx`, `src/components/ui.tsx`] — current control inventory and alias usage, verified via grep during story creation (counts above)
- [Source: `docs/surface-smoke-checklist.md`] — camelCase config-key trap, resync-useEffect trap
- [Source: `_bmad-output/project-context.md` — Framework-Specific Rules] — config camelCase, `save_config_locked`, ADR-0015, no new direct Tauri calls
- [Source: `_bmad-output/implementation-artifacts/sprint-status.yaml` — epic-8 re-scope comment, 2026-07-06] — 8-2 re-port scope, `conductor/epic-8` non-merge decision

## Dev Agent Record

### Agent Model Used

Claude Sonnet 5 (claude-sonnet-5), via bmad-dev-story skill.

### Debug Log References

- `npm run build` (tsc + vite): green, no errors.
- `cargo check --target x86_64-pc-windows-gnu`: fails at `ort-sys` build script
  ("downloaded binaries not available for target x86_64-pc-windows-gnu") — this is the
  pre-existing baseline failure documented in 8.1's dev record (ggml/ort/whisper cross-compile
  gap), not a regression from this story. No Rust files were touched by this story.
- Task 6.3–6.6 verification greps: all pass (alias tokens, stray Tailwind colors, hex literals,
  native-control count all clean in `src/components/settings/*.tsx` + `ui.tsx` +
  `SettingsPanel.tsx`; the only `type="range"`/`role="switch"` hits are the 4 in the new
  `FormControls.tsx` itself — the underlying primitive implementations the AC's native-control
  ban targets, not a regression).

### Completion Notes List

- Created `FormControls.tsx` with `KToggle`, `KSelect`, `KSlider`, `KSegmented` — token-styled,
  keyboard-operable, no OS chrome. `KSelect` supports optional per-option `disabled` (kept parity
  with the pre-existing disabled-provider-option behavior in `AiProvidersContent`/
  `RecordingAudioContent`); no other speculative props added.
  - `KSelect`: custom `<ul role="listbox">` dropdown, Escape/outside-click close,
    ArrowUp/Down navigate, Enter/Space select, `aria-haspopup`/`aria-expanded`/`aria-selected`.
  - `KSlider`: `.k-slider` CSS added to `styles.css` `@layer utilities` (thumb/track styling,
    focus-visible ring); track-fill gradient computed inline from `(value-min)/(max-min)`.
  - `KSegmented`: `role="group"` of `role="radio"` buttons, active/inactive classes match the
    pre-existing `bg-klarvo-teal/15 text-klarvo-teal` visual language (no new pattern invented).
- Migrated all 9 `iconColor` hex values in `types.ts` to the Studio-Dark mapping table (verified
  byte-identical to the reference branch during story creation).
- Swapped every native `<select>` (8), `<input type="range">` (12), hand-rolled `role="switch"`
  toggle (6), and the 9 `flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5`-pattern segmented groups
  across `RecordingAudioContent`, `AiProvidersContent` (incl. the per-profile cleanup-style/
  language selects), `LanguageContent`, `AppearanceContent`, `ShortcutsContent` — for the K-
  equivalents, with every `onChange` calling the exact pre-existing `setLocal*`/handler (verified
  per-control per Task 3.6 — no new intermediate state introduced, `SettingsPanel.tsx`'s
  `isDirty`/resync `useEffect` untouched).
- The "Recording Button Size" segmented control in `ShortcutsContent.tsx` was deliberately left
  as native buttons — it uses a different container/active-state pattern
  (`rounded-md overflow-hidden border`, not the `flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5`
  pattern) and was not one of the 9 hand-rolled groups named in the Dev Notes control inventory;
  only its `klarvo-primary` alias was migrated to `klarvo-teal`.
- Migrated every remaining `klarvo-primary`/`klarvo-warning`/`klarvo-accent`/`klarvo-border-active`
  alias and stray Tailwind color (`red-300`, `green-400/500`, `blue-400/500`, `orange-500`) in
  `LicenseSettings.tsx`, `ui.tsx` (`StatusDot`, `FillerStatsChart`, `HighlightedText`,
  `INPUT_CLS`/`INPUT_CLS_M`), `DictionaryContent.tsx`, `AboutContent.tsx`,
  `SettingsSubPageHeader.tsx`, and the remaining aliases surfaced in `AiProvidersContent.tsx`/
  `ShortcutsContent.tsx`/`AppearanceContent.tsx` beyond the Task-4 list (e.g. the `ShortcutRecorder`
  listening-state border/text, the theme-preset hover border) to named tokens.
- Migrated `SettingsPanel.tsx`'s outer wrapper (`shadow-klarvo-e3`, `rounded-klarvo-xl`) and Save
  button (saved/dirty states now use `klarvo-teal`).
- `AiProvidersContent`'s 5 API-key `<input type="password">` fields got `font-geist-mono`; `type`
  and the masked-placeholder mechanism (`loadedSettings.*ApiKeyMasked`) untouched.
- No Rust/Tauri changes, no new config keys, no changes to `App.tsx` or the IPC layer.
- **Not verifiable from this environment (WSL/Linux):** the DoD's Windows release build +
  settings smoke (each K-control renders correctly, a setting round-trips through save,
  `config.json` shows the correct camelCase key, `docs/surface-smoke-checklist.md` walked). This
  needs Andi's real-device gate per project convention (surface/UI stories require a Windows
  release build + manual press-to-paste-equivalent smoke; `cargo check`/`tsc` do not exercise the
  Tauri runtime or Windows-only paint paths). Story is left at `review`, not `done`.

### File List

- `src/components/settings/FormControls.tsx` (NEW)
- `src/styles.css`
- `src/components/settings/types.ts`
- `src/components/settings/SettingsRow.tsx`
- `src/components/settings/SettingsHome.tsx`
- `src/components/settings/RecordingAudioContent.tsx`
- `src/components/settings/AiProvidersContent.tsx`
- `src/components/settings/LanguageContent.tsx`
- `src/components/settings/AppearanceContent.tsx`
- `src/components/settings/ShortcutsContent.tsx`
- `src/components/settings/LicenseSettings.tsx`
- `src/components/settings/DictionaryContent.tsx`
- `src/components/settings/AboutContent.tsx`
- `src/components/settings/SettingsSubPageHeader.tsx`
- `src/components/ui.tsx`
- `src/components/SettingsPanel.tsx`

## Change Log

- 2026-07-06 (story creation): Re-created against current `v1-ship` tree; reference branch
  `conductor/epic-8`'s finished 8.2 implementation used as a pattern template (not merged — 238
  commits behind). Control counts re-verified via grep against today's codebase (higher than the
  reference build due to Appearance/Shortcuts growth since 2026-06-14). Settings-Home status-dots
  explicitly scoped out per the fidelity-audit + epic re-scope decision (deferred to 8-7).
- 2026-07-06 (dev-story): Implemented all 6 tasks — `FormControls.tsx` primitives, Settings-Home
  token migration, full sub-page control swap (select/toggle/slider/segmented), remaining alias/
  stray-color migration, `SettingsPanel.tsx` shell/Save-button tokens, and the Task-6 build +
  zero-alias verification gate. `npm run build` green; `cargo check --target
  x86_64-pc-windows-gnu` shows only the pre-existing `ort-sys` cross-compile failure (no new
  errors, no Rust files touched). Status → review; Windows release-build settings smoke is
  Andi's real-device gate.
- 2026-07-06 (code-review fix pass): The first-pass `FormControls.tsx` (241 lines) was a naive
  re-implementation that regressed the finished reference at `conductor/epic-8`
  (455 lines) — non-portal `KSelect` dropdown clipped by the sub-page's `overflow-y-auto`
  scroll container, no focus-return-to-trigger, re-focus-every-render bug, no fallback label
  for an unmatched value, arrow-nav didn't skip disabled options, broken `KSegmented` ARIA,
  and an unguarded `KSlider` divide-by-zero. Restored the reference's robust internals for
  `KSelect`/`KSegmented`/`KSlider`/`KToggle` onto the current file (kept the current file's API
  superset — `className` retained on `KSelect`/`KSlider`/`KSegmented` since existing consumers in
  `LanguageContent.tsx`/`AppearanceContent.tsx`/`RecordingAudioContent.tsx`/`ShortcutsContent.tsx`
  pass it). `KSelect` now renders its dropdown via `createPortal(dropdown, document.body)`,
  `position: fixed` computed from the trigger's `getBoundingClientRect()`, recomputed on
  scroll (walks up to the nearest `.overflow-y-auto` ancestor) and resize; closes and returns
  focus to the trigger on Escape/select/outside-click, and also closes (without stealing focus)
  when Tab moves focus out of the open listbox. Arrow-key nav skips disabled options; committing
  on a disabled option no longer closes without selecting. Corrected Task-6.6's grep gate wording
  (excludes `FormControls.tsx`, which legitimately *defines* the primitives the gate bans
  elsewhere). `npm run build` green, 0 TS errors; all existing consumer call sites in
  `src/components/settings/*.tsx` still type-check with unchanged props/setters (no
  dirty-tracking behavior change). Only `FormControls.tsx` and this story file were touched — no
  Rust files, no 8-7-deferred residuals (yellow-/amber- colors, `#2ac3a8` preview-border default)
  touched.
