# klarvo — Backlog (deferred-work SSOT)

Single source of truth for deferred work. Every scope-cut / phase-deferral lands here with a source ref
(per backlog-discipline). Created 2026-06-10 by `bmad-correct-course` off the cross-platform drift audit.

---

## Cross-Platform Drift — Sorte-2 (deferred, ADR-0016 Amendment 1 = accepted asymmetry → backlog, NOT hard won't-fix)

Source: `docs/cross-platform-drift-audit.md` · routed by `sprint-change-proposal-2026-06-10.md`.
These are pure feature-ports / marginal asymmetries — the ADR-0016 ROI rationale ("don't deepen the
~2000-LOC duplicate for marginal benefit; v2 is the dedup answer") still holds. Listed so they are not
re-filed as bugs and not lost. The fixed (Sorte-1) rows live in Epic 7 (`epics-cross-platform-parity.md`).

| ID | Divergence | ADR-0016 | Class |
|----|------------|----------|-------|
| C2 | Whisper-mode amplify (3 keys + audio gain) — no-op on Android | — | feature port |
| H4 | `outputLanguage` translation — Android never translates | DIV-07 | feature |
| H5 + Recall #2 | Anthropic provider absent on Android (struct field + branch); `llmProvider=anthropic` → silent DeepSeek | — | feature port |
| H8 | `audioDevice` selection ignored (always MIC) | — | feature port |
| H11 | Local cleanup system-prompt abbreviated on Android | DIV-09 | quality |
| H12 + M7 | No provider fallback on 429/5xx; different fallback order | DIV-06 | resilience |
| H15 | Per-app profiles ignored on Android | — | feature port |
| H16 | `sttProvider=openai` silently hits Groq / refuses | DIV-13 | feature |
| M5 | Full 4-state VAD state-machine parity (heavy algo rewrite) | DIV-14 | algorithm |
| M6 | Pre-STT WAV float support (latent; both PCM16 today) | — | latent |
| M14 + Recall #3 | Webhook delivery + `webhookHeaders` never read on Android | — | feature port |
| M15 | Desktop adopt STT retry/backoff (Android is the better side) | — | Desktop enhancement |
| L2 | Waveform amplitude transform differs | — | cosmetic |
| L6 | Empty-transcript toast text differs (same outcome) | — | cosmetic |
| L7 | Voice command (`voiceCommandEnabled`) unimplemented on Android | DIV-10 | feature |
| Recall #4 | Live-preview delta (`PreviewFlushConfig`, Epic 5) — no Android counterpart | — | feature port |
| Dead-config cluster | `advanced.llmTemperature`/`llmMaxTokens`, `chunkThreshold`/`chunkTargetSize`, `sttTemperature`, `llmModel*`/`llmSystemPrompt*` overrides, `autoCapitalize`/`autoPaste` — settable, consumed nowhere | — | latent landmine |

> The dead-config cluster's *current* state (both sides hardcode) is **locked by Epic 7 Story 7.7**
> golden-vectors. The *wire-when-needed* implementations (actually honoring these keys on a platform)
> stay backlog until a key is genuinely needed.

### OPEN-DECISION — M12 (dictionary-in-Chat-style)

**Source:** drift audit M12. Android adds the dictionary for ALL cleanup styles incl. Chat
(`KApi:591-593,749`); Desktop omits `{dict_section}` in the Chat arm (`llm/mod.rs ~232-263`). **Opposite
direction.** Canonical direction is a product call. To be resolved in **Epic 7 Story 7.6**; until then both
sides' current behavior is golden-vector-locked, not changed.

---

## Other deferred work

### License (from B1, `sprint-change-proposal` predecessor work)
- `ls_client::validate()` is dead code (no prod caller) — periodic re-validation never wired; only the
  `activate()` gate is live. Source: `project_v1_feature_roadmap` memory, `deferred-work.md`.
- Live-key acceptance in a release build is unverified (needs a real purchase) — close at real launch.

> Older per-feature deferrals tracked in `_bmad-output/implementation-artifacts/deferred-work.md` remain
> valid; migrate them here opportunistically.
