---
title: '13-1 Debug test provider, both twins'
type: 'feature'
created: '2026-09-19'
status: 'blocked'
baseline_revision: 'd36401c02c440fd80ce2811840a847fee5b7b906'
route: 'full'
route_source: 'auto'
review: 'thorough'
review_source: 'auto'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
followup_review_recommended: false
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-13-context.md'
  - '{project-root}/docs/adr/0016-android-path-parity-strategy.md'
  - '{project-root}/docs/adr/0017-shared-core-stt-path.md'
  - '{project-root}/test-fixtures/README.md'
warnings: ['oversized']
deferred:
  - summary: 'The teal focus ring renders at rest on every K-control, app-wide'
    evidence: '`src/styles.css` defines `.focus-klarvo` as a bare class (box-shadow + outline:none), not a `:focus-visible` rule; `FormControls.tsx` applies it unprefixed on KToggle (l.40), KSelect trigger (l.361) and KSegmented (l.463), so a focused control is visually indistinguishable from an idle one everywhere.'
    location: 'src/styles.css::.focus-klarvo'
    severity: 'medium'
    origin: 'pre-existing'
  - summary: 'Advanced -> System log-level row still uses a raw <select>, not the shipped KSelect'
    evidence: '`src/components/AdvancedSettingsPanel.tsx:397` renders a native `<select>` with its own ad-hoc classes (bg-klarvo-bg, border-klarvo-border/60) predating the Story 8.2 control system; the two debug rows this story adds sit next to it and use KSelect, so the two neighbours do not match.'
    location: 'src/components/AdvancedSettingsPanel.tsx:397'
    severity: 'low'
    origin: 'pre-existing'
---

<intent-contract>

## Intent

**Problem:** Four rows of drift audit #2 (D2/D-H19 empty LLM answer, D3/D-M16 truncated answer,
D9/D-M5+M6 empty STT result, D10/D-M2 malformed answer) are agent-only today: Andi cannot provoke
a misbehaving provider on his own Windows machine or his Xiaomi without standing up a fake API.
Story 13-2 needs those states reproducible before it changes the guards (ADR-0016 Amd 4, H+).

**Approach:** A `debug` provider value for LLM cleanup and for STT in both twins that returns a
**canned wire response** (HTTP status + body) chosen from `advanced.debug*Scenario`, fed through
each twin's own existing response-mapping code. Selectable on both devices in Settings → Advanced →
System behind the existing `expertMode` gate. Never a default, never a fallback candidate, never an
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
| STT, other scenarios | same values as above **minus `truncated`** (`SttError` has no truncation variant) | `malformed`/`http429`/`http5xx`/`transport` behave as their LLM rows, through the STT mapping | `http*`/`transport` are the retryable shapes D9's retry-budget half needs |

## Control states (response-state contract)

The only interactive control this story adds is `KSelect`, twice, reused **verbatim** — same
component, no new class, variant, state or token. The only new strings are the two row labels and the
option labels, and those are *derived* rather than composed (see the React task and the Decisions
entry "Wording is derived, never composed"). No `KToggle` is added: the `expertMode` switch that gates the rows already ships
(`AdvancedSettingsPanel.tsx:413`) and is untouched. Source for every state below is the shipped
precedent `src/components/settings/FormControls.tsx::KSelect` (Story 8.2, at `baseline_revision`, in
use in `LanguageContent.tsx`, `AppearanceContent.tsx`, `AiProvidersContent.tsx`,
`RecordingAudioContent.tsx`). The design canon defines `.select` **idle only**, so it cannot source
the rest — recorded as `canon gap, shipped precedent`, not invented here.

| State | Definition in the shipped component | Observable gate |
|---|---|---|
| idle | trigger `bg-klarvo-surface-2`, `border-klarvo-border`, `rounded-klarvo-sm`, `px-2.5 py-1.5`, `text-xs text-klarvo-text` (FormControls.tsx:357-364) | computed-style equality vs the reference instance |
| hover | `hover:border-klarvo-border-2`, suppressed while open (l.362) | computed border-color after hover, equality vs reference |
| pressed | no distinct pressed styling exists in the component; a press opens the listbox, so "pressed" is observably the open state | equality vs reference (both unchanged on press) |
| focused | `focus:outline-none focus-klarvo` (l.361) — the ring is the app-wide always-on ring recorded under `deferred`; not fixed here | computed box-shadow equality vs reference |
| disabled | `opacity-40 cursor-not-allowed` (l.363); these two rows are never disabled — the `expertMode` gate removes them instead | not exercised; state reachable only via the `disabled` prop, which is not passed |
| open / expanded | trigger `border-klarvo-border-2` + chevron `rotate-180` + `aria-expanded="true"`; portal listbox `bg-klarvo-elevated shadow-klarvo-e2 border border-klarvo-border` (l.298-303) | listbox present in DOM, computed background/border equality vs reference |
| option: selected / keyboard-focused / disabled | `text-klarvo-teal` / `bg-klarvo-surface-2` / `text-klarvo-dim opacity-50` (l.318-328); no option of these rows is ever disabled | computed style equality vs reference option |
| loading / error | not applicable — both option lists are static literals, no async source | — |

**Reference instance** (the named existing `KSelect` every equality check compares against):
the "Dictation language" select at `src/components/settings/LanguageContent.tsx:59`.

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
- `AdvancedSettingsPanel.tsx::expertMode` — the house pattern for hiding footguns (`settings.expertMode`,
  persisted, off by default); gate the new rows behind it exactly as the Audio home-tree entry is
  gated at l.216 (`{expertMode && ( … )}`).
- `AdvancedSettingsPanel.tsx::ActiveSection` — `home | stt | llm | audio | system`; the two rows land
  **inside the existing `system` sub-page** ("Logging & diagnostics"), directly after the log-level
  row (l.397). No new home-tree entry, no new icon, no new section title.
- `src/components/settings/FormControls.tsx::KSelect` — the control to reuse verbatim (import it;
  `AdvancedSettingsPanel.tsx` does not import it yet). Trigger l.340-364, portal listbox l.290-335.
- `src/components/settings/LanguageContent.tsx:59` — the "Dictation language" `KSelect`: the **named
  reference instance** for every computed-style equality check. It is in a different settings
  category than the new rows (`AdvancedSettingsPanel.tsx` contains no other `KSelect`), so the proxy
  run visits Language first, records the reference, then Advanced → System.
- ⚠️ `src/styles.css::.focus-klarvo` — bare class, not `:focus-visible`: the teal ring renders at rest.
  Pre-existing and app-wide; recorded in frontmatter `deferred`, **not fixed here**.
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
      `KSelect` rows appended to the existing `system` sub-page after the Log Level row, wrapped in
      `{expertMode && ( … )}`. Row structure copied from the reference instance's row
      (`LanguageContent.tsx:56-64`): wrapper `flex gap-3 ${isMobile ? "flex-col" : "items-center
      justify-between"}`, a `LABEL_CLS` label span, `KSelect` with
      `className={isMobile ? "w-full" : "w-auto"}`; `KSelect` imported from `./settings/FormControls`
      and used verbatim (no new class, variant, state or token). Wording is derived, not composed:
      labels are the config keys in the panel's existing title case -- `Debug LLM Scenario`,
      `Debug STT Scenario` (cf. `Log Level`) -- option labels are the scenario strings of the I/O
      matrix verbatim (`ok`, `empty`, `truncated`, `malformed`, `http429`, `http5xx`, `transport`;
      STT: the same set minus `truncated`), exactly as the Log Level row labels its options; no hint
      line, no prose. The two fields are mirrored into `AdvancedSettings` and
      `MOCK_ADVANCED_SETTINGS` -- so the rows are selectable on both platforms and match the named
      reference instance in every state of the control-states contract.
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
- Given the two new rows in Advanced → System with `expertMode` on, when each state of the
  control-states contract is driven in the desktop proxy, then every measured computed style equals
  the named reference instance's — no new class, variant, state or token was introduced.

## Implementation Notes

Written during implementation (2026-09-20), from the tree rather than from the plan.

### 1. `malformed` is `{"choices":[]}`, and why — the one place the plan's matrix needed a fact check

The I/O matrix predicts `malformed` → Rust `LlmError::ResponseFormat`. That is true only
for a body that **deserializes** into `ChatResponse` and carries no choice. A body that
fails to deserialize at all (no `choices` key, or not JSON) takes a different live path:
`OpenAiCompatibleCleanup::send_request` parses with `response.json()`, whose decode error
`#[from]`-converts to `LlmError::Request` — which `pipeline::is_retryable_llm_error` treats
as **retryable**. That is exactly what audit row D-M2 records as the Desktop column
("Desktop tries the next provider").

Both readings therefore had to be reconciled without changing behaviour:

- The canned `malformed` body is `{"choices":[]}` — a well-formed envelope with no choice.
  It yields `ResponseFormat("No choices in response")` on Rust (matching the matrix
  verbatim) and a bare `JSONException` on Kotlin (matching the matrix verbatim, and it is
  D10/D-M2's Android observable: not an `IOException`, so the single-call fallback ladder
  never runs).
- `send_request` keeps `response.json()` for the success body. Routing the live path through
  the new `map_chat_response` (which parses with `serde_json::from_str`) would have turned a
  garbled provider answer from retryable `Request` into non-retryable `ResponseFormat` and
  **removed the Desktop fallback** — a silent regression of the very row 13-2 is to fix.
  The extraction is therefore split: `map_chat_error_status` + `map_chat_success` are the
  two halves `send_request` composes, and `map_chat_response(status, body)` is the pure
  wire-level composition the debug provider uses.
- The undecodable-body shape is **recorded, not reproduced**: the fixture vector
  `DEBUG-LLM-MALFORMED-001` names it, says where it lands on each side, and says why the
  debug provider does not use it. 13-2 should read that note before touching `is_retryable_llm_error`.

The STT side had no such tension: `WhisperStt::transcribe` already parsed with
`serde_json::from_slice`, so `map_transcription_response` is a byte-for-byte behaviour
preserving extraction and the STT `malformed` vector uses a genuinely undecodable body.

### 2. The STT JNI signature gained two pass-through arguments

`GroqSttBridge.nativeTranscribe` now takes `sttProvider` and `debugSttScenario`. Kotlin
carries both **uninspected** — the `"debug"` decision is made in `groq_jni.rs`, which swaps
`GroqWhisper` for `DebugStt` behind a `Box<dyn SttProvider>` and leaves the runtime, the
guard chain and the `__ERROR_*` sentinel mapping untouched. This is the same house pattern
as `license/jni.rs` (Kotlin reads config values, Rust owns the logic) and it keeps ADR-0017
intact: `Adr0017BoundaryGuardTest` is green, and no rule it enforces keys on an argument list.

Cost, as the Design Notes predicted: an Android STT change needs the slow
`scripts/android-build.sh`, not `scripts/android-smoke.sh`. The JNI signature change also
means a stale `.so` against a fresh APK would throw `UnsatisfiedLinkError` — both are
rebuilt together by that script, but a partial install will fail loudly rather than silently.

### 3. ⚠ Reachability gap: nothing in the UI sets `llmProvider` / `sttProvider` to `debug`

The spec's surface task is explicit — **two** `KSelect` rows for the two *scenarios* — and the
Decisions section is equally explicit that the switch is the provider value itself
("Provider value is the string `debug` on `llmProvider`/`sttProvider`, not a separate
`advanced.debugProvider` override switch") and that `debug` must never appear in the normal
provider picker. Those two constraints together leave **no UI path to select the provider**.
Built exactly as specified; the gap is recorded here rather than closed by inventing a third
control (which would break the control-states contract's "the only interactive control this
story adds is `KSelect`, twice", the wording rule, and the panel's own save path — the
`AdvancedSettingsPanel` owns only `AdvancedSettings`, not `AppConfig`).

Consequence per platform:
- **Desktop 🖥️**: Andi edits `config.json` (`"llmProvider": "debug"` / `"sttProvider": "debug"`,
  camelCase) and restarts. The allowlist round-trip is proven, so the value survives.
- **Android 📱**: `config.json` is not hand-editable without a computer, which is precisely the
  condition `epic-13-context.md` named ("it must be selectable on both devices without a
  computer attached"). Today the Android path is reachable only by syncing a desktop-written
  `config.json` to the device. **13-2's Android H+ rows depend on closing this** — either a
  third Advanced row that writes `llmProvider`/`sttProvider`, or adding `debug` to the picker
  behind `expertMode`. Needs Andi's call; both options touch decisions this story's spec closed.

### 4. Hot reload: LLM yes, STT no (Desktop only)

`cleanup_provider_reload_needed` gained `|| previous.debug_llm_scenario != next.debug_llm_scenario`
(the spec's explicitly-permitted second option), so changing the LLM scenario in
Advanced → System rebuilds the provider immediately — proven by a test that drives the real
`hot_reload_cleanup_provider` swap and then asserts the NEW scenario answers.

`save_advanced_settings` never rebuilds the **STT** slot, so a changed `debugSttScenario`
takes effect on the next full Settings save or app restart. Not fixed: adding an STT
hot-reload is shipping-code surface beyond the spec, and the reproduction path (restart the
Windows build) is cheap. Android is unaffected — `readConfig` re-reads per dictation.

### 5. Kotlin normalizes a blank scenario to `ok`; Rust leaves it blank

`KlarvoApi.parseDebugScenario` returns `"ok"` for an absent, blank or non-String value.
Rust's serde `default` only fires for an **absent** key, so a stored `""` stays `""` there —
and `llm::debug_llm_canned_wire("")` falls into the same fail-soft `_ =>` arm as `"ok"`.
Both platforms therefore behave as `ok`; only the intermediate representation differs. Stated
in the KDoc and in this note so it is not later "fixed" into a real divergence.

### 6. Harness traps found while building the GATE-4 proxy (not product defects)

1. `KSelect`'s Escape handler does not `stopPropagation`, so Escape closes the whole Settings
   panel. The harness closes a listbox by clicking the trigger again.
2. `className.includes("bg-klarvo-surface-2")` also matches `hover:bg-klarvo-surface-2`,
   which every enabled option carries — the first version compared the *selected* option on
   one control against a plain option on the other. Fixed to `classList.contains`, plus an
   explicit like-for-like assertion. Both are written up in the evidence `verdict.md`.

### 7. Deferred items still need homing

The two frontmatter `deferred` entries (`.focus-klarvo` always-on ring; the Advanced → System
log-level row still a raw `<select>`) are **not** yet in `docs/backlog.md`. The spec assigns
that to the conductor; the first was independently corroborated by the proxy run
(`idle.boxShadow` carries the teal ring on the reference instance too).

## Spec Change Log

- **2026-09-20 (implementation):** no change to the plan's shape. One factual correction is
  recorded under Implementation Notes §1 rather than rewritten into the I/O matrix: the
  matrix's `malformed` row is true for the canned body that was chosen (`{"choices":[]}`) and
  **not** true for an undecodable body, which maps to `LlmError::Request` on the live Rust
  path. The matrix stands as written; the fixture carries the nuance.
- **2026-09-20 (implementation):** the Verification section's device-free JVM gate was run
  exactly as written; the `Tests` task list gained no entries and lost none.

## Review Triage Log

### 2026-09-20 — Review pass

- lenses: blind-hunter, edge-case-hunter, verification-gap, intent-alignment (thorough set, route `full`)
- verdicts: 39 findings — high 5, medium 18, low 15, false 1, maybe-false 0
- outcome: **intent_gap** on the story's own deliverable; per the cascade every lower entry is moot.
  Code reverted, attempted implementation saved at
  `13-1-attempted-implementation.patch` (reverse-verified with `git apply --check --reverse`).
- findings:
  - `[high]` `[intent_gap]` blind-hunter: nothing in either UI sets `llmProvider`/`sttProvider` to `debug`, so the enabler cannot be switched on — verified: the only writers of those keys are `src/components/SettingsPanel.tsx:152-153` (the normal picker, which the intent forbids `debug` from) and `src/Onboarding.tsx:1405-1406` (hardcoded `groq`); `AdvancedSettingsPanel` owns only `AdvancedSettings`. The loaded epic context requires "selectable on both devices without a computer attached (editing the config file is not sufficient for Android)" (`epic-13-context.md:47`).
  - `[high]` `[intent_gap]` blind-hunter: the H+ manual checks cannot produce the state they test — the Windows script never says to set `"llmProvider": "debug"` first, and the Xiaomi line is unperformable. Same root cause as above; the verification text inherits the missing selection path.
  - `[low]` `[reject]` blind-hunter: the `.focus-klarvo` `deferred` entry duplicates `docs/backlog.md:1114` and Implementation Note §7 wrongly says it is not filed — verified at that line; rejected because the fix edits this build's spec. Recorded here so the conductor does not file a second backlog row.
  - `[false]` `[reject]` blind-hunter: `sprint-status.yaml` not updated — not a defect: on this installation sprint-status is booked by the conductor, never by build-auto.
  - `[low]` `[patch]` blind-hunter: `gate4-evidence/13-1/verdict.md` claims "23/23 checks" while `report.md` carries 24 check lines — verified by counting; moot under intent_gap, not applied.
  - `[medium]` `[patch]` blind-hunter: only the first new row is style-compared — `debug-rows-smoke.mjs:441` calls `captureStates(page, 0)` once, so the "Debug STT Scenario" row is never measured although the AC says "the two new rows"; moot, not applied.
  - `[medium]` `[patch]` blind-hunter: both picker scans run before Expert mode is switched on, so a `debug` option exposed only under `expertMode` would pass unnoticed; moot, not applied.
  - `[medium]` `[patch]` blind-hunter: the AI & Providers picker check read 0 dropdowns and 0 labels (harness says so itself) yet `verdict.md` presents that surface as cleared; moot, not applied.
  - `[medium]` `[patch]` blind-hunter: the inversion run compares a different idle state than the green run (`triggerStyle` on residual state vs `captureStates`), so it is not "the same assertion pointed at the wrong control"; the visibility and picker checks have no inversion at all; moot, not applied.
  - `[low]` `[patch]` blind-hunter: the "well-formed answer / `usage` present" comment sits above the `"empty" =>` arm in `debug_llm_canned_wire` — verified at `src-tauri/src/llm/mod.rs:1334-1336`; moot, not applied.
  - `[medium]` `[patch]` blind-hunter: `DEBUG_PROVIDER_NAME`, `DEBUG_MODEL`, `DEBUG_SCENARIO_DEFAULT` are declared Rust↔Kotlin twins but pinned by nothing (`twin-constants-vectors.json` not extended) while `DEBUG_TRANSPORT_URL` is pinned; moot, not applied.
  - `[low]` `[patch]` blind-hunter: `"debug"` is a literal in `pipeline.rs` and both config allowlists while `groq_jni.rs` uses the constant; moot, not applied.
  - `[medium]` `[patch]` blind-hunter: nothing pins the React option lists to the Rust/Kotlin scenario tables (grouped with the verification-gap finding below); moot, not applied.
  - `[low]` `[patch]` blind-hunter: `AdvancedSettingsPanel` comments still say Expert mode reveals "only" the raw audio thresholds, and the gated rows render above the switch that reveals them — the placement itself follows the spec's own task text ("after the Log Level row"); moot, not applied.
  - `[medium]` `[reject]` edge-case: Android `readConfig` returns `null` for `sttProvider = "debug"` with a blank Groq key — the intent excludes it verbatim ("Kotlin's `readConfig` still requires a non-blank Groq key … Consequence, recorded not fixed").
  - `[medium]` `[reject]` edge-case: on an unlicensed device the license gate rewrites `llmProvider` to `groq`, so debug becomes a real call — the intent excludes it ("The license gate is NOT touched — 13-4 owns it").
  - `[medium]` `[patch]` edge-case: `debugSttScenario` does not hot-reload (`save_advanced_settings` never rebuilds the STT slot) while the LLM row does, so two identical-looking rows have different activation semantics and the manual-check script overstates the STT row; moot, not applied.
  - `[low]` `[patch]` edge-case: the Rust `transport` arm sets no client timeout (Kotlin sets 2 s), so a blackholed loopback port would hang rather than refuse; moot, not applied.
  - `[low]` `[reject]` edge-case: `map_chat_response` accepts 200–299 while the Kotlin twin requires exactly 200 — unreachable from the canned set (only 200/429/503) and the fix adds branching.
  - `[medium]` `[patch]` edge-case: the picker option lists can drift from the canned-wire tables unnoticed (grouped with the blind-hunter and verification-gap findings); moot, not applied.
  - `[medium]` `[defer]` edge-case: JNI short-name resolution carries no signature, so a stale `.so` binds the renamed-arity `nativeTranscribe` and misreads arguments instead of throwing `UnsatisfiedLinkError` as Implementation Note §2 claims — bites only on a partial rebuild; moot under intent_gap, not filed.
  - `[low]` `[reject]` edge-case: opening the normal provider picker while `debug` is stored can overwrite it — the proposed fix (a `debug` entry in the picker) contradicts the intent's AC4.
  - `[high]` `[intent_gap]` edge-case: the enabler cannot be switched on from the Xiaomi (same finding and root cause as the first row).
  - `[medium]` `[intent_gap]` edge-case: `send_request` does not call `map_chat_response`; the canned `malformed` body is `{"choices":[]}`, so the shape D-M2 actually exhibits on Desktop (undecodable body → retryable `LlmError::Request` → ladder fires) is the one the debug provider cannot produce — verified in the diff and recorded by the implementer in Implementation Notes §1. Root cause is the I/O matrix inside `<intent-contract>`, which pins wire shape and outcome in a combination that does not reproduce that row.
  - `[medium]` `[patch]` edge-case: the AC says the provider survives a save and restart "verified by re-reading `config.json`", but nothing reads the file back — only in-memory `migrate_and_normalize`; moot, not applied.
  - `[high]` `[patch]` verification-gap: no executing test reaches the Android `debug` STT branch — `Java_..._nativeTranscribe` is called by no test, no JVM test loads the native library, and inverting the comparison keeps every gate green, so the Android half can ship inert; filed disposition `patch` (extract a pure `select_stt_provider` and test it); moot under intent_gap, not applied.
  - `[medium]` `[patch]` verification-gap: `spec_debug_llm_scenario_set_is_the_seven_offered` and its STT sibling claim to be "the contract with AdvancedSettingsPanel's option list" but read only the fixture; the repo has no frontend test runner and the puppeteer harness is throwaway, so a renamed option silently behaves as `ok`; moot, not applied.
  - `[low]` `[patch]` verification-gap (other): stray `usage`/well-formed comment above the `"empty"` arm (same finding as the blind-hunter row); moot, not applied.
  - `[low]` `[patch]` verification-gap (other): the new `cleanup_provider_reload_needed` clause's comment claims the field "never changes otherwise", but the rows render under `expertMode` regardless of provider, so the invariant does not hold (harmless — the expensive local-model case still returns early); moot, not applied.
  - `[high]` `[intent_gap]` intent-alignment: the intent's expectations live at the running app on two devices; the diff's provider selection lives at the config file and its tests at the in-process resolver — no surface anywhere sets the provider on a device (same root cause as the first row).
  - `[medium]` `[defer]` intent-alignment: the matrix's observables are end-of-pipeline (pastes empty, ladder fires, pipeline ends `Stopped`), the tests stop at the mapping return value; residual device risk, named in the spec's manual checks; moot, not filed.
  - `[low]` `[reject]` intent-alignment: control states measured in headless Chromium against the preview server rather than on Windows/Android — that proxy posture is this project's standing machine gate; the pixel/device half is explicitly the human's.
  - `[medium]` `[patch]` intent-alignment: of the three picker files the intent names, one is covered by an 8-label scan, one by a self-declared vacuous check, one not visited (grouped with the two blind-hunter picker rows); moot, not applied.
  - `[medium]` `[intent_gap]` intent-alignment: the intent pinned the `malformed` wire ("body without `choices`") and the diff changed the wire while keeping the predicted outcome (grouped with the edge-case row above).
  - `[low]` `[reject]` intent-alignment: edits beyond the sanctioned extraction (JNI signature, `Config`/`LlmProviderInfo` fields, `effective_cleanup_model`, the reload clause) — each is directed by the spec's Code Map and none changes real-provider behaviour; the one real hazard it creates is the stale-`.so` row above.
  - `[low]` `[reject]` intent-alignment: `Adr0017BoundaryGuardTest` is four regexes, so "green" proves less than the ADR's rule in general — the tripwire's breadth is not this story's to widen.
  - `[medium]` `[patch]` intent-alignment: the STT row's activation semantics contradict the manual-check script (grouped with the edge-case hot-reload row); moot, not applied.
  - `[low]` `[reject]` intent-alignment: no run artifacts for the Rust and JVM gates in the change-set — this session re-ran both itself and records the outcomes under Auto Run Result.
  - `[low]` `[patch]` intent-alignment: the gated rows appear above the Expert-mode switch and its hint still says "Reveals raw audio thresholds" (grouped with the blind-hunter comment row); moot, not applied.

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
- `cd src-tauri && cargo test --lib` -- expected: green, no API keys needed. The baseline exception
  applies against `baseline_revision`; at planning time that is
  `d36401c02c440fd80ce2811840a847fee5b7b906` (HEAD of `conductor/13-1`, clean tree). Re-pin it if
  implementation starts from a later revision.
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
  Asserts, in one run: (a) debug rows absent from Advanced → System with `expertMode` off, present
  with it on; (b) `debug` absent from the normal provider picker (Settings → AI Providers); (c) for
  each state of the control-states contract that the harness can drive -- idle, hover, focus, press,
  open, and the open listbox plus its selected/keyboard-focused option -- `getComputedStyle` of the
  new trigger/listbox/option equals that of the **named reference instance**, the "Dictation
  language" `KSelect` in Settings → Language, on background-color, border-color, border-radius,
  padding, font-size, color and box-shadow. A mismatch on any property fails the gate.
  Inversion: the harness must be shown RED once by comparing against a deliberately different
  control (the raw `<select>` at `AdvancedSettingsPanel.tsx:397`) before the green run is believed.
  It cannot observe persistence (preview writers are no-ops), pixels, or anything Rust.

**Manual checks (H+ reproduction path, Andi):**
- 🖥️ Windows release build via `scripts/windows-build.sh`: Settings → Advanced → Expert mode on →
  System → pick `empty` → dictate → observe.
- 📱 Fresh APK: same path on the Xiaomi. Requires a licensed/trial state and a configured Groq key.
- Which scenario fired is readable in `klarvo.log` on both platforms — no computer needed.

## Auto Run Result

Status: blocked
Blocking condition: intent gap

### 2026-09-20 build run — implemented, verified green, then reverted on an intent gap

**What was built** (baseline `d36401c`, full route, one Opus implementation subagent): the `debug`
provider end to end — `DebugCleanup` + `debug_llm_canned_wire` and `DebugStt` +
`debug_stt_canned_wire` in Rust behind behaviour-preserving extractions (`map_chat_error_status` /
`map_chat_success` / `map_chat_response`, `map_transcription_*`), `"debug"` arms in
`pipeline::{cleanup_provider_for, resolve_cleanup_provider, resolve_stt_provider}` and both config
allowlists, the two `advanced.debug*Scenario` keys, a debug branch in `groq_jni.rs` (two new JNI
pass-through args), the Kotlin twin in `KlarvoApi.kt` (`mapCleanupResponse`, `debugCannedWire`,
`parseDebugScenario`, `"debug" ->` arm, two `Config` fields), two `KSelect` rows in
Advanced → System behind `expertMode` with their TS mirrors, a 14-vector shared fixture
(`test-fixtures/debug-provider-scenario-vectors.json`) and its Rust + JVM readers.

**Verification this session ran itself** (not taken from the implementer's report):
- `cd src-tauri && cargo test --lib` — 737 passed, 0 failed (708 at baseline); the 28 `spec_debug_*`
  tests ran and passed.
- Device-free JVM gate (`./gradlew :app:testUniversalDebugUnitTest --rerun-tasks`, `ANDROID_HOME`
  set) — 25 suites, 208 tests, 0 failures, including `Adr0017BoundaryGuardTest` (3) and
  `DebugProviderScenarioTest` (14).
- `npm run build` — TS strict passed.
- Desktop proxy gate re-run from scratch against `npm run preview` — green 23/23 (report lists 24
  check lines; see triage), and the inversion run exited non-zero as required.
- Matrix test audit: every I/O-matrix row was covered by a test that ran and passed.

**Why it was reverted.** The review's four lenses converged on one unmet deliverable: *nothing in
either UI sets `llmProvider` / `sttProvider` to `debug`*, so the enabler cannot be switched on from a
device. Verified independently — the only writers are the normal picker in `SettingsPanel.tsx`
(which the intent forbids `debug` from) and the hardcoded `Onboarding.tsx`, and
`AdvancedSettingsPanel` owns `AdvancedSettings` only. The loaded epic context states the requirement
plainly (`epic-13-context.md:47`): "it must be selectable on both devices without a computer attached
(editing the config file is not sufficient for Android)". The implementation is not at fault: it
built exactly what the plan specified and recorded the hole itself (Implementation Notes §3).

The root cause sits **inside `<intent-contract>`**, which is why this is an intent gap rather than a
spec repair: the Approach says the debug provider value is "Selectable on both devices in
Settings → Advanced → System", while the control-states contract in the same block says "the only
interactive control this story adds is `KSelect`, **twice**" and the Never-list bars `debug` from the
normal picker. Those cannot all hold at once, and more than one reading is defensible, so the
resolution is a product decision, not an inference this run may make.

**Unresolved questions (a one-line answer to each resumes the run):**
1. How does a device set the provider to `debug`? (a) a third `KSelect` row in Advanced → System that
   writes `llmProvider` / `sttProvider` — this contradicts "KSelect, twice" in the intent contract and
   needs the panel to reach `AppConfig`, which today it does not; (b) `debug` in the normal provider
   picker behind `expertMode` — this contradicts the Never-list and AC4; (c) accept Desktop-only
   reachability (hand-edited `config.json` + restart) and downgrade 13-2's Android H+ rows to
   agent-verified-only, recorded explicitly. One row or two (LLM and STT separately)? What label?
2. Does the `malformed` scenario have to reproduce D-M2's **Desktop** mechanism (an undecodable body →
   retryable `LlmError::Request` → the ladder fires), or is the matrix's `ResponseFormat` outcome the
   intended reproduction? The two cannot both come from one canned body, and the matrix inside the
   intent contract pins the combination that does not reproduce the Desktop column.

**Attempted implementation preserved**, not discarded:
`13-1-attempted-implementation.patch` (2634 lines, all of
`src-tauri/`, `src/`, `android/`, `test-fixtures/`; `git apply --check --reverse` verified it matched
the tree exactly before the revert). Re-apply with
`git apply _bmad-output/implementation-artifacts/13-1-attempted-implementation.patch` once question 1
is answered, then fix the two triage rows that touch it rather than rebuilding from zero. The
`Tasks & Acceptance` boxes were set back to `[ ]` because the code they describe is no longer in the
tree. The GATE-4 evidence in `gate4-evidence/13-1/` and the implementer's Implementation Notes were
kept: they are the record of what the reverted build proved.

**Residual risks to carry into the re-dispatch** (all moot while the code is reverted, all worth
fixing in the next pass): the Android `debug` STT branch is reached by no executing test; the picker
option lists are pinned to the canned-wire tables by nothing; the second new row was never
style-compared; the picker checks ran only with Expert mode off and one of the three named picker
files was never visited; `debugSttScenario` does not hot-reload while `debugLlmScenario` does.

**Bookkeeping for the conductor:** the `.focus-klarvo` entry in frontmatter `deferred` already exists
in `docs/backlog.md:1114` (from 8-1/8-2) — do not file it twice; Implementation Note §7 claims
otherwise and is wrong on that point. `sprint-status.yaml` was deliberately not touched.

### Resolution of the 2026-09-19 halt (planning resumed 2026-09-20)

The first planning pass halted with `intent gap` on two questions. Both are answered by the rules now
carried in the build-auto contract, not by a taste call made here:

1. **Canon vs shipped component.** The canon (`docs/design/overhaul/source/assets/klarvo.css`,
   ADR-0019) defines `.select` **idle only**; it defines no hover, focus, pressed, disabled or
   open/expanded state for a select and no listbox/option surface at all. The response-state check
   admits a second source: a component that exists at `baseline_revision`, is already in use on
   another surface, and is reused verbatim. `KSelect` (and `KToggle`) in
   `src/components/settings/FormControls.tsx` qualify: Story 8.2, in use in four settings contents.
   The states are therefore sourced, not invented — see the control-states contract above — and the
   objective gate is computed-style equality against the named reference instance.
2. **`.focus-klarvo`.** The always-on focus ring exists at `baseline_revision`, reaches every
   K-control app-wide, and the story's intent does not name it: pre-existing defect rule applies. It
   is recorded in frontmatter `deferred` and under Decisions, **not fixed**, and it blocks no
   acceptance criterion of this story (the focus gate is equality with an existing instance, which
   holds whether or not the ring is correct).

No question from the first pass remains open; nothing else in the plan changed except the placement
and verification detail that resolving them required.

### Decisions

- **Routing:** the parked pass was resumed in this same file rather than opened as a `-2` spec — it
  is the same story id, the same intent, and the blocking condition that produced `blocked` was
  answered; the blocked revision stays readable in git (`4ff857c`). Alternative (a fresh `-2` file)
  rejected: it would leave two 13-1 specs for the conductor to reconcile.
- **Canon gap, shipped precedent:** `KSelect` from `src/components/settings/FormControls.tsx` is the
  source for hover/focus/pressed/disabled/open, which the canon leaves undefined for a select. Reused
  verbatim, no new class, variant, state or token; gate = computed-style equality against the
  "Dictation language" `KSelect` at `src/components/settings/LanguageContent.tsx:59`.
- **Pre-existing, deferred, not fixed:** `.focus-klarvo` renders the teal ring at rest app-wide
  (`src/styles.css`); and the Advanced → System log-level row still uses a raw `<select>`
  (`AdvancedSettingsPanel.tsx:397`) rather than `KSelect`, so the new neighbours will not match it.
  Both are in frontmatter `deferred` for the conductor to home in `docs/backlog.md`.
- **Placement:** the two rows go inside the existing `system` sub-page ("Logging & diagnostics"),
  after the log-level row, wrapped in `{expertMode && …}`. Alternative (a new home-tree entry)
  rejected: it would require a new icon, section title and subtitle — product wording this story has
  no source for — while the System page is already the diagnostics home and needs none.
- **STT scenario set = the LLM set minus `truncated`** (`SttError` carries no truncation variant).
  Alternative (STT `empty` only, as the first pass's matrix had it) rejected: D9's second half is the
  retry budget, which needs a *retryable* STT failure — `http429`/`http5xx`/`transport` — to be
  observable on device.
- **Wording is derived, never composed:** row labels are the config keys in the panel's existing
  title case (`Debug LLM Scenario` / `Debug STT Scenario`, cf. the shipped `Log Level` row) and the
  option labels are the scenario strings verbatim, exactly as the Log Level row labels its own
  options; no hint line is added. The degree of freedom is removed rather than exercised, so no
  user-facing prose is invented here. Alternative (explanatory hints per row) rejected: that is
  product wording this story has no source for.
- **Reference instance outside the panel:** `AdvancedSettingsPanel.tsx` contains no other `KSelect`
  (its neighbours are a raw `<select>` and a hand-rolled switch), so the equality gate names an
  instance in Settings → Language instead of "in the same panel". Alternative (comparing against the
  raw `<select>`) rejected: that control is the pre-existing outlier, not the precedent.
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
