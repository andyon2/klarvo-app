# Surface-Story Pre-Smoke Trap Checklist

**Mechanical DoD control for surface/UI stories** (Tauri shell, FloatingBar, Settings panel,
React frontend). Born from Epic 5 (Live-Cleanup-Preview), where **every functional bug surfaced
only in the Windows smoke and was green on Linux `cargo test` + `tsc`**. This is the feature-epic
analog of the reviewer-mechanical-inversion control (Epic-4 retro AI-1): a checklist someone runs,
not a claim someone makes.

> **When this applies:** any story whose DoD already requires a Windows release build + manual
> press-to-paste smoke (the `project-context.md` testing rule). Run this list *before* the smoke,
> as part of writing the code — each item is a known trap that Linux-green hides.

> **Epic 6 update (2026-06-08):** the re-architecture epic re-confirmed traps #1–#5 *and* added
> trap #6 (multi-hop save-chain forwarding) — a class that both automated review layers missed
> because they verified the chain's endpoints but not its middle hop. The ledger grows by exactly
> this mechanism: every new smoke-only bug class gets appended.

## The traps (each one shipped at least once in Epic 5 or 6)

1. **camelCase config keys.** `AppConfig` uses `serde(rename_all = "camelCase")` → the JSON key is
   `livePreviewEnabled`, **not** `live_preview_enabled`. A wrong-case key is **silently ignored**
   by serde → the feature is silently off and the smoke shows "nothing happens." Verify every new
   config key's on-disk spelling against the serde rename, and quote camelCase in any doc/manual
   `config.json` edit. (Epic 5, Story 5-2 — root cause of "no chunks".)

2. **New float/Settings field not in the resync `useEffect`.** A newly added numeric/settings field
   must be added to the `loadedSettings` resync `useEffect` in the Settings panel, or the Save
   button stays dirty forever (f32→f64 serde-widening mismatch leaves the form "changed"). Add the
   field to the resync list when you add it to the schema. (Epic 5, Story 5-3 — stuck-dirty Save.)

3. **FloatingBar is a SEPARATE Tauri window that does NOT re-mount on settings-save.** A
   `getSettings` call that runs only on mount freezes on the app-start value — a saved setting stays
   inert until app restart. Any bar-read settings field needs a **reactive re-read** (re-read on the
   panel closed→open transition, or via a backend settings-changed event), not a mount-only load.
   (Epic 5, Story 5-5 — saved preset inert until restart, caught by all 3 review layers.)

4. **Window geometry / shape region must match the dynamic content and be re-asserted.** The bar
   window's `setSize` + `set_bar_shape`/region must track the *actual* rendered size, not a hardcoded
   pill rectangle, or the panel is clipped (right edge / top). Cold first-expansion and grow-upward
   resize races under-apply the first async `setSize` → re-assert geometry on rAF + a short delay
   after open. Probe `PANEL_WIDTH - 2` for the wrapper border so the last line isn't clipped.
   (Epic 5, Stories 5-2 and 5-5 — multiple smoke-fix rounds for region clip + resize race.)

5. **Push, not poll, and wire the event end-to-end.** New shell events use the colon form
   (`klarvo://...`, never dots — Tauri reserves `.`). Confirm the producer emits and the consumer is
   subscribed (re-enabled as a push sink, not a re-enabled poller). A green unit test does not prove
   the event reaches the window. (Epic 5, Story 5-1/5-2 — `klarvo://live-preview-chunk`.)

6. **Multi-hop save/plumbing chains — trace the whole chain, not just the endpoints.** When a
   surface story adds a config field, it is plumbed through a long chain
   (`config → patch → merge → save_config_locked → getSettings → SettingsView → TS type → MOCK →
   SettingsPanel state/resync/isDirty → onSave → useSettings.handleSaveSettings → saveSettings →
   PreviewPanel reactive read`). A field can be declared correctly at **both ends** (the panel's
   `onSave` and the `saveSettings` signature) yet be **dropped in an intermediate hop** — in Epic 6
   the `useSettings.handleSaveSettings` hook neither declared nor forwarded the 7 appearance args, so
   Save silently nulled them → merge kept defaults → every appearance setting reset on save. **Both
   automated review layers missed it** because they checked the endpoints and never traced the middle
   hop; Linux-green never sees it. For any new config field, walk the *entire* chain end-to-end and
   confirm each hop forwards the field. (Epic 6, Story 6-6 — appearance reset-on-save.)

## How to use it

- **At create-story / dev time:** for a surface story, copy the applicable items into the story's
  DoD as explicit pre-smoke checks. Not every item applies to every story — pick the ones the
  story's surface touches (new config key → #1/#3/#6; new Settings numeric → #2; bar geometry → #4;
  new event → #5; any new config field plumbed through the Settings save chain → #6).
- **The checklist is mechanical, not a self-attestation.** "I checked #1" is worth nothing; the
  value is in actually verifying the on-disk key spelling, the resync list membership, the reactive
  re-read, the region match. Same lesson as reviewer-inversion: the control has teeth only when the
  check is executed, not claimed.
- **Add to it.** When a new surface trap is found only in a Windows smoke, append it here with the
  story ref. This file is the running ledger of "things Linux-green hides."

## Why a checklist and not CI

Same reasoning that rejected mutation-CI in Epic-4 retro: this is a no-user (Early-Access-withdrawn)
codebase, the WSL-build → Windows-smoke loop is manual by design (`scripts/sync-and-build.ps1`),
and the failure mode is "surfaced in the smoke, recoverable in one fix round" — not a shipped
regression. A lightweight human checklist matches the actual risk; a Windows-in-CI harness would be
real cost against a problem the manual smoke already catches.
