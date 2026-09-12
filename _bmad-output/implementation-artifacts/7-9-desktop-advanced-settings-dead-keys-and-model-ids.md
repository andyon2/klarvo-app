# Story 7.9: Desktop Advanced settings + AutoSend — remove dead keys, wire 4 model IDs

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a BYOK power user,
I want the Advanced settings panel to show only keys that act at runtime, and the four cleanup model IDs to be a real override on both platforms,
so that no setting silently does nothing, and a retired provider model ID does not need an app update.

## Context & Governing Decisions

**Source:** `docs/backlog.md` "DECIDED 2026-09-11 — Desktop Advanced settings + AutoSend: remove 14 dead keys,
wire 4 model IDs" (Andi). Audit: `sprint-change-proposal-2026-09-10.md` (evidence row "7.7 dead-config-cluster").
**Rows:** M13 (bubble auto-send) · dead-config cluster (`docs/cross-platform-drift-audit.md`, "Dead-config catalogue").

**Governing decisions:**
- **ADR-0016 Amendment 1** — settable-but-silently-dead config keys are in parity scope; *pure feature ports*
  stay accepted asymmetries (→ `docs/backlog.md`). This is why an Android Anthropic provider is **not** built
  here (drift row **H5**, accepted).
- **ADR-0017** — the shared Rust core is STT + license only. Cleanup/LLM routing is a **Rust↔Kotlin twin** →
  the model-ID wiring is fixed **twice** (Rust `pipeline.rs` + Kotlin `KlarvoApi.kt`).
- **Story 7-8 scope guard** — the twin-constant lock deliberately did **not** freeze the dead keys ("freezing
  that would cement a lying UI"). This story executes the decision 7-8 deferred.
- **Epic scope guard (verbatim):** *Out of scope: the M12 dictionary decision (7.6), the style switches (epic
  candidate B), any change to STT, VAD, JNI or the top-level `custom_prompt`.*

**Character of the work:** removal across three layers (React/TS UI + plumbing, Rust config + commands, Kotlin
config + overlay service) with **no runtime behaviour change** for the removed keys, plus **one real runtime
change**: the four `llmModel*` keys become the model sent to the cleanup provider.

> **Open design/wording/scope questions** that this story file does **not** decide are listed in
> *Dev Notes → Open questions (Q1–Q8)*. Where a task depends on one, it says so. Do not invent the answer.

## Acceptance Criteria

### AC1 — Remove the 13 named dead keys (UI, Rust config, Kotlin twin)

**Given** the epic/backlog record names 13 keys (the backlog header counts 14 — see **Q1**; the exact set
pinned from code is the 13 below, each verified to have **no runtime reader** on today's tree),
**When** the removal is complete,
**Then** each key is gone from every layer that carries it:

| # | JSON key | Where in `config.json` | Rendered today | Runtime reader today (verified) |
|---|---|---|---|---|
| 1 | `sttTemperature` | `advanced` | Advanced → Speech-to-Text (expert only) | none — Whisper runs at 0.0: `stt::WhisperStt` default `temperature: 0.0`, `pipeline::resolve_stt_provider` never calls `with_temperature`; Android passes `temperature = 0.0f` to `GroqSttBridge.nativeTranscribe` |
| 2 | `llmTemperature` | `advanced` | Advanced → Text Cleanup → "Model & Parameters" | none — `OpenAiCompatibleCleanup::DEFAULT_TEMPERATURE` (0.3) ↔ `KlarvoApi.CLEANUP_TEMPERATURE` |
| 3 | `llmMaxTokens` | `advanced` | same | none — `OpenAiCompatibleCleanup::DEFAULT_MAX_TOKENS` (2048) ↔ `KlarvoApi.CLEANUP_MAX_TOKENS` |
| 4 | `chunkThreshold` | `advanced` | same (expert only) | none — `llm::CHUNK_THRESHOLD` (400) |
| 5 | `chunkTargetSize` | `advanced` | same (expert only) | none — `llm::CHUNK_TARGET_SIZE` (350) |
| 6 | `autoPaste` | `advanced` | **Settings → Shortcuts → "Paste & Behavior"** (`ShortcutsContent`), NOT the Advanced panel | none |
| 7 | `autoCapitalize` | `advanced` | same as #6 | none |
| 8 | `bubbleTapAutoSend` | top-level `AppConfig` | **no toggle anywhere** — declared in `ShortcutsContentProps`, passed by `SettingsPanel`, never destructured or rendered | Kotlin reads it, then `KlarvoOverlayService.loadBubbleControls` overwrites `tapAutoSend = false` |
| 9 | `bubbleLongPressAutoSend` | top-level `AppConfig` | same as #8 | same — `longPressAutoSend = false` |
| 10 | `llmSystemPromptPolished` | `advanced` | Advanced → Text Cleanup → "Custom Cleanup Instructions" (paid) | only the license gate in `commands::settings::save_advanced_settings` |
| 11 | `llmSystemPromptVerbatim` | `advanced` | same | same |
| 12 | `llmSystemPromptChat` | `advanced` | same | same |
| 13 | `llmCommandModePrompt` | `advanced` | same | same |

**And** a grep for each key in **both** spellings (camelCase + snake_case, e.g. `autoPaste|auto_paste`) over
`src/`, `src-tauri/src/` and `android/kotlin-src/` returns **no production hit** — the only permitted hits are
tests/fixtures that assert the key's *absence* or feed an *old* config (AC4),
**And** no key outside this table is removed (the further unread keys are **Q1**, not this story's call).

> **Kotlin twin, precisely:** the Kotlin side reads **none** of the `advanced.*` dead keys today (verified:
> no `sttTemperature`/`llmTemperature`/`chunk*`/`autoPaste`/`autoCapitalize`/`llmSystemPrompt*` in
> `android/kotlin-src/`). The Kotlin removal is therefore **only** #8 + #9 (`KlarvoApi.Config`,
> `KlarvoApi.readConfig`, `KlarvoOverlayService`).

### AC2 — Runtime behaviour stays exactly as it is

**Given** none of the 13 keys had a reader,
**When** they are removed,
**Then** Whisper temperature stays fixed at 0.0 on both platforms; cleanup stays 0.3 / 2048; chunking stays
400 / 350 / join `\n`,
**And** the **existing five entries** of `test-fixtures/twin-constants-vectors.json` keep their values, and both
7-8 halves (`llm::tests::spec_twin_constants_*`, `TwinConstantsVectorsTest`) stay green,
**And** "Custom Instructions" (`AppConfig::custom_prompt` / Kotlin `Config.customPrompt`) is untouched,
**And** the **"Auto-Send"** row in Shortcuts → Paste & Behavior **stays**: it is bound to `insertAndSendSlot1`
(a live desktop key, `HotkeySlot::insert_and_send`), **not** to `bubbleTapAutoSend`. Do not confuse them.

### AC3 — Live keys are untouched

**Given** these keys act at runtime: `sttPromptDe`, `sttPromptEn`, `sttPromptAuto` (`pipeline.rs` STT hint),
`silenceThreshold`, `minRecordingMs` (both platforms), `whisperModeThreshold`, `whisperModeGain` (Desktop),
**Then** their fields, defaults, serde names, UI rows, Kotlin reads (`Config.silenceThreshold`,
`KlarvoApi.parseMinRecordingMs`) and the JNI signature `nativeSilenceCheck` are unchanged,
**And** the other `AdvancedSettings` fields that the decision did not name — `pasteDelayMs`, `logLevel`,
`uiScale`, `expertMode`, `webhookHeaders`, `webhookTimeoutSecs` — are **not removed** by this story (see **Q1**).

### AC4 — Old `config.json` files still load, on both platforms (pinned by a test)

**Given** a `config.json` that still carries all 13 removed keys with **non-default** values, plus live keys
with non-default values in the same file (e.g. `advanced.minRecordingMs: 750`, `advanced.silenceThreshold: 0.012`),
**When** Rust loads it through `config::load_config_reporting`,
**Then** the live values survive, **no** `config.json.corrupt-*` backup is created, and **no** warning is pushed,
**And** when the same JSON text is fed through the Kotlin production parse seams (`org.json.JSONObject` →
`KlarvoApi.parseMinRecordingMs` and the new model-ID parse seam from AC5), the live values come out.

> **⚠ Vacuous-pass trap.** `load_config` **never fails** — a parse error returns defaults *and* backs the file up
> (`backup_corrupt_config`, Story 1-2). A test that only asserts "loads without error" passes even if serde
> rejects the file. The test must assert a **non-default live value survived** *and* **no corrupt backup
> exists** (reuse the `corrupt_backups(dir)` helper from the Story 1.2 test block in `config/mod.rs`).
> No struct carries `#[serde(deny_unknown_fields)]` today (verified) — keep it that way.

*Expected, not a bug:* the next save writes `config.json` without the removed keys (serde drops unknown fields).
Android never writes `config.json` (ADR-0015 single writer) — it only reads.

### AC5 — Wire the model-ID overrides (Rust: 4 keys · Kotlin: the 3 providers Android has)

**Given** the cleanup clients hard-code model IDs on both platforms and the `with_model` builders exist but are
never called (`#[allow(dead_code)] // builder API for future use` on `DeepSeekCleanup`, `OpenAiCleanup`,
`GroqCleanup`, `AnthropicCleanup`),
**When** the override is wired,
**Then (Rust)** `advanced.llmModelDeepseek` / `llmModelOpenai` / `llmModelGroq` / `llmModelAnthropic` set the model
of the matching provider on **every** construction path:
- `pipeline::resolve_cleanup_provider` (primary selection, including its `"anthropic"` arm),
- `pipeline::cleanup_provider_for` (shared by the primary path **and** `pipeline::resolve_fallback_provider`,
  the Epic-12 fallback ladder — `cleanup_provider_for(name, api_key)` has no config today, so its signature
  must carry the override),
- hence also app start (`lib.rs` → `resolve_providers`) and both `resolve_providers` calls in
  `commands/settings.rs`,

**And** an **empty** value falls back to today's default: `DeepSeekCleanup::DEFAULT_MODEL` `"deepseek-chat"`,
`OpenAiCleanup::DEFAULT_MODEL` `"gpt-4o-mini"`, `GroqCleanup::DEFAULT_MODEL` `"llama-3.3-70b-versatile"`,
`AnthropicCleanup::DEFAULT_MODEL` `"claude-haiku-4-5-20251001"` (whitespace handling: **Q5**),
**And** the `#[allow(dead_code)]` markers are removed from the builders that are now used,

**Then (Kotlin)** `advanced.llmModelDeepseek` / `llmModelOpenai` / `llmModelGroq` are parsed through a **pure
`JSONObject` seam** (same shape as `KlarvoApi.parseMinRecordingMs`), carried on `KlarvoApi.Config`, and used as
`LlmProviderInfo.model` in **both** `KlarvoApi.resolveLlmProvider` **and** `KlarvoApi.cleanupFallbackCandidates`
(the two independent sites 7-8 found drifting for the URL), with the same empty → default rule,
**And** `llmModelAnthropic` is **Desktop-only**: Android has no Anthropic cleanup provider
(`resolveLlmProvider` KDoc "Anthropic is NOT supported"; drift row H5, accepted under ADR-0016 Amendment 1).
No Android Anthropic provider is added (scope assumption — **Q6**),

**And** a changed model ID takes effect **without an app restart** on both platforms:
- Desktop: `save_advanced_settings` today only persists (`save_config_locked`) and does **not** rebuild
  `AppState::cleanup_provider` — only `save_settings` does, via `resolve_providers`. The Advanced-panel save must
  hot-reload the cleanup provider the same way, or the GATE-4 check below cannot pass.
- Android: `KlarvoOverlayService.loadBubbleControls` re-reads `config.json` on every tap/long-press → no extra work.

**And** the parity fixture from 7-8 (`test-fixtures/twin-constants-vectors.json`) gains the **default model IDs**;
the Rust half and the Kotlin half each assert their **production default** against the **fixture literal**
(never against another production symbol). The Anthropic entry is Desktop-asserted only; its Android column is a
written record, stated in the entry (the M12 fixture precedent),
**And** OpenRouter's model literal `"deepseek/deepseek-chat"` stays hard-coded on both sides (no override key
exists; not in the decision — **Q8**).

### AC6 — Inversion check (mandatory, at writing time)

**Given** project-context: *a green measurement can prove the wiring and still miss the claim* (7-1: 6 of 8 tests
stayed green against unfixed code),
**When** each new test is written,
**Then** the drift is re-introduced, shown **RED**, and reverted — at least:
1. **Kotlin:** one provider arm back to a hard-coded model literal that ignores the config (e.g. the DeepSeek arm
   in `cleanupFallbackCandidates`) → RED,
2. **Rust:** one arm of `cleanup_provider_for` back to `…Cleanup::new(api_key)` without the override → RED,
3. **AC4:** add `#[serde(deny_unknown_fields)]` to `AdvancedSettings` → the old-config test goes RED (proves the
   test sees the corrupt-recovery path).

> **⚠ Non-discriminating trap.** A test that only pins the *default* stays **GREEN** against a re-hard-coded
> default, because the literal equals the default. The discriminating assertion is **override flow-through**:
> a non-empty override reaches `LlmProviderInfo.model` (Kotlin) / the provider's request `model` (Rust, e.g. via
> the existing `#[cfg(test)] build_request`). Inversions 1 and 2 must go RED on *that* assertion.

**And** the inversions are recorded as a table *reverted change → red test* (format of the 7-8 record's
"Inversion table (AC8)"), **And** `git status` is clean afterwards.

### AC7 — Gates (DoD)

- `cargo test --lib` in `src-tauri/` green; `npm run build` (strict `tsc`) green.
- JVM gate `./gradlew :app:testUniversalDebugUnitTest --rerun-tasks` green (**`--rerun-tasks` is mandatory** —
  this story edits a fixture JSON, and gradle reports a stale green on fixture-only edits). Run it through
  `scripts/android-smoke.sh` on the laptop AVD (7-8 precedent) or device-free as 7-8's Debug Log describes.
- Every count carries its coverage statement (what was and was **not** exercised).
- **Desktop proxy smoke** (puppeteer vs `npm run preview`, port 1422, click "Setup überspringen" first): the
  Advanced panel — all four sections, expert mode **on and off** — and Settings → Shortcuts → Paste & Behavior show
  only live keys. Harness + evidence in `_bmad-output/implementation-artifacts/gate4-evidence/7-9/`; tree clean after.
- `docs/surface-smoke-checklist.md` traps **#1** (camelCase keys under `advanced`), **#2** (resync/`isDirty` in
  `SettingsPanel`), **#6** (multi-hop save chain) run mechanically before the Windows smoke.
- **Andi's GATE-4:** Windows release build via `scripts/windows-build.sh`; Advanced panel shows only live keys; a
  **changed DeepSeek model ID appears in the request log** (`Klarvo.log`).
  > Today **no Rust log line names the cleanup model** — `[pipeline] LLM cleanup took …ms (provider: …, style: …,
  > input_len: …)` names the provider only, and `CleanupProvider` has no model accessor. The model ID must become
  > visible in `Klarvo.log` for this gate to be passable (scope assumption — **Q7**). Android already logs
  > `[pipeline] cleanup: …ms (${llmProvider.model})`.

## Tasks / Subtasks

- [x] **Task 1 — Rust config + commands: remove the 13 keys** (AC1, AC2, AC3)
  - [x] `config::AdvancedSettings`: delete fields #1–#7 and #10–#13, their `default_*` fns
        (`default_stt_temperature`, `default_llm_temperature`, `default_llm_max_tokens`,
        `default_chunk_threshold`, `default_chunk_target_size`, `default_auto_paste`,
        `default_auto_capitalize`) and their lines in `impl Default for AdvancedSettings`.
  - [x] `config::AppConfig`: delete `bubble_tap_auto_send`, `bubble_long_press_auto_send` (+ `AppConfig::default`).
  - [x] `commands::settings`: `SettingsPatch` fields + its `Default`; `merge_settings`; the two `save_settings`
        command params + the `SettingsPatch { … }` literal; `get_settings` → `SettingsView`.
  - [x] `lib.rs::SettingsView`: delete both fields.
  - [x] `save_advanced_settings`: its license gate checks **only** the four removed prompts → the gate goes with
        them. `LicensedFeature::CustomPrompts` then has no production gate; it stays referenced by
        `license::tests::test_licensed_allows_all_features` — removing the enum variant is not required.
  - [x] Update every test that constructs/asserts a removed field — the compiler enumerates them. Known today:
        `config::tests::{test_advanced_settings_defaults, test_advanced_settings_camel_case,
        test_advanced_settings_roundtrip, test_appconfig_golden_master_full_field_roundtrip}` plus the
        AppConfig save/load roundtrip test that sets `stt_temperature: 0.2` / `bubble_tap_auto_send: true`;
        `commands::settings::tests::{test_bubble_gesture_fields_default_when_absent_from_json,
        test_bubble_gesture_fields_roundtrip}` and the `merge_settings` tests that set `bubble_*_auto_send`;
        `lib::tests::{test_settings_view_camel_case_serialization,
        test_settings_view_hotkey_mode_hold_serializes_lowercase,
        test_settings_view_hotkey_mode_toggle_serializes_lowercase}`. Keep each test's intent for the remaining fields.
  - [x] Update doc comments that now lie: `AdvancedSettings::expert_mode` ("chunking, STT temperature").

- [x] **Task 2 — Rust old-config test** (AC4)
  - [x] Inline `#[cfg(test)]` in `config/mod.rs` (no new `tests/` file): write a raw JSON with all 13 keys
        (non-default) + non-default live keys, `load_config_reporting`, assert live values, `warnings.is_empty()`,
        `corrupt_backups(dir).is_empty()`.

- [x] **Task 3 — Rust model-ID wiring** (AC5, AC7)
  - [x] Thread the override into `pipeline::cleanup_provider_for` and `resolve_cleanup_provider` (incl. the
        `"anthropic"` arm); empty → `DEFAULT_MODEL` (same predicate everywhere, **Q5**). One place decides — do not
        duplicate the empty-check per arm.
  - [x] `save_advanced_settings`: hot-reload `AppState::cleanup_provider` after the save (mirror `save_settings`'
        `resolve_providers` step; ADR-0015 — still one `save_config_locked` writer).
  - [x] Legacy `commands::settings::update_api_keys` builds `DeepSeekCleanup::new(key)` directly (no caller in
        `src/` today) — it must not bypass the override; route it through the same resolution.
  - [x] Drop `#[allow(dead_code)]` from the `with_model` builders now in use.
  - [x] Make the resolved cleanup model visible in `Klarvo.log` for GATE-4 (**Q7**).
  - [x] Tests (inline): override flow-through per provider (request `model` via `build_request`), empty → default,
        fallback ladder carries the override (`resolve_fallback_provider`). `pipeline::tests` already has
        `test_resolve_cleanup_provider_*` / `resolve_fallback_provider` tests to extend — reuse, don't duplicate.

- [x] **Task 4 — Frontend removal** (AC1, AC2) — **walk the whole chain (trap #6)**
  - [x] `src/types.ts`: `AdvancedSettings` (#1–#7, #10–#13), `AppSettings` (#8, #9); fix the `expertMode` comment.
  - [x] `src/tauri-commands.ts`: `MOCK_ADVANCED_SETTINGS`, the mock `AppSettings`, and `saveSettings` — both the
        **positional params** `bubbleTapAutoSend`/`bubbleLongPressAutoSend` and their `invoke("save_settings", {…})` keys.
  - [x] `src/hooks/useSettings.ts::handleSaveSettings`: positional params `newBubbleTapAutoSend`,
        `newBubbleLongPressAutoSend` + the forwarding call.
  - [x] `src/components/SettingsPanel.tsx`: `onSave` prop type + the internal save helper's `onSave(…)` call;
        `localBubbleTapAutoSend`/`localBubbleLongPressAutoSend` state, resync `useEffect`, `isDirty` terms, deps;
        `localAutoPaste`/`localAutoCapitalize` state, the advanced-settings mount load, `isDirty`, the advanced
        save block (`updatedAdv`), the `ShortcutsContent` props.
        > **⚠ Positional-argument trap.** `saveSettings`, `handleSaveSettings` and `onSave` are **positional**.
        > Removing an argument in one hop but not the others shifts every later argument (live preview,
        > appearance, bubble size …) silently — the Epic-6 "appearance reset-on-save" class. Edit all three hops
        > plus the call in `SettingsPanel` in one change and re-read them side by side. (Rust's `save_settings`
        > maps by name, so a stray extra key there is harmless; the TS positions are not.)
  - [x] `src/components/settings/ShortcutsContent.tsx`: `ShortcutsContentProps` (#6–#9), destructuring, and the
        "Auto-Paste" + "Auto-Capitalize" rows in "Paste & Behavior". **Keep** the "Auto-Send"
        (`insertAndSendSlot1`) row. The `!localAutoPaste` dimming/disable on "Auto-Send" and "Paste Delay" loses
        its source → **Q4**.
  - [x] `src/components/AdvancedSettingsPanel.tsx`: `ADVANCED_DEFAULTS`; rows for `sttTemperature`,
        `llmTemperature`, `llmMaxTokens`, `chunkThreshold`, `chunkTargetSize`; the whole "Custom Cleanup
        Instructions" subsection (its only content is the four prompts); keep the four model inputs. User-facing
        strings that become false and the resulting layout → **Q2/Q3** (do not invent new copy).

- [x] **Task 5 — Kotlin** (AC1, AC4, AC5)
  - [x] `KlarvoApi.Config`: delete `bubbleTapAutoSend`, `bubbleLongPressAutoSend`; add the three model-override
        fields (default `""`). `readConfig` builds `Config(…)` **positionally** — keep the argument order consistent.
  - [x] `KlarvoApi.readConfig`: delete the two `optBoolean` reads; add the model-ID parse via a new
        `internal fun` seam on `JSONObject` (pattern: `parseMinRecordingMs`).
  - [x] `KlarvoOverlayService`: delete `tapAutoSend`/`longPressAutoSend`, their assignment + debug-log fields in
        `loadBubbleControls`, and the unreachable `shouldAutoSend` block after paste. If
        `KlarvoAccessibilityService.performEnter` loses its last caller, say so in the record; change no other
        paste behaviour.
  - [x] `KlarvoApi.resolveLlmProvider` + `cleanupFallbackCandidates`: model = override-or-default for deepseek,
        openai, groq. Name the default literals as constants so the fixture test binds to a production symbol
        (7-8 precedent: `CLEANUP_TEMPERATURE`).
  - [x] Tests (JUnit 4, `android/kotlin-test/com/klarvo/voice/`): extend `LlmFallbackProviderTest` — override
        flow-through on **both** sites, empty → default; a parse-seam test fed with an old `config.json` string
        carrying the removed keys (AC4 Kotlin half). Its existing literal model assertions
        (`"gpt-4o-mini"`, `"llama-3.3-70b-versatile"`, `"deepseek-chat"`) stay valid for the default path.

- [x] **Task 6 — Parity fixture** (AC5)
  - [x] `test-fixtures/twin-constants-vectors.json`: add one entry per default model ID with `PINS:` /
        `DOES NOT PIN:` (Anthropic: Desktop-asserted, Android column a written record).
  - [x] Both "exactly five" assertions must follow: Rust
        `spec_twin_constants_fixture_is_complete_and_self_describing` (`vectors.len()`), Kotlin
        `TwinConstantsVectorsTest.fixtureCarriesAllFiveTwinsAndDescribesEach` (id set) — rename/re-word honestly.
  - [x] **Claim accuracy (7-8's defect class):** the existing entries' `DOES NOT PIN` clauses and the
        `TwinConstantsVectorsTest` KDoc name dead keys that this story deletes ("the sttTemperature config key (dead
        config, out of scope)", "the dead advanced.llmMaxTokens config key", "the dead advanced.chunkThreshold
        config key", "the dead-config cluster … deliberately NOT locked"). Correct them — values unchanged.

- [x] **Task 7 — Inversions** (AC6): the three required inversions + one per new discriminating assertion;
      table in the Dev Agent Record; `git status` clean.

- [x] **Task 8 — Gates** (AC7): Rust, `tsc`, JVM (`--rerun-tasks`), puppeteer proxy smoke with evidence,
      surface-smoke traps #1/#2/#6, then hand GATE-4 to Andi. Anchor the record by **symbol**, write resolution
      rows from `git diff` (Epic-7 retro D2).

## Dev Notes

### Verified current state (today's tree, `conductor/story-7-9` at `f164056`)

Anchors below are symbols, not line numbers (project-context rule). Re-grep before editing.

**Dead-key sites per layer**

| Layer | Symbol / file | Carries |
|---|---|---|
| Rust | `config::AdvancedSettings` + `default_*` fns + `impl Default` | #1–#7, #10–#13 |
| Rust | `config::AppConfig` | #8, #9 |
| Rust | `commands::settings::{SettingsPatch, merge_settings, save_settings, get_settings}` | #8, #9 |
| Rust | `commands::settings::save_advanced_settings` (license gate) | #10–#13 |
| Rust | `lib.rs::SettingsView` + its tests | #8, #9 |
| TS | `src/types.ts::{AdvancedSettings, AppSettings}` | all 13 |
| TS | `src/tauri-commands.ts::{MOCK_ADVANCED_SETTINGS, mock AppSettings, saveSettings}` | all 13 |
| TS | `src/hooks/useSettings.ts::handleSaveSettings` | #8, #9 |
| TS | `src/components/SettingsPanel.tsx` | #6–#9 |
| TS | `src/components/settings/ShortcutsContent.tsx` | #6–#9 (rows for #6, #7 only) |
| TS | `src/components/AdvancedSettingsPanel.tsx` | #1–#5, #10–#13 (+ defaults for #6, #7) |
| Kotlin | `KlarvoApi.Config`, `KlarvoApi.readConfig` | #8, #9 |
| Kotlin | `KlarvoOverlayService.{tapAutoSend, longPressAutoSend, loadBubbleControls, shouldAutoSend block}` | #8, #9 |

**The "lying UI" (three disagreeing default sets, verified):** `llmMaxTokens` UI `ADVANCED_DEFAULTS` 1024 /
Rust `default_llm_max_tokens` 4096 / runtime 2048 · `llmTemperature` 0.3 / 0.0 / 0.3 · `chunkThreshold` 400 / 800 /
400 · `chunkTargetSize` 300 / 600 / 350 · `autoCapitalize` UI false / Rust true · preview mock chunks 120 / 90 and
`llmModelAnthropic: "claude-haiku-20240307"`. (The backlog's "config 600" for the threshold is the target-size
default; the tree is authoritative.)

**Model-ID sites**

| Provider | Rust default (source of truth) | Rust construction today | Kotlin today |
|---|---|---|---|
| DeepSeek | `DeepSeekCleanup::DEFAULT_MODEL` `"deepseek-chat"` | `cleanup_provider_for` `_` arm; `update_api_keys` | `resolveLlmProvider` `else ->` arm + `cleanupFallbackCandidates` DeepSeek triple (literal) |
| OpenAI | `OpenAiCleanup::DEFAULT_MODEL` `"gpt-4o-mini"` | `cleanup_provider_for` `"openai"` | both sites (literal) |
| Groq | `GroqCleanup::DEFAULT_MODEL` `"llama-3.3-70b-versatile"` | `cleanup_provider_for` `"groq"` | `resolveLlmProvider` `"groq"` arm only (Groq is never a fallback candidate — story 12-1 AC2, keep it that way) |
| Anthropic | `AnthropicCleanup::DEFAULT_MODEL` `"claude-haiku-4-5-20251001"` | `resolve_cleanup_provider` `"anthropic"` arm (not a fallback candidate) | **no provider** (H5) |
| OpenRouter | — (inline `"deepseek/deepseek-chat"`) | `cleanup_provider_for` `"openrouter"` | both sites (literal) — stays hard-coded (**Q8**) |

`KlarvoOverlayService` already logs the resolved model: `[pipeline] cleanup: …ms (${llmProvider.model})` (debug)
and `cleanup fallback succeeded (${fallbackProvider.model})` (info).

### Traps and what must be preserved

- **Positional TS save chain** (trap #6) — see Task 4. Also confirm `App.tsx` still passes
  `settings.handleSaveSettings` unchanged.
- **Stale advanced snapshot (verify, pre-existing):** `SettingsPanel` loads `advancedSettings` once on mount and,
  when its tracked advanced fields differ, saves `{...advancedSettings, …}` via `saveAdvancedSettings` — a
  **whole-block replace**. With model IDs now live, check end-to-end that a model ID saved in the embedded
  Advanced panel is not reverted by a later Settings save in the same session. If it is, that breaks this story's
  feature path.
- **No hot reload on the Advanced save** — see AC5; without it GATE-4 fails until restart.
- **Epic-12 fallback ladder on both sides** must carry the override (Rust `resolve_fallback_provider` →
  `cleanup_provider_for`; Kotlin `resolveFallbackLlmProvider` → `cleanupFallbackCandidates`). Keep Groq out of both
  ladders (12-1 AC2) and keep `LlmProviderInfo.providerName` semantics (12-1 finding C).
- **License gating on Android** (`readConfig` strips paid keys when unlicensed) is unrelated to model IDs —
  do not route the override through that gate.
- **Golden-master tests** (`test_appconfig_golden_master_full_field_roundtrip`) set every field non-default on
  purpose; delete the removed fields there, do not weaken the test.
- **`insertAndSend` ≠ bubble auto-send.** Desktop "Auto-Send" (`HotkeySlot::insert_and_send`, UI
  `insertAndSendSlot1`) is live and stays.
- **Canon check:** `docs/design/overhaul/source/Klarvo Design System.html` shows an "Auto-Paste ins Feld" toggle
  **only inside the "Form-Komponenten-Kit" board** (a component demo next to a "Temperature" slider demo). It is
  not a settings spec and does not pin `autoPaste` as a real setting. No canon screen pins the Advanced panel's
  field list.

### Open questions — DECIDED at GATE 1 (Andi, 2026-09-12; conductor-recorded)

The questions below stay verbatim for traceability. **Binding answers (apply these; do not re-open):**

- **Q1 → only the 13 named keys.** The "14" was a counting slip. `pasteDelayMs`, `logLevel`, `webhookHeaders`,
  `webhookTimeoutSecs` stay untouched; they are recorded as candidates in `docs/backlog.md`.
- **Q2 → copy:** STT home row "Custom prompts" · Text Cleanup home row "Model IDs" · subsection title "Model IDs" ·
  expert-mode hint "Reveals raw audio thresholds". No other wording changes.
- **Q3 → layout:** the "Model & Parameters" accordion goes; the four model-ID inputs sit flat under the "Model IDs"
  title. The Expert Mode toggle stays and gates only the Audio thresholds.
- **Q4 → Paste & Behavior (Desktop):** the dead "Auto-Paste" toggle goes. "Auto-Send" stays an on/off toggle
  (default off, `insertAndSendSlot1`) and "Paste Delay" stays; neither is dimmed or disabled any more. Section
  heading "Paste & Behavior" stays. **Android:** the two `bubble*AutoSend` keys are removed as decided (M13);
  Android stays without auto-send; revival is a future story candidate, not this one.
- **Q5 → whitespace-only is empty:** trim, then empty → default. Same predicate on both platforms, pinned by a vector.
- **Q6 → confirmed:** `llmModelAnthropic` is Desktop-only (H5, ADR-0016 Amendment 1). The epic's "both platforms"
  means the three providers Android has.
- **Q7 → confirmed in scope:** the Rust cleanup path logs the model ID it sends, so GATE-4 can observe it.
- **Q8 → confirmed:** OpenRouter stays hard-coded.

Original questions as raised by create-story:

- **Q1 — 13 named vs "14" counted.** The tree has more persisted-but-unread `AdvancedSettings` keys the decision
  did not name: `pasteDelayMs` (Shortcuts "Paste Delay" row; `paste/mod.rs` hard-codes a 50 ms sleep),
  `logLevel` (Advanced → System select; no reader), `webhookHeaders` / `webhookTimeoutSecs` (no UI, no reader).
  Which one, if any, is the 14th? This story removes only the 13 named.
- **Q2 — Copy made false by the removal.** Advanced home rows "Custom prompts & temperature" (STT) and "Models,
  parameters & instructions" (Text Cleanup); subsection title "Model & Parameters"; expert-mode hint "Reveals raw
  audio thresholds, chunking and STT temperature…". New wording is not pinned by canon.
- **Q3 — Layout after removal.** Text Cleanup keeps only the four model inputs inside the collapsible "Model &
  Parameters" accordion (the "Custom Cleanup Instructions" accordion disappears): keep the accordion or flatten?
  Expert mode then gates only the Audio row (STT and Text Cleanup lose their only expert items): keep as is?
- **Q4 — Paste & Behavior without Auto-Paste.** "Auto-Send" and "Paste Delay" are dimmed/disabled while
  `localAutoPaste` is false. With the toggle gone: always enabled? Does the section keep its heading/layout?
  (If Q1 also removes `pasteDelayMs`, the section holds only "Auto-Send".)
- **Q5 — Empty model ID.** Does whitespace-only count as empty (trimmed → default) or pass through as the model?
  The epic says "an empty value"; both platforms must use the same predicate, pinned by a vector.
- **Q6 — `llmModelAnthropic` on Android.** Assumed Desktop-only (no Android Anthropic provider, H5 accepted).
  The epic says "4 keys, both platforms".
- **Q7 — Log visibility for GATE-4.** Assumed in scope: the Rust log must name the cleanup model, or the gate
  "changed DeepSeek model ID appears in the request log" cannot be observed.
- **Q8 — OpenRouter model.** Stays hard-coded (`"deepseek/deepseek-chat"`); no override key exists.

### Previous-story intelligence (7-8, same epic)

- **Symbol anchors, diff-sourced record rows.** 7-8 spent four review rounds, mostly on record text whose line
  anchors aged inside the fixing commit (Epic-7 retro D2). Keep the record lean and anchored by symbol.
- **Bind tests to production symbols vs fixture literals** ("the SUT must not judge itself"): read the constant
  from production, compare to the fixture literal, never to another production symbol.
- **Inversion discipline:** a fixed inversion has to fail *differently* than the bug did; mark any
  non-discriminating inversion as such (7-8 row E).
- **Gradle stale green** on fixture-only edits → `--rerun-tasks`. Count JVM results over all result XMLs (the
  `android-smoke.sh` banner reads one suite).
- **Device-free JVM run** (when no AVD is reachable): sync `android/kotlin-src` + `android/kotlin-test` into
  `src-tauri/gen/android/app/src/{main,test}/java/com/klarvo/voice`, then
  `./gradlew :app:testUniversalDebugUnitTest --rerun-tasks` with `JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64`
  and `ANDROID_HOME=/home/andyon2/workspace/tools/android-sdk`. The AVD lives on the **laptop**
  (`emulator-5554`); boot only via `scripts/android-emulator.sh`, stop it afterwards.
- 7-8 named `KlarvoApi.DEEPSEEK_CHAT_URL`, `CLEANUP_TEMPERATURE`, `CLEANUP_MAX_TOKENS` — same pattern for model
  default constants.

### Git intelligence

The last five commits are the planning trail for this story: `633f1b8` (backlog decision), `be5da78` (epic-7
reopened), `f164056` (story homed in the epic). The code this story touches was last shaped by 7-8
(`00e771d`/`27de205`/`85aa0ee`: `KlarvoApi` constants, twin-fixture tests in `llm/mod.rs`) and 12-1 (`130c636`: the
fallback ladder on both platforms — the override must follow it). Small scoped commits, never `git add .`; one
commit per platform is allowed (backlog Decision 3).

### Latest technical information

No web research was performed in this session (no web access). The story changes no library version and keeps
today's default model IDs; the override exists precisely so a retired ID can be changed without an update.

### Project Structure Notes

- Rust tests stay **inline `#[cfg(test)]`**; no new file under `src-tauri/tests/`.
- Fixtures stay in the repo-root `test-fixtures/` (both loaders resolve it from there).
- Kotlin sources in `android/kotlin-src/`, tests in `android/kotlin-test/` — never in `gen/android/`.
- No new dependency; do not run `npm install` (Windows build runs `npm ci`).
- Code/comments English; commit subjects English.

### References

- [Source: _bmad-output/planning-artifacts/epics-cross-platform-parity.md#Story 7.9] — ACs, out-of-scope, DoD.
- [Source: docs/backlog.md#DECIDED 2026-09-11 — Desktop Advanced settings + AutoSend] — Decisions 1–3.
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-2026-09-10.md] — audit evidence, 7.7 dead-config row.
- [Source: docs/cross-platform-drift-audit.md] — M13, H5, M16, dead-config catalogue.
- [Source: docs/adr/0016-android-path-parity-strategy.md#Amendment 1] · [Source: docs/adr/0017-shared-core-stt-path.md]
- [Source: _bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md] — fixture discipline, inversion table, gate procedure.
- [Source: _bmad-output/implementation-artifacts/epic-7-retro-2026-09-10.md] — D2 record rule, AI-5.
- [Source: docs/surface-smoke-checklist.md] — traps #1, #2, #6.
- [Source: src-tauri/src/config/mod.rs] — `AdvancedSettings`, `AppConfig`, `load_config_reporting`, `backup_corrupt_config`, Story 1.2 test helpers.
- [Source: src-tauri/src/commands/settings.rs] — `SettingsPatch`, `merge_settings`, `save_settings`, `get_settings`, `save_advanced_settings`, `update_api_keys`.
- [Source: src-tauri/src/pipeline.rs] — `resolve_providers`, `cleanup_provider_for`, `resolve_cleanup_provider`, `resolve_fallback_provider`, cleanup log line.
- [Source: src-tauri/src/llm/mod.rs] — `*Cleanup::DEFAULT_MODEL`, `with_model`, `llm::tests::spec_twin_constants_*`.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoApi.kt] — `Config`, `readConfig`, `parseMinRecordingMs`, `resolveLlmProvider`, `cleanupFallbackCandidates`.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt] — `loadBubbleControls`, auto-send block.
- [Source: src/components/AdvancedSettingsPanel.tsx] · [Source: src/components/settings/ShortcutsContent.tsx] · [Source: src/components/SettingsPanel.tsx] · [Source: src/hooks/useSettings.ts] · [Source: src/tauri-commands.ts] · [Source: src/types.ts]
- [Source: test-fixtures/twin-constants-vectors.json] · [Source: test-fixtures/README.md]
- [Source: _bmad-output/project-context.md] — gates, symbol anchors, "a number states what it covers", no host mutation.

## Dev Agent Record

### Agent Model Used

Claude Opus 5 (claude-opus-5)

### Debug Log References

**Gate runs (all from `conductor/story-7-9`, 2026-09-12):**

- `cargo test --lib` in `src-tauri/` — **674 passed, 0 failed, 0 ignored**.
  Covers: Rust unit + inline spec tests, Linux target. Does NOT cover: Windows-only
  paths (`whisper-rs`, `llama-cpp-2`, `local` cleanup arm, native overlays), the Tauri
  runtime, or `tests/pi_security.rs` (unchanged by this story; its tier tests need keys).
- `npm run build` (`tsc && vite build`) — **green**, `✓ built in 1.53s`. Strict `tsc`
  type-checks the whole positional save chain; it does not execute anything.
- JVM gate `./gradlew :app:testUniversalDebugUnitTest --rerun-tasks` — **184 tests, 0 failures,
  0 errors, 0 skipped across 23 suites**. `--rerun-tasks` was mandatory (this story edits
  `test-fixtures/twin-constants-vectors.json`, and gradle reports a stale green on a
  fixture-only edit).
  - Run **device-free** (7-8 precedent): `android/kotlin-src` + `android/kotlin-test` synced
    with-delete into `src-tauri/gen/android/app/src/{main,test}/java/com/klarvo/voice`, then
    gradle directly. `scripts/android-smoke.sh` was **not** used: it fails on "Kein Gerät
    gefunden" before reaching its JVM gate, and no device/AVD was reachable from this host.
  - Count taken over **all result XMLs in the `testUniversalDebugUnitTest` variant dir**, not
    the smoke banner's single suite. An earlier count of "1156" was wrong — it summed nine
    stale variant dirs (`testArm64Debug…`, `testX86…`) left from older runs; 184 is the real
    figure for this run.
  - Covers: pure Kotlin/JVM logic. Does **NOT** cover JNI (no `nativeTranscribe`/
    `nativeSilenceCheck` call is reached), any device or emulator, Android UI/rendering, or
    the real `readConfig` file I/O and license gating.
- **Desktop proxy smoke** (puppeteer 24.38.0 vs `npm run preview` :1422, real Chromium) —
  **26/26 checks passed, 0 uncaught page errors**. Harness + evidence in
  `_bmad-output/implementation-artifacts/gate4-evidence/7-9/` (`smoke.mjs`, `smoke-report.json`,
  8 screenshots, captured panel text). `git status` clean afterwards; no throwaway edit to
  `src/tauri-commands.ts` was needed (the mocks reach every state this story touches).
  - **What it proves: wiring and structure only** — which rows/inputs/titles the React tree
    actually renders. Walked: Settings → Shortcuts → Paste & Behavior, and the Advanced panel's
    four sections (Speech-to-Text, Text Cleanup, Audio, System) with **expert mode both off and
    on** (Audio's home row is correctly absent with expert off, present with expert on).
  - **What it does NOT prove / never drove:** design or pixels of any kind; the Rust backend
    (preview serves MOCK data, no Tauri — so no `save_advanced_settings`, no cleanup-provider
    hot-reload, no `Klarvo.log` line, no real `config.json`); Android (no Kotlin path); the
    Windows release build; any real cleanup request carrying an overridden model ID; fonts or
    Windows text-scale drift.
  - 2 `console.error` lines appear in preview and are **pre-existing, not from this story**:
    `SettingsPanel.tsx`'s `voice-command-state-changed` effect calls `listen()` from
    `@tauri-apps/api/event` *without* the `isPreviewMode` guard that `tauri-commands.ts`'s own
    `listen()` wrapper applies, so it touches `window.__TAURI_INTERNALS__` (absent in a plain
    browser). Its own `.catch(console.error)` handles it; React StrictMode mounts the panel
    twice, hence two lines. Cannot occur in the real app. Left alone — not mapped to any task here.
- Rust warning count unchanged at **14** before and after (verified by building `HEAD` stashed
  vs the working tree) — no new warnings.

**Two harness traps found and fixed while writing the smoke** (both would have been read as
product defects; recorded so the next story does not re-hit them):

1. A `fullPage: true` screenshot resizes the emulated viewport, which **remounts the app** and
   resets the Settings panel to its home view — measured as 31 divs → 1 → 12, with
   `[aria-label="Back to settings"]` gone and every row query returning `[]`. All screenshots
   are now viewport-only.
2. The evidence dir lives **inside the repo**, and `npm run preview` is the Vite *dev* server:
   its watcher saw each `.png`/`.txt` the harness wrote and fired an HMR full reload mid-run.
   Artifacts are now staged in a temp dir and copied in only after the browser closes.
   Both traps produced *zero-count* queries, so the harness now **fails** on a zero count
   instead of passing vacuously (`VACUOUS —` guards on the Paste & Behavior rows and on the
   section walk).

**Surface-smoke-checklist traps, run mechanically (not self-attested):**

| Trap | Check | Result |
|---|---|---|
| #1 camelCase keys under `advanced` | derived every remaining `AdvancedSettings` serde name from the Rust struct and diffed against `src/types.ts` | **17 = 17**, no key missing on either side |
| #2 resync `useEffect` / `isDirty` | the two surviving advanced fields (`silenceThreshold`, `pasteDelayMs`) present in the mount load, `isDirty` and the `updatedAdv` save block; no removed key left behind | green |
| #6 multi-hop positional save chain | parsed the parameter list of all five hops and compared arity against `HEAD` | every hop **−2 exactly** (51→49, 51→49, 51→49, 53→51, 53→51); no removed key anywhere in the chain, so nothing shifted position |

### Inversion table (AC6)

Each row: the drift re-introduced → the test that went RED → reverted, suite green again.
`git status` is clean; no inversion is still in the tree.

| # | Reverted change (symbol) | Test(s) that went RED | Discriminating? |
|---|---|---|---|
| 1 | **Kotlin** `KlarvoApi.cleanupFallbackCandidates`, DeepSeek triple: `model = effectiveCleanupModel(...)` → hard-coded `"deepseek-chat"` | `LlmFallbackProviderTest.modelOverride_fallbackLadder_reachesProviderInfo`, `.modelOverride_bothCallSitesAgree` (expected `deepseek-reasoner`, got `deepseek-chat`) | yes — and `blankModelOverride_fallsBackToBuiltInDefault` stayed **GREEN**, exactly AC6's non-discriminating trap |
| 2 | **Rust** `pipeline::cleanup_provider_for`, `"openai"` arm: `.with_model(effective_cleanup_model(..))` dropped → bare `OpenAiCleanup::new(api_key)` | `pipeline::tests::spec_model_override_flows_through_primary_selection`, `::spec_model_override_flows_through_fallback_ladder` (expected `gpt-4o`, got `gpt-4o-mini`) | yes — and `test_resolve_cleanup_provider_openai` (default-only) stayed **GREEN**, same trap |
| 3 | **Rust/AC4** `#[serde(deny_unknown_fields)]` added to `config::AdvancedSettings` | `config::tests::spec_old_config_with_removed_dead_keys_still_loads` (live `minRecordingMs` fell back to 500 instead of 750 — i.e. the test saw the corrupt-recovery path) | yes — fails on the *live-value-survived* assertion, not on "it loaded" |
| 4 | **Rust** `llm::effective_cleanup_model`: `override_raw.trim()` → `override_raw` | `llm::tests::spec_effective_model_trims_override`, `::spec_effective_model_blank_override_uses_default`, `pipeline::tests::spec_model_override_is_trimmed`, `::spec_blank_model_override_falls_back_to_provider_default` (4 RED) | yes — pins the Q5 predicate, not just the default |
| 5 | **Kotlin** `KlarvoApi.resolveLlmProvider`, `else ->` arm: `model = effectiveCleanupModel(...)` → hard-coded `"deepseek-chat"` (covers the *primary* site, which inversion 1 did not) | `LlmFallbackProviderTest.modelOverride_primarySelection_reachesProviderInfo`, `.modelOverride_bothCallSitesAgree`, `LlmModelOverrideConfigTest.oldConfigJson_modelOverride_reachesResolvedProvider` | yes |
| 6 | **Kotlin** `KlarvoApi.effectiveCleanupModel`: `override.trim()` → `override` (applied together with 5; 8 RED in total across the two) | `LlmFallbackProviderTest.blankModelOverride_fallsBackToBuiltInDefault`, `.paddedModelOverride_isTrimmed`, `LlmModelOverrideConfigTest.effectiveCleanupModel_blankOverrideUsesDefault`, `.effectiveCleanupModel_trimsPaddedOverride`, `TwinConstantsVectorsTest.cleanupModelDefaultsMatchFixture` | yes — the Kotlin twin of inversion 4 |
| 7 | **Fixture** `TWIN-CLEANUP-MODEL-OPENAI-001.expected_string`: `"gpt-4o-mini"` → `"gpt-4o-mini-DRIFTED"` | **both halves independently**: Rust `llm::tests::spec_twin_constants_cleanup_model_defaults` AND Kotlin `TwinConstantsVectorsTest.cleanupModelDefaultsMatchFixture` | yes — proves neither half can hide the other's drift |

### Completion Notes List

- **AC1 — 13 dead keys removed from every layer.** Verified by grep over `src/`,
  `src-tauri/src/` and `android/kotlin-src/` in **both** spellings
  (`camelCase|snake_case`): **zero production hits** remain for all 13. Before the change the
  same grep returned 13–33 hits per key. `llm::CHUNK_THRESHOLD` / `CHUNK_TARGET_SIZE` (the live
  runtime constants, different case) are untouched.
  - Kotlin removal was exactly #8 + #9 as the story predicted: `KlarvoApi.Config`,
    `KlarvoApi.readConfig`, `KlarvoOverlayService.{tapAutoSend, longPressAutoSend,
    loadBubbleControls, the unreachable shouldAutoSend block}`. No `advanced.*` dead key was
    read on the Kotlin side.
  - **`KlarvoAccessibilityService.performEnter` has lost its last caller** (grep: no reference
    outside its own file). Left in place as the task directs — no other paste behaviour changed.
  - Doc comments that would now lie were corrected: `AdvancedSettings::expert_mode`, the
    "six fields"→"four fields" note above the bubble-gesture block in `AppConfig`,
    `AdvancedSettings` TS doc, and the `Expert mode` inline comment in `AdvancedSettingsPanel`.
  - `save_advanced_settings`' license gate went with the four prompt keys, as specified.
    `LicensedFeature::CustomPrompts` now has no production gate and stays referenced only by
    `license::tests::test_licensed_allows_all_features`; the enum variant was **not** removed.
- **AC2 — no runtime behaviour change for the removed keys.** The five pre-existing
  `twin-constants-vectors.json` entries keep their values (0.3 / 2048 / 400 / 350 / LF) and
  both 7-8 halves stay green. `custom_prompt` untouched. The **"Auto-Send" row stays** and is
  still bound to `insertAndSendSlot1`, not to `bubbleTapAutoSend` — confirmed in the proxy smoke.
- **AC3 — live keys untouched.** `sttPrompt*`, `silenceThreshold`, `minRecordingMs`,
  `whisperMode*`, `pasteDelayMs`, `logLevel`, `uiScale`, `expertMode`, `webhook*` all retained;
  `nativeSilenceCheck`'s JNI signature is unchanged (not touched at all).
- **AC4 — old `config.json` still loads, both platforms.** Rust:
  `config::tests::spec_old_config_with_removed_dead_keys_still_loads` feeds raw JSON carrying
  all 13 removed keys at non-default values plus non-default live keys, then asserts the live
  values survived **and** `corrupt_backups(dir).is_empty()` **and** `warnings.is_empty()` —
  not merely "it loaded" (the vacuous-pass trap). Inversion 3 proves it discriminates. Kotlin:
  `LlmModelOverrideConfigTest.oldConfigJson_withRemovedDeadKeys_stillYieldsLiveValues` drives
  the same JSON text through the real `org.json` production seams. No
  `deny_unknown_fields` was added anywhere.
- **AC5 — model IDs are a real override.**
  - One place decides both halves of the rule: **`llm::effective_cleanup_model`** (trim, then
    empty → the provider's `DEFAULT_MODEL`), with the Kotlin twin
    **`KlarvoApi.effectiveCleanupModel`**. The per-arm empty-check the story warned against
    does not exist.
  - Rust: `pipeline::cleanup_provider_for` now takes `&config::AdvancedSettings` and gained an
    `"anthropic"` arm, so **every** construction path goes through one function —
    `resolve_cleanup_provider` (all arms, incl. `"anthropic"`, which now delegates instead of
    constructing directly), `resolve_fallback_provider`, and therefore `resolve_providers` from
    `lib.rs` and both call sites in `commands/settings.rs`. Adding the arm does **not** make
    Anthropic a fallback candidate: `resolve_fallback_provider`'s candidate list is unchanged
    (deepseek → openai → openrouter), and Groq stays excluded (12-1 AC2, its tests still green).
  - Legacy `commands::settings::update_api_keys` was routed through the same resolution, so it
    can no longer bypass the override.
  - `#[allow(dead_code)] // builder API for future use` removed from **all four**
    `with_model` builders (the story said "the builders now in use"; all four are now used).
  - Kotlin: three overrides parsed through a new pure seam
    `KlarvoApi.parseLlmModelOverride(json, key)` (shape of `parseMinRecordingMs`), carried on
    `Config` (appended at the tail — `readConfig` builds it positionally), and applied at
    **both** independent sites (`resolveLlmProvider` and `cleanupFallbackCandidates`).
    Defaults named as `DEFAULT_MODEL_DEEPSEEK/OPENAI/GROQ` (7-8 precedent).
  - `llmModelAnthropic` stayed Desktop-only (Q6); **no Android Anthropic provider was added**.
  - OpenRouter's `"deepseek/deepseek-chat"` stays hard-coded on both sides (Q8) and is now
    *pinned* as such on both (`test_resolve_cleanup_provider_openrouter`,
    `openrouterModel_staysHardCoded`) so a future "wire everything" pass has to decide
    deliberately.
  - **No app restart needed:** `save_advanced_settings` now rebuilds
    `AppState::cleanup_provider` from the persisted config (mirroring `save_settings`'
    `resolve_providers` step). Still exactly one `save_config_locked` writer (ADR-0015).
  - **Q7 (GATE-4 observability):** `CleanupProvider` gained `fn model(&self) -> &str`
    (default `""` for the in-module test doubles; overridden by all five network providers), and
    the pipeline's cleanup line now reads
    `[pipeline] LLM cleanup took {}ms (provider: {}, model: {}, style: {:?}, input_len: {})`.
    The fallback-success line also names the model, mirroring Android's
    `cleanup fallback succeeded (${'$'}{fallbackProvider.model})`.
- **AC6 — 7 inversions**, table above. Two of them also *document* the non-discriminating
  trap by naming the default-only test that stayed green.
- **Q1–Q8 applied as decided at GATE 1.** Q1: only the 13 named keys — `pasteDelayMs`,
  `logLevel`, `webhookHeaders`, `webhookTimeoutSecs` untouched. Q2/Q3: STT home row
  "Custom prompts", Text Cleanup home row + subsection title "Model IDs", expert hint
  "Reveals raw audio thresholds.", the four model inputs now **flat** (both accordions gone),
  Expert Mode toggle kept and gating only the Audio thresholds. Q4: "Auto-Paste" removed;
  "Auto-Send" and "Paste Delay" kept, **no longer dimmed or disabled** (asserted in the smoke),
  heading "Paste & Behavior" kept. Q5: trim-then-empty on both platforms, pinned by vectors.
- **Fixture claim accuracy (Task 6's 7-8 defect class):** the three stale `DOES NOT PIN`
  clauses that named keys this story deletes were corrected (values unchanged), and the
  `TwinConstantsVectorsTest` KDoc no longer claims the dead-config cluster is "deliberately
  NOT locked" — it now says those keys no longer exist. Both "exactly five" assertions became
  "exactly nine", honestly worded, and `test-fixtures/README.md` records that the Kotlin half
  asserts 8 of 9 (Anthropic is Desktop-only and skipped by id).
- **One in-scope correction made during the work:** I briefly added `localPasteDelayMs` to the
  save-callback dependency array in `SettingsPanel.tsx` — a pre-existing missing dep, not
  mapped to any task — and reverted it. It remains absent, as on `HEAD`.

**Not done / handed on:**

- **Andi's GATE-4 is outstanding**: Windows release build via `scripts/windows-build.sh`,
  Advanced panel shows only live keys, and a **changed DeepSeek model ID visible in
  `Klarvo.log`**. The log line needed for it exists now (see Q7 above), but nothing in this
  session ran on Windows or in the Tauri runtime.
- **The "stale advanced snapshot" check the Dev Notes asked for is only partly closed.** The
  code path is confirmed by reading: `SettingsPanel` loads `advancedSettings` once on mount and
  saves `{...advancedSettings, silenceThreshold, pasteDelayMs}` — a whole-block replace of a
  possibly stale snapshot. With model IDs now live, a model ID saved in the embedded Advanced
  panel *can* be reverted by a later Settings save in the same session, because
  `SettingsPanel`'s `advancedSettings` state is not refreshed after
  `AdvancedSettingsPanel` saves its own block. **I could not drive this end-to-end**: it needs
  a real backend (`save_advanced_settings` + `get_advanced_settings`), which the browser proxy
  does not have. Flagging it rather than claiming it verified.
- No device/emulator Android smoke (no device reachable; the JVM half was run device-free). Per
  project-context the real-device gate is mine to run, so this is a genuine gap, not a hand-off
  — but it is a pure-logic change with no Android UI surface, and the logic half is green.

### File List

**Rust (desktop)**
- `src-tauri/src/config/mod.rs` — removed 11 `AdvancedSettings` fields + 7 `default_*` fns, 2 `AppConfig` fields; corrected doc comments; new AC4 test; updated 5 existing tests
- `src-tauri/src/commands/settings.rs` — `SettingsPatch` (+`Default`), `merge_settings`, `save_settings` params + patch literal, `get_settings`; license gate removed from `save_advanced_settings` + cleanup-provider hot-reload added; `update_api_keys` routed through the override; 4 tests updated
- `src-tauri/src/lib.rs` — `SettingsView`: 2 fields removed (+3 test literals)
- `src-tauri/src/llm/mod.rs` — new `effective_cleanup_model`; `CleanupProvider::model()` + 5 impls; `#[allow(dead_code)]` dropped from 4 `with_model` builders; 3 `DEFAULT_MODEL` consts widened to `pub(crate)`; new fixture test + 5 new spec tests
- `src-tauri/src/pipeline.rs` — `cleanup_provider_for` takes `&AdvancedSettings` and gained an `"anthropic"` arm; all call sites threaded; cleanup + fallback log lines name the model; 6 existing resolution tests strengthened, 4 new override tests

**Kotlin (Android)**
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` — 3 `DEFAULT_MODEL_*` consts + `effectiveCleanupModel` + `parseLlmModelOverride`; `Config` −2/+3 fields; `readConfig` reads; override applied in `resolveLlmProvider` and `cleanupFallbackCandidates`
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — `tapAutoSend`/`longPressAutoSend` fields, their `loadBubbleControls` assignment + debug-log fields, and the unreachable `shouldAutoSend` block removed
- `android/kotlin-test/com/klarvo/voice/LlmFallbackProviderTest.kt` — `baseConfig` +3 params; 6 new tests (9 → 18)
- `android/kotlin-test/com/klarvo/voice/LlmModelOverrideConfigTest.kt` — **new**, 8 tests (parse seam, Q5 predicate, AC4 Kotlin half)
- `android/kotlin-test/com/klarvo/voice/TwinConstantsVectorsTest.kt` — KDoc claims corrected; "five"→"nine"; 2 new tests (6 → 8)

**Frontend (TS/React)**
- `src/types.ts` — `AdvancedSettings` −11, `AppSettings` −2; `expertMode` doc corrected
- `src/tauri-commands.ts` — `MOCK_ADVANCED_SETTINGS` (incl. dropping the stale `llmModelAnthropic: "claude-haiku-20240307"`), mock `AppSettings`, `saveSettings` params + `invoke` keys
- `src/hooks/useSettings.ts` — `handleSaveSettings` params + forwarding call
- `src/components/SettingsPanel.tsx` — `onSave` prop type, the `onSave(...)` call, 4 local states, mount load, resync `useEffect`, `isDirty` terms + deps, `updatedAdv` save block, `ShortcutsContent` props
- `src/components/settings/ShortcutsContent.tsx` — props −8; "Auto-Paste" + "Auto-Capitalize" rows removed; Q4 un-dimming of "Auto-Send" and "Paste Delay"
- `src/components/AdvancedSettingsPanel.tsx` — `ADVANCED_DEFAULTS` −11; STT temperature row; whole Text Cleanup body rebuilt flat under "Model IDs" (both accordions + their state/`toggleSubSection` gone); Q2 copy on 3 strings

**Fixtures / docs / tracking**
- `test-fixtures/twin-constants-vectors.json` — 4 new model-default entries (5 → 9); 3 stale `DOES NOT PIN` clauses corrected
- `test-fixtures/README.md` — reader ledger notes the 8-of-9 Kotlin coverage
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — 7-9 → `review`; `last_updated`
- `_bmad-output/implementation-artifacts/7-9-desktop-advanced-settings-dead-keys-and-model-ids.md` — this record
- `_bmad-output/implementation-artifacts/gate4-evidence/7-9/` — **new**: `smoke.mjs`, `smoke-report.json`, 8 screenshots, 6 captured-state files

### Change Log

| Date | Change |
|---|---|
| 2026-09-12 | Removed the 13 settable-but-dead config keys across React/TS, Rust config+commands and the Kotlin twin; no runtime behaviour change for them (ADR-0016 Amendment 1). |
| 2026-09-12 | Wired `advanced.llmModel{Deepseek,Openai,Groq,Anthropic}` as a real cleanup-model override on every Rust construction path and both Kotlin sites; one shared trim-then-default predicate per platform (`llm::effective_cleanup_model` / `KlarvoApi.effectiveCleanupModel`). Anthropic stays Desktop-only (H5); OpenRouter stays hard-coded (Q8). |
| 2026-09-12 | `save_advanced_settings` now hot-reloads the cleanup provider, and the pipeline logs the resolved model ID — so a changed model ID takes effect without a restart and is observable in `Klarvo.log` (GATE-4, Q7). |
| 2026-09-12 | Advanced panel re-laid out per Q2/Q3 (flat "Model IDs", both accordions gone, corrected copy) and Shortcuts → Paste & Behavior per Q4 (Auto-Paste gone; Auto-Send + Paste Delay kept, no longer dimmed). |
| 2026-09-12 | Parity fixture gained the 4 default model IDs (5 → 9 entries) and its three stale dead-key claims were corrected; both "exactly five" assertions re-worded to nine. |

### Review Findings

Code review (bmad-code-review, 2026-09-12) over the committed range `f164056..HEAD`.
Three layers ran, none failed: Blind Hunter (diff only), Edge Case Hunter (diff + tree),
Acceptance Auditor (diff + spec + context). Every finding below was re-verified against
today's tree before being recorded; anchors are `file::symbol` with a line for convenience.

- [ ] [Review][Decision] **Is the model-ID override a licensed feature?** — `AdvancedSettingsPanel` still renders `{isPaid && isTrial && <TrialBadge />}` on the "Text Cleanup" home row (`src/components/AdvancedSettingsPanel.tsx:205`), but the section's only remaining content — the four model-ID inputs in `renderLlmContent` (`:313-330`) — carries no `disabled={!isPaid}` and no `LockIcon`, unlike the STT section right above it (`:284-302`). The backend gate went with the four prompt keys (`commands::settings::save_advanced_settings`). Badge, input `disabled` and backend must agree in one direction. Not decided by Q1–Q8.
- [ ] [Review][Decision] **A typo'd model ID fails every cleanup with no fallback and no feedback** — `llm::effective_cleanup_model` only trims (`src-tauri/src/llm/mod.rs:1277-1289`), the UI input is unvalidated free text (`src/components/AdvancedSettingsPanel.tsx:313-330`), and the Epic-12 ladder only triggers on `is_retryable_llm_error` (`src-tauri/src/pipeline.rs:1363`) — a 400 `model_not_found` is not retryable, so cleanup fails on every dictation. Making the key live introduces this failure mode. Options: validate/cap at the UI, strip control characters in `effective_cleanup_model`, or fall back to the default on a model-not-found error.

- [ ] [Review][Patch] Advanced whole-block save reverts a saved model-ID override — and the new hot-reload makes the revert take effect instantly (AC5 + the Dev Notes' mandatory end-to-end check) [src/components/SettingsPanel.tsx:240-249, :562-573, :830]
- [ ] [Review][Patch] Rust twin-lock header still carries the two stale claims Task 6 was written to kill ("five Rust↔Kotlin twins", "the dead-config cluster (deliberately not locked)"); the Kotlin half was corrected, so the two halves now disagree [src-tauri/src/llm/mod.rs:2227, :2236-2237]
- [ ] [Review][Patch] `LocalLlmCleanup` has no `model()` impl, so the new GATE-4 log line prints `model: ` empty on the Windows `local` arm [src-tauri/src/llm/mod.rs:322-324 (trait default), src-tauri/src/llm/local.rs (no impl), src-tauri/src/pipeline.rs:265, :1354-1357]
- [ ] [Review][Patch] `update_api_keys` still constructs `DeepSeekCleanup` directly despite its own comment claiming it routes through the shared resolution — a second construction site `cleanup_provider_for` does not own (Task 3) [src-tauri/src/commands/settings.rs:760-768]
- [ ] [Review][Patch] The AC4 test's cross-reference names `ConfigParseSeamTest`, which does not exist; the real Kotlin half is `LlmModelOverrideConfigTest::oldConfigJson_withRemovedDeadKeys_stillYieldsLiveValues` [src-tauri/src/config/mod.rs:4182]
- [ ] [Review][Patch] The one new runtime-state line — the `save_advanced_settings` cleanup-provider swap that AC5's "without an app restart" rests on — has no test; every new Rust test calls `resolve_cleanup_provider` directly [src-tauri/src/commands/settings.rs:700-712]
- [ ] [Review][Patch] The new hot-reload discards a loaded local GGUF model on every advanced-settings save (Windows `local` arm): `LocalLlmCleanup::new` resets `state: None` [src-tauri/src/commands/settings.rs:709-711, src-tauri/src/llm/local.rs:97-109]
- [ ] [Review][Patch] Kotlin `parseLlmModelOverride` coerces a JSON number/boolean into a model ID (`optString`), while the Rust twin rejects a non-string into the corrupt-recovery path — a new seam, untested on both sides [android/kotlin-src/com/klarvo/voice/KlarvoApi.kt:390-391]
- [ ] [Review][Patch] `cleanup_provider_for` gained an `"anthropic"` arm, but the "anthropic is never a fallback candidate" invariant lost its docstring sentence and has no test; only the hard-coded candidate list still enforces it [src-tauri/src/pipeline.rs:207-223, :316-330]
- [ ] [Review][Patch] Broken interpolation renders the literal `${blank.length}` in a loop's failure message; the sibling file gets it right [android/kotlin-test/com/klarvo/voice/LlmFallbackProviderTest.kt:375]
- [ ] [Review][Patch] Record-only: File List says `LlmFallbackProviderTest` went "9 → 18"; `HEAD~2` has 12 `@Test`, today 18 — the "+6 new" is right, the baseline is not [this file, File List / Kotlin]
- [ ] [Review][Patch] Record-only: Debug Log and File List say "6 captured-state files"; the evidence dir holds 7 [_bmad-output/implementation-artifacts/gate4-evidence/7-9/]

- [x] [Review][Defer] `ADVANCED_DEFAULTS.pasteDelayMs: 80` contradicts Rust `default_paste_delay_ms() -> 50` [src/components/AdvancedSettingsPanel.tsx:23, src-tauri/src/config/mod.rs:155-157] — deferred, pre-existing
- [x] [Review][Defer] A rejected `get_advanced_settings()` leaves the panel on `ADVANCED_DEFAULTS` and saveable, so a later save writes defaults over the real block [src/components/AdvancedSettingsPanel.tsx:61-66] — deferred, pre-existing (amplified by the new hot-reload)
- [x] [Review][Defer] `update_api_keys` replaces the cleanup provider with DeepSeek regardless of `cfg.llm_provider` [src-tauri/src/commands/settings.rs:760-768] — deferred, pre-existing
- [x] [Review][Defer] The four TS placeholder literals are a fourth unsynchronised copy of the default model IDs, outside the twin fixture's reach [src/components/AdvancedSettingsPanel.tsx:315-330] — deferred, pre-existing pattern
- [x] [Review][Defer] Rust `AdvancedSettings` String fields reject an explicit JSON `null` and send the loader down the corrupt-recovery path [src-tauri/src/config/mod.rs:50-60] — deferred, pre-existing across the struct
- [x] [Review][Defer] An interior newline in a model ID forges a line in `Klarvo.log`; `effective_cleanup_model` trims only the ends [src-tauri/src/pipeline.rs:1354-1357] — deferred, pre-existing class
- [x] [Review][Defer] In the Kotlin AC4 test, 2 of 4 assertions are `org.json` against `org.json`, and its KDoc reasons about `readConfig`'s `catch` without calling `readConfig` [android/kotlin-test/com/klarvo/voice/LlmModelOverrideConfigTest.kt:142-148, :92-102] — deferred, editorial
- [x] [Review][Defer] `KlarvoAccessibilityService.performEnter` has lost its last caller [android/kotlin-src/com/klarvo/voice/KlarvoAccessibilityService.kt:202] — deferred, the story explicitly directed leaving it and recording it
- [x] [Review][Defer] The Anthropic model input renders unconditionally, including any mobile surface, where no Anthropic provider exists (Q6) [src/components/AdvancedSettingsPanel.tsx:324-327] — deferred, scope
- [x] [Review][Defer] A poisoned `cleanup_provider` write lock returns `Err` after the config is already persisted [src-tauri/src/commands/settings.rs:709-711] — deferred, same pattern as `save_settings:577-579`

**Dismissed as noise (8, with the check that killed them):** "a paid feature was deleted without migration" (the four `llmSystemPrompt*` keys had no runtime reader — AC1; live `custom_prompt` untouched) · "`autoPaste`/`autoCapitalize` were live" (`git grep` at `f164056`: declaration, defaults and tests only) · "tombstone keys stay in `config.json` forever" (the next save serialises the struct and drops them — AC4 says so) · "`useCallback` is now an unused import" (still used at `AdvancedSettingsPanel.tsx:86, :92`; `tsc` green) · "`LicensedFeature`/`require_license!` now unused" (still used at `settings.rs:489`, WhisperMode) · "`effective_cleanup_model` returns the DeepSeek model for OpenRouter" (the `"openrouter"` arm hard-codes its literal and never calls it — Q8) · "the Kotlin/Rust `effectiveCleanupModel` are not real twins" (the per-site default is asserted by `LlmFallbackProviderTest`'s default-path literals) · "positional `Config(...)` tail / only DeepSeek's `model()` is asserted" (tail order verified correct; override flow-through asserted per provider via `build_request`).
