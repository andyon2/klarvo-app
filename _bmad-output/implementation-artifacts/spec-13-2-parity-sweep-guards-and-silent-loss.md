---
title: '13-2 Parity sweep: guards and silent loss'
type: 'feature'
created: '2026-09-21'
status: 'blocked'
baseline_revision: '3f87beffb0023f0024a8f61189d7be69eb8bc46d'
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
  - '{project-root}/_bmad-output/implementation-artifacts/spec-13-1b-test-provider-operability.md'
  - '{project-root}/docs/adr/0016-android-path-parity-strategy.md'
  - '{project-root}/docs/adr/0017-shared-core-stt-path.md'
  - '{project-root}/docs/cross-platform-drift-audit-2026-09-16.md'
  - '{project-root}/test-fixtures/README.md'
warnings: ['multiple-goals', 'oversized']
deferred:
  - summary: 'Desktop skips the Silero engine entirely on sub-floor frames; Android calls it on every frame by documented decision'
    evidence: '`vad/mod.rs::process_frame` calls `self.engine.predict` only inside the `energy_ok` arm, so a quiet passage feeds the stateful Silero model a truncated frame sequence. `KlarvoAudioRecorder` KDoc states the opposite is deliberate ("Silero is stateful; skipping frames corrupts its sliding window"). Audit row D-L19 assumes Android is the drifting side; the evidence points the other way. Not fixed here — see the B6 blocking question.'
    location: 'src-tauri/src/vad/mod.rs::process_frame'
    severity: 'medium'
  - summary: 'Rust VAD onset compares `prob >= onset_threshold`, the shipped Silero AAR compares `prob > threshold`'
    evidence: '`vad/mod.rs::advance_state` uses `>=`; `javap -c -p com.konovalov.vad.silero.VadSilero#predict` shows `fcmpl / ifle`, i.e. strictly greater. A hair-width divergence at exactly 0.5 that no audit row records. Out of scope, recorded so it is not rediscovered.'
    location: 'src-tauri/src/vad/mod.rs::advance_state'
    severity: 'low'
  - summary: 'Three stale source claims in the guard path that this story must not propagate'
    evidence: '(a) `groq_jni.rs` parity comment above the guard chain cites `pipeline.rs:501, 1032`, both dead lines; its test comment repeats the claim. (b) `pipeline::is_prompt_echo` doc says "≥60%" while the code uses 0.7. (c) `stt/groq_jni.rs::tests` is android-gated and has never executed (already in `docs/backlog.md`), so it cannot witness any guard change. Pre-existing.'
    location: 'src-tauri/src/stt/groq_jni.rs'
    severity: 'low'
---

<intent-contract>

## Intent

**Problem:** A second cross-platform drift audit found that Android and Desktop disagree on every
guard and every failure path in the dictation pipeline. On Android a dictionary term of ≥10 bytes is
deleted from *every* transcript, a short sentence made of dictionary words is discarded as a
"prompt echo", the guards run in inverted order, the Whisper conditioning prompt is the *LLM cleanup
instruction*, an empty or truncated LLM answer is pasted and stored as the dictation, a malformed
answer on a short dictation gets no provider fallback, an empty STT result burns three Groq calls,
the transcript reaches `history.db` and Turso *before* the banking guard blocks the paste, a failed
paste shows the success check, a failed cleanup shows the success check, and "Offline" still uploads
audio. Desktop has its own half of the same class: a raw transcript carrying a trailing stockphrase
ghost is dropped whole, the in-app record button obeys a different "offline" rule than the hotkey,
and the pill says "In Clipboard" for text that is in neither the field nor the clipboard.

**Approach:** Close the fifteen actionable audit rows ADR-0016 Amendment 4 assigns to this story, by
making the *shared Rust core* the single source of the guard chain (the JNI calls the same functions
in the same order, with the same inputs, as the desktop pipeline), by installing **one** offline
predicate that all three current definitions read, and by making every Android failure path end in
the same observable state its desktop twin already ends in — reusing shipped states and shipped
wording, never inventing one. Every guard and core-output change is pinned by a fixture read on both
sides, and every new guard is inverted to RED at writing time.

## Boundaries & Constraints

**Always:**
- **The row ids are the contract.** Every task cites its decision-sheet row (B2, B3, …) and its audit
  row (`D-H5`, …). The binding verdict is ADR-0016 Amendment 4's row table; the audit supplies the
  code evidence only. Nothing is re-measured and no verdict is re-opened.
- **ADR-0017 holds: STT request and guard logic live only in Rust.** Android consumes them over the
  JNI. `GroqSttBridge` already declares `nativeIsHallucination`, `nativeIsPromptEcho` and
  `nativeStripPromptFragments`; only the first is called today. Close a guard gap by *calling the
  existing bridge*, never by growing a Kotlin twin. `Adr0017BoundaryGuardTest` stays green.
- **Platform reach is stated per task** (project-context twin rule): shared Rust core → fix once;
  Rust↔Kotlin twin → fix twice; platform-gated → say so.
- **Guards get a fixture read by both sides** (`test-fixtures/`), in the house schema (`PINS:` /
  `DOES NOT PIN:` prose, throwing lookup by id). A fixture with one reader is a written record, not a
  lock. Where ADR-0017 makes the Kotlin column vacuous, the vector says so by construction, following
  the `surface: "stt"` precedent in `test-provider-scenario-vectors.json`.
- **Every new or reshaped guard is inverted at writing time** — real edit, real run, real revert —
  and the evidence lands in `_bmad-output/implementation-artifacts/gate4-evidence/13-2/` in the
  13-1b shape (`code-inversions.json`, `code-inversion-report.md`, `verdict.md`).
- **Terminal states reuse shipped states and shipped wording.** This story designs no new pill state,
  no new bubble drawing, no new toast text. Where a failure must become visible, it enters a state
  that already exists at `baseline_revision` and is already in use on another surface.
- The test provider from 13-1/13-1b is the reproduction vehicle: `advanced.testProviderLlm` /
  `advanced.testProviderStt` with scenarios `empty`, `truncated`, `malformed`, `http429`, `http5xx`,
  `transport`. Its canned wire tables and all 14 scenario vectors stay byte-identical.

**Never:**
- Do not touch the Desktop banking blocklist (that is **13-6**) — only the Android ordering half of
  `D-H3` belongs here.
- Do not build, hide or gate any control (that is **13-3**). This story changes behaviour behind
  controls, not the controls themselves. A *stored* `local` value must still be honoured, because
  13-3 has not landed and a hidden control leaves its value behind.
- Do not touch the license gate or the free-tier definition (**13-4**), nor Turso sync semantics
  beyond the ordering fix (**13-5**).
- Do not "fix" the pre-existing defects listed in frontmatter `deferred`.
- Groq is never a cleanup fallback (Epic 12 FR2). No auto-send on Android. The fallback ladder's
  membership (`deepseek → openai → openrouter`) is unchanged.
- Never add an unconditional dependency or `use` that breaks the Android or Linux build.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|---|---|---|---|
| **B2** echo guard input | Dictionary `Klarvo, Kubernetes`; user says "Klarvo und Kubernetes." | Both platforms: overlap computed against the **hint only** → 0.33 → transcript survives and is pasted | No error expected |
| **B2** fragment strip input | One-term dictionary `Bundesverfassungsgericht`; transcript "Das Bundesverfassungsgericht hat entschieden." | Both platforms: the term is **not** removed (the dictionary is not part of the strip input) | No error expected |
| **B2/D-M9** guard order | Any transcript with a leaked hint fragment ("<DE hint> Danke") | Both platforms: **strip first, then echo/blocklist** → "Danke" survives on both | No error expected |
| **B3** ghost before guard | Raw transcript ends in a stockphrase ghost ("… Klinge") | Both platforms: ghost stripped **before** the hallucination guard → the rest survives (Desktop stops dropping the whole transcript) | No error expected |
| **B3** ghost after cleanup | LLM rationalises a ghost into a fluent stockphrase | Both platforms: ghost stripped **after** cleanup → ghost gone on Android too | No error expected |
| **B4** conditioning prompt | Cleanup Instruction set (preset "Technical"), identical audio | Android conditions Whisper with `advanced.sttPrompt{De,En,Auto}` like Desktop; `customPrompt` reaches the **LLM only** | Missing `sttPrompt*` → the built-in language hint, as Desktop |
| **D2** empty LLM content | Test provider `testProviderLlm = empty` | Both: no paste, **no history row**, raw transcript to clipboard + named cause | Mapper raises the twin of `LlmError::ResponseFormat`; non-retryable |
| **D3** truncated answer | `testProviderLlm = truncated` (`finish_reason == "length"`) | Both: not pasted; raw transcript to clipboard + named cause | Mapper raises the twin of `LlmError::OutputTruncated`; non-retryable |
| **D10** malformed answer, short dictation | `testProviderLlm = malformed`, transcript < `CHUNK_THRESHOLD` (400 B) | Android fires the provider ladder as Desktop does (`deepseek → openai → openrouter`) | Undecodable body is classified retryable on **both** paths, chunked and single-call |
| **D9** empty STT result | `testProviderStt = empty` | Android marks it **non-retryable** and reports at once; retry budget **1** as Desktop | New non-retryable sentinel from the JNI; no 2 s/5 s backoff burn |
| **D4** no focused field | Accessibility connected, no editable node focused | Android: **no success check**; the shipped "Copied: …" clipboard toast fires | Paste result is reported back, not discarded |
| **D5** cleanup failed | `llmCleanupFailed == true` | Android: **no DONE flash** — straight to IDLE, degrade toast carries the cause (the code's own comment already promises this) | No error expected |
| **D6** clipboard write throws (Android) | `setPrimaryClip` throws | Caught; no success check; `pasteErrorCount` incremented | Never an uncaught main-thread exception |
| **D6** clipboard write fails (Desktop, normal run) | `set_clipboard` returns `Err` with no cleanup degrade | The shipped `ClipboardWriteFailed` cause fires and the "TEXT LOST" card shows — the pill stops claiming "In Clipboard" | A *focus* failure (`ClipboardOnly`, `Ok`) keeps today's wording |
| **D11** nothing recognized | Mini-tap / silent capture / blank transcript / hallucination verdict | Android is **silent**, like Desktop (`PipelineEvent::idle()`) | `"No audio recorded"` (a capture fault, not a recognition result) is kept |
| **B1-Android** banking guard | Blocklisted app in the foreground | History row **and** Turso push happen only after a non-blocking verdict; a blocked dictation writes neither | Guard verdict is read on the main looper as today |
| **E1** preview flush offline | Stored `sttProvider = local`, live preview on | No delta WAV leaves the device — the flush is not installed and is re-checked at flush time, as Desktop | No network call at all, key present or not |
| **E2** one offline rule | `sttProvider = local` + any cloud `llmProvider`, hotkey / in-app button / Android | One predicate: local STT ⇒ local cleanup **or none**. Identical raw-text outcome on all three | `llmProvider = "local"` where no local LLM exists degrades to **no cleanup**, never to a silent network call |

</intent-contract>

## Code Map

Anchors are symbols; line numbers are navigation hints at `baseline_revision` only. Audit anchors are
stale — `pipeline.rs` ≈ +43, `stt/mod.rs` ≈ +33, `groq_jni.rs` ≈ +93 vs. the audit's tree, and 13-1
extracted both HTTP mappers.

### Rust — the guard chain (shared core, serves both platforms)

- `src-tauri/src/pipeline.rs`
  - `is_prompt_echo(transcription, stt_hint) -> bool` :424-509 — overlap **≥0.7** :481-490, `>30`-word
    bail :477, diversity branch :492. ⚠ its doc still says "≥60%" (deferred).
  - `strip_prompt_fragments(text, stt_hint) -> String` :535-593 — ≥10-byte rule :549/:555,
    case-insensitive substring removal :563-577, punctuation-token drop :585-592. `DEFAULT_STT_HINTS`
    :515.
  - `post_stt_skip(transcript, stt_hint) -> Option<PostSttSkip>` :690-698 and `enum PostSttSkip`
    :681-686 — **this is the seam**: the desktop guard decision already lives in one pure function.
  - Desktop chain: strip :1427-1433 (rationale comment :1423-1426) → `post_stt_skip` :1438 → skip
    terminal :1446-1450 (`ProcessOutcome::Stopped`, nothing pasted, nothing saved).
  - `stt_hint` selection from `advanced.stt_prompt_{de,en,auto}` :1794-1806; wire prompt via
    `stt::build_stt_prompt_with_hint` :1807-1811; **`stt_hint_text` = the hint only** :1814-1818.
  - `custom_prompt` :1162, :1865-1884 — consumed **only** at the LLM call sites :1505, :1542.
  - Ghost strip, desktop: `strip_stockphrase_ghosts` called **once**, after `sanitize_llm_output`
    :1617-1618, rationale :1614-1616.
  - `is_retryable_stt_error` :357-360 (`ResponseFormat` → non-retryable); non-retryable terminal
    :1396-1416.
  - Offline: `is_offline` :643-645, caller :1824; `select_llm_path` :713-721 → `LlmPath::OfflineRaw`
    :704; skip site :1459, :1469-1475. Local-LLM arm `#[cfg(target_os = "windows")]` :307-321 and the
    `_ => cleanup_provider_for("deepseek", …)` fall-through :322-323. `effective_llm_provider_name`
    :290-296 (13-1b) — the **neighbour** of the new predicate, not the predicate.
  - Live preview: `preview_flush_should_install` :2456-2458, install site :2627, flush-time recheck
    :2545-2553, `flush_preview_delta` :2524-2586.
  - Clipboard truth: `deliver_text` :2361-2402, `paste_error_count` bump :2011-2016, terminal emit
    :2195-2209 (`DoneClipboard` :2195-2207, plain `Done` :2208), **`terminal_degrade_cause`
    :2427-2436 — the `Some(_) if paste_failed` arm is the whole bug**. Pinned as intended by
    `spec_non_degraded_run_never_gains_a_cause` :5772-5778 (that test must be rewritten, not deleted).
- `src-tauri/src/stt/hallucination.rs` — `strip_stockphrase_ghosts` :181-~280; `is_hallucination`
  :314-374, stockphrase block **without a word-count gate** :324-345 (the `>8`-word gate only applies
  afterwards :347-352). Re-exported at `stt/mod.rs:30`.
- `src-tauri/src/stt/mod.rs` — `build_stt_prompt_with_hint` :121-146; `custom_hint` **replaces** the
  built-in hint :128-136; terms appended with **no separator** :139 (`format!("{hint}{terms}")`) —
  safe only because each built-in literal ends in a space :132-134. `map_transcription_http_response`
  :~395-465 with the three `ResponseFormat` messages :440-442, :450-452, :457-459.
  `WhisperStt::transcribe` :362-387 sends the whole multipart body before any 401.
  Host-reachable JNI selector tests `spec_android_select_stt_provider_*` :1403-1500; fixture loader
  `load_test_vectors` :1133, throwing lookup `test_vector` :1145.
- `src-tauri/src/stt/groq_jni.rs` — **the divergence site.** Current `nativeTranscribe` params
  :211-222: `wav_base64, api_key, language, dictionary_terms, custom_prompt, stt_model, temperature,
  test_provider_stt` (arity 8 since 13-1b; the `#[no_mangle]` stale-`.so` misbind warning is
  :195-198). Prompt built :266-268. Guard chain **inverted and mis-fed**: echo :307-311 with
  `hint = prompt` (the *full* prompt incl. dictionary and `customPrompt`) → strip :312 → ghost :314.
  Parity-claiming comment :302-306 (cites two dead desktop lines). Sentinels: `__ERROR_EMPTY_AUDIO__`
  :262/:320, `__ERROR_API:` :323, `__ERROR_NETWORK:` :328 (**catch-all — `ResponseFormat` lands
  here**), documented :202-208. `nativeIsPromptEcho` :370-389 and `nativeStripPromptFragments`
  :392-418 exist and are off the transcribe path. `select_stt_provider` :93-110 is the **extraction
  precedent**: `#[cfg_attr(not(any(test, target_os = "android")), allow(dead_code))]` + a plain-Rust
  signature, so `cargo test --lib` reaches it. Everything else in the file is
  `#[cfg(target_os = "android")]`; the file's own `mod tests` :504-506 is android-gated and has never
  run.
- `src-tauri/src/commands/recording.rs` — `is_offline_mode` :101-112 (`stt_provider == "local"`
  **alone**) used by `cleanup_text` :243-247 → early `return Ok(raw_text)`. This is the React in-app
  button path (`src/hooks/useRecording.ts`), which also runs on Android inside `TauriActivity`.
- `src-tauri/src/overlay_message.rs` — `enum DegradeCause` :218-238, `status_line()` :244-259,
  `card()` `ClipboardWriteFailed` → header `"TEXT LOST"` :298-307, `PILL_LABEL_CLIPBOARD =
  "In Clipboard"` :191.
- `src-tauri/src/paste/mod.rs` — `set_clipboard` :120-126 (wraps `arboard`, maps to
  `PasteError::Clipboard(String)`), `copy_only` :91-104. **The failure is already observable and
  already plumbed to `deliver_text` as `delivery.paste_failed`** — nothing new must be measured.
- `src-tauri/src/native_pill.rs` — `DoneClipboard` / `DoneDegraded` :132-134, mapped :192. Desktop
  already has the degraded terminal states Android lacks.
- `src-tauri/src/vad/mod.rs` — `VadConfig` :53-79 / `Default` :81-93 (`onset 0.5`, `offset 0.35`,
  `hangover_ms 608`, `min_onset_frames 3`, `energy_floor 0.001`); `enum HysteresisState` :190-200;
  `process_frame` energy gate :304-317 (**engine not called** on sub-floor frames :311-315);
  `advance_state` :323-375 (offset is load-bearing twice: Speaking→Hangover and Hangover→Speaking);
  hangover test :509-549. Runtime overrides from the user's silence threshold:
  `src-tauri/src/audio/mod.rs:1074`, `:1182`.
- `src-tauri/src/test_helpers.rs` — `temp_dir()`, `make_state(&TempDir) -> AppState`.

### Kotlin (`android/kotlin-src/com/klarvo/voice/`)

- `KlarvoApi.kt` (1832 L)
  - `mapCleanupResponse(responseCode, body, model): String` :212-224 — non-200 →
    `IOException("LLM cleanup failed (…): HTTP $code -- $body")` :214; else `getString("content")
    .trim()` :216-222 → `sanitizeLlmOutput` :223. **No emptiness check, `finish_reason` never read.**
    ⚠ its KDoc :186-211 names D-H19/D-M16/D-M2 as deliberate, fixture-pinned divergences and says
    *"story 13-2 owns the fix"* — that KDoc must be rewritten with the behaviour, not left lying.
    Callers :252, :287, :1482, :1489.
  - `CLEANUP_MAX_TOKENS = 2048` :73, emitted :1473 inside `cleanup(…)` :1339-1491. Canned wire
    `truncated` already carries `"finish_reason":"length"` :166.
  - `collectChunkResults` :1637-1645 (re-wrap :1640-1644); `CHUNK_THRESHOLD = 400` :1494;
    `shouldChunk` :1625; `isTrivialChunk` :1507.
  - `saveToHistory` :971-1000 — returns **Unit**, no row id exists anywhere. `pushToTurso` :1089
    re-reads unsynced rows from `history.db` itself.
  - `readConfig` — `sttProvider` parsed :814; the admitting line **:933**
    (`gatedSttProvider != "local" && groqKey.isBlank()` → null), i.e. `local` is admitted with no Groq
    key. `Config` :383 has **no `sttPrompt*` field at all**. `gateProvidersForLicense` is 13-1b's pure
    seam and is **13-4's subject** — do not change its decision.
  - `pasteErrorCount` declared :1745, read :1765, serialised :1784 — **never incremented**.
  - `sanitizeLlmOutput` strips control characters only; there is no post-cleanup ghost strip.
- `KlarvoOverlayService.kt` (2851 L)
  - `enum RecordingState { IDLE, RECORDING, TRANSCRIBING, DONE }` :299 — **no degraded state**;
    `setState` :2585-2601 maps 1:1 to `FloatingBubbleView.State` (`FloatingBubbleView.kt:79`, dispatch
    :733-741). `DEBUG_SET_STATE` string map :475-478 (the harness reachability seam).
    `doneFlashRunnable` :627-634 (800 ms).
  - `DeliveryDecision(paste, showCopiedToast)` :184 and `decideDelivery` :2212-2222 — pure, already
    JVM-tested by `CleanupFailureDeliveryTest`. Today it **pre-decides** `showCopiedToast =
    !accessibilityConnected` :221 and `llmCleanupFailed -> DeliveryDecision(false, false)` :216.
  - Step layout: Step 2 cleanup :2182-2281 (`finalText` :2183, catch :2210, fallback gate :2236-2240)
    → **Step 3 `saveToHistory` :2284-2296 → Step 3b Turso thread :2298-2307** → Step 4 header
    :2311-2322 → `handler.post {` :2323 → `BankingGuard.shouldBlockPaste(bankingAppActive)` :2327 →
    blocked branch :2328-2334 (`return@post`) → `copyToClipboard` :2337 → paste :2347-2351 →
    `showCopiedToast` :2358 (`"Copied: $preview"`) → DONE flash :2389-2396.
  - `copyToClipboard` :2642-2646 — **unguarded** inside `handler.post`.
  - `isRetryableCleanupFailure` :2718-2721 (private, pure `String → Boolean`, regex `HTTP (\d{3})`).
  - `transcribeWithRetry` :2734-2743 (8 params since 13-1b); `retryDelaysMs = listOf(2_000L, 5_000L)`
    :2745; `for (attempt in 0..retryDelaysMs.size)` :2748 = **3 attempts**; sentinel `when`
    :2766-2816 (`__ERROR_EMPTY_AUDIO__` :2771 non-retryable, `__ERROR_API:` :2778 4xx throw / 5xx
    retry, `__ERROR_NETWORK:` :2799-2800 retry, unknown → retry); terminal throw
    `"Groq STT failed after retries: …"` :~2818. Local-Whisper net :2095-2126, gate
    `isRetryableSttFailure` :2707-2709.
  - Preview: `RecordingMode.shouldInstallPreviewFlush(mode, livePreviewEnabled)` :294-295 — **no
    `sttProvider` parameter**; install :1666; `flushPreviewDelta` :1754-1781 passes `groqApiKey`
    blind. `nativeIsHallucination` call sites :1773, :2165 (the **only** bridge guard Kotlin calls).
  - Offline branch :2183 — `config.llmProvider == "local"` only.
  - `customPrompt` passed as the JNI `custom_prompt` :2754, call sites :1768 (preview) / :2091.
  - Toasts (all string literals): :1919 `"No audio recorded"`, :1945 `"Recording too short"`, :1958
    `"No speech detected"`, :2141 `"No speech detected"`, :2168 `"Speech not recognized"`; keep :1983
    (no API keys) and :2328 (`"Paste blocked — banking app active."`). All five share an identical
    6-line epilogue (`autoLoopActive = false; hideListeningPanel(); prev; setState(IDLE);
    adjustLayoutForState(IDLE, prev)`).
- `KlarvoAccessibilityService.kt` (303 L) — `pasteIntoFocusedField()` :179-185 returns **Unit**;
  `rootInActiveWindow ?: return` :180, `findFocusedEditable` :181 (defined :231-240),
  `focusedNode?.performAction(ACTION_PASTE)` :182 with its `Boolean` **discarded**. Three silent
  failure modes, zero logging. **Exactly one caller: `KlarvoOverlayService.kt:2350`.** Good precedent
  next door: `performEnter` :202-229 uses the same finder and already logs all three misses.
- `KlarvoAudioRecorder.kt` (768 L) — `vadGateDecision` :248-261 (companion, pure, the 7-2 extraction
  precedent), call site :532-539; `processVadFrame` :520-630, onset :559-575, hangover :577-616,
  **one-shot at :590 `silentFrames >= requiredSilentFrames`** (N-th frame); constants :156-186
  (`VAD_ONSET_FRAMES 3`, `VAD_FRAMES_PER_SECOND 31.25`, `MIN_SILENT_FRAMES 7`); `VadSilero`
  construction :407-412 (`Mode.NORMAL`). **No test constructs `KlarvoAudioRecorder`.**
- `BankingGuard.kt` — `shouldBlockPaste` :23 (pure, JVM-tested by `BankingGuardTest`); its KDoc
  :11-13 states `bankingAppActive` (`KOS:343`, written :877-879) is **main-looper-owned**, which is
  why the verdict is read inside `handler.post`.
- `GroqSttBridge.kt` — `nativeTranscribe` and the three guard externs `nativeIsPromptEcho` :92,
  `nativeStripPromptFragments` :99.

### Fixtures & tests

- `test-fixtures/` — `README.md` carries the reader ledger (update it for every new fixture).
  Schema: flat JSON array; per vector `id`, `surface`, `scenario`, `description` (with `PINS:` /
  `DOES NOT PIN:`), `wire`, `rust`, `kotlin`, `expected_divergence`.
  `test-provider-scenario-vectors.json` — `TEST-LLM-EMPTY-001`, `TEST-LLM-TRUNCATED-001`,
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
  71 L. **Plain JUnit only — no Robolectric, no mocking library.** Anything touching `android.util.Log`
  (every `KlarvoLogger` call), `Context`, `Handler`, `Toast`, `ClipboardManager` or
  `AccessibilityNodeInfo` throws "not mocked"; the 13-1/13-1b answer is a pure companion seam.
- `Adr0017BoundaryGuardTest` scans `.kt` files **one level** under `android/kotlin-src/com/klarvo/voice/`,
  strips comments string-aware, then matches four shapes: a `class|object|interface` named
  `HallucinationFilter`/`SilencePreFilter`, `fun buildMultipartBody`, `multipart/form-data`,
  `audio/transcriptions`. Comments are safe; `android/kotlin-test/` is out of scope.

## Tasks & Acceptance

**Execution:**

*Group 1 — the guard chain becomes one chain (B2 / D-H5, D-H6, D-M9; B3 / D-H7)*

- [ ] `src-tauri/src/pipeline.rs` — extract the desktop chain into one pure, host-reachable function
      (e.g. `guard_transcript(raw, stt_hint) -> GuardOutcome`) that performs, in this order: ghost
      strip → `strip_prompt_fragments` → `post_stt_skip`. Rewire the desktop call sites
      (:1427-1450) to it so desktop behaviour is defined by the same function Android will call.
      Adding the **pre-guard ghost strip on Desktop is a deliberate behaviour change** (B3): a raw
      transcript with a trailing ghost stops being dropped whole. Keep `PostSttSkip` and the skip
      terminal as they are.
- [ ] `src-tauri/src/stt/groq_jni.rs` — replace the inline chain (:307-314) with a call to that one
      function, fed the **hint only** (the same value `pipeline` computes as `stt_hint_text`), not
      the full built prompt. This closes D-H5, D-H6 and D-M9 together. Give the JNI a plain-Rust
      helper in the `select_stt_provider` shape
      (`#[cfg_attr(not(any(test, target_os = "android")), allow(dead_code))]`) so `cargo test --lib`
      can reach the chain — without it every B2/B3 claim is agent-only. Delete the stale parity
      comment :302-306 rather than re-citing dead lines.
- [ ] `src-tauri/src/stt/groq_jni.rs` + `KlarvoOverlayService.kt` — the hint must reach the JNI
      separately from the wire prompt (today only the combined prompt crosses). Either pass the hint
      as its own argument (JNI arity change ⇒ **both sides in one commit**, and Andi's reproduction
      needs `scripts/android-install-debug.sh <ip:port> --full`), or rebuild the hint inside the JNI
      from the fields it already receives. State which, and why, in Implementation Notes.
- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — after cleanup, put the
      transcript through the **existing** `GroqSttBridge.nativeStripPromptFragments` bridge (it
      already applies `strip_stockphrase_ghosts` to its output) or a sibling extern, so the
      post-cleanup ghost strip (B3's second half) runs on Android **in Rust**. Do not write a Kotlin
      ghost-strip twin — `Adr0017BoundaryGuardTest` would not catch it, but ADR-0017 and
      project-context both forbid it.
- [ ] `test-fixtures/guard-chain-vectors.json` (new) — vectors for: dictionary-word-only utterance
      (echo overlap), a ≥10-byte dictionary term inside a sentence (fragment strip), a leaked hint
      fragment (order), a trailing raw ghost (pre-guard), a cleanup-rationalised ghost (post-cleanup).
      Rust reader in `pipeline.rs`; Kotlin column declares itself n/a-by-construction (ADR-0017)
      following the `surface: "stt"` precedent, **plus** a Kotlin assertion that the bridge is what is
      called. Add the ledger row to `test-fixtures/README.md`.

*Group 2 — conditioning prompt (B4 / D-H4)*

- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` — add `sttPromptDe` / `sttPromptEn` /
      `sttPromptAuto` to `Config` (appended **last**; the constructor is positional) and read them in
      `readConfig` from `advanced`. Twin of `pipeline.rs:1794-1806`.
- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — stop passing `config.customPrompt`
      as the JNI `custom_prompt` (:2754, and the preview path :1768); pass the language-selected
      `sttPrompt*` instead. `customPrompt` keeps going to the LLM (:2204/:2248) and **only** there.
- [ ] `src-tauri/src/stt/mod.rs` — `build_stt_prompt_with_hint` joins hint and terms with no
      separator (:139) and is safe only because the built-in literals end in a space. A user-supplied
      `sttPromptDe` without one now glues into the first dictionary term on **both** platforms. Make
      the join explicit and pin it with a vector.

*Group 3 — silent loss on the LLM path (D2 / D-H19, D3 / D-M16, D10 / D-M2)*

- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt::mapCleanupResponse` — raise the twin of
      `LlmError::ResponseFormat` on empty content and of `LlmError::OutputTruncated` on
      `finish_reason == "length"`. Both are **non-retryable**: no provider ladder, no paste, no
      history row — the raw transcript goes to the clipboard with a named cause, exactly as
      `pipeline.rs:1549-1565` does. Rewrite the KDoc :186-211, which currently documents the
      divergence as intended.
- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — classify an undecodable body as
      retryable on the **single-call** path too, so the ladder fires as it already does on the chunked
      path. Move `isRetryableCleanupFailure` (:2718) to the companion (visibility only, no behaviour
      change) so a JVM test can drive it. Ladder membership is unchanged.
- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — an empty `finalText` must never
      reach `saveToHistory` (:2284-2296), the clipboard (:2337) or the paste (:2347-2351).
- [ ] `test-fixtures/test-provider-scenario-vectors.json` — update the `kotlin` columns and the
      `expected_divergence` strings of `TEST-LLM-EMPTY-001`, `TEST-LLM-TRUNCATED-001`,
      `TEST-LLM-MALFORMED-001` (they currently say "Story 13-2 closes this"). Keep every `wire`
      payload and all 14 scenario bodies byte-identical. Update
      `TestProviderScenarioTest.kt` :242, :256, :274, :298 accordingly.

*Group 4 — silent loss on the STT path (D9 / D-M5, D-M6)*

- [ ] `src-tauri/src/stt/groq_jni.rs` — match `SttError::ResponseFormat` **before** the
      `__ERROR_NETWORK:` catch-all (:327-331) and emit a distinct non-retryable sentinel; document it
      beside the others (:202-208).
- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — handle the new sentinel as
      non-retryable in the `when` (:2766-2816) and cut the retry budget to **1** (`retryDelaysMs`
      :2745) to match Desktop's single attempt. Extract the sentinel→verdict decision into a pure
      companion function (`classifySttSentinel`) so the whole ladder is JVM-testable; the backoff
      loop itself stays on-device.

*Group 5 — terminal states tell the truth (D4 / D-H20, D5 / D-M24, D6 / D-M12)*

- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoAccessibilityService.kt` — `pasteIntoFocusedField`
      returns a result instead of `Unit` (the three misses are already distinguishable; `performEnter`
      :202-229 is the shipped logging precedent). One caller to update (`KOS:2350`).
- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — Step 4 stops deciding on
      `instance != null` (:2347-2351). Extend the pure `decideDelivery` (:2212-2222) to take the paste
      outcome so the toast decision moves **after** the paste: a failed paste shows the shipped
      `"Copied: $preview"` toast (:2358) and **no DONE flash**. Extend `CleanupFailureDeliveryTest`.
- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — the DONE flash (:2389-2396) runs
      only on a genuine success, which is what its own comment already claims
      (*"Only the success path gets the DONE state; error paths go straight to IDLE"*). Put the choice
      in a pure companion function (`terminalStateFor(...)`) so it is JVM-testable. **No new
      `RecordingState`, no new bubble drawing** — a failed run ends in the shipped IDLE state with the
      existing degrade toast carrying the cause.
- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — wrap `copyToClipboard`
      (:2642-2646) in try/catch; on failure increment `pasteErrorCount` (`KlarvoApi.kt:1745`, today
      never incremented) and do not show success. Never an uncaught main-thread exception.
- [ ] `src-tauri/src/pipeline.rs::terminal_degrade_cause` (:2427-2436) — widen the `Some(_) if
      paste_failed` arm so a clipboard-write failure on a **normal** run also produces the shipped
      `ClipboardWriteFailed` cause and its shipped `"TEXT LOST"` card
      (`overlay_message.rs:298-307`). Discriminate against a *focus* failure
      (`PasteResult::ClipboardOnly` returns `Ok`), which keeps today's `"In Clipboard"` wording.
      Rewrite `spec_non_degraded_run_never_gains_a_cause` (:5772-5778) — it pins the bug.

*Group 6 — the quiet Android (D11 / D-M14)*

- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — remove the four
      nothing-recognized toasts (:1945, :1958, :2141, :2168), leaving their shared 6-line epilogue
      intact. **Keep** `"No audio recorded"` (:1919 — a capture fault, not a recognition result),
      `"No API keys configured…"` (:1983) and `"Paste blocked — banking app active."` (:2328). See
      Design Notes for why the row's scope stops there.

*Group 7 — ordering (B1 Android half / D-H3)*

- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — move Step 3 `saveToHistory`
      (:2284-2296) and Step 3b `pushToTurso` (:2298-2307) **after** the guard verdict (:2327); a
      blocked dictation writes neither. `saveToHistory` returns `Unit` and `pushToTurso` re-reads
      `history.db` itself, so nothing depends on the old order — but the guard is read on the main
      looper, so the writes must be posted back to a worker thread rather than run there. The separate
      `savePendingHistoryEntry` path (outer IOException catch, ~:2420) is unaffected.

*Group 8 — one offline rule (E1 / D-H9, E2 / D-H10, D-M20, D-M21)*

- [ ] `src-tauri/src/pipeline.rs` — install one pure predicate on `&AppConfig` beside
      `effective_llm_provider_name` (:290-296): local cleanup counts as available only where it
      exists, and `is_offline` (:643-645) is expressed through it. Make the
      `#[cfg(target_os = "windows")]` fall-through (:322-323) stop resolving an unavailable `"local"`
      to a silent network DeepSeek call — under G2a the answer is **no cleanup** (the shipped
      `OfflineRaw` outcome), never a cloud call.
- [ ] `src-tauri/src/commands/recording.rs` — delete `is_offline_mode` (:101-112) and make
      `cleanup_text` (:243-247) read the shared predicate. The helper stays `&AppConfig`-shaped, not
      `&AppState`-shaped. This is the whole D-M20 fix.
- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — the offline branch (:2183) reads
      the twin of that predicate instead of `config.llmProvider == "local"` alone. Put the twin in a
      pure companion function so a JVM test drives it (twinned behaviour, per project-context: fix
      twice, pin with a fixture).
- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — `shouldInstallPreviewFlush`
      (:294-295) takes `sttProvider` and refuses to install for a stored `local`; `flushPreviewDelta`
      (:1754-1781) re-checks at flush time, as `pipeline.rs:2545-2553` does. Extend
      `ShouldInstallPreviewFlushTest`. This is E1 and it is mandatory independently of 13-3.
- [ ] `test-fixtures/offline-rule-vectors.json` (new) — the config matrix
      (`stt ∈ {local, cloud}` × `llm ∈ {local, cloud}` × platform-availability) with one expected
      outcome per row, read by a Rust test and a JVM test. Ledger row in `README.md`.

*Group 9 — B6 (VAD): see the blocking question in Design Notes*

- [ ] `android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt` — **D-L21 only, and only once B6 is
      decided:** the hangover one-shot fires on the **(N+1)-th** consecutive non-speech frame, as
      `vad/mod.rs::advance_state` does, not on the N-th (:590). Extract the counter into a pure
      companion function first (the `vadGateDecision` precedent at :248-261) — there is no test that
      constructs `KlarvoAudioRecorder`, so without the extraction the change is unverifiable. Add a
      Rust reader for `vad-gate-golden-vectors-7-2.json` (Kotlin-only today) or a new both-sides
      fixture that pins the trigger **edge**, not just `expected_frames`.
- [ ] **D-M10 (dual threshold 0.5/0.35) is not executable as specified** — see Design Notes. No task
      is written until the question is answered.

**Acceptance Criteria:**

- Given a dictionary containing `Klarvo, Kubernetes` and the utterance "Klarvo und Kubernetes.", when
  the transcript passes the guard chain on **either** platform, then it survives — and given a
  one-term dictionary `Bundesverfassungsgericht`, the term is not deleted from the sentence.
- Given a transcript carrying a leaked hint fragment, when the chain runs on either platform, then
  fragments are stripped **before** the echo/blocklist verdict, and the same input yields the same
  verdict on both — asserted against one fixture read by a Rust test, with the Android side asserted
  to delegate to the JNI bridge rather than to a Kotlin re-implementation.
- Given a raw transcript ending in a stockphrase ghost, when it is processed on either platform, then
  the ghost is stripped and the remainder survives (Desktop no longer drops the whole transcript); and
  given a ghost the LLM rationalises during cleanup, then it is stripped after cleanup on **both**
  platforms.
- Given a Cleanup Instruction is set (preset "Technical") and identical audio, when Android transcribes,
  then Whisper is conditioned with `advanced.sttPrompt*` — not with the cleanup instruction — and
  `customPrompt` appears in no STT request on either platform.
- Given `advanced.testProviderLlm = "empty"`, when a dictation runs, then on **both** platforms nothing
  is pasted, **no history row is written**, the raw transcript is in the clipboard, the cause is named,
  and the log names the scenario. The same holds for `"truncated"`.
- Given `advanced.testProviderLlm = "malformed"` and a dictation shorter than 400 bytes, when cleanup
  runs on Android, then the provider ladder fires (`deepseek → openai → openrouter`, Groq never a
  candidate) exactly as on the chunked path and as Desktop does.
- Given `advanced.testProviderStt = "empty"`, when STT runs on Android, then the failure is classified
  non-retryable, exactly one attempt is made, and no 2 s/5 s backoff is burned.
- Given the accessibility service is connected but no editable field is focused, when delivery runs,
  then no success check appears and the shipped `"Copied: …"` toast fires; and given cleanup failed,
  then no DONE flash occurs at all — both decided by pure functions a JVM test drives.
- Given the clipboard write throws on Android, when delivery runs, then the exception is caught,
  `pasteErrorCount` is incremented, and no success is shown; and given `set_clipboard` fails on Desktop
  on a run with no cleanup degrade, then the shipped `ClipboardWriteFailed` cause fires and the pill
  stops claiming "In Clipboard" — while a *focus*-only failure keeps today's wording.
- Given a mini-tap, a silent capture, a blank transcript or a hallucination verdict on Android, when
  the run ends, then no toast is shown (Desktop parity) — while `"No audio recorded"`,
  `"No API keys configured…"` and `"Paste blocked — banking app active."` still are.
- Given a blocklisted app in the foreground, when a dictation completes on Android, then neither a
  `history.db` row nor a Turso push exists for it, and the paste is still blocked.
- Given a stored `sttProvider = "local"` and live preview enabled on Android, when the user pauses,
  then no delta WAV is sent — with or without a Groq key — asserted at both the install and the flush
  decision.
- Given `sttProvider = "local"` with any cloud `llmProvider`, when a dictation runs via the hotkey, via
  the React in-app button, and on Android, then all three produce the identical raw-text outcome with no
  network cleanup call; and given `llmProvider = "local"` where no local LLM exists, then the result is
  no cleanup, never a silent DeepSeek call.
- Given every new or reshaped guard, vector and predicate, when it is inverted at writing time, then it
  goes RED, and the evidence is recorded in `gate4-evidence/13-2/` in the 13-1b shape.
- Given the whole change, when `Adr0017BoundaryGuardTest` runs, then it is green and no Kotlin twin of
  an STT guard exists.

## Implementation Notes

## Spec Change Log

## Review Triage Log

## Design Notes

### Delivery-state contract (which state each terminal path enters, and its source)

This story changes *which* state is entered, never what a state looks like. No new state, no new
wording, no new drawing.

| Path | Android terminal state today | After | Source |
|---|---|---|---|
| success | DONE flash 800 ms → IDLE | unchanged | shipped |
| cleanup failed (D5) | DONE flash + degrade toast | IDLE + the same degrade toast | shipped IDLE; the DONE-flash comment already promises this |
| paste failed, clipboard holds text (D4) | DONE flash, no toast | IDLE + shipped `"Copied: $preview"` toast (`KOS:2358`) | shipped toast, already used when accessibility is not connected |
| empty / truncated LLM answer (D2, D3) | pasted + stored | the shipped cleanup-failure path: clipboard + named cause | shipped (7-10, `CLEANUP_FAILED_CLIPBOARD_MSG`) |
| clipboard throws (D6) | crash | IDLE, no success, `pasteErrorCount++` | shipped IDLE |
| nothing recognized (D11) | toast | silent, IDLE | Desktop `PipelineEvent::idle()` |
| Desktop clipboard write failed (D6) | pill `"In Clipboard"` | shipped `ClipboardWriteFailed` cause → `"TEXT LOST"` card | shipped (7-10 degrade branch), reused verbatim |

Recorded as **canon gap, shipped precedent**: the Android degraded-terminal states have no design-canon
entry; each reuses a component that exists at `baseline_revision` and is already in use on another
surface. Desktop's `DoneClipboard`/`DoneDegraded` (`native_pill.rs:132-134`) is the conceptual twin but
is *not* ported — porting it would mean a new Android bubble state, which this story does not design.

### Why D11 stops at four of five toasts

ADR-0016 Amendment 4 names the subject as *"kein Toast bei 'nichts erkannt'"*, direction Desktop; the
epic repeats *"no toast on 'nothing recognized'"*. Audit row D-M14 lists five Android sites. Four of
them are recognition results (mini-tap, pre-STT silence, blank transcript, hallucination verdict) and
have a mute desktop twin. `"No audio recorded"` (`KOS:1919`) fires on an **empty capture buffer** — a
recorder/plumbing fault, not a recognition result. Removing it would create exactly the silent loss this
story exists to remove, so it stays. If the reviewer reads the row as all five, that is a one-line
change — the reading is recorded here rather than buried.

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
new name — but ADR-0017 and project-context both forbid it, and `GroqSttBridge` already declares
`nativeStripPromptFragments`, which applies `strip_stockphrase_ghosts` to its output. Reuse beats a
twin: one implementation, one fixture, no drift to re-audit next quarter.

### Why one offline predicate, and where it lives

`effective_llm_provider_name` (13-1b) has the right shape (`&AppConfig → String`, pure, twin-documented)
but folds only the test-provider override; what is missing is *platform availability*. One predicate
beside it, read by all three current definitions, is an extraction next to an existing seam — not a new
construct. The `_ => deepseek` fall-through must stop swallowing an unavailable `"local"`: under G2a the
correct degrade is **no cleanup**, the shipped `OfflineRaw` outcome, so no new user-facing wording is
needed.

### B6 — BLOCKING QUESTION (VAD dual threshold is not executable as specified)

ADR-0016 Amendment 4 closes B6 with *"Hysterese 0,5/0,35 + Hangover-Frame in Kotlin"*, size **S**,
direction Desktop — overriding Amendment 3's "asymmetry" verdict on Andi's instruction
("beides gleichmachen"). The size rests on a premise the tree refutes.

**Evidence.** The shipped dependency is `com.github.gkonovalov.android-vad:silero:2.0.10`
(`src-tauri/gen/android/app/build.gradle.kts:69`, patched in by `scripts/android-build.sh:200-203`).
`javap -public com.konovalov.vad.silero.VadSilero` shows the only inference entry points are
`isSpeech(short[]|byte[]|float[]) -> boolean`. `extractResult`, `threshold()` and `predict(float[])`
are **private**; `javap -c -p` on `threshold()` gives `NORMAL = 0.5f`, `AGGRESSIVE = 0.8f`,
`VERY_AGGRESSIVE = 0.95f`. **The raw probability is not obtainable**, so a second threshold at 0.35
cannot be evaluated. The four options are: (a) accept the dual threshold as a documented carve-out and
build only the hangover frame; (b) change `Mode` — still one threshold, and a different one;
(c) use the library's unused public `speechDurationMs`/`silenceDurationMs` — a time-shaped debounce,
not 0.35; (d) bypass the library and drive `ai.onnxruntime.OrtSession` against the AAR's own
`silero_vad.onnx`. **Only (d) delivers 0.5/0.35, and it is not an S.** Choosing (a) would silently
restore the asymmetry Andi struck; choosing (d) would put an L-class dependency change inside a sweep
story. Neither is this run's decision.

**A second, independent re-framing in the same row.** Audit row D-L19 treats "Android calls `isSpeech`
on every frame" as Android drift, with direction Desktop. The tree documents the opposite:
`KlarvoAudioRecorder`'s KDoc states that Silero is stateful and that skipping frames corrupts its
sliding window, while `vad/mod.rs::process_frame` skips the engine entirely on sub-floor frames.
"Direction Desktop" here would make Android *worse*. And the row's premise "Android has no hysteresis"
is only half true today: Android already has onset hysteresis (3 frames, matching `min_onset_frames`)
and a 200 ms hangover floor since 7-2/11-2. The only two real deltas left are the dual threshold and
the N-vs-N+1 hangover edge.

**What is needed:** one decision — (a), (b), (c) or (d) for the dual threshold, and whether D-L19's
direction stands given the statefulness evidence. D-L21 (the hangover off-by-one) is executable and
small either way, and its task is written above.

## Verification

**Commands:**
- `cd src-tauri && cargo test --lib` — expected green with no API keys. **Measure the baseline at
  `3f87beffb0023f0024a8f61189d7be69eb8bc46d` first** and apply the baseline exception before treating
  any red as this story's. 13-1b measured **749 passing** at its own baseline.
- Device-free JVM gate (`scripts/android-smoke.sh` cannot be used here — it hard-fails on
  "Kein Gerät gefunden" before reaching the gate):
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
- `npm run build` — TS strict, only if a `src/` file is touched (no task above requires one).
- Inversion evidence — every new guard, vector, sentinel and predicate is broken deliberately, run,
  shown RED, and reverted. Output to
  `_bmad-output/implementation-artifacts/gate4-evidence/13-2/{code-inversions.json,code-inversion-report.md,verdict.md}`
  in the 13-1b schema (per-entry `inversion`, `file`, `filter`/`expected_red_test`, `RED`, `evidence`).
  The tree is re-confirmed green after the last revert and `git status` is clean.

**What no unattended run may claim.** `cargo test --lib` and the JVM gate decide logic, wiring and
structure on Linux only. They do not decide: the real Whisper conditioning result for real audio (B4),
the banking guard on a real foreground app (B1), the clipboard/accessibility behaviour on a real device
(D4, D6-Android), the bubble's appearance, or anything on Windows. Every count reported must name what
it did not exercise.

**H+ reproduction path (named before build, per Verifikations-Symmetrie):**
- **D2, D3, D10, D9 — reproducible by Andi, no computer needed.** Settings → Advanced → Expert mode →
  System → `Test provider (LLM)` = `empty` / `truncated` / `malformed`, or `Test provider (STT)` =
  `empty`; press the Advanced `Save` (one button, both chains, since 13-1b); dictate a short sentence.
  Keep it under 400 characters — above that, cleanup chunks and the canned answer is returned per
  chunk, which changes the `malformed` verdict on Android. Which scenario fired is in `klarvo.log` on
  both platforms. ⚠ A downloaded local Whisper model can silently rescue a test STT failure via
  `pipeline::try_local_whisper_fallback` — check that first if an STT scenario appears not to fire.
- **B2 — 📱 Xiaomi:** dictionary `Klarvo, Kubernetes`, say "Klarvo und Kubernetes." → the sentence
  survives (today it is discarded).
- **B3 — 🤖 H+ 📱:** dictate a known ghost phrase at the end; it is stripped on both devices.
- **B4 — 📱 Xiaomi:** set preset "Technical", dictate the same audio twice around the change; the raw
  transcript's punctuation/casing changes.
- **D4, D5, D11 — 📱 Xiaomi:** paste into a non-editable surface (D4), force a cleanup failure with the
  test provider (D5), mini-tap the bubble (D11).
- **E1 — 🤖 H+:** store `sttProvider = local`, enable live preview, pause; `klarvo.log` shows no upload.
- **E2 — 🖥️ Windows in-app button:** choose Offline, dictate with filler words; the "ähm" stay.
- **D6 — Weg 2, agent-verified only, written down rather than implied.** Neither a Desktop
  `set_clipboard` failure nor an Android `setPrimaryClip` throw is producible on Andi's machines, and
  the test provider does not inject them. ADR-0016 Amendment 4 already records this downgrade.
- **B1-Android — 📱 Xiaomi:** dictate into a blocklisted app; afterwards History shows no entry.
- **Android install:** if the JNI signature changes (Group 1), `scripts/android-install-debug.sh
  <ip:port> --full` is **mandatory** — `#[no_mangle]` exports the short name, so a plain install
  misbinds a stale `.so` silently instead of throwing.
- **Surface DoD (G-D):** the rows above that change what Andi sees end with a Windows release build via
  `scripts/windows-build.sh` (run in the background) and a fresh APK on the Xiaomi.
</content>
</invoke>

## Auto Run Result

Status: blocked
Blocking condition: intent gap

### 2026-09-21 — planning run (halt after planning requested; halted earlier, at the intent gate)

Planning completed for **14 of the 15** rows ADR-0016 Amendment 4 assigns to this story. The spec
above is written and self-contained: Code Map anchors are re-measured against
`baseline_revision` (the audit's anchors are stale by roughly +43 / +33 / +93 lines in
`pipeline.rs` / `stt/mod.rs` / `groq_jni.rs`, and 13-1 extracted both HTTP mappers), so a resumed run
does not have to re-investigate.

**The open question — B6 (audit rows D-M10, D-L19).** ADR-0016 Amendment 4 closes B6 with
"Hysterese 0,5/0,35 + Hangover-Frame in Kotlin", size S, direction Desktop, explicitly overriding
Amendment 3's asymmetry verdict on Andi's instruction ("beides gleichmachen"). That size rests on a
premise the tree refutes:

1. **The dual threshold is not obtainable from the shipped VAD library.**
   `com.github.gkonovalov.android-vad:silero:2.0.10` exposes only
   `isSpeech(short[]|byte[]|float[]) -> boolean`; `extractResult`, `threshold()` and
   `predict(float[])` are private, and `Mode.NORMAL` hardcodes 0.5f (verified by `javap -public` and
   `javap -c -p` against the AAR in the Gradle cache). Without the raw probability, a second
   threshold at 0.35 cannot be evaluated at all. Options: (a) document the dual threshold as a
   carve-out and build only the hangover frame; (b) change `Mode` — still a single, different
   threshold; (c) use the library's unused public `speechDurationMs`/`silenceDurationMs`, a
   time-shaped debounce rather than 0.35; (d) bypass the library and drive `ai.onnxruntime.OrtSession`
   against the AAR's own `silero_vad.onnx`. Only (d) delivers 0.5/0.35, and it is not an S.
   (a) would silently restore the asymmetry Andi struck; (d) would put an L-class dependency change
   inside a sweep story. Picking either is a product/architecture decision, not this run's.

2. **D-L19's direction is contradicted by the tree.** The audit reads "Android calls `isSpeech` on
   every frame" as Android drift with direction Desktop. `KlarvoAudioRecorder`'s own KDoc states the
   opposite is deliberate — Silero is stateful, so skipping frames corrupts its sliding window —
   while `vad/mod.rs::process_frame` skips the engine entirely on sub-floor frames. "Direction
   Desktop" would make Android worse. Also, the row's premise "Android has no hysteresis" is only
   half true today: Android has had onset hysteresis (3 frames, matching `min_onset_frames`) and a
   200 ms hangover floor since 7-2/11-2. Two real deltas remain: the dual threshold and the
   N-vs-(N+1) hangover edge.

**What unblocks the run:** one answer — (a), (b), (c) or (d) for the dual threshold, and whether
D-L19's direction stands given the statefulness evidence. D-L21 (the hangover off-by-one) is
executable and small under every option; its task is already written, together with its precursor
(extract the frame counter into a pure companion function — no test constructs `KlarvoAudioRecorder`
today, so without that extraction the change is unverifiable).

### Decisions taken during planning (recorded, not silent)

- **D11 scope: four of the five toasts.** Amendment 4 and the epic both name "nothing recognized";
  four Android sites are recognition results with a mute desktop twin, while `"No audio recorded"` is
  an empty-capture fault. Removing that one would create the silent loss this story exists to remove,
  so it stays. Alternative (remove all five, matching the audit row's broader framing) is a one-line
  change; the reading is in Design Notes rather than buried.
- **No new Android terminal state.** D4/D5 are met by entering the shipped IDLE state and firing the
  shipped `"Copied: …"` toast, which is what the DONE-flash comment already promises. Desktop's
  `DoneClipboard`/`DoneDegraded` is the conceptual twin but is deliberately **not** ported — that
  would be a new bubble state, i.e. a design decision this story does not own.
  Recorded as `canon gap, shipped precedent`.
- **D6 desktop half reuses the shipped `ClipboardWriteFailed` cause and its "TEXT LOST" card**, so no
  new user-facing literal is introduced. `docs/backlog.md:378-382` proposes a new literal (Q2-class,
  routed to Andi); reuse avoids that wording decision entirely. Alternative recorded here.
- **B1-Android: a blocked dictation writes neither history nor Turso.** The row is only meaningful
  under that reading, and 13-6's AC states the same semantics for the desktop twin.
- **The guard chain is extracted into one shared Rust function rather than mirrored.** B2's three
  divergences are one defect — `nativeTranscribe` re-implements the chain inline instead of calling
  it. The extraction is also the only way `cargo test --lib` can witness any of B2/B3: everything
  inside `nativeTranscribe` is `#[cfg(target_os = "android")]` today, and `groq_jni.rs::tests` is
  android-gated and has never executed.
- **Android's post-cleanup ghost strip goes through the existing JNI bridge**, not a Kotlin twin.
  `Adr0017BoundaryGuardTest` pins four shapes and would not catch a renamed twin, but ADR-0017 and
  project-context both forbid one.

### What this run does NOT claim

No code was changed; the working tree is clean at `3f87bef`. Nothing was built, no test was run
beyond a read-only `cargo check --lib` during investigation (clean, 13 pre-existing warnings). The
Code Map was re-measured by reading the tree, not by executing it. The 14 planned rows are planned,
not verified.
