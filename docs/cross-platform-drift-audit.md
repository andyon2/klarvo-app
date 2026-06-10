# Cross-Platform Drift Audit — Desktop (Rust) ↔ Android (Kotlin)

**Date:** 2026-06-10
**Method:** A/B drift-discovery run (Claude Fable 5 vs Claude Opus 4.8) on the same frozen `v1-ship` tree, identical prompt. Both lists then verified claim-by-claim against the real code by an independent pass (4 parallel readers, one per subsystem), plus a recall sweep for divergences neither model reported.
**Scope (per ADR-0016):** Config-Key-Contract + shared runtime behavior — NOT feature parity. A divergence = same config + same input → different behavior across platforms, with no error.

Paths: Desktop = `src-tauri/src/...`; Android = `android/kotlin-src/com/klarvo/voice/...` (`KApi` = `KlarvoApi.kt`, `KOS` = `KlarvoOverlayService.kt`, `KAR` = `KlarvoAudioRecorder.kt`).

**This file is the input for the Golden-Vector parity net** (the structural drift-detection net, see [[cross-platform-parity-net]]). Every HIGH/CRITICAL row below is a golden-vector candidate.

---

## CRITICAL

| # | Shared behavior | Desktop | Android | Note |
|---|---|---|---|---|
| C1 | License / trial enforcement | HMAC signature validation + 14-day trial via `firstInstallAt` + LemonSqueezy revalidation; feature-gates (`commands/license.rs:49-93`, `license/mod.rs`, gate at `pipeline.rs:751`) | `isLicensed()` **exists but has ZERO call sites** (`KApi:158-164`); `firstInstallAt` never read; no signature check → **any non-blank `licenseKey` passes** | Android licensing is effectively unenforced. Launch-blocker. Desktop gates (commit `c745bec`) have no Android counterpart. |
| C2 | Whisper mode (amplify quiet speech) | reads `whisperMode` + `whisperModeGain`(3.0) + `whisperModeThreshold`(0.001); applies gain (`pipeline.rs:1257-1300`, `audio/mod.rs:431/454`) | none of the 3 keys read; no gain; no Config field | Complete no-op on Android. |

## HIGH

| # | Shared behavior | Desktop | Android | Note |
|---|---|---|---|---|
| H1 | Live auto-stop energy gate | VAD `energy_floor` derived from config `silenceThreshold` (default 0.005) (`pipeline.rs:636-651`, `audio/mod.rs:1072-1076`) | hardcoded `0.02f`, config never read (`KAR:58`, applied `:257-258`) | Android gate is **4× stricter** at defaults → can auto-stop mid-sentence; Expert-mode tuning only affects Desktop. |
| H2 | Chunking length unit | `raw_text.len()` = UTF-8 **bytes**; split indices in bytes (`llm/mod.rs:1244-1306`) | `text.length` = UTF-16 **code units** (`KApi:816-884`) | German umlauts cross the 400 threshold earlier on Desktop → different chunk splits → different LLM output. Hits the core use case. |
| H3 | STT prompt conditioning (whole surface) | builds + sends multipart `prompt`: language hint `sttPromptDe/En/Auto`, dictionary terms, `customPrompt` (`stt/mod.rs:226-256`, `pipeline.rs:1373-1398`) | `buildMultipartBody` sends model/response_format/language/file only — **no `prompt` field at all**; dictionary + customPrompt routed to LLM cleanup only (`KApi:917-953`) | Android STT gets zero conditioning → worse punctuation/caps, no term biasing. |
| H4 | `outputLanguage` translation | appends "Translate the cleaned output to {name}…" (`llm/mod.rs:140-148`, `pipeline.rs:1473-1490`) | `outputLanguage` never read; no translation block | Output-language setting: Desktop translates, Android stays in source language. |
| H5 | Anthropic LLM provider | fully supported (`pipeline.rs:145-169`, `llm/mod.rs:989-1227`) | no anthropic branch; `anthropicApiKey` never read; `llmProvider=anthropic` → silent DeepSeek fallback (`KApi:93-147`) | Also: `anthropicApiKey` field is **absent from the Android Config struct entirely**. |
| H6 | Prompt-echo guard | `is_prompt_echo()` exact-fragment + 70% overlap heuristic (`pipeline.rs:234-314`) | absent (0 call sites) | Whisper echoing its prompt is filtered on Desktop, pasted on Android. |
| H7 | Prompt-fragment stripping | `strip_prompt_fragments()` removes leaked hint sentences (`pipeline.rs:325-393`) | absent | Leaked prompt fragments reach output on Android. |
| H8 | Input device selection | `audioDevice` + 3-tier fallback (`audio/mod.rs:697-712`, `pipeline.rs:555`) | never read; always `AudioSource.MIC` (`KAR:141`) | User-selected mic ignored on Android. |
| H9 | STT model | `cfg.sttModel` via `.with_model()` (`pipeline.rs:49-53`), default turbo | hardcoded `whisper-large-v3-turbo` (`KApi:924`) | Non-turbo choice ignored on Android. |
| H10 | Local Whisper model | `ggml-{cfg.localWhisperModel}.bin`, default `ggml-tiny-german-1224-q8_0.bin` (`pipeline.rs:84-99`) | hardcoded `ggml-small.bin` with `// TODO: read model name from config` (`KOS:1017`) | Different model → different transcripts offline. |
| H11 | Local LLM cleanup prompt | full `CleanupStyle::system_prompt` incl. punctuation table/dictionary/custom/translation (`llm/local.rs:290/314`) | abbreviated `buildSystemPrompt`, none of the above (`KApi:558,570-574`) | Offline cleanup much weaker on Android. |
| H12 | Runtime LLM fallback on 429/5xx | retryable error → switch provider, then raw (`pipeline.rs:1123-1177`) | no provider fallback; any IOException → raw transcript (`KApi:902-907`) | Under outage Desktop still cleans, Android pastes raw. |
| H13 | Chunk join separator | chunks joined with `\n` (`llm/mod.rs:1334`) | joined with `\n\n` (`KApi:909`) | >400-char dictation: Android inserts blank lines. |
| H14 | Hallucination single-word match | `lower.contains(phrase)` substring for all entries incl. `ard/zdf/wdr` (`stt/hallucination.rs:160-164`) | single-word entries require whole-word match — **deliberate ROB-03 fix, NOT back-ported to Rust** (`HallucinationFilter.kt:100-109`) | Desktop false-positive-discards real words ("Standard","Milliarde") in ≤8-word utterances. Android is the correct side. |
| H15 | Per-app profiles | `profiles` matched on window title → style/prompt override (`pipeline.rs:1437-1463`) | `profiles` never read; no Config field | Per-app overrides apply on Desktop only. |
| H16 | `sttProvider=openai` | routes to OpenAI Whisper (`pipeline.rs:42-53`) | cloud path Groq-only; non-local provider posts to Groq; config rejected (null) if `groqApiKey` blank (`KApi:274,506-508`) | OpenAI STT silently hits Groq or refuses to run. |
| H17 | Silence→stop 200ms floor | `hangover_ms.max(200)` (`audio/mod.rs:1074`) | `(silenceSecs*31).toInt().coerceAtLeast(1)` → floor of 1 frame (~32ms) (`KAR:77-78`) | Android can fire after a single silent frame; Desktop never under 200ms. |

## MEDIUM

| # | Shared behavior | Desktop | Android | Note |
|---|---|---|---|---|
| M1 | Pre-STT `silenceThreshold` | config-driven 0.005, swaps to `whisperModeThreshold` (`pipeline.rs:1296-1312`) | hardcoded `0.005f`, no whisper swap (`SilencePreFilter.kt:27`) | Identical at default; diverges when tuned / whisper on. |
| M2 | Pre-STT `minRecordingMs` | config-driven 500 (`pipeline.rs:1303-1312`) | hardcoded `500L` (`SilencePreFilter.kt:26`) | Identical at default; tuning ignored. |
| M3 | 85 Hz highpass before VAD | applied every sample (`vad/mod.rs:73,267-271`) | none; raw frame to VAD | Bass bleed mis-classified as speech on Android. |
| M4 | RMS scale/precision | f32 on normalized [-1,1] samples (`audio/mod.rs:1314-1321`) | Double on raw i16, normalized after `/32768f` (`KAR:369-375,257`) | Numeric thresholds not directly comparable; compounds H1. |
| M5 | VAD hysteresis | 4-state dual prob thresholds 0.5/0.35 (`vad/mod.rs:82-90,323-374`) | boolean `vad.isSpeech()` Mode.NORMAL + manual counters (`KAR:262-287`) | Different mechanism; diverges at borderline SNR. |
| M6 | Pre-STT WAV format | handles Int + Float (`pipeline.rs:413-438`) | `audioFormat != 1` → null, PCM16 only (`SilencePreFilter.kt:59-95`) | Latent (both self-encode PCM16). |
| M7 | LLM fallback provider order | deepseek→openai→groq→anthropic→openrouter (`config/mod.rs:1476-1493`) | deepseek→groq→openai→openrouter, no anthropic (`KApi:121-142`) | With multiple keys → different fallback pick + different output text. |
| M8 | Chunk-failure handling | first chunk error aborts via `?` (`llm/mod.rs:1331-1333`) | failed chunk → retry whole text as one call, then raw (`KApi:902-907`) | Different degradation under partial failure. |
| M9 | DeepSeek endpoint URL | `…/v1/chat/completions` (`llm/mod.rs:719`) | `…/chat/completions` (no `/v1`) (`KApi:112,124`) | Both work today; Android breaks if `/v1` enforced. |
| M10 | Groq auto-select blank check | `deepseek_api_key.is_empty()` (`config/mod.rs:1451-1454`) | `deepseekKey.isBlank()` (`KApi:259-264`) | Whitespace-only key flips the decision differently. |
| M11 | Unknown `cleanupStyle` value | serde rejects unknown enum at parse (`config/mod.rs:925`) | `else -> polished` silent fallback (`KApi:570/717`) | Desktop errors, Android silently Polished. |
| M12 | Dictionary in Chat style | Chat arm OMITS `{dict_section}` (`llm/mod.rs` ~232-263) | `appendPromptExtensions` adds dictionary for ALL styles incl. chat (`KApi:591-593,749`) | **Opposite direction** — here Android does more than Desktop. |
| M13 | Bubble auto-send | Desktop writes `bubbleTapAutoSend`/`bubbleLongPressAutoSend` to config | reads the keys (`KApi:232,235`) then hardcodes both `false` (`KOS:353-355`) | Setting round-trips + persists but has zero runtime effect on Android. (Surface-operable trap.) |
| M14 | Webhook delivery | POSTs to `webhookUrl` after dictation (`pipeline.rs:1723-1740`) | `webhookUrl` never read | Android dictations never reach the webhook. Also `webhookHeaders` never read. |
| M15 | STT failure handling | single attempt; error propagates (`stt/mod.rs`) | 2 retries, 2s/5s backoff, 4xx excluded (`KOS:1343-1378`) | Transient blip: Android recovers, Desktop errors. |
| M16 | `pasteDelayMs` settle delay | hardcoded `sleep(50ms)`; config value ignored (`paste/mod.rs:115,247`) | no pre-paste settle delay (`KlarvoAccessibilityService.kt:166-172`) | Config dead on Desktop; Android may paste before clipboard ready. |

## LOW

| # | Shared behavior | Desktop | Android | Note |
|---|---|---|---|---|
| L1 | VAD frames-per-second | 31.25 fps via ceil (`vad/mod.rs:240-242`) | integer `31` (`KAR:70`) | Marginally longer silence window on Android. |
| L2 | Waveform amplitude transform | raw RMS to callback (`audio/mod.rs:1351`) | noiseFloor 0.04 + 2.5× gain + rolling avg (`KAR:387-405`) | Cosmetic. |
| L3 | STT temperature field | sends `temperature=0.0` (`stt/mod.rs:241`) | omits the field (API default 0) | Same effective value; wire differs. |
| L4 | Chunk threshold operator | `len < 400` (`llm/mod.rs:1306`) | `length <= 400` (`KApi:884`) | Off-by-one at exactly 400 chars. |
| L5 | `deviceId` default | auto-generated UUID v4 (`config/mod.rs default_device_id`) | fallback `""` (`KApi:225`) | Only if config.json lacks the key. |
| L6 | Empty-transcript handling | empty → `is_hallucination=true`, silent skip (`stt/hallucination.rs:150`) | extra `isBlank()` "No speech detected" toast (`KOS:1080-1089`) | Different UX message; skip outcome same. |
| L7 | Voice command (`voiceCommandEnabled`) | monitor + `recognize_command` (`lib.rs:985-1003`) | key never read; no impl | Borderline feature-parity, but the config key is written by Desktop, never read by Android. |

---

## Dead config on BOTH sides (latent contract land-mines)

Identical behavior today only because *neither* platform consumes the key — the moment one side wires it, the other silently diverges.

- `advanced.llmTemperature` (default 0.0) → both hardcode 0.3 (`llm/mod.rs:473` ↔ `KApi:775`)
- `advanced.llmMaxTokens` (default 4096) → both hardcode 2048 (`llm/mod.rs:474` ↔ `KApi:776`)
- `chunkThreshold` (default 800) → both hardcode 400 (`config/mod.rs` def ↔ `llm/mod.rs:1236` ↔ `KApi:804`)
- `chunkTargetSize` (default 600) → both hardcode 350 (`llm/mod.rs:1240` ↔ `KApi:805`)
- `advanced.sttTemperature` → wired on neither side
- `advanced.llmModel{Deepseek,Openai,Anthropic,Groq}` overrides → `with_model()` never called Desktop; not read Android
- `advanced.llmSystemPrompt{Polished,Verbatim,Chat}`, `llmCommandModePrompt` → built-ins always win Desktop; not read Android
- `autoCapitalize` (true), `autoPaste` (true) → defined Desktop, consumed nowhere; not read Android

---

## Missed by BOTH models (independent recall sweep)

Real divergences neither Fable 5 nor Opus 4.8 reported — the evidence that a one-shot LLM pass is **not** the parity net:

1. Android's live-autostop gate (`0.02f`, `KAR:58`) is desynced from Android's **own** pre-STT gate (`0.005f`, `SilencePreFilter.kt:27`) — two thresholds that should match, don't.
2. `anthropicApiKey` is absent from the Android Config struct entirely (not merely unread).
3. `webhookHeaders` never read on Android.
4. Live-preview delta (`PreviewFlushConfig`, Epic 5) has no Android counterpart.
5. `customPrompt` vs `sttPrompt*` conflation: Android has a single `customPrompt` field (LLM only); Desktop separates `custom_prompt` (LLM) from `advanced.stt_prompt_*` (STT).

---

## False positives caught in verification (for the record)

Both from the Opus 4.8 run; refuted against real code:

- **Opus #24** "general fallback predicate `is_empty()` vs `isNotBlank()`" — Android logic is null-return + iteration, not an `isNotBlank` predicate. Refuted.
- **Opus #36** "hallucination word-count off-by-one via Regex split" — both count words identically (Kotlin strips empty tokens). Refuted. (Fable flagged the same spot at low confidence rather than asserting a bug.)

---

## A/B outcome (summary)

- Precision near-equal: Opus ~92% (2 hard false positives), Fable ~97% (0). 
- Recall of high-severity divergences favored Fable: the two heaviest findings of the whole audit — **C1 (license unenforced)** and **H2 (UTF-8/UTF-16 chunking)** — came only from Fable, plus the persisted-but-dead settings cluster.
- Opus uniquely found H6/H7 (the two prompt guards) and the dead-config catalogue.
- **The union + verifier + recall sweep beat either model alone** — both together still missed 5 real divergences. This validates the structural Golden-Vector approach over "run the smartest model once."
