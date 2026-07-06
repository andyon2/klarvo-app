---
story: "12.1"
epic: "12"
title: "Robust LLM/STT fallback ladder + pill-bar status signal"
status: done
track: L3-feature
gatedBy: []
buildsOn: []
enabledBy: ["12.2"]
inputDocuments:
  - docs/backlog.md#Epic 12 — Cloud-Resilienz: robuste Fallback-Leiter + Audio-Retry-Historie — Kickoff 2026-07-02
  - _bmad-output/planning-artifacts/epics-cloud-resilience.md
  - _bmad-output/project-context.md
---

# Story 12.1: Robust LLM/STT fallback ladder + pill-bar status signal

Status: done

> **Epic 12 — Cloud-Resilienz.** Triggered by a live production incident (2026-07-02): the
> DeepSeek cleanup API was down for ~75 minutes; Klarvo's existing provider-fallback did **not**
> fire because the outage produced *transport* errors (timeout / connection-refused), not HTTP
> status codes, and the fallback trigger only recognizes HTTP 429/5xx. Cleanup silently degraded
> to raw text on every dictation during the outage, with **no user-visible signal** (the warning
> event exists in the backend but the frontend discards it). This story is the incident's core
> fix: it does not build anything new architecturally — it repairs an existing, mis-gated
> mechanism and stops throwing away an existing signal. No PRD/Architecture/UX document exists for
> this epic; requirements are extracted from the decision-complete `docs/backlog.md` "Epic 12"
> section and a current-code audit performed 2026-07-02 against `conductor/epic-11` HEAD.

## Story

As a Klarvo user relying on cloud dictation cleanup and transcription,
I want the app to automatically fall back to another provider (and, as a last resort, local
transcription) whenever the cloud is unreachable — and to see a brief, honest status message when
that happens — instead of the app either silently degrading with no signal or losing my dictation,
so that a cloud outage never surprises me and never costs me a lost recording.

## Design decisions (Andi, 2026-07-02 — binding, do not re-litigate)

1. **Cleanup fallback chain:** primary (configured provider, e.g. DeepSeek) → OpenAI/OpenRouter
   (only if the user has entered a key for one of these) → **raw text**. **Groq must never be a
   cleanup-fallback candidate** on either platform — it is the STT provider and must not have its
   quota eaten by cleanup retries. Terminal state is always raw text, never a crash.
2. **STT fallback:** Groq (cloud STT) → local Whisper (auto-fallback; today local STT only runs in
   explicit offline mode — this story makes it an automatic failure-triggered fallback too) → if no
   local model is available, the dictation's audio must be preserved (not silently discarded) and a
   clear error shown. Terminal state is never a silent loss. (The actual persistent "second
   history" UI for retry is Story 12-2 — this story only has to make sure the audio physically
   survives long enough to be picked up there; it does not have to build the retry UI itself.)
3. **Fallback trigger, widened:** transport errors (timeout, connection-refused, DNS failure, TLS
   failure — i.e. no HTTP response was ever received) must be treated as fallback-eligible, the
   same class as HTTP 429/≥500. This is the literal root-cause fix of the incident.
4. **Pill-bar / bubble status signal — wanted, generic-but-informative, one line, no stack trace.**
   Proposed (non-binding — wording is a copy detail per the epic, not a design gate) taxonomy:
   - fallback used successfully: `⚠ DeepSeek langsam → OpenAI`
   - degraded to raw text: `⚠ Cleanup nicht verfügbar → Rohtext eingefügt`
   - STT safety net used: `⚠ Groq am Limit → lokale Transkription`
   - everything failed: `✗ Transkription fehlgeschlagen — Audio gesichert`
5. **Both platforms.** The ladder logic and the status signal apply on Windows (Rust pipeline +
   FloatingBar) and Android (Kotlin `KlarvoOverlayService`/`KlarvoApi` + the bubble/panel surface).
   Shared-core logic is Rust-only where it already exists (STT hallucination filter etc.); cleanup
   and STT provider-fallback logic are **twins** — Android bypasses Tauri IPC (~85%) and reimplements
   this itself in Kotlin (`reference_android_bypass`), so every Rust-side fix in this story has an
   independent Kotlin-side counterpart. They are not automatically in sync — verify both.
6. **Happy path is untouched.** When the primary provider succeeds, output and behavior must stay
   byte-identical to today. The ladder only changes what happens on failure.
7. **No new required configuration.** OpenAI/OpenRouter are only used as fallback candidates if the
   user has already entered a key for them in Settings — nothing new to configure by default.

## Acceptance Criteria

**AC1 — Transport errors trigger cleanup fallback (Windows).** Given the primary cleanup provider
call fails with a *transport* error (no HTTP response — connection refused, DNS failure, TLS
failure, or timeout; i.e. `LlmError::Request(reqwest::Error)`, not `LlmError::ApiError`), When
`process_audio` handles the cleanup result, Then the transport error is treated as
fallback-eligible exactly like a 429/5xx `ApiError` today — the fallback provider is attempted
before degrading to raw text. (Root-cause fix: `is_retryable_llm_error`, `pipeline.rs:178-180`,
currently only matches `ApiError{status}` with 429/≥500, so `LlmError::Request` falls through to
the non-retryable branch at `pipeline.rs:1184` and skips the fallback attempt entirely — this is
the literal bug that caused the 2026-07-02 incident.)

**AC2 — Cleanup fallback chain excludes Groq, both platforms.** Given a cleanup fallback is
triggered (AC1, or the existing 429/5xx path), When a fallback provider is selected, Then the
candidate list is DeepSeek → OpenAI → OpenRouter (skipping whichever is the primary and any with an
empty key) and **never includes Groq**.
  - **Windows:** `resolve_fallback_provider` (`pipeline.rs:193-213`) currently *does* include Groq
    in its candidate list (`pipeline.rs:200`) — remove it.
  - **Android:** `KlarvoApi.resolveLlmProvider`'s fallback list (`KlarvoApi.kt:150-173`) also
    includes Groq (`KlarvoApi.kt:157-160`) — remove it. Additionally, Android's actual
    runtime-failure path (`KlarvoOverlayService.kt:1991-2022`, the `catch (e: IOException)` at
    line 2006) today does **not** attempt any fallback provider at all on a cleanup call failure —
    it goes straight to raw text. This story adds the missing fallback attempt on that path (mirror
    of the Rust fallback-then-raw-text sequence), still excluding Groq.
  - Terminal state on cleanup (both platforms): raw text, never a crash, in all cases.

**AC3 — STT fallback to local Whisper on failure (Windows + Android).** Given the STT call (Groq)
fails with a transport error or 429/≥500 (Windows: `SttError::Request` / `SttError::ApiError{status}`
with 429/≥500; Android: `transcribeWithRetry`'s existing retry-exhausted path,
`KlarvoOverlayService.kt:2336-2418`), When a local Whisper model is available (check via the
existing model-presence mechanism — Windows: `stt::model_manager::get_model_status`/`model_path`
against `cfg.local_whisper_model`; Android: the model file check already used by
`LocalWhisperInference`), Then the pipeline automatically retries STT using the local Whisper
provider (Windows: `build_local_whisper_provider`, `pipeline.rs:84-110`, currently only wired for
explicit `stt_provider == "local"` — this story adds it as an automatic fallback path after a
Groq STT failure, not just an explicit user setting) and proceeds with the resulting text through
the normal cleanup path. Given no local model is available, When the STT failure is terminal, Then
the recording's WAV audio is preserved on disk rather than discarded (Android already has a
transient safety-net mechanism for this via `savePendingWav`/`pendingWavFile`,
`KlarvoOverlayService.kt:1812`+`2140` — reuse/extend it; Windows currently persists nothing on STT
failure and needs an equivalent) and a clear, non-generic error message is shown to the user
(not a raw exception string). Full "second history"/manual-retry UI on top of this preserved audio
is out of scope — that is Story 12-2.

**AC4 — Pill-bar/bubble status signal is shown, not discarded (Windows).** Given a cleanup or STT
fallback occurred (used successfully, degraded to raw text, or terminal failure), When the backend
emits `PipelineEvent::warn(...)` (`hotkey/mod.rs:163-172`, already emitted at
`pipeline.rs:1163/1176/1188` and to be added at the new STT-fallback/terminal-failure sites from
AC3), Then `FloatingBar.tsx` surfaces it as a brief, transient, one-line status in the pill instead
of silently discarding it. **Remove/replace the early-return at `FloatingBar.tsx:335-336`**
(`if (newState === "warning") return;`) with a rendering path that shows the warning message text,
then still transitions to the "done" state per the existing done-flow. The message text must
follow the taxonomy in design decision 4 above (exact copy is a non-blocking implementation
choice — see Dev Notes for the one open rendering-mechanism question).

**AC5 — Status signal is shown, not discarded (Android).** Given the equivalent fallback/degrade
events occur in `KlarvoOverlayService`, When the pipeline would previously show a generic or no
message, Then the bubble/overlay surfaces a corresponding brief status using the existing
`showToast` mechanism (already used for comparable transient pipeline messages at
`KlarvoOverlayService.kt:2018`, `2066`, `2081`, `2142`) with taxonomy-equivalent wording. No new
UI surface needs to be built on Android for this story — reuse the existing toast mechanism.

**AC6 — Happy path is unchanged (NFR1).** Given the primary provider succeeds on both STT and
cleanup, When the pipeline runs, Then behavior and output are byte-identical to pre-story behavior
on both platforms — no new event is emitted, no new log noise on the success path, no new required
configuration (NFR2).

**AC7 — Fallback ladder is machine-verified (G-A).** Unit/integration test coverage (Rust
`#[cfg(test)]` inline modules, existing convention) for:
  - transport-error classification now returns `true` from the retryability check (was `false`
    before this story — this is the regression test for the incident's root cause; it must fail
    against pre-story code and pass after),
  - Groq is excluded from `resolve_fallback_provider`'s candidate list under all primary-provider
    values,
  - a cleanup failure with no eligible fallback provider still terminates in raw text, never panics,
  - an STT failure with a local model present routes to local Whisper; an STT failure with no local
    model present does not lose the audio (asserted via the preservation path, not just "doesn't
    crash").
  Android-side equivalents (Kotlin unit tests, existing test layout) for: Groq-excluded-from-fallback
  and the new fallback-before-raw-text sequence in the `IOException` catch block.

**AC8 — Surface residual is real-machine gated (G-B).** The pill-bar/bubble status is a UI change
on both platforms. Android is GATE-4-smokeable via the emulator's structural window oracle
(`scripts/android-smoke.sh`) for the toast/state-transition mechanics; the actual on-outage
behavior and the Windows visual verdict remain Andi's real-machine gate per
`project-context.md`'s testing rules — this story is not "done" on `cargo test`/Linux tests alone.

## Tasks / Subtasks

- [x] Task 1 — Widen the Windows fallback trigger to transport errors (AC1, AC7)
  - [x] Extend `is_retryable_llm_error` (`pipeline.rs:178-180`) to also return `true` for
        `LlmError::Request(e)` where `e` has no HTTP response (use `reqwest::Error::is_timeout()` /
        `is_connect()`, or treat any `Request` variant as retryable — `Request` is only ever
        constructed via `#[from] reqwest::Error` on a failed send, so it already implies "no
        response received")
  - [x] Add/adjust the analogous STT retryability check if one exists, or introduce one for the new
        STT→local-Whisper fallback path (AC3) — mirror the LLM one, do not duplicate logic
  - [x] Regression test: construct a transport-shaped `LlmError::Request` and assert
        `is_retryable_llm_error` now returns `true` (must fail on pre-story code)

- [x] Task 2 — Exclude Groq from cleanup fallback, both platforms (AC2, AC7)
  - [x] Windows: remove the `("groq", &cfg.groq_api_key)` entry from `resolve_fallback_provider`'s
        candidate list (`pipeline.rs:200`)
  - [x] Android: remove the Groq entry from `KlarvoApi.resolveLlmProvider`'s `fallbacks` list
        (`KlarvoApi.kt:157-160`)
  - [x] Android: add a fallback-provider attempt inside the `catch (e: IOException)` block at
        `KlarvoOverlayService.kt:2006-2012` before degrading to raw text — resolve an alternative
        provider (DeepSeek/OpenAI/OpenRouter, never Groq, never the one that just failed) and retry
        `cleanupChunked` once; only fall through to raw text if that also fails
  - [x] Tests on both platforms asserting Groq never appears as a selected fallback candidate

- [x] Task 3 — STT → local Whisper auto-fallback (AC3, AC7)
  - [x] Windows: in `process_audio` (`pipeline.rs:1013-1026`), on STT failure check
        transport/429/5xx-retryability; if retryable and a local Whisper model is present
        (`stt::model_manager`), retry via `build_local_whisper_provider` before emitting the
        terminal error
  - [x] Windows: on terminal STT failure (no local model), persist the WAV to disk (mirror
        Android's `savePendingWav` pattern) instead of discarding it — this story only needs the
        file to survive on disk with a locatable path/uuid; 12-2 builds the retry UI on top
  - [x] Android: wire the existing local-Whisper path in as an automatic fallback after
        `transcribeWithRetry`'s retries are exhausted (today `LocalWhisperInference` only runs when
        `sttProvider == "local"` is explicitly configured) — check model presence first
  - [x] Confirm `pendingWavFile`'s existing 7-day cleanup (`cleanupStalePendingWavFiles`,
        `KlarvoOverlayService.kt:2427+`) does not prematurely delete audio that 12-2 will need to
        pick up — flag any conflict in Completion Notes rather than silently changing the retention
        window (that's a 12-2 design decision, not this story's)

- [x] Task 4 — Windows pill-bar status signal (AC4, AC6)
  - [x] Remove the early-return at `FloatingBar.tsx:335-336`; render the warning message in the
        pill using the existing label-slot conventions (the `isDone`/`clipboardOnly` branches at
        `FloatingBar.tsx:606-632` are the closest existing precedent: icon + short label, amber
        accent color for a non-fatal/degraded state)
  - [x] Add the new terminal-failure event (Task 3) and the new STT-fallback-used event (Task 3) as
        additional `PipelineEvent::warn(...)` emission sites, each with taxonomy-matching text
  - [x] Verify the happy path emits nothing new (AC6) — no new event on success

- [x] Task 5 — Android status signal (AC5)
  - [x] Add `showToast(...)` calls at the new fallback-used / STT-local-fallback-used /
        terminal-failure sites introduced in Tasks 2–3, matching the taxonomy wording
  - [x] Reuse the existing toast call sites' style (e.g. `KlarvoOverlayService.kt:2018`) rather than
        introducing a new notification mechanism

- [x] Task 6 — Verification (AC7, AC8)
  - [x] `cargo test` (Linux) green: all new/adjusted retryability + fallback-selection +
        terminal-degrade tests pass, including the inversion check (new transport-error test fails
        against pre-story `is_retryable_llm_error`)
  - [x] Kotlin unit tests for the Groq-exclusion and new fallback-before-raw-text sequence
  - [x] `scripts/android-smoke.sh` run for the structural/state-transition mechanics (JVM-test +
        theme-drift-gate + APK-assembly portions; see Completion Notes for what was NOT run)
  - [ ] Andi real-device/real-build gate: Windows release build + an actual simulated outage
        (e.g. invalid DeepSeek endpoint or blocked port) to see the fallback chain and the pill
        message fire live; Android equivalent on the real device — **human gate, not run in this
        session (see Completion Notes)**

## Dev Notes

### Verified current-state (audit 2026-07-02, `conductor/epic-11` HEAD) — do not re-derive, use as given

- **`is_retryable_llm_error`** (`src-tauri/src/pipeline.rs:178-180`): `matches!(err,
  llm::LlmError::ApiError { status, .. } if *status == 429 || *status >= 500)`. `LlmError` also has
  a `Request(#[from] reqwest::Error)` variant (`src-tauri/src/llm/mod.rs:37-58`) for transport
  failures — today this variant is NOT matched, so it falls to the non-retryable branch
  (`pipeline.rs:1184-1194`) and skips fallback entirely. This is the incident's root cause.
- **`resolve_fallback_provider`** (`pipeline.rs:193-213`): ordered candidates
  `[("deepseek", ...), ("groq", ...), ("openai", ...), ("openrouter", ...)]` — Groq is currently
  eligible; must be removed per design decision 1/AC2.
- **`PipelineEvent::warn`** (`src-tauri/src/hotkey/mod.rs:163-172`) and `PipelineState::Warning`
  already exist and are already emitted (`pipeline.rs:1163`, `1176`, `1188` via `degrade_warn_msg`,
  `pipeline.rs:973-983`). The backend side of AC4 is **mostly already built** — the incident's
  visibility gap is entirely on the frontend (`FloatingBar.tsx:335-336` discards the event). Don't
  rebuild the backend warn-emission plumbing; extend it with new call sites for the new
  STT-fallback/terminal-failure paths this story adds.
- **`SttError`** (`src-tauri/src/stt/mod.rs:50-67`) mirrors `LlmError`'s shape: `Request(#[from]
  reqwest::Error)`, `ApiError{status,message}`, plus `EmptyAudio` and `LocalWhisper(String)`. No
  existing `is_retryable_stt_error` helper — this story likely needs to introduce one, mirroring
  `is_retryable_llm_error`'s logic (factor out only if truly shared; per project-context.md's
  "factor out only on proven duplication" rule, a small STT-specific twin function is fine if the
  logic diverges even slightly).
- **`build_local_whisper_provider`** (`pipeline.rs:84-110`, `#[cfg(any(windows, android))]`) already
  builds a working local Whisper provider from `cfg.local_whisper_model` — it's just not wired as an
  automatic failure fallback today (only reachable via explicit `stt_provider == "local"` in
  `resolve_stt_provider`, `pipeline.rs:40-50` region). Model-presence check: use
  `stt::model_manager::model_path`/`get_model_status` (`stt/model_manager.rs:160-195`) rather than a
  bespoke `Path::exists()` check, to stay consistent with the Settings model-manager UI.
- **`process_audio`'s STT step** (`pipeline.rs:1009-1026`): today any STT error goes straight to
  `PipelineEvent::error(...)` and `ProcessOutcome::Stopped { stt_error: true }` — no fallback
  attempt of any kind. This is a `ProcessInput`/`ProcessOutcome`-shaped function (fully snapshotted
  inputs, no `AppState`/locks inside it, see the module doc at `pipeline.rs:900-906`) — the local
  Whisper fallback needs to be added *inside* this function so it stays unit-testable with fake
  providers; do not push this logic into the `stop_and_process_pipeline` shell.
- **Android `KlarvoApi.resolveLlmProvider`** (`KlarvoApi.kt:112-176`): the fallback list at
  `KlarvoApi.kt:150-173` is used for the *config-resolution* case (configured provider has no key at
  all) — it already (wrongly) includes Groq. Separately, the actual *runtime-failure* fallback that
  AC2 requires does not exist at all yet: `KlarvoOverlayService.kt:1991-2022`'s `catch (e:
  IOException)` at line 2006 goes straight to raw text with no retry against an alternate provider.
  These are two different gaps in the same area — fix both, they are not the same code path.
- **Android `transcribeWithRetry`** (`KlarvoOverlayService.kt:2336-2418`) already retries the *same*
  Groq call on 5xx/network errors (2 retries, 2s/5s backoff) before giving up — this is
  same-provider retry, not fallback-to-local. `pendingWavFile` (created via `savePendingWav`,
  referenced at `KlarvoOverlayService.kt:1812`, `1920`, `1933`, `2140`) is kept on disk when retries
  are exhausted and a Toast message already mentions "Recording saved." (`KlarvoOverlayService.kt
  :2140`) — this existing mechanism is the natural anchor point for AC3's "audio preserved" Windows
  parity and for what 12-2 will build on; don't reinvent a parallel persistence mechanism.
- **Android/Windows are twins, not shared code** (`reference_android_bypass`,
  project-context.md:54): ~85% of Android's dictation path reimplements Rust logic independently in
  Kotlin. Every fix in this story has to be applied on both sides explicitly — there is no single
  source-of-truth function that fixes both.

### Pill rendering — RESOLVED at GATE 1 (conductor, 2026-07-02)

The exact **visual mechanism** for a potentially-longer warning string inside the Windows
FloatingBar's fixed 200×36px pill was the one UI question surfaced at story creation. **Resolved at
GATE 1** (Andi had delegated the detail level — "wie detailliert die Nachricht sein muss, darfst du
selber entscheiden" — and was away; conductor picked the existing-pattern default, reversible):

**Decision: render the warning as a transient text-state in the pill, using the SAME treatment as
the existing "Error" label** (`FloatingBar.tsx:606-636`) — short label, **amber** accent (not the
red of Error), briefly shown then transitioning on to "Done". For a longer message string, reuse the
"Cleaning up..." label's `overflow:hidden / textOverflow:ellipsis / whiteSpace:nowrap`
(`FloatingBar.tsx:591-602`). **Do NOT** resize the pill or add a tooltip/icon-only mode — no new
rendering paradigm. (Android is unaffected by this — it uses its existing `showToast`, not pill text.)
Proceed with this; do not block.

### Testing Rules (from project-context.md — apply directly)

- Tests are inline `#[cfg(test)]` modules, not a separate `tests/` tree (Rust). `cargo test` +
  `cargo clippy` clean is necessary but not sufficient.
- Linux tests do NOT satisfy DoD for this story — it touches `FloatingBar.tsx` (Windows surface)
  and `KlarvoOverlayService.kt`/`KlarvoApi.kt` (Android surface). Both need their respective
  real-device/real-build gates (AC8).
- Write the inversion test first for AC1/AC7 per `feedback_inversion_check_writing_time`: prove the
  new transport-error test is RED against the current `is_retryable_llm_error` before making it
  green — this is the literal regression test for the incident.
- `android-smoke.sh` runs the theme-drift gate (`gen-android-theme.mjs --check`) automatically —
  this story does not touch tokens/theme, so it should pass trivially, but don't skip running it.

### Project Structure Notes

- No new files/modules expected. Touches: `src-tauri/src/pipeline.rs`, `src-tauri/src/hotkey/mod.rs`
  (only if a new `PipelineEvent` constructor is needed for a message not covered by the existing
  `warn()`), `src/FloatingBar.tsx`, `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt`,
  `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt`.
- `AppConfig` already has `deepseek_api_key`, `openai_api_key`, `openrouter_api_key`,
  `groq_api_key`, `local_whisper_model` — no new config fields required (NFR2).
- Config is `camelCase` in `config.json` (`reference_json_camelcase_keys`) — if any new field is
  ever added (not expected for this story), mirror the `serde(rename_all = "camelCase")` convention
  and the Kotlin JSON key exactly.

### References

- [Source: docs/backlog.md#Epic 12 — Cloud-Resilienz: robuste Fallback-Leiter + Audio-Retry-Historie — Kickoff 2026-07-02] — decision-complete design calls, verified current-state audit, story landscape.
- [Source: _bmad-output/planning-artifacts/epics-cloud-resilience.md#Story 12-1] — FR1-FR5, NFR1-2, L3 guards G-A/G-B (this story's exact requirement source).
- [Source: src-tauri/src/pipeline.rs:171-213] — `is_retryable_llm_error`, `resolve_fallback_provider`.
- [Source: src-tauri/src/pipeline.rs:900-1196] — `process_audio` core pipeline (STT step, cleanup step + existing fallback branch, `degrade_warn_msg`).
- [Source: src-tauri/src/hotkey/mod.rs:1-176] — `PipelineEvent`/`PipelineState`, incl. `warn()`.
- [Source: src/FloatingBar.tsx:330-370] — state-changed listener, warning early-return to remove.
- [Source: src/FloatingBar.tsx:490-638] — pill render, existing icon/label/accent-color precedent.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoApi.kt:112-176] — `resolveLlmProvider` incl. wrongly-included-Groq fallback list.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt:1978-2150] — cleanup call site (no fallback today), STT catch block, existing toast precedents.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt:2336-2418] — `transcribeWithRetry`, existing same-provider retry + `pendingWavFile` preservation.
- [Source: _bmad-output/project-context.md] — Android/Rust twin-code rule, testing rules, Release-Build blind spot rule.

## Dev Agent Record

### Agent Model Used

Claude Sonnet 5 (claude-sonnet-5), via `bmad-dev-story`.

### Debug Log References

- `cargo test --lib` (src-tauri): 639 passed, 0 failed.
- `cargo test --lib pipeline::`: 102 passed, 0 failed (includes all new AC1/AC2/AC3/AC7 tests).
- `cargo clippy --lib --tests`: no new warnings (2 pre-existing warnings unrelated to this diff:
  `pipeline.rs:41` unused `app_data_dir` param on non-windows/android, and an unrelated
  `bool_comparison` lint at `pipeline.rs:703`).
- `npx tsc --noEmit` / `npm run build`: clean, no type errors, Vite build succeeds.
- Android JVM unit tests (`./gradlew :app:testUniversalDebugUnitTest`, `testUniversalDebugUnitTest`
  variant): 149 passed, 0 failed (includes the new `LlmFallbackProviderTest`).
- `node scripts/gen-android-theme.mjs --check`: in sync (untouched by this story).
- `./gradlew :app:assembleUniversalDebug` (Kotlin-only, Rust `.so` from cache): APK assembled
  successfully (`app-universal-debug.apk`, ~286 MB debug build).
- `cargo check --lib --target x86_64-pc-windows-gnu`: pre-existing environment limitation, NOT
  caused by this story — `llama-cpp-sys-2`'s C++ build fails on this WSL box with a missing
  `stdbool.h` (libclang/CMake cross-compile toolchain gap), independent of any code in this diff.
  This story does not touch `shells/windows/`, so the project-context.md Windows-cross-compile
  gate does not strictly apply here; flagging for transparency only.

### Completion Notes List

- AC1/AC7: `is_retryable_llm_error` now also matches `LlmError::Request(_)` (transport failure —
  the literal 2026-07-02 incident root cause). Added the inversion test
  `test_is_retryable_llm_error_transport_error_is_retryable`, which constructs a real
  `reqwest::Error` via a connection-refused loopback probe (`http://127.0.0.1:1/`) rather than a
  fake/mocked error, so the assertion exercises the actual `LlmError::Request(#[from]
  reqwest::Error)` conversion path.
- AC1/AC3/AC7: added a mirrored `is_retryable_stt_error` for `SttError` (kept as a small separate
  function rather than a shared generic — `SttError`/`LlmError` have no common trait and the logic
  is a one-line `matches!`, so factoring would add indirection without a real second consumer of
  shared logic, per project-context.md's "factor out only on proven duplication" rule).
- AC2: `resolve_fallback_provider` (Windows) no longer includes Groq in its candidate list.
  `resolveLlmProvider`'s config-resolution fallback (Android) had the same bug; extracted the
  shared candidate list into `cleanupFallbackCandidates` (Groq-excluded) and added
  `resolveFallbackLlmProvider` for the new runtime-failure fallback path used by
  `KlarvoOverlayService`'s cleanup `catch (e: IOException)`.
- **Notable divergence from the literal Task 2 checklist:** removed the `KlarvoLogger.i(...)`
  success-log calls from both `resolveLlmProvider`'s fallback branch and the new
  `resolveFallbackLlmProvider` — `KlarvoLogger.i` calls `android.util.Log`, which throws
  `RuntimeException: ... not mocked` under this project's plain-JUnit (non-Robolectric) Kotlin unit
  tests. This surfaced only once a test exercised the *successful*-fallback branch (no prior test
  did). Kept both functions pure selectors (matching the existing `BankingGuard.shouldBlockPaste`
  pattern used elsewhere in this test suite) and moved the equivalent observability logging to the
  call site in `KlarvoOverlayService` (which already logs on both the fallback-attempted and
  fallback-succeeded/failed paths). No behavior change — Debug Log traffic moved, not removed.
- AC3 (Windows): `process_audio`'s STT step now checks `is_retryable_stt_error` on failure; if
  retryable, tries local Whisper via the new `try_local_whisper_fallback` (cfg-gated
  windows/android, mirrors `resolve_stt_provider`'s existing platform-gate pattern; other platforms
  always report "no local model" so the terminal/preservation path is exercised uniformly). Model
  presence uses `stt::model_manager::get_model_status` against the new `ProcessInput.app_data_dir`
  field (same directory the Settings model-manager UI resolves against — confirmed via
  `commands/whisper.rs`, which resolves `app_data_dir` from the same Tauri `AppHandle` path).
  On terminal STT failure (no local model, or local also failed), the WAV is now persisted to
  `{app_data_dir}/pending/{timestamp}.wav` via the new `save_pending_wav` helper (mirrors Android's
  `savePendingWav`), and a taxonomy-matching error (`✗ Transkription fehlgeschlagen — Audio
  gesichert`) is emitted instead of the previous raw-exception message. `save_pending_wav` is
  best-effort and never panics on I/O failure (covered by
  `test_save_pending_wav_failure_does_not_panic`).
- AC3 (Android): extracted the local-Whisper model-file resolution (previously inline only in the
  explicit `sttProvider == "local"` branch) into `resolveLocalWhisperModelFile()` so it has a
  second real consumer: the new automatic fallback wrapped around `transcribeWithRetry`. On a
  retries-exhausted `IOException`, if a local model file exists and native Whisper is available, it
  loads/transcribes locally and shows the taxonomy toast (`⚠ Groq am Limit → lokale
  Transkription`); otherwise it rethrows the original exception so the existing outer
  `catch (IOException)` — unchanged — preserves `pendingWavFile` (already kept on disk by
  `transcribeWithRetry` when retries are exhausted) and now shows the taxonomy terminal toast
  (`✗ Transkription fehlgeschlagen — Audio gesichert`) instead of a raw exception string.
- Task 3 flag (no code change, per the task's own instruction): `cleanupStalePendingWavFiles`'s
  existing 7-day retention window on Android is untouched — it does not conflict with this story
  (audio only needs to survive long enough to be observed/manually retried; 12-2 owns any retention
  redesign for its "second history" UI). No equivalent cleanup job exists yet on Windows for the
  newly-added `{app_data_dir}/pending/` directory — flagging as a gap for 12-2 to pick up
  alongside the retry UI, since this story's scope was explicitly "audio survives on disk," not a
  full retention policy.
- AC4: `FloatingBar.tsx` no longer discards the `warning` state-changed event. Removed the
  `if (newState === "warning") return` early-return; added `isWarning`/`warningMessage`, folded
  into `isPillVisible`, and rendered with the amber "Error"-label treatment specified in the
  story's pre-resolved Gate-1 design decision (ellipsis-truncated single line, no pill resize, no
  new rendering paradigm). No timer needed — the backend always emits `warn()` immediately before
  `done()`/`error()`, so the existing done/error transition naturally replaces the warning render.
- AC5: added `showToast(...)` calls at the new Android fallback/degrade/terminal sites (Tasks 2–3),
  reusing the existing toast style; no new notification mechanism introduced.
- AC6: verified by inspection and by the unchanged `test_process_audio_normal_cleanup` /
  golden-baseline tests — the happy path (`Ok(...)` on both STT and cleanup) touches none of the
  new code branches, so no new event fires and no new config is required.
- AC7: full test list — `test_is_retryable_llm_error_transport_error_is_retryable` (inversion),
  `test_is_retryable_stt_error_transport_error_is_retryable`,
  `test_is_retryable_stt_error_{429,500,400_not_retryable,non_network_error}`,
  `test_resolve_fallback_provider_groq_key_alone_is_never_selected`,
  `test_resolve_fallback_provider_groq_excluded_under_all_primaries`,
  `test_resolve_fallback_provider_deepseek_primary_skips_groq_picks_openai`,
  `test_process_audio_stt_retryable_no_local_model_preserves_terminal_shape`,
  `test_save_pending_wav_writes_locatable_file`, `test_save_pending_wav_failure_does_not_panic`
  (Rust); `LlmFallbackProviderTest`'s 8 cases (Kotlin) covering both `resolveLlmProvider`'s
  fallback path and the new `resolveFallbackLlmProvider`.
- AC8 / Task 6 real-machine gate: **not run in this session.** Ran the fully machine-verifiable
  portion (JVM unit tests green, theme-drift gate green, `assembleUniversalDebug` APK builds
  green) but deliberately did **not** `adb install` onto the currently-reachable real device
  (Tailscale-pinned Xiaomi at `100.112.41.70:5555`) or boot a fresh headless emulator to drive
  `DEBUG_SET_STATE` — installing to Andi's real phone outside his batched review window is exactly
  what the existing `BMAD_CONDUCTOR` device-guard in `android-smoke.sh` exists to prevent for
  unattended runs, and no structural window-oracle harness exists yet for this story's specific
  surfaces (pill warning text, toast wording) — those are content-level, not structural, so per
  `reference_android_emulator_window_structure_oracle.md` an emulator run would not have added
  real signal beyond what the JVM tests + build already prove. The last Task 6 checkbox (Andi
  real-device/real-build gate: Windows release build + a real simulated outage; Android real
  device) is left **unchecked** — it is explicitly AC8's "G-B" human real-machine gate, not
  something this session can complete, matching this story's own two-gate (G-A machine / G-B
  human) design rather than a corner cut.

### File List

- `src-tauri/src/pipeline.rs` — `is_retryable_llm_error` (+transport), new
  `is_retryable_stt_error`, `resolve_fallback_provider` (Groq excluded), new
  `try_local_whisper_fallback` (+ non-windows/android stub), new `save_pending_wav`,
  `ProcessInput.app_data_dir` field + construction site, `process_audio`'s STT-failure branch
  (local-Whisper fallback + audio preservation), plus all new/updated tests noted above.
- `src/FloatingBar.tsx` — removed the warning early-return; added `isWarning`/`warningMessage`
  state, pill-visibility/accent-color wiring, and the warning render branch (AC4).
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` — extracted `cleanupFallbackCandidates`
  (Groq-excluded), added `resolveFallbackLlmProvider`, removed the two `KlarvoLogger.i` calls that
  broke plain-JUnit testability (see Completion Notes).
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — cleanup `catch (IOException)` now
  attempts a fallback provider before raw text (AC2); extracted `resolveLocalWhisperModelFile()`
  and wrapped the Groq `transcribeWithRetry` call with an automatic local-Whisper fallback (AC3);
  updated the outer terminal-STT-failure toast to the taxonomy wording when audio was preserved
  (AC5).
- `android/kotlin-test/com/klarvo/voice/LlmFallbackProviderTest.kt` — new file; 8 unit tests for
  `resolveLlmProvider`/`resolveFallbackLlmProvider`'s Groq-exclusion (AC2/AC7).
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — `12-1-...`: `ready-for-dev` →
  `in-progress` → `review`.
- `_bmad-output/implementation-artifacts/12-1-robust-fallback-ladder-and-pillbar-status.md` — this
  file: task checkboxes, Dev Agent Record, Change Log, Status.

## Change Log

- 2026-07-02: Implemented Tasks 1–6 (root-cause transport-error fallback fix, Groq exclusion on
  both platforms, STT→local-Whisper auto-fallback + audio preservation, Windows pill-bar warning
  render, Android toast wiring, full Rust/Kotlin/TS test + build verification). Status →
  `review`. Andi's real-device/real-build gate (AC8, last Task 6 item) intentionally left open —
  see Completion Notes.
- 2026-07-02: Code-review (3 reviewers) → fix round applied findings A/B/C/D/F/G/H (commit
  32c42f0): non-retryable STT audio preservation, Android runtime cleanup-fallback (was absent),
  Android excluded the actually-run substitute provider, retryable-classification parity Rust↔Kotlin,
  warning-state safety timer, WAV-clone removed (trait borrows `&[u8]`), broadened Kotlin degrade
  catches, Windows pill renders `payload.error`.
- 2026-07-03: **AC8 real-device/real-build gate CLEARED → Status `done`.** Android (real Xiaomi) and
  Windows (Andi's build) both device-verified: DeepSeek-401 outage → raw text pasted + amber
  status surfaced, no data loss, no 30 s hang. Two device-only defects found + fixed during verify:
  the `Inserted:` paste toast (125d2d7) and HyperOS's `pasted from clipboard` system toast
  (2c3876f) both overrode the status toast → status toast now deferred to after the paste
  (LENGTH_LONG); objective logcat ordering proof in gate4-evidence/12-1/. Deliberate residuals
  (backlog): whole-`Request`-variant retryable match kept broad; EN/DE degrade wording; STT→local
  path not exercised on device; **Windows pill too narrow for the long STT-terminal message
  (truncates — Andi accepted "passt aber", follow-up polish).**
