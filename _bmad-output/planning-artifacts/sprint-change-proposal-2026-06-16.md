# Sprint Change Proposal — Story 9-5 Re-fashion against ADR-0019 / extended canon

**Date:** 2026-06-16
**Author:** Dev (correct-course)
**Trigger story:** 9-5 (Bubble state sequence + listening panel + waveform)
**Scope classification:** Moderate (story re-spec + epic AC update; re-implementation of an existing, partially-correct build)

---

## 1. Issue Summary

Story 9-5 was implemented and reviewed (2026-06-15), then the real-device smoke surfaced a
cross-platform **interaction-semantics defect**, not a rendering bug. As built, 9-5:

- **suppressed the bubble to a static idle squircle** during recording, and
- used the **red square in the panel as "Send"** (`stopAndProcessRecording`), with a separate
  neutral ✗ added later as cancel.

Andi flagged that this **inverts the desktop semantics**: on Windows the red square = **Cancel**
(`src/FloatingBar.tsx`, `cancelRecording`), and there was **no clear affordance for "how do I send"**.

This was root-caused and decided in **[ADR-0019](../../docs/adr/0019-cross-platform-design-ssot.md)**
(cross-platform design SSOT). The canon was extended (approved Option A, MANIFEST in-repo extension,
fingerprint `2bb99032…`) with a real bubble recording state. The decided interaction model:

- **red square = Abbrechen** (cancel/discard), identical on both platforms;
- **tap the bubble = Senden** (confirm → stop → transcribe → paste);
- the bubble gets a **recording state** (`.ab-bubble.recording`: teal gradient + amber pulse-ring +
  send-glyph) instead of staying idle.

The previously-committed 9-5 approach (suppress-to-idle bubble, red=stop, extra ✗) is therefore
**superseded** and must be re-fashioned against the extended canon. (Token-codegen, the other ADR-0019
precondition, already landed as Story 9-10 / `cef0e9c`.)

**Evidence:**
- ADR-0019 §Decision 3–4 and §Folgearbeiten (last bullet: "der committete 9-5-Stand … wird dadurch abgelöst").
- Canon `docs/design/overhaul/source/Klarvo Design System.html` lines 43–51 (`.ab-bubble.recording` +
  `@keyframes abbubblepulse`), 715–729 (RECORDING artboard: panel red square `aria-label="Abbrechen"`
  + recording-bubble with send-glyph), label line 729: *"Bubble bleibt sichtbar mit Amber-Puls +
  Send-Glyph (antippen = senden) · rotes Quadrat = abbrechen"*.
- 9-5 story Change Log entry 2026-06-16 (final): documents the supersession.

**Secondary issue (status hygiene):** the 9-5 story file frontmatter is still `Status: review` while
`sprint-status.yaml` already has it at `backlog` — a frontmatter↔tracker drift to be resolved as part
of the re-fashion.

---

## 2. Impact Analysis

### Epic Impact
- **Epic 9 (Android Visual Overhaul)** stays valid and on plan. No new/removed/resequenced epics.
- 9-5 remains "the big one"; this is a **scope correction within the story**, not an epic change.
- Downstream 9-6/9-7/9-8 are unaffected in intent. 9-7 (short-press gesture modes mirror desktop) and
  9-8 (long-press popover) now have a **clearer semantic anchor** (red=cancel, bubble-tap=send) — a
  benefit, not a conflict.

### Artifact Conflicts
| Artifact | Conflict | Action |
|---|---|---|
| `epics-visual-overhaul.md` Story 9.5 ACs (lines 623–648) | AC still says "a red **stop**"; no recording-bubble / tap-to-send / red=cancel semantics | **Update** the AC block to the ADR-0019 model. (Doc intro line 293 already references the rebuild.) |
| `9-5-…waveform.md` story file | Body = superseded model (suppress-to-idle, red=send, extra ✗); frontmatter `review` vs tracker `backlog` | **Re-fashion in place**: status→`backlog`; rewrite bubble-state + affordance ACs/tasks; mark old Dev Record superseded-but-informative; keep the still-correct detail (panel layout/tokens/coordinate math + the standing double-window fix). |
| `sprint-status.yaml` | 9-5 already `backlog` | **No change** (drift resolved by fixing the file frontmatter). |
| PRD / Architecture | No conflict (ADR-0019 IS the architecture decision; already Accepted). | N/A |

### Technical Impact (what stands vs what flips)
The build is **not** thrown away — the double-window defect fix and the panel are good code.

**Stands (keep):**
- `ListeningPanelView` as a **separate `TYPE_APPLICATION_OVERLAY` window** (panel ≠ inside bubble).
- Panel content per canon: grip, K-badge, amber live-dot + pulse, 5-bar amber waveform, timer,
  multiline mono transcript + amber caret, footer; TRANSCRIBING teal spinner + dimmed text; DONE
  checkmark→idle.
- The **double-window root-cause fix**: only ONE recording overlay form (the old HOLD-tap "expand to
  bar" window stays retired), and the **bubble window stays alive so taps reach `handleTouch`** — now
  load-bearing, because tap-to-send depends on it.
- Token values landed via 9-10 codegen (`KlarvoTheme.kt`).

**Flips (change):**
1. **Bubble recording visual:** `suppressedForPanel`→static-idle  ⟶  render `.ab-bubble.recording`
   (teal-gradient squircle + amber pulse-ring `abbubblepulse` 1400ms ease-out + **send-glyph**
   paper-plane `m22 2-7 20-4-9-9-4 20-7z`, ~20dp, `OnTeal` stroke) replacing the "K".
2. **Bubble tap = Senden:** bubble `ACTION_UP` (short tap in recording) → `stopAndProcessRecording()`.
   The primary confirm affordance.
3. **Panel red square = Abbrechen:** the `.ab-bar-stop` red square → `cancelRecording()` (discard, no
   paste). Inverts the old `stopAndProcessRecording` wiring. Parity with desktop red square.
4. **Remove the extra neutral ✗ button** (added in the 2026-06-16 double-window fix) — cancel is now the
   red square; send is the bubble-tap. Two distinct, correctly-coloured affordances.

No Rust/Tauri/Desktop files. (Desktop **parity check** is a separate ADR-0019 follow-up, not part of 9-5.)

---

## 3. Recommended Approach

**Option 1 — Direct Adjustment (chosen).** Re-fashion the existing 9-5 story in place and update the
epic AC; re-implement the four flips on top of the standing build.

- **Effort:** Medium. **Risk:** Low–Medium (touch-routing + per-transition state reset are the known
  traps; the postmortem already mapped them).
- **Why not rollback (Option 2):** the double-window defect fix and the panel are correct and load-
  bearing — reverting them would re-introduce the very bug just fixed. Rejected.
- **Why not MVP review (Option 3):** MVP unaffected; this is a within-story semantic correction.

This matches ADR-0019 §Mitigations step 3 ("Android 9-5 gegen die neue Spec neu fassen").

---

## 4. Detailed Change Proposals

### 4a. `epics-visual-overhaul.md` — Story 9.5 AC block (lines 623–648)

**OLD (excerpt):**
> **Then** a Klarvo-owned panel shows a grab handle, K + amber live-dot, a reactive RMS waveform, a timer,
> and a **red stop**; the footer reads "keyboard paused · returns on insert".

**NEW (add a recording-bubble + affordance-semantics AC; flip "red stop"→"red cancel"):**
> **Then** a Klarvo-owned panel shows a grab handle, K + amber live-dot, a reactive RMS waveform, a timer,
> and a **red square = Abbrechen (cancel/discard, parity with desktop)**; the footer reads "keyboard
> paused · returns on insert".
>
> **And** during recording the **bubble stays visible in its recording state** (`.ab-bubble.recording`:
> teal squircle + amber pulse-ring + send-glyph, NOT the idle K); **tapping the bubble = Senden**
> (stop → transcribe → paste). Confirm (bubble-tap) and Cancel (red square) are distinct affordances;
> **red is never the send/confirm action** (ADR-0019 colour-semantics rule).

(DONE-state AC unchanged. The "in-field live preview impossible by AR5a" inversion stays.)

### 4b. `9-5-…waveform.md` — re-fashion in place

- Frontmatter `Status: review` → **`Status: backlog`** (resolves the tracker drift).
- Insert a **"Superseded approach (do NOT rebuild)"** note + a **"Stands vs Flips"** table (§2 above) at
  the top of Dev Notes so the dev agent knows the build is a *modification*, not greenfield.
- Rewrite the bubble-related ACs/Tasks:
  - **AC1**: bubble renders `.ab-bubble.recording` (not suppressed-idle); red square labelled
    **Abbrechen** wired to `cancelRecording()`.
  - **New AC (Send/Cancel semantics)**: bubble short-tap in recording → `stopAndProcessRecording()`
    (Send); red square → `cancelRecording()` (Cancel); remove the neutral ✗; inversion gate: *red wired
    to send = review failure; bubble staying idle during recording = review failure*.
  - Keep AC2 (no in-field preview / AR5a), AC3 (transcribing teal, amber absent), AC4 (done), AC5
    (separate overlay window), AC6 (9.4 harness) — these stood.
- Old Dev Agent Record / completed-task checkboxes: keep as **historical** under a clear
  "— superseded 2026-06-16, retained for context —" header; new tasks tracked fresh.
- Canon refs: add HTML lines 43–51 (`.ab-bubble.recording`, `abbubblepulse`) and 727/729 (send-glyph +
  affordance labels) to the binding-values list.

### 4c. `sprint-status.yaml`
- No edit (9-5 already `backlog`). Drift is closed by 4b's frontmatter flip.

---

## 5. Implementation Handoff

- **Scope:** Moderate → **Developer agent** executes the re-implementation; the re-fashioned 9-5 story
  file is the spec (no separate create-story needed — the standing-code context must be preserved).
- **Sequence:** (1) apply 4a + 4b + commit the spec edits → (2) implement the four flips
  (`bmad-dev-story` or `bmad-story-conductor 9-5`) → (3) **GATE 4 = Andi's real device** across modes
  (PTT hold/release, HOLD-tap, TOGGLE, AUTOSTOP, AUTO → recording→transcribing→done). **Emulator is not
  a visual oracle (E9).**
- **Known traps to carry (from postmortem):** touch-stream capture on the bubble (ACTION_DOWN→UP),
  per-transition `alpha=1.0f` reset must not un-suppress/override the recording form, MIUI harness
  liveness (the DEBUG_SET_STATE harness is dead on MIUI — separate harness story still owed; real-device
  drive remains manual).
- **Success criteria:** on Andi's device, recording shows the amber-pulsing send-bubble + panel with a
  red **Abbrechen** square; tapping the bubble sends (cleaned text lands), the red square cancels
  (nothing pasted); no double overlay; matches desktop red=cancel semantics.

---

## Change Log
- 2026-06-16: Proposal created (correct-course). Trigger: ADR-0019 supersedes the committed 9-5
  interaction model. Recommended path: Direct Adjustment (re-fashion in place). Awaiting Andi approval.
