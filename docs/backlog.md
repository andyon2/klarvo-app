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

## Story 9-6 (keyboard-collapse via a11y service) — PARKED (obsolete)

Source: Andi's decision 2026-06-16 (this session). 9-6 was scoped to collapse the soft keyboard via the
accessibility service before showing recording UI. With the new pop-up **preview window**, the preview
simply lays *over* the keyboard — so the a11y keyboard-collapse mechanism is no longer needed. Parked,
not won't-fixed: revisit only if the overlay-over-keyboard approach turns out insufficient on a real
device. Status stays `backlog` in sprint-status (no formal "parked" state); the routing hook steers the
cascade away from it. **9-8 is NOT parked** — the long-press popover still needs verifying and stays
blocked on the real-device state-driving harness (DEBUG_SET_STATE dead on HyperOS, above).

## Story 9-7 follow-up — make Android silence-selection swap-safe (call-site wiring)

Source: Story 9-7 code-review (story-conductor, 2026-06-16), one Medium finding accepted as residual by
Andi at GATE 3. The AC6 regression test locks the *pure* `RecordingMode.selectSilenceSecs()` mapping,
but the **call site** in `KlarvoOverlayService.startRecording()` passes four same-typed `Float`
arguments (`tapSilenceSecs` / `longPressSilenceSecs` / `autostopSilenceSecs` / `autoModeSilenceSecs`).
A swap of two of those at the call site would regress production silence-selection while every JVM test
stays GREEN — and because the production defaults are all `2.0f`, such a swap is value-invisible until a
user sets the fields to different values. NOTE: this is NOT the original silence-field divergence
(wrong-field-*read*), which 9-7 DID lock; it is a new, low-probability surface introduced by the
testability extraction itself.

- **STORY — close the call-site gap.** Either (a) give the four silence durations distinct value-class
  types so a call-site swap fails to **compile**, or (b) add a test that exercises the real
  `startRecording()` field→param wiring (heavier — needs Android-context test infra). Option (a) is the
  by-construction fix and is preferred. **AC:** a deliberately swapped call-site argument fails to
  compile (a) or turns a test RED (b).

## Epic 9 — on-device defects found 2026-06-16 (Andi real-device test, after 9-7)

Source: Andi's real-device smoke 2026-06-16 (the device test I wrongly skipped at 9-7 GATE-4). Three
findings; the first is a real functional bug, the other two Andi explicitly flagged for "later".

- **RESOLVED into Story 9-11** — Android Auto mode quiet-speech miss. Initially reported as "Auto totally
  broken"; instrumented-build device telemetry (2026-06-16) showed that was a **stale build** — Auto works
  ~90%. The real residual: the RMS energy pre-gate is hard-coded `0.02` on Android (4× the desktop default
  `0.005`) and ignores the user's `silence_threshold` setting, so quiet speech (mic peaks ~0.026–0.037)
  fails the onset and merges into the next utterance. Root cause device-evidenced (`/tmp/9-7-auto-vad.log`).
  → **Story 9-11** (Android honors `silence_threshold` + default 0.005 + multi-fire guard), being built now.
  9-7 closed (its mode-mirroring scope was met; this was never a mode bug).

- **STORY 9-12 (follow-up to 9-11) — proactive sensitivity hint.** Andi's idea: rather than chase a magic
  threshold, make the recurring pre-filter problem **self-serviceable**. When the app detects the pattern
  "Silero VAD reports speech but the energy pre-gate vetoes it → onset never confirms → utterance dropped/
  merged" (exactly the `vadTrue` high / `speechFrames` low / `onsetFrames=0` signature in the device log),
  surface a user-facing hint: *"Speech detected but discarded as too quiet — adjust the sensitivity?"* with
  a shortcut to the slider. Precondition: **9-11** (the adjustable control must exist first). Cross-platform:
  consider whether desktop wants the same hint. Scope as a proper story.

- **NEXT STORY — bubble must show which mode is active.** On-device, the bubble's appearance does not make
  the current gesture mode (Hold / Toggle / AutoStop / Auto) visible at all — the user can't tell what is
  about to happen on a tap. Andi: "müssen wir in einer nächsten Story auf jeden Fall planen." Scope a
  story that surfaces the active mode on the bubble (a badge/affordance — note the `RecordingMode` enum
  already carries a one-letter `badge` H/T/S/A, currently unused on the bubble surface). Cross-platform:
  check desktop parity.

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
  - **Re-confirmed 2026-06-16 (Andi real-device, after 9-7):** the preview display itself does not work
    on Android, AND the **waveform inside the preview/listening panel does not work** either. Both stay
    "later" per Andi. Scope together with the preview-build investigation above.

- **No Android Settings control for the live-preview.** Desktop exposes a preview opt-in toggle + pause
  slider (Stories 5-3 / 5-5); Android Settings has no equivalent. A parity story once the Android
  preview itself exists.
  - **Re-confirmed 2026-06-19 (Andi 9-5 GATE-4):** the desktop-style *cleaned* live-preview is still
    not built on Android (answers "in which story is live-preview planned?" — it is **not yet a story**,
    only this backlog item). Needs its own story via on-device Whisper (benchmark-first; 3 design gates:
    local-vs-cloud ADR / paused-vs-continuous / raw-preview). 9-5 ships caret-only RECORDING feedback.

## In-app big-mic-button on Android — fate undecided (surfaced 2026-06-21 during 9-9)

Source: 9-9 conductor run + Andi. While re-skinning the in-app `RecordButton` (`android-05`), found that
this surface is **not in the current Model-B design canon**: `.inapp-mic` (klarvo.css:452) is an orphan
CSS rule, never instantiated in `docs/design/overhaul/source/Klarvo Design System.html`. The canon's
Android recording language is the bubble overlay only. The big round in-app mic button (App.tsx:51/734)
still **exists and is reachable on Android** (open app → tap mic → in-app record with Mic/Stop/Spinner),
but it is shared-desktop React, not an Android-designed surface.

- 9-9 closed it as a pure DT-closure (red→amber + remove hardcoded colors), NOT a design realization.
- **Open product decision (not 9-9's job):** should the in-app big-mic button on Android be **kept** as a
  standalone-recorder fallback, **hidden** on Android (recording = bubble only), or **redesigned** to the
  Model-B symbolism? If kept/redesigned, the canon needs an explicit in-app-recording surface (Phase A)
  first — today it has none.

## Story 9-5 GATE-4 green — Modell B interaction follow-ups (2026-06-19)

Source: Andi's 9-5 real-device GATE-4 (2026-06-19) — **9-5 approved → done**. These refine the *passing*
Modell B build; they are NOT 9-5 gaps. #2 and #4 change the recording-cluster **interaction** → each needs
its design decision settled in the **canon first (ADR-0019 §4′ amendment), design-gate = human**, before
build. Anchor: `docs/design/overhaul/source/` + `mockup-bubble-preview-modelB.html`.

> **✅ DESIGN-GATE AUFGELÖST 2026-06-21 (Andi-approved).** #2 + #4 sind im Canon entschieden:
> [ADR-0019 §4′-Amendment 2026-06-21](adr/0019-cross-platform-design-ssot.md#§4-amendment-2026-06-21--9-5-gate-follow-ups-2--4)
> + Canon-Quelle (Fingerprint `fc9ef745…`, MANIFEST 2026-06-21) + Render `mockup-9-5-followups-2-4.html`.
> **#2** = Cluster getauscht `[✗ links · Waveform · ➤ Senden rechts]`. **#4** = HOLD-Variante (halten=aufnehmen ·
> loslassen=senden · wegziehen=abbrechen · hoch=🔒→normaler Cluster); eigene Build-Story, 9-7 NICHT still erweitern.
> **#1** war immer design-frei. → Alle drei sind jetzt **build-reif** (kein Design-Gate mehr). #1 (design-frei)
> eignet sich direkt für den story-conductor; #2/#4 brauchen je eine Build-Story gegen den neuen Canon.

- **(1) Cluster waveform must be RMS-reactive (voice-driven), like the desktop app.** Today the amber
  waveform in the recording cluster animates but does not track the live voice amplitude — it looks
  static/idle. AC4 already specified "bars driven by RMS amplitude (reuse `drawWaveformBarsInZone()`)",
  so the amplitude feed into the **cluster** waveform zone is evidently unwired (or always falls back to
  the flat-idle `abwv` animation). **Fidelity fix** — trace the RMS stream into the cluster waveform.

- **(2) Swap ➤ Send and ✗ Cancel positions in the cluster.** Human habit expects the **➤ Send button at
  the same screen position as the idle "K" bubble** (the dock spot the thumb just tapped). Current layout
  is `[➤ send · waveform · ✗ cancel]` with the idle bubble at the right dock edge → Send should move to
  the idle-bubble side. **Design change to ADR-0019 §4′ cluster geometry** (Andi-decided direction; exact
  canon + `mockup-bubble-preview-modelB.html` update belongs to the fresh session).

- **(4) HOLD (push-to-talk) mode needs different bubble behavior.** In HOLD mode, releasing the hold
  **already sends**, so a separate ➤ Send button is redundant AND the ✗ Cancel button is effectively
  unusable (you'd have to keep holding to reach it, and releasing sends before you can cancel). The Modell
  B cluster (designed for tap/toggle) doesn't fit HOLD. **Design decision needed** — likely a per-mode
  cluster variant (e.g. HOLD shows only a slide/hold-to-cancel affordance, no ➤). Amends ADR-0019 §4′ for
  the HOLD gesture mode; relates to 9-7 (gesture modes) — do **not** silently expand 9-7.

- **OPEN DESIGN DECISION — where the "end dictation" affordance lives.** 9-5 (per ADR-0019) put *Senden*
  on the bubble (tap), with the panel's red square = *Abbrechen*. Andi is reconsidering whether the
  end-dictation control should instead sit **in the preview view next to the stop button**. This would
  revisit the ADR-0019 Android interaction model — handle via correct-course / an ADR amendment if
  pursued, not an ad-hoc change. Decision pending; no action now.

---

## Tooling — klarvo BMAD-Version auf 6.8 ziehen (eigene, bewusste Entscheidung; NICHT jetzt)

Source: BMAD-Internals-Session 2026-06-16 (Skill-Inventar-Diff klarvo 6.6.1-next.2 ↔ awos 6.8.0).

klarvo läuft BMAD `6.6.1-next.2`, awos `6.8.0`. Die Versions-Differenz erzeugt Skill-Namens-Drift
(6.8 hat Umbrella-Skills `bmad-prd`/`bmad-ux` + neue `bmad-investigate`/`bmad-spec`, die klarvo fehlen).
Andis Instinkt war, die Versionen anzugleichen — bewusst **entkoppelt** vom Routing-Guide-Cleanup, weil:
(a) es Cause-A (optionale Module CIS/TestArch) NICHT löst, nur die Namens-Drift; (b) ein Update **mitten
im Conductor-Lauf riskant** ist — unsere handgebauten Conductors hängen an genau den Skill-Internas
(On-Activation-Boilerplate, code-review-Step-Struktur, sprint-status, customize.toml-Schema), die ein
6.6→6.8-Sprung verschieben kann. **Vorbedingung vor Durchführung:** Blast-Radius-Prüfung (was ändert
sich an den Skills, an denen die Conductors hängen) + ein sicherer Zeitpunkt (kein laufender Epic-Lauf).
Referenz-Mechanik: `~/.bmad/guides/bmad-internals.md`. Status: **geparkt, Decision pending.**
