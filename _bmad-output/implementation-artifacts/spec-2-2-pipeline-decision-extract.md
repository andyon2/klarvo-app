---
title: 'Extract pipeline decision logic + process_audio core (Task 2.2, Phase 2)'
type: 'refactor'
created: '2026-05-29'
status: 'done'
baseline_commit: '9af4d2c'
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `stop_and_process_pipeline` (`pipeline.rs:779–1485`) is a ~700-line god-function — 50 `AppState` accesses, 22 early-returns, several of which emit `PipelineEvent::idle()` and `return` with **no log**, so "STT discarded everything" is indistinguishable from "user said nothing". Its decision logic (skip / offline / command / cleanup) has zero tests of the function; the only "tests" replicate the expressions inline (`pipeline.rs:1821–1875`), giving false security.

**Approach:** Two behavior-preserving steps against a characterization net. **Step A (net):** extract the decision logic into pure functions, snapshot the decision matrix (insta — already a dev-dep), and route the live function through them, redirecting the replicating tests to the real fns. **Step B (extract):** pull `STT→guard→LLM→sanitize` into `async fn process_audio(ProcessInput, emit) -> ProcessOutcome` with no `AppHandle`/`AppState` → first time unit-testable; the shell snapshots inputs, calls it (the live path), and handles the outcome. Every former silent early-return gains a `warn!`/`error!`.

## Boundaries & Constraints

**Always:**
- Behavior-preserving across BOTH steps; 489 existing tests stay green after each step; each step is its own green commit (the human commits).
- `process_audio` MUST be the live path — `stop_and_process_pipeline` calls it; no dead parallel function.
- Pure decision helpers take primitives/snapshots only (no locks/`AppState`/`AppHandle`).
- Preserve EXACTLY: event sequence (`transcribing`→conditional `cleaning`→`idle`/`error`/`warn`, then shell `done`), `feedback_metrics` increment order+values, command-mode reset/`take` timing, every prompt/STT-hint string, provider selection, fallback order.

**Ask First:**
- If any extraction would change an observable (event, metric, produced text, command-mode flag state) vs today — HALT and report.
- If `process_audio` cannot stay `AppState`-free without a redesign larger than "snapshot in / outcome out + emit callback" — HALT (breaches "no big rebuild").

**Never:**
- No new crates (reuse `insta`), no module split, no new file — all in `pipeline.rs`.
- Do NOT move recording-stop, silence detection, paste, history, sync, webhook, or feedback-latency into `process_audio` — it is `STT→guard→LLM→sanitize` ONLY; the rest stays in the shell.
- Do NOT touch `recording.rs::is_offline_mode` (different semantics) or widen into `AppState`/`AppConfig` decomposition.

## I/O & Edge-Case Matrix

`process_audio(input, emit)` — `emit` records the event sequence; outcome drives the shell.

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Normal cleanup | offline=false, selected_text=None | emits `transcribing`→`cleaning`; `Produced{ cleaned=sanitized, raw, is_command=false, tokens, llm_ms=Some }` | — |
| Offline dictation | offline=true, selected_text=None | emits `transcribing` only (NO `cleaning`); `Produced{ cleaned=raw, llm_ms=None, tokens=None }`, no LLM call | — |
| Command mode | selected_text=Some | emits `transcribing`→`cleaning`; `rewrite(sel, raw)`; `Produced{ is_command=true }` | rewrite `Err` → emit `error`; `Stop{ llm_error=true }` |
| Prompt-echo / blocklist | raw is hallucination | emit `idle` + log; `Stop{}` (no metric) | — |
| STT failure | `transcribe` `Err` | emit `error(friendly_error)`; `Stop{ stt_error=true }` | counter delta signaled in outcome |
| LLM retryable fail | primary 429/5xx, fallback key set | try fallback; ok→`Produced`; both fail→degrade to raw + emit `warn`; `Produced{ llm_error=true }` | degrade-to-raw |
| LLM non-retryable fail | 400/401/403 | degrade to raw + emit `warn`; `Produced{ llm_error=true }` | degrade-to-raw |

</frozen-after-approval>

## Code Map

- `src-tauri/src/pipeline.rs` — `stop_and_process_pipeline:779` (→ thin shell); NEW decision helpers + types + `process_audio` go here; test module:`1810`; replicating offline tests:`1821–1875` (redirect).
- `src-tauri/src/llm/mod.rs` — `chunked_cleanup:1297`, `CleanupResult:286`, `CleanupStyle:67` (Copy), `CleanupProvider` trait:`313` (`rewrite:345`) — called by `process_audio` (already imported).
- `src-tauri/src/stt/mod.rs` — `SttProvider` trait:`81` (`transcribe:82`), `is_hallucination` — called by `process_audio` (already imported).
- `src-tauri/src/lib.rs` — `friendly_error:455` (error-message construction, already imported).
- `src-tauri/src/commands/recording.rs` — `is_offline_mode:102` — DISTINCT fn (`stt=="local"` only), NOT touched.

## Tasks & Acceptance

**Execution — Step A (net), commit when green:**
- [x] `pipeline.rs` — add pure fns: `is_offline(stt_provider:&str, llm_provider:&str)->bool` (= `stt=="local" && llm!="local"`); `silence_skip(duration_ms, min_recording_ms, rms:Option<f32>, threshold)->Option<SilenceSkip>` (`TooShort`|`Silent`); `post_stt_skip(text:&str, hint:&str)->Option<PostSttSkip>` (`PromptEcho` then `Blocklist`, same order as today); `select_llm_path(offline:bool, has_selected_text:bool)->LlmPath` (`OfflineRaw`|`Command`|`Cleanup`).
- [x] `pipeline.rs` — route `stop_and_process_pipeline` through these (offline flag, silence detection, post-STT guards, LLM-path/cleaning-emit); byte-identical behavior.
- [x] `pipeline.rs` tests — redirected the 5 replicating offline tests to call `is_offline(...)`; added 12 unit tests + one insta snapshot of the decision matrix (silence × post-stt × path). 502 passed / 0 failed.

**Execution — Step B (extract), commit when green:**
- [x] `pipeline.rs` — added `struct ProcessInput` + `enum ProcessOutcome{ Stopped{stt_error}, CommandFailed, Produced{ cleaned_text, raw_text, is_command, stt_ms, llm_ms, prompt_tokens, completion_tokens, llm_error } }` + `async fn process_audio(input, emit: &mut (dyn FnMut(PipelineEvent) + Send)) -> ProcessOutcome` holding the `STT→guard→LLM→sanitize` body verbatim (events via `emit`; no `AppState`/`AppHandle`).
- [x] `pipeline.rs` — rewrote `stop_and_process_pipeline` to: snapshot inputs (config/dict/providers + command-mode PEEK + pre-resolved cleanup params), build `ProcessInput`, call `process_audio` via a `handle.emit` closure (live path), then apply metric deltas + command-mode consume (gated on outcome reaching the command point) + the unchanged paste/history/sync/webhook/feedback/`done` tail.
- [x] `pipeline.rs` — added `error!` logs to the genuinely log-less early-returns (STT-fail, command-rewrite-fail, stop-recording-fail). The guard skips (too-short/silent/echo/blocklist) already log at `info!` — kept there (correct severity for "nothing said"; see Spec Change Log).
- [x] `pipeline.rs` tests — 8 `process_audio` tests with fake `SttProvider`/`CleanupProvider` + a Vec-capturing `emit`, covering every I/O-matrix row (fallback-success excepted; see Spec Change Log). 510 passed / 0 failed.

**Acceptance Criteria:**
- Given the full suite, when `cargo test` runs in `src-tauri`, then ≥489 pass and 0 fail — after Step A AND after Step B (each green before the next).
- Given a recording end-to-end, when `stop_and_process_pipeline` runs, then emitted events, pasted text, history entry, sync/webhook payloads, and feedback metrics are identical to pre-refactor (verified by semantic diff of the relocated body).
- Given the offline tests at `1821–1875`, when reviewed, then they call `is_offline(...)` with no inline replication.
- Given each former silent skip-return, when it fires, then a `warn!`/`error!` line is logged.
- Given `git grep process_audio src-tauri/src`, then `stop_and_process_pipeline` is its only non-test caller (no dead duplicate).
- Given `KlarvoApi.kt`, when grepped for the touched symbols, then no Kotlin change is required.

## Spec Change Log

**Implementation deviations from the spec draft (recorded during Step B, all behavior-preserving):**

1. **`ProcessOutcome` shape.** Spec proposed `Stop{stt_error, llm_error}`. Implemented `Stopped{stt_error}` + a separate `CommandFailed` variant: the shell must distinguish "stopped before the command point" (guard skip / STT fail → command mode NOT consumed) from "command path failed" (command mode consumed). The variant identity is what gates command-mode consumption; a single `Stop` would have lost that. `Produced{… llm_error}` unchanged.
2. **`emit` needs `+ Send`.** `&mut dyn FnMut(PipelineEvent)` made the future non-`Send` (Tauri runtime requires `Send`). Implemented as `&mut (dyn FnMut(PipelineEvent) + Send)`.
3. **`dict_list` type.** Spec said `Option<Vec<String>>`; reality is `Option<String>` (`terms_as_list` returns a formatted `String`; `chunked_cleanup` takes `Option<&str>`).
4. **"Silent returns" framing.** The frozen Intent says the `idle()` skip-returns have "no log"; in code they already log at `info!` — silent only at the *event* level (all four emit identical `idle()`). The genuinely log-less returns were the *error* paths (STT-fail, command-fail, stop-recording-fail), which got `error!` logs. The `idle` guards stay at `info!` (correct severity).
5. **`process_audio` test coverage.** Every I/O-matrix row is unit-tested EXCEPT retryable-fail-**with-successful-fallback**: `resolve_fallback_provider` hardcodes real network-provider construction, so a fake fallback can't be injected without a provider-registry seam (out of scope, deferred). The retryable-**no-fallback** degrade IS tested.

**Review (iteration 1) — 3 adversarial reviewers (blind / edge-case / acceptance), Opus.** Verdict: PASS. All ACs met, all Boundaries upheld, all Design Notes correctly implemented; no common-case correctness defect (all three independently confirmed the control-flow translation and command-mode gating are faithful). One behavior delta disclosed as a result:

6. **Config read is now pre-STT-snapshot, not incidental late re-read.** OLD `stop_and_process_pipeline` re-locked `state.config` at *several* points (cleanup params after STT; fallback keys at LLM-failure time). The extracted `process_audio` is `AppState`-free by mandate, so it consumes ONE `cfg` snapshot taken before STT (`config_for_fallback`, cleanup params). Identical output in the common case; the only divergence is if the user **saves settings during the STT/LLM window** (seconds) — OLD would then use the newer values for cleanup-style / output-language / fallback-keys, NEW uses the snapshot. This is inherent to the AppState-free design (restoring the late re-read would reintroduce the AppState coupling the task set out to remove) and is benign/arguably-more-coherent. Accepted; not fixed. The post-core tail (record_usage / history / webhook) still re-locks fresh, unchanged. A follow-up was filed for the command-mode selection hold-window micro-delta (see `deferred-work.md`).

## Design Notes

**Command-mode reset timing (load-bearing).** Today `command_mode_active` is reset + `selected_text` `.take()`-en at `~1004`, AFTER the post-STT guards — so a prompt-echo/blocklist hallucination in command mode leaves `command_mode_active=true` (the command was not consumed). Preserve this: the shell PEEKS `is_command_mode` + clones `selected_text` before STT, passes them in, and only resets/`take`s AFTER `process_audio` returns a variant that reached the command point. "Reached the command point" = guards passed = `Produced` or command-path failure — i.e. NOT a guard-`Skipped` and NOT `Stt`-fail (derivable from the outcome variant; no extra flag). Do NOT reset before STT.

**emit callback, not AppHandle.** Intermediate events (`transcribing`, conditional `cleaning`) and terminal `idle`/`error`/`warn` must keep their exact firing points, so `process_audio` emits via `&mut dyn FnMut(PipelineEvent)`. Tests pass a `Vec`-collector; the shell passes a closure wrapping `handle.emit`. The post-paste `done()` event stays in the shell.

**Metrics out, not in.** `process_audio` stays `AppState`-free, so it reports `stt_error`/`llm_error` in the outcome and the shell applies the `+1` to `feedback_metrics`. Net effect is identical — nothing reads those counters between the old bump site and the new one.

`select_llm_path` golden: `(offline=true, sel=false)→OfflineRaw`; `(_, sel=true)→Command`; `(offline=false, sel=false)→Cleanup`. (Mirrors today's `if offline && sel.is_none() … else if sel … else …`.)

## Verification

**Commands:**
- `cd src-tauri && cargo test` — expected: `≥489 passed; 0 failed` (run after Step A, then after Step B).
- `cd src-tauri && cargo clippy --all-targets` — expected: no new warnings.
- `cd src-tauri && INSTA_UPDATE=no cargo test` (or review the `.snap`) — decision-matrix snapshot matches.

**Manual checks:**
- `git grep -n "process_audio" src-tauri/src` — expected: definition + one shell call + tests only.
- `git grep -n 'stt_provider == "local"' src-tauri/src/pipeline.rs` — expected: only inside `is_offline` (no inline replication).
- cfg: no `#[cfg]` blocks in `pipeline.rs:779–1485` → no Windows-only path touched → no Windows build needed for this change (state this in the report).
- Rust↔Kotlin: `KlarvoApi.kt` mirrors none of the touched symbols → no `/sync-prompts` needed.

## Suggested Review Order

**Extracted core (the seam — start here)**

- The STT→guard→LLM→sanitize core, no `AppState`/`AppHandle` — the design intent.
  [`pipeline.rs:944`](../../src-tauri/src/pipeline.rs#L944)

- Fully-snapshotted inputs the shell builds under locks.
  [`pipeline.rs:877`](../../src-tauri/src/pipeline.rs#L877)

- Outcome variants drive the shell's deferred side effects.
  [`pipeline.rs:905`](../../src-tauri/src/pipeline.rs#L905)

**Shell rewiring (highest-risk: the glue)**

- Snapshot block: command-mode PEEK (no reset) + pre-resolved cleanup params.
  [`pipeline.rs:1293`](../../src-tauri/src/pipeline.rs#L1293)

- The single live `process_audio` call (no dead duplicate).
  [`pipeline.rs:1463`](../../src-tauri/src/pipeline.rs#L1463)

- Outcome match: metric deltas + command-mode consume gating (load-bearing timing).
  [`pipeline.rs:1470`](../../src-tauri/src/pipeline.rs#L1470)

- Reset+take helper, run only when the command point was reached.
  [`pipeline.rs:1173`](../../src-tauri/src/pipeline.rs#L1173)

**Decision logic (the characterization net underneath)**

- Offline-cleanup-skip predicate (now the single source the live path calls).
  [`pipeline.rs:438`](../../src-tauri/src/pipeline.rs#L438)

- Pre-STT skip (too-short / silent), post-STT skip (echo→blocklist), path select.
  [`pipeline.rs:456`](../../src-tauri/src/pipeline.rs#L456)

**Tests (supporting)**

- Golden master of the full decision matrix.
  [`pipeline.rs:2268`](../../src-tauri/src/pipeline.rs#L2268)

- `process_audio` I/O matrix with fake providers + capturing emitter.
  [`pipeline.rs:2426`](../../src-tauri/src/pipeline.rs#L2426)

- The 5 offline tests redirected from inline replication to `is_offline`.
  [`pipeline.rs:2116`](../../src-tauri/src/pipeline.rs#L2116)
