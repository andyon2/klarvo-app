---
story: "12.2"
epic: "12"
title: "Audio-retry history (primitive A + manual re-process)"
status: review
track: L3-feature
gatedBy: ["12.1"]
buildsOn: ["12.1"]
enabledBy: ["12.3"]
inputDocuments:
  - docs/backlog.md#Epic 12 — Cloud-Resilienz
  - _bmad-output/planning-artifacts/epics-cloud-resilience.md
  - _bmad-output/project-context.md
---

# Story 12.2: Audio-retry history (primitive A + manual re-process)

Status: review

> **Epic 12 — Cloud-Resilienz.** Story 12-1 made the fallback ladder robust and, as its last-resort
> safety net, **preserves the raw WAV to disk** when transcription fails terminally (no text could be
> produced after the ladder), so the recording is never silently lost. But today that preserved audio
> is invisible and unreachable: it sits in `{app_data}/pending/*.wav` with no history entry and no way
> to act on it. This story makes a terminally-failed dictation **visible in the history and manually
> re-processable** once the cloud is reachable again. It is "primitive A": transient audio retention
> (the WAV is deleted on a successful re-process) with a manual re-process action — deliberately built
> on a **B-capable data model** (status field + audio-as-file) so Story 12-3 (provider comparison,
> deferred) can sit on it later without a schema rebuild. No PRD/Architecture/UX document exists for
> this epic; requirements come from the decision-complete `docs/backlog.md` "Epic 12" section and the
> code audit in `epics-cloud-resilience.md`.

## Story

As a Klarvo user whose dictation failed because the cloud was unreachable,
I want the failed recording to appear in my history as a "pending" entry that I can re-process (or
discard) once I'm back online,
so that a cloud outage never permanently costs me a recording, and I stay in control of the saved
audio.

## Design decisions (Andi, 2026-07-06 — binding, do not re-litigate)

Epic 12 is a **reliability epic with no visual design canon** (prose-only; the status-message wording
is a copy detail, not a design gate). The one settled UI/intent decision:

1. **Pending entries live in the dictation history** (the `is_note = 0` history view, NOT the Voice
   Notes panel), inline with normal entries, visually marked as pending (amber treatment). Placeholder
   text stands in for the missing transcript, e.g. `⏳ Audio gesichert — noch nicht transkribiert`
   plus a short reason line and the timestamp. (Exact wording is a copy detail, not binding.)
2. **A pending entry offers exactly two actions: „Erneut verarbeiten" (re-process) and „Verwerfen"
   (discard).**
   - **Erneut verarbeiten** → re-runs STT + cleanup on the stored WAV. While running, the action shows
     a busy/disabled state. On **success** the entry becomes a **normal transcription entry** (status
     `done`, transcript filled in) and the **stored WAV is deleted** (A-retention = transient). On
     **failure** it stays `pending` with a brief inline error and the WAV is kept.
   - **Verwerfen** → deletes the history entry **and** its stored WAV.
3. **Audio retention is transient (A).** The WAV is kept only until a successful re-process or an
   explicit discard. There is **no** "keep audio" option and **no** durable retention in this story —
   that is 12-3 territory. Compression is likewise out of scope (raw WAV for now).
4. **Out of scope (per plan):** automatic background retry, permanent audio retention, the
   provider/settings comparison UI (all 12-3).

## Acceptance Criteria

**AC1 — B-capable schema, migrated backward-compatibly.** Given an existing `history.db` with the
current columns, When the app opens the DB, Then the `history` table has two new columns —
`status TEXT NOT NULL DEFAULT 'done'` (values: `pending` | `done` | `failed`) and
`audio_path TEXT` (nullable) — added via an additive `ALTER TABLE` migration in the same idempotent
style as the existing `is_note` migration (`history/mod.rs:137-140`), such that every pre-existing row
is preserved verbatim and reads back as `status = 'done'`, `audio_path = NULL`. No destructive
migration; old DBs open without data loss. (Machine-testable — G-A.)

**AC2 — A terminal STT failure creates a pending history entry (Windows).** Given a dictation fails
terminally (STT could not produce text after the ladder — the path that already calls
`save_pending_wav`, `pipeline.rs:1153 / 1161 / 1181`), When the WAV is persisted, Then a history row is
created with `status = 'pending'`, `audio_path` = the saved WAV path, `text` empty/placeholder,
`is_note = 0`, so the failed recording is discoverable in the dictation history. (The WAV persistence
itself already exists from 12-1; this AC wires the accompanying history entry.) (Machine-testable — G-A.)

**AC3 — Android persists the WAV and creates the pending entry (Android).** Given the same terminal
STT failure on Android, When the failure is reached (native Kotlin path in `KlarvoOverlayService.kt`),
Then the raw WAV is persisted to disk **and** a `pending` history row (same shape as AC2) is created —
mirroring the Windows behaviour. NOTE (cross-platform reality, verify at build): a `savePendingWav`
equivalent does **not** currently exist in the Kotlin sources (grep found none as of authoring), so on
Android both the persistence and the entry are net-new here. Terminal state on both platforms = never
a silent loss.

**AC4 — Pending entries render distinctly with both actions (surface, Windows).** Given the dictation
history view (`is_note = 0`) contains a `pending` entry, When the user opens history, Then that entry
is visually marked as pending (amber), shows placeholder text + reason + timestamp instead of a
transcript, and exposes exactly two actions: **Erneut verarbeiten** and **Verwerfen** (per Design
Decision 2). Normal (`done`) entries are unchanged. (Surface — Windows visual verdict is Andi's
real-machine gate, G-B/E11.)

**AC5 — Re-process re-runs STT+cleanup, promotes on success, deletes the WAV.** Given a `pending`
entry with a stored WAV and a reachable cloud, When the user triggers **Erneut verarbeiten**, Then the
stored WAV is re-run through STT + cleanup; on success the entry is updated in place to `status = 'done'`
with the produced transcript, and the stored WAV file is **deleted**; on failure the entry stays
`pending` with a brief inline error and the WAV is retained. The action shows a busy/disabled state
while running. (Logic machine-testable — G-A; the UI feedback is surface.)

**AC6 — Discard deletes entry and WAV.** Given a `pending` entry, When the user triggers **Verwerfen**,
Then both the history row and its stored WAV file are deleted. If the WAV is already gone, the row is
still removed without error. (Machine-testable — G-A.)

**AC7 — Happy-path parity / no regression.** Given a dictation that succeeds normally, When it is
stored in history, Then behaviour and output are byte-identical to today (`status = 'done'`,
`audio_path = NULL`, no WAV written); the pending machinery only engages on terminal failure. Existing
history/notes reads (`get_history`, `get_notes`) continue to return their current shape plus the new
fields. (Machine-testable — G-A.)

## Tasks / Subtasks

1. [x] **Schema + migration (AC1).** Add `status` + `audio_path` columns to the `CREATE TABLE`
   (`history/mod.rs:108`) and an additive migration mirroring the `is_note` pattern
   (`history/mod.rs:137-140`). Extend the row struct (`history/mod.rs:36-51`) + the SELECT column lists
   (`:239`, `:249`, `:273-295`) + the insert (`:210`). Unit-test the migration preserves a pre-existing
   row and defaults it to `done`/`NULL` (mirror the existing migration test at `history/mod.rs:916`).
2. [x] **Create pending entry on terminal STT failure — Windows (AC2).** At the terminal-failure sites that
   already call `save_pending_wav` (`pipeline.rs:1153/1161/1181`), also insert a `pending` history row
   carrying the returned WAV path. Keep `save_pending_wav`'s fail-soft contract (never panic). Unit-test.
3. [x] **Re-process + discard commands (AC5, AC6).** Add Tauri commands to (a) re-process a pending entry
   by id — load the WAV from `audio_path`, run STT+cleanup, on success update row to `done` + delete WAV,
   on failure keep pending; (b) discard a pending entry — delete row + WAV (tolerate missing WAV).
   Unit-test the state transitions + WAV deletion.
4. [x] **Pending-entry surface (AC4, AC5 UI).** In the dictation-history render (locate the `is_note = 0`
   list — `getHistory` in `App.tsx` / `tauri-commands.ts`; VoiceNotesPanel is notes-only), render
   `pending` entries with the amber treatment + placeholder + the two actions, wired to the new commands,
   with a busy/disabled state during re-process and an inline error on failure.
5. [x] **Android parity (AC3).** Add WAV persistence on the terminal STT-failure path in
   `KlarvoOverlayService.kt` and create the matching `pending` history row (native Kotlin history
   access). Surface the pending entry + actions in whatever dictation-history UI Android exposes (shared
   React via `TauriActivity` if reachable; otherwise mark the Android UI as a residual — see Dev Notes).
6. [x] **Guards (G-A).** Rust unit/integration tests for: migration backward-compat; pending-entry creation
   on terminal failure; re-process success→done+WAV-deleted; re-process failure→stays pending; discard→
   row+WAV gone; happy-path parity (no WAV, status done).

## Dev Notes

### Verified current-state (audit 2026-07-06) — use as given, do not re-derive
- **WAV persistence already exists (Windows).** `save_pending_wav` (`pipeline.rs:159-186`) writes
  `{app_data}/pending/{millis}.wav` and is already called on all three terminal-STT-failure sites
  (`pipeline.rs:1153`, `:1161`, `:1181`). It is fail-soft (swallows I/O errors, never panics — tests at
  `pipeline.rs:3531-3544`). This story adds the *history entry* alongside it + the delete-on-success.
- **History schema + access:** `src-tauri/src/history/mod.rs`. Table created at `:108`; row struct at
  `:36-51`; `is_note` migration (the pattern to mirror) at `:137-140`; inserts at `:210`; the
  dictation vs. notes split is `WHERE is_note = 0` (`:239-240`) vs. `is_note = 1` (`:249-250`).
- **UI split:** `VoiceNotesPanel.tsx` renders **notes only** (`is_note = 1`). Pending entries are failed
  **dictations** (`is_note = 0`) → they belong in the dictation-history view, which is fetched via
  `getHistory` (`App.tsx` / `tauri-commands.ts`). Locate that render site (it may be inline in `App.tsx`;
  there is no separate `HistoryPanel.tsx`).
- **Android has NO WAV persistence yet.** Grep found no `savePendingWav` (or equivalent) in the Kotlin
  sources — the `pipeline.rs:161` comment claiming "Android's existing savePendingWav mechanism" is
  inaccurate. On Android both the persistence and the pending entry are net-new (native path in
  `KlarvoOverlayService.kt`). Config/behaviour parity is the #1 drift source (project-context.md,
  ADR-0016) — mirror the Windows semantics.

### Cross-story / cross-platform dependency (E2)
- **Builds on 12-1** (`save_pending_wav` + the terminal-failure ladder). Windows persistence is done;
  this story is mostly the entry + UI + re-process + Android persistence.
- **Android UI reachability is an open technical question** (not a design gate). If the shared React
  dictation-history view is reachable on Android, the pending surface comes "for free"; if not, the
  Android *visual* verdict is a residual for Andi's real-device gate and the machine-verifiable
  deliverable there is the Kotlin persistence + entry logic.
- **B-capability is a constraint, not a feature to expose:** ship the `status` + `audio_path` model so
  12-3 needs no schema rebuild, but expose nothing beyond A (transient) in this story.

### Testing Rules (from project-context.md — apply directly)
- Rust tests are inline `#[cfg(test)]` modules (not a `tests/` tree). Bind tests to the real code paths.
- **Surface DoD:** the Windows pending-entry UI + on-outage behaviour are Andi's real-machine gate; run
  the applicable `docs/surface-smoke-checklist.md` items before smoke. Linux `cargo test` does NOT
  satisfy the surface DoD.
- **Android:** on-device/emulator smoke via `scripts/android-smoke.sh`; the emulator's structural window
  oracle is for *overlay* windows — a dictation-history *list* is not an overlay, so its Android visual
  verdict is Andi's real-device residual, not an emulator screenshot. Regenerate `KlarvoTheme.kt` via
  codegen if touched, never hand-edit.
- **Config/behaviour parity (ADR-0016):** any shared behaviour must be mirrored in BOTH the Rust and
  Kotlin paths.

## Dev Agent Record

### Completion Notes

- **Schema (AC1):** `status TEXT NOT NULL DEFAULT 'done'` + `audio_path TEXT` added to both the
  fresh `CREATE TABLE` and as an additive `ALTER TABLE` migration (mirrors the `is_note` pattern).
  All four SELECT sites (`get_entries`, `get_notes`, `search_entries` ×2, plus new `get_entry_by_id`)
  now share one `SELECT_COLUMNS` constant so they can't drift apart. `add_entry`'s signature and SQL
  were left completely untouched — normal inserts get `status='done'`/`audio_path=NULL` purely from
  the column defaults, which is what makes AC7 (byte-identical happy path) hold structurally rather
  than by convention.
- **Windows pending entry (AC2):** `ProcessOutcome::Stopped` gained an `audio_path: Option<PathBuf>`
  field, threaded through from the three `save_pending_wav` call sites in `process_audio`. The shell
  (`deliver_outcome`, now also takes `language: &str`) creates the `history::add_pending_entry` row
  when `audio_path` is `Some` — best-effort (a DB error is logged, never escalated, so a pending-entry
  write failure can't turn an already-degraded pipeline run into a second error surface).
- **Commands (AC5/AC6):** `reprocess_pending_entry` loads the entry, verifies `status == "pending"`,
  reads the WAV from `audio_path`, re-runs STT (current `stt_provider`) + `chunked_cleanup` (current
  `cleanup_provider`, current `cleanup_style`). On success: `promote_pending_to_done` + WAV deleted.
  On any failure (missing file / STT error / cleanup error) it returns `Err` *before* touching the DB
  or the file — the entry stays untouched (still pending, WAV still on disk), matching AC5's "on
  failure keep pending" exactly. `discard_pending_entry` deletes the WAV (tolerating `NotFound`) then
  the row.
- **Frontend (AC4):** `historyEntries.map` now branches on `entry.status === "pending"` — a distinct
  amber card with the placeholder line, the two German-labelled actions, a `disabled` busy state
  during re-process, and an inline red error line from the last failed attempt (`pendingErrors` keyed
  by entry id). Normal entries render through the pre-existing, unchanged branch (AC7 surface parity).
- **Android (AC3) — audit correction:** the story's Dev Notes (based on a same-day code audit) stated
  Android had **no** `savePendingWav` equivalent. That was already stale by the time of dev: Android's
  `KlarvoOverlayService.kt` already calls `savePendingWav` (Story 12-1 shipped it for both platforms)
  and already shows a "audio gesichert" toast on terminal failure — it just never wrote a history row.
  This story adds exactly that missing piece: `KlarvoApi.savePendingHistoryEntry` (new, shares a
  factored-out `ensureHistorySchema` with the existing `saveToHistory` so the CREATE TABLE + migration
  ladder isn't duplicated) is called from the `IOException` catch block in
  `startProcessingPipeline`/`processAudio` when `audioPreserved` is true, mirroring the Rust row shape
  (status='pending', empty text/raw_text, is_note=0).
- **Android UI reachability:** `MainActivity.kt` hosts the Settings/History screen via `TauriActivity`
  — i.e. the *same* React bundle this story's Task 4 changes live in, so the pending-entry surface is
  not Android-specific new work. What is **unverified** is whether `reprocess_pending_entry` actually
  round-trips correctly through the Tauri IPC bridge on Android at runtime (the Rust STT/cleanup
  providers are platform-agnostic `reqwest` calls, so there's no structural reason it wouldn't, but
  Android's live-recording path bypasses Tauri IPC entirely per ADR-0016 — this command path was never
  exercised that way before). Per the story's own Dev Notes framing ("Android UI reachability is an
  open technical question, not a design gate"), this is left as Andi's real-device residual rather than
  asserted as verified.
- **Verification performed:** full Rust `cargo test --lib` (650/650 green, includes ~15 new/updated
  tests across `history::mod.rs` and `pipeline.rs`), `tsc --noEmit` + `npm run build` (clean), and a
  full `scripts/android-build.sh` run (Kotlin + Rust cross-compile to a signed release APK — confirms
  the Kotlin changes compile against the real Android/Gradle toolchain, not just read-reviewed).
  **Not performed:** an on-device/emulator install-and-tap-through smoke of the actual pending-entry
  flow on either platform — that is the Windows/Android surface DoD residual for Andi's real-machine
  gate (project-context.md "Surface DoD" / "Android on-device smoke" rules), consistent with this
  being a dev-story pass, not the smoke pass.

### File List

- `src-tauri/src/history/mod.rs` — schema/migration, `HistoryEntry.status`/`.audio_path`, `SELECT_COLUMNS`,
  `add_pending_entry`, `get_entry_by_id`, `promote_pending_to_done`, tests.
- `src-tauri/src/pipeline.rs` — `ProcessOutcome::Stopped.audio_path`, `deliver_outcome` pending-entry
  creation + `language` param, updated/new tests.
- `src-tauri/src/commands/history.rs` — `reprocess_pending_entry`, `discard_pending_entry`.
- `src-tauri/src/lib.rs` — registers the two new commands.
- `src/types.ts` — `HistoryEntry.status` / `.audioPath`.
- `src/tauri-commands.ts` — `reprocessPendingEntry`, `discardPendingEntry`.
- `src/App.tsx` — pending-entry render branch + re-process/discard handlers + busy/error state.
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` — `ensureHistorySchema` (factored out),
  `savePendingHistoryEntry`.
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — calls `savePendingHistoryEntry` on
  terminal STT failure with preserved audio.
- `_bmad-output/implementation-artifacts/12-2-audio-retry-history.md` — this story file (status,
  tasks, Dev Agent Record, Change Log).
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — `12-2-audio-retry-history: review`.

## Change Log

| Date       | Change                                              | Author |
|------------|-----------------------------------------------------|--------|
| 2026-07-06 | Story authored in Phase A (epic-conductor); design decision (pending-entry UX + actions) settled with Andi. | Claude (Phase A) |
| 2026-07-06 | Implemented AC1-AC7: schema migration, Windows pending-entry wiring, re-process/discard Tauri commands, pending-entry surface (App.tsx), Android WAV+history-entry parity. Status → review. | Claude (dev-story) |
