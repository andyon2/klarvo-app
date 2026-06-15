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

## STT re-scope 2026-06-12 — deliberate deferrals + tooling

Source: `sprint-change-proposal-2026-06-12.md` + ADR-0017 (shared-core STT path).

- **ADR-0017 consolidation extension — VAD gate:** moving the live auto-stop VAD gate into the
  Rust core over JNI (realtime frame stream, ~31 Hz). Deliberately deferred — large lift,
  speech-truncation risk; the felt problem (text hallucination) doesn't touch it. Story 7.2 narrows
  the gap in Kotlin instead.
- **ADR-0017 consolidation extension — chunking/LLM path:** extending the "shared logic only in
  Rust" Hard Rule beyond STT (chunking, LLM routing). Deferred; 7.1/7.5 fix per-row, 7.7 pins
  against re-drift. Candidate for a future decision (or v2).
- **TOOLING — `scripts/dictation-quality-audit.py`:** commit the `raw_text` hallucination-marker
  detectors from `docs/dictation-quality-android-vs-desktop-2026-06-12.md` as a repo tool; run
  manually on a cadence (monthly / pre-release — needs the phone over adb/Tailscale, no cloud cron).
  Quality-layer sibling of the 7.7 parity net. Attached to Story 7.7.

---

## Epic 8 (Studio-Dark) — DT1 token-closure residual

Source: Story 8-6 review (2026-06-15). AC#3's closing DT1 grep gate was specified "zero covered-role
hits across ALL of `src/`", but that over-reaches Epic 8's actual surface scope. **The five Epic-8
desktop surfaces (FloatingBar, Settings [SettingsPanel + settings/*], Live-Preview, Main-Window/History,
Onboarding) ARE migrated and verified clean** of covered-role inline hex (the only inline hex left on a
surface is FloatingBar's value-correct literals — the documented 8-3 carve-out residual). AC#3 was
re-scoped to "the Epic-8 desktop surfaces" with this entry as the homing record (conductor, not an
inline Dev-Notes wave-away). Remaining DT1 work, all in **non-Epic-8-surface files**:

- **`AdvancedSettingsPanel.tsx` category-badge palette** (lines 185/210/238/263): four category icon
  badges use ad-hoc hex — `#14b8a6` (old teal, STT), `#8b5cf6` (purple, Text-Cleanup), `#f59e0b`
  (old amber, Audio), `#6b7280` (gray, System). Migrating needs a **per-category color decision**:
  teal/amber map to `klarvo-teal`/`klarvo-amber`, but **purple and gray have no Studio-Dark token** —
  pick canonical accents for those category roles, or collapse to a teal/amber/neutral scheme. Hidden
  expert/power-user panel, not on the Epic-8 surface list. Migrate as one coherent change (don't
  partially tokenize 2 of 4).
- **`MobileTextarea.tsx:54`** `bg-[#0a0a0b]` (canvas role; canonical `klarvo-bg-deep` is `#0A0B0C`) —
  mobile component (Epic 9 territory), trivially `bg-klarvo-bg-deep` when that surface is touched.
- **`ThemeSwitcher.tsx`** still carries legacy `Inter` + old teal — not an Epic-8 surface.
- **DT1 alias-layer closure:** the back-compat aliases from 8-1 still have live consumers
  (`klarvo-primary` ×9, `klarvo-warning` ×2, `klarvo-warm` ×2). Inline a sweep to the canonical
  `klarvo-teal`/`klarvo-amber` names and drop the aliases once consumers are zero.
- **Per-user preview rgba duplication:** the preview color-picker stores user-editable `rgba()` and
  cannot reference `var()` tokens; the duplicated literals across ~6 files have call-out comments only,
  no lint guard (DT1 SSOT spirit). Optional: a lint/grep CI guard so the Studio-Dark hex can't silently
  drift. (8-6 already pulled the AppearanceContent fallback args to canonical.)

---

## Other deferred work

### License (from B1, `sprint-change-proposal` predecessor work)
- `ls_client::validate()` is dead code (no prod caller) — periodic re-validation never wired; only the
  `activate()` gate is live. Source: `project_v1_feature_roadmap` memory, `deferred-work.md`.
- Live-key acceptance in a release build is unverified (needs a real purchase) — close at real launch.

> Older per-feature deferrals tracked in `_bmad-output/implementation-artifacts/deferred-work.md` remain
> valid; migrate them here opportunistically.
