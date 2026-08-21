# Story 8.8: Action feedback — Copy and Delete (interaction affordance)

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a user acting on a history entry,
I want the app to answer my click,
so that I know the copy succeeded and a mis-click on Delete does not cost me a dictation.

## Acceptance Criteria

**AC1 — Copy confirms.**
**Given** a reachable Copy affordance on a desktop surface
**When** the user clicks it and the clipboard write succeeds
**Then** that control's label becomes `Copied` in `text-klarvo-success` for **1500 ms** and then
returns to its resting label and resting colour — without the surrounding layout shifting.

**AC2 — Confirmation is per-control, never global.**
**Given** several Copy controls are on screen (the history list is a `map`)
**When** the user clicks one
**Then** **only that control** shows `Copied`. Every other Copy control keeps its resting label.

**AC3 — Failure never claims success.**
**Given** the clipboard write rejects
**When** the user clicks Copy
**Then** the control shows `Copy failed` in `text-klarvo-danger` for 1500 ms and then returns to its
resting label. It never shows `Copied`. A console log alone does NOT satisfy this AC.

**AC4 — Delete collapses the row in place and offers Undo.**
**Given** a history entry
**When** the user clicks Delete
**Then** the entry's card is replaced **at its own list position** by the undo strip — `Deleted` on the
left, `Undo` on the right — and the backend `delete_history_entry` call has **not** been issued yet.

**AC5 — The undo window is 6000 ms, then the delete commits.**
**Given** the undo strip is showing
**When** 6000 ms pass without a click on `Undo`
**Then** the strip disappears, `delete_history_entry` is called exactly once for that id, and the entry
is gone after an app restart.

**AC6 — Undo restores in place and issues no backend call.**
**Given** the undo strip is showing
**When** the user clicks `Undo` inside the window
**Then** the card returns **unchanged and at the same list position**, its pending timer is cleared, and
`delete_history_entry` is never called for that id.

**AC7 — A pending delete is not resurrected by a refetch.**
**Given** an entry is inside its undo window
**When** the history list refetches from the backend (`onOpenHistory`, `handleHistorySearch`, or any
other reload)
**Then** the entry does **not** reappear as a normal card. Its pending-delete state and its timer
survive the refetch, or the delete commits — it never silently returns to the list as if untouched.

**AC8 — Closing the history panel commits pending deletes.**
**Given** one or more entries are inside their undo window
**When** the history panel closes or unmounts
**Then** every pending delete is flushed to the backend and no timer is left dangling.

**AC9 — Coverage of the reachable Copy sites.**
**Given** the story is done
**When** the reachable desktop `clipboard.writeText` call sites are enumerated
**Then** all **five** currently-silent sites carry AC1/AC3 feedback (`src/App.tsx` lines ~714, ~727,
~748, ~902, ~918), and `PreviewComments.tsx` keeps its existing confirmation but is aligned to the
1500 ms decided here. `VoiceNotesPanel.tsx` is deliberately **out of scope** — see the scope guard.

**AC10 — Tokens, not hex.**
**Given** the new states
**When** their colours are set
**Then** they use the existing named theme tokens `text-klarvo-success`, `text-klarvo-danger`,
`text-klarvo-teal`, `text-klarvo-dim`, `bg-klarvo-surface-2`. Zero inline hex is added. No new theme
token is invented — every token this story needs already exists in `src/styles.css`.

## Tasks / Subtasks

- [x] **Task 1 — A reusable copy-with-feedback hook** (AC: 1, 2, 3, 9)
  - [x] 1.1 Add one small local hook (e.g. `useCopyFeedback`) in `src/App.tsx` or a sibling module. It
        keys state by a caller-supplied string id so two controls never share a state (AC2).
  - [x] 1.2 It exposes: a `copy(id, text)` action and a `statusOf(id)` returning
        `"idle" | "copied" | "failed"`. `copy` awaits `navigator.clipboard.writeText`, sets `"copied"`
        on resolve and `"failed"` on reject, and schedules the reset after `1500`.
  - [x] 1.3 Clear every outstanding timer on unmount. Re-clicking the same control restarts its timer
        rather than stacking a second one.
  - [x] 1.4 Put the `1500` and the `6000` in two named module constants
        (`COPY_FEEDBACK_MS`, `UNDO_WINDOW_MS`) with a comment naming them as Andi's 2026-08-19 decision.
        Do not scatter magic numbers.

- [x] **Task 2 — Wire the five silent Copy sites** (AC: 1, 2, 3, 9)
  - [x] 2.1 `src/App.tsx` ~714 `Copy Original` (history card, raw text) — currently **no `.catch`**.
  - [x] 2.2 `src/App.tsx` ~727 `Copy` (history card, raw-text hover overlay) — currently **no `.catch`**.
  - [x] 2.3 `src/App.tsx` ~748 `Copy` (history card, main action) — currently `.catch(console.error)`;
        replace the console-only path with the visible `Copy failed` state.
  - [x] 2.4 `src/App.tsx` ~902 `Copy Original` (current recording) — currently **no `.catch`**.
  - [x] 2.5 `src/App.tsx` ~918 `Copy` (current recording, raw textarea overlay) — currently **no
        `.catch`**.
  - [x] 2.6 Use a distinct id per site. For mapped history rows the id must include `entry.id`
        (e.g. `` `hist-raw-${entry.id}` ``), otherwise AC2 breaks.
  - [x] 2.7 The two hover-overlay buttons (~727, ~918) sit inside `opacity-0 group-hover/*:opacity-100`
        wrappers. Confirm the confirmation is still readable while the pointer stays on the card.

- [x] **Task 3 — Align the one site that already confirms** (AC: 9)
  - [x] 3.1 `src/components/PreviewComments.tsx:434` already does `setCopied(true)` +
        `setTimeout(..., 2000)` and already has a textarea fallback. **Do not rewrite it into the hook.**
        Change only the `2000` to the shared `COPY_FEEDBACK_MS` so both surfaces confirm for the same
        time. Leave its fallback path intact.

- [x] **Task 4 — Optimistic delete with an in-place undo strip** (AC: 4, 5, 6, 7, 8, 10)
  - [x] 4.1 Hold pending deletes in component state keyed by entry id, e.g.
        `pendingDeletes: Map<number, {entry: HistoryEntry, timer: number}>`. The **entry object stays in
        `historyEntries`** so its list position is preserved (AC4/AC6) — do not filter it out.
  - [x] 4.2 In the list render, when an entry's id is in `pendingDeletes`, render the undo strip
        **instead of** the card body, in the same list slot.
  - [x] 4.3 Strip markup mirrors the canon `.note.deleted`: a flex row on `bg-klarvo-surface-2`, with
        `Deleted` left in `font-geist-mono text-[11px] text-klarvo-dim` and an `Undo` button right in
        `text-[11px] text-klarvo-teal`. The strip is **not** red — see Dev Notes.
  - [x] 4.4 `handleDeleteHistoryEntry` no longer awaits the backend. It records the pending delete and
        starts a `UNDO_WINDOW_MS` timer whose callback calls `deleteHistoryEntry(id)`, then removes the
        entry from both `pendingDeletes` and `historyEntries`.
  - [x] 4.5 `handleUndoDelete(id)` clears the timer and drops the id from `pendingDeletes`. It must not
        touch `historyEntries` (the entry never left) and must not call the backend.
  - [x] 4.6 Guard the backend call so it runs **exactly once** per id even if the timer callback and a
        flush (Task 4.7) race.
  - [x] 4.7 Flush on unmount and on panel close: commit every pending delete and clear every timer
        (AC8). Wire it to the same lifecycle that `onClose`/`panels.close("history")` already uses.

- [x] **Task 5 — Refetch safety** (AC: 7)
  - [x] 5.1 `onOpenHistory` and `handleHistorySearch` both replace `historyEntries` wholesale from the
        backend. A pending-deleted entry is still in the DB, so it comes back as a normal card and its
        strip vanishes while the timer keeps running — a silent, confusing delete.
  - [x] 5.2 Fix it deterministically: on any refetch, either (a) re-apply `pendingDeletes` to the fresh
        list so those rows render as strips again, or (b) flush the pending deletes before refetching.
        Pick one, implement it in **one** place, and say which in the Completion Notes.
  - [x] 5.3 Note that the desktop history list currently has no live push updates; the only refetch
        paths are the two named above plus `onRefresh` style callbacks. Grep for `setHistoryEntries(`
        and cover every assignment that comes from the backend.

- [x] **Task 6 — Verification gates**
  - [x] 6.1 `npm run build` (tsc + vite) green, zero errors.
  - [x] 6.2 Grep gate: no new inline hex in the touched files —
        `grep -nE "#[0-9a-fA-F]{3,8}" src/App.tsx src/components/PreviewComments.tsx` shows no NEW hits
        beyond the pre-existing ones (record the before/after counts).
  - [x] 6.3 Grep gate: `grep -n "clipboard.writeText" src/App.tsx src/components/*.tsx` — every hit
        except the `VoiceNotesPanel.tsx` one routes through the feedback path.
  - [x] 6.4 GATE-4 structural smoke — see Dev Notes › Verification.

## Dev Notes

### The design canon governs this story — read it, do not re-decide it

The binding visual source is `docs/design/overhaul/source/` (HTML render + `assets/klarvo.css`,
ADR-0019). It was **extended for this story on 2026-08-19**; canon fingerprint
`74441200bdf4214adaf9b8fbe46a7bc6`. Read the governing rules there, do not transcribe values from this
file:

- `.note .acts`, `.note .act.copy`, `.note .act.copy.done`, `.note .act.del` — the Copy/Delete controls
  and the confirmed state.
- `.note.deleted`, `.note.deleted .state`, `.note.deleted .undo` — the undo strip.
- The History artboard (`.board[data-screen-label="History"]`) shows all three states rendered.

**Two canon rules that are easy to get wrong:**

1. **The confirmed state is `--k-success`, not teal.** It inherits the FloatingBar `done` artboard's
   language (`--k-success` + "kurzer Check, dann dematerialisieren"). In Tailwind that is
   `text-klarvo-success` (`--color-klarvo-success: #4FC58A`, already in `src/styles.css:87`).
2. **The undo strip is NOT red.** The canon reserves `danger` for the destructive *control*
   ("Rot = zerstörerisch … sonst nie"). The strip is a recovery offer: neutral `bg-klarvo-surface-2`
   ground, dim mono state label, teal `Undo`. The `Delete` button itself stays `text-klarvo-danger`.

**Scope of the canon extension: the two STATES, not the PLACEMENT.** Copy and Delete stay exactly where
Story 8-5 built them and passed GATE-4. Do not move, resize or restyle the existing controls.

### Settled decisions — do NOT re-open these

Andi decided all four on 2026-08-19 (Phase A). They are inputs, not questions:

| Point | Decision |
|---|---|
| Copy confirmation | in-place label swap to `Copied` |
| Delete safety | **undo window**, deliberately NOT a confirmation prompt |
| Undo form | the row collapses **in place** to a strip; the entry keeps its list slot |
| Timing | `Copied` = **1500 ms** · undo window = **6000 ms** |
| Language | **English** labels, matching the existing `Copy` / `Delete` buttons |
| Voice-Notes site | **out of scope**, recorded in `docs/backlog.md` |

Source: `_bmad-output/planning-artifacts/sprint-change-proposal-2026-08-19.md` + the canon MANIFEST row
dated 2026-08-19.

### Two corrections to the epic text — verified at the tree on 2026-08-19

The epic's Story 8.8 entry says "7 sites … no site is left silent". The tree says otherwise. Trust
this section, not the epic prose:

1. **`PreviewComments.tsx:434` is already NOT silent.** It has `setCopied(true)` +
   `setTimeout(..., 2000)` and a `document.createElement("textarea")` fallback for a rejected clipboard
   write. **Reuse it, do not reinvent it.** The only change there is the duration (Task 3.1).
2. **`VoiceNotesPanel.tsx:109` is unreachable.** The header toggle that opens the panel is commented
   out at `src/App.tsx:491` ("Notes toggle -- hidden for Early Access (feature incomplete)"), so
   `panels.showNotes` can never become true through the UI. Andi's decision: leave it untouched and
   record it. Verified — the component IS still imported and rendered at `src/App.tsx:834`, but only
   behind that dead flag.

Net: **five** silent sites to fix, **one** to align, **one** to skip.

### Source tree — what this story touches

| File | Change |
|---|---|
| `src/App.tsx` | copy-feedback hook + 5 call sites; `handleDeleteHistoryEntry` (~line 320) becomes optimistic; pending-delete state; undo strip in the history list render (~line 746) |
| `src/components/PreviewComments.tsx` | one constant (`2000` → `COPY_FEEDBACK_MS`) |

**Files NOT to touch:** anything under `src-tauri/` · `android/` · `src/styles.css` (every needed token
already exists) · the design canon (already extended) · `VoiceNotesPanel.tsx`.

### Hard scope guards

- **No Rust change. No SQLite schema change. No new dependency.** The undo window is a frontend-only
  optimistic delete. `delete_history_entry` (`src-tauri/src/commands/history.rs:68`) stays exactly as it
  is — a hard `DELETE`, called later rather than immediately.
- **No soft-delete column.** It was considered and rejected in the change proposal: it would need a
  purge path and would drag in the Android store.
- **Desktop only.** The Android clipboard twin (`KlarvoOverlayService.kt`) has the same gap and belongs
  to Epic 9. It is already in `docs/backlog.md`. Do not touch Kotlin.
- **Accepted failure mode:** if the app is killed inside the undo window, the backend delete never runs
  and the entry survives. That is by design — nothing is lost. Do not add persistence to "fix" it.

### Traps that will bite this implementation

1. **A single `copied` boolean lights up every Copy button.** The history list is a `map`. Key the
   state by entry id. This is AC2 and it is the single most likely defect.
2. **The refetch resurrection (AC7).** `onOpenHistory` and `handleHistorySearch` overwrite
   `historyEntries` from the DB, where the pending-deleted row still exists. Without Task 5 the row
   silently returns to normal while its timer keeps ticking, and then it vanishes 6 s later with no
   explanation.
3. **Double delete.** The timer callback and the unmount flush can both fire for the same id. Guard it
   (Task 4.6). A second `delete_history_entry` on a gone id is harmless in SQL but hides real bugs.
4. **Dangling timers on unmount.** The history panel is conditionally rendered. Clear on unmount, and
   flush the pending deletes rather than dropping them (AC8).
5. **Layout shift on the label swap.** `Copy` → `Copied` and `Copy` → `Copy failed` are wider. Reserve
   the width or align so neighbouring controls do not jump (AC1: "without the surrounding layout
   shifting").
6. **`historyLoadState`.** Story 8-5's fix rounds replaced a `historyLoaded` latch with a load-state
   (`setHistoryLoadState("loaded")`, `src/App.tsx:~67`). The empty-state renders off it. An undo strip
   must not make the list look empty, and flushing the last entry must land in the correct empty state.

### Verification

**Gates you run yourself:**

- `npm run build` (tsc + vite) — the real green gate for this story.
- The grep gates in Task 6.
- **No `cargo check` needed** — this story touches zero Rust. (For reference: 8-1/8-2/8-5 all recorded
  a *pre-existing* `ort-sys` failure on `--target x86_64-pc-windows-gnu`; do not chase it, and do not
  run it at all here.)
- **GATE-4 structural smoke, unattended:** drive the real React surface in Chromium via
  `npm run preview` (port 1422) and the repo-pinned **puppeteer 24.38.0** (`node_modules`), **not** a
  floating `npx playwright` — the cached Playwright browser (1228) does not match the version npx
  resolves (1234). Assert behaviour, not pixels: click Copy → the clicked control's text becomes
  `Copied` and no sibling control changed → after 1500 ms it reads `Copy` again. Click Delete → the
  strip appears in the same list slot → `Undo` restores the card → a second Delete left alone for
  6000 ms removes the row and issues exactly one backend call. Write the evidence to
  `_bmad-output/implementation-artifacts/gate4-evidence/8-8/`.
- There is **no unit-test suite for `App.tsx`** (8-5 precedent). The gates above plus the smoke are the
  verification. Do not invent a test harness for this story.

**Andi's gate (residual, not yours):** the Windows release build via `scripts/windows-build.sh`, then
the feel — how 1,5 s and 6 s actually read on screen. A green Chromium run proves the wiring and the
structure; it does not prove the aesthetics. Never present proxy-green as a visual pass.

### Project Structure Notes

- React 19.1 · TypeScript 5.8.3 strict · Vite 7 · TailwindCSS 4.2. ESM only.
- Tailwind theme tokens live in the `@theme` block of `src/styles.css` as `--color-klarvo-*`. Every
  token this story needs is already there: `success` (line 87), `danger` (86), `teal` (76), `dim` (72),
  `surface-2` (64). **Do not add one.**
- Code and comments in English; chat in German. Commits small and scoped, **never `git add .`**.
- Branch: this story builds on `conductor/epic-8-run2`, branched off `v1-ship` at `dd06afe`.

### References

- [Source: `_bmad-output/planning-artifacts/epics-visual-overhaul.md#Story 8.8`] — story statement, AC
  skeleton, scope guard, epic-scope amendment, UX-DR6.
- [Source: `_bmad-output/planning-artifacts/sprint-change-proposal-2026-08-19.md`] — trigger, evidence
  table, the two settled design decisions, the rejected soft-delete alternative.
- [Source: `docs/design/overhaul/source/MANIFEST.md#In-repo extensions`] — the 2026-08-19 row: what the
  canon now says about these two states, and why the strip is not red.
- [Source: `docs/design/overhaul/source/assets/klarvo.css`] — `.note .acts`, `.act.copy.done`,
  `.note.deleted` (value truth).
- [Source: `_bmad-output/project-context.md#Testing Rules`] — Linux green ≠ DoD for surface stories;
  the rendering-oracle anti-pattern.
- [Source: `_bmad-output/implementation-artifacts/8-5-main-window-history-re-skin.md#Dev Agent Record`]
  — no `App.tsx` test suite; `historyLoadState` shape; the `ort-sys` cross-check baseline.
- [Source: `docs/backlog.md`] — the deferred Android clipboard twin and the deferred Voice-Notes site.

## Dev Agent Record

### Agent Model Used

claude-sonnet-5 (bmad-dev-story)

### Debug Log References

- `npm run build` (tsc + vite): green, 0 errors.
- No `cargo check` — this story touches zero Rust, per its own Dev Notes.
- Grep gate (Task 6.2): `grep -nE "#[0-9a-fA-F]{3,8}" src/App.tsx src/components/PreviewComments.tsx`
  — 0 hits before, 0 hits after. No inline hex introduced.
- Grep gate (Task 6.3): `grep -n "clipboard.writeText" src/App.tsx src/components/*.tsx` — 0 hits in
  `App.tsx` (all 5 sites route through `copyFeedback.copy`); `PreviewComments.tsx:435` keeps its own
  existing `navigator.clipboard.writeText` + fallback (Task 3, not rewritten into the hook, by design);
  `VoiceNotesPanel.tsx:109` is the one deliberately out-of-scope site (unreachable — its opener is
  commented out at `App.tsx:491`).
- Grep gate (Task 5.3): `grep -n "setHistoryEntries(" src/App.tsx` — all backend-driven wholesale
  assignments (`loadHistory`, `handleHistorySearch`) are preceded by `flushPendingDeletes()`; the two
  other call sites (`handleReprocessPendingEntry`, `handleDiscardPendingEntry`) target one known id each
  and are not refetches, so they need no flush.
- GATE-4 structural smoke (Task 6.4): `npm run preview` (port 1422) + repo-pinned puppeteer 24.38.0,
  headless Chromium, against the mock backend. 18/18 assertions passed — AC1/AC2/AC3/AC4/AC5/AC6/AC9/AC10
  all exercised behaviourally (click → DOM state → timer wait → DOM state). Clipboard success/failure was
  driven by toggling the real CDP `clipboardSanitizedWrite` permission on/off (a genuine permission
  denial, not a mock/stub) — discovered along the way that puppeteer 24.38.0's high-level
  `overridePermissions("clipboard-write")` does NOT reliably gate `navigator.clipboard.writeText` in
  headless Chromium; the raw CDP permission name `clipboardSanitizedWrite` via `Browser.grantPermissions`
  does. AC7 (refetch safety) and AC8 (panel-close flush) were verified by source read, not by the browser
  script, because the mock `deleteHistoryEntry` never removes an entry from the static `MOCK_HISTORY`
  array — a real refetch in preview mode always re-serves all 3 mock rows regardless of the app's actual
  flush logic, so a browser-observed "did it come back?" signal would be meaningless for those two ACs.
  Full evidence: `_bmad-output/implementation-artifacts/gate4-evidence/8-8/` (`verdict.md` + 5 screenshots
  + `measurements.json`). The driving script lived outside the repo tree and was deleted after the run —
  nothing under `_bmad-output/implementation-artifacts/gate4-evidence/8-8/` is test-harness source, only
  evidence.

### Completion Notes List

- Found the implementation already substantially in place in the working tree at story start (hook,
  5 wired Copy sites, `PreviewComments.tsx` alignment, optimistic delete + undo strip, refetch-safety
  flush) but with every task checkbox still unchecked and no Dev Agent Record — treated as unverified
  in-progress work, not as done. Verified every task against its AC via code read, the grep gates, and a
  live GATE-4 browser run before checking anything off.
- Task 4.1 stores pending deletes in a `useRef<Map<number, {entry, timer}>>` as the canonical store (so
  the "exactly once" commit guard in Task 4.6 is synchronous, not dependent on React's batched state
  updates), with a small `pendingDeleteIds: Set<number>` mirroring its keys purely to drive re-renders.
  The entry itself is never removed from `historyEntries` until the delete actually commits, matching
  AC4/AC6's "same list position" requirement.
- Task 5.2: implemented option (b) — flush pending deletes before refetching — in the two places that do
  a wholesale `historyEntries` replace (`loadHistory`, `handleHistorySearch`), rather than re-applying
  `pendingDeletes` to the fresh list. Confirmed via the Task 5.3 grep that no third backend-driven
  `setHistoryEntries(` call site exists.
- AC9's "five silent sites": confirmed via `grep -n "clipboard.writeText\|copyFeedback.copy" src/App.tsx`
  that zero direct `clipboard.writeText` calls remain in `App.tsx` and all five now go through
  `copyFeedback.copy(id, text)` with a distinct id per site (mapped history rows key by `entry.id`).
- No unit-test suite exists for `App.tsx` (8-5 precedent, restated in this story's own Dev Notes) — the
  build, the three grep gates, and the GATE-4 browser smoke are the verification for this story.

### File List

- `src/hooks/useCopyFeedback.ts` (NEW)
- `src/App.tsx` (MODIFIED)
- `src/components/PreviewComments.tsx` (MODIFIED)

## Change Log

- 2026-08-21 (dev-story): Implemented all 6 tasks. `useCopyFeedback` hook (Task 1) wired into the five
  silent Copy sites in `App.tsx` (Task 2) and aligned `PreviewComments.tsx`'s existing confirmation to the
  shared `COPY_FEEDBACK_MS` (Task 3). Optimistic delete with an in-place undo strip, guarded single-commit
  backend call, and panel-close/unmount flush (Task 4). Refetch safety via flush-before-refetch in
  `loadHistory` and `handleHistorySearch` (Task 5). All verification gates green: `npm run build`, the
  three grep gates, and an 18/18 GATE-4 puppeteer smoke against the mock backend (Task 6). Status → review.
