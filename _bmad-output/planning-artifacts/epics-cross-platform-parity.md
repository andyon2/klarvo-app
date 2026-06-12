---
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics"]
status: in-progress
inputDocuments:
  - docs/cross-platform-drift-audit.md  # verified A/B drift audit — the requirements source
  - docs/dictation-quality-android-vs-desktop-2026-06-12.md  # STT evidence run — drove the 2026-06-12 re-scope
  - docs/adr/0016-android-path-parity-strategy.md  # Amendment 1 gates the per-row stories; Amendment 2 the STT consolidation
  - docs/adr/0017-shared-core-stt-path.md  # Hard Rule: shared STT/guard logic only in Rust, over JNI
  - _bmad-output/planning-artifacts/sprint-change-proposal-2026-06-10.md  # original correct-course routing
  - _bmad-output/planning-artifacts/sprint-change-proposal-2026-06-12.md  # STT re-scope routing (supersedes the STT rows)
  - _bmad-output/project-context.md
trackType: brownfield
featureEpic: 7
note: >
  Separate planning artifact by design (mirrors Epics 5/6). epics.md is the CLOSED
  robustness-remediation breakdown (Epics 1-4). This is the Cross-Platform Config-Contract
  Parity epic (Epic 7), routed via bmad-correct-course (sprint-change-proposal-2026-06-10.md)
  off the verified drift audit, gated by ADR-0016 Amendment 1. RE-SCOPED 2026-06-12
  (sprint-change-proposal-2026-06-12.md, ADR-0017): the STT-class rows (old 7.3 + 7.4 +
  silence-filter part of 7.2 + H14) collapse into one Rust-core consolidation story (new 7.3);
  old 7.4 removed; 7.6 shrinks to the M12 decision. No PRD/Architecture/UX — the requirements
  sources are docs/cross-platform-drift-audit.md (row IDs C/H/M/L + recall-sweep) and the
  2026-06-12 evidence run. Shares the sprint-status.yaml ledger. C1 (license) is already
  fixed (22553bc) and OUT of scope.
---

# klarvo - Epic Breakdown (Cross-Platform Config-Contract Parity · Epic 7)

## Overview

**Brownfield drift remediation.** ADR-0016 Amendment 1 (2026-06-10) moves the parity line
*chirurgisch*: only **core-output-determinism drift** and **settable-but-silently-dead config keys**
cross from "accepted asymmetry" into stories. Pure feature-ports stay accepted → `docs/backlog.md`.

**RE-SCOPE 2026-06-12 (ADR-0017 + sprint-change-proposal-2026-06-12.md):** the STT evidence run
proved both platforms hit the identical Groq engine through **two divergent request/guard
implementations**. The STT-class rows are therefore no longer per-row Kotlin ports: Story 7.3 is now
the **Rust-core STT consolidation over JNI** (absorbs old 7.3 + 7.4 + the silence-filter part of 7.2
+ H14, deletes the Kotlin twins) plus new hallucination hardening. The Hard Rule (ADR-0017) is
STT-only; chunking (7.1), the VAD gate (7.2) and LLM routing (7.5) stay per-row Kotlin fixes.

Each remaining per-row story implements a cluster of audit rows (IDs from
`docs/cross-platform-drift-audit.md`). Unless noted, the fix is on the **Android** (Kotlin) side;
Story 7.3 changes **both** sides (shared Rust core).

**Epic DoD:** every HIGH/CRITICAL row fixed here carries a golden-vector regression that runs against
**both** Rust and Kotlin (built incrementally per story, consolidated by Story 7.7). Test-Architect
`*risk`/`*design` on the core-output stories (7.1, 7.2, 7.3).

**Sequencing:** 7.3 first (highest-value core change; 7.1 is independent and can run in parallel).
7.2/7.5/7.6 independent. 7.7 (the structural parity net) runs **last** so it locks the just-fixed
behavior + the dead-config landmines.

---

## Story 7.1: Android chunking parity (core output)

**Rows:** H2, H13, L4, M8.

As a klarvo user dictating long German text on Android,
I want chunk splitting to behave exactly as on Desktop,
So that the same dictation produces the same cleaned output on both platforms.

**Acceptance Criteria (outcomes — full Given/When/Then in create-story):**
- **H2** — Android chunk-split indices are computed over **UTF-8 byte length** (matching `raw_text.len()`,
  `llm/mod.rs:1244-1306`), not UTF-16 `text.length` (`KApi:816-884`). Umlaut-heavy text splits at the same point.
- **H13** — chunks are joined with `\n`, not `\n\n` (`KApi:909` → match `llm/mod.rs:1334`).
- **L4** — the threshold operator is `< 400`, not `<= 400` (off-by-one at exactly 400, `KApi:884`).
- **M8** — chunk-failure handling matches Desktop's abort-on-first-error semantics (or a deliberately
  documented, golden-vector-locked divergence), not "retry whole text as one call then raw" (`KApi:902-907`).

**Golden-vectors:** German-umlaut strings straddling the 400-char boundary; input that splits into N chunks.
**Test-Architect:** `*risk` + `*design` (core output path, hits the primary use case).

---

## Story 7.2: Android live auto-stop VAD-gate parity *(narrowed 2026-06-12)*

**Rows:** H1, H17, M2, M3, M4, L1. *(Recall #1 + M1-read moved to 7.3 — the pre-STT silence
filter is consolidated there; no second source left to desync against.)*

As a klarvo user on Android,
I want live auto-stop driven by my configured thresholds, like on Desktop,
So that recording doesn't cut me off mid-sentence and Expert-mode tuning actually takes effect.

**Acceptance Criteria (outcomes):**
- **H1** — the live auto-stop energy gate derives `energy_floor` from config `silenceThreshold`
  (default 0.005), not a hardcoded `0.02f` (`KAR:58`). Defaults are no longer 4× stricter than Desktop.
- **H17** — silence→stop honors a 200ms floor (`hangover_ms.max(200)`, `audio/mod.rs:1074`); not a
  ~32ms single-frame floor (`KAR:77-78`).
- **M2** — `minRecordingMs` is config-driven (not hardcoded `500L`).
- **M3** — an 85 Hz highpass is applied before VAD (parity with `vad/mod.rs:73`).
- **M4** — RMS is computed at parity scale/precision so numeric thresholds are comparable (compounds H1).
- **L1** — VAD fps matches 31.25 (ceil), not integer 31.

> **Note:** This stays a **Kotlin** fix by design (ADR-0017 Scope): the VAD gate is a realtime frame
> stream — moving it over JNI is a large lift with speech-truncation risk, deliberately out of the
> consolidation. M1's *whisper-mode threshold swap* depends on whisper-mode (C2, **backlog**). **M5**
> (full 4-state VAD state machine) stays accepted → backlog (heavy algorithm rewrite, DIV-14).

**Golden-vectors:** energy-floor + stop-latency at default config and at one tuned config.
**Test-Architect:** `*risk` + `*design` (can truncate user speech).

---

## Story 7.3: Shared-core STT request + guard path via JNI *(re-scoped 2026-06-12 — consolidation centerpiece)*

**Rows:** H3, Recall #5, H9, H10, L3 (old 7.3) · H6, H7 (old 7.4) · Recall #1, M1-read (silence
filter, from old 7.2) · H14 (from old 7.6) · **+ new hallucination hardening** (not in the row audit;
source: `docs/dictation-quality-android-vs-desktop-2026-06-12.md`). *(Governed by ADR-0017; overrides
ADR-0016 DIV-08 + DIV-11 via consolidation instead of porting.)*

As a klarvo user,
I want both platforms to send the **same** STT request and apply the **same** hallucination/silence guards,
So that dictation quality is identical and stockphrase ghosts never reach my text — on either device.

**Acceptance Criteria (outcomes — full Given/When/Then in create-story):**
- **Single Rust STT path:** Android transcription routes through the Rust `GroqWhisper`/`WhisperStt`
  path over `stt/jni_bridge.rs`; `KlarvoApi.transcribe` + `buildMultipartBody` are **deleted**. This
  subsumes the old 7.3 rows by construction: prompt conditioning (language hint + dictionary +
  `customPrompt`, **H3**/**Recall #5**), `sttModel` read (**H9**), `localWhisperModel` read (**H10**),
  STT temperature parity (**L3**).
- **Shared guards:** `is_prompt_echo()` (**H6**) and `strip_prompt_fragments()` (**H7**) become the
  single Rust guards both platforms inherit; `HallucinationFilter.kt` is **deleted**.
- **H14 — explicit regression guard:** the Rust filter (`stt/hallucination.rs:160-164`) still
  substring-matches single-word entries, while the Kotlin twin already has the whole-word fix
  (ROB-03, `HallucinationFilter.kt:100-109`). The shared Rust filter MUST adopt whole-word matching
  **in this story** — otherwise deleting the Kotlin twin regresses Android on an already-fixed behavior.
- **Shared silence pre-filter:** one Rust pre-STT silence filter consumed by both platforms;
  `SilencePreFilter.kt` is **deleted**; the `0.02f`/`0.005f` self-desync (**Recall #1**) and the
  `silenceThreshold` config-read (**M1-read**) are resolved by construction.
- **Hallucination hardening (new):**
  - Blocklist the `Groß- und Kl(inge|ingel|einschreibung)[, Satzzeichen und Interpunktion]` stockphrase
    family, `Untertitelung des ZDF`, `amara.org`/credit/subtitle lines, `[Musik]`, subscribe/thank-you
    sign-offs — and run the **trailing-ghost** match **regardless of clip length** (kill the ≤8-word
    gate, `hallucination.rs:155-158`, that lets long-clip trailing ghosts through, ~1.1% of long clips).
  - Switch Groq to `response_format=verbose_json` and **drop segments** by `no_speech_prob` /
    `compression_ratio` / `avg_logprob` thresholds.
  - **Cleanup-no-invent:** LLM cleanup must not manufacture the full stockphrase from a recognizable
    ghost (`Klinge` → `Kleinschreibung`, observed desktop ids 2708/2891/2777); enforce by stripping the
    stockphrase family *after* cleanup and/or constraining the cleanup prompt.
- **Verifiability split (named decision, per Verifikations-Symmetrie):** stockphrase blocklist + paste
  path → **live on-device smoke** (Andi-reproducible: short/silent clips reliably trigger the ghosts).
  Confidence-drop (`verbose_json`) → **golden-vector fixtures** (segment metadata → expected drop);
  the human gate for this sub-part is deliberately downgraded to fixture-verified.

**Golden-vectors:** ZDF/`Kleinschreibung` ghosts on short clips → stripped; trailing ghost on a long
clip → stripped; low-confidence segment → dropped; "Standard"/"Milliarde" in a short utterance → NOT
discarded (H14, both platforms); prompt-body assembly given dictionary + language hint + customPrompt.
**Test-Architect:** `*risk` + `*design` (core output path, primary use case, crosses the JNI boundary).

---

## Story 7.5: Android LLM-routing contract hygiene

**Rows:** M9, M10, M11, M13, M16, L5.

As a klarvo user on Android,
I want provider routing, config-key handling and paste timing to follow the same contract as Desktop,
So that no setting silently no-ops and no future provider change silently breaks Android.

**Acceptance Criteria (outcomes):**
- **M9** — DeepSeek endpoint uses `…/v1/chat/completions` (parity with `llm/mod.rs:719`).
- **M10** — blank-key check uses whitespace-trimming semantics matching Desktop's `is_empty()` decision.
- **M11** — an unknown `cleanupStyle` is rejected (error), not silently mapped to Polished (`KApi:570/717`).
- **M13** — `bubbleTapAutoSend` / `bubbleLongPressAutoSend` are honored at runtime, not read-then-hardcoded
  `false` (`KOS:353-355`). *(Surface-operable trap: the setting round-trips but has zero effect today.)*
- **M16** — a pre-paste settle delay is applied (clipboard-ready) — parity with Desktop's 50ms.
- **L5** — `deviceId` default parity (auto-generated UUID v4 when the key is absent).

**Golden-vectors:** config round-trip asserting each key changes runtime behavior; unknown-`cleanupStyle` rejection.

---

## Story 7.6: M12 open product decision — dictionary in Chat style *(shrunk 2026-06-12)*

**Rows:** M12 (open decision). *(H14 moved to 7.3 — the shared Rust filter adopts the whole-word
match there, so both platforms get it from one fix.)*

As a klarvo user,
I want dictionary handling in Chat style to behave the same on both platforms,
So that my term-biasing is predictable regardless of device.

**Acceptance Criteria (outcomes):**
- **M12** — **resolve the open product decision first** (dictionary-in-Chat-style: should Chat include the
  dictionary like Android, or omit it like Desktop?), then make both platforms agree. Tracked as
  `OPEN-DECISION` in `docs/backlog.md`. If unresolved at story time, lock current behavior as a golden-vector
  and defer the code change.

**Golden-vectors:** Chat-style request assembly with a dictionary present → the decided canonical behavior.

---

## Story 7.7: Golden-Vector parity net (C1-proper) + dead-config lock

**Rows:** the structural net; locks every HIGH/CRITICAL Epic-7 fix + the dead-config-both-sides cluster.

As the klarvo maintainer,
I want shared golden-vector fixtures (config + input → expected behavior) run against BOTH Rust and Kotlin in CI,
So that any future re-divergence trips the net instead of shipping silently.

**Acceptance Criteria (outcomes):**
- A shared fixture format (config + input → expected behavior/output) consumed by both a Rust test target and
  a Kotlin test target.
- Every HIGH/CRITICAL row fixed in 7.1–7.6 has a corresponding golden-vector. For the consolidated STT path
  (7.3) the net pins the **shared contract** (ADR-0017 boundary): a vector asserts Android's transcription
  enters via the JNI bridge (no Kotlin STT request path exists to drift).
- **Quality-layer sibling (tooling, → backlog item):** commit the dictation-quality marker detectors from the
  2026-06-12 evidence run as `scripts/dictation-quality-audit.py`; run manually on a cadence (needs the phone
  over adb/Tailscale — no cloud cron). The parity net pins config-contract equality; this pins output-quality
  drift.
- The **dead-config-both-sides cluster** (`advanced.llmTemperature/llmMaxTokens`, `chunkThreshold/TargetSize`,
  `sttTemperature`, `llmModel*`/`llmSystemPrompt*` overrides, `autoCapitalize`/`autoPaste`) is locked: a vector
  asserts the current both-hardcode value, so the moment one side wires the key the net goes RED.
- CI runs the net on both platforms.
- **Inversion-check (mandatory):** deliberately re-introduce one fixed drift (e.g. revert H13 to `\n\n`) and
  prove the net goes RED — at writing time, not just review.

**Sequencing:** run after 7.1–7.6 so it locks the corrected behavior. This is the durable structural
drift-detection net (the [[cross-platform-parity-net]] deliverable, C1-proper) — it supersedes
"run the smartest model once" (the A/B audit's own finding: union + verifier + recall sweep still missed 5
real divergences).

---

## Out of scope (→ `docs/backlog.md`)

Pure feature-ports / accepted asymmetries (ADR-0016 Amendment 1): C2, H4, H5, H8, H11, H12+M7, H15, H16, M5,
M6, M14, M15, L2, L6, L7, Recall #4, and the dead-config cluster's *wire-when-needed* implementations.
C1 (license) is already fixed (`22553bc`) and out of scope.
**Deliberate deferrals of the 2026-06-12 re-scope:** extending the ADR-0017 consolidation to the VAD gate
(realtime JNI lift) and to the chunking/LLM path — the Hard Rule is STT-only for now; 7.7 pins the rest
against silent re-drift.
