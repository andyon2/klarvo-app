---
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics", "step-03-create-stories"]
status: in-progress
inputDocuments:
  - docs/backlog.md  # "Epic 12 — Cloud-Resilienz" section — ✅ ENTSCHIEDEN 2026-07-02 (verified-code Ist-Zustand + Andi decisions)
  - _bmad-output/project-context.md
trackType: brownfield-feature
featureEpic: 12
note: >
  Separate planning artifact by design. Epic 12 was triggered by a live production
  incident (2026-07-02 DeepSeek API outage) and scoped in a design pass with Andi.
  The requirements source is the decision-complete "Epic 12 — Cloud-Resilienz" section
  in docs/backlog.md, grounded in a current-code audit performed this session against
  conductor/epic-11 HEAD (the branch this epic builds on). Shares the sprint-status.yaml
  ledger. Built via the L3 feature route; no PRD/Architecture/UX document.
---

# klarvo - Epic Breakdown (Cloud-Resilienz · Epic 12)

## Overview

A **reliability** epic, not a visual one. On 2026-07-02 the DeepSeek cleanup API went
down; Klarvo's existing provider-fallback did **not** fire (root cause below), cleanup
ran into ~30 s timeouts and silently degraded to raw text with **no user-visible
signal**. Epic 12 makes the failure behaviour robust and legible, and adds a brand-new
**audio-retry history** so a dictation is never lost when the cloud is unreachable.

There is no PRD/Architecture/UX document. Requirements below are extracted from the
decision-complete `docs/backlog.md` "Epic 12" section and a current-code audit.

## Verified current-state (audit 2026-07-02, conductor/epic-11 HEAD)

- **Fallback exists but was mis-gated.** `resolve_fallback_provider` (src-tauri/src/pipeline.rs:193)
  walks deepseek→groq→openai→openrouter. It only fires on `is_retryable_llm_error`
  (pipeline.rs:178) = `ApiError{status}` with 429 or ≥500. The outage produced **transport
  errors** ("error sending request for url" = timeout / connection-refused), which are NOT
  `ApiError{status}`, so they hit the non-retryable branch (pipeline.rs:1184) → straight to
  raw text, fallback never attempted. **← the incident's root cause.**
- **Warn message exists, UI discards it.** Backend emits `PipelineEvent::warn(degrade_warn_msg(..))`
  ("Cleanup failed — raw text inserted. <reason>", pipeline.rs:973/1163/1176/1188).
  `src/FloatingBar.tsx:335` deliberately drops `warning` events (`if (newState === "warning") return;`),
  so the user sees nothing.
- **Cleanup always degrades to raw text** (never a crash). **STT cannot** — no text means nothing
  to degrade to. Groq is today's STT provider AND the first cleanup fallback candidate → a cleanup
  fallback onto Groq eats the STT quota.
- **Audio is never persisted.** WAV bytes live only transiently (`last_recording`); the `history`
  table holds text only (`text, raw_text, style, language, is_note, app_name, uuid, device_id`).
  No audio column/blob/path, no re-processing. The audio-retry history is genuinely new.
- **Building blocks present:** local Whisper (`build_local_whisper_provider`, Windows+Android,
  pipeline.rs:84) today only on explicit offline mode; local llama.cpp cleanup also exists.

## Requirements Inventory

### Story 12-1 — Robust LLM/STT fallback ladder + pill-bar status signal

- **FR1 — Transport errors trigger fallback.** Timeout / connection-refused / DNS / TLS errors
  from a cleanup or STT provider must be treated as fallback-eligible (same class as 429/5xx),
  not as non-retryable. This is the core fix of the incident.
- **FR2 — Cleanup fallback chain, never Groq.** Cleanup fallback order: primary (DeepSeek) →
  OpenAI / OpenRouter (only if a key is present) → **raw text**. **Groq is never a cleanup
  fallback candidate** — it must be excluded so the STT quota is protected. (Adjust
  `resolve_fallback_provider`'s candidate list for the *cleanup* path accordingly.) Terminal =
  raw text, never a crash.
- **FR3 — STT fallback to local Whisper.** When the cloud STT provider (Groq) fails
  (transport error or 429/5xx), automatically fall back to the local Whisper provider if a model
  is available (today this only runs in explicit offline mode). If no local model is available,
  the dictation's audio is preserved for retry (handoff to 12-2) and a clear error is shown.
  Terminal = never a silent loss.
- **FR4 — Pill-bar status signal.** The FloatingBar must surface the degradation/fallback as a
  brief, transient status instead of discarding the `warning` event (remove/replace the
  `FloatingBar.tsx:335` early-return). Messages are generic-but-informative, one line, no stack
  trace. Proposed taxonomy (final wording a copy detail, not a design gate):
  fallback ran `⚠ DeepSeek langsam → OpenAI` · degraded to raw `⚠ Cleanup nicht verfügbar → Rohtext eingefügt` ·
  STT safety net `⚠ Groq am Limit → lokale Transkription` · all failed `✗ Transkription fehlgeschlagen — Audio gesichert`.
- **FR5 — Both platforms.** The fallback ladder + status signal apply on **Windows and Android**
  (shared-core logic where it exists; the pill/bubble surface on each).
- **NFR1 — Output parity on the happy path.** When the primary provider succeeds, behaviour and
  output are byte-identical to today; the ladder only changes the failure path.
- **NFR2 — No new user-facing configuration required** for the default ladder (OpenAI/OpenRouter
  are used only if the user has already entered those keys).

### Story 12-2 — Audio-retry history (primitive A + manual re-process)

- On **terminal** pipeline failure (STT could not produce text after the ladder), persist the
  recording's raw **WAV to disk** (Windows + Android) and create a **second-history** entry with
  a status field (`pending`). Data model is **B-capable** (status pending/done/failed + audio-as-file),
  so 12-3 sits on it without a rebuild. Compression is noted as a later concern for B (raw WAV now).
- Provide a **manual "re-process" action** on a pending entry that re-runs STT+cleanup when the
  cloud is reachable; on success, delete the stored audio (A-retention = transient).
- Out of scope: automatic background retry, permanent audio retention.

### Story 12-3 — Provider/settings comparison on the same recording (north star, later)

- Re-run one stored recording through different providers/settings and compare results. Builds on
  the 12-2 primitive; requires durable audio retention + compression + a comparison UI. Deferred.

## L3 guards (carried into 12-1 and 12-2)

- **(G-A)** Rust unit/integration tests for the fallback ladder: transport-error classification,
  Groq-excluded-from-cleanup-fallback, terminal-degrades-to-raw, STT→local handoff. The fallback
  logic is machine-verifiable — cover it.
- **(G-B)** Surface residual: the pill-bar status is a UI change on Windows AND Android. Android is
  GATE-4-smokeable via the emulator's structural window oracle where applicable; the Windows visual
  verdict + the actual on-outage behaviour remain Andi's real-machine gate (surface-DoD,
  project-context.md testing rules).
