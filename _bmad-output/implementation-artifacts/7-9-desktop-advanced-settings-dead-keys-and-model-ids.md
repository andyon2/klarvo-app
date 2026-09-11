# Story 7.9: Desktop Advanced settings + AutoSend — remove dead keys, wire 4 model IDs

Status: ready-for-dev

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

- [ ] **Task 1 — Rust config + commands: remove the 13 keys** (AC1, AC2, AC3)
  - [ ] `config::AdvancedSettings`: delete fields #1–#7 and #10–#13, their `default_*` fns
        (`default_stt_temperature`, `default_llm_temperature`, `default_llm_max_tokens`,
        `default_chunk_threshold`, `default_chunk_target_size`, `default_auto_paste`,
        `default_auto_capitalize`) and their lines in `impl Default for AdvancedSettings`.
  - [ ] `config::AppConfig`: delete `bubble_tap_auto_send`, `bubble_long_press_auto_send` (+ `AppConfig::default`).
  - [ ] `commands::settings`: `SettingsPatch` fields + its `Default`; `merge_settings`; the two `save_settings`
        command params + the `SettingsPatch { … }` literal; `get_settings` → `SettingsView`.
  - [ ] `lib.rs::SettingsView`: delete both fields.
  - [ ] `save_advanced_settings`: its license gate checks **only** the four removed prompts → the gate goes with
        them. `LicensedFeature::CustomPrompts` then has no production gate; it stays referenced by
        `license::tests::test_licensed_allows_all_features` — removing the enum variant is not required.
  - [ ] Update every test that constructs/asserts a removed field — the compiler enumerates them. Known today:
        `config::tests::{test_advanced_settings_defaults, test_advanced_settings_camel_case,
        test_advanced_settings_roundtrip, test_appconfig_golden_master_full_field_roundtrip}` plus the
        AppConfig save/load roundtrip test that sets `stt_temperature: 0.2` / `bubble_tap_auto_send: true`;
        `commands::settings::tests::{test_bubble_gesture_fields_default_when_absent_from_json,
        test_bubble_gesture_fields_roundtrip}` and the `merge_settings` tests that set `bubble_*_auto_send`;
        `lib::tests::{test_settings_view_camel_case_serialization,
        test_settings_view_hotkey_mode_hold_serializes_lowercase,
        test_settings_view_hotkey_mode_toggle_serializes_lowercase}`. Keep each test's intent for the remaining fields.
  - [ ] Update doc comments that now lie: `AdvancedSettings::expert_mode` ("chunking, STT temperature").

- [ ] **Task 2 — Rust old-config test** (AC4)
  - [ ] Inline `#[cfg(test)]` in `config/mod.rs` (no new `tests/` file): write a raw JSON with all 13 keys
        (non-default) + non-default live keys, `load_config_reporting`, assert live values, `warnings.is_empty()`,
        `corrupt_backups(dir).is_empty()`.

- [ ] **Task 3 — Rust model-ID wiring** (AC5, AC7)
  - [ ] Thread the override into `pipeline::cleanup_provider_for` and `resolve_cleanup_provider` (incl. the
        `"anthropic"` arm); empty → `DEFAULT_MODEL` (same predicate everywhere, **Q5**). One place decides — do not
        duplicate the empty-check per arm.
  - [ ] `save_advanced_settings`: hot-reload `AppState::cleanup_provider` after the save (mirror `save_settings`'
        `resolve_providers` step; ADR-0015 — still one `save_config_locked` writer).
  - [ ] Legacy `commands::settings::update_api_keys` builds `DeepSeekCleanup::new(key)` directly (no caller in
        `src/` today) — it must not bypass the override; route it through the same resolution.
  - [ ] Drop `#[allow(dead_code)]` from the `with_model` builders now in use.
  - [ ] Make the resolved cleanup model visible in `Klarvo.log` for GATE-4 (**Q7**).
  - [ ] Tests (inline): override flow-through per provider (request `model` via `build_request`), empty → default,
        fallback ladder carries the override (`resolve_fallback_provider`). `pipeline::tests` already has
        `test_resolve_cleanup_provider_*` / `resolve_fallback_provider` tests to extend — reuse, don't duplicate.

- [ ] **Task 4 — Frontend removal** (AC1, AC2) — **walk the whole chain (trap #6)**
  - [ ] `src/types.ts`: `AdvancedSettings` (#1–#7, #10–#13), `AppSettings` (#8, #9); fix the `expertMode` comment.
  - [ ] `src/tauri-commands.ts`: `MOCK_ADVANCED_SETTINGS`, the mock `AppSettings`, and `saveSettings` — both the
        **positional params** `bubbleTapAutoSend`/`bubbleLongPressAutoSend` and their `invoke("save_settings", {…})` keys.
  - [ ] `src/hooks/useSettings.ts::handleSaveSettings`: positional params `newBubbleTapAutoSend`,
        `newBubbleLongPressAutoSend` + the forwarding call.
  - [ ] `src/components/SettingsPanel.tsx`: `onSave` prop type + the internal save helper's `onSave(…)` call;
        `localBubbleTapAutoSend`/`localBubbleLongPressAutoSend` state, resync `useEffect`, `isDirty` terms, deps;
        `localAutoPaste`/`localAutoCapitalize` state, the advanced-settings mount load, `isDirty`, the advanced
        save block (`updatedAdv`), the `ShortcutsContent` props.
        > **⚠ Positional-argument trap.** `saveSettings`, `handleSaveSettings` and `onSave` are **positional**.
        > Removing an argument in one hop but not the others shifts every later argument (live preview,
        > appearance, bubble size …) silently — the Epic-6 "appearance reset-on-save" class. Edit all three hops
        > plus the call in `SettingsPanel` in one change and re-read them side by side. (Rust's `save_settings`
        > maps by name, so a stray extra key there is harmless; the TS positions are not.)
  - [ ] `src/components/settings/ShortcutsContent.tsx`: `ShortcutsContentProps` (#6–#9), destructuring, and the
        "Auto-Paste" + "Auto-Capitalize" rows in "Paste & Behavior". **Keep** the "Auto-Send"
        (`insertAndSendSlot1`) row. The `!localAutoPaste` dimming/disable on "Auto-Send" and "Paste Delay" loses
        its source → **Q4**.
  - [ ] `src/components/AdvancedSettingsPanel.tsx`: `ADVANCED_DEFAULTS`; rows for `sttTemperature`,
        `llmTemperature`, `llmMaxTokens`, `chunkThreshold`, `chunkTargetSize`; the whole "Custom Cleanup
        Instructions" subsection (its only content is the four prompts); keep the four model inputs. User-facing
        strings that become false and the resulting layout → **Q2/Q3** (do not invent new copy).

- [ ] **Task 5 — Kotlin** (AC1, AC4, AC5)
  - [ ] `KlarvoApi.Config`: delete `bubbleTapAutoSend`, `bubbleLongPressAutoSend`; add the three model-override
        fields (default `""`). `readConfig` builds `Config(…)` **positionally** — keep the argument order consistent.
  - [ ] `KlarvoApi.readConfig`: delete the two `optBoolean` reads; add the model-ID parse via a new
        `internal fun` seam on `JSONObject` (pattern: `parseMinRecordingMs`).
  - [ ] `KlarvoOverlayService`: delete `tapAutoSend`/`longPressAutoSend`, their assignment + debug-log fields in
        `loadBubbleControls`, and the unreachable `shouldAutoSend` block after paste. If
        `KlarvoAccessibilityService.performEnter` loses its last caller, say so in the record; change no other
        paste behaviour.
  - [ ] `KlarvoApi.resolveLlmProvider` + `cleanupFallbackCandidates`: model = override-or-default for deepseek,
        openai, groq. Name the default literals as constants so the fixture test binds to a production symbol
        (7-8 precedent: `CLEANUP_TEMPERATURE`).
  - [ ] Tests (JUnit 4, `android/kotlin-test/com/klarvo/voice/`): extend `LlmFallbackProviderTest` — override
        flow-through on **both** sites, empty → default; a parse-seam test fed with an old `config.json` string
        carrying the removed keys (AC4 Kotlin half). Its existing literal model assertions
        (`"gpt-4o-mini"`, `"llama-3.3-70b-versatile"`, `"deepseek-chat"`) stay valid for the default path.

- [ ] **Task 6 — Parity fixture** (AC5)
  - [ ] `test-fixtures/twin-constants-vectors.json`: add one entry per default model ID with `PINS:` /
        `DOES NOT PIN:` (Anthropic: Desktop-asserted, Android column a written record).
  - [ ] Both "exactly five" assertions must follow: Rust
        `spec_twin_constants_fixture_is_complete_and_self_describing` (`vectors.len()`), Kotlin
        `TwinConstantsVectorsTest.fixtureCarriesAllFiveTwinsAndDescribesEach` (id set) — rename/re-word honestly.
  - [ ] **Claim accuracy (7-8's defect class):** the existing entries' `DOES NOT PIN` clauses and the
        `TwinConstantsVectorsTest` KDoc name dead keys that this story deletes ("the sttTemperature config key (dead
        config, out of scope)", "the dead advanced.llmMaxTokens config key", "the dead advanced.chunkThreshold
        config key", "the dead-config cluster … deliberately NOT locked"). Correct them — values unchanged.

- [ ] **Task 7 — Inversions** (AC6): the three required inversions + one per new discriminating assertion;
      table in the Dev Agent Record; `git status` clean.

- [ ] **Task 8 — Gates** (AC7): Rust, `tsc`, JVM (`--rerun-tasks`), puppeteer proxy smoke with evidence,
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

### Open questions (not decided by this file — Andi's call)

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

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created

### File List
