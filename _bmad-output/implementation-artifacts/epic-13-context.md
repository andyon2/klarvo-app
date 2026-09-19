# Epic 13 Context: Parity line over audit #2

<!-- Generated from planning artifacts. Regenerate with compile-epic-context if planning docs change. -->

## Goal

Klarvo ships one product across a Tauri/Rust desktop and a native Kotlin Android app, and a second cross-platform drift audit found 78 rows where the two sides disagree — guards that delete text on one platform only, failures reported as success, controls that render but do nothing, a paywall enforced on Android and absent on Desktop, and an "offline" mode that still uploads audio. Every row now carries a per-row verdict from the product owner (close / gate / port / fix / strike, or explicitly "stays an asymmetry"). Epic 13 executes the actionable verdicts in nine stories, so that a dictation is never silently lost, mangled or falsely confirmed; every visible control either acts or is gone; both local paths on Android are hidden behind gates until a separate decision builds them; and the license means the same thing on both platforms. Rows judged asymmetry, documentation-only, or "candidate without release" are deliberately out of scope.

## Stories

- Story 13.1: Debug test provider in both twins (H+ enabler)
- Story 13.2: Parity sweep 1 — guards, core output, silent loss
- Story 13.3: Parity sweep 2 — Android gates and config hygiene
- Story 13.4: Desktop license gate and one free-tier definition
- Story 13.5: Parity sweep 3 — license side rows and sync hygiene
- Story 13.6: Desktop banking/password guard by process name
- Story 13.7: Android per-app profiles and per-profile language
- Story 13.8: `previewBgBlur` drives the native preview overlay
- Story 13.9: OpenAI as STT provider via the Rust core

## Requirements & Constraints

No PRD, architecture or UX document exists for this epic (L3 brownfield route). The binding inputs are the parity ADR's fourth amendment (the decision-complete row verdicts), the drift audit that supplies the code evidence per row, the backlog decision entry, and the project context rules digest. Nothing in this epic re-measures the audit: every requirement keeps its audit row id (`D-*`) and its decision-sheet row id, and those ids are the traceability anchors a story spec cites.

Four principle decisions bind every story:

- **License on both sides.** The desktop dictation pipeline enforces the license like Android does. There is exactly **one** free-tier definition for both platforms: **DeepSeek + Groq**. Android widens from its current Groq-only definition.
- **"Offline" means no byte leaves the device.** Local STT implies local cleanup or no cleanup. Desktop-hotkey semantics are the norm; the Rust in-app button and Android follow.
- **Android hides both local paths.** Offline STT, local cleanup and the model manager disappear behind platform gates. Android has no local path until a separate, later decision builds one — explicitly not in this epic.
- **Gate definition.** A gate both hides the control on Android **and** neutralizes an already-stored value at config load. Each story spec names the normalization target per key. A stored "local" must never become a silent switch to cloud.

Cross-cutting rules carried into every story:

- **Twin rule.** Every shared-behaviour change lands in both the Rust and the Kotlin path, or is explicitly platform-gated. Each story states its platform reach.
- **Golden-vector net.** Guard and core-output changes get a fixture read by both a Rust and a JVM test; every new vector or guard passes an inversion check that is RED at writing time. The ADR-0017 boundary tripwire test stays green.
- **Verification symmetry.** Fifteen agent-only rows were flagged "build the reproducibility with it" (H+): such a story names, *before* build, how the product owner reproduces the state on his own device. Where no cheap path exists, the story records the downgrade to agent-verified-only explicitly — never implied. Story 13.1 exists to make four of those rows reproducible.
- **Surface DoD.** Rows that change what the user sees (pill, toast, settings) end with a Windows release build and/or a fresh APK on the real Android device.
- **Standing decisions untouched.** No auto-send on Android. Groq is never a cleanup fallback. The software license choice (BSL/PolyForm) and the publication question stay open. Nothing is built before its own go.

## Technical Decisions

- **The shared Rust core covers STT and license only.** Cleanup, chunking, VAD and LLM routing are Rust↔Kotlin twins. Before fixing a row, determine which of the two it is: shared means fix once, twinned means fix twice. STT request and guard logic must not reappear in Kotlin — a tripwire test fails on it.
- **Android bypasses Tauri IPC** (~85%): Kotlin talks to the JNI bridge and HTTP directly. This is the origin of nearly every row in this epic.
- **Config is the single source of truth** in `config.json`; gates act at config load, which is the only place a stored value can be neutralized. Both twins currently hardcode provider URLs, so a provider change is a two-sided edit.
- **Platform gating in the React settings uses the existing `isDesktop` / `isMobile` helpers** — no ad-hoc user-agent checks.
- **Test infrastructure:** golden vectors in `test-fixtures/`, read by `cargo test --lib` on the Rust side and the Android smoke script's JVM gate on the Kotlin side. No CI runs them. Kotlin compiles and unit-tests run device-free. A fixture-only edit needs a forced re-run; Gradle otherwise reports a stale green.
- **Desktop overlays are native Win32 layered windows painted with tiny-skia**, not WebView2 windows — relevant to the blur-slider wiring, which also needs an amendment note on the native-overlay ADR and an objective pixel metric before acceptance.
- **Cleanup defaults to DeepSeek; Groq serves STT.** A free tier without DeepSeek would force every free cleanup onto Groq, which is why the free-tier definition is DeepSeek + Groq.
- **No remote telemetry** (BYOK product promise): evidence for the H+ reproduction paths comes from local logs and the UI, never from a phone-home.
- **The generated Android tree is regenerated and gitignored** — durable Kotlin sources, resources and manifest edits live in the tracked `android/` tree only.

## UX & Interaction Patterns

- **The pill is a status light**, established by the preceding cleanup-failure story: it never shows a success check for a run that failed, and never claims text is in the clipboard when nothing is.
- **A failure is named, not swallowed.** Empty, truncated or malformed provider answers produce no paste and no empty history entry; a cleanup failure yields clipboard plus cause; "no focused field" yields a clipboard hint instead of a success check; "nothing recognized" is silent on both platforms.
- **A gated control is absent, not disabled-looking.** After the Android gates, the affected settings must not be findable in the Android settings UI at all.
- **The paywall states the truth.** Locked provider controls name the two free-tier providers, the paywall advertises only features that exist on the platform showing it, and a license decision is always visible (pill status or settings lock), never silent.
- **The debug test provider is invisible in normal use:** never a default, never part of any production fallback ladder, reachable on both devices without a computer attached.

## Cross-Story Dependencies

- **13.2 depends on 13.1** for its four H+ rows (empty / truncated / malformed LLM answer, empty STT result); without the debug provider those rows fall back to agent-verified-only. A clipboard-failure row is H+ only if the same mechanism can inject that failure, otherwise the downgrade is recorded.
- **13.5 depends on 13.4** for license semantics (no-restart activation, trial start).
- **13.3 and 13.4 overlap at the paywall lock text**: the free-tier providers named by the Android lock must match the single free-tier definition 13.4 establishes.
- **13.3's local-path gates are the precondition** for the "stored value" wording in 13.2 (offline rule) and 13.5 (license override exemption for local) — a hidden control still leaves a stored value behind.
- **13.6, 13.7, 13.8 and 13.9 are independent** of the sweeps and of each other. 13.6 (Desktop banking guard) and the Android guard-order fix inside 13.2 are two halves of the same audit row and must not both claim it.
- Suggested critical path: 13.1 → 13.2 → 13.3 → 13.4 → 13.5, then the four independent stories; sprint planning may reorder.
