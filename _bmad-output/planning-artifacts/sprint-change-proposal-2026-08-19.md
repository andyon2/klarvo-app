# Sprint Change Proposal — Click feedback for Copy and Delete

Date: 2026-08-19 · Author: Correct Course workflow · Requested by: Andi
Mode: Batch (small change surface — all edits presented together for one approval)

---

## 1. Issue Summary

**Trigger story:** 8-5 (Main-Window / History re-skin), during Andi's Windows smoke on 2026-08-19 —
the first run of the freshly built `klarvo.exe` containing 8-5.

**Issue type:** New requirement emerged from stakeholder review. Not a defect in 8-5: every 8-5
acceptance criterion holds. The re-skin exposed a pre-existing interaction gap that the old visual
noise had masked.

**Problem statement.** The history actions give the user no response. Two distinct problems:

1. **Copy is silent.** The user clicks, the text lands in the clipboard, and nothing on screen says
   so. The user cannot tell success from a dead button.
2. **Delete is silent AND irreversible.** `delete_history_entry` issues a hard SQL `DELETE`. There is
   no confirmation, no feedback, and no way back. One mis-click costs a dictation permanently.

**Evidence (verified against the tree, 2026-08-19).**

| Finding | Location |
|---|---|
| 7 `clipboard.writeText` call sites, none with feedback | `src/App.tsx`, `src/components/VoiceNotesPanel.tsx`, `src/components/PreviewComments.tsx` |
| Hard delete, no soft-delete column | `src-tauri/src/commands/history.rs:68` → `history::delete_entry` |
| Android twin also silent | `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` (ClipboardManager) |
| Two copy handlers lack even a `.catch` | `src/App.tsx:714`, `src/App.tsx:727` |

**Design decisions already settled by Andi at the gate (2026-08-19).** These are human decisions, not
defaults, and the implementing story must not re-open them:

- **Copy** → the button label changes to "Copied" for a short moment after the click.
- **Delete** → an **undo window** ("Deleted · Undo", a few seconds), *not* a confirmation prompt
  beforehand. Rationale: it forgives the mis-click without taxing every intentional delete.

---

## 2. Impact Analysis

### Epic Impact

**Epic 8 can still be completed as planned.** No story is invalidated, no resequencing is needed,
8-1/8-2/8-5 stay done, and 8-6 (Onboarding) remains the live next surface.

**But one scope covenant must be amended, explicitly.** Epic 8 defines itself as a re-skin:

> "identical functions and flows, but now 'from one cast'" — Epic 8 overview
> "behavior and IA are unchanged (re-skin only)" — Story 8.6 AC

Click feedback is an interaction addition, and the undo window is genuinely new behaviour. Filing
this under Epic 8 without saying so would quietly break the epic's own definition and make a later
reader mistrust the covenant everywhere else. **Recommendation: amend Epic 8 narrowly** — it gains
exactly one interaction-affordance story, named as such, and the re-skin covenant continues to hold
for every other story in the epic.

**Epic 9 (Android) is touched but NOT in this story.** The Android clipboard twin has the same gap.
The existing epic boundary is Epic 8 = desktop, Epic 9 = Android, and Andi's report was about the
desktop history. Forcing the Kotlin twin into a desktop story would cross that boundary for no gain.
It goes to `docs/backlog.md` with a source reference, per backlog discipline.

No other epic is affected. No epic becomes obsolete. No new epic is needed.

### Artifact Conflicts

| Artifact | Impact |
|---|---|
| PRD | No separate PRD exists in this project. The requirements authority for this strand is the **Requirements Inventory** in `epics-visual-overhaul.md`. |
| Requirements Inventory | **UX-DR4** covers density, mono timestamps, amber tags, empty-states, search affordances. It does **not** cover action feedback. → add **UX-DR6**. |
| `epics-visual-overhaul.md` | Add Story 8.8 + the narrow epic-scope amendment. |
| `sprint-status.yaml` | Add one entry: `8-8-action-feedback-copy-and-delete: backlog`. |
| Architecture (`docs/ARCHITECTURE.md`, ADRs) | **No impact** — see the implementation note below. |
| `docs/backlog.md` | Add the deferred Android twin, with a source reference to this proposal. |
| Design canon (`docs/design/overhaul/source/`) | To be checked by the story: if the canon carries no state for a confirmed action, that token decision is a GATE-1 question for Andi, not an invention by the implementer. |

### Technical Impact

**The undo window needs no schema change and no Rust change.** The simplest shape that satisfies the
decision is an **optimistic delete in the frontend**: hide the row, hold the entry in React state,
and call `delete_history_entry` only once the undo window expires. Undo restores the row and never
calls the backend.

The failure mode points the safe way: if the app closes inside the window, the delete simply never
happens and the entry survives. Nothing is lost. A soft-delete column in `history.db` would be the
heavier alternative, would need a purge path, and would drag in the Android store — rejected as
premature.

Consequence: **this story is React-only.** No ADR is affected, no state-file writer is added, no
cross-platform twin is touched.

---

## 3. Recommended Approach

**Option 1 — Direct Adjustment. Selected.**
Add one story inside the existing epic structure. Effort: **Low**. Risk: **Low**. No timeline impact
on 8-6.

**Option 2 — Rollback: not viable and not needed.** Story 8-5 met every acceptance criterion; its
GATE-4 evidence stands. The gap is additive, not a regression. Reverting would destroy verified work
to solve nothing.

**Option 3 — MVP review: not applicable.** No MVP goal is threatened. The change adds polish to a
shipped surface.

**Justification.** The change is small, its design is already decided by the human, and it needs no
architectural movement. The only thing that genuinely requires a decision is the epic-scope
amendment — and that is a two-line honesty fix, not a replan.

---

## 4. Detailed Change Proposals

### 4.1 — New Story 8.8 (in `epics-visual-overhaul.md`, after Story 8.6)

```
### Story 8.8: Action feedback — Copy and Delete (interaction affordance)

As a user acting on a history entry,
I want the app to answer my click,
So that I know the copy succeeded and a mis-click on Delete does not cost me a dictation.

**Acceptance Criteria:**

**Given** any Copy affordance on a desktop surface
**When** the user clicks it
**Then** the control confirms the action ("Copied") for a short, self-clearing moment, and returns to
its resting label afterwards.

**Given** the clipboard write fails
**When** the user clicks Copy
**Then** the control does NOT claim success, and the failure is visible to the user — not only in the
console.

**Given** a history entry
**When** the user clicks Delete
**Then** the row disappears immediately and an undo affordance ("Deleted · Undo") is offered for a
few seconds; the backend delete runs only after that window expires.

**Given** the undo affordance
**When** the user clicks Undo inside the window
**Then** the entry returns to the list unchanged and no backend delete is issued.

**Given** the covered surfaces
**When** the story is done
**Then** every desktop `clipboard.writeText` call site carries the feedback (7 sites at the time of
writing, in App.tsx / VoiceNotesPanel.tsx / PreviewComments.tsx) — no site is left silent.

**And** zero inline hex for covered roles (DT1); the feedback state uses named tokens. If the design
canon carries no token for a confirmed-action state, that is a GATE-1 question for Andi, not an
implementer's choice.

**Scope guard:** desktop only. The Android clipboard twin is deferred to Epic 9 (see backlog).
This story adds NO SQLite schema change and NO Rust change — the undo window is an optimistic
frontend delete.

**DoD:** Windows release build + smoke (copy from a history card and confirm the label changes;
delete an entry and undo it; delete an entry and let the window lapse, then confirm it is gone after
a restart); `tsc`/`vite` green.
```

### 4.2 — Epic 8 scope amendment (in `epics-visual-overhaul.md`, Epic 8 overview)

```
OLD:
Standalone: builds only on existing v1 desktop surfaces; enables nothing downstream but retires the
visual-inconsistency debt.

NEW:
Standalone: builds only on existing v1 desktop surfaces; enables nothing downstream but retires the
visual-inconsistency debt.

**Amendment 2026-08-19 (scope).** Epic 8 is a re-skin — "identical functions and flows" holds for
every story EXCEPT 8.8. Story 8.8 adds interaction behaviour (action feedback + an undo window),
admitted deliberately after Andi's 8-5 smoke found that the re-skinned surfaces answer no clicks.
The re-skin covenant continues to bind 8.1-8.7.
```

### 4.3 — New requirement UX-DR6 (in `epics-visual-overhaul.md`, UX Design Requirements)

```
- **UX-DR6** — **Action feedback (desktop)**: every user-triggered action on a surface answers the
  click. Copy confirms ("Copied") and self-clears; a destructive action (Delete) is recoverable for a
  short window ("Deleted · Undo") rather than gated behind a prompt. Discovered 2026-08-19 at the 8-5
  smoke; covered by Story 8.8.
```

### 4.4 — `sprint-status.yaml`

```
OLD:
  8-7-studio-dark-fidelity-pass: backlog
  epic-8-retrospective: optional

NEW:
  8-7-studio-dark-fidelity-pass: backlog
  8-8-action-feedback-copy-and-delete: backlog
  epic-8-retrospective: optional
```

### 4.5 — `docs/backlog.md` (deferred item)

```
- **Android clipboard twin has no copy feedback.** `KlarvoOverlayService.kt` copies to the
  ClipboardManager with no on-screen confirmation — the same gap Story 8.8 closes on desktop.
  Belongs to Epic 9 (Android), not Epic 8. Source: sprint-change-proposal-2026-08-19.md.
```

---

## 5. Implementation Handoff

**Scope classification: Moderate.** The code change is Minor, but it carries a backlog change (one
new story) and an epic-scope amendment, so it needs the PO/DEV path rather than a direct dev handoff.

| Recipient | Responsibility |
|---|---|
| Correct Course (this workflow) | Apply 4.2–4.5 on approval: epic amendment, UX-DR6, sprint-status entry, backlog deferral. |
| `bmad-create-story` | Expand Story 8.8 into the full story file. **Carry Andi's two settled design decisions in verbatim** — they are decided, not open. |
| `bmad-dev-story` | Implement. React only. |
| Andi | GATE-4: the Windows smoke named in the DoD. The feel of the timing — how long "Copied" lingers, how long Undo stays — is his call, not a measured one. |

**Success criteria.** No Copy affordance on a desktop surface is silent. A deleted entry is
recoverable for the length of the undo window. Story 8-5 stays closed and untouched.

**Sequencing.** Independent of 8-6. It can run before, after, or beside it. It depends only on 8-1
(token foundation), which is done.
