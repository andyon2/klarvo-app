---
title: '13-2 Parity sweep: guards and silent loss'
type: 'feature'
created: '2026-09-21'
status: 'review'
baseline_revision: '4e4bc00b4ef28a75370acdb44e905e5699ec9388'
route: 'full'
route_source: 'auto'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
followup_review_recommended: true
context:
  - '{project-root}/_bmad-output/project-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/epic-13-context.md'
  - '{project-root}/_bmad-output/implementation-artifacts/spec-13-1b-test-provider-operability.md'
  - '{project-root}/_bmad-output/implementation-artifacts/spec-13-2-parity-sweep-guards-and-silent-loss.md'
  - '{project-root}/docs/adr/0016-android-path-parity-strategy.md'
  - '{project-root}/docs/adr/0017-shared-core-stt-path.md'
  - '{project-root}/docs/cross-platform-drift-audit-2026-09-16.md'
  - '{project-root}/test-fixtures/README.md'
warnings: ['multiple-goals', 'oversized']
deferred:
  - summary: 'The Android live-preview pause counter keeps the same hangover off-by-one that D-L21 fixes for auto-stop'
    evidence: '`KlarvoAudioRecorder.kt:608-610` (`previewSilentFrames++` then `>= previewRequiredSilentFrames`) has the identical `>=` shape as the auto-stop counter at :588-590, and its desktop twin (`audio/mod.rs:1180-1191`, same `SileroVad` hysteresis) fires on the (N+1)-th frame. Audit row D-L21 cites only `KAR:588-590`, and ADR-0016:260 limits B6 to "Nur die drei gemessenen Deltas", so widening is a fourth unmeasured delta. Deliberately not fixed; one line on top of this story''s extraction if the verdict widens. pre-existing.'
    location: 'android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:608-610'
    severity: 'low'
  - summary: '`SileroVad::reset` does not reset the Silero engine, so the LSTM hidden state leaks across recording sessions'
    evidence: '`vad/mod.rs:292-296` clears the highpass filter, the ring buffer and the hysteresis state, but never calls the crate''s `VoiceActivityDetector::reset` (`voice_activity_detector` 0.2.1 exposes it, klarvo never calls it). The ONNX `Session` is a process-wide `LazyLock<Arc<Mutex<Session>>>` shared by all three VAD instances. Pre-existing, but this story''s D-L19 change makes the hidden-state trajectory load-bearing where it previously froze during quiet passages, so the leak matters more afterwards than before. Not named by any audit row; not fixed here. pre-existing.'
    location: 'src-tauri/src/vad/mod.rs::reset'
    severity: 'medium'
  - summary: 'ADR-0016 Amendment 4 points at an epic file that does not exist'
    evidence: '`docs/adr/0016-android-path-parity-strategy.md:389` names `_bmad-output/planning-artifacts/epics-parity-line-audit-2.md`; Epic 13 actually lives at `_bmad-output/planning-artifacts/epics.md:3535`. The same Nachtrag''s claim at :380 ("keine Story wechselt den Status") is also stale against `sprint-status.yaml:179-189`. Documentation drift only. pre-existing.'
    location: 'docs/adr/0016-android-path-parity-strategy.md:380-389'
    severity: 'low'
  - summary: 'Three stale source claims in the guard path that this story must not propagate'
    evidence: '(a) `groq_jni.rs` parity comment above the guard chain cites `pipeline.rs:501, 1032`, both dead lines; its test comment repeats the claim. (b) `pipeline::is_prompt_echo` doc says ">=60%" while the code uses 0.7. (c) `stt/groq_jni.rs::tests` is android-gated and has never executed (already in `docs/backlog.md`), so it cannot witness any guard change. pre-existing.'
    location: 'src-tauri/src/stt/groq_jni.rs'
    severity: 'low'
  - summary: 'The previous (blocked) 13-2 spec file carries stray tool markup'
    evidence: 'Lines 659-660 of `spec-13-2-parity-sweep-guards-and-silent-loss.md` contain a literal `</content>` and `</invoke>` written into the artifact by the first planning session, between the Verification section and `## Auto Run Result`. Cosmetic; that file is now a historical record. Not reproduced here. pre-existing.'
    location: '_bmad-output/implementation-artifacts/spec-13-2-parity-sweep-guards-and-silent-loss.md'
    severity: 'low'
---

<intent-contract>

## Intent

**Problem:** A second cross-platform drift audit found that Android and Desktop disagree on every
guard and every failure path in the dictation pipeline. On Android a dictionary term of >=10 bytes is
deleted from *every* transcript, a short sentence made of dictionary words is discarded as a
"prompt echo", the guards run in inverted order, the Whisper conditioning prompt is the *LLM cleanup
instruction*, an empty or truncated LLM answer is pasted and stored as the dictation, a malformed
answer on a short dictation gets no provider fallback, an empty STT result burns three Groq calls,
the transcript reaches `history.db` and Turso *before* the banking guard blocks the paste, a failed
paste shows the success check, a failed cleanup shows the success check, and "Offline" still uploads
audio. Desktop has its own half of the same class: a raw transcript carrying a trailing stockphrase
ghost is dropped whole, the in-app record button obeys a different "offline" rule than the hotkey,
the pill says "In Clipboard" for text that is in neither the field nor the clipboard, and the VAD
starves the stateful Silero model of frames during quiet passages.

**Approach:** Close the audit rows ADR-0016 Amendment 4 assigns to this story, by making the
*shared Rust core* the single source of the guard chain (the JNI calls the same functions in the
same order, with the same inputs, as the desktop pipeline), by installing **one** offline predicate
that all three current definitions read, and by making every Android failure path end in the same
observable state its desktop twin already ends in -- reusing shipped states and shipped wording,
never inventing one. Where the audit's direction is refuted by the tree, the human reversed it: on
the VAD, Desktop adapts to Android. Every guard and core-output change is pinned by a fixture read
on both sides, and every new guard is inverted to RED at writing time.

## Boundaries & Constraints

**Always:**
- **The row ids are the contract.** Every task cites its decision-sheet row (B2, B3, ...) and its
  audit row (`D-H5`, ...). The binding verdict is ADR-0016 Amendment 4's row table **as amended by
  Andi at the 13-2 intent gate on 2026-09-21** (see Design Notes -- B6); the audit supplies the code
  evidence only. Nothing else is re-measured and no other verdict is re-opened.
- **ADR-0017 holds: STT request and guard logic live only in Rust.** Android consumes them over the
  JNI. `GroqSttBridge` already declares `nativeIsHallucination`, `nativeIsPromptEcho` and
  `nativeStripPromptFragments`; only the first is called today. Close a guard gap by *calling the
  existing bridge*, never by growing a Kotlin twin. `Adr0017BoundaryGuardTest` stays green.
- **Platform reach is stated per task** (project-context twin rule): shared Rust core -> fix once;
  Rust<->Kotlin twin -> fix twice; platform-gated -> say so.
- **Guards get a fixture read by both sides** (`test-fixtures/`), in the house schema (`PINS:` /
  `DOES NOT PIN:` prose, throwing lookup by id). A fixture with one reader is a written record, not
  a lock. Where ADR-0017 makes the Kotlin column vacuous, the vector says so by construction,
  following the `surface: "stt"` precedent in `test-provider-scenario-vectors.json`.
- **Every new or reshaped guard is inverted at writing time** -- real edit, real run, real revert --
  and the evidence lands in `_bmad-output/implementation-artifacts/gate4-evidence/13-2/` in the
  13-1b shape (`code-inversions.json`, `code-inversion-report.md`, `verdict.md`).
- **Terminal states reuse shipped states and shipped wording.** This story designs no new pill state,
  no new bubble drawing, no new toast text. Where a failure must become visible, it enters a state
  that already exists at `baseline_revision` and is already in use on another surface.
- The test provider from 13-1/13-1b is the reproduction vehicle: `advanced.testProviderLlm` /
  `advanced.testProviderStt` with scenarios `empty`, `truncated`, `malformed`, `http429`, `http5xx`,
  `transport`. Its canned wire tables and all 14 scenario vectors stay byte-identical.

**Never:**
- Do not touch the Desktop banking blocklist (that is **13-6**) -- only the Android ordering half of
  `D-H3` belongs here.
- Do not build, hide or gate any control (that is **13-3**). This story changes behaviour behind
  controls, not the controls themselves. A *stored* `local` value must still be honoured, because
  13-3 has not landed and a hidden control leaves its value behind.
- Do not touch the license gate or the free-tier definition (**13-4**), nor Turso sync semantics
  beyond the ordering fix (**13-5**).
- **Do not build the VAD dual threshold 0.5/0.35 on Android.** It requires bypassing the shipped VAD
  library for `ai.onnxruntime.OrtSession`; Andi cut that as its own L-size story
  (`docs/backlog.md`, "DECIDED 2026-09-21"). This story documents the carve-out and builds the
  hangover frame only.
- Do not "fix" the pre-existing defects listed in frontmatter `deferred`.
- Groq is never a cleanup fallback (Epic 12 FR2). No auto-send on Android. The fallback ladder's
  membership (`deepseek -> openai -> openrouter`) is unchanged.
- Never add an unconditional dependency or `use` that breaks the Android or Linux build.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|---|---|---|---|
| **B2** echo guard input | Dictionary `Klarvo, Kubernetes`; user says "Klarvo und Kubernetes." | Both platforms: overlap computed against the **hint only** -> 0.33 -> transcript survives and is pasted | No error expected |
| **B2** fragment strip input | One-term dictionary `Bundesverfassungsgericht`; transcript "Das Bundesverfassungsgericht hat entschieden." | Both platforms: the term is **not** removed (the dictionary is not part of the strip input) | No error expected |
| **B2/D-M9** guard order | Any transcript with a leaked hint fragment ("<DE hint> Danke") | Both platforms: **strip first, then echo/blocklist** -> "Danke" survives on both | No error expected |
| **B3** ghost before guard | Raw transcript ends in a stockphrase ghost ("... Klinge") | Both platforms: ghost stripped **before** the hallucination guard -> the rest survives (Desktop stops dropping the whole transcript) | No error expected |
| **B3** ghost after cleanup | LLM rationalises a ghost into a fluent stockphrase | Both platforms: ghost stripped **after** cleanup -> ghost gone on Android too | No error expected |
| **B4** conditioning prompt | Cleanup Instruction set (preset "Technical"), identical audio | Android conditions Whisper with `advanced.sttPrompt{De,En,Auto}` like Desktop; `customPrompt` reaches the **LLM only** | Missing `sttPrompt*` -> the built-in language hint, as Desktop |
| **D2** empty LLM content | Test provider `testProviderLlm = empty` | Both: no paste, **no history row**, raw transcript to clipboard + named cause | Mapper raises the twin of `LlmError::ResponseFormat`; non-retryable |
| **D3** truncated answer | `testProviderLlm = truncated` (`finish_reason == "length"`) | Both: not pasted; raw transcript to clipboard + named cause | Mapper raises the twin of `LlmError::OutputTruncated`; non-retryable |
| **D10** malformed answer, short dictation | `testProviderLlm = malformed`, transcript < `CHUNK_THRESHOLD` (400 B) | Android fires the provider ladder as Desktop does (`deepseek -> openai -> openrouter`) | Undecodable body is classified retryable on **both** paths, chunked and single-call |
| **D9** empty STT result | `testProviderStt = empty` | Android marks it **non-retryable** and reports at once; retry budget **1** as Desktop | New non-retryable sentinel from the JNI; no 2 s/5 s backoff burn |
| **D4** no focused field | Accessibility connected, no editable node focused | Android: **no success check**; the shipped "Copied: ..." clipboard toast fires | Paste result is reported back, not discarded |
| **D5** cleanup failed | `llmCleanupFailed == true` | Android: **no DONE flash** -- straight to IDLE, degrade toast carries the cause (the code's own comment already promises this) | No error expected |
| **D6** clipboard write throws (Android) | `setPrimaryClip` throws | Caught; no success check; `pasteErrorCount` incremented | Never an uncaught main-thread exception |
| **D6** clipboard write fails (Desktop, normal run) | `set_clipboard` returns `Err` with no cleanup degrade | The shipped `ClipboardWriteFailed` cause fires and the "TEXT LOST" card shows -- the pill stops claiming "In Clipboard" | A *focus* failure (`ClipboardOnly`, `Ok`) keeps today's wording |
| **D11** nothing recognized | Mini-tap / silent capture / blank transcript / hallucination verdict | Android is **silent**, like Desktop (`PipelineEvent::idle()`) | `"No audio recorded"` (a capture fault, not a recognition result) is kept |
| **B1-Android** banking guard | Blocklisted app in the foreground | History row **and** Turso push happen only after a non-blocking verdict; a blocked dictation writes neither | Guard verdict is read on the main looper as today |
| **E1** preview flush offline | Stored `sttProvider = local`, live preview on | No delta WAV leaves the device -- the flush is not installed and is re-checked at flush time, as Desktop | No network call at all, key present or not |
| **E2** one offline rule | `sttProvider = local` + any cloud `llmProvider`, hotkey / in-app button / Android | One predicate: local STT => local cleanup **or none**. Identical raw-text outcome on all three | `llmProvider = "local"` where no local LLM exists degrades to **no cleanup**, never to a silent network call |
| **D-L19** quiet frame, Desktop | RMS below `energy_floor` | Silero is **still called** with the frame (its recurrent state stays coherent); the frame is still classified Silence | Per-frame verdict provably unchanged; only later frames' probabilities improve |
| **D-L21** hangover edge, Android | N-th consecutive non-speech frame after speech, N = `requiredSilentFrames` | Auto-stop does **not** fire yet; it fires on the **(N+1)-th**, as `vad/mod.rs::advance_state` does | No error expected |

</intent-contract>

## Code Map

Anchors are symbols; line numbers are navigation hints at `baseline_revision` only, and were
re-measured against the tree. The audit's own anchors are stale -- `pipeline.rs` ~ +43,
`stt/mod.rs` ~ +33, `groq_jni.rs` ~ +93 vs. the audit's tree, and 13-1 extracted both HTTP mappers.
`baseline_revision` `4e4bc00` differs from the first planning run's `3f87bef` by two commits that
touch **no source file** (`git diff --stat 3f87bef 4e4bc00` = the run ledger, the old spec,
`docs/backlog.md`), so every anchor below is equally valid at either revision.

### Rust -- the guard chain (shared core, serves both platforms)

- `src-tauri/src/pipeline.rs`
  - `is_prompt_echo(transcription, stt_hint) -> bool` :424-509 -- overlap **>=0.7** :481-490, `>30`-word
    bail :477, diversity branch :492. Its doc still says ">=60%" (deferred).
  - `strip_prompt_fragments(text, stt_hint) -> String` :535-593 -- >=10-byte rule :549/:555,
    case-insensitive substring removal :563-577, punctuation-token drop :585-592. `DEFAULT_STT_HINTS`
    :515.
  - `post_stt_skip(transcript, stt_hint) -> Option<PostSttSkip>` :690-698 and `enum PostSttSkip`
    :681-686 -- **this is the seam**: the desktop guard decision already lives in one pure function.
  - Desktop chain: strip :1427-1433 (rationale comment :1423-1426) -> `post_stt_skip` :1438 -> skip
    terminal :1446-1450 (`ProcessOutcome::Stopped`, nothing pasted, nothing saved). Verified: there
    is **no** pre-guard ghost strip on Desktop today.
  - `stt_hint` selection from `advanced.stt_prompt_{de,en,auto}` :1794-1806; wire prompt via
    `stt::build_stt_prompt_with_hint` :1807-1811; **`stt_hint_text` = the hint only** :1814-1818.
  - `custom_prompt` :1162, :1865-1884 -- consumed **only** at the LLM call sites :1505, :1542.
  - Ghost strip, desktop: `strip_stockphrase_ghosts` called **once**, after `sanitize_llm_output`
    :1617-1618, rationale :1614-1616.
  - `is_retryable_stt_error` :357-360 (`ResponseFormat` -> non-retryable); non-retryable terminal
    :1396-1416.
  - Offline: `is_offline` :643-645, caller :1824; `select_llm_path` :713-721 -> `LlmPath::OfflineRaw`
    :704; skip site :1459, :1469-1475. Local-LLM arm `#[cfg(target_os = "windows")]` :307-321 and the
    `_ => cleanup_provider_for("deepseek", ...)` fall-through :322-323. `effective_llm_provider_name`
    :290-296 (13-1b) -- the **neighbour** of the new predicate, not the predicate.
  - Live preview: `preview_flush_should_install` :2456-2458, install site :2627, flush-time recheck
    :2545-2553, `flush_preview_delta` :2524-2586.
  - Clipboard truth: `deliver_text` :2361-2402, `paste_error_count` bump :2011-2016, terminal emit
    :2195-2209 (`DoneClipboard` :2195-2207, plain `Done` :2208), **`terminal_degrade_cause`
    :2427-2436 -- the `Some(_) if paste_failed` arm is the whole bug**. Pinned as intended by
    `spec_non_degraded_run_never_gains_a_cause` :5772-5778 (that test must be rewritten, not deleted).
- `src-tauri/src/stt/hallucination.rs` -- `strip_stockphrase_ghosts` :181-~280; `is_hallucination`
  :314-374, stockphrase block **without a word-count gate** :324-345 (the `>8`-word gate only applies
  afterwards :347-352). Re-exported at `stt/mod.rs:30`.
- `src-tauri/src/stt/mod.rs` -- `build_stt_prompt_with_hint` :121-146; `custom_hint` **replaces** the
  built-in hint :128-136; terms appended with **no separator** :139 (`format!("{hint}{terms}")`) --
  safe only because each built-in literal ends in a space :132-134. `map_transcription_http_response`
  :~395-465 with the three `ResponseFormat` messages :440-442, :450-452, :457-459.
  `WhisperStt::transcribe` :362-387 sends the whole multipart body before any 401.
  Host-reachable JNI selector tests `spec_android_select_stt_provider_*` :1403-1500; fixture loader
  `load_test_vectors` :1133, throwing lookup `test_vector` :1145.
- `src-tauri/src/stt/groq_jni.rs` -- **the divergence site.** Current `nativeTranscribe` params
  :211-222: `wav_base64, api_key, language, dictionary_terms, custom_prompt, stt_model, temperature,
  test_provider_stt` (arity 8 since 13-1b; the `#[no_mangle]` stale-`.so` misbind warning is
  :195-198). Prompt built :266-268. Guard chain **inverted and mis-fed**: echo :307-311 with
  `let hint = prompt.as_deref().unwrap_or("")` (the *full* prompt incl. dictionary and
  `customPrompt`) -> strip :312 -> ghost :314. Parity-claiming comment :302-306 (cites two dead
  desktop lines). Sentinels: `__ERROR_EMPTY_AUDIO__` :262/:320, `__ERROR_API:` :323,
  `__ERROR_NETWORK:` :328 (**catch-all -- `ResponseFormat` lands here**), documented :202-208.
  `nativeIsPromptEcho` :370-389 and `nativeStripPromptFragments` :392-418 exist and are off the
  transcribe path. `select_stt_provider` :93-110 is the **extraction precedent**:
  `#[cfg_attr(not(any(test, target_os = "android")), allow(dead_code))]` + a plain-Rust signature, so
  `cargo test --lib` reaches it. Everything else in the file is `#[cfg(target_os = "android")]`; the
  file's own `mod tests` :504-506 is android-gated and has never run.
- `src-tauri/src/commands/recording.rs` -- `is_offline_mode` :101-112 (`stt_provider == "local"`
  **alone**) used by `cleanup_text` :243-247 -> early `return Ok(raw_text)`. This is the React in-app
  button path (`src/hooks/useRecording.ts`), which also runs on Android inside `TauriActivity`.
- `src-tauri/src/overlay_message.rs` -- `enum DegradeCause` :218-238, `status_line()` :244-259,
  `card()` `ClipboardWriteFailed` -> header `"TEXT LOST"` :298-307, `PILL_LABEL_CLIPBOARD =
  "In Clipboard"` :191.
- `src-tauri/src/paste/mod.rs` -- `set_clipboard` :120-126 (wraps `arboard`, maps to
  `PasteError::Clipboard(String)`), `copy_only` :91-104. **The failure is already observable and
  already plumbed to `deliver_text` as `delivery.paste_failed`** -- nothing new must be measured.
- `src-tauri/src/native_pill.rs` -- `DoneClipboard` / `DoneDegraded` :132-134, mapped :192. Desktop
  already has the degraded terminal states Android lacks.
- `src-tauri/src/test_helpers.rs` -- `temp_dir()`, `make_state(&TempDir) -> AppState`.

### Rust -- the VAD (D-L19, direction reversed: Desktop adapts to Android)

- `src-tauri/src/vad/mod.rs`
  - `VadConfig` :53-79 / `Default` :81-93 (`onset 0.5`, `offset 0.35`, `hangover_ms 608`,
    `min_onset_frames 3`, `energy_floor 0.001`). The `energy_floor` doc :75-78 says *"Minimum RMS
    energy required before Silero is called ... saving CPU on truly silent passages"* -- that
    sentence is what this change falsifies and must be rewritten.
  - `enum HysteresisState` :190-200; `SileroVad` fields :220-228 (`engine: VoiceActivityDetector`,
    a concrete crate type, **not** a trait object -- a closure seam is therefore the only cheap way
    to observe the call); `with_config` :237-259.
  - `process_frame` :304-317 -- **the change site**. Today:
    `let prob = if energy_ok { self.engine.predict(frame.to_vec()) } else { 0.0 };`
  - `advance_state` :323-375 -- already takes `energy_ok` as its own parameter and ANDs it into both
    `above_onset` :324 and `above_offset` :325. **This is why the change is verdict-preserving:**
    when `energy_ok` is false the frame is Silence whatever `prob` holds.
  - `feed` :267-283 (highpass -> ring buffer -> `process_frame` per 512 samples), `reset` :293-297,
    `current_speech_state` :379-388, free fn `rms` :397.
  - Tests that drive `advance_state` directly (unaffected): onset :~480-499, hangover :509-549.
    `test_energy_gate_suppresses_very_quiet_signal` :~553+ drives `feed` and must stay green -- it
    pins the *verdict*, which does not change.

### Kotlin (`android/kotlin-src/com/klarvo/voice/`)

- `KlarvoApi.kt` (1832 L)
  - `mapCleanupResponse(responseCode, body, model): String` :212-224 -- non-200 ->
    `IOException("LLM cleanup failed (...): HTTP $code -- $body")` :214; else `getString("content")
    .trim()` :216-222 -> `sanitizeLlmOutput` :223. **No emptiness check, `finish_reason` never read.**
    Its KDoc :186-211 names D-H19/D-M16/D-M2 as deliberate, fixture-pinned divergences and says
    *"story 13-2 owns the fix"* -- that KDoc must be rewritten with the behaviour, not left lying.
    Callers :252, :287, :1482, :1489.
  - `CLEANUP_MAX_TOKENS = 2048` :73, emitted :1473 inside `cleanup(...)` :1339-1491. Canned wire
    `truncated` already carries `"finish_reason":"length"` :166.
  - `collectChunkResults` :1637-1645 (re-wrap :1640-1644); `CHUNK_THRESHOLD = 400` :1494;
    `shouldChunk` :1625; `isTrivialChunk` :1507.
  - `saveToHistory` :971-1000 -- returns **Unit**, no row id exists anywhere. `pushToTurso` :1089
    re-reads unsynced rows from `history.db` itself.
  - `readConfig` -- `sttProvider` parsed :814; the admitting line **:933**
    (`gatedSttProvider != "local" && groqKey.isBlank()` -> null), i.e. `local` is admitted with no
    Groq key. `Config` :383 has **no `sttPrompt*` field at all**. `gateProvidersForLicense` is
    13-1b's pure seam and is **13-4's subject** -- do not change its decision.
  - `pasteErrorCount` declared :1745, read :1765, serialised :1784 -- **never incremented**.
  - `sanitizeLlmOutput` strips control characters only; there is no post-cleanup ghost strip.
- `KlarvoOverlayService.kt` (2851 L)
  - `enum RecordingState { IDLE, RECORDING, TRANSCRIBING, DONE }` :299 -- **no degraded state**;
    `setState` :2585-2601 maps 1:1 to `FloatingBubbleView.State` (`FloatingBubbleView.kt:79`,
    dispatch :733-741). `DEBUG_SET_STATE` string map :475-478 (the harness reachability seam).
    `doneFlashRunnable` :627-634 (800 ms).
  - `DeliveryDecision(paste, showCopiedToast)` :184 and `decideDelivery` :2212-2222 -- pure, already
    JVM-tested by `CleanupFailureDeliveryTest`. Today it **pre-decides** `showCopiedToast =
    !accessibilityConnected` :221 and `llmCleanupFailed -> DeliveryDecision(false, false)` :216.
  - Step layout: Step 2 cleanup :2182-2281 (`finalText` :2183, catch :2210, fallback gate :2236-2240)
    -> **Step 3 `saveToHistory` :2284-2296 -> Step 3b Turso thread :2298-2307** -> Step 4 header
    :2311-2322 -> `handler.post {` :2323 -> `BankingGuard.shouldBlockPaste(bankingAppActive)` :2327
    -> blocked branch :2328-2334 (`return@post`) -> `copyToClipboard` :2337 -> paste :2347-2351 ->
    `showCopiedToast` :2358 (`"Copied: $preview"`) -> DONE flash :2389-2396.
  - `copyToClipboard` :2642-2646 -- **unguarded** inside `handler.post`.
  - `isRetryableCleanupFailure` :2718-2721 (private, pure `String -> Boolean`, regex `HTTP (\d{3})`).
  - `transcribeWithRetry` :2734-2743 (8 params since 13-1b); `retryDelaysMs = listOf(2_000L, 5_000L)`
    :2745; `for (attempt in 0..retryDelaysMs.size)` :2748 = **3 attempts**; sentinel `when`
    :2766-2816 (`__ERROR_EMPTY_AUDIO__` :2771 non-retryable, `__ERROR_API:` :2778 4xx throw / 5xx
    retry, `__ERROR_NETWORK:` :2799-2800 retry, unknown -> retry); terminal throw
    `"Groq STT failed after retries: ..."` :~2818. Local-Whisper net :2095-2126, gate
    `isRetryableSttFailure` :2707-2709.
  - Preview: `RecordingMode.shouldInstallPreviewFlush(mode, livePreviewEnabled)` :294-295 -- **no
    `sttProvider` parameter**; install :1666; `flushPreviewDelta` :1754-1781 passes `groqApiKey`
    blind. `nativeIsHallucination` call sites :1773, :2165 (the **only** bridge guard Kotlin calls).
  - Offline branch :2183 -- `config.llmProvider == "local"` only.
  - `customPrompt` passed as the JNI `custom_prompt` :2754, call sites :1768 (preview) / :2091.
  - Toasts (all string literals): :1919 `"No audio recorded"`, :1945 `"Recording too short"`, :1958
    `"No speech detected"`, :2141 `"No speech detected"`, :2168 `"Speech not recognized"`; keep :1983
    (no API keys) and :2328 (`"Paste blocked -- banking app active."`). All five share an identical
    6-line epilogue (`autoLoopActive = false; hideListeningPanel(); prev; setState(IDLE);
    adjustLayoutForState(IDLE, prev)`).
- `KlarvoAccessibilityService.kt` (303 L) -- `pasteIntoFocusedField()` :179-185 returns **Unit**;
  `rootInActiveWindow ?: return` :180, `findFocusedEditable` :181 (defined :231-240),
  `focusedNode?.performAction(ACTION_PASTE)` :182 with its `Boolean` **discarded**. Three silent
  failure modes, zero logging. **Exactly one caller: `KlarvoOverlayService.kt:2350`.** Good precedent
  next door: `performEnter` :202-229 uses the same finder and already logs all three misses.
- `KlarvoAudioRecorder.kt` (768 L)
  - `vadGateDecision` :248-261 (companion, pure, the 7-2 extraction precedent) -- **this is the shape
    Desktop must adopt.** It calls `val vadSpeech = isSpeech(filteredFrame)` **unconditionally** and
    returns `energyAboveGate && vadSpeech`: the energy gate decides the verdict, never the call.
    `isSpeech` is a function parameter precisely so a JVM test can drive the real sequence with a
    fake. Call site :532-539.
  - `processVadFrame` :520-630; onset :559-575 (`onsetFrames >= VAD_ONSET_FRAMES`); hangover
    :577-616 -- **the one-shot at :588-590**: `silentFrames++` then
    `if (silentFrames >= requiredSilentFrames)`, i.e. it fires on the **N-th** non-speech frame.
    Repeatable preview-pause edge :618-627 has the *same* shape with `previewSilentFrames` /
    `previewRequiredSilentFrames`.
  - Constants :156-186: `VAD_FRAME_SIZE 512`, `VAD_ONSET_FRAMES 3`, `VAD_FRAME_MS 32.0`,
    `VAD_FRAMES_PER_SECOND 31.25`, `HANGOVER_FLOOR_MS 200f`, `MIN_SILENT_FRAMES 7`,
    `HIGHPASS_CUTOFF_HZ 85f`; `framesForSeconds` :155-156. `VadSilero` construction :407-412
    (`Mode.NORMAL`). **No test constructs `KlarvoAudioRecorder`** -- only its companion is reachable.
- `BankingGuard.kt` -- `shouldBlockPaste` :23 (pure, JVM-tested by `BankingGuardTest`); its KDoc
  :11-13 states `bankingAppActive` (`KOS:343`, written :877-879) is **main-looper-owned**, which is
  why the verdict is read inside `handler.post`.
- `GroqSttBridge.kt` -- `nativeTranscribe` and the three guard externs `nativeIsPromptEcho` :92,
  `nativeStripPromptFragments` :99.

### Fixtures & tests

- `test-fixtures/` -- `README.md` carries the reader ledger (update it for every new fixture).
  Schema: flat JSON array; per vector `id`, `surface`, `scenario`, `description` (with `PINS:` /
  `DOES NOT PIN:`), `wire`, `rust`, `kotlin`, `expected_divergence`.
  `test-provider-scenario-vectors.json` -- `TEST-LLM-EMPTY-001`, `TEST-LLM-TRUNCATED-001`,
  `TEST-LLM-MALFORMED-001`, `TEST-STT-EMPTY-001` each carry an `expected_divergence` string naming
  this story; those strings and the `kotlin` columns must be updated when the divergence closes.
  `vad-gate-golden-vectors-7-2.json` is **Kotlin-only** (`VAD-LATENCY-*` pin `expected_frames` =
  *required* frames, not the trigger edge).
- Rust readers duplicate `load_test_vectors()` per module (`llm/mod.rs:2986`, `stt/mod.rs:1133`);
  Kotlin readers resolve four candidate paths from `user.dir` (`TestProviderScenarioTest.loadFixture`,
  `Adr0017BoundaryGuardTest.kotlinSrcDir`).
- `android/kotlin-test/com/klarvo/voice/` (25 files): `TestProviderScenarioTest.kt` 968 L (drives
  `mapCleanupResponse` directly: empty :242, truncated :256, malformed-is-not-IOException :274,
  chunked re-wrap :298), `Adr0017BoundaryGuardTest.kt` 229 L, `CleanupFailureDeliveryTest.kt` 169 L,
  `BankingGuardTest.kt` 72 L, `VadGateGoldenVectorsTest.kt` 315 L, `ShouldInstallPreviewFlushTest.kt`
  71 L. **Plain JUnit only -- no Robolectric, no mocking library.** Anything touching `android.util.Log`
  (every `KlarvoLogger` call), `Context`, `Handler`, `Toast`, `ClipboardManager` or
  `AccessibilityNodeInfo` throws "not mocked"; the 13-1/13-1b answer is a pure companion seam.
- `Adr0017BoundaryGuardTest` scans `.kt` files **one level** under `android/kotlin-src/com/klarvo/voice/`,
  strips comments string-aware, then matches four shapes: a `class|object|interface` named
  `HallucinationFilter`/`SilencePreFilter`, `fun buildMultipartBody`, `multipart/form-data`,
  `audio/transcriptions`. Comments are safe; `android/kotlin-test/` is out of scope.

## Tasks & Acceptance

**Execution:**

*Group 1 -- the guard chain becomes one chain (B2 / D-H5, D-H6, D-M9; B3 / D-H7)*

- [x] `src-tauri/src/pipeline.rs` -- extract the desktop chain into one pure, host-reachable function
      (e.g. `guard_transcript(raw, stt_hint) -> GuardOutcome`) that performs, in this order: ghost
      strip -> `strip_prompt_fragments` -> `post_stt_skip`. Rewire the desktop call sites
      (:1427-1450) to it so desktop behaviour is defined by the same function Android will call.
      Adding the **pre-guard ghost strip on Desktop is a deliberate behaviour change** (B3): a raw
      transcript with a trailing ghost stops being dropped whole. Keep `PostSttSkip` and the skip
      terminal as they are.
- [x] `src-tauri/src/stt/groq_jni.rs` -- replace the inline chain (:307-314) with a call to that one
      function, fed the **hint only** (the same value `pipeline` computes as `stt_hint_text`), not
      the full built prompt. This closes D-H5, D-H6 and D-M9 together. Give the JNI a plain-Rust
      helper in the `select_stt_provider` shape
      (`#[cfg_attr(not(any(test, target_os = "android")), allow(dead_code))]`) so `cargo test --lib`
      can reach the chain -- without it every B2/B3 claim is agent-only. Delete the stale parity
      comment :302-306 rather than re-citing dead lines.
- [x] `src-tauri/src/stt/groq_jni.rs` + `KlarvoOverlayService.kt` -- the hint must reach the JNI
      separately from the wire prompt (today only the combined prompt crosses). Either pass the hint
      as its own argument (JNI arity change => **both sides in one commit**, and Andi's reproduction
      needs `scripts/android-install-debug.sh <ip:port> --full`), or rebuild the hint inside the JNI
      from the fields it already receives. State which, and why, in Implementation Notes.
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` -- after cleanup, put the
      transcript through the **existing** `GroqSttBridge.nativeStripPromptFragments` bridge (it
      already applies `strip_stockphrase_ghosts` to its output) or a sibling extern, so the
      post-cleanup ghost strip (B3's second half) runs on Android **in Rust**. Do not write a Kotlin
      ghost-strip twin -- `Adr0017BoundaryGuardTest` would not catch it, but ADR-0017 and
      project-context both forbid it.
- [x] `test-fixtures/guard-chain-vectors.json` (new) -- vectors for: dictionary-word-only utterance
      (echo overlap), a >=10-byte dictionary term inside a sentence (fragment strip), a leaked hint
      fragment (order), a trailing raw ghost (pre-guard), a cleanup-rationalised ghost (post-cleanup).
      Rust reader in `pipeline.rs`; Kotlin column declares itself n/a-by-construction (ADR-0017)
      following the `surface: "stt"` precedent, **plus** a Kotlin assertion that the bridge is what is
      called. Add the ledger row to `test-fixtures/README.md`.

*Group 2 -- conditioning prompt (B4 / D-H4)*

- [x] `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` -- add `sttPromptDe` / `sttPromptEn` /
      `sttPromptAuto` to `Config` (appended **last**; the constructor is positional) and read them in
      `readConfig` from `advanced`. Twin of `pipeline.rs:1794-1806`.
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` -- stop passing `config.customPrompt`
      as the JNI `custom_prompt` (:2754, and the preview path :1768); pass the language-selected
      `sttPrompt*` instead. `customPrompt` keeps going to the LLM (:2204/:2248) and **only** there.
- [x] `src-tauri/src/stt/mod.rs` -- `build_stt_prompt_with_hint` joins hint and terms with no
      separator (:139) and is safe only because the built-in literals end in a space. A user-supplied
      `sttPromptDe` without one now glues into the first dictionary term on **both** platforms. Make
      the join explicit and pin it with a vector.

*Group 3 -- silent loss on the LLM path (D2 / D-H19, D3 / D-M16, D10 / D-M2)*

- [x] `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt::mapCleanupResponse` -- raise the twin of
      `LlmError::ResponseFormat` on empty content and of `LlmError::OutputTruncated` on
      `finish_reason == "length"`. Both are **non-retryable**: no provider ladder, no paste, no
      history row -- the raw transcript goes to the clipboard with a named cause, exactly as
      `pipeline.rs:1549-1565` does. Rewrite the KDoc :186-211, which currently documents the
      divergence as intended.
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` -- classify an undecodable body as
      retryable on the **single-call** path too, so the ladder fires as it already does on the chunked
      path. Move `isRetryableCleanupFailure` (:2718) to the companion (visibility only, no behaviour
      change) so a JVM test can drive it. Ladder membership is unchanged.
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` -- an empty `finalText` must never
      reach `saveToHistory` (:2284-2296), the clipboard (:2337) or the paste (:2347-2351).
- [x] `test-fixtures/test-provider-scenario-vectors.json` -- update the `kotlin` columns and the
      `expected_divergence` strings of `TEST-LLM-EMPTY-001`, `TEST-LLM-TRUNCATED-001`,
      `TEST-LLM-MALFORMED-001` (they currently say "Story 13-2 closes this"). Keep every `wire`
      payload and all 14 scenario bodies byte-identical. Update
      `TestProviderScenarioTest.kt` :242, :256, :274, :298 accordingly.

*Group 4 -- silent loss on the STT path (D9 / D-M5, D-M6)*

- [x] `src-tauri/src/stt/groq_jni.rs` -- match `SttError::ResponseFormat` **before** the
      `__ERROR_NETWORK:` catch-all (:327-331) and emit a distinct non-retryable sentinel; document it
      beside the others (:202-208).
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` -- handle the new sentinel as
      non-retryable in the `when` (:2766-2816) and cut the retry budget to **1** (`retryDelaysMs`
      :2745) to match Desktop's single attempt. Extract the sentinel->verdict decision into a pure
      companion function (`classifySttSentinel`) so the whole ladder is JVM-testable; the backoff
      loop itself stays on-device.

*Group 5 -- terminal states tell the truth (D4 / D-H20, D5 / D-M24, D6 / D-M12)*

- [x] `android/kotlin-src/com/klarvo/voice/KlarvoAccessibilityService.kt` -- `pasteIntoFocusedField`
      returns a result instead of `Unit` (the three misses are already distinguishable; `performEnter`
      :202-229 is the shipped logging precedent). One caller to update (`KOS:2350`).
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` -- Step 4 stops deciding on
      `instance != null` (:2347-2351). Extend the pure `decideDelivery` (:2212-2222) to take the paste
      outcome so the toast decision moves **after** the paste: a failed paste shows the shipped
      `"Copied: $preview"` toast (:2358) and **no DONE flash**. Extend `CleanupFailureDeliveryTest`.
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` -- the DONE flash (:2389-2396) runs
      only on a genuine success, which is what its own comment already claims
      (*"Only the success path gets the DONE state; error paths go straight to IDLE"*). Put the choice
      in a pure companion function (`terminalStateFor(...)`) so it is JVM-testable. **No new
      `RecordingState`, no new bubble drawing** (Andi confirmed 2026-09-21) -- a failed run ends in the
      shipped IDLE state with the existing degrade toast carrying the cause.
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` -- wrap `copyToClipboard`
      (:2642-2646) in try/catch; on failure increment `pasteErrorCount` (`KlarvoApi.kt:1745`, today
      never incremented) and do not show success. Never an uncaught main-thread exception.
- [x] `src-tauri/src/pipeline.rs::terminal_degrade_cause` (:2427-2436) -- widen the `Some(_) if
      paste_failed` arm so a clipboard-write failure on a **normal** run also produces the shipped
      `ClipboardWriteFailed` cause and its shipped `"TEXT LOST"` card
      (`overlay_message.rs:298-307`). Discriminate against a *focus* failure
      (`PasteResult::ClipboardOnly` returns `Ok`), which keeps today's `"In Clipboard"` wording.
      Rewrite `spec_non_degraded_run_never_gains_a_cause` (:5772-5778) -- it pins the bug.

*Group 6 -- the quiet Android (D11 / D-M14)*

- [x] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` -- remove the four
      nothing-recognized toasts (:1945, :1958, :2141, :2168), leaving their shared 6-line epilogue
      intact. **Keep** `"No audio recorded"` (:1919 -- a capture fault, not a recognition result),
      `"No API keys configured..."` (:1983) and `"Paste blocked -- banking app active."` (:2328).
      Andi confirmed the four-of-five reading on 2026-09-21; see Design Notes.

*Group 7 -- ordering (B1 Android half / D-H3)*

- [x] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` -- move Step 3 `saveToHistory`
      (:2284-2296) and Step 3b `pushToTurso` (:2298-2307) **after** the guard verdict (:2327); a
      blocked dictation writes neither. `saveToHistory` returns `Unit` and `pushToTurso` re-reads
      `history.db` itself, so nothing depends on the old order -- but the guard is read on the main
      looper, so the writes must be posted back to a worker thread rather than run there. The separate
      `savePendingHistoryEntry` path (outer IOException catch, ~:2420) is unaffected.

*Group 8 -- one offline rule (E1 / D-H9, E2 / D-H10, D-M20, D-M21)*

- [x] `src-tauri/src/pipeline.rs` -- install one pure predicate on `&AppConfig` beside
      `effective_llm_provider_name` (:290-296): local cleanup counts as available only where it
      exists, and `is_offline` (:643-645) is expressed through it. Make the
      `#[cfg(target_os = "windows")]` fall-through (:322-323) stop resolving an unavailable `"local"`
      to a silent network DeepSeek call -- under G2a the answer is **no cleanup** (the shipped
      `OfflineRaw` outcome), never a cloud call.
- [x] `src-tauri/src/commands/recording.rs` -- delete `is_offline_mode` (:101-112) and make
      `cleanup_text` (:243-247) read the shared predicate. The helper stays `&AppConfig`-shaped, not
      `&AppState`-shaped. This is the whole D-M20 fix.
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` -- the offline branch (:2183) reads
      the twin of that predicate instead of `config.llmProvider == "local"` alone. Put the twin in a
      pure companion function so a JVM test drives it (twinned behaviour, per project-context: fix
      twice, pin with a fixture).
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` -- `shouldInstallPreviewFlush`
      (:294-295) takes `sttProvider` and refuses to install for a stored `local`; `flushPreviewDelta`
      (:1754-1781) re-checks at flush time, as `pipeline.rs:2545-2553` does. Extend
      `ShouldInstallPreviewFlushTest`. This is E1 and it is mandatory independently of 13-3.
- [x] `test-fixtures/offline-rule-vectors.json` (new) -- the config matrix
      (`stt in {local, cloud}` x `llm in {local, cloud}` x platform-availability) with one expected
      outcome per row, read by a Rust test and a JVM test. Ledger row in `README.md`.

*Group 9 -- VAD: the two deltas Andi released, and the one he cut (B6 / D-M10, D-L19, D-L21)*

Andi answered the first run's blocking question on 2026-09-21. B6 ships **two** of its three measured
deltas; the third is cut to its own story. B6 carries **no H+** (`ADR-0016:289`, marker `🤖`), so the
whole group is Weg 2 -- agent-verified, golden vector only. Do not promise a device repro.

- [x] `src-tauri/src/vad/mod.rs` -- **D-L19, direction REVERSED by Andi: Desktop adapts to Android.**
      `process_frame` (:304-317) calls `self.engine.predict` on **every** frame; the energy floor keeps
      its *verdict* role only. Mirror the Kotlin seam `KlarvoAudioRecorder.vadGateDecision` (:248-261)
      literally -- extract a pure, host-reachable helper that **always** invokes the predictor and
      returns `(prob, energy_ok)`, e.g.
      `pub(crate) fn frame_decision(frame: &[f32], energy_floor: f32, predict: impl FnOnce(&[f32]) -> f32) -> (f32, bool)`,
      and have `process_frame` pass `|f| self.engine.predict(f.to_vec())`. **The per-frame verdict is
      provably unchanged**: `advance_state` (:324-325) ANDs `energy_ok` into both `above_onset` and
      `above_offset`, so the probability is irrelevant on a sub-floor frame. **Do not remove that
      AND** -- it is what keeps the energy gate meaningful. Rewrite the `VadConfig::energy_floor` doc
      (:75-78): the floor no longer gates the *call*, and its "saving CPU on truly silent passages"
      claim is now false.
      **Reach: shared Rust, three instances inherit it** -- auto-stop (`audio/mod.rs:1078`),
      live-preview flush (`audio/mod.rs:1186`) and the voice-command engine
      (`voice_command/mod.rs:161`). Kotlin needs **no** change: it already calls on every frame.
- [x] `src-tauri/src/vad/mod.rs` -- a test that drives the new helper with a **counting fake
      predictor** on a sub-floor frame and asserts (a) the predictor ran and (b) `energy_ok == false`.
      Without it the change is invisible: `engine` is the concrete `VoiceActivityDetector` (crate
      `voice_activity_detector` 0.2.1, `predict(&mut self)`), not a trait object, so no other seam
      exists. Inversion: restore the `if energy_ok` guard around the call -> RED.
      Then re-run `test_silence_stays_silence` (:456): it moves from a deterministic `prob = 0.0`
      path to real ONNX output on digital-silence frames. If it turns red or flaky, **report it and
      stop -- do not weaken the assertion.**
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt` -- **D-L21**: the auto-stop one-shot
      (:588-590) fires on the **(N+1)-th** consecutive non-speech frame, as `vad/mod.rs::advance_state`
      does (`VAD:352-368`; its own test feeds 1+18+1 for N=19), not on the N-th. Extract the edge into
      a pure companion function first (the `vadGateDecision` precedent at :248-261), e.g.
      `internal fun hangoverFired(silentFrames: Int, requiredSilentFrames: Int): Boolean` -- **no test
      constructs `KlarvoAudioRecorder`**, so without the extraction the change is unverifiable.
      Change **only** the `onSilenceDetected` counter. The preview-pause counter (:608-610) has the
      identical shape and stays as it is -- see Design Notes.
- [x] `test-fixtures/vad-gate-golden-vectors-7-2.json` (extend) -- pin the hangover **trigger edge**
      (today's `VAD-LATENCY-*` vectors pin `expected_frames` = *required* frames, not the edge) and
      pin "the engine is consulted on a sub-floor frame". Add a **Rust** reader -- the file is
      Kotlin-only today -- alongside the Kotlin reader in `VadGateGoldenVectorsTest`. Update the
      ledger row in `test-fixtures/README.md`.
- [x] `android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt` -- **D-M10 carve-out, documentation
      only, no behaviour change.** In the KDoc beside the VAD constants (:156-186) and at the
      `VadSilero` construction (:407-412), record: the shipped
      `com.github.gkonovalov.android-vad:silero:2.0.10` exposes only `isSpeech -> boolean`;
      `predict` / `threshold` / `extractResult` are private and `Mode.NORMAL` hardcodes `0.5f`, so the
      desktop offset threshold 0.35 **cannot** be evaluated on Android, and Android's comparison is
      `prob > 0.5` where Rust's onset is `prob >= 0.5`. Cite `docs/backlog.md`
      ("DECIDED 2026-09-21 -- Android-VAD: Doppelschwelle 0,5/0,35 ist eine eigene Story, Groesse L")
      and ADR-0016 Amendment 4 row B6. **Do not add the ONNX dependency and do not touch `Mode`.**

**Acceptance Criteria:**

- Given a dictionary containing `Klarvo, Kubernetes` and the utterance "Klarvo und Kubernetes.", when
  the transcript passes the guard chain on **either** platform, then it survives -- and given a
  one-term dictionary `Bundesverfassungsgericht`, the term is not deleted from the sentence.
- Given a transcript carrying a leaked hint fragment, when the chain runs on either platform, then
  fragments are stripped **before** the echo/blocklist verdict, and the same input yields the same
  verdict on both -- asserted against one fixture read by a Rust test, with the Android side asserted
  to delegate to the JNI bridge rather than to a Kotlin re-implementation.
- Given a raw transcript ending in a stockphrase ghost, when it is processed on either platform, then
  the ghost is stripped and the remainder survives (Desktop no longer drops the whole transcript); and
  given a ghost the LLM rationalises during cleanup, then it is stripped after cleanup on **both**
  platforms.
- Given a Cleanup Instruction is set (preset "Technical") and identical audio, when Android transcribes,
  then Whisper is conditioned with `advanced.sttPrompt*` -- not with the cleanup instruction -- and
  `customPrompt` appears in no STT request on either platform.
- Given `advanced.testProviderLlm = "empty"`, when a dictation runs, then on **both** platforms nothing
  is pasted, **no history row is written**, the raw transcript is in the clipboard, the cause is named,
  and the log names the scenario. The same holds for `"truncated"`.
- Given `advanced.testProviderLlm = "malformed"` and a dictation shorter than 400 bytes, when cleanup
  runs on Android, then the provider ladder fires (`deepseek -> openai -> openrouter`, Groq never a
  candidate) exactly as on the chunked path and as Desktop does.
- Given `advanced.testProviderStt = "empty"`, when STT runs on Android, then the failure is classified
  non-retryable, exactly one attempt is made, and no 2 s/5 s backoff is burned.
- Given the accessibility service is connected but no editable field is focused, when delivery runs,
  then no success check appears and the shipped `"Copied: ..."` toast fires; and given cleanup failed,
  then no DONE flash occurs at all -- both decided by pure functions a JVM test drives, and no new
  `RecordingState` value exists.
- Given the clipboard write throws on Android, when delivery runs, then the exception is caught,
  `pasteErrorCount` is incremented, and no success is shown; and given `set_clipboard` fails on Desktop
  on a run with no cleanup degrade, then the shipped `ClipboardWriteFailed` cause fires and the pill
  stops claiming "In Clipboard" -- while a *focus*-only failure keeps today's wording.
- Given a mini-tap, a silent capture, a blank transcript or a hallucination verdict on Android, when
  the run ends, then no toast is shown (Desktop parity) -- while `"No audio recorded"`,
  `"No API keys configured..."` and `"Paste blocked -- banking app active."` still are.
- Given a blocklisted app in the foreground, when a dictation completes on Android, then neither a
  `history.db` row nor a Turso push exists for it, and the paste is still blocked.
- Given a stored `sttProvider = "local"` and live preview enabled on Android, when the user pauses,
  then no delta WAV is sent -- with or without a Groq key -- asserted at both the install and the flush
  decision.
- Given `sttProvider = "local"` with any cloud `llmProvider`, when a dictation runs via the hotkey, via
  the React in-app button, and on Android, then all three produce the identical raw-text outcome with no
  network cleanup call; and given `llmProvider = "local"` where no local LLM exists, then the result is
  no cleanup, never a silent DeepSeek call.
- Given a desktop audio frame whose RMS is below `energy_floor`, when the VAD processes it, then the
  Silero engine **is** invoked with that frame and the frame is **still** classified Silence --
  asserted with a counting fake predictor, with `test_energy_gate_suppresses_very_quiet_signal` still
  green.
- Given N = `requiredSilentFrames` consecutive non-speech frames after speech on Android, when the
  N-th arrives, then auto-stop has **not** fired; and when the (N+1)-th arrives, then it has --
  asserted by a JVM test against a pure companion function and by a fixture that pins the trigger
  edge on both sides.
- Given the Android VAD source, when the dual threshold 0.5/0.35 is looked for, then the code states
  why it is absent and names where the decision lives -- and no ONNX dependency and no `Mode` change
  was introduced.
- Given every new or reshaped guard, vector and predicate, when it is inverted at writing time, then it
  goes RED, and the evidence is recorded in `gate4-evidence/13-2/` in the 13-1b shape.
- Given the whole change, when `Adr0017BoundaryGuardTest` runs, then it is green and no Kotlin twin of
  an STT guard exists.

## Implementation Notes

**Route chosen for the JNI hint (Group 1, task 3): rebuild inside the JNI, no arity
change.** `nativeTranscribe` now computes the guard hint itself with
`stt::stt_hint_text(&lang, custom_opt)` — the same function `pipeline.rs` uses — from
two arguments that already cross the boundary. Reason: `#[no_mangle]` exports the short
name, so an arity change misbinds a stale `.so` *silently*; rebuilding costs one line
and removes that class entirely. Group 2 makes this exact: `custom_prompt` now carries
`advanced.sttPrompt*` rather than the LLM cleanup instruction, so the two sides compute
the identical string from identical inputs. The JNI parameter list is unchanged at 8.

**A new native symbol was added anyway: `nativeStripStockphraseGhosts`.** B3's second
half needs `strip_stockphrase_ghosts` alone; `nativeStripPromptFragments` also applies
the fragment strip, which the desktop does *not* do after cleanup, so reusing it would
have bought parity with a behaviour difference. A missing new symbol throws
`UnsatisfiedLinkError` — loud, unlike a silent misbind — but the `.so` must still be
rebuilt: **`scripts/android-install-debug.sh <ip:port> --full`.**

**The guard chain's inner order is fragment strip → ghost strip → verdict, not the
order the task list named.** Measured, not preferred: the German built-in hint contains
the `STOCKPHRASE_BLOCKLIST` entry "Groß- und Kleinschreibung". Ghost-stripping first
mutilates a leaked hint before `strip_prompt_fragments` can recognise it, and
`"<DE hint> Danke"` comes out as `"Korrekte Satzzeichen und Interpunktion. Danke"` and
is dropped as an echo — precisely the outcome D-M9 exists to remove, and precisely what
the I/O matrix row promises ("'Danke' survives on both"). Fragment strip first yields
`"Danke"` and costs the ghost row nothing, because a trailing `"… Klinge"` matches no
hint fragment. Both orders are pinned by `GUARD-ORDER-001`, which records the wrong
order's output so it fails loudly. Recorded in `docs/backlog.md` as a FOUND note.

**"No history row" (D2) is implemented as ADR-0016 row D2 words it: `kein „"-History-
Eintrag`.** The empty and truncated answers now raise, so `finalText` becomes the raw
transcript and a row *is* written — exactly as Desktop writes one on its degrade path
(`pipeline.rs` history block, `cleaned_text == raw_text`). What can no longer be written
is the empty answer. Group 3's third task is implemented literally as a hard guard: a
blank `deliveredText` reaches neither `saveToHistory`, nor the clipboard, nor the paste,
and ends the run silently.

**`is_offline` was removed rather than kept as a second name.** The spec asked for it to
be "expressed through" the new predicate; a one-line delegation would have been a second
name for one rule in a story about three names for one rule. There is
`offline_rule(stt, llm)`, its availability-parameterised form `offline_rule_with(…, bool)`
(so the Linux gate can reach the half of the matrix where D-M21 lives), and
`config_skips_cleanup(&AppConfig)` for the two config-shaped callers.

**Two shipped tests were rewritten because they pinned the defects this story closes**,
per the spec's instruction: `spec_non_degraded_run_never_gains_a_cause`
(`terminal_degrade_cause`) and `test_offline_flag_false_when_both_local`, now
`test_offline_flag_follows_local_cleanup_availability_when_both_local`. Neither was
deleted.

**One shipped test needed a mechanical repair, not a verdict change.**
`TestProviderScenarioTest.localSttBranchIsTakenOnlyWhileTheTestProviderIsOff` assumed
the *first* `config.sttProvider == "local"` in the file is the local-STT branch. E1 adds
a second, legitimate occurrence (the flush-time re-check), so it now asserts that *some*
occurrence is the gated one — the claim the row actually makes.

**Deliberately unchanged, though nearby:** the "no accessibility service connected"
ending still flashes DONE and still shows `"Copied: …"`. It is clipboard delivery
working as intended, not a failure, no audit row re-opens it, and Desktop's twin
(`DoneClipboard`) is likewise a terminal state rather than an error. Only an *attempted*
paste that missed (D4), a cleanup failure (D5) and a failed clipboard write (D6) lose
the checkmark.

**`KlarvoApi.cleanupLocal` is no longer called from the overlay path** and is left in
place. With `LOCAL_CLEANUP_AVAILABLE = false` the predicate answers "no cleanup" for
every `llmProvider = "local"` config, so the branch was unreachable — and it was already
inert (D-H18: it threw and the catch degraded to the raw transcript, the same output the
predicate now produces directly). Removing the function is 13-3's gate work, not this
story's.

**Three literal copies became one.** `stt::STT_HINT_DE/EN/AUTO` +
`stt::DEFAULT_STT_HINTS`: the same three conditioning prompts existed in
`build_stt_prompt_with_hint` (with a trailing space, load-bearing as the separator), in
`pipeline::DEFAULT_STT_HINTS` (without one) and a third time inline in
`stop_and_process_pipeline`'s `stt_hint_text`. `strip_prompt_fragments` only works while
they are byte-identical, and nothing enforced that. The separator moved into the builder
(B4) and the literals became one const each.

## Spec Change Log

- **Group 1, task 1 — the chain's inner order was reversed against the task text**
  (fragment strip before ghost strip). The task named ghost-strip-first; that order
  measurably breaks the I/O matrix's own D-M9 row on the German hint. Evidence and
  reasoning in Implementation Notes, in `pipeline::guard_transcript`'s doc comment, in
  `GUARD-ORDER-001`, and in `docs/backlog.md`. Both AC sentences ("fragments are
  stripped before the echo/blocklist verdict", "the ghost is stripped and the remainder
  survives") hold in the order shipped.
- **Group 1, task 4 — a sibling extern rather than a reuse.** The task allowed either;
  `nativeStripStockphraseGhosts` is exact desktop parity, `nativeStripPromptFragments`
  would have added a fragment strip the desktop does not run after cleanup.
- **Two fixture vectors were added that the task list did not name**
  (`GUARD-ECHO-DROP-001`, `GUARD-BLOCKLIST-DROP-001`). An inversion of
  `guard_transcript_for_jni` came back GREEN: every planned guard vector asserts
  survival, so the drop arm was never exercised. See `code-inversion-report.md`.
- **Group 3, task 3 read as ADR-0016's `kein „"-History-Eintrag`**, not as "no history
  row at all" — see Implementation Notes. The Desktop twin writes a row on its degrade
  path, so "no row" would have been a new divergence.

## Review Triage Log

## Design Notes

### Delivery-state contract (which state each terminal path enters, and its source)

This story changes *which* state is entered, never what a state looks like. No new state, no new
wording, no new drawing. Confirmed by Andi 2026-09-21 for D4/D5.

| Path | Android terminal state today | After | Source |
|---|---|---|---|
| success | DONE flash 800 ms -> IDLE | unchanged | shipped |
| cleanup failed (D5) | DONE flash + degrade toast | IDLE + the same degrade toast | shipped IDLE; the DONE-flash comment already promises this |
| paste failed, clipboard holds text (D4) | DONE flash, no toast | IDLE + shipped `"Copied: $preview"` toast (`KOS:2358`) | shipped toast, already used when accessibility is not connected |
| empty / truncated LLM answer (D2, D3) | pasted + stored | the shipped cleanup-failure path: clipboard + named cause | shipped (7-10, `CLEANUP_FAILED_CLIPBOARD_MSG`) |
| clipboard throws (D6) | crash | IDLE, no success, `pasteErrorCount++` | shipped IDLE |
| nothing recognized (D11) | toast | silent, IDLE | Desktop `PipelineEvent::idle()` |
| Desktop clipboard write failed (D6) | pill `"In Clipboard"` | shipped `ClipboardWriteFailed` cause -> `"TEXT LOST"` card | shipped (7-10 degrade branch), reused verbatim |

Recorded as **canon gap, shipped precedent**: the Android degraded-terminal states have no design-canon
entry; each reuses a component that exists at `baseline_revision` and is already in use on another
surface. Desktop's `DoneClipboard`/`DoneDegraded` (`native_pill.rs:132-134`) is the conceptual twin but
is *not* ported -- porting it would mean a new Android bubble state, which this story does not design.

### B6 -- what Andi decided at the intent gate (2026-09-21), and what it costs

The first planning run halted here. ADR-0016 Amendment 4 closes B6 with *"Hysterese 0,5/0,35 +
Hangover-Frame in Kotlin"*, size **S**, direction Desktop (`ADR-0016:289`), limited by `ADR-0016:260`
to *"Nur die drei gemessenen Deltas"*. Two of those three premises did not survive contact with the
tree, and Andi ruled on both.

**1. The dual threshold (D-M10) is cut to its own story.** The shipped
`com.github.gkonovalov.android-vad:silero:2.0.10` exposes only `isSpeech(...) -> boolean`;
`extractResult`, `threshold()` and `predict(float[])` are private, and `Mode.NORMAL` hardcodes `0.5f`
(measured with `javap -public` / `javap -c -p` against the AAR in the Gradle cache). Without the raw
probability there is no offset threshold at 0.35. The only route that delivers it is bypassing the
library for `ai.onnxruntime.OrtSession` against the AAR's own `silero_vad.onnx` -- an L-class
dependency change. **Andi chose option (a):** 13-2 builds the hangover frame only and documents the
dual threshold as a carve-out; the OrtSession route is a story candidate recorded in `docs/backlog.md`
("DECIDED 2026-09-21"), **not cut and not released**. This story must not build it.

**2. D-L19's direction is reversed: Desktop adapts to Android.** The audit (`D-L19`, LOW) reads
"Android calls `isSpeech` on every frame" as Android drift. The tree says the opposite is the correct
behaviour: `KlarvoAudioRecorder`'s KDoc states Silero is stateful and that skipping frames corrupts
its sliding window, and the crate confirms it -- `voice_activity_detector` 0.2.1's
`predict(&mut self)` mutates the LSTM hidden state. Desktop freezes that state during every sub-floor
frame. **Andi reversed the direction:** the desktop VAD now calls the engine on every frame; Android
stays exactly as it is.

**Why this is verdict-preserving, and what it actually costs.** `advance_state` already takes
`energy_ok` as a separate parameter and ANDs it into both thresholds, so a sub-floor frame is Silence
whatever probability Silero returns -- the per-frame classification is provably identical before and
after. What changes is the *hidden-state trajectory*, so probabilities on subsequent loud frames
differ. That is the point of the row, and it is also the honest risk: this is a real behavioural
change to speech detection that no Linux test can validate against real audio. Two costs to record
rather than discover: (a) inference now runs on every frame for all three desktop VAD instances, and
the ONNX `Session` is a process-wide `LazyLock<Arc<Mutex<Session>>>` shared by all of them, so lock
traffic roughly doubles when the voice-command engine runs alongside a recording; (b)
`test_silence_stays_silence` moves from a deterministic `prob = 0.0` path to real model output on
digital silence -- expected to stay green, but it becomes model-dependent, and if it is not, that is
a finding to report, not a test to soften.

### Why D-L21 stops at the auto-stop counter

`KlarvoAudioRecorder` has **two** silence counters with the identical `>=` off-by-one: the auto-stop
one-shot (:588-590) and the repeatable live-preview pause edge (:608-610, driven by
`previewPauseSilenceSecs`). Audit row D-L21 cites **only** `KAR:588-590`, and `ADR-0016:260` limits
B6 to *"Nur die drei gemessenen Deltas"*, keeping full state-machine parity (L) explicitly closed.
Widening to the preview counter would add a fourth, unmeasured delta to a row the human just narrowed.
So the preview edge keeps its one-frame difference and is recorded in frontmatter `deferred` instead
of fixed. If the reviewer reads B6 the other way, it is a one-line change on top of the same
extraction -- the reading is written here rather than buried. (Note the asymmetry is not new and is
not made worse: today both Android counters sit at N and both desktop paths at N+1; after this story
the auto-stop pair agrees and the preview pair still differs by one frame.)

### Why D11 stops at four of five toasts

ADR-0016 Amendment 4 names the subject as *"kein Toast bei 'nichts erkannt'"*, direction Desktop; the
epic repeats *"no toast on 'nothing recognized'"*. Audit row D-M14 lists five Android sites. Four of
them are recognition results (mini-tap, pre-STT silence, blank transcript, hallucination verdict) and
have a mute desktop twin. `"No audio recorded"` (`KOS:1919`) fires on an **empty capture buffer** -- a
recorder/plumbing fault, not a recognition result. Removing it would create exactly the silent loss this
story exists to remove, so it stays. Andi confirmed this reading on 2026-09-21.

### Why the guard chain is extracted rather than mirrored

The three divergences B2 closes (echo input, strip input, order) are not three bugs; they are one:
`groq_jni::nativeTranscribe` re-implements the desktop chain inline instead of calling it. `pipeline`
already has the decision in one pure function (`post_stt_skip`), so the work is to widen that seam to
the whole chain and let the JNI call it. That also fixes the testability problem: today everything
inside `nativeTranscribe` is `#[cfg(target_os = "android")]`, so `cargo test --lib` cannot witness any
of it. `select_stt_provider` (`groq_jni.rs:93-110`) is the shipped precedent for a plain-Rust function
the host gate can reach. Without this extraction the whole of B2/B3 ships agent-only.

### Why the post-cleanup ghost strip on Android goes through the JNI

`Adr0017BoundaryGuardTest` pins four *shapes* and would not catch a Kotlin ghost-strip written under a
new name -- but ADR-0017 and project-context both forbid it, and `GroqSttBridge` already declares
`nativeStripPromptFragments`, which applies `strip_stockphrase_ghosts` to its output. Reuse beats a
twin: one implementation, one fixture, no drift to re-audit next quarter.

### Why one offline predicate, and where it lives

`effective_llm_provider_name` (13-1b) has the right shape (`&AppConfig -> String`, pure, twin-documented)
but folds only the test-provider override; what is missing is *platform availability*. One predicate
beside it, read by all three current definitions, is an extraction next to an existing seam -- not a new
construct. The `_ => deepseek` fall-through must stop swallowing an unavailable `"local"`: under G2a the
correct degrade is **no cleanup**, the shipped `OfflineRaw` outcome, so no new user-facing wording is
needed.

## Verification

**Commands:**
- `cd src-tauri && cargo test --lib` -- expected green with no API keys. **Measure the baseline at
  `4e4bc00b4ef28a75370acdb44e905e5699ec9388` first** and apply the baseline exception before treating
  any red as this story's. 13-1b measured **749 passing** at the end of its run, and no source file
  changed between that tip and this baseline, so 749 is the number to expect.
- Device-free JVM gate (`scripts/android-smoke.sh` cannot be used here -- it hard-fails on
  "Kein Geraet gefunden" before reaching the gate):
  ```
  export JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64
  export ANDROID_HOME=/home/andyon2/workspace/tools/android-sdk
  rm -f  src-tauri/gen/android/app/src/main/java/com/klarvo/voice/*.kt
  cp android/kotlin-src/com/klarvo/voice/*.kt  src-tauri/gen/android/app/src/main/java/com/klarvo/voice/
  rm -f  src-tauri/gen/android/app/src/test/java/com/klarvo/voice/*.kt
  cp android/kotlin-test/com/klarvo/voice/*.kt src-tauri/gen/android/app/src/test/java/com/klarvo/voice/
  # ensure testImplementation("org.json:json:20231013") in src-tauri/gen/android/app/build.gradle.kts
  cd src-tauri/gen/android && ./gradlew :app:testUniversalDebugUnitTest --rerun-tasks
  ```
  `--rerun-tasks` is **mandatory** after any fixture edit or gradle reports a stale green. Expected:
  all suites green including `Adr0017BoundaryGuardTest`. 13-1b measured **25 suites / 217 tests**.
- `npm run build` -- TS strict, only if a `src/` file is touched (no task above requires one).
- Inversion evidence -- every new guard, vector, sentinel and predicate is broken deliberately, run,
  shown RED, and reverted. Output to
  `_bmad-output/implementation-artifacts/gate4-evidence/13-2/{code-inversions.json,code-inversion-report.md,verdict.md}`
  in the 13-1b schema (per-entry `inversion`, `file`, `filter`/`expected_red_test`, `RED`, `evidence`).
  The tree is re-confirmed green after the last revert and `git status` is clean.

**What no unattended run may claim.** `cargo test --lib` and the JVM gate decide logic, wiring and
structure on Linux only. They do not decide: the real Whisper conditioning result for real audio (B4),
the banking guard on a real foreground app (B1), the clipboard/accessibility behaviour on a real device
(D4, D6-Android), the real-audio effect of the VAD change (D-L19/D-L21), the bubble's appearance, or
anything on Windows. Every count reported must name what it did not exercise.

**H+ reproduction path (named before build, per Verifikations-Symmetrie):**
- **D2, D3, D10, D9 -- reproducible by Andi, no computer needed.** Settings -> Advanced -> Expert mode ->
  System -> `Test provider (LLM)` = `empty` / `truncated` / `malformed`, or `Test provider (STT)` =
  `empty`; press the Advanced `Save` (one button, both chains, since 13-1b); dictate a short sentence.
  Keep it under 400 characters -- above that, cleanup chunks and the canned answer is returned per
  chunk, which changes the `malformed` verdict on Android. Which scenario fired is in `klarvo.log` on
  both platforms. A downloaded local Whisper model can silently rescue a test STT failure via
  `pipeline::try_local_whisper_fallback` -- check that first if an STT scenario appears not to fire.
- **B2 -- Xiaomi:** dictionary `Klarvo, Kubernetes`, say "Klarvo und Kubernetes." -> the sentence
  survives (today it is discarded).
- **B3 -- Windows + Xiaomi:** dictate a known ghost phrase at the end; it is stripped on both devices.
- **B4 -- Xiaomi:** set preset "Technical", dictate the same audio twice around the change; the raw
  transcript's punctuation/casing changes.
- **D4, D5, D11 -- Xiaomi:** paste into a non-editable surface (D4), force a cleanup failure with the
  test provider (D5), mini-tap the bubble (D11).
- **E1 -- Windows:** store `sttProvider = local`, enable live preview, pause; `klarvo.log` shows no
  upload.
- **E2 -- Windows in-app button:** choose Offline, dictate with filler words; the "aehm" stay.
- **D-L19 / D-L21 -- see Design Notes.** Both are timing changes with no new user-visible control.
  D-L21 is reproducible on the Xiaomi (auto-stop fires ~32 ms later; Andi will not perceive this, so
  the honest gate is the JVM test plus the fixture, and the device check is only "auto-stop still
  works at all"). D-L19 is **Weg 2, agent-verified only**: the per-frame verdict is provably
  unchanged, so there is nothing for Andi to observe; the claim that Silero's state improves on later
  frames is not measurable on his machine.
- **D6 -- Weg 2, agent-verified only, written down rather than implied.** Neither a Desktop
  `set_clipboard` failure nor an Android `setPrimaryClip` throw is producible on Andi's machines, and
  the test provider does not inject them. ADR-0016 Amendment 4 already records this downgrade.
- **B1-Android -- Xiaomi:** dictate into a blocklisted app; afterwards History shows no entry.
- **Android install:** if the JNI signature changes (Group 1), `scripts/android-install-debug.sh
  <ip:port> --full` is **mandatory** -- `#[no_mangle]` exports the short name, so a plain install
  misbinds a stale `.so` silently instead of throwing.
- **Surface DoD (G-D):** the rows above that change what Andi sees end with a Windows release build via
  `scripts/windows-build.sh` (run in the background) and a fresh APK on the Xiaomi.

## Auto Run Result

Status: ready-for-dev
Blocking condition: none

### 2026-09-21 -- planning run 2 (resume after the intent gate; halt after planning requested)

This is the resume of the run that halted `blocked / intent gap` on B6. The blocked spec
`spec-13-2-parity-sweep-guards-and-silent-loss.md` is left at `status: blocked` as the historical
record; this `-2` file is the live spec, per step-01's Route rule (an existing spec that is not
`draft` gets a `-2` suffix) and the precedent this project already set with
`spec-13-1-debug-test-provider-both-twins-2.md`.

All **15** decision-sheet rows ADR-0016 Amendment 4 assigns to this story are now planned; the count
was re-verified against `ADR-0016:369-371` and `epics.md:3611-3614`. 31 tasks, 18 acceptance criteria.

**What the human decided at the gate (2026-09-21), now part of the intent:**

- **B6 / D-M10 -- option (a).** Build only the hangover frame; document the 0.5/0.35 dual threshold
  on Android as a carve-out. The `ai.onnxruntime.OrtSession` route is a separate L story, recorded
  in `docs/backlog.md` ("DECIDED 2026-09-21") and **not** built here.
- **B6 / D-L19 -- direction reversed.** Desktop adapts to Android: the desktop VAD calls the Silero
  engine on every frame. Android is unchanged. This moved from the first run's frontmatter
  `deferred` into Group 9 as executable work.
- **D11 -- four of five toasts**, `"No audio recorded"` stays. Confirmed.
- **D4/D5 -- no new Android bubble state**; shipped IDLE plus the shipped `"Copied: ..."` toast.
  Confirmed.

### Decisions

- **Resumed onto a `-2` spec rather than re-opening the blocked file.** Alternative: flip the
  blocked spec back to `draft` in place. Chosen because step-01 routes a `blocked` spec to HALT and
  the sanctioned resume path is a new id-suffixed spec, which also preserves the first run's
  evidence trail unaltered.
- **`baseline_revision` moved to `4e4bc00`** (was `3f87bef`). `git diff --stat 3f87bef 4e4bc00`
  touches only the run ledger, the old spec and `docs/backlog.md` -- **no source file** -- so every
  Code Map anchor carried over from the first run is valid unchanged. Anchors were additionally
  spot-verified against the tree (`groq_jni.rs:300-335`, `pipeline.rs:681-700`, `:1423-1452`,
  `vad/mod.rs:40-100`, `:280-390`, `KlarvoAudioRecorder.kt:150-190`, `:240-265`, `:520-632`).
- **The desktop D-L19 change is shaped as a closure seam mirroring Kotlin's `vadGateDecision`.**
  Alternative: a trait object over the engine, or no seam at all. Chosen because `engine` is the
  concrete `VoiceActivityDetector` (crate 0.2.1) with `predict(&mut self)`, so a closure parameter
  is the only cheap observable -- and because copying the Kotlin seam literally is what "Desktop
  adapts to Android" means here.
- **D-L21 is scoped to the auto-stop counter only**; the identically-shaped live-preview pause
  counter (`KlarvoAudioRecorder.kt:608-610`) is recorded in `deferred` instead. Alternative: fix
  both. Chosen because audit row D-L21 cites only `KAR:588-590` and `ADR-0016:260` narrows B6 to
  "nur die drei gemessenen Deltas"; widening would add a fourth, unmeasured delta to a row the
  human had just narrowed. The reading is written into Design Notes so a reviewer can overturn it
  with one line.
- **The D-M10 carve-out is documented at the code site**, in the `KlarvoAudioRecorder` KDoc beside
  the VAD constants and the `VadSilero` construction, citing `docs/backlog.md` and ADR-0016 B6.
  Alternative: a new ADR amendment. Chosen because the backlog entry already carries the decision
  and its evidence, and a build story appending to an ADR would re-open a verdict this run is told
  not to re-open. Flagged for the conductor: if the ADR's B6 row should record that it shipped half,
  that is an amendment with its own commit, not this story's work.
- **The `prob >= 0.5` (Rust) vs `prob > 0.5` (Android AAR) hair-width divergence is folded into the
  D-M10 carve-out documentation** rather than carried as a separate deferred item. The first run
  recorded it as "a divergence no audit row records"; that is wrong -- audit row D-M10 states
  `speech iff prob > 0.5` explicitly (`docs/cross-platform-drift-audit-2026-09-16.md:74`).

### Costs and risks recorded rather than discovered

- Calling Silero on every desktop frame affects **three** VAD instances, not one: auto-stop
  (`audio/mod.rs:1078`), live-preview flush (`audio/mod.rs:1186`) and the voice-command engine
  (`voice_command/mod.rs:161`). The ONNX `Session` is a process-wide `LazyLock<Arc<Mutex<Session>>>`
  shared by all of them, so lock traffic roughly doubles when a voice-command engine runs alongside
  a recording.
- `test_silence_stays_silence` (`vad/mod.rs:456`) moves from a deterministic `prob = 0.0` path to
  real ONNX output on digital-silence frames. Expected green, now model-dependent. The spec instructs
  the implementer to report a red there rather than weaken the assertion.
- The five `spec_vad_autostop_*` tests in `audio/mod.rs` use `make_fast_vad()` with
  `energy_floor: 0.0` and therefore already take the predict path on every frame -- no change.
- B6 carries **no H+** marker (`ADR-0016:289`), so the whole VAD group is Weg 2, agent-verified only.
  The spec does not promise Andi a device reproduction for it.

### What this run does NOT claim

No code was changed; the working tree was clean at `4e4bc00` before this run and the only new file is
this spec. Nothing was built and no test was run -- not `cargo test --lib`, not the JVM gate. The Code
Map was re-measured by reading the tree, not by executing it. The 15 planned rows are planned, not
verified. The claim that the D-L19 change is verdict-preserving is a reading of
`advance_state`'s existing `energy_ok` AND (`vad/mod.rs:324-325`), not a measured result.
