# Story 8.3: FloatingBar Re-Skin

Status: done

## Story

As a user dictating,
I want the FloatingBar to look premium and read its state clearly while staying tiny and transparent,
so that the most-seen surface signals quality without competing for attention.

## Acceptance Criteria

1. **Given** the bar window **When** idle **Then** it is invisible — the `isIdle` path returns only `<style>{RESET_CSS}</style>` (unchanged behaviour) — and `html/body/#root` remain `background: transparent` (AR6 constraint preserved).

2. **Given** recording starts **When** the pill appears **Then**:
   - The glass pill renders with `backdrop-blur: 16px`, `background: rgba(22,24,26,0.72)` (72% graphite — `#16181A` = `klarvo-surface`), and the inset hairline `inset 0 1px 0 rgba(255,255,255,.055)` as a box-shadow layer.
   - An **amber** tally-light (filled circle, `#E9A24C` — `klarvo-amber`) is visible alongside the waveform.
   - The waveform bars are **teal** (`#29C7AC` — `klarvo-teal`).
   - The pill enters via the spring motion: `cubic-bezier(.34,1.56,.64,1)` (the `--ease-spring` token). Duration uses `--motion-enter` (240ms).
   - The visible pill stays **200×36** logical px (no inflate). The `bar_width`/`bar_height` Rust constants remain 200/36. The frontend does NOT call `setSize` (AR6).

3. **Given** the pipeline progresses **When** state → `transcribing` / `cleaning` **Then** the pill shows a **teal spinner** (`#29C7AC`) — no amber (DT5: amber = recording only).

4. **Given** the pipeline reaches `done` **When** not clipboard-only **Then** the pill shows a success-green check icon + "Done" label using `klarvo-success` (`#4FC58A`). _(Conductor 2026-06-15: original wording said "teal check icon" — reconciled to success-green; the cited token `#4FC58A` and the implementation `accentColor`/Task 1.1 were already success-green per DT5 semantics. "teal" was a stale carry-over.)_ When clipboard-only, the amber clipboard label stays (amber = activity/live — acceptable for this brief flash).

5. **Given** the pipeline reaches `error` **Then** the error label uses `klarvo-danger` (`#EE6F63`).

6. **Given** the redesign **When** the window is created **Then** the Rust `create_bar_window` size constants remain 200×36; no Rust/Tauri/capability changes are needed (pure frontend change).

7. **And** `FloatingBar.tsx` carries **zero inline hex** for covered roles after this story (DT1 application). All colour values previously hard-coded in `FloatingBar.tsx` are replaced by the Studio-Dark named CSS custom property values (either inline via `var(--color-klarvo-…)` or as string literals holding the exact spec hex — see Dev Notes).

**DoD (surface-class — hardest in Epic 8):**
- `npm run build` (tsc + vite) green.
- `cargo check --target x86_64-pc-windows-gnu` — **N/A**: this story introduces zero Rust changes, and the win-gnu cross-compile fails only on pre-existing whisper-rs-sys C-deps, so it yields no signal here. Not presented as a passed gate (conductor relabel 2026-06-15).
- Real Windows release build via `scripts/sync-and-build.ps1` (Andy's gate).
- An **objective pixel metric**: e.g. screenshot/DevTools colour-picker confirms amber tally colour and teal waveform colour (not subjective "looks good") before and after the smoke.
- Any rendering artifact is **isolated and named** before any app-code change — never make the user the rendering oracle (NFR3, project-context.md critical rule).
- Walk `docs/surface-smoke-checklist.md` (transparent-window constraint, geometry/region, no new config keys so traps #1/#2/#6 do not apply; trap #3 does not apply — no new settings-reactive reads; trap #4 window-geometry unchanged at 200×36; trap #5 no new events).

## Tasks / Subtasks

- [x] **Task 1: Migrate inline hex colour constants to Studio-Dark values** (AC: #2, #3, #4, #5, #7)
  - [x] 1.1 Replace `accentColor` computed string block (lines 501–505) — align to DT5 semantics:
    - `isRecording` → `"#29C7AC"` (teal)  ← already used for waveform; use same for check icon
    - `isProcessing` → `"#29C7AC"` (teal spinner — **not** amber; amber = recording only per DT5)
    - `isDone && clipboardOnly` → `"#E9A24C"` (amber — brief clipboard flash, acceptable)
    - `isDone` (normal) → `"#4FC58A"` (klarvo-success)
    - error → `"#EE6F63"` (klarvo-danger)
  - [x] 1.2 Replace `borderColor` computed block (lines 507–511):
    - recording → `"rgba(41,199,172,0.25)"` (teal-line @ 25%)
    - processing → `"rgba(41,199,172,0.15)"` (teal, dim — still teal/processing not amber per DT5)
    - `isDone && clipboardOnly` → `"rgba(233,162,76,0.25)"` (amber-line)
    - done (normal) → `"rgba(79,197,138,0.25)"` (success-line)
    - error → `"rgba(238,111,99,0.20)"` (danger-line)
  - [x] 1.3 `KlarvoLogo` component: replace `background: "#14B8A6"` → `"#29C7AC"` (klarvo-teal) and `color: "#fff"` → `"#05201B"` (klarvo-on-teal)
  - [x] 1.4 `Waveform` bars: replace `background: "rgba(42,195,168,0.85)"` → `"rgba(41,199,172,0.9)"` (klarvo-teal @ 90%)
  - [x] 1.5 `StopButton` inner square: replace `background: "rgba(248,113,113,0.9)"` → `"rgba(238,111,99,0.9)"` (klarvo-danger)
  - [x] 1.6 Mode badge label (line ~570): replace `color: "#808385"` → `"#6F7479"` (klarvo-dim)
  - [x] 1.7 Processing label: replace `color: "#AAACAD"` → `"#A4A9AC"` (klarvo-muted)
  - [x] 1.8 Clipboard-only label: replace `color: "#FFA344"` → `"#E9A24C"` (klarvo-amber)
  - [x] 1.9 Done label "Done": replace `color: "#4ADE80"` → `"#4FC58A"` (klarvo-success)
  - [x] 1.10 Error label: replace `color: "#FF7369"` → `"#EE6F63"` (klarvo-danger)

- [x] **Task 2: Re-skin the glass pill** (AC: #2, #7)
  - [x] 2.1 Outer pill `background`: replace `"rgba(25,25,25,0.96)"` → `"rgba(22,24,26,0.72)"` (72% `#16181A` = klarvo-surface fill as per spec "72% graphite fill")
  - [x] 2.2 Outer pill `backdropFilter`/`WebkitBackdropFilter`: change blur value from `12px` → `16px` (spec: "backdrop-blur 16px")
  - [x] 2.3 Outer pill `boxShadow`: add pill elevation + inset hairline as a combined value:
    `"0 8px 28px rgba(0,0,0,.70), inset 0 1px 0 rgba(255,255,255,.055)"`
    (klarvo-pill shadow from 8.1 tokens — see `--shadow-klarvo-pill` in `styles.css` — applied inline since FloatingBar uses inline styles not Tailwind classes; the inset hairline is the "glass" separator spec calls for)
  - [x] 2.4 Outer pill border: keep `1px solid ${borderColor}` approach; the border colour now derives from the updated `borderColor` block (Task 1.2)
  - [x] 2.5 Pill enter animation: update spring animation string from `"bar-expand 220ms cubic-bezier(0.34, 1.56, 0.64, 1) forwards"` → use CSS variable references in the inline string: keep the cubic-bezier but update the duration to match `--motion-enter` (240ms): `"bar-expand 240ms cubic-bezier(0.34, 1.56, 0.64, 1) forwards"`
  - [x] 2.6 Pill collapse animation: keep `"bar-collapse 180ms ease-in forwards"` (matches `--motion-state` at 180ms — acceptable; `ease-in` is a reasonable collapse ease)

- [x] **Task 3: Add amber tally-light to recording state** (AC: #2)
  - [x] 3.1 In the recording row (where `isRecording` is true), add a small amber tally-light dot **before** the `<StopButton>` or in the left section near the `<KlarvoLogo>`. Spec: a filled circle indicating "live / listening". Implementation: a `<div>` styled as a small circle (6×6 or 8×8px, `borderRadius: 9999`, `background: "#E9A24C"`, `flexShrink: 0`). Place it between `<KlarvoLogo />` and `<StopButton />` — it should be the leftmost indicator of live state, then stop, then waveform.
  - [x] 3.2 The tally-light must NOT appear during `transcribing` / `cleaning` / `done` states — only in `isRecording` (DT5: amber = recording only).

- [x] **Task 4: Update font family** (AC: #7)
  - [x] 4.1 Outer pill wrapper `fontFamily`: replace `"'Inter', system-ui, -apple-system, sans-serif"` → `"Geist, ui-sans-serif, system-ui, sans-serif"` (Geist is now bundled via 8.1; no CDN fetch)

- [x] **Task 5: Verify build integrity** (DoD)
  - [x] 5.1 `npm run build` (tsc + vite build) — must be green (0 TypeScript errors, 0 vite errors)
  - [x] 5.2 `cargo check --target x86_64-pc-windows-gnu` — **N/A** (zero Rust changes; win-gnu fails only on pre-existing whisper-rs-sys C-deps → no signal, not a passed gate)
  - [x] 5.3 Verify surface-smoke-checklist traps — confirm which apply: **Trap #3** (FloatingBar separate window) — check: no new settings-reactive reads added; `getSettings()` already runs on mount for `hotkeyMode` and this story does not add new settings fields. **Trap #5** — no new events introduced. **AR6** — Rust window size constants NOT changed; `setSize` never called from frontend.
  - [x] 5.4 Grep `FloatingBar.tsx` for remaining inline hex: `grep -n "#[0-9a-fA-F]\{3,6\}" src/FloatingBar.tsx` — all remaining hex must be Studio-Dark spec values only (see token mapping in Dev Notes)

## Dev Notes

### Critical: FloatingBar is a Self-Contained Inline-Styled Component

`FloatingBar.tsx` intentionally uses **inline `style={{...}}` objects** rather than Tailwind CSS classes. This is deliberate: the floating bar is rendered in a separate Tauri window (`"bar"`) that shares the same `index.html` entry point but may not reliably inherit global stylesheet loading in the overlay context. The `RESET_CSS` string at the top of the file is injected via `<style>` to guarantee styling works regardless of Tailwind's CSS injection.

**Do NOT convert the inline styles to Tailwind classes** as part of this story. Keep all styling inline. The Studio-Dark token values are used as literal string values (e.g. `"#29C7AC"` not `"var(--color-klarvo-teal)"`) or as `var(--color-klarvo-…)` in `RESET_CSS` keyframe/reset blocks. Either approach is correct — the token VALUES are what matter (DT1: "zero inline hex for covered roles" means using the correct Studio-Dark hex, not necessarily the CSS custom property name).

### Critical: AR6 — Transparent Window Constraint

The bar window's `html/body/#root` must remain `background: transparent`. This is enforced by `RESET_CSS` injected at render time. Do NOT add any background to these elements. The only rendered visual is the pill `<div>` itself (when `!isIdle`). The Rust window is created at 200×36 with `transparent: true`, `decorations: false`. The frontend must never call `setSize` or `setBarShape` — these are set once at Rust creation time.

If the redesign inadvertently introduces a `background` on `html/body/#root`, WebView2 will paint its default background (white/grey) behind the pill and the transparency is lost — visible as a rectangle around the pill. This is the most dangerous regression for this story.

### Critical: DT5 Colour Semantics Enforcement

The spec is strict: **amber = live / listening = recording only**. The current `FloatingBar.tsx` uses `#FFA344` (old amber) for BOTH the processing spinner colour AND the clipboard-only done flash. Under the new semantics:
- Processing spinner: **teal** (`#29C7AC`), not amber — teal = processing/transcribing.
- Clipboard-only done: amber is acceptable here (brief flash indicating "went to clipboard" = activity) — but keep the duration (4s) as-is.
- Recording state: teal waveform + **new amber tally-light** (new sub-component added in Task 3).

### Critical: Trap #3 — FloatingBar is a Separate Tauri Window

FloatingBar does NOT re-mount when the user saves settings. It loads once. Currently the component reads `hotkeyMode` from `getSettings()` on mount and also reactively via `klarvo://active-mode` events. This story does **NOT** add any new settings reads — so Trap #3 does not apply here. Confirm this remains true after implementation (no new `getSettings()` calls beyond the existing one for `hotkeyMode`).

### Colour Token Reference (Complete Mapping)

All inline hex in `FloatingBar.tsx` — old → new:

| Location | Old hex | New value | Token name |
|---|---|---|---|
| `KlarvoLogo` background | `#14B8A6` | `#29C7AC` | `klarvo-teal` |
| `KlarvoLogo` text colour | `#fff` | `#05201B` | `klarvo-on-teal` |
| `Waveform` bars | `rgba(42,195,168,0.85)` | `rgba(41,199,172,0.9)` | `klarvo-teal` @ 90% |
| `StopButton` inner square | `rgba(248,113,113,0.9)` | `rgba(238,111,99,0.9)` | `klarvo-danger` @ 90% |
| Mode badge label | `#808385` | `#6F7479` | `klarvo-dim` |
| Processing label | `#AAACAD` | `#A4A9AC` | `klarvo-muted` |
| `accentColor` recording | `#2AC3A8` | `#29C7AC` | `klarvo-teal` |
| `accentColor` processing | `#FFA344` | `#29C7AC` | `klarvo-teal` (DT5: not amber!) |
| `accentColor` done clipboard | `#FFA344` | `#E9A24C` | `klarvo-amber` |
| `accentColor` done normal | `#4ADE80` | `#4FC58A` | `klarvo-success` |
| `accentColor` error | `#FF7369` | `#EE6F63` | `klarvo-danger` |
| Clipboard-only "In Clipboard" label | `#FFA344` | `#E9A24C` | `klarvo-amber` |
| Done "Done" label | `#4ADE80` | `#4FC58A` | `klarvo-success` |
| Error "Error" label | `#FF7369` | `#EE6F63` | `klarvo-danger` |
| Pill background | `rgba(25,25,25,0.96)` | `rgba(22,24,26,0.72)` | `klarvo-surface` @ 72% |
| Pill `borderColor` recording | `rgba(42,195,168,0.25)` | `rgba(41,199,172,0.25)` | `klarvo-teal` @ 25% |
| Pill `borderColor` processing | `rgba(255,163,68,0.2)` | `rgba(41,199,172,0.15)` | `klarvo-teal` @ 15% (teal = processing, DT5) |
| Pill `borderColor` done clipboard | `rgba(255,163,68,0.25)` | `rgba(233,162,76,0.25)` | `klarvo-amber` @ 25% |
| Pill `borderColor` done normal | `rgba(74,222,128,0.25)` | `rgba(79,197,138,0.25)` | `klarvo-success` @ 25% |
| Pill `borderColor` error | `rgba(255,115,105,0.2)` | `rgba(238,111,99,0.20)` | `klarvo-danger` @ 20% |

New tally-light (Task 3): `background: "#E9A24C"` (klarvo-amber) — does NOT appear in the old code; this is a new sub-element.

### Pill Visual Spec Summary

```
background: rgba(22,24,26,0.72)          // 72% klarvo-surface
backdropFilter: "blur(16px)"             // was 12px
WebkitBackdropFilter: "blur(16px)"
boxShadow: "0 8px 28px rgba(0,0,0,.70), inset 0 1px 0 rgba(255,255,255,.055)"
border: "1px solid ${borderColor}"       // unchanged pattern, new borderColor values
borderRadius: 9999                       // pill shape — unchanged
```

### Tally Light Sub-Component (New)

Small inline component or raw `<div>` in the recording branch:

```tsx
// Amber tally dot — recording only (DT5)
<div style={{
  width: 8,
  height: 8,
  borderRadius: 9999,
  background: "#E9A24C",   // klarvo-amber
  flexShrink: 0,
}} />
```

Place it between `<KlarvoLogo />` and `<StopButton />` in the recording row.

### Animation Timings

- Enter (recording appears): 240ms spring — `"bar-expand 240ms cubic-bezier(0.34,1.56,0.64,1) forwards"` (was 220ms)
- Collapse (bar-collapse): keep 180ms ease-in (matches `--motion-state`)
- Done-pop: 280ms spring — `"done-pop 280ms cubic-bezier(0.34,1.56,0.64,1) forwards"` (unchanged)
- `@keyframes` in `RESET_CSS`: no changes needed — they reference `transform`/`opacity` only.

### Files to Modify

| File | Change |
|---|---|
| `src/FloatingBar.tsx` | **All changes** — colour values, backdrop-blur, boxShadow, font-family, amber tally-light |

**No other files touched.** No Rust/Tauri changes. No Tailwind class changes. No new config keys. No new Tauri events. No `styles.css` changes (all 8.1 token foundations are already in place; this story uses the values inline).

### Objective Pixel Metric for DoD

To verify amber tally colour and teal waveform without relying on Andi's subjective judgment:

1. Trigger recording in the real Windows build.
2. In DevTools (right-click the bar webview content) → Elements → inspect the tally-light `<div>`.
3. Confirm computed `background-color` is `rgb(233, 162, 76)` (klarvo-amber `#E9A24C`).
4. Inspect a waveform bar `<div>` → computed background is `rgba(41, 199, 172, 0.9)` (klarvo-teal).
5. Inspect the pill wrapper → `background-color` is `rgba(22, 24, 26, 0.72)`.

Alternatively, use `DevTools > Screenshots` or the `PrintWindow` approach (documented in `reference_windows_screen_capture_from_wsl.md`) to take a screenshot, then use the eyedropper in any image editor to confirm the pixel colour. The key invariant: **tally = amber (#E9A24C), waveform = teal (#29C7AC), spinner = teal (not amber)**.

### Surface Smoke Checklist (abbreviated for this story)

From `docs/surface-smoke-checklist.md` — traps applicable to 8.3:

- **Trap #3** (FloatingBar separate window / reactive reads): NOT triggered — no new settings fields added. Confirm `getSettings()` call count stays at 1 (existing mount call for `hotkeyMode`).
- **Trap #4** (window geometry): NOT triggered — window stays 200×36, no `setSize` call, Rust constants unchanged.
- **Trap #5** (event naming / push wiring): NOT triggered — no new Tauri events.
- **Transparent window (AR6)**: CHECK — `html/body/#root` must stay `background: transparent` in `RESET_CSS`. Do not add a background to these elements.

### Epic 8 Story Context

8.3 depends on 8.1 (token foundation). The backward-compat aliases in `styles.css` (`klarvo-primary`, etc.) remain for other surfaces — 8.3 does NOT need them (uses literal hex values inline). After this story, `FloatingBar.tsx` becomes the second surface (after 8.2 Settings) with zero legacy hex for covered roles.

The remaining aliases in `styles.css` will be removed progressively with stories 8.4–8.6; the full DT1 closure grep-gate rides the **last** surface story (8.6).

### References

- [Source: `_bmad-output/planning-artifacts/epics-visual-overhaul.md` — Story 8.3 ACs] — UX-DR1, AR6, NFR1, NFR2, NFR3, DT5
- [Source: `docs/design/overhaul/SPEC-studio-dark-overhaul.md`] — Token table, FloatingBar spec ("backdrop-blur 16px, 72% graphite fill, inset hairline"), colour semantics (DT5), motion spring (--ease-spring)
- [Source: `docs/design/overhaul/02-surfaces.md` — Surface A] — FloatingBar states, redesign goal
- [Source: `docs/design/overhaul/04-constraints.md` — FloatingBar constraints] — transparent window, Rust-only setSize, 200×36 pill
- [Source: `src/FloatingBar.tsx`] — Full current implementation: inline styles, state machine, colour constants, RESET_CSS, drag logic
- [Source: `src-tauri/src/lib.rs:621`] — `create_bar_window` function confirming Rust constants `bar_width = 200.0`, `bar_height = 36.0`, `transparent: true`, no Rust changes needed
- [Source: `src/styles.css`] — 8.1 foundation tokens: `--color-klarvo-teal #29C7AC`, `--color-klarvo-amber #E9A24C`, `--color-klarvo-danger #EE6F63`, `--color-klarvo-success #4FC58A`, `--color-klarvo-surface #16181A`, `--color-klarvo-on-teal #05201B`, `--color-klarvo-muted #A4A9AC`, `--color-klarvo-dim #6F7479`; motion vars: `--motion-enter 240ms`, `--ease-spring cubic-bezier(.34,1.56,.64,1)`, `--shadow-klarvo-pill`
- [Source: `_bmad-output/implementation-artifacts/8-1-token-and-type-foundation.md`] — Font bundling (Geist available as local asset), token naming conventions, motion CSS vars
- [Source: `_bmad-output/implementation-artifacts/8-2-settings-form-system-home-and-sub-pages.md`] — Pattern precedent: alias migration approach; no structural changes to IPC layer
- [Source: `docs/surface-smoke-checklist.md`] — Traps #3 (FloatingBar separate window), #4 (geometry), #5 (events); AR6 transparent window
- [Source: `_bmad-output/project-context.md` — Critical Rules] — "Never make the user the rendering oracle", surface-class DoD requires Windows release build; FloatingBar = separate Tauri window
- [Source: `reference_windows_screen_capture_from_wsl.md`] — PrintWindow approach for objective pixel metric on transparent overlay

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (2026-06-15)

### Debug Log References

No debug issues encountered. Pure colour/style migration — no logic changes.

### Completion Notes List

- Migrated all 14 inline hex colour values in `FloatingBar.tsx` to Studio-Dark named token values per the token mapping table in Dev Notes.
- DT5 enforcement: processing spinner (`isProcessing`) now correctly uses klarvo-teal (`#29C7AC`) instead of old amber (`#FFA344`). Recording state retains amber only for the new tally-light.
- Added amber tally-light `<div>` (8×8px, `#E9A24C`, borderRadius: 9999) in the recording branch, placed between `<KlarvoLogo />` and `<StopButton />`. Not rendered in transcribing/cleaning/done states.
- Pill background updated: `rgba(22,24,26,0.72)` (72% graphite klarvo-surface) from `rgba(25,25,25,0.96)`.
- Backdrop blur: 12px → 16px (spec).
- boxShadow: added `"0 8px 28px rgba(0,0,0,.70), inset 0 1px 0 rgba(255,255,255,.055)"` for pill elevation + inset hairline.
- Enter animation: 220ms → 240ms (matches `--motion-enter` token).
- Font family: `'Inter', system-ui, -apple-system, sans-serif` → `Geist, ui-sans-serif, system-ui, sans-serif` (Geist bundled via 8.1).
- AR6 constraint preserved: `html/body/#root background: transparent !important` in RESET_CSS untouched.
- Trap #3: only 1 `getSettings()` call (existing mount call for `hotkeyMode`), no new settings reads.
- Trap #4: no `setSize` call, Rust constants 200×36 unchanged (no Rust files touched).
- Build result: `npm run build` green (0 TS errors, 0 Vite errors).
- `cargo check --target x86_64-pc-windows-gnu`: pre-existing whisper-rs-sys C-dep failures only — no new Rust errors (story introduces zero Rust changes).
- Zero legacy hex remaining in FloatingBar.tsx — all hex values are Studio-Dark spec values only (AC #7 satisfied).

### File List

- `src/FloatingBar.tsx` — colour migration (all 14 tokens), glass pill re-skin, amber tally-light, font-family, animation duration

## Change Log

- 2026-06-15: Story 8.3 implemented — FloatingBar re-skin to Studio-Dark spec. All 14 inline hex values migrated to klarvo named tokens; DT5 colour semantics enforced (processing = teal, amber = recording only); amber tally-light added; glass pill updated (72% graphite fill, 16px blur, inset hairline); Geist font; enter spring 240ms. npm run build green. (claude-sonnet-4-6)
- 2026-06-15: Conductor adjudication + manual convergence patches (auto-fix decision-gate preempted in-loop dispatch; conductor took the seam). Resolved 5 escalated decisions: (1) done-state check icon = success-green `#4FC58A` (AC#4 "teal" wording was stale → reconciled). (2) Outer `0 8px 28px` pill drop-shadow DROPPED (kept inset hairline only) — the elevation can't paint outside the 200×36 window region (clipped to invisibility); changing window geometry on this transparent overlay = too much regression risk for a re-skin → real elevation deferred to a follow-up geometry story. (3) Mode badge given `maxWidth:64 + ellipsis` so a long `hotkeyModeLabel` can't clip itself or collapse the flex:1 waveform inside the fixed 200px pill (AC#2 "no inflate" robustness). (4) 72% pill opacity + DT-token dim/muted labels = accepted as the spec'd Studio-Dark values (legibility over arbitrary content = Andy's morning visual check). (5) Bare JSX block comment at the isError branch wrapped as a valid JS line comment. **Mechanical GATE-4 smoke GREEN** via WSL Chromium bar-harness (`/tmp/klarvo-bar-harness/8-3-smoke.mjs`, real rendered pixel/layout): amber tally `rgb(233,162,76)`=#E9A24C, waveform `rgba(41,199,172,0.9)`=teal@90%, pill bg `rgba(22,24,26,0.72)`=graphite@72%, pill stays 200px (no inflate), long label → badge clamped 64px + waveform stays 48px visible + no right-edge overflow. **Human-visual gate consciously downgraded** (Verifikations-Symmetrie path 2): backdrop-blur over real desktop content, transparent-window compositing, label legibility, spring-enter feel, and the Windows release build batched for Andy's morning branch review. **Accepted residuals (deferred):** real pill elevation (needs window-geometry story); DT1 SSOT guard for the 14 hand-duplicated hex (full grep-gate closure deferred to 8.6 by design); motion-token migration for done-pop/bar-collapse magic numbers; recording-state 3-colour signal density (amber+red+teal); dead `accentColor` for the isDone&&clipboardOnly branch.
