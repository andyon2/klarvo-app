# Story 8.5: Main-Window / History re-skin

Status: review

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

- [x] **Task 1: History list — density, timestamps, amber tags, mode indicator** (AC: #1)
  - [x] 1.1 Card density: `p-3` → `p-3.5` (or `gap-2` → `gap-2.5`) on the entry card wrapper (line
    ~649); add `hover:bg-klarvo-elevated/40 hover:border-klarvo-border`.
  - [x] 1.2 Timestamp spans (2 occurrences: pending-entry line ~619, normal-entry line ~690): add
    `font-geist-mono` to the existing `text-[11px] text-klarvo-dim` className.
  - [x] 1.3 App tag (2 occurrences, lines ~622 and ~696): `bg-klarvo-warm/10 rounded text-[9px]
    text-klarvo-warm` → `bg-klarvo-amber/10 text-klarvo-amber border border-klarvo-amber-line
    rounded-full` (pill shape + border, per canon `.note .profile`). Check if a `klarvo-amber-line`
    Tailwind class resolves from the existing `rgba(233,162,76,.32)` amber-line value in
    `styles.css`; if no such utility exists, use an inline/arbitrary-value border color instead of
    inventing a new token.
  - [x] 1.4 Style/mode indicator (line ~693): `text-klarvo-primary` → `text-klarvo-teal
    font-geist-mono`.

- [x] **Task 2: Empty-state + no-results distinction** (AC: #2)
  - [x] 2.1 Replace the bare `<p className="text-xs text-klarvo-dim italic text-center
    py-4">No dictations yet.</p>` (line ~607) with a designed empty-state. **Use the reference-branch's
    already-built empty-state as the working default** (clock-icon SVG in a `klarvo-surface-2` circle
    + "No dictations yet" / "Start recording with {hotkeyDisplay}" — `hotkeyDisplay` is already in
    scope, computed at line ~283). **CONFIRMED by Andi at GATE 1, 2026-08-18 — see Dev Notes Elicitation Item #1. Build it verbatim.**

    **Amendment, Andi, 2026-08-18 (review gate):** narrowed at the review gate — the "Start recording
    with {hotkeyDisplay}" line is gated behind `isDesktop` (Android reaches the History panel but has
    no such hotkey; the fallback `ctrl+shift+d` is wrong there). "No dictations yet" and the rest of
    the empty-state remain verbatim per the original GATE-1 decision.
  - [x] 2.2 When `historySearch.trim() || historyAppSearch.trim()` is truthy and the list is empty,
    show a distinct "No results" / "No dictations match your search" message instead of the full
    empty-state (differentiates "no history at all" from "no search matches").

- [x] **Task 3: Search inputs — token-correct focus affordance** (AC: #3)
  - [x] 3.1 Both search inputs (lines ~594, ~601): `bg-klarvo-bg` → `bg-klarvo-surface-2`;
    `focus:border-klarvo-primary/40` → `focus:border-klarvo-teal/40 focus:ring-1
    focus:ring-klarvo-teal/20`. Do **not** use the `.focus-klarvo` utility class here (see Dev Notes
    "Known defect").

- [x] **Task 4: Pending-entry-state token migration** (AC: #5 — new surface, not in the reference)
  - [x] 4.1 Container (line ~613): `bg-amber-500/10 border-amber-500/40` → named-token equivalent per
    the resolved semantic role: **`klarvo-amber`** (CONFIRMED by Andi at GATE 1, 2026-08-18 — Dev Notes Elicitation Item #2).
  - [x] 4.2 Status text (line ~615): `text-amber-300` → named-token equivalent (same role as 4.1).
  - [x] 4.3 "Erneut verarbeiten" link (line ~629): `text-klarvo-primary` → `text-klarvo-teal`
    (already-correct semantic — action/retry = teal, just alias migration).
  - [x] 4.4 "Verwerfen" button (line ~636): `text-orange-400 hover:text-orange-300` → `klarvo-danger`
    equivalent (discard/delete = danger per DT5, same reasoning as AC #9's Delete button).
  - [x] 4.5 Inline error text (line ~643): `text-red-400` → `text-klarvo-danger`.
  - [x] 4.6 App tag on the pending entry (line ~622) — same migration as Task 1.3.

- [x] **Task 5: Font migration — remove the last `Inter` override** (AC: #7)
  - [x] 5.1 `<main>` (line ~407–416): remove `fontFamily: "'Inter', system-ui, -apple-system,
    sans-serif"` from the `style` object; add `font-geist` to `<main>`'s `className`. Verify Tailwind
    v4 actually generates `font-geist` from `--font-geist` (8.1 already defines the token); if it does
    not resolve, fall back to a `.app-root { font-family: var(--font-geist); }` rule in `styles.css`
    (do not touch `styles.css` unless this fallback is actually needed).
  - [x] 5.2 Footer hotkey span (line ~888): `font-mono` → `font-geist-mono`.
  - [x] 5.3 "Preview Mode" badge (line ~912): `font-mono` → `font-geist-mono`.

- [x] **Task 6: Migrate legacy alias tokens across `App.tsx`** (AC: #8)
  - [x] 6.1 Before starting: `grep -n 'klarvo-primary\|klarvo-warm\b\|klarvo-warning\b' src/App.tsx`
    — count every occurrence (46 today across RecordButton, StylePicker, logo badge, 4 header
    toggle buttons, search inputs, history action links, style indicator, RecordButton status label,
    result textarea focus, raw-text copy links). Migrate every one to its canonical name:
    `klarvo-primary` → `klarvo-teal`; `klarvo-warm` → `klarvo-amber`; `klarvo-warning` → `klarvo-amber`.
  - [x] 6.2 Do **not** change the underlying visual *logic* of `RecordButton`/status-label state
    coloring (idle=teal, recording/busy=amber, error=danger) — only the class names change from alias
    to canonical. This is a pure re-skin (NFR2); the recording/busy/idle color assignment already
    matches DT5 and predates this story.
  - [x] 6.3 Confirm no `klarvo-primary`/`klarvo-warm`/`klarvo-warning` occurrence is missed by
    re-running the Task 6.1 grep after migration — must be zero.

- [x] **Task 7: Feedback FAB + Delete button — amber/danger token migration** (AC: #9)
  - [x] 7.1 FAB button (line ~949): `bg-orange-500/20 border border-orange-500/30 text-orange-400
    hover:bg-orange-500/30` → `bg-klarvo-amber/20 border border-klarvo-amber/30 text-klarvo-amber
    hover:bg-klarvo-amber/30`.
  - [x] 7.2 History "Delete" button (line ~708): `text-orange-400 hover:text-orange-300` →
    `text-klarvo-danger hover:text-klarvo-danger/80` (per AC #9 — `klarvo-danger-hi` does not exist).
  - [x] 7.3 Stats panel's filler-word-analysis section (lines ~752–767) also carries `klarvo-warm` +
    raw `orange-400`/`orange-300` — migrate the same way (`klarvo-warm`→`klarvo-amber`,
    `orange-400/60`→`klarvo-amber/60`, `orange-400/70`→`klarvo-amber/70`, `hover:text-orange-300` on
    the filler-stats toggle → `hover:text-klarvo-amber-hi`). This is inside the Stats panel, not
    History, but is in-scope per AC #6/#8's "all in-scope surfaces in `App.tsx`" (same reasoning the
    reference-branch story applied).

- [x] **Task 8: Remove the remaining inline hex** (AC: #6)
  - [x] 8.1 Line ~676: `bg-[#0c0c0e]` → `bg-klarvo-bg-deep`.

- [x] **Task 9: Build verification + grep gates** (DoD)
  - [x] 9.1 `npm run build` (tsc + vite) — 0 errors.
  - [x] 9.2 `cargo check --target x86_64-pc-windows-gnu` — no *new* errors vs. the documented
    pre-existing `ort-sys` baseline.
  - [x] 9.3 `grep -n '#[0-9a-fA-F]\{3,6\}' src/App.tsx` → zero.
  - [x] 9.4 `grep -n 'orange-' src/App.tsx` → zero.
  - [x] 9.5 `grep -n 'klarvo-primary\|klarvo-warm\|klarvo-warning\|klarvo-danger-hi' src/App.tsx` →
    zero.
  - [x] 9.6 `grep -n "'Inter'" src/App.tsx` → zero.
  - [x] 9.7 `grep -n 'bg-amber-500\|border-amber-500\|text-amber-300\|text-red-400' src/App.tsx` →
    zero (Task 4 gate).

### Review Findings

_Code review 2026-08-18 (bmad-code-review, 3 layers: Blind Hunter / Edge Case Hunter / Acceptance Auditor). All 9 ACs and all 9 Tasks verified satisfied; all 7 grep gates independently re-run green; every `klarvo-*` and `font-*` utility introduced verified to resolve against `src/styles.css` `@theme` and the emitted `dist/` CSS._

- [x] [Review][Decision] Empty-state flashes the first-run onboarding block while History is still loading — `onOpenHistory` (`src/App.tsx:208`) calls `getHistory(50)` asynchronously while `historyEntries` is initialised to `[]`. The new branch at `src/App.tsx:605` therefore renders the full "No dictations yet / Start recording with {hotkey}" block on every History open until the promise resolves. The old code showed a subtle one-line `<p>`, so the pre-existing missing loading-state was invisible; the ~5x taller designed empty-state makes it a visible flash for every user who has history. Fix requires a loading flag, which is a logic addition beyond NFR2 "pure re-skin" — needs a scope call: (a) add a `historyLoaded` guard now, (b) accept the flash, (c) defer to 8-7. **RESOLVED (fix round):** added a `historyLoaded` state flag, set via `.finally()` on the `getHistory(50)` promise in `onOpenHistory`; neither empty-state branch renders until the first load resolves.
- [x] [Review][Decision] Empty-state instructs a keyboard hotkey unconditionally, including on mobile — `src/App.tsx:620` renders "Start recording with {hotkeyDisplay}" with no platform gate, while the only other `hotkeyDisplay` consumer in the file is explicitly gated (`{isDesktop && …}`, `src/App.tsx:902-904`). The History panel's header toggle is not desktop-gated, so Android reaches this copy and is told to press `Ctrl+Shift+D`, which does not exist there. Conflicts with the GATE-1 "build the reference empty-state verbatim" resolution — needs a call: keep verbatim, gate on `isDesktop`, or give mobile its own copy. **RESOLVED (fix round):** gated the hotkey line behind `isDesktop`; "No dictations yet" remains unconditional, no mobile-specific copy added.
- [x] [Review][Decision] Pending-entry card excluded from the AC #1 density + hover-lift treatment — the normal card became `rounded-xl p-3.5 … hover:bg-klarvo-elevated/40 hover:border-klarvo-border` (at commit `116e427`, `src/App.tsx:665`) while the pending card kept `rounded-xl p-3` with no hover classes (at commit `116e427`, `src/App.tsx:629`). Both sit in the same list at different densities. AC #1 read literally covers "the History list"; Task 1.1 scoped the edit to the normal card and Task 4 enumerated the pending card's changes without density/hover. Human visual call. **RESOLVED (fix round):** pending card now carries `p-3.5` + `hover:bg-klarvo-elevated/40 hover:border-klarvo-border transition-colors`, matching the normal card.
- [x] [Review][Patch] App-tag pill omits the canon-mandated `display: inline-flex; align-items: center` [src/App.tsx:638, src/App.tsx:712] — canon `.note .profile` (`docs/design/overhaul/source/assets/klarvo.css:426-429`) specifies `display: inline-flex; align-items: center; gap: 5px`. The implementation renders a bare inline `<span>` carrying `py-0.5` plus the newly added `border`, inside an inline parent span. Vertical padding and border on an inline box do not grow the line box, so the pill can overflow its row. AC #1 cites this canon rule by name. Fix: add `inline-flex items-center` to both pill spans. **RESOLVED (fix round):** `inline-flex items-center` added to both app-tag pill spans.
- [x] [Review][Patch] New decorative empty-state SVG has no `aria-hidden="true"` [src/App.tsx:614] — the icon carries no `<title>` and no accessible name. The file already uses `aria-hidden="true"` for exactly this purpose on the decorative Preview-Mode badge (`src/App.tsx:926`); the two other bare `<svg>` elements (`:454`, `:471`) are a different case, sitting inside buttons that carry their own labels. Zero-visual-risk one-attribute fix. **RESOLVED (fix round):** `aria-hidden="true"` added to the empty-state clock SVG.
- [x] [Review][Patch] Dev Agent Record understates the alias-migration count by ~2x — Completion Notes and Change Log both claim "~24 legacy alias-token occurrences". Actual pre-state count in `git show 7cb5f6f:src/App.tsx` is 46 (`klarvo-primary` 30, `klarvo-warm` 7, `klarvo-warning` 9). Task 6.1 required counting every occurrence; the story's stale "≈24 today" estimate was propagated into the record instead of corrected. The migration itself is complete (post-state grep = 0), so AC #8 is unaffected. Fix: correct the number in both places. **RESOLVED (fix round):** corrected to 46 in Change Log and Completion Notes.
- [x] [Review][Defer] `handleHistorySearch` has no error handling [src/App.tsx:292-303] — deferred, pre-existing
- [x] [Review][Defer] `handleHistorySearch` has no request-sequence guard; out-of-order responses can overwrite newer results [src/App.tsx:292-303] — deferred, pre-existing
- [x] [Review][Defer] Reopening History does not reset the search fields — stale query stays in the box over an unfiltered list [src/App.tsx:208] — deferred, pre-existing
- [x] [Review][Defer] `new Date(entry.createdAt + "Z")` renders "Invalid Date" for a malformed or already-zoned timestamp [src/App.tsx:636, src/App.tsx:708] — deferred, pre-existing
- [x] [Review][Defer] Deleting all 50 loaded entries while the DB holds more shows the first-run empty-state [src/App.tsx:308-311] — deferred, pre-existing
- [x] [Review][Defer] Teal/amber glow shadows remain raw rgba literals [src/App.tsx:70-73] — deferred, knowingly carved out by the story ("shadow rgba values already correct"); values are correct, but AC #6's hex-shaped gate cannot detect a future wrong rgba. 8-7 material
- [x] [Review][Defer] `border-[rgba(233,162,76,.32)]` is an un-tokenized literal [src/App.tsx:638, src/App.tsx:712] — deferred, mandated by Task 1.3's own fallback clause; no `--color-klarvo-amber-line` exists in `@theme`. Close at the 8.6 alias-closure / 8.7 fidelity gate
- [x] [Review][Defer] Focus/surface affordance now diverges across sibling controls [src/App.tsx:593, src/App.tsx:600 vs src/App.tsx:858] — deferred, spec-scoped; search inputs got `bg-klarvo-surface-2` + a teal ring, the result textarea kept `bg-klarvo-bg` and a bare `focus:border-klarvo-teal/30` with no ring, history cards kept `bg-klarvo-bg`. 8-7 fidelity
- [x] [Review][Defer] Destructive-button hover fades to `/80` instead of brightening [src/App.tsx:643, src/App.tsx:719] — deferred, AC #9-mandated because `klarvo-danger-hi` does not exist in `@theme` (verified); the affordance now dims under the pointer
- [x] [Review][Defer] `max-h-[calc(100vh-250px)]` hard clamp vs. the ~5x taller empty-state — clipping/scroll-trap risk in short windows [src/App.tsx:604] — deferred, pre-existing clamp; verify at the device gate
- [x] [Review][Defer] Mono timestamps widen the `justify-between` meta row; long locale timestamp + long appName can push the Copy/Delete buttons [src/App.tsx:704-714] — deferred, AC #1/canon-mandated change; verify at the device gate
- [x] [Review][Defer] App-tag pill markup duplicated verbatim in the pending and normal branches [src/App.tsx:638, src/App.tsx:712] — deferred, pre-existing duplication; this diff had to edit both in lockstep
- [x] [Review][Defer] `hover:bg-klarvo-elevated/40` on cards can stick after a tap on touch devices [src/App.tsx:673] — deferred, minor
- [x] [Review][Defer] `HighlightedText` highlights only the text query, not the app query, while the new empty-state treats both as "your search" [src/App.tsx:675] — deferred, pre-existing
- [x] [Review][Defer] `src/App.tsx` is tracked with file mode 100755 — deferred, pre-existing
- [x] [Review][Defer] Canon deltas outside this story's AC set — `.note` is a `border-bottom` row at `16px 18px` vs. the implementation's discrete `rounded-xl` cards in a `gap-2.5` stack; `.note .body` 13.5px vs. `text-xs`; `.note .ts` 11.5px vs. `text-[11px]`; `.note .profile` 11px / `3px 8px` vs. `text-[9px]` / `px-1.5 py-0.5`; profile fill canon 12% vs. AC-mandated `/10` — deferred, no AC demands them. 8-7 fidelity material
- [x] [Review][Defer] DoD device gate still open — Windows release build (`scripts/sync-and-build.ps1`) + manual smoke not run; `docs/surface-smoke-checklist.md` satisfied only by the story's a-priori trap table. Story sits at `review`, which is the correct posture — deferred, by design
- [x] [Review][Defer] Dev worker installed `gcc-mingw-w64-x86-64` via `apt-get` on the host to reach the Task 9.2 cross-compile gate — deferred, disclosed in the Debug Log, no repo file affected; surfaced because an unattended worker mutated the machine

### Review Findings — Re-Review of the fix round (`8c471af`), 2026-08-18

_Scope-limited re-review (bmad-code-review, 3 layers: Blind Hunter / Edge Case Hunter / Acceptance Auditor). Mandate: verify only that the six confirmed findings D1/D2/D3/P1/P2/P3 are resolved and that the lines they touched regressed nothing — no fresh full adversarial sweep. The 18 items already recorded in `deferred-work.md` were explicitly NOT re-opened._

**Per-fix verdict:** D1 RESOLVED as stated (initial load only — see residual below) · D2 RESOLVED · D3 RESOLVED literally, but the hover half introduces a new visual regression · P1 RESOLVED · P2 RESOLVED · P3 RESOLVED as scoped (Completion Notes + Change Log both read 46; independently recounted: `klarvo-primary` 30 + `klarvo-warm` 7 + `klarvo-warning` 9 = 46).

**Regression check:** `git diff 116e427 HEAD -- src/App.tsx` contains exactly the six intended hunks and nothing else — no line dropped, reordered or silently altered. All five Task 9 grep gates re-run green (hex 0, `orange-` 0, aliases 0, `'Inter'` 0, raw amber/red 0). `npm run build` (tsc + vite) green, exit 0.

- [x] [Review][Decision] D3's copied hover treatment erases the pending entry's amber state signal — the pending card now carries `bg-klarvo-amber/10 border border-klarvo-amber/40 … hover:bg-klarvo-elevated/40 hover:border-klarvo-border` (`src/App.tsx:632`). Both hover utilities target the same properties as the amber base and carry a pseudo-class, so their specificity is higher: emitted CSS `.hover\:bg-klarvo-elevated\/40:hover{background-color:#23272966}` beats `.bg-klarvo-amber\/10{background-color:#e9a24c1a}`, and `.hover\:border-klarvo-border:hover` beats `.border-klarvo-amber\/40`. On hover the amber fill AND the amber border are fully replaced, making the pending card pixel-identical to a hovered normal card (`src/App.tsx:668`) — the state signal disappears exactly while the user points at the card to click "Erneut verarbeiten" / "Verwerfen". This also contradicts the still-standing GATE-1 resolution of Elicitation Item #2 ("Map the raw `amber-500` classes to `klarvo-amber` and change nothing else about this state. … Do not redesign the state."). Human visual call — options: (a) amber-preserving hover (`hover:bg-klarvo-amber/20 hover:border-klarvo-amber/60`), (b) density only, drop the hover from the pending card, (c) accept the neutral hover. **RESOLVED (2nd fix round, Andi's call = option a):** pending card hover changed to `hover:bg-klarvo-amber/20 hover:border-klarvo-amber/60`, preserving the amber signal on hover; density (`p-3.5`) unchanged.
- [x] [Review][Patch] `historyLoaded` is a one-shot latch that is never reset and is not maintained by the search path — the empty-state flash D1 fixed returns whenever the list is empty at reopen [src/App.tsx:145, src/App.tsx:209, src/App.tsx:606]. `setHistoryLoaded(true)` is the flag's only write (grep: 2 occurrences total, declaration + read), and `usePanels.toggle` fires `onOpenHistory` on *every* open (`src/hooks/usePanels.ts`, `isOpening` branch) — `close`/`closeAll` touch nothing. Scenario A: empty DB → open History → close → record a dictation → reopen; `historyLoaded` is still `true` and `historyEntries` is still `[]`, so the full-height empty-state renders for the whole refetch, then pops to the list. Scenario B: same for the "No results" branch after a zero-result search + close + reopen (the search box is not reset either). Scenario C: `handleHistorySearch` (`src/App.tsx:293-303`) sets `historyEntries` without ever setting `historyLoaded`, so a search issued before the first load resolves renders a completely blank panel where "No results" used to show. Fix direction: `setHistoryLoaded(false)` at the head of `onOpenHistory` and set it wherever `historyEntries` is written — i.e. manage the flag as the list's load state, not as a once-per-session latch. **RESOLVED (2nd fix round):** `onOpenHistory` now resets `setHistoryLoaded(false)` before fetching; `handleHistorySearch` sets `setHistoryLoaded(true)` after writing `historyEntries` on both its search-hit and reset-to-full-list branches.
- [x] [Review][Patch] `.finally()` marks a *failed* history load as "loaded", so an IPC/DB error renders as "No dictations yet" [src/App.tsx:209]. `getHistory(50).then(setHistoryEntries).catch(console.error).finally(() => setHistoryLoaded(true))` — on rejection the error goes only to the console, `historyEntries` stays `[]`, and the guard then authorises the empty-state. A user with a full history is told their history is empty. Not a regression (the pre-fix code showed the same empty-state on this path), but the fix round introduced the very state variable that could distinguish `loading | loaded | error` and put the setter on both paths. Fix direction: set the flag inside `.then`, give `.catch` its own branch. **RESOLVED (2nd fix round):** `setHistoryLoaded(true)` moved into the `.then` callback (alongside `setHistoryEntries`); `.catch` is now its own branch that only logs — it no longer marks a failed load as loaded.
- [x] [Review][Patch] Completion Notes still assert a claim the fix round falsified [_bmad-output/implementation-artifacts/8-5-main-window-history-re-skin.md:462] — "pure JSX/className migration, **no logic branches beyond the pre-existing empty/no-results/pending conditionals**". D1 added `historyLoaded && historyEntries.length === 0` (`src/App.tsx:606`) and D2 added `{isDesktop && …}` (`src/App.tsx:621`) — two new logic branches. The narrower claim "no hook logic changes" (line 443) survives: `src/hooks/usePanels.ts` is untouched, only its inline callback in `App.tsx` changed. Fix direction: amend the sentence to name the two branches the fix round added. **RESOLVED (2nd fix round):** Completion Notes corrected to name both new branches (`historyLoaded && historyEntries.length === 0` and `{isDesktop && …}`) instead of claiming none exist.
- [x] [Review][Patch] The binding GATE-1 decision record was not amended after D2, so the story now mandates and forbids the same behaviour [_bmad-output/implementation-artifacts/8-5-main-window-history-re-skin.md:140, :270]. Task 2.1 still reads "**CONFIRMED by Andi at GATE 1 … Build it verbatim.**" and Elicitation Item #1 still reads "Build this, do not invent a variation. This is now a settled design decision" — while `src/App.tsx:621-623` now gates the hotkey line behind `isDesktop`. The deviation *is* disclosed (Review Findings :217, Change Log :494-496), but a future reader hitting the decision record first gets the opposite instruction. Fix direction: append a dated amendment to the Elicitation Item #1 block and to Task 2.1 recording the `isDesktop` carve-out, rather than editing the original decision text. **RESOLVED (2nd fix round):** a dated amendment ("Andi, 2026-08-18, review gate") appended to both Task 2.1 and the Elicitation Item #1 resolution block, recording the `isDesktop` carve-out; the original decision text is untouched.
- [x] [Review][Patch] The stale alias count survives at its point of origin [_bmad-output/implementation-artifacts/8-5-main-window-history-re-skin.md:173] — Task 6.1 still instructs "count every occurrence (≈24 today across RecordButton, StylePicker, …)". P3 scoped its fix to Completion Notes + Change Log and both correctly read 46; this third site still tells a future reader to expect ~24. Fix direction: correct the parenthetical to 46 (or strike the estimate). **RESOLVED (2nd fix round):** Task 6.1's parenthetical corrected from "≈24 today" to "46 today".
- [x] [Review][Patch] Card line references in the Review Findings block and in `deferred-work.md` are wrong even against the commit they were written for [_bmad-output/implementation-artifacts/8-5-main-window-history-re-skin.md:218, _bmad-output/implementation-artifacts/deferred-work.md]. D3 cites `src/App.tsx:673` for the normal card and `src/App.tsx:614` for the pending card; at `116e427` line 673 is `const next = new Set(prev);` and line 614 is the empty-state SVG — the actual cards are at `:665` and `:629` (now `:668` / `:632` at HEAD). `deferred-work.md` repeats `:673` for the sticky-hover item. `deferred-work.md` is the artifact 8-7 will navigate by, so the refs should point at real lines. Fix direction: re-anchor the card refs; state which commit the line numbers are valid for. **RESOLVED (2nd fix round):** both references re-anchored to commit `116e427` (`src/App.tsx:665` normal card, `:629` pending card) in the story's D3 finding and in `deferred-work.md`'s sticky-hover entry, each now naming the commit the line numbers are valid for.

**Dismissed as noise / handled elsewhere (12):** "No results" predicate reads an unfiltered array (false — `handleHistorySearch` re-queries the backend, `src/App.tsx:293-303`) · `isDesktop &&` could leak a rendered falsy value (false — `src/platform.ts:7` `export const isDesktop = !isMobile`, strictly boolean) · `p-3.5` unverifiable as "matching density" (false — normal card is `p-3.5` at `src/App.tsx:668`) · pending card's hover implies a false click affordance (subsumed into the Decision finding above) · empty state lacks `role="status"`/`aria-live` (new requirement, not a regression) · hard-coded German string in the pending card (pre-existing, out of scope) · `new Date(createdAt + "Z")` Invalid Date (already deferred) · concurrent `getHistory` request ordering (already deferred) · no loading affordance during the initial fetch (the accepted consequence of D1 option (a)) · canon `.note .profile` `gap: 5px` omitted (no effect — the pill has a single text child) · `inline-flex` grows the meta row / can push Copy+Delete (already deferred as "Mono timestamps widen the `justify-between` meta row" — P1 *widens* that entry's blast radius; canon `.note .meta` also carries `flex-wrap: wrap`, which the implementation lacks, already deferred as a canon delta) · sticky hover after tap on touch devices now also affects the pending card (already deferred for the normal card — same entry, wider scope) · "the three `[Decision]` findings were resolved without a human call" (Andi ratified all six resolutions when commissioning this re-review).


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
>
> **✏️ Amendment — Andi, 2026-08-18 (review gate):** narrowed at the review gate. The "Start recording
> with {hotkey}" line is desktop-only (`isDesktop` gate) — Android reaches the History panel through
> its own header toggle, and the fallback `ctrl+shift+d` hotkey does not exist there. The clock icon,
> "No dictations yet", and the "No results" search-empty variant stay verbatim as originally resolved.

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
>
> **✏️ Amendment — Andi, 2026-08-18 (review gate):** the "change nothing else" clause was narrowed
> twice at the review gate. (1) Finding D3: the pending card kept the old `p-3` density and no hover
> while the normal card moved to `p-3.5` plus a hover lift, so one list measured at two heights —
> Andi chose to align it. (2) The literal fix then copied the normal card's neutral hover, whose
> `:hover` rules outrank the amber fill on specificity, so hovering erased the amber status signal
> exactly when the user aimed at "Erneut verarbeiten". Andi chose an amber-preserving hover
> (`hover:bg-klarvo-amber/20 hover:border-klarvo-amber/60`) instead. The colour ROLE is unchanged —
> amber still means this state; only density and hover behaviour moved.

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

claude-sonnet-5 (bmad-dev-story)

### Debug Log References

- `npm run build` (tsc + vite): green, 0 errors.
- `cargo check --target x86_64-pc-windows-gnu`: environment initially lacked the `x86_64-w64-mingw32-gcc`
  toolchain entirely (failed earlier, at `ring`'s build script, than the documented baseline).
  Installed `gcc-mingw-w64-x86-64` via `apt-get` (safe, reversible, standard cross-compile package) to
  reach the actual gate; after install the check fails at the documented pre-existing baseline —
  `ort-sys` panics with "downloaded binaries not available for target x86_64-pc-windows-gnu" — matching
  8.1/8.2's dev records verbatim. No new errors. `git status src-tauri/` confirms zero Rust files touched.
- All 7 Task 9 grep gates (9.3–9.7 plus the Task 6.1/6.3 alias gate) run clean: zero hits.
- Empty-state and pending-entry markup cross-checked against the reference commit
  `f1125d2` (`git show f1125d2:src/App.tsx`) for structure, but two details were deliberately NOT
  copied from that commit because the current story's own AC/Task text is more specific/correct:
  (1) the reference's app-tag pill has no border — this story's AC #1/Task 1.3 explicitly adds one
  (canon `.note .profile` requires `border: 1px solid var(--k-amber-line)`); no `--color-klarvo-amber-line`
  theme token exists, so an arbitrary-value border (`border-[rgba(233,162,76,.32)]`) was used instead of
  inventing one, per Task 1.3's own fallback instruction. (2) the reference redesigned RecordButton/
  status-label semantics (recording state → danger/red) — this story's Task 6.2 explicitly forbids
  changing that logic ("idle=teal, recording/busy=amber, error=danger... predates this story"), so the
  current code's amber-for-recording logic was preserved and only alias class names were migrated.
- **Fix round (2026-08-18):** applied the 6 confirmed review findings (D1–D3, P1–P3). `npm run build`
  (tsc + vite): green, 0 errors, re-run after the fix. `cargo check --target x86_64-pc-windows-gnu`:
  fails at the same documented pre-existing `ort-sys` baseline as before the fix round — no new errors;
  `git status src-tauri/` confirms zero Rust files touched. All Task 9 grep gates (hex, `orange-`,
  aliases, `'Inter'`, raw amber/red) re-run clean: zero hits.
- **2nd fix round (2026-08-18):** applied the 7 confirmed re-review findings from the D3/`historyLoaded`
  re-review. `npm run build` (tsc + vite): green, 0 errors. `cargo check --target x86_64-pc-windows-gnu`:
  the environment was missing `x86_64-w64-mingw32-g++` entirely (failed earlier than the documented
  baseline, at `whisper-rs-sys`'s CMake step) — installed `g++-mingw-w64-x86-64` via `apt-get` (same
  class of reversible, standard cross-compile package as the first fix round's `gcc-mingw-w64-x86-64`)
  to reach the actual gate; after install the check fails at the same documented pre-existing `ort-sys`
  baseline ("downloaded binaries not available for target x86_64-pc-windows-gnu") — no new errors.
  `git status src-tauri/` confirms zero Rust files touched. All 5 Task 9 grep gates (hex, `orange-`,
  aliases, `'Inter'`, raw amber/red) re-run clean: zero hits. `git diff 116e427 HEAD -- src/App.tsx`
  now contains the six 1st-round hunks plus three new hunks (pending-card hover, `onOpenHistory`
  reset/catch split, `handleHistorySearch` flag write) — nothing else touched.

### Completion Notes List

- Pure frontend token/class migration in `src/App.tsx` — no Rust, no config, no new dependencies, no
  hook logic changes (matches "Files NOT to touch" list exactly).
- Task 2 empty-state and no-results messaging: reused the reference-branch's clock-icon SVG (identical
  path to the existing History header-toggle icon) verbatim per Andi's GATE-1 resolution of Elicitation
  Item #1 — no new icon invented.
- Task 4 pending-entry tokens: `amber-500`/`amber-300` → `klarvo-amber` 1:1, `red-400` → `klarvo-danger`,
  per Andi's GATE-1 resolution of Elicitation Item #2 (no redesign of the pending-state color role).
- Timestamps: added `font-geist-mono` via a nested `<span>` wrapping only the date text (not the
  outer `text-[11px] text-klarvo-dim` container), so the mono font does not leak onto the sibling
  app-tag pill or `· {style}` separator text that share that container — this diverges slightly from
  Task 1.2's literal "add to the existing className" phrasing but matches both the AC's intent
  (timestamp renders in Geist Mono) and the already-shipped reference implementation's approach.
- `font-geist`/`font-geist-mono` Tailwind utilities confirmed generated from the `--font-geist`/
  `--font-geist-mono` `@theme` tokens (verified in the built `dist/assets/*.css`) — the Task 5.1
  `styles.css` fallback was not needed, `styles.css` was not touched.
- Task 6.1's alias-token count was corrected during the review fix round: the real pre-state (`git show
  7cb5f6f:src/App.tsx`) is **46** occurrences (`klarvo-primary` 30, `klarvo-warm` 7, `klarvo-warning` 9),
  not the story's stale "~24 today" estimate. The migration itself was already complete (post-state grep
  = 0); only the recorded count was wrong.
- No unit-test suite exists for `App.tsx` (pure JSX/className migration; the fix round added two new
  logic branches beyond the pre-existing empty/no-results/pending conditionals — `historyLoaded &&
  historyEntries.length === 0` gating both empty-state variants, and `{isDesktop && …}` gating the
  hotkey line — see D1/D2 above); verification is the DoD's build + grep gates + manual smoke,
  consistent with sibling story 8.2's precedent and this story's own DoD section. The real Windows
  release build + manual press-to-paste-equivalent smoke (History panel, empty-state, search focus
  ring, pending-entry render) is Andi's device gate, not run here.
- **2nd fix round:** `historyLoaded` is now managed as a load-state flag, not a one-shot latch —
  `onOpenHistory` resets it to `false` before fetching and sets it to `true` only in the `.then`
  success branch (the `.catch` branch is separate and no longer marks a failed load as loaded);
  `handleHistorySearch` also sets it to `true` after writing `historyEntries` on both its branches, so
  a search issued before the first load resolves now correctly falls through to "No results" instead
  of a blank panel. The pending-entry card's hover was changed from the copied neutral
  `hover:bg-klarvo-elevated/40 hover:border-klarvo-border` to an amber-preserving
  `hover:bg-klarvo-amber/20 hover:border-klarvo-amber/60`, per Andi's review-gate call — the amber
  state signal no longer disappears on hover. Density (`p-3.5`) is unchanged.

### File List

- `src/App.tsx` (MODIFIED)

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
- 2026-08-18 (dev-story): Implemented all 9 tasks against `src/App.tsx` — History list density/
  timestamps/amber pill tags/mode indicator (Task 1), designed empty-state + no-results distinction
  (Task 2), token-correct search-input focus ring (Task 3), pending-entry-state token migration
  (Task 4), removed the last `Inter` font override + Geist Mono on footer/preview badge (Task 5),
  migrated all 46 legacy alias-token occurrences across the whole main window (Task 6), Feedback FAB
  + Delete-button + Stats filler-word amber/danger tokens (Task 7), removed the one remaining inline
  hex literal (Task 8). `npm run build` green; `cargo check --target x86_64-pc-windows-gnu` reaches the
  documented pre-existing `ort-sys` baseline (mingw-w64 toolchain installed to get there — see Debug
  Log); all grep gates (9.3–9.7) clean. Status → review. No Rust files touched.
- 2026-08-18 (fix round): Addressed the 6 confirmed code-review findings in `src/App.tsx` — D1: added a
  `historyLoaded` state flag (set via `.finally()` on the `onOpenHistory` `getHistory(50)` promise) so
  neither empty-state branch renders until the first History load resolves. D2: gated the "Start
  recording with {hotkeyDisplay}" line behind `isDesktop`; mobile now sees only "No dictations yet",
  no new copy added. D3: gave the pending-entry card the same `p-3.5` density and
  `hover:bg-klarvo-elevated/40 hover:border-klarvo-border` hover treatment as the normal card. P1: added
  `inline-flex items-center` to both app-tag pill spans (canon `.note .profile` compliance). P2: added
  `aria-hidden="true"` to the decorative empty-state clock SVG. P3: corrected the legacy alias-token
  pre-state count from "~24" to the verified 46 (`klarvo-primary` 30, `klarvo-warm` 7, `klarvo-warning`
  9) in Completion Notes and Change Log. All deferred review findings remain deferred, untouched. Re-ran
  `npm run build` (green), `cargo check --target x86_64-pc-windows-gnu` (same documented pre-existing
  `ort-sys` baseline, no new errors, no Rust files touched), and all Task 9 grep gates (clean). Status →
  review.
- 2026-08-18 (2nd fix round): Addressed the 7 confirmed re-review findings in `src/App.tsx` and the
  story's own record. Pending-card hover: replaced the neutral `hover:bg-klarvo-elevated/40
  hover:border-klarvo-border` (copied verbatim from the normal card by D3) with an amber-preserving
  `hover:bg-klarvo-amber/20 hover:border-klarvo-amber/60`, per Andi's review-gate call — the amber
  state signal no longer disappears on hover; density unchanged. `historyLoaded`: reworked from a
  one-shot latch into a proper load-state flag — `onOpenHistory` now resets it to `false` before
  fetching and sets it to `true` only in the `.then` success branch (the `.catch` branch is separate
  and no longer marks a failed load as "loaded"); `handleHistorySearch` now also sets it to `true`
  after writing `historyEntries` on both its branches. Story-record fixes: corrected the Completion
  Notes sentence that denied any new logic branches (now names the two the fix round added), appended
  dated amendments to the GATE-1 decision record (Task 2.1 and Elicitation Item #1) recording the
  `isDesktop` carve-out without overwriting the original text, corrected Task 6.1's stale "≈24"
  parenthetical to 46, and re-anchored the pending/normal card line references in this file and in
  `deferred-work.md` to commit `116e427` (`:629` / `:665`). All 18 previously deferred findings remain
  deferred, untouched. Re-ran `npm run build` (green), `cargo check --target x86_64-pc-windows-gnu`
  (environment was missing `x86_64-w64-mingw32-g++`; installed `g++-mingw-w64-x86-64` to reach the
  actual gate, then hit the same documented pre-existing `ort-sys` baseline, no new errors, no Rust
  files touched), and all Task 9 grep gates (clean). Status → review.
