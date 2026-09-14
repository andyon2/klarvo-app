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

### Out of scope (verbatim from the epic)

> **Out of scope:** the failed-entries inbox (backlog candidate), pill buttons (assessed + parked in 7-9),
> any change to STT, VAD, JNI, the fallback ladder or the config schema.

## Tasks / Subtasks

- [x] **Task 1 — Desktop: thread `llm_error` to the paste step and branch** (AC1, AC6)
  - [x] `pipeline::deliver_outcome`: carry `llm_error` out in the returned tuple (7 → 8 fields); keep the
        `llm_error_count` increment and every other invariant in its docstring intact. Update the `let Some((…))
        else` destructuring in `stop_and_process_pipeline` and the two existing
        `test_deliver_outcome_*` tests.
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

- [x] **Task 5 — Gates** (AC7): `cargo test --lib`, the JVM gate, trap #5, coverage statements, then hand GATE-4
      to Andi with the exact steps from the epic DoD. Anchor the record **by symbol**, write resolution rows from
      `git diff` (Epic-7 retro D2).

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
  The amber the code uses for both `Warning` and `DoneClipboard` (255,163,68 = `#FFA344`) already matches.
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
`spec_degrade_msg_present_exactly_when_llm_error` asserts `degrade_msg.is_some() == llm_error` across
all three degrade paths plus the success path.

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

- **688 Rust / 195 Kotlin green prove wiring, logic and structure — not design, not pixels.**
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

**OPEN — AC7 is not fully met. Andi's GATE-4 is outstanding** (Windows release build via
`scripts/windows-build.sh`, which was not run in this session). Exact steps, from the epic DoD:

1. Set a **wrong DeepSeek model ID** (Settings → Advanced → Model IDs) and switch **Insert+Send ON**.
2. Dictate into a chat-style target window.
3. Expect: **nothing lands in the active window**; **no Enter is sent**; the pill shows the warning
   **with the model ID** (`Model '<id>' not found — in clipboard`, amber, ~4 s via `DONE_CLIPBOARD_MS`);
   **Ctrl+V pastes the raw text**.
4. Regression in the same build: with a **correct** model ID, paste and Insert+Send behave as before.

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
- `android/kotlin-test/com/klarvo/voice/CleanupFailureDeliveryTest.kt` — 9 JVM tests for the Step-4 decision and the twin wording (**GATE 2:** `degradeMessage_mirrorsDesktopPillWording` retargeted to the hint-free literal, `degradeMessage_hasNoKeyHintOnAndroid` added).

**Not modified (verified):** `src-tauri/src/config.rs`, `src/components/**`, `src-tauri/src/history/mod.rs`, `test-fixtures/**`, `KlarvoAccessibilityService.performEnter`.

## Change Log

| Date | Change |
|---|---|
| 2026-09-14 | **GATE-2 directive applied (Andi):** the Android toast drops the `(Ctrl+V)` hint — `KlarvoOverlayService.CLEANUP_FAILED_CLIPBOARD_MSG` is now `Cleanup failed — raw text in clipboard`. Desktop pill wording and `degrade_warn_msg_for_model` (model-not-found) **unchanged**. One constant + its wording test, as scoped: test retargeted, one negative assertion added. JVM gate RED (2 failures) → GREEN 24 suites / 195 tests / 0 failures; `cargo test --lib` 688/688 unchanged. **Andi's GATE-4 still outstanding.** |
| 2026-09-14 | Story 7-10 implemented. Desktop: `llm_error` threaded to the paste step; clipboard-only branch via new `PasteHandler::copy_only`; degrade cause carried on a single terminal `DoneClipboard` event (Q1) and rendered by the pill; wording reworked per Q2. Android twin: explicit `llmCleanupFailed`, pure `decideDelivery` seam, Step-4 branch, one combined English toast (Q4/Q5). Gates: `cargo test --lib` 688/688, JVM 194/194 (24 suites), `npm run build` clean, trap #5 executed. Both AC6 inversions shown RED and reverted. `cargo clippy` blocked (not installed on host). **Andi's GATE-4 outstanding.** |
