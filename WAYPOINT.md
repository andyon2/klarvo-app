# WAYPOINT — Klarvo (v1-ship)

_Last session: 2026-06-07. Written for a context-less BMAD session to continue with zero chat history._

## Resume — next BMAD action

**Story 6-6 (preview-box appearance, REDESIGN / Increment A) is functionally complete and confirmed working on the real Windows build by Andi** — themes, visual colour pickers + opacity, font-family dropdown, in-panel live preview card, and Save/persistence all good. It is **NOT done**: one open **cosmetic** blocker remains — the preview box's **bottom-left corner renders rough/jagged** on the real build (top corners are clean → asymmetric). `sprint-status.yaml` has `6-6 = review`.

**Next action:** resume the 6-6 surface-smoke fix for the corner artifact — **observe-first, do NOT guess-and-rebuild.** This is a hard rule now (`project-context.md` rule 29 "never make the user the rendering oracle" + the conductor skill's GATE 4). Concretely: get an *observable* isolation of the corner first — a zoomed screenshot from Andi, or a one-flip reproduction — and **name the cause BEFORE any code change**; then route the fix through a fresh `bmad-dev-story` / `bmad-quick-dev` worker, re-review, re-smoke. **One observed cause → one change.**

Alternatively, Andi may consciously **defer** the corner and pick the next Epic-6 story instead: `6-3` (font-SIZE axis = Increment B of the *same* panel) or `6-4` / `6-5` (all `backlog`, depend on 6-2 which is done).

## Open from last session

- **6-6 corner artifact — cosmetic, OPEN, parked.** Bottom-left corner of the preview box renders rough on the real build; top corners clean. **Ruled out as causes (PROVEN — do not retry):** (a) the GDI window region `set_preview_shape` — a *freshly created* preview window with no region ever set still showed it (verified via the Klarvo.log process-start at 15:53 + the `[preview] shown` line); (b) `backdrop-filter: blur` — disabled, still rough. Code baseline = commit **`0406104`** (both speculative experiments were reverted to keep a clean, known-good tree). The artifact is asymmetric (bottom-left only), which argues *against* generic anti-aliasing. Next attempt must start from an observed cause, not a hypothesis.
- **Save-chain bug — FIXED (commit `0406104`), confirmed by Andi.** The third hop `SettingsPanel.onSave → useSettings.handleSaveSettings → saveSettings` was dropping the 7 appearance fields (the hook neither declared nor forwarded them) → Save reset everything to defaults. Already recorded in the 6-6 story file + sprint-status note. No action needed.
- **Anti-"test-machine" control — ADDED this session, durable, no action needed.** `project-context.md` rule 29 (commit `ff8aa97`, loaded as a `persistent_fact` by dev/quick-dev/create-story/code-review/checkpoint-preview) + the conductor skill's **GATE 4** (post-smoke: a failed surface smoke re-opens the story and demands observe-first, never a bare-loop hot-patch; two failed smokes ⇒ escalate for a method change). A `PreToolUse` hook was considered and rejected by Andi as too invasive.

## Context / why

- 6-6 was **redesigned** this session (commit `449b83d`) after the first text-input version failed Andi's smoke (raw rgba/hex inputs were unusable, no live feedback, and a separate save bug made it look like "no effect"). Confirmed design decisions: **themes-first + visual pickers + live in-panel card**, **global** styling (not per-width-preset), **one coherent panel**. Font-SIZE was deliberately folded OUT to the re-scoped Story **6-3** (Increment B of the same panel, to be built *after* 6-6's smoke is GREEN — it touches geometry/`k`, a higher risk class).
- The corner is the **only** thing between 6-6 and `done`. Everything else is confirmed working on the real build.
- New surface-debugging workflow for Andi (Windows) / Claude (WSL): a desktop shortcut **"Klarvo Win Dev"** now runs `dev.ps1 -SkipNpm` = `tauri dev` (debug + Vite HMR) for **frontend-only** iteration; `dev.ps1` syncs WSL→`D:\apps\klarvo` once at start, so a WSL edit needs **"Klarvo Win Sync"** to reach the running build. Use the release `sync-and-build.ps1` ("Klarvo Win Rebuild") only when Rust/Tauri/signing is involved.

## Local commits this session (unpushed, branch `v1-ship`)

```
ff8aa97 docs(project-context): add "never make the user the rendering oracle" anti-pattern (rule 29)
0406104 fix(6-6): forward preview-appearance fields through handleSaveSettings hook
449b83d feat(6-6): redesign preview-box appearance panel — themes + visual pickers + live card
c61534d feat(bar-redesign): Story 6.6 — preview-box appearance customization (FR11/12/13)
```

## Consistency

Green. `sprint-status.yaml` (authority): `6-6 = review` (correct — not done, corner open); `6-1`/`6-2` done; `6-3`/`6-4`/`6-5` backlog. No drift vs `epics-bar-redesign.md` (it carries no per-story cosmetic `Status:` markers). No BMAD parsed contract was written by this waypoint.
