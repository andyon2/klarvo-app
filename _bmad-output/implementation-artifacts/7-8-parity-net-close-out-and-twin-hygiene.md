# Story 7.8: Parity-net close-out + twin hygiene

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As the klarvo maintainer,
I want the existing golden-vector net closed around the last unguarded twins and the accepted 7-2 residuals cleared,
so that a future re-divergence trips a test on the gates that actually exist, and the net is not carrying known-false claims.

## Context & Governing Decisions

**This story supersedes 7.5 and 7.7** (`sprint-change-proposal-2026-09-10.md`, applied in `22140c2`).
The June cut assumed a CI and an empty fixture directory; **neither holds**. There is no CI in this
repo (no `.github/`, no GitLab file), and 7-1/7-2/7-3 already delivered the shared fixture format and
**both** harnesses. What remains is *closing* the net, not building it.

**Rows:** M9 · 7-3 AC9 guard · twin-constant lock · 7-2 residuals · `android-smoke.sh` traps.

**Character of the work: test-heavy, code-light.** The only runtime change in the entire story is
**two URL strings** in `KlarvoApi.kt` (M9). Everything else is fixtures, JVM/Rust tests, KDoc/comment
corrections, and two `android-smoke.sh` shell corrections.

**Governing decisions:**
- **ADR-0017** — Hard Rule: shared STT/guard logic lives **only** in the Rust core; a parallel Kotlin
  re-implementation is forbidden. AC2 is the *mechanical enforcement* that 7-3 AC9 promised but did
  not build.
- **ADR-0016 Amendment 1** — the parity line covers core-output determinism + settable-but-silently-dead
  config keys. Amendment 2 keeps LLM routing (M9) as a platform-local per-row fix — **not** a JNI
  consolidation.
- **Epic close rule:** after this story is `done`, Epic 7 may close with 7.6 parked if M12 is still
  undecided.

## Acceptance Criteria

### AC1 — M9: DeepSeek endpoint parity (the only runtime change)

**Given** Desktop calls DeepSeek at `https://api.deepseek.com/v1/chat/completions`
(`src-tauri/src/llm/mod.rs:719`, `pub(crate) const BASE_URL`),
**And** both Android call sites today omit the `/v1` segment —
`KlarvoApi.kt:164` (the `resolveLlmProvider` `else ->` arm) and
`KlarvoApi.kt:198` (the `cleanupFallbackCandidates` DeepSeek triple), both
`url = "https://api.deepseek.com/chat/completions"`,
**When** Android resolves the DeepSeek provider on either path (primary selection **or** the Epic-12
cross-provider fallback ladder),
**Then** both sites use `https://api.deepseek.com/v1/chat/completions`,
**And** a JVM test pins the URL constant so a future edit of either site trips RED,
**And** the two sites do not drift from each other (the test covers **both**, not just one).

> **Reuse, do not create a new test file:** `android/kotlin-test/com/klarvo/voice/LlmFallbackProviderTest.kt`
> (168 lines) already exercises `KlarvoApi.resolveLlmProvider` / `resolveFallbackLlmProvider` — i.e.
> exactly the two functions that own these URLs. Extend it.

> **Scope guard:** `src-tauri/src/commands/settings.rs:979`
> (`"https://api.deepseek.com/v1/models"`) is the *model-list* endpoint used for key validation, is
> already correct, and is **out of scope** — do not touch it.
> DeepSeek accepts both hosts today, so this is **drift, not an outage**. Do not describe it as a fix
> for a broken feature.

### AC2 — ADR-0017 boundary guard (mechanical, inside an existing gate)

**Given** 7-3 deleted the Kotlin STT twins (`KlarvoApi.transcribe`, `buildMultipartBody`,
`HallucinationFilter.kt`, `SilencePreFilter.kt`) but left **no guard against re-growth**,
**When** the boundary check runs,
**Then** it **fails** if a Kotlin STT request or guard path re-appears: a multipart transcription body,
a `HallucinationFilter` / `SilencePreFilter` *implementation*, or any `audio/transcriptions` string
outside the JNI bridge,
**And** it runs **inside an existing gate** — either a JVM test in `android/kotlin-test/` or a step in
`scripts/android-smoke.sh` — **never** a new pipeline, script or CI file,
**And** it is **GREEN on today's clean tree**.

> **⚠ THE TRAP THAT WILL BREAK A NAIVE IMPLEMENTATION — read before writing the guard.**
> A plain `grep -r 'HallucinationFilter\|SilencePreFilter\|buildMultipartBody'` over `android/`
> **goes RED on a clean tree today**. Those names legitimately survive in **KDoc and comments** that
> document the deletion:
> - `android/kotlin-src/com/klarvo/voice/GroqSttBridge.kt:7-8` — *"The Kotlin twins (KlarvoApi.transcribe, buildMultipartBody, HallucinationFilter, SilencePreFilter) are deleted and replaced by calls to this bridge."*
> - `GroqSttBridge.kt:63` — *"Replaces HallucinationFilter.isHallucination() — same logic, shared Rust source."*
> - `KlarvoOverlayService.kt:2073` — *"Replaces HallucinationFilter.isHallucination() …"*
> - `KlarvoOverlayService.kt:2632` — *"ADR-0017: KlarvoApi.transcribe + buildMultipartBody deleted …"*
>
> **Additionally allowed (must NOT trip the guard):**
> - `LocalWhisperInference.kt:20,50,52,99` — `nativeTranscribe` on the **dormant local-whisper** path.
>   7-3 explicitly ruled this a *different surface*, not to be deleted.
> - `GroqSttBridge.kt:31,51` — `object GroqSttBridge` / `external fun nativeTranscribe`: this **is**
>   the sanctioned JNI bridge, the thing the guard exists to protect.
> - `KlarvoOverlayService.kt:1703,1869,2074,2649` — call sites *into* the bridge.
> - `KlarvoOverlayService.kt:2635` `private fun transcribeWithRetry(` — ADR-0017 deliberately keeps the
>   **retry/4xx loop** in Kotlin (`GroqSttBridge.kt:21-24`); 7-3 restored its 5xx semantics as a review
>   fix. It is not an STT request implementation.
> - `MainActivity.kt:265,270` — the microphone-permission rationale string contains the word
>   *"transcribe"*. Pure grep false positive.
> - `KlarvoTheme.kt:5` — `// Provenance: ADR-0019 (… generate-not-transcribe).` Same.
> - `android/kotlin-test/**` — `VadGateRmsFixtureTest.kt:19` and `DeltaSnapshotSliceTest.kt:16,50`
>   mention the deleted twins in comments.
>
> **Naming note (reconcile, do not "fix"):** ADR-0017's Decision text names
> `src-tauri/src/stt/jni_bridge.rs` as the consumption point, but on today's tree that file holds
> **only** the `LocalWhisperInference` entry points (`:69,130,221,250`). The actual Groq STT + guard
> JNI surface is `src-tauri/src/stt/groq_jni.rs` (`:132` `…_nativeTranscribe`, `:251`
> `…_nativeIsHallucination`, `:275` `…_nativeIsPromptEcho`, `:304` `…_nativeStripPromptFragments`,
> `:343` `…_nativeSilenceCheck`). Point the guard and its comment at `groq_jni.rs`. **Do not edit the
> ADR** — that is not this story's scope; just do not repeat its stale path.
>
> The guard must therefore key on **Kotlin implementation**, not on the mere appearance of a string:
> e.g. ignore comment/KDoc lines, allowlist `GroqSttBridge.kt` + `LocalWhisperInference.kt`, and
> assert on *declarations* (a `class`/`object`/`fun` named `HallucinationFilter`/`SilencePreFilter`/
> `buildMultipartBody`, a `multipart/form-data` body builder, or an `audio/transcriptions` literal
> outside the bridge). **A guard that is red on a clean tree is worthless and will be reverted.**

### AC3 — Twin-constant lock (one fixture, both harnesses)

**Given** these Rust↔Kotlin twins are unlocked today and can silently re-diverge,
**When** the lock is in place,
**Then** **one** shared fixture in `test-fixtures/` pins all five twin constants,
**And** a **Rust** test *and* a **Kotlin** test read **the same file**:

| Twin | Rust (source of truth) | Kotlin |
|---|---|---|
| LLM temperature `0.3` | `llm/mod.rs:473` `const DEFAULT_TEMPERATURE: f32 = 0.3;` | `KlarvoApi.kt:971` `put("temperature", 0.3)` |
| `max_tokens` `2048` | `llm/mod.rs:474` `const DEFAULT_MAX_TOKENS: u32 = 2048;` | `KlarvoApi.kt:972` `put("max_tokens", 2048)` |
| chunk threshold `400` | `llm/mod.rs:1236` `const CHUNK_THRESHOLD: usize = 400;` | `KlarvoApi.kt:1000` `private const val CHUNK_THRESHOLD = 400` |
| chunk target `350` | `llm/mod.rs:1240` `const CHUNK_TARGET_SIZE: usize = 350;` | `KlarvoApi.kt:1001` `private const val CHUNK_TARGET_SIZE = 350` |
| chunk join `\n` | `llm/mod.rs:1406-1408` — `if i > 0 && !combined_text.is_empty() { combined_text.push('\n'); }` | `KlarvoApi.kt:1158-1165` `fun joinChunkResults(…)` — `if (sb.isNotEmpty()) sb.append('\n')` |

**And** each side asserts against the **production symbol**, not a re-declared test literal (the
Epic-1 rule: *bind tests to the real code paths they cover*; the 7-1 review killed a suite that
re-implemented the behavior it claimed to lock),
**And** the fixture entry for each constant **states what it covers** (project-context: *"a number
states what it covers"*).

> **Scope guard (load-bearing):** this locks **twin parity only**. It does **NOT** lock the
> dead-config cluster (`advanced.llmTemperature`, `llmMaxTokens`, `chunkThreshold`/`chunkTargetSize`,
> `sttTemperature`, `llmModel*`/`llmSystemPrompt*`, `autoCapitalize`/`autoPaste`). Freezing those
> would cement a lying UI — Desktop renders them and no runtime code reads them, with three
> disagreeing default sets. That is an OPEN-DECISION for Andi in `docs/backlog.md`, **not** this story.
>
> **Note on `max_tokens`/temperature:** `llm/mod.rs` declares a second `DEFAULT_TEMPERATURE`/
> `DEFAULT_MAX_TOKENS` pair at `:1005-1006`, and `llm/local.rs:26,28` declares its own for the local
> llama path. Pin the pair that is the twin of the Kotlin cleanup request (`:473-474`); if the Rust
> test can cheaply assert the others agree, note it — do not silently pick the wrong pair.

### AC4 — M12 current-state vector (records, does NOT decide)

**Given** the M12 divergence is real and still open on today's tree:
Desktop's **Chat** arm omits `{dict_section}` (`llm/mod.rs:254` ends
`{custom_section}{translation_section}{sandwich}`, unlike Polished `:193` and Verbatim `:226` which
both carry `{dict_section}`), while Android's `appendPromptExtensions`
(`KlarvoApi.kt:783-788`, called unconditionally at `:945`) appends the dictionary for **every** style,
**When** the fixture is added,
**Then** it records **today's divergence as the documented current state**,
**And** it is explicitly labelled as a current-state record, so Story 7.6 flips **one vector** once
Andi decides M12.

> **Do NOT decide M12 in this story.** Do not change either platform's prompt assembly. The vector
> documents the divergence; it does not adjudicate it. M12 is Andi's open product decision
> (`docs/backlog.md` OPEN-DECISION).

### AC5 — 7-2 residuals: all 8 round-3 findings fixed + 2 adjacent sites of the same two defects

**Given** the 8 accepted `[Review][Patch]` findings in
`7-2-android-live-auto-stop-vad-gate-parity.md` → `### Review Findings — round 3` (lines 417-424),
**And** two further sites of the *same* two defects, found while verifying and now **in scope**
(see R3-P2 EXT and R3-P8 EXT below),
**When** the hygiene pass is complete,
**Then** each is fixed as prescribed,
**And** the two extension sites are fixed with the *same* remedy as their parent finding — a defect
class is not closed while a known second copy of it survives in the tree.

> **⚠ Several anchors *inside the findings themselves* have drifted** — the findings are 1 day old but
> were written against the pre-close-out tree. Verified today; locate by **content**, not by the cited
> number: R3-P1's lambda is at `KlarvoAudioRecorder.kt:534` (not `:537`; `:537` is
> `val vadSpeech = gateResult.vadSpeech`) · R3-P2's `signal_freq_hz` call is at
> `VadGateGoldenVectorsTest.kt:207` (not `:206`) · R3-P4's test body is `HighpassFilterTest.kt:269-286`
> with the assertion at `:280-285` (not `:271-281`) · R3-P5's own stale citation now sits at
> `scripts/android-smoke.sh:186` (not `:184`) · R3-P6's JSON descriptions are at `:9,:19,:49`
> (not `:8,:19,:47`) · R3-P8's clause is on `KlarvoAudioRecorder.kt:37` (sentence starts `:36`).
> This is the *same* defect class the findings are about — do not re-commit it. Cite by content.

All anchors below were **re-verified against today's tree**:

1. **R3-P1 — `HighpassFilterTest` false claim** [`HighpassFilterTest.kt:231`, `:264`, story line 333].
   The seam tests drive `vadGateDecision` **directly**; `processVadFrame` is private/stateful and
   referenced by no test — so *dropping the seam call in `processVadFrame` leaves every test green*,
   yet three places claim it is caught. **Fix:** correct the three claims to name the remaining gap,
   **or** make `processVadFrame`'s per-frame body callable from a test.
2. **R3-P2 — vacuous-pass `optDouble`** [`VadGateGoldenVectorsTest.kt:202`, `:206`; parser default at
   `:53-54` `fun optDouble(key: String, default: Double = 0.0)`]. A missing/typo'd `amplitude_short`
   or `signal_freq_hz` yields an all-zero signal → RMS 0 → gate closed → **VAD-GATE-001 and
   VAD-GATE-004 pass vacuously**. **Fix:** a throwing accessor (`getDouble`), or
   `optDouble(key, Double.NaN)` + `require(!it.isNaN())`.
   - **R3-P2 EXT (in scope, added on review directive)** — the *same* vacuous-`optDouble` shape at two
     further call sites in the same file, verified today: **`VadGateGoldenVectorsTest.kt:195`**
     (`silence_threshold` → default `0.0`, which is the most permissive threshold possible) and
     **`:231`** (`silence_secs` → default `0.0`, which floors to 7 frames and would let
     **VAD-LATENCY-003** pass vacuously). R3-P2 itself names only `amplitude_short` / `signal_freq_hz`.
     **Fix: apply the identical remedy to these two sites** — the throwing accessor is the same
     one-line change; it costs nothing extra once `getDouble` exists. This is no longer a judgement
     call for the dev.
     > **Scope boundary — do not widen further:** the directive names **`:195` and `:231` only**.
     > `:232` (`expected_frames`) and `:202`/`:207` (already covered by R3-P2 proper) are the other
     > `optDouble` sites in this file; `:232` is **not** named. If the remedy is a throwing accessor
     > on `JsonVal`, `:232` may follow mechanically — but do not report it as an AC5 deliverable.
3. **R3-P3 — `vadGateDecision` return value never asserted** [`HighpassFilterTest.kt:248-266`]. The
   test discards `VadGateResult` and passes `threshold = 0f`, so `energyAboveGate && vadSpeech` →
   `vadSpeech` alone (or `||`) stays green; the `capturedRms < rawNormalizedRms * 0.5f` assertion is
   one-sided (an all-zero buffer passes). **Fix:** assert `normalizedRms`; add a `threshold = 1f` /
   `isSpeech = { true }` case asserting `isSpeechFrame == false`; add a lower bound (`> 1e-4f`).
4. **R3-P4 — tautological "inversion" test** [`HighpassFilterTest.kt:271-281`]. It never touches
   `HighpassFilter` or `vadGateDecision`; the name misleads the inversion table. Also `:241-244`'s
   "drives this toward zero" is wrong (measured 14.4 % of raw), and the DC choice makes it insensitive
   to `HIGHPASS_CUTOFF_HZ`. **Fix:** rename to `sanity_…` **or** make it a genuine bypass inversion;
   correct the comment.
5. **R3-P5 — line-number anchor invalidated by its own commit** [`scripts/android-smoke.sh:184`].
   The comment cites *"the same idempotent, grep-guarded patch as `android-build.sh:206-213`"*;
   verified today the block spans **206-215** (comment 206-211, `if` 212, `sed` 213, `echo` 214,
   `fi` 215) — the cited range cuts off the `echo` and `fi`. **Fix:** cite by **content** (*"the
   `org.json` `testImplementation` patch in `android-build.sh`"*), not by line range.
6. **R3-P6 — three inaccurate fixture descriptions**
   [`test-fixtures/vad-gate-golden-vectors-7-2.json:8`, `:19`, `:47`]. (a) `amplitude_short=1000` is
   called *"a LITERAL derived offline from 32767"* — it is an arbitrary round amplitude; the other
   five genuinely are `ceil(target · 32767)`. (b) *"comfortably ABOVE … (0.02)"* — 0.021566 is only
   7.8 % above. (c) VAD-GATE-002/005 described as *"at threshold (>= semantics)"* — `ceil` places them
   strictly above, so flipping `>=` to `>` leaves all six green; the `>=` boundary is actually pinned
   by `SilenceThresholdTest.kt:75,:94`. Also the class KDoc `VadGateGoldenVectorsTest.kt:33-37`
   implies the vectors guard the divisor — they do not (that is `VadGateRmsFixtureTest.kt:43`).
   **Fix:** state what each vector actually pins.
7. **R3-P7 — four stale story-record claims** [`7-2-…md:350`, `:564`, `:244`]. (a) R2-D2 still asserts
   in present tense that `android-smoke.sh` applies no gradle patches — GATE-4 made that false in the
   same commit. (b) The `### File List` entry for `vad-gate-golden-vectors-7-2.json` still describes
   `target_normalized_rms` + Nyquist-square, which that commit deleted/changed. (c) The main
   `### File List` omits `scripts/android-smoke.sh` and `src-tauri/src/pipeline.rs`. (d) Task 7's
   sub-bullet still claims no Rust changes beyond the AC5 doc-comment, yet carries a test-only
   `pipeline.rs` arm. **Fix:** correct all four.
8. **R3-P8 — stale `KlarvoAudioRecorder` KDoc** [`KlarvoAudioRecorder.kt:36-37`]. Verified verbatim
   today: *"The RMS energy gate is kept as a pre-filter: frames below [energyGateThreshold] are
   treated as silence without even calling the VAD model, saving CPU."* — but `vadGateDecision:255`
   calls `isSpeech(filteredFrame)` **unconditionally**.
   **Fix: DELETE the CPU-saving clause. Do NOT add the short-circuit** — Silero is stateful and a
   short-circuit would change its window trajectory (see `VadGateResult` KDoc warning at `:224-229`).
   Companion nit: the state-machine ASCII arrows at `:502-504` are at display columns 52/52/**53**.
   - **R3-P8 EXT (in scope, added on review directive)** — a **second copy of the same false claim**
     lives in the `energyGateThreshold` constructor-param KDoc at **`KlarvoAudioRecorder.kt:47-50`**,
     verified verbatim today: *"RMS energy gate threshold (normalized 0..1). Frames below this are
     treated **as silence without calling the VAD model**. Defaults to 0.005, which matches the Rust
     default_silence_threshold() in src-tauri/src/config/mod.rs:209."* R3-P8 names only the class KDoc
     at `:36-37`. **Fix: strike the false "without calling the VAD model" claim here too, with the
     same remedy — delete the claim, do NOT add the short-circuit.** Fixing only `:36-37` leaves the
     tree still asserting the untrue thing four lines below the class it describes.
     > **Verified, leave alone:** the trailing anchor `src-tauri/src/config/mod.rs:209` in that same
     > KDoc is **correct on today's tree** (`fn default_silence_threshold() -> f32 { 0.005 }`), and the
     > `0.005` value matches. Only the VAD-model clause is false — do not "fix" the anchor or the
     > default, and do not touch the Story-9-11 provenance paragraph at `:51-55`.

> The 8 `[Review][Defer]` items in the same section (lines 428-435) are **out of scope** — they are
> already accepted as deferred and are not part of this story's AC set.
>
> **The two "adjacent instances" that earlier drafts of this story parked as out-of-scope are now
> IN scope** as R3-P2 EXT and R3-P8 EXT above (review directive, 2026-09-10). Nothing else was
> widened: AC5 is **8 findings + exactly 2 named extension sites**, no more.

### AC6 — `android-smoke.sh` traps (the story's own gate bites it otherwise)

**Given** two verified traps in the very script this story uses as its gate,
**When** they are fixed,
**Then**:

- **(a) Stale-file prune.** `scripts/android-smoke.sh:157` (`cp "$SRC"/*.kt "$DST/"`) and `:176`
  (`cp "$TEST_SRC"/*.kt "$TEST_DST/"`) copy **without pruning**. The laptop tree still carried
  `WavRmsVectorsTest.kt` + 2 siblings deleted in `652f128` (2026-07-12) and **failed the JVM gate on
  a fixture change**. **Fix:** sync with delete — `rsync --delete` **or** clear-then-copy — for
  **both** `kotlin-src` and `kotlin-test`.
- **(b) arm64 install on an emulator.** `:262` installs with a plain
  `${ADB} -s "$DEVICE_SERIAL" install -r "$APK_PATH"`. On the x86_64 AVD, Android picks the x86_64
  split, which has **no `libklarvo_lib.so`** (the Rust core is arm64-only) → every JNI call dies with
  `UnsatisfiedLinkError`. This hit the 7-2 stop-path oracle on 2026-09-10; the smoke stayed green
  because the VAD-config cases never reach JNI. **Fix:** when the serial matches `emulator-*` (the
  script already branches on this at `:136-138`), install with `--abi arm64-v8a -r -g`.
  `scripts/android-emulator-smoke.sh` **already does this correctly — copy that, do not invent it.**

**And** the fallback re-install path at `:265-266` (`uninstall` → `install`) stays consistent with the
new flags,
**And** the `gen-android-theme.mjs --check` gate (`:147`) and the JVM gate (`:201`) are unchanged.

### AC7 — Gates replace CI (documented, one paragraph)

**Given** there is **no CI** in this repo,
**When** a future maintainer asks how to run the parity net,
**Then** `test-fixtures/README.md` documents — in **one paragraph** — the two commands that run the
whole net:
- `scripts/android-smoke.sh` (the JVM gate: `./gradlew :app:testUniversalDebugUnitTest`, `:201`)
- `cargo test --lib` in `src-tauri/`

**And** it lists the fixtures currently in the net (`chunking-cleanup-vectors.json`,
`wav-rms-vectors.json`, `vad-gate-golden-vectors-7-2.json`, plus the two added by this story),
**And** it states plainly that **no CI runs these** — they are manual gates.
**One paragraph. Do not write a test-strategy document.**

### AC8 — Inversion check (mandatory, at writing time)

**Given** project-context: *a green measurement can prove the wiring and still miss the claim*, and
7-1's review found **6 of 8** tests stayed green against the unfixed code,
**When** each new lock is written,
**Then** the drift is **deliberately re-introduced and shown RED — at writing time, not at review**,
for at least:
1. the **AC2 boundary guard** (e.g. add a dummy Kotlin `HallucinationFilter`-shaped declaration → RED,
   then remove it),
2. the **AC3 twin-constant lock** (e.g. flip Kotlin `max_tokens` to 1024, or the join to `\n\n` → RED),
3. **at least one repaired AC5 assertion** (e.g. drop the `vadGateDecision` seam call, or remove the
   fixture key that R3-P2 made throwing → RED),

**And** each inversion is recorded in the Dev Agent Record as a table: *reverted change → red test*
(follow 7-1's format, `7-1-…md:171-180`),
**And** the tree is **clean** afterwards (`git status`) — every inversion is reverted.

## Tasks / Subtasks

- [x] **Task 1 — M9: DeepSeek URL parity** (AC1)
  - [x] Change `KlarvoApi.kt:164` and `:198` to `https://api.deepseek.com/v1/chat/completions`.
  - [x] Consider extracting the literal to one `private const val` so the two sites cannot drift
        again (the file already comments at `:194` that the *candidate list lives in exactly one
        place* — honor that intent). Judgement call: only if it does not disturb the surrounding
        `LlmProviderInfo` construction. → **Extracted** as `KlarvoApi.DEEPSEEK_CHAT_URL`; both sites
        now read `url = DEEPSEEK_CHAT_URL`, the named-argument `LlmProviderInfo` construction is
        otherwise untouched.
  - [x] Add the JVM test pinning the URL for **both** sites.
  - [x] Do **not** touch `commands/settings.rs:979`.

- [x] **Task 2 — ADR-0017 boundary guard** (AC2)
  - [x] Decide the host: JVM test in `android/kotlin-test/` **or** a step in `scripts/android-smoke.sh`.
        → **JVM test** (the recommended default): `Adr0017BoundaryGuardTest.kt`, device-free, runs in
        the same `:app:testUniversalDebugUnitTest` gate as everything else. No new pipeline/script/CI file.
  - [x] Implement so it is **GREEN on the clean tree** — honor the comment/KDoc + allowlist trap above.
        → Guard strips comments **string-aware** (a naive `substringBefore("//")` would truncate at the
        `//` in a URL literal) and keys on *declarations*, not name appearances; allowlists
        `GroqSttBridge.kt` + `LocalWhisperInference.kt`. Green on clean tree.
  - [x] Prove RED by adding a fake Kotlin STT/guard declaration, then revert (AC8). → all 4 rules fired.

- [x] **Task 3 — Twin-constant fixture + both harnesses** (AC3)
  - [x] Add the fixture to `test-fixtures/` (suggested `twin-constants-vectors.json`, matching the
        existing `*-vectors.json` naming). Flat JSON array of objects, one per constant, each with
        `id`, `description` (what it pins), and the expected value. → 5 entries; each description
        carries an explicit `PINS:` / `DOES NOT PIN:` pair, and both harnesses assert that.
  - [x] Rust test: reuse the established loader shape (`llm/mod.rs:1949-1958` /
        `pipeline.rs:4151-4164`) — `CARGO_MANIFEST_DIR` → `.parent()` → `join("test-fixtures/…")`.
  - [x] Kotlin test: reuse the repo-root candidate resolver (`ChunkingVectorsTest.kt:108-124`).
        **Prefer `org.json.JSONObject`/`JSONArray`** … → used `org.json` with the throwing
        `getDouble`/`getInt`/`getString`; no third copy of the hand-rolled `JsonVal` parser.
  - [x] Assert against production symbols, not re-declared literals.
        → Rust: `OpenAiCompatibleCleanup::DEFAULT_TEMPERATURE`/`DEFAULT_MAX_TOKENS`,
        `CHUNK_THRESHOLD`, `CHUNK_TARGET_SIZE`, and the real `chunked_cleanup` join loop driven
        through the existing `MockCleanupProvider`.
        → Kotlin: `KlarvoApi.CLEANUP_TEMPERATURE`/`CLEANUP_MAX_TOKENS` (newly *named*, values
        unchanged — the literals were inline in the request body and unreachable from a test),
        plus the 7-1 seams `shouldChunk` / `splitIntoChunks` / `joinChunkResults` probed
        behaviourally so the private chunk constants stayed private (no chunking edit).
        Each side compares production → **fixture literal**, never to another production symbol.

- [x] **Task 4 — M12 current-state vector** (AC4)
  - [x] Record today's divergence as a fixture entry, explicitly labelled current-state.
        → `test-fixtures/m12-dictionary-scope-vectors.json`; every entry carries
        `record_type: "current-state-record"` + `open_decision: "M12"`, and the Rust test asserts
        that label on every entry. Per-platform values use the established
        `expected_… / expected_…_kotlin` shape from `wav-rms-vectors.json`.
  - [x] Change **no** prompt-assembly code on either platform. → confirmed; the only Rust edit is
        an added `#[cfg(test)]` test, and no Kotlin prompt function was touched (which is also why
        the Android column is a *recorded*, not machine-asserted, value — stated in the fixture).

- [x] **Task 5 — 7-2 residuals** (AC5)
  - [x] Fix R3-P1 … R3-P8 exactly as prescribed above (8 items).
        R3-P1: three claims corrected to name the remaining gap (test block comment, assertion
        message, 7-2 line 333) — the "or the seam call was dropped" over-claim is gone.
        R3-P2: `JsonVal.getDouble` throwing accessor, used for `amplitude_short`/`signal_freq_hz`.
        R3-P3: `VadGateResult` now captured — asserts `normalizedRms`, adds a `threshold = 1f`
        gate-closed case and an `isSpeech = { false }` case (pins `&&` against both `vadSpeech`-alone
        and `||`), plus the `> 1e-4f` lower bound.
        R3-P4: replaced the tautology with a **genuine bypass inversion** (drives the real seam with
        a 1 Hz near-all-pass filter and shows the discriminating assertion fails); corrected the
        "toward zero" comment to the measured ~14.4 % of raw and noted the DC choice's insensitivity
        to `HIGHPASS_CUTOFF_HZ`.
        R3-P5: cite by content — verified the block really spans `android-build.sh:206-215`.
        R3-P6: three fixture descriptions (VAD-GATE-001/002/005) + the class KDoc rewritten to state
        what the vectors do **not** pin (`>=` boundary → `SilenceThresholdTest`; divisor →
        `VadGateRmsFixtureTest`). Numeric fields provably untouched.
        R3-P7: all four story-record claims corrected (a: R2-D2 marked resolved-not-deferred;
        b: fixture File List entry; c: `android-smoke.sh` + `pipeline.rs` added to the main File
        List; d: Task 7's "no Rust changes beyond…" bullet).
        R3-P8: CPU-saving clause deleted (no short-circuit added) + the arrow nit at the
        state-machine block (measured on `7422a86`: the third arrow at col 53 vs its two
        siblings' 52 — matching the AC5 text; all three now sit at 52).
  - [x] Fix **R3-P2 EXT** — same throwing-accessor remedy at `silence_threshold` and `silence_secs`.
  - [x] Fix **R3-P8 EXT** — second copy of the false claim struck in the `energyGateThreshold`
        param KDoc; **no** short-circuit added; `config/mod.rs:209` anchor and the `0.005` default
        verified correct on today's tree and left alone, as was the Story-9-11 provenance text.
  - [x] Re-verify every line anchor against the tree before editing. → Done; several had drifted
        further than the story predicted (my own edits shifted `KlarvoApi.kt` by +13 lines), so
        every site was located by content.

- [x] **Task 6 — `android-smoke.sh` traps** (AC6)
  - [x] (a) prune on copy for `kotlin-src` **and** `kotlin-test` → clear-then-copy (`rm -f
        "$DST"/*.kt`), not `rsync` (no new host dependency). Verified no generated-only `.kt`
        exists in either destination, and that the `generated/` subdirectory is not matched.
  - [x] (b) `--abi arm64-v8a -g` on `emulator-*`, copied from `scripts/android-emulator-smoke.sh`;
        the `uninstall` → `install` fallback path carries the same flags. Empty-array expansion
        verified safe under `set -euo pipefail`. Preserved: theme gate, JVM gate, org.json sed
        patch, conductor `emulator-*` guard.

- [x] **Task 7 — `test-fixtures/README.md`** (AC7) — one paragraph, two commands, fixture list,
      "no CI" stated. Also records the gradle up-to-date caveat found while running the net (a
      fixture-only edit does not re-run the test task — `--rerun-tasks` needed, or you read a
      stale green).

- [x] **Task 8 — Gates + inversion evidence** (AC8, DoD)
  - [x] `cargo test --lib` in `src-tauri/` green → **662 passed, 0 failed, 0 ignored**
        (baseline 657 + 5 new tests). Coverage statement below.
  - [x] `scripts/android-smoke.sh` JVM gate green → **168 tests, 0 failures, 0 errors** across 22
        suites in the `testUniversalDebugUnitTest` variant (baseline 155 + 13 new).
        Run **device-free**: the script's own gate step was reproduced directly
        (`rm`+`cp` sync of `kotlin-src`/`kotlin-test`, then `./gradlew
        :app:testUniversalDebugUnitTest`), because the script hard-fails on "no device" long
        before it reaches that step. Final run used `--rerun-tasks`.
  - [x] Emulator smoke proves the install fix: fresh APK + **one JNI call succeeds**.
        → **PROVEN on the laptop, 2026-09-10.** The earlier "blocked" note was written from
        powerhouse, which has no AVD (no `emulator/`, no `system-images/`) — but the smoke is
        not run from powerhouse. It runs **on the laptop**, where the AVD is local and
        `DEVICE_SERIAL` is `emulator-5554`, so AC6b's `case … in emulator-*)` branch is the
        matching one (review decision D2). Evidence in
        `_bmad-output/implementation-artifacts/gate4-evidence/7-8/`: `smoke-r1-00e771d.log`
        shows the branch firing (*"Emulator-Ziel — erzwinge arm64-v8a-Split"*) and a successful
        install on `emulator-5554`; `structure-install-r1.txt` shows `primaryCpuAbi=arm64-v8a`,
        the `lib/arm64` nativeloader path, **0** `UnsatisfiedLinkError`/FATAL, and live
        `klarvo_lib::config` / `klarvo_lib::license` log lines from the Rust core — i.e. JNI
        calls executed. **Scope of this proof:** the x86_64-split trap is closed on a local AVD
        with an arm64 APK. It does not cover a physical device (unaffected — no `emulator-*`
        serial), nor any network call.
  - [x] Record the inversion table; confirm `git status` clean. → table below; all **seven**
        inversions reverted and audited item-by-item; no probe or scratch file left in the tree.
  - [~] **[HUMAN GATE — Andi] GATE-4:** one dictation on the Xiaomi with DeepSeek cleanup returns
        cleaned text (proves M9 on the real path). → **PENDING — Andi's gate**, cannot be
        self-served. M9 is proven only at the unit level (both URL sites pinned); no network call
        to DeepSeek was made.

## Dev Notes

### Verified current state (re-checked against today's tree, 2026-09-10)

Every anchor in the ACs above was grep-verified on this branch, **not** copied from the epic text.
Do not trust prose over the tree — re-grep before editing (8-5 wrote "≈24" alias sites, the tree had 46).

### Files to MODIFY

| File | What changes | What must be preserved |
|---|---|---|
| `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` | `:164`, `:198` URL strings **only** | The whole `resolveLlmProvider` fallback ladder (`:146-166`) and `cleanupFallbackCandidates` (`:196-…`) shape — Epic-12 cross-provider fallback depends on it. Do not touch chunking (`:1000-1200`) or the LLM request body beyond what AC3 *reads*. |
| `android/kotlin-test/com/klarvo/voice/HighpassFilterTest.kt` | R3-P1, P3, P4 | The genuinely-passing seam assertions; do not delete coverage while "fixing claims". |
| `android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt` | R3-P2 (throwing accessor at `:202`, `:207`), **R3-P2 EXT** (same accessor at `:195`, `:231`), R3-P6 (class KDoc `:33-37`) | The six energy-floor vectors' numeric intent; the `category == "energy-floor"` / `"stop-latency"` filters. |
| `android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt` | R3-P8 class KDoc `:36-37` (**delete clause**), **R3-P8 EXT** param KDoc `:47-50` (**delete the same clause**), `:502-504` arrow nit | **No behavior change.** Do NOT add the short-circuit. Keep the `config/mod.rs:209` anchor, the `0.005` default, and the Story-9-11 provenance text at `:51-55`. |
| `test-fixtures/vad-gate-golden-vectors-7-2.json` | R3-P6 descriptions `:8`, `:19`, `:47` | Every numeric field — descriptions only. Changing a number silently re-scopes 7-2's ACs. |
| `scripts/android-smoke.sh` | `:157`, `:176` prune; `:262` (+`:265-266`) abi flags; `:184` comment anchor | `:147` theme gate, `:201` JVM gate, the `org.json` sed patch, the `emulator-*` conductor guard `:136-138`. |
| `_bmad-output/implementation-artifacts/7-2-…md` | R3-P7 four claims (`:350`, `:564`, `:244`) | Everything else — it is a closed `done` story record. |

**NEW files:** the twin-constant fixture + M12 vector in `test-fixtures/`, their Rust and Kotlin tests,
`test-fixtures/README.md`, and (if Task 2 goes the JVM route) the boundary-guard test.

### Existing infrastructure — reuse, do not reinvent

The fixture net already exists. **Three** fixtures, **both** harnesses, in the repo root
`test-fixtures/`:

- `chunking-cleanup-vectors.json` — Rust `llm/mod.rs:1949-1958` ↔ Kotlin `ChunkingVectorsTest.kt:108-124`
- `wav-rms-vectors.json` — Rust `pipeline.rs:4151-4164` ↔ Kotlin `VadGateRmsFixtureTest.kt:69-75`
- `vad-gate-golden-vectors-7-2.json` — Kotlin `VadGateGoldenVectorsTest.kt:123`

**Format:** flat JSON array of objects; `id` + a long prose `description` that states *what the vector
pins and what it does not*; then expectation fields. Follow it exactly.
**Per-platform expectation convention already in use:** `wav-rms-vectors.json` carries `expected_rms`
(read by Rust) alongside `expected_rms_kotlin` (read by Kotlin) on the same entry. If the twin fixture
ever needs to express a per-platform value — notably the **AC4 M12 current-state vector**, where the
two platforms legitimately differ today — use that established shape rather than inventing one.

**Kotlin test conventions:** JUnit **4** (`org.junit.Test`, `org.junit.Assert.*`), package
`com.klarvo.voice`, files in `android/kotlin-test/com/klarvo/voice/`. Two JSON-reading styles coexist:
`org.json` (`VadGateRmsFixtureTest`, `MinRecordingMsConfigTest`) and a hand-rolled `JsonVal` parser
duplicated in `ChunkingVectorsTest` + `VadGateGoldenVectorsTest`. **Use `org.json` for new work** — it
throws on missing keys, which is the R3-P2 remedy, and it avoids a third copy of the parser.

**The SUT must not judge itself.** Both existing fixture tests carry this comment verbatim
(`ChunkingVectorsTest.kt:128-131`): the triviality predicate is re-implemented independently so that
flipping the production guard cannot blind the test. Apply the same discipline to the twin-constant
test where it makes sense — but note the tension with "assert against the production symbol" (AC3):
read the constant from production, compare it to the **fixture literal**, never to another production
symbol.

### Previous-story intelligence (7-1, 7-2, 7-3 — the three stories that built this net)

- **7-1 (chunking):** the review found the committed Kotlin **did not compile**, so the "all tests
  pass" claim was unbacked, and **6 of 8** tests stayed green against the unfixed code. Remedies now
  standing: (a) a device-free **compile-verify** of the real `android/kotlin-src/**` with
  `kotlin-compiler-embeddable` is the standing substitute when no device is used; (b) production
  **seams** were extracted (`shouldChunk`, `collectChunkResults`, `joinChunkResults`) so tests bind to
  production code. **Its inversion table (`7-1-…md:171-180`) is the format AC8 expects.**
- **7-2 (VAD gate):** three review rounds; the loop ended at a fix cap with these 8 residuals. The
  recurring defect class across all three rounds is **claim accuracy** — comments and test names
  asserting coverage the tests do not have. This story is the cleanup of exactly that class; do not
  reproduce it. Its own coverage statement names what it did not exercise — copy that habit.
- **7-3 (STT consolidation):** deleted the Kotlin twins and promised in AC9 that *"7.7 later pins this
  as a golden-vector"*. **AC2 of this story is that promise.** 7-3's review also fixed a JNI Unicode
  panic and a 5xx-retry regression — evidence that the bridge is load-bearing and must not be touched
  here.

### Git intelligence (last commits on this line)

`22140c2` applied the correct-course re-cut (epic doc, sprint-status, backlog, routing hook) —
this story is its direct output. `3d7de0d`/`40fdc96`/`16eae82` closed 7-2's real-device gate and
**recorded the arm64-install trap in the backlog** (AC6b). `71d08c0` moved 7-2's residuals to the
backlog (AC5). Pattern to follow: **small scoped commits, never `git add .`**, story artifacts
committed per-story, English commit subjects.

### Gates and how to run them (project-context, non-negotiable)

- **Kotlin compiles and unit-tests run DEVICE-FREE.** `scripts/android-smoke.sh` copies
  `kotlin-src` + `kotlin-test` into the generated project and runs
  `./gradlew :app:testUniversalDebugUnitTest` as a hard gate. Never claim "this needs a device" for a
  pure-logic change — nearly all of this story is pure logic.
- **Emulator:** boot **only** via `scripts/android-emulator.sh` (never hand-roll `emulator -avd`; the
  script arms the TTL watchdog). Stop it explicitly with `scripts/android-emulator.sh stop`.
  **Known topology constraint (corrected 2026-09-10, review decision D2):** powerhouse has **no
  AVD** (`tools/android-sdk/` lacks `emulator/` + `system-images/`). The AVD lives on the
  **laptop**, and the smoke is run **there, locally** — it is *not* a remote emulator reached
  from powerhouse over TCP. The serial in that run is therefore `emulator-5554`, which is exactly
  what AC6b's `emulator-*` branch matches (verified 2026-09-10; evidence in
  `_bmad-output/implementation-artifacts/gate4-evidence/7-8/`). AC6b is bounded to a locally
  attached AVD by design. If no emulator is reachable at all, report the gate `blocked`; do
  **not** install tooling around it.
- **Never mutate the host to reach a gate** (no `apt`, `rustup target add`, `cargo install`,
  `npm install -g`).
- **`gen/android/` is generated and gitignored** — every durable edit goes in `android/kotlin-src/`,
  `android/kotlin-test/`, or the scripts. Files hand-placed in `gen/` vanish.
- **No Windows build required.** This story touches no desktop surface/UI; `windows-build.sh` is not
  in its DoD.

### Scope guards (explicit — from the epic)

**Out of scope:** the dead-config cluster (backlog OPEN-DECISION), `dictation-quality-audit.py`
(backlog tooling), and **any** change to STT, VAD, JNI, config schema or UI.
Also out of scope: the 7 `[Review][Defer]` items in 7-2 round 3; M12 itself (7.6 decides);
`commands/settings.rs:979`.

This story is **independent of 7.6** and runs next. Epic 7 may close with 7.6 parked.

### Project Structure Notes

- Fixtures live in the **repo-root** `test-fixtures/` (not under `src-tauri/` or `android/`) — both
  harnesses resolve it by walking up from their own working directory. Keep the new fixture there or
  both loaders break.
- Rust tests are **inline `#[cfg(test)]` modules** — do not add a file under `src-tauri/tests/`
  (the only sanctioned integration suite there is `pi_security.rs`).
- Naming: `snake_case.rs`, Kotlin `PascalCase` classes under `com.klarvo.voice`, tests
  `<Subject>Test.kt`. Code and comments **English**; chat German.
- No new dependency is introduced by this story. The unit-test classpath already carries
  `junit:junit:4.13.2` and `org.json:json:20231013`
  (`src-tauri/gen/android/app/build.gradle.kts:71-72`, the latter injected by the grep-guarded sed
  patch in both scripts). **Do not run `npm install` or re-resolve any lock** — the Windows build runs
  `npm ci` and a re-resolve breaks it.
- `android/kotlin-test/` is the **git-tracked SSOT**; `src-tauri/gen/android/app/src/test/java/…` is a
  plain `cp` copy. Edit only the tracked tree — AC6a exists precisely because that copy never prunes.

### References

- [Source: _bmad-output/planning-artifacts/epics-cross-platform-parity.md#Story 7.8] — outcome-level ACs, DoD, out-of-scope.
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-2026-09-10.md] — the re-cut, evidence table, sequencing, epic-close rule.
- [Source: docs/adr/0017-shared-core-stt-path.md] — Hard Rule (AC2), scope (STT-only).
- [Source: docs/adr/0016-android-path-parity-strategy.md#Amendment 1,2] — parity line; LLM routing stays per-row (M9).
- [Source: _bmad-output/implementation-artifacts/7-2-android-live-auto-stop-vad-gate-parity.md:355-424] — round-3 findings (AC5), verbatim.
- [Source: _bmad-output/implementation-artifacts/7-3-shared-core-stt-request-and-guard-path-via-jni.md#AC9] — the boundary promise AC2 fulfills.
- [Source: _bmad-output/implementation-artifacts/7-1-android-chunking-parity-core-output.md:145-184] — seam extraction + inversion-table format.
- [Source: docs/backlog.md:1192-1225] — "Story 7-2 residuals" incl. both `android-smoke.sh` fix directions.
- [Source: src-tauri/src/llm/mod.rs:473-474,719,1236,1240,1407,1949-1958,228-255] — Rust twins, DeepSeek URL, fixture loader, M12 Chat arm.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoApi.kt:164,198,783-788,945,971-972,1000-1001] — M9 sites, dictionary append, Kotlin twins.
- [Source: android/kotlin-test/com/klarvo/voice/ChunkingVectorsTest.kt:108-131] — repo-root resolver + "SUT must not judge itself".
- [Source: scripts/android-smoke.sh:136-138,147,157,176,184,201,262-266] — gate steps and both traps.
- [Source: _bmad-output/project-context.md] — device-free Kotlin gate, emulator rules, no-host-mutation, grep-before-done, "a number states what it covers".

## Dev Agent Record

### Agent Model Used

Claude Opus 5 (`claude-opus-5`), via `bmad-dev-story`.

### Debug Log References

- JVM gate reproduced device-free: sync `android/kotlin-src` + `android/kotlin-test` into
  `src-tauri/gen/android/app/src/{main,test}/java/com/klarvo/voice`, then
  `./gradlew :app:testUniversalDebugUnitTest`. `scripts/android-smoke.sh` itself cannot be used for
  this because it hard-fails at "kein Gerät gefunden" before reaching its own JVM gate.
- **Gate weakness found while running the net (not fixed — outside AC6's two named traps):** the
  gradle test task is UP-TO-DATE when only a *fixture* JSON changed, since the fixtures are not
  declared task inputs. The first attempt at the R3-P2 inversion reported a false GREEN for exactly
  this reason; it only went RED under `--rerun-tasks`. Recorded in `test-fixtures/README.md` so the
  next person does not read a stale green.
- Two encoding traps hit while authoring `Adr0017BoundaryGuardTest.kt`, both self-inflicted and
  fixed: a stray NUL byte in a char literal, and `/*` sequences inside a KDoc (`voice/*.kt`,
  `kotlin-test/**`) — Kotlin block comments **nest**, so those opened comments that never closed.
- **Fix round 1 (2026-09-10).** JVM gate reproduced the same device-free way as the first run
  (`rm`+`cp` sync of both trees, then `./gradlew :app:testUniversalDebugUnitTest --rerun-tasks`),
  with `JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64` and
  `ANDROID_HOME=/home/andyon2/workspace/tools/android-sdk` — gradle fails with *"SDK location not
  found"* without the latter, which `scripts/android-smoke.sh:45-46` exports for you.
- **Fix round 1 — a fixed inversion has to fail differently than the bug did.** For P8 the obvious
  inversion (85 Hz cutoff) would have tripped the *old* assertion too and proved nothing about the
  new band. The inversion actually used drives the cutoff the other way (0.05 Hz → 99.6 % survival):
  it passes the dropped `> 0.5f` check and fails only the band. Same reasoning behind D and F —
  each is placed exactly where the pre-fix code was blind.

### Completion Notes List

**AC1 (M9)** — Both DeepSeek sites now use `/v1/chat/completions`, extracted to a single
`KlarvoApi.DEEPSEEK_CHAT_URL`. Three JVM tests pin it: the primary-selection path, the Epic-12
fallback-ladder path, and a drift assert that the two agree with each other. `commands/settings.rs`
(`/v1/models`) untouched, as required. RED proven before the fix: exactly the two site pins failed
while the "both agree" test passed — correct, since both sites were wrong the same way.

**AC2 (ADR-0017 guard)** — `Adr0017BoundaryGuardTest`, a JVM test (no new pipeline/script/CI file).
It strips comments **string-aware** and asserts on *declarations*, so the KDoc that documents the
7-3 deletion does not trip it; `GroqSttBridge.kt` and `LocalWhisperInference.kt` are allowlisted and
`transcribeWithRetry` is deliberately not keyed on. A third meta-test proves the stripper itself is
not blind (it must keep a URL string intact while removing a trailing `//` comment).

**AC3 (twin lock)** — One fixture, both harnesses, five twins. Kotlin asserts through the 7-1 seams
(`shouldChunk` boundary probe, `splitIntoChunks` fallback-split probe, `joinChunkResults`) so the
private chunk constants stayed private and no chunking code was touched. The two request-body
literals had no reachable symbol at all, so they were *named* (`CLEANUP_TEMPERATURE`,
`CLEANUP_MAX_TOKENS`) — values identical, no behavior change. Rust drives the real
`chunked_cleanup` join loop through the existing `MockCleanupProvider` rather than re-implementing
the join. `AnthropicCleanup`'s separate pair is coincidence-checked and labelled as *not* the
locked twin; `llm/local.rs`'s pair is target-gated to Windows and unreachable here (stated in the
fixture).

**AC4 (M12)** — Recorded, not decided. No prompt-assembly code changed on either platform. The
desktop column is machine-asserted against the real `CleanupStyle::system_prompt`; the Android
column is a *written* record because both Kotlin prompt builders are private and reachable only
from inside the network-calling function — opening a seam would have violated AC4. The fixture says
so explicitly rather than implying both sides are asserted.

**AC5 (7-2 residuals)** — All 8 findings plus both named extension sites fixed. R3-P4 was closed
with the stronger option (a genuine bypass inversion through the real seam, not a rename). The
7-2 record's round-3 **checkboxes were initially left unchecked** (the Files-to-MODIFY guard says to
change only the R3-P7 claims plus R3-P1's line 333 and to preserve "everything else" in that closed
record) and flagged for Andi rather than decided unilaterally. **Review decision D1 resolved this:
all eight are now ticked**, each with a dated *"Resolved 2026-09-10 by Story 7-8 (commit `00e771d`)"*
note appended. The original finding text is kept verbatim — the record still says what was wrong, it
just no longer says it is still true.

**AC6** — Both traps fixed. (a) is verified by construction (no generated-only `.kt` in either
destination). (b) is **runtime-proven on a local AVD** (`emulator-5554`, on the laptop) —
arm64-v8a split installed, 0 `UnsatisfiedLinkError`, Rust-core log lines present; evidence and
scope in Task 8. The `emulator-*` branch is deliberately bounded to a locally attached AVD
(review decision D2); a physical device does not take it and does not need to.

**AC7** — One paragraph, two commands, fixture table, "no CI" stated plainly.

**Coverage statement — what the green numbers do and do NOT cover.**
`cargo test --lib` 662/0 and the JVM 168/0 are *logic* results on Linux/JVM. What they exercise:
pure functions, seams, fixture agreement, and static source structure. What they do **NOT**
exercise, and what therefore remains unproven by this story:
- **No physical Android device.** A local **AVD** (`emulator-5554`, laptop) was driven for AC6b
  only: fresh APK installed as the arm64-v8a split, JNI reached (Rust-core log lines, 0
  `UnsatisfiedLinkError`). That proves the install fix and nothing else — no dictation, no
  gesture, no overlay and no HyperOS behaviour was exercised on it.
- **No network call** to DeepSeek, Groq or any provider. AC1 proves the URL *string* both resolvers
  emit; it does not prove a request succeeds. That is GATE-4, Andi's.
- **No desktop/Windows build and no UI** — this story touches no surface, so `windows-build.sh` is
  correctly not in its DoD.
- **AC2's guard** covers only `android/kotlin-src/com/klarvo/voice/*.kt` and only four syntactic
  shapes; a semantic re-implementation under different names would pass it.
- **AC3's two halves cannot prove each other** — each asserts its own platform against the shared
  fixture literal.
- **AC4's Android column is not machine-asserted** (reason above).
- The JVM count is the `testUniversalDebugUnitTest` variant only; gradle also ran nine other
  ABI/buildtype variants that duplicate the same suite.

**Fix round 2 (code review round 2) — 11 findings resolved.** Full per-finding detail in
*Review Findings — code review round 2 → Fix round 2 — resolutions*. In short:
- ✅ Resolved review finding [Decision R2-D1]: the M12 `any(platforms_agree == false)` assertion is
  gone (option (a)); "M12 is open" is carried by the fixture's `open_decision` field, and the
  per-entry consistency check stays.
- ✅ Resolved review findings [Patch R2-1, R2-2, R2-10]: `VadGateGoldenVectorsTest`'s accessor KDoc
  now names all three `optString` uses and why none can fake a pass, counts **two**
  `expected_gate_open: false` vectors, and `getBool`'s error message names the wrong-type case.
- ✅ Resolved review findings [Patch R2-4, R2-5]: the unfailable separator-count assertion deleted;
  `EmptyFirstChunkProvider`'s doc now matches `starts_with('a')`.
- ✅ Resolved review finding [Patch R2-6]: the ADR-0017 guard KDoc says "anywhere in production
  `voice/`", matching its own coverage block.
- ✅ Resolved review findings [Patch R2-3, R2-7, R2-8]: three record rows corrected — D2 cited by
  content, inversion row E marked non-discriminating **with a measurement**, P10's row names all
  three dropped README conventions.
- ✅ Resolved review finding [Patch R2-9]: `gate4-evidence/7-8/NOTE-jvm-test-counts.md` reconciles
  the logs' one-suite "24" with the run total 168/22 suites and states the evidence's scope.

**What this round did NOT touch:** no production Kotlin or Rust, no fixture JSON, no script — the
deferred `android-smoke.sh` counting bug and D2's `emulator-*` block are deliberately unchanged.
The one code-behaviour experiment (R2-7's index-based drift) was reverted; `git diff` on
`KlarvoApi.kt` is empty.

**Inversion table (AC8) — every drift re-introduced at writing time, measured, then reverted.**

| # | AC | Reverted change (deliberate drift) | Result |
|---|----|------------------------------------|--------|
| 1 | AC1 | (pre-fix state) both DeepSeek sites without `/v1` | 🔴 `deepseekUrl_primarySelection…`, `deepseekUrl_fallbackLadder…` — 2 failed |
| 2 | AC2 | added `object HallucinationFilter` + `object SilencePreFilter` + `fun buildMultipartBody` + `audio/transcriptions` + `multipart/form-data` in a new Kotlin file | 🔴 `noKotlinSttRequestOrGuardTwinHasRegrown` (all 4 rules fired) + `deletedTwinFilesHaveNotReappeared` |
| 3 | AC3 | Kotlin `CLEANUP_MAX_TOKENS` 2048 → 1024 | 🔴 `cleanupMaxTokensMatchesFixture` |
| 4 | AC3 | Rust `CHUNK_TARGET_SIZE` 350 → 300 | 🔴 `spec_twin_constants_chunking_boundaries` + `spec_twin_constants_chunk_join_separator` |
| 5 | AC5 (R3-P2) | deleted `amplitude_short` from VAD-GATE-001 (`expected_gate_open: false` — the exact vector that used to pass vacuously) | 🔴 `IllegalStateException: fixture vector is missing required numeric key 'amplitude_short'` (only under `--rerun-tasks`; see Debug Log) |
| 6 | AC5 (R3-P3) | Kotlin `energyAboveGate && vadSpeech` → `vadSpeech` | 🔴 `vadGateDecision_combinesEnergyGateAndVadWithAnd_notOr` |
| 7 | AC4 | flipped M12 chat vector to `expected_dictionary_in_prompt: true` | 🔴 `spec_m12_dictionary_scope_current_state_still_holds` |

All seven reverted; `git status` carries no probe or scratch file (audited item-by-item).

### File List

**Modified**
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` — `DEEPSEEK_CHAT_URL` const + both call sites
  (AC1); `CLEANUP_TEMPERATURE` / `CLEANUP_MAX_TOKENS` named and used in the request body (AC3).
- `android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt` — R3-P8 class-KDoc clause deleted,
  R3-P8 EXT param-KDoc clause deleted, state-machine arrow alignment. No behavior change.
- `android/kotlin-test/com/klarvo/voice/LlmFallbackProviderTest.kt` — 3 DeepSeek URL tests (AC1).
- `android/kotlin-test/com/klarvo/voice/HighpassFilterTest.kt` — R3-P1 claim corrections, R3-P3
  result assertions + AND-combination test, R3-P4 genuine bypass inversion.
- `android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt` — R3-P2 + EXT throwing
  `getDouble`, R3-P6 class-KDoc correction.
- `test-fixtures/vad-gate-golden-vectors-7-2.json` — R3-P6 descriptions only (3 lines; numeric
  fields provably unchanged).
- `scripts/android-smoke.sh` — AC6a prune-on-copy (both trees), AC6b arm64 install flags on
  `emulator-*` incl. the fallback re-install, R3-P5 comment cited by content.
- `src-tauri/src/llm/mod.rs` — added `#[cfg(test)]` tests only (5 new): twin-constant lock (AC3) +
  M12 current-state vector (AC4). No production Rust changed.
- `_bmad-output/implementation-artifacts/7-2-android-live-auto-stop-vad-gate-parity.md` — R3-P1
  line-333 claim + R3-P7 (a)(b)(c)(d).
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — status transitions.
- `_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md` — this record.

**New**
- `android/kotlin-test/com/klarvo/voice/Adr0017BoundaryGuardTest.kt` (AC2)
- `android/kotlin-test/com/klarvo/voice/TwinConstantsVectorsTest.kt` (AC3)
- `test-fixtures/twin-constants-vectors.json` (AC3)
- `test-fixtures/m12-dictionary-scope-vectors.json` (AC4)
- `test-fixtures/README.md` (AC7)
- `_bmad-output/implementation-artifacts/gate4-evidence/7-8/smoke-r1-00e771d.log` — smoke run on the
  laptop AVD; shows AC6b's `emulator-*` branch firing and the install succeeding (review D2).
- `_bmad-output/implementation-artifacts/gate4-evidence/7-8/structure-install-r1.txt` — the install
  probe: `primaryCpuAbi=arm64-v8a`, `lib/arm64` nativeloader path, 0 `UnsatisfiedLinkError`, live
  Rust-core log lines (review D2).

**Modified in fix round 1 (code review 2026-09-10)**
- `src-tauri/src/llm/mod.rs` — Rust 399/400 boundary probe through `chunked_cleanup` (P2, test is
  now `#[tokio::test]`), `EmptyFirstChunkProvider` + leading-empty join case (P1), M12 assertion
  derived from the fixture columns instead of three hard-coded literals (P5). Tests only; no
  production Rust changed.
- `android/kotlin-test/com/klarvo/voice/Adr0017BoundaryGuardTest.kt` — `honorsAllowlist` and the
  now-dead `allowlist` removed; all four rules apply to every file; KDoc rewritten (P4).
- `android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt` — throwing `getBool` added and
  used for `expected_gate_open`; dead `optDouble`/`optBool` deleted; KDoc narrowed to the truth (P3).
- `android/kotlin-test/com/klarvo/voice/HighpassFilterTest.kt` — redundant complement assertion
  replaced by a 0.85–0.89 band pinning the stated ~87 % (P8).
- `android/kotlin-test/com/klarvo/voice/TwinConstantsVectorsTest.kt` — leading-empty join case (P1).
- `test-fixtures/twin-constants-vectors.json` — JOIN-SEPARATOR and CHUNK-THRESHOLD descriptions now
  match what the two halves actually assert (P1, P2).
- `test-fixtures/m12-dictionary-scope-vectors.json` — README entry states what flipping a vector
  does and does not require (P5).
- `test-fixtures/README.md` — "Conventions worth keeping" paragraph dropped; its one load-bearing
  sentence folded into the fixture table (P10).
- `scripts/android-smoke.sh` — `org.json` comment names three consumers, not two (P9). The AC6b
  `emulator-*` branch is deliberately **unchanged** (D2).
- `_bmad-output/implementation-artifacts/7-2-android-live-auto-stop-vad-gate-parity.md` — all eight
  round-3 `[Review][Patch]` items ticked with dated resolution notes (D1).

**Modified in fix round 2 (code review round 2)**
- `src-tauri/src/llm/mod.rs` — the `any(platforms_agree == false)` M12 assertion deleted and replaced
  by a comment stating why (R2-D1); the unfailable `matches(sep).count() == 0` assertion deleted
  (R2-4); `EmptyFirstChunkProvider` doc corrected (R2-5). Tests only; no production Rust changed.
- `android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt` — `getDouble` KDoc enumerates
  all three `optString` uses (R2-1), `getBool` KDoc counts two not three (R2-2), `getBool` error
  message names the wrong-type case (R2-10). Comment + message text only; no assertion changed.
- `android/kotlin-test/com/klarvo/voice/Adr0017BoundaryGuardTest.kt` — KDoc scope claim corrected to
  "anywhere in production `voice/`" (R2-6). Comment only; the guard rules are unchanged.
- `_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md` — this record:
  D2 row cited by content (R2-3), inversion row E marked non-discriminating with a measurement
  (R2-7), P10 row names three conventions (R2-8), round-2 findings ticked, fix-round-2 resolution
  table + gate results added.
- `_bmad-output/implementation-artifacts/deferred-work.md` — the round's 13 deferred items appended
  under a new *"round 2, scoped re-review of fix round 1"* heading (in `50bdce3`).

*(`sprint-status.yaml` was listed here in error and is now removed: it was last touched by
`00e771d` and already read `7-8-…: review` before this round — `git diff --name-only
27de205..85aa0ee` does not name it. Corrected in fix round 3, round-3 patch 4.)*

**New in fix round 2**
- `_bmad-output/implementation-artifacts/gate4-evidence/7-8/NOTE-jvm-test-counts.md` — reconciles the
  evidence logs' "24 Tests" banner with the record's 168/22-suite total and states the logs' scope
  (R2-9). The `android-smoke.sh` counting fix itself stays deferred.

*Untouched in fix round 2, deliberately:* `scripts/android-smoke.sh` (R2-9's script fix is deferred;
D2 keeps the `emulator-*` block as-is), `test-fixtures/m12-dictionary-scope-vectors.json` and
`test-fixtures/README.md` (R2-D1 option (a) makes the existing fixture text true as written; R2-8
resolves in the record rather than by re-adding a README paragraph), and all Kotlin/Rust production
code.

**Modified in fix round 3 (code review round 3)**
- `android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt` — throwing `getString` accessor
  added and used for the `category` selector in both test loops (R3-6); `getDouble` KDoc re-counted
  to *"three call sites across two keys"* and its `category` bullet moved to the throwing accessor
  (R3-7). The only assertion-level change of the round; no numeric or expectation logic touched.
- `_bmad-output/implementation-artifacts/gate4-evidence/7-8/NOTE-jvm-test-counts.md` — force-add
  convention declared, both aggregate lines quoted verbatim (R3-D1); *"two lines above"* → *"three
  lines above, at `:14`"* and the invalid 22-files-⇒-not-24 inference dropped (R3-8).
- `_bmad-output/implementation-artifacts/deferred-work.md` — the round-2 M12 deferred entry rewritten
  to the surviving defect and re-anchored by symbol (R3-5), + the round's 6 deferred items under a new
  round-3 heading (in `cf39687`).
- `_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md` — this record:
  P3 row points at the KDoc instead of restating it (R3-1), P5 row's superseded clause struck and
  R2-D1's row corrected (R3-2), the AC8 inversion paragraph and the Change Log split into
  unfailable-vs-over-eager (R3-3), fix-round-2 File List corrected in both directions (R3-4), the
  mirrored M12 deferred entry rewritten (R3-5), Change Log "six" → "eight" (R3-9), round-3 findings
  ticked, fix-round-3 resolution table + inversion row H + gate results added.

**Newly committed in fix round 3** (force-added past `.gitignore:15`'s `*.log`, the same route
`smoke-r1-00e771d.log` already took — convention now declared in `NOTE-jvm-test-counts.md`)
- `_bmad-output/implementation-artifacts/gate4-evidence/7-8/smoke-r2-27de205.log` — carries the
  aggregate line `JVM (laptop, 27de205): tests 168 fail 0 err 0 skip 0` at `:44` (R3-D1).
- `_bmad-output/implementation-artifacts/gate4-evidence/7-8/smoke-r3-85aa0ee.log` — carries
  `JVM (laptop, 85aa0ee): tests 168 fail 0 err 0 skip 0` at `:44` (R3-D1).

*Untouched in fix round 3, deliberately:* all Rust (`src-tauri/**` — `cargo test --lib` was re-run
only as a regression gate), all production Kotlin, every fixture JSON, `scripts/android-smoke.sh`,
`test-fixtures/README.md`, and `sprint-status.yaml` (unchanged since `00e771d`). The 6 deferred
items and the 1 residual of round 3 were not touched.

*Not part of this story:* `_bmad-output/implementation-artifacts/seat-costs.jsonl` was already
untracked in the working tree at story start and was left alone.

## Change Log

- 2026-09-10: Story created from the Epic-7 re-cut (`sprint-change-proposal-2026-09-10.md`). All code
  anchors verified against `conductor/story-7-8` at `7422a86`.
- 2026-09-10: **AC5 widened on review directive** — the two sites previously parked as "adjacent
  instances, NOT in this story's AC set" are now in scope as **R3-P2 EXT**
  (`VadGateGoldenVectorsTest.kt:195` `silence_threshold`, `:231` `silence_secs` — same vacuous
  `optDouble` default) and **R3-P8 EXT** (`KlarvoAudioRecorder.kt:47-50` — second copy of the false
  "without calling the VAD model" claim). AC5 is now *8 findings + exactly 2 named extension sites*.
  Task 5 and the Files-to-MODIFY table updated to match. All four line anchors re-verified against
  today's tree; `config/mod.rs:209` (`default_silence_threshold() -> 0.005`) confirmed still correct
  and explicitly excluded from the fix. No other AC, task or scope guard changed.
- 2026-09-10: **Implemented (dev-story).** AC1–AC5, AC7, AC8 complete; AC6 code-complete with its
  emulator proof **blocked** (no device/emulator/AVD reachable from powerhouse — reported, not
  worked around) and GATE-4 pending Andi. Gates: `cargo test --lib` **662 passed / 0 failed**
  (baseline 657 + 5), JVM `testUniversalDebugUnitTest` **168 tests / 0 failures** (baseline 155 +
  13). Seven inversions re-introduced and shown RED at writing time, then reverted; tree clean.
  Only runtime change is the two DeepSeek URL sites; everything else is tests, fixtures, comment
  corrections and two `android-smoke.sh` shell fixes. Two constants in `KlarvoApi.kt` were *named*
  (same values) because AC3 requires asserting against a production symbol and the literals were
  inline in the request body. Status → review.
- 2026-09-10: **Addressed code review round-3 findings — 10 items resolved** (1 decision + 9
  patches; the 6 deferred items and the 1 residual untouched, no scope added). This was an extra
  fix round authorized past the fix cap. Nine of the ten were record/comment/evidence corrections;
  the one code change is **R3-6**, a throwing `getString` accessor for the `category` selector in
  `VadGateGoldenVectorsTest` — under the old defaulting reader a single mistyped `category` removed
  that vector from **both** test loops while the suite still reported 2/0, the vacuous-pass shape
  this story exists to close, sitting in the reader that decides *which* vectors are covered.
  Measured both ways (row H): the drift is 🔴 under the fix and 🟢 under the pre-fix reader.
  R3-D1 (Andi's call, option **c**): both cited smoke logs are now force-added into the evidence
  directory *and* their `tests 168 fail 0 err 0 skip 0` lines are quoted verbatim in the note, so
  the 168 no longer depends on a file a reader may not have; the previously undeclared force-add
  convention is stated in one line. The record corrections all came from the same class — **three
  round-2 resolution rows described code the same commit had deleted** — so every corrected row was
  re-verified against today's tree before editing, and the M12 deferred entry was re-anchored by
  **symbol** rather than by a line range that had overrun the file. Gates re-run: `cargo test --lib`
  **662 passed / 0 failed / 0 ignored** (unchanged — no Rust file was touched), JVM
  `:app:testUniversalDebugUnitTest --rerun-tasks` **168 tests / 0 failures / 0 errors / 0 skipped
  across 22 suites**, aggregated over all result XMLs (unchanged — R3-6 replaced a reader inside two
  existing test functions rather than adding one). Both are Linux/JVM logic results: **no** device,
  **no** emulator, **no** network call. AC6b's runtime proof is still the earlier laptop-AVD
  evidence, and GATE-4 (one Xiaomi dictation through DeepSeek) remains pending with Andi. Status →
  review.
- 2026-09-10: **Addressed code review round-2 findings — 11 items resolved** (1 decision + 10
  patches; the 13 deferred items untouched, no scope added). R2-D1 removed the M12
  divergence-liveness assertion so the fixture's zero-Rust-edit promise for Story 7.6 is true as
  written, while the per-entry `platforms_agree` consistency check stays. The round removed two
  assertions — one **unfailable** (R2-4: the preceding exact-string assertion already determined
  it) and one **over-eager** (R2-D1: it fired on every correct M12 resolution) — corrected eight
  over-claiming or miscounting comments/record rows (R2-1, R2-2, R2-3, R2-5, R2-6, R2-7, R2-8,
  R2-10), and annotated the AC6b evidence so its "24 Tests" banner no longer
  contradicts the record's 168. Row E of the fix-round-1 inversion table is now marked
  non-discriminating — **measured**: the suggested index-based re-record fails the pre-existing
  Story-7-1 test `ChunkingParityTest.h13` as well, so Kotlin's leading-empty join was already
  covered before this story and the new coverage is the Rust half. **No production code, no
  fixture and no script changed in this round.** Gates re-run: `cargo test --lib` **662 passed / 0
  failed / 0 ignored**, JVM `:app:testUniversalDebugUnitTest --rerun-tasks` **168 tests / 0
  failures / 0 errors across 22 suites** — both counts unchanged, since the round deleted and
  reworded assertions rather than adding test functions. Both are Linux/JVM logic results: no
  device, no emulator, no network call; GATE-4 remains pending with Andi. Status → review.
- 2026-09-10: **Addressed code review findings — 12 items resolved** (2 decisions + 10 patches;
  the 7 deferred items untouched, no scope added). D1: 7-2's eight round-3 items ticked with dated
  resolution notes. D2: `android-smoke.sh` left as-is and the story's own "remote TCP proxy"
  topology claim corrected — the smoke runs on the laptop at `emulator-5554`, which also lifts
  **AC6b from blocked to runtime-proven** (evidence committed under `gate4-evidence/7-8/`). The
  net gained two real assertions it had only *claimed*: a Rust 399/400 threshold-boundary probe
  and a leading-empty join case on both sides; the ADR-0017 guard lost its allowlist hole; the
  M12 test no longer hard-codes the values Story 7.6 may flip. Gates re-run: `cargo test --lib`
  **662 passed / 0 failed**, JVM `testUniversalDebugUnitTest` **168 tests / 0 failures**
  (`--rerun-tasks`) — both counts unchanged, since the round added assertions rather than test
  functions. Seven fix-round inversions shown RED at writing time, then reverted; tree clean.
  Status → review.

### Review Findings — code review 2026-09-10 (range `7422a86..00e771d`)

Three layers (Blind Hunter, Edge Case Hunter, Acceptance Auditor); every finding below was
re-verified against today's tree before being recorded. 2 decision-needed · 10 patch · 7 deferred ·
13 dismissed as noise.

**Decision findings (Andi's call — must be resolved before the patch findings):**

- [x] [Review][Decision] 7-2's eight round-3 `[Review][Patch]` items are still unchecked and still assert their defects are live [`_bmad-output/implementation-artifacts/7-2-android-live-auto-stop-vad-gate-parity.md:421-428`] — all eight remain `- [ ]` with present-tense text that 7-8 has made false (e.g. `:428` "The class KDoc line this commit edited **still promises** a CPU short-circuit the code does not implement"; `:421` "three places claim they do"). The dev disclosed this deliberately (Completion Notes → AC5) because the Files-to-MODIFY guard says to preserve everything else in that closed record — yet the same commit *did* apply dated "Corrected 2026-09-10" treatments to five other places in it, including a `[Review][Defer]` item at `:355`. Leaving them is precisely the "net carrying known-false claims" the story's own goal statement forbids. **Options:** (a) check them off / apply the same dated resolved-by-7-8 note to all eight, (b) declare the closed record frozen and record that decision in 7-8 instead.
- [x] [Review][Decision] AC6b's emulator branch cannot match the emulator topology this project documents [`scripts/android-smoke.sh:280-284`] — `case "$DEVICE_SERIAL" in emulator-*)` matches only a locally-attached AVD. That is exactly what AC6b prescribed and it matches the existing conductor guard at `:136-138`, but per this story's own Task 8 the emulator is a remote proxy on the laptop reached over TCP, so `DEVICE_SERIAL` would be `<ip>:5555` → `INSTALL_ABI_FLAGS=()` → the x86_64 split (no `libklarvo_lib.so`) installs again, i.e. the exact `UnsatisfiedLinkError` trap AC6b exists to close survives on the only emulator topology the project documents. **Options:** (a) widen detection (`adb shell getprop ro.kernel.qemu` / `ro.product.cpu.abi`), which contradicts the AC's explicit prescription, (b) keep `emulator-*` as the repo convention and scope AC6b to local AVDs, stating so.

**Patch findings:**

- [x] [Review][Patch] Twin fixture claims a leading-empty join case that exists in neither harness [`test-fixtures/twin-constants-vectors.json` TWIN-CHUNK-JOIN-SEPARATOR-001] — the `DOES NOT PIN` clause says the empty-result skip rule is "asserted by the two-element and leading-empty cases in the tests". No leading-empty case exists: Kotlin joins `["alpha","beta"]` and `["a","b","c"]` (`TwinConstantsVectorsTest.kt:139,145`), Rust drives a boundary-free input yielding three non-empty chunks (`src-tauri/src/llm/mod.rs:2216`). Fix: add `joinChunkResults(listOf("", "beta"))` plus a mock returning an empty first chunk, or strike the claim.
- [x] [Review][Patch] TWIN-CHUNK-THRESHOLD-001 claims a boundary lock only the Kotlin half performs [`test-fixtures/twin-constants-vectors.json` TWIN-CHUNK-THRESHOLD-001 / `src-tauri/src/llm/mod.rs:2168`] — description: "PINS: the effective decision BOUNDARY, not merely a declared number -- Rust CHUNK_THRESHOLD ... and Kotlin KlarvoApi.shouldChunk". The Rust half is only `assert_eq!(CHUNK_THRESHOLD, threshold)`; flipping `raw_text.len() < CHUNK_THRESHOLD` to `<=` leaves it green. The adjacent Rust comment "Behavioural probe, mirroring the Kotlin half exactly" mirrors only the target-size probe. Fix: add a Rust boundary probe at 399/400 bytes through `chunked_cleanup`, or restate PINS as "declared constant (Rust) / boundary (Kotlin)".
- [x] [Review][Patch] The new throwing-accessor KDoc over-claims, and `optDouble` is now dead [`android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:82-83`] — it asserts "Every key whose absence could fake a pass therefore reads through this accessor" and "[optDouble] is kept only for genuinely optional keys". Both false: `:232` still reads `expected_gate_open` via `optBool(default = false)`, so a dropped/typo'd key still passes on the three `expected_gate_open: false` vectors — the exact R3-P2 shape; and `optDouble` (`:70`) now has zero call sites. Fix: add a throwing `getBool` and use it at `:232`, delete the dead `optDouble`, or narrow the KDoc to what is true.
- [x] [Review][Patch] The ADR-0017 allowlist opens the hole AC2 explicitly forbids [`android/kotlin-test/com/klarvo/voice/Adr0017BoundaryGuardTest.kt:56`, rules at `:153`,`:158`] — the `multipart/form-data` and `audio/transcriptions` rules carry `honorsAllowlist = true` for both `GroqSttBridge.kt` **and** `LocalWhisperInference.kt`. AC2 requires flagging `audio/transcriptions` "outside the JNI bridge", and `LocalWhisperInference.kt` is not the bridge. Verified: neither string appears anywhere in `android/kotlin-src/**` today, so the allowlist buys nothing on these two rules and only creates the hole. Fix: set `honorsAllowlist = false` for both rules (or narrow the allowlist to `GroqSttBridge.kt`) — the guard stays green.
- [x] [Review][Patch] The M12 test hardcodes the three values Story 7.6 is told it can flip [`src-tauri/src/llm/mod.rs:2338-2348`] — `chat["platforms_agree"] == Some(false)`, `chat["expected_dictionary_in_prompt"] == Some(false)`, `chat["expected_dictionary_in_prompt_kotlin"] == Some(true)` compare the fixture to test literals, proving nothing about either platform, and they contradict the fixture's own M12-DICT-SCOPE-README claim that "Story 7.6 can flip ONE vector here once the decision is made" — flipping it fails the suite until `llm/mod.rs` is edited too. Fix: derive the claim from the data (e.g. assert exactly one entry has `platforms_agree == false`), or correct the README entry to say the Rust test must change with it.
- [x] [Review][Patch] Dev record miscounts its own inversion evidence [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:431` vs `:657-667`] — `:431` says "all six inversions reverted and audited item-by-item"; the table has seven rows and closes with "All seven reverted". Fix: seven in both places.
- [x] [Review][Patch] The recorded arrow-column measurement is wrong [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:390`] — "measured: col 52 vs its siblings' 51". Measured on `7422a86`, `KlarvoAudioRecorder.kt:502/503/504` sit at display columns **52/52/53**, matching the AC5 text, not the dev record. The code fix itself is correct (all three now at 52). Fix: correct the record to 53 vs 52.
- [x] [Review][Patch] The "genuine bypass inversion" carries a redundant complement assertion and an unpinned number [`android/kotlin-test/com/klarvo/voice/HighpassFilterTest.kt:359-368`] — `assertFalse(bypassedRms < rawNormalizedRms * 0.5f)` is immediately followed by `assertTrue(bypassedRms > rawNormalizedRms * 0.5f)`; the two differ only at exact equality, so the second cannot fail unless the first already did. The comment states "~87 % of the raw amplitude survives" while nothing pins tighter than 50 %. Fix: drop the redundant assertion, or pin the ~87 % figure with a real band so the stated number is covered.
- [x] [Review][Patch] The rewritten `org.json` comment was made stale by its own commit [`scripts/android-smoke.sh:189-190`] — it still enumerates "MinRecordingMsConfigTest, VadGateRmsFixtureTest **both** parse real JSON" while this same commit adds `TwinConstantsVectorsTest` (imports `org.json.JSONArray`/`JSONObject`) as a third consumer, making the patched dependency strictly more load-bearing. Same defect class as R3-P5/R3-P7, re-committed in the block rewritten to fix it. Fix: name the three consumers, or drop the enumeration.
- [x] [Review][Patch] AC7's "one paragraph" guard is exceeded [`test-fixtures/README.md`] — delivered as heading + lead paragraph + commands paragraph + 7-row table + an unasked "Conventions worth keeping" paragraph. AC7's mandatory content (two commands, fixture list, "no CI" stated) is all present; the conventions paragraph is beyond the AC's explicit "Do not write a test-strategy document". Fix: fold or drop it if the guard is honored literally.

**Fix round 1 — resolutions (2026-09-10).** Both decisions and all ten patch findings applied; the
seven deferred items were not touched. What each one became:

| # | Finding | Resolution |
|---|---------|------------|
| D1 | 7-2's eight round-3 items still unchecked | Option (a): all eight ticked in `7-2-…md`, each with an appended dated *"Resolved 2026-09-10 by Story 7-8 (commit `00e771d`)"* note. Original finding text kept verbatim — the record still says what was wrong, it no longer says it is still true. |
| D2 | AC6b's `emulator-*` branch vs. the documented topology | Option (b): the `case "$DEVICE_SERIAL" in emulator-*)` install-ABI block in `scripts/android-smoke.sh` is **unchanged** (byte-identical; cited by content, not by line range — the round-1 citation `:280-284` was already stale and this round's own edit to that file moved it further); AC6b is bounded to a locally attached AVD. The story's own claim was the error — the smoke runs **on the laptop**, where the serial is `emulator-5554`. Task 8 and the Dev Notes topology paragraph corrected, with the evidence that also lifts AC6b from *blocked* to *proven*. |
| P1 | Join fixture claimed a leading-empty case that existed on neither side | Case **added** on both: Kotlin `joinChunkResults(listOf("", "beta"))`, Rust `EmptyFirstChunkProvider` (blanks the first chunk by content, not by call order, which `join_all` does not guarantee). Fixture clause now names both. |
| P2 | TWIN-CHUNK-THRESHOLD-001 claimed a boundary only Kotlin probed | Rust **399/400 boundary probe** added through the real `chunked_cleanup` (single-call path vs. joined chunked path). The test became `#[tokio::test]`. Fixture description now says both halves probe. |
| P3 | `getDouble` KDoc over-claimed; `optDouble` dead | Throwing `getBool` added and used for `expected_gate_open`; the defaulting `optDouble` **and** `optBool` deleted. KDoc rewritten. **This row deliberately does not restate the enumeration** — the authoritative list of which defaulting readers remain, and why none can fake a pass, is the `getDouble` KDoc in `VadGateGoldenVectorsTest.kt` itself, which was narrowed again by R2-1 and again by round-3 patches 6 and 7. Restating it here is what made this row false twice (round-3 patch 1). |
| P4 | ADR-0017 allowlist opened the hole AC2 forbids | `honorsAllowlist` removed **entirely** — all four rules now apply to every file. That left the `allowlist` set and the flag itself dead, so both were deleted rather than left as unused code asserting a false intent. Class KDoc rewritten: the bridge is safe because no rule keys on its vocabulary, which is stronger than an exemption. |
| P5 | M12 test hard-coded the three values 7.6 may flip | Replaced with a **derived** check: `platforms_agree` must equal `desktop == kotlin` for every styled entry. No test literal names Chat. A correct 7.6 flip (prompt code + vector) now needs **no** edit to `llm/mod.rs`; the fixture README says so explicitly. — *This row originally also claimed a second clause, "at least one style still disagrees". That clause was **superseded by R2-D1**, which deleted it (the `any(platforms_agree == false)` assertion) precisely because it fired on either direction of a correct M12 resolution and so falsified the zero-edit promise in this same row. `llm/mod.rs:2390-2396` is now a comment stating why there is no such assertion.* |
| P6 | "six inversions" vs. a seven-row table | Unified to **seven**. |
| P7 | Arrow-column measurement wrong in the Dev Record | Corrected to **53 vs 52** on `7422a86`, matching the AC5 text. The code fix (all three at 52) was already right. |
| P8 | Redundant complement assertion; unpinned "~87 %" | Redundant `assertTrue(x > y*0.5f)` dropped, replaced by a real **0.85–0.89 band** on `bypassedRms / rawNormalizedRms`. The stated number is now covered. |
| P9 | `org.json` comment enumerated two consumers, its own commit added a third | Now names **three**: `MinRecordingMsConfigTest`, `VadGateRmsFixtureTest`, `TwinConstantsVectorsTest`. |
| P10 | AC7's "one paragraph" exceeded | The unrequested "Conventions worth keeping" paragraph **dropped**. It carried **three** conventions, not one: (i) every entry has an `id` + a description stating what it pins **and what it does not**; (ii) per-platform expectations live on the same entry under a `_kotlin` suffix rather than in a second file; (iii) each side asserts a production symbol or seam against the **fixture literal**, never against another production symbol. Only the M12 exception (the Android column is a written record) survived, folded into the fixture table. (iii) is AC3's governing method rule and is stated there — AC3 and the `## Existing infrastructure` Dev Note — rather than re-added to the README, since restoring a paragraph would re-open the AC7 guard P10 exists to honor. All AC7-mandatory content still present. |

**Fix-round inversion table — every new or changed assertion re-introduced as drift, measured, reverted.**

| # | Finding | Reverted change (deliberate drift) | Result |
|---|---------|------------------------------------|--------|
| A | P2 | Rust `raw_text.len() < CHUNK_THRESHOLD` → `<=` | 🔴 `spec_twin_constants_chunking_boundaries` — *"exactly 400 bytes must already take the chunked path"*. **This is the exact drift the pre-fix `assert_eq!` stayed green on.** |
| B | P1 | Rust `if i > 0 && !combined_text.is_empty()` → `if i > 0` | 🔴 `spec_twin_constants_chunk_join_separator` — *"an empty first chunk result must not emit a leading separator"* |
| C | P5 | M12 chat vector `platforms_agree` false → true (columns left disagreeing) | 🔴 `spec_m12_…` — *"chat: platforms_agree must state what the two recorded columns actually show"* |
| D | P4 | added `audio/transcriptions` literal **inside `LocalWhisperInference.kt`** — the file the old allowlist exempted | 🔴 `Adr0017BoundaryGuardTest.noKotlinSttRequestOrGuardTwinHasRegrown`. Pre-fix this probe was green: that was the hole. |
| E | P1 | Kotlin `if (sb.isNotEmpty())` → `if (true)` in `joinChunkResults` | 🔴 `TwinConstantsVectorsTest.chunkJoinSeparatorMatchesFixture` (+ the pre-existing `ChunkingParityTest` pair) — **NON-DISCRIMINATING, corrected in fix round 2 (round-2 finding 7).** `if (true)` also prepends a separator to the *first* element, so it fails the pre-existing `["alpha","beta"]` assertion too and was RED before P1's leading-empty case existed. It proves the join is pinned; it proves nothing about the coverage P1 asked for. The suggested index-based re-record (`if (i > 0)`) does not fix that either: **measured in fix round 2** — with that drift, `TwinConstantsVectorsTest.chunkJoinSeparatorMatchesFixture` *and* `ChunkingParityTest.h13_emptyLeadingResultProducesNoBlankLine` both go RED (16 tests, 2 failed). `h13` is a Story-7-1 test that already pinned `joinChunkResults(listOf("", …))` on the Kotlin side, so **no** drift of this function can be caught only by the new case. The Kotlin half of P1 was therefore already covered before this story; the genuinely new coverage is the **Rust** half, row **B**, which is discriminating. |
| F | P3 | deleted `expected_gate_open` from VAD-GATE-001 (the vector that expects `false`, so the old `optBool` default agreed with it) | 🔴 `IllegalStateException: fixture vector is missing required boolean key 'expected_gate_open'` |
| G | P8 | bypass cutoff 1 Hz → 0.05 Hz (survival 0.9957) | 🔴 the new band — *"expected 0.85..0.89 … got 0.99572057"*. **The dropped `> 0.5f` assertion would have passed this**, so the band is not a restatement. |

All seven reverted; `git status` shows only the intended edits — no probe or scratch file (audited file-by-file).

**Fix-round gate results.** `cargo test --lib` in `src-tauri/`: **662 passed, 0 failed, 0 ignored**
(unchanged count — the fixes added assertions and one mock provider, not new test functions).
JVM `:app:testUniversalDebugUnitTest` with `--rerun-tasks`: **168 tests, 0 failures, 0 errors**
across 22 suites (also unchanged count, same reason). The four touched Kotlin suites individually:
`Adr0017BoundaryGuardTest` 3/0, `TwinConstantsVectorsTest` 6/0, `VadGateGoldenVectorsTest` 2/0,
`HighpassFilterTest` 9/0. **Coverage of these numbers is unchanged from the original run's coverage
statement above** — they are Linux/JVM logic results. This fix round drove **no** device, **no**
emulator and **no** network call; AC6b's runtime proof is the earlier laptop-AVD evidence cited in
Task 8, not something re-run here, and GATE-4 remains Andi's.

**Deferred (real, out of this story's AC scope or pre-existing class):**

- [x] [Review][Defer] Boundary guard scans one directory level only [`android/kotlin-test/com/klarvo/voice/Adr0017BoundaryGuardTest.kt:136`] — deferred, coverage stated + not reachable today
- [x] [Review][Defer] Boundary-guard rules match per line, so a split declaration evades [`android/kotlin-test/com/klarvo/voice/Adr0017BoundaryGuardTest.kt:170`] — deferred, tripwire by design
- [x] [Review][Defer] Duplicate fixture ids collapse silently on the Kotlin side [`android/kotlin-test/com/klarvo/voice/TwinConstantsVectorsTest.kt:55-62`] — deferred, pre-existing class
- [x] [Review][Defer] `vector(id)` re-reads and re-parses the fixture on every lookup [`android/kotlin-test/com/klarvo/voice/TwinConstantsVectorsTest.kt:66`] — deferred, pre-existing
- [x] [Review][Defer] `android-build.sh` still carries the un-pruned `cp` that AC6a fixed in the smoke script [`scripts/android-build.sh:70`] — deferred, outside AC6a's named scope
- [x] [Review][Defer] The `kotlin-test` prune sits inside the "has .kt files" guard [`scripts/android-smoke.sh:183-186`] — deferred, degenerate case
- [x] [Review][Defer] Two more copies of the repo-root fixture resolver [`android/kotlin-test/com/klarvo/voice/TwinConstantsVectorsTest.kt:40`, `Adr0017BoundaryGuardTest.kt:59`] — deferred, pre-existing duplication class

**Dismissed as noise (13, not persisted):** `info` helper "missing" (defined at `android-smoke.sh:38`) · `INSTALL_FAILED_NO_MATCHING_ABIS` "unhandled" (caught by the third `elif` at `:296`; only the message is generic) · `stripComments` "fails open" (needs source that would not compile) · `deepseekUrl_bothCallSitesAgree` "structurally unfailable" (extraction authorized by Task 1; still guards re-inlining) · Anthropic coincidence-check "contradicts its own comment" (rationale stated in code, disclosed in the fixture) · inversion row 2 "not reproducible" (consistent if the probe file was named `HallucinationFilter.kt`) · `KlarvoApi.kt` "exceeds Files-to-MODIFY" (Tasks 1/3 authorize it; intra-spec conflict) · four hypothetical fixture-authoring guards from the edge layer (`expected_char_code` range, f32/u32 precision, 4th `CleanupStyle` variant, `amplitude_short` i16 range) · gradle up-to-date on fixture-only edits (already found and documented by this story) · AC6b runtime proof and GATE-4 (already recorded as blocked/pending by the dev, not new findings).

### Review Findings — code review round 2 (re-review of fix round, range `7422a86..27de205`)

Scoped re-review, not a fresh sweep: the mandate was to verify that round 1's confirmed findings
(D1, D2, P1–P10) are genuinely resolved on **today's tree** and that the touched lines regressed
nothing. Three layers (Blind Hunter, Edge Case Hunter, Acceptance Auditor); every finding below was
re-verified against the tree before being recorded. Both gates were re-run independently rather than
read from the record: `cargo test --lib` **662 passed / 0 failed / 0 ignored** and JVM
`:app:testUniversalDebugUnitTest --rerun-tasks` **168 tests / 0 failures across 22 suites** — both
match the record, so the fix round regressed no gate.
1 decision-needed · 10 patch · 13 deferred · 10 dismissed as noise.

**Round-1 verdicts:** D1 ✅ · D2 ✅ · P1 ✅ · P2 ✅ · P3 ⚠ partially (R2-1 below) · P4 ✅ · P5 ⚠
partially (R2-D1 below) · P6 ✅ · P7 ✅ · P8 ✅ · P9 ✅ · P10 ✅.

**Decision findings (Andi's call — must be resolved before the patch findings):**

- [x] [Review][Decision] P5 is only half resolved: the fixture's new "no Rust edit needed" promise is falsified by the assertion added in the same commit [`test-fixtures/m12-dictionary-scope-vectors.json:6` vs `src-tauri/src/llm/mod.rs:2397-2402`] — the hard-coded literals are genuinely gone and `platforms_agree` is now derived (`:2385-2396`), but the round added `assert!(vectors.iter().any(|v| v["platforms_agree"].as_bool() == Some(false)))`. The fixture has exactly three styled entries and exactly one disagreeing (`chat`: desktop `false` / kotlin `true`), so **either** direction of a correct M12 resolution makes all three agree and fires that assertion. The fixture text written in the same commit says the opposite — *"It does NOT require editing the Rust test … so it follows a correct flip on its own and fails only on an incorrect one"* — and the fix-round resolution row repeats it (`7-8-…md:827`, *"needs **no** edit to `llm/mod.rs`; the fixture README says so explicitly"*). The assertion's own failure message concedes the contradiction (*"retire the record deliberately"*). Round-1 P5's complaint — *"flipping it fails the suite until `llm/mod.rs` is edited too"* — is therefore still true; only the failing line moved, and one false claim was replaced by another. **Options:** (a) drop the `any(false)` assertion and carry "M12 is still open" in the fixture's `open_decision` field instead, so the zero-edit promise becomes true; (b) keep the divergence-liveness assertion and strike the "does NOT require editing the Rust test" clause from both the fixture and the resolution row, stating that resolving M12 means retiring that assertion.

**Patch findings:**

- [x] [Review][Patch] P3's replacement KDoc still over-claims, with the counter-examples in the same file [`android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:80-82`]
- [x] [Review][Patch] The new `getBool` KDoc miscounts the vectors it names — "three" where the fixture has two [`android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:94-95`]
- [x] [Review][Patch] The fix round committed a fresh stale line anchor in the row that records the anchor-staleness decision [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:822`]
- [x] [Review][Patch] The P1 fix added an assertion that cannot fail — the shape P8 was raised to remove [`src-tauri/src/llm/mod.rs:2276-2280`]
- [x] [Review][Patch] `EmptyFirstChunkProvider`'s doc comment does not describe its code [`src-tauri/src/llm/mod.rs:2284-2285` vs `:2302`]
- [x] [Review][Patch] The rewritten ADR-0017 guard KDoc contradicts its own coverage statement two paragraphs later [`android/kotlin-test/com/klarvo/voice/Adr0017BoundaryGuardTest.kt:33-34` vs `:47-52`]
- [x] [Review][Patch] Fix-round inversion row E does not discriminate the assertion it is offered as evidence for [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:842`]
- [x] [Review][Patch] P10's resolution row misdescribes a three-convention deletion as "one load-bearing sentence" [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:832`]
- [x] [Review][Patch] The committed AC6b evidence carries a test count that contradicts the record and states nothing about its own scope [`_bmad-output/implementation-artifacts/gate4-evidence/7-8/smoke-r1-00e771d.log:17`]
- [x] [Review][Patch] `getBool`'s error message drops the wrong-type case that its sibling `getDouble` names [`android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:100-103`]

**Patch detail:**

1. **P3's replacement KDoc still over-claims** [`VadGateGoldenVectorsTest.kt:80-82`] — the new text reads *"The one defaulting reader that remains is [optString] for `signal`"*. `optString` is also the reader for `category` (`:239`, `:275`) and for `id` (`:242`, `:278`), i.e. at least three uses, not one. Round-1 P3 was precisely *"the KDoc asserts a universal that the file contradicts"*; the universal was narrowed rather than made true, and the fix-round resolution row (`:825`) repeats it. Verified there is **no** vacuous pass behind it: a dropped `category` key is caught by `assertTrue(… isNotEmpty())` at `:240`/`:276`, and `id` is only used in failure messages. Fix: enumerate the three uses and say why each cannot fake a pass, or route `category`/`id` through a throwing accessor.
2. **`getBool` KDoc miscounts its own vectors** [`VadGateGoldenVectorsTest.kt:94-95`] — *"the three `expected_gate_open: false` vectors"*. `test-fixtures/vad-gate-golden-vectors-7-2.json` carries **two** `false` and four `true`, and the 7-2 finding preserved verbatim in the same commit says *"VAD-GATE-001 and VAD-GATE-004 (**both** `expected_gate_open: false`)"*. Newly written in the fix round, in the story that enforces *"a number states what it covers"*. Fix: two.
3. **A fresh stale line anchor in the D2 resolution row** [`7-8-…md:822`] — the row states *"`scripts/android-smoke.sh:280-284` **unchanged**"*. On today's tree the `case "$DEVICE_SERIAL" in emulator-*)` block is at `:282-286` (`case` `:282`, emulator arm `:283-284`, `*)` `:285`, `esac` `:286`); the round-1 citation was already off, and this commit's own `+1`-net edit to that file moved it further. The decision itself is honored — the block is genuinely byte-identical. This is the R3-P5/P9 defect class re-committed in the round that fixed P9. Fix: cite by content ("the `emulator-*` install-ABI `case` block in `android-smoke.sh`").
4. **An assertion that cannot fail, added by the P1 fix** [`src-tauri/src/llm/mod.rs:2276-2280`] — `assert_eq!(result.text.matches(sep).count(), 0, …)` sits directly after `assert_eq!(result.text, "B".repeat(50), …)`. The first pins the string exactly, so for any separator that is not a substring of `"B"*50` the second is determined and cannot be the sole failure — verbatim the shape round-1 P8 was raised to remove. Corroborated by the fix round's own evidence: inversion row B lists only the first message going red, and no inversion row exists for the second. Fix: delete it.
5. **`EmptyFirstChunkProvider`'s doc does not describe its code** [`src-tauri/src/llm/mod.rs:2284-2285` vs `:2302`] — the doc says *"returns an EMPTY result for any chunk made of `a`s"*; the code is `raw_text.starts_with('a')`, which also blanks a mixed chunk. Load-bearing for reading the test: the input is `"a"*target + "b"*50`, so the doc's rule and the code's rule differ exactly where the split lands. Fix: *"any chunk that begins with `a`"*.
6. **ADR-0017 guard KDoc contradicts itself** [`Adr0017BoundaryGuardTest.kt:33-34` vs `:47-52`] — the P4 rewrite claims *"the `audio/transcriptions` rule really does mean **'anywhere in Kotlin'**, as AC2 asks"*, while the coverage block twelve lines later states it covers only the production Kotlin files **directly under** `android/kotlin-src/com/klarvo/voice/`, and explicitly not `android/kotlin-test/`, the Rust side, or `gen/android/`. The guard change itself is right and verified green (no rule matches anything under `kotlin-src` today). Fix: *"anywhere in production `voice/`"*.
7. **Inversion row E is non-discriminating** [`7-8-…md:842`] — the recorded drift `if (sb.isNotEmpty())` → `if (true)` in `KlarvoApi.joinChunkResults` (`KlarvoApi.kt:1185`) also prepends a separator to the first element, so it already fails the pre-existing `["alpha","beta"]` assertion (`TwinConstantsVectorsTest.kt:141-146`) — it was RED before the leading-empty case was added and therefore proves nothing about the coverage P1 asked for. This violates the dev's own standard recorded in the Debug Log (`:610-614`, *"a fixed inversion has to fail differently than the bug did"*). The Rust twin, row B, **is** discriminating. Fix: re-record E with an index-based drift (e.g. `if (results.indexOf(r) > 0)`) that only the leading-empty case catches, or mark E explicitly as non-discriminating.
8. **P10's resolution row misdescribes what was deleted** [`7-8-…md:832`] — it says *"its one load-bearing sentence folded into the fixture table"*. The dropped `test-fixtures/README.md` paragraph (`00e771d:test-fixtures/README.md:22-29`) carried **three** conventions: (i) every entry has an `id` + a description stating what it pins and what it does not; (ii) per-platform expectations live on the same entry under a `_kotlin` suffix; (iii) each side asserts a production symbol against the **fixture literal**, never against another production symbol. Only the M12 exception survived, into a table cell — and (iii) is AC3's governing method rule. Fix: state that three conventions were dropped, or restore (iii) somewhere durable in the fixture tree.
9. **The AC6b evidence log's test count contradicts the record and states nothing about its scope** [`gate4-evidence/7-8/smoke-r1-00e771d.log:17`] — the committed log reports *"[ok]    24 Tests, 0 Failures — alle grün"* for the JVM gate, while the record insists on **168 tests across 22 suites** for the same gate, the same day, the same tree (and the same log says *"22 Test-Dateien kopiert"*). Cause verified: `scripts/android-smoke.sh:217-220` takes `find … -print -quit` plus `head -1`, i.e. the `tests="…"` attribute of **one** result XML, so the banner is a single suite's count, not the run total. The log is sound as AC6b install evidence, but it ships an unreconciled number in the story whose thesis is *"a number states what it covers"*. Fix: annotate the evidence file (or the Task-8 citation) with what the 24 covers. The script fix itself is deferred below.
10. **`getBool`'s error message drops the wrong-type case** [`VadGateGoldenVectorsTest.kt:100-103`] — it reports only *"is missing required boolean key"*, though the same `as? Num` narrowing also fires when the key is present as a string or object; the sibling `getDouble` (`:86-89`) says *"(or it is not a number)"*. A wrong-typed key sends the reader after an absent key. Fix: mirror the parenthetical.

**Fix round 2 — resolutions (2026-09-10).** The decision and all ten patch findings applied exactly
as confirmed; the 13 deferred items were not touched and no scope was added.

| # | Finding | Resolution |
|---|---------|------------|
| R2-D1 | The `any(platforms_agree == false)` assertion falsifies the fixture's "no Rust edit needed" promise | Option (a): the assertion is **deleted** from `llm/mod.rs` and replaced by a comment stating why there deliberately is none. The per-entry consistency check (`platforms_agree` must equal `desktop == kotlin`) **stays**, so a flag that stops matching its own columns still fails loudly. "M12 is still open" is now carried solely by the fixture's `open_decision` field. The clause in `m12-dictionary-scope-vectors.json` is therefore **true as written** and was kept unchanged — **only that fixture clause**. The round-1 `P5` row was *not* left true by this deletion: it still described the deleted "at least one style still disagrees" clause and was corrected in fix round 3 (round-3 patch 2). |
| R2-1 | `getDouble` KDoc named one `optString` use, the file has three | All three enumerated with why each cannot fake a pass: `signal` (schema default, wrong type changes the RMS rather than zeroing it, unknown value hits `else -> error`), `category` (a dropped key empties the list → the `isNotEmpty()` guard fails loudly), `id` (labels failure messages only, feeds no assertion). |
| R2-2 | `getBool` KDoc said "three `expected_gate_open: false` vectors" | Corrected to **two** — re-counted in the fixture: 2 `false` / 4 `true`. |
| R2-3 | Fresh stale line anchor in the D2 resolution row | Row now cites the `case "$DEVICE_SERIAL" in emulator-*)` install-ABI block **by content**, and says the block is byte-identical rather than pointing at a range. |
| R2-4 | An assertion that cannot fail, added by the P1 fix | `assert_eq!(result.text.matches(sep).count(), 0, …)` **deleted**. The preceding `assert_eq!(result.text, "B".repeat(50))` already pins the string exactly. `sep` is still used by the three-chunk case, so nothing went dead. |
| R2-5 | `EmptyFirstChunkProvider` doc did not describe its code | *"any chunk made of `a`s"* → *"any chunk that begins with `a`"*, matching `raw_text.starts_with('a')`. |
| R2-6 | ADR-0017 guard KDoc contradicted its own coverage statement | *"anywhere in Kotlin"* → *"anywhere in production `voice/`"*, pointing at the coverage block below it. The guard code is unchanged. |
| R2-7 | Inversion row E does not discriminate | Marked **non-discriminating**, with the reason measured rather than asserted. The suggested index-based re-record was **tried**: `if (i > 0)` fails `TwinConstantsVectorsTest.chunkJoinSeparatorMatchesFixture` **and** the pre-existing `ChunkingParityTest.h13_emptyLeadingResultProducesNoBlankLine` (16 tests, 2 failed), so option (a) is not achievable — `h13` (Story 7-1) already pinned Kotlin's leading-empty join before this story. Row E now records that, and that the genuinely new coverage is the **Rust** half (row B). |
| R2-8 | P10's row misdescribed a three-convention deletion | Row now names all **three** dropped conventions and states that (iii) — production symbol vs. **fixture literal**, never vs. another production symbol — is AC3's governing method rule and lives in AC3 + the Dev Note, not re-added to the README (restoring a paragraph would re-open the very AC7 guard P10 exists to honor). |
| R2-9 | AC6b evidence log's "24 Tests" contradicts the record's 168 | Evidence annotated with a new note file, `gate4-evidence/7-8/NOTE-jvm-test-counts.md`: the banner is **one suite** (the script reads a single result XML's `tests="…"` via its `find … -print -quit` block), the run total is **168/0** (`smoke-r2-27de205.log:44`), and `structure-install-r1.txt` carries **no** test counts at all — it is the install probe. The note also restates the logs' scope (install evidence only). The **script fix stays deferred**; `android-smoke.sh` is unchanged. |
| R2-10 | `getBool`'s error message dropped the wrong-type case | Mirrors `getDouble`: *"missing required boolean key '$key' **(or it is not a boolean)**"*. |

**Fix-round-2 inversion.** One inversion was run, and it is the finding's own evidence: the
index-based `joinChunkResults` drift for R2-7 (measured RED on two suites, see the row above), then
reverted — `git diff` on `KlarvoApi.kt` is empty. The other ten items are **text, doc-comment and
record corrections plus two assertion deletions**; a deletion has no drift to re-introduce. The two
deletions were removed for **opposite** reasons, and the earlier wording ("neither could fail on its
own") was true of only one of them (corrected in fix round 3, round-3 patch 3): R2-4's
`matches(sep).count() == 0` was **unfailable** — the preceding exact-string assertion determined it;
R2-D1's `any(platforms_agree == false)` was **over-eager** — it fired on every correct M12
resolution, which is exactly what made the fixture's zero-edit promise untrue. Skipping the
inversion is right in both cases, but for different reasons. No new assertion was added in this
round, so there is nothing else to invert.

**Fix-round-2 gate results.** Re-run independently after the fixes, not read from the record:
`cargo test --lib` in `src-tauri/`: **662 passed, 0 failed, 0 ignored** — unchanged, since deleting
an assertion inside a test function does not change the function count. JVM
`:app:testUniversalDebugUnitTest --rerun-tasks`: **168 tests, 0 failures, 0 errors across 22
suites** (aggregated over all result XMLs, not read off the smoke banner — see R2-9), also
unchanged. **Coverage of these two numbers is identical to the original run's coverage statement
above:** they are Linux/JVM *logic* results — pure functions, seams, fixture agreement and static
source structure. This round drove **no** device, **no** emulator and **no** network call. AC6b's
runtime proof remains the earlier laptop-AVD evidence cited in Task 8 (not re-run here), and GATE-4
— one Xiaomi dictation through DeepSeek — remains Andi's and is still **pending**.

**Deferred (real, out of this re-review's scope, pre-existing, or explicitly bounded by an AC):**

- [x] [Review][Defer] The Rust 399/400 probe's path discriminator is a hard-coded `'\n'` and silently depends on `CHUNK_TARGET_SIZE < CHUNK_THRESHOLD` [`src-tauri/src/llm/mod.rs:2186-2206`] — deferred, cannot produce a false green, only a failure message that blames the wrong constant
- [x] [Review][Defer] The leading-empty Rust case assumes a chunk layout the fixture explicitly does not pin [`src-tauri/src/llm/mod.rs:2268-2280`] — deferred, passes today, no divergence
- [x] [Review][Defer] Trailing-empty and interior-empty join results are still unpinned on both sides [`src-tauri/src/llm/mod.rs:2262-2283`, `android/kotlin-test/com/klarvo/voice/TwinConstantsVectorsTest.kt:155-161`] — deferred, P1 asked only for the leading-empty case
- [x] [Review][Defer] The byte-length boundary probe never exercises multi-byte UTF-8 [`src-tauri/src/llm/mod.rs:2176-2210`] — deferred, `raw_text.len()` → `chars().count()` would stay green
- [x] [Review][Defer] `android-smoke.sh` reports one result XML's counts as the whole run [`scripts/android-smoke.sh:217-220`] — deferred, pre-existing, outside AC6's two named traps
- [x] [Review][Defer] The smoke banner's "Dauer" prints `BUILD_SECS`, the APK-build time only [`scripts/android-smoke.sh:328`] — deferred, pre-existing, cosmetic
- [x] [Review][Defer] D2's bounded scope is stated only in the story record, not at the `case` a maintainer reads [`scripts/android-smoke.sh:282-286`] — deferred, D2 explicitly resolved to leave the script unchanged
- [x] [Review][Defer] The P4 tightening is stricter than AC2's wording and has no override path [`android/kotlin-test/com/klarvo/voice/Adr0017BoundaryGuardTest.kt:150-176`] — deferred, the stricter default is correct under ADR-0017; record it as a deliberate tightening if the bridge ever needs the literal
- [x] [Review][Defer] The M12 derived check skips style-less entries, so `platforms_agree` on such an entry is neither derived nor rejected [`src-tauri/src/llm/mod.rs`, test fn `spec_m12_dictionary_scope_current_state_still_holds` — re-anchored by symbol in fix round 3; the original `:2381-2402` overran the 2398-line file] — deferred, only the README entry is style-less today and it carries no `platforms_agree`. **Rewritten in fix round 3 (round-3 patch 5):** the entry's original argument (*"the divergence assertion at `:2397` scans all vectors"*) and its prescribed fix (*"scope the `any()` to styled entries"*) both died with the `any()` that R2-D1 deleted in the same round. The surviving defect is the skip itself; fix shape is now "assert that a style-less entry carries no `platforms_agree` field". Mirrored in `deferred-work.md`.
- [x] [Review][Defer] The retained `assertFalse(bypassedRms < rawNormalizedRms * 0.5f)` is now implied by the 0.85–0.89 band [`android/kotlin-test/com/klarvo/voice/HighpassFilterTest.kt:362-367`] — deferred, P8 offered an "or" and the band satisfies it; the redundancy shape merely survives inverted
- [x] [Review][Defer] `getBool` coerces any non-zero number to true [`android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:99-104`] — deferred, hypothetical fixture-authoring guard
- [x] [Review][Defer] `optString("signal")` returns its default when the key is present but not a string [`android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:251`] — deferred, same class
- [x] [Review][Defer] D1's own citation no longer resolves after D1 was applied [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:800`] — deferred, round-1 finding text is preserved verbatim by convention; the eight items now span `7-2-…md:422-437`

**Dismissed as noise (10, not persisted):** `android-smoke.sh:45-46` "stale anchor" (verified correct — `JAVA_HOME` `:45`, `ANDROID_HOME` `:46`) · D1's resolution stamps "cite the wrong commit" (they say *resolved by* `00e771d` and then attribute later work explicitly to "7-8's own review round 1" — self-consistent) · evidence timestamps "postdate the commit" (a timezone offset, not a contradiction) · "three 'all seven' sentences are ambiguous" (the two tables are separately headed round-1 / fix-round) · "the M12 desktop column may not be asserted against the real prompt" (verified false — `llm/mod.rs:2360-2371` drives `CleanupStyle::system_prompt` and asserts `prompt.contains(dict)`) · the guard comment's "neither literal appears anywhere under kotlin-src" as a point-in-time claim (it is stated as the rationale for a past decision) · f64-vs-f32 intermediates in the "~87 %" comment (the comment states the band is sized for f32 rounding) · "AC7 still exceeds one paragraph" (P10's prescribed fix — drop the unasked paragraph — was applied) · boundary-guard non-recursive scan and per-line matching (already accepted as round-1 deferred items) · three hypothetical fixture-authoring guards (`expected_int` ≤ 1 underflow, duplicate `style` entries, NaN ratio when the raw RMS is 0 — the class round 1 already dismissed).

### Review Findings — code review round 3 (re-review of fix round 2, range `7422a86..85aa0ee`)

Scoped re-review, not a fresh sweep: the mandate was to verify that round 2's confirmed findings
(R2-D1, R2-1…R2-10) are genuinely resolved on **today's tree** and that the touched lines regressed
nothing. Three layers (Blind Hunter, Edge Case Hunter, Acceptance Auditor); every finding below was
re-verified against the tree before being recorded. Both gates were re-run independently rather than
read from the record: `cargo test --lib` **662 passed / 0 failed / 0 ignored**, and the JVM gate
reproduced device-free (`rm`+`cp` sync of both trees, then `./gradlew :app:testUniversalDebugUnitTest
--rerun-tasks`) **168 tests / 0 failures / 0 errors across 22 suites**, aggregated over all result
XMLs rather than read off the smoke banner. Both match the record, so fix round 2 regressed no gate.
1 decision-needed · 9 patch · 6 deferred · 1 residual · 12 dismissed as noise.

**Round-2 verdicts (code):** R2-D1 ✅ code / ⚠ record · R2-1 ⚠ partially · R2-2 ✅ · R2-3 ✅ ·
R2-4 ✅ · R2-5 ✅ · R2-6 ✅ · R2-7 ✅ · R2-8 ✅ · R2-9 ⚠ partially · R2-10 ✅.
Every code-level fix landed exactly as confirmed. **The residue of this round is almost entirely in
the record**: three resolution rows and the Change Log now describe a tree that the same commit
changed underneath them.

**Decision findings (Andi's call — must be resolved before the patch findings):**

- [x] [Review][Decision] R2-9's evidence note anchors the 168 in a file that is not in the repository [`_bmad-output/implementation-artifacts/gate4-evidence/7-8/NOTE-jvm-test-counts.md:5,17` vs `.gitignore:15`] — the note opens *"Both **committed** smoke logs in this directory print …"* and cites `smoke-r2-27de205.log:44` as the source of the run total (*"JVM (laptop, 27de205): tests 168 fail 0 err 0 skip 0"*). That file is matched by `.gitignore:15` (`*.log`) and is **untracked**: `git ls-tree -r HEAD -- gate4-evidence/7-8/` returns only `NOTE-jvm-test-counts.md`, `smoke-r1-00e771d.log` and `structure-install-r1.txt`. The one committed log carries **no** aggregate line at all (verified: no `tests 168` anywhere in `smoke-r1-00e771d.log`; its only count is the `24` banner at `:17`). So the note written to reconcile *"24 vs 168"* sends a future reader to a file they will not have, and the only number that survives in the repo is the one R2-9 called unreconciled. `smoke-r1-00e771d.log` is itself tracked despite the same ignore rule, i.e. it was force-added — so committing evidence logs is an existing but *undeclared* convention here. **Options:** (a) `git add -f smoke-r2-27de205.log` so both cited logs are actually in the repo, matching how `smoke-r1` got there, and note the force-add convention; (b) leave the log out, strike the word "committed", and quote the aggregate line verbatim inside the note so the 168 is self-contained; (c) both.

**Patch findings:**

- [x] [Review][Patch] Round-1's P3 resolution row still carries the exact false claim R2-1 was raised to kill [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:889`]
- [x] [Review][Patch] Round-1's P5 resolution row describes an assertion R2-D1 deleted in the same round — and the R2-D1 row declares that row "true as written" [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:891` vs `:983`]
- [x] [Review][Patch] The AC8 inversion paragraph and the Change Log both state a false reason for the two deletions [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:999`, `:831-832`]
- [x] [Review][Patch] The "Modified in fix round 2" File List is wrong in both directions [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:791`]
- [x] [Review][Patch] The M12 deferred entry argues from an assertion the same round deleted, and its anchor overruns the file [`_bmad-output/implementation-artifacts/deferred-work.md` round-2 section, `_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:1023`]
- [x] [Review][Patch] R2-1's `category` justification is still an over-claim — one typo'd `category` silently drops that vector from coverage [`android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:86-89`]
- [x] [Review][Patch] The rewritten `getDouble` KDoc miscounts again — "three uses" for three keys at five call sites [`android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:80`]
- [x] [Review][Patch] The R2-9 note miscounts inside the sentence written to fix a miscount, and its supporting inference does not follow [`_bmad-output/implementation-artifacts/gate4-evidence/7-8/NOTE-jvm-test-counts.md:16-17`]
- [x] [Review][Patch] The Change Log undercounts the round's own corrections — "six" where the table lists eight [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:832`]

**Patch detail:**

1. **P3's resolution row still says what R2-1 disproved** [`7-8-…md:889`] — the row reads *"KDoc rewritten to what is now true (the only defaulting reader left is `optString("signal")`, whose default cannot fake a pass)"*. On today's tree `optString` also reads `category` (`:248`, `:284`) and `id` (`:251`, `:287`). R2-1's own finding text named this row explicitly (*"the fix-round resolution row (`:825`) repeats it"*) — the fix corrected the KDoc and left the row. Fix: correct the row to match the KDoc (three keys, five call sites), or point it at the corrected KDoc instead of restating it.
2. **P5's resolution row describes the deleted clause; R2-D1's row certifies that row as true** [`7-8-…md:891` vs `:983`] — P5's row: *"Replaced with a **derived** check: `platforms_agree` must equal `desktop == kotlin` for every styled entry, **plus "at least one style still disagrees"**."* That second clause is the `any(platforms_agree == false)` assertion R2-D1 deleted in this same commit (`llm/mod.rs:2390-2396` is now a comment). Meanwhile R2-D1's row states *"the clause in `m12-dictionary-scope-vectors.json` and the round-1 resolution row (`P5`) are therefore **true as written** and were kept unchanged."* The fixture clause is now true; the P5 row is not. Fix: strike the "plus …" clause from P5's row (or mark it superseded by R2-D1), and correct R2-D1's row to say only the fixture clause was left as-is.
3. **A false reason recorded for skipping an inversion** [`7-8-…md:999`, `:831-832`] — *"neither deleted assertion could fail on its own (that is precisely why they were removed)"* and *"The round removed two assertions that could not fail on their own"*. True for R2-4's `matches(sep).count() == 0`. **False for R2-D1's `any(platforms_agree == false)`**: that assertion's whole problem was that it *does* fire — on either direction of a correct M12 resolution, which is what made the fixture's zero-edit promise untrue. Skipping its inversion was still correct (deleting an over-eager assertion has no drift to re-introduce), but the stated reason is wrong. Fix: split the sentence — one deletion was unfailable, one was over-eager.
4. **The fix-round-2 File List is wrong in both directions** [`7-8-…md:791`] — it lists *"`sprint-status.yaml` — status transitions"*, but the file was last touched by `00e771d` (`git log 7422a86..85aa0ee -- sprint-status.yaml`) and already read `7-8-…: review` before this round; `git diff-tree 85aa0ee` names five files and `git diff --stat 27de205..85aa0ee` six, neither including it. And `deferred-work.md` — which the round *did* modify (in `50bdce3`, thirteen new items) — appears in no File List section. This is verbatim the R3-P7(c) defect class (*"the main File List omits files it touched"*), re-committed in the story that exists to close it. Fix: drop the sprint-status line, add `deferred-work.md`.
5. **A deferred entry whose argument and prescribed fix were deleted by the same round** [`deferred-work.md` round-2 section, M12 bullet; mirrored at `7-8-…md:1023`] — the entry argues *"the divergence assertion at `:2397` scans **all** vectors — so a `platforms_agree: false` added to the README entry would satisfy 'M12 is still live' without any columns backing it"* and prescribes *"scope the `any()` to styled entries"*. `85aa0ee` deleted that `any()`. `src-tauri/src/llm/mod.rs` is **2398** lines, so the cited range `:2381-2402` also overruns the file. The residual defect is still real (both loops `continue` on a style-less entry, so a `platforms_agree` there is neither derived nor rejected) — only its stated impact and its fix shape are dead. A backlog item that is unactionable on arrival is the same "net carrying known-false claims" the story's goal statement forbids. Fix: rewrite the entry to the surviving defect and re-anchor by symbol.
6. **`category` can still fake a pass, one vector at a time** [`VadGateGoldenVectorsTest.kt:86-89`] — the new KDoc justifies `category` with *"a dropped key would empty the filtered list, which the `assertTrue("… must not be empty", vectors.isNotEmpty())` guard on the next line fails loudly rather than passing on zero vectors."* That holds only if the key is dropped from **every** vector. `optString(key, default = "")` returns `""`, which matches neither `"energy-floor"` nor `"stop-latency"`, so **one** typo'd or renamed `category` silently removes that vector from both loops while the suite stays green at 2/0 — precisely the vacuous-pass shape R3-P2 and R2-1 exist to close, now in the reader that *selects* the vectors. Fix: route `category` through the throwing accessor, or add `assertEquals(loadFixture().size, energyFloorCount + stopLatencyCount)` so no vector can leave coverage unnoticed, or narrow the KDoc claim to the all-vectors case.
7. **"Three uses" for five call sites** [`VadGateGoldenVectorsTest.kt:80`] — *"The defaulting reader that remains is [optString], at three uses"*. `optString` is called at `:248`, `:251`, `:260`, `:284`, `:287` — **five** sites across **three** keys, and the KDoc's own sub-bullets say so (*"in the `loadFixture().filter { … }` line of **both tests**"*, *"**once per test loop**"*). Written in the same edit that corrected R2-2's "three"→"two" miscount, in the story that enforces *"a number states what it covers"*. Fix: *"three keys, at five call sites"*.
8. **The note miscounts, and its supporting inference does not follow** [`NOTE-jvm-test-counts.md:16-17`] — (a) *"The same log states `22 Test-Dateien kopiert` **two lines above**"*: it is at `smoke-r1-00e771d.log:14`, **three** lines above `:17`. (b) *"… so 24 cannot be the whole run"* is not a valid inference — 22 source files can perfectly well hold 24 test methods. The sound argument is the `168` aggregate two paragraphs down; the file-count line adds a bad proof to a correct conclusion. Fix: "three lines above", and drop the inference.
9. **The Change Log undercounts its own round** [`7-8-…md:832`] — *"The round removed two assertions …, corrected **six** over-claiming or miscounting comments/record rows, and annotated the AC6b evidence"*. Two deletions (R2-D1, R2-4) plus one annotation (R2-9) leaves **eight** corrections: R2-1, R2-2, R2-3, R2-5, R2-6, R2-7, R2-8, R2-10. Fix: eight.

**Fix round 3 — resolutions (2026-09-10).** The decision and all nine patch findings applied exactly
as confirmed; the 6 deferred items and the 1 residual were not touched and no scope was added. Every
record row corrected below was checked against **today's tree first** (the round-3 lesson: three
round-2 resolution rows described code the same commit had deleted).

| # | Finding | Resolution |
|---|---------|------------|
| R3-D1 | The 168 is anchored in a file that is not in the repository | Option (c), both halves. `smoke-r2-27de205.log` and `smoke-r3-85aa0ee.log` are **force-added** (`git add -f`) alongside the already-force-added `smoke-r1-00e771d.log`, and `NOTE-jvm-test-counts.md` now declares that convention in one line (`.gitignore:15` ignores `*.log`; this evidence directory overrides it with `-f`). **And** both aggregate lines are quoted **verbatim** in the note — `JVM (laptop, 27de205): tests 168 fail 0 err 0 skip 0` and `JVM (laptop, 85aa0ee): tests 168 fail 0 err 0 skip 0` (both at `:44` of their log) — so the 168 stands without the logs. The note also states that `smoke-r1-00e771d.log` predates the aggregate line and carries only the `24` banner. |
| R3-1 | P3's row still restated the claim R2-1 killed | The row no longer restates the enumeration at all: it points at the `getDouble` KDoc in `VadGateGoldenVectorsTest.kt` as the authoritative list and records that restating it here is what made the row false twice. |
| R3-2 | P5's row describes a clause R2-D1 deleted; R2-D1's row certifies that row | P5's row: the *"plus 'at least one style still disagrees'"* clause is struck and marked **superseded by R2-D1**, with the reason (it fired on either direction of a correct M12 flip) and the current state of the code (`llm/mod.rs:2390-2396` is a comment — re-verified: the file is 2398 lines and holds no `any(` on `platforms_agree`). R2-D1's row now says **only the fixture clause** was left as-is, and that the P5 row was not. |
| R3-3 | A false reason recorded for skipping an inversion | Split: R2-4's `matches(sep).count() == 0` was **unfailable**; R2-D1's `any(platforms_agree == false)` was **over-eager**. Skipping both inversions stays correct, for different reasons. Corrected in the AC8 inversion paragraph **and** in the Change Log entry. |
| R3-4 | The fix-round-2 File List is wrong in both directions | `sprint-status.yaml` removed (verified: last touched by `00e771d`; `git diff --name-only 27de205..85aa0ee` does not name it), `deferred-work.md` added (verified: `50bdce3`, +16 lines = heading + **13** deferred entries). Both directions stated in the record so the correction is auditable. |
| R3-5 | The M12 deferred entry argues from an assertion the same round deleted | Rewritten in `deferred-work.md` **and** in this record's round-2 deferred list, anchored by **symbol** (`spec_m12_dictionary_scope_current_state_still_holds`) rather than by the line range that overran the 2398-line file. The dead argument (*"the divergence assertion at `:2397` scans all vectors"*) and the dead fix shape (*"scope the `any()` to styled entries"*) are gone; the surviving defect — both loops `continue` on a style-less entry, so a `platforms_agree` there is neither derived nor rejected — is stated with the fix shape that still works. Verified today: the only style-less entry, `M12-DICT-SCOPE-README`, carries no `platforms_agree`. |
| R3-6 | `category` can still fake a pass, one vector at a time | **Closed in code**, the first of the two preferred options: a throwing `getString` accessor was added and `category` now reads through it in both `loadFixture().filter { … }` lines. A typo'd or renamed `category` on a single vector now throws instead of silently removing that vector from both loops. Verified green: all 9 fixture entries carry `category`. |
| R3-7 | "Three uses" for five call sites | Re-counted **after** the R3-6 change, which moved `category`'s two sites onto `getString`: `optString` now has **three call sites across two keys** — `id` (`:266`, `:302`) and `signal` (`:275`). The KDoc says exactly that, and its bullet list dropped the `category` entry, which now belongs to the throwing accessor. |
| R3-8 | The note miscounts, and its inference does not follow | (a) *"two lines above"* → **three lines above the banner, at `:14`** (verified: `22 Test-Dateien kopiert` is `smoke-r1-00e771d.log:14`, banner `:17`). (b) The *"so 24 cannot be the whole run"* inference is **dropped** — 22 files can hold 24 methods — and the note now says explicitly that the load-bearing argument is the 168 aggregate, not the file count. |
| R3-9 | The Change Log undercounts its own round | *"corrected six"* → **eight**, with the eight named inline (R2-1, R2-2, R2-3, R2-5, R2-6, R2-7, R2-8, R2-10), leaving the two deletions and the one annotation counted separately. |

**Fix-round-3 inversion.** One assertion-level change was made in this round (R3-6's throwing
`getString`); the other nine items are record, KDoc and evidence-note text, and a text correction has
no drift to re-introduce. The R3-6 inversion is row **H**, continuing the fix-round lettering. It is
**discriminating by construction**: the drift is the exact defect the finding names (one vector's
`category` renamed, not all of them), and it was measured **both** ways — under the fixed reader and
under the pre-fix reader — so the row shows the fix failing where the bug was silent.

| # | Finding | Reverted change (deliberate drift) | Result |
|---|---------|------------------------------------|--------|
| H | R3-6 | `"category"` → `"categorie"` on **VAD-GATE-001 only** (a single vector, which is the finding's mode — a whole-file drop was already caught by the `isNotEmpty()` guard) | 🔴 both tests: `IllegalStateException` from `getString` at `VadGateGoldenVectorsTest.kt:263` and `:299` — *"fixture vector is missing required string key 'category'"*. **Counter-measurement with the same drift under the pre-fix reader** (`optString("category", "")` restored, fixture drift kept): 🟢 **2 tests, 0 failures** — VAD-GATE-001 silently left both loops and the suite still reported green. That is the gap R3-6 closes. Both edits reverted. |

**Fix-round-3 gate results.** Re-run after the fixes, not read from the record. `cargo test --lib`
in `src-tauri/`: **662 passed, 0 failed, 0 ignored** — unchanged, and correctly so: no Rust file was
touched in this round. JVM `:app:testUniversalDebugUnitTest --rerun-tasks`, reproduced device-free
(clear-then-copy sync of `android/kotlin-src` + `android/kotlin-test` into the generated project,
`JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64`, `ANDROID_HOME=…/tools/android-sdk`): **168 tests, 0
failures, 0 errors, 0 skipped across 22 suites**, aggregated over all result XMLs rather than read
off the smoke banner (see R2-9 / R3-8). Unchanged, since R3-6 replaced a reader inside two existing
test functions rather than adding one. **Scope of these two numbers is identical to the original
run's coverage statement above:** Linux/JVM *logic* results — pure functions, seams, fixture
agreement and static source structure. This round drove **no** device, **no** emulator and **no**
network call. AC6b's runtime proof remains the earlier laptop-AVD evidence cited in Task 8 (not
re-run here), and GATE-4 — one Xiaomi dictation through DeepSeek — remains Andi's and is still
**pending**. `git status` after the inversion carries only the intended edits; the fixture and the
test file were both restored (audited by `git status` + grep).

**Deferred (real, out of this re-review's scope, pre-existing, or already an accepted deferral):**

- [x] [Review][Defer] An identical unfailable assertion survives twelve lines above the one R2-4 deleted [`src-tauri/src/llm/mod.rs:2255-2259`] — deferred, pre-existing since `00e771d`, not a round-2 regression
- [x] [Review][Defer] `getBool`'s new "(or it is not a boolean)" advertises a check it does not perform for numbers [`android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:111`] — deferred, the coercion itself is already a round-2 deferred item
- [x] [Review][Defer] The round-2 deferred/dismissed anchors were already stale when committed [`_bmad-output/implementation-artifacts/deferred-work.md` round-2 section] — deferred, finding text is preserved verbatim by repo convention (same accepted class as round 2's own "D1's citation no longer resolves")
- [x] [Review][Defer] `EmptyFirstChunkProvider`'s name asserts the positional semantics R2-5 corrected the doc away from [`src-tauri/src/llm/mod.rs:2280`] — deferred, cosmetic; the adjacent comment states the by-content choice
- [x] [Review][Defer] Nothing pins that `open_decision: "M12"` is present, though R2-D1 names it the sole carrier of "M12 is open" [`src-tauri/src/llm/mod.rs:2335`] — deferred, D1 option (a) was Andi's call and the code comment is honest that there is no assertion
- [x] [Review][Defer] The ADR-0017 KDoc's "anywhere in production `voice/`" still reads wider than the non-recursive scan [`android/kotlin-test/com/klarvo/voice/Adr0017BoundaryGuardTest.kt:33-34`] — deferred, R2-6 prescribed this wording verbatim and it cross-references the coverage block; the non-recursive scan is a round-1 deferred item

**Residual (genuinely new and independent — reported, not fixed in this round):**

- [x] [Review][Residual] AC3's twin table cites a stale Kotlin anchor [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:125`] — the table gives `KlarvoApi.kt:1158-1165` for `fun joinChunkResults(…)`; on today's tree it is `:1182-1189`. Spec-authored text from story creation, untouched by any fix round; the story's own Task 5 records the +13-line shift in that file that caused it.

**Dismissed as noise (12, not persisted):** P8's row "dropped" vs. the deferred item's "retained" (different predicates — `assertTrue(x > y*0.5f)` was dropped, `assertFalse(x < y*0.5f)` retained, verified at `HighpassFilterTest.kt:362-367`) · inversion row E's "measured" framing (the label attaches to the index-based re-record, which is what the row says) · row E's *"no drift can be caught only by the new case"* called an over-generalisation (it is sound, not sampled — `ChunkingParityTest.h13_emptyLeadingResultProducesNoBlankLine:148-154` pins the same input shape) · row E vs. the P1 row "disagreeing" (both true; P1's finding was about the fixture's false claim) · the `Adr0017` KDoc dropping "as AC2 asks" (AC2 traceability survives elsewhere in the same KDoc) · the D2 row's "this round's own edit" (the row sits in the round-1 table; "this round" = fix round 1, and that edit is real) · the note not naming *which* suite the 24 covers (`find … -print -quit` picks arbitrarily; the note says so) · "gates re-run independently" called unbacked (both were re-run here: 662/0/0 and 168/0/0 across 22 suites) · the round-2 verdict line marking only P3/P5 partial (it reports round-1 resolution; R2-4/5/7 concern assertions the P1 fix *added*) · all thirteen deferred items checked `[x]` (repo convention, identical to round 1) · `category`'s default called a *widening* risk (verified false — the default `""` matches no filter; the real mode is narrowing, which is patch 6) · the P10 row's `##`-vs-`###` heading level and "AC3 states only half of (iii)" (the row itself names `:492`/`:358` as where (iii) lives).

### Review Findings — code review round 4 (re-review of fix round 3, range `7422a86..5128432`)

Scoped re-review, not a fresh sweep: the mandate was to verify that round 3's confirmed findings
(R3-D1, R3-1…R3-9) are genuinely resolved on **today's tree** and that the touched lines regressed
nothing. Three layers (Blind Hunter, Edge Case Hunter, Acceptance Auditor); every finding below was
re-verified against the tree before being recorded. No gate was re-run in this round — the round-3
fix touched no Rust, no production Kotlin, no fixture and no script, and its only code hunk replaces
a reader inside two existing test functions (verified: `git diff --name-only 85aa0ee..5128432` names
six files, four of them record/evidence text).
1 decision-needed · 5 patch · 5 residual · 2 deferred · 12 dismissed as noise.

**Round-3 verdicts (all ten): R3-D1 ✅ · R3-1 ✅ · R3-2 ✅ · R3-3 ✅ · R3-4 ✅ · R3-5 ✅ ·
R3-6 ✅ code · R3-7 ✅ · R3-8 ✅ · R3-9 ✅.** Every one is genuinely resolved on the tree, not merely
claimed resolved — anchors re-resolved, counts re-counted, code re-read. This is the first round in
which no round-N resolution row describes a tree state the same commit falsified. **The residue of
this round sits almost entirely in the evidence note `NOTE-jvm-test-counts.md`**, i.e. in the artifact
D1 and R3-8 rewrote: three of the five patch findings are claims that edit introduced.

**Decision findings (Andi's call — must be resolved before the patch findings):**

- [x] [Review][Decision] The record's **"across 22 suites"** rests on no committed artifact, and its only source is the line R3-8 just demoted [`NOTE-jvm-test-counts.md:24`, `:43` vs record `:1104`, `:1176`, `:878-879`] — D1 made the **168** self-standing by quoting both aggregate lines verbatim; those lines read `tests 168 fail 0 err 0 skip 0` and carry **no suite count**. The only "22" anywhere in the evidence directory is `22 Test-Dateien kopiert` (`smoke-r*.log:14`), the file-copy count, and the note now says of exactly that line *"it only tells you how many suites were in the run"* — the number R3-8 was raised to strip of load-bearing weight is now the sole support for a figure the record repeats three times. **Options:** (a) commit the per-suite XML summary (or the `--rerun-tasks` console tail) from the round-3 run so "22 suites" has the same verbatim anchor the 168 now has; (b) drop "across 22 suites" everywhere and state only `tests 168 fail 0 err 0 skip 0`, which is what the evidence carries; (c) both — commit the summary and, until it lands, state the 168 without the suite count.

**Patch findings (regressions on lines fix round 3 itself touched):**

- [x] [Review][Patch] The sentence D1 rewrote from two logs to three still says "the same tree" — three logs, three commits [`_bmad-output/implementation-artifacts/gate4-evidence/7-8/NOTE-jvm-test-counts.md:15-17`]
- [x] [Review][Patch] "Each smoke log carries an aggregate line at `:44`" is contradicted six lines later by the same edit [`_bmad-output/implementation-artifacts/gate4-evidence/7-8/NOTE-jvm-test-counts.md:27` vs `:33-34`]
- [x] [Review][Patch] The inference R3-8 removed was replaced by the same invalid shape, one noun over [`_bmad-output/implementation-artifacts/gate4-evidence/7-8/NOTE-jvm-test-counts.md:24`]
- [x] [Review][Patch] "Untouched in fix round 3" asserts a `sprint-status.yaml` transition the round did not make — R3-4's own defect, one round later [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:835`]
- [x] [Review][Patch] The fix-round-3 File List names `deferred-work.md` but records only half of what the round did to it [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:817-818`]

**Patch detail:**

1. **"the same gate, the same day, the same tree" now spans three commits** [`NOTE:15-17`] — fix round 3 rewrote this exact sentence, *"Both committed smoke logs"* → *"All three committed smoke logs"*, and added `smoke-r3-85aa0ee.log:17` to the citation list while leaving the trailing clause untouched. The three logs were captured on `00e771d`, `27de205` and `85aa0ee` — three different trees. "Same gate" and "same day" hold; "same tree" is false as committed, in the sentence the same commit edited. Fix: *"for the same gate, on the same day, across the three commits named above"*.
2. **The paragraph written to make the 168 self-standing contradicts itself** [`NOTE:27` vs `:33-34`] — `:27` opens *"**The run totals.** Each smoke log carries an aggregate line at `:44`"*; `:33-34`, inside the same new paragraph, concedes *"`smoke-r1-00e771d.log` predates the aggregate line and carries only the `24` banner"*. Verified: `smoke-r1-00e771d.log` is 41 lines and contains no `tests` line at all. Both sentences are new in `5128432`. Fix: *"Both later smoke logs carry an aggregate line at `:44`"*.
3. **The replacement inference is the same shape as the one it replaced** [`NOTE:24`] — R3-8 correctly dropped *"so 24 cannot be the whole run"*, and the replacement reads *"That line does not prove the banner is partial — 22 source files can hold 24 test methods — it only tells you how many suites were in the run."* `22 Test-Dateien kopiert` counts **test source files copied into the target tree**; a file can hold zero test classes or several. The clause therefore asserts files = suites in the same breath in which it has just argued files ≠ methods. It is also the sole support for the decision finding above. Fix: *"it only tells you how many test source files were synced"*.
4. **A `sprint-status.yaml` transition claimed for a round that did not touch the file** [`7-8-…md:835`] — the *"Untouched in fix round 3, deliberately"* paragraph ends *"…and `sprint-status.yaml` beyond the status transition"*, which asserts that this round wrote one. `git diff --name-only 85aa0ee..5128432` names six files and not that one; the file's only change in the whole range is `00e771d` (`backlog` → `review`), and the story already read `review` before the round. This is verbatim the defect R3-4 patched for fix round 2 — *"`sprint-status.yaml` was listed here in error … already read `7-8-…: review` before this round"* — re-committed nine lines below the correction note that records it. Fix: *"`sprint-status.yaml` (unchanged since `00e771d`; the story already read `review`)"*.
5. **The round-3 File List halves what the round did to `deferred-work.md`** [`7-8-…md:817-818`] — the entry records only *"the round-2 M12 deferred entry rewritten to the surviving defect and re-anchored by symbol (R3-5)"*. The round also appended a new heading *"round 3, scoped re-review of fix round 2"* plus **six** deferred entries (`deferred-work.md:381-388`, added in `cf39687`). The round-2 entry sets the precedent and states its own append explicitly (*"the round's 13 deferred items appended under a new … heading (in `50bdce3`)"*), i.e. the convention covers the review-recording commit as well as the fix commit. Name-level the list is complete; description-level it is not — the same "wrong in one direction" class R3-4 closed. Fix: append *"+ the round's 6 deferred items under a new round-3 heading (in `cf39687`)"*.

**Residual (genuinely new and independent — reported, not fixed in this round):**

- [x] [Review][Residual] R3-6 closes `category` **key** drift but not `category` **value** drift [`android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:263`, `:299`] — `getString` throws when the key is absent or non-string, which is the mode the finding named and the mode row H measured. A `category` whose *value* drifts (`"energy-floor"` → `"energy_floor"`, or a new vector with a third category) still parses, still returns a `String`, and still leaves that vector out of both `filter` loops with the suite green — the same vacuous-pass shape, one layer down. R3-6's second offered option, `assertEquals(loadFixture().size, energyFloorCount + stopLatencyCount)`, is the one that closes it and was not taken (option (a) was, as prescribed). Latent, not live: all 9 fixture vectors carry a valid category today (6 energy-floor + 3 stop-latency = 9). Fix shape: the partition assertion, or `require(category in setOf(…))`.
- [x] [Review][Residual] The aggregate lines D1 anchors the 168 in are **not** script output [`NOTE:27-35`; `smoke-r2-27de205.log:44`, `smoke-r3-85aa0ee.log:44`] — verified: `grep -rn "JVM (laptop\|fail 0 err" scripts/` returns nothing, and in both logs the line sits *after* the closing `╚═══╝` box followed by `primaryCpuAbi=` / `lastUpdateTime=`, i.e. a hand-appended tail. The note presents it as log content (*"Each smoke log carries an aggregate line at `:44`"*) without saying it was added by hand. D1 was raised about the provenance of the 168; the remedy anchors it in a manually written line whose manual origin is undisclosed. Fix shape: one clause — "aggregate appended by hand from the `--rerun-tasks` console output; `android-smoke.sh` does not print it".
- [x] [Review][Residual] Both logs force-added in this round carry an undisclosed staleness warning [`smoke-r2-27de205.log:22-24`, `smoke-r3-85aa0ee.log:22-24`] — *"`[warn]  APK-Timestamp nicht aktualisiert — Gradle hat inkrementell nichts neu gebaut.`"* / *"`[warn]  Kotlin-Änderungen evtl. nicht drin.`"*. The note's **Scope of these logs** paragraph (`:40-44`) states what they do not prove (no dictation, no gesture, no overlay, no HyperOS, no network) but not that the APK step was a no-op in both runs. Under *"a number states what it covers"*, evidence committed in this round should carry its own warning. Fix shape: name it in the scope paragraph.
- [x] [Review][Residual] A stray bare `0` is the last line of one of the two new logs [`smoke-r3-85aa0ee.log:47`] — it is the only reason r3 is 47 lines and r2 is 46, while the note treats the three logs as structurally identical (*"All three committed smoke logs … print"*). Almost certainly an `echo $?`. Fix shape: strip it, or label it.
- [x] [Review][Residual] `getString`'s message blames a missing key for a not-an-object element, and the edit dropped a shape check [`android/kotlin-test/com/klarvo/voice/VadGateGoldenVectorsTest.kt:80-85`, call sites `:263`, `:299`] — `(this as? Obj)?…?: error("fixture vector is missing required string key '$key' (or it is not a string)")`: if a fixture array element is not an object at all, the message names the key. The two call sites also dropped their old `(it as JsonVal.Obj)` hard cast, so the `ClassCastException` that previously surfaced that shape is gone. Same class as the already-deferred *"`getBool`'s '(or it is not a boolean)' advertises a check it does not perform"*, committed new in the round that deferred it. Fix shape: split the not-an-object branch from the missing-key branch.

**Deferred (real, out of this re-review's scope, pre-existing, or already an accepted deferral):**

- [x] [Review][Defer] Round 3's own deferred anchors into `VadGateGoldenVectorsTest.kt` were invalidated by the same round's code change [`_bmad-output/implementation-artifacts/deferred-work.md:385`, mirrored at `7-8-…md:1187`] — the item that prescribes *"cite deferred items by symbol name rather than by line"* is itself line-cited: *"(`getBool` is at `:108-113`)"* is today `:123-128` and *"`optString("signal")` is at `:260`"* is today `:275`, both +15 from R3-6's `getString` insertion; the story mirror cites `:111` for the `getBool` message, which is now inside `getDouble`'s. Its other three anchors still hold. Deferred, not patched: round findings are preserved verbatim by repo convention, and rounds 2 and 3 both accepted the identical shape as a deferred item.
- [x] [Review][Defer] AC3's twin table still cites a stale Kotlin anchor [`_bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md:125`] — `KlarvoApi.kt:1158-1165` for `fun joinChunkResults(…)`, today `:1182-1189`. Round 3's own residual, carried forward unchanged and still accurate as stated; spec-authored text from story creation, untouched by any fix round.

**Dismissed as noise (12, not persisted):** the P5 / R3-2 rows citing `llm/mod.rs:2390-2396` by line while R3-5 switched to symbols (the anchor resolves correctly today — verified, `:2390-2396` is exactly the D1 comment block in a 2398-line file; R3-5 prescribed symbol-anchoring for the deferred entry, not repo-wide) · *"skipping the R2-D1 inversion is a non-sequitur / net coverage loss"* (re-litigates decision R2-D1, Andi's explicit call, and is already the round-3 deferred item *"Nothing pins that `open_decision: "M12"` is present"*) · the new logs showing `Gerät: emulator-5554` while the round says *"no emulator"* (the record states at `:1179-1180` that AC6b's runtime proof is the earlier laptop-AVD evidence, **not re-run here**) · `Dauer: 1s`/`2s` read as proof of a replayed gradle run (`Dauer` prints `BUILD_SECS`, the APK-build time only — already a round-2 deferred item) · no committed log for `5128432`'s own re-run (no round committed a post-fix log; the round-3 gate paragraph states its method and its exclusions) · the dismissed-noise slot count 12 vs 13 and dismissal 9's rebuttal naming P3/P5 where the verdict line marks R2-1/R2-9 (bookkeeping inside a previous round's dismissed list, preserved verbatim) · the round-3 residual marked `[x]` while unfixed (repo convention; round 3 dismissed the identical point) · `:1104`'s *"0 errors"* vs `:1176`'s *"0 errors, 0 skipped"* (same run, both name their coverage) · the 168 lacking a build-variant qualifier in the record (the note carries it: *"the `:app:testUniversalDebugUnitTest` variant total"*) · `:813`'s *"only assertion-level change"* for what is a reader swap (subsumed by the fifth residual) · six hypothetical fixture-authoring guards from the edge layer (`amplitude_short` Short range, `expected_frames` integrality, `silence_secs` floor arm, `getBool` numeric coercion, `optString("signal")` type mismatch, `id` as a label-only key) — all already round-2/round-3 deferred items or explicitly dismissed there · `category` as an empty/whitespace string treated as a separate defect (it is the same value-drift path as the first residual).
- 2026-09-10: **Close-out → done (GATE 3, Andi).** Three fix rounds (the third authorized by Andi beyond
  the cap of two), loop ended on review round 4 (`17197d9`): all round-3 findings resolved, no code
  regression, gates Rust 662/0 + JVM 168/0 (22 suites). Review-4 D1 → (a) `jvm-suites-5128432.md`;
  its 5 record/note patches applied in this close-out commit. GATE-4 proxy GREEN on the laptop AVD
  (`gate4-evidence/7-8/verdict.md`: arm64 install + Rust-side JNI proof on `5128432`). Residuals
  (5 + 2 deferred) → `docs/backlog.md` "Story 7-8 residuals". **Open real-device gate (Andi):** one
  Xiaomi dictation with DeepSeek cleanup — close with a docs commit here.
