# Sprint Change Proposal — Epic 7 Re-Scope: STT Path Consolidation

**Date:** 2026-06-12
**Author:** Correct-Course (Andi)
**Mode:** Batch
**Trigger source:** `docs/dictation-quality-android-vs-desktop-2026-06-12.md` (STT-hallucination evidence run, with 2026-06-12 correction) + Memory `reference_android_stt_is_groq_cloud`
**Scope classification:** **Major** — re-architects the STT half of an already-planned epic (Epic 7), supersedes part of ADR-0016 Amendment 1, and introduces a new Hard-Rule ADR (0017).
**Relationship to prior proposal:** **Partially supersedes** `sprint-change-proposal-2026-06-10.md`. That proposal created Epic 7 as a *per-row Kotlin parity remediation*. This proposal keeps the non-STT rows as-is but **replaces the STT-class rows with a single Rust-core consolidation**. The 2026-06-10 classification table (which audit row → which story) remains the authoritative requirements ledger; only the *delivery shape* of the STT rows changes.

---

## Section 1 — Issue Summary

**What triggered this:** Andi's lived experience that phone dictation "feels better" than laptop prompted an evidence run comparing hallucination rates. The run produced one **false premise** and one **strengthened real finding**:

- **Retracted:** The phone does **not** use local Whisper. Three independent sources (phone `config.json` → `sttProvider:"groq"`; code dispatch; runtime log → 46/46 runs `provider=groq`, 0 local) confirm **both platforms run the identical Groq engine and model (`whisper-large-v3-turbo`)**. "Converge the engine" is moot — there is no engine to converge. The Android "0/48 hallucinations" is statistically indistinguishable from the desktop base rate (P(0 hits | 1.5%, n=48) ≈ 0.48), i.e. small-sample noise, not better quality.

- **Strengthened:** The real structural defect is the **two-strands problem**. There are **two separate STT request implementations hitting the same Groq endpoint** — Rust `GroqWhisper` (`src-tauri/src/stt/mod.rs`) and Kotlin `KlarvoApi.transcribe` (`android/kotlin-src/com/klarvo/voice/KlarvoApi.kt`) — and the audit shows they **already send different parameters** (silence handling, `response_format`, prompt conditioning, model selection). The guard logic is likewise duplicated (`HallucinationFilter.kt`, `SilencePreFilter.kt` vs the Rust `pipeline.rs` guards), and the duplication has real bugs:
  - Real, observed desktop hallucinations: `Groß- und Klinge…` / `…Kleinschreibung, Satzzeichen und Interpunktion`, `Untertitelung des ZDF, 2020`, repetition loops — concentrated on **short/silent clips (8.6 %)** and as **trailing ghosts on long clips (~1.1 %)**.
  - The existing blocklist filter has an **≤8-word gate**, so it **never runs on long clips** → trailing ghosts pass through.
  - **Cleanup amplification:** LLM cleanup rationalizes the recognizable junk `Klinge` into the *convincing* full stockphrase `Kleinschreibung`, turning detectable noise into fluent, undetectable noise.

**The reframe:** The original Epic-7 plan (2026-06-10) treats this as *config drift to patch row-by-row on the Kotlin side*. The evidence says the durable fix is **structural**: one shared Rust STT request + guard path, consumed by Android over JNI, so both platforms hit Groq **identically and inherit the same guards by construction** — exactly the pattern already proven by the Android license consolidation (`22553bc`, `src-tauri/src/license/jni.rs`). The architectural question is therefore **not** "which engine" but **"one shared STT request + guard path in the Rust core vs. two divergent ones."**

---

## Section 2 — Impact Analysis

### Epic impact
**Epic 7 (Cross-Platform Config-Contract Parity) is re-scoped, not replaced.** Epics 1–6 are unaffected. The non-STT drift rows keep their per-row Kotlin shape; the STT-class rows collapse into one consolidation story. Decision (confirmed with Andi 2026-06-12):

- **STT request + STT-output guards + pre-STT silence filter** → consolidate into the Rust core, consumed via JNI; delete the Kotlin twins. **Hard rule** (ADR-0017): shared STT/guard logic lives only in Rust; a Kotlin re-implementation is forbidden.
- **Live auto-stop VAD gate** → stays a narrow **Kotlin** parity fix (realtime audio over JNI is a large, risky lift for marginal benefit; the felt problem is text hallucination, not auto-stop).
- **Chunking (7.1) and LLM-routing (7.5)** → stay per-row **Kotlin** fixes; the Hard Rule applies to the STT path only for now. LLM-path consolidation is a deliberate later/v2 move.

### Story impact (Epic 7 before → after)

| Old story (2026-06-10) | New disposition |
|---|---|
| 7.1 — Android chunking parity (core output) | **UNCHANGED** (Kotlin per-row) |
| 7.2 — Android live auto-stop gate parity | **NARROWED** → VAD-gate threshold reads only; the pre-STT silence filter (SilencePreFilter) moves into 7.3 |
| 7.3 — Android STT conditioning + model selection | **RE-SCOPED → consolidation centerpiece** (see new 7.3) |
| 7.4 — Android output guards (prompt-echo + fragment-strip) | **FOLDED into 7.3** (the guards become shared Rust guards) → 7.4 key removed |
| 7.5 — Android LLM-routing contract hygiene | **UNCHANGED** (Kotlin per-row) |
| 7.6 — Desktop back-port whole-word hallucination (H14) + M12 | **SHRUNK** → H14 is auto-resolved by 7.3 (shared filter ⇒ both platforms); 7.6 reduces to the M12 open product decision |
| 7.7 — Golden-vector parity net + dead-config lock | **UNCHANGED in intent**, smaller surface (fewer Kotlin twins) + now also pins the shared-STT contract & hallucination guards |

### Artifact conflicts
- **ADR-0016** (Android-path-parity-strategy, Accepted; Amendment 1 = 2026-06-10): needs **Amendment 2** — for the STT path the strategy is now *consolidation* (ADR-0017), not chirurgischer Kotlin-port. Amendment 1's per-row line still governs the non-STT rows. *(Never overwrite the Decision block — append.)*
- **ADR-0017** (new): the Hard-Rule ADR. Draft in Section 4c.
- **`_bmad-output/planning-artifacts/epics-cross-platform-parity.md`**: rewrite the Story 7.2/7.3/7.4/7.6 sections; keep 7.1/7.5/7.7.
- **`sprint-status.yaml`**: remove `7-4…` key, rename/re-scope `7-3…`, narrow `7-2…` and `7-6…` labels. All stay `backlog`.
- **`docs/backlog.md`**: add the recurring cross-platform quality-audit tool (`scripts/dictation-quality-audit.py`) as a tooling item attached to 7.7 (manual cadence, needs phone over adb — no cloud cron).

### Technical impact
- **Core-path code moves**, not just changes: the Groq STT request, the hallucination/silence guards, and the silence pre-filter become single-sourced in Rust behind the existing `src-tauri/src/stt/jni_bridge.rs` surface. Android's `KlarvoApi.transcribe` / `HallucinationFilter.kt` / `SilencePreFilter.kt` are deleted in favor of JNI calls.
- **New behavior** (not in the 2026-06-10 audit, added by this evidence run): stockphrase-family blocklist that also catches **trailing ghosts on long clips**; Groq `verbose_json` + confidence-based segment drop (`no_speech_prob` / `compression_ratio` / `avg_logprob`); cleanup-no-invent guard so cleanup cannot manufacture `Kleinschreibung` from `Klinge`.
- **Verifiability (per Andi's Verifikations-Symmetrie rule):** the stockphrase blocklist is **Andi-reproducible** — short/silent clips reliably trigger the ghosts, so the on-device smoke is real. The `verbose_json` confidence drop is **not** cleanly Andi-reproducible (needs the specific audio that produces a low-confidence segment), so its gate is **golden-vector fixtures** (recorded/synthetic segment metadata → expected drop), with the human smoke explicitly downgraded to "blocklist + paste-path verified live; confidence-drop verified by fixture." This is a named decision, not an accidental gap.

---

## Section 3 — Recommended Approach

**Hybrid — Direct Adjustment of Epic 7 + one structural re-architecture story.** Keep the per-row Kotlin fixes that are cheap and off the felt-quality path (7.1, 7.5, narrowed 7.2); replace the STT-class rows with one consolidation story (7.3) governed by a Hard-Rule ADR (0017); keep the parity net (7.7) as the durable guard.

**Why this and not the alternatives:**
- **Why not keep the row-by-row Kotlin port (the 2026-06-10 shape)?** It fixes the *symptoms* of the two-strands problem (porting each diverging parameter) while leaving the *cause* (two strands) in place. The audit's own finding is that even a smart multi-model sweep missed 5 real divergences — patching known rows does not stop the next unknown one. Consolidation removes the divergence class by construction.
- **Why not also consolidate the VAD gate + LLM/chunking path now?** Effort/risk vs. payoff. The STT request is a one-shot call (record → WAV → transcribe) — safe over JNI. The VAD gate is a realtime frame stream (~31 Hz) — moving it over JNI is a large lift that can truncate user speech, for a problem Andi doesn't feel. Chunking/LLM-routing are real but small and off the felt-quality path; the parity net pins them against silent re-drift. Both are deliberate "smallest sharp cut" deferrals, not oversights.
- **Risk of the chosen path:** core STT code moves across the JNI boundary → regression risk on *both* platforms. Mitigated by: the JNI bridge already exists (`stt/jni_bridge.rs`) and the pattern is proven (license `22553bc`); golden-vectors (7.7) lock the corrected behavior; Test-Architect `*risk`/`*design` on 7.3.

**Effort:** 7.3 = **High** (core re-architecture + new hardening). 7.1/7.2/7.5/7.6/7.7 = Low–Medium each.
**Timeline:** MVP (`v1-ship`) is **not** redefined; this is hardening/integrity work within the existing v1 plan. The felt-quality win (clean STT + no stockphrase ghosts) ships with 7.3.

---

## Section 4 — Detailed Change Proposals

### 4a — Re-scoped Epic 7 story list

> Requirements ledger (audit row → story) from `sprint-change-proposal-2026-06-10.md` §4a still holds; the rows below note where each moved.

**7.1 — Android chunking parity (core output)** — *UNCHANGED.*
Rows H2, H13, L4, M8. Kotlin per-row fix; golden-vectors on the 400-char umlaut boundary. Test-Architect `*risk`/`*design`.

**7.2 — Android live auto-stop VAD-gate parity** — *NARROWED.*
Rows H1, H17, M2, M3, M4, L1 (VAD-gate side only). Read `silenceThreshold` (H1), 200 ms stop floor (H17), config-driven `minRecordingMs` (M2), 85 Hz highpass (M3), RMS precision (M4), fps 31.25 (L1). **Moved out:** the pre-STT `SilencePreFilter` / M1-read + Recall #1 self-desync are now resolved by 7.3 (shared silence filter ⇒ no second source to desync against). Golden-vectors: energy-floor + stop-latency at default and one tuned config.

**7.3 — Shared-core STT request + guard path via JNI (consolidation)** — *NEW CENTERPIECE (absorbs old 7.3 + 7.4 + silence-filter part of 7.2 + H14-guard).*
As a klarvo user, I want both platforms to send the **same** STT request and apply the **same** hallucination/silence guards, so that dictation quality is identical and stockphrase ghosts never reach my text — on either device.
Acceptance outcomes (full Given/When/Then in create-story):
- **Single Rust STT path:** Android transcription routes through the Rust `GroqWhisper`/`WhisperStt` path over `stt/jni_bridge.rs`; `KlarvoApi.transcribe` + `buildMultipartBody` are deleted. Subsumes old 7.3: prompt conditioning (lang hint + dictionary + `customPrompt`, H3/Recall#5), `sttModel` read (H9), `localWhisperModel` read (H10), STT temperature parity (L3).
- **Shared guards:** the prompt-echo guard (`is_prompt_echo`, H6) and fragment-strip (`strip_prompt_fragments`, H7) become the single Rust guards both platforms inherit; `HallucinationFilter.kt` is deleted. **H14 — explicit regression guard:** the Rust filter (`stt/hallucination.rs`) still does substring matching (`lower.contains`), while the Kotlin twin already has the whole-word fix (ROB-03). The shared Rust filter MUST adopt whole-word matching for single-word entries **in this story** — otherwise deleting the Kotlin twin regresses Android on H14. Golden-vector: "Standard"/"Milliarde" in a short utterance → not discarded, on both platforms.
- **Shared silence pre-filter:** one Rust pre-STT silence filter consumed by both; `SilencePreFilter.kt` deleted; the `0.02f`/`0.005f` self-desync (Recall #1) is gone by construction.
- **Hallucination hardening (NEW, from the 2026-06-12 evidence run):**
  - Blocklist the `Groß- und Kl(inge|ingel|einschreibung)[, Satzzeichen und Interpunktion]` stockphrase family, `Untertitelung des ZDF`, `amara.org`/credit/subtitle lines, `[Musik]`, subscribe/thank-you sign-offs — and run the **trailing-ghost** match **regardless of clip length** (kill the ≤8-word gate that lets long-clip trailing ghosts through).
  - Switch Groq to `response_format=verbose_json` and **drop segments** by `no_speech_prob` / `compression_ratio` / `avg_logprob` thresholds.
  - **Cleanup-no-invent:** LLM cleanup must not manufacture the full stockphrase from a recognizable ghost (`Klinge` → `Kleinschreibung`); enforce by stripping the stockphrase family *after* cleanup and/or constraining the cleanup prompt.
- **Verifiability split (named):** stockphrase blocklist + paste path → **live on-device smoke** (Andi-reproducible via short/silent clips). Confidence-drop (`verbose_json`) → **golden-vector fixtures** (segment metadata → expected drop); human gate for this sub-part deliberately downgraded to fixture-verified.
Golden-vectors: ZDF/`Kleinschreibung` ghosts on short clips → stripped; trailing ghost on a long clip → stripped; low-confidence segment → dropped; prompt-body assembly given dictionary + lang hint + customPrompt. **Test-Architect `*risk` + `*design`** (core output path, the primary use case, crosses the JNI boundary).

**~~7.4~~ — REMOVED** (folded into 7.3).

**7.5 — Android LLM-routing contract hygiene** — *UNCHANGED.*
Rows M9, M10, M11, M13, M16, L5. Kotlin per-row fix (DeepSeek `/v1`, `isBlank` parity, reject unknown `cleanupStyle`, honor bubble auto-send, paste settle delay, `deviceId` default).

**7.6 — Desktop M12 open product decision** — *SHRUNK.*
H14 is now resolved by 7.3. This reduces to resolving M12 (dictionary-in-Chat-style: should Chat include the dictionary like Android, or omit it like Desktop?) as an `OPEN-DECISION` in `docs/backlog.md`, then making both platforms agree. If unresolved at story time, lock current behavior as a golden-vector and defer.

**7.7 — Golden-vector parity net + dead-config lock** — *UNCHANGED intent, updated surface.*
Shared fixtures (config + input → expected behavior) run against **both** Rust and Kotlin in CI. Now also pins: the shared-STT contract (3.3), the hallucination guards (3.3), and the dead-config-both-sides cluster. Mandatory inversion-check (re-introduce one fixed drift, prove the net goes RED, at writing time). **Attached tooling item → backlog:** commit `scripts/dictation-quality-audit.py` (the `raw_text` marker detectors from the evidence run) as the quality-layer sibling of the parity net; run manually on a cadence (needs phone over adb/Tailscale — no cloud cron).

**Sequencing:** 7.3 is the highest-value core change → first (or in parallel with the independent 7.1). 7.2/7.5/7.6 independent. 7.7 last (locks corrected behavior).

### 4b — Out of scope (unchanged from 2026-06-10 → `docs/backlog.md`)
All pure feature-ports / accepted asymmetries (C2, H4, H5, H8, H11, H12+M7, H15, H16, M5, M6, M14, M15, L2, L6, L7, Recall #4, dead-config wire-when-needed). C1 (license) already fixed (`22553bc`). M5 (full VAD state machine) stays accepted. **LLM/chunking path consolidation** is explicitly deferred (Hard Rule is STT-only for now).

### 4c — ADR-0017 (new — ready to write)

```markdown
# ADR-0017: Shared-Core STT Path — Single Rust STT Request + Guards over JNI

**Status:** Accepted
**Date:** 2026-06-12
**Relates to:** ADR-0016 (Android path parity strategy) — see its Amendment 2.

## Context
Both platforms use the identical Groq engine (`whisper-large-v3-turbo`); the phone does
NOT run local Whisper (verified: config `sttProvider=groq`, dispatch, 46/46 runtime logs).
Yet two separate STT request implementations hit the same endpoint — Rust `GroqWhisper`
and Kotlin `KlarvoApi.transcribe` — and demonstrably send different parameters (silence
handling, response_format, prompt conditioning, model). The guard logic is likewise
duplicated (HallucinationFilter.kt / SilencePreFilter.kt vs the Rust pipeline guards),
with real bugs (≤8-word gate misses long-clip trailing ghosts; cleanup rationalizes
`Klinge`→`Kleinschreibung`). Per-row porting fixes symptoms, not the divergence class.
The Android license consolidation (22553bc) already proved the JNI-shared-Rust pattern.

## Decision
The STT request, the STT-output guards (hallucination filter, prompt-echo, fragment-strip),
and the pre-STT silence filter are **single-sourced in the Rust core** and consumed by
Android **via JNI** (`src-tauri/src/stt/jni_bridge.rs`). The Kotlin twins
(`KlarvoApi.transcribe`/`buildMultipartBody`, `HallucinationFilter.kt`, `SilencePreFilter.kt`)
are deleted.

**Hard rule:** shared STT/guard logic MUST live only in the Rust core. A parallel Kotlin
re-implementation of any STT-request or STT-guard behavior is **forbidden**; Android calls
the Rust path. New STT/guard behavior is added in Rust and exposed over JNI, never re-coded
in Kotlin.

## Scope
STT path only. The live auto-stop VAD gate (realtime audio), text chunking, and LLM-provider
routing remain platform-local per-row parity (ADR-0016 Amendment 1) until a future decision
extends this rule. The golden-vector parity net (Epic 7.7) enforces the boundary.

## Consequences
+ Divergence in the STT path becomes impossible by construction; both platforms inherit
  every guard and parameter change for free.
+ The ~2000-LOC Android duplicate SHRINKS for the STT path (vs. ADR-0016's "grows minimally").
− Core STT code now crosses the JNI boundary → regression risk on both platforms; mitigated
  by golden-vectors (7.7) and Test-Architect *risk/*design on 7.3.
```

### 4d — ADR-0016 Amendment 2 (append — does NOT overwrite Decision or Amendment 1)

```markdown
## Amendment 2 (2026-06-12) — STT-Pfad: Konsolidierung statt chirurgischem Kotlin-Port

**Auslöser:** `docs/dictation-quality-android-vs-desktop-2026-06-12.md`. Die Prämisse
"Phone = local Whisper" ist FALSCH — beide Plattformen fahren denselben Groq-Engine. Der
reale, gestärkte Befund ist die Zwei-Strang-Drift: zwei getrennte STT-Request-Implementierungen
(Rust `GroqWhisper` + Kotlin `KlarvoApi.transcribe`) gegen denselben Endpoint mit
unterschiedlichen Parametern.

**Linienverschiebung (nur STT-Pfad):** Für den STT-Request + dessen Guards + den Pre-STT-
Silence-Filter gilt ab jetzt **Konsolidierung in den Rust-Kern via JNI** (ADR-0017), NICHT
mehr der per-row Kotlin-Port aus Amendment 1. Die betroffenen Epic-7-Rows (alt 7.3 STT-
Conditioning, 7.4 Output-Guards, Silence-Teil von 7.2, H14) verschmelzen in eine
Konsolidierungs-Story (neu 7.3); die Kotlin-Twins werden gelöscht.

**Unverändert (Amendment 1 gilt weiter):** Live-Autostop-VAD-Gate, Chunking (7.1) und
LLM-Routing (7.5) bleiben platform-lokale per-row-Parität. Die Hard-Rule (ADR-0017) ist
bewusst STT-only.
```

---

## Section 5 — Implementation Handoff

**Scope: Major.** Route to sprint-status update → story cycle.

1. **Apply concrete edits** (on approval, this session): write `docs/adr/0017-shared-core-stt-path.md`; append Amendment 2 to `docs/adr/0016-android-path-parity-strategy.md`; rewrite the 7.2/7.3/7.6 sections and remove 7.4 in `_bmad-output/planning-artifacts/epics-cross-platform-parity.md`; update `sprint-status.yaml` (remove `7-4`, re-scope `7-3`, narrow `7-2`/`7-6`); add the quality-audit tool + M12 OPEN-DECISION to `docs/backlog.md`.
2. **Story cycle** — `bmad-create-story 7.3` first (highest-value, Test-Architect `*risk`/`*design`, crosses JNI), then 7.1/7.2/7.5/7.6; 7.7 last so it locks corrected behavior.
3. **Commits/pushes:** human-controlled (project convention). No auto-commit.
4. **Retire the routing directive:** once Epic 7 is re-cut here, withdraw the `bmad-sprint-status.toml` persistent_fact that forced this correct-course (its Re-Eval trigger: "sobald die Correct-Course Epic 7 neu schneidet").

**Success criteria:**
- STT request + guards + silence filter single-sourced in Rust; Kotlin twins deleted; Android transcribes via JNI (no `provider`/parameter divergence possible).
- Stockphrase family (incl. long-clip trailing ghosts) blocked; `verbose_json` confidence-drop active; cleanup cannot invent `Kleinschreibung`.
- Every HIGH/CRITICAL fix carries a golden-vector running against both Rust and Kotlin; 7.7 inversion-check trips RED on a re-introduced drift.
- On-device Android smoke: short/silent clips produce no stockphrase ghost in the pasted text (Andi-reproducible). Confidence-drop sub-part fixture-verified (named downgrade).
