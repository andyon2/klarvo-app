---
workflowStatus: 'completed'
totalSteps: 5
stepsCompleted: ['step-01-detect-mode','step-02-load-context','step-03-risk-and-testability','step-04-coverage-plan','step-05-generate-output']
lastStep: 'step-05-generate-output'
nextStep: ''
lastSaved: '2026-06-12'
inputDocuments:
  - _bmad-output/implementation-artifacts/7-3-shared-core-stt-request-and-guard-path-via-jni.md
  - _bmad-output/planning-artifacts/epics-cross-platform-parity.md
  - _bmad-output/planning-artifacts/sprint-change-proposal-2026-06-12.md
  - docs/adr/0017-shared-core-stt-path.md
  - docs/adr/0016-android-path-parity-strategy.md
  - docs/dictation-quality-android-vs-desktop-2026-06-12.md
  - _bmad-output/project-context.md
  - src-tauri/src/stt/mod.rs
  - src-tauri/src/stt/hallucination.rs
  - src-tauri/src/stt/jni_bridge.rs
  - src-tauri/src/license/jni.rs
  - src-tauri/src/pipeline.rs
  - android/kotlin-src/com/klarvo/voice/KlarvoApi.kt
  - android/kotlin-src/com/klarvo/voice/HallucinationFilter.kt
  - android/kotlin-src/com/klarvo/voice/SilencePreFilter.kt
  - android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt
---

# Test Design: Epic 7 — Story 7.3 (Shared-core STT request + guard path via JNI)

**Date:** 2026-06-12
**Author:** Andi (Master Test Architect run)
**Mode:** Epic-Level (story-scoped to 7.3, the Epic-7 consolidation centerpiece)
**Status:** Draft
**Governing:** ADR-0017 (Hard Rule), ADR-0016 Amendment 2

---

## Executive Summary

**Scope:** Risk-based test design for the re-architecture that single-sources the STT request +
STT-output guards + pre-STT silence filter in the Rust core and consumes them on Android **over JNI**,
deleting the Kotlin twins.

**Risk Summary:**
- Total risks identified: **9**
- High-priority risks (≥6): **4** (R-001 … R-004)
- Critical category: **TECH** (the JNI boundary crossing) + **DATA** (output correctness regressions)

**Coverage Summary:**
- P0 scenarios: **~15 tests** (~30–40 h) — the JNI runtime path, H14 regression guard, panic-safety, verbose_json parse tolerance
- P1 scenarios: **~25 tests** (~25–35 h) — stockphrase hardening, silence-filter parity, prompt assembly, retry semantics, confidence-drop fixtures
- P2/P3 scenarios: **~15 tests + 2 manual smokes** (~12–20 h)
- **Total effort:** **~67–95 h (~9–12 days)** — consistent with the "High effort" rating in the sprint-change-proposal

**Headline:** One risk gates everything — **R-001 (async Groq request over a JNI context with no Tokio
runtime)** is score **9 / BLOCK**. It must be resolved in design (settle the runtime shape) **before**
`dev-story` starts. Everything else is mitigable with the standard Rust/Kotlin/golden-vector layers.

---

## Not in Scope

| Item | Reasoning | Mitigation |
| --- | --- | --- |
| **Live auto-stop VAD gate over JNI** | Realtime ~31 Hz frame stream; large JNI lift + speech-truncation risk. ADR-0017 Scope is STT-only. | Stays a Kotlin per-row fix in **Story 7.2**; 7.7 golden-vectors pin it against re-drift. |
| **Chunking (7.1) / LLM routing (7.5) consolidation** | Off the felt-quality path; Hard Rule is STT-only for now. | Per-row Kotlin fixes; 7.7 parity net. |
| **Full 7.7 golden-vector net build** | 7.7 consolidates fixtures; 7.3 only *seeds* them. | This story authors the seed fixtures; 7.7 wires the cross-platform CI harness. |
| **Final NFR PASS/CONCERNS/FAIL evidence audit** | test-design plans validation; it does not assess implemented evidence. | Run `*nfr` / `*trace` after dev. |
| **Local-whisper path (`LocalWhisperInference.kt`, `stt/jni_bridge.rs`)** | Inactive (Groq is the live engine); different surface. | Left untouched unless design explicitly folds it in. |

---

## Testability Review (controllability / observability / verifiability)

**🚨 Testability Concerns (actionable)**

1. **Verifiability asymmetry — confidence-drop is NOT human-reproducible (ASR, ACTIONABLE).**
   Andi can reliably produce stockphrase ghosts (short/silent clips) → real on-device smoke. He **cannot**
   cleanly produce a specific low-confidence `verbose_json` segment (needs the exact audio). Per the
   Verifikations-Symmetrie rule this is a controllability gap. **Resolved by design decision (AC8):** the
   confidence-drop sub-part is gated by **golden-vector fixtures** (segment metadata → expected drop), and
   its human gate is a **named downgrade** — do not hand Andi an impossible test.
2. **JNI boundary is observable only on a device/emulator (ACTIONABLE).** The Groq-request-over-JNI round
   trip can't be exercised by Linux `cargo test`. Controllability mitigation: split the logic so the
   *guards + request-building* are pure Rust unit-testable on Linux, and only the thin JNI marshalling +
   network call needs the device. Keep the network boundary mockable (wiremock on the Rust side, per
   ADR-0005 stack) so the request shape is asserted without a live Groq call.
3. **Both-shape response parsing (ACTIONABLE).** Switching `response_format` to `verbose_json` changes the
   desktop parse path too — observability requires fixtures of *both* `json` and `verbose_json` Groq
   responses to prove desktop didn't regress.

**✅ Testability strengths**
- The guard functions (`is_hallucination`, `is_prompt_echo`, `strip_prompt_fragments`, `silence_skip`,
  `compute_wav_rms`) are already pure, already have inline `#[cfg(test)]` coverage — extending them is cheap.
- The JNI consolidation pattern is **proven** (license `22553bc`, `license/jni.rs`) with an established
  panic-safety + fail-soft convention to copy.
- Golden-vector fixtures give a deterministic, device-free assertion layer for output correctness.

---

## Risk Assessment

### High-Priority Risks (Score ≥6)

| Risk ID | Cat | Description | P | I | Score | Action |
| --- | --- | --- | --- | --- | --- | --- |
| **R-001** | TECH | **Async Groq `reqwest` request runs from a JNI context with no Tokio runtime** (`stt/jni_bridge.rs:24-29`). `GroqWhisper::transcribe` is `async`. → **RESOLVED 2026-06-12: Weg A** (per-call throwaway runtime + `block_on`; ANR already handled by background-thread call site). Residual = prove in first P0 test. | 3 | 3 | **9→prove** | RESOLVED |
| **R-002** | DATA | **H14 regression:** Kotlin twin (whole-word, `HallucinationFilter.kt:100-109`) deleted while Rust filter still substring-matches (`hallucination.rs:160-163`) → "Standard"/"Milliarde"/"Hardware" silently discarded on Android. | 2 | 3 | **6** | MITIGATE |
| **R-003** | OPS | **JNI panic into the JVM = unrecoverable Android crash.** New network + base64 + verbose_json parse code has more panic surfaces than the pure license bridge. | 2 | 3 | **6** | MITIGATE |
| **R-004** | DATA | **`verbose_json` parse change regresses the desktop pipeline.** Desktop currently parses `response_format=json` (`stt/mod.rs:240`); a non-tolerant parser breaks desktop STT. | 2 | 3 | **6** | MITIGATE |

### Medium-Priority Risks (Score 3–4)

| Risk ID | Cat | Description | P | I | Score | Action |
| --- | --- | --- | --- | --- | --- | --- |
| **R-005** | BUS | **Over-aggressive hardening false-positives:** removing the ≤8-word gate for the trailing-ghost match also kills legitimate long dictation ending on a blocklist term. | 2 | 2 | 4 | MITIGATE |
| **R-006** | TECH | **Cross-platform build breakage:** new deps / `reqwest::blocking` drag native-TLS features, breaking the `default-features=false`+`rustls-tls` constraint or the Linux/Android build. | 2 | 2 | 4 | MITIGATE |
| **R-007** | OPS | **Retry/4xx semantics lost** when `transcribeWithRetry` (`KlarvoOverlayService.kt:1343-1365`, 2 s/5 s, no-retry-on-4xx) is rerouted around the JNI call. | 2 | 2 | 4 | MITIGATE |
| **R-008** | BUS | **Cleanup-no-invent leaks:** LLM cleanup still manufactures `Kleinschreibung` from `Klinge` if only the prompt is constrained (non-deterministic). | 2 | 2 | 4 | MITIGATE |

### Low-Priority Risks (Score 1–2)

| Risk ID | Cat | Description | P | I | Score | Action |
| --- | --- | --- | --- | --- | --- | --- |
| **R-009** | OPS | Confidence-drop thresholds tunable only via fixtures (named verifiability downgrade, AC8). | 2 | 1 | 2 | MONITOR |

### Category legend
TECH = technical/architecture · SEC = security · PERF = performance · DATA = data integrity · BUS = business/UX · OPS = operations.

---

## NFR Planning

| NFR Category | Requirement / Threshold | Risk Link | Planned Validation | Evidence Needed |
| --- | --- | --- | --- | --- |
| Reliability | No JVM crash from any JNI input; fail-soft on every error path | R-001, R-003 | Fuzz/edge inputs to the JNI fn (bad base64, empty/oversized WAV, malformed response); assert structured fallback, never panic | Rust+integration test report; on-device no-crash run |
| Reliability | Desktop STT unchanged after `verbose_json` switch | R-004 | Both-shape parse fixtures; existing desktop pipeline tests stay green | `cargo test` report; desktop smoke |
| Correctness (DATA) | Real speech never discarded; ghosts always discarded | R-002, R-005 | H14 whole-word RED→GREEN tests + stockphrase golden-vectors (positive & negative) | Golden-vector fixtures (both platforms) |
| Performance (ANR) | STT request must not block the Android UI/overlay thread | R-001 | Confirm the JNI call runs off the main thread (design); manual responsiveness check during smoke | Design note + on-device observation |
| Maintainability | No Kotlin STT/guard re-implementation survives (ADR-0017 boundary) | — | Static check / grep assert in CI | CI grep gate (seeded here, enforced by 7.7) |
| Security/Privacy | No new phone-home; only the existing Groq request | — | Review network calls added | Code review |

**Unknown thresholds (do not invent):** the `no_speech_prob` / `compression_ratio` / `avg_logprob`
cut-offs for segment drop are **UNKNOWN** — must be chosen during dev against recorded segment metadata
and locked as golden-vectors. Marked as R-009 / assumption, not guessed here.

---

## Test Coverage Plan

Test levels available in this codebase: **Rust unit** (inline `#[cfg(test)]`), **Kotlin unit** (Android),
**Integration** (JNI round-trip on device/emulator), **Golden-vector** (shared cross-platform fixtures,
seeded here / consolidated by 7.7), **Manual on-device smoke**. No browser E2E (native desktop + Android).

### P0 (Critical) — run on every commit · 100% pass required

| Requirement (AC) | Test Level | Risk | ~Count | Notes |
| --- | --- | --- | --- | --- |
| Groq request over JNI returns a transcript, no panic (AC1) | Integration (device/emulator) + Rust unit w/ wiremock for request shape | R-001 | 4 | Request-shape asserted device-free via wiremock; round-trip on device |
| H14 whole-word match, RED→GREEN before deletion (AC3) | Rust unit | R-002 | 3 | "Standard/Milliarde/Hardware" pass; "ZDF/amara.org" blocked. Test must fail on current `contains` impl first |
| Panic-safety: bad base64 / empty WAV / malformed response → fail-soft, never panic (AC10) | Rust unit + integration | R-003 | 5 | Mirror `license/jni.rs` fail-soft convention |
| `verbose_json` + legacy `json` both parse; desktop not regressed (AC6, AC10) | Rust unit (fixtures) | R-004 | 3 | Both-shape tolerance is the regression guard |

**Total P0: ~15 tests, ~30–40 h**

### P1 (High) — run on PR to main · ≥95% pass

| Requirement (AC) | Test Level | Risk | ~Count | Notes |
| --- | --- | --- | --- | --- |
| Stockphrase family + long-clip trailing-ghost stripped; real long speech w/ incidental "ZDF" passes (AC5) | Rust unit + golden-vector | R-005 | 8 | Negative cases (real speech) are the false-positive guard |
| Shared silence pre-filter parity: exactly MIN_RECORDING_MS → Pass; exactly SILENCE_THRESHOLD → Pass; malformed WAV → Pass (AC4) | Rust unit + golden-vector | R-002 | 5 | Boundary `<` not `<=`; fold in `SilencePreFilter.kt` contract |
| Prompt assembly: dictionary + lang hint + customPrompt → expected body (AC1: H3/Recall#5) | Rust unit + golden-vector | — | 4 | Single Rust `build_stt_prompt_with_hint` source |
| Retry/4xx semantics preserved around the JNI call (AC1) | Kotlin/integration | R-007 | 3 | 2 s/5 s retry; 4xx not retried |
| Confidence-drop by `no_speech_prob`/`compression_ratio`/`avg_logprob` on fixture segments (AC6) | Golden-vector fixtures | R-009 | 5 | The named fixture-only gate (AC8) |

**Total P1: ~25 tests, ~25–35 h**

### P2 (Medium) — run nightly

| Requirement (AC) | Test Level | Risk | ~Count | Notes |
| --- | --- | --- | --- | --- |
| `sttModel` / `localWhisperModel` / temperature config reads reflected in the Rust request (AC1: H9/H10/L3) | Rust unit | — | 4 | camelCase config keys |
| Cleanup-no-invent: post-cleanup deterministic strip removes manufactured stockphrase (AC7) | Rust unit | R-008 | 3 | Prefer deterministic strip over prompt-only |
| ADR-0017 boundary: no Kotlin STT request/guard survives (AC9) | Static/CI grep gate | — | 2 | Seeded here, enforced by 7.7 |
| Cross-platform build: `cargo check --target x86_64-pc-windows-gnu` + Android target compile (AC10) | CI | R-006 | 2 | Verify reqwest feature set unchanged |

**Total P2: ~11 tests, ~8–14 h**

### P3 (Low / Manual) — on-demand

| Requirement (AC) | Test Level | ~Count | Notes |
| --- | --- | --- | --- |
| **On-device Android smoke (the human gate, AC8):** short/silent clips → no stockphrase ghost in pasted text; normal dictation pastes correctly | Manual (`scripts/android-smoke.sh`) | 1 | Andi-reproducible; do NOT include confidence-drop here |
| Desktop regression smoke: existing press-to-paste STT still works after the `verbose_json` switch | Manual / existing | 1 | Guards R-004 on the desktop side |

**Total P3: ~2 manual smokes + exploratory, ~2–5 h**

---

## Execution Strategy (PR / Nightly / Weekly)

- **PR:** all P0 + P1 Rust unit + golden-vector tests (device-free subset < 15 min). The H14 RED→GREEN
  and both-shape parse tests are the PR blockers.
- **Nightly:** P2 (config-read, boundary CI, build-matrix), full golden-vector set.
- **On merge / pre-release (manual):** the two P3 on-device + desktop smokes (need the phone over
  adb/Tailscale — no cloud cron, mirrors the 7.7 quality-audit tooling cadence).

---

## Resource Estimates

| Priority | Count | Hours/Test | Total | Notes |
| --- | --- | --- | --- | --- |
| P0 | ~15 | ~2.0 | ~30–40 h | JNI/runtime, device setup, panic fuzz |
| P1 | ~25 | ~1.0 | ~25–35 h | Hardening + fixtures |
| P2 | ~11 | ~0.5 | ~8–14 h | Config reads, CI gates |
| P3 | ~2 + expl. | manual | ~2–5 h | On-device + desktop smoke |
| **Total** | **~53** | — | **~67–95 h (~9–12 days)** | High effort, as scoped |

---

## Quality Gate Criteria

- **R-001 resolved in design before dev** — the runtime/JNI shape is settled and recorded (BLOCK gate).
- **P0 pass rate = 100%** (no exceptions).
- **H14 RED-first proven:** the whole-word test fails on the current `contains` impl, then passes — at
  writing time (mirror the project's mandatory inversion-check discipline).
- **No panic paths:** every JNI input class has a fail-soft test.
- **Desktop not regressed:** both-shape parse tests + desktop smoke green.
- **P1 pass rate ≥ 95%.**
- **ADR-0017 boundary holds:** the no-Kotlin-STT/guard grep gate is green.
- **NFR evidence identified** for reliability + correctness; full PASS/CONCERNS deferred to `*nfr`.

---

## Mitigation Plans (high-priority)

### R-001 — Async request over JNI (Score 9 → RESOLVED 2026-06-12)
**Decision (Andi):** **Weg A** — a per-call throwaway `tokio` current-thread runtime inside the JNI fn
that `block_on`s the existing async `WhisperStt::transcribe`. Reuses the async request code unchanged
(no parallel `reqwest::blocking` path, no shared runtime). The existing async reqwest client stays
`default-features=false`+`rustls-tls` — Weg A needs no new reqwest feature. **ANR is already handled:**
Kotlin calls transcription from a background `Thread` (`KlarvoOverlayService.kt:897/1072/1347`), not the
UI thread, so `block_on` does not freeze the overlay. **Residual gate (downgraded from BLOCK to PROVE):**
the first P0 integration test (Groq-over-JNI round trip on device) must show Weg A runs cleanly; request
shape asserted device-free via wiremock. If that test fails → escalate, do not iterate blind.
**Status:** Decision made; proof pending in first dev step.

### R-002 — H14 regression (Score 6)
**Strategy:** Adopt whole-word matching for single-word blocklist entries in `hallucination.rs` **in the
same change that deletes `HallucinationFilter.kt`**; never delete the twin while the Rust filter still
substring-matches. **Verification:** RED→GREEN unit test + cross-platform golden-vector. **Status:** Planned.

### R-003 — JNI panic → crash (Score 6)
**Strategy:** Copy the `license/jni.rs` discipline — check every conversion, fail-soft string return, log
via `log::error!`; consider `catch_unwind` at the boundary. **Verification:** malformed-input fuzz suite.
**Status:** Planned.

### R-004 — verbose_json desktop regression (Score 6)
**Strategy:** Tolerant parser accepting both `json` and `verbose_json`; fixtures of both shapes; keep
desktop pipeline tests green. **Verification:** both-shape unit fixtures + desktop smoke. **Status:** Planned.

---

## Assumptions and Dependencies

**Assumptions**
1. Groq `verbose_json` returns `no_speech_prob` / `compression_ratio` / `avg_logprob` per segment (verify against a live response before locking thresholds).
2. The Android cdylib can perform the reqwest/rustls HTTPS call (the app already links Rust; license bridge proves JNI calls work).
3. `KlarvoApi.cleanup`/`cleanupChunked` stay (LLM path = Story 7.5); only `transcribe`+`buildMultipartBody` are deleted.

**Dependencies**
1. A device/emulator reachable over adb (for P0 integration + P3 smoke) — no cloud cron.
2. Test-Architect `*risk` consumed → this design → then `dev-story`.

**Risks to plan**
- **Risk:** R-001 runtime shape proves harder than expected (e.g. TLS feature conflict).
  - **Impact:** dev start slips; could force a spike.
  - **Contingency:** time-box a runtime spike before committing to the consolidation; if blocked, the
    fallback is a *thin* Kotlin HTTP shim that calls Rust *only* for request-building + guards (degrades
    the Hard Rule but preserves the guard consolidation) — escalate to Andi before taking it.

---

## Interworking & Regression

| Component | Impact | Regression scope |
| --- | --- | --- |
| Desktop `pipeline.rs` STT path | `verbose_json` switch + guard changes touch shared code | Existing `pipeline.rs` / `hallucination.rs` unit tests + desktop press-to-paste smoke must stay green |
| `KlarvoOverlayService.kt` | Pre-filter (`:947`), hallucination (`:1091`), transcribe (`:1358`) call sites rerouted to JNI | Android on-device dictation smoke; retry/4xx behavior |
| `stt/mod.rs` `WhisperStt` | request builder becomes the JNI body | Desktop Groq transcription unchanged |

---

## Follow-on Workflows (manual)

- `*atdd` — generate the failing P0 tests (H14 RED-first, panic-safety) before implementation.
- `*trace` — after dev, map ACs ↔ tests.
- `*nfr` — after evidence exists, assess reliability/correctness PASS/CONCERNS.
- These golden-vectors are **seeds**; Story 7.7 consolidates them into the cross-platform CI parity net.

---

## Appendix — Knowledge Base References

- `risk-governance.md`, `probability-impact.md` (1–3 × 1–3 scoring), `test-levels-framework.md`, `test-priorities-matrix.md`

### Related Documents
- Story: `_bmad-output/implementation-artifacts/7-3-shared-core-stt-request-and-guard-path-via-jni.md`
- Epic: `_bmad-output/planning-artifacts/epics-cross-platform-parity.md#Story 7.3`
- ADR: `docs/adr/0017-shared-core-stt-path.md`, `docs/adr/0016-android-path-parity-strategy.md#Amendment 2`
- Evidence: `docs/dictation-quality-android-vs-desktop-2026-06-12.md`

---

**Generated by:** BMad TEA — Test Architect Module · Workflow `bmad-testarch-test-design` (v6)
