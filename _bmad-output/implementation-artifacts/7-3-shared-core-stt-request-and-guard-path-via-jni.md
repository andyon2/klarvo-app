# Story 7.3: Shared-core STT request + guard path via JNI

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->
<!-- Test-Architect REQUIRED before dev-story: run *risk + *design on this story (crosses the JNI boundary, primary use case). See Dev Notes → "Pre-dev: Test-Architect gate". -->

## Story

As a klarvo user,
I want both platforms to send the **same** STT request and apply the **same** hallucination/silence guards,
so that dictation quality is identical and stockphrase ghosts never reach my pasted text — on either device.

## Context & Governing Decisions

This is the **re-scoped Epic 7 centerpiece** (correct-course 2026-06-12). It is a brownfield
**re-architecture**, not a feature: the STT request + STT-output guards + pre-STT silence filter
are consolidated into the **Rust core**, and Android consumes them **over JNI** — the Kotlin twins
are deleted. Governed by **ADR-0017** (Hard Rule: shared STT/guard logic lives only in Rust;
a Kotlin re-implementation is forbidden). Supersedes ADR-0016 DIV-08 + DIV-11 for the STT path
via consolidation instead of per-row porting.

It absorbs these audit rows from the earlier per-row plan: H3, Recall #5, H9, H10, L3 (old 7.3) ·
H6, H7 (old 7.4) · Recall #1, M1-read (silence filter, from old 7.2) · H14 (from old 7.6) ·
**plus new hallucination hardening** not in the original audit (source:
`docs/dictation-quality-android-vs-desktop-2026-06-12.md`).

**Engine fact (load-bearing, do not re-litigate):** both platforms already run the *identical*
Groq engine + model (`whisper-large-v3-turbo`); the phone does **not** run local Whisper
(verified: phone `config.json` `sttProvider=groq`, code dispatch, 46/46 runtime logs `provider=groq`).
There is no engine to converge. The defect is purely the **two divergent request/guard strands**.

## Acceptance Criteria

Full Given/When/Then below. The deletions (Kotlin twins) are as load-bearing as the additions —
a Kotlin re-implementation surviving is an ADR-0017 violation.

### AC1 — Single Rust STT request path (Android transcribes via JNI)

**Given** Android is configured for Groq STT (the only active provider),
**When** the overlay service transcribes a recorded WAV,
**Then** the request is built and sent by the **Rust** `WhisperStt`/`GroqWhisper` path
(`src-tauri/src/stt/mod.rs`) reached over a JNI bridge function,
**And** `KlarvoApi.transcribe` (`KlarvoApi.kt:554`) and `KlarvoApi.buildMultipartBody`
(`KlarvoApi.kt:965`) are **deleted**,
**And** the Android call sites (`KlarvoOverlayService.kt:1358` `transcribeWithRetry`, and the pre-STT
filter at `:947`, the hallucination guard at `:1091`) route through the new JNI surface,
**And** this subsumes by construction the old-7.3 rows:
- **H3 / Recall #5** — prompt conditioning (language hint + dictionary terms + `customPrompt`) is built
  by the Rust `build_stt_prompt_with_hint` (`stt/mod.rs:112`), not re-assembled in Kotlin.
- **H9** — the `sttModel` config value selects the model on the Rust side (no hardcoded
  `whisper-large-v3-turbo` literal in Kotlin — currently `KlarvoApi.kt:972`).
- **H10** — `localWhisperModel` config is read on the Rust side (parity, even though Groq is active).
- **L3** — STT temperature parity: the Rust `temperature` (`stt/mod.rs:174`, default `0.0`) is the single
  source, not Kotlin's separate value.

### AC2 — Shared STT-output guards (one Rust implementation, both platforms)

**Given** a transcription returns from Groq,
**When** the post-STT guards run,
**Then** the prompt-echo guard `is_prompt_echo` (**H6**, `pipeline.rs:234`) and the fragment-strip
`strip_prompt_fragments` (**H7**, `pipeline.rs:345`) are the **single Rust guards** both platforms
inherit over JNI,
**And** the hallucination filter `is_hallucination` (`stt/hallucination.rs:146`) is the single Rust guard,
**And** `HallucinationFilter.kt` is **deleted** and its call site (`KlarvoOverlayService.kt:1091`) calls
the Rust guard over JNI.

### AC3 — H14 regression guard (whole-word match adopted in the SAME story)

**Given** the Kotlin twin already has the whole-word fix (ROB-03, `HallucinationFilter.kt:100-109`)
but the Rust filter still substring-matches single-word entries (`stt/hallucination.rs:160-163`,
`lower.contains(phrase)`),
**When** the shared Rust filter becomes the single source and the Kotlin twin is deleted,
**Then** the Rust filter MUST adopt **whole-word matching for single-word blocklist entries** in this
story (multi-word entries stay substring-matched),
**So that** deleting the Kotlin twin does **not** regress Android on an already-fixed behavior.
**Golden-vector / test:** "Standard", "Milliarde", "Hardware" in a short utterance → **not** discarded
(would be wrongly killed by the old `contains("ard")`); "ZDF", "amara.org" short utterances → still
discarded. Verified on the Rust side; locked by a 7.7 fixture.

### AC4 — Shared pre-STT silence filter (one Rust source)

**Given** a recorded WAV before the STT call,
**When** the pre-STT silence/duration filter runs,
**Then** one **Rust** pre-STT silence filter (the `silence_skip` + `compute_wav_rms` logic,
`pipeline.rs`) is consumed by both platforms over JNI,
**And** `SilencePreFilter.kt` is **deleted** and its call site (`KlarvoOverlayService.kt:947-970`,
the `TooShort`/`Silent`/`Pass` branches) routes through the Rust result,
**And** the `0.02f` vs `0.005f` self-desync (**Recall #1**) and the `silenceThreshold` config-read
(**M1-read**) are resolved **by construction** (no second source to desync against),
**And** boundary parity holds: exactly `MIN_RECORDING_MS` → Pass (`<`, not `<=`); exactly
`SILENCE_THRESHOLD` → Pass; malformed-WAV RMS → skip silent check → Pass (current Kotlin contract,
`SilencePreFilter.kt:19-23`).

### AC5 — Hallucination hardening: stockphrase family + long-clip trailing ghosts

**Given** real observed desktop hallucinations — the `Groß- und Kl(inge|ingel|einschreibung)…`
stockphrase family, `Untertitelung des ZDF, 2020`, `amara.org`/credit/subtitle lines, `[Musik]`,
subscribe/thank-you sign-offs — concentrated on short/silent clips **and** as trailing ghosts on
**long** clips (~1.1%),
**When** the shared Rust hallucination guard runs,
**Then** the stockphrase family is blocklisted,
**And** the trailing-ghost match runs **regardless of clip length** — i.e. the `word_count > 8` gate
(`stt/hallucination.rs:155-158`) that lets long-clip trailing ghosts pass is removed **for the
trailing-ghost / stockphrase-family match** (the gate's original false-positive-prevention intent for
short incidental mentions must be preserved for the generic single-word entries — see Dev Notes for the
shape; this is a `*design` decision, not a blanket gate removal).

### AC6 — Confidence-based segment drop (verbose_json)

**Given** Groq supports `response_format=verbose_json` with per-segment metadata,
**When** the Rust STT request is built,
**Then** `response_format` switches from `json` (`stt/mod.rs:240`) to `verbose_json`,
**And** segments are dropped by `no_speech_prob` / `compression_ratio` / `avg_logprob` thresholds
(thresholds chosen in `*design`, locked by golden-vector fixtures),
**And** the response parser tolerates both shapes during transition (no crash if a segment field is
absent) — fail-soft, structured error, never panic.

### AC7 — Cleanup-no-invent guard

**Given** LLM cleanup can rationalize a recognizable ghost (`Klinge`) into the convincing full
stockphrase (`Kleinschreibung`), turning detectable noise into fluent undetectable noise,
**When** cleanup runs,
**Then** the stockphrase family is stripped **after** cleanup as well (and/or the cleanup prompt is
constrained), so cleanup cannot manufacture the full stockphrase from a ghost fragment.

### AC8 — Verifiability split (named decision, not an accidental gap)

- **Stockphrase blocklist + paste path** → **live on-device Android smoke** (Andi-reproducible:
  short/silent clips reliably trigger the ghosts). This is the human gate.
- **Confidence-drop (`verbose_json`)** → **golden-vector fixtures** (recorded/synthetic segment
  metadata → expected drop). Its human gate is **deliberately downgraded** to "fixture-verified";
  it is **not** cleanly Andi-reproducible (needs the specific audio producing a low-confidence segment).
  This downgrade is an explicit, recorded decision (Andi's Verifikations-Symmetrie rule), not an oversight.

### AC9 — ADR-0017 boundary holds (no Kotlin STT/guard logic survives)

**Given** the Hard Rule (ADR-0017),
**Then** after this story there is **no** Kotlin implementation of any STT-request or STT-guard
behavior: `KlarvoApi.transcribe`, `buildMultipartBody`, `HallucinationFilter.kt`,
`SilencePreFilter.kt` are gone; a grep for a parallel Kotlin STT request/guard returns nothing.
(7.7 later pins this as a golden-vector: Android transcription enters via the JNI bridge only.)

### AC10 — No regression on either platform

**Given** core STT code now crosses the JNI boundary,
**Then** the existing Rust desktop pipeline behavior is unchanged for desktop (the guards keep their
desktop call sites in `pipeline.rs`),
**And** the JNI functions are **panic-safe** (a Rust panic into the JVM is an unrecoverable crash —
every conversion checked, fail-soft return on error, per the established `stt/jni_bridge.rs:30-36`
and `license/jni.rs:20-24` convention),
**And** the Linux/desktop build and Android build both compile (heavy deps stay `#[cfg]`-gated).

## Tasks / Subtasks

- [ ] **Task 0 — Pre-dev Test-Architect gate (REQUIRED, before any code)** — run `*risk` then `*design`
  on this story. The central decision `*design` must settle: **how the async Groq network request runs
  from a JNI context with no Tokio runtime** (see Dev Notes → "The central architectural question").
  Do not start Task 1 until `*design` has fixed the runtime approach + the JNI surface shape.
- [ ] **Task 1 — Define the Rust JNI surface for the Groq STT request** (AC1)
  - [ ] Add the new `#[no_mangle] Java_com_klarvo_voice_*` function(s) (new module e.g.
        `stt/groq_jni.rs` or extend `stt/jni_bridge.rs`), `#![cfg(target_os = "android")]`.
  - [ ] Inputs Kotlin must pass: WAV bytes (base64 or direct byte[]), api key, language, dictionary
        terms, `customPrompt`, `sttModel`, temperature — i.e. everything `build_form` +
        `build_stt_prompt_with_hint` need. Mirror the license-bridge param-marshalling convention.
  - [ ] Run the request via the runtime approach decided in Task 0; return transcript or a structured
        error string (fail-soft, panic-safe).
- [ ] **Task 2 — Expose the guards over JNI** (AC2, AC3, AC4)
  - [ ] JNI functions for `is_hallucination`, `is_prompt_echo`, `strip_prompt_fragments`, and the
        silence/duration pre-filter (`silence_skip` + `compute_wav_rms`).
  - [ ] Decide whether the guards run **inside** the Rust transcribe JNI call (single round-trip, fewer
        boundary crossings — preferred) or as separate JNI calls Kotlin orchestrates. `*design` input.
- [ ] **Task 3 — H14: whole-word match in the shared Rust filter** (AC3)
  - [ ] Port the Kotlin whole-word logic (`HallucinationFilter.kt:100-109`) into
        `stt/hallucination.rs`: single-word entries → whole-word; multi-word → substring.
  - [ ] Add Rust unit tests: "Standard"/"Milliarde"/"Hardware" pass; "ZDF"/"amara.org" blocked.
        (Prove RED→GREEN: the new test must fail on the current `contains` impl.)
- [ ] **Task 4 — Hallucination hardening** (AC5, AC7)
  - [ ] Add the stockphrase family + new entries to the blocklist; implement the trailing-ghost match
        that runs regardless of clip length **without** re-introducing the short-incidental-mention
        false positives (shape per `*design`).
  - [ ] Cleanup-no-invent: strip the stockphrase family post-cleanup and/or constrain the cleanup prompt.
- [ ] **Task 5 — verbose_json + confidence drop** (AC6)
  - [ ] Switch `response_format` to `verbose_json` in `build_form` (`stt/mod.rs:240`).
  - [ ] Parse segments; drop by `no_speech_prob`/`compression_ratio`/`avg_logprob` (thresholds from
        `*design`). Tolerate both response shapes (fail-soft).
- [ ] **Task 6 — Delete the Kotlin twins + reroute call sites** (AC1, AC2, AC4, AC9)
  - [ ] Delete `KlarvoApi.transcribe` + `buildMultipartBody`, `HallucinationFilter.kt`,
        `SilencePreFilter.kt`.
  - [ ] Reroute `KlarvoOverlayService.kt` call sites (`:947` pre-filter, `:1091` hallucination,
        `:1358` transcribe) + add/keep the native declarations on the Kotlin side.
  - [ ] Keep `transcribeWithRetry`'s retry/4xx semantics (`:1343-1365`) — decide whether retry stays in
        Kotlin around the JNI call or moves to Rust (`*design`; current behavior must be preserved).
- [ ] **Task 7 — Golden-vector seeds for 7.7** (AC3, AC5, AC6, AC8)
  - [ ] Author fixtures (these are *consolidated* by 7.7, but seed them here): stockphrase ghosts on
        short clips → stripped; trailing ghost on a long clip → stripped; low-confidence segment →
        dropped; prompt-body assembly given dictionary + lang hint + customPrompt; H14 whole-word cases.
- [ ] **Task 8 — Builds + tests + on-device smoke** (AC8, AC10)
  - [ ] `cargo test` (Rust guards + new logic) green on Linux.
  - [ ] Android build via `scripts/android-build.sh` (freshness gate — there is **no** in-UI version
        screen; verify APK timestamp/filename, not a version number).
  - [ ] **On-device smoke** via `scripts/android-smoke.sh`: short/silent clips produce **no** stockphrase
        ghost in the pasted text; normal dictation still pastes correctly. Confidence-drop sub-part is
        fixture-verified (named downgrade, AC8) — do **not** ask Andi to reproduce it.

## Dev Notes

### The central architectural question (settle in `*design` before coding)

The proven JNI pattern on this codebase (license `22553bc`, `src-tauri/src/license/jni.rs`) and the
existing `stt/jni_bridge.rs` are both **pure / blocking**: a `#[no_mangle]` fn marshals args, calls
shared Rust, returns a string. **The Groq STT request is different — it is an async `reqwest` network
call**, and the existing bridge explicitly notes: *"From JNI there is no Tokio runtime available"*
(`stt/jni_bridge.rs:24-29`), which is why local whisper uses a `transcribe_blocking` helper.

`GroqWhisper`/`WhisperStt::transcribe` is `async fn` (`stt/mod.rs:82, 268`) over `reqwest`. So the
network call must either (a) get a blocking entry point (build a small `reqwest::blocking` path or
`block_on` a scoped runtime inside the JNI fn), or (b) reuse a managed runtime. This is the #1 risk and
the primary `*design` decision. Note: `reqwest` MUST keep `default-features = false` + `rustls-tls`
(no native OpenSSL) — do not pull `reqwest::blocking` if it drags incompatible features; verify the
feature set. Android already links the Rust cdylib, so reqwest-over-JNI is feasible; the question is
*shape*, not *possibility*.

### Files to MODIFY (read fully before changing — current state documented)

- `src-tauri/src/stt/mod.rs` — `WhisperStt` (the generic Groq/OpenAI struct), `build_form` (`:220-251`,
  currently `response_format=json` at `:240`), `build_stt_prompt_with_hint` (`:112`), `with_model`/
  `with_temperature`. **Change:** `verbose_json` + segment-drop; this struct's request becomes the JNI
  entry's body. **Preserve:** the desktop `SttProvider::transcribe` path and trait contract.
- `src-tauri/src/stt/hallucination.rs` — `is_hallucination` (`:146`), `word_count > 8` gate (`:155-158`),
  `lower.contains(phrase)` (`:160-163`), `HALLUCINATION_BLOCKLIST` (`:49`). **Change:** whole-word match
  (H14), stockphrase family, length-independent trailing-ghost match. **Preserve:** the existing
  passing tests' intent (long incidental "ZDF" mention still passes).
- `src-tauri/src/pipeline.rs` — `is_prompt_echo` (`:234`), `strip_prompt_fragments` (`:345`),
  `silence_skip`/`compute_wav_rms`, `is_hallucination` call site, `PostSttSkip` (`:493`). These guards
  are **already the desktop source**; the work is exposing them over JNI, not rewriting desktop behavior.
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — call sites `:947` (pre-filter),
  `:1091` (hallucination), `:1343-1365` (`transcribeWithRetry` → `KlarvoApi.transcribe`). Reroute to JNI;
  preserve retry/4xx semantics.
- `src-tauri/src/lib.rs` — `#[cfg(target_os = "android")]` wiring (`:1142-1162`) if a new native fn set
  needs registration/visibility.

### Files to DELETE (the deletions are AC, not cleanup)

- `android/.../KlarvoApi.kt` → `transcribe` (`:554`) + `buildMultipartBody` (`:965`) only (the file has
  other surface — `cleanup`, `cleanupChunked` — that stays; LLM path is 7.5, not this story).
- `android/.../HallucinationFilter.kt` → whole file.
- `android/.../SilencePreFilter.kt` → whole file.
- Note `LocalWhisperInference.kt` + `stt/jni_bridge.rs` are the **local-whisper** path (inactive, Groq is
  used). Do **not** delete them as part of this story unless `*design` explicitly folds them in; they are
  a different surface.

### Established conventions this story must follow (from project-context.md)

- **JNI panic-safety:** functions must not panic into the JVM; check every conversion, fail-soft return
  (empty/structured-error string), log via `log::error!`. Pattern: `license/jni.rs`, `stt/jni_bridge.rs`.
- **Fail-soft, never `todo!()`/`unimplemented!()`/`panic!()`** — structured `AppError`.
- **Platform-gate heavy deps** — mirror existing `#[cfg(target_os = "android")]` / `cfg(windows)`; never
  add an unconditional dep/`use` that breaks the Linux or Android build.
- **Android = native Kotlin + JNI, bypasses Tauri IPC (~85%)** — the overlay service can't call Tauri
  commands; JNI is the only bridge. This story shrinks that duplicate for the STT path (vs. ADR-0016's
  "grows minimally").
- **`jni` is pinned at 0.21** (NOT 0.22 — that is the v2 archive). Use the 0.21 API.
- **No remote telemetry / BYOK** — the only network call added is the Groq STT request the user already
  makes; add nothing that phones home.
- **Config is `config.json` (camelCase keys via serde rename_all)** — e.g. `sttModel`, `silenceThreshold`,
  `customPrompt`. A snake_case key is silently ignored by serde.
- **Tests bound to real code paths**, inline `#[cfg(test)]`, not a parallel mock.

### Verifiability symmetry (Andi can/can't produce the test state)

- **CAN reproduce** (so it's a real human gate): short/silent clips → stockphrase ghosts. On-device smoke
  is genuine.
- **CANNOT cleanly reproduce** (so it's fixture-gated, named downgrade): a specific low-confidence
  `verbose_json` segment. Do not hand Andi an impossible test — AC8 records this split deliberately.

### Sequencing / scope guards

- 7.3 is **first** in Epic 7 (highest-value core change); 7.1 is independent/parallel. 7.7 (the parity
  net) runs **last** and consolidates the fixtures seeded here. Do **not** build the full 7.7 net here.
- ADR-0017 is **STT-only**: do **not** pull the live auto-stop VAD gate (7.2), chunking (7.1) or
  LLM-routing (7.5) into Rust-over-JNI in this story — explicitly deferred (realtime-stream / out-of-path).

### Project Structure Notes

- New Rust JNI surface fits under `src-tauri/src/stt/` next to `jni_bridge.rs` (local) — name it
  distinctly (e.g. `groq_jni.rs`) so the active cloud path is not confused with the inactive local path.
- Kotlin native declarations live on a Kotlin object/class under `com.klarvo.voice`; mirror the existing
  `LocalWhisperInference` / `LicenseValidator` native-method declaration style.

### References

- [Source: docs/adr/0017-shared-core-stt-path.md] — Hard Rule, decision, scope, named regression trap (H14).
- [Source: docs/adr/0016-android-path-parity-strategy.md#Amendment 2] — STT-path line shift to consolidation.
- [Source: _bmad-output/planning-artifacts/sprint-change-proposal-2026-06-12.md#Section 4a] — re-scoped 7.3 spec, verifiability split.
- [Source: _bmad-output/planning-artifacts/epics-cross-platform-parity.md#Story 7.3] — outcome-level ACs + row IDs.
- [Source: docs/dictation-quality-android-vs-desktop-2026-06-12.md] — evidence run (stockphrase family, trailing ghosts, cleanup amplification).
- [Source: src-tauri/src/license/jni.rs] — proven pure JNI consolidation pattern (license, `22553bc`).
- [Source: src-tauri/src/stt/jni_bridge.rs:24-36] — JNI no-Tokio-runtime note + panic-safety convention.
- [Source: src-tauri/src/stt/mod.rs:220-251] — `build_form` (response_format, model, temperature, prompt).
- [Source: src-tauri/src/stt/hallucination.rs:146-164] — current filter (word-count gate + substring match).
- [Source: src-tauri/src/pipeline.rs:234,345] — `is_prompt_echo` (H6), `strip_prompt_fragments` (H7).
- [Source: android/.../HallucinationFilter.kt:100-109] — Kotlin whole-word fix to adopt (H14/ROB-03).
- [Source: android/.../SilencePreFilter.kt] — Kotlin silence filter contract to fold into Rust.
- [Source: _bmad-output/project-context.md] — JNI panic-safety, jni 0.21 pin, platform-gating, config camelCase, Android on-device smoke DoD.

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List
