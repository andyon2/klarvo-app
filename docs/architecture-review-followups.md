# Architecture-Review Follow-ups — handoff for the Klarvo agent

**Created:** 2026-06-17 · **Type:** transient handoff · **Owner of next step:** Klarvo agent (BMAD)

## What this is

A short list of work that the 2026-06-17 architecture pass surfaced as **must-be-done but
not yet planned** — i.e. the `— not yet owned` rows in
[`ARCHITECTURE.md`](./ARCHITECTURE.md) §8. The architecture doc describes *state* and must
not carry a to-do list; this file carries the to-do, separately, so it is visible and not lost.

**This document is disposable.** Once each item has a real home in BMAD (a `docs/backlog.md`
entry or a created story in `sprint-status.yaml`), delete the corresponding section here. When
the file is empty, delete the file. The goal is a single source of truth for the plan (BMAD) —
this is a bridge, not a second backlog.

**Decision the Klarvo agent owns:** for each item, *backlog entry* (defer, keep visible) vs.
*create-story now* (schedule it). Scoping and acceptance criteria are a BMAD job + Andi's gate —
the briefs below are starting points, not finished stories.

---

## 1. Desktop token-enforcement gate (the remaining piece of "wall A")

**Problem (current state).** Android cannot drift off the design tokens: `KlarvoTheme.kt` is
codegen'd from the `--k-*` CSS custom properties and a build-gate
(`scripts/gen-android-theme.mjs --check`, story 9-10) fails the build if it was hand-edited.
**Desktop has no equivalent.** Nothing stops a component in `src/` from hardcoding a raw
`#hex`/`rgba()` instead of referencing a token — and several do today (e.g.
`src/FloatingBar.tsx`). Epic 8 ("Studio Dark") is re-skinning the surfaces onto tokens, but a
re-skin is a one-time cleanup; **without an enforced gate the same drift can return** the next
time anyone touches a surface.

**Why it matters.** This is the structural counterpart to ADR-0019 §2 ("tokens are generated,
not hand-copied") — established for Android, never extended to desktop *enforcement*. It is the
last open piece of the design-drift wall: re-skinning without a gate fixes the symptom, not the
cause.

**Candidate approach (for the agent/Andi to refine, not prescriptive):**
- A lint/CI check that flags raw `#hex` / `rgb()` / `rgba()` in `src/**/*.{tsx,ts,css}`
  **outside** the token definition file (`src/styles.css`), allowing only `var(--k-*)` /
  Tailwind token utilities. Mirrors the *spirit* of the Android `--check` gate.
- Tool choice (ESLint rule, stylelint, or a small repo script in `scripts/`) is open — match
  what already exists in the build.
- A short allowlist for any intentional exceptions, so the gate is honest, not bypassed wholesale.

**Suggested home.** A story under **Epic 8** (it is the enforcement half of the visual
overhaul), or a small standalone story. May warrant a one-line **ADR-0019 amendment**
recording that the "generated/enforced, not hand-typed" principle now covers desktop too.

**Context pointers.** ADR-0019 (design SSOT) · story 9-10 (Android token codegen + `--check`) ·
`ARCHITECTURE.md` §7 + §8.

### Implementation draft (starting point — NOT committed code)

The project has no ESLint/stylelint, so the lowest-blast-radius gate mirrors the Android one:
a small `scripts/*.mjs` run from the build. Drop-in draft for the agent to review/refine:

`scripts/check-desktop-tokens.mjs`:

```js
#!/usr/bin/env node
/**
 * check-desktop-tokens.mjs  — desktop counterpart to gen-android-theme.mjs --check.
 * Fails if a component hardcodes a raw color instead of a design token.
 *
 * Allowed:   var(--k-* / --color-klarvo-*), Tailwind token utilities (bg-klarvo-*, …).
 * Forbidden: raw #hex, rgb()/rgba(), hsl()/hsla() in components.
 * Legit raw colors live ONLY in the token layer (src/styles.css), which is allowlisted.
 *
 * Usage:  node scripts/check-desktop-tokens.mjs     # exit 1 if any offenders
 * ADR-0019 extended to desktop: tokens are enforced, not hand-typed.
 */
import { readFileSync, readdirSync, statSync } from 'fs';
import { resolve, dirname, relative, extname } from 'path';
import { fileURLToPath } from 'url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const SRC  = resolve(ROOT, 'src');

// Files where raw color literals are legitimate (the token / global layer).
const ALLOWLIST = new Set([
  'src/styles.css',   // @theme — the design tokens themselves
  'src/App.css',      // global base styles (confirm: keep or tokenize)
]);
const EXTS = new Set(['.ts', '.tsx', '.css']);
const PATTERNS = [ /#[0-9a-fA-F]{3,8}\b/, /\brgba?\s*\(/, /\bhsla?\s*\(/ ];
const OPT_OUT = /klarvo-allow-color/;   // honest per-line exception

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    const full = resolve(dir, name);
    if (statSync(full).isDirectory()) walk(full, out);
    else if (EXTS.has(extname(name))) out.push(full);
  }
  return out;
}

const offenders = [];
for (const file of walk(SRC)) {
  const rel = relative(ROOT, file).replace(/\\/g, '/');
  if (ALLOWLIST.has(rel)) continue;
  readFileSync(file, 'utf8').split('\n').forEach((line, i) => {
    if (OPT_OUT.test(line)) return;
    if (PATTERNS.some(re => re.test(line)))
      offenders.push(`  ${rel}:${i + 1}  ${line.trim().slice(0, 100)}`);
  });
}

if (offenders.length) {
  console.error('\nRAW COLORS in src/ — use design tokens, not hardcoded values.');
  console.error('Allowed: var(--k-*) / bg-klarvo-* utilities. Token home: src/styles.css.');
  console.error('Intentional exception: add `klarvo-allow-color` on the line.\n');
  console.error(offenders.join('\n'));
  console.error(`\n${offenders.length} offending line(s).`);
  process.exit(1);
}
console.log('[ok] No raw colors in src/ outside the token layer.');
```

Wiring in `package.json` (add a script + chain it ahead of the build, like android-smoke runs the android `--check`):

```json
"scripts": {
  "dev": "vite",
  "check:tokens": "node scripts/check-desktop-tokens.mjs",
  "build": "node scripts/check-desktop-tokens.mjs && tsc && vite build",
  "preview": "vite --port 1422 --host",
  "tauri": "tauri"
}
```

**Current reality (measured 2026-06-17, this draft run against `src/`):** 353 raw-color lines
total — but **261 are in `src/components/ThemeSwitcher.tsx`** (a theme/palette picker that
legitimately defines color sets). That file needs its own call first: allowlist it as a palette
source, or fold it into tokens. The remaining **~92 lines are the real component drift** —
`FloatingBar.tsx` (20), `settings/previewAppearance.ts` (17), `AppearanceContent.tsx` (15),
`PreviewPanel.tsx` (9), `SettingsPanel.tsx` (9), `settings/types.ts` (9), … — i.e. exactly the
Epic 8 surfaces. So once ThemeSwitcher is decided, the triage is small.

**Sequencing — the one real decision (not mechanical):** Epic 8 hasn't finished re-skinning the
surfaces, so making this **build-blocking now would fail on the not-yet-migrated files**
(`FloatingBar.tsx` etc. — exactly the Epic 8 surfaces). Two clean orders:
1. Land it first as non-blocking `check:tokens` (CI/manual report), flip it into `build` once
   `src/` is clean; **or**
2. Land the gate as the **last** story of Epic 8, after the surfaces are migrated.

**Known limitations (by design — same pragmatism as the Android gate):** it's a line-grep, not a
CSS/AST parser. False positives are possible (a hex inside a non-color string, a URL fragment,
a regex) — handle via the file `ALLOWLIST` or the per-line `klarvo-allow-color` opt-out. Run it
once against current `src/` and triage the initial batch. Catching the *wrong* token
(old `#2AC3A8` vs new `#29C7AC`) is out of scope — the raw-hex ban already forces token use.

**Definition of done (sketch — agent/Andi own the final ACs):**
- `npm run check:tokens` exits 0 on a clean `src/`, exits 1 with `file:line` on a planted raw hex.
- Wired into build or CI per the sequencing decision above.
- `src/styles.css` (and any confirmed token files) allowlisted; the `klarvo-allow-color` opt-out works.
- One-line **ADR-0019 amendment**: the "enforced, not hand-typed" rule now covers desktop too —
  components reference tokens only; `src/styles.css` is the sole raw-color home; the gate enforces it.

**Context pointers (impl).** Mirror `scripts/gen-android-theme.mjs` (ESM, fs/path, loud exit-1
box) and how `scripts/android-smoke.sh` runs its `--check` before building.

---

## 2. Full mechanical project map (`document-project` re-scan)

**Problem (current state).** `docs/index.md` notes "no full project scan has been run". The
new `ARCHITECTURE.md` is the *human-facing* front door, but the *mechanical* code map that
should sit under it does not exist.

**Why it matters.** The two are complementary: `ARCHITECTURE.md` ties things together and gives
the rationale/index; `document-project` produces the exhaustive code-as-is map. Having both is
what a fresh agent needs to orient without re-deriving the codebase.

**Candidate approach.** Run BMAD `document-project` → "Re-scan entire project". This is a
BMAD-native action, not a code change.

**Suggested home.** Not really a story — a one-off BMAD task. A backlog note is enough.

**Context pointers.** `docs/index.md` · `ARCHITECTURE.md` §8 + §10.

---

## Housekeeping

- When item 1 becomes a backlog entry or story → remove section 1 here.
- When item 2 is run (or backlogged) → remove section 2 here.
- File empty → delete `architecture-review-followups.md` (+ `.html`).
</content>
