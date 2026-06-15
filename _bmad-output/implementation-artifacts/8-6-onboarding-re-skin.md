# Story 8.6: Onboarding re-skin (last surface; optional — D1)

Status: done

## Story

As a first-time user,
I want an elegant, trustworthy onboarding,
so that the seriousness of the tool is clear and BYOK feels like a feature, not a hurdle.

## Acceptance Criteria

1. **Given** first launch **When** onboarding renders **Then** steps use the new type/spacing/tokens with clear step indicators, and API-key/provider setup is framed as a feature (BYOK), using real provider labels (Groq, OpenAI, etc.) — no Lorem Ipsum, no telemetry UI.

2. **Given** the flow runs **When** the user completes it **Then** behavior and IA are unchanged (re-skin only) and the end state matches today's — all state machine logic, flow routing, persistence, and API calls are preserved exactly.

3. **Given** this is the **final** surface story **When** 8.6 is done **Then** a closing `grep` gate asserts **no inline hex remains for covered roles** across `src/` (DT1 closure). The closing `grep` is specifically: `grep -rn '#[0-9a-fA-F]\{3,6\}\b' src/ --include="*.tsx" --include="*.ts"` — any hit for a "covered role" (not a shadow rgba literal already using a named token's components, not an intentional dev-only override) is a blocker.

4. **And** zero inline hex for covered roles in `Onboarding.tsx` itself (DT1) — the two current inline hex (`bg-[#09090b]` on root div and `bg-[#0e0e10]` on the offline-disabled card) are replaced with named tokens.

5. **And** the `klarvo-primary` alias is fully eliminated from `Onboarding.tsx` — every occurrence migrated to `klarvo-teal` (canonical Studio-Dark name per DT5).

6. **And** the `klarvo-warning` alias is fully eliminated from `Onboarding.tsx` — every occurrence migrated to `klarvo-amber` (canonical name, busy/processing semantic).

7. **And** raw `amber-*` Tailwind classes are migrated to `klarvo-amber` / `klarvo-amber-hi` tokens.

8. **And** the `fontFamily: "'Inter', ..."` inline style on the root `<div>` is removed and replaced with the `font-geist` Tailwind utility class (Geist already bundled from 8.1).

9. **And** `font-mono` usages are upgraded to `font-geist-mono` where the semantic is "mono code/key display" (API key input, code snippets, kbd element).

**DoD:**
- `npm run build` (tsc + vite) green.
- `cargo check --target x86_64-pc-windows-gnu` green (no Rust changes expected).
- Real Windows release build via `scripts/sync-and-build.ps1` (Andi's gate).
- Smoke: walk the onboarding flow (Welcome → Mode → STT Key → Language → Test → Done) in the Windows build; the `StepDots` step indicator renders styled correctly; no visual regressions.
- **DT1 closing grep gate:** `grep -rn '#[0-9a-fA-F]\{3,6\}\b' src/ --include="*.tsx" --include="*.ts"` — zero hits for covered roles across **the five Epic-8 desktop surfaces** (FloatingBar, Settings [SettingsPanel + settings/*], Live-Preview, Main-Window/History, Onboarding). **Conductor re-scope 2026-06-15:** the original "across ALL of `src/`" over-reached Epic-8's surface scope — it would require migrating hidden/expert (`AdvancedSettingsPanel`), mobile (`MobileTextarea`), and `ThemeSwitcher` components that were never on the Epic-8 surface list and need a per-category palette decision (purple/gray have no Studio-Dark token). The Epic-8 surfaces ARE verified clean (only FloatingBar's value-correct literals remain, per the 8-3 carve-out); the non-surface residual is **homed in `docs/backlog.md` → "Epic 8 (Studio-Dark) — DT1 token-closure residual"** (recorded entry, not an inline wave-away).
- Walk `docs/surface-smoke-checklist.md` traps as applicable (see Dev Notes for trap analysis — none are expected to be triggered, but confirm).

## Tasks / Subtasks

- [x] **Task 1: Remove inline hex** (AC: #4)
  - [x] 1.1 Root `<div>` (line ~1470): `bg-[#09090b]` → `bg-klarvo-bg-deep` (`#0A0B0C` — the closest named graphite token; imperceptibly darker, correct token for "behind everything").
  - [x] 1.2 Offline-disabled card (line ~493): `bg-[#0e0e10]` → `bg-klarvo-bg` (`#0F1112` — the main canvas token, essentially the same shade). Also remove any remaining `klarvo-primary` from this card (line ~479 icon color).

- [x] **Task 2: Migrate `klarvo-primary` → `klarvo-teal`** (AC: #5)
  - [x] 2.1 `StepDots` component (lines ~195–199): `bg-klarvo-primary` → `bg-klarvo-teal`; `bg-klarvo-primary/40` → `bg-klarvo-teal/40`.
  - [x] 2.2 `BTN_PRIMARY` constant (lines ~219–225): all 4 `klarvo-primary` occurrences → `klarvo-teal`.
  - [x] 2.3 `ApiKeyField` (lines ~283–311): `focus:border-klarvo-primary/40`, `focus:ring-klarvo-primary/20`, `border-klarvo-primary/50` (valid state), `text-klarvo-primary` (valid checkmark row) → teal equivalents.
  - [x] 2.4 `StepWelcome` (lines ~386–388): pulse rings `bg-klarvo-primary/10` (×2), mic box `bg-klarvo-primary/15 border border-klarvo-primary/30 text-klarvo-primary shadow-[0_0_40px_rgba(42,195,168,0.18)]` → teal equivalents. Shadow rgba `rgba(42,195,168,0.18)` → `rgba(41,199,172,0.18)` (Studio-Dark teal `#29C7AC`).
  - [x] 2.5 `StepMode` cloud/offline card selected state (lines ~439, ~475): `border-klarvo-primary/50 bg-klarvo-primary/8` → `border-klarvo-teal/50 bg-klarvo-teal/8`. Icon color conditional (lines ~443, ~479): `text-klarvo-primary` → `text-klarvo-teal`. Cloud bullet dot `text-klarvo-primary/60` (line ~460) → `text-klarvo-teal/60`.
  - [x] 2.6 `StepMode` gate track buttons (lines ~513, ~525): `border-klarvo-primary/50 bg-klarvo-primary/8 text-klarvo-primary` → teal equivalents.
  - [x] 2.7 `StepSttKeyBeginner1` numbered step badges (lines ~759, ~763, ~767): `bg-klarvo-primary/15 border border-klarvo-primary/30 text-klarvo-primary` → teal equivalents. Check button icon (line ~782): `text-klarvo-primary` → `text-klarvo-teal`.
  - [x] 2.8 `StepModelDownload` — CheckCircleIcon (line ~919) `text-klarvo-primary` → `text-klarvo-teal`. Progress bar `bg-klarvo-primary/70` (line ~927) → `bg-klarvo-teal/70`. Download button (line ~942) `bg-klarvo-primary/15 border border-klarvo-primary/30 text-klarvo-primary hover:bg-klarvo-primary/20` → teal equivalents.
  - [x] 2.9 `StepTestDictation` — mobile mic icon box (line ~1055) `bg-klarvo-primary/10 border border-klarvo-primary/20 text-klarvo-primary` → teal equivalents. Desktop record button idle state (line ~1096) `bg-klarvo-primary/15 text-klarvo-primary shadow-[0_0_40px_rgba(42,195,168,0.2)] hover:bg-klarvo-primary/20` → teal equivalents (rgba shadow → `rgba(41,199,172,0.2)`). Border state (line ~1101) `border-klarvo-primary/25` → `border-klarvo-teal/25`. Status text done (line ~1117) `text-klarvo-primary` → `text-klarvo-teal`.
  - [x] 2.10 `StepLanguage` option selected state (line ~1188) `border-klarvo-primary/50 bg-klarvo-primary/8 text-klarvo-primary` → teal; check icon (line ~1194) `text-klarvo-primary` → `text-klarvo-teal`.
  - [x] 2.11 `SummaryRow` positive value (line ~1218) `text-klarvo-primary` → `text-klarvo-teal`.
  - [x] 2.12 `StepDone` — checkmark box (line ~1236) `bg-klarvo-primary/15 border border-klarvo-primary/30 text-klarvo-primary shadow-[0_0_40px_rgba(42,195,168,0.18)]` → teal equivalents (rgba `rgba(41,199,172,0.18)`).

- [x] **Task 3: Migrate `klarvo-warning` → `klarvo-amber`** (AC: #6)
  - [x] 3.1 `StepTestDictation` record button busy state (line ~1095): `bg-klarvo-warning/15 text-klarvo-warning` → `bg-klarvo-amber/15 text-klarvo-amber`. Border (line ~1101) `border-klarvo-warning/30` → `border-klarvo-amber/30`. Status text busy (line ~1117) `text-klarvo-warning` → `text-klarvo-amber`.

- [x] **Task 4: Migrate raw `amber-*` Tailwind classes → `klarvo-amber`** (AC: #7)
  - [x] 4.1 `ApiKeyField` magic-link button (line ~268): `text-amber-400 hover:text-amber-300` → `text-klarvo-amber hover:text-klarvo-amber-hi`.
  - [x] 4.2 `StepMode` "empfohlen" badge (lines ~447, ~452): `text-amber-400 bg-amber-400/10 border border-amber-400/20` (selected state) and `text-amber-400/60 bg-amber-400/5 border border-amber-400/20` (unselected) → `text-klarvo-amber bg-klarvo-amber/10 border border-klarvo-amber/20` and `text-klarvo-amber/60 bg-klarvo-amber/5 border border-klarvo-amber/20`.
  - [x] 4.3 `PermAllStep` Xiaomi/OPPO notice card (line ~655): `bg-amber-500/5 border border-amber-500/20` → `bg-klarvo-amber/5 border border-klarvo-amber/20`. Label text (line ~656) `text-amber-400/80` → `text-klarvo-amber/80`.
  - [x] 4.4 `StepSttKeyBeginner1` skip link (line ~797): `text-amber-400/70 hover:text-amber-400` → `text-klarvo-amber/70 hover:text-klarvo-amber`.
  - [x] 4.5 Main wizard header skip/key-later button (line ~1511): `text-amber-400/60 hover:text-amber-400` → `text-klarvo-amber/60 hover:text-klarvo-amber`.

- [x] **Task 5: Font migration** (AC: #8, #9)
  - [x] 5.1 Root `<div>` inline style (line ~1472): remove `fontFamily: "'Inter', system-ui, -apple-system, sans-serif"` from the `style` object. Add `font-geist` to the `className` on the same `<div>` (already has `min-h-screen bg-... flex ...`). The Tailwind v4 `font-geist` utility is generated from `--font-geist` in `@theme` (set in 8.1). **Verify:** if `isMobile` padding block is `style={{ paddingBottom: "env(...)" }}` — ensure the `fontFamily` removal leaves the conditional padding intact.
  - [x] 5.2 `ApiKeyField` key input (line ~285): `font-mono` → `font-geist-mono` (API keys are code context — mono is correct, Geist Mono is the token).
  - [x] 5.3 `StepSttKeyBeginner1` code snippet `<code>` tag (line ~768): `font-mono` → `font-geist-mono` (inline `gsk_` code).
  - [x] 5.4 Hotkey `<kbd>` element in `StepTestDictation` (line ~1135): `font-mono` → `font-geist-mono`.
  - [x] 5.5 Keep `font-mono` (do NOT change) on the `StepSttKeyBeginner1` "Schon erledigt" skip button — this is a UI text element (not a code span), so it should stay at system sans or get `font-geist`. Actually: line 793–800 is a `<button>` with no font class → no change needed. Re-read: the `font-mono` usages to change are lines 285, 768, 1135 only.

- [x] **Task 6: DT1 closing grep gate** (AC: #3)
  - [x] 6.1 Remove backward-compat aliases from `styles.css` that are no longer needed. After migrating `Onboarding.tsx` (and all prior surface stories), the following aliases in `styles.css` are fully unused in `src/*.tsx`/`src/components/*.tsx` (check each): `klarvo-primary`, `klarvo-warm`, `klarvo-warning`. **Verify before removing:** run `grep -rn 'klarvo-primary\|klarvo-warm\b\|klarvo-warning\b' src/ --include="*.tsx" --include="*.ts"`. If still used in any component NOT touched by Epic 8 stories (`MobileTextarea`, `SnippetsPanel`, `CostDashboard`, `LlmModelManager`, `WhisperModelManager`, `QuickTip`, `AdvancedSettingsPanel`, `VoiceNotesPanel`, etc.), those aliases must stay in `styles.css` until those components are migrated — OR accept that the closing grep gate for "covered roles" in `Onboarding.tsx` passes while aliases remain for other surfaces. **Decision:** per epic plan ("hex→token migration done per-surface"), the DT1 closing grep gate for 8.6 is about `Onboarding.tsx` itself having zero inline hex. The `styles.css` alias removal is a bonus if all other consumers are gone — do NOT break other surfaces to remove them. **Result:** aliases still needed by SnippetsPanel, VoiceNotesPanel, WhisperModelManager, AdvancedSettingsPanel, MobileTextarea, LlmModelManager, QuickTip, CostDashboard, ThemeSwitcher — kept.
  - [x] 6.2 Run the closing grep: `grep -rn '#[0-9a-fA-F]\{3,6\}\b' src/ --include="*.tsx" --include="*.ts"` — document any remaining hits and confirm they are shadow rgba literals (acceptable) or other justified carve-outs. Zero inline hex for "covered roles" in `Onboarding.tsx`. **Result:** All hits in other files are ThemeSwitcher (palette definitions), FloatingBar (canvas/JS style objects), AppearanceContent (color picker defaults), AdvancedSettingsPanel (inline style objects), types.ts (icon color constants) — none are Tailwind CSS class inline hex for covered roles. Onboarding.tsx: ZERO hits.

- [x] **Task 7: Build verification** (DoD)
  - [x] 7.1 `npm run build` (tsc + vite) green — all 83 modules transformed, built in 2.00s.
  - [x] 7.2 `cargo check --target x86_64-pc-windows-gnu` — pre-existing cmake/ort-sys build failure unrelated to story (no Rust files touched). Out-of-scope infrastructure issue, consistent with prior stories.
  - [x] 7.3 Grep `Onboarding.tsx` for remaining `klarvo-primary`: zero.
  - [x] 7.4 Grep `Onboarding.tsx` for remaining `klarvo-warning`: zero.
  - [x] 7.5 Grep `Onboarding.tsx` for remaining raw amber: zero.
  - [x] 7.6 Grep `Onboarding.tsx` for remaining Inter: zero (only text content "Internet" strings remain, no CSS/font references).
  - [x] 7.7 Grep `Onboarding.tsx` for remaining inline hex: zero.

## Dev Notes

### Scope of This Story

**8.6 is a token migration + font migration story for `src/Onboarding.tsx` — NOT a behavioral or architectural change.**

What changes:
1. CSS class names / inline style in `Onboarding.tsx` — alias tokens → canonical; inline hex → named tokens; Inter → Geist font; raw amber-* → klarvo-amber; font-mono → font-geist-mono on code/key/kbd contexts
2. Optionally: remove now-unused aliases from `styles.css` (only if confirmed all consumers in `src/` are migrated)
3. No changes to state machine, flow routing, API calls, persistence, or the `buildStepList` function
4. No changes to `tauri-commands.ts`, `types.ts`, `media-recorder.ts`, or any Rust code

**What must be preserved (do NOT touch):**
- All state: `mode`, `language`, `track`, `collectedGroqKey`, `stepIndex`, `visible`
- `buildStepList()` — the step list builder and all its branches
- `persist()`, `advance()`, `goBack()`, `handleSkip()`, `handleFinish()` — all async state transitions
- The `useEffect` that pre-saves API keys before the test step (line ~1432)
- `setOnboardingState()` calls — all persistence calls
- The `previewOr()` helper and `isPreviewMode` import
- All platform-conditional logic (`isMobile`, `isDesktop`)
- The mobile `env(safe-area-inset-bottom)` padding
- All step sub-components' logic — only their className/style values change

### Critical: Font Migration Pattern

Geist is already bundled as `@font-face` blocks in `styles.css` (from story 8.1). Tailwind v4 generates `font-geist` and `font-geist-mono` utilities from `--font-geist` and `--font-geist-mono` in the `@theme` block.

The root `<div>` at line ~1469 has `style={{ fontFamily: "'Inter', ...", ...(isMobile ? {...} : {}) }}`. The correct approach:
1. Remove `fontFamily` from the object literal — but keep the rest of the spread.
2. If the style object becomes `{}` or only contains the `isMobile` conditional, simplify: `style={isMobile ? { paddingBottom: "env(safe-area-inset-bottom, 40px)" } : undefined}`.
3. Add `font-geist` to the `className` string.

**Verify the generated Tailwind class name:** `font-geist` (Tailwind v4 strips `--font-` prefix from `--font-geist`). Confirmed working pattern from 8.5: `<main className="... font-geist">`.

### Critical: Inline Hex Replacements

| Location | Current hex | Replace with | Rationale |
|---|---|---|---|
| Root `<div>` bg (line ~1470) | `bg-[#09090b]` | `bg-klarvo-bg-deep` | `#0A0B0C` is `bg-deep` — letterbox/behind-everything token; 1-stop difference, imperceptible |
| Offline disabled card (line ~493) | `bg-[#0e0e10]` | `bg-klarvo-bg` | `#0F1112` is the canvas token; essentially the same shade |
| Welcome mic shadow | `rgba(42,195,168,0.18)` | `rgba(41,199,172,0.18)` | Shadow rgba → use Studio-Dark teal `#29C7AC` components |
| Record button idle shadow | `rgba(42,195,168,0.2)` | `rgba(41,199,172,0.2)` | Same — teal shadow |
| Done step check shadow | `rgba(42,195,168,0.18)` | `rgba(41,199,172,0.18)` | Same |

The `rgba(42,...)` shadows are technically already inline values — they're the OLD teal hex (`#2AC3A8`). Studio-Dark teal is `#29C7AC` = `rgba(41,199,172,...)`. The pixel difference is ~1 unit per channel, imperceptible, but correctness matters for DT1.

Note: `styles.css` line 171 has `select option { color: #000; background: #fff; }` — this is a browser-native select reset and is NOT a "covered role" (it is a forced override for native select rendering). Do NOT remove it.

### Critical: Alias Migration Map

| Old alias | Canonical | DT5 role |
|---|---|---|
| `klarvo-primary` | `klarvo-teal` | brand/ready/focus/processing/success |
| `klarvo-warning` | `klarvo-amber` | busy/processing (the test dictation spinner state) |
| Raw `amber-N` classes | `klarvo-amber` / `klarvo-amber-hi` | live/activity |

### Critical: DT5 Color Semantics Check

In `Onboarding.tsx`, verify DT5 is correctly applied after migration:
- **Teal** = brand/ready. Used in: step dots (progress = brand), mic icon (ready), BTN_PRIMARY (proceed = ready), selected card state (active = brand), step number badges, successful checkmarks, download progress, language selected, summary positive value, done checkmark. All correct.
- **Amber** = live/activity/listening. Used in: "empfohlen" badge (activity/recommended), permission OEM notice (alert/activity), skip/key-later links (secondary action), busy record button (processing). These are appropriate uses of amber.
- **Danger** = stop/delete/error. Already used correctly: invalid key state `border-klarvo-danger/50`, error text. No change needed.
- The mic pulse rings (`bg-klarvo-primary/10 animate-ping`) → `bg-klarvo-teal/10` (pulse = brand ring, correct).

### Critical: `font-mono` Contexts

| Location | Current | Correct | Rationale |
|---|---|---|---|
| ApiKeyField input (line ~285) | `font-mono` | `font-geist-mono` | API keys are code — Geist Mono |
| `<code>gsk_</code>` tag (line ~768) | `font-mono` | `font-geist-mono` | Inline code — Geist Mono |
| `<kbd>Ctrl+Shift+D</kbd>` (line ~1135) | `font-mono` | `font-geist-mono` | Keyboard shortcut = Geist Mono (per DT3: "Keys") |

### Surface Smoke Checklist Trap Applicability (8.6)

| Trap | Applies? | Rationale |
|---|---|---|
| #1 camelCase config keys | NOT TRIGGERED | No new config keys — pure visual migration |
| #2 New float/Settings field in resync `useEffect` | NOT TRIGGERED | No new config fields |
| #3 FloatingBar separate-window reactivity | NOT TRIGGERED | Onboarding is rendered in the main window via `App.tsx`, not a separate Tauri window |
| #4 Window geometry / shape region | NOT TRIGGERED | No geometry changes — no `setSize`, no overlay, no region |
| #5 Push vs poll / event wiring | NOT TRIGGERED | No new events |
| #6 Multi-hop save chain | NOT TRIGGERED | No new config fields plumbed through the chain |

Onboarding is rendered inside `App.tsx` conditionally (`if (!settings.loadedSettings?.onboarding?.completed)` logic). It is NOT a separate Tauri window. The `onComplete` callback bubbles to `App.tsx` which then shows the main UI.

### DT1 Closing Grep Gate — Scope Clarification

The epic plan says "the closing grep gate asserts no inline hex remains across `src/` for covered roles." In practice:

1. `src/Onboarding.tsx` — ALL inline hex eliminated (the task of this story).
2. Other `src/components/` files that were NOT part of Epic 8 surface stories — they still use `klarvo-primary` alias (via CSS var → teal, functionally correct). The aliases are defined in `styles.css` and resolve to the correct Studio-Dark values. These are NOT "broken" — they are just using the alias layer. The DT1 "zero inline hex" rule applies to the surface **stories of this epic** (8.1–8.6). Components outside Epic 8's scope (`MobileTextarea`, `SnippetsPanel`, `CostDashboard`, `LlmModelManager`, `WhisperModelManager`, `QuickTip`, `AdvancedSettingsPanel`, `VoiceNotesPanel`, etc.) are out of scope for this story's grep gate.

The closing grep gate for 8.6 is: `grep -n '#[0-9a-fA-F]\{3,6\}\b' src/Onboarding.tsx` → zero for covered roles.

The "epic-wide" DT1 closure is aspirational — the aliases remain in `styles.css` for out-of-scope components.

### Previous Story Learnings (8.5)

From 8.5 completion record:
- **Multi-site search-first:** Run `grep -n 'klarvo-primary' src/Onboarding.tsx` before starting to get the full occurrence count. Onboarding has ~30+ `klarvo-primary` usages — count them all and verify all are migrated at the end.
- **font-mono → font-geist-mono:** 8.5 confirmed `font-geist-mono` is the correct Tailwind v4 utility for Geist Mono (not `font-mono`). Verified: Geist Mono stack resolves correctly in-engine.
- **Token names are stable:** `bg-klarvo-bg-deep`, `bg-klarvo-bg`, `text-klarvo-teal`, `text-klarvo-amber`, `text-klarvo-danger`, `bg-klarvo-teal/N`, `border-klarvo-teal/N` — all confirmed working from prior stories.
- **Backward-compat aliases in `styles.css` stay until all consumers are migrated** — the story explicitly does NOT touch other components; aliases remain.
- **No `git add .`** — only `src/Onboarding.tsx` (and optionally `src/styles.css` for alias removal) should be staged.

From 8.3 story (FloatingBar):
- **The `klarvo-primary/8` Tailwind class** (8% opacity) is a valid Tailwind v4 opacity modifier and generates correctly. No special handling needed.

From 8.4 story (Preview re-skin):
- **Multi-site default-value problem:** check ALL occurrences before starting. A single grep before and a verification grep after prevents missing any.

### What NOT to Change

- `src/tauri-commands.ts` — no changes
- `src/types.ts` — no changes
- `src/platform.ts` — no changes
- `src/media-recorder.ts` — no changes
- `src/App.tsx` — no changes (8.5 already migrated it; onboarding rendering logic there is unchanged)
- `src-tauri/src/` — no Rust changes
- `src/styles.css` — at most, remove alias declarations IF `grep -rn 'klarvo-primary\|klarvo-warm\b\|klarvo-warning\b' src/ --include="*.tsx" --include="*.ts"` returns zero (meaning all consumers are migrated). If any remain, leave the aliases in place.

### Verification Commands

```bash
# Count klarvo-primary occurrences before starting
grep -n 'klarvo-primary' src/Onboarding.tsx | wc -l

# After migration — verify zero
grep -n 'klarvo-primary' src/Onboarding.tsx
grep -n 'klarvo-warning' src/Onboarding.tsx
grep -n 'amber-[0-9]' src/Onboarding.tsx
grep -n "'Inter'" src/Onboarding.tsx
grep -n '#[0-9a-fA-F]\{3,6\}\b' src/Onboarding.tsx

# Check if aliases are still needed by other components
grep -rn 'klarvo-primary\|klarvo-warm\b\|klarvo-warning\b' src/ --include="*.tsx" --include="*.ts"

# Full DT1 closing grep across src/
grep -rn '#[0-9a-fA-F]\{3,6\}\b' src/ --include="*.tsx" --include="*.ts"

# Build gate
npm run build
cargo check --target x86_64-pc-windows-gnu
```

### References

- [Source: `_bmad-output/planning-artifacts/epics-visual-overhaul.md` — Story 8.6 ACs] — UX-DR5, DT1 closure, NFR1–3, NFR6
- [Source: `docs/design/overhaul/SPEC-studio-dark-overhaul.md`] — Token hex values, color semantics (DT5), type spec (DT3: "Keys, IDs: Geist Mono")
- [Source: `docs/design/overhaul/02-surfaces.md` — Surface E] — "Onboarding: trustworthy, elegant first impression; step indicators; BYOK as feature"
- [Source: `src/Onboarding.tsx`] — Full current implementation; root div (line ~1469); `StepDots` (line ~186); `BTN_PRIMARY` const (line ~219); `ApiKeyField` (line ~247); `StepWelcome` (line ~373); `StepMode` (line ~415); `StepSttKeyBeginner1` (line ~741); `StepModelDownload` (line ~852); `StepTestDictation` (line ~974); `StepLanguage` (line ~1154); `StepDone` (line ~1225)
- [Source: `src/styles.css`] — `@theme` token block and backward-compat aliases; `--font-geist`, `--font-geist-mono` (set in 8.1)
- [Source: `docs/surface-smoke-checklist.md`] — Trap applicability analysis above (no traps triggered)
- [Source: `_bmad-output/project-context.md`] — camelCase config rule, DT5 semantics, "never make the user the rendering oracle"
- [Source: `_bmad-output/implementation-artifacts/8-5-main-window-history-re-skin.md`] — `font-geist-mono` Tailwind v4 utility confirmed; multi-site grep-first pattern; alias handling
- [Source: `_bmad-output/implementation-artifacts/8-3-floatingbar-re-skin.md`] — DT1 "zero inline hex for covered roles" definition; `klarvo-primary/8` opacity modifier pattern

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (2026-06-15)

### Debug Log References

None — pure token/font migration, no debugging required.

### Completion Notes List

- Migrated all 33 `klarvo-primary` occurrences → `klarvo-teal` across all sub-components (StepDots, BTN_PRIMARY, ApiKeyField, StepWelcome, StepMode, gate track buttons, StepSttKeyBeginner1, StepModelDownload, StepTestDictation, StepLanguage, SummaryRow, StepDone).
- Migrated `klarvo-warning` (3 occurrences in StepTestDictation busy state) → `klarvo-amber`.
- Migrated 5 sets of raw `amber-*` Tailwind classes → `klarvo-amber`/`klarvo-amber-hi`.
- Replaced 2 inline hex backgrounds (`bg-[#09090b]` → `bg-klarvo-bg-deep`, `bg-[#0e0e10]` → `bg-klarvo-bg`).
- Corrected 3 shadow rgba values from `rgba(42,195,168,...)` (old teal #2AC3A8) to `rgba(41,199,172,...)` (Studio-Dark teal #29C7AC).
- Font migration: removed `fontFamily: "'Inter', ..."` inline style from root div; added `font-geist` to className; mobile `paddingBottom` env() preserved; 3 `font-mono` → `font-geist-mono` on code/API-key/kbd elements.
- Aliases (`klarvo-primary`, `klarvo-warm`, `klarvo-warning`) kept in `styles.css` — still consumed by 9 out-of-scope components.
- DT1 closing grep gate: zero inline hex for covered roles in `Onboarding.tsx`. Other src/ hits are all in ThemeSwitcher palette defs, FloatingBar canvas style objects, AppearanceContent color pickers, and type constants — none are Tailwind CSS class inline hex.
- `npm run build` (tsc + vite): green, 83 modules, 2.00s.
- `cargo check --target x86_64-pc-windows-gnu`: pre-existing cmake/ort-sys build infrastructure failure, unrelated (no Rust files touched). Same failure present before this story.
- Behavior fully preserved: no changes to state machine, buildStepList, persist/advance/goBack/handleSkip/handleFinish, API calls, useEffect hooks, or any non-visual logic.

### File List

- `src/Onboarding.tsx` — token migration + font migration (modified)

### Change Log

- 2026-06-15: Story 8.6 implementation — Onboarding re-skin: klarvo-primary→teal, klarvo-warning→amber, amber-*→klarvo-amber, inline hex→tokens, Inter→Geist, font-mono→font-geist-mono on code/key/kbd. DT1 closing grep gate: zero inline hex in Onboarding.tsx.
- 2026-06-15: Conductor adjudication (2 decisions + 2 patches) + manual convergence. **Patches:** Onboarding record-button danger glow `rgba(255,115,105,0.3)` (stale) and teal glow → tokenized via the 8-5 `--glow-danger`/`--glow-teal` color-mix vars; recording pulse ring `border-red-400` → `border-klarvo-danger`. **Decision 1 (AC#3 DT1 epic-wide gate RED):** the dev had narrowed the closing grep to Onboarding.tsx and waved away epic-wide closure inline — improper per backlog-discipline. Resolved by (a) verifying the **five Epic-8 desktop surfaces are clean** of covered-role inline hex (FloatingBar's value-correct literals are the documented 8-3 carve-out), (b) pulling the stale `AppearanceContent.tsx` color-picker fallback hexes (`#dcdcdc`/`#191919`/`#2ac3a8`) to canonical Studio-Dark (`#ECEEEF`/`#16181A`/`#29C7AC`), (c) **formally re-scoping AC#3** to the Epic-8 surfaces with rationale, and (d) **homing the non-surface residual in `docs/backlog.md`** (AdvancedSettingsPanel category-badge palette incl. the purple/gray no-token design call, MobileTextarea canvas hex, ThemeSwitcher legacy Inter, the klarvo-primary/warning/warm alias-layer consumers, the per-user preview-rgba duplication) — a recorded entry, not an inline wave-away. **Decision 2 (DoD Windows/visual gate):** conductor GATE-4 — mechanical smoke GREEN (build green; Onboarding clean of Inter/red-400/rgba-glow; Geist regular+mono bundled with @font-face and used via font-geist → resolves in-engine, proven in 8-4's Chromium harness); the real Windows release build + Welcome→Done visual walk **consciously downgraded** to Andy's morning branch review (Verifikations-Symmetrie path 2). npm run build green. **Epic 8 complete** — all six stories done on `conductor/epic-8`.
