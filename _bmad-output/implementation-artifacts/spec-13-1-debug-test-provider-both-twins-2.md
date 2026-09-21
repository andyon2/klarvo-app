---
title: '13-1 Debug test provider, both twins'
type: 'feature'
created: '2026-09-20'
status: 'done'
baseline_revision: '11f83818d498ba0ca6f5651e7fd7245f529f2e7e'
route: 'full'
route_source: 'auto'
review: 'thorough'
review_source: 'auto'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
followup_review_recommended: true
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-13-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/spec-13-1-debug-test-provider-both-twins.md'
  - '{project-root}/_bmad-output/implementation-artifacts/13-1-attempted-implementation.patch'
  - '{project-root}/docs/adr/0016-android-path-parity-strategy.md'
  - '{project-root}/docs/adr/0017-shared-core-stt-path.md'
  - '{project-root}/test-fixtures/README.md'
warnings: ['oversized']
deferred:
  - summary: "groq_jni's own test module has never executed, and one of its assertions is wrong"
    evidence: '`src-tauri/src/stt/groq_jni.rs` carried `#![cfg(target_os = "android")]` and was declared `#[cfg(target_os = "android")] pub mod groq_jni;`, so its `#[cfg(test)] mod tests` never compiled on the platform `cargo test --lib` runs on and never ran on Android either. Story 13-1 ungated the module''s non-JNI half; ungating the test block too turns `test_panic_safety_is_hallucination_unusual_inputs` red, because `is_hallucination("\0")` is false. That is a never-executed expectation from story 7-3, not a product defect. The test block was therefore left android-gated (status quo) with a comment naming the trap.'
    location: 'src-tauri/src/stt/groq_jni.rs::tests'
    severity: 'low'
    origin: 'pre-existing'
  - summary: >-
      No device-free gate checks that the Kotlin and Rust halves of the 9-parameter
      `nativeTranscribe` JNI signature still agree in arity and order.
    evidence: |-
      `GroqSttBridge.nativeTranscribe` and `Java_com_klarvo_voice_GroqSttBridge_nativeTranscribe`
      both grew `sttProvider` + `debugSttScenario` in commit `e0da32a`. Nothing compares them:
      the JVM gate never loads the `.so`, the `extern "system"` entry points stay
      `#[cfg(target_os = "android")]` so `cargo test --lib` never builds them, and
      `Adr0017BoundaryGuardTest` is a Kotlin-only source-text tripwire whose own KDoc excludes
      the Rust side. Because `#[no_mangle]` exports the short JNI name with no signature suffix,
      a one-sided edit (or a stale `.so`) misbinds silently rather than throwing: the scenario
      string lands in `provider_name`, `select_stt_provider` sees `"ok" != "debug"`, and the
      STT half of the enabler degrades to a real Groq call with no error and no log line.
      The gap class predates this story -- that boundary has never had a gate -- and closing it
      needs an adb/emulator step that calls `nativeTranscribe` once after a rebuild, which is
      larger than this story. The manual check already named in `## Verification`
      (`scripts/android-build.sh`, not `scripts/android-smoke.sh`) covers this run.
    location: >-
      android/kotlin-src/com/klarvo/voice/GroqSttBridge.kt:59 + src-tauri/src/stt/groq_jni.rs:202
    severity: 'medium'
    origin: 'pre-existing gap class, widened by this story'
---

<intent-contract>

## Intent

**Problem:** Four rows of drift audit #2 (D2/D-H19 empty LLM answer, D3/D-M16 truncated answer,
D9/D-M5+M6 empty STT result, D10/D-M2 malformed answer) are agent-only today: Andi cannot provoke a
misbehaving provider on his Windows machine or his Xiaomi without standing up a fake API. Story 13-2
needs those states reproducible before it changes the guards (ADR-0016 Amd 4, H+).

**Approach:** A `debug` provider value for LLM cleanup and for STT in both twins that returns a
**canned wire response** (HTTP status + body) chosen from `advanced.debug*Scenario`, fed through each
twin's own existing response-mapping code. Settings → Advanced → System, behind the existing
`expertMode` gate, gains **four** `KSelect` rows: two that select the provider (`llmProvider`,
`sttProvider`, each offering `debug`) and two that select the scenario. Never a default, never a
fallback candidate, never an option in the *normal* provider picker.

## Boundaries & Constraints

**Always:**
- **Canned at the wire, and the twin's own real mapping decides the outcome.** The debug provider
  yields `(status, body)`; what that becomes is decided by the same code a real provider's answer
  runs through. On Rust this is achieved by synthesising a `reqwest::Response` in memory
  (`impl<T: Into<Body>> From<http::Response<T>> for reqwest::Response`) and handing it to a
  **behaviour-preserving extraction** of the response half of `send_request` / `transcribe`. A
  pre-mapped `Result`, or a mapper that re-parses the body with `serde_json::from_str`, is forbidden:
  both change which error variant appears and therefore whether the fallback ladder fires.
- **`malformed` reproduces D-M2's mechanism, not a look-alike.** The canned `malformed` body is a
  body that genuinely fails to deserialize. On Rust/LLM that makes `response.json()` fail, which
  `#[from]`-converts to `LlmError::Request` — **retryable** — so the production ladder fires, which is
  D-M2's Desktop column. Whatever a twin's real code then does with that shape is the finding, never
  something the spec overrides.
- **STT debug lives in Rust only** (ADR-0017: STT is shared core). `Adr0017BoundaryGuardTest` stays
  green; no STT request/guard logic enters Kotlin.
- Twin rule: the LLM half lands in `llm/mod.rs` **and** `KlarvoApi.kt` (twinned); the STT half lands
  once in `stt/` (shared) and is selected in `groq_jni.rs`.
- Every new vector gets an inversion check that is RED at writing time (G-B).
- Provider strings must be added to the config allowlists or they are normalized away at load.
- **One client-side owner per config key.** `llmProvider` / `sttProvider` are held in exactly one
  React component. The new rows read and write that one owner through props; no second writer to
  those two keys is created on either side of the IPC boundary.

**Never:**
- No change to real provider behaviour. The only edits to shipping provider code are
  **behaviour-preserving extractions** of the response→result mapping into callable functions.
- Not added to `pipeline::resolve_fallback_provider`'s candidate array nor to
  `KlarvoApi::cleanupFallbackCandidates` — both lists stay literal and closed (Epic 12 FR2: Groq is
  never a cleanup fallback).
- Never the default. `debug` never appears in the **normal provider picker** — the model/provider
  `KSelect`s in `RecordingAudioContent.tsx` and the provider arrays in `SettingsPanel.tsx`, and
  nothing provider-shaped is added to `AiProvidersContent.tsx`. The four Advanced → System rows are a
  separate, `expertMode`-gated diagnostics control and are not that picker.
- The license gate is NOT touched — 13-4 owns it. Consequence, recorded not fixed: `debug` is a
  non-free provider, so `KlarvoApi.readConfig`'s gate rewrites both keys to `groq` on an unlicensed
  device, and a blank Groq key still makes `readConfig` return `null`. Android reproduction therefore
  needs a licensed/trial device with a Groq key configured.
- No clipboard-failure injection. D6 is **not** served by this mechanism; 13-2 records the Weg-2
  downgrade for D6.
- No remote telemetry; no host mutation; no `cargo check --target x86_64-pc-windows-gnu` (retired).

## I/O & Edge-Case Matrix

`advanced.debugLlmScenario` / `advanced.debugSttScenario`, camelCase, serde `default = "ok"`.
Every row below is the outcome of the twin's **own** mapping code, not an outcome the debug provider
asserts.

| Scenario | Canned wire response | Rust | Kotlin |
|---|---|---|---|
| `ok` | 200, well-formed body, canned text | canned text returned | canned text returned |
| `empty` | 200, `content: ""` | `LlmError::ResponseFormat("Empty content in response")` | returns `""` (unguarded → empty paste) — **D2/D-H19** |
| `truncated` | 200, `finish_reason: "length"` | `LlmError::OutputTruncated` | undetected, partial text returned — **D3/D-M16** |
| `malformed` | 200, body that does **not** deserialize as `ChatResponse` | `response.json()` decode error → `LlmError::Request` → **retryable** → ladder fires — **D10/D-M2 Desktop column** | `JSONObject(body)` throws `JSONException`, which is not an `IOException`, so the single-call ladder does **not** fire — **D10/D-M2 Android column**. On the chunked path (text ≥ `CHUNK_THRESHOLD` 400 bytes) `collectChunkResults` rewraps it as an `IOException` whose message carries no `HTTP nnn`, so there the ladder *does* fire. Both halves are pinned. |
| `http429` | 429, error body | `LlmError::ApiError{429}` → retryable → ladder fires | `IOException(… HTTP 429 …)` → retryable → ladder fires |
| `http5xx` | 503, error body | `ApiError{503}` → retryable → ladder fires | `IOException(… HTTP 503 …)` → retryable |
| `transport` | no response: real request to `http://127.0.0.1:1/` | genuine `reqwest` transport error → `LlmError::Request` → retryable | `IOException` with no `HTTP nnn` in the message → retryable |
| STT `ok` | 200, valid transcription body | canned transcript | n/a (Rust-only, ADR-0017) |
| STT `empty` | 200, empty text | `SttError::ResponseFormat`; pipeline currently ends `Stopped` — **D9** | n/a |
| STT `malformed` | 200, undecodable body | `SttError::ResponseFormat("Cannot parse STT response…")` — **non-retryable**, because the real STT path reads `bytes()` and parses with `serde_json::from_slice` rather than `response.json()`. The LLM↔STT retryability asymmetry is real, pre-existing and pinned as a finding. | n/a |
| STT `http429` / `http5xx` / `transport` | as the LLM rows | `ApiError{429\|503}` / `Request` → retryable — the shapes D9's retry-budget half needs | n/a |
| STT `truncated` | not offered | `SttError` has no truncation variant | n/a |

Provider-row option sets (each pinned to its Rust allowlist constant, see Tasks):

| Row | Writes | Options |
|---|---|---|
| `LLM Provider` | `llmProvider` | `VALID_LLM_PROVIDERS` + `debug` |
| `STT Provider` | `sttProvider` | `VALID_STT_PROVIDERS` + `debug` |

## Control states (response-state contract)

The only interactive control this story adds is `KSelect`, **four times**, reused **verbatim** — same
component, no new class, variant, state or token. No `KToggle` is added: the `expertMode` switch that
gates the rows already ships (`AdvancedSettingsPanel.tsx:407-423`) and is untouched. The only new
strings are the four row labels and the option labels, and those are *derived*, not composed (see the
Decisions entry "Wording is derived, never composed").

Source for every state below is the shipped precedent `src/components/settings/FormControls.tsx::KSelect`
(Story 8.2, at `baseline_revision`, in use in `LanguageContent.tsx`, `AppearanceContent.tsx`,
`AiProvidersContent.tsx`, `RecordingAudioContent.tsx`). The design canon defines `.select` **idle
only**, so it cannot source the rest — recorded as `canon gap, shipped precedent`, not invented here.

| State | Definition in the shipped component | Observable gate |
|---|---|---|
| idle | trigger `bg-klarvo-surface-2`, `border-klarvo-border`, `rounded-klarvo-sm`, `px-2.5 py-1.5`, `text-xs text-klarvo-text` (FormControls.tsx:357-364) | computed-style equality vs the reference instance |
| hover | `hover:border-klarvo-border-2`, suppressed while open (l.362) | computed border-color after hover, equality vs reference |
| pressed | no distinct pressed styling exists; a press opens the listbox, so "pressed" is observably the open state | equality vs reference (both unchanged on press) |
| focused | `focus:outline-none focus-klarvo` (l.361) — the always-on ring is a pre-existing app-wide defect already filed in `docs/backlog.md`; not fixed here | computed box-shadow equality vs reference |
| disabled | `opacity-40 cursor-not-allowed` (l.363); these four rows are never disabled — the `expertMode` gate removes them instead | not exercised; reachable only via the `disabled` prop, which is not passed |
| open / expanded | trigger `border-klarvo-border-2` + chevron `rotate-180` + `aria-expanded="true"`; portal listbox `bg-klarvo-elevated shadow-klarvo-e2 border border-klarvo-border` (l.298-303) | listbox present in DOM, computed background/border equality vs reference |
| option: selected / keyboard-focused / disabled | `text-klarvo-teal` / `bg-klarvo-surface-2` / `text-klarvo-dim opacity-50` (l.318-329); no option of these rows is ever disabled | computed style equality vs reference option, compared like-for-like (selected vs selected) |
| loading / error | not applicable — all four option lists are static literals, no async source | — |

**Reference instance** (the named existing `KSelect` every equality check compares against): the
"Dictation language" select at `src/components/settings/LanguageContent.tsx:56-63`.

**Scope of the equality gate:** the `KSelect` control only (trigger, portal listbox, options). The
label span beside it is **not** in scope: the reference row uses `LABEL_CLS_M` while every row in
`AdvancedSettingsPanel` uses `LABEL_CLS`, and matching the panel's own neighbours is the correct
consistency axis. The four new labels therefore use `LABEL_CLS`, exactly like the Log Level row above
them.

</intent-contract>

## Code Map

**Reuse, do not rebuild.** `_bmad-output/implementation-artifacts/13-1-attempted-implementation.patch`
(2150 insertions, 15 files) is the reverted first build. It applies cleanly at `baseline_revision`
(`git apply --check` exit 0, verified 2026-09-20). Apply it first, then make the amendments below.
Everything it contains is correct except the `malformed` mechanism and the missing provider rows.

**Rust — LLM (`src-tauri/src/llm/mod.rs`)**
- `OpenAiCompatibleCleanup::send_request` (:602-655) — the response half (**:611-652**: status check,
  non-2xx `text()`+`ApiErrorResponse` branch, `response.json()`, usage, first-choice, `finish_reason`,
  empty-content guard) moves **verbatim** into `async fn map_chat_http_response(response: reqwest::Response)
  -> Result<CleanupResult, LlmError>`. `send_request` becomes build→send→call it.
  ⚠️ This **replaces** the patch's `map_chat_error_status` / `map_chat_success` / `map_chat_response`
  trio. `map_chat_response(status, &str)` must not survive: it parses with `serde_json::from_str`,
  which reports `ResponseFormat` where the live path reports `Request`, and that is exactly the
  misrepresentation this amendment removes.
- `DebugCleanup` (patch) — keep, but `canned()` now builds
  `http::Response::builder().status(s).body(body.as_bytes().to_vec())`, converts with
  `reqwest::Response::from(..)`, and calls `map_chat_http_response`. `transport` keeps the real POST
  to `DEBUG_TRANSPORT_URL`.
- `LlmError` (:36-58) — exactly one `#[from]`: `Request(reqwest::Error)` (:39). No variant is
  constructible from a `String` that is retryable. **Do not add a variant.**
- `debug_llm_canned_wire` (patch) — the `malformed` arm changes to an undecodable body.
- `effective_cleanup_model` (:1306-1322) — the patch's `DEBUG_PROVIDER_NAME` arm stays.
- `ChatResponse`/`ChatChoice`/`ChatMessageResponse`/`ChatUsage` (:400-429), `ApiErrorResponse` (:431-439).

**Rust — STT (`src-tauri/src/stt/mod.rs`, shared, serves Android too)**
- `WhisperStt::transcribe` (:358-435) — the response half (**:384-429**) moves verbatim into
  `async fn map_transcription_http_response(response: reqwest::Response) -> Result<String, SttError>`.
  Replaces the patch's `map_transcription_response(status, &[u8])` seam for the same reason.
  `DebugStt` synthesises its `reqwest::Response` the same way `DebugCleanup` does.
- `SttError` (:49-66) — `Request(#[from] reqwest::Error)` is the only `#[from]`.
- `DebugStt::transcribe` ignores the audio entirely (no `EmptyAudio` guard) — load-bearing for the
  JNI selector test below.

**Rust — config / pipeline / commands**
- `src-tauri/Cargo.toml` — **add `http = "1"`** to `[dependencies]`. `http 1.4.0` is already resolved
  in `Cargo.lock` as a transitive dep of reqwest 0.12.28, so this adds zero crates; the lockfile's
  `klarvo` entry changes and must be committed. reqwest does not re-export `http::Response`.
- `src-tauri/src/config/mod.rs::VALID_LLM_PROVIDERS` (:1179) / `::VALID_STT_PROVIDERS` (:1178) — the
  patch adds `"debug"`; without it `migrate_and_normalize` step (f) (:1364-1378) rewrites the stored
  value at load and the next save persists the rewrite.
- `src-tauri/src/config/mod.rs::AdvancedSettings` (:30+, `rename_all = "camelCase"`) — the patch's two
  `#[serde(default = "default_debug_scenario")]` fields.
- `src-tauri/src/pipeline.rs` — the patch's `"debug"` arms in `resolve_stt_provider` (:43-55),
  `cleanup_provider_for` (:217-248), `resolve_cleanup_provider` (:257-285). Catch-alls otherwise
  swallow the value into Groq/DeepSeek. `resolve_fallback_provider` (:334-355) **untouched**.
- `src-tauri/src/pipeline.rs::is_retryable_llm_error` (:299-302) / `::is_retryable_stt_error`
  (:313-316) — `ApiError{429|>=500}` or `Request(_)`. **Read-only**: these decide whether `malformed`
  fires the ladder, and changing them would change real behaviour.
- `src-tauri/src/pipeline.rs::try_local_whisper_fallback` (:130-159) — ⚠️ can silently rescue a debug
  STT error where a local model is downloaded. Name it in the reproduction instructions.
- `src-tauri/src/commands/settings.rs::save_settings` (:400-599) — merges via `SettingsPatch`
  (`llm_provider`/`stt_provider` at :274-275, `None` preserves) and then hot-reloads **both** slots
  (:576-579). This is the only writer of the two provider keys and the reason the surface route below
  needs no new command.
- `::save_advanced_settings` (:757-778) — whole-block replace; rebuilds **only** the cleanup slot.
- `::cleanup_provider_reload_needed` (:714-727) — the patch adds the `debug_llm_scenario` clause.

**Rust — JNI (`src-tauri/src/stt/groq_jni.rs`)**
- `Java_com_klarvo_voice_GroqSttBridge_nativeTranscribe` (:131-142) — the patch appends
  `stt_provider: JString, debug_stt_scenario: JString`. ⚠️ JNI binds by name + arity: Kotlin and Rust
  must change in the same commit, and a partial rebuild misbinds rather than throwing.
- **New:** the provider choice moves out of the `extern "system"` fn into a plain
  `fn select_stt_provider(provider_name: &str, debug_scenario: &str, api_key: &str, model: &str,
  temperature: f32) -> Box<dyn SttProvider>` so a Rust test can reach the Android debug branch — the
  review's one unfixed `high`.
- The debug branch must leave the runtime, guard chain and `__ERROR_*` sentinel mapping untouched.

**Kotlin — LLM twin (`android/kotlin-src/com/klarvo/voice/`)**
- `KlarvoApi.kt::cleanup` (:982-1126) — URL built at :1077 (the patch's short-circuit point), status
  at :1108, non-200 throw at :1111, body parse at :1114-1125 (no `finish_reason`, no empty guard).
  The patch's `mapCleanupResponse` extraction is correct and stays; only the canned body changes.
- `::resolveLlmProvider` (:241-284) — `else ->` at :262-267 maps anything unknown to DeepSeek; the
  patch's explicit `DEBUG_PROVIDER_NAME ->` arm is mandatory for reachability.
- `::cleanupFallbackCandidates` (:295-322, private) — literal deepseek→openai→openrouter; **do not touch**.
- `::readConfig` (:438-599) — `llmProvider` :466, `sttProvider` :477, license-gate rewrite :554-574
  (forces both to `groq` when unlicensed; `debug` counts as `sttIsAlternative`), blank-Groq-key `null`
  return :576-577. Not fixed here (13-4).
- `::Config` (:140-227) — constructed **positionally**; new fields go last, after `llmModelGroq` (:226).
- `::collectChunkResults` (:1265-1282) — the only place a `JSONException` becomes an `IOException`;
  load-bearing for the chunked half of the `malformed` row.
- `KlarvoOverlayService.kt::isRetryableCleanupFailure` (:2703-2706) — `Regex("HTTP (\\d{3})")`,
  `null|429|>=500` → retryable; gate at :2221 requires `e is IOException` first.
- `GroqSttBridge.kt::nativeTranscribe` (:49-58) — the patch appends the two params.

**React — surface**
- `src/components/AdvancedSettingsPanel.tsx` — props :34-43 (today only `onClose`, `isPaid`,
  `isTrial`, `embedded`); `expertMode` :54; local `set()` :86-88 (state only); own `handleSave` :92-105
  → `saveAdvancedSettings` :96; own sticky footer :459-477; Log Level row **:394-403** (a raw
  `<select>`, the new rows go directly after it); Expert-mode switch :407-423; `{expertMode && …}`
  precedent :215-239; `LABEL_CLS` imported at :6; `isMobile` is a module import at :7, not a prop.
- `src/components/SettingsPanel.tsx` — instantiates the panel at **:877-879**;
  `localSttProvider`/`localLlmProvider` at **:152-153**; ⚠️ **re-seed effect :286-300** rewrites a
  persisted `llmProvider` to a keyed provider whenever `llmKeyMap[value]` is undefined — it would eat
  `"debug"` on every refresh; dirty check :377-378; `saveCurrentSettings` :494-653 passes the two
  providers at :549; `handleSttProviderChange` :472-490 (reached only by the Cloud/Offline toggle —
  a secondary hazard, leave it alone but do not let it see `debug`).
- `src/components/settings/FormControls.tsx::KSelect` — `KSelectOption` :66-70, props :72-79, wrapper
  :338, trigger :357-364, portal listbox :298-303, options :318-329.
- `src/components/settings/LanguageContent.tsx:56-63` — the named reference instance; the row
  structure to copy (wrapper `flex gap-3 ${isMobile ? "flex-col" : "items-center justify-between"}`,
  label span, `KSelect` with `className={isMobile ? "w-full" : "w-auto"}`).
- `src/components/settings/RecordingAudioContent.tsx` — the **only** real provider picker: STT via
  `CLOUD_STT_MODELS` :10-14 + `KSelect` :81-100, LLM via an inline literal array :145-151. No shared
  provider constant exists anywhere — do not create one.
- `src/components/settings/AiProvidersContent.tsx` — API-key inputs only; its two `KSelect`s are
  cleanup-style and language. Nothing provider-shaped to leak into; assert that positively.
- `src/types.ts::AppSettings` :30-105 (`sttProvider`/`llmProvider` :44-45) and `::AdvancedSettings`
  :180-203; `src/tauri-commands.ts::MOCK_ADVANCED_SETTINGS` :100-118, `MOCK_SETTINGS` :44-98,
  `isPreviewMode` :20-21 (preview: reads return fresh mocks, writes are silent no-ops).
- `src/components/settings/types.ts:64-69` — the `advanced` category carries no `desktopOnly`, so the
  panel and the new rows render on Android (`MainActivity : TauriActivity` hosts the same bundle).

**Tests**
- Rust doubles to copy: `llm::tests::MockCleanupProvider`, `pipeline::tests::FakeStt` / `::FakeCleanup`.
- Kotlin: JUnit 4, **no mocking library** (`android.util.Log` throws "not mocked"), pure seams only,
  throwing `org.json` accessors (never `optString`-with-default), a "what this does NOT cover" KDoc.
  Fixture-loading convention: `TwinConstantsVectorsTest.kt:1-75` (CWD path walk, loud failure, id map).
- `test-fixtures/README.md` — the reader ledger; a new fixture adds a row.

## Tasks & Acceptance

**Execution:**
- [x] Apply `_bmad-output/implementation-artifacts/13-1-attempted-implementation.patch` unchanged as
      the starting point (`git apply`), then perform every task below as an amendment to it. Do not
      rebuild from zero; the patch's Rust/Kotlin/TS/fixture/test skeleton is correct except where a
      task says otherwise.
- [x] `src-tauri/Cargo.toml` + `src-tauri/Cargo.lock` -- add `http = "1"` as a direct dependency (zero
      new crates; already resolved at 1.4.0) -- so a `reqwest::Response` can be built in-process.
- [x] `src-tauri/src/llm/mod.rs` -- replace the patch's `map_chat_error_status` / `map_chat_success` /
      `map_chat_response` with ONE verbatim extraction `map_chat_http_response(response: reqwest::Response)`
      carrying `send_request`'s lines 611-652 unchanged; `send_request` calls it; `DebugCleanup::canned()`
      synthesises its `reqwest::Response` from the canned `(status, body)` and calls the same function.
      Change the `malformed` arm of `debug_llm_canned_wire` to a body that does not deserialize as
      `ChatResponse` -- so the debug provider produces the real `LlmError::Request` and the ladder fires.
- [x] `src-tauri/src/stt/mod.rs` -- the same single extraction `map_transcription_http_response(response)`
      carrying `WhisperStt::transcribe`'s lines 384-429 unchanged, replacing the patch's
      `map_transcription_response`; `DebugStt` synthesises its response the same way and keeps ignoring
      the audio bytes.
- [x] `src-tauri/src/stt/groq_jni.rs` -- extract the provider choice into a plain
      `select_stt_provider(...) -> Box<dyn SttProvider>` called by `nativeTranscribe`, so the Android
      debug branch becomes reachable by a Rust unit test.
- [x] `src/components/SettingsPanel.tsx` -- (a) pass `llmProvider`, `sttProvider`,
      `onLlmProviderChange`, `onSttProviderChange` into `AdvancedSettingsPanel` at :877-879, bound to
      the existing `localLlmProvider` / `localSttProvider` state and their setters -- keeping exactly
      one client-side owner of those two keys; (b) fix the re-seed effect at :286-300 so a persisted
      `"debug"` is preserved instead of being rewritten to the first keyed provider. No new Tauri
      command, no second write path.
- [x] `src/components/AdvancedSettingsPanel.tsx` + `src/types.ts` + `src/tauri-commands.ts` -- four
      `KSelect` rows appended to the existing `system` sub-page after the Log Level row (:394-403),
      inside one `{expertMode && ( … )}` wrapper: `LLM Provider`, `STT Provider`, `Debug LLM Scenario`,
      `Debug STT Scenario`. Row structure copied from the reference instance
      (`LanguageContent.tsx:56-63`); label class `LABEL_CLS` (the panel's own); `KSelect` imported from
      `./settings/FormControls` and used verbatim -- no new class, variant, state or token. Wording is
      derived, not composed: labels are the config keys in the panel's existing title case (cf. the
      shipped `Log Level`), option labels are the config values verbatim; no hint line, no prose. The
      two scenario fields are mirrored into `AdvancedSettings` and `MOCK_ADVANCED_SETTINGS`; the two
      provider values come from props.
- [x] `test-fixtures/debug-provider-scenario-vectors.json` + `test-fixtures/README.md` -- amend
      `DEBUG-LLM-MALFORMED-001` (wire body, `rust.error` -> `Request`, `message_contains`,
      `description`, `expected_divergence`) to the new mechanism; amend `DEBUG-STT-MALFORMED-001`'s
      description to state that STT's undecodable body is NON-retryable and why (`from_slice`, not
      `response.json()`); add `DEBUG-LLM-PROVIDER-OPTIONS-001` and `DEBUG-STT-PROVIDER-OPTIONS-001`
      carrying the expected option arrays -- so the React lists are pinned by a fixture rather than by
      nothing.
- [x] `src-tauri/src/llm/mod.rs`, `src-tauri/src/stt/mod.rs`, `src-tauri/src/config/mod.rs`,
      `src-tauri/src/pipeline.rs`, `src-tauri/src/commands/settings.rs`, `src-tauri/src/stt/groq_jni.rs`
      -- amend the patch's `#[cfg(test)]` tests to the new mechanism and add: the `malformed` LLM
      vector asserts `LlmError::Request` AND `is_retryable_llm_error(..) == true`; a test that drives
      `map_chat_http_response` with a 200 + `{"choices":[]}` and still gets
      `ResponseFormat("No choices in response")` (proving the extraction is behaviour-preserving for
      the shape that is no longer a scenario); `select_stt_provider("debug", …)` with empty audio
      yields the canned mapping while `select_stt_provider("groq", …)` with empty audio yields
      `EmptyAudio` without a network call (the Android-branch reachability test and its inversion); a
      test asserting the two provider-option fixture arrays equal `VALID_LLM_PROVIDERS` /
      `VALID_STT_PROVIDERS`; a config test that writes and re-reads an actual `config.json` file and
      finds `"debug"` intact. Each new vector carries an inversion check RED at writing time.
- [x] `android/kotlin-test/com/klarvo/voice/DebugProviderScenarioTest.kt` -- amend the `malformed`
      tests to the undecodable body; keep and re-aim the discriminating assertion so it pins BOTH
      halves of the Android column: `JSONException` is not an `IOException` (single-call path, ladder
      silent) and `collectChunkResults` rewraps it into an `IOException` whose message carries no
      `HTTP nnn` (chunked path, ladder fires).

**Acceptance Criteria:**
- Given a licensed device with `expertMode` on, when Andi opens Settings → Advanced → System, then the
  `LLM Provider` and `STT Provider` rows are present and selecting `debug` in each persists to
  `config.json` and survives an app restart — the enabler is switchable on the device itself, with no
  computer attached and no hand-edited config file.
- Given `llmProvider = "debug"` and `debugLlmScenario = "empty"`, when Andi dictates a short sentence,
  then the pre-13-2 behaviour is observable unchanged per platform (Desktop: cleanup error path;
  Android: an empty paste) — proving the enabler.
- Given `sttProvider = "debug"` and `debugSttScenario = "empty"`, when Andi dictates, then the empty
  STT result reaches the pipeline and `klarvo.log` names the scenario that fired.
- Given `debugLlmScenario = "malformed"` on Desktop, when cleanup runs, then the error is retryable
  and the production ladder fires — the same mechanism audit row D-M2 records, not a look-alike.
- Given `debugLlmScenario = "http429"`, when cleanup runs, then the ladder fires
  deepseek → openai → openrouter and `debug` never appears as a fallback candidate.
- Given the normal provider picker on either platform (`RecordingAudioContent`, and the provider
  arrays in `SettingsPanel`), when it is opened with `expertMode` both off **and** on, then `debug` is
  not among its options; and `AiProvidersContent` exposes no provider picker at all.
- Given `expertMode` is off, when Advanced → System is opened, then all four rows are absent.
- Given the ADR-0017 tripwire, when the JVM gate runs, then `Adr0017BoundaryGuardTest` is green.
- Given the four new rows with `expertMode` on, when each state of the control-states contract is
  driven in the desktop proxy for **every one of the four**, then each measured computed style of the
  `KSelect` equals the named reference instance's — no new class, variant, state or token.

## Implementation Notes

Written during implementation (2026-09-20), from the tree rather than from the plan.

### 1. The patch was applied unchanged, then amended

`git apply 13-1-attempted-implementation.patch` at `baseline_revision` — clean, 2150 insertions,
15 files. Everything below is an amendment to it; nothing was rebuilt from zero.

### 2. One extraction per twin, driving a synthesised `reqwest::Response`

- `llm::map_chat_http_response(response) -> Result<CleanupResult, LlmError>` now carries
  `send_request`'s whole response half verbatim (`status()` → `text()`+`ApiErrorResponse` →
  `response.json()` → usage → first choice → `finish_reason` → empty-content guard). The patch's
  `map_chat_error_status` / `map_chat_success` / `map_chat_response` trio is **gone**: nothing in
  the tree parses a chat body with `serde_json::from_str` any more, so there is no second mapping
  that could report a different variant than the live path.
- `stt::map_transcription_http_response(response)` is the same move for `WhisperStt::transcribe`,
  replacing `map_transcription_error_status` / `map_transcription_body` / `map_transcription_response`.
- Both debug providers build their response with `llm::debug_canned_response(status, body)` —
  `http::Response::builder()…body(Vec<u8>)` + `reqwest::Response::from(..)`. It returns `Option`
  rather than panicking on an invalid status; every caller feeds it a literal from a closed table,
  so the `None` arm is unreachable in practice and is mapped to a fail-soft `ResponseFormat`.
- `http = "1"` is a direct dependency now. `cargo metadata` added exactly one line to `Cargo.lock`
  (the `klarvo` package's dependency list) — zero new crates, as the spec predicted.

### 3. `malformed` is now a body that ends mid-string

`{"choices":[{"message":{"content":"Debug provider truncated stream` — verified on both twins:
Rust's `response.json()` fails → `LlmError::Request` → `is_retryable_llm_error == true` → the
ladder fires; Kotlin's `JSONObject(body)` throws `JSONException`. The body deliberately carries no
`HTTP <nnn>` substring, because the chunked Kotlin path rewraps the exception's own message into an
`IOException` and `isRetryableCleanupFailure` regex-matches that message.

Both Android halves are now pinned by `DebugProviderScenarioTest`: the silent single-call path
(`JSONException` is not an `IOException`) and the chunked path, driven through the real
`KlarvoApi.collectChunkResults` with a real `Future`, plus a discriminating half proving an HTTP
failure *does* keep its status through the same seam.

**The STT `malformed` body was NOT changed** — only its description. Its Rust outcome is
`ResponseFormat` and therefore non-retryable, because that path reads `bytes()` and parses with
`serde_json::from_slice`. The fixture now states that asymmetry, and both `malformed` vectors carry
a `rust.retryable` field that the Rust readers assert against the real `is_retryable_*` predicate.

### 4. ⚠ `select_stt_provider` lives in `groq_jni.rs` — and the module had to be ungated

The spec asks for the extraction so "a Rust test can reach the Android debug branch". Putting it in
`groq_jni.rs` alone would not have achieved that: the file carried `#![cfg(target_os = "android")]`
*and* was declared `#[cfg(target_os = "android")] pub mod groq_jni;`, so on the platform the
`cargo test --lib` gate runs on it does not exist. The module gate was therefore moved from the
module to its items: every `extern "system"` entry point and every JNI helper keeps
`#[cfg(target_os = "android")]`, while `select_stt_provider` compiles and is tested everywhere.

Consequence found while doing it, recorded not fixed: `groq_jni`'s own `#[cfg(test)] mod tests` had
**never executed** (android-gated file, and `cargo test` never runs for Android). Ungating it turns
`test_panic_safety_is_hallucination_unusual_inputs` red, because `is_hallucination("\0")` is false
against today's guard — a never-executed test's wrong expectation from story 7-3, not a product
defect. That test module is therefore left `#[cfg(target_os = "android")]`, i.e. exactly as
(non-)executing as before, with a comment naming the trap for whoever ungates it. It is in
frontmatter `deferred`.

### 5. Surface: one owner, props down, and the re-seed fix

`AdvancedSettingsPanel` gained four **required** props (`llmProvider`, `sttProvider`,
`onLlmProviderChange`, `onSttProviderChange`) bound to `SettingsPanel`'s existing
`localLlmProvider` / `localSttProvider` state and their setters. Required rather than optional so
TS refuses a panel instance that is not wired; there is exactly one call site.

`SettingsPanel.tsx`'s re-seed effect now reads `if (llmProv !== "debug" && !llmKeyMap[llmProv])`.
Without it a persisted `"debug"` was rewritten to the first keyed provider on every refresh and the
next Save persisted the rewrite.

The four rows sit in ONE `{expertMode && (…)}` wrapper directly after the Log Level row, in the
order `LLM Provider`, `STT Provider`, `Debug LLM Scenario`, `Debug STT Scenario`, each a verbatim
`KSelect` with a `LABEL_CLS` label. The provider option arrays mirror `VALID_LLM_PROVIDERS` /
`VALID_STT_PROVIDERS` and are pinned from both ends by the two new fixture vectors (a Rust test
compares them to the constants; the proxy harness compares them to the DOM).

### 6. Two save paths, both converging — stated, not fixed

The provider rows are saved by the **Settings** footer (`save_settings`, which rebuilds both the
cleanup and the STT slot); the scenario rows by the **Advanced** panel's own footer
(`save_advanced_settings`, which rebuilds only the cleanup slot). Both footers can be visible at
once. Because only the former rebuilds the STT slot, the reproduction instructions say to press the
Settings Save last. Unchanged in code, handled operationally — as the spec directs.

### 7. Gate-4 harness: what changed and one new trap

`gate4-evidence/13-1/debug-rows-smoke.mjs` was extended, not rewritten from zero: four rows instead
of one, fixture-driven option lists, a positive AI-&-Providers assertion (it **adds a profile** so
the only dropdowns that surface can show actually exist, instead of scanning zero elements), a
picker scan in both Expert-mode states, three inversion groups, and one check count derived from
the result records.

New trap #5, recorded: the "Expert mode ON" picker scan is not reachable by clicking, because
preview writers are no-ops. It is reached with the project's documented technique — a throwaway
edit to `src/tauri-commands.ts` (`MOCK_ADVANCED_SETTINGS.expertMode`), asserted to have taken
effect, then restored to the original bytes inside a `finally`, with the restore itself asserted.
New trap #6: `innerText` applies CSS `text-transform`, so a case-sensitive "did the page render"
probe read as "page empty" against `uppercase` section headings.

### 8. Inversions, actually run

Every "RED at writing time" claim in this build was executed, not asserted:
`malformed` → `{"choices":[]}` reddens `spec_debug_llm_malformed`; flipping `select_stt_provider`'s
comparison reddens all three `spec_android_select_stt_provider_*`; dropping `"debug"` from
`VALID_LLM_PROVIDERS` reddens the normalization, file-round-trip and option-list tests; dropping
`"debug"` from a fixture options array reddens the option-list test; renaming `debugSttScenario`'s
serde name reddens the camelCase and round-trip tests. The harness's three inversion groups each
produced a red.

## Spec Change Log

- **2026-09-20 (implementation):** no change to the plan's shape. One deviation from the letter of a
  task, recorded rather than silently taken: the task says to extract `select_stt_provider` in
  `src-tauri/src/stt/groq_jni.rs` "so the Android debug branch becomes reachable by a Rust unit
  test". That file was gated `#![cfg(target_os = "android")]` and declared behind the same cfg, so a
  function inside it is not compiled on the platform the test gate runs on. The function stays in
  that file, as directed; the module's **gate** moved from the module to its JNI-specific items so
  the selector compiles and is tested on the `cargo test --lib` gate (Implementation Notes §4). Also
  recorded there: ungating exposed a never-executed wrong assertion in that module's own test block,
  which is left non-executing and filed under frontmatter `deferred` rather than adjudicated here.
- **2026-09-20 (planning, successor spec):** opened as `-2` after the predecessor
  `spec-13-1-debug-test-provider-both-twins.md` reached `blocked` with `intent gap`. Two product
  answers from Andi (recorded in `gate4-evidence/RUN-2026-09-20.md`) are now built into the intent
  contract: (1) route (a), two provider rows in Advanced → System behind `expertMode`, `debug` still
  absent from the normal picker, labels derived like the shipped `Log Level` row; (2) `malformed`
  reproduces D-M2's Desktop mechanism (undecodable body → retryable → ladder), not the
  `ResponseFormat` outcome. The contradiction that caused the halt — "the only interactive control
  this story adds is `KSelect`, twice" versus "selectable on both devices" — is removed: it is
  `KSelect`, four times.

## Review Triage Log

### 2026-09-20 — Review pass
- verdicts: 36 findings — high 0, medium 11, low 20, false 5, maybe-false 0
- findings:
  - `[medium]` `[patch]` blind-hunter: the Android `debug` LLM branch in `KlarvoApi.cleanup` is reached by no executing test — verified: `grep -rn "\.cleanup(" android/kotlin-test/` had no hits; the class KDoc waived it. Fixed by extracting `internal fun debugCleanupOrNull(provider)` with the `KlarvoLogger` call hoisted, driven for `ok`, `http429` and seven non-debug names, plus a source-text tripwire pinning the call site ABOVE `val url = URL(provider.url)` (commits `f1f07cb`, `d90f83a`).
  - `[low]` `[reject]` blind-hunter: the normal provider picker renders the literal `debug` as its trigger label once the enabler is on — verified real (`FormControls.tsx:93` `options.find(...)?.label ?? value`), but the intent's Never clause and its AC are both about the option *list*, which stays clean; showing the active value is honest, the state is expert-mode-only, and every fix adds a branch to hide the truth.
  - `[low]` `[reject]` blind-hunter: the rewritten `scanPickers()` dropped the predecessor's `document.body.innerText` `debug` scan — verified dropped, but the harness never sets any row to `debug` (the preview mock is `deepseek`), so the old scan could not have caught the leak above either; re-adding it would add a check that cannot fire.
  - `[medium]` `[patch]` blind-hunter: `handleSttProviderChange` silently clobbers `llmProvider = "debug"` — verified: `SettingsPanel.tsx:757` passes it to `RecordingAudioContent` as `setLocalSttProvider`, and `RecordingAudioContent.tsx:82-90` calls it from the cloud STT Model picker as well as the Cloud/Offline toggle. Fixed with the same guard shape as the re-seed effect at :302, in functional form (`f1f07cb`).
  - `[low]` `[patch]` blind-hunter: the Rust `transport` scenario builds a bare `reqwest::Client::new()` with no timeout — verified: every shipped provider in the same two files uses `Client::builder().connect_timeout(15s).timeout(30s)` (`llm/mod.rs:502`, `:1071`, `stt/mod.rs:284`). Fixed at both debug sites with the identical builder plus `.no_proxy()` (`f1f07cb`).
  - `[low]` `[reject]` blind-hunter: `debugSttScenario` does not hot-reload while `debugLlmScenario` does — verified real (`save_advanced_settings` rebuilds only the cleanup slot), but the spec records it operationally and the manual checks prescribe pressing the Settings Save last, which rebuilds both slots; the fix adds a new STT-slot rebuild path.
  - `[low]` `[reject]` blind-hunter: two stacked sticky Save footers with different scopes, and nothing on screen says which saves what — verified real (`SettingsPanel.tsx:906` renders outside the category branch). Rejected: the intent mandates both "no second writer to those two keys" and "no hint line, no prose", which together force this shape; a wrong press is self-revealing and recoverable, and the manual checks name the order in bold.
  - `[low]` `[reject]` blind-hunter: INVERSION-1 is overclaimed in `verdict.md` (only `idle`, only `ROWS[0]`) — verified: the run prints `INVERSION-1: 1/10 RED` itself, so the count is not concealed; the raw `<select>` has no listbox or options, so the other six states cannot be pointed at it, and the spec asks for one red run.
  - `[low]` `[patch]` blind-hunter: the cross-twin fixture names `llm::map_chat_response` / `stt::map_transcription_response`, the two mappers this change deliberately deleted — verified at fixture lines 6 and 159. Fixed to the `*_http_response` names (`f1f07cb`).
  - `[low]` `[patch]` blind-hunter: `DEBUG-LLM-MALFORMED-001` pins reqwest's Display string with no version reference — verified. Fixed by naming reqwest 0.12.28 (`src/error.rs`, `Kind::Decode`) in the description and restating that `rust.error` + `rust.retryable` are the load-bearing claims (`f1f07cb`).
  - `[medium]` `[patch]` blind-hunter: the React option arrays are pinned only by a script no gate runs — verified both halves myself: removing `"debug"` from `VALID_LLM_PROVIDERS` reddens 3 tests; removing it from `LLM_PROVIDER_OPTIONS` reddens none. Fixed with `llm::tests::spec_react_option_arrays_and_debug_guards_are_pinned_to_rust`, a source-text tripwire in the `Adr0017BoundaryGuardTest` style, five inversions run RED (`f1f07cb`).
  - `[low]` `[patch]` blind-hunter: `"debug"` is a bare literal on the TS twin only — same root cause as the entry above; the new tripwire builds both `SettingsPanel.tsx` guards from `DEBUG_PROVIDER_NAME`, so a rename reddens it (`f1f07cb`).
  - `[low]` `[patch]` edge-case-hunter: `transport` has no connect/read timeout — same entry as the blind-hunter timeout finding; fixed together (`f1f07cb`).
  - `[low]` `[patch]` edge-case-hunter: `HTTP_PROXY` would route the loopback probe through a proxy, falsifying "no byte leaves the device" — same entry; `.no_proxy()` added at both sites (`f1f07cb`).
  - `[low]` `[reject]` edge-case-hunter: only `debug_stt_scenario` changing rebuilds no STT slot — same claim as the blind-hunter hot-reload finding; rejected for the same reason.
  - `[false]` `[reject]` edge-case-hunter: a blank Groq key makes `readConfig` return `null` even for the key-free debug STT provider — the bad outcome is real but the intent names it verbatim as out of scope ("a blank Groq key still makes `readConfig` return `null`… Android reproduction therefore needs a licensed/trial device with a Groq key configured"); 13-4 owns the gate.
  - `[false]` `[reject]` edge-case-hunter: `sttIsAlternative` rewrites `sttProvider = "debug"` to `groq` on an unlicensed device — likewise named verbatim in the intent ("the gate rewrites **both keys** to `groq` on an unlicensed device"); the claim that only the llmProvider half is documented is refuted by the intent text itself.
  - `[low]` `[reject]` edge-case-hunter: the Advanced STT row writes the raw setter, so picking `local` there leaves a cloud LLM configured — verified real, but routing it through `handleSttProviderChange` would reintroduce exactly the `debug` clobber the patch above removes, and the resulting state is legal and reversible from the normal picker.
  - `[low]` `[patch]` edge-case-hunter: `isTrivialChunk` returns letter/digit-free input verbatim before `cleanup` is reached, contradicting "deterministic regardless of what was dictated" — verified at `KlarvoApi.kt:1529` and its Rust twin; the product behaviour is correct and shared by both twins, so only the overbroad comment was wrong. Reworded in both twins (`f1f07cb`).
  - `[medium]` `[patch]` edge-case-hunter (claim): "pinned from both sides by the fixture" is false for the React arrays — same entry as the tripwire finding; closed by it (`f1f07cb`).
  - `[false]` `[reject]` edge-case-hunter (claim): the new rows use `LABEL_CLS` while the reference row uses `LABEL_CLS_M` — the intent-contract prescribes `LABEL_CLS` explicitly and puts the label span outside the equality gate's scope, so this is the specified outcome, not a defect.
  - `[medium]` `[patch]` verification-gap: React option lists pinned only by a throwaway harness no recurring gate runs — same entry as the tripwire finding; the lens's demonstration was reproduced before patching (`f1f07cb`).
  - `[medium]` `[patch]` verification-gap: `KlarvoApi.cleanup`'s debug short-circuit is driven by no test, and moving it below `URL(provider.url)` keeps every gate green — same entry as the blind-hunter Kotlin finding; the extraction closed the coverage half (`f1f07cb`) and the source-text ordering tripwire closed the placement half (`d90f83a`).
  - `[medium]` `[defer]` verification-gap: the widened `nativeTranscribe` JNI signature is checked by nothing that runs device-free — verified: no gate loads the `.so`, the `extern "system"` entry points are android-gated, and the export binds by short name. The gap class predates this story (no gate has ever checked that boundary) and closing it needs an emulator/adb step; deferred with severity medium.
  - `[low]` `[patch]` verification-gap (other): stale `map_chat_response` / `map_transcription_response` names in the fixture — same entry as the blind-hunter fixture finding; fixed (`f1f07cb`).
  - `[low]` `[reject]` verification-gap (other): `KSelect`'s `currentLabel` fallback renders "debug" in the normal picker — same entry as the blind-hunter trigger-label finding; rejected for the same reason.
  - `[low]` `[reject]` intent-alignment: the UI → IPC `save_settings` → `config.json` seam is covered by nothing — verified real, but the merge is a provider-agnostic `Option<String>` passthrough and the only `debug`-specific hazard in it (allowlist normalization) is pinned twice, including by a real-file round-trip; a `save_settings` integration test needs a Tauri `AppHandle`.
  - `[low]` `[reject]` intent-alignment: the rows' own Save button does not save the provider rows — same entry as the two-footers finding; rejected for the same reason.
  - `[medium]` `[patch]` intent-alignment: "one owner" is satisfied for state but two writers with different semantics touch `llmProvider` — the clobbering half is the `handleSttProviderChange` entry and was patched (`f1f07cb`); the `local`-coupling half was rejected above.
  - `[false]` `[reject]` intent-alignment: the provider rows are a second, differently-optioned picker (`anthropic` reachable, `local` a one-way exit) — the intent's matrix prescribes the option sets verbatim as `VALID_*_PROVIDERS` + `debug`, and `local` is not allowlisted so a stored `"local"` is already rewritten at load; the spec's Decisions name the pre-existing picker/allowlist disagreement.
  - `[medium]` `[patch]` intent-alignment: the Kotlin matrix cells are about the ladder while the evidence is at the `KlarvoApi` seam, and the `cleanup` debug branch is executed by no test — the executable half is the Kotlin entry and was patched (`f1f07cb`, `d90f83a`); that `isRetryableCleanupFailure` is private and untestable without Robolectric is stated in the test's own KDoc.
  - `[medium]` `[patch]` intent-alignment: the React lists are pinned by a script outside any gate — same entry as the tripwire finding; closed (`f1f07cb`).
  - `[false]` `[reject]` intent-alignment: `expertMode` ON is measured against the preview mock, not the product flag — the React branch condition is the same boolean whatever its origin, so the gate's conclusion about the React surface holds; the Rust side of that flag is separately pinned by the config tests.
  - `[medium]` `[defer]` intent-alignment: two platforms are named but only the desktop layout branch is measured, and the JNI arity is checked by no executing gate — the JNI half is the deferred entry above; the layout half is refuted, because the new rows copy the reference row's own `isMobile` wrapper verbatim and the equality gate's scope is explicitly the `KSelect` only, which is platform-invariant.
  - `[low]` `[reject]` intent-alignment: inversion evidence is artifacts for the harness but source comments for Rust and Kotlin — I spot-checked one claim myself (removing `"debug"` from `VALID_LLM_PROVIDERS` reddened 3 tests with the expected messages), and the implementer re-ran five more for the new tripwire; recording every red run as an artifact is process evidence, not a code fix.
  - `[low]` `[patch]` intent-alignment: the shared fixture names symbols the amendment deleted — same entry as the fixture finding; fixed (`f1f07cb`).

## Design Notes

**Why wire-level injection.** A trait-level canned `Result` would be ~40 lines per twin and would test
nothing: the defect 13-2 fixes lives in the *mapping* (Kotlin returns `""` for an empty answer and
never inspects `finish_reason`; Rust errors on both). Injecting `(status, body)` and reusing each
twin's own mapping is the only shape in which the enabler reproduces the drift instead of hiding it.

**Why a synthesised `reqwest::Response` rather than a `(status, &str)` mapper.** The first build
extracted a mapper that re-parsed the body with `serde_json::from_str`. That silently changed which
error variant an undecodable body produces — `ResponseFormat` (non-retryable) instead of the live
path's `Request` (retryable) — which is precisely the D-M2 Desktop column the enabler exists to
reproduce. `reqwest::Response` has no public constructor for a network response, but
`impl<T: Into<Body>> From<http::Response<T>> for Response` (reqwest 0.12.28,
`src/async_impl/response.rs:456`, not feature-gated) builds one from an in-memory body; `.json()` then
collects those bytes and maps a decode failure through `crate::error::decode` into a genuine
`reqwest::Error`. So the debug provider traverses the *same* `status()`/`text()`/`json()` sequence the
real provider does, no socket is opened, and the retryability verdict comes from the real
`is_retryable_llm_error`. Rejected alternatives: a new `LlmError` variant (touches every `match` plus
both `is_retryable_*` — a real behaviour change); an ephemeral loopback HTTP listener (ships a server
socket in the product binary, flaky under sandboxes); promoting the `wiremock` dev-dependency to a
normal one (ships a test stack); making `ResponseFormat` retryable (changes live behaviour).

**Why a loopback request for `transport`.** `transport` means *no response at all*, so there is
nothing to synthesise. A real request to `http://127.0.0.1:1/` yields a genuine transport error
through the real client and keeps every byte on the device.

**Why STT debug is Rust-only.** ADR-0017 makes STT shared core. A Kotlin-side canned transcript would
forge the `__ERROR_*` sentinels rather than produce them. Cost, stated: an Android STT change needs
the slow `scripts/android-build.sh`, not `scripts/android-smoke.sh`.

**Why the provider rows use props rather than a new command.** `llmProvider`/`sttProvider` are written
today only by `save_settings`, from state that `SettingsPanel` owns. Any second writer — a narrow
`set_providers` command, or the Advanced panel calling `saveSettings` itself — leaves `SettingsPanel`'s
mount-time snapshot stale, and the next press of its Save button writes the stale value back over
`debug`. Passing the value and its setter down keeps exactly one owner, which rules the clobber out
structurally instead of guarding against it. Known, accepted consequence: the provider rows are saved
by the Settings footer while the scenario rows are saved by the Advanced panel's own footer, so both
can be visible at once. Both orders converge — `save_settings` rebuilds both provider slots from the
persisted config, `save_advanced_settings` rebuilds the cleanup slot — and because only the former
rebuilds the **STT** slot, the reproduction instructions say to press the Settings Save last.

**D6 (clipboard failure) is not served.** Clipboard failure is in the paste path, not the provider
path; injecting it would mean a second, unrelated debug seam. 13-2 records the Weg-2 downgrade.

## Verification

**Commands:**
- `cd src-tauri && cargo test --lib` -- expected: green, no API keys needed. Baseline at
  `baseline_revision` was 708 passing. The baseline exception applies: re-run a red check against
  `11f83818d498ba0ca6f5651e7fd7245f529f2e7e` before treating it as this story's failure.
- Device-free JVM gate -- sync `android/kotlin-src/*.kt` and `android/kotlin-test/*.kt` into
  `src-tauri/gen/android/app/src/{main,test}/java/com/klarvo/voice/` (delete-then-copy; story 7-8
  AC6a), ensure `testImplementation("org.json:json:20231013")` is in
  `src-tauri/gen/android/app/build.gradle.kts`, then
  `cd src-tauri/gen/android && ./gradlew :app:testUniversalDebugUnitTest --rerun-tasks`
  (`--rerun-tasks` is mandatory after a fixture-only edit). Expected: all suites green incl.
  `Adr0017BoundaryGuardTest`.
- `npm run build` -- expected: TS strict passes with the two new `AdvancedSettings` fields mirrored
  and the four new props typed.
- Desktop proxy gate: throwaway puppeteer script against `npm run preview` (port 1422) in real
  Chromium; evidence in `_bmad-output/implementation-artifacts/gate4-evidence/13-1/`. Start from the
  previous run's harness `gate4-evidence/13-1/debug-rows-smoke.mjs` and extend it. Click
  "Setup überspringen" first. It asserts, in one run:
  (a) all four rows absent from Advanced → System with `expertMode` off, present with it on;
  (b) `debug` absent from the normal provider picker in `RecordingAudioContent` **with `expertMode`
  both off and on**, and `AiProvidersContent` asserted to contain no provider picker at all (a
  positive assertion, not a zero-element scan reported as a pass);
  (c) the rendered option lists of all four rows read from
  `test-fixtures/debug-provider-scenario-vectors.json` and compared element-wise;
  (d) for **each of the four** rows and each drivable state of the control-states contract — idle,
  hover, focus, press, open, and the open listbox plus its selected and keyboard-focused option —
  `getComputedStyle` equals the named reference instance's on background-color, border-color,
  border-radius, padding, font-size, color and box-shadow, compared like-for-like (`classList.contains`,
  never `className.includes`, and selected-vs-selected). A mismatch on any property fails the gate.
  The run reports ONE check count, derived from the result records, not a hand-written number.
  Inversion: the harness must be shown RED once, using the *same* assertion pointed at a deliberately
  different control (the raw `<select>` at `AdvancedSettingsPanel.tsx:394-403`), before the green run
  is believed; the visibility and picker checks each get their own inversion too.
  It cannot observe persistence (preview writers are no-ops), pixels, or anything Rust — say so.
- Harness traps already found, do not rediscover: `KSelect`'s Escape handler does not
  `stopPropagation`, so Escape closes the whole Settings panel — close a listbox by clicking the
  trigger again; `className.includes("bg-klarvo-surface-2")` also matches `hover:bg-klarvo-surface-2`.

**Manual checks (H+ reproduction path, Andi):**
- 🖥️ Windows release build via `scripts/windows-build.sh`: Settings → Advanced → Expert mode on →
  System → set `LLM Provider` = `debug` and `Debug LLM Scenario` = `empty` → press the **Settings**
  Save last → dictate a short sentence → observe.
- 📱 Fresh APK via `scripts/android-build.sh` (not `android-smoke.sh` — the JNI signature changed):
  same path on the Xiaomi. Requires a licensed/trial state and a configured Groq key.
- Keep the dictation short: cleanup chunks above 400 characters, the canned answer is returned **per
  chunk**, and on Android the chunked path changes the `malformed` verdict (the ladder fires there).
- ⚠️ A downloaded local Whisper model can silently rescue a debug STT error via
  `try_local_whisper_fallback` — check that first if an STT scenario appears not to fire.
- Which scenario fired is readable in `klarvo.log` on both platforms — no computer needed.

## Auto Run Result

Status: done
Blocking condition: none

### 2026-09-20 implementation run

Gates, each re-run from the tree after the last edit:

- `cd src-tauri && cargo test --lib` — **743 passed, 0 failed** (708 at `baseline_revision`).
- Device-free JVM gate (`android/kotlin-{src,test}` delete-then-copied into
  `src-tauri/gen/android/app/src/{main,test}/java/com/klarvo/voice/`, `ANDROID_HOME` set,
  `./gradlew :app:testUniversalDebugUnitTest --rerun-tasks`) — **25 suites, 209 tests, 0 failures**,
  counted from `app/build/test-results/testUniversalDebugUnitTest/*.xml`, including
  `Adr0017BoundaryGuardTest` and `DebugProviderScenarioTest` (15 tests).
- `npm run build` — TS strict passed with the four new props and the two mirrored fields.
- Desktop proxy gate vs `npm run preview` — green run **66 checks, 0 failed**; inversion run **3/3
  groups red** as required. Evidence and the coverage statement:
  `gate4-evidence/13-1/{report.md,inversion-report.md,verdict.md}` plus the raw style maps and
  screenshots. `git status` on `src/tauri-commands.ts` was verified clean of the throwaway edit
  after both runs.
- Every inversion this build claims was executed, not asserted (Implementation Notes §8).

Not run by this session, and therefore not claimed: the review lens set (`review` frontmatter is
empty), the Windows release build, and anything on a device. The two manual H+ checks in
**Verification** are unchanged and still Andi's.

### 2026-09-20 review pass (this session)

**What was implemented.** A `debug` provider value for LLM cleanup (twinned: `llm/mod.rs` +
`KlarvoApi.kt`) and for STT (Rust only, ADR-0017, selected in `groq_jni.rs`), returning a canned
`(status, body)` pair that is run through each twin's *own* response mapping. On Rust the pair is
turned into a real in-memory `reqwest::Response` (`http = "1"` + `From<http::Response<T>>`) and handed
to one behaviour-preserving extraction per twin, so `malformed` produces the genuine
`LlmError::Request` and the production ladder fires — D-M2's mechanism, not a look-alike. Four
`expertMode`-gated `KSelect` rows in Settings → Advanced → System select the two providers and the two
scenarios; the provider rows write `SettingsPanel`'s state through props, keeping one client-side
owner of `llmProvider`/`sttProvider`.

**Files changed**

| File | What changed |
|---|---|
| `src-tauri/Cargo.toml` + `Cargo.lock` | `http = "1"` as a direct dep (already resolved at 1.4.0; zero new crates) |
| `src-tauri/src/llm/mod.rs` | one verbatim extraction `map_chat_http_response(reqwest::Response)`; `DEBUG_PROVIDER_NAME`, `DEBUG_TRANSPORT_URL`, `debug_canned_response`, `debug_llm_canned_wire`, `DebugCleanup`; the React source-text tripwire test |
| `src-tauri/src/stt/mod.rs` | the twin extraction `map_transcription_http_response`; `debug_stt_canned_wire`, `DebugStt`; the `select_stt_provider` tests |
| `src-tauri/src/stt/groq_jni.rs` | module cfg moved from the module to its items; `select_stt_provider` extracted so the Android debug branch is test-reachable; `nativeTranscribe` gained two params |
| `src-tauri/src/config/mod.rs` | `debugLlmScenario` / `debugSttScenario` (serde default `"ok"`); `"debug"` on both provider allowlists |
| `src-tauri/src/pipeline.rs` | explicit `debug` arms in `resolve_stt_provider`, `cleanup_provider_for`, `resolve_cleanup_provider`; `resolve_fallback_provider` untouched |
| `src-tauri/src/commands/settings.rs` | `debug_llm_scenario` clause in `cleanup_provider_reload_needed` |
| `android/kotlin-src/.../KlarvoApi.kt` | `mapCleanupResponse` + `debugCleanupOrNull` extractions, `debugCannedWire`, `parseDebugScenario`, the `DEBUG_PROVIDER_NAME` resolve arm, two `Config` fields |
| `android/kotlin-src/.../GroqSttBridge.kt`, `KlarvoOverlayService.kt` | the two new JNI params, carried through uninspected |
| `android/kotlin-test/.../DebugProviderScenarioTest.kt` | new, 17 tests |
| `src/components/AdvancedSettingsPanel.tsx` | the four `KSelect` rows behind `expertMode`, four new props |
| `src/components/SettingsPanel.tsx` | props wired to the one state owner; re-seed guard; `handleSttProviderChange` guard |
| `src/types.ts`, `src/tauri-commands.ts` | the two mirrored `AdvancedSettings` fields |
| `test-fixtures/debug-provider-scenario-vectors.json` + `README.md` | 16 vectors, read by Rust, JVM and the proxy harness |
| `_bmad-output/.../gate4-evidence/13-1/` | harness, reports, verdict, style maps, screenshots |

**Review findings.** Four lenses ran (blind-hunter, edge-case-hunter, verification-gap,
intent-alignment); 36 findings — high 0, medium 11, low 20, false 5, maybe-false 0. Full rows in
`## Review Triage Log`.

- **Patched: 7 entries — medium 3, low 4.**
  1. *(medium)* the React option arrays and both TS `debug` guards were pinned by no recurring gate →
     `llm::tests::spec_react_option_arrays_and_debug_guards_are_pinned_to_rust`, five inversions RED.
  2. *(medium)* `KlarvoApi.cleanup`'s debug short-circuit was executed by no test and its placement
     above `URL(provider.url)` was unpinned → `debugCleanupOrNull` extraction + a source-text ordering
     tripwire, two inversions RED.
  3. *(medium)* `handleSttProviderChange` was a second writer that clobbered `llmProvider = "debug"` →
     guarded in functional form, same shape as the re-seed guard.
  4. *(low)* the `transport` probe used a bare `reqwest::Client::new()` → the shipped builder
     (`connect_timeout(15s)`, `timeout(30s)`) plus `.no_proxy()` at both sites.
  5. *(low)* the fixture named two deleted mappers → corrected to the `*_http_response` names.
  6. *(low)* the fixture pinned reqwest's Display string unversioned → reqwest 0.12.28 named.
  7. *(low)* "deterministic regardless of what was dictated" ignored the trivial-chunk guard →
     reworded in both twins.
- **Deferred: 1 entry** *(medium, frontmatter `deferred`)* — no device-free gate checks that the two
  halves of the 9-parameter `nativeTranscribe` signature still agree; `#[no_mangle]` binds by short
  name, so a one-sided edit misbinds silently. Pre-existing gap class; closing it needs an adb step.
- **Rejected, with reasons:**
  - `debug` renders as the *trigger label* of the normal picker once selected (`FormControls.tsx:93`
    falls back to the raw value) — low; the intent's Never clause and its AC are about the option
    list, which stays clean, and hiding the active value would make the control lie.
  - the harness dropped the predecessor's body-text `debug` scan — low; the harness never sets a row
    to `debug`, so the old scan could not have fired either.
  - `debugSttScenario` does not hot-reload — low; recorded operationally, and the manual checks
    prescribe pressing the Settings Save last, which rebuilds both slots.
  - two stacked Save footers with different scopes — low; the intent mandates both "no second writer"
    and "no hint line, no prose", which together force this shape; a wrong press is self-revealing.
  - INVERSION-1 covers `idle` on one row only — low; the harness prints `1/10 RED` itself, and a raw
    `<select>` has no listbox to point the other states at.
  - blank Groq key / unlicensed rewrite of `debug` → `groq` — false; the intent names both verbatim
    as 13-4's.
  - `LABEL_CLS` vs `LABEL_CLS_M` — false; the intent prescribes `LABEL_CLS` and excludes the label
    from the gate's scope.
  - the Advanced STT row uses the raw setter, so `local` there leaves a cloud LLM — low; routing it
    through `handleSttProviderChange` would reintroduce the clobber patch 3 removes.
  - the UI → `save_settings` → `config.json` seam is untested — low; the merge is a provider-agnostic
    passthrough and its only `debug`-specific hazard (normalization) is pinned twice.
  - the provider rows are a second, differently-optioned picker (`anthropic`, `local`) — false; the
    intent's matrix prescribes `VALID_*_PROVIDERS` + `debug` verbatim.
  - `expertMode` ON measured against the preview mock — false; the React branch condition is the same
    boolean whatever its origin, and the Rust side of the flag is pinned by the config tests.
  - only the desktop layout branch is style-measured — false; the rows copy the reference row's own
    `isMobile` wrapper verbatim and the gate's scope is the `KSelect` only, which is invariant.
  - inversion evidence is prose for Rust/Kotlin — low; one claim was spot-checked by this session and
    five more were re-run for the new tripwire.

**Verification performed by this session** (every command re-run against the final tree, `d90f83a`):

- `cd src-tauri && cargo test --lib` — **744 passed, 0 failed** (708 at `baseline_revision`).
- Device-free JVM gate, after verifying `android/kotlin-{src,test}` and
  `src-tauri/gen/android/app/src/{main,test}/java/com/klarvo/voice/` are byte-identical, with
  `ANDROID_HOME` set and `--rerun-tasks` — **25 suites, 211 tests, 0 failures**, counted from the
  result XMLs; `Adr0017BoundaryGuardTest` 3/3 green, `DebugProviderScenarioTest` 17/17 green.
- `npm run build` — TS strict green.
- Desktop proxy gate against the running `npm run preview` (port 1422, real Chromium): green run
  **66 ordinary checks, 0 failed, exit 0**; inversion run **3/3 groups RED, exit 0**. `git status`
  clean of the throwaway `src/tauri-commands.ts` edit after both runs.
- Inversion spot-check by this session: removing `"debug"` from `VALID_LLM_PROVIDERS` reddened
  `spec_debug_provider_survives_normalization`,
  `spec_debug_provider_and_scenarios_survive_a_real_config_file_round_trip` and
  `spec_debug_provider_option_lists_match_the_config_allowlists` with the expected messages; the file
  was restored and the suite re-confirmed green.
- **Matrix test audit:** every row of the I/O & Edge-Case Matrix has a covering test that ran and
  passed — LLM `ok`/`empty`/`truncated`/`malformed`/`http429`/`http5xx`/`transport` via
  `llm::tests::spec_debug_llm_*` (the `malformed` vector asserting `LlmError::Request` **and** the
  real `is_retryable_llm_error(..) == true`) and `DebugProviderScenarioTest` for the Kotlin column;
  STT `ok`/`empty`/`malformed`/`http429`/`http5xx`/`transport` via `stt::tests::spec_debug_stt_*`;
  STT `truncated` via `spec_debug_stt_truncated_is_not_offered`; both provider-option rows via
  `spec_debug_provider_option_lists_match_the_config_allowlists`.

**What this run does NOT claim.** No pixels, no aesthetics, no device. Nothing on the Xiaomi and
nothing on Windows was executed; Android Rust compilation is unverified on this host (no NDK, and
installing one is forbidden). The proxy gate proves wiring, structure and computed-style equality on
the desktop React surface only, and cannot observe persistence because preview writers are no-ops.
The two manual H+ checks in **Verification** are unchanged and still Andi's — and because the JNI
arity changed, the Android one needs a fresh APK via `scripts/android-build.sh`, not
`scripts/android-smoke.sh`.

**Follow-up review recommended: true.** Named risk: three `medium` entries were patched in the last
round, and two of the three fixes are *form-sensitive source-text tripwires* — the Kotlin ordering
assertion and the React array/guard assertion both match code by regex, so a harmless reformat of
`KlarvoApi.cleanup`'s prologue or of the four `.tsx` arrays reddens them for a non-product reason,
and neither has been exercised by anything other than its own inversion. None of the three patched
surfaces (the Kotlin `cleanup` prologue, the `handleSttProviderChange` guard, the TSX arrays) has run
on a real device in this story.

### Relationship to the predecessor spec

This file supersedes `spec-13-1-debug-test-provider-both-twins.md` (status `blocked`, intent gap,
revision `6a50b88`), which stays readable as the record of the first build and its review. Same story
id, same intent; opened as `-2` because the predecessor's status is not `draft`. Only one of the two
is live — the conductor retires the predecessor when it books this one.

### Residual risks carried forward from the first build (each now has an owner above)

| Risk recorded 2026-09-20 | Where it is closed in this spec |
|---|---|
| `high` — the Android `debug` STT branch is reached by no executing test; inverting the comparison keeps every gate green | `select_stt_provider` extraction + its Rust test and inversion (empty audio: `debug` → canned mapping, `groq` → `EmptyAudio`, neither touches the network) |
| `medium` — nothing pins the React option lists to the Rust/Kotlin scenario tables | two new fixture vectors read by a Rust test (vs `VALID_*_PROVIDERS`) **and** by the proxy harness (vs the DOM) |
| `medium` — only the first new row was style-compared | the gate runs for each of the four rows |
| `medium` — both picker scans ran with Expert mode off; the AI-Providers scan read 0 elements and was reported as cleared | picker scan runs in both `expertMode` states; `AiProvidersContent` gets a positive "no provider picker" assertion |
| `medium` — the inversion run compared a different idle state than the green run; visibility and picker checks had no inversion | the inversion uses the *same* assertion against the raw `<select>`; visibility and picker checks each get their own |
| `medium` — `debugSttScenario` does not hot-reload while `debugLlmScenario` does | unchanged in code; handled operationally — `save_settings` rebuilds both slots, so the reproduction path presses the Settings Save last, and the manual checks say so |
| `medium` — the AC claimed "verified by re-reading `config.json`" but nothing read the file back | a config test that writes and re-reads an actual file |
| `medium` — stale `.so` misbinds the renamed-arity `nativeTranscribe` by short name instead of throwing | stated in the Code Map and the manual checks (`android-build.sh`, not `android-smoke.sh`) |
| `low` — `verdict.md` reported 23 checks while `report.md` listed 24 | the harness reports one count derived from the result records |
| `low` — a stray "well-formed answer / `usage` present" comment sat above the `"empty"` arm | fix while amending `debug_llm_canned_wire` |
| `low` — `"debug"` is a literal in `pipeline.rs` and both allowlists while `groq_jni.rs` uses the constant | use `DEBUG_PROVIDER_NAME` consistently while amending those files |
| rejected, do not re-file | the `.focus-klarvo` always-on ring and the raw `<select>` Log Level row are **already** in `docs/backlog.md` (l.1122, l.1126, filed by `4aa37d1`); neither is in frontmatter `deferred`, by intent. *(Implementation added ONE new `deferred` entry that the review had not seen: `groq_jni`'s never-executed test module — Implementation Notes §4.)* |

### Decisions

- **Routing:** opened as `-2` rather than resumed in the predecessor file, per the workflow's rule for
  an existing spec whose status is not `draft`. Alternative (editing the blocked file) rejected: it
  would overwrite the record of the reverted build and its 39-finding review.
- **`malformed` mechanism:** synthesise a `reqwest::Response` in-process (`http = "1"` +
  `From<http::Response<T>>`) and run it through one verbatim extraction of the real response half, so
  an undecodable body yields the real `LlmError::Request`. Alternatives (new error variant / loopback
  listener / promoting `wiremock` / making `ResponseFormat` retryable) all rejected in Design Notes —
  each either changes real behaviour or ships machinery in the product binary.
- **One extraction per twin, not three helpers.** The patch's `map_chat_response(status, &str)` is
  removed rather than kept beside the new one: leaving it would leave a second, divergent mapping in
  the tree that reports a different variant than the live path.
- **STT `malformed` stays non-retryable.** Andi's answer names D-M2, which is an LLM row. The real STT
  path parses with `serde_json::from_slice`, so its undecodable body genuinely maps to
  `ResponseFormat`. The rule "the twin's own mapping decides" produces the asymmetry; it is pinned as
  a finding, not normalised away.
- **Provider-row option sets = the Rust allowlist constants.** Derived from a shipped constant, not
  composed: it is exactly the set `migrate_and_normalize` accepts, and it lets the row switch *back*
  out of `debug`, without which the control would be a one-way door. Alternative (a two-option
  debug/off row) rejected: "off" has no source. Noted, not fixed: the allowlist and the normal picker
  already disagree today (`anthropic` is allowlisted but unpickable, `local` is pickable for LLM but
  not allowlisted) — pre-existing, out of scope.
- **Surface plumbing by props, not a new command** (Design Notes) — keeps one client-side owner of
  `llmProvider`/`sttProvider`; the `SettingsPanel.tsx:286-300` re-seed fix is part of it, because
  without it a persisted `"debug"` is rewritten on every refresh.
- **Label class `LABEL_CLS`, and the equality gate covers the `KSelect` only.** The reference row uses
  `LABEL_CLS_M`; every row in `AdvancedSettingsPanel` uses `LABEL_CLS`. Matching the panel's own
  neighbours is the right consistency axis, and the label span is not part of the reused component.
- **Wording is derived, never composed:** row labels are the config keys in the panel's existing title
  case (`LLM Provider`, `STT Provider`, `Debug LLM Scenario`, `Debug STT Scenario`, cf. the shipped
  `Log Level`); option labels are the config values verbatim; no hint line. The degree of freedom is
  removed rather than exercised, so no user-facing prose is invented here.
- **Canon gap, shipped precedent:** `KSelect` from `src/components/settings/FormControls.tsx` sources
  hover/focus/pressed/disabled/open, which the canon leaves undefined for a select. Reused verbatim;
  gate = computed-style equality against `LanguageContent.tsx:56-63`.
- Scenario carried in two keys (`debugLlmScenario`, `debugSttScenario`), not one — D9 needs a debug
  STT while the LLM behaves normally.
- Provider value is the string `debug` on `llmProvider`/`sttProvider`, not a separate
  `advanced.debugProvider` override.
- STT debug implemented once in Rust (shared core), LLM debug twinned — follows ADR-0017.
- A debug 429/5xx/transport/malformed **does** trigger the production ladder and therefore a real call
  to the user's configured fallback provider. Intended — it is the observable — and named here so it
  is not mistaken for a leak.
- License gate untouched; `debug` therefore needs a licensed/trial device, and on Android a non-blank
  Groq key. 13-4 owns that gate.
