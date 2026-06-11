# Sprint Change Proposal — Cross-Platform Drift Remediation

**Date:** 2026-06-10
**Author:** Correct-Course (Andi)
**Trigger source:** `docs/cross-platform-drift-audit.md` (A/B drift audit, Fable 5 vs Opus 4.8, verified claim-by-claim)
**Mode:** Batch
**Scope classification:** **Major** — introduces a new epic (Epic 7) + amends an Accepted ADR (0016) + creates the backlog SSOT.

---

## Section 1 — Issue Summary

The frozen `v1-ship` tree was audited for Rust(Desktop) ↔ Kotlin(Android) drift under the ADR-0016 definition: *same config + same input → different behavior across platforms, with no error.* The verified audit (`docs/cross-platform-drift-audit.md`) found **1 CRITICAL (open), 17 HIGH, 16 MEDIUM, 7 LOW**, plus a dead-config-both-sides cluster and 5 recall-sweep findings neither model reported.

Two findings are **new** — they were never adjudicated by the original robustness audit (`robustness-audit-2026-05-30.md` §3, DIV-01..14) on which the ADR-0016 line was drawn:

- **H2** — chunking length unit: Desktop counts UTF-8 **bytes**, Android counts UTF-16 **code units**. German umlauts cross the 400-char split threshold at different points → different chunk splits → **different LLM cleanup output on the core dictation use case.**
- **C1** — license/trial enforcement unenforced on Android. **Already fixed** (`22553bc`, on-device smoke 4/4 GREEN, `e772fde`). Out of scope here.

This re-frames the problem: ADR-0016 weighed the divergences as *feature-parity ROI* ("don't deepen the ~2000-LOC duplicate for marginal benefit; v2 is the dedup answer"). The new audit surfaces a class the line never saw — **core-output-determinism drift** and **settable-but-silently-dead config keys** (surface-operable traps) — which is a config-contract integrity problem, not a feature-parity problem.

## Section 2 — Impact Analysis

- **Epic impact:** New **Epic 7 — Cross-Platform Config-Contract Parity** (Android drift remediation, chirurgisch). Epics 1–6 unaffected. C1 already closed outside this proposal.
- **Story impact:** 7 new stories (7.1–7.7). Story 7.6 touches **Desktop** (back-port), all others touch Android Kotlin.
- **Artifact conflicts:**
  - **ADR-0016 (Accepted)** — requires an Amendment (never overwrite the Decision block; see ADR-amendment convention). Six rows it accepted as won't-fix are re-adjudicated: 3 cross the line into Epic 7 (H3/H6/H7), 3 stay accepted → backlog (H4/H11/H12).
  - **epics.md** — append Epic 7 section.
  - **docs/backlog.md** — does not exist yet; create it as the deferred-work SSOT (per backlog-discipline memory).
  - **sprint-status.yaml** — add `epic-7` + stories as backlog (via sprint-planning handoff).
- **Technical impact:** Core-path output changes (chunking, auto-stop gate) on Android → require golden-vector regression coverage. Story 7.7 builds the structural parity net (the durable [[cross-platform-parity-net]] deliverable, C1-proper).

## Section 3 — Recommended Approach

**Direct Adjustment** — add Epic 7 within the existing plan, run the normal story cycle (create-story → dev → review), Test-Architect `*risk`/`*design` on the core-output stories (7.1, 7.2). The chirurgisch ADR-0016 amendment keeps the "no duplicate-for-marginal-features" rationale intact while pulling the core-output + output-guard class across the line.

**Decisions locked (this session):**
1. Granularity → **new Epic 7, full story cycle** (not quick-dev pickoffs).
2. Settable-but-dead heavy capabilities (H5/H8/H16/C2/H15/M14/L7) → **all Backlog** (feature ports, ADR-0016 rationale holds).
3. Reverse-direction cases → **H14 back-port to Rust** (Epic 7, Desktop); **M12 flagged as open product decision**.
4. ADR-0016 line → **chirurgisch**: H6/H7 (output guards) + H3 (STT conditioning) cross into Epic 7; H4/H11/H12 stay accepted → Backlog.

## Section 4 — Detailed Change Proposals

### 4a — Full classification (every audit row routed, nothing dropped)

**→ Epic 7 (fix now):**

| Row | What | Story |
|---|---|---|
| H2 | UTF-16 → byte chunking indices | 7.1 |
| H13 | chunk join `\n\n` → `\n` | 7.1 |
| L4 | chunk operator `<=` → `<` (off-by-one @400) | 7.1 |
| M8 | chunk-failure handling | 7.1 |
| H1 | auto-stop gate read `silenceThreshold` (4× too strict) | 7.2 |
| H17 | silence→stop 200ms floor | 7.2 |
| Recall #1 | Android's own `0.02f` vs `0.005f` desync | 7.2 |
| M1 (read-part) | pre-STT `silenceThreshold` config-driven | 7.2 |
| M2 | `minRecordingMs` config-driven | 7.2 |
| M3 | 85 Hz highpass before VAD | 7.2 |
| M4 | RMS scale/precision parity | 7.2 |
| L1 | VAD fps 31 → 31.25 | 7.2 |
| H3 | STT prompt conditioning (lang hint + dict + customPrompt) — *overrides DIV-08* | 7.3 |
| Recall #5 | `customPrompt`/`sttPrompt*` conflation | 7.3 |
| H9 | read `sttModel` | 7.3 |
| H10 | read `localWhisperModel` | 7.3 |
| L3 | STT temperature wire parity | 7.3 |
| H6 | prompt-echo guard — *overrides DIV-11* | 7.4 |
| H7 | prompt-fragment stripping — *overrides DIV-11* | 7.4 |
| M9 | DeepSeek `/v1` URL | 7.5 |
| M10 | `isEmpty` vs `isBlank` blank-key parity | 7.5 |
| M11 | reject unknown `cleanupStyle` (no silent Polished) | 7.5 |
| M16 | `pasteDelayMs` settle delay | 7.5 |
| M13 | bubble auto-send honor key (stop hardcoding false) | 7.5 |
| L5 | `deviceId` default parity | 7.5 |
| H14 | **DESKTOP** back-port whole-word hallucination match | 7.6 |
| — | Golden-Vector parity net + dead-config lock | 7.7 |

**→ Backlog (Sorte-2, deferred):**

| Row | What | Why deferred |
|---|---|---|
| C2 | whisper-mode amplify (3 keys + audio gain) | feature port |
| H4 | `outputLanguage` translation | DIV-07 accepted; feature |
| H5 + Recall #2 | Anthropic provider (struct field + branch) | feature port |
| H8 | `audioDevice` selection | feature port |
| H11 | local cleanup prompt completeness | DIV-09 accepted |
| H12 + M7 | provider fallback on 429/5xx + order | DIV-06 accepted; resilience |
| H15 | per-app profiles | feature port |
| H16 | `sttProvider=openai` routing | DIV-13 accepted; feature |
| M5 | full VAD state-machine parity | DIV-14 accepted; heavy algo rewrite |
| M6 | pre-STT WAV float support | latent (both PCM16 today) |
| M14 + Recall #3 | webhook delivery + headers | feature port |
| M15 | Desktop adopt STT retry/backoff | Desktop enhancement (Android is better side) |
| L2 | waveform amplitude transform | cosmetic |
| L6 | empty-transcript toast text | cosmetic (same outcome) |
| L7 | voice command | DIV-10 accepted; feature |
| Recall #4 | live-preview delta Android port | feature (Epic 5 desktop-only) |
| Dead-config cluster | llmTemperature/MaxTokens/chunk*/model-overrides/system-prompt-overrides/autoCapitalize/autoPaste | latent landmines → golden-vector lock (7.7), wire-when-needed |

**→ Open product decision (not routed yet):**

- **M12** — dictionary-in-Chat-style: Android adds dictionary for ALL styles incl. chat; Desktop omits it in Chat. Opposite direction. Canonical direction is a product call — resolve before either side is touched. Tracked in backlog as `OPEN-DECISION`.

### 4b — Epic 7 story breakdown (titles + outcomes)

- **7.1 — Android chunking parity (core output).** Byte-indexed splitting (H2), `\n` join (H13), `<400` operator (L4), chunk-failure handling (M8). Golden-vectors: German-umlaut strings straddling the 400 boundary. *Test-Architect `*risk`/`*design`.*
- **7.2 — Android live auto-stop gate parity.** Read `silenceThreshold` (H1), 200ms floor (H17), resolve the `0.02f`/`0.005f` self-desync (Recall #1), config-driven `minRecordingMs` (M2), 85 Hz highpass (M3), RMS precision (M4), fps 31.25 (L1). Golden-vectors: energy-floor at default + tuned config. *Test-Architect `*risk`/`*design`.*
- **7.3 — Android STT conditioning + model selection.** Send `prompt` multipart field (language hint + dictionary + customPrompt) (H3), split `customPrompt` vs `sttPrompt*` (Recall #5), read `sttModel` (H9) + `localWhisperModel` (H10), temperature wire (L3). *Overrides ADR-0016 DIV-08.*
- **7.4 — Android output guards.** Port `is_prompt_echo()` (H6) + `strip_prompt_fragments()` (H7). *Overrides ADR-0016 DIV-11. Near the guardian class.*
- **7.5 — Android LLM-routing contract hygiene.** DeepSeek `/v1` (M9), `isBlank` parity (M10), reject unknown `cleanupStyle` (M11), paste settle delay (M16), honor `bubbleTap/LongPressAutoSend` (M13 — surface-operable trap), `deviceId` default (L5).
- **7.6 — Desktop: back-port whole-word hallucination match (H14).** Adopt Android's ROB-03 fix in Rust so "Standard"/"Milliarde" aren't false-discarded in ≤8-word utterances. Revisit M12 here once the open decision is resolved.
- **7.7 — Golden-Vector parity net (C1-proper).** Shared golden-vector fixtures (config + input → expected behavior) executed against **both** Rust and Kotlin in CI. Locks every Epic-7 fix + the dead-config landmines (so wiring one side later trips the net). The durable structural drift-detection net; supersedes "run the smartest model once."

### 4c — ADR-0016 Amendment (ready to append — does NOT overwrite the Decision block)

```markdown
## Amendment 1 (2026-06-10) — chirurgische Linienverschiebung nach A/B-Drift-Audit

**Auslöser:** `docs/cross-platform-drift-audit.md` (verifizierter A/B-Lauf) fand Divergenzen,
die der ursprüngliche Robustheits-Audit (DIV-01..14) nicht adjudiziert hat — insbesondere
**Kern-Output-Determinismus-Drift** (H2 UTF-8-Bytes vs UTF-16-Code-Units beim Chunking; H1/H17
Auto-Stop-Energie-Gate liest `silenceThreshold` nicht) und **settable-but-silently-dead Config-Keys**
(surface-operable Traps). Die Originallinie wurde auf einer unvollständigen Liste gezogen.

**Reframe:** Die ursprüngliche Decision wog die Divergenzen als *Feature-Paritäts-ROI*. Für
*Kern-Output-Determinismus* und *Config-Contract-Integrität* gilt diese Abwägung nicht: hier
erzeugt dieselbe Config + derselbe Input nachweislich anderen Diktat-Output bzw. ein gesetzter
Nutzer-Wert verpufft still. Das ist Notwehr an der Datenintegrität/Erwartungstreue, kein Feature.

**Härten-Klasse erweitert (chirurgisch) — neu in Stories (Epic 7):**
- Kern-Output: H2 (Chunking-Längeneinheit), H13/L4 (Join/Operator), M8; H1/H17/Recall#1/M1-3/M4/L1
  (Auto-Stop-Gate + Pre-STT-Schwellen).
- STT-Konditionierung (übersteuert DIV-08): H3 + Recall#5; H9/H10/L3 (Model-/Temp-Reads).
- Output-Guards (übersteuert DIV-11): H6 (prompt-echo), H7 (fragment-strip).
- Routing-Contract-Hygiene: M9/M10/M11/M13/M16/L5.
- Gegenrichtung (Desktop ist die falsche Seite): H14 — Androids Whole-Word-Match nach Rust
  zurückportieren.
- Struktureller Wächter: Golden-Vektor-Paritäts-Netz (C1-proper) gegen künftige Drift.

**Weiterhin bewusst akzeptiert (→ Backlog, NICHT hard-won't-fix):** Die ROI-Begründung der
Original-Decision bleibt für reine Feature-Ports gültig — sie wandern in `docs/backlog.md` statt
in Stories, damit sie nicht als Bug re-gefiled werden, aber auch nicht verloren gehen:
- DIV-06 (H12 Provider-Fallback), DIV-07 (H4 outputLanguage), DIV-09 (H11 Local-Cleanup-Prompt),
  DIV-10 (L7 Voice-Command), DIV-13 (H16 OpenAI-STT), DIV-14 (M5 VAD-Statemachine).
- Feature-Ports: C2 (Whisper-Mode), H5 (Anthropic), H8 (Mic-Wahl), H15 (Per-App-Profiles),
  M14 (Webhook), Recall#4 (Live-Preview), M6 (WAV-Float).

**Offene Decision:** M12 (Dictionary-in-Chat-Style) — Gegenrichtung, kanonische Seite ist Produktfrage.

**Rationale-Erhalt:** Die Linie bewegt sich nur dort, wo Drift den *Kern-Output* verfälscht oder
einen *gesetzten Config-Wert still verschluckt*. Das ~2000-LOC-Duplikat wächst minimal und gezielt;
die strategische Dedup-Antwort bleibt v2.
```

## Section 5 — Implementation Handoff

**Scope: Major.** Route to sprint-planning → story cycle.

1. **Apply concrete edits** (this session, on approval): append Amendment 1 to `docs/adr/0016-android-path-parity-strategy.md`; add Epic 7 to `_bmad-output/planning-artifacts/epics.md`; create `docs/backlog.md` with the Sorte-2 list + M12 OPEN-DECISION.
2. **`bmad-sprint-planning`** — fold Epic 7 (7.1–7.7) into `sprint-status.yaml` as backlog.
3. **Story cycle** — `bmad-create-story` 7.1 first (highest-value core-output, Test-Architect `*risk`/`*design`), then 7.2 … 7.7. 7.7 (parity net) ideally after 7.1–7.6 so it locks the just-fixed behavior.
4. **Commits/pushes:** human-controlled (per project convention). No auto-commit.

**Success criteria:** Epic 7 stories carry golden-vectors for every HIGH/CRITICAL fix; the parity net (7.7) is wired into CI and trips on a deliberately re-introduced drift (inversion-check); the backlog SSOT captures all 17 deferred rows + M12 so nothing is silently lost.
