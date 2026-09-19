---
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics", "step-03-create-stories"]
status: in-progress
inputDocuments:
  - docs/adr/0016-android-path-parity-strategy.md  # Amendment 4 (2026-09-17) = the decision-complete source: 4 + 46 verdicts with form, direction, size, test symbol
  - docs/cross-platform-drift-audit-2026-09-16.md  # Audit #2: code evidence per D-* row (not re-measured here)
  - docs/backlog.md  # "DECIDED 2026-09-17 — Parity-Linie über Audit #2"
  - _bmad-output/project-context.md
trackType: brownfield-feature
featureEpic: 13
note: >
  Separate planning artifact by design. Epic 13 turns Andi's per-row verdicts over drift
  audit #2 (ADR-0016 Amendment 4, answered 2026-09-17 through the decision-sheet artifact)
  into nine stories. Nothing here re-measures: every row keeps its D-* id, and the code
  evidence lives in the audit. Cut released by Andi 2026-09-17 ("J zu allem"). The build of
  each story is its own go. Shares the sprint-status.yaml ledger. L3 brownfield route; no
  PRD/Architecture/UX document.
---

# klarvo - Epic Breakdown (Parity-Linie über Audit #2 · Epic 13)

## Overview

A **parity** epic in the sense of ADR-0016 Amendment 4: v1-ship is the only product, Android
included, and every row of drift audit #2 now carries Andi's verdict. Epic 13 executes the
verdicts that are **close / gate / port / fix / strike**. Rows judged **asymmetry** or **open**
stay in the ADR list and are out of scope here.

Four principle decisions shape the stories:

- **G1a — license on both sides.** Desktop enforces the license in the dictation pipeline like
  Android. One free-tier definition for both platforms: **DeepSeek + Groq**.
- **G2a — "Offline" means no byte leaves the device.** Desktop-hotkey semantics are the norm;
  the Rust in-app button and Android follow.
- **G3a/G3b — Android hides both local paths for now.** Offline STT and local cleanup (plus the
  model manager) disappear behind platform gates. Android has no local path until a separate
  decision builds one (L, not in this epic).
- **Gate definition (new in Amendment 4):** a gate hides the control on Android **and**
  neutralizes a stored value at config load. The story spec names the normalization target;
  never a silent switch to cloud where the user had chosen local.

**Verification symmetry rule for this epic.** Andi ticked "Herstellbarkeit mitbauen" (**H+**)
on 15 agent-only rows. A story that carries an H+ row names, **before build**, how Andi
reproduces the state on his own device. Where no cheap path exists, the story records the
downgrade to "agent-verified only" (Weg 2) explicitly. Story 13-1 exists to make four of those
rows reproducible.

## Verified current-state (audit #2, 2026-09-16 — pointers, not re-measured)

- **License is reversed.** Android enforces; the Desktop hotkey pipeline has no gate; the
  Command gate reads `llm_priority` instead of `llm_provider`; the free-tier definition differs
  (Groq-only on Android, DeepSeek + Groq on Desktop). D-C1, D-H1, D-H2.
- **Guards moved after ADR-0017.** On Android the echo/fragment guard sees the dictionary
  (short sentences of dictionary words are dropped; a term ≥ 10 chars is deleted from every
  transcript), the order is inverted, ghost-strip runs before the guard only; Desktop strips
  after cleanup only. `customPrompt` goes to Whisper as a prompt on Android. D-H4–D-H7, D-M9.
- **No banking/password guard on Desktop.** Android has one but writes history/Turso before
  the guard blocks the paste. D-H3.
- **Android local cleanup cannot execute** (`libklarvo_mnn` never built, wrong model path) and
  reports raw text as success. D-H18. Local STT on Android runs `ggml-small` only, without
  dictionary or ghost-strip. D-H8, D-H13.
- **Silent loss / silent no-op on Android:** empty or truncated LLM answer is pasted, "nothing
  focused" shows the success check, clipboard failure crashes, cleanup failure shows success.
  D-H19, D-H20, D-M12, D-M16, D-M24.
- **Dead controls on Android:** `sttModel`, `localWhisperModel`, `outputLanguage`, profiles,
  silence slider, auto-send, Anthropic fields, Whisper fields, `pasteDelayMs`, `logLevel`,
  statistics panel, history app filter. D-H12–D-H17, D-M15, D-M17, D-L1–D-L3, D-L27.
- **Offline privacy:** Android "offline" + live preview uploads every pause to Groq; offline STT
  + cloud cleanup sends to DeepSeek on Android, not on Desktop; the in-app button has a third
  rule. D-H9, D-H10, D-M20, D-M21.
- **Twin infrastructure exists:** `test-fixtures/*.json` golden vectors read by Rust and JVM
  tests; `android/kotlin-test/.../Adr0017BoundaryGuardTest.kt` fails if a Kotlin STT/guard path
  reappears; `scripts/android-smoke.sh` (JVM gate) and `cargo test --lib` in `src-tauri/` run
  the net. Both twins hardcode provider URLs (`llm/mod.rs`, `KlarvoApi.kt`); the React settings
  gate with `isDesktop` / `isMobile` from `src/platform.ts`.

## Requirements Inventory

Row ids (A1 … G-Fix) and D-* ids are those of ADR-0016 Amendment 4 / audit #2. Test symbols:
📱 Andi on the Android device · 🖥️ Andi on the Windows machine · 🤖 agent only · **H+** Andi
ordered reproducibility. Sizes are Amendment-4 sizes.

## Epic 13: Parity line over audit #2

### Story 13.1: Debug test provider, both twins

**Key `13-1` — Debug test provider in both twins (H+ enabler) · S–M**

**Rows served:** H+ on D2, D3, D9, D10 (and D6 if the mechanism can inject a clipboard failure).

As the klarvo maintainer,
I want a debug provider for LLM and STT on **both** platforms that returns a configured canned
answer,
So that Andi can provoke "empty answer", "truncated answer", "malformed answer", "429/5xx" and
"transport error" on his own devices without a server.

**Acceptance Criteria (outcomes — full Given/When/Then in create-story):**
- A provider value (name chosen in the spec) exists in the Rust twin (`llm/mod.rs`, `stt/`) and
  the Kotlin twin (`KlarvoApi.kt`) and produces each of the five answer shapes from configuration.
- Selectable on both devices **without a computer attached** — an Advanced/Debug section in the
  React settings (renders on both platforms) or an equivalent on-device path; `config.json` alone
  is not enough for Android.
- Never part of the production fallback ladders (Epic 12 FR2 stays: Groq is never a cleanup
  fallback); never the default; invisible in the normal provider picker.
- Rust and JVM unit tests cover every shape; inversion check RED at writing time.

**Out of scope:** any change to real provider behaviour. **DoD:** tests green; 🖥️📱 Andi selects
"empty answer" on each device and sees the current (pre-13-2) behaviour, which proves the enabler.

### Story 13.2: Parity sweep: guards and silent loss

**Key `13-2` — Parity sweep 1: guards, core output, silent loss · S rows**

**Rows:** B2 (D-H5, D-H6, D-M9) · B3 (D-H7) · B4 (D-H4) · B6 (D-M10, D-L19, D-L21) · B1 Android
part (D-H3: history/Turso only after the guard) · D2 (D-H19) · D3 (D-M16) · D4 (D-H20) · D5
(D-M24) · D6 (D-M12) · D9 (D-M5, D-M6) · D10 (D-M2) · D11 (D-M14) · E1 (D-H9) · E2 (D-H10,
D-M20, D-M21). **Depends on 13-1** for the H+ rows D2, D3, D9, D10.

As a klarvo user on either platform,
I want the guards and the failure paths to behave the same on Android as on Desktop,
So that a dictation is never silently deleted, mangled, pasted half, or reported as success when
it failed.

**Acceptance Criteria (outcomes):**
- **B2** — in Rust `groq_jni.rs`, direction Desktop: echo/fragment guard without the dictionary,
  guard order as Desktop. 📱 Dictionary "Klarvo, Kubernetes", say "Klarvo und Kubernetes." → the
  sentence survives.
- **B3** — ghost-strip runs before the guard **and** after cleanup on both sides. 🤖 H+: a known
  ghost phrase dictated at the end is stripped on both devices.
- **B4** — Kotlin reads `advanced.sttPrompt*` and passes them through; `customPrompt` goes to the
  LLM only. 📱 Preset "Technical", same audio, transcript changes accordingly.
- **B6** — VAD hysteresis 0.5/0.35 and hangover frame in Kotlin as on Desktop (three measured
  deltas only; the state-machine parity L stays closed). 🤖 golden vector.
- **B1 Android part** — history and Turso writes happen after the banking guard's verdict. 🤖.
- **D2 / D3** — empty or truncated LLM answer ⇒ no paste, no `""` history entry (twin of Desktop).
  🤖 H+ via 13-1.
- **D4** — no focused field ⇒ "In Zwischenablage" hint instead of the success check. 📱.
- **D5** — pill = status light (as 7-10): no success check on cleanup failure. 📱.
- **D6** — Android try/catch around the clipboard; Desktop pill never says "In Clipboard" when
  nothing is in it. 🤖 H+ if 13-1 can inject it, else Weg 2 recorded.
- **D9** — empty STT result marked non-retryable; retry budget 1 like Desktop. 🤖 H+ via 13-1.
- **D10** — fallback also on a malformed provider answer to a short dictation. 🤖 H+ via 13-1.
- **D11** — no toast on "nothing recognized" on Android (direction Desktop). 📱.
- **E1** — live preview uploads nothing while a stored `local` STT value is active. 🤖 H+:
  stored `local`, airplane mode off, log shows no upload.
- **E2** — one offline rule: "offline" STT ⇒ cleanup local or none, on the Rust in-app button and
  on Android (after G3a only for stored values). 🖥️ in-app button: choose offline, dictate with
  filler words, the "ähm" stay.
- Golden vectors for B2, B3, D2, D3 in `test-fixtures/`; `Adr0017BoundaryGuardTest` stays green;
  inversion check RED at writing time for every new vector.

**Out of scope:** the Desktop banking blocklist (13-6), any UI gate (13-3). **DoD:** JVM gate +
`cargo test --lib` green with inversion evidence; 📱 rows above on Andi's Xiaomi; 🖥️ E2 on Windows.

### Story 13.3: Parity sweep: Android gates and config hygiene

**Key `13-3` — Parity sweep 2: Android gates + config hygiene · S rows**

**Rows:** A2 (D-H2) · C1 (D-H12, port) · C2 (D-H13, gate = G3a) · C3 (D-H14) · C5 (D-H16) · C6
(D-H17) · C7 (D-M17) · C8 (D-L1) · C9 (D-L2, D-L3, strike) · C11 (D-L6, strike) · C12 (D-L27,
port) · C13 gate (D-M15) · D1 (D-H18, gate = G3b + fix) · G-Fix list.

As a klarvo user on Android,
I want every control I can see to do something, and every hidden path to stay hidden,
So that a setting never silently dies and the two local paths cannot be reached on Android.

**Acceptance Criteria (outcomes):**
- **Gates (Amendment-4 definition: hide + neutralize stored value, never silently to cloud):**
  C2 `localWhisperModel` and the offline-STT option (G3a); D1 "Local (Offline)" cleanup + model
  manager (G3b); C6 auto-send; C7 Anthropic key + model fields; C8 the two Whisper Advanced fields
  (mobile); C13 statistics panel. The spec names the normalization target per key. 📱.
- **Ports / fixes:** C1 Kotlin reads `sttModel` and passes it to the JNI (🤖 H+: change the model,
  the log shows the request); C3 translate sentence in `KlarvoApi.appendPromptExtensions`, twin
  of `llm/mod.rs` (📱); C5 the silence slider takes effect on Android (📱); C12 Android writes the
  package name from the a11y service into history so the "App…" filter works (📱); D1 fix: a
  cleanup failure ⇒ clipboard + cause, never "success" (📱).
- **A2** — provider controls locked with `isPaid`; the lock names the two free-tier providers
  (DeepSeek, Groq). 📱.
- **Strikes:** C9 remove the `pasteDelayMs` and `logLevel` controls on both platforms (🖥️📱);
  C11 remove `voiceNotesHotkey`, `bubbleSize`, `bubbleOpacity`, `bubbleRecordingMode`,
  `webhookHeaders`, `webhookTimeoutSecs` from the config (both twins, load stays tolerant of
  old files). 🤖.
- **G-Fix hygiene list** (🤖 H+ where a state is reachable, else Weg 2 recorded): D-M1 config-load
  ladder (direction 12-1) · D-M11 unknown `cleanupStyle` ⇒ silently Polished, no config reset
  (direction Android) · D-M18 empty-vs-blank predicates · D-L5 deviceId · D-L10 trim · D-L11
  alphanumeric · D-L12 HTTP-2xx · D-L13 empty-term guard · D-L14 sanitize passthrough · D-L15
  blank tables · D-L16 clamp · D-L32 dead entry point · three wrong docstrings (audit §8).
  D-M22 (local cleanup without chunking) stays latent until a G3b build.
- Platform-reach statement per change (project-context rule); every gate uses `isDesktop` /
  `isMobile` from `src/platform.ts`, no ad-hoc user-agent checks.

**Out of scope:** the license gate itself (13-4), statistics port (candidate, not cut). **DoD:**
JVM gate + `cargo test --lib` + `npm` type-check green; 📱 Andi opens Settings on the Xiaomi and
finds none of the gated controls; 🖥️📱 C9 controls gone on both.

### Story 13.4: Desktop license gate and one free tier

**Key `13-4` — Desktop license gate + one free-tier definition (G1a) · M + S**

**Rows:** D-C1 (M) · D-H1 (S) · the Command-gate key bug (`llm_priority` vs `llm_provider`).

As the klarvo maintainer,
I want the Desktop dictation pipeline to enforce the license exactly like Android, with one
free-tier definition (DeepSeek + Groq) on both platforms,
So that the paywall means the same thing wherever Klarvo runs.

**Acceptance Criteria (outcomes):**
- Desktop hotkey pipeline applies the same license decision as Android's runtime: unlicensed ⇒
  providers outside the free tier are not used; the result is visible (pill status or settings
  lock), never silent.
- One free-tier definition **DeepSeek + Groq**, shared by both twins (fixture-pinned like the twin
  constants). Android widens from Groq-only accordingly; the A2 lock text (13-3) matches.
- The Command gate reads `llm_provider`. Local cleanup is not overridden by the license (A1 lands
  in 13-5, but nothing in 13-4 may widen the gap).
- Rust + JVM tests pin the free-tier set and the gate decision on both paths; inversion RED.
- 🖥️ Andi removes his license key on Windows and dictates with OpenAI configured ⇒ same behaviour
  as on Android today (cleanup runs on a free-tier provider, the UI says why).

**Out of scope:** the software license choice (BSL/PolyForm) and the publication question —
untouched, see `docs/backlog.md` "Lizenzwahl OFFEN". Trial and activation rows (13-5).

### Story 13.5: Parity sweep: license rows and sync hygiene

**Key `13-5` — Parity sweep 3: license side rows + sync hygiene · S rows**

**Rows:** A1 (D-C2) · A3 immediate (D-M23, D-L30, D-L7) · A4 (D-L31, D-L32) · A5 (D-M19) · A6
(D-L9) · F1 (D-M3) · F3 (D-L25) · F4 D-L24 · F5 D-L28. **Depends on 13-4** (A5, A6 semantics).

As a klarvo user,
I want trial, activation and sync to behave the same on both platforms,
So that deleting a file never restarts a trial, a new license works without a restart, and sync
never claims success it did not verify.

**Acceptance Criteria (outcomes):**
- **A1** — Android exempts `local` from the license override (after G3b: stored values). 🤖 H+.
- **A3 immediate** — the paywall advertises only features that exist on the platform showing it.
  🖥️📱. The feature fate of Snippets / Cross-Device-Sync / Command-Mode stays open (not here).
- **A4** — a library load error is reported as such, not as "No API keys configured"; the dead
  second license entry point is removed. 🤖 H+ (rename the library, see the message).
- **A5** — trial start no longer derived from `config.json` (direction Android: install time or
  equivalent). 🤖 H+: delete `config.json`, no new trial.
- **A6** — Desktop recognizes a newly activated license without restart. 🖥️.
- **F1** — Android reads the Turso response before marking rows synced. **F3** — Android rejects
  `http://` Turso URLs like Desktop. **F4** — D-L24 date format aligned. **F5** — D-L28 atomic
  writers (ADR-0015 class). 🤖 H+ needs a reachable Turso DB + log visibility; if Andi has none,
  the story records Weg 2 for the F rows.
- Docs: D-L23, D-L26, D-L29 documented as no-action / asymmetry in the story (Amendment 4 list).

**Out of scope:** F2 (open, coupled to A3). **DoD:** tests green; 🖥️ A6 on Windows; 🖥️📱 A3.

### Story 13.6: Desktop banking guard (process blocklist)

**Key `13-6` — Desktop banking/password guard by process name (B1) · M**

As a klarvo user on Windows,
I want dictation into a banking or password-manager window to be blocked like on Android,
So that a dictated secret never lands in history, sync or the wrong field.

**Acceptance Criteria (outcomes):** a blocklist by process name on Desktop mirrors Android's
`BankingGuard` semantics (block paste, no history/Turso write, pill status says why); default
list and its source named in the spec; Rust tests for the decision; 🖥️ Andi focuses a
blocklisted process and dictates ⇒ blocked with a visible status. **Out of scope:** the Android
order fix (13-2).

### Story 13.7: Android per-app profiles and profile language

**Key `13-7` — Android per-app profiles + per-profile language (C4) · M + S**

As a klarvo user on Android,
I want per-app profiles to fire by package name, and the language set on a profile to apply on
both platforms,
So that the profile UI keeps its promise.

**Acceptance Criteria (outcomes):** profiles match on the foreground package (the `BankingGuard`
already knows it), not on a window title; `profiles[].language` is consumed by both twins
(fixture-pinned); 📱 Andi sets a profile for one app with another language and dictates there.
**Out of scope:** new profile fields.

### Story 13.8: Native preview blur wiring

**Key `13-8` — `previewBgBlur` drives the native preview overlay (C10) · M**

As a klarvo user on Windows,
I want the blur slider to change the native preview overlay,
So that the control does what it says (lifts the Epic-10 defer "blur slider no-op").

**Acceptance Criteria (outcomes):** the slider value reaches `native_preview.rs` and changes the
painted background (tiny-skia); ADR-0021 gets an amendment note (VR3 dropped blur; reintroduced
natively); objective pixel metric before acceptance (`feedback_verify_surface_fix_with_objective_pixel_metric`);
🖥️ Andi moves the slider and sees the overlay change. **Out of scope:** blur on Android.

### Story 13.9: OpenAI STT provider via Rust

**Key `13-9` — OpenAI as STT provider via the Rust core (D-H11) · S–M**

As a klarvo user on either platform,
I want `sttProvider = openai` to transcribe with OpenAI,
So that the setting is not silently Groq.

**Acceptance Criteria (outcomes):** provider switch in `stt/groq_jni.rs` (ADR-0017: STT only in
Rust), Kotlin passes the OpenAI key; `Adr0017BoundaryGuardTest` stays green; Rust tests for the
request shape; 🖥️📱 Andi sets OpenAI + key, dictates, the log shows the OpenAI endpoint and the
text arrives. **Out of scope:** any Kotlin STT code.

## Not in this epic (candidates without release, recorded in ADR-0016 Amendment 4 / backlog)

- C13 statistics **port** (Kotlin writes usage rows), M.
- G3b **build** Android local cleanup (native library, build, model path), L.
- A3 / F2 feature fate: Snippets, Cross-Device-Sync, Command-Mode (build or strike from the paywall).
- Rows judged asymmetry (D12 D-M13, F5 D-L29, VAD state machine M5) and documentation-only rows
  (G-Dok: D-L8, D-L17, D-L18, D-L20, D-L22; F4 D-L23, D-L26).
- 12-3, 9-8, 8-6, 8-7 keep their own numbers.

## L3 guards (carried into every story)

- **(G-A) Twin rule.** Every shared-behaviour change lands in both the Rust and the Kotlin path or
  is platform-gated; each story states its platform reach (project-context "Android bypasses
  Tauri IPC").
- **(G-B) Golden-vector net.** Guard and core-output rows get a fixture in `test-fixtures/` read by
  both a Rust and a JVM test; **inversion check RED at writing time** for every new vector or
  guard. `Adr0017BoundaryGuardTest` stays green; STT logic stays in Rust.
- **(G-C) Verification symmetry.** A story with an H+ row names the reproduction path before
  build; a downgrade to agent-only is written into the story, never implied. Human gates only
  where the human sees what the machine cannot (Epic 7 retro).
- **(G-D) Surface DoD.** Rows that change what Andi sees (pill, toast, settings) end with a
  Windows release build and/or a fresh APK on his Xiaomi (`feedback_smoke_test_dod_gate`).
- **(G-E) Standing decisions untouched.** No auto-send on Android; Groq never a cleanup fallback
  (Epic 12 FR2); license choice BSL/PolyForm not decided here; nothing built before its own go.

## Proposed order (critical path, sprint-planning may reorder)

13-1 → 13-2 (needs 13-1 for H+) → 13-3 (independent, visible on the Xiaomi) → 13-4 → 13-5
(needs 13-4) → 13-6 · 13-7 · 13-8 · 13-9 (independent). Rationale: 13-2 carries the user-visible
guard bugs (a dictionary term ≥ 10 chars deleted from every Android transcript), so it goes
right after its enabler.
