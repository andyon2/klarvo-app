---
title: '13-1b Test provider operability'
type: 'feature'
created: '2026-09-21'
status: 'done'
baseline_revision: '1b3621fe7fa98a316ff7ed82eb35b2cdb5a1f189'
route: 'full'
route_source: 'pinned'
review: 'thorough'
review_source: 'auto'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
followup_review_recommended: true
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-13-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/spec-13-1-debug-test-provider-both-twins-2.md'
  - '{project-root}/docs/adr/0016-android-path-parity-strategy.md'
  - '{project-root}/docs/adr/0017-shared-core-stt-path.md'
  - '{project-root}/test-fixtures/README.md'
warnings: ['multiple-goals', 'oversized']
deferred:
  - summary: >-
      The JNI arity change (nativeTranscribe 9 -> 8 parameters) is verified by nothing that
      executes: no gate on this host links the Kotlin declaration against the Rust entry point.
    evidence: |-
      Pre-existing gap, not caused by this story, but widened by it: `#[no_mangle]` exports the
      short JNI symbol with no signature suffix, so a one-sided edit or a stale
      `libklarvo_lib.so` misbinds silently instead of throwing. powerhouse has no NDK and
      installing one is forbidden (project-context: never mutate the host to reach a gate), so
      Android Rust is not compiled here at all. Mitigated for the device check by the mandatory
      `scripts/android-install-debug.sh <ip:port> --full`, which rebuilds the `.so`.
      What would settle it: an Android build that links both sides, or a device run.
    location: >-
      src-tauri/src/stt/groq_jni.rs::Java_com_klarvo_voice_GroqSttBridge_nativeTranscribe
      and android/kotlin-src/com/klarvo/voice/GroqSttBridge.kt::nativeTranscribe
    severity: medium
---

<intent-contract>

## Intent

**Problem:** Story 13-1's test provider works but cannot be *operated*. Andi's H+ device check
(2026-09-21, Xiaomi + Windows) cost three failed attempts, none of them a code defect: four adjacent
rows saved by two different buttons (so provider-on/scenario-unsaved was reachable), the Advanced
`Save` scrolled out of sight so it was never pressed, the value `debug` sat one row under
`Log Level = debug` and was mistaken for it, and the feedback FAB covered controls on the phone.
Every later Epic-13 H+ check drives this enabler, so its operability is on the critical path.

**Approach:** Collapse each chain to **one** select whose value *is* the state — `off` plus 13-1's
scenario set — living in `AdvancedSettings`, so one button saves it and a half-configured state is
unreachable by construction. Rename the provider `test` everywhere it is named (config key, persisted
value, label, log line, symbol). Pin the Advanced `Save` to the bottom edge of the settings card's
scroll area while dirty, and stop rendering the feedback FAB.

## Boundaries & Constraints

**Always:**
- **The value is the state.** `advanced.testProviderLlm` / `advanced.testProviderStt` each hold one
  of `off | ok | empty | truncated | malformed | http429 | http5xx | transport` (STT: the same set
  **minus `truncated`**, `SttError` has no truncation variant). `off` is the serde default and the
  normalization target. There is no second key and no provider-name coupling, so "provider on but
  scenario unsaved" cannot be expressed.
- **Selection moves from the provider name to the advanced key.** `pipeline::resolve_cleanup_provider`
  / `::resolve_stt_provider` return the test provider when their key is not `off`, *before* they
  consult `llm_provider` / `stt_provider`. `cleanup_provider_for` and `resolve_fallback_provider` stay
  closed to it.
- **Named `test`, not `debug`,** in every layer that names it: the config keys, the persisted values,
  the two row labels, both twins' log lines, and the Rust/Kotlin symbols. Rust twin, Kotlin twin and
  React read the **same** camelCase keys.
- **Mapping behaviour is untouched.** The canned wire tables, `map_chat_http_response`,
  `map_transcription_http_response` and all 14 scenario outcomes are byte-identical to 13-1. This
  story changes *selection, surface and naming* only.
- **The Android license gate keeps its observable effect on the test provider.** `KlarvoApi.readConfig`'s
  unlicensed branch forces the two new keys to `off`, exactly as it previously forced
  `llmProvider`/`sttProvider` to `groq`. Behaviour is preserved across the shape change; changing the
  gate itself remains 13-4's.
- **Both twins change in the same commit** where the JNI signature moves (arity 9 → 8): `#[no_mangle]`
  exports the short name with no signature suffix, so a one-sided edit or a stale `.so` misbinds.
- Every new or reshaped vector gets an inversion check that is RED at writing time (G-B).

**Never:**
- No change to real provider behaviour, to the fallback ladders, or to the license gate's *semantics*.
  The only edits to shipping provider or license code are **behaviour-preserving extractions** that
  make an existing decision reachable by a test (13-1's precedent).
- Never a default (`off` is), never a fallback candidate, never an option in the normal provider
  picker (`RecordingAudioContent.tsx`; `AiProvidersContent.tsx` has no provider picker at all).
- No deprecated-field carry-over and no `MigrationWrite` step: a stored `llmProvider: "debug"` is
  **ignored**, not migrated (see Design Notes).
- `FeedbackModal` and its host stay mounted; only the FAB and its tooltip stop rendering.
- Out of scope: the unlicensed-Android gate *decision* (13-4) · `KSelect` opening upward (struck by
  Andi) · the phone's signing line (`scripts/android-install-debug.sh`, tooling) · the two 13-1
  deferrals already filed in `docs/backlog.md`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|---|---|---|---|
| Test provider off (default) | `testProviderLlm/Stt` absent or `"off"` | `resolve_*_provider` returns the real provider from `llm_provider`/`stt_provider`; output byte-identical to today | none |
| LLM chain on | `testProviderLlm = "empty"`, `llmProvider = "deepseek"` | `TestCleanup` is built; `llmProvider` is ignored for selection; log `[llm] TEST cleanup provider active: scenario=empty` | none |
| STT chain on | `testProviderStt = "http429"` | `TestStt` is built; retryable → real ladder fires, as in 13-1 | none |
| Unknown value | `testProviderLlm = "banana"` | `migrate_and_normalize` rewrites to `"off"` + `log::warn!`; test provider inactive | fail-safe, never active with a bogus scenario |
| Old-shape config, Rust | `llmProvider: "debug"`, `advanced.debugLlmScenario: "empty"` | `"debug"` is out of `VALID_LLM_PROVIDERS` → step (f) rewrites to `"deepseek"` with its existing warning; the unknown `advanced` keys are dropped by serde; `testProviderLlm` defaults `"off"` | user lands on a real working provider, never a dead one |
| Old-shape config, Kotlin | same file, Rust has not re-saved it | `resolveLlmProvider`'s `else ->` maps the unknown name to DeepSeek (or the ladder if no key) | never a dead provider |
| Unlicensed Android | licensed = false, `testProviderLlm = "empty"` | both test keys forced to `off`; the existing `[license]` log line still fires | 13-1's gate behaviour preserved |
| One save | value chosen, Advanced footer pressed | `save_advanced_settings` persists both keys **and** rebuilds the cleanup *and* STT runtime slots | save error surfaces in the footer button as today |
| Dirty Advanced panel | any advanced field changed, embedded in Settings | the footer is pinned to the bottom edge of the settings card's scroll area, fully visible without scrolling | none |

## Control states (response-state contract)

No new control and no new state is designed. Both controls are reused **verbatim** — same component,
no new class, variant, state, token or wording beyond the two derived row labels.

| Control | Shipped source | Observable gate |
|---|---|---|
| The two selects | `src/components/settings/FormControls.tsx::KSelect` (Story 8.2, at `baseline_revision`, in use in `LanguageContent.tsx`, `AppearanceContent.tsx`, `RecordingAudioContent.tsx`) | `getComputedStyle` equality against the **named reference instance** — the "Dictation language" `KSelect` at `src/components/settings/LanguageContent.tsx:56-63` — for idle, hover, pressed, focused, open, the open listbox, its selected and its keyboard-focused option, compared like-for-like. `disabled` is never passed (the `expertMode` gate removes the rows instead). Recorded as `canon gap, shipped precedent`: the canon defines `.select` idle only. |
| The `Save` button | `src/components/AdvancedSettingsPanel.tsx:564-578`, unchanged | The button's own classes are untouched; only its **container** gains positioning. Equality against its own pre-change computed style, plus the geometry gate in Verification. |

Row labels are derived, not composed: `Test provider (LLM)` / `Test provider (STT)`, the option label
is the config value itself — exactly how the `Log Level` row above labels its options.

</intent-contract>

## Code Map

**Reuse, do not rebuild.** 13-1 shipped and was accepted on both devices; its canned-wire tables,
response mappings and all 14 scenario fixture vectors are correct and must survive this story
byte-identical. Only selection, surface and naming move.

**Rust — config (`src-tauri/src/config/mod.rs`)**
- `VALID_STT_PROVIDERS` :1204-1205 / `VALID_LLM_PROVIDERS` :1206-1213 — both reference
  `crate::llm::DEBUG_PROVIDER_NAME`; **remove that element from both**.
- `AdvancedSettings` :126-137 — the two `debug_*_scenario` fields + `default_debug_scenario()` :188-192
  + the `impl Default` lines :214-215. Struct carries `#[serde(rename_all = "camelCase")]`.
- `migrate_and_normalize` :1249; step (f) :1394-1411 rewrites an out-of-allowlist provider to
  `default_stt_provider()` = `"groq"` :853-855 / `default_llm_provider()` = `"deepseek"` :857-859, each
  with an existing `log::warn!`. **This is the whole old-shape story** — extend it with the two new
  value allowlists, nothing more.
- ⚠️ **There is no version field and no migration ladder** in this repo; the only mechanism is
  `MigrationWrite` :1223-1228, condition-keyed (existing labels :1337, :1371, :1389). This story adds
  none.
- Auto-fallback "chosen provider has no API key": `current_key_empty` :1435-1442 (`_ => false` at :1441)
  and the candidate array :1445-1451 — **read-only**, they must stay blind to the test provider.

**Rust — providers**
- `src-tauri/src/llm/mod.rs`: `DEBUG_PROVIDER_NAME` :1296 (the single value-definition site),
  `DEBUG_TRANSPORT_URL` :1303, `debug_canned_response` :1320-1326, `debug_llm_canned_wire` :1338-1379,
  `DebugCleanup` :1400-1402 (`DEFAULT_MODEL` :1407, `canned()` :1416, log line :1417-1420,
  `impl CleanupProvider` :1461), `effective_cleanup_model` :1547 with its arm at :1564.
  `map_chat_http_response` :638 — **do not touch**.
- `src-tauri/src/stt/mod.rs`: `debug_stt_canned_wire` :480-509, `DebugStt` :526-528 (log line :546-549),
  `map_transcription_http_response` :414 — **do not touch**.
- `src-tauri/src/pipeline.rs`: the three arms to delete — `resolve_stt_provider` :49,
  `cleanup_provider_for` :250, `resolve_cleanup_provider` :275-277. Functions at :43 / :221 / :268.
  `resolve_fallback_provider` :349 (candidates :354-358) — **read-only, stays closed**.
- `src-tauri/src/stt/groq_jni.rs`: `select_stt_provider` :88-106 (debug branch :96-99, log :97);
  `nativeTranscribe` :200-214, the last two params (`stt_provider`, `debug_stt_scenario`) are 13-1's,
  unmarshalled :239-240, selector call :269-270. The module is deliberately **not** android-gated
  (doc :41-48) so `cargo test --lib` reaches the selector; keep that.
- `src-tauri/src/commands/settings.rs`: `cleanup_provider_reload_needed` :714-733 with the 13-1 clause
  at :732; `hot_reload_cleanup_provider` :744-755; `save_advanced_settings` :765-781 — ⚠️ it rebuilds
  **only** the cleanup slot, which is why the STT chain needs the addition below. `save_settings` :402
  and `merge_settings` :274-275 keep writing the two real provider keys; `resolve_providers` is called
  at :576-579 and is the model for the STT rebuild.

**Kotlin (`android/kotlin-src/com/klarvo/voice/`)**
- `KlarvoApi.kt`: consts `DEBUG_PROVIDER_NAME` :104, `DEBUG_MODEL` :109, `DEBUG_SCENARIO_DEFAULT` :112,
  `DEBUG_TRANSPORT_URL` :118; `debugCannedWire` :134, `mapCleanupResponse` :182 (**do not touch**),
  `debugTransportRequest` :206, `debugCleanupOrNull` :253, `parseDebugScenario` :651.
  `resolveLlmProvider` :423 — the debug arm :453-459 goes; ⚠️ the `else ->` :460 maps an unknown name
  to DeepSeek, which is what makes an old-shape file harmless. `cleanupFallbackCandidates` :493-512 —
  **closed literal, do not touch**. `readConfig` :662-829: `llmProvider` :690, `sttProvider` :701,
  the two scenario reads :724-725, the **license-gate block :787-802** (`gatedLlmProvider` :797,
  `sttIsAlternative` :798, `gatedSttProvider` :799, log :800-802), positional `Config(...)` :806-825.
  `Config` :314-409 — the 13-1 fields are appended **last** at :407-408 because the constructor is
  positional; keep that discipline. Log line :1315-1317, call :1318, ⚠️ `val url = URL(provider.url)`
  :1320 — the test branch must stay **above** it (see the tripwire below). `collectChunkResults` :1509,
  `CHUNK_THRESHOLD` :1366.
- `GroqSttBridge.kt`: `nativeTranscribe` :58-69, the two 13-1 params :67-68.
- `KlarvoOverlayService.kt`: `transcribeWithRetry` params :2731-2732, JNI call :2739-2754, callers
  :1770-1771 and :2081-2082. `isRetryableCleanupFailure` :2707 — **read-only**.

**React**
- `src/components/AdvancedSettingsPanel.tsx`: `DEBUG_LLM_SCENARIOS` :18, `DEBUG_STT_SCENARIOS` :19,
  `LLM_PROVIDER_OPTIONS` :32, `STT_PROVIDER_OPTIONS` :33, defaults :56-57, the four props :69-79 +
  :87-90, the four rows :467-506, the Expert-mode switch :507-526 (untouched), `embedded` `outerCls`
  :174-176, the inner scroller :557, **the footer :562-580**.
- `src/components/SettingsPanel.tsx`: the card :745 (`panelMaxH` :729), **the scroll container :759**
  (closed :906), the embedded mount :896-904, its own flex-pinned footer :910-926 (the shipped
  "save at the bottom edge" precedent). ⚠️ the re-seed guard at :301
  (`if (llmProv !== "debug" && !llmKeyMap[llmProv])`) and the guard at :497
  (`setLocalLlmProvider((prev) => (prev === "debug" ? prev : fallback))`) exist **only** for 13-1 and
  must go with it. Dirty-check lines :383-384 stay.
- `src/App.tsx`: the settings wrapper :604-614 (`overflow-hidden` :607, `max-h` :612) — read-only, it
  does **not** clip the card (card `100vh-168px` fits the wrapper's `100vh-164px` content box on
  mobile, `100vh-120px` inside `100vh-116px` on desktop); the clip Andi saw is purely that the
  embedded footer flows at the end of SettingsPanel's scroller. **FAB + tooltip :1093-1126**
  (container :1094, tooltip gate :1096, button :1117-1125, `aria-label="Send feedback"` :1119);
  `FeedbackModal` host :954-968 and `showFeedbackTooltip` :178-188 — keep referenced.
- `src/types.ts`: `AdvancedSettings` :185-213 (`debugLlmScenario` :208, `debugSttScenario` :212).
- `src/tauri-commands.ts`: `MOCK_ADVANCED_SETTINGS` :100-124 (`expertMode` :120, the two scenarios
  :122-123); `isPreviewMode` :20-21 — reads return fresh mock copies, writes are resolving no-ops.
- `src/platform.ts`: `isMobile` is a **user-agent** test evaluated at module load — so the proxy
  harness can drive the phone layout with `page.setUserAgent(...)` *before* `goto`.
- `src/styles.css:196-198`: `.mobile-safe-bottom { padding-bottom: 56px; }` (rationale :185-195).

**Tests / fixtures**
- `test-fixtures/debug-provider-scenario-vectors.json` — flat array of 16; 14 scenario vectors
  (`surface: "llm"|"stt"`) keep their `wire`/`rust`/`kotlin` payloads **byte-identical**; the 2
  `surface: "provider-options"` vectors pin the *old* rows and are replaced. Ledger row
  `test-fixtures/README.md:23`.
- Rust readers: `llm/mod.rs:2972`, `stt/mod.rs:1128` (both via `CARGO_MANIFEST_DIR` → parent).
  Test blocks: `config/mod.rs:1832-1958` + golden master :3473-3479; `llm/mod.rs:2956-3244` +
  `spec_debug_provider_option_lists_match_the_config_allowlists` :3319 + ⚠️
  **`spec_react_option_arrays_and_debug_guards_are_pinned_to_rust` :3410** — a source-text tripwire
  over `AdvancedSettingsPanel.tsx` / `SettingsPanel.tsx` that pins the four TS identifiers and both
  `"debug"` guards; it **will** break and must be rewritten, not deleted. `stt/mod.rs:1109-1450`
  (incl. the three `spec_android_select_stt_provider_*`); `pipeline.rs:3076-3186`;
  `commands/settings.rs:2371`.
- `android/kotlin-test/com/klarvo/voice/DebugProviderScenarioTest.kt` — 681 lines, 17 tests, fixture
  walk :93-115, source walk `kotlinSrcFile` :463-478, ⚠️ the ordering tripwire
  `debugBranchIsTakenBeforeTheProviderUrlIsBuilt` :418-448 (its two regexes :422-423 pin the call-site
  and `URL(provider.url)` lines verbatim). JUnit 4, **no mocking library**, throwing `org.json`
  accessors, a "what this does NOT cover" KDoc.
- Proxy harness to extend, not rewrite: `_bmad-output/implementation-artifacts/gate4-evidence/13-1/debug-rows-smoke.mjs`
  (736 lines; shared predicates `assertStatesEqual` :380 / `rowsPresent` :412 / `leakedDebug` :415;
  throwaway-mock helpers :418-428; inversion scoring :706-712; five documented traps :60-80).

## Tasks & Acceptance

**Execution:**
- [x] `src-tauri/src/config/mod.rs` — replace the two `debug_*_scenario` fields with
      `test_provider_llm` / `test_provider_stt` (`#[serde(default = "default_test_provider")]`,
      wire keys `testProviderLlm` / `testProviderStt`, default `"off"`); add `VALID_TEST_PROVIDER_LLM`
      and `VALID_TEST_PROVIDER_STT` beside the provider allowlists; remove the debug element from
      `VALID_LLM_PROVIDERS` / `VALID_STT_PROVIDERS`; extend step (f) to normalize an out-of-set test
      value to `"off"` with a `log::warn!` in the style of its neighbours.
- [x] `src-tauri/src/llm/mod.rs` — rename `DEBUG_PROVIDER_NAME` → `TEST_PROVIDER_NAME` (**value
      `"test"`**), `DEBUG_TRANSPORT_URL` → `TEST_TRANSPORT_URL`, `debug_canned_response` →
      `test_canned_response`, `debug_llm_canned_wire` → `test_llm_canned_wire`, `DebugCleanup` →
      `TestCleanup` (`DEFAULT_MODEL = "test"`); log line becomes
      `[llm] TEST cleanup provider active: scenario={}`. Canned bodies and `map_chat_http_response`
      unchanged. ⚠️ `effective_cleanup_model`'s arm (:1564) becomes unreachable once selection leaves
      the provider name — delete it rather than leave dead code; `TestCleanup::model()` still reports
      `"test"`, which is what the runtime actually uses.
- [x] `src-tauri/src/stt/mod.rs` — same rename (`test_stt_canned_wire`, `TestStt`); log line
      `[stt] TEST STT provider active: scenario={}`. `map_transcription_http_response` unchanged.
- [x] `src-tauri/src/pipeline.rs` — delete the three provider-name arms; in `resolve_cleanup_provider`
      and `resolve_stt_provider` return the test provider when the corresponding advanced key is not
      `"off"`, **before** the existing match. Leave `cleanup_provider_for` and
      `resolve_fallback_provider` closed to it.
- [x] `src-tauri/src/stt/groq_jni.rs` — `select_stt_provider(test_provider_stt: &str, api_key: &str,
      model: &str, temperature: f32)`; drop `provider_name`; `nativeTranscribe`'s last two params
      collapse to one `test_provider_stt: JString` (arity 9 → 8). Keep the item-level `cfg` gating and
      the fail-soft `unwrap_or_default()` unmarshalling.
- [x] `src-tauri/src/commands/settings.rs` — retarget the reload clause at :732 to
      `test_provider_llm`; make `save_advanced_settings` **also** rebuild the STT slot when
      `test_provider_stt` changed, so one button makes both chains take effect.
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` — mirror the rename; `Config` swaps the two
      13-1 fields for `testProviderLlm` / `testProviderStt` (still appended **last**); `readConfig`
      reads the new nested keys; `resolveLlmProvider` returns the test `LlmProviderInfo` when
      `testProviderLlm != "off"` **before** the `when`, and loses its provider-name arm; the
      license-gate block forces both new keys to `"off"` when unlicensed; log line becomes
      `[test-provider] LLM cleanup scenario=…` and stays **above** `val url = URL(provider.url)`.
- [x] `android/kotlin-src/com/klarvo/voice/GroqSttBridge.kt` + `KlarvoOverlayService.kt` — drop
      `sttProvider` / `debugSttScenario` from `nativeTranscribe`, `transcribeWithRetry` and both
      callers; pass `config.testProviderStt`.
- [x] `src/components/AdvancedSettingsPanel.tsx` — delete the four rows, the four props and the four
      option constants; add `TEST_PROVIDER_LLM_OPTIONS` / `TEST_PROVIDER_STT_OPTIONS` (leading `off`)
      and two `KSelect` rows labelled `Test provider (LLM)` / `Test provider (STT)` in the same
      `{expertMode && …}` wrapper after the Log Level row, each written through `set(...)`; make the
      footer container `sticky bottom-0` with the card background (and a `z-index` above the rows)
      when `embedded`, leaving the standalone branch as it is.
- [x] `src/components/SettingsPanel.tsx` — drop the four props from the embedded mount and both
      `"debug"` guards (:301, :497), restoring the pre-13-1 conditions.
- [x] `src/types.ts` + `src/tauri-commands.ts` — mirror the two fields (mock default `"off"`).
- [x] `src/App.tsx` — add `const SHOW_FEEDBACK_FAB = false;` and guard the FAB+tooltip block so
      `showFeedbackTooltip`, `dismissFeedbackTooltip` and `FeedbackIcon` stay referenced (TS strict
      runs with `noUnusedLocals`); leave the `FeedbackModal` host mounted.
- [x] `test-fixtures/debug-provider-scenario-vectors.json` → `test-provider-scenario-vectors.json`,
      ids `DEBUG-*` → `TEST-*`; the two `provider-options` vectors are replaced by
      `TEST-PROVIDER-LLM-OPTIONS-001` / `TEST-PROVIDER-STT-OPTIONS-001` carrying `config_key`
      `testProviderLlm` / `testProviderStt`, the new option arrays and `rust.constant`
      `VALID_TEST_PROVIDER_LLM` / `VALID_TEST_PROVIDER_STT`. The 14 scenario vectors keep their
      `wire` / `rust` / `kotlin` payloads byte-identical. Update `test-fixtures/README.md:23` and
      every reader path.
- [x] Rust tests — rename throughout (`config/mod.rs:1832-1958` + golden master :3473-3479,
      `llm/mod.rs:2956-3244`, `stt/mod.rs:1109-1450`, `pipeline.rs:3076-3186`,
      `commands/settings.rs:2371`) and add these pins:
      (i) `config` — a real old-shape file (`llmProvider: "debug"` + both `debug*Scenario` keys) loads
      to `llm_provider == "deepseek"`, `stt_provider == "groq"`, both test keys `"off"`, and a
      re-serialize contains neither old key; an out-of-set test value normalizes to `"off"`; the two
      new keys round-trip camelCase through a real file.
      (ii) `pipeline` — a non-`off` key selects the test provider **while `llm_provider` stays
      `"deepseek"`**, `"off"` selects the real one, and `resolve_fallback_provider`'s candidates are
      unchanged (extend the existing `spec_debug_is_never_a_cleanup_fallback_candidate`).
      (iii) `commands/settings` — `save_advanced_settings` rebuilds the cleanup slot on a
      `test_provider_llm` change **and** the STT slot on a `test_provider_stt` change.
      (iv) `llm/mod.rs:3410` — **rewrite** `spec_react_option_arrays_and_debug_guards_are_pinned_to_rust`
      to pin the two new TS identifiers against `VALID_TEST_PROVIDER_LLM` / `VALID_TEST_PROVIDER_STT`
      **and** to assert the four old identifiers and both `"debug"` guards are gone from
      `AdvancedSettingsPanel.tsx` / `SettingsPanel.tsx`.
      (v) `stt/mod.rs` — the three `spec_android_select_stt_provider_*` follow the new 4-arg selector,
      keeping their near-miss discriminators (`"Test"`, `"testx"`, `""`).
- [x] `android/kotlin-test/com/klarvo/voice/DebugProviderScenarioTest.kt` →
      `TestProviderScenarioTest.kt` — rename throughout, keep the fixture walk (:93-115), the source
      walk `kotlinSrcFile` (:463-478) and the ordering tripwire (:418-448, both regexes retargeted to
      the renamed call site), and add: `resolveLlmProvider` returns the test provider when
      `testProviderLlm != "off"` and the real one when `"off"`; an old-shape `llmProvider = "debug"`
      resolves to DeepSeek; and the license gate forces both test keys to `"off"` when unlicensed.
      ⚠️ The suite's KDoc waives `readConfig` (file I/O + `android.util.Log`), so the gate is
      unreachable as it stands. Make it reachable with a **behaviour-preserving extraction** of the
      gate's value decision into a pure `internal` helper in `KlarvoApi.kt` (the same seam pattern
      13-1 used for `mapCleanupResponse` / `debugCleanupOrNull`), called from `readConfig` at :797-799
      with no change to what it decides, and drive that helper from the test. Andi cannot un-license
      his phone, so this row is machine-verified by construction — do not hand it to him.
- [x] `_bmad-output/implementation-artifacts/gate4-evidence/13-1b/test-provider-rows-smoke.mjs` —
      extend 13-1's harness (do not rewrite): two rows instead of four, the new fixture, the sticky-footer
      geometry gate, the FAB-absence gate, and a phone-viewport pass via `setUserAgent`.

**Acceptance Criteria:**
- Given a clean install, when the config is loaded, then `testProviderLlm` and `testProviderStt` are
  `"off"`, no test provider is constructed, and a dictation's output is byte-identical to
  `baseline_revision`.
- Given `advanced.testProviderLlm = "empty"` and `llmProvider = "deepseek"`, when a dictation runs,
  then the test provider is used, `klarvo.log` names the scenario on both platforms, and the
  observable outcome is 13-1's (Desktop: "Cleanup failed", raw text in the clipboard; Android: empty
  paste until 13-2).
- Given a config file written by 13-1 (`llmProvider: "debug"`, `advanced.debugLlmScenario: "empty"`),
  when it is loaded, then `llm_provider` is `"deepseek"`, `testProviderLlm` is `"off"`, no key named
  `debug*Scenario` survives a save, and the user is on a working provider — and when the same file is
  read by Kotlin before any re-save, then `resolveLlmProvider` yields DeepSeek, never a dead provider.
- Given Settings → Advanced → System with Expert mode on, when the section is rendered, then exactly
  **two** test-provider rows exist (`Test provider (LLM)`, `Test provider (STT)`) with the option
  lists from the fixture, the four 13-1 rows are gone, and with Expert mode off neither row exists.
- Given a value is chosen in either row, when the **Advanced** footer `Save` is pressed and nothing
  else, then both keys are persisted (readable in `config.json`) and both runtime slots reflect them
  without a restart — no second button is involved and no half-configured state was reachable.
- Given the Advanced panel is dirty and embedded in the settings card, when the panel is scrolled to
  the top and to the bottom, at desktop and at phone viewport, then the `Save` button's bounding box
  is fully inside the scroll container's visible rect in every case.
- Given any screen of the app, when the DOM is queried, then no element with
  `aria-label="Send feedback"` and no feedback tooltip exists, while `FeedbackModal` is still imported
  and its host still renders when `panels.showFeedback` is true.
- Given an unlicensed state and a non-`off` test value stored, when the license gate's extracted
  decision helper runs, then both test keys come back `"off"`, and given a licensed state they come
  back unchanged — 13-1's gate behaviour, preserved across the shape change and now machine-checked.
- Given the normal provider picker (`RecordingAudioContent`) in either Expert-mode state, when its
  options are read, then no test-provider value appears; and `AiProvidersContent` still carries no
  provider picker at all (asserted positively, never as a zero-element scan).
- Given the test provider is active with a retryable scenario, when the ladder fires, then its
  candidates are unchanged (`deepseek → openai → openrouter`; Groq never a cleanup fallback).
- Given every new or reshaped guard and vector, when it is inverted at writing time, then it goes RED;
  the evidence is recorded.

## Implementation Notes

Written during implementation (2026-09-21), from the tree rather than from the plan.

### 1. The value is the state, in five layers

`AdvancedSettings.test_provider_llm` / `::test_provider_stt` (serde `camelCase`,
`default = "default_test_provider"` → `"off"`) replace the two `debug_*_scenario` fields.
`config::TEST_PROVIDER_OFF` is the single `"off"` literal; `VALID_TEST_PROVIDER_LLM` /
`VALID_TEST_PROVIDER_STT` sit beside the provider allowlists, `off` first, and
`migrate_and_normalize` step (f) normalizes an out-of-set value to `off` with a
`log::warn!` in the style of its two neighbours. `"debug"` is gone from
`VALID_LLM_PROVIDERS` / `VALID_STT_PROVIDERS`.

Selection moved to the top of `pipeline::resolve_cleanup_provider` and
`::resolve_stt_provider` — an early return *before* the provider-name match, so
`llm_provider` / `stt_provider` keep whatever the user configured and switching the row
back to `off` returns them to it with nothing to remember. `cleanup_provider_for` and
`resolve_fallback_provider` stayed closed; `spec_cleanup_provider_for_is_closed_to_the_test_provider`
now pins that from the other side (it asserts no NAME can build the test provider).

### 2. `effective_cleanup_model`'s arm was deleted, not moved

Once selection leaves the provider name, no caller can reach that function with the test
provider's name, so story 13-1's arm became unreachable. It is deleted rather than left
as dead code implying a path that does not exist. The runtime still reports the right
model: `TestCleanup::DEFAULT_MODEL` is now defined AS `TEST_PROVIDER_NAME` (one
value-definition site), and `model()` is what `Klarvo.log` reads.
`spec_test_cleanup_model_is_its_own_entry` pins both halves — the model id AND the
absence of the arm.

### 3. The rename reached the symbols, and the canned bodies were protected from it

`DEBUG_PROVIDER_NAME` → `TEST_PROVIDER_NAME` (value `"test"`), `DEBUG_TRANSPORT_URL` →
`TEST_TRANSPORT_URL`, `debug_canned_response` → `test_canned_response`,
`debug_llm_canned_wire` / `debug_stt_canned_wire` → `test_*`, `DebugCleanup` →
`TestCleanup`, `DebugStt` → `TestStt`; the Kotlin twin the same, plus
`debugCannedWire` → `testCannedWire`, `debugCleanupOrNull` → `testCleanupOrNull`,
`parseDebugScenario` → `parseTestProvider`, `LlmProviderInfo.debugScenario` →
`::testScenario`. Log lines: `[llm] TEST cleanup provider active: scenario={}`,
`[stt] TEST STT provider active: scenario={}`, `[groq_jni] TEST STT provider active: …`,
Kotlin `[test-provider] LLM cleanup scenario=…`.

⚠️ **One thing the rename must NOT touch, found by a red test:** the canned wire
*bodies* carry the literal word `Debug` (`"Debug provider canned answer."`, …). They are
wire payload pinned byte-identical on both twins by the fixture — the "same bytes, two
mappings" premise the whole enabler rests on. A prose sweep renamed
`"debug provider malformed body"` inside `stt::test_stt_canned_wire` and
`spec_test_stt_malformed` caught it immediately. The literal is restored with a comment
saying why it must stay. All 14 scenario vectors were then diffed against
`HEAD:test-fixtures/debug-provider-scenario-vectors.json` programmatically:
`wire` / `rust` / `kotlin` / `surface` / `scenario` identical on every one.

### 4. `save_advanced_settings` grew an STT rebuild — "one save" now means one effect

`cleanup_provider_reload_needed`'s 13-1 clause was retargeted to `test_provider_llm`, and
a twin `stt_provider_reload_needed` + `hot_reload_stt_provider` was added beside it (same
bare-`RwLock` shape, so a unit test can supply one without an `AppState`).
`save_advanced_settings` calls both. Without it, pressing Save would persist the STT
scenario and leave the running STT slot on the real provider until a restart — which is
exactly the class of defect this story exists to remove.

### 5. JNI arity 9 → 8, both sides in one commit

`select_stt_provider(test_provider_stt, api_key, model, temperature)` — `provider_name`
is gone, and the two arguments that could disagree with each other became one.
`nativeTranscribe` lost `sttProvider` / `debugSttScenario` for one `testProviderStt` on
both sides (`groq_jni.rs` and `GroqSttBridge.kt`), and `transcribeWithRetry` plus both of
its call sites in `KlarvoOverlayService.kt` followed. `""` (the fail-soft value the JNI
caller substitutes when it cannot read the argument) is treated exactly like `"off"`: a
test run that cannot read its own arguments degrades to the real provider, never the
reverse. Both declarations carry a ⚠️ comment naming the stale-`.so` misbind and the
`--full` install.

### 6. The Android license gate: a behaviour-preserving extraction, because Andi cannot un-license his phone

`readConfig`'s inline gate decision moved into a pure
`KlarvoApi.gateProvidersForLicense(licensed, llmProvider, sttProvider, testProviderLlm,
testProviderStt) -> GatedProviders`. It decides exactly what the inline code decided —
`llmProvider` → `"groq"`, the STT allowlist shape (`neither groq nor local` is
alternative), and now the two test keys → `off` — plus `gated`, the predicate the
existing `[license]` log line keys on (extended to fire when a test key is what got
rewritten, so the log's observable behaviour survives the shape change too).
`readConfig` keeps the file I/O and the logging, the same seam pattern 13-1 used for
`mapCleanupResponse` / `testCleanupOrNull`. Two JUnit tests drive it: the gate itself,
and its unchanged decision for every other input. **Verifikations-Symmetrie, Weg 2,
written down rather than implied: this row is machine-verified by construction and is
NOT in Andi's manual list.**

### 7. Surface: two rows, one owner, a pinned footer, no FAB

`AdvancedSettingsPanel` lost its four provider props and its four option constants; it
now carries `TEST_PROVIDER_LLM_OPTIONS` (listed) and `TEST_PROVIDER_STT_OPTIONS`
(derived by `.filter((s) => s !== "truncated")`, so the derivation itself is pinnable),
and two verbatim `KSelect` rows labelled `Test provider (LLM)` / `Test provider (STT)`,
both written through the panel's own `set(...)`. `SettingsPanel` dropped the four props
and both `"debug"` guards, restored to their pre-13-1 conditions — with selection in
`advanced`, `llmProvider` only ever holds a real key-bearing provider again, so the
exemptions would be dead code blessing a value nothing can store.

The embedded footer gained `sticky bottom-0 z-10 bg-klarvo-surface`. Embedded, the
panel's root has no height bound, so its `flex-1 min-h-0` scroller is inert and the
footer was simply the last item of `SettingsPanel`'s scroll area — which is why the Save
button could sit below the fold and never be pressed. The standalone branch is untouched;
`mobile-safe-bottom` stays, because padding BELOW a pinned button never hides it.

`src/App.tsx` got `const SHOW_FEEDBACK_FAB = false;` and the FAB + tooltip block behind
it. ⚠️ **Stated, not implied:** the FAB was that panel's only trigger, so the feedback
panel now has no UI entry point at all. The `FeedbackModal` host stays mounted and
outside the guard, so it still renders when `panels.showFeedback` is true — giving
feedback a new home is a separate decision, not this story's.

### 8. What pins the React surface, since no recurring gate runs TypeScript

`llm::tests::spec_react_test_provider_arrays_are_pinned_to_rust` was **rewritten**, not
deleted: it pins `TEST_PROVIDER_LLM_OPTIONS` against `VALID_TEST_PROVIDER_LLM`, pins the
STT derivation *as source text* and then evaluates it against
`VALID_TEST_PROVIDER_STT`, and adds the negative half — the four story-13-1 identifiers,
both `"debug"` guards and the two provider callbacks must be **gone**.
`spec_react_feedback_fab_is_off_and_its_modal_stays_mounted` is new and does the same for
the FAB: the flag is `false`, the button is inside the guard, the modal host is outside
it, and the import survives.

### 9. Old-shape configs are ignored, and both twins prove it

Rust: a real 13-1-shaped `config.json` is written to disk by hand and loaded —
`llm_provider` `"deepseek"`, `stt_provider` `"groq"`, both test keys `"off"`, and a
re-save contains neither `debug*Scenario` key. (A DeepSeek key is present in that fixture
on purpose: without it the shipped Groq-Llama pre-rule flips `llm_provider` to `"groq"`
for an unrelated reason and the test would be measuring the wrong thing — found by a red
run, not by reading.) Kotlin: `resolveLlmProvider` maps the unknown name `"debug"` to
DeepSeek through its `else ->` arm, and to the ladder when there is no DeepSeek key.

### 10. Inversions were executed, not asserted

All 10 Rust and 3 Kotlin inversions were driven mechanically — real edit, real test run,
real revert — and every one went RED; the harness produced 4/4 red groups. Evidence:
`gate4-evidence/13-1b/code-inversion-report.md` (+ `code-inversions.json`),
`inversion-report.md`, `verdict.md`. The tree was re-confirmed green after the last
revert.

## Spec Change Log

- **2026-09-21 (implementation):** no change to the plan's shape; every task was
  executed as written. Three things are recorded rather than silently taken:
  1. **The canned wire bodies keep the word `Debug`.** The spec says the rename reaches
     "every layer that names it" AND that the canned tables stay byte-identical. Those
     meet inside the body literals (`"Debug provider canned answer."`). The bodies are
     wire payload pinned by the fixture on both twins, so they were left alone and a
     comment now says why (Implementation Notes §3). A prose sweep had renamed one of
     them; `spec_test_stt_malformed` caught it.
  2. **`select_stt_provider` treats `""` like `"off"`.** The spec names the fail-soft
     `unwrap_or_default()` unmarshalling but not what the resulting empty string means
     once `provider_name` is gone. It means "real provider", which is the only
     fail-safe direction; pinned by
     `spec_android_select_stt_provider_keeps_groq_for_off_and_unreadable`.
  3. **The `[license]` log predicate was widened with the two new keys.** The spec asks
     for the gate's *observable effect* to be preserved. Under the old shape an
     unlicensed test run always tripped `resolvedLlmProvider != "groq"`; under the new
     one it need not, so the predicate now also fires when a test key is what got
     rewritten. Nothing else about the decision changed, and 13-4 still owns the gate.

## Review Triage Log

### 2026-09-21 — Review pass

- verdicts: 38 findings — high 0, medium 12, low 23, false 3, maybe-false 0
- lenses (launch order): blind-hunter, edge-case-hunter, verification-gap, intent-alignment. Rows below are in the order the lenses reported.
- entries: 15 grouped entries routed `patch` (5 at medium, 10 at low), 1 `defer`, 8 `reject`. No `intent_gap`, no `bad_spec`, so no loopback; `review_loop_iteration` stays 0.
- findings:
  - `[medium]` `[patch]` **The cleanup fallback ladder lost its first rung while the test provider ran** — verified: `pipeline.rs` passed `cfg.llm_provider` as `resolve_fallback_provider`'s `excluding`, so with the test provider active DeepSeek was excluded and the ladder started at OpenAI, against the AC "candidates are unchanged (deepseek → openai → openrouter)". Fixed by extracting `effective_llm_provider_name(&cfg)` and using it at the call site; the ladder test now pins the call the RUNTIME makes plus a discriminating half proving a real primary IS excluded from its own ladder. (entry G1)
  - `[medium]` `[patch]` **Same change re-opened a Rust↔Kotlin ladder divergence** — verified: `KlarvoOverlayService.kt` passes `llmProvider.providerName` (= `test`), so Android still started at DeepSeek while Desktop started one rung later for the identical config. Same root cause as the row above; closed by the same fix. (G1)
  - `[medium]` `[patch]` **Two `[pipeline]` log lines named `deepseek` for a run that never touched it** — verified at the cleanup-timing and primary-failure log lines. Same root cause; both now carry the effective provider name. (G1)
  - `[medium]` `[patch]` **The Windows `llm_provider == "local"` early return sat above the new test clause** — verified: `cleanup_provider_reload_needed` returned `false` before reaching `previous.test_provider_llm != next.test_provider_llm`, so on Windows with offline STT (which `SettingsPanel::handleSttProviderChange` forces to `llmProvider = "local"`, per the function's own doc) Save persisted the key while the slot kept `LocalLlmCleanup` until restart — the exact defect this story removes, on Andi's H+ platform. Clause moved above the guard; because a Linux `cfg!` cannot observe the branch, the ordering is additionally pinned as source text, which is what inverts RED. (G2)
  - `[low]` `[patch]` **`hot_reload_stt_provider` omits the local-model guard its documented twin carries** — verified real but bounded: `LocalWhisperProvider::new` resets `ctx` to `None`, so a rebuild discards a loaded context. The cleanup twin's guard protects against rebuilds triggered by UNRELATED `llm_model_*` changes, whereas here the trigger IS the test key, so skipping would defeat the rebuild; the cost is one lazy ~100–200 ms `ensure_context` reload per toggle. Recorded in the doc comment rather than guarded. (G15)
  - `[medium]` `[patch]` **Inversion row 7 recorded a RED the named test cannot produce** — verified by measurement: deleting the `hot_reload_stt_provider` call from `save_advanced_settings` leaves the suite green at 748 passed, because the helper stays referenced by the test module's `use super::{…}` and the test drives it directly. The evidence file contradicted itself two sections later. Row 7 corrected in `code-inversion-report.md` and `code-inversions.json`, row 7b added, and the wire is now pinned by `spec_save_advanced_settings_rebuilds_both_runtime_slots`; the call-site gap is filed in `docs/backlog.md`. (G3)
  - `[low]` `[patch]` **`spec_test_cleanup_model_is_its_own_entry` asserts the opposite of its name** — verified: the body pins `effective_cleanup_model(TEST_PROVIDER_NAME, "") == DeepSeekCleanup::DEFAULT_MODEL`. Renamed to `spec_test_cleanup_model_comes_only_from_the_provider`. (G4)
  - `[low]` `[patch]` **The FAB tripwire's containment check is one-sided** — verified: `guard < fab` only proves the button follows the guard's opening, so a FAB moved below the guard's close still passed, which is the regression the tripwire exists to catch. Upper bound added; inverted RED. (G5)
  - `[low]` `[patch]` **Three production doc comments still name the removed keys, one with an inverted claim** — verified: `llm/mod.rs` and `stt/mod.rs` still said `advanced.debugLlmScenario` / `advanced.debugSttScenario`, and `TestStt`'s doc described `select_stt_provider("debug", …)` contrasted with `"groq"`, which under the 4-arg selector both now select `TestStt`. These are the surviving instances of exactly the grep confusion the rename was justified by. All three corrected. (G6)
  - `[low]` `[patch]` **The Kotlin twin does not implement the fail-safe the Rust twin documents as load-bearing** — verified: `parseTestProvider` accepted any non-blank string, so `"banana"` (or `"truncated"` on the STT chain) left the test provider ACTIVE on Android while Rust normalizes it to `off`. `VALID_TEST_PROVIDER_LLM` / `_STT` added as the twin of the Rust constants, filtered per chain, both pinned against the fixture. (G7)
  - `[low]` `[patch]` **The recompiled epic context still forbids what this commit did** — verified at `epic-13-context.md` ("Neither 13.1b nor 13.3 may touch that gate") against the extracted, extended and log-widened gate. Corrected to record what 13-1b carried across and what stays 13-4's. (G8)
  - `[low]` `[patch]` **The fixture's prose was not carried through the rename** — verified: three `description` fields still said `debug`. Prose only; every `wire` / `rust` / `kotlin` payload re-diffed byte-identical afterwards, since those are pinned on both twins. (G9)
  - `[low]` `[reject]` **The canned bodies still read "Debug provider …", the one string the device actually shows** — real and deliberate: the payloads are held byte-identical on both twins by the recorded decision, so the rename stops at identifiers, keys, labels and log lines. The proposed fix is to amend this build's spec's manual-check section, which triage does not do. Carried instead as a named residual risk under `## Auto Run Result` so the human gate is told before the device check.
  - `[low]` `[patch]` **The feedback panel losing its only entry point is not filed in `docs/backlog.md`** — verified: recorded in the spec and in code comments only, against the project's backlog discipline. Filed with a source ref. (G10)
  - `[medium]` `[patch]` **Windows `llm_provider == "local"` defeats the new cleanup-rebuild clause and no test covers the combination** — arrived pre-verified from the verification-gap lens with its evidence trail; same entry as the Windows ordering row above. (G2)
  - `[medium]` `[patch]` **Android routes STT by `sttProvider` alone, so the STT test provider is unreachable in offline mode** — verified at `KlarvoOverlayService.kt`: `if (config.sttProvider == "local")` takes the local-whisper branch and never consults `config.testProviderStt`, while the Rust twin returns the test provider BEFORE reading `stt_provider` — which the spec's Boundaries require of both twins. The state was unreachable under 13-1. One condition added plus a source-walk tripwire that also asserts the gate stays within two lines of the branch. (G11)
  - `[medium]` `[patch]` **The new `hot_reload_stt_provider` call is unobserved and its recorded inversion could not have been RED** — same entry as the inversion-row-7 finding above. (G3)
  - `[low]` `[patch]` **The desktop license gate over the test provider disappeared, unrecorded** — verified: `commands/recording.rs::active_stt_provider_id` reads `cfg.stt_provider`, which under 13-1 carried `"debug"` and tripped `require_license!(AlternativeProviders)`; it no longer does. Harm is bounded — the test provider returns canned junk, so no paid capability is unlocked — and the spec's Never forbids editing license code here, so the fix is the record: filed in `docs/backlog.md` for 13-4, which owns the desktop gate. No license code touched. (G12)
  - `[low]` `[patch]` **Doc comments describe the removed keys and the removed selector** — same entry as the stale-doc row above. (G6)
  - `[false]` `[reject]` **`merge_settings` could clobber the new keys / `isDirty` needs a field list** — the lens checked and cleared both itself: `merge_settings` carries `advanced: existing.advanced` and is pinned by an existing assertion, and `AdvancedSettingsPanel`'s `isDirty` is a whole-object `JSON.stringify` compare. No bad outcome at either location.
  - `[low]` `[patch]` **The sticky footer got no recurring tripwire while the FAB did, and nothing pinned each row's key wiring** — verified: the story's second headline fix was observed only by the throwaway harness, and a crossed `set("testProviderLlm"/"testProviderStt", …)` wiring would have passed every gate. Both added to the React source-text tripwire; both inverted RED. (G13)
  - `[low]` `[reject]` **The sticky footer is measured at viewports constructed to produce overflow, and occlusion is untested** — verified and refuted in part: the reduced desktop viewport is disclosed in the harness header and in `verdict.md`, and the gate asserts the overflow first precisely so a containment check on a non-scrolling scroller cannot report a vacuous green. The transient overlap of rows beneath a pinned footer is the defining behaviour of one, not a defect — at `scrollTop = max` the footer sits at its natural position and nothing is hidden. Unlikely to be met, and the fix would add an occlusion harness.
  - `[medium]` `[patch]` **"One save" is verified as three disconnected halves; the composition is untested** — same entry as the inversion-row-7 finding. (G3)
  - `[low]` `[patch]` **React's only recurring guard over the changed surface is source text** — same entry as the sticky-footer tripwire row. (G13)
  - `[medium]` `[patch]` **`hover ≡ idle` is a measurement artifact and INVERSION-1 scored 1/10** — verified: both state maps recorded `hover` byte-identical to `idle` while `open` reads the same `rgb(53, 58, 62)` token, so `hover:border-klarvo-border-2` never applied and the equality compared two idle samples. Root cause measured during the fix: Tailwind v4 wraps every `hover:` utility in `@media (hover: hover)`, which is false in headless Chromium; four escapes were tried and measured, none works. `hover` is now dropped from the comparison and reported under a NOT EXERCISED section rather than green, with a structural hover-variant class check substituted. (G14)
  - `[low]` `[reject]` **The Save button's own computed style against a pre-change baseline is unmeasured** — true but harmless: the diff shows the button element's own class string untouched and only its container gaining positioning, so that half of the response-state contract is established by the diff itself.
  - `[low]` `[defer]` **The JNI arity change is verified by nothing that executes** — pre-existing gap, not caused by this story: no NDK on this host, and `#[no_mangle]` misbinds a stale `.so` silently instead of throwing. Already disclosed in the spec and filed in `docs/backlog.md`, and mitigated by the mandatory `--full` install in Andi's manual path. What would settle it: an Android build that links both sides, or a device run.
  - `[low]` `[reject]` **The rename's one user-visible residue — the canned bodies still read "Debug provider …"** — same claim as the canned-bodies row above; rejected for the same reason and carried as a named residual risk.
  - `[medium]` `[patch]` **Windows build with `llm_provider == "local"` and a changed `testProviderLlm` needs a restart** — same entry as the Windows ordering row. (G2)
  - `[low]` `[reject]` **`save_advanced_settings` applies no allowlist, so an out-of-set value could run until restart** — no path produces one: the command's only caller is the app's own webview, the row offers allowlisted values only (pinned by a test), and `migrate_and_normalize` normalizes at load. The fix adds a branch for a state nothing can reach.
  - `[false]` `[reject]` **`test_provider_stt == ""` makes Desktop serve canned transcripts where the JNI twin keeps Groq** — the bad outcome does not occur: `""` is outside `VALID_TEST_PROVIDER_STT`, so step (f) rewrites it to `"off"` at load, and the UI cannot emit `""`. The JNI twin's extra `is_empty()` check is belt-and-braces for an unreadable JNI argument, not a divergence in reachable behaviour.
  - `[low]` `[patch]` **Android keeps the test provider active on a file Desktop normalizes to off** — same entry as the Kotlin allowlist row. (G7)
  - `[false]` `[reject]` **`clear_api_key` forces `GroqWhisper` and silently deactivates the test provider** — it does not: it calls `resolve_providers(&new_cfg, &inner.app_data_dir)`, which delegates to `resolve_stt_provider` / `resolve_cleanup_provider` and therefore honours both test keys.
  - `[low]` `[reject]` **A poisoned-lock `Err` from the cleanup reload skips the STT reload, leaving a persisted config with a stale slot** — a poisoned lock is a process-level failure, not everyday use; the single-`?` shape pre-dates this story; and the fix reorders error handling for an unreachable state.
  - `[medium]` `[patch]` **Save persists and the old provider keeps running on Windows with offline STT** — same entry as the Windows ordering row. (G2)
  - `[low]` `[reject]` **A keyless 13-1 config leaves cleanup unavailable or auth-failing** — having no cleanup provider without an API key is correct behaviour, identical to a fresh install, and the existing allowlist warning already fires. The lens filed it at low confidence and the "dead provider" outcome the AC forbids does not occur.
  - `[low]` `[patch]` **A reader looks for a config key that cannot switch the provider on** — same entry as the stale-doc row. (G6)
  - `[low]` `[reject]` **Removing the 13-1 Advanced row removed the only UI offering `anthropic`** — verified against `baseline_revision`: `RecordingAudioContent` never carried Anthropic, so the picker is restored to its pre-13-1 shipped shape; the row that offered it was a four-day-old diagnostics scaffold behind Expert mode, and Anthropic stays reachable through the keyed auto-seed in `SettingsPanel.tsx`. The fix would add product surface this story did not ask for.

## Design Notes

**Why one key per chain rather than a switch plus a dependent row.** Andi's decision (2) is "one row
per chain". Folding `off` into the scenario set makes the AC "a half-configured state is impossible by
construction" a *structural* property, not a rule someone must remember: there is no second field that
could be out of step, and the single field lives in `AdvancedSettings`, so exactly one button owns it.
The rejected alternative — keeping `llmProvider = "test"` and hiding the scenario row behind it —
keeps two writers across two commands (`save_settings` + `save_advanced_settings`), which is precisely
the two-button split that cost the three attempts.

**Why the old shape is ignored rather than migrated.** This repo has no config version field and no
migration ladder; the only mechanism is the condition-keyed `MigrationWrite`. A migration would need
the two removed fields kept as deprecated `Option<String>`s purely to be read once — three new moving
parts to preserve one setting for the one person who holds it, who is about to re-pick it anyway in a
row built to make that one tap. Doing nothing is already correct and already logged: `"debug"` falls
out of the allowlist, step (f) rewrites it to `"deepseek"` / `"groq"` with its existing warning, and
serde drops the unknown `advanced` keys. The story permits either ("migrates **or** is ignored");
what it forbids — a dead provider — cannot occur, on either twin, and both halves get a test.

**Why `save_advanced_settings` must grow an STT rebuild.** It replaces the whole advanced block and
then hot-reloads only the *cleanup* slot; 13-1 could live with that because the STT provider was
selected by `sttProvider`, which `save_settings` rebuilt. With selection moved into the advanced
block, "one save" would otherwise be true for persistence and false for effect — the STT scenario
would need an app restart, and the next H+ check would fail for exactly the reason this story exists.

**Why sticky rather than a taller card.** The card already fits its wrapper on both platforms (the
arithmetic is in the Code Map), so raising a `max-h` changes nothing. Embedded, the Advanced panel's
root has no height bound, so its `flex-1 min-h-0` scroller is inert and the footer is simply the last
item of `SettingsPanel`'s scroll area. `position: sticky; bottom: 0` inside that scroller pins it to
the bottom edge in both viewports without touching the standalone branch, and it is the same
"save sits at the bottom edge" behaviour `SettingsPanel`'s own footer already ships by flex layout.
`mobile-safe-bottom` stays on the mobile footer: extra padding below a pinned button never hides it.

**Why the rename reaches the symbols.** Andi's decision (3) names the value and the label. Renaming
only those would leave `testProviderLlm` selecting a `DebugCleanup` through `DEBUG_PROVIDER_NAME` —
a developer grepping `test provider` would find the config and the UI but not the provider, which is
the same class of confusion the decision exists to remove. Every rename site is compiler-enforced
(Rust, Kotlin, TS) or fails loudly on a throwing fixture lookup, so the churn carries no silent-failure
risk. The alternative (surface-only rename) is cheaper by roughly one mechanical pass and was rejected
on coherence.

**JNI arity change.** `#[no_mangle]` exports the short name without a signature suffix, so a stale
`.so` misbinds rather than throwing — 13-1 filed exactly this as a deferred gap. Both sides move in
one commit, and Andi's Android reproduction must use `scripts/android-install-debug.sh <ip:port>
--full`, which rebuilds `libklarvo_lib.so`; a plain install would pair new Kotlin with the old `.so`.

## Verification

**Commands:**
- `cd src-tauri && cargo test --lib` — expected green, no API keys. 13-1 measured 743 passing at its
  own baseline; **measure the baseline at `1b3621fe7fa98a316ff7ed82eb35b2cdb5a1f189` first** and apply
  the baseline exception before treating any red as this story's.
- Device-free JVM gate — delete-then-copy `android/kotlin-src/*.kt` and `android/kotlin-test/*.kt`
  into `src-tauri/gen/android/app/src/{main,test}/java/com/klarvo/voice/`, ensure
  `testImplementation("org.json:json:20231013")` is in `src-tauri/gen/android/app/build.gradle.kts`,
  then `cd src-tauri/gen/android && ./gradlew :app:testUniversalDebugUnitTest --rerun-tasks`
  (`--rerun-tasks` is mandatory after a fixture edit). Expected: all suites green incl.
  `Adr0017BoundaryGuardTest`. 13-1 measured 25 suites / 209 tests.
- `npm run build` — expected: TS strict passes with the two renamed `AdvancedSettings` fields, the
  four removed props, and the FAB guard keeping its locals referenced.
- Desktop proxy gate — throwaway puppeteer script against `npm run preview` (port 1422) in real
  Chromium; evidence in `_bmad-output/implementation-artifacts/gate4-evidence/13-1b/`. Click
  "Setup überspringen" first. Extend 13-1's harness so that one run asserts:
  (a) both rows absent with `expertMode` off, present with it on, and **exactly two** `KSelect` rows
  added — the four 13-1 labels must be absent by name;
  (b) no test-provider value in the normal picker (`RecordingAudioContent`) with `expertMode` both off
  and on, and `AiProvidersContent` positively asserted to carry no provider picker;
  (c) both rendered option lists read from the fixture and compared element-wise;
  (d) the control-states equality of the **Control states** table against the named reference
  instance, for both rows, compared like-for-like (`classList.contains`, never `className.includes`;
  selected-vs-selected);
  (e) **sticky footer geometry** — with the panel made dirty, the `Save` button's rect is fully
  contained in the scroll container's client rect at `scrollTop = 0` **and** at `scrollTop = max`,
  measured once at desktop viewport and once at phone viewport with an Android user agent set
  **before** `goto` (`src/platform.ts` reads the UA at module load);
  (f) **FAB absence** — no `[aria-label="Send feedback"]` anywhere, in both viewports, while the
  `FeedbackModal` host still mounts when its panel is opened.
  Each group gets its own inversion, shown RED once with the *same* predicate before the green run is
  believed — states pointed at the raw `<select>` of the Log Level row, visibility asserted with
  Expert mode off, the picker scan pointed at a page that does carry the value, and the geometry gate
  run against the pre-change non-sticky footer. The run reports ONE check count derived from the
  result records.
  Traps already found, do not rediscover: preview boots into Onboarding; the evidence dir is inside
  the repo and preview is the Vite dev server, so stage artifacts in a temp dir and copy after the
  browser closes; `KSelect`'s Escape does not `stopPropagation`, so close a listbox by re-clicking its
  trigger; `hover:bg-klarvo-surface-2` matches a naive `includes`; preview writers are no-ops, so
  `expertMode: true` is reached by a throwaway edit to `src/tauri-commands.ts`, asserted to have taken
  effect and restored in a `finally` with the restore asserted — `git status` must be clean afterwards.
  **It decides wiring, structure, computed style and geometry in Chromium only.** It cannot observe
  persistence, anything Rust, real Android, pixels, font rasterisation or the Windows text-scale drift.

**Manual checks (H+ reproduction path, Andi — the DoD):**
- 🖥️ Windows release build via `scripts/windows-build.sh` (run it in the background): Settings →
  Advanced → Expert mode on → System → set `Test provider (LLM)` = `empty` → press the Advanced
  `Save` → dictate a short sentence. Expected, in **one** attempt with no instruction beyond
  "Advanced → System": the "CLEANUP FAILED / raw text is in the clipboard" panel and the
  "Cleanup failed" pill.
- 📱 `scripts/android-install-debug.sh <ip:port> --full` (⚠️ `--full` is mandatory — the JNI signature
  changed), then the same path on the Xiaomi. Expected: empty paste, i.e. 13-1's D2/D-H19 behaviour,
  until 13-2 closes it. Requires a licensed/trial state and a configured Groq key (13-4 owns that
  gate).
- Keep the dictation short: cleanup chunks above 400 characters and the canned answer is returned per
  chunk, which changes the `malformed` verdict on Android.
- ⚠️ A downloaded local Whisper model can silently rescue a test STT error via
  `pipeline::try_local_whisper_fallback` — check that first if an STT scenario appears not to fire.
- Which scenario fired is readable in `klarvo.log` on both platforms — no computer needed.

## Auto Run Result

Status: done
Blocking condition: none

### 2026-09-21 implementation run

Every gate re-run from the tree after the last edit (and after the last inversion revert).

- `cd src-tauri && cargo test --lib` — **749 passed, 0 failed** (748 before the review fixes).
  Baseline measured first at `baseline_revision` `1b3621fe7fa98a316ff7ed82eb35b2cdb5a1f189`:
  **744 passed, 0 failed**. No baseline exception was needed.
- Device-free JVM gate — `android/kotlin-{src,test}` delete-then-copied into
  `src-tauri/gen/android/app/src/{main,test}/java/com/klarvo/voice/`,
  `testImplementation("org.json:json:20231013")` confirmed present, `ANDROID_HOME` set,
  `./gradlew :app:testUniversalDebugUnitTest --rerun-tasks` — **25 suites, 217 tests,
  0 failures, 0 errors** after the review fixes (214 before them; 13-1 measured 25/211),
  counted from `app/build/test-results/testUniversalDebugUnitTest/*.xml`.
  `Adr0017BoundaryGuardTest` 3/3 green; `TestProviderScenarioTest` 23/23 green.
- `npm run build` — TS strict green with the two renamed `AdvancedSettings` fields, the
  four removed props and the FAB guard keeping its locals referenced.
- Desktop proxy gate against `npm run preview` (port 1422, real Chromium) —
  green run **59 checks, 0 failed, exit 0** plus one NOT-EXERCISED note (`hover`, see the
  Review Triage Log); inversion run **4/4 groups RED, 20 ordinary checks, 0 failed,
  exit 0**. `git status` on `src/tauri-commands.ts` verified clean of
  the throwaway edit after both runs. Evidence + coverage statement:
  `gate4-evidence/13-1b/{report.md,inversion-report.md,verdict.md}` plus the style maps,
  the three geometry JSONs and the screenshots.
- Inversion evidence for the Rust and Kotlin guards — **10/10 Rust and 3/3 Kotlin
  inversions RED**, plus **6/6 Rust and 3/3 Kotlin** for the guards the review fixes added, each a real edit + real run + real revert:
  `gate4-evidence/13-1b/code-inversion-report.md`.

### Decisions taken during implementation

- **Desktop viewport for the geometry gate is 1280×520, not 1280×950.** At 950 the
  Advanced → System scroller does not overflow at all (measured: `scrollHeight 412 ==
  clientHeight 412`), which would have made the containment claim vacuous. The harness
  now asserts the overflow as a precondition and would fail rather than pass for the
  wrong reason; the phone pass (393×660) overflows by 123 px on its own.
- **The four old row labels are asserted absent BY NAME**, in both Expert-mode states,
  rather than only counting two `KSelect`s — a count alone would not notice a leftover
  label rendered without its control.

### What this run does NOT claim

No pixels, no aesthetics, no device. Nothing ran on the Xiaomi and nothing on Windows;
Android Rust compilation is unverified on this host (no NDK, and installing one is
forbidden). The proxy gate decides wiring, structure, computed style and geometry in
Chromium on the desktop React surface plus its mobile *layout branch*; it cannot observe
persistence, because preview writers are no-ops. The JNI arity itself is still checked by
nothing that executes — the pre-existing gap in `docs/backlog.md`, updated there to name
the new 8-parameter signature and the mandatory `--full` install. The two manual H+ checks
in **Verification** are unchanged and still Andi's.

(Superseded on the point of review: the four-lens thorough set WAS run by the build-auto
session afterwards — see the Review Triage Log and the closing section below.)


---

### 2026-09-21 — build-auto close-out (review pass + finalization)

**What was implemented.** The test provider stops being a provider NAME and becomes one
value per chain in `AdvancedSettings` — `testProviderLlm` / `testProviderStt`, `off` plus
13-1's scenario set, `off` the serde default and the normalization target. Selection moved
into `pipeline::resolve_cleanup_provider` / `::resolve_stt_provider` as an early return
ahead of the provider-name match, so `llmProvider` / `sttProvider` keep the user's real
provider and "provider on but scenario unsaved" cannot be expressed. The provider is named
`test` through config keys, persisted values, row labels, log lines and the Rust/Kotlin
symbols; the JNI signature collapsed 9 → 8 parameters on both sides in one commit.
`save_advanced_settings` now rebuilds the STT slot as well as the cleanup slot, so one
button press is true for effect and not only for persistence. Two `KSelect` rows replace
13-1's four; the embedded Advanced footer is `sticky bottom-0` while dirty; the feedback
FAB no longer renders. The 14 scenario vectors are byte-identical to 13-1 — selection,
surface and naming moved, mapping behaviour did not.

**Files changed** (3 commits: `cf04bf6` feature, `fb97a47` docs/evidence, `2fa7fd7` review
fixes):

- `src-tauri/src/config/mod.rs` — the two new fields, `TEST_PROVIDER_OFF`, both value
  allowlists, step (f) normalization; `"debug"` off both provider allowlists.
- `src-tauri/src/pipeline.rs` — early returns in both resolvers, the three provider-name
  arms deleted, `effective_llm_provider_name` for the fallback ladder and the log lines.
- `src-tauri/src/llm/mod.rs` — `TestCleanup` and the rename; the React source-text
  tripwires (option arrays, FAB guard, sticky footer, per-row key wiring).
- `src-tauri/src/stt/mod.rs` — `TestStt` and the rename.
- `src-tauri/src/stt/groq_jni.rs` — 4-argument `select_stt_provider`, 8-parameter
  `nativeTranscribe`.
- `src-tauri/src/commands/settings.rs` — `stt_provider_reload_needed` +
  `hot_reload_stt_provider`, the reload-clause ordering fix, the command's source tripwire.
- `android/kotlin-src/.../KlarvoApi.kt` — the twin rename, the two `Config` fields, the
  value allowlists, `parseTestProvider`, the extracted `gateProvidersForLicense`.
- `android/kotlin-src/.../GroqSttBridge.kt`, `KlarvoOverlayService.kt` — the JNI arity and
  the local-STT branch gate.
- `android/kotlin-test/.../TestProviderScenarioTest.kt` — renamed suite, 23 tests.
- `src/components/AdvancedSettingsPanel.tsx`, `SettingsPanel.tsx`, `src/App.tsx`,
  `src/types.ts`, `src/tauri-commands.ts` — the two rows, the removed props and `"debug"`
  guards, the sticky footer, the FAB flag, the mirrored fields.
- `test-fixtures/test-provider-scenario-vectors.json` + `README.md` — renamed fixture.
- `docs/backlog.md`, `_bmad-output/implementation-artifacts/epic-13-context.md` — records.
- `_bmad-output/implementation-artifacts/gate4-evidence/13-1b/` — harness, style maps,
  geometry, inversion reports, screenshots.

**Review findings.** Four lenses (blind-hunter, edge-case-hunter, verification-gap,
intent-alignment) reported **38 findings**: high 0, medium 12, low 23, false 3. Grouped into
15 `patch` entries (5 medium, 10 low), 1 `defer`, 8 `reject`. No `intent_gap` and no
`bad_spec`, so no loopback; `review_loop_iteration` stayed 0. Every row, verdict and
refutation is in the Review Triage Log above. The two findings that were product defects
this story introduced — the fallback ladder losing DeepSeek while the test provider ran, and
`Test provider (STT)` being inert on Android with offline STT — were both fixed and pinned.
Two more were false claims in this story's own evidence (inversion row 7, and a `hover`
state that had never been exercised); both were corrected rather than re-asserted.

Deferred: 1 item (the JNI arity change is verified by nothing that executes — pre-existing).
Rejected: 8 findings, each with its reason recorded in the triage log — the canned bodies
still reading "Debug provider …" (twice, deliberate and byte-identical by decision; the fix
would edit this spec), the sticky-footer viewport/occlusion critique (disclosed, and a
pinned footer overlapping rows transiently is its defining behaviour), the Save button's own
pre-change style (established by the diff), `save_advanced_settings` applying no allowlist
(unreachable), a poisoned-lock reload ordering (unreachable), a keyless old-shape config
(correct behaviour), and Anthropic leaving the Advanced row (restores the pre-13-1 shipped
picker). Three findings were refuted outright: `merge_settings` clobbering the new keys,
`test_provider_stt == ""` reaching Desktop, and `clear_api_key` forcing `GroqWhisper`.

**Follow-up review recommended: true.** First pass; no `high` entry was patched, but five
`medium` entries were (G1 ladder, G2 reload ordering, G3 unpinned save wire, G11 Android STT
branch, G14 hover gate). The specific unverified risk: **the G2 reload-ordering fix cannot be
falsified by anything that runs on this host.** `cfg!(target_os = "windows")` is false on
Linux, so the runtime cases pass with the clause on either side of the guard, and only a
source-text tripwire pins the ordering — while the behaviour it protects lives on Andi's
Windows H+ target. A second pass should re-derive that the clause is still above the guard
and that the tripwire still matches the real statement.

**Verification performed by this session**, independently re-run against the final tree:

- `cd src-tauri && cargo test --lib` — **749 passed, 0 failed** (748 before the patches;
  baseline at `1b3621f` measured 744). No baseline exception needed.
- Device-free JVM gate — sources delete-then-copied into `src-tauri/gen/android`,
  `ANDROID_HOME` set, `./gradlew :app:testUniversalDebugUnitTest --rerun-tasks` —
  **25 suites, 217 tests, 0 failures, 0 errors**, counted from
  `app/build/test-results/testUniversalDebugUnitTest/*.xml` (214 before the patches).
- `npm run build` — `tsc && vite build` green.
- Desktop proxy gate — real Chromium against `npm run preview` on 1422: green run **59
  checks, 0 failed, exit 0**, plus one NOT-EXERCISED note; inversion run **4/4 groups RED,
  20 ordinary checks, 0 failed, exit 0**. `git status` clean afterwards both times.
- Matrix Test Audit — all nine I/O-matrix rows map to a test that ran and passed in the
  output above (config defaults and round-trip, both `pipeline` selection tests, the
  normalization test, the old-shape Rust and Kotlin tests, both license-gate Kotlin tests,
  both reload tests, and the harness's desktop + phone geometry checks).

**Residual risks.**

1. **The canned answer still reads "Debug provider canned answer." / "… canned transcript."**
   That is the string Andi will see during the device check, while the row says
   `Test provider (LLM)`. Deliberate — the payloads are pinned byte-identical on both twins —
   but it reads as a mismatch at the moment of the H+ run and is worth saying out loud first.
2. **`hover` is not exercised by the proxy gate at all.** Tailwind v4 wraps `hover:`
   utilities in `@media (hover: hover)`, which headless Chromium cannot satisfy; four escapes
   were measured and none works. The rendered hover colour is Andi's real-screen gate.
3. **INVERSION-1 still discriminates only on `idle`.** The remaining state comparisons report
   equality against the raw `<select>` control the inversion points at, so those individual
   assertions are not demonstrated to be able to fail. The group meets its stated bar (at
   least one RED) and the green run's equality is real, but the per-state discrimination is
   weaker than the check count suggests.
4. **Nothing ran on Windows or on the Xiaomi, and Android Rust is uncompiled here** (no NDK).
   The JNI arity moved on both sides in one commit, and `#[no_mangle]` misbinds a stale `.so`
   silently rather than throwing — so `scripts/android-install-debug.sh <ip:port> --full` is
   mandatory for the device check; a plain install would pair new Kotlin with the old `.so`.
5. **The feedback panel now has no UI entry point.** `FeedbackModal` and its host stay
   mounted and still render when `panels.showFeedback` is true, but the FAB was that flag's
   only trigger. Giving feedback a new home is a separate decision; filed in `docs/backlog.md`.
6. **The desktop license gate over the test provider lapsed** as a side effect of moving
   selection off `sttProvider`. No license code was touched (the spec forbids it here); filed
   in `docs/backlog.md` for 13-4, which owns the desktop gate.
