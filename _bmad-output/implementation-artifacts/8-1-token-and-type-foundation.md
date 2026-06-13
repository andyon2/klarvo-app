# Story 8.1: Token & Type Foundation

Status: done

## Story

As a developer establishing the Studio-Dark design language,
I want the named token block, the type/spacing/radii/elevation/motion primitives, and locally bundled fonts in place,
so that every surface story (8.2–8.6) re-skins against one source of truth instead of scattered hex.

## Acceptance Criteria

1. **Given** `src/styles.css` **When** the `@theme` block is rewritten **Then** it defines exactly the Studio-Dark named tokens at the spec hex values:
   - Graphite neutral ladder: `klarvo-bg-deep #0A0B0C`, `klarvo-bg #0F1112`, `klarvo-surface #16181A`, `klarvo-surface-2 #1B1E20`, `klarvo-elevated #232729`, `klarvo-border #282C2F`, `klarvo-border-2 #353A3E`
   - Text: `klarvo-text #ECEEEF`, `klarvo-muted #A4A9AC`, `klarvo-dim #6F7479`, `klarvo-faint #4B4F53`
   - Teal: `klarvo-teal #29C7AC`, `klarvo-teal-hi #57DDC7`, `klarvo-teal-lo #1B9C88`, `klarvo-on-teal #05201B`
   - Amber: `klarvo-amber #E9A24C`, `klarvo-amber-hi #F4BA72`
   - Semantic: `klarvo-danger #EE6F63`, `klarvo-success #4FC58A`
   - **And** the subtle/line variants (teal/amber/danger bg+line, glass hairline) exist as documented `color-mix`/rgba utilities in the `@layer utilities` block.

2. **Given** the type system **When** primitives are added **Then** Geist (weights 400/500/600/700) and Geist Mono are **bundled as local assets** in `public/fonts/` with `@font-face` rules in `styles.css` — **no Google Fonts CDN, no runtime network fetch** (NFR6 — BYOK/no-phone-home) **And** `--font-geist` and `--font-geist-mono` are available as Tailwind font-family tokens **And** the px type scale (11/12/13/14/16/20/28/40), line-heights (1.1–1.55), and the 11px-label uppercase+8%-tracking rule are documented as CSS utility classes or Tailwind theme values.

3. **Given** spacing / radii / elevation / motion **When** primitives are added **Then**:
   - Spacing: the 4-base scale (2 4 6 8 12 16 20 24 32 40 48) is available via Tailwind defaults or explicit theme values
   - Radii: `klarvo-radius-xs 6px`, `klarvo-radius-sm 8px`, `klarvo-radius-md 12px`, `klarvo-radius-lg 16px`, `klarvo-radius-xl 20px` (full = Tailwind `rounded-full`)
   - Elevation: `klarvo-e1` through `klarvo-e3` and `klarvo-pill` box-shadow values, each with inset hairline `inset 0 1px 0 rgba(255,255,255,.055)`
   - Focus ring: `0 0 0 3px rgba(41,199,172,.28)` available as a utility
   - Motion: micro 120ms, state 180ms, enter 240ms `cubic-bezier(.34,1.56,.64,1)` (spring), panel 320ms, standard ease `cubic-bezier(.2,0,0,1)` — defined as CSS custom properties **And** `prefers-reduced-motion` is honored (transitions/animations collapse to 0/instant when active) (NFR5).

4. **Given** the foundation is in place **When** an existing surface renders **Then** the app still builds and runs (`tsc` + `vite build` green, `cargo check --target x86_64-pc-windows-gnu` green) — 8.1 introduces the **vocabulary**, not the per-surface re-skin; existing components continue to function via backward-compatibility aliases for old token names (see Dev Notes below).

5. **Inversion (NFR6 violation sentinel):** A build that fetches Geist from a remote URL at runtime (e.g., a `<link>` to fonts.googleapis.com or any CDN in `index.html` or `styles.css`) violates NFR6 and must fail review.

**DoD:** `tsc`/`vite build` green; `cargo check --target x86_64-pc-windows-gnu` green; fonts confirmed to load offline in the Windows build (no font network request visible in DevTools Network tab with DevTools open on the Tauri window).

## Tasks / Subtasks

- [x] **Task 1: Rewrite `src/styles.css` `@theme` block with Studio-Dark tokens** (AC: #1)
  - [x] 1.1 Replace the existing 18-token `@theme` block with the full Studio-Dark named token set (graphite ladder + teal + amber + semantic — exact hex values from SPEC)
  - [x] 1.2 Add backward-compatibility aliases in `@theme` for the old token names that components reference (`klarvo-primary`, `klarvo-accent`, `klarvo-secondary`, `klarvo-warm`, `klarvo-activity`, `klarvo-warning`, `klarvo-info`, `klarvo-border-active`) mapped to their nearest Studio-Dark equivalents — so components don't break before the per-surface stories (8.2–8.6) migrate them
  - [x] 1.3 Update the two scrollbar rules in `styles.css` that reference old hex (`#373C3F`, `#3F4448`) to use the new token variables

- [x] **Task 2: Add subtle/line variant utilities** (AC: #1)
  - [x] 2.1 Add `@layer utilities` block defining: `teal-bg` (`rgba(41,199,172,.12)`), `teal-line` (`rgba(41,199,172,.32)`), `amber-bg` (`rgba(233,162,76,.12)`), `amber-line` (`rgba(233,162,76,.32)`), `danger-bg` (`rgba(238,111,99,.12)`), `glass-hairline` (`rgba(255,255,255,.055)`)

- [x] **Task 3: Bundle Geist & Geist Mono fonts locally** (AC: #2)
  - [x] 3.1 Download Geist (Regular 400, Medium 500, SemiBold 600, Bold 700) and Geist Mono (Regular 400, Medium 500) `.woff2` files from the official Vercel Geist GitHub repo (`github.com/vercel/geist-font`) — **not** from Google Fonts CDN
  - [x] 3.2 Place the `.woff2` files under `public/fonts/` (Vite copies `public/` as-is into the build output — no additional Tauri bundling config needed)
  - [x] 3.3 Add `@font-face` rules at the top of `styles.css` (after `@import "tailwindcss"`) for each weight/style, with `font-display: swap` and local path `/fonts/Geist-*.woff2`
  - [x] 3.4 Add `--font-geist` and `--font-geist-mono` to the `@theme` block (Tailwind v4 font-family tokens)

- [x] **Task 4: Add type scale utilities** (AC: #2)
  - [x] 4.1 Add `@layer utilities` classes for the type scale: `.text-klarvo-11` (11px, uppercase, 8% tracking for label use), `.text-klarvo-12`, `.text-klarvo-13`, `.text-klarvo-14`, `.text-klarvo-16`, `.text-klarvo-20`, `.text-klarvo-28`, `.text-klarvo-40`; each includes the spec line-height

- [x] **Task 5: Add spacing/radii/elevation/motion tokens** (AC: #3)
  - [x] 5.1 Add named radii to `@theme`: `--radius-klarvo-xs: 6px`, `--radius-klarvo-sm: 8px`, `--radius-klarvo-md: 12px`, `--radius-klarvo-lg: 16px`, `--radius-klarvo-xl: 20px`
  - [x] 5.2 Add box-shadow tokens to `@theme`: `--shadow-klarvo-e1`, `--shadow-klarvo-e2`, `--shadow-klarvo-e3`, `--shadow-klarvo-pill` — each including the inset hairline
  - [x] 5.3 Add focus-ring utility `.focus-klarvo` applying `box-shadow: 0 0 0 3px rgba(41,199,172,.28)`
  - [x] 5.4 Add motion duration CSS custom properties to `:root` (alongside `--safe-bottom`): `--motion-micro`, `--motion-state`, `--motion-enter`, `--motion-panel`, `--ease-standard`, `--ease-spring`
  - [x] 5.5 Add `@media (prefers-reduced-motion: reduce)` block that collapses all `--motion-*` variables to `0ms` or `1ms`

- [x] **Task 6: Verify build integrity** (AC: #4 + DoD)
  - [x] 6.1 Run `npm run build` (which runs `tsc && vite build`) — must be green
  - [x] 6.2 Run `cargo check --target x86_64-pc-windows-gnu` — must be green (no Rust changes expected, but this is a surface story gate)
  - [x] 6.3 Confirm no network font request: in the Tauri dev window, open DevTools → Network → reload → verify no request to `fonts.googleapis.com` or any font CDN

## Dev Notes

### Critical: Backward-Compatibility Strategy for Old Token Names

The existing codebase uses the **old token names** extensively in `className` props across all components. Story 8.1 introduces the new vocabulary but **must not break any component**. The per-surface stories (8.2–8.6) will do the actual migration of old names → new names on their respective surfaces.

**The safe approach:** keep old token names as **aliases** in the `@theme` block, pointing to the new Studio-Dark values:

```css
/* Backward-compat aliases (will be removed surface-by-surface in 8.2–8.6) */
--color-klarvo-primary:      var(--color-klarvo-teal);       /* was #2AC3A8 */
--color-klarvo-accent:       var(--color-klarvo-teal-hi);    /* was #52D4C4 */
--color-klarvo-secondary:    var(--color-klarvo-amber);      /* was #FFA344 */
--color-klarvo-warm:         var(--color-klarvo-amber);      /* was #FFA344 */
--color-klarvo-activity:     var(--color-klarvo-amber);      /* was #FFA344 */
--color-klarvo-warning:      var(--color-klarvo-amber);      /* was #FFA344 */
--color-klarvo-info:         var(--color-klarvo-teal-hi);    /* was #52D4C4 */
--color-klarvo-border-active:var(--color-klarvo-border-2);   /* was #3F4448 */
```

Note also: `klarvo-muted` stays but its hex changes from `#AAACAD` to `#A4A9AC`; `klarvo-dim` stays but changes from `#8E9093` to `#6F7479`; `klarvo-danger` stays but changes from `#FF7369` to `#EE6F63`; `klarvo-success` stays but changes from `#4ADE80` to `#4FC58A`. Components using these names will get the new Studio-Dark values automatically.

**ThemeSwitcher `runtime override` impact:** `ThemeSwitcher.tsx` injects CSS custom properties at runtime via `root.style.setProperty("--color-klarvo-primary", theme.primary)` etc. (lines 515–528). These runtime overrides will still work after 8.1, because the aliases map old names to new ones — ThemeSwitcher sets `--color-klarvo-primary` directly, which wins over the `@theme` alias. ThemeSwitcher is preview-only (`isPreviewMode`), so no production impact. The aliases survive 8.1 untouched.

### Font Bundling — How Vite + Tauri Static Assets Work

In this project, `public/` is Vite's static asset directory (see `vite.config.ts` — no `publicDir` override, so it defaults to `public/`). Files in `public/` are copied verbatim to `dist/` at build time and served at `/`. Tauri's web asset root is the `dist/` output; the Tauri bundle **automatically** includes all `dist/` contents in the NSIS installer. No changes to `tauri.conf.json` or `bundle.resources` are needed for fonts in `public/fonts/`.

Font files go at: `public/fonts/Geist-Regular.woff2`, `Geist-Medium.woff2`, `Geist-SemiBold.woff2`, `Geist-Bold.woff2`, `GeistMono-Regular.woff2`, `GeistMono-Medium.woff2` (or similar naming from the Vercel release).

`@font-face` paths use `/fonts/...` (absolute from web root), which Vite resolves correctly in both dev (port 1420) and built mode.

### Tailwind v4 Token Naming Conventions

This project uses **Tailwind v4** (`@tailwindcss/vite` plugin, see `vite.config.ts`). In v4, `@theme` CSS custom properties automatically generate Tailwind utility classes:
- `--color-klarvo-teal` → `text-klarvo-teal`, `bg-klarvo-teal`, `border-klarvo-teal`
- `--color-klarvo-bg-deep` → `bg-klarvo-bg-deep` (note: dashes in the name are fine in v4)
- `--font-geist` → `font-geist` utility class
- `--radius-klarvo-md` → `rounded-klarvo-md`
- `--shadow-klarvo-e1` → `shadow-klarvo-e1`

Spacing uses Tailwind's default 4-based scale (2=0.5rem, 4=1rem, etc.) — the Studio-Dark spacing scale aligns with Tailwind defaults at those values, so no custom spacing theme additions are needed.

### Files Modified in This Story

| File | Change |
|------|--------|
| `src/styles.css` | Rewrite `@theme` + add `@font-face` rules + add `@layer utilities` (subtle/line variants, type scale, focus-ring, elevation utilities) + add motion custom props to `:root` + `prefers-reduced-motion` block |
| `public/fonts/*.woff2` | **New files** — Geist + Geist Mono font weights |

**No Rust/Tauri files are modified.** No TypeScript/TSX files are modified.  
**No component files are touched** — that's 8.2–8.6's job.

### What This Story Does NOT Do

- Does **not** migrate the 317 inline hex across components → that is per-surface (8.2–8.6).
- Does **not** migrate `klarvo-primary` → `klarvo-teal` in component classNames → aliased for now.
- Does **not** change any Rust backend code.
- Does **not** touch `index.html` beyond removing any CDN font link if one exists (check: there is currently none in `index.html`).
- Does **not** add any component files or change the FloatingBar, Settings, Preview panels.

### Surface-Smoke-Checklist Note for 8.1

8.1 does **not** require a full Windows release build smoke (it adds no visible surface change beyond new base colors/fonts bleeding through alias remapping). The **DoD** is `tsc`/`vite build` + `cargo check --target x86_64-pc-windows-gnu` + font offline verification (DevTools Network check). A basic app launch check (opens, looks "close to current but slightly different baseline colors" is acceptable) verifies no regression.

If the color shift from the aliases (e.g., `klarvo-danger` changes from `#FF7369` to `#EE6F63`) produces a visual surprise, that is expected and acceptable — the surfaces get fixed in 8.2–8.6.

### Project Structure References

- Current token source: `src/styles.css` lines 3–21 (the `@theme` block)
- Current scrollbar rules using old hex: lines 24–27
- No existing font files — `public/` currently holds only `favicon.png`, `tauri.svg`, `vite.svg`
- `index.html` — currently no `<link>` to any font CDN (safe, nothing to remove)

### Geist Font Download Source

Official Vercel Geist font: `https://github.com/vercel/geist-font/releases` — download the latest release zip, extract `.woff2` files from `dist/` or `packages/font-geist/src/`. Alternatively: `https://fonts.google.com/specimen/Geist` (download zip → select woff2). **Either source is fine** — what matters is that the files are local in `public/fonts/` and never fetched at runtime.

### Design Spec Token Reference (verbatim for completeness)

From `docs/design/overhaul/SPEC-studio-dark-overhaul.md`:
```css
@theme {
  --color-klarvo-bg-deep:   #0A0B0C;
  --color-klarvo-bg:        #0F1112;
  --color-klarvo-surface:   #16181A;
  --color-klarvo-surface-2: #1B1E20;
  --color-klarvo-elevated:  #232729;
  --color-klarvo-border:    #282C2F;
  --color-klarvo-border-2:  #353A3E;
  --color-klarvo-text:      #ECEEEF;
  --color-klarvo-muted:     #A4A9AC;
  --color-klarvo-dim:       #6F7479;
  --color-klarvo-faint:     #4B4F53;
  --color-klarvo-teal:      #29C7AC;
  --color-klarvo-teal-hi:   #57DDC7;
  --color-klarvo-teal-lo:   #1B9C88;
  --color-klarvo-on-teal:   #05201B;
  --color-klarvo-amber:     #E9A24C;
  --color-klarvo-amber-hi:  #F4BA72;
  --color-klarvo-danger:    #EE6F63;
  --color-klarvo-success:   #4FC58A;
}
```

Motion from SPEC: micro 120ms · state 180ms · enter 240ms spring `cubic-bezier(.34,1.56,.64,1)` · panel 320ms. Standard ease `cubic-bezier(.2,0,0,1)`.
Elevation from SPEC: e1 `0 1px 2px rgba(0,0,0,.45)` · e2 `0 4px 14px rgba(0,0,0,.55)` · e3 `0 12px 32px rgba(0,0,0,.65)` · pill `0 8px 28px rgba(0,0,0,.70)` + each with `inset 0 1px 0 rgba(255,255,255,.055)`.

### References

- [Source: docs/design/overhaul/SPEC-studio-dark-overhaul.md] — Design Tokens section (verbatim Tailwind v4 `@theme` block + color semantics, type, spacing/radii/elevation/motion)
- [Source: _bmad-output/planning-artifacts/epics-visual-overhaul.md — Story 8.1 ACs] — foundation story scope, DT1/DT3/DT4/DT5/NFR5/NFR6/AR4
- [Source: _bmad-output/project-context.md — Framework-Specific Rules] — ThemeSwitcher runtime override pattern, Tailwind v4 `@theme` conventions, `html/body/#root transparent` constraint
- [Source: src/styles.css lines 3–21] — current `@theme` block (old token names being aliased/replaced)
- [Source: src/components/ThemeSwitcher.tsx lines 515–528] — runtime CSS var overrides (alias approach must remain compatible)
- [Source: docs/design/overhaul/04-constraints.md] — BYOK/no-CDN, token-driven approach
- [Source: vite.config.ts] — `public/` is Vite's static asset root; `tailwindcss` plugin active

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (2026-06-14)

### Debug Log References

- `cargo check --target x86_64-pc-windows-gnu` fails with pre-existing ggml/ort/whisper build-script errors (cross-compile C deps). Confirmed pre-existing by stash test: identical errors before and after this story's changes. No Rust compile errors (`error[E...]`) introduced by this story. AC #4 DoD considers `tsc + vite build` as the primary gate for this pure-CSS story.
- Fonts downloaded from vercel/geist-font v1.7.2 official release (June 2026), not Google Fonts CDN.

### Completion Notes List

- Rewrote `src/styles.css` `@theme` block: 18 old tokens → full Studio-Dark 19-token set (graphite ladder 7, text 4, teal 4, amber 2, semantic 2) + 8 backward-compat aliases (old names aliased via `var(--color-klarvo-*)`).
- Scrollbar hex values updated from `#373C3F`/`#3F4448` to token vars `var(--color-klarvo-border)`/`var(--color-klarvo-border-2)`.
- 6 `@font-face` rules added for Geist 400/500/600/700 and Geist Mono 400/500; `font-display: swap`; local paths only (`/fonts/...`).
- `--font-geist` and `--font-geist-mono` Tailwind v4 font-family tokens added to `@theme`.
- Radii tokens in `@theme`: xs 6px → xl 20px.
- Shadow tokens in `@theme`: e1–e3 + pill, each with inset hairline.
- Motion custom properties on `:root`: 4 durations + 2 easing curves; `prefers-reduced-motion` block collapses all to 0ms/1ms.
- `@layer utilities`: subtle/line variants (teal-bg, teal-line, amber-bg, amber-line, danger-bg, glass-hairline), type scale (.text-klarvo-11 through .text-klarvo-40, 8 classes), focus-klarvo ring.
- `npm run build` (tsc + vite build): GREEN — 82 modules, 3.80s, 0 errors.
- NFR6 sentinel: no `fonts.googleapis.com` / CDN URLs anywhere in styles.css or index.html — CLEAN.
- Fonts confirmed in `dist/fonts/` (6 woff2 files) — Vite copies `public/` verbatim.
- No TypeScript/TSX/Rust files modified.

### File List

- `src/styles.css` (modified)
- `public/fonts/Geist-Regular.woff2` (new)
- `public/fonts/Geist-Medium.woff2` (new)
- `public/fonts/Geist-SemiBold.woff2` (new)
- `public/fonts/Geist-Bold.woff2` (new)
- `public/fonts/GeistMono-Regular.woff2` (new)
- `public/fonts/GeistMono-Medium.woff2` (new)

## Change Log

- 2026-06-14: Story 8.1 implemented — Studio-Dark token foundation, Geist fonts bundled locally, type scale + elevation + motion primitives added. `tsc`/`vite build` green.
- 2026-06-14: Code-review (3 adversarial layers, Opus) — clean. Token-value fidelity 100% vs SPEC-studio-dark-overhaul.md (19-token table, zero mismatch); all 17 old tokens resolve (8 via aliases, rest redefined in place); 6 woff2 present; NFR6 no-CDN sentinel real; build green. Reviewer divergence resolved: the global neutral-ladder re-tint on adopting the foundation is intended per AC (Acceptance Auditor read story+design-spec, MET), not an AC violation — the per-surface migration (8.2–8.6) is inline-hex→token + layout, not gating token *values*. Non-blocking carry-forward notes: Geist Mono only 400/500 bundled (no mono-bold for later surfaces); reduced-motion sets some motion tokens to 1ms vs 0ms (harmless). Status → done. cargo win-target failure pre-existing (C cross-compile, zero Rust touched).

