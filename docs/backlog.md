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

## Other deferred work

### License (from B1, `sprint-change-proposal` predecessor work)
- `ls_client::validate()` is dead code (no prod caller) — periodic re-validation never wired; only the
  `activate()` gate is live. Source: `project_v1_feature_roadmap` memory, `deferred-work.md`.
- Live-key acceptance in a release build is unverified (needs a real purchase) — close at real launch.

> Older per-feature deferrals tracked in `_bmad-output/implementation-artifacts/deferred-work.md` remain
> valid; migrate them here opportunistically.

---

## Conductor verification — GATE-4 structural-assertion smoke (Android overlay)

Source: spike 2026-06-16 (memory `reference_android_emulator_window_structure_oracle`); postmortem
`docs/postmortem-2026-06-15-epic-conductor.md`; `project-context.md` Testing Rules (the "emulator =
structural-not-pixel oracle" rule). The conductor skills (`bmad-epic-conductor` / `bmad-story-conductor`,
E9 + GATE-4) now **assume** this assertion exists — this story makes it real.

- **STORY — harden `scripts/android-smoke.sh` to assert overlay-window STRUCTURE at GATE-4.** Today the
  unattended smoke drives to a state and screencaps; the emulator's *pixels* are an unreliable oracle
  (software GPU, cold-boot SystemUI ANR, `mForceHideNonSystemOverlayWindow`, no HyperOS skin) — that is
  why 9-5 went "emulator-green" while the real Xiaomi was broken. But the emulator reports overlay-window
  **structure** faithfully: `adb shell dumpsys window windows` exposes each `ty=APPLICATION_OVERLAY`
  window's requested size / gravity / visibility. Add a step that, per driven state, asserts the expected
  window structure (e.g. recording = exactly 1 panel ~1080×525 bottom + 1 idle bubble 162×162) and FAILS
  the smoke on a mismatch; emit `structure-<state>.txt` into the GATE-4 evidence dir. This is the machine
  oracle that would have caught 9-5 unattended; the residual (HyperOS pixel aesthetics) stays Andi's
  real-device gate. **AC:** a deliberately re-introduced 9-5-class regression (bubble in recording form
  instead of idle squircle) makes the structural assertion RED on the emulator.

- **RELATED-BUT-SEPARATE — real-device state-driving harness (already open).** `DEBUG_SET_STATE` broadcast
  is dead on HyperOS (background restrictions block the manifest receiver) → Andi's real-device morning
  gate can't be scripted, blocks 9-6/9-8. Replace with a HyperOS-survivable trigger. Tracked in the
  postmortem; listed here so it is not conflated with the structural-assertion story above.

## Accessibility — canvas-drawn listening panel has no TalkBack labels

Source: Story 9-5 code-review (story-conductor, 2026-06-16). AC1 / Task 2.2 said "relabel
`contentDescription` to Abbrechen", but the listening panel (`ListeningPanelView`) is a custom
`View` that draws its controls (red Abbrechen square, K-badge, waveform, timer) directly on a
`Canvas` — there is no child `View` to carry a `contentDescription`, and there never was one to
"relabel". The red square (and the whole panel) is therefore invisible to TalkBack. Pre-existing
limitation of the canvas-drawn approach, NOT introduced by 9-5 — deferred rather than faked.

- **STORY — give the listening panel a real accessibility surface.** Add an
  `AccessibilityNodeProvider` / `ExploreByTouchHelper` virtual view hierarchy over
  `ListeningPanelView` so TalkBack exposes the Abbrechen square (`contentDescription="Abbrechen"`)
  and the live transcript/timer. Scope is the whole canvas panel, not just the red square. **AC:**
  TalkBack focuses and announces the Abbrechen control; activating it cancels recording.

## Android live-preview parity + open dictation-end interaction question

Source: Andi's 9-5 real-device review (2026-06-16). 9-5 function approved ("erstmal abgesegnet");
these are **design decisions for later — NOT functional gaps in 9-5**, captured so they don't get lost.
Not yet scoped into stories.

- **Android live cleanup-preview is not working / likely not built yet.** The desktop has the live
  cleanup preview (Epics 5/6). On Android it does not work. Confirm whether it's simply unbuilt on
  Android vs. broken, then scope. (Note: 9-5's listening panel shows the live RAW transcript inside the
  panel per AC2 — distinguish that from the desktop's *cleaned* live-preview when scoping.)

- **No Android Settings control for the live-preview.** Desktop exposes a preview opt-in toggle + pause
  slider (Stories 5-3 / 5-5); Android Settings has no equivalent. A parity story once the Android
  preview itself exists.

- **OPEN DESIGN DECISION — where the "end dictation" affordance lives.** 9-5 (per ADR-0019) put *Senden*
  on the bubble (tap), with the panel's red square = *Abbrechen*. Andi is reconsidering whether the
  end-dictation control should instead sit **in the preview view next to the stop button**. This would
  revisit the ADR-0019 Android interaction model — handle via correct-course / an ADR amendment if
  pursued, not an ad-hoc change. Decision pending; no action now.
