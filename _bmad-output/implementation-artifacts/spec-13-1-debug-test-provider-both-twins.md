---
title: '13-1 Debug test provider, both twins'
type: 'feature'
created: '2026-09-19'
status: 'blocked'
route: 'full'
route_source: 'auto'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
followup_review_recommended: false
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-13-context.md'
  - '{project-root}/docs/adr/0016-android-path-parity-strategy.md'
  - '{project-root}/docs/adr/0017-shared-core-stt-path.md'
  - '{project-root}/test-fixtures/README.md'
warnings: ['oversized']
deferred: []
---

<intent-contract>

## Intent

**Problem:** Four rows of drift audit #2 (D2/D-H19 empty LLM answer, D3/D-M16 truncated answer,
D9/D-M5+M6 empty STT result, D10/D-M2 malformed answer) are agent-only today: Andi cannot provoke
a misbehaving provider on his own Windows machine or his Xiaomi without standing up a fake API.
Story 13-2 needs those states reproducible before it changes the guards (ADR-0016 Amd 4, H+).

**Approach:** A `debug` provider value for LLM cleanup and for STT in both twins that returns a
**canned wire response** (HTTP status + body) chosen from `advanced.debug*Scenario`, fed through
each twin's own existing response-mapping code. Selectable on both devices in Settings → Advanced
behind the existing `expertMode` gate. Never a default, never a fallback candidate, never an
option in the normal provider picker.

## Boundaries & Constraints

**Always:**
- **Canned at the wire, not at the trait.** The debug provider yields `(status, body)` and lets the
  twin's real mapping decide the outcome. The mapping is what 13-2 tests; a pre-mapped `Result`
  would bypass it — and the twins map differently today (Rust reads `finish_reason == "length"`,
  Kotlin has no truncation detection at all). That divergence must stay visible.
- **STT debug lives in Rust only** (ADR-0017: STT is shared core). `Adr0017BoundaryGuardTest` stays
  green; no STT request/guard logic enters Kotlin.
- Twin rule: the LLM half lands in `llm/mod.rs` **and** `KlarvoApi.kt` (twinned); the STT half lands
  once in `stt/` (shared).
- Every new vector gets an inversion check that is RED at writing time (G-B).
- Provider strings must be added to the config allowlists or they are normalized away at load.

**Never:**
- No change to real provider behaviour. The only edit to shipping provider code is a
  **behaviour-preserving extraction** of the response→result mapping into a callable function.
- Not added to `pipeline::resolve_fallback_provider`'s candidate array nor to
  `KlarvoApi::cleanupFallbackCandidates` — both lists stay literal and closed (Epic 12 FR2: Groq is
  never a cleanup fallback).
- Never the default; no `debug` option in the normal provider picker
  (`RecordingAudioContent.tsx`, `SettingsPanel.tsx`, `AiProvidersContent.tsx`).
- The license gate is NOT touched — 13-4 owns it. Consequence, recorded not fixed: `debug` is a
  non-free provider, so it works on a **licensed/trial device only**, and Kotlin's `readConfig`
  still requires a non-blank Groq key or it returns `null` and the bubble refuses to record.
- No clipboard-failure injection. D6 is **not** served by this mechanism (see Design Notes) —
  13-2 records the Weg-2 downgrade for D6.
- No remote telemetry; no host mutation; no `cargo check --target x86_64-pc-windows-gnu`
  (retired gate).

## I/O & Edge-Case Matrix

`advanced.debugLlmScenario` / `advanced.debugSttScenario`, camelCase, serde `default = "ok"`.

| Scenario | Canned wire response | Expected Output / Behavior | Error Handling |
|---|---|---|---|
| `ok` | 200, valid body, canned text | Cleanup/transcript returns the canned text | No error expected |
| `empty` | 200, `content: ""` | Rust: `LlmError::ResponseFormat("Empty content in response")`. Kotlin: returns `""` (today unguarded → pastes empty) | Divergence is the finding D2 consumes |
| `truncated` | 200, `finish_reason: "length"` | Rust: `LlmError::OutputTruncated`. Kotlin: undetected, returns the partial text | Divergence is the finding D3 consumes |
| `malformed` | 200, body without `choices` | Rust: `LlmError::ResponseFormat`. Kotlin: `JSONException` → skips the ladder | D10's observable |
| `http429` | 429, error body | Retryable on both twins → production fallback ladder fires | Real call to the user's configured fallback provider — intended |
| `http5xx` | 503, error body | As `http429` | As above |
| `transport` | no response: real request to `http://127.0.0.1:1/` | Genuine `reqwest::Error` / `IOException`, no `HTTP nnn` in the message | Loopback only — no byte leaves the device |
| STT `empty` | 200, empty text | Rust maps to `SttError::ResponseFormat`; pipeline currently ends `Stopped` | D9's observable |

</intent-contract>

## Code Map

**Rust — LLM**
- `src-tauri/src/llm/mod.rs::CleanupProvider` — trait, `#[async_trait]`, one required method
  `cleanup(raw_text, style, dictionary_terms, custom_prompt) -> Result<CleanupResult, LlmError>`.
  Add `DebugCleanup` here.
- `src-tauri/src/llm/mod.rs::OpenAiCompatibleCleanup::send_request` — **extract** the
  response→`Result` half into a pure `fn map_chat_response(status, body) -> Result<CleanupResult, LlmError>`;
  both the real provider and `DebugCleanup` call it. Behaviour-preserving, no logic change.
- `LlmError` variants to reuse: `ApiError { status, message }` (429/5xx), `OutputTruncated`,
  `ResponseFormat(String)` (empty *and* malformed collapse here), `Request(reqwest::Error)`
  (transport — **no public constructor**, hence the loopback request).
- `src-tauri/src/pipeline.rs::cleanup_provider_for` and `::resolve_cleanup_provider` — catch-all
  `_ => deepseek`; both need an explicit `"debug"` arm or the value silently becomes DeepSeek.
- `src-tauri/src/pipeline.rs::resolve_fallback_provider` — candidate array; **do not touch**.
- `src-tauri/src/llm/mod.rs::effective_cleanup_model` — its own `match provider` default-model
  table; needs a `debug` entry.
- `src-tauri/src/llm/mod.rs::chunked_cleanup` — wraps cleanup above 400 chars; a canned answer is
  returned once **per chunk**. Keep dictations short when reproducing, or assert per-chunk.

**Rust — STT (shared, serves Android too)**
- `src-tauri/src/stt/mod.rs::SttProvider` — `transcribe(audio, language, prompt) -> Result<String, SttError>`.
  Add `DebugStt`. `SttError` has **no** truncation variant.
- `src-tauri/src/pipeline.rs::resolve_stt_provider` — catch-all `_ => GroqWhisper`; needs a `"debug"` arm.
- `src-tauri/src/stt/groq_jni.rs::Java_com_klarvo_voice_GroqSttBridge_nativeTranscribe` — the Android
  STT entry. The debug branch belongs here so Kotlin stays clean; it must emit the **same** sentinel
  strings the production path emits.
- `src-tauri/src/pipeline.rs::try_local_whisper_fallback` — ⚠️ can silently rescue a debug STT error
  where a local model is downloaded. Name this in the reproduction instructions.

**Rust — config**
- `src-tauri/src/config/mod.rs::VALID_LLM_PROVIDERS` / `::VALID_STT_PROVIDERS` — add `"debug"` to
  both, else `migrate_and_normalize` rewrites it to the default at load and the next save persists
  the rewrite.
- `src-tauri/src/config/mod.rs::AdvancedSettings` — `#[serde(rename_all = "camelCase")]`, every field
  `#[serde(default)]`; add `debug_llm_scenario` / `debug_stt_scenario`. No migration needed. No
  `deny_unknown_fields`, so a key without a Rust field is silently dropped on the next write.
- `src-tauri/src/commands/settings.rs::save_advanced_settings` — replaces the whole block, then
  `::hot_reload_cleanup_provider` → provider swap **without restart**.
- `src-tauri/src/commands/settings.rs::cleanup_provider_reload_needed` — diffs only the four
  `llm_model_*` fields; a scenario change will NOT rebuild the provider. Read the scenario at call
  time (preferred) or extend this predicate.

**Kotlin — LLM twin**
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt::cleanup` — the single funnel for every provider.
  Debug branch as an early return keyed on `provider.providerName == "debug"`, placed **before**
  `URL(provider.url)`. Extract the body-parse/throw half into an `internal` pure function so a JVM
  test can drive it (house pattern: `::parseLlmModelOverride`).
- `KlarvoApi::resolveLlmProvider` — ⚠️ its `else ->` maps any unknown string to DeepSeek; an explicit
  `"debug" ->` arm is mandatory for reachability.
- `KlarvoApi::cleanupFallbackCandidates` — literal 3-entry list (deepseek → openai → openrouter);
  **do not touch**; debug is out by construction.
- `KlarvoApi::readConfig` — flat `optString("llmProvider", …)`; license gate rewrites to `"groq"`
  when unlicensed; hard gate returns `null` when the Groq key is blank. Add a pure
  `parseDebugScenario(json, key)` beside `::parseMinRecordingMs`.
- `KlarvoOverlayService::isRetryableCleanupFailure` — regex `HTTP (\d{3})`; `null|429|>=500` →
  fallback. `KlarvoApi::collectChunkResults` re-throws chunk causes as `IOException`.
- `KlarvoApi.Config` — new fields go **last** (positional construction in `readConfig`).

**React — surface**
- `src/components/AdvancedSettingsPanel.tsx::AdvancedSettingsPanel` — the Advanced drill-down,
  `embedded`, own `getAdvancedSettings`/`saveAdvancedSettings`, own `set(key, value)`. Already
  renders on **both** platforms (`SETTINGS_CATEGORIES` entry `advanced` carries no `desktopOnly`).
- `AdvancedSettingsPanel.tsx::expertMode` — the house pattern for hiding footguns; gate the new rows
  behind it exactly as the Audio section is gated.
- `src/components/settings/FormControls.tsx::KSelect` — the control to reuse verbatim.
- ⚠️ `src/components/SettingsPanel.tsx::handleSave` — the documented stale-whole-block trap: re-read
  `getAdvancedSettings()` immediately before the merge; on a failed re-read, skip the advanced save.
- `src/types.ts::AdvancedSettings` and `src/tauri-commands.ts::MOCK_ADVANCED_SETTINGS` — mirrors that
  must gain the two fields (TS strict gates `npm run build`).
- `android/kotlin-src/com/klarvo/voice/MainActivity.kt::MainActivity : TauriActivity` — hosts the same
  React bundle, so the same `save_advanced_settings` writes the same `config.json` the Kotlin service
  reads. No separate Android settings surface exists.

**Tests**
- Rust doubles to copy: `llm::tests::MockCleanupProvider`, `pipeline::tests::FakeStt` /
  `::FakeCleanup` / `::CleanupBehavior`.
- Kotlin conventions: JUnit 4, **no mocking library**, pure seams only (`android.util.Log` throws
  "not mocked"), a "what this does NOT cover" KDoc block. Nearest examples:
  `CleanupFailureDeliveryTest.kt`, `LlmModelOverrideConfigTest.kt`.
- `test-fixtures/README.md` — the reader ledger; a new fixture adds a row.

## Tasks & Acceptance

**Execution:**
- [ ] `src-tauri/src/llm/mod.rs` -- extract `map_chat_response(status, body)` from
      `OpenAiCompatibleCleanup::send_request` (behaviour-preserving), then add `DebugCleanup`
      implementing `CleanupProvider` over a canned `(status, body)` per scenario -- so the real
      mapping decides the outcome.
- [ ] `src-tauri/src/stt/mod.rs` -- add `DebugStt` implementing `SttProvider` the same way;
      `transport` performs a real request to `http://127.0.0.1:1/`.
- [ ] `src-tauri/src/pipeline.rs` -- `"debug"` arms in `cleanup_provider_for`,
      `resolve_cleanup_provider`, `resolve_stt_provider`. Fallback candidate arrays untouched.
- [ ] `src-tauri/src/config/mod.rs` -- `"debug"` into both allowlists; `debug_llm_scenario` /
      `debug_stt_scenario` on `AdvancedSettings`; `effective_cleanup_model` debug entry.
- [ ] `src-tauri/src/stt/groq_jni.rs` -- debug branch in `nativeTranscribe` emitting the production
      sentinel strings, so Android STT scenarios need no Kotlin logic.
- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` -- `"debug" ->` arm in `resolveLlmProvider`;
      early-return debug branch in `cleanup`; extract the body-parse half into an `internal` pure
      function; `parseDebugScenario`; two `Config` fields appended last.
- [ ] `src/components/AdvancedSettingsPanel.tsx` + `src/types.ts` + `src/tauri-commands.ts` -- two
      `KSelect` rows under the existing `expertMode` gate, reusing an existing row's markup verbatim.
      **Blocked — see Auto Run Result.**
- [ ] `test-fixtures/debug-provider-scenario-vectors.json` + `test-fixtures/README.md` -- one vector
      per scenario with each twin's expected mapping, incl. the Kotlin truncation blind spot as an
      explicit expected-divergence entry (pattern: `twin-constants-vectors.json`).
- [ ] `src-tauri/src/llm/mod.rs`, `src-tauri/src/stt/mod.rs` -- inline `#[cfg(test)]` tests reading
      the fixture, one per scenario; each with its inversion check.
- [ ] `android/kotlin-test/com/klarvo/voice/DebugProviderScenarioTest.kt` -- JVM twin reading the same
      fixture; asserts the Kotlin mapping incl. the documented divergences.

**Acceptance Criteria:**
- Given a licensed device with `llmProvider = "debug"` and `advanced.debugLlmScenario = "empty"`,
  when Andi dictates a short sentence, then the pre-13-2 behaviour is observable unchanged on each
  platform (Desktop: cleanup error path; Android: an empty paste) — proving the enabler.
- Given `sttProvider = "debug"` and `debugSttScenario = "empty"`, when Andi dictates, then the empty
  STT result reaches the pipeline and `klarvo.log` names the scenario that fired.
- Given any scenario, when the run happens, then `llmProvider`/`sttProvider` survive a settings save
  and an app restart (allowlist round-trip), verified by re-reading `config.json`.
- Given the normal provider picker on either platform, when it is opened, then `debug` is **not**
  among its options; and given `expertMode` is off, then the debug rows are absent from the tree.
- Given `debugLlmScenario = "http429"`, when cleanup runs, then the production ladder fires
  deepseek → openai → openrouter and `debug` never appears as a fallback candidate.
- Given the ADR-0017 tripwire, when the JVM gate runs, then `Adr0017BoundaryGuardTest` is green.

## Implementation Notes

## Spec Change Log

## Review Triage Log

## Design Notes

**Why wire-level injection.** A trait-level canned `Result` would be ~40 lines per twin and would
test nothing: the defect 13-2 fixes lives in the *mapping* (Kotlin returns `""` for an empty answer
and never inspects `finish_reason`; Rust errors on both). Injecting `(status, body)` and reusing each
twin's own mapping is the only shape in which the enabler reproduces the drift instead of hiding it.

**Why a loopback request for `transport`.** `reqwest::Error` has no public constructor, so a synthetic
transport error is impossible without a new `LlmError`/`SttError` variant — which would touch every
`match` over those enums plus `is_retryable_*`. A real request to `http://127.0.0.1:1/` yields a
genuine transport error through the real client code and keeps every byte on the device.

**Why STT debug is Rust-only.** ADR-0017 makes STT shared core. A Kotlin-side canned transcript would
forge the `__ERROR_*` sentinels rather than produce them, and would put STT outcome logic back in
Kotlin — the grey zone the tripwire exists to police. Cost, to state in the story record: an Android
STT change needs the slow `scripts/android-build.sh`, not `scripts/android-smoke.sh`.

**D6 (clipboard failure) is not served.** Clipboard failure is in the paste path, not the provider
path; injecting it would mean a second, unrelated debug seam. 13-2 records the Weg-2 downgrade.

## Verification

**Commands:**
- `cd src-tauri && cargo test --lib` -- expected: green, no API keys needed. Baseline exception
  applies against `3980b7051881e6a453bd8e33319a183259128adb`.
- Device-free JVM gate -- sync `android/kotlin-src/*.kt` and `android/kotlin-test/*.kt` into
  `src-tauri/gen/android/app/src/{main,test}/java/com/klarvo/voice/` (delete-then-copy; story 7-8
  AC6a), ensure `testImplementation("org.json:json:20231013")` is in
  `src-tauri/gen/android/app/build.gradle.kts`, then
  `cd src-tauri/gen/android && ./gradlew :app:testUniversalDebugUnitTest --rerun-tasks`
  (`--rerun-tasks` is mandatory after a fixture-only edit -- Gradle otherwise reports a stale green).
  Expected: all suites green incl. `Adr0017BoundaryGuardTest`.
- `npm run build` -- expected: TS strict passes with the two new `AdvancedSettings` fields mirrored.
- Desktop proxy: throwaway puppeteer script vs `npm run preview` (port 1422), evidence in
  `_bmad-output/implementation-artifacts/gate4-evidence/13-1/`. Click "Setup überspringen" first.
  Asserts: debug rows absent with `expertMode` off, present with it on; `debug` absent from the
  normal provider picker; computed styles of the new `KSelect` equal to a named existing `KSelect`
  in the same panel. It cannot observe persistence (preview writers are no-ops) or anything Rust.

**Manual checks (H+ reproduction path, Andi):**
- 🖥️ Windows release build via `scripts/windows-build.sh`: Settings → Advanced → Expert mode on →
  pick `empty` → dictate → observe.
- 📱 Fresh APK: same path on the Xiaomi. Requires a licensed/trial state and a configured Groq key.
- Which scenario fired is readable in `klarvo.log` on both platforms — no computer needed.

## Auto Run Result

Status: blocked
Blocking condition: intent gap

### Open questions (planning halted here)

The story's on-device selector is an interactive control, so the response-state check applies. The
binding canon (`docs/design/overhaul/source/assets/klarvo.css`, ADR-0019) was grepped for every
`:hover|:focus|:active|:disabled|.on|.open|aria-` selector. It defines: toggle idle-off/idle-on
(`.toggle`, `.toggle.on`), select **idle only** (`.select`), input focus (`.input:focus`, the only
focus rule in the canon), button hover (`.btn:hover`), row hover (`.srow:hover`), a spinner, and
danger tokens.

**States the canon does not define for the control this story adds (`KSelect`):** hover, focus,
pressed, open/expanded, disabled — **and the open dropdown surface itself** (no listbox/option
selector exists anywhere in the canon; `--k-elevated` is a token without a rule). Same gap for
`KToggle`: hover, focus, pressed, disabled. `--k-teal-lo` is annotated `/* pressed */` and
`--k-faint` `/* disabled */`, but no selector consumes either.

1. **May this story treat `src/components/settings/FormControls.tsx` (Story 8.2, shipped, in use
   across the settings UI) as the binding source for those states — reusing `KSelect` verbatim with
   no new class or component — instead of the canon?** No state would be invented; the objective gate
   would be computed-style equality against a named existing `KSelect` in the same panel. This is a
   pre-existing canon gap affecting every settings row, not a question this story creates.
2. **Is the `.focus-klarvo` defect in scope?** `src/styles.css::.focus-klarvo` is a bare class, not a
   `:focus-visible` rule, and every K-control uses it unprefixed — so the teal focus ring renders **at
   rest** app-wide and focus is visually indistinguishable from idle. It would affect the new control
   too. In scope for 13-1, or explicitly deferred to the backlog?

Everything else is planned and recorded above; a one-line answer to both resumes the run.

### Decisions

- Provider value is the string `debug` on `llmProvider`/`sttProvider`, not a separate
  `advanced.debugProvider` override switch — the AC says "a provider value … exists in the Rust twin
  and the Kotlin twin" and "invisible in the normal provider picker" presupposes a provider value.
- Scenario carried in `advanced.debugLlmScenario` / `debugSttScenario` (two keys, not one shared) —
  D9 needs a debug STT while the LLM behaves normally; one key would couple them.
- Injection at the wire `(status, body)`, not at the trait's return value — alternative rejected
  because it bypasses the mapping that 13-2 is about (see Design Notes).
- `transport` uses a real loopback request instead of a new error variant — alternative rejected
  because a new variant touches every `match` over `LlmError`/`SttError` and both `is_retryable_*`.
- STT debug implemented once in Rust (shared core), LLM debug twinned — follows ADR-0017.
- Surface is Settings → Advanced behind the existing `expertMode` boolean — the story's "or an
  equivalent on-device path" branch is unnecessary: the React settings tree already renders in full
  on Android via `MainActivity : TauriActivity`, and `BubbleSettingsMenu.kt` records that the React
  panel is already the decided on-device settings surface.
- A debug 429/5xx/transport **does** trigger the production fallback ladder and therefore a real call
  to the user's configured provider. Intended — it is D10's observable — and named here so it is not
  mistaken for a leak.
- D6 (clipboard failure) is out of scope for this mechanism; 13-2 records the Weg-2 downgrade.
- License gate untouched; `debug` therefore requires a licensed/trial device. 13-4 owns that gate.
