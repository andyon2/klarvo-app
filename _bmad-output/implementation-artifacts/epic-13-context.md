# Epic 13 Context: Parity line over audit #2

<!-- Generated from planning artifacts. Regenerate with compile-epic-context if planning docs change. -->

## Goal

Klarvo ships one product across a Tauri/Rust desktop app and a native Kotlin Android app, and a second cross-platform drift audit found 78 rows where the two sides disagree: guards that delete text on one platform only, failures reported as success, controls that render but do nothing, a paywall enforced on Android and absent on Desktop, and an "offline" mode that still uploads audio. Every row now carries a product verdict (close / gate / port / fix / strike, or explicitly "stays an asymmetry"). Epic 13 executes the actionable verdicts in ten stories, so that a dictation is never silently lost, mangled or falsely confirmed; every visible control either acts or is gone; both local paths on Android stay hidden until a separate decision builds them; and the license means the same thing wherever Klarvo runs. Rows judged asymmetry, documentation-only, or "candidate without release" are out of scope by decision.

## Stories

- Story 13.1: Debug test provider, both twins
- Story 13.1b: Test provider operability
- Story 13.2: Parity sweep: guards and silent loss
- Story 13.3: Parity sweep: Android gates and config hygiene
- Story 13.4: Desktop license gate and one free tier
- Story 13.5: Parity sweep: license rows and sync hygiene
- Story 13.6: Desktop banking guard (process blocklist)
- Story 13.7: Android per-app profiles and profile language
- Story 13.8: Native preview blur wiring
- Story 13.9: OpenAI STT provider via Rust

## Requirements & Constraints

No PRD, architecture or UX document exists for this epic (L3 brownfield route). The binding inputs are the parity ADR's fourth amendment (the decision-complete row verdicts) and the drift audit holding the code evidence per row. Nothing here re-measures: every requirement keeps its audit row id (`D-*`) plus its decision-sheet row id, and a story spec cites those ids as its traceability anchors.

Four principle decisions bind every story:

- **License on both sides.** The desktop dictation pipeline enforces the license the way Android already does. Exactly **one** free-tier definition for both platforms: **DeepSeek + Groq**. Android widens from its current Groq-only definition.
- **"Offline" means no byte leaves the device.** Local STT implies local cleanup or no cleanup. Desktop-hotkey semantics are the norm; the Rust in-app button and Android follow.
- **Android hides both local paths.** Offline STT, local cleanup and the model manager go behind platform gates. Android has no local path until a separate, later decision builds one — explicitly not this epic.
- **Gate definition.** A gate hides the control on Android **and** neutralizes an already-stored value at config load. Each story spec names the normalization target per key; a stored "local" must never become a silent switch to cloud.

Rules carried into every story:

- **Platform reach is stated.** Every shared-behaviour change lands in both the Rust and the Kotlin path, or is explicitly platform-gated, and the story says which.
- **Golden-vector net.** Guard and core-output changes get a fixture read by both a Rust and a JVM test, and every new vector or guard passes an inversion check that is RED at writing time.
- **Verification symmetry.** Fifteen agent-only rows were flagged "build the reproducibility with it" (H+): such a story names, *before* build, how the product owner reproduces the state on his own device. Where no cheap path exists, the downgrade to agent-verified-only is written into the story, never implied. Reproducibility must also be *operable* — a test state the owner cannot reach unaided has not been made reproducible.
- **Surface DoD.** Rows that change what the user sees (pill, toast, settings) end with a Windows release build and/or a fresh APK on the real Android device.
- **Standing decisions untouched.** No auto-send on Android; Groq is never a cleanup fallback; the software-license choice and the publication question stay open; nothing is built before its own go.

## Technical Decisions

- **The shared Rust core covers STT and license only.** Cleanup, chunking, VAD and LLM routing are Rust↔Kotlin twins. Decide per row which it is: shared means fix once, twinned means fix twice. STT request and guard logic must never reappear in Kotlin — a boundary tripwire test fails on it, and the OpenAI STT switch therefore lands in the Rust STT path with Kotlin only passing the key.
- **Gates act at config load**, the only place a stored value can be neutralized; the visible half uses the existing platform helpers in the React settings, never an ad-hoc user-agent check.
- **Both twins hardcode provider URLs**, so adding a provider (test provider, OpenAI STT) is a two-sided edit.
- **Free-tier rationale:** cleanup is DeepSeek-primary by product decision, so a free tier without DeepSeek would force every free cleanup onto Groq — hence DeepSeek + Groq. Pin the free-tier set and the gate decision in fixtures read by both twins.
- **The test provider is the epic's H+ vehicle.** It returns canned answer shapes (ok / empty / truncated / malformed / 429 / 5xx / transport) on both platforms, is never part of a production fallback ladder, never a default, and invisible in the normal provider picker. It must be selectable on both devices without a computer attached — editing the config file is not sufficient for Android. It is named **`test`**, not `debug`, to avoid colliding with the log-level value; Rust twin, Kotlin twin and React must read the **same** config keys, an older `debug`-shaped config must migrate or be ignored without stranding the user on a dead provider, and the log line names the scenario that fired on both platforms.
- **The desktop preview overlay is a native Win32 layered window painted with tiny-skia**, not a WebView2 window: the blur value must reach the native painter, the native-overlay ADR gets an amendment note (blur was dropped there, now reintroduced natively), and acceptance needs an objective pixel metric.

## UX & Interaction Patterns

- **The pill is a status light.** It never shows a success check for a run that failed, and never claims text is in the clipboard when nothing is.
- **A failure is named, not swallowed.** Empty, truncated or malformed provider answers produce no paste and no empty history entry; a cleanup failure yields clipboard plus cause; "no focused field" yields a clipboard hint instead of a success check; "nothing recognized" stays silent on both platforms.
- **A gated control is absent, not merely inert** — after the Android gates the affected settings must not be findable in the Android settings UI at all.
- **The paywall states the truth.** Locked provider controls name the two free-tier providers, the paywall advertises only features that exist on the platform showing it, and a license decision is visible (pill status or settings lock), never silent.
- **A setting is one control and one save.** A half-configured state must be impossible by construction rather than avoided by care: one row per chain instead of a switch plus a dependent row, saved by exactly one button, and a dirty panel's save control fully visible at the bottom edge without scrolling on phone and desktop. Nothing may overlap it.
- **Surface work reuses shipped controls verbatim** (the existing select, the existing panel footer button); this epic designs no new control states.

## Cross-Story Dependencies

- **13.1b builds directly on 13.1's shipped enabler** and must land before 13.2: the owner drives the test provider in every later H+ check, so its operability is on the critical path, not polish. 13.1b changes 13.1's config shape, hence the migration requirement.
- **13.2 depends on 13.1/13.1b** for its H+ rows (empty / truncated / malformed LLM answer, empty STT result); without a usable test provider those rows fall back to agent-verified-only. The clipboard-failure row is H+ only if the same mechanism can inject that failure, otherwise the downgrade is recorded.
- **13.4 owns the license gate over the test provider**: on an unlicensed Android device the provider allowlist currently redirects it, so the enabler stays unreachable there until 13.4 lands. 13.3 may not touch that gate. **Corrected 2026-09-21 after 13.1b shipped:** 13.1b had to touch it, because its own shape change would otherwise have silently REMOVED it. Under 13.1 the gate neutralised the test provider for free, by rewriting `llmProvider` / `sttProvider` to `groq`; once selection moved to `advanced.testProvider*` those keys survived the rewrite. 13.1b therefore extracted the gate'"'"'s value decision into the pure `KlarvoApi.gateProvidersForLicense`, extended it to force both new keys to `off` when unlicensed, and widened the `[license]` log predicate so the line still fires. **Behaviour preserved, not changed — the gate DECISION (who is licensed, what the free tier is) remains entirely 13.4'"'"'s.** Also recorded by 13.1b, not fixed: the DESKTOP half of that gate lapsed in the same move (`commands/recording.rs::active_stt_provider_id` reads `cfg.stt_provider`, which no longer carries the test provider), so an unlicensed desktop user can run the test STT provider. Filed in `docs/backlog.md`; it is 13.4'"'"'s subject.
- **13.5 depends on 13.4** for license semantics (trial start, no-restart activation, the local-cleanup exemption).
- **13.3 and 13.4 meet at the paywall lock text**: the free-tier providers named by the Android lock must match the single free-tier definition 13.4 establishes.
- **13.3's local-path gates are the premise** for the "stored value" wording in 13.2 (offline rule) and 13.5 (license exemption for local) — a hidden control still leaves a stored value behind.
- **13.6 and 13.2 split one audit row**: the Desktop banking blocklist is 13.6, the Android ordering fix (history/sync write only after the guard's verdict) is 13.2; neither claims the other half.
- **13.6, 13.7, 13.8, 13.9 are otherwise independent.** Suggested critical path: 13.1 → 13.1b → 13.2 → 13.3 → 13.4 → 13.5, then the four independent stories; sprint planning may reorder.
