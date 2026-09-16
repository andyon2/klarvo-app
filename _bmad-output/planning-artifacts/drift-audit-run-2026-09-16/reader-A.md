# Cross-platform drift discovery — Reader A

## 0. Run header

- **Model:** Reader A
- **Tree:** `e9543fe` (branch `v1-ship`), repo `/home/andyon2/workspace/products/klarvo`, READ-ONLY.
- **Time spent:** ~2 h of reading (config spine → twin walk → degrade matrix → June mapping).
- **Path shorthand in tables:** `cfg` = `src-tauri/src/config/mod.rs`, `pipe` = `src-tauri/src/pipeline.rs`, `llm` = `src-tauri/src/llm/mod.rs`, `stt` = `src-tauri/src/stt/mod.rs`, `jni` = `src-tauri/src/stt/groq_jni.rs`, `KApi` = `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt`, `KOS` = `…/KlarvoOverlayService.kt`, `KAR` = `…/KlarvoAudioRecorder.kt`, `KAS` = `…/KlarvoAccessibilityService.kt`, `RAC` = `src/components/settings/RecordingAudioContent.tsx`, `AIP` = `src/components/settings/AiProvidersContent.tsx`, `SC` = `src/components/settings/ShortcutsContent.tsx`, `APP` = `src/components/settings/AppearanceContent.tsx`, `LC` = `src/components/settings/LanguageContent.tsx`, `ASP` = `src/components/AdvancedSettingsPanel.tsx`, `SP` = `src/components/SettingsPanel.tsx`.
- **Files actually read** (whole or the cited ranges):
  - Rust: `config/mod.rs` 1-1560; `pipeline.rs` 1-700, 1090-2560, 2900-2985; `llm/mod.rs` 30-740, 929-1020, 1280-1530; `llm/local.rs` (outline + 284-340); `stt/mod.rs` 100-600; `stt/groq_jni.rs` full; `stt/jni_bridge.rs` 170-215; `stt/local_whisper.rs` (grep 221-322); `stt/hallucination.rs` 175-240 + outline; `stt/model_manager.rs` (grep); `vad/mod.rs` 1-420; `audio/mod.rs` 1055-1175 + grep; `paste/mod.rs` 240-300 + outline; `sync/mod.rs` outline; `history/mod.rs` 218-335 + outline; `dictionary/mod.rs` 1-140; `license/mod.rs` 353-370 + outline; `commands/recording.rs` 90-300; `commands/history.rs` 160-260; `commands/misc.rs` 108-160; `commands/settings.rs`, `commands/whisper.rs`, `commands/llm_model.rs`, `commands/dictionary.rs`, `lib.rs` (grep only).
  - Kotlin: `KlarvoApi.kt` full; `KlarvoOverlayService.kt` 110-300, 835-860, 1152-1175, 1598-2440, 2600-2829; `KlarvoAudioRecorder.kt` 150-300, 520-625 + grep; `KlarvoAccessibilityService.kt` 175-230 + grep; `LicenseValidator.kt` full; `BankingGuard.kt` full; `BankingAppBlocklist.kt` 1-40; `ListeningPanelView.kt` 270-300.
  - React: `platform.ts`; `SettingsPanel.tsx` 40-120, 380-420, 468-700, 700-900; `AdvancedSettingsPanel.tsx` 1-120 + grep; `settings/SettingsHome.tsx`, `settings/types.ts`, `settings/LanguageContent.tsx`, `settings/RecordingAudioContent.tsx` full; `settings/AiProvidersContent.tsx` 1-80, 195-330; `settings/ShortcutsContent.tsx` 1-120, 200-620; `settings/AppearanceContent.tsx` 20-120 + grep; `App.tsx` 700-735, 890-910, 1050-1075 + grep; `hooks/useRecording.ts` 55-110; `tauri-commands.ts`, `media-recorder.ts`, `WhisperModelManager.tsx`, `LlmModelManager.tsx` (grep).
  - Fixtures/docs: `test-fixtures/README.md`, `test-fixtures/twin-constants-vectors.json` (ids); `docs/cross-platform-drift-audit.md` (full, read last, step 4). Not read: `docs/backlog.md`, ADR amendments.

---

## 1. Config contract table

Legend for "Kotlin reader": `parse-only` = field is in `KApi.Config` but no runtime use found. Line refs for Kotlin runtime USE, not the `readConfig` parse. "React render + gate" cites the control's render site and its platform gate (`none` = renders on both platforms; `not rendered` = no control anywhere in the current drill-down UI, so settable only by editing `config.json`).

### 1a. `AppConfig` top-level keys (camelCase)

| key | Rust reader | Kotlin reader | React render + gate | verdict |
|---|---|---|---|---|
| `groqApiKey` | `pipe:51-54` (Groq STT), `pipe:261` (Groq LLM), `cfg:1389-1400` (Groq-Llama rule) | `KOS:2074` (STT via JNI), `KApi:244-248` (Groq cleanup), `KApi:577` (config rejected if blank) | `AIP:79-96` none | ok |
| `deepseekApiKey` | `pipe:279`, `cfg:1392/1408` | `KApi:266-270`, `KApi:296`, gated `KApi:566` | `AIP:103-120` none | ok |
| `openaiApiKey` | `pipe:45` (OpenAI Whisper), `pipe:259`, `pipe:341` (ladder) | `KApi:250-254`, `KApi:302`, gated `KApi:567` | `AIP:127-144` none | ok |
| `anthropicApiKey` | `pipe:260` | **none** (no field in `KApi.Config` 143-227; comment `KApi:81-85`) | `AIP:151-168` none | android-visible-but-dead |
| `openrouterApiKey` | `pipe:262`, `pipe:342` | `KApi:256-260`, `KApi:308`, gated `KApi:568` | `AIP:175-192` none | ok |
| `sttProvider` | `pipe:43-56`, `pipe:1780` (`is_offline`) | `KOS:2007` (`== "local"` else Groq path), `KApi:571-577` (license gate) | `RAC:62-75` (Cloud/Offline) none; `RAC:83-95` sets `"openai"` when `whisper-1` chosen, none | unit/default-mismatch (`"openai"` → Groq on Android, see A7) |
| `llmProvider` | `pipe:257-282`, `pipe:1780`, `cfg:1373-1428` (validate + auto-fallback) | `KApi:241-273`, `KOS:2168` (`== "local"`), gated `KApi:569` | `RAC:141-152` none (no `anthropic` option) | ok (fallback ladder differs, A18) |
| `sttPriority` | none (deprecated; migration only `cfg:1288-1295`); `commands/recording.rs:374-414` tests write it | none | not rendered | dead-both (tombstone) |
| `llmPriority` | `commands/recording.rs:120-128` walks it for the in-app license gate (always empty post-migration → gate inert) | none | not rendered | dead-both (tombstone; see A1 note) |
| `language` | `pipe:1751-1773`, `pipe:1848`, `commands/history.rs:199-204` | `KOS:2076` (STT), `KOS:2275` (history), `KOS:1765` (preview) | `LC:58-64` none | ok |
| `cleanupStyle` | `pipe:1826/1831`, `pipe:2007-2016` (history style) | `KOS:2171/2183/2274` | `App.tsx:100` (`STYLE_OPTIONS`) none | ok (unknown value: serde reject vs silent polished, A26) |
| `hotkey` | `lib.rs:734-737` (migration fallback only) | none | not rendered (slots UI instead) | dead-android (desktop-only by nature) |
| `hotkeyMode` | `lib.rs:740` | none | not rendered | dead-android (desktop-only) |
| `hotkeySlots[]` | `pipe:2678`, `pipe:2720-2746` (register) | none | `SC:261-395` `isDesktop` | ok (desktop-only, gated) |
| `hotkeySlots[].hotkey` | `pipe:2720` | none | `SC:266-283`, `SC:303-330` `isDesktop` | ok |
| `hotkeySlots[].mode` | `pipe:2744-2746` | none | `SC:285-293` `isDesktop` | ok |
| `hotkeySlots[].insertAndSend` | `pipe:2744-2746` → `AppState.active_insert_and_send` → `pipe:1949-1958`, `pipe:2339-2346` (send_enter) | **none** (`KAS:202 performEnter` has zero call sites; `KOS:212-227 decideDelivery` pastes only) | `SC:567-573` "Auto-Send" **none** (renders on Android) | android-visible-but-dead |
| `audioDevice` | `pipe:701`, `pipe:1061`, `commands/recording.rs:45` | none (always `AudioSource.MIC`) | `RAC:164-176` `isDesktop` | ok (gated) |
| `sttModel` | `pipe:51-54` (`with_model`), `pipe:1906` (cost rate) | **none** — hardcoded `"whisper-large-v3-turbo"` `KOS:2075`, `KOS:1766` | `RAC:78-102` none | android-visible-but-dead |
| `customPrompt` | `pipe:1814-1832` (LLM system prompt) | `KOS:2077` **as STT hint** (→ `jni:177-179`), `KOS:2186` (LLM) | `AIP:199-249` none (disabled when `!isPaid`) | unit/default-mismatch (semantics differ, A3) |
| `profiles[]` (+ `.name`, `.appPattern`, `.cleanupStyle`, `.language`, `.customPrompt`) | `pipe:1805-1832` (`cleanupStyle`, `customPrompt`; **`.language` never applied**), `commands/misc.rs:21-40` | **none** | `AIP:252-330` none (paid) | android-visible-but-dead (`profiles[].language`: dead-both) |
| `autostart` | `lib.rs:759` | none | not rendered (`SP:722 void localAutostart`) | dead-android, not rendered |
| `whisperMode` | `pipe:1642-1644` (gain), `pipe:1673-1677` (threshold swap) | none | not rendered (`SP:723 void localWhisperMode`) | dead-android, not rendered |
| `commandHotkey` | `pipe:2686` | none | not rendered | dead-android (desktop-only) |
| `outputLanguage` | `pipe:1842-1847` → `llm:135-142` (translation section), `commands/recording.rs:281`, `commands/history.rs:220-223` | **none** (no field; `KApi.cleanup 1015-1078` has no translation) | `LC:68-74` "Translate to" **none** | android-visible-but-dead |
| `snippets[]` (`.name`, `.content`) | `commands/misc.rs:50-70` | none | not rendered (`SnippetsPanel` not mounted in `App.tsx`) | dead-both, not rendered |
| `voiceNotesHotkey` | **none** | none | not rendered | dead-both |
| `webhookUrl` | `pipe:2093-2120` (POST after dictation) | **none** | not rendered (`SP:724 void localWebhookUrl`) | dead-android, not rendered |
| `tursoUrl` | `pipe:2016`, `pipe:2039-2088`, `commands/misc.rs:123-146` | `KOS:2285-2287` → `KApi:732` | not rendered (`SP:725 void localTursoUrl`) | ok (config.json-only) |
| `tursoToken` | same as above | same | not rendered | ok (config.json-only) |
| `deviceId` | `pipe:2016/2028`, `commands/history.rs:96/307`, `commands/misc.rs:146` | `KOS:2276`, `KOS:2416` | n/a (internal) | ok (default `""` on Kotlin if absent, `KApi:450`; Rust generates UUID `cfg:872`) |
| `bubbleSize` | none | parse-only (`KOS:1147/2607` comments: no longer applied) | not rendered | dead-both |
| `bubbleOpacity` | none | parse-only (`KOS:2615` alpha forced 1.0) | not rendered | dead-both |
| `bubbleSizeDp` | none | `KOS:1157-1158` | `SC:498-522` `!isDesktop` | ok |
| `bubbleEdgeSnap` | none | `KOS:1097`, `KOS:1437` | `SC:529-533` `!isDesktop` | ok |
| `recordingButtonSizeDp` | none | `KOS:2613` (via `KApi:505-506` clamp) | `SC:542-560` `!isDesktop` | ok |
| `advanced.*` | see 1b | see 1b | `ASP` (embedded, no platform gate) | see 1b |
| `localWhisperModel` | `pipe:101` (`ggml-{model}.bin`), `pipe:137`, `commands/whisper.rs:437` | **none** — hardcoded `ggml-small.bin` `KOS:2660` | `RAC:114-121` `WhisperModelManager` **none** | android-visible-but-dead |
| `localWhisperGpu` | **none** outside `commands/settings.rs` round-trip (no reader in `stt/` or `commands/whisper.rs`) | none | `RAC:120` `showGpuToggle={isDesktop}` | desktop-visible-but-dead |
| `licenseKey` | `lib.rs:377`, `commands/license.rs` | `KApi:554-559` → `LicenseValidator.kt:73-92` (JNI) | `LicenseSettings.tsx` none (written via Rust commands on both) | ok |
| `licenseValidatedAt` | `lib.rs:381`, `commands/license.rs:69-128` | `KApi:554-559` | n/a | ok |
| `licenseSource` | `lib.rs:378` | `KApi:554-559` | n/a | ok |
| `lsInstanceId` | `lib.rs:379` | `KApi:554-559` | n/a | ok |
| `lsLastValidatedAt` | `lib.rs:380` | `KApi:554-559` | n/a | ok |
| `insertAndSend` (global) | migration tombstone `cfg:1348-1361`; runtime reads slot flag | none | not rendered | dead-both (tombstone) |
| `autostopSilenceSecs` | `pipe:838` | `KOS:843` → `KOS:280` (AUTOSTOP) | `SC:360-372` `isDesktop` | **not settable on Android although Android's AUTOSTOP reads it** (A13) |
| `autoModeSilenceSecs` | `pipe:902` | `KOS:844` → `KOS:279` (AUTO) | `SC:377-389` `isDesktop` | same as above (A13) |
| `livePreviewEnabled` | `pipe:2566`, `pipe:2404-2406` (guard incl. `stt_provider != "local"`), `native_preview.rs:310/1542` | `KOS:1666` → `KOS:294-295` (no provider check), `KOS:2453` | `APP:117-121` none | ok (guard differs, A14) |
| `previewPauseSilenceSecs` | `pipe:2567` | `KOS:1653` | `APP:135` none | ok |
| `previewPanelForm` | `native_preview.rs:252` | none | `APP:378` `!hidePanelForm` (desktop) | ok (gated) |
| `previewTextColor` | `native_preview.rs:260` | `ListeningPanelView.kt:283` | `APP` none | ok |
| `previewBgColor` | `native_preview.rs:258` | `ListeningPanelView.kt:276` | `APP` none | ok |
| `previewBgBlur` | **none** (`native_preview.rs` has no blur) | parse-only | `APP:250` `!hideBgBlur` (desktop) | desktop-visible-but-dead |
| `previewBorderColor` | `native_preview.rs:262` | `ListeningPanelView.kt:277` | `APP` none | ok |
| `previewBorderWidth` | `native_preview.rs:304` | `ListeningPanelView.kt:281` | `APP` none | ok |
| `previewBorderRadius` | `native_preview.rs:305` | `ListeningPanelView.kt:280` | `APP` none | ok |
| `previewFontFamily` | `native_preview.rs:266` | `ListeningPanelView.kt:287` | `APP` none | ok |
| `previewFontSize` | `native_preview.rs:227` | `ListeningPanelView.kt:288` | `APP` none | ok |
| `previewLineSpacing` | `native_preview.rs:247` | `ListeningPanelView.kt:289` | `APP` none | ok |
| `barX` / `barY` | `pipe:756/790`, `commands/misc.rs:174-236`, `native_pill.rs:1382` | none | n/a (internal) | ok (desktop-only) |
| `bubbleRecordingMode` | none | parse-only (`KApi:157` "no longer used") | not rendered | dead-both |
| `bubbleTapMode` | none | `KOS:839` | `SC:412-424` `!isDesktop` | ok |
| `bubbleTapSilenceSecs` | none | `KOS:841` → `KOS:281-284` **only for HOLD/TOGGLE**, which never install `onSilenceDetected` (`KOS:1656-1660`) | `SC:425-440` "Silence Duration" `!isDesktop`, shown only when tap mode is autostop/auto | android-visible-but-dead |
| `bubbleLongPressMode` | none | `KOS:840` | `SC:451-463` `!isDesktop` | ok |
| `bubbleLongPressSilenceSecs` | none | `KOS:842` → same dead branch as tap | `SC:464-479` `!isDesktop` | android-visible-but-dead |
| `onboarding.{completed,skipped,currentStep,mode,language,track}` | `commands/settings.rs:996-1000` (get/set) | none | onboarding wizard (both) | ok (React-only state) |
| `voiceCommandEnabled` | `lib.rs:889-902`, `commands/voice_command.rs` | none | not rendered (`SP:726 void localVoiceCommandEnabled`) | dead-android, not rendered |
| `firstInstallAt` | `lib.rs:382`, `lib.rs:724-730` (stamp) | `KApi:551` (fallback: package install time `KApi:547-551`) | n/a | ok |
| `feedbackWebhookUrl` | `commands/feedback.rs:347` | none | not rendered (operator field) | ok (desktop-only by design) |

### 1b. `advanced.*` (`AdvancedSettings`, camelCase)

| key | Rust reader | Kotlin reader | React render + gate | verdict |
|---|---|---|---|---|
| `advanced.sttPromptDe` | `pipe:1752-1753` | **none** | `ASP:297` none (paid) | android-visible-but-dead |
| `advanced.sttPromptEn` | `pipe:1755-1756` | **none** | `ASP:302` none (paid) | android-visible-but-dead |
| `advanced.sttPromptAuto` | `pipe:1758-1759` | **none** | `ASP:307` none (paid) | android-visible-but-dead |
| `advanced.llmModelDeepseek` | `pipe:243` via `llm:1306-1322` | `KApi:268`, `KApi:298` (via `effectiveCleanupModel` 131-134) | `ASP:323` none | ok |
| `advanced.llmModelOpenai` | `pipe:225` | `KApi:252`, `KApi:304` | `ASP:327` none | ok |
| `advanced.llmModelAnthropic` | `pipe:233` | **none** | `ASP:331` none | android-visible-but-dead |
| `advanced.llmModelGroq` | `pipe:229` | `KApi:246` | `ASP:335` none | ok |
| `advanced.silenceThreshold` | `pipe:1677` (pre-STT), `pipe:838/902/2569` (VAD `energy_floor`, `audio/mod.rs:1074`) | `KOS:845` → `KOS:1652` (VAD gate, clamped `KAR:274`), `KOS:1936` (pre-STT via `jni:408`) | `ASP:345` behind `expertMode` (`ASP:216`), none | ok (clamp differs, A37) |
| `advanced.whisperModeThreshold` | `pipe:1675` | none | `ASP:349` expertMode, none | dead-android (whisper mode unreachable, see A29) |
| `advanced.minRecordingMs` | `pipe:1681/1690` | `KOS:1933` via `KOS:150-151` (`jni:408`) | `ASP:353` expertMode, none | ok |
| `advanced.whisperModeGain` | `pipe:1644` | none | `ASP:357` expertMode, none | dead-android |
| `advanced.pasteDelayMs` | **none** (`paste/mod.rs:281` hardcoded 50 ms) | **none** | `SC:575-578` **none** (both platforms) | dead-both, visible-both |
| `advanced.webhookHeaders` | **none** | none | not rendered | dead-both |
| `advanced.webhookTimeoutSecs` | **none** (`pipe:2113` hardcoded 10 s) | none | not rendered | dead-both |
| `advanced.logLevel` | **none** | none (`KlarvoLogger.kt` reads no config) | `ASP:397` none | dead-both, visible-both |
| `advanced.uiScale` | none (React-only: `src/hooks/useUiScale.ts:32`) | none | `ASP:381` none | ok (frontend-consumed on both) |
| `advanced.expertMode` | none (UI flag by design, `cfg:120-124`) | none | `ASP:415` none | ok (UI-only) |

---

## 2. Divergences by severity

### CRITICAL

| # | Shared behavior | Desktop (file:line) | Android (file:line) | React gate | Note | June-ID/NEW | confidence |
|---|---|---|---|---|---|---|---|
| A1 | License gate on alternative STT/LLM providers in the **dictation pipeline** | Hotkey pipeline has **no** license check: `pipe:257-282` (`resolve_cleanup_provider`), `pipe:43-56` (`resolve_stt_provider`), `pipe:1281-1590` (`process_audio`) — the only `require_license!(AlternativeProviders)` sites are the in-app commands `commands/recording.rs:150/197/250`, and the LLM one walks the deprecated, always-empty `llm_priority` (`recording.rs:120-128`) so it never fires | `KApi:554-577`: unlicensed/expired → DeepSeek/OpenAI/OpenRouter keys blanked, `llmProvider` forced `"groq"`, `sttProvider` `"openai"` forced `"groq"` | `RAC:141-152` provider picker is **not** `isPaid`-gated | After trial expiry the same config keeps using OpenAI Whisper / DeepSeek / OpenAI / OpenRouter on Desktop, while Android silently drops to Groq-Llama cleanup and refuses OpenAI STT — the paywall exists on exactly one platform. | C1 (CHANGED: direction reversed since June) | high |
| A2 | Unlicensed user in **Offline** mode: where cleanup runs | `pipe:265-278` `"local"` → llama.cpp on Windows; `pipe:1780` `is_offline` false when LLM is local → local cleanup; no license check | `KApi:569` `gatedLlmProvider = if (licensed) … else "groq"` overrides `"local"` too → `KOS:2168` takes the cloud branch → `KApi:244` Groq cleanup if a Groq key exists (else raw + toast `KOS:2263`) | `SP:472-476` selecting Offline auto-sets `llmProvider="local"` (both) | An unlicensed Android user who chose "Offline" has every transcript sent to Groq cloud for cleanup; Desktop keeps it on-device. Privacy outcome differs with no error. | NEW (side-effect of C1 fix) | high |

### HIGH

| # | Shared behavior | Desktop (file:line) | Android (file:line) | React gate | Note | June-ID/NEW | confidence |
|---|---|---|---|---|---|---|---|
| A3 | STT conditioning prompt source | Hint = `advanced.sttPromptDe/En/Auto` (`pipe:1751-1766`); `customPrompt` used only in LLM prompt (`pipe:1814-1832`) | `KOS:2077` passes `config.customPrompt` as the `custom_prompt` arg → `jni:177-179` `build_stt_prompt_with_hint(dict, lang, custom)` → `stt:117-142` replaces the language hint with the LLM cleanup instruction | `AIP:199-249` Cleanup Instructions, none | With any cleanup instruction set (e.g. preset "Formal"), Android's Whisper is conditioned with "Always use formal language…" instead of the DE/EN dictation hint → different raw transcript (punctuation/casing), and that text becomes echo-guard content (see A5/A6). | Recall #5 (PERSISTS, now load-bearing), H3 (CHANGED) | high |
| A4 | `advanced.sttPrompt*` user-edited Whisper hints | `pipe:1751-1761` read and sent | never parsed (`KApi:143-227`), never sent | `ASP:297/302/307` none | Hints edited on Android have no effect; Desktop uses them. | H3 (CHANGED), NEW as UI-visible dead key | high |
| A5 | Prompt-echo guard input | `pipe:1394` `post_stt_skip(&raw, &stt_hint_text)` with hint text **only** (`pipe:1770-1773`) | `jni:213-215` `is_prompt_echo(&text, hint)` where `hint` = full prompt **including dictionary terms** (`jni:179`, `stt:134`) | Dictionary `DictionaryContent` none | A short dictation made mostly of dictionary words ("Klarvo Kubernetes") hits the ≥70 % overlap rule (`pipe:436-444`) → Android drops it ("No speech detected" `KOS:2126`); Desktop pastes it. | H6 (CHANGED) / NEW | high |
| A6 | Prompt-fragment stripping input | `pipe:1384` `strip_prompt_fragments(&raw, &stt_hint_text)` — fragments come from hint sentences only | `jni:216` same fn with the full prompt → `pipe:509-517` splits on `". "`/`"."` so the dictionary tail (`stt:134` `"{hint}{terms}"`) becomes a fragment when ≥10 chars | Dictionary none | A single dictionary term ≥10 chars (e.g. "Bundesverfassungsgericht"), or the exact comma-list for several terms, is case-insensitively deleted from **every** Android transcript before cleanup; Desktop keeps it. | H7 (CHANGED) / NEW | high |
| A7 | `sttProvider = "openai"` | `pipe:45` → OpenAI Whisper `whisper-1` with `openaiApiKey` | `KOS:2007` only `"local"` is special-cased → OpenAI selection runs the Groq path with `groqApiKey` (`KOS:2074`); `KApi:577` returns `null` (→ "No configuration found" toast `KOS:1615`) when the Groq key is blank | `RAC:83-95` choosing "OpenAI — Whisper 1" sets provider `openai`, none | User picks OpenAI Whisper on Android and silently gets Groq turbo, or cannot dictate at all with an OpenAI-only key. | H16 (PERSISTS) | high |
| A8 | `sttModel` | `pipe:51-54` `.with_model(cfg.stt_model)` | hardcoded `"whisper-large-v3-turbo"` `KOS:2075` (dictation) and `KOS:1766` (preview) | `RAC:78-102` Model picker, none | "Groq — Large V3" selected on Android still transcribes with turbo → different transcripts, and cost accounting model (`pipe:1906`) is desktop-only anyway. | H9 (PERSISTS) | high |
| A9 | `localWhisperModel` | `pipe:101` `ggml-{cfg.local_whisper_model}.bin`, default `tiny-german-1224-q8_0` (`cfg:876`) | hardcoded `ggml-small.bin` `KOS:2660` (`// TODO`) | `RAC:114-121` `WhisperModelManager` (5 models, default tiny-german) none | An Android user who downloads the default German model gets "Whisper model not downloaded" (`KOS:2019`); only `small` works; offline transcripts differ when both have a model. | H10 (PERSISTS) | high |
| A10 | `outputLanguage` translation | `pipe:1842-1847` → `llm:135-142` "Translate the cleaned output to {name}…" appended | no field, no translation section (`KApi:1015-1078`, `911-929`) | `LC:68-74` "Translate to", none | Same setting: Desktop output is translated, Android output stays in the source language. | H4 (PERSISTS) | high |
| A11 | App profiles (`profiles[]`) | `pipe:1805-1832` window-title match → style/custom prompt override | never read | `AIP:252-330` (paid), none | Profiles defined on Android are inert; the control still offers a "window title pattern". (`profiles[].language` is applied on **neither** platform — `pipe:1814-1832` ignores it.) | H15 (PERSISTS) | high |
| A12 | Auto-Send after paste (`hotkeySlots[0].insertAndSend`) | `pipe:1949-1958` → `pipe:2339-2346` `send_enter()` | `KAS:202 performEnter()` has **zero** call sites; `KOS:212-227` `decideDelivery` pastes only | `SC:567-573` "Auto-Send — Send Enter after pasting", **none** (renders on Android) | Toggle on Android persists but never sends Enter. | M13 (CHANGED: old bubble keys gone, global toggle now leaks to Android) | high |
| A13 | Silence window for Android AUTOSTOP/AUTO bubble modes | n/a (desktop reads `autostop_silence_secs` `pipe:838`, `auto_mode_silence_secs` `pipe:902`) | `KOS:269-284` AUTO→`autoModeSilenceSecs`, AUTOSTOP→`autostopSilenceSecs`; `bubbleTapSilenceSecs`/`bubbleLongPressSilenceSecs` reach only HOLD/TOGGLE, which never install `onSilenceDetected` (`KOS:1656-1660`) | `SC:425-440`/`464-479` "Silence Duration" slider (`!isDesktop`) writes `bubbleTapSilenceSecs`; the keys Android actually uses are behind `isDesktop` (`SC:360-389`) | Moving the Android slider changes nothing; the effective window is the desktop-only setting (default 2.0 s), unreachable from the phone. | NEW (post-`759087f`) | high |
| A14 | Live-preview flush vs Offline STT | `pipe:2404-2406` `live_preview_enabled && stt_provider != "local"` — no flush when offline | `KOS:294-295` `shouldInstallPreviewFlush(mode, livePreviewEnabled)` has no provider check; `KOS:1754-1770` flush → `transcribeWithRetry` → Groq cloud | `APP:117-121` Live Preview toggle, none | "Offline" + Live Preview on Android streams audio deltas to Groq (when a Groq key exists); Desktop never leaves the device. | Recall #4 (FIXED) → NEW follow-on | high (code) / med (key present) |

### MEDIUM

| # | Shared behavior | Desktop (file:line) | Android (file:line) | React gate | Note | June-ID/NEW | confidence |
|---|---|---|---|---|---|---|---|
| A15 | Local LLM cleanup prompt | `llm/local.rs:308/332` full `system_prompt(_with_translation)` incl. punctuation table, dictionary, custom prompt, translation | `KApi:871-899` `buildSystemPrompt` — 5-line prompt, **no** dictionary/custom/translation, no punctuation commands | `RAC:150` "Local (Offline)" / `SP:472-476`, none | Offline cleanup differs in content and ignores dictionary + instructions on Android. | H11 (PERSISTS) | high |
| A16 | Local LLM failure degrade | `pipe:1549-1565` any `LlmError` → `llm_error=true` → clipboard-only + cause card (`pipe:2319`, `2155`) | `KApi:876-879` load failure returns raw text **as success**; `KOS:2171-2178` exception → raw text, `llmCleanupFailed` stays false → normal paste, no toast | none | Local-model failure: Desktop withholds paste and names the cause; Android pastes filler-laden raw text silently. | NEW | high |
| A17 | Local STT prompt + guards | `pipe:1312` passes `dict_prompt` → `stt/local_whisper.rs:319-322` `initial_prompt`; then strip/echo/blocklist `pipe:1384-1404` | `stt/jni_bridge.rs:193` prompt `None`; `KOS:2007-2062` no strip/echo, only `nativeIsHallucination` `KOS:2150` | none | Offline: dictionary biasing and prompt guards exist on Desktop only. | H3/H6/H7 residue (local path) — NEW | high |
| A18 | LLM provider whose key is missing | `cfg:1405-1428` at load: candidates deepseek→openai→**groq→anthropic**→openrouter | `KApi:241-273` → `KApi:296-311` deepseek→openai→openrouter only → `null` → raw paste + toast `KOS:2261-2267` | none | e.g. `llmProvider=openai` with only a Groq key: Desktop cleans with Groq-Llama, Android pastes raw. | M7 (CHANGED) | med |
| A19 | STT returns empty/unparseable text | `stt:410-416` `SttError::ResponseFormat` → non-retryable (`pipe:313-316`) → immediate pending WAV + "✗ … Audio gesichert" (`pipe:1351-1370`) | `jni:230-234` maps it to `__ERROR_NETWORK` → `KOS:2729` 2 retries (2 s, 5 s) → same terminal | none | Same end state, but Android burns 3 Groq calls and ~7 s before reporting. | M15 (PERSISTS) | high |
| A20 | Paste target missing / no focused field | `paste/mod.rs:256-293` → `ClipboardOnly` → `pipe:2155` `done_with_clipboard_only` (pill/card says clipboard) | `KAS:179-185` `pasteIntoFocusedField` silently no-ops; `KOS:2333-2341` decides on `instance != null` only → DONE flash, no toast | none | Same failure: Desktop tells the user the text is in the clipboard; Android shows success and nothing appears. | NEW | high |
| A21 | Clipboard write failure on the degrade path | `pipe:2373-2384` `ClipboardWriteFailed` cause; `deliver_text` coerces error (`pipe:2323-2328`) | `KOS:2627-2631` `copyToClipboard` unguarded inside `handler.post` (`KOS:2322`) → uncaught exception (documented as missing twin `KOS:170-174`) | none | Desktop degrades gracefully; Android has no handling at all. | NEW | med |
| A22 | Pending-entry reprocess vs live path (Android) | `commands/history.rs:170-247` Rust on both: Rust STT provider (`cfg.stt_model`/`local_whisper_model`), Rust prompts incl. translation, no strip/echo/blocklist, no sanitize | Same Rust code runs inside `TauriActivity` — i.e. a retried Android entry is processed by the **Desktop** pipeline, not the Kotlin one (`KOS:1914-2430`) | pending card `App.tsx:725+` both | Same audio yields different text on Android depending on whether it was live or retried (translation applied, guards skipped). | NEW | med |
| A23 | Turso auto-push scope/bootstrap | `pipe:2064` `push_single_entry` (current entry only, no `CREATE TABLE` `sync/mod.rs:395-440`); catch-up only via `sync_history` (`commands/misc.rs:118`), which has **no UI caller** (`tauri-commands.ts:654` unused) and no pull on Android | `KApi:732-830` pushes **all** unsynced rows + `ensureRemoteTable` | not rendered | Fresh Turso DB: Desktop pushes fail until Android creates the table; failed Desktop pushes never retry; Android never pulls. | NEW | med |
| A24 | Stockphrase-ghost strip position | post-LLM only `pipe:1574` | pre-LLM only `jni:218`; LLM output not re-stripped | none | A ghost that survives/gets rationalised by the LLM is caught on Desktop, not on Android; a ghost in raw text is removed pre-LLM on Android, not on Desktop. | NEW | med |
| A25 | VAD speech/silence decision | `vad/mod.rs:60-64, 323-370` Silero prob with dual thresholds 0.5/0.35 hysteresis | `KAR:408-412` `VadSilero(Mode.NORMAL)` boolean `isSpeech`; `KAR:520-575` manual counters | none | Borderline-SNR trailing speech ends the window earlier on Android. | M5 (PERSISTS) | med |
| A26 | Unknown `cleanupStyle` value | serde rejects enum → **whole config reset to defaults + backup** (`cfg:1444-1450`) | `KApi:447` string passthrough → `KApi:1015` `else ->` polished | none | Desktop loses all settings (recoverable file), Android silently uses Polished. | M11 (PERSISTS) | high |
| A27 | Anthropic cleanup provider | `pipe:233/260`, `llm:1020-1290` | none (`KApi:81-85`) | key `AIP:151-168` + model `ASP:331` render on Android; provider **not** selectable in `RAC:144-150` on either platform | Only reachable via legacy `config.json`; the two input fields on Android are inert. | H5 (PERSISTS) | high |
| A28 | Banking-app foreground guard | none | `KOS:2311-2320` blocks clipboard + paste, toast; history already saved `KOS:2271` | none | Android-only guard; Desktop pastes into anything. | NEW (class 3/5) | high |

### LOW

| # | Shared behavior | Desktop (file:line) | Android (file:line) | React gate | Note | June-ID/NEW | confidence |
|---|---|---|---|---|---|---|---|
| A29 | Whisper mode (gain + lower threshold) | `pipe:1642-1644`, `1673-1677` | none | toggle **not rendered** (`SP:723`); thresholds `ASP:349/357` under expertMode on both | Only reachable via `config.json`; the two expert fields are dead on Android. | C2 (PERSISTS) | high |
| A30 | `advanced.pasteDelayMs` | none (`paste/mod.rs:281` fixed 50 ms) | none | `SC:575-578` none | Field renders on both, read by neither. | M16 (PERSISTS) | high |
| A31 | `advanced.logLevel` | none | none | `ASP:397` none | Dead on both, visible on both. | NEW | high |
| A32 | `previewBgBlur` | none (`native_preview.rs` has no blur) | parse-only | `APP:250` desktop | Cosmetic; slider visible on Desktop only, dead there too. | NEW | high |
| A33 | Webhook after dictation | `pipe:2093-2120` | none | not rendered | `config.json`-only feature; Android dictations never reach it. | M14 (PERSISTS) | high |
| A34 | Usage/cost recording | `pipe:1898-1935` `record_usage` | none | Statistics panel `App.tsx:905` both | Android stats show only in-app-button dictations. | NEW | high |
| A35 | Empty-transcript / hallucination / echo UX | silent idle `pipe:1399/1404`, `1692-1701` | toasts "No speech detected" `KOS:2126`, "Speech not recognized" `KOS:2153`, "Recording too short" `KOS:1946` | none | Same skip outcome, different (or no) message. | L6 (PERSISTS) | high |
| A36 | Groq-Llama auto-select blank check | `cfg:1392` `is_empty()` | `KApi:529` `isBlank()` | none | Whitespace-only DeepSeek key flips differently. | M10 (PERSISTS) | high |
| A37 | Energy-gate threshold clamp | `audio/mod.rs:1074` unclamped | `KAR:274` `coerceIn(0.001f, 0.1f)` | `ASP:345` | Only outside the slider range. | NEW | high |
| A38 | Empty LLM `content` | `llm:643-647` error → raw clipboard-only | `KApi:1119-1125` `""` → history `""` + paste `""` (`KOS:2271/2322`) | none | Rare provider quirk. | NEW | med |
| A39 | `finish_reason == "length"` | `llm:638-640` `OutputTruncated` → raw | no check → truncated text pasted | none | Practically unreachable with 350-byte chunks. | NEW | high |
| A40 | Trailing whitespace of LLM output | never trimmed (`llm:645-649`; `hallucination.rs:186-188` fast path) | `KApi:1124` `.trim()` | none | A trailing newline from the LLM is pasted on Desktop only. | NEW | med |
| A41 | `is_trivial_chunk` predicate | `llm:1343-1345` `char::is_alphanumeric` (incl. No/Nl) | `KApi:1143` `isLetterOrDigit` (L*/Nd only) | none | "²", "Ⅳ" count as content on Desktop only. | NEW | med |
| A42 | HTTP success predicate (cleanup) | `llm:617` `is_success()` (2xx) | `KApi:1109` `!= 200` | none | Wire-level. | NEW | high |
| A43 | Turso `created_at` wire format | `pipe:2050-2053` `%Y-%m-%dT%H:%M:%S` on auto-push (batch push uses SQLite text) | `KApi:786` SQLite `YYYY-MM-DD HH:MM:SS` | not rendered | Mixed timestamp formats in one remote table. | NEW | high |
| A44 | Live-preview chunk conditioning | `pipe:2512` `transcribe(…, None)`, no guards | `KOS:1760-1775` full prompt (incl. `customPrompt`) + JNI guards + `nativeIsHallucination` | `APP:117` | Display-only preview text differs. | NEW | high |
| A45 | `deviceId` default when key absent | `cfg:872` UUID | `KApi:450` `""` | n/a | Only if `config.json` predates the key. | L5 (PERSISTS) | high |
| A46 | `voiceCommandEnabled` | `lib.rs:889-902` | none | not rendered | `config.json`-only. | L7 (PERSISTS) | high |
| A47 | Dictionary-term presence predicate in LLM prompt | `llm:129-133` `!terms.is_empty()` | `KApi:917` `isNullOrBlank()` | none | Whitespace-only list unreachable via `add_term` (`dictionary/mod.rs:48-52`). | NEW (7-6 residual) | high |

---

## 3. Degrade-path matrix

| failure | Desktop outcome (file:line) | Android outcome (file:line) | same? |
|---|---|---|---|
| STT 429/5xx/transport | 1 attempt → local Whisper if model present (`pipe:1316-1333`, warn "⚠ Groq am Limit → lokale Transkription") → else pending WAV + "✗ Transkription fehlgeschlagen — Audio gesichert" + `pending` history row (`pipe:1334-1349`, `2205-2223`) | 3 attempts (2 s/5 s, `KOS:2729-2789`) → local Whisper if model + native lib (`KOS:2080-2108`, toast "⚠ Groq am Limit → lokale Transkription" deferred) → else pending WAV + same toast + `pending` row (`KOS:2393-2430`) | same end state; extra ~7 s + 2 calls on Android |
| STT 4xx (bad key) | pending WAV + "✗ … Audio gesichert" (`pipe:1351-1370`) | `KOS:2091-2093` rethrow → outer catch → pending WAV kept → same toast + `pending` row | same |
| STT empty / unparseable body | non-retryable → pending immediately (`stt:410-416`, `pipe:1358-1370`) | treated as network error → retried twice → pending (`jni:230-234`, `KOS:2729`) | differs (latency, 3× calls) |
| Empty transcript after strip | `is_hallucination("")` true → silent idle (`pipe:1401-1404`) | `KOS:2124-2133` toast "No speech detected" → idle | outcome same, UX differs |
| Prompt-echo hit | silent idle (`pipe:1395-1400`) | JNI returns `""` (`jni:214`) → "No speech detected" (`KOS:2126`); dictionary words count as prompt (A5) | outcome same at default; differs with dictionary |
| Hallucination blocklist hit | silent idle (`pipe:1401-1404`) | toast "Speech not recognized" (`KOS:2150-2160`) | outcome same, UX differs |
| Too short / silent (pre-STT) | silent idle (`pipe:1692-1701`) | toasts "Recording too short" / "No speech detected" (`KOS:1946/1957`) | outcome same, UX differs |
| Cleanup 429/5xx/transport, fallback succeeds | pasted normally, no user-visible notice (`pipe:1502-1514`) | pasted + LENGTH_LONG toast "⚠ Cleanup-Anbieter gewechselt" (`KOS:2241`, `2355`) | outcome same, notice differs |
| Cleanup retryable, fallback also fails / none available | raw → **clipboard-only**, no paste, no Enter; card names cause (`pipe:1515-1548`, `2319`, `2155`) | raw → clipboard-only, no paste; toast `CLEANUP_FAILED_CLIPBOARD_MSG` (`KOS:2247-2255`, `2333-2350`) | same |
| Cleanup non-retryable (400/401/model-not-found) | same clipboard-only + `ModelNotFound`/generic cause (`pipe:1549-1565`, `1214-1276`) | `KOS:2221-2225` no fallback → same clipboard-only path; generic message only | same (message detail differs) |
| Local LLM failure | clipboard-only + cause (`pipe:1549-1565`) | raw **pasted normally**, no notice (`KApi:876-879`, `KOS:2171-2178`) | differs (A16) |
| No LLM key at all | Only reachable with `stt=local`: `OfflineRaw` → raw pasted, no notice (`pipe:1426-1431`) | raw pasted + toast "Text pasted without cleanup" (`KOS:2261-2267`) | outcome same, UX differs |
| Clipboard write fails | `paste_failed` → `ClipboardWriteFailed{in_history}` cause (`pipe:2373-2384`) | unguarded `setPrimaryClip` on main thread (`KOS:2627-2631`) → uncaught | differs (A21) |
| No target / focus lost | clipboard-only + "In Clipboard" event (`paste/mod.rs:256-293`, `pipe:2155`) | silent no-op paste, DONE flash (`KAS:179-185`, `KOS:2333-2341`) | differs (A20) |
| Accessibility service off | n/a | clipboard + toast "Copied: …" (`KOS:2333-2344`) | n/a |
| Banking app in foreground | no guard, pastes | blocked, toast "Paste blocked — banking app active." (`KOS:2311-2320`); history already written | differs (A28) |
| Recording produced 0 bytes | `stop_recording_with_gain` error event (`pipe:1646-1652`) | toast "No audio recorded" (`KOS:1916-1926`) | outcome same, UX differs |
| History write fails | logged, `history_saved=false` feeds the degrade card (`pipe:2024-2034`) | swallowed (`KApi:640-644`) | same (best-effort) |
| Turso push fails | entry stays `synced=0`; no automatic retry (`pipe:2076-2082`) | stays `synced=0`; retried on next dictation (`KApi:732-810`) | differs (A23) |

---

## 4. June rows — status today

| June ID | status | evidence |
|---|---|---|
| C1 | CHANGED (direction reversed) | Android enforces via JNI `KApi:554-577`, `LicenseValidator.kt:73-92`; Desktop hotkey pipeline has no gate (`pipe:257-282`, `1281-1590`), only in-app commands `commands/recording.rs:150/197/250` (LLM gate inert: `recording.rs:120-128` walks empty `llm_priority`) → A1/A2 |
| C2 | PERSISTS | `pipe:1642-1644`, `1673-1677` vs no Kotlin field; toggle no longer rendered (`SP:723`) → A29 |
| H1 | FIXED | `KOS:845` → `KOS:1652` energy gate from `advanced.silenceThreshold`; `KAR:274` clamp |
| H2 | FIXED | `KApi:1142-1150` byte offsets, `KApi:1261` `encodeToByteArray().size >= 400` |
| H3 | CHANGED | prompt now sent via `jni:177-179`, but hint = `customPrompt` not `sttPrompt*` (`KOS:2077`) → A3/A4 |
| H4 | PERSISTS | `LC:68-74` renders on Android; no Kotlin field/translation (`KApi:1015-1078`) → A10 |
| H5 | PERSISTS | `KApi:81-85`; key/model fields render on Android (`AIP:151`, `ASP:331`) → A27 |
| H6 | CHANGED | `jni:213-215` applies `is_prompt_echo`, but with dictionary-laden hint → A5 |
| H7 | CHANGED | `jni:216` applies `strip_prompt_fragments`, but with dictionary-laden hint → A6 |
| H8 | FIXED (UI-gated) | `RAC:164-176` Microphone behind `isDesktop`; Kotlin still `AudioSource.MIC` (inherent) |
| H9 | PERSISTS | `KOS:2075`, `KOS:1766` hardcoded; picker `RAC:78-102` ungated → A8 |
| H10 | PERSISTS | `KOS:2660` `ggml-small.bin` TODO; `RAC:114-121` ungated → A9 |
| H11 | PERSISTS | `KApi:896-899` short prompts vs `llm/local.rs:308/332` → A15 |
| H12 | FIXED | `KOS:2221-2246` `resolveFallbackLlmProvider` on retryable; `KApi:329-333` |
| H13 | FIXED | `KApi:1288-1294` single `'\n'` |
| H14 | FIXED | shared Rust via `jni:248-262`; `stt/hallucination.rs:83` `STOCKPHRASE_WHOLE_WORD_ENTRIES` + `test_h14_*` |
| H15 | PERSISTS | `AIP:252-330` renders on Android; no Kotlin read → A11 |
| H16 | PERSISTS | `KOS:2007`, `KApi:577` → A7 |
| H17 | FIXED | `KAR:176-177` `HANGOVER_FLOOR_MS = 200`, `KAR:155-156` floor |
| M1 | CHANGED | config-driven now (`KOS:1935-1938`); whisper-mode swap still absent (whisper mode unreachable) |
| M2 | FIXED | `KApi:434-435` `parseMinRecordingMs`, `KOS:1933` |
| M3 | FIXED | `KAR:184`, `KAR:327`, `KAR:215-224` |
| M4 | FIXED | `KAR:197` divisor 32767, float RMS `KAR:114` |
| M5 | PERSISTS | `KAR:408-412` boolean `VadSilero` vs `vad/mod.rs:60-64` → A25 |
| M6 | FIXED | shared `compute_wav_rms` via `jni:408-425` |
| M7 | CHANGED | runtime ladder aligned (`pipe:340-343` ↔ `KApi:296-311`); config-load fallback still includes groq/anthropic on Desktop (`cfg:1417-1423`) → A18 |
| M8 | FIXED | `KApi:1273-1281` abort on first error |
| M9 | FIXED | `KApi:55` `DEEPSEEK_CHAT_URL` with `/v1` |
| M10 | PERSISTS | `cfg:1392` vs `KApi:529` → A36 |
| M11 | PERSISTS | `cfg:1444-1450` vs `KApi:447/1015` → A26 |
| M12 | FIXED | `llm:238` Chat arm carries `{dict_section}`; fixture `m12-dictionary-scope-vectors.json` |
| M13 | CHANGED | `bubbleTapAutoSend`/`bubbleLongPressAutoSend` keys removed from `AppConfig`; global Auto-Send toggle now renders on Android (`SC:567-573`) and is dead there → A12 |
| M14 | PERSISTS | `pipe:2093-2120` vs no Kotlin read; URL no longer rendered (`SP:724`) → A33 |
| M15 | PERSISTS | `KOS:2729-2789` retries vs `pipe:1311-1370` single attempt → A19 |
| M16 | PERSISTS | `paste/mod.rs:281` fixed 50 ms; no Kotlin delay; renders on both `SC:575-578` → A30 |
| L1 | FIXED | `KAR:169-170` `31.25` |
| L2 | PERSISTS | `KAR:748` `smoothedAmplitude` (cosmetic, out of scope) |
| L3 | FIXED | `KOS:2761` `temperature = 0.0f` → `jni:161` → `stt:334` |
| L4 | FIXED | `KApi:1261` `>= 400` ≡ Rust `< 400` single-call (`llm:1460`) |
| L5 | PERSISTS | `KApi:450` `""` vs `cfg:872` → A45 |
| L6 | PERSISTS | `KOS:2124-2133` toast vs `pipe:1401-1404` idle → A35 |
| L7 | PERSISTS | `lib.rs:889-902` vs no Kotlin read; toggle not rendered (`SP:726`) → A46 |
| dead: `advanced.llmTemperature` | FIXED (removed) | key absent from `cfg:32-125`; twin constants `llm:487`, `KApi:69` pinned by `TWIN-LLM-TEMPERATURE-001` |
| dead: `advanced.llmMaxTokens` | FIXED (removed) | key absent; `llm:488`, `KApi:70`, `TWIN-LLM-MAX-TOKENS-001` |
| dead: `chunkThreshold` | FIXED (removed) | key absent; `llm:1332`, `KApi:1130`, `TWIN-CHUNK-THRESHOLD-001` |
| dead: `chunkTargetSize` | FIXED (removed) | key absent; `llm:1336`, `KApi:1131`, `TWIN-CHUNK-TARGET-SIZE-001` |
| dead: `advanced.sttTemperature` | FIXED (removed) | key absent from `cfg:32-125` |
| dead: `advanced.llmModel{Deepseek,Openai,Anthropic,Groq}` | FIXED (wired; Anthropic Desktop-only) | `pipe:217-246`, `KApi:246/252/268/298/304` |
| dead: `advanced.llmSystemPrompt*`, `llmCommandModePrompt` | FIXED (removed) | keys absent from `cfg:32-125` |
| dead: `autoCapitalize`, `autoPaste` | FIXED (removed) | keys absent from `cfg:380-801` |
| recall #1 (Android 0.02 vs 0.005 desync) | FIXED | both gates read `advanced.silenceThreshold` (`KOS:845`, `KOS:1936`) |
| recall #2 (`anthropicApiKey` absent from Config) | PERSISTS | `KApi:143-227` |
| recall #3 (`webhookHeaders` unread on Android) | CHANGED | now unread on **both** sides and not rendered (no Rust reader; `pipe:2093-2120` sends no headers) |
| recall #4 (live-preview delta) | FIXED | `KOS:1754-1799` `flushPreviewDelta`; `KAR:627` `deltaSnapshotWav` (new guard gap → A14) |
| recall #5 (`customPrompt` vs `sttPrompt*` conflation) | PERSISTS (now load-bearing) | `KOS:2077` → `jni:177-179` → A3 |

---

## 5. Things I suspected but could not confirm

- **Kotlin `VadSilero(Mode.NORMAL)` threshold value.** I did not open the third-party library; A25 rests on the Rust side using 0.5/0.35 hysteresis and Kotlin using a single boolean. The `VadGateGoldenVectorsTest` fixture is Kotlin-only per `test-fixtures/README.md`, so nothing pins the cross-platform decision.
- **Hangover frame counting off-by-one.** Rust enters `Hangover{frames_left: N}` on the first non-speech frame and reaches Silence when `frames_left <= 1` (`vad/mod.rs:352-368`); Kotlin counts `silentFrames >= requiredSilentFrames` (`KAR:589-590`). I believe both fire after exactly N non-speech frames but did not trace a vector.
- **`isRetryableCleanupFailure` regex on error bodies.** `KOS:2703-2706` extracts `HTTP (\d{3})` from the whole message; a 400 whose *body* contains "HTTP 500" text would be misclassified as retryable. Theoretical.
- **In-app record button on Android** (`useRecording.ts:66-75`: `transcribe_audio_bytes` + `cleanup_text`, both Rust) is a third pipeline on Android with Rust prompts/translation and no guards/sanitize — within-Android inconsistency mirroring A22; cross-platform it matches Desktop's in-app button, so not tabled.
- **`resolve_cleanup_provider` `"local"` arm is `cfg(windows)`** (`pipe:264-278`): on Android the Rust side (used by A22's reprocess and the in-app button) resolves `llmProvider=local` to DeepSeek. Consequence for Android offline users on those two paths not verified end-to-end.
- **Onboarding wizard writes** (`onboarding.mode = "offline"`): did not check whether the wizard sets `sttProvider`/`llmProvider` differently per platform.
- **`FloatingBubbleView.TAP_BUTTON_SIZE_MIN/MAX`** clamp on `recordingButtonSizeDp` (`KApi:505-506`) vs the React set `{52,60,72,84,96}` (`SC:547`) — did not open `FloatingBubbleView.kt` to confirm the range includes 52 and 96.
- **Desktop `UnlimitedHistory` free-tier limit** (`commands/history.rs:31`) applies to *reading*; Android `saveToHistory` is unbounded but the shared React reads through the same Rust command — believed identical, not traced.
- **Kotlin `HttpURLConnection` error body on 429 for STT** — whether Groq's 429 body parses to a status the retry wrapper honours; the code path (`KOS:2748-2768`) reads status from the JNI message, so a missing `HTTP <status>` prefix would fall to the retry branch. Low.
