# Story 7.8: Parity-net close-out + twin hygiene

Status: ready-for-dev

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

- [ ] **Task 1 — M9: DeepSeek URL parity** (AC1)
  - [ ] Change `KlarvoApi.kt:164` and `:198` to `https://api.deepseek.com/v1/chat/completions`.
  - [ ] Consider extracting the literal to one `private const val` so the two sites cannot drift
        again (the file already comments at `:194` that the *candidate list lives in exactly one
        place* — honor that intent). Judgement call: only if it does not disturb the surrounding
        `LlmProviderInfo` construction.
  - [ ] Add the JVM test pinning the URL for **both** sites.
  - [ ] Do **not** touch `commands/settings.rs:979`.

- [ ] **Task 2 — ADR-0017 boundary guard** (AC2)
  - [ ] Decide the host: JVM test in `android/kotlin-test/` **or** a step in `scripts/android-smoke.sh`.
        (A JVM test is the recommended default — it is device-free, runs in the same gate as
        everything else here, and is visible to `dev-story` without a device. Either is sanctioned by
        the epic.)
  - [ ] Implement so it is **GREEN on the clean tree** — honor the comment/KDoc + allowlist trap above.
  - [ ] Prove RED by adding a fake Kotlin STT/guard declaration, then revert (AC8).

- [ ] **Task 3 — Twin-constant fixture + both harnesses** (AC3)
  - [ ] Add the fixture to `test-fixtures/` (suggested `twin-constants-vectors.json`, matching the
        existing `*-vectors.json` naming). Flat JSON array of objects, one per constant, each with
        `id`, `description` (what it pins), and the expected value.
  - [ ] Rust test: reuse the established loader shape (`llm/mod.rs:1949-1958` /
        `pipeline.rs:4151-4164`) — `CARGO_MANIFEST_DIR` → `.parent()` → `join("test-fixtures/…")`.
  - [ ] Kotlin test: reuse the repo-root candidate resolver (`ChunkingVectorsTest.kt:108-124`).
        **Prefer `org.json.JSONObject`/`JSONArray`** (already on the unit-test classpath via the
        gradle patch; `getDouble`/`getInt` **throw** on a missing key) over the hand-rolled `JsonVal`
        parser — the hand-rolled `optDouble` default is precisely the vacuous-pass defect of R3-P2.
  - [ ] Assert against production symbols, not re-declared literals.

- [ ] **Task 4 — M12 current-state vector** (AC4)
  - [ ] Record today's divergence as a fixture entry, explicitly labelled current-state.
  - [ ] Change **no** prompt-assembly code on either platform.

- [ ] **Task 5 — 7-2 residuals** (AC5)
  - [ ] Fix R3-P1 … R3-P8 exactly as prescribed above (8 items).
  - [ ] Fix **R3-P2 EXT** — same throwing-accessor remedy at `VadGateGoldenVectorsTest.kt:195`
        (`silence_threshold`) and `:231` (`silence_secs`).
  - [ ] Fix **R3-P8 EXT** — strike the second copy of the false "without calling the VAD model"
        claim at `KlarvoAudioRecorder.kt:47-50`. Same remedy as R3-P8: delete the claim, **no**
        short-circuit. Leave the `config/mod.rs:209` anchor and the `0.005` default alone (both
        verified correct).
  - [ ] Re-verify every line anchor against the tree before editing (project-context: *grep before
        declaring done* — prose drifts).

- [ ] **Task 6 — `android-smoke.sh` traps** (AC6)
  - [ ] (a) prune on copy for `kotlin-src` **and** `kotlin-test`.
  - [ ] (b) `--abi arm64-v8a -r -g` on `emulator-*`; mirror `scripts/android-emulator-smoke.sh`.

- [ ] **Task 7 — `test-fixtures/README.md`** (AC7) — one paragraph, two commands, fixture list,
      "no CI" stated.

- [ ] **Task 8 — Gates + inversion evidence** (AC8, DoD)
  - [ ] `cargo test --lib` in `src-tauri/` green (7-2 measured **657 passed, 0 failed** as the
        baseline — report the new number **and** what it does not cover).
  - [ ] `scripts/android-smoke.sh` JVM gate green.
  - [ ] Emulator smoke proves the install fix: fresh APK + **one JNI call succeeds** (this is the
        specific proof that (b) worked — a green smoke that never reaches JNI proves nothing; that is
        exactly how the trap hid).
  - [ ] Record the inversion table; confirm `git status` clean.
  - [ ] **[HUMAN GATE — Andi] GATE-4:** one dictation on the Xiaomi with DeepSeek cleanup returns
        cleaned text (proves M9 on the real path).

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
  **Known topology constraint:** powerhouse has **no AVD** (`tools/android-sdk/` lacks `emulator/` +
  `system-images/`) — the emulator proxy runs on the **laptop**. If the emulator is unreachable,
  AC6b's proof is `blocked`; report it, do **not** install tooling around it.
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

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List

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
