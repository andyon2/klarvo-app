# Story 7.10: Cleanup failure → raw text clipboard-only, no paste, no auto-send

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a dictating user whose cleanup call failed (wrong model ID, provider down, key missing),
I want the raw transcript to land only in the clipboard — not pasted into the active window and never auto-sent —
so that filler-laden raw text is never inserted or submitted behind my back, while Ctrl+V still gives me the text in one keystroke.

## Context & Governing Decisions

**Source:** `docs/backlog.md` "DECIDED 2026-09-13 — Cleanup-Fehler: Rohtext NUR in die Zwischenablage, kein Einfügen,
kein Auto-Send" (Andi), born from the Story 7-9 GATE-4 finding **3a**. **Rows:** degrade path (Epic-12 principle
"never silent loss").

**How the decision was reached (for scope discipline):** Andi's original idea was buttons in the pill
("Übernehmen" → clipboard, "Einfügen"). That was assessed and **rejected**: buttons mean new mouse UI in the
native Win32 overlay plus an Android twin, for a rare path, and they break press-to-paste. The decision is
**Option C** — a reduction to existing primitives: Ctrl+V *is* the "Einfügen" button, the clipboard *is*
"Übernehmen". **Zero new UI.** What the first assessment missed and what makes this urgent: **Auto-Send sends the
filler-laden raw text immediately** on a cleanup failure — real damage, not just confusion.

**Governing decisions:**
- **ADR-0017** — the shared Rust core is STT + license only. Cleanup/LLM routing and the paste path are a
  **Rust↔Kotlin twin** → this behaviour is implemented **twice** (Rust `pipeline.rs` + Kotlin
  `KlarvoOverlayService.kt`). Do not move it into the shared core.
- **ADR-0016 Amendment 1** — core-output/contract parity is in scope; pure feature ports are not.
- **Epic-12 principle** — never silent loss: the History entry stays exactly as today (`raw_text` written).
- **7-9 D2 stays** — a model-not-found failure keeps naming the model ID in the warning.
- **7-9 finding 3a** — the degrade warning must stay readable; a follow-up terminal event must not erase it.

**Character of the work:** one new decision (`llm_error` → clipboard-only) threaded through a path that currently
**discards the flag before it reaches the paste step**, on both platforms, plus the warning/terminal-event
collision from 3a. No new config key, no new setting, no new UI surface.

> **Open design/wording/intent questions that this story file does NOT decide** are listed in
> *Dev Notes → Open questions (Q1–Q5)*. Where a task depends on one, it says so. **Do not invent the answer.**

## Acceptance Criteria

### AC1 — Desktop: clipboard-only branch on `llm_error`

**Given** cleanup failed and the pipeline degraded to raw text — i.e. `process_audio` returned
`ProcessOutcome::Produced { llm_error: true }` (all three degrade paths: retryable+fallback-also-failed,
retryable+no-fallback-available, non-retryable),
**When** the shell reaches the paste step in `pipeline::stop_and_process_pipeline`,
**Then** the text is written to the clipboard **only** — the run ends in `PasteResult::ClipboardOnly` (today
reached only on a missing/dead target window or failed focus verification),
**And** `simulate_ctrl_v` is **not** invoked for this run,
**And** because `send_enter` is gated on `paste_result == PasteResult::Pasted`, Insert+Send is skipped **without a
second switch** — no new condition is added to the auto-send gate,
**And** with `llm_error: false` nothing changes: the paste path, the Insert+Send gate and the Return-to-Current
step behave exactly as today.

> **Blocking implementation fact (verified today):** `pipeline::deliver_outcome` **drops `llm_error`**. It matches
> `ProcessOutcome::Produced { …, llm_error }`, uses it only to bump `feedback_metrics.llm_error_count`, and returns
> a 7-tuple `(cleaned_text, raw_text, is_command, stt_ms, llm_ms, prompt_tokens, completion_tokens)` that does not
> carry it. Everything downstream — paste, Insert+Send, history, the done event — is blind to the degrade today.
> The flag must be threaded through that return value (the signature already carries
> `#[allow(clippy::type_complexity)]`). Keep the existing counter behaviour.

### AC2 — Android twin: `copyToClipboard` only, no accessibility paste

**Given** cleanup failed on Android and Step 2 of `KlarvoOverlayService::processAudio` degraded to the sanitized raw
transcript,
**When** Step 4 runs on the main thread,
**Then** `copyToClipboard(finalText)` is called and `KlarvoAccessibilityService.instance?.pasteIntoFocusedField()`
is **skipped**,
**And** the auto-send path is not taken (see the note below — today it is already unreachable),
**And** the banking guard keeps its position: `BankingGuard.shouldBlockPaste(bankingAppActive)` still runs **first**
and still aborts the whole block before anything reaches the clipboard,
**And** the toast wording mirrors the desktop pill (**Q2/Q3** — the current literal
`"⚠ Cleanup nicht verfügbar → Rohtext eingefügt"` says "inserted" and becomes false),
**And** on a successful run (cleanup OK, or the **fallback provider succeeded** →
`"⚠ Cleanup-Anbieter gewechselt"`) the paste still happens exactly as today. A successful fallback is **not** a
cleanup failure.

> **Verified today:** Android has **no failure flag at Step 4**. Cleanup failure is expressed only as a local
> `var degradeStatusMsg: String?` inside `processAudio`, captured as `capturedDegradeMsg` for a deferred toast.
> `finalText` looks identical whether cleanup succeeded or degraded, and `degradeStatusMsg` is also set on the
> **success** case "fallback provider worked". So `degradeStatusMsg != null` is **not** a usable failure predicate —
> an explicit boolean is required (see Task 3).

> **Auto-send on Android is already dead (Story 7-9, row M13).** `bubbleTapAutoSend` / `bubbleLongPressAutoSend`
> were removed from `KlarvoApi.Config`, `readConfig` and `loadBubbleControls`;
> `KlarvoAccessibilityService.performEnter()` still exists but has **zero callers**. That half of the AC is
> therefore satisfied by construction — **state it in the record, do not re-wire anything, and do not delete
> `performEnter`** (7-9 deliberately left it; Android auto-send revival is a separate backlog candidate).

### AC3 — The warning survives the terminal event (design constraint from 7-9 finding 3a)

**Given** the pipeline emitted `PipelineEvent::warn(degrade_warn_msg_for_model(...))` and then completes the run
clipboard-only,
**When** the terminal event reaches the native pill,
**Then** the user can still read the degrade warning — **no separate Done/DoneClipboard event may overwrite it**,
**And** the resolution is one of the two shapes the decision allows (**Q1 — not decided here**): either **ONE**
event carries the warning text *and* the "in the clipboard" meaning, or **DoneClipboard carries the warning text**,
**And** the wording is *"Cleanup failed — raw text in the clipboard, Ctrl+V to paste"*, replacing today's
*"Cleanup failed — raw text inserted."* (`pipeline::degrade_warn_msg`),
**And** a model-not-found failure **keeps naming the model ID** — 7-9 D2 (`Model '<id>' not found — check Advanced
→ Model IDs`, `pipeline::degrade_warn_msg_for_model` + `is_model_not_found_error`) stays in force (**Q2** governs
how the clipboard hint and the model-ID text combine).

> **Why this AC is not already satisfied by 7-9's fix (verified in `native_pill.rs`):** `warning_hold_active()`
> makes the pill ignore a plain `Done` (and the `None` status message posted ahead of it) while a Warning is
> younger than `WARNING_HOLD_MS` (4 s). Its docstring is explicit that **`DoneClipboard`, `Error` and any new
> activity still override** — "DoneClipboard (the text did NOT land)". This story's happy path produces exactly
> `Warning` → `DoneClipboard`, so today the amber degrade text (with the model ID) would be replaced by the static
> label after ~0 ms. That is the regression AC3 exists to prevent.

### AC4 — No new setting, no new UI

**Given** this is the **new default** behaviour,
**Then** no config key, no toggle, no pill button and no new UI surface is introduced on either platform,
**And** overwriting the user's clipboard on the degrade path is **accepted** (Andi's explicit decision),
**And** the pill keeps its canon geometry — `Klarvo Design System.html` pins *"Fläche bleibt 200×36 — kein
Aufblasen"* (ADR-0019), so the pill must not grow to fit longer text (**Q3**).

### AC5 — History unchanged

**Given** the Epic-12 principle "never silent loss",
**Then** the history entry is written exactly as today — `history::add_entry(…, &cleaned_text, Some(&raw_text), …)`
on desktop and `KlarvoApi.saveToHistory(finalText, rawText = transcript, …)` on Android — with no new column, no
status marker and no change to the Turso push or the webhook payload.
*(The failed-entries inbox that would mark such entries is a separate backlog candidate — out of scope.)*

### AC6 — Tests, both platforms, with a mandatory inversion

**Given** the gates that exist (no CI),
**Then** the following tests exist and are green:
- **Desktop, outcome level:** the existing `pipeline::tests::test_process_audio_nonretryable_degrades_to_raw`
  still proves `llm_error == true` and the `Warning` event, extended as the chosen Q1 shape requires.
- **Desktop, paste level:** a test proving `llm_error: true` → `ClipboardOnly` **and no Enter** — i.e. that
  `send_enter` is not called even with Insert+Send on, and that `llm_error: false` still pastes and still sends.
  *(Requires a seam — see Task 1 and scope assumption **S1**; today `stop_and_process_pipeline` is untestable.)*
- **Kotlin twin:** a JVM test for the Step-4 decision *(requires the pure-function seam — Task 3, **S3**)*.

**And** the **inversion check is mandatory, at writing time**: re-enable the paste on `llm_error` and show the test
**RED**, on both platforms; revert; record as a *reverted change → red test* table (7-8/7-9 format). `git status`
clean afterwards.

> **⚠ Non-discriminating-inversion trap (7-9's lesson).** A test that only asserts "`ClipboardOnly` on the degrade
> path" can stay GREEN against code that reaches `ClipboardOnly` for the *wrong* reason (no target window, focus
> verification failed, or a coerced `PasteError`). The discriminating assertion pairs a **valid paste target** with
> `llm_error: true` and asserts no Ctrl+V and no Enter — and the same target with `llm_error: false` asserting both
> *do* happen.

### AC7 — Gates (DoD)

- `cargo test --lib` in `src-tauri/` green.
- JVM gate `./gradlew :app:testUniversalDebugUnitTest` green — via `scripts/android-smoke.sh` on the laptop AVD, or
  device-free per the 7-8/7-9 procedure (Dev Notes). `--rerun-tasks` is only mandatory after a **fixture** edit;
  this story is not expected to touch `test-fixtures/` (**S6**).
- Inversion evidence recorded (AC6).
- Every count carries its **coverage statement** — what was and was **not** exercised (project-context: "a number
  states what it covers").
- **`docs/surface-smoke-checklist.md` trap #5** (push, not poll; wire the event end-to-end; colon form
  `klarvo://…`) run mechanically for whatever event shape Q1 selects. Traps #1/#2/#3/#6 do not apply — no new
  config key, no Settings field.
- **Andi's GATE-4** (Windows release build via `scripts/windows-build.sh`): wrong DeepSeek model ID + **Insert+Send
  on** → **nothing lands in the active window**, **no Enter is sent**, the pill shows the warning **with the model
  ID**, and **Ctrl+V pastes the raw text**.

### AC8 — Message card: the pill is a status light, the preview card carries the message *(GATE-4 re-open, Andi 2026-09-14)*

**Source of truth:** canon `docs/design/overhaul/source/Klarvo Design System.html` (pill "Constraint-treu" list, 7.10 line)
+ MANIFEST row 2026-09-14 (second) + approved render `docs/design/overhaul/mockup-7-10-message-card.html`.
**Why:** Andi's Windows smoke on `d73082d`: the 200×36 pill cut `Model 'deepseek-typo' not found — in clipboard`
mid-sentence; the clipboard hint never showed. The message was on the wrong surface. Supersedes Q2/Q3's
"accept tail truncation" and AC3's wording on the pill.

**Given** cleanup failed and the raw text went clipboard-only (AC1),
**When** the terminal event reaches the desktop shell,
**Then** the native pill shows the static amber label `Cleanup failed` (no dynamic text, no truncation); the plain
focus-loss `DoneClipboard` keeps `In Clipboard`; both hold `DONE_CLIPBOARD_MS` (4 s) as today.
**And** the native preview card (`native_preview.rs`, the existing window, 8 px above the pill, centered) shows the
message **regardless of `live_preview_enabled`**: header line (mono, uppercase) `CLEANUP FAILED` with an amber dot;
cause line (`Model 'deepseek-typo' not found` — the model ID in mono on an amber chip, or the generic reason such as
`DeepSeek did not respond (timeout)`); next line `Raw text is in the clipboard · Ctrl+V to paste`; hint line
`Check Advanced → Model IDs` only for model-not-found. Amber border (`--k-amber-line`). All English.
**And** the card holds 4 s, then fades out over 1 s; a new recording or a click on the card dismisses it at once.
If live preview is active, the message replaces the preview text in the same card.
**And** every other pipeline message uses the same card: STT-ladder warning (`Warning` header, amber), boot-time
config warnings (`Warning`), errors (`Error` header, danger border `--k-danger`); the pill shows only static labels
for `Warning` / `Error` states — the pill's `fit_text` message rendering for status text is removed, not kept.
**And** the main-window status line (D1) keeps showing the cause text; Android is unchanged (one combined toast).
**And** the pipeline carries the message in a shape the card can lay out (cause / clipboard-only / model-not-found
are distinguishable — the exact seam is the dev's call; the visible outcome above is the contract).
**Inversion:** the card's message-mode gate off → the Rust-side layout/unit tests for the message model go RED.

### Out of scope (verbatim from the epic)

> **Out of scope:** the failed-entries inbox (backlog candidate), pill buttons (assessed + parked in 7-9),
> any change to STT, VAD, JNI, the fallback ladder or the config schema.

## Tasks / Subtasks

- [x] **Task 1 — Desktop: thread `llm_error` to the paste step and branch** (AC1, AC6)
  - [x] `pipeline::deliver_outcome`: carry `llm_error` out in the returned tuple (7 → 8 fields); keep the
        `llm_error_count` increment and every other invariant in its docstring intact. Update the `let Some((…))
        else` destructuring in `stop_and_process_pipeline` and the two existing
        `test_deliver_outcome_*` tests.
        **Record correction (review round 1, finding 7):** the destructuring was updated (and the tuple went 7 → 9,
        not 8 — see the Completion Notes' deviation entry), but **neither `test_deliver_outcome_*` test was touched,
        and neither needed to be**: `test_deliver_outcome_creates_pending_entry_when_audio_preserved` and
        `test_deliver_outcome_no_pending_entry_without_audio_path` both drive `ProcessOutcome::Stopped` and assert
        `result.is_none()`, so the `Produced` arm's tuple width is invisible to them. The task text predicted a
        change that the implementation did not need; it is corrected here rather than left as a false claim.
  - [x] In `stop_and_process_pipeline`, on `llm_error == true`: write the clipboard and end the run as
        `PasteResult::ClipboardOnly` **without** calling `paste_handler.paste(...)`'s Ctrl+V path. **Reuse the
        existing primitives — do not write a second clipboard implementation** (the arboard call lives inside
        `paste::windows::WindowsPasteHandler::paste` step 1, and in `paste::linux::set_clipboard`); extend the
        `PasteHandler` trait rather than duplicating (e.g. a `copy_only(text)` method with a default
        implementation), so Windows, Linux, Fallback and Android handlers stay consistent.
  - [x] Leave the Insert+Send gate **textually unchanged** (`if insert_and_send && paste_result ==
        PasteResult::Pasted`) — AC1 rests on it staying the single switch.
  - [x] **Seam for the paste-level test (S1):** extract the paste + Insert+Send + terminal-event tail of
        `stop_and_process_pipeline` into a function that takes the handler (`&dyn PasteHandler`) and the flags,
        mirroring how `process_audio` takes `emit: &mut dyn FnMut(PipelineEvent)`. Keep it behaviour-preserving;
        this is a seam, not a refactor of the pipeline's logic.

- [x] **Task 2 — Desktop: the warning must survive the terminal event** (AC3) — **depends on Q1**
  - [x] Change the degrade wording in `pipeline::degrade_warn_msg` from "Cleanup failed — raw text inserted." to
        the AC3 wording. Keep `friendly_error` appending as today unless Q2 says otherwise.
  - [x] `pipeline::degrade_warn_msg_for_model` / `is_model_not_found_error`: keep D2's model-ID message
        (**Q2** decides how it combines with the clipboard hint). Do **not** loosen the needles added by 7-9
        finding 3b (DeepSeek's live 400 wording).
  - [x] Implement the Q1 shape:
        - *Option (a) one event* — `PipelineEvent` already carries **both** `warning` and `clipboard_only`
          (`hotkey/mod.rs`), so a single `Done` event with `warning: Some(...)` + `clipboard_only: Some(true)` is
          structurally possible today without a schema change. Then `lib::emit_pipeline_state` forwards
          `event.warning.or(event.error)` as `status_msg` — but `native_pill.rs` renders `DoneClipboard` as the
          **static** label `"In Clipboard"` and ignores `status_msg`; that renderer arm must read `status_msg`.
        - *Option (b) DoneClipboard carries the warning* — same renderer change, plus `warning_hold_active`'s
          override rule must stop letting `DoneClipboard` erase a held Warning.
  - [x] Whichever shape: re-check `native_pill.rs::handle_timer` still has exactly one path back to `Idle` for the
        new sequence (during a hold, `Done` is dropped and `done_at` is never set — the warning timer is the only
        dismissal), and that `DONE_CLIPBOARD_MS` (4000) / `WARNING_HOLD_MS` (4000) do not fight each other.
  - [x] **Frontend consumer (trap #5):** `src/hooks/useRecording.ts` reads `p.warning` **only** in the
        `p.state === "warning"` branch and returns early; a `Done`-shaped event carrying `warning` would drop the
        message in the main window. Verify the consumer for the chosen shape — the React `FloatingBar` no longer
        exists (Epic 10, native overlays), so the pill is the primary surface, but `useRecording` is still live.

- [x] **Task 3 — Android twin: an explicit failure flag + the Step-4 branch** (AC2, AC6)
  - [x] In `KlarvoOverlayService::processAudio` Step 2, introduce an explicit `var llmFailed: Boolean` (or fold it
        into a small result holder) set **only** on the true failure branches — cloud primary failed with no
        fallback, fallback also failed, and the local-MNN `catch` (**Q4** decides the local-MNN and no-key cases).
        It must stay **false** when the fallback provider **succeeded** (`"⚠ Cleanup-Anbieter gewechselt"`).
        `degradeStatusMsg != null` is not a substitute.
  - [x] Capture it alongside `capturedDegradeMsg` for the `handler.post { … }` block.
  - [x] Step 4: keep `BankingGuard.shouldBlockPaste` first; then `copyToClipboard(finalText)`; then call
        `pasteIntoFocusedField()` **only** when the decision says paste. Keep `setState(RecordingState.DONE)`,
        `adjustLayoutForState`, `doneFlashRunnable` and the AUTO-loop restart unchanged.
  - [x] Decide what happens to the existing `if (!pasted) showToast("Copied: $preview")` line on the new path
        (**Q5**) — today `pasted` is `KlarvoAccessibilityService.instance != null`, which is not a paste result.
  - [x] Update the degrade toast literal (**Q2/Q3**) and the stale `activeGesture` KDoc that still mentions
        "autoSend".
  - [x] **Seam (S3):** extract the decision as a **pure function** into the `KlarvoOverlayService` companion object
        (or next to `BankingGuard`), following the established repo pattern — `shouldApplyPreviewAppearance`,
        `sanitizePreviewChunk`, `resolveMinRecordingMsForSilenceFilter`, `RecordingMode.selectSilenceSecs`,
        `shouldInstallPreviewFlush`, `BankingGuard.shouldBlockPaste` — and have Step 4 call the real one. No
        mocking library exists on the test classpath; this is the only way to a JVM test.

- [x] **Task 4 — Tests + inversions** (AC6)
  - [x] Rust: extend `test_process_audio_nonretryable_degrades_to_raw` as Q1 requires. Note the test helper
        `pipeline::tests::run` records **only `ev.state`** and discards the message — a message/`clipboardOnly`
        assertion needs the full `PipelineEvent` (extend `run` or add a sibling helper; `degrade_warn_msg_for_model`
        is also directly unit-testable, as `spec_model_not_found_warning_names_the_model` shows).
  - [x] Rust: the paste-level test against the Task-1 seam with a fake `PasteHandler` recording
        paste/copy/`send_enter` calls (no such fake exists today).
  - [x] Kotlin: a JUnit4 test for the Task-3 pure function, in `android/kotlin-test/com/klarvo/voice/`.
  - [x] Both inversions (AC6) + the non-discriminating trap check; table in the Dev Agent Record;
        `git status` clean.

- [x] **Task 6 — Message card, pill as status light** (AC8) — GATE-4 re-open. Desktop only. `native_pill.rs`:
      static labels per state, remove dynamic status text; `native_preview.rs`: message mode (any pipeline message,
      independent of `live_preview_enabled`), header/cause/next/hint layout, amber or danger border, 4 s + 1 s fade,
      dismiss on new recording or click; pipeline/event: structured message fields; tests for the message model
      + inversion; Windows-gated code compiles via the conductor's Windows build (GATE-4), not on Linux.
  - [x] New `src-tauri/src/overlay_message.rs` — **not** platform-gated, so the message model is reachable by
        `cargo test --lib` on Linux: `MessageTone`, `CauseLine` (chip split), `OverlayMessage`, `DegradeCause`
        (`ModelNotFound` / `Generic` / `ClipboardWriteFailed`) with `status_line()` + `card()`, and the five static
        `PILL_LABEL_*` constants.
  - [x] Pipeline carries the **cause**, not a sentence: `ProcessOutcome::Produced.degrade_msg: Option<String>` →
        `degrade_cause: Option<DegradeCause>`; `degrade_cause_for_model` / `generic_degrade_cause` replace the
        string builders at the three degrade sites; `terminal_degrade_msg` → `terminal_degrade_cause`.
        `status_line()` reproduces the pre-AC8 wording **byte-identically**, so D1 and every wording spec are
        untouched.
  - [x] `hotkey::PipelineEvent` gains `#[serde(skip_serializing)] message: Option<OverlayMessage>` — in-process
        only, so **no payload field and no event name changed** (trap #5). `done_with_clipboard_only` takes the
        `DegradeCause` and derives both forms; `warn` / `error` carry their own card.
  - [x] `native_pill.rs`: `fit_text` **removed**; `status_msg` / `pending_msg` / `WM_PILL_SET_MSG` /
        `set_status_msg` removed with it; new `TerminalKind` (Pasted / ClipboardOnly / Degraded) rides LPARAM;
        new `NativePillState::DoneDegraded`; all five terminal/message arms fold into `draw_static_label`.
  - [x] `native_preview.rs`: `WM_PREVIEW_SET_MESSAGE`, `render_message_card` (header + dot · cause with amber chip ·
        next · hint, canon tokens, tone-coloured border), `present()` factored out for the fade,
        `MSG_HOLD_MS` 4 s + `MSG_FADE_MS` 1 s on a 33 ms timer, `WM_LBUTTONDOWN` dismiss with `WS_EX_TRANSPARENT`
        toggled only while a card is up, Recording clears the card.
  - [x] Card exists without live preview: `pipeline::start_recording_only` creates the preview window
        unconditionally, and `lib::run` creates one at setup so the boot-time config warnings have a surface.
  - [x] Tests + inversions: 10 new specs (7 in `overlay_message`, 3 in `hotkey`); both inversions shown RED and
        reverted; `git status` clean.
- [x] **Task 5 — Gates** (AC7): `cargo test --lib`, the JVM gate, trap #5, coverage statements, then hand GATE-4
      to Andi with the exact steps from the epic DoD. Anchor the record **by symbol**, write resolution rows from
      `git diff` (Epic-7 retro D2).

### Review Findings (bmad-code-review, 2026-09-14, range `6cedbb5..HEAD`)

Three layers ran, none failed: Blind Hunter (diff only), Edge Case Hunter (diff + tree), Acceptance Auditor
(diff + story + epic + project-context). Anchors are `file::symbol` per the project-context rule.
**1 decision-needed · 10 patch · 12 deferred · 10 dismissed as noise.**

- [x] [Review][Decision] **Q1's "the main window must show the carried message" is unmet, and meeting it collides with AC4** — `src/hooks/useRecording.ts` captures `p.warning` before the state branch and `useRecording` returns `warningMessage`, but nothing renders it: `src/App.tsx` never reads `recording.warningMessage` (the only `warning` hit in `App.tsx` is an unrelated comment). Pre-existing — the value had no reader before this story either, so the diff regresses nothing — but Q1 names the main window as a consumer that "must show the carried message so nothing is lost", while AC4 forbids a new UI surface. Options: (a) accept the pill as the only degrade surface and amend Q1's wording; (b) render the message in the existing status line (`App.tsx`, the `recording.errorMessage` slot) — arguably not a *new* surface; (c) defer to a follow-up story. Andi's call.

- [x] [Review][Patch] A stale `status_msg` now bleeds into the message-less `DoneClipboard` pill [`src-tauri/src/native_pill.rs::render_frame` DoneClipboard arm + `pill_wnd_proc` `WM_PILL_SET_MSG`/`WM_PILL_SET_STATE`]
- [x] [Review][Patch] MANIFEST provenance row carries the literal `%s` instead of the fingerprint [`docs/design/overhaul/source/MANIFEST.md`, row `2026-09-14`]
- [x] [Review][Patch] `<id>` is unescaped in the canon HTML, so the rendered canon drops the placeholder [`docs/design/overhaul/source/Klarvo Design System.html`, pill "Constraint-treu" list, new `cleanup-degrade` `<li>`]
- [x] [Review][Patch] Canon + MANIFEST still state the pre-GATE-2 Android wording ("Android-Toast spiegelt den Wortlaut") [`docs/design/overhaul/source/MANIFEST.md` row `2026-09-14` + the canon `<li>`]
- [x] [Review][Patch] The `degrade_msg` invariant docstring and the record overclaim coverage — 2 of 3 degrade sites [`src-tauri/src/pipeline.rs::ProcessOutcome::Produced.degrade_msg` KDoc + `tests::spec_degrade_msg_present_exactly_when_llm_error`]
- [x] [Review][Patch] Stale comments describe the Warning→Done sequence Q1 removed [`src-tauri/src/native_pill.rs::handle_timer` warning branch + `warning_hold_active` docstring]
- [x] [Review][Patch] Duplicate Kotlin test wearing a misleading name [`android/kotlin-test/com/klarvo/voice/CleanupFailureDeliveryTest.kt::successfulFallbackProvider_isNotACleanupFailure_andStillPastes`]
- [x] [Review][Patch] `Delivery::enter_sent`'s docstring says "Return was sent" but it is `true` after a failed `send_enter` [`src-tauri/src/pipeline.rs::Delivery` + `deliver_text`]
- [x] [Review][Patch] The degrade message still claims "raw text in clipboard" when the clipboard write itself failed [`src-tauri/src/pipeline.rs::deliver_text`, the coerced `Err` arm]
- [x] [Review][Patch] Record inaccuracy: Task 1 claims the two `test_deliver_outcome_*` tests were updated; the diff touches neither [story Task 1 / `src-tauri/src/pipeline.rs::tests::test_deliver_outcome_*`]

- [x] [Review][Defer] `warningMessage` is never cleared on a hotkey-driven run [`src/hooks/useRecording.ts`, the `onStateChanged` listener vs `handleRecordToggle`] — deferred, pre-existing → **promoted by Andi and fixed in review round 2** (see Review Follow-ups below); the round-1 deferral rested on "nothing renders the value", a premise D1 removed in the same round
- [x] [Review][Defer] `copyToClipboard` has no `try/catch`; a `SecurityException` kills the run before the single degrade toast [`android/.../KlarvoOverlayService.kt::copyToClipboard`] — deferred, pre-existing
- [x] [Review][Defer] The banking guard aborts before the clipboard write and before the degrade toast, so a cleanup failure surfaces no cause at all [`android/.../KlarvoOverlayService.kt::processAudio` Step 4, `BankingGuard.shouldBlockPaste`] — deferred, pre-existing (Story 2-4 DIV-04)
- [x] [Review][Defer] `accessibilityConnected` is a snapshot, not a paste result; a silent `pasteIntoFocusedField()` no-op yields no toast either [`android/.../KlarvoOverlayService.kt::processAudio` Step 4] — deferred, pre-existing (named in the diff's own NOTE)
- [x] [Review][Defer] A cleanup failure overwrites an earlier STT-degrade message in the single `degradeStatusMsg` slot [`android/.../KlarvoOverlayService.kt::processAudio`] — deferred, pre-existing
- [x] [Review][Defer] Android never produces the model-not-found form; the constant's KDoc parity claim reads stronger than the code [`android/.../KlarvoOverlayService.kt::CLEANUP_FAILED_CLIPBOARD_MSG`] — deferred, already stated precisely in the Completion Notes
- [x] [Review][Defer] Android local-MNN cleanup failure stays silent and still pastes raw text [`android/.../KlarvoOverlayService.kt::processAudio` Step 2, local `catch`] — deferred, Q4 decision, already in `docs/backlog.md` (`606e9ac`)
- [x] [Review][Defer] `deliver_outcome`'s 9-field positional tuple; `deliver_text`'s two adjacent booleans transpose silently [`src-tauri/src/pipeline.rs::deliver_outcome`, `deliver_text`] — deferred, the named-struct refactor is punted by the pre-existing `#[allow(clippy::type_complexity)]`
- [x] [Review][Defer] The generic degrade wording is likely tail-truncated inside the 200×36 pill, cutting the clipboard hint [`src-tauri/src/native_pill.rs::fit_text`] — deferred, Q3 accepted tail truncation; Andi's GATE-4 judgement
- [x] [Review][Defer] The Android degrade toast slot now mixes languages (English constant vs German `"⚠ Cleanup-Anbieter gewechselt"`) and drops the `⚠` glyph [`android/.../KlarvoOverlayService.kt::processAudio` Step 2] — deferred, Q2/Q3 decided English for this path; the sibling is pre-existing
- [x] [Review][Defer] Android `set_clipboard` returns `Ok(())` without writing, so `copy_only` would report `ClipboardOnly` having written nothing [`src-tauri/src/paste/mod.rs::set_clipboard` android arm] — deferred, latent; matches the pre-existing `AndroidPasteHandler::paste` stub and the Rust delivery path is not reached on Android
- [x] [Review][Defer] `paste_error_count` is bumped for an `EmptyText` rejection on a run where no paste was attempted [`src-tauri/src/pipeline.rs::deliver_text` / the shell's `paste_failed` arm] — deferred, pre-existing metric semantics

**Dismissed as noise (10).** The new `DoneClipboard` render arm is dead code (false — `lib::emit_pipeline_state` posts `set_status_msg(event.warning.or(event.error))` before `set_state`) · `fit_text` NUL-terminator asymmetry (false — `fit_text` drops the NUL itself; the new branch matches the Error/Warning arms) · command-mode degrade drops the cause (false — `deliver_outcome`'s command branch only calls `consume_command_mode` and returns the full tuple; no early return) · the frontend capture now fires on the error surface (false — `PipelineEvent::error` sets `warning: None`) · empty-string `warning` (unreachable — both degrade builders always return a non-empty fixed prefix) · the model message drops D2's "check Advanced → Model IDs" pointer · the generic and model forms disagree about the `(Ctrl+V)` hint · `Model 'x' … in clipboard` reads ambiguously (all three: Andi's explicit Q2 decision) · removing the mid-run `Warning` event loses live feedback (Andi's explicit Q1 decision) · `insert_and_send` read moved before the paste (behaviour-preserving under the same no-reentrancy premise, disclosed in the record) · Windows focus is not restored on the degrade branch (intended — no paste, no focus restore).

**Machine gates re-run by the review, not taken from the record:** `cargo test --manifest-path src-tauri/Cargo.toml --lib` → `test result: ok. 688 passed; 0 failed` (matches). 24 Kotlin suite files and 9 `@Test` in `CleanupFailureDeliveryTest.kt` counted in today's tree (matches); gradle itself was **not** run by the review. `md5(Klarvo Design System.html + assets/klarvo.css)` = `43d926f743fd91caf2ad51afd63cfcb8` (matches the MANIFEST header — the table cell is the defect). **Andi's GATE-4 remains outstanding**; nothing in this review substitutes for it.

### Review Follow-ups (AI) — round 2 (2026-09-14)

One confirmed finding, promoted from round 1's deferral list by Andi. Nothing else from the deferred or
dismissed lists is in this round's scope.

- [x] [AI-Review][Med] **`warningMessage` is never cleared on a hotkey-driven run** [`src/hooks/useRecording.ts`,
      the `onStateChanged` listener — the `p.warning` capture line]. Since D1 (`8aa7167`) the main window renders
      `warningMessage`, but the only two `setWarningMessage(null)` sites live in `handleRecordToggle`, which a
      hotkey run never reaches. Consequence: the next **successful** run shows the previous run's amber degrade
      text instead of "Done".
  - [x] Add the `else` branch at the capture line so a state event arriving **without** a warning resets it.
  - [x] Test + inversion: extend the D1 proxy smoke with the sequence (degraded run → clean run, **same browser
        boot**, no record-button click) asserting "Done" in teal; shown RED before the fix.

### Review Findings (bmad-code-review RE-REVIEW, 2026-09-14, range `6cedbb5..HEAD`, fix commits `8aa7167` + `df836a0`)

Scoped re-review, **not** a fresh adversarial sweep: verify the round-1 findings 1–7 + D1 and the round-2 fix are
resolved and that the touched lines regressed nothing. Three layers ran, none failed. New independent findings are
reported as residual, not fixed here.
**3 decision-needed · 6 patch · 5 deferred · 5 dismissed as noise.**

**Verification verdicts.** RESOLVED: F1 (`native_pill::pill_wnd_proc` `pending_msg` staging — the FIFO argument in
the round-1 record is factually correct; `lib::emit_pipeline_state` is the sole caller and posts both messages
back-to-back inside one `native_pill.lock()` guard) · F2 · F4 (no coverage lost — the removed test's call and
assertions are a subset of `cleanupOk_pastesWhenAccessibilityConnected`) · F5 · F7 · D1 (one colour arm + one text
branch, AC4 holds, `text-klarvo-amber` is a canon token) · round 2 (`done_with_clipboard_only` is the run's last
emit; the new clear does not erase the 7-9 3a degrade text). **PARTIALLY RESOLVED:** F3 (a third copy of the stale
sequence survives) · F6 (the degrade branch is fixed; the symmetric non-degrade branch is not).

- [x] [Review][Decision] (Andi 2026-09-14: pre-existing on the normal run, not 7-10's → backlog) **F6's correction is gated on `degrade_msg`, not on `paste_failed` — the identical false promise survives one branch over** — `pipeline::terminal_degrade_msg` only replaces the text when a cleanup degrade was in flight. On Windows `PasteHandler::paste` returns `Err` **only** for `EmptyText` or a failed `set_clipboard` (`paste/mod.rs::WindowsPasteHandler::paste` steps 1–6 — every focus failure returns `Ok(ClipboardOnly)`), so `paste_failed == true && degrade_msg == None` means *the clipboard write failed on an ordinary run*: the pill renders the static "In Clipboard" and the main window teal "Done" for text that reached neither window nor clipboard — the exact defect F6 was raised for. Options: (a) key the replacement on `paste_failed` independently of `degrade_msg` and let Andi pick the wording; (b) a distinct terminal variant for "nothing landed anywhere"; (c) accept and record it as out of 7-10's scope. Andi's call — the fix needs new user-facing wording, which is a Q2-class decision.
- [x] [Review][Decision] (Andi 2026-09-14: deliberate divergence, registered in KDoc + backlog) **Twin-parity gap: the new user-facing literal has no Kotlin mirror and no divergence record** — `pipeline::terminal_degrade_msg`'s `"Cleanup failed — clipboard write failed"` is a second degrade literal and a new decision (`paste_failed` replaces the cause) added on desktop only. `KlarvoOverlayService::CLEANUP_FAILED_CLIPBOARD_MSG` has no equivalent branch, and its KDoc still claims the wording mirrors `pipeline::degrade_warn_msg`. Project-context/ADR-0016–0017: paste logic is a Rust↔Kotlin twin — fix twice or record the divergence. `grep -rn "clipboard write failed"` hits only `pipeline.rs` and the round-1 resolution row; no `DIV-` entry. Options: (a) mirror the branch in Kotlin (needs the deferred `copyToClipboard` try/catch first); (b) record a deliberate divergence in the twin register + the Kotlin KDoc. Andi's call.
- [x] [Review][Decision] (Andi 2026-09-14: pre-existing pill constant → backlog; Dev Notes corrected) **D1 puts the same string on two surfaces in two different ambers, and the Dev Notes' canon claim is wrong** — pill `Warning`/`DoneClipboard` amber is `#FFA344` (`native_pill.rs` colour consts), the main-window status line is canon `--color-klarvo-amber` `#E9A24C` (`src/styles.css`). The canon records `#FFA344 → #E9A24C` as a deliberate replacement, so the Dev Notes' "the amber the code uses for both `Warning` and `DoneClipboard` … already matches" is false against the canon. Pre-existing text, but `8aa7167` is what made the divergence visible on two surfaces at once. Options: (a) correct the Dev Notes claim only; (b) additionally align the pill constant (Windows-gated, no machine coverage, arguably a separate story). Andi's call.

- [x] [Review][Patch] (editorial close-out, conductor) F3 left a third copy of the removed Warning→Done degrade sequence, describing it as current [`src-tauri/src/native_pill.rs`, the `WARNING_HOLD_MS` const comment]
- [x] [Review][Patch] (editorial close-out, conductor) The new "only the STT fallback ladder produces a Warning" claim is false — `lib.rs` emits `PipelineEvent::warn` for boot-time config warnings through the same funnel [`src-tauri/src/native_pill.rs::warning_hold_active` docstring + the `handle_timer` warning branch; second producer at `lib::run`'s `config_warnings` loop]
- [x] [Review][Patch] (editorial close-out, conductor) The `status_msg` comment rewritten in `8aa7167` keeps a stale example (`"⚠ DeepSeek langsam → OpenAI"`, a cleanup-provider switch) that contradicts the "only the STT ladder" claim rewritten in the same commit; no Rust site emits it [`src-tauri/src/native_pill.rs::PillWindowState::status_msg` comment + the `render_frame` Warning arm comment]
- [x] [Review][Patch] (editorial close-out, conductor) `spec_non_degraded_run_never_gains_a_cause`'s docstring justifies `(None, true)` as "the plain focus-failure `DoneClipboard`" — factually wrong, focus failure never sets `paste_failed` [`src-tauri/src/pipeline.rs::tests::spec_non_degraded_run_never_gains_a_cause`]
- [x] [Review][Patch] (editorial close-out, conductor) Record: the File List still describes `gate4-evidence/7-10/report.txt` as the round-1 "final green run (`report.txt` 10/10)", but round 2 overwrote the artifact in place — the committed file reads `13/13`, so the round-1 figure is no longer reproducible from evidence [story File List + the round-1 gate table row]
- [x] [Review][Patch] (editorial close-out, conductor) Record: the round-1 "⚠ FLAGGED, NOT ACTED ON" paragraph still reads "**not applied**" and "the D1 smoke … neither triggers nor rules this out" — round 2 applied the fix and CASE C exercises exactly that sequence; unlike the deferral row and the coverage bullet, this paragraph carries no forward pointer [story Dev Agent Record, round-1 fix-round narrative]

- [x] [Review][Defer] `terminal_degrade_msg` conflates `PasteError::EmptyText` with a clipboard-write failure — `copy_only` rejects empty text before touching the clipboard, so a degraded run emptied by `sanitize_llm_output` would be told "clipboard write failed" when no write was attempted [`src-tauri/src/pipeline.rs::terminal_degrade_msg` / `Delivery::paste_failed`] — deferred, low reachability; needs the `PasteError` variant carried out of `Delivery`
- [x] [Review][Defer] D1's status line has no display hold — in Auto-Loop the next run's `recording` event lands ~200 ms later and replaces the amber cause, while the pill holds it for `DONE_CLIPBOARD_MS` (4 s) [`src/App.tsx` status label vs `src-tauri/src/native_pill.rs::DONE_CLIPBOARD_MS`] — deferred, not introduced by either fix round (the state change alone did this pre-round-2)
- [x] [Review][Defer] D1 renders the pill's front-loaded message in a surface with no truncation, so a long cause wraps or overflows; the D1 smoke asserts text and computed colour only, never geometry [`src/App.tsx` status label] — deferred, Q2's front-loading rationale is a pill constraint; Andi's GATE-4 judgement
- [x] [Review][Defer] The `set_status_msg`/`set_state` pairing is now load-bearing but enforced only by a doc comment, in a file with no test module and no compilation on this host [`src-tauri/src/native_pill.rs::set_status_msg` / `set_state`] — deferred, hardening; a single `(state, msg)` message would remove the FIFO assumption structurally
- [x] [Review][Defer] No test asserts that a failing `copy_only` yields `paste_failed = true` — `FailingHandler` does not override `copy_only`, so driving it with `llm_error = true` would touch the real system clipboard [`src-tauri/src/pipeline.rs::tests::FailingHandler`] — deferred, disclosed in the record; a spy returning `Err` from `copy_only` closes it

**Dismissed as noise (5).** "An interleaved `set_state` steals the staged message" (unreachable — `set_state` has exactly one caller, `lib::emit_pipeline_state`, which posts both under one mutex) · "the cleanup-fallback-*succeeded* site still emits a `Warning`, so F3's claim is wrong" (false for Rust — `grep -rn "PipelineEvent::warn" src-tauri/src/` returns two sites, neither is that one; `"⚠ Cleanup-Anbieter gewechselt"` exists only in the Kotlin twin) · "round 2 added no Change Log row and no File List entry for `useRecording.ts`" (false — both land in `df836a0`) · "the round-1 coverage bullet still lists the stale-`warningMessage` sequence as not exercised" (false — the round-2 section corrects it in place and names what CASE C still does not reach) · "CASE C does not reproduce the real hotkey event sequence" (already stated in the story's own coverage notes — a disclosed scope note, not a finding).

**Machine gates re-run by the review, not taken from the record:** `cargo test --manifest-path src-tauri/Cargo.toml --lib` → **691 passed, 0 failed** (matches). Kotlin JVM result XML in `src-tauri/gen/android/app/build/test-results/testUniversalDebugUnitTest/` → **24 suites / 194 tests / 0 failures**, newest file timestamped at round 1 — consistent with round 2's own `UP-TO-DATE` disclosure; gradle was **not** re-run by the review. 8 `@Test` counted in `CleanupFailureDeliveryTest.kt`. Canon md5 matches the MANIFEST header. `git status` clean apart from the pre-existing `seat-costs.jsonl`.

**Not verified by this review:** `src-tauri/src/native_pill.rs` is `cfg(target_os = "windows")` with no test module — **F1 and F3 have zero machine coverage anywhere**, they were checked by reading the source and tracing the producer/consumer graph. No gradle run, no `npm run build`, no D1 puppeteer re-run, no Android device, no Windows build, no pixels. **Andi's GATE-4 remains outstanding and nothing here substitutes for it.**

### Review Findings (bmad-code-review, 2026-09-14, range `6cedbb5..HEAD`, focus commit `8b81864` — AC8 / Task 6)

Full adversarial sweep on the **new** work only (`8b81864`: pill as status light, preview card as message surface,
new `overlay_message.rs`); earlier commits only where `8b81864` touches them — rounds 1 and 2 already reviewed
those and their findings are resolved or accepted as residual above. Three layers ran, none failed: Blind Hunter
(diff only), Edge Case Hunter (diff + tree), Acceptance Auditor (diff + story + epic + canon + project-context).
Anchors are `file::symbol` per the project-context rule.
**3 decision-needed · 11 patch · 10 deferred · 11 dismissed as noise.**

- [x] [Review][Decision] (Andi 2026-09-14 → **D1**, option (a), applied) **The card has no surface at all when the pill sits near the top of the work area** — `native_preview::compute_preview_geometry` clamps the window to the space *above* the pill (`max_avail = (pill_y - GAP - work_top/scale - 12).max(0.0)`) and returns `phys_h.max(1)`. With the pill dragged to the top of the screen `phys_h` collapses to **1 px**, so `render_message_card`'s `max_card_h = ph - 2*inset` goes negative, `round_rect_path` returns `None`, and nothing is painted — while `present()` still runs. Before AC8 the message lived on the pill, which always exists; AC8 moved it onto a window that can be zero-height, so on that (persisted, draggable) pill position **every** pipeline message is now silently lost. Options: (a) drop the card *below* the pill when there is no room above; (b) enforce a minimum card height and let it overlap the pill; (c) keep the pill's static light as the only surface there and accept the loss, recorded. Needs a UX call — Andi's.
- [x] [Review][Decision] (Andi 2026-09-14 → **D2**, option (b) plus new wording, applied) **`ClipboardWriteFailed` — the one total-data-loss case — is rendered amber `Warning`, not danger** — `overlay_message::DegradeCause::card`, `ClipboardWriteFailed` arm (`tone: MessageTone::Warning`, header `CLEANUP FAILED`). The text reached neither the window nor the clipboard, yet the card wears the same amber as the *successful* clipboard fallback, the pill shows the same amber `Cleanup failed` label, and the pipeline state is `Done`. AC8 assigns `--k-danger` to errors. The choice is deliberate and documented in the KDoc ("the cleanup family is one colour on the canon board"), but it was never put to Andi. Options: (a) keep amber as one family; (b) `MessageTone::Error` + a distinct header for this variant. Wording/design call — Andi's.
- [x] [Review][Decision] (Andi 2026-09-14 → **D3**, option (a), applied) **Machine-readable error tokens now get a prominent, untruncated 4 s card** — `hotkey::PipelineEvent::error` → `OverlayMessage::error(msg)`. `pipeline.rs` emits `PipelineEvent::error("feature_requires_license:CommandMode")`, which the card renders verbatim as its cause line. Pre-existing content — the pill's old `Error` arm showed the same string, truncated — but AC8 promotes it from a cut-off fragment to a full, readable four-line card. Options: (a) map the known tokens to user wording; (b) accept and record. Needs a wording decision.

- [x] [Review][Patch] A message-less pipeline event dismisses a live card, so the STT-ladder warning and the boot-time config warnings lose their only surface [`src-tauri/src/lib.rs::emit_pipeline_state` (the unconditional `preview.set_message(event.message.clone())`) + `src-tauri/src/native_preview.rs::preview_wnd_proc` `WM_PREVIEW_SET_MESSAGE` `None` arm]
- [x] [Review][Patch] The card fills with the user's configurable preview background while every text run is a fixed canon dark-theme colour — the shipped "Light" theme renders the failure message near-white on near-white [`src-tauri/src/native_preview.rs::render_message_card` vs `src/components/settings/previewAppearance.ts::PREVIEW_THEMES`]
- [x] [Review][Patch] The card's typeface is `Segoe UI`, not the bundled `Geist` the canon pins and the pill beside it uses [`src-tauri/src/native_preview.rs::MSG_SANS_FACE`]
- [x] [Review][Patch] False provenance comment: "Cascadia Code ships with the app's font set" — only the three Geist faces are bundled [`src-tauri/src/native_preview.rs`, the `MSG_MONO_FACE` doc comment]
- [x] [Review][Patch] Record claim false: the card's **radius** does not follow the user's appearance settings [story Dev Agent Record, "What AC8 changed" table, row *Border width* / `src-tauri/src/native_preview.rs::render_message_card`]
- [x] [Review][Patch] `degrade_cause_for_model`'s docstring calls it "the one-line projection of that cause" (it returns `DegradeCause`) and links `[degrade_warn_msg]`, which now exists only inside `mod tests` [`src-tauri/src/pipeline.rs::degrade_cause_for_model`]
- [x] [Review][Patch] The model-ID chip rectangle ignores the accessibility `text_scale` while its glyphs honour it [`src-tauri/src/native_preview.rs::render_message_card`, the chip geometry block]
- [x] [Review][Patch] More than one boot-time config warning: each card replaces the previous one instantly, so only the last is ever readable [`src-tauri/src/lib.rs::run`, the `config_warnings` loop]
- [x] [Review][Patch] `SetTimer`'s return value is ignored and `msg_timer_active` is set anyway — on failure the card never fades, never dismisses, and the window stays non-click-through [`src-tauri/src/native_preview.rs::preview_wnd_proc` `WM_PREVIEW_SET_MESSAGE`]
- [x] [Review][Patch] `Pixmap::new` failure returns the fade alpha, so the caller presents the previous frame as the message [`src-tauri/src/native_preview.rs::render_message_card`]
- [x] [Review][Patch] The chip fallback drops the model ID's delimiters — `Model deepseek-typo not found` instead of AC8's `Model 'deepseek-typo' not found` [`src-tauri/src/overlay_message.rs::CauseLine::text` + `DegradeCause::card`]

- [x] [Review][Defer] `WM_PREVIEW_SET_MESSAGE` leaks the boxed message when the state pointer is null — the early return precedes `Box::from_raw` [`src-tauri/src/native_preview.rs::preview_wnd_proc`] — deferred, unreachable today (`GWLP_USERDATA` is set in `WM_CREATE`, before any posted message)
- [x] [Review][Defer] `card_h` is clamped to `max_card_h` but the text tops stay derived from the unclamped `body_h`, so overflow lines are drawn below the card; an unbreakable token wider than the card is clipped mid-token [`src-tauri/src/native_preview.rs::render_message_card`, `wrap_text_lines`] — deferred, disclosed in the coverage statement and unreachable with today's four message shapes; note the coverage bullet says "clipped at the top", it is the bottom
- [x] [Review][Defer] Fade-end and click-dismiss hide the window unconditionally instead of falling back to an armed live preview, unlike the `None` arm [`src-tauri/src/native_preview.rs::render_frame` message branch + the `WM_LBUTTONDOWN` arm] — deferred, effectively unreachable: the state event accompanying every message disarms the live preview microseconds later
- [x] [Review][Defer] `TerminalKind::Degraded` is inferred from `message.is_some()` — an invariant held by comment, not by the type [`src-tauri/src/lib.rs::emit_pipeline_state`] — deferred, sound today and pinned from the producer side by `hotkey::tests::spec_focus_failure_clipboard_only_carries_no_card`
- [x] [Review][Defer] The card is fully re-measured, re-wrapped, re-laid-out and re-composited ~30×/s for 5 s to animate what is only a `SourceConstantAlpha` change [`src-tauri/src/native_preview.rs::render_message_card` via `WM_TIMER`] — deferred, performance only
- [x] [Review][Defer] `spec_pill_labels_are_static_literals` is near-vacuous — the bound is 14 chars and the longest label is exactly 14 [`src-tauri/src/overlay_message.rs::tests`] — deferred, the story already states the real proof is GATE-4
- [x] [Review][Defer] Removing `fit_text` leaves no clip guard at all for a future or localised pill label [`src-tauri/src/native_pill.rs::draw_static_label`] — deferred, today's five labels are fixed literals
- [x] [Review][Defer] `generic_cause_text` strips arbitrarily many leading colons and asserts "The cleanup provider did not respond" whenever the reason is empty [`src-tauri/src/overlay_message.rs::generic_cause_text`] — deferred, low reachability
- [x] [Review][Defer] The `STATE_RECORDING` branch dismisses the card but neither repaints nor hides — a defensive branch that would not defend [`src-tauri/src/native_preview.rs::preview_wnd_proc` `WM_PREVIEW_SET_STATE`] — deferred, unreachable via `emit_pipeline_state`, which posts its own `set_message(None)` first
- [x] [Review][Defer] `SetWindowLongPtrW(GWL_EXSTYLE)` is not followed by a `SWP_FRAMECHANGED` `SetWindowPos` [`src-tauri/src/native_preview.rs::set_click_through`] — deferred, uncertain for `WS_EX_TRANSPARENT` hit-testing and Windows-gated with no coverage; one item for GATE-4 to watch

**Dismissed as noise (11).** Clearing `WS_EX_TRANSPARENT` makes the whole window rect eat clicks (false — `present` uses `UpdateLayeredWindow` with `ULW_ALPHA`/`AC_SRC_ALPHA`, so per-pixel alpha-0 areas stay click-through and only the painted card catches the click) · `*g = Some(preview)` drops the old preview under the mutex → deadlock shape (false — `Drop for NativePreview` only `PostMessageW`s `WM_PREVIEW_SHUTDOWN`, no join, no lock) · two `config.lock()` calls give a torn snapshot (pre-existing, same shape before AC8) · `#[allow(dead_code)]` on the pill labels also silences Windows (they are referenced by `native_pill.rs` there) · the `!json.contains("CLEANUP FAILED")` assertion is weak (the real guarantee is the `#[serde(skip_serializing)]` attribute) · `render_message_card`'s `else { return 0 }` and `WM_TIMER`'s `else` are dead (harmless defensive code) · creating the preview unconditionally is a regression for users with live preview off (that is exactly what AC8 requires) · a poisoned `native_preview` mutex loses the message (pre-existing shape, no new path) · `NativePreview::create` failing at both sites leaves no surface (logged; a fallback dialog would be new UI → AC4) · `GetWindowLongPtrW` returning 0 on error in `set_click_through` (theoretical — `GWL_EXSTYLE` is never 0 on a live layered window) · `MSG_HEADER_PX` 10.0 vs the render's 10.5 (sub-pixel after the `as i32` truncation at every scale factor).

**Machine gates re-run by the review, not taken from the record:** `cargo test --manifest-path src-tauri/Cargo.toml --lib` → **701 passed, 0 failed** (matches). `cargo build --lib` → 13 warnings, all pre-existing (matches). Parse check on `native_pill.rs` / `native_preview.rs` / `overlay_message.rs` → exit 0 (matches, and proves parsing only). 7 specs in `overlay_message.rs` + 3 new in `hotkey/mod.rs` = 10 (matches). `md5(Klarvo Design System.html + assets/klarvo.css)` matches the MANIFEST header. `fit_text`, `status_msg`, `pending_msg`, `WM_PILL_SET_MSG` and `set_status_msg` are gone from the tree (matches). No new `.emit(` or `klarvo://` line — trap #5 holds. `git status` clean apart from the untracked `gate4-evidence/7-10/windows-build-r2.exit`. Gradle was **not** re-run (correctly — `git diff --stat 8b81864^..8b81864 -- android/ test-fixtures/ src/` is 0 lines); no puppeteer re-run, no Android device, no pixels.

**Not verified by this review:** `native_pill.rs` and `native_preview.rs` are `cfg(target_os = "windows")` with no test module — **every card and pill finding above was reached by reading the source and tracing the producer/consumer graph, not by running anything.** The conductor's Windows release build (`d03a4da`, exit 0) establishes that they *compile*; it says nothing about layout, fade, hit-testing or a single pixel. **Andi's GATE-4 remains outstanding and nothing here substitutes for it.**

## Dev Notes

### Verified current state (today's tree, `conductor/story-7-10` at `6cedbb5`)

Anchors are symbols, not line numbers (project-context rule). **Re-grep before editing** — every count below is
from today's tree, not from the story text.

**Desktop — the tail of `pipeline::stop_and_process_pipeline`, in order:**
`process_audio` (emits all progress/terminal events via the `emit` closure) → `deliver_outcome` (**drops
`llm_error`**) → `history::record_usage` ×2 → **paste** → **Insert+Send + Return-to-Current** → history write →
Turso push + webhook → feedback metrics → **the done event**.

| Concern | Symbol |
|---|---|
| Outcome | `pipeline::ProcessOutcome::Produced { cleaned_text, raw_text, is_command, stt_ms, llm_ms, prompt_tokens, completion_tokens, llm_error }` |
| Degrade sites (3, all `llm_error = true` + `emit(warn)`) | retryable+fallback-failed · retryable+no-fallback · non-retryable `Err(ref e)` arm |
| Retry predicate | `pipeline::is_retryable_llm_error` — `429 \|\| >= 500 \|\| LlmError::Request(_)` |
| Warning text | `pipeline::degrade_warn_msg_for_model` → `pipeline::degrade_warn_msg` / `pipeline::is_model_not_found_error` |
| Flag sink (today) | `pipeline::deliver_outcome` — `feedback_metrics.llm_error_count` only |
| Paste | `paste::create_paste_handler(prev_hwnd)` → `Box<dyn PasteHandler>`; `paste::PasteResult::{Pasted, ClipboardOnly}` |
| Auto-send | `PasteHandler::send_enter` (no-op default; `paste::windows::simulate_return()`), gated `insert_and_send && paste_result == PasteResult::Pasted`; flag = `AppState::active_insert_and_send: AtomicBool`, stored at recording start from the per-slot `HotkeySlot::insert_and_send` |
| Terminal event | `PipelineEvent::done` vs `PipelineEvent::done_with_clipboard_only`, chosen by `paste_result == PasteResult::ClipboardOnly` |
| Event bus | one name only: `hotkey::EVENT_STATE_CHANGED = "klarvo://state-changed"` (pinned by a test). There is **no** separate Done/DoneClipboard/warning event name |
| Emit funnel | `lib::emit_pipeline_state` — posts `set_status_msg(event.warning.or(event.error))` **before** `set_state(state, clipboard_only)` (FIFO) |
| Pill | `native_pill::NativePillState::{…, Done, DoneClipboard, Error, Warning}`, `from_code(code, clipboard_only)`, `warning_hold_active`, `WARNING_HOLD_MS = 4000`, `DONE_CLIPBOARD_MS = 4000` |

**`ClipboardOnly` is reached today by four routes** (only the first three are real paste outcomes):
no target HWND recorded · target window no longer valid (`IsWindow`) · focus verification failed
(`GetForegroundWindow() != hwnd`) · **plus** a coerced hard `PasteError` in `stop_and_process_pipeline`
(`m.paste_error_count` bumped). This story adds a **fifth, deliberate** route. The AC6 trap exists because these
are indistinguishable in a naive assertion.

**Android — `KlarvoOverlayService::processAudio` steps:** pre-STT guards → Step 1 STT → hallucination guard →
Step 2 cleanup (`finalText`) → Step 3 history → Step 3b Turso → **Step 4 clipboard + paste**.
Step-4 order today: `BankingGuard.shouldBlockPaste(bankingAppActive)` → `copyToClipboard(finalText)` →
`val pasted = KlarvoAccessibilityService.instance != null` → `pasteIntoFocusedField()` →
`if (!pasted) showToast("Copied: …")` → deferred `Toast.makeText(capturedDegradeMsg, LENGTH_LONG)` → metrics thread
→ `setState(DONE)` + `adjustLayoutForState` → `doneFlashRunnable` (800 ms) → AUTO-loop restart.

**Android degrade literals today** (all inline Kotlin, no string resources — `android/res-values/strings.xml` holds
only `app_name`, `main_activity_title`, `accessibility_service_description`):

| Situation | Literal | Is it a cleanup failure? |
|---|---|---|
| Fallback provider succeeded | `"⚠ Cleanup-Anbieter gewechselt"` | **No** — cleanup worked |
| Fallback also failed | `"⚠ Cleanup nicht verfügbar → Rohtext eingefügt"` | Yes |
| No fallback available / non-retryable | `"⚠ Cleanup nicht verfügbar → Rohtext eingefügt"` | Yes |
| No LLM key configured | `"Text pasted without cleanup (no LLM key configured)."` (immediate `showToast`) | **Q4** |
| Local MNN cleanup failed | *(none — silent)* | **Q4** |

### Traps and what must be preserved

- **`deliver_outcome` is the flag's grave.** Any fix that reads `llm_error` after that call without threading it
  through is impossible; any fix that re-derives it (e.g. `cleaned_text == raw_text`) is wrong — a successful
  cleanup can legitimately return the input unchanged, and `cleaned_text` on the degrade path is
  `strip_stockphrase_ghosts(sanitize_llm_output(raw_text))`, not `raw_text` verbatim.
- **`DoneClipboard` overrides a held Warning by design** (`warning_hold_active` docstring: "DoneClipboard (the text
  did NOT land)"). That rule was correct for the focus-failure case it was written for. Changing it must not
  resurrect 3a's root cause for other paths (`Error` and new activity must keep overriding).
- **The pill renders `DoneClipboard` as a static `"In Clipboard"` label** and ignores `status_msg`; only the `Error`
  and `Warning` arms render `s.status_msg` (through `fit_text` truncation at the pill's label width). Long text is
  truncated, not wrapped — see **Q3**.
- **The React `FloatingBar` is gone** (Epic 10 native overlays). `clipboardOnly` survives only in
  `src/types.ts::StateChangedPayload`; no component reads it. The main-window consumer is
  `src/hooks/useRecording.ts` — see Task 2.
- **`native_pill.rs` is `#[cfg(target_os = "windows")]` and has no test module** (`warning_hold_active`,
  `from_code` are untested). The AC3 display behaviour is **not machine-observable on Linux** — it is Andi's GATE-4
  (7-9's 3a was found exactly this way, after a green Linux suite). Do not claim it from a passing `cargo test`.
- **Never make the user the rendering oracle.** If the pill sequence needs iteration, isolate it deterministically
  (state-sequence reasoning, instrumented logging) before spending a Windows build cycle on a hypothesis
  (project-context, Epic-6 lesson).
- **Android: `pasted` is a lie today.** `KlarvoAccessibilityService.instance != null` only tests whether the
  service is connected; `pasteIntoFocusedField()` returns `Unit` and silently no-ops when `rootInActiveWindow` is
  null or there is no focused editable node. Do not build the new branch on that variable — pre-existing defect,
  not this story's to fix, but do not deepen it.
- **Android sanitization is at the source, not the sink.** `KlarvoApi.sanitizeLlmOutput` is applied on every raw
  degrade branch in Step 2 (Story 2-3); `copyToClipboard` is a raw sink with no guard of its own. Do not add a
  second sanitize, and do not remove the existing ones.
- **`BankingGuard.shouldBlockPaste` must stay the first statement** in the Step-4 block (Story 2-4, DIV-04): a
  banking app aborts the whole block *before* the clipboard write. Clipboard-only is not an exemption.
- **Chunk-failure semantics stay abort-on-first-error** (`KlarvoApi.collectChunkResults` unwraps
  `ExecutionException` → `IOException` precisely because the service gates the fallback on `e is IOException`).
  Do not touch it.
- **`Adr0017BoundaryGuardTest`** scans `android/kotlin-src/` for re-grown STT/guard code. No rule keys on
  paste/clipboard vocabulary, so Step-4 edits cannot trip it — but do not add STT-shaped code to Kotlin.
- **Do not delete `KlarvoAccessibilityService.performEnter`** (callerless since 7-9; deliberately kept).
- **Canon check performed (ADR-0019, `docs/design/overhaul/source/Klarvo Design System.html`):** the pill board
  pins *"Fläche bleibt 200×36 — kein Aufblasen"*, *"Fenster-Hintergrund transparent"*, *"Position persistiert
  (draggable)"* and *"clipboard-only → Amber „In Zwischenablage""*. It does **not** pin the degrade/warning text.
  The amber the code uses for both `Warning` and `DoneClipboard` (255,163,68 = `#FFA344`) is the pill's own
  pre-existing constant; the canon token is `--k-amber` `#E9A24C` (recorded as the deliberate replacement of
  `#FFA344`). D1's main-window status line uses the canon token, so the same string renders in two ambers.
  Pre-existing pill divergence, Windows-gated, no machine coverage → backlog (re-review 2026-09-14), not this story.
- **No host mutation for a gate.** If a gate's tool is missing, the gate is `blocked` — report it, do not install
  around it.

### Test seams — the honest state

- **Rust:** `stop_and_process_pipeline` takes a live `tauri::AppHandle`, is `async`, and touches locks, SQLite, the
  network (Turso/webhook) and `std::thread::sleep`. It is **not unit-testable today**, and there are **zero** tests
  covering `paste_result`, the Insert+Send gate, or the `done` vs `done_with_clipboard_only` selection.
  `PasteHandler` is a trait but there is **no injection seam**: the shell calls the free function
  `create_paste_handler(prev_hwnd)` directly and the concrete handler is chosen by `#[cfg(target_os)]`. Existing
  `paste::tests` cover only `PasteError::EmptyText`, `Display` strings and `PasteResult` `PartialEq`/`Copy`.
  → AC6's paste-level test requires the Task-1 seam (**S1**).
- **Kotlin:** no test constructs `KlarvoOverlayService` or calls `processAudio`; `copyToClipboard` is `private` and
  calls `getSystemService`; `pasteIntoFocusedField` needs a live `AccessibilityService`. **No Robolectric, no
  Mockito** anywhere in the tree. The repo's established answer is the pure-function-in-companion pattern
  (**S3**). `BankingGuardTest`'s own KDoc states the boundary honestly ("the full integration path … is covered by
  the on-device smoke") — mirror that honesty in the new test's KDoc.
- **Device-free JVM run** (7-8/7-9 precedent, when no AVD is reachable): sync `android/kotlin-src` +
  `android/kotlin-test` into `src-tauri/gen/android/app/src/{main,test}/java/com/klarvo/voice`, then
  `./gradlew :app:testUniversalDebugUnitTest` with `JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64` and
  `ANDROID_HOME=/home/andyon2/workspace/tools/android-sdk`. The AVD lives on the **laptop** (`emulator-5554`); boot
  only via `scripts/android-emulator.sh`, and stop it afterwards. `android-smoke.sh` empties the target `*.kt`
  folders before copying, and its banner reads ONE result XML — count from `app/build/test-results/**/*.xml`.

### Open questions — DECIDED at GATE 1 (Andi, 2026-09-14; conductor session)

**Binding answers** (anchored in the canon: `docs/design/overhaul/source/Klarvo Design System.html`, pill
"Constraint-treu" list + MANIFEST provenance row 2026-09-14). The original question text is kept below for context.

- **Q1 → ONE event.** On the degrade-to-raw path the pipeline emits **no separate `Warning`**; it emits exactly one
  terminal event: `DoneClipboard` carrying the warning text (`PipelineEvent::done_with_clipboard_only` + the
  `warning`/message field). The native pill renders `DoneClipboard` with that text in amber for the existing
  `DONE_CLIPBOARD_MS` (4 s) — when a message is present it replaces the static "In Clipboard" label. No change to
  `warning_hold_active` (it stays for the retry-ladder warning, which is unchanged). Every consumer of the done
  event (native pill, `useRecording.ts` / main window) must show the carried message so nothing is lost.
- **Q2 → cause first, hint last; D2 pointer is replaced by the clipboard hint on this path.** Generic:
  `Cleanup failed — raw text in clipboard (Ctrl+V)`. Model-not-found: `Model '<id>' not found — in clipboard`.
  The model ID stays at the front so it survives truncation. `friendly_error` short reason may follow the generic
  text only if it fits; the fixed part comes first.
- **Q3 → English, accept tail truncation, never widen.** All pill labels are English today; the canon's German
  "In Zwischenablage" vs the code's "In Clipboard" is a pre-existing divergence → **backlog note, not this story**.
- **Q4 → true cleanup failures only (desktop parity).** Android's clipboard-only branch fires on the explicit
  cleanup-failed boolean (cloud primary failed AND fallback failed/absent). *No LLM key configured* keeps today's
  paste. *Local MNN cleanup failed (silent)* is **out of scope → backlog note** (separate silent-failure bug).
- **Q5 → one combined toast.** On the clipboard-only path Step 4 shows exactly one `LENGTH_LONG` toast after
  `copyToClipboard`: the same wording as the pill (generic or model-not-found form). The `"Copied: …"` toast is
  suppressed on this path. English, mirroring the desktop pill literally.
  **GATE-2 addendum (Andi, 2026-09-14):** the Android toast drops the key hint — `Cleanup failed — raw text in
  clipboard` (no "(Ctrl+V)"; there is no Ctrl+V on a phone). The model-not-found form stays identical to the pill.

---

**Original open questions (context only — answered above):**


- **Q1 — Which AC3 shape?** The decision record offers exactly two and picks neither: **(a)** ONE event carries the
  warning text + the "in the clipboard" meaning, or **(b)** `DoneClipboard` carries the warning text. Both are
  buildable today (`PipelineEvent` already has `warning` *and* `clipboard_only`). Consequences differ: (a) means
  the pill never enters a Warning state on this path (so `warning_hold_active` never engages) and
  `useRecording.ts`'s `p.state === "warning"` branch never fires — the main window would lose the message unless
  the consumer is also changed; (b) keeps the Warning→DoneClipboard sequence and requires relaxing
  `warning_hold_active`'s "DoneClipboard still overrides" rule. Andi's constraint is only the outcome: *"Sonst
  verliert der Nutzer die Model-ID wieder."*
- **Q2 — How do the clipboard hint and the model-ID text combine?** The target wording is *"Cleanup failed — raw
  text in the clipboard, Ctrl+V to paste"*. D2's model-not-found message is a **replacement**, not a suffix:
  `degrade_warn_msg_for_model` returns `Model '<id>' not found — check Advanced → Model IDs` and never reaches
  `degrade_warn_msg`. So a model-not-found failure currently carries **no** clipboard hint. Options: keep D2
  verbatim (user learns the cause but not that the text is in the clipboard) · append the clipboard hint to D2
  (longer — see Q3) · replace "check Advanced → Model IDs" with the clipboard hint (loses the 7-9 fix's pointer).
  The epic says only "keeps naming the model ID".
- **Q3 — Pill text length (and language).** `native_pill.rs` truncates the status text with `fit_text` to the
  label width (single line, no wrap). "Cleanup failed — raw text in the clipboard, Ctrl+V to paste" is markedly
  longer than today's "Cleanup failed — raw text inserted." and longer still if Q2 concatenates. Options: shorten
  the canonical wording · accept truncation. **Canon settles one option out:** `docs/design/overhaul/source/Klarvo
  Design System.html` pins *"Fläche bleibt 200×36 — kein Aufblasen"* — widening the pill is **not** available
  (also AC4). **Canon also pins the clipboard-only label in German** — *"clipboard-only → Amber „In
  Zwischenablage"* — while the code renders the English `"In Clipboard"` (pre-existing divergence, not this
  story's to fix unless Q1 option (b) rewrites that arm). Canon does **not** pin the degrade/warning text itself.
- **Q4 — Which Android non-failures count?** The epic's trigger is "an LLM failure" (desktop: `llm_error == true`).
  Two Android states produce filler-laden raw text without being a cloud-cleanup failure: **no LLM key configured**
  (cleanup deliberately skipped; toast "Text pasted without cleanup…") and **local MNN cleanup failed** (silent
  today, sets no `degradeStatusMsg`). The user-harm rationale ("filler-laden raw text is never inserted") arguably
  covers both; the desktop twin has the analogous `LlmPath::OfflineRaw` / no-key paths where `llm_error` stays
  `false`. Options: clipboard-only for all raw-text outcomes · only for true cleanup failures (literal reading) ·
  per-case. Affects both platforms' twin symmetry.
- **Q5 — Android toast composition.** Today Step 4 can show up to two toasts: `showToast("Copied: $preview")`
  (`LENGTH_SHORT`, only when the accessibility service is absent) and the deferred degrade toast (`LENGTH_LONG`).
  On the new path nothing is pasted, so the "Copied:" condition's meaning changes. The in-code note says HyperOS's
  own "pasted from your clipboard" system toast competes and **the newest toast wins**. Options: one combined
  toast · suppress "Copied:" and keep only the degrade toast · keep both and fix the ordering. Related to
  **Q2/Q3** (German vs English: the desktop pill's *warning* text is English, the Android degrade literals are
  German, and the epic asks the toast to "mirror the desktop pill" — whether that means the same wording or its
  German equivalent is not pinned. Note canon writes the pill's clipboard-only label in German, "In
  Zwischenablage" — see Q3.)

### Scope assumptions made by this story file (flagged, not decided)

- **S1 — A Rust seam is unavoidable.** The epic requires "a paste-level test (`llm_error` → `ClipboardOnly`, no
  Enter)". No such test can exist without extracting the paste/Insert+Send/terminal-event tail behind a
  `&dyn PasteHandler` parameter. The story assumes that behaviour-preserving extraction is in scope; the epic does
  not name it.
- **S2 — `deliver_outcome`'s signature changes** (7 → 8 fields). Unavoidable for AC1; touches two existing tests.
- **S3 — A Kotlin pure-function seam is unavoidable** for the required Kotlin twin test, plus an explicit
  `llmFailed` boolean, because Step 4 has no failure flag today and `degradeStatusMsg` is set on a **success**
  case too. The epic says only "Kotlin twin test for Step 4".
- **S4 — The Android auto-send half of AC2 is already satisfied** (7-9 removed M13; `performEnter` has no caller).
  The story treats it as a record statement, not work.
- **S5 — AC3 is not Linux-verifiable.** `native_pill.rs` is Windows-gated with no tests; the display outcome rests
  on Andi's GATE-4. The story does not promise a machine gate for it.
- **S6 — No `test-fixtures/` change is expected.** This is not a twin-constant lock (7-8's fixture net pins
  constants, not control flow), so the gradle `--rerun-tasks` fixture trap should not apply. If the dev agent does
  touch a fixture, `--rerun-tasks` becomes mandatory.

### Previous-story intelligence (7-9, same epic — this story exists because of its GATE-4)

- **A green Linux suite proved nothing about the pill.** 7-9 shipped with `cargo test --lib` 674→681 green and a
  28/28 proxy smoke; Andi's GATE-4 still found the warning structurally invisible (3a) and the DeepSeek classifier
  blind to the live wording (3b). Both defects lived in exactly the code this story touches.
- **External-fact verification gap (3b).** 7-9's fixture fed the *whole JSON body* into `LlmError::message`, a
  shape the live extraction never produces — the spec test was green against a synthetic input. If this story
  asserts anything about provider wording, assert the **observed** shape (the verbatim `Klarvo.log` line is in
  `gate4-evidence/7-9/verdict.md`).
- **Inversion discipline:** a fixed inversion must fail *differently* than the bug did; mark any
  non-discriminating inversion as such (7-8 row E).
- **Record hygiene (Epic-7 retro D2):** anchor at `file::symbol`, never line numbers — a `file:line` reference ages
  inside the very commit that writes it. Write resolution rows from `git diff`, not memory. A re-review whose
  findings are record-only text goes to an editorial close-out pass, not a fix round.
- **Grep before declaring done.** Re-verify every count against today's tree (7-9's "14 keys" was 13).

### Git intelligence

The last four commits are this story's planning trail: `3b6f909` (backlog DECIDED 2026-09-13), `62985d6`
(waypoint), `6a9c9a3` (epic-7 back to in-progress for this home), `6cedbb5` (story homed in the epic). The code
this story touches was last shaped by **`06603d2`** (7-9 fix round: `native_pill.rs` `warning_hold_active` +
`WARNING_HOLD_MS`, and `pipeline::is_model_not_found_error`'s live-wording needles) — read that commit before
touching the pill. The paste/auto-send tail is Epic-10/12 territory (`f53da68` native overlays merge, `130c636`
fallback ladder). Small scoped commits, **never `git add .`**; one commit per platform is acceptable.

### Latest technical information

No web research was performed (no web access in this session). This story adds **no dependency** and changes no
library version — it reuses `arboard` (already the clipboard writer on both desktop targets) and Android's
`ClipboardManager`. Nothing here is version-sensitive.

### Project Structure Notes

- Rust tests stay **inline `#[cfg(test)]`**; no new file under `src-tauri/tests/` (that path is reserved for
  `pi_security.rs`).
- Kotlin sources in `android/kotlin-src/`, tests in `android/kotlin-test/` — **never** in `gen/android/`
  (generated, gitignored, and emptied by `android-smoke.sh`).
- Code and comments English; chat German; commit subjects English.
- Module files `snake_case.rs`; Kotlin classes `PascalCase` under `com.klarvo.voice`.
- **Factor out only on proven duplication** (≥2 real consumers) — keep the new decision helpers module-local.
- No new dependency; do not run `npm install` (the Windows build runs `npm ci`).

### References

- [Source: _bmad-output/planning-artifacts/epics-cross-platform-parity.md#Story 7.10] — ACs, out-of-scope, DoD.
- [Source: docs/backlog.md#DECIDED 2026-09-13 — Cleanup-Fehler: Rohtext NUR in die Zwischenablage] — the decision,
  Option C, the rejected button idea, the design constraint from 3a.
- [Source: docs/backlog.md#STORY-CANDIDATE — Failed-entries inbox] — the deliberately deferred sibling.
- [Source: _bmad-output/implementation-artifacts/7-9-desktop-advanced-settings-dead-keys-and-model-ids.md] — D2
  (warning names the model), review/inversion discipline, gate procedure.
- [Source: _bmad-output/implementation-artifacts/gate4-evidence/7-9/verdict.md] — findings 3a + 3b, the verbatim
  DeepSeek 400 log line, the fix round, Andi's re-check.
- [Source: _bmad-output/implementation-artifacts/epic-7-retro-2026-09-10.md] — D2 record rule.
- [Source: docs/adr/0017-shared-core-stt-path.md] — STT-only Hard Rule; cleanup/paste are twins.
- [Source: docs/adr/0016-android-path-parity-strategy.md#Amendment 1] — parity scope.
- [Source: docs/surface-smoke-checklist.md] — trap #5 (event push-wiring).
- [Source: src-tauri/src/pipeline.rs] — `ProcessOutcome`, `process_audio`, `deliver_outcome`,
  `stop_and_process_pipeline`, `degrade_warn_msg`, `degrade_warn_msg_for_model`, `is_model_not_found_error`,
  `is_retryable_llm_error`, `tests::{run, test_process_audio_nonretryable_degrades_to_raw, test_deliver_outcome_*,
  spec_model_not_found_warning_names_the_model}`.
- [Source: src-tauri/src/paste/mod.rs] — `PasteHandler`, `PasteResult`, `PasteError`, `create_paste_handler`,
  `capture_foreground_window`, `restore_focus`.
- [Source: src-tauri/src/paste/windows.rs] — `WindowsPasteHandler::paste`, `simulate_ctrl_v`, `simulate_return`,
  `reliable_set_foreground`. · [Source: src-tauri/src/paste/linux.rs] — `set_clipboard`, `simulate_ctrl_v`.
- [Source: src-tauri/src/hotkey/mod.rs] — `PipelineState`, `PipelineEvent` (+ `done`,
  `done_with_clipboard_only`, `warn`, `error`), `EVENT_STATE_CHANGED`.
- [Source: src-tauri/src/lib.rs] — `emit_pipeline_state`, `friendly_error`, `update_tray_tooltip`,
  `AppState::active_insert_and_send`.
- [Source: src-tauri/src/native_pill.rs] — `NativePillState`, `from_code`, `warning_hold_active`,
  `WARNING_HOLD_MS`, `DONE_CLIPBOARD_MS`, `handle_timer`, `pill_wnd_proc`, the `DoneClipboard`/`Warning` render
  arms, `fit_text`.
- [Source: src-tauri/src/history/mod.rs] — `add_entry`, `add_pending_entry`.
- [Source: src/hooks/useRecording.ts] · [Source: src/tauri-commands.ts] (`onStateChanged`) ·
  [Source: src/types.ts] (`StateChangedPayload.clipboardOnly`).
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt] — `processAudio` Steps 2 + 4,
  `degradeStatusMsg`, `copyToClipboard`, `showToast`, `setState`, `doneFlashRunnable`, `bankingAppActive`,
  `onBankingAppStateChanged`, the companion-object pure functions.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoAccessibilityService.kt] — `pasteIntoFocusedField`,
  `findFocusedEditable`, `performEnter` (callerless).
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoApi.kt] — `cleanup`, `cleanupChunked`, `collectChunkResults`,
  `cleanupLocal`, `sanitizeLlmOutput`, `resolveLlmProvider`, `resolveFallbackLlmProvider`, `saveToHistory`.
- [Source: android/kotlin-src/com/klarvo/voice/BankingGuard.kt] — `shouldBlockPaste`.
- [Source: android/kotlin-test/com/klarvo/voice/BankingGuardTest.kt] · [Source:
  android/kotlin-test/com/klarvo/voice/ChunkingParityTest.kt] — the seam + honesty precedent.
- [Source: test-fixtures/README.md] — the parity-net ledger and the two commands that run it.
- [Source: _bmad-output/project-context.md] — twins vs shared core, gates, symbol anchors, "a number states what
  it covers", never the user as rendering oracle, no host mutation.

## GATE-4 — round 1 (Andi, Windows, build `d73082d`, 2026-09-14): FAILED on presentation

Observed: the pill shows the amber warning, but the 200×36 label cuts the text mid-sentence; the clipboard part is
never visible. Functional core (nothing pasted, no Enter) not yet confirmed by Andi — re-check in round 2.
Cause named before any change: the message (tier 3: cause + consequence + action) was placed on the status light
(tier 2: ~25 chars). Decision (Andi, brainstorm + mock 2026-09-14): Option A — pill = status light, the existing
preview card = message surface, for every pipeline message. Recorded as AC8 + Task 6; canon + MANIFEST updated;
render `docs/design/overhaul/mockup-7-10-message-card.html`. Story re-opened (`in-progress`, both fields).

## Dev Agent Record

### Agent Model Used

Claude Opus 5 (`claude-opus-5`), `bmad-dev-story`, 2026-09-14.

### Debug Log References

Gates run on powerhouse (Linux). No device, no Windows build in this session.

| Gate | Command | Result |
|---|---|---|
| Rust unit suite | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | `test result: ok. 688 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| Kotlin JVM gate | `./gradlew :app:testUniversalDebugUnitTest` (device-free sync per 7-8/7-9) | `BUILD SUCCESSFUL`; counted from `app/build/test-results/testUniversalDebugUnitTest/*.xml`: **24 suites, 194 tests, 0 failures, 0 errors, 0 skipped** |
| Frontend build | `npm run build` (`tsc && vite build`) | `✓ built in 1.52s`, tsc clean |
| Lint | `cargo clippy` | **blocked** — `'cargo-clippy' is not installed for the toolchain`. Not installed around (project-context: never mutate the host for a gate). Not an AC7 item. |
| Trap #5 | mechanical, `docs/surface-smoke-checklist.md` | pass — see below |

**GATE-2 re-run (2026-09-14, Android toast wording — same host, Linux, no device):**

| Gate | Command | Result |
|---|---|---|
| Kotlin JVM gate — **RED first** (wording test retargeted, constant still old) | `./gradlew :app:testUniversalDebugUnitTest` | `195 tests completed, 2 failed`; `BUILD FAILED` — `degradeMessage_mirrorsDesktopPillWording` `org.junit.ComparisonFailure at CleanupFailureDeliveryTest.kt:156`, `degradeMessage_hasNoKeyHintOnAndroid` `java.lang.AssertionError at CleanupFailureDeliveryTest.kt:165` |
| Kotlin JVM gate — **GREEN after** the constant change | same | `BUILD SUCCESSFUL`; counted from `app/build/test-results/testUniversalDebugUnitTest/*.xml`: **24 suites, 195 tests, 0 failures, 0 errors, 0 skipped** |
| Rust unit suite (regression — desktop untouched) | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | `test result: ok. 688 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` — unchanged from the first round, as expected |
| Scope check | `git diff --stat` + `grep -rn "Ctrl+V" android/` | 2 files, both `android/`; no `Ctrl+V` remains in any user-facing literal (3 remaining hits are 2 KDoc lines + the negative assertion) |

Kotlin went 194 → **195** tests (+1 negative assertion), suites unchanged at **24**. `test-fixtures/`
untouched again, so the gradle `--rerun-tasks` fixture trap still did not apply. **This re-run was
ordinary red→green, not an AC6 inversion** — the AC6 inversion table below is from the first round and
was not re-executed; the delivery logic it covers was not touched.

**Counts are re-verified against today's tree, not against the story text.** Two story-text
numbers were stale and are corrected here: the story cites `src-tauri/src/paste/windows.rs` and
`paste/linux.rs` — those files do not exist; `windows` and `linux` are **inline `mod` blocks inside
`src-tauri/src/paste/mod.rs`**. The symbol anchors themselves were correct.

Rust baseline was 681 at 7-9 → **688** (+7 new tests). Kotlin was 23 suites → **24** (+1 file, +8 tests;
186 → 194).

**Trap #5 (push, not poll; wire the event end-to-end; colon form) — executed, not attested:**
this story introduces **no new event name and no new `.emit()` call** (`git diff | grep -E 'klarvo://|klarvo\.|\.emit\('` on added lines → empty). The message rides the existing
`hotkey::EVENT_STATE_CHANGED = "klarvo://state-changed"` (colon form, pinned by
`test_event_name_constant`). Chain verified hop by hop: producer
`pipeline::stop_and_process_pipeline` → `PipelineEvent::done_with_clipboard_only(…, degrade_msg)`;
funnel `lib::emit_pipeline_state` → `set_status_msg(event.warning.or(event.error))` posted **before**
`set_state` (FIFO); consumer A `native_pill.rs` `DoneClipboard` arm now reads `s.status_msg`;
consumer B `src/hooks/useRecording.ts` is a push sink (`onStateChanged`) and now reads `p.warning`
**before** the `p.state === "warning"` early return.

**REVIEW ROUND 1 — FIX ROUND (2026-09-14, same host: Linux, no device, no Windows build):**

| Gate | Command | Result |
|---|---|---|
| Rust unit suite | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | `test result: ok. 691 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` (688 → **691**, +3 for `terminal_degrade_msg`) |
| Kotlin JVM gate | `./gradlew :app:testUniversalDebugUnitTest` (device-free sync per 7-8/7-9) | `BUILD SUCCESSFUL`; counted from `app/build/test-results/testUniversalDebugUnitTest/*.xml`: **24 suites, 194 tests, 0 failures, 0 errors, 0 skipped** (195 → **194**, −1: the duplicate test finding 4 named was removed, not replaced) |
| Frontend build | `npm run build` (`tsc && vite build`) | `✓ built in 1.52s`, tsc clean |
| D1 proxy smoke | `bash _bmad-output/implementation-artifacts/gate4-evidence/7-10/run-d1-smoke.sh` | **10/10 checks passed** (round-1 artifact preserved as `report-round1.txt`; `report.txt` is round 2's 13/13) |
| Scope check | `git status` after both throwaway inversions and the smoke's throwaway edit | clean — `src/tauri-commands.ts` untouched, no inversion residue |

**Why the D1 test is a puppeteer harness and not a unit test.** This repo has **no JS test runner** (`package.json` has no vitest/jest), and adding one is forbidden (project-context: no `npm install` from a story; the Windows build runs `npm ci`). The documented desktop MACHINE gate for a React surface is puppeteer against `npm run preview` in real Chromium, with states the mocks cannot produce made by a **throwaway edit to `src/tauri-commands.ts`**, measured, then restored. `run-d1-smoke.sh` performs that edit, refuses to run if the file is already dirty, and reverts it in a `trap`; the run printed `throwaway edit reverted; src/tauri-commands.ts clean.`

**Fix-round inversions — every changed behaviour shown RED first.** `git status` clean afterwards.

| # | Finding | Reverted change (symbol) | Went RED | Verbatim failure | Discriminating? |
|---|---|---|---|---|---|
| 1 | 6 | `pipeline::terminal_degrade_msg` — the `paste_failed` arm made a pass-through | `pipeline::tests::spec_failed_clipboard_write_stops_promising_the_clipboard` | `690 passed; 1 failed`, `panicked at src/pipeline.rs:5407` | **Yes.** The assertion is "no `in clipboard` / `Ctrl+V` while `paste_failed`", which only the coercion bug can produce; the two sibling specs pin the untouched paths so a blanket `None` would fail them instead. |
| 2 | D1 (text) | `App.tsx` — the `warningMessage && state === "done"` text branch deleted | D1 smoke, 3 checks | `7/10 checks passed`; `D1: status line shows the degrade cause verbatim`, `…the model ID survives…`, `…the 'Done' label is replaced` | **Yes.** Asserted against the verbatim pill wording incl. the model ID, so a generic "some text is shown" could not pass it. |
| 3 | D1 (colour) | `App.tsx` — the done-state colour pinned back to `text-klarvo-teal` | D1 smoke, 1 check | `9/10 checks passed`; `D1: rendered in the canon amber token` | **Yes.** Computed `rgb()` compared to the literal `--color-klarvo-amber` value, not to a class name — a rebound Tailwind class cannot pass it. The text and colour halves fail **independently**, so neither masks the other. |

**Findings 1 and 3 have NO test and cannot have one.** Both are in `native_pill.rs`, which is `#[cfg(target_os = "windows")]` and has no test module — it is **not compiled at all** by `cargo test --lib` on this host, so the 691 green says nothing about it, not even that it compiles. `cargo check --target x86_64-pc-windows-gnu` is a **retired** gate (project-context) and was not attempted. This is S5 unchanged: the staging fix rides on Andi's GATE-4.

**Resolution rows — written from `git diff`, anchored at `file::symbol`:**

| # | Finding | What changed |
|---|---|---|
| 1 | Stale `status_msg` bleeds into a message-less `DoneClipboard` | `native_pill::PillWindowState` gains `pending_msg: Option<Option<String>>`. `WM_PILL_SET_MSG` no longer applies the message — it **stages** it. `WM_PILL_SET_STATE` takes the staged slot on every accepted state entry (`s.status_msg = s.pending_msg.take().flatten()`) and discards it along with a Done it drops. **The review's first suggestion — "clear `status_msg` on state entry" — is not implementable as written:** `emit_pipeline_state` posts msg **before** state (FIFO, verified as its only caller), so an unconditional clear in `SET_STATE` would erase the message that state just arrived with. Staging is that suggestion made correct: the message is owned by its own state and no state can inherit the previous one's. |
| 2 | `degrade_msg` invariant overclaims coverage | Stated honestly on **both** sides. `ProcessOutcome::Produced.degrade_msg`'s KDoc no longer says "every degrade site sets both" as a tested claim: it names 2 of 3 as pinned and the third as held by review. `spec_degrade_msg_present_exactly_when_llm_error`'s docstring names *which* two and *why* the third is unreachable (`make_input` uses `AppConfig::default()` → no keys → `resolve_fallback_provider` returns `None` → `Retryable` always lands in the no-fallback branch). **The test was not extended**: the fallback provider is a real HTTP client built inside `process_audio` from config keys with no injection seam, so reaching the fallback-also-failed site means a live network call. Adding that seam is a refactor this fix round has no mandate for. |
| 3 | Stale comments describe the Warning→Done sequence Q1 removed | `native_pill::handle_timer`'s warning branch and `warning_hold_active`'s docstring rewritten to name the **STT fallback ladder** (`process_audio`'s `emit(PipelineEvent::warn("⚠ Groq am Limit → lokale Transkription"))` — grepped, the only surviving `PipelineEvent::warn` in the tree) as the sole remaining Warning producer, and to say explicitly that cleanup degrade-to-raw never engages the hold. `NativePill::set_status_msg`'s doc updated to the staging contract. |
| 4 | Duplicate Kotlin test wearing a misleading name | `successfulFallbackProvider_isNotACleanupFailure_andStillPastes` **removed** — it was call- and assert-identical to `cleanupOk_pastesWhenAccessibilityConnected` and handed `llmCleanupFailed = false` in by hand, so it could not detect the danger its name claimed. The claim is **bound to the surviving test's KDoc** with its honest scope: the successful-fallback case reaches Step 4 with the flag false, and what is asserted is the *consequence* of that; whether Step 2 leaves it false on that branch is a `processAudio` branch and stays in the "does NOT cover" list. Not merged into a third test — binding it to the real one is the point. |
| 5 | `Delivery::enter_sent` doc says "Return was sent" | Documented as **"Insert+Send was triggered"**, with the reason the `Err` case still counts: focus has moved to the paste target either way, so Return-to-Current is as necessary after a failed Return as after a successful one. Behaviour deliberately unchanged — binding it to `Ok` would silently drop Return-to-Current on exactly the runs that need it most. |
| 6 | The degrade message claims "raw text in clipboard" when the clipboard write failed | New pure `pipeline::terminal_degrade_msg(degrade_msg, paste_failed)`, called in the shell between `deliver_text` and the terminal event. On `paste_failed` the cause is **replaced**, not dropped, with `Cleanup failed — clipboard write failed`. Dropping it was the other option the review offered and is worse: the `None` path falls back to the static `"In Clipboard"` label, i.e. the same false claim minus the cause. The model ID is lost on this path (it stays in `Klarvo.log`) — this is the double-failure case, not the one 7-9 D2 was written for. |
| 7 | Task 1 claims the two `test_deliver_outcome_*` tests were updated | Corrected in Task 1 itself. `git diff 6cedbb5..HEAD -- src-tauri/src/pipeline.rs \| grep -c test_deliver_outcome` → **0**. Both tests drive `ProcessOutcome::Stopped` and assert `result.is_none()`, so the `Produced` arm's tuple width is invisible to them — they did not need updating. Editorial close-out per Epic-7 retro D2. |
| D1 | Main window never shows the carried message (decision) | Andi's directive, verbatim: the existing status line, amber, `done`-with-warning, same wording as the pill, no new surface. `src/App.tsx` status `<p>`: the colour ternary's `done` arm becomes `warningMessage ? amber : teal`, and the text ternary gains one `warningMessage && state === "done"` branch ahead of `STATUS_LABELS`. **No new element, no new attribute, no new component** — AC4 holds. Q1's "every consumer shows the carried message" is now met for both consumers. |

**REVIEW ROUND 2 — FIX ROUND (2026-09-14, same host: Linux, no device, no Windows build).** Exactly one
confirmed finding: the round-1 flag below, promoted by Andi. Scope is that finding only.

| Gate | Command | Result |
|---|---|---|
| D1 proxy smoke (extended) | `bash _bmad-output/implementation-artifacts/gate4-evidence/7-10/run-d1-smoke.sh` | **13/13 checks passed** (10 → 13, +3 for the round-2 sequence) |
| Frontend build | `npm run build` (`tsc && vite build`) | `✓ built in 1.53s`, tsc clean |
| Rust unit suite (regression — backend untouched) | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | `test result: ok. 691 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` — unchanged from round 1, as expected |
| Kotlin JVM gate (regression — Kotlin untouched) | `./gradlew :app:testUniversalDebugUnitTest` (device-free sync per 7-8/7-9) | `BUILD SUCCESSFUL`, but **`:app:testUniversalDebugUnitTest` reported `UP-TO-DATE` — the suite was NOT re-executed this round.** The 24 suites / 194 tests / 0 failures counted from `app/build/test-results/**/*.xml` are round 1's execution, replayed. Honest reading: this round changed no `.kt` file (`git diff --stat` → zero `android/` paths), so gradle correctly found nothing to redo; it is a no-regression-possible statement, not a fresh green. |
| Scope check | `git status` after the RED run and the fix | clean — `src/tauri-commands.ts` untouched (the runner's `trap` reverted its throwaway edit and said so), no inversion residue |

**Inversion (AC6 discipline) — the RED was the pre-fix state itself, not a synthetic revert.**
`git status` clean afterwards.

| # | Finding | State shown RED | Test that went RED | Verbatim failure | Discriminating? |
|---|---|---|---|---|---|
| 1 | round-2 (stale `warningMessage`) | the tree **before** the `else` was added — i.e. the shipped round-1 code | D1 smoke, the 3 new `round 2:` checks | `10/13 checks passed`; `round 2: a clean run after a degraded run shows the plain done label` → `text="Model 'deepseek-typo' not found — in clipboard" expected="Done"`; `round 2: the previous run's degrade cause is gone, not stale`; `round 2: a clean run after a degraded run is teal, not amber` → `computed=rgb(233, 162, 76) expected=rgb(41, 199, 172)` | **Yes.** The failure names the **previous run's** literal, including its model ID — no generic "some label is wrong" could produce that string, and no fresh-boot run can reach it (CASE B, which reloads first, stayed green throughout). Text and colour fail independently, so neither masks the other. |

**Resolution row — written from `git diff`, anchored at `file::symbol`:**

| # | Finding | What changed |
|---|---|---|
| 1 | `warningMessage` is never cleared on a hotkey-driven run | `src/hooks/useRecording.ts`, the `onStateChanged` listener: the `p.warning` capture gains its `else setWarningMessage(null)`. `handleRecordToggle`'s two existing `setWarningMessage(null)` sites are **untouched** — they serve the button path, which the listener never sees. The contract is now "`warningMessage` is the warning carried by the **latest** state event", which is what D1's renderer (`App.tsx`, `warningMessage && state === "done"`) reads. **Consequence worth naming:** the STT fallback-ladder warning (`process_audio`'s `emit(PipelineEvent::warn("⚠ Groq am Limit → lokale Transkription"))`) arrives as its own transient `warning` event *mid-run* and is now cleared by the very next event of the **same** run, so the main window no longer carries it into the `done` state. That is the intended reading of the existing "Warning is transient" comment and matches Q1's design — the cleanup degrade cause rides the *terminal* event precisely so it cannot be lost — but it is a behaviour change relative to round 1, not a pure bug fix, and it is stated here rather than left to be discovered. The pill (`native_pill::warning_hold_active`, 4 s) remains the ladder warning's surface and is untouched. |

**⚠ RESOLVED — the round-1 flag below is this round's finding.** Kept verbatim for the trail:

**⚠ FLAGGED, NOT ACTED ON (in round 1 — APPLIED in round 2, `df836a0`, CASE C of the D1 smoke exercises exactly this sequence) — D1 makes a deferred defect user-visible.** Deferral row 1 (`warningMessage` is never cleared on a hotkey-driven run, `useRecording.ts`'s `onStateChanged` vs `handleRecordToggle`) was deferred as pre-existing *on the premise that nothing rendered the value*. D1 removes that premise. Concretely: a degraded run sets `warningMessage`; if the user does not click the record button (the only clearing path), the **next successful** hotkey run emits `done` with no warning, `warningMessage` is still set, and the status line shows the **previous** run's amber degrade text instead of "Done". The one-line fix would be an `else` on the capture in `useRecording.ts` — **not applied**, because deferred findings were explicitly out of this round's scope. It should be re-decided now that it is visible. The D1 smoke emits one run per browser boot, so it neither triggers nor rules this out.

**GATE-4 ROUND 1 → TASK 6 / AC8 (2026-09-14, same host: Linux, no device, no Windows build).** Scope is
AC8 only; Tasks 1–5 were left alone except where AC8 replaces them (the pill's message rendering, Q2/Q3's
"accept tail truncation", and the carrier the cause travels in).

| Gate | Command | Result |
|---|---|---|
| Rust unit suite | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | `test result: ok. 701 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` (691 → **701**, +10 new specs) |
| Rust build | `cargo build --manifest-path src-tauri/Cargo.toml --lib` | `Finished \`dev\` profile`; **13 warnings, all pre-existing** (`llm/mod.rs` private-in-public ×8, `license/ls_client.rs` ×2, `commands/feedback.rs`, `pipeline::resolve_stt_provider`) — the new module adds none |
| Parse check, Windows-gated files | `rustc --edition 2021 --crate-type lib --emit=metadata src/{native_pill,native_preview,overlay_message}.rs` | exit **0** for all three. **This proves parsing only** — the inner `#![cfg(target_os = "windows")]` strips both overlay files to an empty crate after the parser has run, so nothing about types, borrows or Win32 API shapes is checked. `overlay_message.rs` is the exception: it has no cfg gate, so it really did type-check standalone. |
| Frontend build | `npm run build` (`tsc && vite build`) | `✓ built in 1.50s`, tsc clean |
| D1 proxy smoke (regression) | `bash _bmad-output/implementation-artifacts/gate4-evidence/7-10/run-d1-smoke.sh` | **13/13 checks passed** — unchanged from round 2. AC8 explicitly keeps the main-window wording, and the smoke confirms `status_line()` still emits `Model 'deepseek-typo' not found — in clipboard` in `rgb(233, 162, 76)`. Artifact copied to `report-task6.txt`; `report.txt` is this run (identical apart from the timestamp). |
| Kotlin JVM gate | — | **NOT RE-RUN.** Task 6 is desktop-only: `git diff --stat -- android/ test-fixtures/ src/` returns **0 lines**. This is a scope statement, not a green. |
| Lint | `cargo clippy` | **blocked** — not installed for the toolchain; not installed around (project-context: never mutate the host for a gate). Unchanged from round 1. |
| Scope check | `git status` after both inversions | clean — no inversion residue |

**Inversions (AC6 discipline, applied to AC8's own gate).** `git status` clean afterwards.

| # | Reverted change (symbol) | Test(s) that went RED | Verbatim failure | Discriminating? |
|---|---|---|---|---|
| 1 | **The card's message-mode gate off** (AC8's named inversion): `hotkey::PipelineEvent::done_with_clipboard_only` — `message: cause.as_ref().map(\|c\| c.card())` → `message: None` | `hotkey::tests::test_pipeline_event_done_clipboard_only_carries_warning` | `700 passed; 1 failed`; `panicked at src/hotkey/mod.rs:295:34: AC8: the degrade event carries a card` | **Yes.** Exactly one test failed, and the surviving `warning`/`clipboardOnly` assertions in the *same* test stayed green — so the failure isolates the card, not the event. `spec_focus_failure_clipboard_only_carries_no_card` pins the other side, so a blanket `Some(...)` would fail there instead. |
| 2 | **The chip split off**: `overlay_message::DegradeCause::card` — the `ModelNotFound` cause built as one plain sentence instead of `before`/`chip`/`after` | `overlay_message::tests::spec_model_not_found_card_has_chip_clipboard_line_and_hint`, `hotkey::tests::test_pipeline_event_done_clipboard_only_carries_warning`, `pipeline::tests::spec_successful_clipboard_write_keeps_the_original_cause` | `698 passed; 3 failed`; three × `left: None / right: Some("deepseek-typo")` | **Yes.** The reverted form still *reads* correctly (`Model 'deepseek-typo' not found`) and `CauseLine::text()` still reassembles a sentence — only the structural claim fails. That is the 200×36 defect in miniature: a message that looks right and cannot be laid out. |

**What AC8 changed, written from `git diff`, anchored at `file::symbol`:**

| Concern | What changed |
|---|---|
| The carrier | `pipeline::ProcessOutcome::Produced.degrade_msg` (String) → `degrade_cause` (`overlay_message::DegradeCause`). The three degrade sites call `pipeline::degrade_cause_for_model`; `pipeline::terminal_degrade_cause` replaces `terminal_degrade_msg`. **No re-derivation:** the card is built from the failure the pipeline already classified, never by parsing the flat string apart (Dev Notes' explicit trap). `degrade_warn_msg` / `degrade_warn_msg_for_model` had no production caller left and moved into `pipeline::tests` as one-line projections, so the dozen wording specs still read as before. |
| The wire | `hotkey::PipelineEvent.message`, `#[serde(skip_serializing)]`. The frontend payload is **byte-identical** to before AC8 — no new field, no new event name, no new `.emit()` (`git diff -U0 \| grep -E 'klarvo://\|\.emit\('` on added lines → one comment line, no code). Trap #5 re-executed on that basis. |
| The pill | `native_pill::fit_text` **deleted**, and with it `PillWindowState::{status_msg, pending_msg}`, `WM_PILL_SET_MSG` and `NativePill::set_status_msg` — the whole staging apparatus review round 1 (F1) built existed only to keep dynamic text honest, and there is no dynamic text left. New `native_pill::TerminalKind` travels in LPARAM; new `NativePillState::DoneDegraded` gives the degrade route its own static label while sharing `DONE_CLIPBOARD_MS` and the clipboard icon with `DoneClipboard`. The five label arms fold into `native_pill::draw_static_label`. |
| The card | `native_preview::render_message_card` — header (mono, `--k-dim`, with a tone-coloured dot) · cause (model ID on an `--k-amber-bg` chip in mono `--k-amber-hi`, falling back to plain wrapped text when the line does not fit) · next (`--k-muted`) · hint (`--k-dim`), over the card's own background with a 1 px `--k-amber-line` / `--k-danger` border. `native_preview::present` was factored out of `render_frame` so the fade can drive `SourceConstantAlpha`. |
| Lifetime | `MSG_HOLD_MS` 4000 + `MSG_FADE_MS` 1000 on a 33 ms `TIMER_MESSAGE`; `native_preview::dismiss_message` on fade-end, on `WM_LBUTTONDOWN`, and on a Recording state. `WS_EX_TRANSPARENT` is cleared only while a card is up (`native_preview::set_click_through`) — otherwise the card could not receive the click AC8 asks for. |
| Availability | The card must show "regardless of `live_preview_enabled`", so `pipeline::start_recording_only` now creates the preview window unconditionally (the setting still gates *arming*, i.e. the live text), and `lib::run` creates one at setup — the boot-time config warnings AC8 puts on the card are emitted before any recording exists. |
| Border width | Fixed at 1 px instead of the user's `previewBorderWidth`: AC8 pins an amber line, and a configured `0` would erase it. ~~The card's background and radius still follow the user's appearance settings — it is the same card.~~ **Corrected in the AC8 fix round (P5, P2):** the **radius** never followed the user's settings — `MSG_RADIUS` is a fixed 16 px (canon `--k-r-lg`) while the live-preview path uses `config.border_radius`. The **background** followed them until this round and now does not either (`native_preview::MSG_BG` = the render's `rgba(14,16,18,.82)`): every text run on the card is a fixed canon dark-theme colour, so the shipped "Light" preview theme rendered the failure message near-white on near-white. Background, radius and border width are all pinned to the approved render; only the card's *position and width* still follow the user's preview settings. |

**AC8 REVIEW — FIX ROUND (2026-09-14, same host: Linux, no device, no Windows build in this session).**
Scope: the **11 confirmed patch findings P1–P11** plus Andi's three directives D1–D3, which decide the three
decision-needed findings. Nothing from the deferred (10) or dismissed (11) lists was touched.

| Gate | Command | Result |
|---|---|---|
| Rust unit suite | `cargo test --manifest-path src-tauri/Cargo.toml --lib` | `test result: ok. 705 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` (701 → **705**, +4 new specs) |
| Rust build | `cargo build --manifest-path src-tauri/Cargo.toml --lib` | `Finished`; **13 warnings, all pre-existing** (the same `llm/mod.rs` private-in-public ×8, `license/ls_client.rs` ×2, `commands/feedback.rs`, `pipeline::resolve_stt_provider` set) — this round adds none |
| Parse check, Windows-gated files | `rustc --edition 2021 --crate-type lib --emit=metadata src/{native_pill,native_preview,overlay_message}.rs` | exit **0** for all three. **Parsing only** — the inner `#![cfg(target_os = "windows")]` strips both overlay files to an empty crate after the parser has run. `overlay_message.rs` has no cfg gate and really did type-check. |
| Frontend build | `npm run build` (`tsc && vite build`) | `✓ built in 1.52s`, tsc clean |
| D1 proxy smoke (regression) | `bash _bmad-output/implementation-artifacts/gate4-evidence/7-10/run-d1-smoke.sh` | **13/13 checks passed** — unchanged. `DegradeCause::status_line()` was not touched by this round, and the smoke re-proves the main window still reads `Model 'deepseek-typo' not found — in clipboard` in `rgb(233, 162, 76)`. |
| Trap #5 | `git diff -U0 -- src-tauri/src src/ \| grep '^+' \| grep -E 'klarvo://\|klarvo\.\|\.emit\('` | **no hits** — no new event name, no new payload field, no new emit. The one new producer (`hotkey::PipelineEvent::warn_all`) rides the existing `klarvo://state-changed`. |
| **Windows release build** | `scripts/windows-build.sh` on `d778d5c` | **exit 0 — `FERTIG. klarvo.exe enthält d778d5c`**, exe written 12:37:08 against a commit of 12:34:17 (the script's own freshness check, exit 3 would have caught a stale exe). Installer bundled + `rsign` signature verified. `klarvo` (lib) generated **74 warnings — the exact same count as build r2 (`d03a4da`)**, and `grep -c '^warning: unused \`BOOL\`'` is **56 in both**, so this round added none. Artifacts: `gate4-evidence/7-10/windows-build-r3.{log,exit}`. **This is the ONLY gate that compiles `native_preview.rs` and `native_pill.rs` at all** — it proves types, borrows, the Win32/GDI signatures and the tiny-skia calls. It proves **nothing** about layout, the fade, hit-testing, the below-pill flip or a single pixel. |
| Kotlin JVM gate | — | **NOT RUN.** Desktop-only round: `git diff --stat -- android/ test-fixtures/` returns **0 lines**. A scope statement, not a green. |
| Lint | `cargo clippy` | **blocked** — still not installed for the toolchain; not installed around (project-context). Unchanged from every previous round. |
| Scope check | `git status` after all four inversions | clean — no inversion residue |

**Inversions (AC6 discipline) — every changed behaviour that a Linux host can observe, shown RED first.**
`git status` clean afterwards. **P1, P2, P3, P4, P7, P9, P10 and D1 have no inversion and can have none**: they
live in `native_preview.rs` / `native_pill.rs`, which are `#[cfg(target_os = "windows")]` with no test module and
are not compiled on this host at all.

| # | Finding | Reverted change (symbol) | Test(s) that went RED | Verbatim failure | Discriminating? |
|---|---|---|---|---|---|
| 1 | D2 | `overlay_message::DegradeCause::card` — the `ClipboardWriteFailed` arm put back to `MessageTone::Warning` / `CLEANUP FAILED` / "The clipboard write failed — the text did not land" | `spec_clipboard_write_failure_card_never_promises_the_clipboard`, `spec_clipboard_write_failure_keeps_the_degraded_pill_label` | `8 passed; 2 failed`; `assertion left == right failed: D2: danger line, not amber`, `left: Warning / right: Error` | **Yes.** Two independent assertions fail on the *tone*, while the `next: None` / no-`Ctrl+V` assertions in the same test stay green — so the failure isolates D2's new decision from F6's older one, which the revert leaves intact. |
| 2 | D3 | `overlay_message::OverlayMessage::error` — `user_facing_error_text(&text)` → `text` (translation off) | `spec_known_error_tokens_are_translated_unknown_ones_are_verbatim` | `9 passed; 1 failed`; `left: "feature_requires_license:CommandMode" / right: "Command mode needs a license"` | **Yes.** The failure names the raw token verbatim — only a missing translation can produce it. The same test's two *unknown*-token assertions stay green, so a blanket rewrite would fail there instead. |
| 3 | P11 | `overlay_message::DegradeCause::card` — `before`/`after` back to `"Model "` / `" not found"` (delimiters dropped) | `spec_model_not_found_card_has_chip_clipboard_line_and_hint` | `704 passed; 1 failed`; `left: "Model deepseek-typo not found" / right: "Model 'deepseek-typo' not found"` | **Yes.** The chip assertion in the same test stays green, so the failure is the delimiters and not the split — the two halves of P11's claim fail independently. |
| 4 | P8 | `overlay_message::OverlayMessage::warnings` — `texts.join("\n")` → `texts[0]` (only the first warning survives, i.e. the old per-event behaviour's outcome) | `overlay_message::tests::spec_boot_warnings_merge_into_one_card`, `hotkey::tests::spec_boot_config_warnings_travel_as_one_event` | `703 passed; 2 failed`; twice `left: "config.json was corrupt" / right: "config.json was corrupt\ndictionary.json was corrupt"` | **Yes.** The failure is the *second* warning's absence, which is exactly the defect (only the last/first card readable). The single-warning and empty-list assertions in the same test stay green, so a broken helper would fail differently. |

**Resolution rows — written from `git diff`, anchored at `file::symbol`:**

| # | Finding | What changed |
|---|---|---|
| P1 | A message-less event dismisses a live card | `native_preview::preview_wnd_proc`, `WM_PREVIEW_SET_MESSAGE`'s `None` arm: it now returns early while `message_at.elapsed() < MSG_HOLD_MS`. Andi's directive verbatim — only a new recording (`WM_PREVIEW_SET_STATE` `STATE_RECORDING`), a click (`WM_LBUTTONDOWN`) and the fade timer dismiss a card. `lib::emit_pipeline_state` still posts `set_message` for every event (it is what clears the card *after* the hold and what the Recording path relies on); its comment now names the hold. **Not gated on the fade:** during the 1 s fade a message-less event may still take the card down, which is the directive's own boundary. |
| P2 | The card fills with the user's configurable preview background | New `native_preview::MSG_BG` = `(14,16,18)` + `MSG_BG_ALPHA` = `0.82` — the approved render's `rgba(14,16,18,.82)`, pinned exactly like the border. `render_message_card` no longer reads `s.config.bg_*`. The "Light" preview theme (`previewAppearance.ts::PREVIEW_THEMES`) can no longer put near-white text on a near-white card. Record row corrected in the same edit (P5). |
| P3 | The card's typeface is `Segoe UI`, not the bundled `Geist` | `native_preview::MSG_SANS_FACE` → `"Geist"`. **Boot order is the real work here:** Geist is *bundled, not installed* — it only exists in GDI after `AddFontMemResourceEx`, which until now ran in the pill thread only, and since AC8 the preview window can be created at `lib::run` setup, before the pill exists. `native_pill::load_embedded_font`'s three calls are now `native_pill::load_embedded_geist()`, a `pub(crate)` `Once`-guarded registrar; `preview_thread` calls it before creating its message fonts. Whichever overlay starts first registers; the other is a no-op. |
| P4 | False provenance comment ("Cascadia Code ships with the app's font set") | `native_preview::MSG_MONO_FACE`'s doc says the truth: nothing mono is bundled — the app embeds exactly three Geist sans faces — and Consolas is named because it is a stock Windows face, which is what the canon's `ui-monospace` cascade resolves to there anyway. The constant's value is unchanged; only the claim was wrong. |
| P5 | Record claim false: the card's radius does not follow the user's appearance settings | Corrected in the "What AC8 changed" table, *Border width* row, with the old sentence struck through rather than deleted (Epic-7 retro D2 — the trail stays readable). The row now states all three pinned values: background (as of P2), radius (`MSG_RADIUS`, always was fixed) and border width. |
| P6 | `degrade_cause_for_model`'s docstring: wrong return description + dead intra-doc link | `pipeline::degrade_cause_for_model` — "Builds the user-facing warning" → "Classifies a degraded cleanup"; the dead `[degrade_warn_msg]` link (moved into `mod tests` at AC8) replaced by `[generic_degrade_cause]`, which is where a non-model error actually goes; the "one-line projection" paragraph now says the function returns the `DegradeCause` and names `tests::degrade_warn_msg_for_model` as the projection. `grep '\[degrade_warn_msg\]'` over `src-tauri/src/` → 0 hits outside `mod tests`. |
| P7 | The chip rectangle ignores `text_scale` while its glyphs honour it | `native_preview::render_message_card`, the chip geometry block: the rect's height is now `cause_h * MSG_CHIP_H_FRACTION` (the new constant is the render's `1.25 / MSG_LINE_MULT`) and its padding `MSG_CHIP_PAD_X * sc * text_scale`. `cause_h` already carries DPI **and** the accessibility text scale, so the box tracks the glyphs at 225% instead of staying put. The *measuring* pass (the fit test that decides chip-vs-plain) uses the same padding term — otherwise the fit test and the drawn rect disagree. |
| P8 | Several boot-time config warnings: each card replaces the previous | New `overlay_message::OverlayMessage::warnings(Vec<String>) -> Option<Self>` (one line per warning, `\n`-joined — `native_preview::wrap_text_lines` already breaks on `\n`) and `hotkey::PipelineEvent::warn_all(Vec<String>) -> Option<Self>` built on it. `lib::run`'s `for warning in config_warnings { emit(warn(warning)) }` becomes one `if let Some(event) = PipelineEvent::warn_all(config_warnings)`. The empty case emits nothing, as before. Bound to the real boot path, not to a parallel helper (project-context). |
| P9 | `SetTimer`'s return is ignored and `msg_timer_active` set anyway | `native_preview::preview_wnd_proc`, `WM_PREVIEW_SET_MESSAGE`'s `Some` arm: the timer is armed **first**, `msg_timer_active` is set only on a non-zero return, a failure is logged with `GetLastError()`, and `set_click_through(hwnd, false)` now runs **only** when the timer really started. Directive: on failure the click-through bit stays — a card that never fades and is not click-through would swallow every click meant for the app underneath, forever. |
| P10 | `Pixmap::new` failure returns the fade alpha | `native_preview::render_message_card` returns **0** on that path, which is the caller's dismiss signal (`render_frame` hides the window). Returning `alpha` made `present()` ship the DIB it still held — the previous frame, or the live preview's last text — as if it were the message. Log text updated from "skipping message frame" to "dismissing the message card". |
| P11 | The chip fallback drops the model ID's delimiters | `overlay_message::DegradeCause::card`, `ModelNotFound` arm: `before`/`after` become `"Model '"` / `"' not found"`, so `CauseLine::text()` reassembles AC8's and 7-9 D2's wording `Model '<id>' not found` while the **chip still carries the bare ID** the renderer measures and draws in mono. The test that pinned the quote-less form (`spec_model_not_found_card_has_chip_clipboard_line_and_hint`) is corrected with a comment saying where the quotes live and why. |
| D1 | The card has no surface when the pill sits near the top (decision) | Andi's directive: *"Wenn oben weniger Platz als die Kartenhöhe ist, erscheint die Karte 8 px UNTER der Pill; gleiches gilt für die Live-Preview in dieser Lage."* `native_preview::compute_preview_geometry` gains `work_bottom` and returns a 5th value `below_pill`: when the room above the pill is less than the card wants (`BASE_MAX_HEIGHT × k`), the window is placed `GAP_LOGICAL` (8 px) **below** the pill, sized to the room there. New `PILL_HEIGHT_LOGICAL` (36, mirroring `native_pill::PILL_H`). `PreviewWindowState.below_pill` flips **both** renderers from bottom-aligned to top-aligned (`render_message_card` and `render_frame` — the live preview follows, as the directive says). Recomputed on every `WM_PREVIEW_SET_PILL_POS`, i.e. on every pill drag. **One arithmetic guard, named because it is mine and not Andi's:** the flip is skipped when it would yield *less* height than staying above (`avail_below > avail_above`), since moving the card into an even smaller box would defeat the directive it implements. |
| D2 | `ClipboardWriteFailed` rendered amber, not danger (decision) | Andi's directive, applied literally: `overlay_message::DegradeCause::card`'s `ClipboardWriteFailed` arm is now `MessageTone::Error` (danger line + danger dot), header **`TEXT LOST`**, cause **`Clipboard write failed — raw text is in History`**, no `next` line (no Ctrl+V hint — the clipboard holds whatever was there *before* this run). **The pill label is deliberately unchanged at `Cleanup failed`**: it is chosen by the run's ending in `lib::emit_pipeline_state` (`TerminalKind::Degraded`), not by the card's tone, so no code change was needed — and `spec_clipboard_write_failure_keeps_the_degraded_pill_label` now pins that divergence so it cannot drift silently. `status_line()` is untouched, so the main window and every wording spec are unaffected. |
| D3 | Machine tokens get a prominent, untruncated card (decision) | Andi's directive: new `overlay_message::ERROR_TOKEN_TEXT`, a two-row table, and `user_facing_error_text`, applied inside `OverlayMessage::error`. `feature_requires_license:CommandMode` → `Command mode needs a license`; `feature_requires_license:OfflineMode` → `Offline transcription needs a license`. **Unknown tokens stay verbatim** — the raw string is what matches `Klarvo.log` and what a support answer needs; guessing is worse. `PipelineEvent::error` keeps the raw token in its own `error` field, so **the frontend payload and every consumer of `event.error` are unchanged** — only the card is translated. The two rows are the `feature_requires_license:*` tokens that exist in today's tree (`grep -rn feature_requires_license src-tauri/src/` → `lib.rs`'s `require_license!`, `pipeline.rs` command-mode gate, `commands/whisper.rs` ×2). |

**Coverage statement — AC8 fix round. What 705 does NOT cover.** *(project-context: "a number states what it covers".)*

- **705 Rust green proves the message MODEL and the boot-path producer — nothing about the card.** The 4 new specs
  decide: D2's tone/header/cause and that the pill label deliberately diverges from it, D3's translation table with
  its verbatim fallback, and that several boot warnings become one event carrying one card. Claim proved: **logic
  and structure**. Not design, not layout, not a pixel.
- **8 of the 14 items in this round have ZERO test coverage.** P1, P2, P3, P4, P7, P9, P10 and D1 are all in
  `native_preview.rs` / `native_pill.rs`, which are `#[cfg(target_os = "windows")]` with no test module:
  `cargo test --lib` does not compile them, so the 705 says nothing about them. The **Windows release build on
  `d778d5c` (exit 0)** closes exactly one half of that hole — it proves the eight changes *compile*, with types,
  borrows, Win32/GDI signatures and tiny-skia calls checked, and with the same 74-warning profile as the previous
  build. It proves nothing about behaviour: no test ran on Windows, and nobody looked at the screen. That they
  *work* and *look* right is Andi's GATE-4, step 6.
- **D1 has never been seen.** Not exercised: that the window actually lands below a top-positioned pill, that the
  top-aligned card hugs it, the live preview in that position, the flip while a card is already up (the geometry
  is recomputed on `WM_PREVIEW_SET_PILL_POS` and the card re-renders, read from the source, not run), the
  band where `avail_above` and `avail_below` are nearly equal, and multi-monitor work areas (`SPI_GETWORKAREA`
  returns the **primary** monitor's rect — pre-existing, unchanged by this round, named because D1 is the first
  change to depend on `work_bottom`).
- **P3's font change is unverified in both directions.** Not exercised: that `"Geist"` resolves in GDI at all in
  the preview thread, that `load_embedded_geist`'s `Once` really wins the race when the preview thread starts
  first, and what the card falls back to if `AddFontMemResourceEx` fails (GDI substitutes; the log line is the
  only signal). The `Once` is read, not run.
- **P9 and P10 are failure paths with no reachable trigger.** Nothing in this session made `SetTimer` return 0 or
  `Pixmap::new` fail; both branches are defensive code verified by reading.
- **The D1 proxy smoke's 13/13 is a React-tree regression number, and only for the status line.** It re-proves
  that this round did not change the main window's wording or colour — which is exactly what `status_line()`
  being untouched predicts. Not exercised: the Rust backend (payloads are hand-written), the pill, the card,
  Android, pixels.
- **Android was not touched and not re-run.** `git diff --stat -- android/ test-fixtures/` → 0 lines. The Kotlin
  JVM gate's 24 suites / 194 tests remain round 1's execution; this round makes no claim about them beyond "no
  Kotlin file changed". S6 held — no fixture edit, so the gradle `--rerun-tasks` trap does not apply.
- **D2 changes a user-facing wording that no smoke renders.** `TEXT LOST` and `Clipboard write failed — raw text
  is in History` are compared as strings in a JVM-free Rust test. Whether the danger-red card reads as intended
  next to an amber pill saying `Cleanup failed` is Andi's screen judgement. It is also the hardest of the four
  message shapes to reach in a smoke — it needs cleanup to fail **and** the clipboard write to fail.
- **The 10 deferred and 11 dismissed findings of the AC8 review were not touched** and are not covered by
  anything here.

### Completion Notes List

**Shape chosen (Q1 = ONE event).** The three degrade sites in `process_audio` no longer
`emit(PipelineEvent::warn(...))`. They set `degrade_msg` on `ProcessOutcome::Produced`, which
`deliver_outcome` now returns, and the shell puts it on the single terminal
`PipelineEvent::done_with_clipboard_only`. The degrade path's event sequence is therefore
`Transcribing → Cleaning → DoneClipboard(msg)` — no `Warning` state, so `warning_hold_active` is
never engaged on this path and needed **no change** (it still serves the STT retry-ladder warning at
`process_audio`'s local-Whisper fallback, which is untouched).

**Deviation from Task 1's letter, forced by Q1.** Task 1 says `deliver_outcome` goes 7 → 8 fields.
It went 7 → **9**: Q1 was decided after the task text was written and requires the *message*, not
just the flag, to reach the terminal event. `llm_error` (AC1's named anchor, read by the paste
branch) and `degrade_msg` (AC3's text) are both returned. They are redundant by construction, so the
redundancy is pinned as a checked invariant rather than left to drift —
`spec_degrade_msg_present_exactly_when_llm_error` asserts `degrade_msg.is_some() == llm_error`.
**Corrected in review round 1 (finding 2):** that test covers **2 of the 3** degrade paths
(non-retryable, and retryable-with-no-fallback) plus the success path — **not** all three. The
fallback-also-failed site is unreachable from a test and is held by code review; both the KDoc and the
test's docstring now say so.

**No second clipboard implementation.** `set_clipboard` was hoisted out of the Linux inline module to
`paste::set_clipboard` (cfg-gated: `arboard` on desktop, no-op on Android — `arboard` is
desktop-only in `Cargo.toml`). Windows' inline `arboard` block and the fallback handler now call it
too, so there is **one** clipboard writer and one error mapping. `PasteHandler::copy_only` is a trait
method with a default implementation built on it, so Windows/Linux/Fallback/Android stay consistent.

**The Insert+Send gate is textually unchanged** (`insert_and_send && paste_result == PasteResult::Pasted`).
`copy_only` returns `ClipboardOnly`, so Enter is skipped without a second switch — AC1 rests on that.

**Seam (S1).** `pipeline::deliver_text(&dyn PasteHandler, text, llm_error, insert_and_send) -> Delivery`
holds the paste/clipboard-only choice and the Insert+Send gate; the shell keeps only the effects that
need `AppState`/HWNDs (`paste_error_count`, Return-to-Current). Behaviour-preserving, with one
ordering note: `active_insert_and_send` is now read just **before** the paste instead of just after.
Safe for the same reason the original comment gives — the hotkey handler cannot fire again mid-pipeline.

**Android (Task 3).** Explicit `var llmCleanupFailed` set only on the two true-failure branches
(fallback also failed; no fallback available). Per **Q4** it is NOT set on: successful fallback
(`"⚠ Cleanup-Anbieter gewechselt"`), no-LLM-key (keeps today's paste), or the silent local-MNN catch
(out of scope → backlog). Step 4 calls the pure `KlarvoOverlayService.decideDelivery(...)` (S3 seam,
companion object, repo pattern). `BankingGuard.shouldBlockPaste` **verified still the first statement**
in the `handler.post` block. `performEnter` **verified still present and still callerless** — AC2's
auto-send half is satisfied by construction (7-9 M13), nothing re-wired, nothing deleted.

**✅ RESOLVED at GATE 2 — "(Ctrl+V)" dropped from the Android toast.** The flag raised above was
answered by Andi (2026-09-14): the phone toast reads `Cleanup failed — raw text in clipboard`, with
no key hint. Implemented as predicted — **one constant plus its wording test**:
`KlarvoOverlayService.CLEANUP_FAILED_CLIPBOARD_MSG` lost the `(Ctrl+V)` suffix, and
`CleanupFailureDeliveryTest.degradeMessage_mirrorsDesktopPillWording` was retargeted, plus a new
`degradeMessage_hasNoKeyHintOnAndroid` negative assertion that pins the hint out.

**What GATE 2 did NOT change.** The desktop side is **untouched** — `git diff` for this round lists
exactly two files, both under `android/`. `pipeline::degrade_warn_msg` still emits
`Cleanup failed — raw text in clipboard (Ctrl+V){friendly_error}` on the pill, where Ctrl+V is
literally true. **Model-not-found stays identical to the pill:**
`pipeline::degrade_warn_msg_for_model` still returns `Model '<id>' not found — in clipboard`, which
carries no key hint on either platform and was not edited. Precision note: Android never *produces*
the model-not-found form — both Step-2 failure sites set the generic constant — so "identical" here
means the pill's form is unchanged, not that the phone renders it.

The two platforms' generic wording now diverges by exactly the four-character hint. The twin-wording
test documents that divergence deliberately; it remains a **written record, not a lock** (no shared
fixture — a desktop-only edit still cannot fail the Kotlin test).

**Inversion evidence (AC6) — both platforms, at writing time.** `git status` clean afterwards (only
this story's own files modified; no inversion residue).

| # | Platform | Reverted change (symbol) | Test that went RED | Verbatim failure | Discriminating? |
|---|---|---|---|---|---|
| 1 | Rust | `pipeline::deliver_text` — `if llm_error` → `if false && llm_error` (re-enables the Ctrl+V paste on the degrade path) | `pipeline::tests::spec_degraded_cleanup_is_clipboard_only_and_never_sends` | `assertion left == right failed; left: Pasted, right: ClipboardOnly` | **Yes.** The spy's `paste()` returns `Pasted` — a *valid* target. The failure names `Pasted`, which none of the four innocent `ClipboardOnly` routes (no HWND, dead window, focus-verify fail, coerced `PasteError`) can produce. This is exactly the non-discriminating trap AC6 warns about, and it is avoided. |
| 2 | Kotlin | `KlarvoOverlayService.decideDelivery` — `llmCleanupFailed -> DeliveryDecision(paste = false, …)` → `paste = true` | `CleanupFailureDeliveryTest > cleanupFailure_withAccessibilityConnected_doesNotPaste` | `java.lang.AssertionError at CleanupFailureDeliveryTest.kt:54`; `194 tests completed, 1 failed`; `BUILD FAILED` | **Yes.** Asserted with `accessibilityConnected = true` — a target that really could have been pasted into. Asserting against a disconnected service would pass against code that never had the option. |

**Coverage statement — what these numbers do NOT cover.** *(project-context: "a number states what it covers".)*

- **691 Rust / 194 Kotlin green prove wiring, logic and structure — not design, not pixels.**
  *(Round-1 figures were 688 / 195; the fix round added 3 Rust specs and removed 1 duplicate Kotlin test.)*
- **The two `native_pill.rs` fixes (findings 1 and 3) are covered by NOTHING.** The file is
  `#[cfg(target_os = "windows")]` with no test module, so `cargo test --lib` on Linux does not
  compile it, let alone run it. Not exercised: that the staging change compiles, the
  Warning→DoneClipboard sequence it fixes, and every other pill behaviour. Both rest on Andi's
  GATE-4 (S5).
- **The D1 smoke proves the React tree, not the product.** It drives a *synthetic* payload through
  the real components in real Chromium. Not exercised: the Rust backend (no Tauri — no
  `emit_pipeline_state`, no real `klarvo://state-changed`, no pipeline run), the native pill,
  Android, pixels/fonts/truncation. That the backend really puts `degrade_msg` on the terminal event
  is the Rust suite's claim, not the smoke's.
  *(Round 2 corrects one item of this list: the stale-`warningMessage` sequence WAS "not exercised —
  one run per browser boot". It now is — CASE C runs a degraded run and a clean run in the same boot.
  Still not exercised by it: two runs driven by the **real** hotkey pipeline, and the record-button
  path, which has its own clearing sites and was not re-measured.)*
- **Round 2's 13/13 is a React-tree number, and only for the status line.** Not exercised: that the
  real pipeline emits a warning-less terminal event on a clean run after a degraded one (the payloads
  are hand-written), the pill's own staleness behaviour on the same sequence (`native_pill.rs` is
  Windows-gated and still uncompiled here), Android's toast on consecutive runs, and every other
  consumer of `warningMessage` — `App.tsx`'s status line is the only reader.
- **Round 2's 691 Rust / 194 Kotlin are regression statements about untouched code, and the Kotlin one
  was replayed, not re-run.** The round changed exactly one product file, `src/hooks/useRecording.ts`;
  gradle reported the test task `UP-TO-DATE`. Neither number was earned again this round.
- **`terminal_degrade_msg` is tested as a pure function only.** Its 3 specs decide the *message*;
  no test drives a real `arboard` clipboard failure through `stop_and_process_pipeline`, which
  remains untested (unchanged by this round).
- **The GATE-2 wording change was never seen rendered.** `degradeMessage_*` compares two strings in a
  JVM. Not exercised: the toast actually drawn on a phone, its truncation at `LENGTH_LONG`, and its
  ordering against HyperOS's own "pasted from your clipboard" system toast. That the shorter string
  *reads* better on a real toast is Andi's device judgement, not a test result.
- **Never executed on Windows.** `native_pill.rs` is `#[cfg(target_os = "windows")]` and still has
  **no test module**; `warning_hold_active`, `from_code`, `handle_timer` and the `DoneClipboard`
  render arm I changed were **not run once** in this session. AC3's display outcome is **S5 — not
  Linux-verifiable**, exactly the hole 7-9's GATE-4 fell into. Do not read "688 green" as evidence
  the pill shows anything.
- **Never executed on a device or emulator.** The Kotlin gate is JVM-only. Not exercised: the real
  `copyToClipboard` write, the real `pasteIntoFocusedField()`, toast rendering, toast ordering
  against HyperOS's own system toast, the banking guard's runtime behaviour, and **whether Step 2
  actually sets `llmCleanupFailed` on the right branches** (the test covers the decision function,
  not its caller). No JNI path was reached.
- **The Rust spy overrides `copy_only`,** so the real `arboard` clipboard write, `SendInput`,
  `xdotool` and all window/focus behaviour are untested. The tests decide *routing*, not delivery.
- **`deliver_text` is tested; `stop_and_process_pipeline` is still not.** History, Turso, webhook and
  metrics ordering around the new call remain uncovered (unchanged by this story).
- **The twin-wording test is a written record, not a lock.** No shared fixture — a desktop-only
  wording edit cannot fail the Kotlin test. `test-fixtures/` untouched (S6 held), so the gradle
  `--rerun-tasks` fixture trap did not apply.
- **AC4 verified by diff, not by eye:** no file under `src-tauri/src/config.rs` or `src/components/`
  is touched. **AC5 verified by diff:** no `add_entry` / `saveToHistory` / `pushToTurso` / webhook
  line changed.

**Coverage statement — AC8 / Task 6. What 701 does NOT cover.** *(project-context: "a number states what it covers".)*

- **701 Rust green proves the message MODEL — structure, wording and the event's carrier. It proves nothing
  about the card.** The 10 new specs decide: which lines a `DegradeCause` produces, that the model ID is a
  separate chip, that a clipboard-write failure carries no `Ctrl+V` line, that `status_line()` is unchanged, that
  `warn`/`error` carry their own card, that progress states carry none, and that the pill labels are short fixed
  literals. Claim proved: **logic and structure**. Not design, not layout, not a pixel.
- **`native_pill.rs` and `native_preview.rs` were NOT COMPILED on this host.** Both are
  `#[cfg(target_os = "windows")]` with no test module. `cargo test --lib` does not read them. The only thing run
  against them was a **parse** check (`rustc --emit=metadata`, exit 0) — after the parser, the inner `#![cfg]`
  strips both files to an empty crate, so **types, borrows, GDI/Win32 signatures, the tiny-skia calls, the timer,
  the `WS_EX_TRANSPARENT` toggle and the whole layout are unverified**. Whether this even compiles on Windows is
  the conductor's build, not a claim made here. `cargo check --target x86_64-pc-windows-gnu` is a **retired**
  gate (project-context) and was not attempted; no tool was installed.
- **The card has never been rendered.** Not exercised: the header/cause/next/hint layout, the chip rect vs its
  glyphs, wrapping at any width, the 4 s hold, the 1 s fade, click-to-dismiss, the card with live preview running,
  the card at any DPI or text-scale, and the boot-time warning path. Every one of those is Andi's GATE-4.
- **The pill's new label has never been drawn.** `PILL_LABEL_DEGRADED` = "Cleanup failed" is asserted to be ≤14
  characters by `spec_pill_labels_are_static_literals`; that is a **constant** check, not a fit check. That it
  fits the ~131 px label box at `font_label_lg` is arithmetic, not a measurement — the same class of claim that
  failed at GATE-4 round 1. Andi's build decides it.
- **The D1 13/13 is a React-tree regression number, and only for the status line.** It re-proves that AC8 did not
  change the main window's wording or colour. Not exercised: the Rust backend (the payloads are hand-written), the
  pill, the card, Android, pixels. Claim proved: **wiring and structure** in a proxy render (browser preview) —
  by construction it cannot prove design.
- **Android was not touched and not re-run.** `git diff --stat -- android/ test-fixtures/ src/` → 0 lines. The
  Kotlin JVM gate's 24 suites / 194 tests are round 1's execution; this round makes no claim about them beyond
  "no Kotlin file changed".
- **The STT-ladder warning's text stays German** (`"⚠ Groq am Limit → lokale Transkription"`, 12-1). AC8's "all
  English" governs the cleanup-failure card's own lines, which are English; re-wording a pre-existing 12-1 literal
  would be scope this task does not have. Named here rather than left to be found on the card.
- **The card's `next`/`hint` lines are wrapped, never truncated** — but nothing measures that the wrapped card
  fits the available height above the pill. `card_h` is clamped to the window's max height; a message long enough
  to exceed it is clipped at the top with no fade. Not reachable with today's four message shapes (the longest is
  four short lines); stated because it is unmeasured, not because it is impossible.

**OPEN — AC7 is not fully met. Andi's GATE-4 is outstanding** (Windows release build via
`scripts/windows-build.sh`, which was not run in this session). Exact steps, from the epic DoD:

1. Set a **wrong DeepSeek model ID** (Settings → Advanced → Model IDs) and switch **Insert+Send ON**.
2. Dictate into a chat-style target window.
3. Expect (**AC8 wording — this replaces round 1's step 3**): **nothing lands in the active window**; **no Enter
   is sent**; the **pill** shows the static amber label **`Cleanup failed`** (no truncated sentence) for ~4 s; the
   **card above the pill** shows `CLEANUP FAILED` · `Model '<id>' not found` (ID on an amber chip) ·
   `Raw text is in the clipboard · Ctrl+V to paste` · `Check Advanced → Model IDs`, amber-bordered, holding ~4 s
   and fading over ~1 s; **Ctrl+V pastes the raw text**.
4. Regression in the same build: with a **correct** model ID, paste and Insert+Send behave as before, and **no**
   card appears.
5. **New at AC8, worth one extra look:** (a) with **live preview OFF** the card must still appear — that path
   never existed before; (b) a **click on the card** dismisses it at once and the next click goes through to the
   app underneath (the overlay drops `WS_EX_TRANSPARENT` only while a card is up); (c) a **boot-time config
   warning** (e.g. rename `config.json` to something unparseable once) should land on the card at startup.
6. **New at the AC8 fix round** (the eight items below have no machine coverage at all — see the coverage
   statement): (a) **D1** — drag the pill to the **top** of the screen and trigger a failure: the card must appear
   **8 px below** the pill and hug it from there, and the **live preview** must do the same in that position;
   (b) **P1** — the card must stay readable for its full ~4 s, i.e. the `Done`/`Idle` state arriving right behind
   it must not blank it; (c) **P2** — switch the preview appearance to the **Light** theme and re-trigger: the
   card must stay dark, and readable; (d) **P3** — the card's text should be **Geist**, matching the pill beside
   it (check after a cold start, where the preview window is created before the pill); (e) **P8** — with two
   corrupt files at boot, **both** warnings must be on one card; (f) **P7** — at Windows text scale 150–225 % the
   model ID must still sit inside its amber chip; (g) **D2** — a clipboard-write failure (hard to force; if it
   cannot be reached, say so) shows a **red** card reading `TEXT LOST` while the pill still says `Cleanup failed`;
   (h) **D3** — trigger command mode without a license: the card should read `Command mode needs a license`, not
   `feature_requires_license:CommandMode`.

**Backlog notes this story deliberately did not act on** (GATE-1 deferrals, already recorded in
`docs/backlog.md` by commit `606e9ac`): the pill's label language (canon says
„In Zwischenablage", code says `In Clipboard` — pre-existing divergence, Q3) and the silent local-MNN
cleanup failure on Android (Q4).

### File List

Paths relative to repo root.

**Modified**
- `src-tauri/src/pipeline.rs` — `ProcessOutcome::Produced.degrade_msg`; three degrade sites set it instead of emitting `Warning`; `degrade_warn_msg` / `degrade_warn_msg_for_model` rewording (Q2); `deliver_outcome` returns 9 fields; new `deliver_text` + `Delivery` seam; shell rewired; terminal event carries `degrade_msg`; tests (`run_full`, `SpyPasteHandler`, 6 new specs, 4 existing updated).
- `src-tauri/src/paste/mod.rs` — module-level cfg-gated `set_clipboard` (hoisted out of `mod linux`, reused by `mod windows` and the fallback handler); new `PasteHandler::copy_only` default method.
- `src-tauri/src/hotkey/mod.rs` — `PipelineEvent::done_with_clipboard_only` takes `warning: Option<String>`; 1 test updated, 1 added.
- `src-tauri/src/native_pill.rs` — `DoneClipboard` render arm renders `status_msg` (via `fit_text`, `font_label`) when present, static `"In Clipboard"` otherwise; `handle_timer`'s done-timeout clears `status_msg` on dismissal.
- `src/hooks/useRecording.ts` — `p.warning` captured before the `p.state === "warning"` early return.
- `src/types.ts` — `StateChangedPayload.warning` / `.clipboardOnly` comments updated.
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — `CLEANUP_FAILED_CLIPBOARD_MSG`, `DeliveryDecision`, pure `decideDelivery` (companion object); explicit `llmCleanupFailed` in `processAudio` Step 2 + `capturedLlmFailed`; Step 4 branch; degrade toast literal (Q2/Q5); stale `activeGesture` auto-send KDoc corrected. **GATE 2:** `CLEANUP_FAILED_CLIPBOARD_MSG` dropped its `(Ctrl+V)` suffix; its KDoc now states the deliberate desktop divergence and that the model-not-found form is unchanged.

**Added**
- `android/kotlin-test/com/klarvo/voice/CleanupFailureDeliveryTest.kt` — JVM tests for the Step-4 decision and the twin wording (**GATE 2:** `degradeMessage_mirrorsDesktopPillWording` retargeted to the hint-free literal, `degradeMessage_hasNoKeyHintOnAndroid` added). **Review round 1:** 9 → **8** tests — `successfulFallbackProvider_isNotACleanupFailure_andStillPastes` removed as a duplicate (finding 4), its claim bound to `cleanupOk_pastesWhenAccessibilityConnected`'s KDoc.

**Modified — review round 1 (fix round)**
- `src-tauri/src/native_pill.rs` — `PillWindowState.pending_msg` added; `WM_PILL_SET_MSG` stages instead of applying; `WM_PILL_SET_STATE` claims the staged message on state entry and discards it with a dropped Done (finding 1). `handle_timer`'s warning branch + `warning_hold_active` + `NativePill::set_status_msg` docs rewritten (finding 3). **Not compiled on this host** — Windows-gated, no test module.
- `src-tauri/src/pipeline.rs` — `ProcessOutcome::Produced.degrade_msg` KDoc + `spec_degrade_msg_present_exactly_when_llm_error` docstring state 2-of-3 coverage honestly (finding 2); `Delivery::enter_sent` doc corrected to "Insert+Send triggered" (finding 5); new pure `terminal_degrade_msg` + its call in `stop_and_process_pipeline`, plus 3 new specs (finding 6).
- `src/App.tsx` — the existing status line renders `recording.warningMessage` in amber on a `done`-with-warning run (directive D1). No new element or attribute.
- `_bmad-output/implementation-artifacts/7-10-…md` — Task 1 record correction (finding 7), findings checked off, this round's gates/inversions/resolution rows.

**Added — review round 1**
- `_bmad-output/implementation-artifacts/gate4-evidence/7-10/d1-status-line-smoke.mjs` — puppeteer proxy smoke for D1 (10 checks).
- `_bmad-output/implementation-artifacts/gate4-evidence/7-10/run-d1-smoke.sh` — applies/reverts the throwaway `tauri-commands.ts` edit, boots preview, runs the smoke.
- `_bmad-output/implementation-artifacts/gate4-evidence/7-10/` — run artifacts from the **final green run** (`report.txt` 13/13 after round 2; round 1's 10/10 kept as `report-round1.txt`, `status-degraded.json`, `status-ok.json`, `ist-status-degraded.png`, `ist-status-ok.png`). The runner also writes `preview-server.log`, which is **not committed** — `.gitignore:15` (`*.log`).

**Modified — review round 2 (fix round)**
- `src/hooks/useRecording.ts` — the `onStateChanged` listener's `p.warning` capture gains
  `else setWarningMessage(null)`, so a state event without a warning resets it and a hotkey-driven run
  cannot inherit the previous run's degrade text. `handleRecordToggle` untouched.
- `_bmad-output/implementation-artifacts/gate4-evidence/7-10/d1-status-line-smoke.mjs` — CASE C added
  (degraded run → clean run in the same browser boot, 3 checks); the header's "not exercised" list
  corrected accordingly. 10 → **13** checks.
- `_bmad-output/implementation-artifacts/gate4-evidence/7-10/report.txt` — run artifact, final green run (round 2, 13/13); `report-round1.txt` — round 1 (10/10).
- `_bmad-output/implementation-artifacts/7-10-…md` — round-2 follow-up task, gates, inversion,
  resolution row, coverage corrections, this File List block, Change Log.

**Added — review round 2**
- `_bmad-output/implementation-artifacts/gate4-evidence/7-10/status-clean-after-degrade.json` and
  `ist-status-clean-after-degrade.png` — CASE C artifacts from the final green run.

**Added — Task 6 / AC8**
- `src-tauri/src/overlay_message.rs` — the message model. `MessageTone`, `CauseLine` (+`plain`, `text`),
  `OverlayMessage` (+`warning`, `error`), the five `PILL_LABEL_*` constants, `DegradeCause`
  (+`status_line`, `card`), `generic_cause_text`; 7 specs. **Not** platform-gated — this is the only part of AC8
  a Linux host can execute.
- `_bmad-output/implementation-artifacts/gate4-evidence/7-10/report-task6.txt` — the Task-6 D1 regression run
  (13/13, identical to round 2 apart from the timestamp); `report.txt` is that same run.

**Modified — Task 6 / AC8**
- `src-tauri/src/pipeline.rs` — `ProcessOutcome::Produced.degrade_msg` → `degrade_cause: Option<DegradeCause>`;
  new `degrade_cause_for_model` + `generic_degrade_cause` at the three degrade sites; `terminal_degrade_msg` →
  `terminal_degrade_cause`; `deliver_outcome`'s 9th tuple field re-typed; the terminal event takes the cause;
  `start_recording_only` creates the preview window unconditionally. `degrade_warn_msg` /
  `degrade_warn_msg_for_model` moved into `mod tests` as one-line projections (no production caller left).
  4 existing specs adapted, 3 of them gaining a card assertion.
- `src-tauri/src/hotkey/mod.rs` — `PipelineEvent.message` (`#[serde(skip_serializing)]`);
  `done_with_clipboard_only` takes `Option<DegradeCause>`; `warn` / `error` build their own card; 3 new specs,
  1 existing extended.
- `src-tauri/src/native_pill.rs` — `fit_text`, `status_msg`, `pending_msg`, `WM_PILL_SET_MSG` and
  `set_status_msg` **removed**; new `TerminalKind` + `NativePillState::DoneDegraded`; `from_code` and `set_state`
  re-signed; the five label arms folded into the new `draw_static_label`; `handle_timer` /
  `warning_hold_active` docs rewritten around "a light, not a sentence". **Not compiled on this host.**
- `src-tauri/src/native_preview.rs` — message mode: `WM_PREVIEW_SET_MESSAGE`, `NativePreview::set_message`,
  `render_message_card`, `present` (factored out of `render_frame` for the fade), `dismiss_message`,
  `set_click_through`, `text_width`, `draw_msg_line`, `fill_round_rect`, `msg_line_h`, the `WM_TIMER` /
  `WM_LBUTTONDOWN` arms, 5 message fonts, and the canon colour constants. **Not compiled on this host.**
- `src-tauri/src/lib.rs` — `mod overlay_message`; `emit_pipeline_state` derives `TerminalKind` and posts
  `set_message` before `set_state`, and no longer posts any text to the pill; `run`'s setup creates the native
  preview so boot-time config warnings have a surface.
- `src-tauri/src/commands/misc.rs` — `ensure_preview_window`'s docstring: the window now also exists at setup and
  is created unconditionally at recording start.
- `_bmad-output/implementation-artifacts/7-10-…md` — Task 6 checked off with subtasks, this round's gates,
  inversions, change table, coverage statement, the AC8-corrected GATE-4 steps, File List and Change Log.

**Not modified in this round (verified by `git diff --stat`):** every path under `android/`, `test-fixtures/` and
`src/` — 0 lines. `docs/design/overhaul/**` unchanged (canon + MANIFEST + render already landed in `9118f1f`).

**Modified — AC8 review fix round (P1–P11 + D1–D3)**
- `src-tauri/src/overlay_message.rs` — `OverlayMessage::warnings` **added** (P8); `OverlayMessage::error` runs its
  text through the new `user_facing_error_text` + `ERROR_TOKEN_TEXT` table (D3); `DegradeCause::card`'s
  `ModelNotFound` arm restores the quote delimiters around the chip (P11) and its `ClipboardWriteFailed` arm
  becomes `MessageTone::Error` / `TEXT LOST` / "Clipboard write failed — raw text is in History" with no `next`
  line (D2); KDoc on `card` and on the `ClipboardWriteFailed` variant rewritten. Tests: 2 updated
  (`spec_model_not_found_card_…`, `spec_clipboard_write_failure_card_…`), **3 added**
  (`spec_clipboard_write_failure_keeps_the_degraded_pill_label`,
  `spec_known_error_tokens_are_translated_unknown_ones_are_verbatim`, `spec_boot_warnings_merge_into_one_card`).
  7 → **10** specs.
- `src-tauri/src/hotkey/mod.rs` — `PipelineEvent::warn_all` added (P8) + `spec_boot_config_warnings_travel_as_one_event`.
- `src-tauri/src/lib.rs` — `run`'s `config_warnings` loop becomes one `warn_all` event (P8);
  `emit_pipeline_state`'s AC8 comment names the card's hold (P1).
- `src-tauri/src/pipeline.rs` — `degrade_cause_for_model`'s docstring: return description corrected and the dead
  `[degrade_warn_msg]` intra-doc link replaced by `[generic_degrade_cause]` (P6). Doc-only; no code line changed.
- `src-tauri/src/native_preview.rs` — `MSG_BG` / `MSG_BG_ALPHA` added and used instead of `config.bg_*` (P2);
  `MSG_SANS_FACE` → `"Geist"` + `load_embedded_geist()` call in `preview_thread` (P3); `MSG_MONO_FACE`'s
  provenance comment corrected (P4); `MSG_CHIP_H_FRACTION` added and the chip rect + padding derived from
  `cause_h` / `text_scale`, in the measuring pass too (P7); `WM_PREVIEW_SET_MESSAGE`'s `Some` arm checks
  `SetTimer` and keeps click-through on failure (P9), its `None` arm honours `MSG_HOLD_MS` (P1);
  `render_message_card` returns 0 when `Pixmap::new` fails (P10); `compute_preview_geometry` takes `work_bottom`
  and returns `below_pill`, `PILL_HEIGHT_LOGICAL` added, `PreviewWindowState.{work_bottom, below_pill}` added, and
  both `render_message_card` and `render_frame` flip their card alignment on it (D1); module doc updated.
  **Not compiled on this host.**
- `src-tauri/src/native_pill.rs` — `load_embedded_geist()`, a `pub(crate)` `Once`-guarded registrar for the three
  bundled Geist faces, extracted from `pill_thread` so `native_preview` can call it regardless of boot order (P3).
  **Not compiled on this host.**
- `_bmad-output/implementation-artifacts/7-10-…md` — the AC8 findings checked off with the three decisions
  annotated, the *Border width* record row corrected (P5), this round's gates / inversions / resolution rows /
  coverage statement, this File List block and the Change Log.
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — `7-10` back to `in-progress` for the fix round,
  then `review`.
- `_bmad-output/implementation-artifacts/gate4-evidence/7-10/report.txt` — run artifact, this round's D1
  regression run (13/13, identical to the Task-6 run apart from the timestamp).

**Added — AC8 review fix round**
- `_bmad-output/implementation-artifacts/gate4-evidence/7-10/windows-build-r3.exit` — the Windows release build's
  exit code on `d778d5c` (**0**). The full `windows-build-r3.log` is written next to it but **not committed** —
  `.gitignore:15` (`*.log`), same as `windows-build-r2.log` and `preview-server.log`. The numbers quoted from it
  above (74 warnings, 56 `unused BOOL`, both identical to build r2) were counted from the file on disk.

**Not modified (verified):** `src-tauri/src/config.rs`, `src/components/**`, `src-tauri/src/history/mod.rs`, `test-fixtures/**`, `KlarvoAccessibilityService.performEnter`. **Round 2 additionally:** no file under `src-tauri/` and no file under `android/` (`git diff --stat` → zero such paths), and `src/App.tsx` unchanged — the fix is in the hook, not the renderer.

## Change Log

| Date | Change |
|---|---|
| 2026-09-14 | **AC8 review fix round — 11 patch findings (P1–P11) + Andi's three directives (D1–D3) applied; 10 deferrals and 11 dismissals untouched.** Card: background pinned to the render's `rgba(14,16,18,.82)` so the "Light" preview theme can no longer put near-white text on a near-white card (P2); typeface `Segoe UI` → the bundled `Geist`, with `native_pill::load_embedded_geist()` extracted as a `Once`-guarded registrar both overlay threads call, because since AC8 the preview window can be created before the pill exists (P3); the model-ID chip's rect and padding derived from `cause_h` so they track the accessibility text scale its glyphs already honoured (P7); `Pixmap::new` failure now dismisses instead of presenting the previous frame as the message (P10); `SetTimer` checked, and on failure the card stays click-through rather than becoming a permanent click trap (P9). Lifetime: a message-less event no longer takes a live card down inside its 4 s hold — only a new recording, a click and the timer dismiss (P1). Boot: several config warnings become ONE event and ONE card via new `PipelineEvent::warn_all` / `OverlayMessage::warnings` (P8). Wording: the model-ID chip's quote delimiters restored (P11); **D2** — the clipboard-write failure becomes a danger card, `TEXT LOST` · `Clipboard write failed — raw text is in History`, while the pill deliberately stays `Cleanup failed`; **D3** — known machine tokens are translated on the card (`feature_requires_license:CommandMode` → `Command mode needs a license`), unknown ones stay verbatim, and `event.error` keeps the raw token for every other consumer. **D1** — when the card does not fit above the pill the whole window flips 8 px **below** it and hugs it from there, live preview included (`compute_preview_geometry` gains `work_bottom` and returns `below_pill`). Docs: `degrade_cause_for_model`'s wrong return description + dead intra-doc link (P6), the false "Cascadia Code ships with the app" provenance (P4), and the record's false "background and radius follow the user's appearance settings" row (P5). Gates: `cargo test --lib` 701 → **705**, `cargo build --lib` 13 pre-existing warnings, `npm run build` clean, D1 proxy smoke **13/13** (regression — `status_line()` untouched), trap #5 re-executed (no new event name, field or emit). **Windows release build green on `d778d5c` (exit 0, exe fresh, 74 warnings — the same count as build r2)**, which is the only gate that compiles the two overlay files. Four inversions shown RED and reverted; `git status` clean. **8 of the 14 items are in those Windows-gated files and have ZERO test coverage** — the build proves they compile, nothing more. **Andi's GATE-4 remains outstanding**, now with 8 new items in step 6. |
| 2026-09-14 | **Task 6 / AC8 implemented — pill becomes a status light, the preview card becomes the message surface.** GATE-4 round 1 failed on presentation: the 200×36 pill cut the degrade sentence and the clipboard hint never showed. New `src-tauri/src/overlay_message.rs` (not platform-gated) carries the message as a *model* — `DegradeCause` → `status_line()` for the main window, `card()` for the overlay — so nothing is re-derived by parsing a string apart. `PipelineEvent` gains an in-process-only `message` field (`#[serde(skip_serializing)]`), so the frontend payload and every event name are unchanged. `native_pill.rs` loses `fit_text` and the whole `status_msg` staging apparatus and renders five fixed labels, with `DoneDegraded` ("Cleanup failed") splitting the degrade route off the focus-loss "In Clipboard". `native_preview.rs` gains message mode: header + dot · cause with the model ID on an amber chip · clipboard line · hint, tone-coloured border, 4 s hold + 1 s fade, click-to-dismiss, shown regardless of `live_preview_enabled` — so the window is now created at setup *and* at every recording start. Gates: `cargo test --lib` 691 → **701**, `npm run build` clean, D1 proxy smoke **13/13** (regression: AC8 deliberately keeps the main-window wording). Both inversions shown RED and reverted; `git status` clean. **The two overlay files were not compiled on this host** — only parsed; the Windows build is the conductor's step and **Andi's GATE-4 remains outstanding**, now with AC8's expectation in the steps. |
| 2026-09-14 | **Review cleared (conductor close-out).** 2 fix rounds, loop closed on the re-review; 3 residual decisions → backlog (Andi), 6 editorial items fixed in `d73082d`. Windows release build green (`d73082d`, exe fresh). GATE-4 self-verification + Andi's residual steps: `gate4-evidence/7-10/verdict.md`. Status stays `review` (both fields) until Andi's Windows smoke. | conductor |
| 2026-09-14 | **Review round 2 fix round — exactly one confirmed finding applied.** `src/hooks/useRecording.ts`'s `onStateChanged` listener now clears `warningMessage` when a state event carries no warning (`else` at the capture line), so a hotkey-driven run cannot show the previous run's amber degrade text instead of "Done". `handleRecordToggle` untouched. The D1 proxy smoke gained CASE C — a degraded run followed by a clean run in the **same** browser boot — and its header's "not exercised" list was corrected. Gates: D1 smoke 10 → **13/13**; `npm run build` clean; `cargo test --lib` **691** and the Kotlin gate **24 suites / 194 tests** carried over as regression statements about untouched code (gradle reported the test task `UP-TO-DATE` — replayed, not re-executed). The pre-fix tree was the RED, naming the previous run's model ID verbatim. **Named, not hidden:** the STT fallback-ladder warning is now cleared by the next event of its own run, so the main window no longer carries it into `done` — intended per Q1, but a behaviour change; the pill remains that warning's surface. Deferred/dismissed findings untouched. **Andi's GATE-4 still outstanding.** |
| 2026-09-14 | **Review round 1 fix round — 7 patch findings + decision D1 applied; 3 canon patches already landed in `a371091`; 12 deferrals untouched.** Pill: `status_msg` is now staged and claimed by its own state, so no state inherits the previous one's text (F1); stale Warning→Done comments rewritten around the STT ladder (F3). Pipeline: the `degrade_msg` invariant states 2-of-3 test coverage honestly instead of "all three" (F2); `enter_sent` documented as "Insert+Send triggered" (F5); new pure `terminal_degrade_msg` stops the message promising a clipboard the write never reached (F6). Kotlin: the duplicate `successfulFallbackProvider_*` test removed, its claim bound to the real one (F4). Record: Task 1's `test_deliver_outcome_*` claim corrected against `git diff` (F7). **D1 (Andi):** the main window shows the cleanup failure in the existing status line, amber, same wording as the pill — one colour arm + one text branch in `App.tsx`, no new surface. Gates: `cargo test --lib` 688 → **691**, JVM **24 suites / 194 tests** (195 → 194, duplicate removed), `npm run build` clean, D1 proxy smoke **10/10**. Three inversions shown RED and reverted; `git status` clean. **F1/F3 are in Windows-gated `native_pill.rs` — not compiled, not tested on this host. Andi's GATE-4 still outstanding.** ⚠ Flagged: D1 makes the deferred stale-`warningMessage` defect user-visible — needs re-deciding, not fixed here. |
| 2026-09-14 | **GATE-2 directive applied (Andi):** the Android toast drops the `(Ctrl+V)` hint — `KlarvoOverlayService.CLEANUP_FAILED_CLIPBOARD_MSG` is now `Cleanup failed — raw text in clipboard`. Desktop pill wording and `degrade_warn_msg_for_model` (model-not-found) **unchanged**. One constant + its wording test, as scoped: test retargeted, one negative assertion added. JVM gate RED (2 failures) → GREEN 24 suites / 195 tests / 0 failures; `cargo test --lib` 688/688 unchanged. **Andi's GATE-4 still outstanding.** |
| 2026-09-14 | Story 7-10 implemented. Desktop: `llm_error` threaded to the paste step; clipboard-only branch via new `PasteHandler::copy_only`; degrade cause carried on a single terminal `DoneClipboard` event (Q1) and rendered by the pill; wording reworked per Q2. Android twin: explicit `llmCleanupFailed`, pure `decideDelivery` seam, Step-4 branch, one combined English toast (Q4/Q5). Gates: `cargo test --lib` 688/688, JVM 194/194 (24 suites), `npm run build` clean, trap #5 executed. Both AC6 inversions shown RED and reverted. `cargo clippy` blocked (not installed on host). **Andi's GATE-4 outstanding.** |
