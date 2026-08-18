# Story 8.5: Main-Window / History re-skin

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a user reviewing past dictations,
I want the history list and main window to have clear hierarchy and pleasant density,
so that I can scan and find past dictations easily.

## Context (why this story exists now, and a scope correction)

Epic 8 originally built its Studio-Dark re-skin entirely on `conductor/epic-8` (a branch that has
since drifted 238+ commits behind `v1-ship` and will **not** be merged — re-scope commit `fe4f0a0`,
2026-07-06, `docs/backlog.md` §"Epic 8 „Studio Dark" — RE-SCOPE"). That branch's finished 8.5 work
(commit `f1125d2`, "Main-Window / History Studio-Dark re-skin", done 2026-06-15) serves as a
**pattern-template reference only** — same approach as Story 8.2's re-port. This story re-ports the
same intent against the *current* `v1-ship` `src/App.tsx`, which has moved on since (Story 12-2 added
a `"pending"` history-entry state with its own, never-audited color usage — see AC #5).

**Scope correction (verified against the current tree, not carried over from docs):** `docs/backlog.md`
line 742 claims "History lebt heute in `VoiceNotesPanel.tsx` (nicht mehr `App.tsx`)". This is **stale
and factually wrong for the current `v1-ship` tree** — grep + full read of both files confirms:
- The entire History panel (state, search/filter handlers, list rendering, empty-state, pending-entry
  handling) lives in `src/App.tsx` (History state: lines ~144–152; handlers: ~292–343; JSX: ~569–721).
- `src/components/VoiceNotesPanel.tsx` implements a **different, currently-hidden** feature ("Voice
  Notes" — record-and-save without pasting; its header toggle button is commented out in `App.tsx`
  lines 477–490, "hidden for Early Access (feature incomplete)"). It is explicitly **out of scope**
  for this story (see "Files NOT to touch" below).

This story's target is **`src/App.tsx`** — matching both the epic's own reference-branch precedent
(`f1125d2` touched `App.tsx`, not `VoiceNotesPanel.tsx`) and the epic title "Main-Window / History
re-skin" (the *whole main window*, not just the History JSX block — the reference story's own AC #5
scoped it this way: "the History panel section (and all other in-scope surfaces in `App.tsx`)").

**Depends on 8.1 (done)** — the Studio-Dark token/type/motion foundation and backward-compat aliases
already exist in `src/styles.css`. Per the epic's dependency flow, 8.5 depends only on 8.1 and is
otherwise parallel to 8.2/8.3/8.4/8.6 (8.3/8.4 are since superseded by native overlays — irrelevant
here).

## Acceptance Criteria

1. **Given** the History list **When** it renders **Then**:
   - Card density improves (padding `p-3` → `p-3.5` or gap `gap-2` → `gap-2.5`), with a subtle hover
     lift (`hover:bg-klarvo-elevated/40 hover:border-klarvo-border`).
   - Timestamps (`entry.createdAt`) render in **Geist Mono** (`font-geist-mono`) — currently bare,
     no mono class at all.
   - App/profile tags (`entry.appName`) render in **amber** (`bg-klarvo-amber/10 text-klarvo-amber`,
     canonical, no alias) as a **pill** (`border-radius: full` + `border: 1px solid` amber-line, per
     canon `.note .profile` — currently `rounded` with no border).
   - The style/mode indicator (`entry.style !== "polished"`) uses `text-klarvo-teal` (not the
     `klarvo-primary` alias) **and** `font-geist-mono`, per canon `.note .mode` (`font-family:
     var(--k-mono); color: var(--k-teal)`).

2. **Given** an empty history or a no-match filter **When** nothing matches **Then** a designed
   empty-state renders (not a bare blank), and the two cases are **visually distinguished**: "no
   history at all" vs. "no results for this search" (see Dev Notes — Elicitation Item #1: the exact
   empty-state visual is not pinned by the design canon).

3. **Given** the search/filter inputs (text search + app search) **When** rendered **Then** they use
   `klarvo-surface-2` fill (not `klarvo-bg`), `border-klarvo-border/60`, and a **teal** focus
   affordance (`focus:border-klarvo-teal/40 focus:ring-1 focus:ring-klarvo-teal/20` — not the
   `.focus-klarvo` utility class, see Dev Notes "Known defect: do not use `.focus-klarvo` here"), per
   canon `.searchbar .input` / `.input:focus`.

4. **Given** real content **When** shown **Then** real dictation text + real labels are used (already
   true — the panel reads from actual `history.db`; no Lorem Ipsum is introduced by this story).

5. **Given** a history entry with `entry.status === "pending"` (Story 12-2's audio-retry-history
   state — did not exist when the reference-branch 8-5 was built, so it has never been token-audited)
   **When** it renders **Then** the raw Tailwind palette classes (`bg-amber-500/10`,
   `border-amber-500/40`, `text-amber-300`, `text-orange-400`/`text-orange-300` on the Verwerfen
   button, `text-red-400` on the inline error) are replaced with named Studio-Dark tokens (see Dev
   Notes — Elicitation Item #2: which semantic role "pending / awaiting processing" should map to is
   not pinned by DT5).

6. **And** the History panel section — and all other in-scope surfaces in `App.tsx` (the Main-Window:
   header, RecordButton, StylePicker, Stats panel, Feedback FAB — matching the reference-branch's own
   AC #5 scope) — carry **zero inline hex** for covered roles. Today there is exactly **one**:
   `bg-[#0c0c0e]` (History raw-text-expand background, line ~676) → `bg-klarvo-bg-deep`. (The
   equivalent site in the home/record raw-textarea, line ~868, is **already** migrated — do not touch
   it again.)

7. **And** the `<main>` element's inline `fontFamily: "'Inter', system-ui, -apple-system, sans-serif"`
   style override is removed, replaced by the `font-geist` Tailwind utility (from the `--font-geist`
   `@theme` token, 8.1) added to `<main>`'s `className`. The footer hotkey span and the "Preview Mode"
   badge (`font-mono` → `font-geist-mono`, 2 occurrences) get the real Geist Mono stack.

8. **And** legacy backward-compat alias tokens used across `App.tsx` — `klarvo-primary` (→
   `klarvo-teal`), `klarvo-warm` (→ `klarvo-amber`), `klarvo-warning` (→ `klarvo-amber`) — are
   migrated to their canonical Studio-Dark names per DT5 roles (teal = brand/ready/processing/focus;
   amber = live/activity). The aliases themselves **stay** in `styles.css` (removed only at the 8.6
   DT1-closure gate) — this story only migrates `App.tsx`'s usages off them.

9. **Given** the Feedback FAB button **When** rendered **Then** it uses `klarvo-amber` instead of raw
   `orange-500`/`orange-400` Tailwind classes (DT5: amber = activity/accent, no bare "orange"
   vocabulary). The History "Delete" action button (`text-orange-400 hover:text-orange-300`) uses
   `klarvo-danger` instead (Delete = destructive = danger per DT5, not amber) — **do not** use
   `klarvo-danger-hi` for the hover state; that token does not exist in the current `@theme` block
   (the reference-branch story names it, but it does not resolve — use `hover:text-klarvo-danger/80`,
   the same pattern already used elsewhere in this file for teal hover states).

**DoD (surface-class):**
- `npm run build` (tsc + vite) green.
- `cargo check --target x86_64-pc-windows-gnu` — no *new* errors vs. the pre-existing `ort-sys`
  cross-compile baseline (documented in 8.1/8.2's dev records); no Rust files are touched by this
  story.
- Real **Windows release build** (`scripts/sync-and-build.ps1`) + manual smoke: History panel opens,
  list shows real entries with improved density + Geist Mono timestamps + amber pill tags; empty-state
  renders when history is empty (and a distinct message when a search returns no results); search
  inputs show the teal focus ring; a `"pending"` entry (if reproducible — see Dev Notes) renders with
  named tokens, not raw Tailwind amber/orange/red.
- Walk `docs/surface-smoke-checklist.md` — see Dev Notes "Trap applicability" (none of the 6 traps
  apply; no new config keys/fields/events/geometry).
- Grep gates (Task 9) all green.

## Tasks / Subtasks

- [ ] **Task 1: History list — density, timestamps, amber tags, mode indicator** (AC: #1)
  - [ ] 1.1 Card density: `p-3` → `p-3.5` (or `gap-2` → `gap-2.5`) on the entry card wrapper (line
    ~649); add `hover:bg-klarvo-elevated/40 hover:border-klarvo-border`.
  - [ ] 1.2 Timestamp spans (2 occurrences: pending-entry line ~619, normal-entry line ~690): add
    `font-geist-mono` to the existing `text-[11px] text-klarvo-dim` className.
  - [ ] 1.3 App tag (2 occurrences, lines ~622 and ~696): `bg-klarvo-warm/10 rounded text-[9px]
    text-klarvo-warm` → `bg-klarvo-amber/10 text-klarvo-amber border border-klarvo-amber-line
    rounded-full` (pill shape + border, per canon `.note .profile`). Check if a `klarvo-amber-line`
    Tailwind class resolves from the existing `rgba(233,162,76,.32)` amber-line value in
    `styles.css`; if no such utility exists, use an inline/arbitrary-value border color instead of
    inventing a new token.
  - [ ] 1.4 Style/mode indicator (line ~693): `text-klarvo-primary` → `text-klarvo-teal
    font-geist-mono`.

- [ ] **Task 2: Empty-state + no-results distinction** (AC: #2)
  - [ ] 2.1 Replace the bare `<p className="text-xs text-klarvo-dim italic text-center
    py-4">No dictations yet.</p>` (line ~607) with a designed empty-state. **Use the reference-branch's
    already-built empty-state as the working default** (clock-icon SVG in a `klarvo-surface-2` circle
    + "No dictations yet" / "Start recording with {hotkeyDisplay}" — `hotkeyDisplay` is already in
    scope, computed at line ~283). **CONFIRMED by Andi at GATE 1, 2026-08-18 — see Dev Notes Elicitation Item #1. Build it verbatim.**
  - [ ] 2.2 When `historySearch.trim() || historyAppSearch.trim()` is truthy and the list is empty,
    show a distinct "No results" / "No dictations match your search" message instead of the full
    empty-state (differentiates "no history at all" from "no search matches").

- [ ] **Task 3: Search inputs — token-correct focus affordance** (AC: #3)
  - [ ] 3.1 Both search inputs (lines ~594, ~601): `bg-klarvo-bg` → `bg-klarvo-surface-2`;
    `focus:border-klarvo-primary/40` → `focus:border-klarvo-teal/40 focus:ring-1
    focus:ring-klarvo-teal/20`. Do **not** use the `.focus-klarvo` utility class here (see Dev Notes
    "Known defect").

- [ ] **Task 4: Pending-entry-state token migration** (AC: #5 — new surface, not in the reference)
  - [ ] 4.1 Container (line ~613): `bg-amber-500/10 border-amber-500/40` → named-token equivalent per
    the resolved semantic role: **`klarvo-amber`** (CONFIRMED by Andi at GATE 1, 2026-08-18 — Dev Notes Elicitation Item #2).
  - [ ] 4.2 Status text (line ~615): `text-amber-300` → named-token equivalent (same role as 4.1).
  - [ ] 4.3 "Erneut verarbeiten" link (line ~629): `text-klarvo-primary` → `text-klarvo-teal`
    (already-correct semantic — action/retry = teal, just alias migration).
  - [ ] 4.4 "Verwerfen" button (line ~636): `text-orange-400 hover:text-orange-300` → `klarvo-danger`
    equivalent (discard/delete = danger per DT5, same reasoning as AC #9's Delete button).
  - [ ] 4.5 Inline error text (line ~643): `text-red-400` → `text-klarvo-danger`.
  - [ ] 4.6 App tag on the pending entry (line ~622) — same migration as Task 1.3.

- [ ] **Task 5: Font migration — remove the last `Inter` override** (AC: #7)
  - [ ] 5.1 `<main>` (line ~407–416): remove `fontFamily: "'Inter', system-ui, -apple-system,
    sans-serif"` from the `style` object; add `font-geist` to `<main>`'s `className`. Verify Tailwind
    v4 actually generates `font-geist` from `--font-geist` (8.1 already defines the token); if it does
    not resolve, fall back to a `.app-root { font-family: var(--font-geist); }` rule in `styles.css`
    (do not touch `styles.css` unless this fallback is actually needed).
  - [ ] 5.2 Footer hotkey span (line ~888): `font-mono` → `font-geist-mono`.
  - [ ] 5.3 "Preview Mode" badge (line ~912): `font-mono` → `font-geist-mono`.

- [ ] **Task 6: Migrate legacy alias tokens across `App.tsx`** (AC: #8)
  - [ ] 6.1 Before starting: `grep -n 'klarvo-primary\|klarvo-warm\b\|klarvo-warning\b' src/App.tsx`
    — count every occurrence (≈24 today across RecordButton, StylePicker, logo badge, 4 header
    toggle buttons, search inputs, history action links, style indicator, RecordButton status label,
    result textarea focus, raw-text copy links). Migrate every one to its canonical name:
    `klarvo-primary` → `klarvo-teal`; `klarvo-warm` → `klarvo-amber`; `klarvo-warning` → `klarvo-amber`.
  - [ ] 6.2 Do **not** change the underlying visual *logic* of `RecordButton`/status-label state
    coloring (idle=teal, recording/busy=amber, error=danger) — only the class names change from alias
    to canonical. This is a pure re-skin (NFR2); the recording/busy/idle color assignment already
    matches DT5 and predates this story.
  - [ ] 6.3 Confirm no `klarvo-primary`/`klarvo-warm`/`klarvo-warning` occurrence is missed by
    re-running the Task 6.1 grep after migration — must be zero.

- [ ] **Task 7: Feedback FAB + Delete button — amber/danger token migration** (AC: #9)
  - [ ] 7.1 FAB button (line ~949): `bg-orange-500/20 border border-orange-500/30 text-orange-400
    hover:bg-orange-500/30` → `bg-klarvo-amber/20 border border-klarvo-amber/30 text-klarvo-amber
    hover:bg-klarvo-amber/30`.
  - [ ] 7.2 History "Delete" button (line ~708): `text-orange-400 hover:text-orange-300` →
    `text-klarvo-danger hover:text-klarvo-danger/80` (per AC #9 — `klarvo-danger-hi` does not exist).
  - [ ] 7.3 Stats panel's filler-word-analysis section (lines ~752–767) also carries `klarvo-warm` +
    raw `orange-400`/`orange-300` — migrate the same way (`klarvo-warm`→`klarvo-amber`,
    `orange-400/60`→`klarvo-amber/60`, `orange-400/70`→`klarvo-amber/70`, `hover:text-orange-300` on
    the filler-stats toggle → `hover:text-klarvo-amber-hi`). This is inside the Stats panel, not
    History, but is in-scope per AC #6/#8's "all in-scope surfaces in `App.tsx`" (same reasoning the
    reference-branch story applied).

- [ ] **Task 8: Remove the remaining inline hex** (AC: #6)
  - [ ] 8.1 Line ~676: `bg-[#0c0c0e]` → `bg-klarvo-bg-deep`.

- [ ] **Task 9: Build verification + grep gates** (DoD)
  - [ ] 9.1 `npm run build` (tsc + vite) — 0 errors.
  - [ ] 9.2 `cargo check --target x86_64-pc-windows-gnu` — no *new* errors vs. the documented
    pre-existing `ort-sys` baseline.
  - [ ] 9.3 `grep -n '#[0-9a-fA-F]\{3,6\}' src/App.tsx` → zero.
  - [ ] 9.4 `grep -n 'orange-' src/App.tsx` → zero.
  - [ ] 9.5 `grep -n 'klarvo-primary\|klarvo-warm\|klarvo-warning\|klarvo-danger-hi' src/App.tsx` →
    zero.
  - [ ] 9.6 `grep -n "'Inter'" src/App.tsx` → zero.
  - [ ] 9.7 `grep -n 'bg-amber-500\|border-amber-500\|text-amber-300\|text-red-400' src/App.tsx` →
    zero (Task 4 gate).

## Dev Notes

### ⚠️ Elicitation Item #1 — Empty-state visual is NOT pinned by the design canon

The binding canon (`docs/design/overhaul/source/Klarvo Design System.html`, History board, lines
644–694) only shows the **populated** History list (3 real-looking notes) — there is **no**
empty-state markup anywhere in the canon HTML or `klarvo.css` (verified: `grep -ni "empty" ...` across
both files returns nothing). Yet epics.md AC #2 explicitly demands "a designed empty-state renders."

The reference-branch (`conductor/epic-8`, commit `f1125d2`, done/Andi-facing-review 2026-06-15)
**invented** a specific empty-state (clock-icon SVG in a `klarvo-surface-2` circle + "No dictations
yet" / "Start recording with {hotkey}", plus a distinct "No results" search-empty variant) that was
**never confirmed against a canon render** — its own changelog says the human-visual gate for
"empty-states" was **downgraded** ("batched for Andy's morning," `history.db` render), not that Andi
explicitly signed off the empty-state design specifically.

This story's task list defaults to **reusing that exact prior design** (Task 2.1) as the lowest-risk,
lowest-net-new-invention option — it is already-built, coherent with the token system, and was at
least shipped once without objection. But per the "don't invent the answer" rule, this is flagged for
a human decision, not silently decided:

- **Option A (default in Task 2.1):** reuse the reference-branch's empty-state verbatim (clock icon +
  copy + hotkey reminder).
- **Option B:** design something new for this pass (would need a fresh canon render or Andi's
  freehand direction — more up-front cost).
- **Option C:** keep it minimal (upgrade typography only, no icon) — lowest effort, weakest fidelity
  to "designed empty-state."

> **✅ RESOLVED — Andi, GATE 1, 2026-08-18: Option A.** Reuse the reference-branch empty-state
> verbatim: clock icon in a `klarvo-surface-2` circle, "No dictations yet", "Start recording with
> {hotkey}", plus the distinct "No results" variant for an empty search. Build this, do not invent
> a variation. This is now a settled design decision, not a default.

### ⚠️ Elicitation Item #2 — "Pending" entry-state color role is NOT pinned by DT5

Story 12-2 ("Audio-Retry-Historie") added a `status === "pending"` history-entry state **after** the
Studio-Dark design handoff was produced — the canon has **zero** coverage of it (no `.note.pending` or
equivalent class exists anywhere in `Klarvo Design System.html` / `klarvo.css`). DT5's color semantics
table doesn't have an obvious fit either:
- **Teal** = brand/ready/processing/success — "pending" is not yet processing (it is *paused*,
  awaiting a manual retry), so this doesn't cleanly fit.
- **Amber** = "live/hört zu" (recording only, per DT5's own parenthetical scope note) — the *current*
  code already (informally) uses amber (raw `amber-500`, not the named token) for this state, which is
  the closest existing precedent but is explicitly narrower than DT5's written definition.
- **Danger** = stop/delete/error only — "pending" is not an error (it succeeded at recording, failed
  only at transcription) — using it here would misrepresent severity.

This story's task list defaults to migrating the **raw amber-500 classes to the named `klarvo-amber`
token 1:1** (Task 4.1/4.2) — the smallest possible change (keeps today's visual, only fixes the
non-namespaced-Tailwind-color violation), not a redesign of what color the pending state *should* be.
Flagged because a human may want "pending / awaiting action" to read differently from "amber = live
recording" now that both exist side-by-side in the same list.

> **✅ RESOLVED — Andi, GATE 1, 2026-08-18: amber, migrated 1:1 to the named token.** Map the raw
> `amber-500` classes to `klarvo-amber` and change nothing else about this state. The canon backs
> this: `klarvo.css` line 33 defines amber as "live / activity / recording / **warning**", and a
> paused entry awaiting a manual retry is a warning. Accepted consequence: amber then carries both
> the app tags and the pending state in the same list. Do not redesign the state.

### Known defect: do not use `.focus-klarvo` for the new search-input focus ring

`docs/backlog.md` §"[Defekt, vorbestehend aus 8-1] `.focus-klarvo` ist ein Dauer-Ring" — the
`.focus-klarvo` utility (`src/styles.css:253`) is a **bare class**, not `:focus-visible`-gated, so any
element carrying it statically shows a **permanent** teal ring, not a focus-only one. This is a known,
deferred-to-8-7 bug. Task 3 therefore uses explicit `focus:border-klarvo-teal/40 focus:ring-1
focus:ring-klarvo-teal/20` Tailwind classes (which correctly gate on the native `:focus` pseudo-class),
**not** `.focus-klarvo` — do not "fix" this by reaching for the shared utility, that would ship the bug
onto two new elements.

### Files NOT to touch (out of scope for this story)

- `src/components/VoiceNotesPanel.tsx` — a different, currently-hidden feature (see "Context" above).
  Do not "fix" it under the assumption it's the History surface.
- `src/components/SettingsPanel.tsx`, `src/components/settings/*` — Story 8.2, done.
- `src/styles.css` — no changes expected; the `@theme` tokens and aliases already exist from 8.1. Only
  touch it if Task 5.1's `font-geist` Tailwind-utility fallback is genuinely needed (verify first).
- `src-tauri/` (Rust) — no changes; this is a pure frontend token/class migration, no new config
  fields, no new Tauri commands.
- `src/hooks/useRecording.ts`, `useSettings.ts`, `usePanels.ts`, `useLicense.ts`, `useUiScale.ts` — no
  changes to any hook logic.
- `src/components/FeedbackModal.tsx`, `CostDashboard.tsx`, `QuickTip.tsx`, `ThemeSwitcher.tsx`,
  `icons.tsx` — only their *call sites* in `App.tsx` may change (e.g. FAB button classes); the
  components themselves are untouched.

### Trap applicability (`docs/surface-smoke-checklist.md`)

| Trap | Applies? | Rationale |
|---|---|---|
| #1 camelCase config keys | NOT TRIGGERED | No new config keys. |
| #2 Settings resync `useEffect` | NOT TRIGGERED | No new config fields, no Settings panel changes. |
| #3 Separate-window reactivity | NOT TRIGGERED | `App.tsx` is the main window, not a separate Tauri window. |
| #4 Window geometry / shape region | NOT TRIGGERED | Main window has no shape region; no size changes. |
| #5 Push vs. poll / event wiring | NOT TRIGGERED | No new events. |
| #6 Multi-hop save chain | NOT TRIGGERED | No new config fields plumbed anywhere. |

### DT5 Color Semantics (for Task 4/6/7 decisions)

- **Teal** = brand / ready / processing / success / focus-ring.
- **Amber** = "live / hört zu" (recording only — tally light; see Elicitation Item #2 for why the
  pending state's fit is imperfect).
- **Danger** = stop / delete / error only. Never used for send/confirm.

### Token Reference

| Old (alias or raw) | New (canonical) | Role |
|---|---|---|
| `klarvo-primary` | `klarvo-teal` | brand/ready/focus/processing |
| `klarvo-warm` | `klarvo-amber` | activity/live tags |
| `klarvo-warning` | `klarvo-amber` | busy/processing RecordButton state (logic unchanged, AC #8) |
| `orange-400`/`orange-500`/`orange-300` | `klarvo-amber` or `klarvo-danger` (per site, see Tasks 4/7) | — |
| `amber-500`/`amber-300` (raw Tailwind) | `klarvo-amber` | pending-entry state (Task 4) |
| `red-400` | `klarvo-danger` | pending-entry inline error (Task 4.5) |
| `#0c0c0e` | `klarvo-bg-deep` (`#0A0B0C`) | raw-text-expand background |
| `font-mono` | `font-geist-mono` | actual Geist Mono stack (`font-mono` = system mono, no Geist) |

### Canon Anchor (render/CSS — binding; prose SPEC is narrative-only)

- `docs/design/overhaul/source/Klarvo Design System.html`, lines 644–694 (`data-screen-label="History"`
  board): search bar, note-item structure, timestamp/mode/profile-tag markup. No empty-state markup
  present (see Elicitation Item #1).
- `docs/design/overhaul/source/assets/klarvo.css`, lines 417–430 (`/* ======= history ======= */`):
  `.searchbar`, `.note`, `.note .body/.meta/.ts/.mode/.profile/.showorig` — exact values used above.
- `docs/design/overhaul/SPEC-studio-dark-overhaul.md` is **explicitly superseded for visual values**
  (its own banner, line 3–9) — used here only for narrative framing (Overview, color-semantics prose),
  never as a value source.

### Previous Story Learnings (8.2, done)

- **Re-verify counts against today's tree, not the reference branch.** 8.2 found the reference's
  control-inventory numbers were stale (codebase grew since). This story hit the same pattern: the
  reference's 8.5 AC list didn't know about the Story-12-2 pending-entry state (Task 4 is net-new).
- **`isDirty`/resync `useEffect` is not touched here** — irrelevant to this story (no Settings changes,
  no new config fields).
- **Grep before declaring done** — 8.2's Task 6 gate pattern (re-run every migration grep after the
  edits, not just once at the start) is replicated in Task 9 here.

### Objective Smoke Verification

1. Open History in the real Windows build with existing entries — app tags should read amber
   (`#E9A24C`) as a pill with a border, not a plain rounded-corner chip.
2. Inspect a timestamp element in DevTools → Computed → `font-family` should resolve to "Geist Mono"
   first.
3. Inspect `<main>` → Computed → `font-family` should resolve to "Geist", not "Inter".
4. Clear the search fields and delete all history (or use a fresh profile) → the full empty-state
   should render; type a search string that matches nothing → the distinct "no results" message
   should render instead.
5. `grep -n '#[0-9a-fA-F]\{3,6\}' src/App.tsx` after changes → zero output.
6. If a pending entry can be produced (see Story 12-2's dev record for how to force a terminal STT/
   cleanup failure) — confirm it renders with `klarvo-amber`/`klarvo-danger`, not raw
   `amber-500`/`orange-400`/`red-400`.

### References

- [Source: `_bmad-output/planning-artifacts/epics-visual-overhaul.md` — Story 8.5 ACs, lines 447–475] — UX-DR4, DT1, DT3, DT5, NFR1–3 scope
- [Source: `docs/design/overhaul/source/Klarvo Design System.html`, lines 644–694] — History board render (canon, binding)
- [Source: `docs/design/overhaul/source/assets/klarvo.css`, lines 417–430] — History surface values (canon, binding)
- [Source: `docs/design/overhaul/source/MANIFEST.md`] — canon-vs-prose contradiction methodology; render/CSS wins over prose
- [Source: `docs/design/overhaul/SPEC-studio-dark-overhaul.md`] — narrative-only (superseded for values, own banner lines 3-9); §"Surfaces (Desktop)" line 111
- [Source: `src/App.tsx`] — full current implementation; History state ~144-152, handlers ~292-343, JSX ~569-721, RecordButton/StylePicker ~53-120, header ~417-492, Stats panel ~723-777, FAB ~921-953
- [Source: `src/styles.css`, lines 59-101] — `@theme` token block + backward-compat alias definitions (`klarvo-primary` etc.); line 253 `.focus-klarvo` (known defect, see Dev Notes)
- [Source: git `remotes/origin/conductor/epic-8` commit `f1125d2`, `_bmad-output/implementation-artifacts/8-5-main-window-history-re-skin.md` (done, 2026-06-15)] — pattern-template reference; **not mergeable** (238+ commits behind); re-verified against today's tree, several corrections applied (App.tsx-not-VoiceNotesPanel scope, pending-entry state didn't exist yet, `klarvo-danger-hi` doesn't resolve, shadow rgba values already correct)
- [Source: `_bmad-output/implementation-artifacts/8-2-settings-form-system-home-and-sub-pages.md`] — sibling re-port precedent (grep-driven migration, re-verify-against-current-tree discipline, alias-stays-until-8.6 pattern)
- [Source: `_bmad-output/implementation-artifacts/epic-8-fidelity-audit.md`] — confirms History was a "starker Match" against the mockup at 8-1 foundation time; only known deferred gap is DE-compact date format (→ 8-7, out of scope here)
- [Source: `docs/backlog.md`, §"Epic 8 „Studio Dark" — RE-SCOPE" and §"[Defekt, vorbestehend aus 8-1] `.focus-klarvo`..."] — re-port scope decision, non-merge decision, `.focus-klarvo` known-defect (do not reuse for new focus states)
- [Source: `docs/surface-smoke-checklist.md`] — trap applicability analysis (none triggered)
- [Source: `_bmad-output/project-context.md`] — "never make the user the rendering oracle," Windows-release-build DoD rule, camelCase config rule (n/a here, no config touched)
- [Source: `_bmad-output/implementation-artifacts/sprint-status.yaml` — epic-8 comment block] — 8-5 re-port scope

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

## Change Log

- 2026-08-18 (story creation): Re-created against current `v1-ship` tree via `bmad-create-story`.
  Reference branch `conductor/epic-8`'s finished 8.5 implementation (`f1125d2`, done 2026-06-15) used
  as a pattern template (not merged — 238+ commits behind). Scope corrected from a stale
  `docs/backlog.md` claim ("History lives in `VoiceNotesPanel.tsx`") to the verified actual location
  (`src/App.tsx`). Added net-new scope for Story 12-2's pending-entry state (post-dates the reference
  build, never token-audited). Two design/UI decisions flagged for human resolution rather than
  defaulted silently: the empty-state visual (Elicitation Item #1) and the pending-entry-state color
  role (Elicitation Item #2) — both default in the task list to the lowest-risk/lowest-invention
  option; BOTH were confirmed by Andi at GATE 1 on 2026-08-18 (Option A / amber 1:1) and are now settled.
