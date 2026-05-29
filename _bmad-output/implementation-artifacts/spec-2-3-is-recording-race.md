---
title: 'Fix is_recording check-then-act race + harden session-lock poison (Task 2.3, Phase 2)'
type: 'bugfix'
created: '2026-05-29'
status: 'done'
baseline_commit: '6e03489'
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `start_recording_only` (`pipeline.rs:526`) pre-checks `is_recording()` (529), then captures the foreground window + sets up the emitter, and only *then* calls `start_recording()` (549). Two fast hotkey presses spawn two async tasks: both pass the pre-check, both overwrite `prev_foreground_hwnd`/`prev_window_title`, then one wins `start_recording` and the loser emits a spurious "Failed to start recording" event — clobbering the paste-back window and accumulating stale state (the documented "toggle aborts after 30–60 min" bug). Separately, `session`/`monitor_session` use `.lock().unwrap()` (audio/mod.rs:278, 358, 406, 433, 460, 471) — a poison panics the hotkey thread.

**Approach:** The recorder ALREADY holds the `session` lock atomically across check+insert and returns `Err(AlreadyRecording)` (audio/mod.rs:278–281). Make callers rely on that single atomic gate, not a racy pre-check, and stop the loser from clobbering shared state. **Step A (net):** device-free characterization tests pinning the recorder's atomic-rejection contract + a poison-recovery regression test. **Step B (harden):** in `start_recording_only`, drop the pre-check and reorder so observable side effects (fg-capture, `recording_start`, bar, `recording()` event) run only after `start_recording` returns `Ok`; treat `Err(AlreadyRecording)` as a silent `debug!` no-op. Replace the 6 session/monitor `.lock().unwrap()` with poison-recovering access (`unwrap_or_else(into_inner)` + `warn!`).

## Boundaries & Constraints

**Always:**
- Behavior-preserving in the HAPPY path; the only intended changes are at the race-loss and poison paths (the bug). 510 existing tests stay green after each step; each step is its own green commit (the human commits).
- Preserve `start_recording_only`'s "returns silently if already recording" contract — now via the `Err(AlreadyRecording)` arm (no error event).
- Callbacks (`setup_audio_level_emitter`, silence) stay installed before `start_recording` (consumed by `.take()` inside it); only loser-observable effects move after the gate.
- Poison recovery = `unwrap_or_else(|e| e.into_inner())` + `warn!` — never a silent default; the `Option<Session>` is never torn (mutations are atomic take/insert).

**Ask First:**
- If fg-capture after the gate proves to capture a different window in practice (focus shifts during init) — HALT and report.
- If poison recovery (vs `map_err`-to-error) for the `Result` fns is contentious — flag at review; default recover (self-healing, avoids bricking recording on a poisoned mutex).

**Never:**
- No `SessionSlot` 3-variant rewrite / no decoupling device-init from the lock (deferred — see Design Notes). No new crates, no module split.
- Don't touch the toggle/autostop/auto start-vs-stop pre-checks (`run_dictation_pipeline:1787`, handlers `1968`/`1988`) — now race-safe via the authoritative gate.
- Don't touch STOP-side `is_recording()` guards (`dispatch.rs:400`, `commands/voice_command.rs:62`, `pipeline.rs:1194`) — `stop_recording` is idempotent (`NotRecording`).
- Don't widen into `commands/recording.rs::start_recording` (same latent pattern, but maps error to frontend — no spurious event; note only).

## I/O & Edge-Case Matrix

`start_recording_only` after Step B — and the recorder gate it relies on:

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Single press, idle | not recording | `start_recording` Ok → THEN capture fg window, set `recording_start`, show bar, emit `recording()` | — |
| Double press, 2nd loses race | already recording | `start_recording` → `Err(AlreadyRecording)`; `debug!` + silent return; NO fg clobber, NO error event | benign no-op |
| Genuine start failure | no mic / device error | `start_recording` → `Err(DeviceError\|NoInputDevice)`; emit `error()` event; return | error event |
| Poisoned session lock | lock poisoned | `is_recording`/`start_recording` recover via `into_inner` + `warn!`; no panic | recover + warn |
| Stop when not recording | idle | `stop_recording` → `Err(NotRecording)`; callers ignore (unchanged) | unchanged |

</frozen-after-approval>

## Code Map

- `src-tauri/src/audio/mod.rs` — `start_recording:277` (holds `session` lock over check+insert; `Err(AlreadyRecording)`:280), `stop_recording_with_gain:357`, `is_recording:404`, `start_monitor:428`, `stop_monitor:459`, `is_monitoring:469`. The 6 `.lock().unwrap()` to harden: 278, 358, 406, 433, 460, 471. Test module ends ~1340 (insert dummy-session helper here). `RecordingSession:113` (constructible device-free via `sync_channel`/`channel`).
- `src-tauri/src/pipeline.rs` — `start_recording_only:526` (the racy caller — reorder + drop pre-check at 529). `start_autostop_recording:598`/`start_auto_recording:661` delegate to it (their pre-checks 601/668 left as harmless fast-paths). `AudioError` already in scope.
- `src-tauri/src/lib.rs` — `lock!:335` (poison-safe macro used by recording.rs; not needed in audio/mod.rs — recover inline). `setup_audio_level_emitter` (`#[cfg(desktop)]`, called pre-gate).
- `android/.../KlarvoApi.kt` — no `is_recording`/`start_recording` mirror → no `/sync-prompts` needed.

## Tasks & Acceptance

**Execution — Step A (net), commit when green:**
- [x] `audio/mod.rs` — add `#[cfg(all(test, desktop))]` helper to insert a dummy `RecordingSession` (throwaway `sync_channel(1)` + `channel()`, no cpal). Tests: (1) with dummy session present, `start_recording(None)` returns `Err(AlreadyRecording)` device-free (proves the is_some short-circuit before device init); (2) `is_recording()` reflects the dummy session true→false; (3) poison-recovery: poison the `session` lock (panic under `catch_unwind` while holding it), then `is_recording()` does not panic. Test (3) is RED before Step B, GREEN after.

**Execution — Step B (harden), commit when green:**
- [x] `audio/mod.rs` — replace `.lock().unwrap()` on `session` (278, 358, 406) and `monitor_session` (433, 460, 471) with `.lock().unwrap_or_else(|e| { log::warn!(...); e.into_inner() })`. Happy path unchanged.
- [x] `pipeline.rs` — `start_recording_only`: remove `is_recording()` pre-check (529); call `start_recording` as the gate (keep emitter/callback install before it); on `Ok` run fg-capture + `recording_start` + bar + `recording()` event; on `Err(AlreadyRecording)` `debug!` + return silently (no event); on other `Err` emit `error()` event.

**Acceptance Criteria:**
- Given the full suite, when `cargo test` runs in `src-tauri`, then ≥510 pass / 0 fail — after Step A AND after Step B.
- Given a losing double-fire press, when `start_recording_only` runs, then it neither overwrites `prev_foreground_hwnd`/`prev_window_title` nor emits an `error()` event (verified by code review of the reordered control flow).
- Given a poisoned `session`/`monitor` lock, when `is_recording`/`is_monitoring`/`start_recording`/`stop_recording_with_gain`/`start_monitor`/`stop_monitor` run, then no panic and a `warn!` is logged.
- Given `KlarvoApi.kt` grepped for the touched symbols, then no Kotlin change is required.
- Given a Windows release build (DoD gate), when two rapid toggle presses fire, then exactly one recording starts and paste targets the correct window (manual smoke test).

## Spec Change Log

**Implementation deviations (recorded during Step B, all within the approved approach):**

1. **Poison-recovery test placement.** The Step A task lists test (3) but notes it is RED until Step B; a green Step A commit cannot contain it. Implemented in Step B alongside the `lock_recover` fix (red→green there). Step A commits with tests (1)+(2) green (512 passed).
2. **Poison test is representative, not exhaustive.** All six session/monitor lock sites route through one `lock_recover` helper; the regression test exercises `is_recording()` (the hottest getter) as the representative. The other five are wired identically (verified by code review), not each given a separate test.
3. **`lock_recover` centralization.** Spec proposed inline `unwrap_or_else(into_inner)` per site. Implemented as one shared `#[cfg(desktop)] fn lock_recover<'a, T>(&Mutex<T>, label) -> MutexGuard` — DRY + a single place for the safety rationale. Behavior identical.
4. **Windows-gnu cross-compile not run.** Verified instead that no `#[cfg(target_os="windows")]` code was touched and no changed signature (`start_recording`, `is_recording`, `start_recording_only`) shifted, so windows-cfg callers are unaffected; all changed code is `#[cfg(desktop)]`/non-cfg and compiled + tested on Linux-desktop (513 tests). The windows-gnu target isn't installed and would only re-check unchanged windows-cfg code. The real Windows verification remains the runtime press-to-paste smoke test (DoD hard-gate).

**Review (iteration 1) — 3 adversarial reviewers (blind / edge-case / acceptance), Opus, no shared context.** Acceptance auditor verdict: **PASS** — all code-verifiable ACs met (AC1-AC4; AC5 = the human Windows-smoke DoD gate, correctly deferred), all Always/Never boundaries respected, no Ask-First HALT slipped through, every I/O-matrix row matches, all 4 deviations above accurate. No critical/high findings; the race-fix gate logic and reorder confirmed correct by all three (loser does nothing observable; shared fg/timestamp state written only by the unique gate-winner). Dispositions:

5. **patch — `lock_recover` doc corrected.** Blind reviewer caught the doc claiming "logs once" while `into_inner` does not clear the poison flag (so it warns on EACH recovery → log-spam on a persistently-poisoned hot-path lock). Rewrote the doc to state the accurate logging behavior AND the recovery-soundness invariant the edge-case reviewer flagged (the lock-held windows must stay panic-free; a panic there + recovery could double-start). Code behavior unchanged; comment now accurate.
6. **defer ×3 (out of declared scope, see `deferred-work.md` Task 2.3 section).** (a) Extend poison-recovery to `level_callback`/`silence_config` (now on the hot path, spec scoped them out). (b) Decouple the recording-start claim from device init — the `SessionSlot` reserve refactor already deferred in Design Notes (makes `lock_recover`'s recovery structurally safe + removes the during-init `is_recording()` block). (c) Pre-existing stale-`recording_start` window (low; not worsened by this change).
7. **reject.** Orphan level-callback on lost-race/error presses — two reviewers independently confirmed benign (callbacks are equivalent, overwritten next cycle, matches the documented comment).

## Design Notes

**Why no `SessionSlot` 3-state rewrite (considered, deferred).** Decoupling the atomic claim from device init (an `Idle/Starting/Active` enum) would enable a device-free *concurrent* "one winner" test and remove the `is_recording()`-blocks-during-init latency. Rejected here: larger than the briefing scopes, adds new edge surface (stop-during-Starting, monitor-pause interaction) that could trade one race for another, and isn't the bug's root cause (the caller clobber + spurious error are). The existing held-lock check+insert is already atomic — the fix is to make callers rely on it. File a separate story if the init latency bites.

**Net strategy.** Concurrent-from-empty can't be unit-tested device-free (the winner needs a real mic). The device-free net pins the invariant the caller refactor depends on — "recorder rejects a 2nd start" — and real concurrent timing is covered by the Windows smoke test (Smoke-Test-DoD gate).

**Reorder rationale.** The doc says capture fg "BEFORE start recording", but the user just pressed the hotkey — focus doesn't change during the ~tens-of-ms init, so capturing after the `Ok` gate yields the same window while the loser captures nothing. The one intended timing shift; flagged in the matrix.

## Verification

**Commands:**
- `cd src-tauri && cargo test` — expected: `≥510 passed; 0 failed` (after Step A, then Step B).
- `cd src-tauri && cargo clippy --all-targets` — expected: no new warnings.
- `cd src-tauri && cargo check --target x86_64-pc-windows-gnu` — expected: clean (cfg(desktop) blocks touched; Linux tests don't compile Windows-only paths).

**Manual checks:**
- `git grep -n '\.lock()\.unwrap()' src-tauri/src/audio/mod.rs` — expected: no matches on `session`/`monitor_session` lines (other locks e.g. `level_callback`/`live_buffer` out of scope).
- Windows release build (`scripts/sync-and-build.ps1`) + manual press-to-paste: two rapid toggle presses → one recording, correct paste window. **Hard DoD gate** — Linux `cargo test` does not exercise the cpal hotkey path.
- Rust↔Kotlin: `KlarvoApi.kt` mirrors none of the touched symbols → no `/sync-prompts`.

## Suggested Review Order

**The race fix (start here — the design intent)**

- Entry point: `start_recording` is now the single atomic gate; no racy `is_recording()` pre-check precedes it.
  [`pipeline.rs:541`](../../src-tauri/src/pipeline.rs#L541)

- The race loser does nothing observable — silent `debug!`, no foreground clobber, no error event.
  [`pipeline.rs:543`](../../src-tauri/src/pipeline.rs#L543)

- Foreground-window capture moved to AFTER the gate, so only the winner writes shared state (the clobber fix).
  [`pipeline.rs:558`](../../src-tauri/src/pipeline.rs#L558)

**Poison hardening (the lock the gate relies on)**

- `lock_recover`: poison no longer panics the hotkey thread; doc states the recovery-soundness invariant.
  [`audio/mod.rs:209`](../../src-tauri/src/audio/mod.rs#L209)

- The recorder's held-lock check+insert — the atomic `Err(AlreadyRecording)` contract callers now depend on.
  [`audio/mod.rs:303`](../../src-tauri/src/audio/mod.rs#L303)

- `is_recording()` now recovers a poisoned lock instead of `.unwrap()`-panicking (representative of all 6 sites).
  [`audio/mod.rs:430`](../../src-tauri/src/audio/mod.rs#L430)

**Tests (the net, supporting)**

- Characterization: a 2nd start is rejected device-free (dummy session, no cpal).
  [`audio/mod.rs:1386`](../../src-tauri/src/audio/mod.rs#L1386)

- Regression: a poisoned session lock recovers without panic (red before the fix, green after).
  [`audio/mod.rs:1416`](../../src-tauri/src/audio/mod.rs#L1416)
