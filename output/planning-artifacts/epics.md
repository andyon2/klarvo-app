---
stepsCompleted: ['step-01-validate-prerequisites', 'step-02-design-epics', 'step-03-create-stories-epic-1A', 'step-03-create-stories-epic-1B', 'step-03-create-stories-epic-1C']
scopePhase: 'phase-1'
uxSpec: 'none'
uxSpecRationale: 'Phase-1 dogfooding-prototype; acceptedFriction: Kein Onboarding, config.toml-only Konfiguration, toleriert Rough-Edges. Minimal-UI-Elemente der Windows-Shell (Tray-Icon, Notifications) erhalten ACs direkt aus PRD-FRs in Shell-Adapter-Stories. Ref personaTiering.phase1Target in prd.md.'
inputDocuments:
  - output/planning-artifacts/product-brief-klarvo.md
  - output/planning-artifacts/product-brief-klarvo-distillate.md
  - output/planning-artifacts/architecture.md
  - output/planning-artifacts/prd.md
  - docs/index.md
  - docs/project-overview.md
  - docs/rebuild-discussion.md
  - docs/adr/README.md
  - docs/adr/0001-vad-provider-trait-signature.md
  - docs/adr/0002-tauri-specta-2-rc-acceptance.md
  - docs/adr/0003-jni-spike-outcome.md
  - docs/adr/0004-v1-to-v2-migration-strategy.md
  - docs/adr/0005-https-client-and-http-mock-stack.md
  - docs/migration/v1-to-v2.md
  - memory/MEMORY.md
  - memory/feedback_polished_designschwaeche.md
  - memory/project_i18n_three_axes.md
  - memory/project_i18n_core_contract.md
  - memory/project_api_key_os_keystore_mvp.md
  - memory/project_no_remote_telemetry.md
  - memory/project_market_positioning.md
  - memory/project_klarvo_v2_rebuild.md
  - memory/project_prd_phase1_in_progress.md
---

# klarvo - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for klarvo Phase 1 (dogfooding-prototype, thin-slice-walking-skeleton), decomposing the requirements from the PRD and Architecture requirements into implementable stories. UX-Spec abwesend per Phase-1-Persona-Tiering (Phase-1 `dogfooding-prototype`, rough-edges accepted).

**Phase-Boundary-Discipline:** Epics-Scope = PRD-Scope = strict Phase-1. Forward-References zu Phase 2/3/4 (z. B. FR40-telemetry-stub → Phase-2-UI-Expansion, Verbatim-only → Phase-2-Polished-Mode) leben als Inline-Notiz in den betreffenden Phase-1-Epics. Keine Phase-2/3-Platzhalter-Epics — Scope-Creep über Phasengrenze ist verboten.

## Requirements Inventory

### Functional Requirements

**A. Core Library & Plugin Composition**

- **FR1:** `klarvo-core` exposes the `SttProvider`-Trait for Speech-to-Text-Transkription of audio-streams.
- **FR2:** `klarvo-core` exposes the `CleanupStyle`-Trait for Post-Processing of transcribed text.
- **FR3:** `klarvo-core` exposes the `VadProvider`-Trait for Voice-Activity-Detection on audio-streams.
- **FR4:** `klarvo-core` exposes the `PipelineStage`-Base-Trait for all Pipeline-Stage-Implementations.
- **FR5:** `klarvo-core` parses Pipeline-Manifest-TOML files into resolved Pipeline-Definitions at Boot-Time.
- **FR6:** `klarvo-core`'s Pipeline-Executor erroriert zur Boot-Zeit mit hartem Fehler (`AppError::kind::PipelineValidation`) bei unbekannten Stage-Types oder Type-Mismatches zwischen Stages. `warn!+skip` ist verboten. Die Menge erlaubter Stage-Types wird zur Compile-Zeit durch Cargo-Features und `#[serde(tag = "type")]`-Enum-Variants bestimmt.
- **FR7:** `klarvo-core` exposes an Event-Bus-API emitting typed events via tauri-specta-generated bindings.
- **FR8:** `klarvo-core` emittiert Events und Errors ausschließlich als i18n-Keys, niemals als User-facing Strings (G3-enforced).
- **FR9:** `klarvo-core` exposes the `AppError`-Struktur mit `kind`-Enum, `user_message`-i18n-Key und Cause-Chain.
- **FR10:** Plugin-Crates (`klarvo-plugin-groq`, `klarvo-plugin-verbatim`) implementieren Core-Traits als Reference-Implementations.
- **FR11:** Pipeline-Manifest-TOML supportet Schema-Version-Header; Manifest-Parser validiert Schema-Version vor Type-Resolution.

**B. Audio Capture & Pipeline Execution (Canonical Reference-Workflow)**

- **FR12:** User can initiate audio-capture via Global-Hotkey (Hold-to-Talk).
- **FR13:** `klarvo-core` captures audio from Windows-Microphone-Input during Hotkey-Hold.
- **FR14:** `klarvo-core` runs VAD on captured audio-stream during capture.
- **FR15:** `klarvo-core` sendet captured audio an konfiguriertes STT-Plugin zur Transkription.
- **FR16:** `klarvo-core` passes STT-Output durch konfiguriertes CleanupStyle-Plugin.
- **FR17:** `klarvo-core` delivers final Cleanup-Output an Shell-Adapter für Paste-Injection.
- **FR18:** Pipeline-Execution is fully exercisable without Shell-Dependency (headless-capable).

**C. Windows Shell Adapter**

- **FR19:** Windows-Shell registriert Global-Hotkey aus `config.toml` (Default `CommandOrControl+Shift+Space`) via Win32-API.
- **FR20:** Windows-Shell displays Tray-Icon und swapped State zwischen Idle und Recording.
- **FR21:** Windows-Shell performs Auto-Paste in active Foreground-Window via Clipboard-Set + simuliertes `Ctrl+V`.
- **FR22:** Windows-Shell enforces Single-Instance-Lock via Named-Mutex; zweiter Startversuch terminiert mit Log-Entry (kein Popup), bestehende Instanz bleibt funktional.
- **FR23:** Windows-Shell handled MIC-Permission-denied als `AppError::kind::PermissionDenied` mit `user_message` `"error.permission.microphone"`.
- **FR24:** Windows-Shell consumiert ausschließlich tauri-specta-generierte TypeScript-Bindings; kein Ad-hoc-Core-Access.

**D. Configuration & Internationalization**

- **FR25:** System loads User-Configuration aus `config.toml` at startup; kein Settings-UI, kein Runtime-Config-Override außer Diagnostics-Flags.
- **FR26:** System supports drei unabhängige i18n-Achsen: UI-Language (Shell-Strings), Dictionary-Language (Plugin-Dictionary-Lookups), Output-Language (Cleanup-Target-Language).
- **FR27:** Shell resolves i18n-Keys (emittiert durch Core) gegen Shell-owned Translation-Tables (z. B. `de.json`).

**E. Error Handling & Failure Recovery**

- **FR28:** Pipeline-Strict-Error bei unknown-stage-type propagiert als `AppError::kind::PipelineValidation` mit actionable `user_message`-Key.
- **FR29:** Groq-API-Failures (5xx, Timeout, Network-Error) surfacen als `AppError::kind::UpstreamUnavailable` mit `user_message`-Key und gelogged Cause.
- **FR30:** Keystore-Miss bei Plugin-Init surfaced als `AppError::kind::KeyMissing` mit `user_message`-Key und Plugin-Identifier in Cause.
- **FR31:** Alle user-facing Errors werden als i18n-Keys emittiert; Shell resolves zu localized Strings at Display-Time.

**F. Developer Tooling & Gate-Enforcement**

- **FR32:** `cargo xtask manifest-strict` fails Build bei Pipeline-Manifest-Verletzungen (unknown oder type-incompatible stages).
- **FR33:** `cargo xtask bindings-drift` fails Build wenn Shell Core-APIs konsumiert, die nicht in tauri-specta-generierten Bindings existieren.
- **FR34:** `cargo xtask lint-events` (G3) fails Build bei literalen User-facing Strings in Core-emittierten Events/Errors.
- **FR35:** `cargo xtask verify-release` (G2) enforced Release-Hardening-Invarianten (Phase-0-etabliert).
- **FR36:** Publizierte Core-Traits und Core-APIs sind via rustdoc dokumentiert mit Intent + Contract-Conditions (Quality-Norm, kein CI-Gate Phase 1).

**G. Observability & Diagnostics**

- **FR37:** System schreibt structured Log-Einträge in Rolling-File-Log bei konfigurierbarer Verbosity.
- **FR38:** System transmittiert keine Telemetry, Errors oder Usage-Data an Remote-Endpoints.
- **FR39:** Uncaught Panics in `klarvo-core` werden als `level=ERROR` tracing-Events geloggt und landen im Rolling-File-Log (nicht Rust-Default-Stderr-Trace).
- **FR40:** `klarvo-core` exposes a `telemetry::export`-Module-Stub in Phase 1; full UI-triggered Zip-Generation (Debug-Export) is deferred to Phase 2.

**H. V1→V2 Data Migration**

- **FR41:** Andy can invoke v1→v2-Import via CLI-Subcommand (`cargo xtask import-v1` oder Äquivalent).
- **FR42:** v1→v2-Import reads v1-AppData (SQLite-History + `config.toml`) und writes v2-AppData-Layout preserving Dictation-History-Records.
- **FR43:** v1→v2-Import migriert WEDER API-Keys (Security-Hygiene — User re-enters) NOCH Polished-Mode-Settings (v2 baut Polished neu in späterer Phase).

**I. Security & Key Management**

- **FR44:** `klarvo-core` exposes a `KeyStore`-Abstraction-Trait für API-Key-Retrieval und -Storage, konsumiert von allen STT/LLM-Plugins at Init-Time.
- **FR45:** Phase-1-Default-Implementation ist `PlainSqliteKeyStore`, gated behind `dev-keystore`-Cargo-Feature; nicht enabled in Release-Builds.
- **FR46:** OS-Keystore-Implementations (Windows-Credential-Manager, macOS-Keychain, Linux-Secret-Service) sind als Scaffolds prepared für Phase-4-Release-Default-Swap, ohne `KeyStore`-Trait-Signature-Änderungen.

### NonFunctional Requirements

**Performance**

- **NFR1:** End-to-End-Latency Hotkey-Release → Clipboard-Paste wird im Rolling-File-Log mit `ts_ms` erfasst und als Observable für Dogfooding-Regression-Detection exposed (keine harte SLA-Grenze Phase 1).
- **NFR2:** Audio-Capture-Thread droppt keine Samples während Hold-to-Talk, unabhängig von Downstream-Processing-Latency.
- **NFR3:** Alle Core-Events nutzen session-relative monotone `ts_ms` (Caller-Clock; nicht Wall-Clock, nicht Sample-Count) — ref ADR-0001/0003.

**Security & Privacy**

- **NFR4:** Der `dev-plain-keystore`-Default ist explizit Security-Theater — eine Windows-ACL-Restriktion auf Current-User (Read/Write) mitigiert casual-access durch andere OS-User, schützt aber nicht gegen privileged-process-read oder Disk-Backup-Extraction. Echte API-Key-Protection kommt via OS-Keystore-Impl (Phase-4-Release-Default). Phase-1-Nutzer sind informiert, dass lokale Keys nicht als produktions-sicher behandelt werden sollen.
- **NFR5:** Audio-Daten und Transkriptions-Text werden NICHT im Rolling-File-Log persistiert; Log enthält nur Metadata (Event-Types, Error-Keys, Latency-Metrics).
- **NFR6:** System führt keine Outbound-Network-Calls außer zu user-konfigurierten Upstream-Providern (Groq, zukünftige LLM-APIs via BYOK). Kein Telemetry, kein Auto-Update-Check, keine Crash-Reports.

**Compliance**

- **NFR7:** Klarvo agiert NICHT als Daten-Processor für GDPR-Zwecke — User ist Controller für Upstream-Provider-Usage via eigener API-Key-Account. Kein Klarvo-Backend, keine Data-Processing-Agreements erforderlich.

**Compatibility & Integration**

- **NFR8:** Windows 10 und Windows 11 sind supported Target-OS Phase 1; Windows 7/8/8.1 sind non-goal.
- **NFR9:** STT-Provider-Kompatibilität ist Phase-1-limited auf Groq-Whisper-Cloud-API; Trait-Stability (`SttProvider`) ermöglicht Phase-2-Einhängung alternativer Provider (Deepgram, Azure, etc.) ohne Trait-Signature-Änderung.

**Reliability**

- **NFR10:** Runtime-Failures in plugin-dispatched Pipeline-Stages werden als `AppError` propagiert und im Rolling-File-Log erfasst; subsequent Hotkey-Triggers bleiben funktional. Plugin-Init-Failures zum Startup-Zeitpunkt werden vom Orchestrator als fatale Errors behandelt und führen zu kontrolliertem App-Beenden mit spezifischem Exit-Code (kein silent-crash).
- **NFR11:** Klarvo recovered graceful von Groq-API-Failures: User kann Hotkey erneut triggern nach Upstream-Error ohne App-Neustart.

### Additional Requirements

**Starter Template & Workspace Structure** (Phase-0 etabliert — Epic 1 Story 1 baut darauf auf, re-initialisiert NICHT):

- Custom Cargo Workspace (Resolver v3) + Tauri-Template für Windows-Shell + Android-Studio (post-Phase-3); kein Single-Starter deckt die Kombination ab, Scaffolding-Pfad mandatiert in `architecture.md § Selected Approach`.
- Workspace-Layout: `klarvo-core/`, `klarvo-plugins/<name>/`, `shells/windows/` (Tauri+React+TS+Vite+Tailwind), `xtask/` — bereits Phase-0-initialisiert.
- Rust 2024 Edition gepinnt via `rust-toolchain.toml`.
- Windows-Tauri-Identifier `de.klarvo.app` (v2) — getrennt vom v1-Identifier `com.klarvo.voice` (ref `memory/reference_klarvo_v1_tauri_identifier.md`).

**Plugin-Architecture Contracts:**

- **Phase-1-Trait-Stability-Set ist strict 4 Traits:** `SttProvider`, `CleanupStyle`, `VadProvider`, `PipelineStage`. Die übrigen architektonischen Traits (`LlmProvider`, `TextFilter`, `OutputTarget`, `AudioFilter`, `AudioSource`, `PluginMigration`, `VoiceCommandHandler`) sind Erweiterungsfläche, **nicht Phase-1-Stability-Anker** — ihre Signaturen dürfen Phase-2/3/4 evolvieren ohne Phase-1-Success-Violation. Kein Epic darf auf 8-Trait-Stability-ACs aufbauen (Scope-Creep-Gate). Ref PRD §Journey Requirements Summary R-C Amendment + §Success Criteria Anker 1.
- **Innovation-A-Mechanism ist zweischichtig (FR6/FR32-ACs müssen beide Layer benennen):**
  - *Compile-Time-Safety* = Stage-Registry-Set: Cargo-Features + `#[serde(tag = "type")]`-Enum-Variants schließen die akzeptierten Stage-Types ab. Das definiert, was der Executor *kennen kann*.
  - *Boot-Time-Parse* = Manifest-Content-Match: `pipeline-manifest.toml` wird via serde gegen die Stage-Registry geparst; Hard-Fail (`AppError::kind::PipelineValidation`) triggert zur Boot-Zeit bei unbekannten Stage-Types oder Type-Mismatches — nicht zur Compile-Zeit.
  - Ungrammatische Formulierungen wie „refuses to compile for unknown manifest stage" in Story-ACs sind ausdrücklich zu vermeiden. Ref `memory/project_manifest_boot_time_parse.md`, PRD §Innovation-A-Amendment.
- **Executor-Behavior ist hart-erroren, nie warn!+skip** bei unknown Stage-Types zur Runtime (Kern-Kontrakt, kein Polish). Das ist eine AC-Invariante für die Pipeline-Executor-Story. Ref `memory/feedback_manifest_compile_contract.md`.
- Pipeline-Manifest-TOML embedded via `include_str!("../../pipeline-manifest.toml")` zur Compile-Zeit; User-Override lebt optional in AppData-Dir (`%APPDATA%\de.klarvo.windows\pipeline.toml`). **Kein `read-from-working-dir`-Fallback** — bricht Deterministik-Kontrakt.
- Reference-Implementations Phase 1: `klarvo-plugin-groq` (SttProvider) + `klarvo-plugin-verbatim` (CleanupStyle).

**JNI-Bridge Dual-Surface** (Phase-1-relevant NUR für Core-Trait-Stability; Android-Shell ist Phase-3):

- JNI-Bridge ist dual-surface: `uniffi` (Control-Plane) + raw `jni` (Data-Plane/Streams). `uniffi` hat KEINE Stream-Support (ref `memory/project_jni_dual_surface.md`, ADR-0003).
- Phase-1-Mandat: Core-Traits müssen diese Split-Fähigkeit offen halten (Signature-Decisions blockieren sonst Phase 3).
- **Android-Scope-Fence (hart):** Die Android-Shell selbst ist Phase 3, gated durch AccessibilityService-Play-Store-Policy-Audit (ref `memory/project_play_store_phase3_blocker.md`). Falls beim Epic-Breakdown eine Android-Story entstehen sollte — **hartes Out-of-Scope**. Nur Trait-Signature-Compatibility-ACs (die Core-Traits dürfen keine Android-inkompatiblen Konstrukte einführen) sind Phase-1-zulässig.

**i18n Core/Shell-Separation** (G3-Gate enforced):

- Core hat NIE User-facing Strings; emittiert i18n-Keys. Shell übersetzt via Shell-owned Translation-Tables. G3 (`cargo xtask lint-events`) erzwingt das mechanisch — FR34.
- i18n-drei-Achsen-Model: UI-Language / Dictionary-Language / Output-Language; alle drei sind unabhängige Config-Felder.

**IPC-Boundary & Specta-Conventions:**

- Windows-Shell konsumiert ausschließlich `tauri-specta`-generierte TypeScript-Bindings (rc.24). FR24 + FR33 (bindings-drift-gate).
- Event-Naming: `#[tauri_specta(event_name = "<domain>.<event>")]`-Convention (G1-Gate, Phase-0-etabliert).
- Kein Ad-hoc-Core-Access aus Shell; Serialisierung passiert nur an IPC-Boundary.

**Keystore-Abstraction:**

- `KeyStore`-Trait ist Phase-1-stable; `PlainSqliteKeyStore` als Default gated hinter `dev-keystore`-Cargo-Feature (Security-Theater explizit, NFR4); OS-Keystore-Scaffolds (Windows-Credential-Manager) für Phase-4-Release-Default-Swap prepared — ohne Trait-Signature-Änderung.

**Audio Pipeline Integration:**

- Audio-Dataflow: `AudioSource → broadcast::Channel → VadProvider::gate → AudioFilter-Chain → SttProvider → CleanupStyle → OutputTarget → IPC-Event-Emission`.
- Internal Communication: `tokio::sync::broadcast` (Events, multi-consumer), `tokio::sync::mpsc` (Command-Queues). Direkte Funktionsaufrufe über Trait-Objekte im PluginRegistry.
- External: Groq Cloud-API via HTTPS (BYOK, Keys aus KeyStore); Windows `global-shortcut`-Crate für Hotkeys; Windows Credential Manager via `windows-rs` (Phase-4-Scaffold).

**Testing Infrastructure:**

- `klarvo-test-fixtures`-Crate für shared Rust-Fixtures; `test-assets/` (Root, cross-crate) für Binary-Fixtures.
- Audio-Fixtures > 1MB via Git LFS (`.gitattributes`).
- Unit-Tests: `#[cfg(test)]` co-located; Integration-Tests: `<crate>/tests/*.rs`; Snapshot-Tests: `insta`, `.snap`-Files per-crate-owned.
- Phase-1-Mandat: jede Core-Story-AC enthält „läuft in headless integration test ohne Shell" (PRD-Innovation-Axis-B).

**CI-Pipelines (Phase-0 etabliert, Phase-1 extends):**

- `.github/workflows/`: `ci-core.yml`, `ci-feature-lint.yml`, `ci-event-lint.yml`, `ci-bindings-drift.yml`, `release.yml` (Release-Hardening-Gate).
- Phase-1-neue Gates erweitern `.github/workflows/`; existing Gates werden NICHT rewritten.

**V1→V2 Migration Contract:**

- v1-AppData-Pfade: v1-Windows-Tauri-Identifier `com.klarvo.voice` (bereits verifiziert, `memory/reference_klarvo_v1_tauri_identifier.md`).
- v2-Import-Modul: `v1_import` (Phase-0-vorhandener parse-only Bundle, commits `aefa1aa` + `7346af4`); Phase-1-Erweiterung: actual write-to-v2-AppData.
- ADR-0004 definiert Migration-Strategy (ref).

**Development Workflow Mandates:**

- Bei Unsicherheit: ADR in `docs/adr/` schreiben statt ad-hoc entscheiden (ref `reference_adr_directory.md`).
- Plugin-Crates via `cargo xtask new-plugin <name>` (wenn Phase-0-Utility verfügbar); manuelle Kopie nur in Ausnahme.
- Reference-Block (architecture.md § Reference-Block) ist Pflichtlektüre vor erstem Phase-1-Code-Commit.

### UX Design Requirements

**Keine UX Design Requirements für Phase 1.**

Begründung: UX-Spec ist nicht Phase-1-Scope. Phase-1-Persona ist `dogfooding-prototype` (Andy + 1-2 interne Sanity-Tester) mit `acceptedFriction: 'Kein Onboarding, config.toml-only Konfiguration, toleriert Rough-Edges'`. Die einzigen UI-Touchpoints in Phase 1 sind:

- Windows Tray-Icon (Idle/Recording-State-Swap, FR20)
- System-Notifications für Errors (implizit via Shell-Adapter, FR31)

Diese erhalten Acceptance-Criteria direkt aus den PRD-FRs in den betreffenden Shell-Adapter-Stories — keine Shadow-UX-Extraction nötig. UX-Spec-Arbeit ist für Phase 2/3 (Pill-Bar/Bubble-UX, Onboarding-Flow, Settings-UI) explizit deferred gemäß `personaTiering.phase2And3Target.requirements`.

### FR Coverage Map

| FR | Epic | Notes |
|---|---|---|
| FR1 | 1A | `SttProvider`-Trait-Signature |
| FR2 | 1A | `CleanupStyle`-Trait-Signature |
| FR3 | 1A | `VadProvider`-Trait-Signature |
| FR4 | 1A | `PipelineStage`-Base-Trait-Signature |
| FR5 | 1B | Pipeline-Manifest-TOML-Parser (Boot-Time) |
| FR6 | 1B | Pipeline-Executor strict-fail (beide Layer: Compile-Time-Registry + Boot-Time-Match) |
| FR7 | 1A | Event-Bus-API (Trait-Signatures via tauri-specta) |
| FR8 | 1A | i18n-Key-only Events/Errors (G3-enforced) |
| FR9 | 1A | `AppError`-Struktur (kind + user_message-Key + Cause) |
| FR10 | 1B | Reference-Plugin-Crates `klarvo-plugin-groq` + `klarvo-plugin-verbatim` |
| FR11 | 1B | Schema-Version-Header + Validation |
| FR12 | 2 | Hotkey-initiated Audio-Capture |
| FR13 | 2 | Windows-Microphone-Capture während Hotkey-Hold |
| FR14 | 2 | VAD während Capture |
| FR15 | 2 | STT-Plugin-Dispatch |
| FR16 | 2 | CleanupStyle-Plugin-Dispatch |
| FR17 | 2 | Cleanup-Output → Shell-Adapter-Delivery |
| FR18 | 1A | Headless-Test-Infrastructure (Pipeline exercisable ohne Shell) |
| FR19 | 3 | Global-Hotkey-Registration via Win32 |
| FR20 | 3 | Tray-Icon State-Swap (Idle/Recording) |
| FR21 | 3 | Auto-Paste via Clipboard + Ctrl+V |
| FR22 | 3 | Single-Instance-Lock (Named-Mutex) |
| FR23 | 3 | MIC-Permission-denied → AppError::PermissionDenied |
| FR24 | 3 | Bindings-only Core-Access (tauri-specta) |
| FR25 | 4 | config.toml-only Configuration |
| FR26 | 4 | 3-Achsen-i18n-Model (UI/Dictionary/Output) |
| FR27 | 4 | Shell Translation-Tables (de.json, etc.) |
| FR28 | 4 | AppError::PipelineValidation für unknown stage-type |
| FR29 | 2 | Groq-API-Failure-Recovery (Runtime-Pipeline-Behavior) |
| FR30 | 4 | AppError::KeyMissing mit Plugin-Identifier in Cause |
| FR31 | 4 | i18n-keyed User-Errors Shell-resolved at Display-Time |
| FR32 | 5 | `cargo xtask manifest-strict` Gate |
| FR33 | 5 | `cargo xtask bindings-drift` Gate |
| FR34 | 5 | `cargo xtask lint-events` G3-Gate |
| FR35 | 5 | `cargo xtask verify-release` G2-Gate |
| FR36 | 1A | Rustdoc-Contract-Documentation (Quality-Norm) |
| FR37 | 6 | Structured Rolling-File-Log |
| FR38 | 6 | No-Remote-Telemetry-Enforcement |
| FR39 | 6 | Uncaught-Panic-Capture als tracing-ERROR |
| FR40 | 6 | `telemetry::export`-Module-Stub (UI-Triggered-Zip → Phase 2) |
| FR41 | 7 | CLI-Invocation `cargo xtask import-v1` |
| FR42 | 7 | v1-AppData-Read + v2-Layout-Write (History preserving) |
| FR43 | 7 | Exclude-Policy (kein API-Key, kein Polished) |
| FR44 | 1C | `KeyStore`-Abstraction-Trait |
| FR45 | 1C | `PlainSqliteKeyStore` Phase-1-Default (dev-keystore-Feature) |
| FR46 | 1C | OS-Keystore-Scaffolds (Windows-Credential-Manager etc.) |

**Coverage: 46/46 ✓**

**NFR-Distribution (Invarianten über Epics):**
- NFR1 (Latency-Observable) → Epic 2 + Epic 6
- NFR2 (Drop-freier Audio-Capture) → Epic 2
- NFR3 (ts_ms-Convention) → Epic 1A (Event-Bus-Struct-Definition) + Epic 2 (Caller-Clock-Usage)
- NFR4 (Plain-Keystore-Disclosure) → Epic 1C
- NFR5 (kein Audio/Text im Log) → Epic 6
- NFR6 (no Outbound-Calls außer BYOK) → Epic 6 + Epic 2 (Groq-Upstream-Scope)
- NFR7 (GDPR-No-Processor-Positioning) → dokumentarisch (kein eigener Epic-Scope)
- NFR8-9 (Win 10/11, Groq-only-Phase-1) → Scope-Boundary (global invariant)
- NFR10 (Runtime-vs-Init-Failure-Discipline) → Epic 1B (Executor) + Epic 1A (AppError-Shape)
- NFR11 (Upstream-Recovery-ohne-Restart) → Epic 2

## Epic List

### Epic 1A: Plugin-Contract Surface

Plugin-Developer schreibt gegen stabile Trait-Signatures, Types checken, Compile geht durch — ohne dass die Pipeline-Runtime existieren muss. Foundation-Layer für alle Plugin-Author-Arbeit.

**FRs covered:** FR1, FR2, FR3, FR4, FR7, FR8, FR9, FR18, FR36 — 9 FRs

**Dependencies:** Keine (nur Phase-0-Workspace-Foundation).

**Implementation Notes:**
- 4-Trait-Stability-Discipline: nur `SttProvider`, `CleanupStyle`, `VadProvider`, `PipelineStage` sind Phase-1-Anker. Die architektonischen Traits (`LlmProvider`, `TextFilter`, `OutputTarget`, `AudioFilter`, `AudioSource`, `PluginMigration`, `VoiceCommandHandler`) sind Erweiterungsfläche, nicht Phase-1-Signature-Stability-Anker.
- Event-Bus-Structs via `#[tauri_specta(event_name = "<domain>.<event>")]`-Convention (G1-Gate). Wire-Name-Contract wird mit Epic 5 FR34 mechanisch enforced.
- i18n-Key-only-Discipline: Core emittiert nur Keys, NIE User-Strings (G3). Konsumenten-Test in Epic 1A gegen Mock-Translation-Lookup.
- `klarvo-test-fixtures`-Crate als shared Test-Harness. Jede Story-AC enthält „läuft in headless integration test ohne Shell".
- Phase-0 hat Workspace + xtask-Skeleton bereits — Epic-1A Story 1 ist **nicht** „initialize workspace", sondern erste konkrete Trait-Signature-Publikation.

**Stories-Sequencing:** 1A.1 test-fixtures → 1A.2 AppError → 1A.3 Event-Bus → 1A.4 i18n-Contract → 1A.5 PipelineStage Base → 1A.6 STT+Cleanup (text-domain pair) → 1A.7 VAD (per ADR-0001).

#### Story 1A.1: `klarvo-test-fixtures`-Crate Scaffold + Headless-Test-Harness

As a Plugin-Developer,
I want a shared `klarvo-test-fixtures`-Crate with primitive Mock-Helpers and Headless-Test-Environment-Setup,
So that subsequent Epic-1A/1B/1C/2-Stories ihre ACs mit „läuft in headless integration test ohne Shell" consistent formulieren können.

**Acceptance Criteria:**

**Given** the Cargo-Workspace (Phase-0-initialisiert),
**When** ein neuer Workspace-Member `klarvo-test-fixtures/` mit einem initialen `Cargo.toml` als Library-Crate deklariert wird,
**Then** `cargo check -p klarvo-test-fixtures` passt ohne Warnings durch,
**And** Crate-Version ist `0.0.1` (pre-stability-marker per Trait-Stability-Discipline).

**Given** den Test-Fixtures-Public-API-Scope,
**When** die Primitive-Surface definiert wird,
**Then** exposiert sie mindestens: (a) `FakeClock` — session-relative monotone `ts_ms`-Generator per NFR3, (b) `NoNetworkGuard` — Assertion-Helper der bei Outbound-Socket-Attempt panict (erzwingt NFR6 in Tests), (c) `HeadlessTestEnv`-Struct als Context-Holder für später additive hinzugefügte Mocks (1A.2-1A.7 extenden additive).

**Given** dass 1A.2-1A.7 additive Extensions in dieses Crate schreiben (AppError-Roundtrip-Harness, Event-Bus-Harness, Trait-Mocks),
**When** eine spätere Story ihr Mock-Helper hinzufügt,
**Then** ist das ein neuer Module im Crate ohne API-Break der in 1A.1 definierten Primitives — strikte additive Extension.

**Given** `test-assets/`-Directory-Convention für Binary-Fixtures,
**When** 1A.1 shippt,
**Then** ist `.gitattributes` im Workspace-Root konfiguriert mit `*.wav filter=lfs diff=lfs merge=lfs -text` (Audio > 1MB via Git LFS per Additional-Req §Testing-Infrastructure), **And** `test-assets/`-Directory existiert als Placeholder mit einem README, das die Konvention dokumentiert.

**Given** FR18 (Pipeline-Execution headless-capable),
**When** ein Dev das Crate mit `cargo test -p klarvo-test-fixtures` ausführt,
**Then** alle Tests passen ohne GUI-Dependency, ohne Audio-Device-Access und ohne Network-Access — **And** Test-Runtime unter 5 Sekunden total (CI-Baseline).

---

#### Story 1A.2: `AppError`-Struktur mit `kind`-Enum + i18n-Key + Cause-Chain

As a Core-Developer or Plugin-Developer,
I want a canonical `AppError`-struct mit `kind`-Enum, i18n-key-valued `user_message` und Cause-Chain,
So that alle Error-Paths in `klarvo-core` und Plugins eine einheitliche Shape haben und i18n-Resolution zur Shell delegiert wird.

**Acceptance Criteria:**

**Given** `klarvo-core`'s `error`-module,
**When** `AppError` und `ErrorKind` definiert werden,
**Then** hat `AppError` die Felder (a) `kind: ErrorKind` (exhaustive-Enum mit initialen Variants `PipelineValidation`, `UpstreamUnavailable`, `KeyMissing`, `PermissionDenied`, `Io`, `Configuration` — erweiterbar), (b) `user_message: Option<String>` (i18n-Key per architecture.md:632, Comment dokumentiert Key-Semantik), (c) `source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>` für Cause-Chain.

**Given** ein Core-Code-Path der `AppError` konstruiert,
**When** `AppError::new(ErrorKind::PipelineValidation).with_message("error.pipeline.unknown_stage")` aufgerufen wird,
**Then** enthält `user_message` den dot-notation-i18n-Key (nicht einen übersetzten User-String per FR8/NFR-G3); Enforcement ist Runtime-Lint via 1A.4 und statisch via FR34 in Epic 5, nicht Type-System.

**Given** Cause-Chain-Semantik,
**When** `AppError` aus einem `std::io::Error` via `AppError::from_io(e)` erzeugt und dann via `std::error::Error::source()` traversiert wird,
**Then** ist die Source-Kette vollständig erreichbar, **And** `Display`-Impl renders `kind` + Key + optionale Source-Description als ein-zeiliges debug-friendly Format.

**Given** FR36 (Rustdoc-Contract-Documentation),
**When** `AppError`, `ErrorKind` und `AppError::new`/`with_message`/`from_io`-Helper exposed werden,
**Then** hat jedes Item Rustdoc mit (a) Intent, (b) Contract-Condition (z.B. „`user_message` muss ein i18n-Key in dot-notation sein"), (c) Beispiel-Code-Snippet.

**Given** 1A.1 test-fixtures-Harness,
**When** 1A.2 shippt,
**Then** extended `klarvo-test-fixtures` einen `error_roundtrip_harness`-Helper, der ein `AppError` mit Cause-Chain roundtrippt und asserted: Cause-Preservation, Key-Format-Conformance (delegates an 1A.4's Assertion-Helper).

---

#### Story 1A.3: Event-Bus-API + tauri-specta-Event-Struct-Convention

As a Core-Developer,
I want eine typed Event-Bus-API, die Events via `tokio::sync::broadcast` an Konsumenten verteilt und via tauri-specta-generierte TypeScript-Bindings zur Shell exportiert,
So that Shell + künftige Bindings-Konsumenten compile-time-typed Event-Definitions erhalten und die i18n-Key-only-Discipline über das gesamte Event-Spektrum gilt.

**Acceptance Criteria:**

**Given** `klarvo-core`'s `event`-module,
**When** die Event-Bus-API definiert wird,
**Then** verwendet sie `tokio::sync::broadcast` (Multi-Consumer, per architecture.md:1263) und exposiert ein typed Event-Enum mit Initial-Variants: `RecordingStarted`, `RecordingStopped`, `PipelineStageStarted`, `PipelineStageCompleted`, `ErrorEmitted` — alle Variants carrying nur i18n-key-typed Payloads + `ts_ms`-Field (keine User-Strings per FR8).

**Given** tauri-specta rc.24-Integration,
**When** Event-Structs definiert werden,
**Then** trägt jedes Struct ein `#[tauri_specta(event_name = "<domain>.<event>")]`-Attribut (per `reference_tauri_specta_rc24_event_name.md`, **nicht** `#[specta(rename)]`), **And** der Wire-Name folgt dot-notation (z.B. `recording.started`, `pipeline.stage_completed`), **And** Default-Kebab-Case vom Struct-Ident wird explizit überschrieben wo die Dot-Notation-Convention abweicht.

**Given** NFR3 (ts_ms = session-relative monotone Caller-Clock),
**When** ein Event konstruiert wird,
**Then** enthält es ein `ts_ms: u64`-Feld, das via `Clock`-Abstraktion bezogen wird (Trait-Seam, production-Impl ist `MonotonicClock`, Test-Impl ist 1A.1's `FakeClock`).

**Given** Object-Safety + Broadcast-Channel-Shape,
**When** der `EventBus` als `pub struct EventBus { tx: broadcast::Sender<Event> }` exposed wird,
**Then** sind `EventBus::new(capacity: usize)` und `EventBus::subscribe() -> broadcast::Receiver<Event>` die einzigen Konstruktion/Konsumtion-APIs; kein global-state, kein singleton.

**Given** FR36,
**When** EventBus + Event-Enum + Variants exposed werden,
**Then** hat jedes Public-Item Rustdoc mit Emission-Trigger („wann wird dieser Event emittiert?") + Payload-Semantics + Downstream-Consumer-Hint (z.B. „Shell-Tray-Icon konsumiert `RecordingStarted`/`RecordingStopped` für State-Swap").

**Given** 1A.1 test-fixtures,
**When** 1A.3 shippt,
**Then** extended `klarvo-test-fixtures` einen `event_bus_harness`-Helper mit `assert_emitted!(bus, Event::RecordingStarted{..})` und `assert_emitted_sequence!(bus, [..])`-Macros für ordered Event-Assertion-Patterns.

---

#### Story 1A.4: i18n-Key-Only-Contract (Runtime-Assertion-Primitive)

As a Core-Developer,
I want ein Runtime-Assertion-Helper + Key-Format-Contract in `klarvo-core::i18n`,
So that Core-emittierte Events/Errors bereits vor der CI-Gate (`cargo xtask lint-events` FR34 / Epic 5) lokal testbar auf i18n-Key-Format validiert werden und die Key-Format-Regex als Single-Source-of-Truth vom xtask-Tool importiert wird.

**Acceptance Criteria:**

**Given** `klarvo-core::i18n`-Module,
**When** `KEY_REGEX: &str` als `pub const` exposed wird,
**Then** ist der Wert `r"^[a-z][a-z0-9_]*(\.[a-z0-9_]+)+$"` (dot-notation, feature-namespaced, ASCII-lowercase), **And** begleitet von `pub fn assert_is_key(s: &str)` die panict bei Mismatch mit diagnostic-Message das den Input-Value enthält.

**Given** den Assertion-Helper und die Format-Contract,
**When** ein i18n-Key-Value (z.B. `user_message` oder ein Event-Key-Field) dem Helper übergeben wird,
**Then** passt er für valide Keys wie `"error.pipeline.unknown_stage"` oder `"recording.started"` durch, **And** panict bei invaliden Values: Whitespace (`"error pipeline"`), Non-ASCII (`"fehler.löschen"`), Punctuation außer Dot (`"error,key"`), Empty-String, Uppercase (`"Error.Key"`).

**Given** FR34-xtask-lint-events-Gate (Epic 5),
**When** das xtask-Tool die statische Analyse macht,
**Then** importiert es `klarvo_core::i18n::KEY_REGEX` direkt (z.B. via `[build-dependencies]` oder via Workspace-Crate-Path), **And** keine Format-Regex-Duplikation zwischen Runtime-Assertion und Static-Linter — Single-Source-Contract.

**Given** 1A.2 (AppError) und 1A.3 (Event-Bus),
**When** ihre Test-Harnesses i18n-Keys validieren müssen,
**Then** verwenden sie `klarvo_core::i18n::assert_is_key` als Runtime-Primitive — additive extension, keine 1A.2/1A.3-Signature-Änderung.

**Given** NFR5 (kein Audio/Text im Log) + NFR4 (Security-Hygiene),
**When** Assertion-Failures passieren,
**Then** enthält Panic-Message nur den Key-Value (Keys sind strukturell public) und keine umgebende Payload-Daten (z.B. kein Audio-Buffer-Debug, kein API-Key-Context).

**Given** FR36,
**When** `i18n`-Module exposed wird,
**Then** rustdoc dokumentiert: Key-Format-Contract, Assertion-Helper-Usage, Kebab-Case-vs-Snake-Case-Decision (snake_case chosen), Cross-Reference zu FR34 (Epic 5) und NFR G3.

---

#### Story 1A.5: `PipelineStage`-Base-Trait + Rustdoc

As a Plugin-Developer,
I want ein `PipelineStage`-Base-Trait in `klarvo-core`, das die gemeinsame Composition-Shape für alle Stage-Implementations (STT, Cleanup, VAD) definiert und mit dem Executor (Epic 1B) polymorphisch dispatchbar ist,
So that SttProvider/CleanupStyle/VadProvider auf einem einheitlichen Contract aufsetzen und das Innovation-A-Compile-Time-Stage-Registry-Muster (FR6) funktioniert.

**Acceptance Criteria:**

**Given** `klarvo-core::pipeline::stage`-Module,
**When** `PipelineStage` definiert wird,
**Then** ist es ein `#[async_trait]` Trait (per architecture.md:235, Object-Safety via `Box<dyn PipelineStage>`) mit (a) Associated-Types `type Input: Send;` und `type Output: Send;`, (b) Method `async fn process(&self, input: Self::Input) -> Result<Self::Output, AppError>`, (c) Method `fn stage_type(&self) -> &'static str` als Discriminator für `#[serde(tag = "type")]`-Enum-Mapping (FR6-Compile-Time-Layer).

**Given** Object-Safety-Requirement (`Box<dyn PipelineStage>` muss compilen),
**When** das Trait gepackt wird,
**Then** enthält es: keine `Self`-returning-Methods, keine Generic-Method-Parameters ohne `dyn`-Marker, keine `impl Trait`-in-Return-Position; **And** Compile-Test in test-fixtures: `fn _obj_safe(x: Box<dyn PipelineStage<Input=(), Output=()>>) {}` compiliert.

**Given** FR6-Compile-Time-Layer,
**When** 1A.5 shippt,
**Then** wird zusätzlich eine `PipelineStageType`-Enum mit `#[serde(tag = "type", rename_all = "kebab-case")]` skelettiert (Variants werden von 1A.6/1A.7/1B additive ergänzt); **And** der Compile-Time-Registry-Mechanism ist via Cargo-Feature-Gates vorbereitet (ein Platzhalter-Feature `stage-passthrough` aktiviert einen No-Op-Stage-Variant), **And** 1A.5's rustdoc dokumentiert das Innovation-A-Zweischicht-Modell explizit: *Compile-Time* = Stage-Registry-Set via Features + Enum, *Boot-Time* = Manifest-Match gegen Registry (Details in Epic 1B).

**Given** Phase-1-Stability per 4-Trait-Set,
**When** `PipelineStage`-Signature commited wird,
**Then** ist die Signature **Phase-1-stable**; nachträgliche Trait-Änderung triggert Breaking-Change-Review.

**Given** FR36,
**When** `PipelineStage` exposed wird,
**Then** hat es rustdoc mit Intent (base composition shape for all pipeline stages), Contract (was erwartet Executor vom Impl: Idempotency-Erwartung, Error-Propagation-Semantics, Ordering-Constraints), Minimal-Impl-Example (eine 10-Zeilen-Passthrough-Stage), Cross-References zu 1A.6/1A.7 und Epic 1B.

**Given** 1A.1 test-fixtures,
**When** 1A.5 shippt,
**Then** extended test-fixtures `MockPipelineStage<I, O>`-generic-Helper (configurable Return-Values + optional Error-Injection) + `harness_run_stage(stage, input)`-Function für headless Stage-Execution-Verification.

---

#### Story 1A.6: `SttProvider` + `CleanupStyle`-Trait-Signatures + Rustdoc (Text-Domain-Pair)

As a Plugin-Developer,
I want `SttProvider`- und `CleanupStyle`-Trait-Signatures in `klarvo-core`, die beide auf `PipelineStage` (1A.5) aufsetzen und Text-Domain-Transformationen definieren (Audio-Bytes → Text, Text → Text),
So that ich Text-Domain-Plugins (z.B. `klarvo-plugin-groq` für STT, `klarvo-plugin-verbatim` für Cleanup) gegen ein stabiles Contract schreiben kann.

**Acceptance Criteria:**

**Given** `klarvo-core::stt`-Module und 1A.5 `PipelineStage`,
**When** `SttProvider` definiert wird,
**Then** ist es ein `#[async_trait]`-Trait mit Supertrait-Bound `: PipelineStage<Input = AudioBuffer, Output = String>`, **And** `AudioBuffer` ist ein Struct `pub struct AudioBuffer { pub samples: Vec<f32>, pub sample_rate: u32, pub channels: u8 }` (pub Fields für Zero-Cost-Serde-Roundtrip in test-fixtures), **And** eine primäre Method `async fn transcribe(&self, audio: AudioBuffer) -> Result<String, AppError>` mit Default-Impl die auf `PipelineStage::process` delegiert.

**Given** `klarvo-core::cleanup`-Module und 1A.5 `PipelineStage`,
**When** `CleanupStyle` definiert wird,
**Then** ist es ein `#[async_trait]`-Trait mit Supertrait-Bound `: PipelineStage<Input = CleanupInput, Output = String>`, **And** `CleanupInput` ist `pub struct CleanupInput { pub raw: String, pub context: CleanupContext }`, **And** `CleanupContext` captures Output-Language-Achse (i18n-Axis-3 per `project_i18n_three_axes`) als `pub output_language: String` (BCP-47-Tag) + optional `pub dictionary_refs: Vec<String>` (Plugin-Keys), **And** eine primäre Method `async fn apply(&self, input: CleanupInput) -> Result<String, AppError>`.

**Given** Object-Safety für beide Traits,
**When** `Box<dyn SttProvider>` und `Box<dyn CleanupStyle>` gepackt werden,
**Then** Compile-Tests in test-fixtures passen (`fn _obj_safe_stt(x: Box<dyn SttProvider>) {}` + analog für CleanupStyle).

**Given** Phase-1-Stability per 4-Trait-Set,
**When** beide Trait-Signatures committed werden,
**Then** sind sie Phase-1-stable; Änderungen post-Close triggern Breaking-Change-Review; **And** rustdoc markiert `#[must_use]` wo passend + dokumentiert Stability-Garantie explizit.

**Given** FR36,
**When** beide Traits exposed werden,
**Then** hat jeder Trait rustdoc mit: Intent + Method-Contract (Input/Output-Preconditions, Error-Varianten-Erwartung) + Mock-Impl-Example + Cross-Reference zu `klarvo-plugin-groq` (Epic 1B Reference-Impl) bzw. `klarvo-plugin-verbatim`.

**Given** 1A.1 test-fixtures,
**When** 1A.6 shippt,
**Then** extended test-fixtures (a) `MockSttProvider { canned_transcriptions: Vec<String> }` mit `async fn transcribe()` das sequenziell aus der Queue returned, (b) `MockCleanupStyle { mode: MockCleanupMode }` mit Modes `Identity`/`UpperCase`/`ErrorInject`, (c) Helpers `assert_stt_call_count!(mock, n)` und `assert_cleanup_input!(mock, expected)`.

---

#### Story 1A.7: `VadProvider`-Trait-Signature + Rustdoc

As a Plugin-Developer,
I want ein `VadProvider`-Trait-Signature in `klarvo-core`, der Voice-Activity-Detection per ADR-0001-Signature-Entscheidung definiert,
So that VAD-Implementations (RMS-based in Epic 2, später WebRTC/Silero-Plugins) einheitlich gegen das Trait coden können und micro-latency-sensitive Evaluation ohne Async-Overhead arbeitet.

**Acceptance Criteria:**

**Given** `klarvo-core::vad`-Module und ADR-0001 (`docs/adr/0001-vad-provider-trait-signature.md`),
**When** `VadProvider` definiert wird,
**Then** ist die Signature 1:1 zur ADR-0001-Decision: (a) Input-Type `AudioFrame { pub samples: Vec<f32>, pub sample_rate: u32 }` repräsentiert ein 10-30ms-Window, (b) Output-Type `pub enum VadVerdict { Speech, Silence, Uncertain }`, (c) **Sync** Method `fn evaluate(&mut self, frame: AudioFrame) -> VadVerdict` (CPU-bound, micro-latency-sensitive — kein async).

**Given** dass architecture.md:234 vermerkt „VAD bleibt Core-intern",
**When** die Beziehung zu `PipelineStage`-Base (1A.5) entschieden wird,
**Then** ist `VadProvider` **kein** `PipelineStage`-Subtrait (sync-vs-async-Mismatch + Core-internal-Nature) — es ist ein eigenständiges Core-Interface; **And** rustdoc dokumentiert diese Design-Decision mit Link zu ADR-0001 + architecture.md:234.

**Given** Object-Safety-Requirement,
**When** `Box<dyn VadProvider>` gepackt wird,
**Then** passt Compile-Test in test-fixtures; falls `&mut self` Object-Safety blockiert, dokumentiert rustdoc das alternative Accessmuster (z.B. `Arc<Mutex<dyn VadProvider>>` oder Owned-Move-Pattern).

**Given** Phase-1-Stability per 4-Trait-Set,
**When** `VadProvider`-Signature committet wird,
**Then** ist die Signature Phase-1-stable per ADR-0001-Contract.

**Given** FR36,
**When** `VadProvider` exposed wird,
**Then** hat rustdoc Intent + Contract (Frame-Size-Erwartung, Return-Semantik, State-Machine-Hint für `&mut self`) + Minimal-Impl-Example + Link zu ADR-0001 + Epic-2-Forward-Reference für RMS-Impl.

**Given** 1A.1 test-fixtures,
**When** 1A.7 shippt,
**Then** extended test-fixtures (a) `MockVadProvider` mit configurable Verdict-Sequence (z.B. `[Speech, Speech, Silence, Silence, Speech]`), (b) `RmsVadReference`-Scaffold (minimale Signature-Impl ohne RMS-Algorithm-Body — Epic-2-RMS-Logic wird dort **nicht** implementiert, nur Scaffold für Epic-2-Story-Referenz).

---

### Epic 1B: Pipeline-Composition Runtime

Plugin-Developer bindet seine Crate via Manifest ein und führt sie end-to-end aus. Pipeline-Manifest-Parser + Executor + Reference-Plugins machen die 1A-Traits von *signature-only* zu *runnable*.

**FRs covered:** FR5, FR6, FR10, FR11 — 4 FRs

**Dependencies:** Epic 1A (Trait-Signatures + AppError + Event-Bus).

**Implementation Notes:**
- **Innovation-A-Mechanism zweischichtig (FR6-Story-ACs müssen beide Layer sauber benennen):**
  - *Compile-Time-Safety* = Stage-Registry-Set via Cargo-Features + `#[serde(tag = "type")]`-Enum-Variants. Das schließt die akzeptierten Stage-Types compile-time ab.
  - *Boot-Time-Parse* = `pipeline-manifest.toml` wird via serde gegen die Registry geparst; Hard-Fail (`AppError::kind::PipelineValidation`) triggert zur Boot-Zeit bei unbekannten Stage-Types oder Type-Mismatches.
  - Ungrammatische Formulierungen wie „refuses to compile for unknown manifest stage" in ACs ausdrücklich vermeiden.
- **Executor-Behavior AC-Invariante:** hart-erroren bei unknown Stage-Types zur Runtime, nie `warn!+skip`. Kern-Kontrakt, kein Polish.
- Manifest embedded via `include_str!("../../pipeline-manifest.toml")` — kein `read-from-working-dir`-Fallback.
- Reference-Plugins `klarvo-plugin-groq` (SttProvider-Impl) + `klarvo-plugin-verbatim` (CleanupStyle-Impl) validieren die Trait-Contracts. Plugin-Crates via `cargo xtask new-plugin <name>` scaffolded.

**Stories-Sequencing:** 1B.1 Stage-Registry (Compile-Time-Layer) → 1B.2 Manifest-Parser + Schema-Version → 1B.3 `klarvo-plugin-verbatim` (CleanupStyle Ref-Impl) → 1B.4 `klarvo-plugin-groq` (SttProvider Ref-Impl) → 1B.5 Pipeline-Executor (Boot + Runtime Dispatch).

#### Story 1B.1: `PipelineStageType`-Enum Extension + Stage-Registry (Compile-Time-Layer)

As a Core-Developer composing Pipeline-Manifests,
I want das in 1A.5 skelettierte `PipelineStageType`-Enum zur vollständigen Phase-1-Compile-Time-Registry ausgebaut, mit Cargo-Feature-gated Variants `Stt` (Feature `stage-stt`) und `Cleanup` (Feature `stage-cleanup`) additiv zum bestehenden `Passthrough` (Feature `stage-passthrough`),
So that die Menge erlaubter Stage-Types zur Compile-Zeit abgeschlossen ist, Boot-Time-Parse (1B.2) gegen diese Registry serde-matcht und Runtime-Executor (1B.5) exhaustive-matchen muss — Innovation-A Compile-Time-Layer.

**Acceptance Criteria:**

**Given** das in 1A.5 skelettierte `PipelineStageType`-Enum in `klarvo-core::pipeline::stage` (mit `#[serde(tag = "type", rename_all = "kebab-case")]` und `Passthrough`-Variant hinter Feature `stage-passthrough`),
**When** 1B.1 additive die Variants `Stt { plugin_id: String }` (hinter Feature `stage-stt`) und `Cleanup { plugin_id: String }` (hinter Feature `stage-cleanup`) hinzufügt,
**Then** ist das Enum-Layout `pub enum PipelineStageType { #[cfg(feature = "stage-passthrough")] Passthrough, #[cfg(feature = "stage-stt")] Stt { plugin_id: String }, #[cfg(feature = "stage-cleanup")] Cleanup { plugin_id: String } }`, **And** die Serde-Tag-Wire-Names sind `"passthrough"`, `"stt"`, `"cleanup"` (kebab-case-Derivation vom Variant-Ident), **And** der `plugin_id`-Field lebt unter dem Tag-Object in TOML (Shape: `[[pipeline.stages]] type = "stt" plugin_id = "groq"`).

**Given** Cargo-Feature-Default-Configuration für Phase-1-Happy-Path,
**When** `klarvo-core/Cargo.toml` die Features definiert,
**Then** enthält `[features]` die Einträge `default = ["stage-passthrough", "stage-stt", "stage-cleanup"]`, `stage-passthrough = []`, `stage-stt = []`, `stage-cleanup = []` (keine inter-feature-Dependencies — Enum-Variants sind unabhängige Toggles), **And** `cargo check -p klarvo-core` mit Default-Features kompiliert clean, **And** `cargo check -p klarvo-core --no-default-features --features stage-passthrough` kompiliert clean (Minimal-Config: nur Passthrough-Variant).

**Given** Exhaustive-Match-Enforcement als Compile-Time-Hard-Fail-Invariante (per `memory/feedback_manifest_compile_contract`),
**When** Downstream-Konsumenten in Core (1B.2-Parser, 1B.5-Executor) auf `PipelineStageType` matchen,
**Then** verbietet Rustdoc-Contract explicit Wildcard-Arms (`_ => ...`) in Core-Match-Sites — jeder Consumer muss exhaustive matchen, damit neue Variants compile-error triggern (forced-update-invariant), **And** kein `#[non_exhaustive]`-Attribut auf dem Enum (würde den Compile-Error-Invariant untergraben), **And** Rustdoc dokumentiert diese Invariante explizit: *„Adding a variant here is a breaking change by design; downstream `match` sites MUST be updated. No `_` wildcard arms in core match-sites."*

**Given** Innovation-A-Zweischicht-Modell (per PRD §Innovation-A-Amendment + `memory/project_manifest_boot_time_parse`),
**When** `PipelineStageType`-Rustdoc geschrieben wird,
**Then** dokumentiert es drei Rollen explizit: (1) *Compile-Time-Registry* — die Menge erlaubter Stage-Types wird durch Cargo-Features und Enum-Variants abgeschlossen, (2) *Boot-Time-Parse* — Manifest-serde-Parse matcht gegen diese Registry (Details 1B.2), (3) *Runtime-Dispatch* — Executor-Match dispatcht zu Plugin-Impl via `plugin_id` (Details 1B.5), **And** die Rustdoc benennt beide Innovation-A-Failure-Modes explizit in einem Satz: *„Compile-time enforcement happens at the registry-set level: adding a new stage-type requires an enum-variant addition plus feature declaration in `klarvo-core`. Unknown stage-types in a user-authored manifest fail at boot-time deserialization (1B.2), not at compile-time — these are distinct failure modes."*, **And** die Rustdoc adressiert Consumer-cfg-Gating: *„Consumers matching on `PipelineStageType` under non-default feature combinations must mirror the `#[cfg(feature = …)]` gating in their match arms. See example in module-level doc."*, **And** Module-Level-Rustdoc (`pub mod stage` in `klarvo-core::pipeline`) enthält ein inline-runnable Example-Snippet: ein `match`-Arm-Pattern mit `#[cfg(feature = "stage-*")]`-annotated Arms, das unter `cargo test --doc` compiliert und keinen Wildcard verwendet.

**Given** Serde-Roundtrip-Integrity für Test + Manifest-Authoring,
**When** die Derives spezifiziert werden,
**Then** derives `PipelineStageType` die Traits `Debug`, `Clone`, `PartialEq`, `Eq`, `serde::Deserialize`, `serde::Serialize`, **And** ein Roundtrip-Test in `klarvo-core/tests/stage_type_roundtrip.rs` verifiziert pro Feature-gated Variant: `let t = PipelineStageType::Stt { plugin_id: "groq".into() }; let s = toml::to_string_pretty(&SingleStageWrapper(t.clone())).unwrap(); let parsed: SingleStageWrapper = toml::from_str(&s).unwrap(); assert_eq!(parsed.0, t);` (analog für `Cleanup` und `Passthrough`), **And** `SingleStageWrapper` ist Test-only-Struct mit `#[serde(transparent)]` und dokumentierendem Test-Kommentar: *„TOML top-level must be a table; variant-isolated roundtrip uses transparent wrapper to exercise serde-tag-parsing without depending on Pipeline-Shape — Pipeline-Shape-Roundtrip is 1B.2-Scope."*

**Given** 1A.1 test-fixtures-Primitive-Pattern (additive Extension),
**When** 1B.1 shippt,
**Then** extended `klarvo-test-fixtures` einen `stage_type`-Helper-Module mit Constructor-Functions `stage_type_stt(plugin_id: &str) -> PipelineStageType`, `stage_type_cleanup(plugin_id: &str) -> PipelineStageType`, `stage_type_passthrough() -> PipelineStageType` (alle feature-gated analog zur klarvo-core-Enum), **And** die Helpers verfügen über Rustdoc mit Usage-Example für Manifest-Parse-Tests (1B.2-Forward-Reference).

**Given** FR36 (Rustdoc-Contract),
**When** die Public-Items des Enums + Feature-Flags dokumentiert werden,
**Then** hat jeder Variant Rustdoc mit (a) Intent — welche Trait-Familie der Stage-Type repräsentiert (`Stt` → 1A.6 `SttProvider`, `Cleanup` → 1A.6 `CleanupStyle`, `Passthrough` → 1A.5 No-Op), (b) Contract-Condition — `plugin_id` muss einem at-Runtime registrierten Plugin matchen (sonst Boot-Time-Hard-Fail in 1B.5), (c) Cross-Reference zu korrespondierender 1A.x-Trait-Story + 1B.2 Parser-Story + 1B.5 Executor-Story, **And** `Passthrough`-Variant-Rustdoc benennt die Plugin-Asymmetrie explizit: *„Passthrough is the only plugin-free variant in Phase 1; all other stage-types carry `plugin_id` for plugin-registry resolution at boot-time (1B.5)."*

**Given** FR18 (headless-capable) + 1A.1 test-fixtures-Convention,
**When** 1B.1 shippt,
**Then** alle Tests (Roundtrip + Feature-Combination-Checks) laufen via `cargo test -p klarvo-core` und `cargo test -p klarvo-test-fixtures` headless ohne Audio-Device, Network oder GUI-Dependency, **And** Test-Runtime unter 5 Sekunden total (CI-Baseline-conform).

*Forward-Reference (non-AC):* Epic-5 FR34-`lint-events`-Gate-Extension-Hint — ein xtask-Lint könnte Core-Crate-Source nach `match ... PipelineStageType { _ => ... }`-Patterns greppen, um den No-Wildcard-Contract statisch zu enforcen. Notiz für Epic-5-Story-Scope-Discussion, nicht 1B.1-AC-Scope.

---

#### Story 1B.2: Pipeline-Manifest-Parser + Schema-Version-Header (Boot-Time Hard-Fail Parse)

As a Core-Developer at App-Boot,
I want einen `klarvo-core::manifest`-Module, der das via `include_str!` eingebettete `pipeline-manifest.toml` zur Boot-Zeit via serde gegen 1B.1's `PipelineStageType`-Registry parst, Schema-Version-Header zuerst validiert und bei unbekannten Stage-Types oder Schema-Version-Mismatch mit `AppError::kind::PipelineValidation` hart erroriert,
So that die Innovation-A-Parse-Layer-Hard-Fail-Semantik — *„unknown stage-type at parse layer → `AppError::PipelineValidation`, no warn+skip, no silent fallback"* — als Boot-Time-Invariante komplett ist und das Manifest als Compile-Contract-via-`include_str!` über serde-tag-Matching realisiert wird.

**Acceptance Criteria:**

**Given** `klarvo-core::manifest`-Module und den Additional-Req-Contract *„Pipeline-Manifest-TOML embedded via `include_str!("../../pipeline-manifest.toml")` zur Compile-Zeit"*,
**When** das Modul die Embedded-Default-Source-of-Truth exposed,
**Then** existiert ein `pub const EMBEDDED_MANIFEST: &str = include_str!("../../pipeline-manifest.toml");` (Pfad relativ zu `klarvo-core/src/manifest.rs`, verifiziert gegen Phase-0-Workspace-Layout ohne `crates/`-Prefix — zwei Levels up), **And** die File `pipeline-manifest.toml` lebt im Workspace-Root mit einem realen Phase-1-Minimal-Manifest (Schema-Version-Header + ein `passthrough`-Stage-Entry), **And** Rustdoc auf `EMBEDDED_MANIFEST` dokumentiert den Compile-Time-Embed-Contract: *„Entfernen oder Umbenennen der `pipeline-manifest.toml` führt zu `cargo build`-Failure (`include_str!`-IO-Error) — das ist der gewollte Compile-Time-Hard-Fail-Mechanismus, komplementär zum Boot-Time-Parse-Hard-Fail."*

**Given** FR11 Schema-Version-Header-Validation + Sequential-Check-Contract,
**When** der Parser die Manifest-Struct definiert und den Two-Pass-Parse-Path implementiert,
**Then** existiert `pub struct PipelineManifest { pub schema_version: u32, pub pipeline: PipelineSpec }` mit `PipelineSpec { pub stages: Vec<PipelineStageType> }`, **And** der Parser implementiert explizit **Two-Pass-Parse**: Pass 1 deserialisiert `struct VersionPeek { schema_version: u32, pipeline: toml::Value }` — Pass 2 rejects wenn `schema_version != 1` mit `AppError::kind::PipelineValidation` + i18n-key `"error.pipeline.schema_version_unsupported"`, Pass 3 (nur bei Version-OK) re-deserialisiert `pipeline: toml::Value` in `PipelineSpec`, **And** Rustdoc dokumentiert den Two-Pass-Intent explizit: *„Schema-version is validated before stage-type resolution so that a user authoring a schema_version=2 manifest (Phase-2) doesn't get a bogus `unknown_stage_type`-error for a stage that is valid under v2 but not v1."*, **And** Phase-1-akzeptierte `schema_version` ist **exakt** `1` (kein Range, kein Wildcard — per Additional-Req-Schema-Version-Contract), **And** die Cause-Chain carriert received-vs-expected Version-Numbers.

**Given** FR5 Boot-Time-Parse + FR6 Parse-Layer-Hard-Fail-Invariante (per `memory/feedback_manifest_compile_contract`),
**When** der Parser einen `PipelineManifest` aus `EMBEDDED_MANIFEST` (oder Test-injected String) deserialisiert und trifft auf einen Stage-Entry mit unknown `type`-Tag (z. B. `type = "quantum-fft"`) — Schema-Version ist bereits als valid bestätigt,
**Then** failt der serde-Parse (Pass 3) zur Boot-Zeit mit `AppError::kind::PipelineValidation` und i18n-key `"error.pipeline.unknown_stage_type"`, **And** die Cause-Chain enthält den offending `type`-Tag-String + Stage-Index-im-Vec + den aufgetretenen serde-Error, **And** es existiert **kein** `warn!+skip`-Path, **kein** `tracing::debug!+continue`-Path und **kein** silent fallback im Parser — Code-Review-Invariante: der Parser enthält nirgends eine `_ => ...`-Arm oder `if let Err(_) = ... { tracing::warn!(...) }`-Pattern in Stage-Resolution-Logic.

**Given** die Public-Parser-API für Boot-Integration,
**When** `klarvo-core::manifest` exposiert wird,
**Then** existiert `pub fn parse_embedded() -> Result<PipelineManifest, AppError>` als Primary-Entrypoint (parst `EMBEDDED_MANIFEST`), **And** `pub fn parse_from_str(toml_src: &str) -> Result<PipelineManifest, AppError>` als Test-Injection-Entrypoint (von `klarvo-test-fixtures` und xtask-Harness genutzt), **And** keine weitere Public-API (kein `parse_from_path`-Fallback — per Additional-Req *„kein `read-from-working-dir`-Fallback"*; User-Override-Manifest-Path ist Phase-Post-1-Scope).

**Given** FR8 i18n-Key-only-Discipline + 1A.4 `assert_is_key` + Keys-Submodule-Convention,
**When** AppError-Emission-Sites im Parser AppErrors konstruieren,
**Then** sind alle `user_message`-Keys durch 1A.4's Runtime-Assertion-Primitive in Debug-Builds validiert (via `debug_assert!(klarvo_core::i18n::is_key(key))` oder equivalent), **And** die Key-Set des Parsers lebt in einem `pub mod keys { pub const SCHEMA_VERSION_UNSUPPORTED: &str = "error.pipeline.schema_version_unsupported"; pub const UNKNOWN_STAGE_TYPE: &str = "error.pipeline.unknown_stage_type"; pub const TOML_PARSE_FAILURE: &str = "error.pipeline.toml_parse_failure"; }`-Submodule (für rein-syntaktische TOML-Fehler vor serde-Resolution), **And** die Keys sind Grep-bar + Rename-safe + statisch referenzierbar aus Shell-Translation-Tables (Epic 4 Forward-Reference).

**Given** 1A.1 test-fixtures + 1B.1 `stage_type`-Helpers,
**When** 1B.2 shippt,
**Then** extended `klarvo-test-fixtures` ein `manifest`-Helper-Module mit (a) `valid_minimal_manifest_toml() -> String` (schema_version=1 + single Passthrough-Stage), (b) `manifest_with_unknown_stage_toml(unknown_tag: &str) -> String`, (c) `manifest_with_wrong_schema_version_toml(version: u32) -> String`, (d) `assert_parse_fails_with_kind!(manifest_toml, AppError::kind::PipelineValidation)`-Macro, **And** die Helpers ermöglichen in 1B.5-Executor-Integration-Tests reproducible Bad-Input-Scenarios ohne Embedded-Manifest-Änderung.

**Given** FR32 xtask-manifest-strict-Gate-Kooperation (Epic 5-Forward-Reference),
**When** 1B.2 shippt,
**Then** ist `parse_from_str` explizit **nicht** `#[cfg(test)]`-gated — xtask-manifest-strict (FR32) wird diesen Entrypoint aus einem Harness-Binary konsumieren (Harness-Compile-Time feedet Test-Manifests per `memory/project_manifest_boot_time_parse`), **And** Rustdoc auf `parse_from_str` dokumentiert diesen Cross-Epic-Consumer explizit: *„Used by `cargo xtask manifest-strict` (Epic 5 FR32) to exercise bad-input scenarios at harness-compile-time — not a runtime user-facing API."*

**Given** FR36 Rustdoc-Contract,
**When** Public-Items (`PipelineManifest`, `PipelineSpec`, `parse_embedded`, `parse_from_str`, `keys`-Submodule) dokumentiert werden,
**Then** hat jedes Item Rustdoc mit (a) Intent, (b) Contract-Condition (z. B. *„`parse_embedded` never succeeds if `pipeline-manifest.toml` lacks a `schema_version` field or references an unknown stage-type — both fail fast at boot with `AppError::kind::PipelineValidation`"*), (c) Cross-Reference zu 1B.1 Registry, 1B.5 Executor (inkl. Forward-Reference zu Runtime-Type-Chaining-Check, der im Parse-Layer **nicht** stattfindet), Epic 5 FR32 Gate, Epic 4 Error-Surface.

**Given** FR18 headless-capable + FR8 i18n-key-only,
**When** 1B.2 shippt,
**Then** alle Parser-Tests (valid + invalid Manifests, Schema-Version-Variations) laufen via `cargo test -p klarvo-core` und `cargo test -p klarvo-test-fixtures` headless ohne Audio-Device, Network oder GUI-Dependency, **And** Test-Runtime unter 5 Sekunden total, **And** kein Test asserted gegen User-Strings (nur Error-Kind + i18n-Key-Values via `assert_is_key`-Guarded Comparisons).

*Scope-Deferrals (non-AC):* Type-Chaining-Compat-Check (Stage-N-Output vs Stage-N+1-Input-Mismatch) → 1B.5 Runtime-Dispatch-Layer (architecture.md:234-238 mandatet keinen Parse-Layer-Check; 1A.5-Amendment-Friction vermieden). Plugin-Registry-Lookup-Hard-Fail (Plugin-not-found) → 1B.5 (Registry existiert erst nach 1B.3/1B.4). User-Override-Manifest-Path (`%APPDATA%\...`) → Phase-Post-1.

---

#### Story 1B.3: `klarvo-plugin-verbatim` — CleanupStyle Reference-Impl (Identity-Passthrough)

As a Plugin-Developer,
I want den Phase-0-scaffolded `klarvo-plugins/klarvo-plugin-verbatim/`-Crate zur vollständigen Post-Epic-1A `CleanupStyle`-Reference-Impl ausgebaut — strict literal-identity-passthrough, KeyStore-free,
So that Epic 1B's Reference-Plugin-Pair den Text-Domain-Output-Side mit einer thinnen Phase-1-Verbatim-Only-CleanupStyle-Impl abdeckt und das Trait-Contract aus external-crate-Perspektive validiert.

**Acceptance Criteria:**

**Given** der bestehende Phase-0-Scaffold unter `klarvo-plugins/klarvo-plugin-verbatim/` (Cargo.toml + src/lib.rs + src/provider.rs bereits als Workspace-Member listed, mit speculative Phase-0-API: `klarvo_core::PluginError` + `klarvo_core::traits::CleanupStyle`),
**When** 1B.3 die Scaffold-Implementierung zur Post-Epic-1A-Reference-Impl migriert,
**Then** ist die Public-API des Crate: `pub struct Verbatim;` (struct-name unverändert aus Phase-0), `impl Verbatim { pub fn new() -> Self }` + `Default`-Impl, `pub const ID: &str = "verbatim";` (Plugin-Identifier, referenziert von Manifest-`plugin_id` via 1B.1 `PipelineStageType::Cleanup { plugin_id: "verbatim".into() }`), **And** `Verbatim` implementiert `klarvo_core::cleanup::CleanupStyle` per 1A.6-Signature (`: PipelineStage<Input = CleanupInput, Output = String>`-Supertrait-Bound + `async fn apply(&self, input: CleanupInput) -> Result<String, AppError>`), **And** die Migration von Phase-0's `PluginError` → v2-`AppError` und von `klarvo_core::traits::*` → `klarvo_core::cleanup::*`-Modulpfad ist komplett, **And** die `pub fn register(registry: &mut klarvo_core::PluginRegistry)`-Function registriert `Verbatim::new()` unter `ID` als `CleanupStyle`-Impl (konkrete Arc/Box-Shape resolved durch 1B.5 `PluginRegistry`-API).

**Given** Scope-Strictness *„Identity-Passthrough im strengsten Sinn"* (per `memory/feedback_polished_designschwaeche`),
**When** `Verbatim::apply(input: CleanupInput) -> Result<String, AppError>` aufgerufen wird,
**Then** returned es `Ok(input.raw.clone())` — **kein** Trim, **kein** Whitespace-Collapse, **kein** Case-Change, **kein** Normalization, **kein** Punctuation-Strip, **keine** Dictionary-Anwendung, **keine** Output-Language-Transformation (auch wenn `input.context.output_language` gesetzt ist — Phase-1-Verbatim ignoriert das Field explizit), **And** der CleanupContext wird akzeptiert aber nicht konsumiert — die Semantik ist rein input-raw-passthrough.

**Given** External-Crate-Contract-Validation als Real-Value der Story,
**When** das Crate gegen post-Epic-1A `klarvo-core` kompiliert,
**Then** hängt `klarvo-plugin-verbatim` ausschließlich an `klarvo-core`'s **public** API (`klarvo_core::cleanup::{CleanupStyle, CleanupInput, CleanupContext}`, `klarvo_core::pipeline::stage::PipelineStage`, `klarvo_core::AppError`, `klarvo_core::PluginRegistry`) — kein `pub(crate)`-Access, kein Workspace-Path-Trick zu privaten Modulen, keine `klarvo_core::__private::*`-Imports, **And** `cargo check -p klarvo-plugin-verbatim` kompiliert mit klarvo-core-Default-Features ohne zusätzliche Feature-Flags, **And** die existierenden 5 Phase-0-Unit-Tests (empty, ASCII, filler-words, multiline/whitespace, unicode/punctuation) werden auf die 1A.6-`CleanupInput`-Signature portiert (CleanupInput-Wrapping des raw-strings + `CleanupContext::default()` wo ausreichend) und bleiben grün.

**Given** 1B.5-E2E-Executor-Test-Dependency (companion-Story benötigt Plugin-Registry-Entry),
**When** 1B.3 shippt,
**Then** existiert ein Integration-Test `klarvo-plugins/klarvo-plugin-verbatim/tests/external_contract.rs`, der den vollen CleanupStyle-Contract aus external-crate-Perspektive exerziert: konstruiert `CleanupInput { raw: "test", context: CleanupContext::default() }`, invokes `Verbatim::new().apply(input).await`, assertet `result == Ok("test".to_string())`, **And** der Test läuft headless ohne Network/Audio/GUI via `cargo test -p klarvo-plugin-verbatim --test external_contract`, unter 5 Sekunden, **And** der Test existiert **zusätzlich** zu den `#[cfg(test)]`-Unit-Tests in `src/provider.rs` — die externe-Test-File validiert „external crate consumes public API only", die Unit-Tests validieren interne Semantik.

**Given** FR36 Rustdoc-Contract + Phase-1-Verbatim-Only-Rationale,
**When** `Verbatim`-struct, `apply`-method, `ID`-const und `register`-fn dokumentiert werden,
**Then** enthält das Module-Level-Rustdoc (a) Scope-Statement: *„`Verbatim` is the literal identity-passthrough `CleanupStyle` implementation — no trim, no normalization, no dictionary application. Phase-1 default CleanupStyle per `docs/rebuild-discussion.md` Polished-Mode-Deferral."*, (b) Forward-Reference zu 1B.4 Groq-Companion: *„The companion reference-plugin `klarvo-plugin-groq` (Story 1B.4) demonstrates an external-API-dependent trait impl (HTTPS client, KeyStore-dependent) — `klarvo-plugin-verbatim` covers the opposite pole of the plugin-complexity-spectrum."*, (c) Phase-2-Deferral-Note: *„Polished-Mode CleanupStyle (with dictionary application and output-language transformation) is deferred to Phase 2 per `memory/feedback_polished_designschwaeche` — `Verbatim` remains the Phase-1 default."*

---

#### Story 1B.4: `klarvo-plugin-groq` — `SttProvider` Reference-Impl via Groq Whisper HTTPS-API

**As a** Plugin-Developer building Cloud-Provider-Integrations,
**I want** den Phase-0-scaffolded `klarvo-plugins/klarvo-plugin-groq/`-Crate zur vollständigen Post-Epic-1A `SttProvider`-Reference-Impl gegen Groq's Whisper HTTPS-API ausgebaut — mit ADR-0005-verankerten Stack-Choices und KeyStore-mock-backed Test-Harness,
**So that** Epic 1B's Reference-Plugin-Pair den Audio-Domain-Input-Side mit einer External-API-dependent-Impl abdeckt, der Trait-Contract aus external-crate-Perspektive validiert wird, und das ADR-0005-Pattern (reqwest + rustls-native-roots + per-Plugin-Instance-Client + wiremock-Test-Harness) als reusable-Template für alle Phase-2+-Cloud-Provider-Plugins etabliert ist.

**Acceptance Criteria:**

**Given** der bestehende Phase-0-Scaffold unter `klarvo-plugins/klarvo-plugin-groq/` (Cargo.toml mit nur `klarvo-core`-dep + src/lib.rs mit minimalem `pub fn register(_registry: &mut PluginRegistry) {}`-Skelett, kein `PluginError`, keine speculative Groq-API-Shape),
**When** 1B.4 die Scaffold-Implementierung zur Post-Epic-1A Reference-Impl ausbaut,
**Then** ist die Public-API des Crate: `pub struct Groq { client: reqwest::Client, api_key: SecretString, endpoint: String, model: String }` (Fields private), **And** `pub fn new(api_key: SecretString) -> Self` als Primary-Constructor mit Default-Endpoint `"https://api.groq.com/openai/v1/audio/transcriptions"`, Default-Model `"whisper-large-v3"` (beide Phase-1-hardcoded; Config-Driven-Override = Phase-2+-Scope) und Default-Client-Timeout `Duration::from_secs(30)` (Phase-1-hardcoded; Plugin-Config-Layer = Epic 4 Scope per ADR-0005-§6), **And** `pub fn new_with_client(api_key: SecretString, client: reqwest::Client) -> Self` als Secondary-Constructor für Test-Injection (ermöglicht short-timeout-Client-Inject in wiremock-Integration-Tests; Production-Code nutzt `new()`), **And** `pub const ID: &str = "groq";` (Plugin-Identifier, referenziert von Manifest-`plugin_id` via 1B.1 `PipelineStageType::Stt { plugin_id: "groq".into() }`), **And** `Groq` implementiert `klarvo_core::stt::SttProvider` per 1A.6-Signature (`: PipelineStage<Input = AudioBuffer, Output = String>`-Supertrait-Bound + `async fn transcribe(&self, audio: AudioBuffer) -> Result<String, AppError>`), **And** **keine** `pub fn register`-Function exposed (divergiert bewusst von 1B.3-Verbatim-Pattern: Groq benötigt einen API-Key zur Konstruktion, ein no-args `register(registry)` kann ohne Key nicht instanziieren; die keyed-Plugin-Registry-Wire-Up-Semantik ist Epic-1C-Design-Question, nicht 1B.4-Scope).

**Given** ADR-0005 als authoritative HTTPS-Stack-Decision,
**When** `Groq::new` den `reqwest::Client` konstruiert und `transcribe` den HTTPS-Call ausführt,
**Then** pinnt die Cargo.toml `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls-native-roots", "json", "multipart"] }` (multipart ist Plugin-level-Feature für Groq-Whisper-Audio-Upload-Form per ADR-0005-§3), `secrecy = "0.8"`, `hound = "3.5"` (WAV-Encoding von `AudioBuffer::samples` als in-memory PCM-f32-WAV-Bytes vor dem multipart-Upload; Plugin-local per Phase-1-Scope, Factor-out zu `klarvo-core::audio::wav` deferred until Phase-2+ proves Duplication) und `tokio = { workspace = true, features = ["rt"] }`, **And** der `reqwest::Client` in `Groq::new` wird mit `ClientBuilder::new().timeout(Duration::from_secs(30)).build()` konstruiert und als Struct-Field gehalten (ADR-0005-§4 per-Plugin-Instance-Pattern; kein `lazy_static!`, keine per-Call-Instanziierung), **And** der HTTPS-Request ist POST `{endpoint}` mit Header `Authorization: Bearer {api_key}` (Bearer-String konstruiert via `SecretString::expose_secret()` innerhalb `transcribe`, nicht persistiert in Struct-Field) und Content-Type `multipart/form-data` mit Parts (a) `file` als WAV-PCM-f32-Bytes + Filename `"audio.wav"` + Content-Type `"audio/wav"`, (b) `model` = `self.model`, (c) `response_format` = `"json"`, **And** die Response wird via `response.json::<GroqTranscriptionResponse>()` deserialized (privates `#[derive(Deserialize)] struct GroqTranscriptionResponse { text: String }`) und der `text`-String zurückgegeben.

**Given** FR29 (Groq-API-Failures surfacen als `AppError::kind::UpstreamUnavailable` mit user_message-Key und gelogged Cause) + 1A.2 `AppError`-Shape,
**When** `transcribe` auf HTTPS-/Transport-Failures reagiert,
**Then** klassifiziert 1B.4 Failures in **7** User-Message-Keys und konstruiert jeweils `AppError::new(ErrorKind::UpstreamUnavailable).with_message(<key>).with_source(e)`:
- `"error.stt.network"` — DNS-Lookup-Fail, Connection-Refused, TLS-Handshake-Fail, allgemeiner Transport-Fehler (via `reqwest::Error::is_connect()` oder `is_request()` ohne Status)
- `"error.stt.timeout"` — Request-Timeout (via `reqwest::Error::is_timeout()`)
- `"error.stt.upstream_5xx"` — Groq-Side 500/502/503/504-Response
- `"error.stt.rate_limited"` — 429 Response
- `"error.stt.auth_failed"` — 401/403 Response (User-Action: API-Key prüfen; Bearer-Token-Leak-Prevention: Key NIE in Error-Message, Cause-Chain trägt nur den HTTP-Status-Body-Exzerpt ohne Headers)
- `"error.stt.invalid_audio"` — 400 Response (User-Action: Audio-Format-Problem, wahrscheinlich VAD-Output-Mismatch)
- `"error.stt.upstream_4xx"` — 402/404/405/andere 4xx Catch-all (User-Action: generic upstream-issue)

**And** jeder Error-Path preserviert den Original-`reqwest::Error` bzw. HTTP-Status-Body-Exzerpt in `AppError::source` für Rolling-File-Log-Visibility (FR37), **And** der API-Key ist niemals in `Debug`/`Display`/`source()` einer AppError repräsentiert (Assert via Test: `format!("{:?}", err)`, `format!("{}", err)` und die vollständige `source()`-Chain-Stringification enthalten weder den Raw-Key noch einen `Bearer `-Prefix — `SecretString` redigiert automatisch, aber Error-Construction-Site könnte trotzdem leaken via `e.to_string()`-Pass-through; Test lockt die Invariante gegen Code-Changes), **And** Rate-Limiting-Retry-Orchestration ist **explizit not-in-scope** von 1B.4 — die Error-Surface ist delivered, Retry-Policy landet in Epic 2 FR29 auf Pipeline-Level per ADR-0005-§6.

**Given** wiremock-based-Test-Harness per ADR-0005-§5 + External-Crate-Contract-Validation als Real-Value der Story,
**When** das Crate gegen post-Epic-1A `klarvo-core` + `klarvo-test-fixtures` kompiliert,
**Then** hängt `klarvo-plugin-groq` ausschließlich an `klarvo-core`'s **public** API (`klarvo_core::stt::{SttProvider, AudioBuffer}`, `klarvo_core::pipeline::stage::PipelineStage`, `klarvo_core::{AppError, ErrorKind}`) — keine `pub(crate)`/`__private`-Imports, **And** `cargo check -p klarvo-plugin-groq` kompiliert mit klarvo-core-Default-Features ohne zusätzliche Feature-Flags, **And** `klarvo-test-fixtures` exportiert zwei neue Primitive: (a) `MockKeyStore { canned_keys: HashMap<String, SecretString> }` mit `pub fn get(&self, key: &str) -> Option<SecretString>` (pre-defined KeyStore-Trait-Integration = Epic 1C), (b) `GroqMockServer` als Thin-Wrapper um `wiremock::MockServer` mit Helpern `with_success_response(text: &str)`, `with_status(code: u16, body: &str)`, `with_delayed_response(text: &str, delay: Duration)` (für Timeout-Tests), `with_network_failure()` (simuliert Connection-Refused via Port-Close), **And** existiert ein Integration-Test `klarvo-plugins/klarvo-plugin-groq/tests/external_contract.rs`, der **7** Test-Cases exerziert: (1) Success-Case: MockServer antwortet `{"text":"hello"}`, `Groq::new(mock_key).transcribe(audio).await == Ok("hello")`, (2) 5xx-Case: MockServer antwortet 503, Result ist `Err` mit `kind=UpstreamUnavailable` + message `"error.stt.upstream_5xx"`, (3) 429-Case: Result ist `Err` mit message `"error.stt.rate_limited"`, (4) 401-Auth-Case: Result ist `Err` mit message `"error.stt.auth_failed"` **und** Assertion dass `format!("{:?}", err)`, `format!("{}", err)` und die vollständige `source()`-Chain-Stringification den raw API-Key + `Bearer `-Prefix nicht enthalten, (5) 400-Invalid-Audio-Case: Result ist `Err` mit message `"error.stt.invalid_audio"`, (6) Timeout-Case: MockServer mit 500ms-delayed-Response, `Groq::new_with_client(mock_key, Client-with-100ms-timeout)`, Result ist `Err` mit message `"error.stt.timeout"`, Test-Runtime unter 300ms, (7) Network-Failure-Case: MockServer-Port ist geschlossen, Result ist `Err` mit message `"error.stt.network"`, **And** der `upstream_4xx`-Catch-all (402/404/405/etc.) wird **nicht** explizit im Integration-Test exerziert — Rustdoc-Note erklärt: *„Catch-all upstream_4xx covers 402/404/405/etc.; not exercised in integration-tests as these are rarely encountered in practice, but code-path is reachable."*, **And** alle Tests laufen headless ohne Audio/GUI via `cargo test -p klarvo-plugin-groq --test external_contract`, unter 10 Sekunden, mit `#[tokio::test]`-Harness (async per ADR-0005-§5).

**Given** FR36 Rustdoc-Contract + ADR-0005-als-Cross-Reference + 1B.3-Companion-Pair-Narrative,
**When** `Groq`-struct, `new`-fn, `new_with_client`-fn, `transcribe`-method (inherited from SttProvider), `ID`-const und Module-Level dokumentiert werden,
**Then** enthält das Module-Level-Rustdoc (a) Scope-Statement: *„`Groq` is the Groq Whisper HTTPS-API-backed `SttProvider` implementation. HTTPS stack (reqwest + rustls-tls-native-roots + wiremock-for-tests) is locked per ADR-0005 and is the reference-pattern for all Phase-2+ Cloud-Provider-Plugins (DeepSeek, OpenAI, Anthropic, OpenRouter)."*, (b) Companion-Reference zu 1B.3-Verbatim: *„The companion reference-plugin `klarvo-plugin-verbatim` (Story 1B.3) demonstrates the opposite pole of the plugin-complexity-spectrum — KeyStore-free, external-API-free, pure identity-passthrough. Together, these two plugins exercise both ends of the `PipelineStage`-contract-surface."*, (c) Forward-References: *„Registry-registration and Manifest-driven instantiation are Epic 1C scope (KeyStore-backed construction). 1B.5 E2E-Executor-Test uses Verbatim-based pipelines only. Pipeline-level retry orchestration for `UpstreamUnavailable` errors is Epic 2 FR29 (not 1B.4). Config-driven endpoint/model/timeout-override is Phase-2+ scope."*, (d) Model-Choice-Rationale: *„Phase-1 prefers accuracy over latency — `whisper-large-v3` is the default; `whisper-large-v3-turbo` and config-driven model-override are Phase-2+ scope."*, (e) WAV-Encoding-Scope-Note: *„WAV-encoding via `hound` is plugin-local; factor-out to `klarvo-core::audio::wav` deferred until Phase-2+ Cloud-STT-Plugins (OpenAI Whisper, DeepSeek) prove duplication — then as dedicated micro-refactor story, not as premature-abstraction in Phase 1."*, (f) API-Key-Safety-Note: *„The `api_key: SecretString` constructor-param is held in `Groq`-struct via `secrecy`-crate — Debug/Display are redacted, source() of AppErrors never leaks the raw key. Bearer-header construction uses `expose_secret()` only inside the `transcribe` HTTPS-call-site. The Auth-Leak-Test in `tests/external_contract.rs` locks this invariant against code-changes; factor-out to an `assert_no_secret_leak!` macro in `klarvo-test-fixtures` is a Phase-2+ pattern-signal once additional API-Key-bearing plugins exist."*

---

#### Story 1B.5: Pipeline-Executor — Boot-Registry-Lookup + Runtime-Dispatch + Type-Chaining-Check + E2E-Headless-Test

**As a** Core-Developer at App-Boot running Dictation-Pipelines,
**I want** den `klarvo-core::pipeline::executor`-Module (Phase-0-bootstrap-Form) zur Post-Epic-1A/1B-Runtime-Shape migriert — ein Executor, der einen via 1B.2-geparsten `PipelineManifest` (bestehend aus 1B.1-`PipelineStageType`-Variants) gegen eine mit 1B.3/1B.4-Plugin-registered `PluginRegistry` zur Boot-Zeit validiert (Plugin-Lookup-Hard-Fail + Type-Chaining-Compat-Check) und zur Runtime-Zeit durch exhaustive-Match-Dispatch über die `PipelineStageType`-Enum die Pipeline-Stages sequentiell abarbeitet,
**So that** Innovation-A's Runtime-Dispatch-Layer — *„Unknown stage-type, Plugin-nicht-registered, oder Type-Mismatch zwischen Stages → `AppError::kind::PipelineValidation`, kein silent fallback"* — als Laufzeit-Invariante komplett ist und Epic 1B (Pipeline-Composition Runtime) mit einer headless-ausführbaren Text-Domain-Verbatim-Pipeline (passthrough → verbatim) geschlossen wird — ohne Shell, Audio-Device, Network- oder KeyStore-Dependency.

**Acceptance Criteria:**

**Given** Phase-0-`klarvo-core/src/pipeline/executor.rs` (sync-ish `pub async fn run(manifest, registry, input: &str) -> Result<String, AppError>` mit Old-`Manifest`-Shape — `manifest_version: String` + `Stage::Cleanup { plugin }`-Enum — und Old-`AppError`-Shape — `{ kind, message, user_message, retryable }` — + Cleanup-only-Stage-Match) + post-1A/1B-Upgrade-Bedarf (1A.2 `AppError { kind, user_message: Option<String>, source: Cause-Chain }`, 1A.5 `PipelineStage`-Trait, 1A.6 `SttProvider`+`CleanupStyle`, 1B.1 `PipelineStageType`, 1B.2 `PipelineManifest` mit `schema_version: u32`),
**When** 1B.5 den Executor-Module zur Runtime-Form migriert,
**Then** existiert eine neue Enum `pub enum StageData { Text(String), Audio(AudioBuffer) }` in `klarvo-core::pipeline::stage_data` (oder Unter-Modul von `pipeline::executor`, Impl-Choice) mit Rustdoc-dokumentiertem Phase-1-Scope: *„StageData is the Phase-1 inter-stage data carrier — `Text` and `Audio` variants cover the three Phase-1 stage-types (Stt: Audio → Text, Cleanup: Text → Text, Passthrough: identity over both variants). CleanupInput is not a distinct variant — it is constructed dispatch-arm-internal via `CleanupInput::from_raw(String)` with `CleanupContext::default()` (Phase-1-amendment to 1A.6; Pipeline-Config-passed-CleanupContext is Epic 4 / Phase-2+ scope). Additional variants (offline-whisper-intermediate-representations, multi-language-token-streams) are Phase-2+ extensions via additive enum-grow per `memory/feedback_manifest_compile_contract` no-wildcard-match discipline."*, **And** derives `Debug`, `Clone`, und hat inherent-method `pub fn type_name(&self) -> &'static str` returning `"text"` oder `"audio"` für Type-Chaining-Error-Reporting.

**Given** erweiterte `PluginRegistry` zur Aufnahme von `SttProvider`-Implementationen + Phase-0-`PluginError`-Residue-Removal,
**When** 1B.5 die Phase-0-`klarvo-core/src/registry.rs` erweitert und Phase-0-`klarvo-core/src/error.rs` bereinigt,
**Then** ist die Registry-API `pub struct PluginRegistry { stt: HashMap<String, Arc<dyn SttProvider>>, cleanup: HashMap<String, Arc<dyn CleanupStyle>> }` (Passthrough ist Executor-built-in, kein Registry-Slot), **And** neue Methods `pub fn register_stt(&mut self, id: impl Into<String>, plugin: Arc<dyn SttProvider>)` (panicked bei Duplicate-ID analog zu Phase-0 `register_cleanup`) + `pub fn stt(&self, id: &str) -> Option<Arc<dyn SttProvider>>` — bestehende `register_cleanup`/`cleanup` unverändert, **And** die Registry verwendet durchgehend `Arc<dyn …>` statt `Box<dyn …>` (1A.5-AC-Object-Safety-Compile-Test bleibt `Box<dyn PipelineStage>` als Surface-Guarantee; Arc-vs-Box ist für Object-Safety irrelevant, Arc wird für Registry-Multi-Session-Instance-Re-use per ADR-0005-§4-Lifetime-Mitigation benötigt; Rustdoc-Note auf Registry dokumentiert diese Inkonsistenz ohne 1A.5-Amendment), **And** der Phase-0-`PluginError`-Type wird aus `klarvo-core/src/error.rs` entfernt (unused post-1A.2/1B.3-Migration — 1A.2 liefert `AppError`-Shape, 1B.3 hat bereits `Verbatim` auf `AppError` portiert per 1B.3-AC; kein Consumer bleibt).

**Given** FR6-Runtime-Layer (per `memory/project_epic_breakdown_phase1` FR6-dreischichtig) + Hard-Fail-Invariante (per `memory/feedback_manifest_compile_contract`),
**When** der Executor eine Pipeline bootet und Stages via `PipelineStageType` dispatched,
**Then** existiert die Public-Signature `pub async fn run_pipeline(manifest: &PipelineManifest, registry: &PluginRegistry, input: StageData) -> Result<StageData, AppError>` als Primary-Entrypoint, **And** der Executor führt **zwei Boot-Time-Checks vor dem ersten Stage-Dispatch** in dieser Reihenfolge aus: (1) **Type-Chaining-Compat-Check** (zuerst, weil manifest-authoring-error unabhängig von Registry-State) — für jedes aufeinanderfolgende Stage-Paar `(stage[N], stage[N+1])` wird die deklarative Stage-Type-Kompatibilität geprüft (`stt` produziert `Text`, `cleanup` konsumiert `Text`, `passthrough` akzeptiert+produziert identisch die Input-Variante); plus die Entry-Variante der `input: StageData` wird gegen das erste Stage's Input-Erwartung geprüft; bei Mismatch (z. B. Manifest `cleanup → stt` — `Text → Audio` nicht wrapbar) failt der Executor mit `AppError::new(ErrorKind::PipelineValidation).with_message("error.pipeline.stage_type_mismatch")`, Cause-Chain enthält `expected_input`-Variant-Name + `actual_output`-Variant-Name + Stage-Index-Pair, (2) **Plugin-Registry-Lookup-Hard-Fail** (danach) — für jeden `PipelineStageType::Stt { plugin_id }` und `PipelineStageType::Cleanup { plugin_id }` in `manifest.pipeline.stages` wird der zugehörige Registry-Lookup (`registry.stt(id)` bzw. `registry.cleanup(id)`) durchgeführt; fehlt der Eintrag, failt der Executor mit `AppError::new(ErrorKind::PipelineValidation).with_message("error.pipeline.plugin_not_found")`, Cause-Chain enthält `plugin_id`-String + Stage-Index-im-Vec + Stage-Type-Name, **And** die Executor-Runtime-Dispatch (nach beiden Boot-Checks) ist eine `exhaustive match` über `PipelineStageType`-Variants (per 1B.1-AC: kein `_` Wildcard, `#[cfg(feature=…)]`-gating wo nötig): `PipelineStageType::Passthrough` returnt die aktuelle `StageData` unverändert (Identity über alle Variants), `PipelineStageType::Stt { plugin_id }` extrahiert `StageData::Audio(audio)` (Extraction per Boot-Check garantiert) + ruft `registry.stt(id).unwrap().transcribe(audio).await?` + wrappt zu `StageData::Text(result)`, `PipelineStageType::Cleanup { plugin_id }` extrahiert `StageData::Text(raw)` (Extraction per Boot-Check garantiert) + konstruiert `CleanupInput::from_raw(raw)` inline-arm (Phase-1-Amendment zu 1A.6: `pub fn from_raw(raw: String) -> Self` erstellt `CleanupInput { raw, context: CleanupContext::default() }`) + ruft `registry.cleanup(id).unwrap().apply(input).await?` + wrappt zu `StageData::Text(result)`, **And** jede Variant-Extract-Operation ist per Rustdoc als Boot-Check-garantiert-invariant dokumentiert (z. B. `let StageData::Audio(audio) = data else { unreachable!("boot-time Type-Chaining-Check guarantees Audio-variant here"); };`).

**Given** 1A.1/1B.2/1B.3-test-fixtures-Primitive-Pattern + Resolution 1 aus 1B.4-Review (E2E-Executor-Test nutzt **ausschließlich** Verbatim-based-Pipelines, **keine** Groq-Instanziierung — Groq-Registry-Wire-Up ist Epic-1C / Epic-2-Scope),
**When** 1B.5 shippt,
**Then** existiert ein Integration-Test `klarvo-core/tests/pipeline_executor_e2e.rs`, der **drei** Test-Cases exerziert: (1) **Happy-Path**: Manifest `passthrough → cleanup{plugin_id="verbatim"}` via neuer Helper `klarvo-test-fixtures::manifest::valid_passthrough_verbatim_manifest_toml()` (additive Extension zum 1B.2-manifest-Submodule), `PluginRegistry` mit `register_cleanup("verbatim", Arc::new(Verbatim::new()))`, Input `StageData::Text("hello world".into())`, Assertion: `Ok(StageData::Text(s))` mit `s == "hello world"` (Passthrough-Identity + Verbatim-Identity-Passthrough), (2) **Plugin-Not-Found-Fail**: identisches Manifest wie (1), aber `PluginRegistry::new()` leer (kein Plugin registered), Assertion: `Err(AppError)` mit `kind == ErrorKind::PipelineValidation` + `user_message == Some("error.pipeline.plugin_not_found")`, Cause-Chain-Stringification enthält `"verbatim"` und Stage-Index, (3) **Type-Chaining-Mismatch-Fail**: Manifest `cleanup{plugin_id="verbatim"} → stt{plugin_id="groq"}` (`Text → Audio` nicht wrapbar), Registry mit nur Verbatim registered (kein Groq — bewusst, um zu validieren dass Type-Chaining-Check **vor** Plugin-Lookup läuft), Input `StageData::Text("...".into())`, Assertion: `Err(AppError)` mit `user_message == Some("error.pipeline.stage_type_mismatch")` (nicht `plugin_not_found`), Rustdoc-Note im Test dokumentiert die Check-Reihenfolge-Invariante explizit: *„Type-Chaining-Check runs before Plugin-Registry-Lookup-Check — type-mismatch is a manifest-authoring-error independent of registry-state. Reverse order would mask manifest-bugs when the required plugin is absent."*, **And** alle drei Tests laufen via `cargo test -p klarvo-core --test pipeline_executor_e2e` headless ohne Audio-Device/Network/GUI, unter 5 Sekunden total, **And** die Keys-Submodule-Convention aus 1B.2 wird extended: `klarvo-core::pipeline::executor::keys` exposes `pub const PLUGIN_NOT_FOUND: &str = "error.pipeline.plugin_not_found"` + `pub const STAGE_TYPE_MISMATCH: &str = "error.pipeline.stage_type_mismatch"` (statisch referenzierbar aus Shell-Translation-Tables, Epic 4 Forward-Reference), **And** Test-Assertions gehen gegen `ErrorKind`-Variant + `user_message`-Key-Equality (nicht User-Strings — i18n-Discipline pro FR8).

**Given** FR36 Rustdoc-Contract + Cross-Epic-Forward-Reference-Kompletierung für Epic-1B-Closure,
**When** `run_pipeline`, `StageData`, `keys`-Submodule, `PluginRegistry`-Extension-Methods und das Pipeline-Executor-Module dokumentiert werden,
**Then** enthält das Module-Level-Rustdoc (a) Scope-Statement: *„Pipeline-Executor is the Runtime-Layer of Innovation-A's three-layer FR6 contract (Compile-Time Registry-Set in 1B.1, Boot-Time Parse in 1B.2, Boot+Runtime Dispatch here). All three layers share the hard-fail-no-warn-skip invariant per `memory/feedback_manifest_compile_contract`. The Executor closes Epic 1B (Pipeline-Composition Runtime)."*, (b) Boot-Check-Ordering-Note: *„Boot-time checks run in order: (1) Type-Chaining-Compat-Check, (2) Plugin-Registry-Lookup-Check. Rationale: Type-mismatch is a manifest-authoring-error independent of registry-state; checking it first surfaces manifest-bugs even when plugins are missing from the registry."*, (c) Per-Pipeline-Correlation-Note: *„Per-Pipeline-Correlation (correlation-IDs, tracing-spans, structured log-events) is Epic 6 Observability scope — not 1B.5. The Executor emits no `tracing` events in Phase 1; instrumentation points are reserved for Epic 6's NFR5/NFR6-respecting Observability layer."*, (d) Groq-Registry-Wire-Up-Note: *„E2E-Pipeline-Tests intentionally use Verbatim-based pipelines only (passthrough + verbatim). Groq-based pipelines require KeyStore-backed API-key-delivery (Epic 1C) and are first exercised end-to-end in Epic 2 (End-to-End Dictation Pipeline headless integration tests)."*, (e) 1A.6-Amendment-Note: *„`CleanupInput::from_raw(raw: String) -> Self` with `CleanupContext::default()` was added as Phase-1-amendment to 1A.6 in 1B.5 to enable dispatch-arm-internal Text→CleanupInput wrapping without forcing CleanupInput to be a distinct StageData-variant. Pipeline-Config-passed-CleanupContext (i18n-Output-Language-Axis per `memory/project_i18n_three_axes`) is Epic 4 / Phase-2+ scope."*, (f) Dispatch-Arm-Invariance-Note: *„Variant-extract operations in dispatch-arms (`let StageData::Audio(audio) = data else { unreachable!(...); };`) are guaranteed by the boot-time Type-Chaining-Check. Removing or weakening that check breaks this invariant — any future refactor that touches the boot-check must preserve the dispatch-arm-type-safety contract."*

---

### Epic 1C: KeyStore Abstraction

Plugin-Developer (z. B. Groq-Plugin-Author) konsumiert API-Keys via Trait-API — ohne Plain-Storage-Fußabdruck in Release-Builds zu vererben. Security-Theater explizit und auditable gated.

**FRs covered:** FR44, FR45, FR46 — 3 FRs

**Dependencies:** Epic 1A (AppError-Referenzierung für `KeyMissing`-Variante).

**Implementation Notes:**
- `PlainSqliteKeyStore` hinter `dev-keystore`-Cargo-Feature; nicht-enabled in Release-Builds. NFR4-Disclosure-Text (Security-Theater) dokumentarisch in Rustdoc + Release-Notes.
- OS-Keystore-Scaffolds (Windows-Credential-Manager, macOS-Keychain, Linux-Secret-Service) als Stub-Impls prepared — **ohne** `KeyStore`-Trait-Signature-Änderungen. Phase-4-Release-Default-Swap ist reine Feature-Flag-Umschaltung.
- Parallel zu Epic 2 lauffähig: Groq-Plugin-Integration mockt KeyStore initial (Story in Epic 2 oder Epic 1B), echter Wire-Up als spätere Story wenn 1C ready.

---

#### Story 1C.1: `KeyStore`-Trait + i18n-Keys + `InMemoryKeyStore`-Fixture (Contract-Layer)

**As a** Core-Developer or Plugin-Developer building BYOK-Cloud-Provider-Integrations,
**I want** einen Phase-1-stabilen `KeyStore`-Trait in `klarvo-core::keystore` mit async `get`/`set`/`delete`-API, `SecretString`-typed Values und `AppError::kind::KeyMissing`-Semantik wired an i18n-keyed User-Messages,
**So that** STT/LLM-Plugin-Authors (Groq, DeepSeek, OpenAI, Anthropic, OpenRouter, …) gegen eine stabile Abstraction-Layer für API-Key-Retrieval programmieren — unabhängig davon welches Backend (Phase-1 Plain-SQLite-dev-only via 1C.2, Phase-4 OS-Keystore-release-default via 1C.3 + Phase-4-Feature-Flip) am Ende compiled wird — und die Trait-Signature ist **unabhängig** vom 4-Trait-Stability-Ring (PipelineStage / SttProvider / CleanupStyle / VadProvider) stabilisiert (separate Concern-Category: Secret-Lifecycle, nicht Data-Flow).

**Acceptance Criteria:**

**Given** `klarvo-core`'s new `keystore`-Module (`klarvo-core/src/keystore/mod.rs` + `klarvo-core/src/keystore/trait_def.rs` + `klarvo-core/src/keystore/keys.rs`, Re-Export via `klarvo_core::keystore::{KeyStore, keys}` aus `lib.rs`),
**When** der `KeyStore`-Trait in `trait_def.rs` definiert wird,
**Then** hat der Trait die exakte Signature:
```rust
#[async_trait]
pub trait KeyStore: Send + Sync + 'static {
    async fn get(&self, key: &str) -> Result<SecretString, AppError>;
    async fn set(&self, key: &str, value: SecretString) -> Result<(), AppError>;
    async fn delete(&self, key: &str) -> Result<(), AppError>;
}
```
**And** verwendet `#[async_trait::async_trait]` als Object-Safety-Layer (analog zum Epic-1A-Trait-Pattern, `Arc<dyn KeyStore>` use-site-kompatibel via `Send + Sync + 'static`-Bounds für Registry-Einbettung), **And** `klarvo-core/Cargo.toml` konsumiert `secrecy = { workspace = true }` (bereits workspace-gepinnt via ADR-0004 `v1_import`-Scope) und `async-trait = { workspace = true }` (bereits workspace-gepinnt via Epic-1A-Trait-Pattern; **keine** Version-String-Literale im Crate-Cargo.toml), **And** `cargo check -p klarvo-core` kompiliert grün, **And** das `delete`-Method-Rustdoc trägt einen Contract-Clause (exakter Text): *„`delete` is idempotent: returns `Ok(())` whether the key existed before the call or not. Use `get` to verify pre-existence if a caller needs that semantic."*

**Given** ADR-0004 `SecretString`-Precedent + `project_api_key_os_keystore_mvp`-Policy (no-plain-String-leak-across-Trait-Boundary) + Andy-Verify-Flag-2 aus Epic-1C-Pre-Flight,
**When** Caller-Code den Trait konsumiert,
**Then** akzeptiert `set(&self, key: &str, value: SecretString)` den Secret ausschließlich als `SecretString`, und `get(&self, key: &str) -> Result<SecretString, AppError>` returnt den Secret ausschließlich als `SecretString`, **And** `SecretString::expose_secret()` wird **nie** innerhalb der Trait-Signature, in Default-Impl-Bodies oder in Rustdoc-Example-Code-der-Trait-Docs aufgerufen — Exposure ist ausschließlich Consumer-Call-Site-Concern (siehe 1B.4 Groq-`transcribe` Bearer-Header-Konstruktion als etablierten Precedent), **And** der Trait-Level-Rustdoc dokumentiert explizit (exakter Text): *„Callers must invoke `SecretString::expose_secret()` only at their immediate use-site (e.g., HTTP Bearer-header construction, platform-API-call). Never log, persist, forward, or clone the exposed value. Keep exposure scope as narrow as syntactically possible."*

**Given** 1A.2's `ErrorKind`-Enum (bereits in epics.md:352 mit initialen Variants inkl. **`KeyMissing`** deklariert — **kein** 1A.2-Amendment nötig, siehe Epic-1C-Pre-Flight-Flag-1-Resolution) + FR30 (Keystore-Miss surfaced als `AppError::kind::KeyMissing` mit `user_message`-Key und Plugin-Identifier in Cause-Chain),
**When** eine `KeyStore`-Trait-Impl einen Missing-Key oder Backend-Unavailability signalisiert,
**Then** definiert 1C.1 im Submodule `klarvo_core::keystore::keys` (File `keystore/keys.rs`) zwei **neue** i18n-Keys — `"error.keystore.not_found"` (Semantik: der angefragte `key`-Identifier existiert nicht im Backend) und `"error.keystore.backend_unavailable"` (Semantik: das KeyStore-Backend ist nicht verfügbar; primär konsumiert von 1C.3 OS-Keystore-Android-Scaffold-Stub und 1C.2 Plain-SQLite-Init-Fails) — beide exposed als `pub const KEY_NOT_FOUND: &str = "error.keystore.not_found";` und `pub const BACKEND_UNAVAILABLE: &str = "error.keystore.backend_unavailable";`, **And** Impls konstruieren Errors via `AppError::new(ErrorKind::KeyMissing).with_message(keys::KEY_NOT_FOUND).with_source(source)` (analog für `BACKEND_UNAVAILABLE`) mit dem angefragten `key`-String als Display-Context — Plugin-Identifier-Cause-Chain-Enrichment (per FR30) ist **Caller-Responsibility**, da die KeyStore-Impl den Plugin-Caller nicht kennt; der Key-String selbst kann Plugin-Identity per Naming-Prefix-Konvention encodieren (z. B. `"groq_api_key"`, `"deepseek_api_key"`), **And** die Keys sind durch 1A.4's `klarvo_core::i18n::assert_is_key`-Runtime-Assert als valid erkannt und werden durch FR34 `cargo xtask lint-events` (Epic 5) statisch in die Key-Inventory-Liste aufgenommen, **And** der `keys`-Submodule-Rustdoc trägt eine Forward-Reference-Note (exakter Text): *„Phase-2+ may extend with `PERMISSION_DENIED` (OS-Keystore user-ACL-dialog-denied, distinct from backend-unavailable) and `INIT_FAILED` (backend-initialization-failure distinct from runtime-unavailability). These are deliberately deferred in Phase 1 to keep the key-inventory minimal."*

**Given** das 4-Trait-Stability-Ring-Constraint (per `memory/project_epic_breakdown_phase1`-Addendum-#1: nur `PipelineStage`, `SttProvider`, `CleanupStyle`, `VadProvider` sind Phase-1-Stability-Anker) + Andy-Verify-Flag-3 aus Epic-1C-Pre-Flight,
**When** das `keystore`-Module dokumentiert wird,
**Then** enthält das Module-Level-Rustdoc in `klarvo-core/src/keystore/mod.rs` eine Separation-Note (exakter Text): *„`KeyStore` is a secret-lifecycle abstraction, architecturally separate from the 4 Phase-1-stability Data-Flow-Traits (`PipelineStage`, `SttProvider`, `CleanupStyle`, `VadProvider`). Stability guarantees for `KeyStore` are independently scoped: the Trait-Signature is locked in Phase 1, but backend-impl-swap (Phase-1 Plain-SQLite dev-only → Phase-4 OS-Keystore release-default per FR46) does not constitute a Trait-Signature change."*, **And** eine Non-Goals-Liste (exakter Text): *„Non-Goals for Phase 1: (a) `list()` / `keys()` for enumerating all stored keys — deferred to Phase 2+ when Settings-UI needs key-enumeration. (b) `exists(key)` / `contains(key)` — Phase-1 callers use `get(key).is_ok()`. (c) Batch-operations (`set_many`, `delete_many`) — deferred until usage-patterns emerge in Phase 2+."*, **And** keine Positionierung des Traits als „5th Pipeline-Stability-Trait" wird irgendwo im Module-Rustdoc, Trait-Rustdoc, Method-Rustdoc oder Module-Layout suggeriert.

**Given** FR36 (Rustdoc-Contract-Documentation) + Epic-1B-etablierte Intent/Contract/Example-Triple-Convention,
**When** `KeyStore`-Trait, `get`/`set`/`delete`-Methods, `keys`-Submodule und Submodule-Konstanten dokumentiert werden,
**Then** hat jedes Public-Item Rustdoc mit (a) **Intent**-Zeile (z. B. für `get`: *„Retrieve an API-key-secret from the configured backend by identifier."*), (b) **Contract-Condition** (z. B. für `get`: *„Returns `AppError::kind::KeyMissing` with `user_message = keys::KEY_NOT_FOUND` when no entry exists for `key`. Returns `AppError::kind::KeyMissing` with `user_message = keys::BACKEND_UNAVAILABLE` when the backend is unreachable or not compiled-in (e.g. OS-Keystore-Android-Scaffold-Stub pre-Phase-3)."*), (c) **Example** (Rust-Code-Snippet-Block zeigt Consumer-Usage-Pattern mit `expose_secret()` **inline** im unmittelbaren Header-Setter-Call, **ohne** Intermediate-Variable) — verbatim:
```rust
let api_key = store.get("groq_api_key").await?;
let response = client
    .post(endpoint)
    .header("Authorization", format!("Bearer {}", api_key.expose_secret()))
    .send()
    .await?;
```

**Given** 1A.1 `klarvo-test-fixtures`-Pattern + 1B.4's intra-crate KeyStore-Mock-Precedent (ad-hoc test-double, nicht shared-fixture),
**When** 1C.1 shippt,
**Then** erweitert 1C.1 `klarvo-test-fixtures` um einen `InMemoryKeyStore`-Struct in `klarvo-test-fixtures/src/keystore.rs`:
```rust
pub struct InMemoryKeyStore { store: tokio::sync::Mutex<HashMap<String, SecretString>> }

impl InMemoryKeyStore {
    pub fn empty() -> Self;
    pub fn with_pairs(pairs: impl IntoIterator<Item = (&'static str, SecretString)>) -> Self;
}

#[async_trait]
impl KeyStore for InMemoryKeyStore { /* get/set/delete, Missing-Key → AppError::KeyMissing + KEY_NOT_FOUND */ }
```
exposed via `klarvo_test_fixtures::InMemoryKeyStore`, **And** 1B.4's intra-crate Mock bleibt unverändert (kein Retroactive-Amendment an Epic 1B — Scope-Fence); der shared `InMemoryKeyStore` wird ab Epic 2 (Groq-Real-Wire-Up) als canonical-Test-Pattern konsumiert und ersetzt in jener Story optional 1B.4's intra-crate-Mock wenn Andy das dort separat anstößt.

---

#### Story 1C.2: `PlainSqliteKeyStore` Dev-Impl hinter `dev-plain-keystore`-Feature + NFR4-Disclosure

**As a** Core-Developer building den Phase-1-dogfooding-prototype,
**I want** eine `PlainSqliteKeyStore`-Implementierung des 1C.1-`KeyStore`-Traits, die API-Keys plain in einer lokalen SQLite-Datei speichert — konditional-compiled hinter dem `dev-plain-keystore`-Cargo-Feature und dokumentiert als NFR4-Security-Theater,
**So that** Andy + Phase-1-Sanity-Tester einen funktionalen KeyStore-Backend für BYOK-Workflows haben (Groq-Integration kann ab Epic 2 realen Wire-Up gegen `Arc<dyn KeyStore>` bauen), **ohne** dass der Plain-Storage-Codepfad je in Release-Builds landet — und die Security-Theater-Disclosure (NFR4) explizit im Dev-User-Rustdoc und in Phase-1-README verankert ist.

**Acceptance Criteria:**

**Given** `klarvo-core`'s bestehendes `keystore`-Module (aus 1C.1) + Cargo-Feature-Policy (per PRD-FR45 post-Amendment-Commit b21b771 + `memory/project_api_key_os_keystore_mvp`) + Andy-Pre-Flight-Flag-4 (Feature-Flag-Location),
**When** `klarvo-core/src/keystore/plain_sqlite.rs` angelegt und Cargo-Feature deklariert wird,
**Then** lebt der `dev-plain-keystore`-Cargo-Feature-Flag in **`klarvo-core/Cargo.toml`** unter `[features]` als `dev-plain-keystore = ["rusqlite"]` (Cargo-idiomatic optional-dep-Feature-Pattern), **And** `rusqlite = { workspace = true, optional = true }` wird in `[dependencies]` deklariert (workspace-gepinnt; **keine** Version-String-Literale im Crate-Cargo.toml), **And** `plain_sqlite.rs` ist in seiner Gesamtheit `#[cfg(feature = "dev-plain-keystore")]`-gated (Module-Level-Gate, nicht per-Item), **And** `klarvo-core/src/keystore/mod.rs` re-exportiert den Typ konditional: `#[cfg(feature = "dev-plain-keystore")] pub use plain_sqlite::PlainSqliteKeyStore;`, **And** Consumer-Crates (z. B. `klarvo-plugin-groq`, Test-Harnesses) propagieren das Feature via `[features] dev-plain-keystore = ["klarvo-core/dev-plain-keystore"]` wenn sie den konkreten Typ konsumieren wollen (Consumer-Propagation ist Consumer-Story-Scope, **nicht** 1C.2-Scope — 1C.2 etabliert nur die Feature-Quelle).

**Given** `PlainSqliteKeyStore`-Struct + Init-Semantics + Mutex-Wrapping für `Send + Sync + 'static`-Compliance des 1C.1-Trait-Bounds,
**When** der Struct + Konstruktoren definiert werden,
**Then** ist die Public-API:
```rust
pub struct PlainSqliteKeyStore {
    conn: tokio::sync::Mutex<rusqlite::Connection>,
}

impl PlainSqliteKeyStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError>;
    pub fn in_memory() -> Result<Self, AppError>;
}
```
(`tokio::sync::Mutex` für async-friendly Locking über await-Boundaries; `rusqlite::Connection` ist sync — rusqlite-Calls laufen inline innerhalb der async-Trait-Methods, **kein** `spawn_blocking`-Overhead für Phase-1-dev-only-Scope), **And** beide Konstruktoren führen Schema-Init aus:
```sql
CREATE TABLE IF NOT EXISTS api_keys (
    name TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
)
```
(Init-Fail → `AppError::new(ErrorKind::KeyMissing).with_message(keys::BACKEND_UNAVAILABLE).with_source(rusqlite_error)`), **And** `open` akzeptiert den Ziel-DB-Pfad explizit (nicht hardcoded; Caller steuert `keystore.db`-Location per PRD line 378; Tests nutzen `:memory:` via `in_memory()` — `:memory:` ist kein valider Filesystem-Path, daher separater Method-Weg statt Magic-String-Parameter), **And** **kein** `rusqlite_migration` wird verwendet (Single-Table-Schema Phase-1-stable; architecture.md:244-Migration-Engine gilt für Plugin-Migrations, nicht Core-KeyStore-Schema).

**Given** `KeyStore`-Trait-Impl + Error-Taxonomy-Mapping (1C.1-`keys`-Submodule) + `delete`-Idempotenz-Contract,
**When** `impl KeyStore for PlainSqliteKeyStore` implementiert wird,
**Then** mappt die Impl rusqlite-Errors auf `AppError` wie folgt:
- `get(&self, key) -> Result<SecretString, AppError>`: `SELECT value FROM api_keys WHERE name = ?1`. `rusqlite::Error::QueryReturnedNoRows` → `AppError::new(ErrorKind::KeyMissing).with_message(keys::KEY_NOT_FOUND).with_source(e)` mit dem angefragten Key-String als Display-Context im Cause-Chain (per 1C.1-AC-3-Caller-Convention). Anderer `rusqlite::Error` → `.with_message(keys::BACKEND_UNAVAILABLE).with_source(e)`. Success → `Ok(SecretString::new(row.get::<_, String>(0)?))`.
- `set(&self, key, value) -> Result<(), AppError>`: `INSERT OR REPLACE INTO api_keys (name, value) VALUES (?1, ?2)` (Upsert-Semantik — konsistent zu HashMap::insert-last-write-wins-Pattern + 1C.1-Idempotent-Delete-Spirit). `value.expose_secret()` inline im rusqlite-Params-Binding (**kein** Intermediate-`String`-Bind — narrow-expose per 1C.1-AC-2-Policy). Fail → `BACKEND_UNAVAILABLE`.
- `delete(&self, key) -> Result<(), AppError>`: `DELETE FROM api_keys WHERE name = ?1`. Rückgabe **immer** `Ok(())` bei erfolgreichem SQL-Execute — unabhängig vom `rows_affected`-Count (1C.1-Idempotenz-Contract). Fail → `BACKEND_UNAVAILABLE`.

**And** alle drei Methods locken `self.conn.lock().await` unmittelbar vor dem rusqlite-Call und halten den Lock bis zum Call-Ende (sequenzielle DB-Access, keine Race-Conditions; Phase-1-dev-only-Scope rechtfertigt keinen Connection-Pool).

**Given** NFR4 (Security-Theater-Disclosure) + PRD line 377-378 `README-phase1.md`-Zip-Bundle-Kontext + `memory/project_api_key_os_keystore_mvp`-Policy + Phase-2-Revisit-Forward-Reference für 1C.3-OS-Keystore-Implementations,
**When** 1C.2 shippt,
**Then** trägt das Module-Level-Rustdoc in `plain_sqlite.rs` zwei Text-Blöcke. **Block 1 — NFR4-Security-Disclosure** (exakter Text): *„`PlainSqliteKeyStore` stores API-keys in plaintext within a local SQLite file. This is **Security-Theater** (NFR4): a Windows-ACL-restriction on current-user read/write mitigates casual-access by other OS-users, but does **not** protect against privileged-process-read, disk-backup-extraction, or malware running as the same user. This implementation exists **only** behind the `dev-plain-keystore` Cargo-feature and is **never** compiled into release-builds. Real API-key-protection comes via the OS-Keystore-Impl (Phase-4 release-default per FR46, ref `memory/project_api_key_os_keystore_mvp`)."* **Block 2 — Phase-2+-Revisit-Note** (exakter Text): *„`rusqlite` calls are inline-blocking inside async-methods — acceptable for Phase-1-dev-only-scope. Phase-2+ OS-Keystore implementations (1C.3) may have non-trivial I/O latency (Windows Credential Manager roundtrips, Android IPC) and should evaluate `tokio::task::spawn_blocking` wrapping at that time."* **And** `docs/phase-1/README.md` (Source-File im Repo-Tree unter `docs/phase-1/` analog zu `docs/adr/` und `docs/migration/`-Struktur; 1C.2 appended oder created falls nicht vorhanden; PRD line 377-378 meint Zip-Bundle-Distribution-Root und ist für Source-Tree-Location neutral — Zip-Bundle-Copy-Step ist Scope einer späteren Release-Story) enthält eine Section `## Security: Plain-SQLite API-Key Storage` mit einer leserlichen Variante derselben NFR4-Disclosure + zusätzlichem Hinweis *„Phase-1-Builds are dogfooding-prototype only — do not treat local API-keys as production-secure. Rotate keys frequently if testing in shared environments."*

**Given** Feature-Gate-Correctness-Attestation + Integration-Test-Harness + Pattern-Parität zu 1B.4-`tests/external_contract.rs`,
**When** 1C.2 shippt,
**Then** verifiziert ein Integration-Test-File `klarvo-core/tests/plain_sqlite_keystore.rs` (strict Integration-Test-Scope, **nicht** Unit-Test-Module; File-Top-Gate `#![cfg(feature = "dev-plain-keystore")]` damit das ganze File ohne Feature compiler-invisible ist) die Contract-Roundtrip-Semantik über **7 Test-Cases**:
- set/get roundtrip — Value-Equality via `expose_secret()`-Compare
- get auf missing-key → `AppError::kind::KeyMissing` + `user_message == keys::KEY_NOT_FOUND`
- delete auf existing-key → `Ok(())`, subsequent get → `KEY_NOT_FOUND`
- delete auf non-existing-key → `Ok(())` (Idempotenz-Contract-Check)
- set auf existing-key (upsert) → `Ok(())`, subsequent get returnt neuen Wert
- `PlainSqliteKeyStore::in_memory()` als Test-Harness-Basis — keine File-I/O, keine Cleanup-Ritual
- `PlainSqliteKeyStore::open` auf einem Pfad mit **non-existent-parent-dir** (z. B. `/tmp/klarvo-test-nonexistent-xyz/keystore.db`) → `Err(AppError)` mit `kind == ErrorKind::KeyMissing` + `user_message == keys::BACKEND_UNAVAILABLE` + `source` wrapping den rusqlite/IO-Error (Init-Error-Mapping-Lock für Schema-Init-Fail-Pfad aus AC-2)

**And** `cargo check -p klarvo-core --features dev-plain-keystore` kompiliert grün, **And** `cargo check -p klarvo-core` (ohne Feature) kompiliert grün und `PlainSqliteKeyStore`-Symbol ist nicht reachable, **And** `cargo test -p klarvo-core --features dev-plain-keystore --test plain_sqlite_keystore` läuft alle 7 Cases grün, **And** `cargo test -p klarvo-core --test plain_sqlite_keystore` (ohne Feature) compiles-zu-leer (0 tests; File-Top-Gate greift), **And** Concurrent-Access-Stress / Read-Only-Path / PoisonError sind explizit out-of-scope für Phase-1-dev-only-Scope (tokio-Mutex poisoniert nicht; Phase-2+-Harden nur wenn Real-Use-Pattern Race-Conditions aufdeckt).

**Given** FR36 (Rustdoc-Contract-Documentation) + 1C.1-etablierte Intent/Contract/Example-Triple-Convention,
**When** `PlainSqliteKeyStore`, `open`, `in_memory`, `impl KeyStore for PlainSqliteKeyStore` dokumentiert werden,
**Then** hat jedes Public-Item Rustdoc mit (a) **Intent**-Zeile, (b) **Contract-Condition** (für `open`/`in_memory` explizit: *„Returns `AppError::kind::KeyMissing` with `user_message = keys::BACKEND_UNAVAILABLE` when SQLite-connection-init or table-CREATE fails."*), (c) **Example**-Snippet (zeigt `open(path)`-Flow + direkte `Arc<dyn KeyStore>`-Verwendung — Example hält sich an 1C.1-AC-5-Pattern `expose_secret()`-narrow bei etwaigem Secret-Handling), **And** `open`/`in_memory`-Rustdoc referenziert das Module-Level-NFR4-Disclosure als Disclaimer: *„See module-level documentation for the NFR4 security-disclosure. Use only in `dev-plain-keystore`-gated builds."*

---

#### Story 1C.3: OS-Keystore-Impl für Windows + Android-Scaffold (Phase-3-Deferred)

**As a** Core-Developer preparing den Phase-4-Release-Default-KeyStore-Backend,
**I want** eine `WindowsKeystore`-Implementierung (Windows-Credential-Manager via `windows-rs`) + eine `AndroidKeystore`-Scaffold-Stub-Impl (fail-soft via `AppError::kind::KeyMissing` + `error.keystore.backend_unavailable`, **kein** `todo!()`/panic per `feedback_scaffold_fail_soft_pattern`) — beide Platform-cfg-gated und den 1C.1-`KeyStore`-Trait implementierend ohne Trait-Signature-Änderung,
**So that** Phase-4-Release-Default-Swap (Toggle von `dev-plain-keystore` OFF → Platform-Native-Impl wird Boot-Default) reine Feature-Flag-Umschaltung bleibt ohne Caller-Code-Änderungen, und Phase-3 Android-Implementation einen stabilen Swap-Point hat ohne dass Phase-1-Android-Cross-Compile oder Runtime-Calls mit `todo!()`-Panics brechen.

**Acceptance Criteria:**

**Given** `klarvo-core/src/keystore/` (aus 1C.1 + 1C.2 bestehend) + Gate-1-Narrowing (macOS/Linux out of Phase-1-scope),
**When** die OS-Keystore-Submodule angelegt werden,
**Then** existiert die Module-Struktur:
```
klarvo-core/src/keystore/os/
├── mod.rs      # module-declaration + Platform-cfg-dispatch
├── windows.rs  # #[cfg(target_os = "windows")]-gated; real impl
└── android.rs  # #[cfg(target_os = "android")]-gated; scaffold-stub impl
```
**And** `keystore/os/mod.rs` deklariert Platform-cfg-dispatched Re-Exports:
```rust
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use windows::WindowsKeystore;

#[cfg(target_os = "android")]
pub mod android;
#[cfg(target_os = "android")]
pub use android::AndroidKeystore;
```
**And** `keystore/mod.rs` re-exportiert das Submodule: `pub mod os;`, **And** **keine** macOS- oder Linux-Stubs (explizit out-of-Phase-1-scope per Gate-1; Phase-5+ als eigene Story + ggf. ADR-0006 dann), **And** **keine** neuen Cargo-Features für die Platform-Impls — Platform-cfg alleine reicht; Phase-4-Release-Default-Swap erfolgt per `dev-plain-keystore`-Toggle-OFF (`PlainSqliteKeyStore` dropped aus Compile → Platform-Native-Impl bleibt einziger KeyStore-Provider), **nicht** per zusätzlichem `os-keystore`-Feature-Toggle-ON, **And** `cargo check -p klarvo-core` auf Linux-Host kompiliert grün (keines der Platform-Module wird aktiviert — Consumer sieht kein KeyStore-Impl, nur den 1C.1-Trait).

**Given** Windows-Credential-Manager-API + `windows-rs`-Crate (workspace-gepinnt per architecture.md:1270),
**When** `windows.rs` implementiert wird,
**Then** ist die Public-API:
```rust
#[cfg(target_os = "windows")]
pub struct WindowsKeystore {
    app_id: String,
}

#[cfg(target_os = "windows")]
impl WindowsKeystore {
    pub fn new(app_id: impl Into<String>) -> Self;
}

#[cfg(target_os = "windows")]
#[async_trait]
impl KeyStore for WindowsKeystore { /* get/set/delete per Signature 1C.1 */ }
```
(`app_id` dient als TargetName-Prefix; Phase-1-Default-Caller passt `"klarvo"`. Empty-or-whitespace `app_id` triggert `debug_assert!` im Constructor-Body — kein Result-return, infallible Constructor), **And** TargetName-Konvention: `format!("{}/{}", self.app_id, key)` (Slash-Separator; Windows-Credential-Manager-UI zeigt hierarchisch), **And** `get` ruft `CredReadW` mit `CRED_TYPE_GENERIC`: `ERROR_NOT_FOUND (1168)` → `AppError::new(KeyMissing).with_message(keys::KEY_NOT_FOUND).with_source(e)`, anderer Error → `BACKEND_UNAVAILABLE`, Success → UTF-8-decoded `CredentialBlob`-Bytes als `SecretString`, **And** `set` ruft `CredWriteW` mit `CRED_PERSIST_LOCAL_MACHINE` (Phase-1-Choice: machine-local, not-roaming, not-session-only) + `value.expose_secret()`-Bytes (UTF-8-encoded) inline im Params-Binding, **And** `delete` ruft `CredDeleteW`; Rückgabe **immer** `Ok(())` bei erfolgreichem Delete ODER bei `ERROR_NOT_FOUND` (Idempotenz-Contract aus 1C.1), anderer Error → `BACKEND_UNAVAILABLE`.

**Given** Gate-4 Scaffold-Fail-Soft-Pattern (`memory/feedback_scaffold_fail_soft_pattern`) + Phase-3-Blocker-Forward-Reference (`memory/project_play_store_phase3_blocker`),
**When** `android.rs` implementiert wird,
**Then** ist die Public-API:
```rust
#[cfg(target_os = "android")]
pub struct AndroidKeystore { app_id: String }

#[cfg(target_os = "android")]
impl AndroidKeystore {
    pub fn new(app_id: impl Into<String>) -> Self;

    fn phase3_scaffold_error() -> AppError {
        AppError::new(ErrorKind::KeyMissing)
            .with_message(keys::BACKEND_UNAVAILABLE)
            .with_source(io::Error::new(
                io::ErrorKind::Unsupported,
                "KeyStore not available on Android in Phase 1 — \
                 Phase-3 scope (AccessibilityService-Policy-Audit blocker, \
                 ref project_play_store_phase3_blocker)",
            ))
    }
}

#[cfg(target_os = "android")]
#[async_trait]
impl KeyStore for AndroidKeystore {
    async fn get(&self, _key: &str) -> Result<SecretString, AppError> { Err(Self::phase3_scaffold_error()) }
    async fn set(&self, _key: &str, _value: SecretString) -> Result<(), AppError> { Err(Self::phase3_scaffold_error()) }
    async fn delete(&self, _key: &str) -> Result<(), AppError> { Err(Self::phase3_scaffold_error()) }
}
```
**And** **keine** JNI-Calls, **keine** `jni`-Crate-Imports, **keine** Android-OS-API-Zugriffe — das Modul ist pure-Rust und kompiliert auf Android-Target **ohne NDK** (NDK ist erst in Phase-3-Real-Impl-Story erforderlich), **And** alle drei Trait-Methods returnen uniform `phase3_scaffold_error()` (intentionale Uniformität, nicht Code-Duplication — Trait-Signature bleibt stabil; Phase-3-Real-Impl ersetzt nur Method-Bodies pro Method durch echte JNI-Logik), **And** die Cause-Chain-Text-Konvention (Phase-3-scope + AccessibilityService-Policy-Audit + memory-Ref) ist load-bearing für FR30-Konformität und FR36-Rustdoc-Cross-Reference.

**Given** Phase-4-Release-Default-Swap-Invariante (per FR46 + `memory/project_api_key_os_keystore_mvp`) + Trait-Signature-Stability-Discipline aus 1C.1-AC-4,
**When** das `os`-Module dokumentiert wird,
**Then** enthält `os/mod.rs` ein Module-Level-Rustdoc mit Swap-Semantics-Dokumentation (exakter Text): *„Platform-native `KeyStore` implementations. Phase-1 provides `WindowsKeystore` (real, via `windows-rs` Credential-Manager) and `AndroidKeystore` (Phase-3-deferred scaffold-stub; all methods return `AppError::kind::KeyMissing` with `keys::BACKEND_UNAVAILABLE`). Phase-4-Release-Default-Swap: disabling the `dev-plain-keystore` Cargo-feature removes `PlainSqliteKeyStore` from the compile, leaving the platform-native impl as the only `KeyStore`-provider on-target. No `KeyStore`-Trait-Signature change is required — the swap is purely a compile-feature-flag toggle. macOS and Linux are explicitly out-of-scope for Phase 1; adding them is a Phase-5+ story with its own ADR."*, **And** `windows.rs` Module-Level-Rustdoc: *„Windows Credential Manager-backed `KeyStore` implementation. `TargetName` convention is `<app_id>/<key>`. Persistence-mode is `CRED_PERSIST_LOCAL_MACHINE` (machine-local, not roaming, not session-only)."*, **And** `android.rs` Module-Level-Rustdoc trägt zwei Text-Blöcke. **Block 1 — Scaffold-Status** (exakter Text): *„Android-Keystore scaffold-stub. All trait-methods return `AppError::kind::KeyMissing` with `keys::BACKEND_UNAVAILABLE` and a cause-chain explaining the Phase-3-Deferral. Real Android-Keystore integration (JNI + Android-Keystore-System-API per `memory/project_jni_dual_surface`) is Phase-3-scope, gated by AccessibilityService-Policy-Audit (ref `memory/project_play_store_phase3_blocker`). Trait-signature is stable across the Phase-3-swap — only method-bodies are replaced."* **Block 2 — Error-Type-Revisit-Note** (exakter Text): *„Phase-1-Scaffold uses `io::Error` as a lightweight source-wrapper. Phase-3-real-impl may introduce a dedicated `KeystoreBackendError`-type if Android-specific error-paths prove diverse enough to warrant dedicated taxonomy."*

**Given** Integration-Test-Pattern (1B.4-`external_contract` + 1C.2-`plain_sqlite_keystore`) + Cross-Platform-Cross-Compile-Verification + RAII-Test-Cleanup-Pattern (`memory/feedback_test_raii_cleanup_pattern`),
**When** 1C.3 shippt,
**Then** existiert ein Integration-Test-File `klarvo-core/tests/windows_keystore.rs` mit File-Top-Gate `#![cfg(target_os = "windows")]` — verifiziert Windows-Impl-Contract-Roundtrip-Semantik über **6 Cases** (analog zu 1C.2 ohne Init-Error-Case, da `WindowsKeystore::new` infallible ist):
- set/get roundtrip (Value-Equality via `expose_secret()`-Compare)
- get auf missing-key → `KEY_NOT_FOUND`
- delete auf existing-key → `Ok(())`, subsequent get → `KEY_NOT_FOUND`
- delete auf non-existing-key → `Ok(())` (Idempotenz)
- set auf existing-key (upsert) → `Ok(())`, subsequent get returnt neuen Wert
- TargetName-Prefix-Konvention verified: test erstellt Key, liest via raw `CredReadW` mit konstruktem `"{app_id}/{key}"`-TargetName, erwartet Success

**And** der Test nutzt einen test-unique `app_id` wie `"klarvo-test-{uuid::Uuid::new_v4()}"` **plus** ein RAII-Guard-Cleanup-Scope-Struct (test-local in `tests/windows_keystore.rs`; general Pattern per `memory/feedback_test_raii_cleanup_pattern`):
```rust
struct TestKeystoreScope {
    app_id: String,
    created_keys: Vec<String>,
}

impl Drop for TestKeystoreScope {
    fn drop(&mut self) {
        for key in &self.created_keys {
            let _ = /* CredDeleteW(format!("{}/{}", self.app_id, key)) */; // ignore-errors, never panic
        }
    }
}
```
Test-Code registriert Keys via `scope.register(key)` bei jedem `set`-Call; Drop-Impl garantiert Cleanup auch bei Panic mid-test. **Doppel-Isolation:** unique-UUID-Namespace (keine Kollision mit User/CI) + guaranteed-Cleanup (kein Leak in realen Windows-Credential-Manager-Store). **Explicit-Teardown-Loops am Test-Ende sind verboten** (panic-fragil, Privacy-Concern).

**And** existiert ein Integration-Test-File `klarvo-core/tests/android_keystore_scaffold.rs` mit File-Top-Gate `#![cfg(target_os = "android")]` — verifiziert Scaffold-Fail-Soft-Semantik über **2 Cases**:
- `AndroidKeystore::new("klarvo")` returnt Self (infallible)
- jede der drei KeyStore-Methods (`get`/`set`/`delete`) returnt `Err(AppError)` mit `kind == ErrorKind::KeyMissing` + `user_message == keys::BACKEND_UNAVAILABLE` + `source`-Chain-`Display` enthält Substrings `"Phase-3 scope"` UND `"AccessibilityService-Policy-Audit blocker"`

**And** `cargo check --target aarch64-linux-android -p klarvo-core` (Android-Cross-Compile-Verify auf Linux-Host, **ohne** NDK — das Scaffold ist pure-Rust) kompiliert grün — der Check setzt den `aarch64-linux-android`-rustup-Target voraus (`rustup target add aarch64-linux-android`, one-time-setup; für Phase-1-Win+Android-MVP-Scope per `memory/project_klarvo_v2_rebuild` legitimes Table-Stakes; CI-Integration dieses Cross-Compile-Checks landet in Epic 5 Developer-Gate-Infrastructure; Android-NDK ist **nicht** erforderlich), **And** `cargo check -p klarvo-core` (Linux-Host-default-target) kompiliert grün ohne eine der OS-Impls aktiviert zu haben.

**Given** FR36 + 1C.1/1C.2-etablierte Intent/Contract/Example-Triple-Convention,
**When** `WindowsKeystore`, `AndroidKeystore`, ihre `new`-Konstruktoren und die KeyStore-Trait-Impls dokumentiert werden,
**Then** hat jedes Public-Item Rustdoc mit (a) **Intent**-Zeile, (b) **Contract-Condition** (Windows: explizite Error-Mapping-Tabelle `ERROR_NOT_FOUND → KEY_NOT_FOUND`, other → `BACKEND_UNAVAILABLE`; Android: *„All methods unconditionally return `AppError::kind::KeyMissing` with `user_message = keys::BACKEND_UNAVAILABLE`. No successful get/set/delete path exists in the Phase-1-scaffold; full impl lands in Phase-3."*), (c) **Example**-Snippet (Windows: `let store = WindowsKeystore::new("klarvo"); let key = store.get("groq_api_key").await?;` + inline-`expose_secret()`-Pattern aus 1C.1-AC-5; Android: fail-soft-Pattern `match store.get(...).await { Err(e) if matches!(e.kind, ErrorKind::KeyMissing) => /* Phase-3-fallback */ , _ => unreachable!("Android scaffold returns KeyMissing uniformly") }`), **And** beide Impls referenzieren das `os/mod.rs`-Swap-Semantics-Rustdoc als Cross-Reference: *„See module-level documentation for Phase-4-Release-Default-Swap semantics."*

---

### Epic 2: End-to-End Dictation Pipeline (Headless Canonical Flow)

User (Andy) kann den kanonischen Hold-to-Talk-Workflow ausführen — Audio-Capture → VAD → STT → Cleanup → Delivery — verifizierbar als headless Integration-Test, bevor Shell-Integration passiert (PRD Journey 1 + Journey 3).

**FRs covered:** FR12, FR13, FR14, FR15, FR16, FR17, FR29 — 7 FRs

**Dependencies:** Epic 1A + Epic 1B (Runtime komplett verfügbar). Parallel mit Epic 1C möglich (KeyStore-Mock → echter Wire-Up).

**Implementation Notes:**
- Headless-First-Mandat: jede Story-AC enthält „läuft in headless integration test ohne Shell". Shell-Delivery-Target (FR17) ist Adapter-Interface-Definition, nicht konkrete Shell-Integration — das kommt in Epic 3.
- Groq-Failure-Recovery-AC (FR29 + NFR11): User kann Hotkey erneut triggern ohne App-Neustart.
- NFR2 (Drop-freier Audio-Capture): Audio-Capture-Thread-AC muss unabhängig von Downstream-Processing-Latency drop-frei sein.
- NFR3 ts_ms-Convention (session-relative monotone Caller-Clock) in Event-Emission-ACs.

---

#### Story 2.1: AudioSource-Trait + Core-Capture-Command-Surface (Contract-Layer)

**As a** Core-Developer wiring the dictation-pipeline,
**I want** einen Phase-1-stabilen `AudioSource`-Trait in `klarvo-core::audio` mit `CaptureConfig`-Injection, opaquem `CaptureHandle`-RAII-Drop und einer `AudioEvent`-Enum-Broadcast-Contract,
**So that** Shell-Implementierungen (Epic 3 Windows-cpal, Phase-3 Android-AudioRecord) gegen eine stabile Core-Abstraction programmieren können, Core-Tests via `MockAudioSource` (aus `klarvo-test-fixtures`) headless laufen, und die `ts_ms`-Invariante (NFR3 session-relative monotone Caller-Clock) in der Trait-Rustdoc verankert ist — bevor eine einzige Zeile cpal-Code die Shell betritt.

**Acceptance Criteria:**

**Given** `klarvo-core/src/audio/events.rs` neu angelegt wird (per architecture.md §Directory-Structure: `audio/events.rs` ist separate Datei für Broadcast-Channel-Types),
**When** die Datei definiert wird,
**Then** enthält sie exakt:
```rust
#[derive(Debug, Clone)]
pub enum AudioEvent {
    Samples { data: std::sync::Arc<[f32]>, ts_ms: u64 },
    Level   { rms: f32,                    ts_ms: u64 },
}
```
**And** Rustdoc auf `AudioEvent::Samples.ts_ms` trägt exakten Text: *"Timestamp of chunk START, caller-monotone ms since session-start (ref ADR-0001, memory/project_event_ts_ms_convention). AudioSource-impls hold one `Instant` captured in `start()` and derive `ts_ms = instant.elapsed().as_millis() as u64` for each emitted chunk."*, **And** `klarvo-core/src/audio/mod.rs` deklariert `pub mod events;` und re-exportiert `pub use events::AudioEvent;`, **And** `klarvo_core::audio::AudioEvent` ist als Public-API-Surface erreichbar, **And** `cargo check -p klarvo-core` kompiliert grün.

**Given** `klarvo-core/src/audio/source.rs` neu angelegt wird und `AudioError` dort co-located mit dem Trait definiert wird (analog `vad/provider.rs`-Pattern),
**When** `AudioError` definiert wird,
**Then** hat `AudioError` genau zwei Varianten, ist `#[non_exhaustive]` und nutzt `thiserror::Error`:
```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AudioError {
    #[error("audio device unavailable")]
    DeviceUnavailable,
    #[error("unsupported audio format")]
    UnsupportedFormat,
}
```
**And** Rustdoc auf dem Enum trägt exakten Text: *"`#[non_exhaustive]` — Epic 3 cpal-impl and Phase-3 Android-impl will extend this enum with hardware-specific variants (e.g. `PermissionDenied`, `CaptureInterrupted`). The `#[non_exhaustive]` attribute prevents match-exhaustiveness-breaks at consumer call-sites when variants are added."*, **And** Compile-Test verifiziert `AudioError: Send + Sync + 'static` (inline `fn _assert<T: Send + Sync + 'static>() {} _assert::<AudioError>();`).

**Given** `klarvo-core/src/audio/keys.rs` neu angelegt wird (parallel zum `keystore/keys.rs`-Muster aus Story 1C.1),
**When** die i18n-Key-Konstanten definiert werden,
**Then** enthält die Datei:
```rust
pub const DEVICE_UNAVAILABLE: &str = "error.audio.device_unavailable";
pub const UNSUPPORTED_FORMAT: &str  = "error.audio.unsupported_format";
```
**And** `klarvo-core/src/audio/mod.rs` deklariert `pub mod keys;` sodass `klarvo_core::audio::keys::{DEVICE_UNAVAILABLE, UNSUPPORTED_FORMAT}` öffentlich erreichbar sind, **And** beide Strings entsprechen der `error.<domain>.<reason>` dot-notation-Konvention per `memory/project_i18n_core_contract`, **And** Rustdoc auf `DEVICE_UNAVAILABLE` trägt: *"Used by Epic 3 cpal-impl (`WindowsCpalAudioSource::start`) when the OS-audio-device is unavailable. Phase-1 `MockAudioSource` never emits this key — defined here to co-locate the error-contract with the Trait-definition (precedent: `memory/project_keystore_trait_surface` 1C.1-pattern)."*, **And** Rustdoc auf `UNSUPPORTED_FORMAT` trägt: *"Emitted when the impl cannot resample/downmix to the advisory 16 kHz mono f32 format. Explicitly named in ADR-0006-Rustdoc as the concrete error-path from `CaptureConfig.sample_rate` advisory-miss."*, **And** ein Key-Format-Unit-Test prüft: `assert!(DEVICE_UNAVAILABLE.starts_with("error.audio.")); assert!(UNSUPPORTED_FORMAT.starts_with("error.audio."));`.

**Given** `CaptureConfig` und `CaptureHandle` in `klarvo-core/src/audio/source.rs` definiert werden,
**When** diese Typen angelegt werden,
**Then** hat `CaptureConfig` genau:
```rust
pub struct CaptureConfig {
    /// Advisory sample-rate. Impls resample to 16 kHz if possible.
    pub sample_rate: u32,
    /// Advisory channel-count. Impls downmix to mono if possible.
    pub channels: u16,
    /// Broadcast-sender; AudioSource publishes AudioEvent variants here.
    pub events: tokio::sync::broadcast::Sender<AudioEvent>,
}
```
**And** `CaptureHandle` ist ein **opaker** Struct ohne öffentliche Felder oder Methoden (exakter internal-Shape ist Impl-internal; ein `tokio::sync::oneshot::Sender<()>` oder äquivalentes Shutdown-Signal ist ein valides private-Field-Muster), **And** Rustdoc auf `CaptureHandle` trägt exakten Text: *"Drop-guard that stops the capture-session and releases OS resources. Hold this value for the lifetime of the capture session; dropping it signals the capture-thread to terminate. Downstream consumers observe `RecvError::Closed` on their broadcast-receivers after the handle is dropped. Panic-safe: `Drop` fires unconditionally on scope-exit, including via panic-unwind (ref `memory/feedback_test_raii_cleanup_pattern`)."*, **And** Rustdoc auf `CaptureConfig.events` trägt: *"Caller constructs the channel via `tokio::sync::broadcast::channel(klarvo_core::audio::DEFAULT_AUDIOEVENT_CAPACITY)` and holds the corresponding `Receiver` for downstream consumption. AudioSource implementations publish `AudioEvent` variants autonomously during an active capture-session."*, **And** Compile-Tests verifizieren `CaptureConfig: Send + 'static` und `CaptureHandle: Send + 'static`.

**Given** `AudioSource`-Trait in `klarvo-core/src/audio/source.rs` definiert wird,
**When** der Trait angelegt wird,
**Then** hat er die exakte Signatur:
```rust
#[async_trait::async_trait]
pub trait AudioSource: Send + 'static {
    async fn start(
        &mut self,
        config: CaptureConfig,
    ) -> Result<CaptureHandle, AudioError>;
}
```
**And** der Trait erscheint **nicht** als `PluginRegistry`-Slot (`register_audio_source` oder ähnliches existiert nicht in `registry.rs`) — AudioSource ist Infrastructure-Category, nicht Plugin-Contract (per ADR-0006-Sub-Decision-6, per `memory/project_phase1_trait_narrowing`), **And** `klarvo-core/src/traits/mod.rs` re-exportiert `pub use crate::audio::source::AudioSource;` sodass `klarvo_core::traits::AudioSource` erreichbar ist (per ADR-0006 Trait-Location-Statement), **And** das Module-Level-Rustdoc auf dem Trait enthält alle folgenden Clauses (exakter Text):
- **(a) Infrastructure-Scope:** *"Infrastructure-Trait (per ADR-0006, Accepted 2026-04-19). Not part of the 4-Trait-Data-Flow-Stability-Ring (`PipelineStage` / `SttProvider` / `CleanupStyle` / `VadProvider`). `AudioSource` is Infrastructure-Category, analogous to `KeyStore` (Epic 1C): one Shell-Binary-scoped impl per platform — never registry-looked-up. Impls live in `shells/windows-tauri/` (Epic 3 cpal) and `shells/android/` (Phase 3 AudioRecord). Core holds only the Trait."*
- **(b) ts_ms-Obligation:** *"Implementations MUST set `ts_ms` on emitted `AudioEvent::Samples` to the chunk-START timestamp, derived from a single `Instant` captured at the start of `start()`, as session-relative monotone milliseconds (ref ADR-0001, `memory/project_event_ts_ms_convention`). Downstream consumers can compute chunk-end as `ts_ms + (data.len() as u64 * 1000 / 16_000)`."*
- **(c) Sample-Format:** *"Implementations SHOULD resample and downmix to 16 kHz mono f32 before emitting `AudioEvent::Samples` (Whisper-standard per ADR-0006-Sub-Decision-2). If the hardware cannot satisfy the advisory `CaptureConfig.sample_rate` / `.channels`, return `AudioError::UnsupportedFormat`. Emitted chunk-size is implementation-internal (example: ~1024 samples = 64 ms @ 16 kHz, subject to OS-audio-driver granularity per ADR-0006-Amendment-Q2)."*
- **(d) `&mut self` rationale:** *"`&mut self` prevents parallel-invocation of `start()` on a single `AudioSource` instance, making the borrow-checker a compile-time guard against overlapping capture-sessions. Multi-session requires multiple `AudioSource` instances (per ADR-0006-Sub-Decision-5, analogous to ADR-0001 §Resolved-Q5 for `VadProvider`)."*
- **(e) Forward-Refs:** *"Windows-cpal-Impl (`WindowsCpalAudioSource`) is Epic 3 scope (`shells/windows-tauri/`). Android-AudioRecord-Impl is Phase-3 scope. Phase-1 Core-tests use `klarvo_test_fixtures::MockAudioSource`. `AudioBuffer` (aggregate-type for `StageData::Audio`) and `audio/buffer.rs` are Story 2.2 scope; `AudioEvent::Samples` chunks are the raw stream that Story 2.2 aggregates."*

**Given** `klarvo-core/src/audio/mod.rs` aktualisiert wird,
**When** das Modul die neuen Submodule aufnimmt,
**Then** enthält `mod.rs`:
```rust
pub mod events;
pub mod keys;
pub mod source;
pub mod vad; // existing, unchanged

pub use events::AudioEvent;
pub use source::{AudioError, AudioSource, CaptureConfig, CaptureHandle};

pub const DEFAULT_AUDIOEVENT_CAPACITY: usize = 256;
```
**And** `klarvo_core::audio::DEFAULT_AUDIOEVENT_CAPACITY` ist als `256_usize` erreichbar (ADR-0007-Amendment-Q1), **And** Rustdoc auf `DEFAULT_AUDIOEVENT_CAPACITY` trägt: *"Default `tokio::sync::broadcast` channel capacity for `AudioEvent` streams. At ~1024 samples per chunk (64 ms @ 16 kHz), 256 slots ≈ 16 s of audio-backlog before a consumer lags. Pass to `broadcast::channel(DEFAULT_AUDIOEVENT_CAPACITY)` or override for testing (e.g., capacity-1 for deterministic lag-simulation in ADR-0007 backpressure tests). Ref ADR-0007-Amendment-Q1."*, **And** `klarvo-core/src/lib.rs` re-exportiert `pub mod audio;` (sofern noch nicht vorhanden — prüfen gegen bestehende `lib.rs`-Deklaration und hinzufügen falls fehlend).

**Given** `klarvo-test-fixtures` um `MockAudioSource` erweitert wird (Minimal-Synthetic-Chunk-Emitter per U3-Resolution + `memory/feedback_premature_abstraction_guard`),
**When** `MockAudioSource::with_synthetic_chunks(count, samples_per_chunk, chunk_interval_ms)` konstruiert und `start(config)` aufgerufen wird,
**Then** gilt:
```rust
impl MockAudioSource {
    pub fn with_synthetic_chunks(
        count: usize,
        samples_per_chunk: usize,
        chunk_interval_ms: u64,
    ) -> Self;
}
#[async_trait::async_trait]
impl AudioSource for MockAudioSource { ... }
```
**And** `start(config)` spawned einen `tokio::task`, der `count` mal `AudioEvent::Samples { data: Arc::from(vec![0.0_f32; samples_per_chunk]), ts_ms: i as u64 * chunk_interval_ms }` (i = 0-basierter Index) auf `config.events` sendet, dann den internen `Sender`-Clone droppt (→ Downstream `RecvError::Closed`), **And** `chunk_interval_ms = 0` ist ein valider Wert (schnellstmögliche Emission ohne `tokio::time::sleep` — für Unit-Tests die keine Realtime-Simulation brauchen), **And** `CaptureHandle::drop()` vor Count-Erschöpfung stopt die Emission vorzeitig (via Shutdown-Signal-Coordination), **And** `MockAudioSource` emittiert **niemals** `AudioError` (returnt immer `Ok(handle)`) — kein OS-State, kein Failure-Path, **And** Rustdoc trägt: *"Test fixture implementing `AudioSource`. Emits synthetic zero-filled chunks at the specified rate. `chunk_interval_ms = 0` for fastest-possible emission (unit-tests); nonzero for backpressure-simulation (ADR-0007 lag-tests). WAV-file-playback variant is Story 2.4 scope. Factor-out deferred until Story 2.4 proves the need (ref `memory/feedback_premature_abstraction_guard`)."*, **And** `MockAudioSource` lebt in `klarvo-test-fixtures/src/audio_source.rs`, re-exportiert als `klarvo_test_fixtures::MockAudioSource`.

**Given** alle obigen Definitionen vorliegen,
**When** `cargo test -p klarvo-core -p klarvo-test-fixtures` läuft,
**Then** passt ein `#[cfg(test)]`-Block in `klarvo-core/src/audio/source.rs` folgende Compile-Tests:
- `audio_error_send_sync_static`: `fn _assert<T: Send + Sync + 'static>() {} _assert::<AudioError>(); _assert::<CaptureConfig>(); _assert::<CaptureHandle>();` — kein Laufzeit-Assert nötig
- `audio_key_format`: `assert!(klarvo_core::audio::keys::DEVICE_UNAVAILABLE.starts_with("error.audio.")); assert!(klarvo_core::audio::keys::UNSUPPORTED_FORMAT.starts_with("error.audio."));`

**And** ein Integration-Test in `klarvo-test-fixtures/tests/mock_audio_source.rs`:
- **`mock_emits_exact_chunk_count`**: `MockAudioSource::with_synthetic_chunks(3, 1024, 64)` → Consumer empfängt genau 3 `AudioEvent::Samples`-Events mit `data.len() == 1024` und `ts_ms ∈ {0, 64, 128}`, danach `RecvError::Closed`.
- **`mock_early_drop_stops_emission`**: Consumer droppt `CaptureHandle` nach dem ersten empfangenen Chunk → Consumer empfängt `RecvError::Closed` bevor alle 3 Chunks eintreffen.

**And** kein `PluginRegistry`-Slot für `AudioSource` existiert (kein `register_audio_source` in `registry.rs`).

**Cross-References:**
- ADR-0006 (AudioSource-Trait-Signatur, CaptureConfig/Handle-Shape, alle Sub-Decisions) — **autoritative**
- ADR-0007-Amendment-Q1 (`DEFAULT_AUDIOEVENT_CAPACITY = 256`)
- `memory/project_event_ts_ms_convention` (ts_ms chunk-start-Semantik)
- `memory/project_phase1_trait_narrowing` (AudioSource = Infrastructure-Category, nicht Ring-Member)
- `memory/project_executor_stage_data_shape` (AudioBuffer-Forward-Ref: `StageData::Audio(AudioBuffer)` — Buffer-Definition Story 2.2)
- `memory/project_i18n_core_contract` (Core emittiert Keys, Shell resolved)
- `memory/feedback_premature_abstraction_guard` (MockAudioSource minimal-first, WAV-deferred)
- `memory/feedback_test_raii_cleanup_pattern` (CaptureHandle-Drop-Panic-Safety)
- `memory/project_keystore_trait_surface` (1C.1-Precedent für i18n-key-co-location-in-Contract-Story)

---

### Epic 3: Windows Shell Integration

Andy triggert Dictation auf Windows via Global-Hotkey und sieht das Ergebnis im aktiven Window eingefügt. Phase-1-Persona-complete UX (Tray + Auto-Paste).

**FRs covered:** FR19, FR20, FR21, FR22, FR23, FR24 — 6 FRs

**Dependencies:** Epic 1A (Bindings-Generation für FR24) + Epic 2 (Pipeline-Invocation existiert).

**Implementation Notes:**
- FR24 (Bindings-only-Consumption) + Epic 5 FR33 (bindings-drift-gate) werden gemeinsam enforced.
- FR20 + FR23 sind die einzigen UI-Touchpoints Phase 1 — ACs direkt aus FR-Wording, kein UX-Spec-Input nötig (uxSpec: none, dogfooding-prototype-Persona).
- Android-Scope-Fence: **keine Android-Shell-Stories** in Epic 3. Android-Shell ist Phase 3, gated durch AccessibilityService-Policy-Audit (`project_play_store_phase3_blocker`).

---

### Epic 4: Configuration & Localized Error Surface

User editiert `config.toml` für Plugin-Auswahl + 3-Achsen-i18n-Setup und sieht localized Errors in gewählter UI-Language. Sanity-Tester-Onboarding-Pfad (PRD Journey 4).

**FRs covered:** FR25, FR26, FR27, FR28, FR30, FR31 — 6 FRs

**Dependencies:** Epic 1A (i18n-Keys + AppError-Shape).

**Implementation Notes:**
- 3-Achsen-Model (UI-Language / Dictionary-Language / Output-Language) als **unabhängige** Config-Felder — nicht ein einzelnes Feld (ref `memory/project_i18n_three_axes.md`).
- FR28 (`PipelineValidation`) + FR30 (`KeyMissing`) sind i18n-keyed AppError-Varianten. Shell-Translation via owned Tables (z. B. `de.json`), G3-Gate (Epic 5 FR34) erzwingt mechanisch, dass Core keine User-Strings leakt.

---

### Epic 5: Developer-Gate Infrastructure

Core-Dev und Plugin-Dev haben mechanische Pre-Commit-Gates, die fail-loud auf Contract-Violations reagieren. Iteration ohne Regression-Risk.

**FRs covered:** FR32, FR33, FR34, FR35 — 4 FRs

**Dependencies:** Epic 5 kann unabhängig gestartet werden, aber Test-Value manifestiert sich erst mit Epic 1B (FR32 manifest-strict braucht Manifest-Format) + Epic 3 (FR33 bindings-drift braucht existierende Bindings-Konsumtion). Delivery-Plan: parallel mit Epic 1A/1B/2 sinnvoll.

**Implementation Notes:**
- FR35 (verify-release G2) und FR34 (lint-events G3) sind Phase-0-etabliert — Epic 5 **extends + hardens**, re-initialisiert nicht.
- FR32 enforced Epic 1B FR6-Invariante auf xtask-Ebene (Pre-Commit-Mirror der Boot-Time-Executor-Strictness).
- Persona-Achsentrennung: dies ist Plugin-Author-Tooling, kein End-User-Feature.

---

### Epic 6: Observability & Privacy-Respecting Diagnostics

Andy kann Failures via structured Logs debuggen, ohne BYOK-Privacy-Narrative zu verletzen. Log-Export-Stub bereit für Phase-2-UI-Expansion.

**FRs covered:** FR37, FR38, FR39, FR40 — 4 FRs

**Dependencies:** Epic 6 unabhängig (kann jederzeit nach Epic 1A starten).

**Implementation Notes:**
- NFR5 (kein Audio/Text im Log) + NFR6 (keine Outbound-Calls außer user-konfigurierter BYOK-Upstream) sind implizite Invarianten in Story-ACs.
- FR40 ist explizit Phase-1-Stub; UI-triggered-Zip-Generation-Forward-Reference → Phase 2 als Inline-Notiz im Epic-6-Stub-Story-AC.
- NFR1 (Latency-Observable via ts_ms im Log) verbindet Epic 6 + Epic 2.

---

### Epic 7: V1→V2 Data Migration Path

V1-User (Andy) kann Dictation-History via CLI-Subcommand nach V2 migrieren — einmalig, API-Keys + Polished-Mode-Settings bewusst exkludiert.

**FRs covered:** FR41, FR42, FR43 — 3 FRs

**Dependencies:** Epic 7 unabhängig (Phase-0 hat `v1_import`-Modul parse-only bereits committed).

**Implementation Notes:**
- v1_import parse-only Phase-0-done (commits `aefa1aa` + `7346af4`, ADR-0004). Epic 7 completes **write-to-v2-AppData**-Schritt.
- v1-Tauri-Identifier `com.klarvo.voice` bereits verifiziert (`memory/reference_klarvo_v1_tauri_identifier.md`).
- FR43 Exclude-Policy-AC: Verbatim-only-V2-Forward-Reference → Polished-Mode-Rebuild Phase 2 als Inline-Notiz (kein Platzhalter-Story, nur Kommentar).
