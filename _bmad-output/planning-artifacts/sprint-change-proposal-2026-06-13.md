# Sprint Change Proposal — 2026-06-13

**Workflow:** bmad-correct-course · **Mode:** Incremental · **Author:** Andi (+ agent)
**Change scope classification:** **Moderate** (backlog reorganization — defer one in-flight epic, add two new epics; no rollback, no replan of completed work).

---

## Section 1 — Issue Summary

A high-fidelity **visual overhaul handoff** ("Studio Dark") arrived from the cloud web-design agent and was reviewed jointly. It is a complete design direction — final color/type/spacing/radii/elevation/motion tokens, per-surface specs, and an Android dictation-bubble **interaction** redesign — delivered as tracked specs in `docs/design/overhaul/` (`SPEC-studio-dark-overhaul.md` + `00..04-*.md`; the HTML/CSS prototypes and screenshots stay local in the gitignored `design-handoff/`).

This is a deliberate **strategic re-prioritization mid-sprint**, not a bug surfaced by a story. The product owner wants the visual overhaul to land **before** the remaining (invisible) cross-platform parity plumbing.

**Evidence / requirements source** (this brownfield-v1 track runs without a PRD by design — same as Epics 5/6/7, each routed via correct-course off a requirements source):
- Tracked design spec `docs/design/overhaul/SPEC-studio-dark-overhaul.md` (binding tokens, surfaces, states, constraints).
- Current-code gap: `src/styles.css` has a 69-line `@theme` block but **317 scattered inline hex** across components ("not from one cast" feeling). Android overlay is **classic View + Canvas `onDraw`** (`FloatingBubbleView.kt` 478 LOC, `KlarvoOverlayService.kt` 1482 LOC) — **no Compose**, and **no Android color/theme file exists yet**.

---

## Section 2 — Impact Analysis

### Epic Impact
- **Epic 7 (Cross-Platform Parity, in-progress):** trigger-independent. 7-3 (shared-core STT via JNI) is **done** (the heavy centerpiece). Five stories remain (7-1, 7-2, 7-5, 7-6, 7-7). The epic can technically be completed as planned but is **deliberately parked**, not modified. On resume: 7-1 first, **7-7 is the capstone that runs last** (it locks 7.1–7.6 + the dead-config landmines).
- **Epics 1–6:** all done. No impact.
- **New work:** does **not** make any planned epic obsolete — Epic 7 is deferred, not cancelled.

### New Epics
- **Epic 8 — Desktop Visual Overhaul ("Studio Dark").** Tauri/React/Tailwind v4. Foundation story = token-ladder consolidation (69-line `@theme` + 317 inline hex → named graphite/teal/amber tokens), then surface re-skins: FloatingBar (signature, transparent-window constraints), Settings-Home + ~9 sub-pages (custom select/segmented/toggle/slider), Live-Cleanup-Preview, History. Pure visual redesign. **Surface-class smoke DoD** (Windows release build + objective pixel metric + observability loop; human visual gate — never make the user the rendering oracle).
- **Epic 9 — Android Visual Overhaul + Bubble Interaction.** Native Kotlin (View+Canvas today, no Compose). Foundation = create the Android token/theme source. Then **bubble behavior** (feature work, not re-skin): state sequence idle→recording→transcribing→done with a Klarvo-owned listening panel + reactive RMS waveform; keyboard-collapse via the a11y service (optional, with fallback); long-press popover that **remaps long-press from today's push-to-talk → menu** (4 gesture modes × 3 cleanup modes). Hard IME constraints (no in-field preview text from an overlay; no native backdrop-blur). **Verifiability-symmetry:** a state harness belongs in scope — the user cannot otherwise produce the bubble states to test them. **On-device smoke DoD** (`scripts/android-smoke.sh`).

### Artifact Conflicts
- **PRD:** N/A (brownfield-v1, no PRD by design).
- **Architecture:** one **load-bearing open decision** → ADR precursor for Epic 9: **Android bubble rendering tech — extend View+Canvas vs. introduce ComposeView**. No other architectural conflicts (re-skin + localized bubble logic). Note: bubble changes that touch shared config keys must mirror Rust ↔ Kotlin (ADR-0016) — applies if the gesture/mode remap reads config.
- **UI/UX:** the handoff **is** the UX spec → no new `bmad-create-ux-design` run.
- **Secondary:** token consolidation has wide blast radius (visual regressions); Epic 9 may need a new Android UI smoke step. No CI/deploy/telemetry changes (BYOK — no telemetry UI).

---

## Section 3 — Recommended Approach

**Selected: Hybrid (Direct Adjustment of the plan, no rollback, no MVP redefinition).**
Defer Epic 7 **and** insert two new epics (8 Desktop, 9 Android), resequenced ahead of Epic 7.

- Option 1 (Direct Adjustment only): insufficient — this is genuinely new epic-scale work, not a story tweak.
- Option 2 (Rollback): N/A — nothing to roll back; 7-3 stays.
- Option 3 (MVP review): N/A.

**Rationale:** separate codebases / risk profiles / human-test surfaces (don't bundle); Epic-6 precedent for treating a visual overhaul as a full L3 epic; lean planning path (handoff = UX, no new PRD/UX workflow). Parking Epic 7 is low-risk because its remaining state is fully documented and 7-7 is explicitly designed to run last whenever the epic resumes.

**Effort / risk:** Epic 8 — medium breadth, low per-item risk, gated by visual smoke. Epic 9 — medium-high (real interaction work + IME constraints + state harness). Timeline impact: Epic 7's remaining tail (notably 7-2 realtime + 7-7 cross-language CI net) is pushed out, accepted.

---

## Section 4 — Detailed Change Proposals (applied)

1. **`_bmad-output/implementation-artifacts/sprint-status.yaml`** — Epic 7 header marked `⏸ PARKED 2026-06-13` (status stays `in-progress`, honest — 7-3 done); added `epic-8: backlog` and `epic-9: backlog`; added a top-level routing comment. *(applied)*
2. **`_bmad-output/planning-artifacts/epics-cross-platform-parity.md`** — one-line PARKED note in the Overview (resume order 7-1 first, 7-7 last). *(applied)*
3. **`_bmad/custom/bmad-sprint-status.toml`** — routing hook rewritten: ignore the bare cascade (do **not** route to 7-1); live strand = Epic 8 → Epic 9; next action = `bmad-create-epics-and-stories` for Epics 8/9; Epic 7 resume note. *(applied — critical: this is the only lever that overrides the deterministic cascade for the next fresh session.)*

**Not done by CC (by design):** the Epic 8/9 story breakdown. Recommended home = a single new planning artifact `epics-visual-overhaul.md` (both epics; `inputDocuments` = the tracked design specs), authored by `bmad-create-epics-and-stories`.

---

## Section 5 — Implementation Handoff

- **Next workflow:** `bmad-create-epics-and-stories` → produce `epics-visual-overhaul.md` (Epic 8 + Epic 9), UX input = `docs/design/overhaul/`. Epic 9's first story (or precursor) = **ADR: Android bubble rendering tech (View+Canvas vs ComposeView)**.
- **Then per story (L3):** `bmad-create-story` → `bmad-testarch-atdd` (where applicable) → `bmad-dev-story` → `bmad-code-review`. Drivable autonomously via `bmad-story-conductor`. No ultracode/Workflow fan-out for now (revisit for planning/review fan-out later if desired).
- **DoD reminders:** Epic 8 = Windows release build + objective pixel metric + surface-smoke-checklist; Epic 9 = on-device smoke + bubble state harness (verifiability symmetry).
- **Success criteria:** Studio-Dark tokens consolidated and applied across both platforms at handoff fidelity; Android bubble behavior matches the spec'd state sequence + long-press popover; Epic 7 cleanly resumable.

**Scope classification: Moderate** → route to Product Owner / Developer (backlog reorganization + create-epics-and-stories), not a PM/Architect replan.
