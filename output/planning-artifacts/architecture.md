---
stepsCompleted: [1, 2, 3, 4, 5]
inputDocuments:
  - output/planning-artifacts/product-brief-klarvo.md
  - output/planning-artifacts/product-brief-klarvo-distillate.md
  - docs/index.md
  - docs/project-overview.md
  - docs/v1-architecture-snapshot.md
  - docs/source-tree-analysis.md
  - docs/component-inventory.md
  - docs/development-guide.md
  - docs/rebuild-discussion.md
workflowType: 'architecture'
project_name: 'klarvo'
user_name: 'Andy'
date: '2026-04-17'
note: 'No formal PRD exists. Product Brief + Distillate (2026-04-17) serve as primary source per Andy-confirmed workflow (memory: reference_product_brief.md). v1 docs/ used as brownfield reference only — v2 is Clean Slate.'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Funktionale Anforderungen (10 Cluster, ~40-45 MVP-Features):**

| Cluster | MVP-Inhalt | Architektur-Implikation |
|---------|-----------|------------------------|
| **Core Pipeline** | STT → Text-Filter → Cleanup → Output → History-Save, Min-Duration, Hallucination-Filter, Prompt-Stripping, Output-Language | Deklarative Pipeline-Stages, Trait-basiert, TOML-Manifest |
| **Recording Modes** | Hold, Toggle, AutoStop (Win); alle 5 Android-Modi | Platform-Adapter für Input-Trigger, State-Machine im Core |
| **Hotkey System** | 2 Slots, Pause/Resume, ShortcutRecorder, Active-Mode-Badge | Windows-nativ; JNI-Bridge auf Android |
| **Text Processing** | Verbatim (Default), Chat, Polished (neu gebaut), Auto-Capitalize | `CleanupStyle`-Trait, Plugin-Crates |
| **Audio** | Device-Selection, RMS-VAD, Live-Events, WAV-Encoding | `AudioFilter`-Trait, cpal cross-platform |
| **Providers** | Groq Whisper + DeepSeek (Default), STT-Priority-List, Fallback, Live-Key-Validation | `SttProvider`+`LlmProvider`-Traits, Plugin-Registry |
| **UI** | Settings-Panel, Pill Bar (Win), Bubble (Android, 17 Features), Return-Focus, Tray, Onboarding, StylePicker, History-Panel | Hybrid: React WebView + native Overlays |
| **History & Stats** | Dictation History, Delete, Clear All | SQLite-Tabelle; Sync kommt in P1 |
| **Dictionary** | Custom Dictionary (capped) | Plugin-Daten via Migration-Trait |
| **Lizenz-System** | HMAC-Validation, Permanent, Trial, 30-Tage-Cache, 48h-Grace | Core-Modul, cross-platform, Cargo-Feature-Gating |

**Nicht-funktionale Anforderungen:**

| NFR | Anforderung |
|-----|-------------|
| Plattform-Parität | 0 LOC Geschäftslogik-Duplikat zwischen Core und Shells (v1-Baseline: ~2.000 LOC dupliziert) |
| Testbarkeit | `klarvo-core` headless testbar ohne UI |
| Latenz | Kein messbarer Overhead im Diktat-Flow — IPC-basierte Plugin-Architekturen wurden abgelehnt |
| Modularität | Neuer Provider/Style ohne Shell-Änderung — nur Plugin-Crate + Feature-Flag |
| Sicherheit | API-Keys in SQLite statt JSON; v1-Risiken (Plain-Text-Keys, CSP aus, Test-Lizenzen) dürfen nicht portiert werden |
| Privacy/Sovereignty | BYOK-Default, kein Anbieter-Lock-in, Offline-Pfad (P1/P2) architektonisch vorbereitet |
| Multi-Language | Deutsch first-class, Dictionary/Provider/Output-Language unabhängig konfigurierbar |
| Lizenzierung | PolyForm NC 1.0.0, Free/Paid via Cargo-Feature-Split (nicht Runtime-Limit) |
| Onboarding | Fresh Install → erstes erfolgreiches Diktat < 2 Min |
| Migration | v1 → v2 Einmal-Import (History, Dictionary, API-Keys, Hotkey-Config) in einem Klick |
| Regressions-Disziplin | Specs-vor-Code-Workflow, headless Core ab Phase 0, Review-Disziplin bei Pipeline-Änderungen |

**Scale & Complexity:**

- Primär-Domäne: Desktop + Mobile-App mit nativen OS-Integrationen (Hotkeys, Overlays, AccessibilityService, IME)
- Komplexitäts-Level: **High** — Multi-Plattform-nativ + Plugin-System + BYOK + Lizenz-System + Sync/Offline-Pfad als strategisches Portfolio
- Architektonische Komponenten (Schätzung): 1 Core-Crate + ~10 Plugin-Traits + ~5-8 initiale Plugin-Crates + 2 Shell-Apps (Tauri/Win, Kotlin/Android) + ~3 shared Utility-Crates

### Technical Constraints & Dependencies

**Harte Constraints:**
- Rust-Workspace ab Phase 0 (nicht optional)
- Null Tauri-spezifischer Code in `klarvo-core` — jeder Tauri-Import im Core ist Architektur-Verletzung
- Compile-time Plugins (Trait + Cargo-Feature); dynamische `.so/.dll`-Plugins explizit abgelehnt (ABI-Stabilität cross-platform)
- Gleicher Prozess — separate Prozesse + IPC abgelehnt wegen Diktat-Latenz
- SQLite via `rusqlite` + Turso (libsql) für Sync
- Hybrid-UI: React WebView für App-UI + native Overlays für FloatingBar/Bubble
- Lizenz: PolyForm Noncommercial 1.0.0 (Plugin-Lizenzen müssen kompatibel sein)

**Offene Constraints (im Workflow zu klären):**
- Android AccessibilityService vs. Google-Play-Policy (spätestens vor Phase 3)
- JNI-Bridge-Design (Core-Exposure, Serialisierung)
- Hotkey-Slot-Skalierung MVP (2) → Post-MVP (4-5) Trigger-Bedingung
- WASM-Kompatibilität der Traits (v2.x-Vorbereitung, keine Blocker)

**Dependencies aus v1:** tokio, rusqlite, reqwest, cpal, whisper-rs, llama-cpp-2 · Tauri v2 (nur Win) · React 19 + TypeScript 5.8 + Tailwind 4 · Kotlin, Silero VAD, MNN (Android)

### Cross-Cutting Concerns

1. **Plugin-Registry & Lifecycle** — Init-Reihenfolge, Fehler-Propagation, Hot-Reload (P1)
2. **Pipeline-Manifest (TOML)** — Validierung, Fehler-Messages, dynamisches Reordering
3. **Plugin-Migrations** — `Plugin::migrations() -> Vec<Migration>`, idempotent, Rollback-Strategie
4. **Konfigurations-Hybrid** — System-Settings (TOML) vs. User-Settings (SQLite `settings(key, value)`)
5. **API-Key-Storage-Evolution** — MVP: SQLite plain; v2.x: OS-Keystore/Master-Password ohne Format-Break
6. **Error-Handling & Fallback-Ketten** — STT-Priority-List, User-facing Errors, Observability
7. **Cross-Platform Audio-Capture** — cpal auf Win, AudioRecord auf Android; gemeinsame Event-Schnittstelle
8. **Cross-Platform Hotkey-Registration** — Win: global-shortcut; Android: AccessibilityService + Bubble
9. **Observability** — Logging, Telemetrie (BYOK-Kontext!), Cost-Tracking (P1), Crash-Reports
10. **Licensing-Integration** — Compile-time Feature-Gating + Runtime HMAC-Validation, Interaktion definieren
11. **Security Baseline** — CSP aktiv, keine Test-Lizenzen in Release-Builds, Secret-Handling in Config-Dateien
12. **Testability** — Headless-Test-Harness für `klarvo-core`, Mock-Plugins, Plattform-Integration-Tests
13. **v1 → v2 Migration** — Einmal-Import berührt Schema-Kompatibilität, API-Key-Migration aus Plain-JSON, History-Format-Transformation; muss als First-Class-Concern entworfen sein, nicht als nachgelagertes Feature
14. **Update/Release-Mechanismus** — Tauri Updater (Win-only) vs. Google Play Store (Android) vs. Lizenz-Check-Interaktion (Update-Gate bei abgelaufener Lizenz?); beeinflusst Crate-Boundaries (Updater-Logik im Core vs. Shell) und muss vor Phase 1 geklärt sein
15. **i18n / Multi-Language (drei unabhängige Achsen)**
    - **UI-Language:** Sprache der App-Oberfläche (Settings, Onboarding, Fehler-Messages) — i18n-Stack im Frontend + Shell-Strings
    - **Dictionary-Language:** Sprache des Custom Dictionary pro Eintrag (Domänen-Vokabular kann multilingual sein)
    - **Output-Language:** Sprache, in die das Diktierte transkribiert/umgeformt wird (STT-Sprache + Cleanup-Prompt-Sprache)
    - Jede Achse hat eigene Persistenz, eigene Default-Logik, eigene UI-Exposure

## Starter Template Evaluation

### Primary Technology Domain

Multi-Shell Rust-Workspace mit nativen Plattform-Shells (Desktop via Tauri v2, Mobile via Kotlin Android). **Kein Single-Starter-Template** deckt diese Kombination ab — bewusst Custom-Scaffolding.

### Starter Options Considered

- **`create-tauri-app` (Tauri v2.10.x)** — Standard für Desktop-Shell, unterstützt Mobile-Target (iOS+Android) über `tauri android init`. Bringt React + TypeScript + Vite + Tailwind out-of-box. **Nur für Win-Shell verwendet.**
- **Bare Cargo Workspace (manuell)** — Für `klarvo-core`, `klarvo-plugin-*`, Utility-Crates. Kein Template-Overhead, volle Kontrolle über Crate-Boundaries.
- **Android Studio „Empty Compose Activity"** — Kanonischer Kotlin-Android-Einstieg. Kein CLI, über IDE-Wizard.
- **Tauri Mobile (Android-Target)** — **Explizit abgelehnt**. Grund: v1-Bypass-Lehre (~85% Android-Bypass, ~2.000 LOC Duplikat), weil Tauri-IPC auf Android die falsche Abstraktion für Overlay-Bubble + AccessibilityService + IME-first-class ist. Android-Code-Sharing passiert über Shared Rust Core via JNI, nicht über Tauri.

### Selected Approach: Custom Cargo Workspace + Tauri-Template (Win) + Android Studio (Android)

**Rationale:**
- Shared Rust Core als `klarvo-core`-Crate ist die zentrale Architektur-Investition; kein vorhandenes Template trifft das
- Tauri-Template liefert fertige Win-Shell mit WebView + Frontend-Stack
- Android-Shell bleibt nativ Kotlin + JNI-Bridge zum Rust-Core

### Initialization Commands (Phase 0)

```bash
# 1. Root workspace
mkdir klarvo && cd klarvo
# klarvo/Cargo.toml als [workspace] mit members-Liste

# 2. Core + Plugin-Crates (manuell)
cargo new --lib klarvo-core
cargo new --lib klarvo-plugins/klarvo-plugin-groq
cargo new --lib klarvo-plugins/klarvo-plugin-deepseek
cargo new --lib klarvo-plugins/klarvo-plugin-verbatim
# ... weitere Plugins nach Bedarf

# 3. Windows-Shell via Tauri-Template
npm create tauri-app@latest -- --template react-ts \
  --manager npm --identifier de.klarvo.app shells/windows

# 4. Android-Shell: Android Studio → "Empty Compose Activity"
#    → shells/android/ (kein CLI, IDE-Wizard)

# 5. xtask-Crate für Build-Orchestration
cargo new --bin xtask
```

### Architectural Decisions Provided by Starter-Combination

**Language & Runtime:**
- **Rust 2024 Edition** (Upgrade von v1/2021) — async closures, verbesserte Lifetime-Rules, `unsafe extern`-Blocks, bessere Temporary-Scopes. `cargo fix --edition` automatisiert Migration weitgehend
- Kotlin 2.x + Jetpack Compose (Android)
- TypeScript 5.8 strict (Frontend)

**Styling Solution:**
- Tailwind CSS 4.x (Vite-Plugin) + minimale Custom Utility-Klassen
- Keine CSS-in-JS-Library (Tauri-WebView-Kontext)
- Jetpack Compose Material3 (Android-Native-UI)

**Build Tooling:**
- Cargo-Workspace mit Resolver v3
- Vite 7 für Frontend-Bundling (Tauri-Template-Default)
- Gradle 8.x + Android Gradle Plugin (AGP) latest (Android)

**Build-Orchestration (neu — wird in Step 4 detailliert):**
- Multi-Shell-Workspace braucht einen Koordinator für „alles bauen", Cross-Shell-Tests, CI-Matrix
- **Option A: `cargo xtask`-Pattern** — eigener `xtask`-Crate im Workspace mit Sub-Commands (`cargo xtask build-all`, `cargo xtask test-core`, `cargo xtask ci`); pure Rust, keine zusätzliche Runtime-Dependency
- **Option B: `just`-Recipes** — `justfile` im Root, ruft Cargo/Gradle/npm-Commands; menschenlesbar, aber zweite Sprache
- Empfehlung (wird in Step 4 bestätigt): **`cargo xtask`** als Haupt-Koordinator; `justfile` optional für häufige Dev-Shortcuts
- CI-Matrix: Windows-Build (Tauri) + Android-Build (Gradle AGP) + Core-Tests (headless Linux) in GitHub Actions

**Testing Framework (Kandidaten — Endscope in Step 4):**
- Rust: `cargo test` + `mockall` für Traits + `insta` für Snapshot-Tests der Cleanup-Outputs
- Frontend: Vitest (Unit) + optional Playwright (E2E der Tauri-Shell)
- Kotlin: JUnit5 + MockK + Espresso (Android-UI)

**Code Organization:**
```
klarvo/
├── Cargo.toml              # [workspace] root
├── klarvo-core/            # Shared core, headless, testbar
├── klarvo-plugins/         # Plugin-Crates (Cargo-Features gegated)
│   ├── klarvo-plugin-groq/
│   ├── klarvo-plugin-deepseek/
│   ├── klarvo-plugin-verbatim/
│   └── ...
├── shells/
│   ├── windows/            # Tauri + React Frontend
│   └── android/            # Kotlin + Compose + JNI-Bridge
├── xtask/                  # Build-Orchestration
├── pipeline-manifest.toml  # Deklarative Pipeline-Konfiguration
├── justfile                # (optional) Dev-Shortcuts
└── docs/
```

### Open Decisions (werden in Step 4 geklärt)

- **Offline-LLM:** `llama-cpp-2` (v1-Pfad, C/CMake-Build) vs. `mistral.rs` (Pure-Rust, vereinfacht Android-JNI, kleinere Community)
- **Offline-STT:** `whisper-rs` 0.16 (v1-Investment) vs. Candle whisper
- **Build-Orchestration:** `cargo xtask` allein vs. `xtask + just` kombiniert
- **Frontend-Test-Framework:** Vitest-only vs. Vitest + Playwright
- **State-Management Frontend:** Zustand vs. Redux Toolkit vs. Context+useReducer

### Verification-Todos vor Phase-0-Start

- **`llama-cpp-2`-Version klären:** v1-`Cargo.toml` spezifiziert 0.1.140, crates.io-Abfrage (April 2026) meldet 0.1.133 als latest. Entweder v1 referenziert eine yanked/pre-release-Version, oder Cargo.lock resolved anders. Vor Phase-0-Start gegen crates.io verifizieren und ggf. in v2-Cargo.toml korrigieren. **Nicht architektur-blockierend.**
- **`whisper-rs` SemVer-Pinning:** `whisper-rs-sys` kann in Patch-Releases breaken — `Cargo.lock` committed halten, Dependabot für sichtbare Upgrades

**Note:** Phase-0-Implementation beginnt mit Workspace-Init + Core-Crate-Skelett + Trait-Definitions (`SttProvider`, `CleanupStyle`, `OutputTarget`, `PluginRegistry`). Erst dann wird die Win-Shell via Tauri-Template generiert. Das xtask-Crate wird parallel aufgebaut, sobald Core-Tests laufen.

## Core Architectural Decisions

### Decision Priority Analysis

**Kritisch — blockieren Phase 0:**
Trait-Menge & Signaturen (#1.1, #1.2) · Plugin-Registration (#1.3) · JNI-Bridge-Strategie (#3.1) · API-Key-Storage-Architektur (#2.4) · Build-Orchestration (#7.1) · Core-API-Surface (#3.4)

**Wichtig — vor Phase 1:**
Config-Hybrid-Schema (#2.2) · Migrations-Tooling (#2.1) · CSP-Policy (#4.3) · Rust↔React-IPC (#3.3) · Audio-Abstraktion (#8.1) · Frontend-State (#5.1) · HMAC-Schlüssel (#4.1)

**Deferred auf P1/Post-MVP:**
Turso-Sync-Strategie (#2.6) · Offline-LLM-Auswahl (#6.2) · Code-Signing-Upgrade (#7.6) · Release-Kanäle (#7.7) · Erste VoiceCommand-Impl (#1.1)

### 1. Plugin-System & Trait-Design

| Zelle | Entscheidung | Rationale |
|-------|-------------|-----------|
| **Trait-Menge** | 7 Traits in Phase 0: `SttProvider`, `LlmProvider`, `CleanupStyle`, `TextFilter`, `OutputTarget`, `AudioFilter`, `PluginMigration`. `VoiceCommandHandler` als Stub-Trait (Interface definiert, keine Impl) | VAD bleibt Core-intern; VoiceCommand-Trait-Stub verhindert spätere Breaking-Changes |
| **Trait-Signaturen** | `async_trait` für Object-Safety (`Box<dyn SttProvider>`); Rust 2024 `impl Trait in Trait` nur wo Object-Safety nicht benötigt | `Box<dyn>` ist notwendig für Plugin-Registry-Collections |
| **Plugin-Registration** | Manuell in `klarvo-core::PluginRegistry::bootstrap()` via Cargo-Feature-gated `cfg`-Module | Klar, debuggbar, keine Linker-Magie; Feature-Gating entspricht Free/Paid/Nischen-Build-Strategie |
| **Pipeline-Manifest** | Embedded Default im Binary + optional User-Override im User-Data-Dir | User-Override explizit aktivieren (Pro-Feature-Gate möglich) |
| **Plugin-Error-Kontrakt** | Eigener `PluginError`-Enum mit `#[non_exhaustive]`; Kategorien `Network`, `Auth`, `RateLimit`, `Fatal`, `Unavailable` | Ermöglicht saubere Fallback-Ketten (STT-Priority-List); `anyhow` bleibt für App-Internes |

### 2. Data Architecture

| Zelle | Entscheidung | Rationale |
|-------|-------------|-----------|
| **SQLite-Migrations** | Custom `PluginMigration`-Trait + `rusqlite_migration` als Exec-Engine; Core sammelt Plugin-Migrations + fährt idempotent | Plugins steuern eigene Tabellen, Core orchestriert |
| **Config-Hybrid** | System-TOML (~5 Felder: DB-Pfad, App-Lang-Default, Telemetry-Flag, Dev-Mode) + User-SQLite-Tabelle `settings(key, value, type)` | Plain-JSON aus v1 abgeschafft |
| **Settings-Schema** | Typisiertes Key-Value + Rust-Accessor-Layer mit Serde-Validierung. **Revisit-Point nach Phase 1** (bei >20 Settings oder ersten strukturierten Composites): Hybrid mit dedizierten Tabellen für Composites (`hotkey_slots`, `provider_priorities`) | Einfach-Start, skaliert weich mit Scope |
| **API-Key-Storage (Revidiert)** | `KeyStore`-Trait ab Phase 0 mit zwei Impls: `PlainSqliteKeyStore` (Dev/Debug-Builds via Cargo-Feature) + `OsKeystoreImpl` (Release-Default, Windows Credential Manager + Android Keystore via JNI). **MVP-Release ist nicht vollständig ohne OsKeystoreImpl (Phase 4 Lock-in).** v1→v2-Migration geht direkt aus `config.json` in OS-Keystore (nie Zwischenstopp in Plain-SQLite) | Plain-SQLite ≡ Plain-JSON kryptographisch; Format-Upgrade ≠ Security-Upgrade. OS-Keystore ab MVP fest |
| **History-Schema** | v1-Schema als Baseline + additive Felder (`plugin_id`, `manifest_version`, `output_language`) | Migration-Trait v1→v2 garantiert Import |
| **Turso-Sync (P1)** | Batch alle 60s + On-Demand-Trigger. MVP ohne Sync | Sync ist P1-Feature, kein MVP-Blocker |

### 3. IPC-Boundaries

| Zelle | Entscheidung | Rationale |
|-------|-------------|-----------|
| **Kotlin↔Rust JNI (Revidiert)** | **Dual-Surface-Strategie.** Control-Plane (Commands): `uniffi` für Request/Response-Funktionen. Data-Plane (Streams): Raw `jni`-Crate + Kotlin-Callback-Interface für High-Frequency-Events. Broadcast-Channels im Core → JNI-Callback → Kotlin-Flow via `callbackFlow`-Adapter. **Phase-0-Spike (1 Tag):** Audio-Level-Meter end-to-end prototypen; bei uniffi-Reibung Fallback auf `jni`-only | uniffi hat verifiziert **keine Stream-Support** (confirmed via [uniffi docs](https://mozilla.github.io/uniffi-rs/latest/futures.html)); Audio-Events brauchen kontinuierliche Emission |
| **JNI-Serialisierung** | Direkte uniffi-Typen auf Control-Plane; `serde_json` über JNI-String für Data-Plane-Events wenn Strukturen komplex werden | Pragmatisch; Performance-Check im Phase-0-Spike |
| **Rust↔React (Tauri)** | Tauri Commands für Request/Response + Tauri Channels (v2-Feature) für Live-Events (Audio-Level, Transcription-Progress) | v1-Pattern beibehalten; Channels haben weniger Overhead als Events für High-Frequency |
| **Core-API-Surface** | Grob-körnig: `start_recording`, `stop_recording`, `transcribe`, `set_style`, `get_history`, `get_devices`, `apply_pipeline_manifest`. Interne Pipeline bleibt im Core verborgen | Stable-API-Boundary; reduziert Breaking-Changes in Shells |

### 4. Security & Licensing

| Zelle | Entscheidung | Rationale |
|-------|-------------|-----------|
| **HMAC-Schlüssel** | Hardcoded + Obfuscated im Binary (String-Obfuscation via `obfstr`-Crate) | Realistisch für PolyForm-NC-Client; reicht gegen Casual-Piracy |
| **Lizenz-Cache** | SQLite-Eintrag in `settings`-Tabelle mit HMAC-Signature über den Cache-Payload | Konsistent mit anderen Settings |
| **CSP (Tauri)** | Strikt default mit expliziten Exceptions für WebView-Assets. v1-Sünde „CSP aus" wird NICHT portiert | Security-Baseline aus v1-Security-Report |
| **Secret-Handling im Build** | Dev: `.env`-File (gitignored); Prod: API-Keys durch Nutzer via UI eingegeben, niemals hardcoded | v1-Sünde „Test-Lizenzen in Prod" + „Hardcoded Secrets in config.json" |
| **Telemetrie (Revidiert)** | **Keine Remote-Telemetry.** Tracing-Stack (`tracing` + `tracing-subscriber`) → Rolling-File in User-Data-Dir (max 10MB, 5 Rotations). Settings-Panel „Debug-Export" erzeugt Zip (Logs + redacted Config + Sys-Info) für User-triggered-Upload. Panic-Hook schreibt Stack-Traces in denselben Stream als `level=ERROR` | Opt-in Sentry widerspricht BYOK-Narrativ aus Brief („your data stays yours"); lokale Logs + User-Export alignen mit Positionierung |

### 5. Frontend-Architektur (React / Tauri-WebView)

| Zelle | Entscheidung | Rationale |
|-------|-------------|-----------|
| **State-Management** | Zustand (mit persist-Middleware für UI-State) | Minimal, debuggbar, kein Boilerplate; Redux-Toolkit-Overhead nicht gerechtfertigt bei ~10 Store-Slices |
| **Routing** | Tab-basiertes State-Routing (kein React Router) | Klarvo ist keine Website; Settings/History/Onboarding als Panels |
| **Form-Handling** | React Hook Form + Zod für Settings-Forms | Validierung kann Rust-Schema-Contract mit Zod spiegeln |
| **Animation** | Tailwind-Animations + CSS-Transitions | Pill-Bar-State-Transitions Rust-getriggert (nicht JS) |
| **Test-Framework** | Vitest Unit ab Phase 0; Playwright E2E ab P1 | Unit-Coverage kritisch, E2E deferred |

### 6. Offline-AI-Stack

| Zelle | Entscheidung | Rationale |
|-------|-------------|-----------|
| **Offline-STT** | `whisper-rs` 0.16 beibehalten (Cargo.lock pinning wegen SemVer-Warnung auf `whisper-rs-sys`) | v1-Investment, Modell-Support breiter; `SttProvider`-Trait macht späteren Swap kostenlos |
| **Offline-LLM** | **Deferred auf P1/P2-Entscheidung.** Architektonisch vorbereitet durch `LlmProvider`-Trait. Vorläufige Präferenz: `mistral.rs` (Pure-Rust, vereinfacht Android-JNI durch Wegfall von CMake/libclang), aber Eval in P1 vs. `llama-cpp-2` | Keine MVP-Dringlichkeit; Trait-Abstraktion erlaubt späte Entscheidung |
| **GPU-Support (Windows)** | MVP/P1: CPU-only. CUDA via Cargo-Feature `gpu-cuda` in P2 | Solo-Dev-Scope-Management |

### 7. Build & Release

| Zelle | Entscheidung | Rationale |
|-------|-------------|-----------|
| **Build-Orchestration** | `cargo xtask` als primärer Koordinator; `justfile` optional für Dev-Shortcuts | Pure Rust, im Workspace versioniert; keine zusätzliche Runtime-Dependency |
| **CI-Plattform** | GitHub Actions mit Matrix (Windows-Build, Android-Build, Linux-Core-Tests) | v1-Kontext, OSS-frei |
| **CI-Matrix-Strategie** | PR: Core-Tests + Lint + Format + Smoke-Build; Merge-to-main: Full Matrix + E2E | Feedback-Loop < 5 min bei PRs, Full-Check nur bei Merges |
| **Update Windows** | Tauri Updater | v1-Pattern, funktioniert |
| **Update Android (Revidiert)** | **Play Store als Primär-Distribution, Phase-3-Blocker.** Vor Phase-3-Start: AccessibilityService-Policy-Audit als Pflicht-Deliverable (Ticket mit Google Developer Support, Justification-Text, Fallback-Plan). Phase 3 wird nicht nur mit funktionierender APK abgeschlossen, sondern mit Play-Submission (oder bestätigtem Fallback-Plan falls Policy-Ablehnung). Fallback: APK-Direct + F-Droid + UX-Anpassung Sideload-Onboarding | Nicht-Techie-Nutzer (Tertiär, RSI-Secondary) sind realistisch nur über Play erreichbar; AccessibilityService-Policy-Risiko zentral |
| **Code-Signing** | MVP-Beta: Self-signed. EV-Cert bei Userbase > 100 (SmartScreen-Threshold). Android: Play-App-Signing (wenn Play-Release) oder Keystore-managed | Kosten-Nutzen für Solo-Dev |
| **Release-Kanäle** | Stable + Beta ab MVP-Release. Beta via Tauri-Updater-Channel-Flag | Deferred bis MVP-Release, aber Architektur-Support eingebaut |

### 8. Audio-Pipeline-Abstraktion

| Zelle | Entscheidung | Rationale |
|-------|-------------|-----------|
| **Capture-Abstraktion** | `AudioSource`-Trait im Core; Implementations in Shells (cpal-based Win, AudioRecord-based Android) | Shared-Core-Prinzip |
| **Event-Flow** | `tokio::sync::broadcast`-Channels im Core für `AudioEvent`-Enum (Samples, VAD-State, Level). Bridge-Layer serialisiert nach außen (Tauri-Channel auf Win, JNI-Callback auf Android) | Core-Semantik stabil; Serialisierungs-Impl-Detail fällt mit #3.1-Spike |
| **Puffer-Format** | f32-Samples intern, Konvertierung am Rand | Audio-Standard |
| **WAV-Encoding** | Im Core via `hound` | v1-Pattern, funktioniert |

### Decision Impact Analysis

**Implementation-Sequence (Phase 0):**
1. Cargo-Workspace + `klarvo-core` + `xtask`-Crate Skelett
2. Core-Traits definieren (7 + VoiceCommand-Stub) + `PluginRegistry`
3. `KeyStore`-Trait + beide Impls + Migration-Tooling
4. `AudioSource`-Trait + Core-interne Pipeline-State-Machine
5. JNI-Bridge-Spike (Audio-Level-Meter end-to-end) → uniffi vs. raw-jni commit
6. Pipeline-Manifest-TOML-Parser + Validation
7. Erste Plugin-Crate-Skelette (`klarvo-plugin-groq`, `klarvo-plugin-verbatim`)
8. Headless-Test-Harness für Core + erste Trait-Mock-Impls

**Cross-Component Dependencies (kritische Ketten):**
- `KeyStore`-Trait-Design → blockiert STT/LLM-Plugin-Auth-Flow → blockiert Provider-Plugin-Implementierungen
- JNI-Spike-Ergebnis (#3.1) → determiniert Audio-Event-Serialisierung (#8.2) → determiniert Android-Shell-Event-Listener-Design
- Pipeline-Manifest-Parser → blockiert Plugin-Registration-Bootstrap → blockiert integrative Tests
- `PluginMigration`-Trait + v1-Schema-Kompat → blockiert v1→v2-Import-Button (MVP-Erfolgskriterium)

**Aktualisierte Erfolgskriterien (Ergänzungen):**
- **OS-Keystore ab MVP-Release** (nicht deferred); Plain-SQLite-KeyStore nur in Dev-Builds aktiv
- **Play-Store-Policy-Klärung als Phase-3-Blocker** (konkrete Audit-Deliverables vor Phase-3-Start)
- **Lokale Logs + User-triggered-Export als einziger Observability-Pfad** (kein Remote-Telemetry)
- **Phase-0-JNI-Spike als Gate** für uniffi-Commit vs. raw-jni-Fallback

Sources für Step 4 Research:
- [UniFFI Async/Future Support](https://mozilla.github.io/uniffi-rs/latest/futures.html) — verifiziert: kein Stream-Support
- [Tokio Streams](https://tokio.rs/tokio/tutorial/streams)
- [Tokio Broadcast Channels](https://tokio.rs/tokio/tutorial/channels)

## Implementation Patterns & Consistency Rules

### Pattern Categories Defined

**14 kritische Konflikt-Punkte identifiziert und adressiert** in fünf Kategorien: Naming, Struktur, Format (IPC-Boundary), Communication, Process. Jede Entscheidung ist kodifiziert, damit unterschiedliche AI-Agents kompatiblen Code schreiben.

Die folgenden Patterns gelten **workspace-weit** für alle Crates, Shells und Frontend-Code. Abweichungen sind nur zulässig mit explizitem Rationale-Kommentar im Code.

### Reference-Block — Accepted Defaults

Diese Defaults folgen etablierten Standards und sind **ohne Diskussion** verbindlich. Agents müssen sie kennen — „keine Regel gefunden, also mache ich irgendwas" ist der konkrete Anti-Pattern, den dieser Block verhindert.

**Rust (Rust API Guidelines):**
- Types/Traits/Enums: `PascalCase`; Akronyme als Camel-Gruppe (`SttProvider`, `HttpClient` — NICHT `STTProvider`)
- Kein `Trait`-Suffix, kein `I`-Prefix — der Trait heißt `SttProvider`, nicht `SttProviderTrait` oder `ISttProvider`
- Funktionen/Module/Variablen: `snake_case`
- Konstanten/Statics: `SCREAMING_SNAKE_CASE`
- Crate-Namen: hyphen (`klarvo-core`, `klarvo-plugin-groq`) — Rust mappt automatisch auf underscore im Code (`use klarvo_plugin_groq::...`)

**SQL/SQLite:**
- Tabellen: `snake_case`, **plural** (`histories`, `settings`, `hotkey_slots`, `plugin_migrations`, `schema_migrations`)
- Spalten: `snake_case` (`created_at`, `plugin_id`, `output_language`)
- Foreign-Keys: `<singular>_id` (`plugin_id`, nicht `fk_plugin`)
- Indexes: `idx_<table>_<cols>` (`idx_histories_created_at`)
- Timestamps: **immer `BIGINT` Unix-Epoch-Millis UTC** (konsistent zum IPC-Date-Format, siehe Format Patterns unten)

**TypeScript/React:**
- Components: `PascalCase.tsx`, 1 Component pro File (`SettingsPanel.tsx`)
- Non-Component-Files: `kebab-case.ts` (`audio-level-formatter.ts`) — visuell unterscheidbar von Components
- Hooks: `useCamelCase` (`useHotkeys`, `useTranscription`)
- Types/Interfaces: `PascalCase` ohne `I`-Prefix (`Settings`, nicht `ISettings`)
- Variablen/Funktionen: `camelCase`

**Kotlin/Android:**
- Composables + Classes: `PascalCase` (`BubbleOverlay`, `SettingsScreen`)
- Funktionen/Variablen: `camelCase`
- Files matchen Top-Level-Class-Namen (`BubbleOverlay.kt` enthält `BubbleOverlay`-Composable/Class)

**TOML (Configs, Pipeline-Manifest):**
- Keys + Section-Headers: `snake_case` (`[plugins.groq]`, `api_key = "..."`)

**Cargo Features:**
- Hyphen-separated
- Gruppen-Prefix verpflichtend: `plugin-*`, `offline-*`, `gpu-*`, `dev-*`
  - Beispiele: `plugin-groq`, `plugin-deepseek`, `offline-stt`, `offline-llm`, `gpu-cuda`, `dev-plain-keystore`

**Rust-Tests:**
- Unit: `#[cfg(test)] mod tests` co-located in Source-File
- Integration: `<crate>/tests/*.rs`
- Shared-Fixtures: via `klarvo-test-fixtures` dev-dependency (siehe Structure Patterns)
- Snapshot-Tests (`insta`): `.snap`-Files vom Crate selbst gemanaged

**Error-Handling (bestätigt aus Step 4 §1, hier als Reference):**
- Plugin-Boundary: `PluginError` mit `#[non_exhaustive]`, Kategorien `Network` / `Auth` / `RateLimit` / `Fatal` / `Unavailable`
- Intern (App-Code): `anyhow::Result` für Kontext-reiche Fehler
- Intern (Library-Code): `thiserror` für typed Errors, mit `From`-Impls zu `PluginError` wo relevant
- `unwrap()` / `expect()` nur in Tests oder mit explizitem Invarianten-Kommentar (`// SAFETY: registry is guaranteed initialized by bootstrap()`)

**Logging (`tracing`-Stack):**
- `error`: User-sichtbar ODER Datenverlust-Risiko
- `warn`: Degradiert (Fallback aktiviert, Config-Default verwendet, Retry vor finalem Fail)
- `info`: State-Transitions (Recording-Start/Stop, Plugin-Load, License-Validation)
- `debug`: Entwickler-Kontext (Payload-Sizes, Timing, Provider-Selection-Reasons)
- `trace`: Hot-Path-Details (pro Audio-Frame, pro Token-Stream-Chunk)
- Spans: pro Pipeline-Stage (`stt.transcribe`, `cleanup.apply`, `history.persist`), **nicht** pro Request
- Log-Timestamps: ISO-8601 (menschliche Konsumenten); IPC-Event-Timestamps hingegen i64 Millis (Maschinen-Konsumenten)

**i18n-Keys (Concern #15, erste Achse — UI-Language):**
- Format: dot-notation `<feature>.<element>.<purpose>` mit `snake_case`-Segmenten
- Beispiele: `settings.save_button.label`, `settings.api_key.validation_error`, `history.empty_state.title`, `onboarding.step_welcome.cta`
- Namespace matched Frontend-Feature-Ordner: `settings.*` → `features/settings/`, `history.*` → `features/history/`
- Shared/Generic Keys unter `common.*`-Präfix: `common.cancel`, `common.save`, `common.retry`, `common.error`
- **`user_message`-Field in `AppError` hält i18n-Key, NICHT übersetzten String** — Frontend resolved zum Display-Zeitpunkt. Konkret: `user_message: Some("history.delete_failed".into())`, nicht `Some("Löschen fehlgeschlagen".into())`
- **Library-Choice: P1-ADR.** React-i18next vs. Lingui vs. Custom-Loader wird entschieden wenn zweite UI-Language (jenseits Deutsch) konkret ansteht. MVP-Keys werden bereits strukturiert angelegt, damit jede Library sie konsumieren kann. Naming-Anker jetzt genügt, Tooling-Commitment später.

### Naming Patterns

#### Event-Naming (cross-boundary: Tauri-Events, JNI-Callbacks, interne Broadcasts)

**Format:** `<domain>.<event>` — dot-notation, lowercase, kebab-case für multi-word Events.

**Beispiele:**
- `recording.started`, `recording.stopped`, `recording.paused`, `recording.resumed`
- `audio.level`, `audio.device-changed`, `audio.vad-state`
- `transcription.progress`, `transcription.completed`, `transcription.failed`
- `plugin.loaded`, `plugin.error`
- `license.validated`, `license.expired`

**Tense-Konvention:**
- Past-Tense für State-Change-Events (`started`, `completed`, `loaded`)
- Present für kontinuierliche Streams (`level`, `progress`)

**Cross-Boundary-Konsistenz:**
- Tauri-Events (Rust→WebView): exakt der Wire-String in `emit()` / `listen()`
- JNI-Callbacks (Rust→Kotlin): Bridge-Layer konvertiert `recording.started` → Kotlin-Methode `onRecordingStarted` (dots werden CamelCase-Segment-Boundaries, `on`-Prefix addiert)
- Interne Rust-Broadcasts: typed Enum (`RecordingEvent::Started`, `AudioEvent::Level(f32)`); Wire-Name via `#[serde(rename)]` nur auf Boundary-Serialisierung

**Ohne tauri-specta droht Drift** zwischen Enum-Variante und Wire-Name — `#[serde(rename = "recording.started")]` kann falsch sein, Listener merkt es nur zur Laufzeit. Siehe Codegen-Sub-Decision in Format Patterns.

#### Settings-Keys (KV-Tabelle aus Step 4 §2)

**Format:** dot-notation mit **enforced Namespace-Prefix.**

**Core-Namespaces (reserviert, nur Core schreibt hier):**
- `app.*` — `app.ui_language`, `app.first_run_completed`, `app.version`
- `audio.*` — `audio.input_device`, `audio.vad_threshold`
- `hotkey.*` — `hotkey.slot1.combo`, `hotkey.slot2.mode`
- `ui.*` — `ui.theme`, `ui.bubble_position`, `ui.font_scale`
- `license.*` — `license.cache_payload`, `license.last_validation_at`
- `history.*` — `history.retention_days`

**Plugin-Namespace:**
- `plugins.<plugin_id>.*` — z. B. `plugins.groq.model`, `plugins.deepseek.temperature`
- Plugin-Code darf **nur** in `plugins.<eigene_id>.*` schreiben (Konvention, optional Core-Layer-Validation ab Phase 1)

**Vorteile dot-notation:**
- Prefix-Queries: `SELECT * FROM settings WHERE key LIKE 'plugins.groq.%'` → Plugin-Uninstall = Prefix-Delete
- Settings-Panel-UI leitet Gruppen aus Namespace ab (kein separates Grouping-Field nötig)
- Composite-Key-Revisit-Point (Step 4 §2): wenn `hotkey.slot1.combo` + `hotkey.slot1.mode` + `hotkey.slot1.active` als Triple auftreten, ist das der Trigger für dedizierte `hotkey_slots`-Tabelle

**Typed Accessor-Layer:**
- Raw-Key-Strings leben nur im Accessor, Rest der Codebase nutzt Methoden: `settings.ui_language()`, `settings.set_ui_language(...)`
- Agents fügen neue Settings immer mit typed Accessor hinzu — niemals `settings.get_string("app.ui_language")` direkt in Feature-Code

#### Platform-Identifiers

**Symmetrische Shell-Identifier:**
- Windows (Tauri): `de.klarvo.windows` (geändert von Step-3-Default `de.klarvo.app`)
- Android: `de.klarvo.android`
- Zukünftige Shells: `de.klarvo.<platform>` (macos, ios, linux)

**Begründung:** Reverse-DNS auf Basis `klarvo.de`, Shell-Suffix differenziert ohne Repo-Interna (`shells/android/`) in End-User-Package-IDs zu bluten.

**Sub-Task für Concern #13 (v1→v2 Migration):** v1-Windows-Tauri-Identifier vor Phase-0-Start gegenchecken (vermutlich `de.klarvo.app` aus v1-Code). Einmal-Import muss alte AppData-Pfade finden, auch wenn der neue Identifier abweicht. Migration-Code erwartet `%APPDATA%\de.klarvo.app\` (oder den tatsächlichen v1-Pfad) als Quelle und schreibt in `%APPDATA%\de.klarvo.windows\` als Ziel — dokumentiert in Migration-Trait-Impl.

### Structure Patterns

#### Frontend-Component-Organisation (Windows-Shell WebView)

**Feature-based mit shared UI-Primitives.**

```
shells/windows/src/
├── features/                      # Feature-Slices, self-contained
│   ├── settings/
│   │   ├── SettingsPanel.tsx
│   │   ├── sections/              # AudioSection, HotkeySection, ...
│   │   ├── hooks/                 # useSaveSettings, ...
│   │   ├── store.ts               # Zustand-Slice
│   │   └── schema.ts              # Zod-Schemas für Forms
│   ├── history/
│   ├── onboarding/
│   ├── transcription/             # NUR WebView-Seite (Live-Event-Listener,
│   │                              # History-Integration, Status-Anzeige).
│   │                              # Pill-Bar selbst ist native Win-Overlay
│   │                              # (Step 4 §1), NICHT React!
│   └── hotkeys/
├── components/ui/                 # Shared Primitives (Button, Modal, Input, Toast)
├── lib/                           # Pure Utilities
├── stores/                        # Cross-cutting Zustand (app state, theme)
├── bindings/                      # tauri-specta-generated Rust-Types (committed)
└── App.tsx
```

**Regeln:**
- Neues Feature = neuer Ordner unter `features/` mit dieser Sub-Struktur
- Component in `components/ui/` nur bei ≥2 Feature-Nutzungen (Rule-of-Two; vorher feature-lokal halten)
- **No-Horizontal-Feature-Imports:** Features dürfen `components/ui/`, `lib/`, `stores/`, `bindings/` importieren — **nie** andere Features. Cross-Feature-Communication ausschließlich über `stores/` oder Tauri-Events.
- Optional für späteres Tooling: `eslint-plugin-boundaries` für mechanische Enforcement der Import-Regeln (nicht MVP-blocking, ab Phase 1 empfohlen)

#### Test-Fixtures (cross-crate)

**Workspace-Root `test-assets/` für Binaries + `klarvo-test-fixtures` Dev-Crate für typed Accessors.**

```
klarvo/                              # Repo-Root
├── test-assets/                     # Binaries + Golden-Outputs
│   ├── audio/                       # WAV-Samples (Speech DE/EN, Silence, Noise)
│   ├── cleanup-golden/              # Snapshot-Baselines
│   └── dictionary/                  # Sample-Dictionaries
└── klarvo-test-fixtures/            # Dev-Crate (publish = false)
    └── src/
        ├── audio.rs                 # fn speech_de_short() -> Vec<f32>
        └── mocks/                   # MockSttProvider, MockLlmProvider
```

**Regeln:**
- Rust-Tests importieren via `klarvo_test_fixtures::audio::speech_de_short()` — **nie** direkte Pfad-Strings
- Kotlin/Frontend-Tests greifen auf `test-assets/` via relativen Root-Pfad (`rootDir`, `path.resolve(__dirname, '../../../test-assets')`)
- Binaries > 1MB via Git LFS; Sample-Audio idealerweise < 500KB (10–20s Mono 16kHz)
- `insta`-Snapshot-Files bleiben im Test-Crate — **nicht** in `test-assets/`

#### Plugin-Crate-Skeleton

**Fixed Struktur für jeden neuen Plugin-Crate:**

```
klarvo-plugin-<name>/
├── Cargo.toml                       # publish = false
├── README.md                        # Purpose, Config-Keys, External-Deps, License
├── src/
│   ├── lib.rs                       # NUR Re-Exports + pub fn register(registry: &mut PluginRegistry)
│   ├── provider.rs                  # Trait-Impl(s)
│   ├── client.rs                    # External-Service-Client (weglassen wenn nicht nötig)
│   ├── error.rs                     # Lokale Errors + From<LocalError> for PluginError
│   ├── types.rs                     # Request/Response-DTOs mit #[serde(rename_all = "camelCase")]
│   └── config.rs                    # Plugin-spezifisches Config-Schema
├── migrations/
│   └── 001_initial.sql              # Optional — nur bei eigenen Tabellen
└── tests/
    └── integration.rs               # Hinter Cargo-Feature `integration-tests` (nicht Default)
```

**Regeln:**
- Neues Plugin = komplette Skeleton-Kopie. Ausnahmen dokumentiert in `README.md`.
- `lib.rs` enthält **nur** Re-Exports + `register()`. Keine Business-Logik.
- `register()` ist der einzige Entry-Point, Cargo-Feature-gated im Core-Bootstrap.
- `error.rs`: lokale Errors via `thiserror`, `From`-Impls mapping auf `PluginError`-Kategorien aus Step 4 §1.
- Integration-Tests hinter `integration-tests`-Feature — CI fährt diese gezielt, Unit-Runs pingen keine externen Services.

**Phase-0-Tooling (nice-to-have):** `cargo xtask new-plugin <name>` als Subcommand, der Skeleton mechanisch erzeugt (inkl. Cargo.toml-Template, lib.rs mit Platzhalter-`register()`, Migration-Folder, README). Verhindert manuelles Copy-Paste-Vergessen. Kostet wenige Stunden in Phase 0, amortisiert sich ab Plugin #3.

#### Generated Bindings (tauri-specta)

**Committed, mit CI-Drift-Check als authoritative Gate:**

```
shells/windows/src/bindings/
├── commands.ts                      # Auto-generiert, committed
├── events.ts                        # Auto-generiert, committed
└── types.ts                         # Auto-generiert, committed
```

**Zwei-Schichten-Enforcement:**
- **CI (authoritativ):** `cargo run -p xtask -- generate-bindings && git diff --exit-code shells/windows/src/bindings/` — failed bei Drift. Kann nicht umgangen werden.
- **Pre-Commit-Hook (quality-of-life):** Gleicher Command, local. Schneller Feedback-Loop, verhindert Surprise-CI-Fails. **Kann via `--no-verify` umgangen werden**, deshalb nicht authoritative.

**Vorteile:**
- Fresh Checkout funktioniert ohne Rust-Toolchain (`npm install && npm run dev`)
- Boundary-Changes sichtbar in PR-Diffs
- Drift-Detection verhindert silent-out-of-sync

### Format Patterns (IPC-Boundary)

#### JSON-Field-Case

**Rust renamed via `#[serde(rename_all = "camelCase")]`** auf allen Typen die über IPC/JNI gehen. Rust-Code bleibt idiomatisch snake_case, Frontend und Kotlin bekommen natives camelCase.

**Sub-Decision: `tauri-specta` / `specta 2.x` als Codegen-Layer. In Phase 0 einziehen.**

Begründung:
- Auto-generierte TS-Types aus Rust-Structs eliminieren Case-Mapping-Diskussionen komplett — Rust-Struct ändert sich → TS-Types regenerieren → Frontend-Compiler findet Breaking-Changes
- Ohne Codegen drohen `#[serde(rename)]`-Drift-Bugs (Rust sagt `recording.started`, Frontend listened auf `recording-started`, Listener taub zur Laufzeit)
- Plugin-Architektur verstärkt den Effekt: neues Plugin addet Commands → TS-Types aktualisieren automatisch
- Setup-Preis ~1–2 Tage, amortisiert ab erstem Plugin

**Ergänzt Specta-Events** (Rust-Structs mit `#[derive(specta::Event)]`): Event-Namen aus Type-Namen ableiten, Emitter + Listener teilen denselben Type. Kein Drift möglich.

#### Date/Time über IPC

**i64 Unix-Epoch-Millis UTC auf Wire, `chrono::DateTime<Utc>` intern in Rust, `new Date(millis)` in TS.**

Trade-offs:
- **Gewählt (i64 Millis):** ~13 Bytes in JSON vs. ~26 bei ISO-8601. Bei 10k History-Rows: 130KB vs. 260KB; bei Live-Events (Audio-Level 20Hz+) relevanter Skalierungsfaktor. TZ-frei (immer UTC). Unlesbar raw in DevTools — gelöst durch Convention: `tracing`-Layer formatiert human-readable, Frontend-Zustand-Devtools via Custom-Formatter.
- **Verworfen (ISO-8601 auf Wire):** Lesbarkeit für +110KB Overhead + Parse-Cost beidseitig + Stringly-typed TZ-Suffix-Risiko.
- **Ausnahme:** `tracing`-Logs nutzen ISO-8601 (menschliche Konsumenten, nicht Maschinen).

#### Error-Shape (Tauri-Commands)

**Native Tauri `Result<T, AppError>` — kein eigener Envelope.**

```rust
#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: AppErrorKind,            // flat String auf Wire, siehe unten
    pub message: String,               // technisch, für Logs
    pub user_message: Option<String>,  // i18n-Key (z. B. "history.delete_failed"), UI-tauglich
    pub retryable: bool,
}

#[derive(Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AppErrorKind {
    Network,
    Auth,
    Validation,
    RateLimit,
    Internal,
    Unavailable,
}
```

**Wire-Format für `kind`: flat String (`"network"`, `"rate_limit"`), NICHT tagged Enum.**

- Serialisierter JSON: `{"kind": "network", "message": "...", "userMessage": null, "retryable": true}`
- Frontend-Switch bleibt trivial: `if (err.kind === 'network') ...`
- Zusatzdaten via eigene Felder (`retryable`, später optional `retryAfterMs`) — spart Frontend-Ceremony (keine `switch(err.kind.type)` + Discriminated-Union-Gymnastik)

**Kontrakt: `From<PluginError> for AppError` ist verbindlich.**

Plugin-Boundary (`PluginError`, Step 4 §1) → Tauri-Boundary (`AppError`) Mapping:

| `PluginError`-Variante | `AppError.kind` | `retryable` |
|------------------------|-----------------|-------------|
| `Network(_)` | `Network` | `true` |
| `Auth(_)` | `Auth` | `false` |
| `RateLimit { retry_after_ms }` | `RateLimit` | `true` (+ `retry_after_ms` in Detail-Feld später) |
| `Fatal(_)` | `Internal` | `false` |
| `Unavailable(_)` | `Unavailable` | `true` |

`impl From<PluginError> for AppError` lebt in `klarvo-core` (nicht in Shells) — zentraler Mapping-Punkt, Shells konsumieren `AppError` ohne `PluginError` zu kennen.

### Communication Patterns

#### IPC-Commands (Rust↔React via Tauri)

- Namenskonvention: `snake_case` Rust-seitig (`start_recording`); tauri-specta generiert TS-Function-Namen automatisch (kann auf `camelCase` gemappt werden via `specta`-Config — einmalige Workspace-Entscheidung in Phase 0)
- Grob-körnig (Step 4 §3): Nicht jede interne Core-Funktion wird exposed, nur der Stable-API-Surface
- Payload-Size: Bei > 100KB (z. B. volle History-Queries) explizit paginieren, nicht in einem Call

#### Live-Events (Rust→UI)

- **Tauri:** Channels (v2-Feature) für High-Frequency (Audio-Level 20Hz+). Events für State-Changes (Recording-Started).
- **JNI:** `callbackFlow` auf Kotlin-Seite, Raw-jni-Crate im Bridge-Layer (Dual-Surface aus Step 4 §3)
- **Interne Rust:** `tokio::sync::broadcast` — typed Enum im Core, Serialisierung erst am Rand

#### State-Updates (Frontend Zustand)

- Immutable-Updates via Zustand's `set(state => ({...}))` — direkte Mutation verboten außerhalb `set`-Callbacks
- Action-Naming: Verb-first (`saveSettings`, `loadHistory`, `clearHistory`), nicht `doSave` / `handleSave`
- Selector-Pattern: `useSettingsStore(state => state.settings.uiLanguage)` für Re-Render-Minimierung; keine ganzen Stores destructuren

### Process Patterns

#### Migrations (drei First-Class-Fälle)

**Schema-Migrations (Struktur):**
- Sequential-per-plugin, SQL-Files in `<crate>/migrations/`, Naming `<3-stellig>_<snake_description>.sql`
- Tracking-Tabelle `schema_migrations (plugin_id, version, name, applied_at, checksum)`, Composite-PK
- `checksum`-Check bei Startup; modifizierte Already-Applied-Migration → hard fail
- **Kein Down-Migrations im MVP** — Rollback via Pre-Update-Backup (Step 4 §7)
- Execution-Order: Core-Migrations zuerst, dann Plugin-Migrations in Load-Order, jedes Plugin in eigenem `SAVEPOINT`

**Settings-Key-Migrations (Umbenennen von Settings-Keys):**
- Laufen über **dasselbe Plugin-Migration-System** — kein paralleler Mechanismus
- Format: SQL in Migration-File
  ```sql
  -- 002_rename_ui_theme_to_color_scheme.sql
  UPDATE settings SET key = 'ui.color_scheme' WHERE key = 'ui.theme';
  ```
- Gilt für Core-Keys (Core-Migration) und Plugin-Keys (Plugin-Migration, nur eigener Namespace)

**Settings-Value-Migrations (Semantik-Änderung von Values):**
- Wenn Enum-Values einer Setting sich ändern (z. B. `ui.theme` hat Werte `light|dark|auto`, wird zu `light|dark|system|high-contrast`) — das ist ein **First-Class-Migration-Case**, kein Ad-hoc-Script
- Format: SQL-UPDATE mit `CASE WHEN` für Value-Mapping
  ```sql
  -- 003_normalize_theme_values.sql
  UPDATE settings
  SET value = CASE value
    WHEN 'auto' THEN 'system'
    ELSE value
  END
  WHERE key = 'ui.theme';
  ```
- Agents müssen erkennen: Enum-Variant-Änderung in Rust-Code ohne korrespondierende Value-Migration → Runtime-Deserialisierungs-Fail

**Plugin-Migration-Dependency-Kontrakt:**
- Plugin-Migrations dürfen **nur Core-Schema** referenzieren (Core-Tabellen, Core-Settings)
- Plugin-Migrations dürfen **NICHT** andere Plugins' Schemas referenzieren — Load-Order ist keine harte Garantie für Inter-Plugin-Deps
- Verstoß → Architektur-Review-Reject; Solution = Core-level Abstraction ziehen, dann beide Plugins referenzieren Core

#### Loading-States (Frontend)

**4-state Union-Type pro Operation, nicht Bool.**

```ts
type AsyncState<E = AppError> =
  | { status: 'idle' }
  | { status: 'loading' }
  | { status: 'success' }
  | { status: 'error'; error: E };
```

- Default-Error-Typ: `AppError` (siehe Format Patterns → Error-Shape; `E = AppError` ist der Standard-Generic-Default, andere Error-Types nur mit Rationale)
- Per-Feature-Zustand-Slice hält Operation-States; kein global `app.loading` Record
- Naming: `<operation>State` (`saveState`, `loadState`, `validateKeyState`)
- React-Query **nicht** in MVP — Tauri-Commands sind Rust-IPC nicht HTTP, Caching-Model passt nicht

**P1-Erweiterung (Flag, nicht MVP):**
Für Listen-Operationen mit Refresh-Semantik (History-List) kann ein 5. State nachgerüstet werden:
```ts
| { status: 'refreshing'; data: T }  // Stale-While-Revalidate
```
Erlaubt „zeige alte History während neue lädt". **Bevor jemand React-Query einführt**, diesen 5. State evaluieren — billiger, weniger Dependencies.

### Enforcement Guidelines

**All AI Agents MUST:**
- Reference-Block-Defaults kennen und anwenden ohne Nachfrage
- Neue Plugin-Crates aus Skeleton generieren (`cargo xtask new-plugin <name>` wenn verfügbar, sonst manuelle Kopie)
- `#[serde(rename_all = "camelCase")]` auf allen IPC-Boundary-Structs setzen
- Settings-Keys ausschließlich in typed Accessor-Layer addieren, nie raw-Keys in Feature-Code streuen
- `From<PluginError> for AppError`-Mapping respektieren bei neuen Error-Varianten
- Settings-Key-Renames + Value-Änderungen immer als Migration-File, nie als Inline-Code-Fix
- i18n-Keys in dot-notation + Feature-Namespace; `user_message` im `AppError` ist immer Key, nie übersetzter String

**Enforcement-Mechanismen:**

| Pattern | Enforcement | Authoritativ? |
|---------|-------------|---------------|
| Cargo-Feature-Prefix-Konvention | `cargo xtask lint-features` in CI | Ja |
| Frontend No-Horizontal-Feature-Imports | `eslint-plugin-boundaries` (ab Phase 1) | Ja (ab Enable) |
| Generated-Bindings-Drift | `cargo xtask generate-bindings && git diff --exit-code` in CI | Ja (CI-Gate) |
| Pre-Commit-Bindings-Hook | Git pre-commit via `husky` oder `lefthook` | Nein (umgehbar via `--no-verify`) |
| Migration-Checksum-Integrity | Runtime-Startup-Check im Core | Ja (Hard Fail) |
| Rust-Naming | `clippy::style` + Rust-API-Guidelines-Lints | Teilweise |
| SQL-Naming | Code-Review + Architektur-Doc | Nein (human-enforced) |
| Event-Naming | tauri-specta + `specta::Event` macht Drift unmöglich | Ja (Type-System-Gate) |
| i18n-Key-Konvention | Code-Review (bis P1-Library-Choice) | Nein (human-enforced) |

**Pattern-Update-Prozess:**
- Änderung an diesem Block = ADR im `docs/adr/`-Ordner mit Rationale
- Bestehender Code wird nicht retroaktiv migriert — Änderungen gelten für Neu-Code ab ADR-Datum
- Breaking Pattern-Changes (z. B. JSON-Case-Umstellung) erfordern Migration-Plan im ADR

### Pattern Examples

**Good — typed Settings-Accessor:**
```rust
impl Settings {
    pub fn ui_language(&self) -> AppLanguage {
        self.get_typed("app.ui_language").unwrap_or(AppLanguage::De)
    }
    pub fn set_ui_language(&mut self, lang: AppLanguage) -> Result<()> {
        self.set_typed("app.ui_language", lang)
    }
}
// Feature-Code nutzt nur Methoden:
let lang = settings.ui_language();
```

**Good — Plugin-Registration mit Feature-Gate:**
```rust
// klarvo-core/src/plugin_registry.rs
pub fn bootstrap(registry: &mut PluginRegistry) {
    #[cfg(feature = "plugin-groq")]
    klarvo_plugin_groq::register(registry);

    #[cfg(feature = "plugin-deepseek")]
    klarvo_plugin_deepseek::register(registry);
}
```

**Good — Error-Mapping an der Boundary:**
```rust
#[tauri::command]
async fn transcribe(text: String, state: State<'_, AppState>) -> Result<String, AppError> {
    state.core.transcribe(&text).await
        .map_err(AppError::from)  // PluginError -> AppError via zentralem From-Impl
}
```

**Good — User-facing Error mit i18n-Key:**
```rust
Err(AppError {
    kind: AppErrorKind::Unavailable,
    message: format!("Groq API returned 503: {}", body),
    user_message: Some("transcription.service_unavailable".into()),  // i18n-Key
    retryable: true,
})
```

**Anti-Pattern — raw Settings-Key in Feature-Code:**
```rust
// SCHLECHT — Key-String wandert in Feature-Code, kein Typ-Check, kein Refactoring-Support
let lang: String = settings.get("app.ui_language").unwrap_or("de".into());

// RICHTIG — typed Accessor, siehe Good-Example oben
let lang = settings.ui_language();
```

**Anti-Pattern — horizontaler Feature-Import im Frontend:**
```ts
// SCHLECHT — shells/windows/src/features/settings/SettingsPanel.tsx
import { useHistoryStore } from '../history/store';

// RICHTIG — Cross-Feature via stores/ oder Tauri-Event
import { useAppStore } from '@/stores/app';
```

**Anti-Pattern — Settings-Key-Rename via Inline-Code:**
```rust
// SCHLECHT — Migration passiert zur Laufzeit in Getter-Code, nie vollständig
fn get_theme(&self) -> String {
    self.raw_get("ui.color_scheme")
        .or_else(|| self.raw_get("ui.theme"))  // Legacy-Fallback wächst ewig
        .unwrap_or("system".into())
}

// RICHTIG — Migration-File einmal, Code nur noch neuer Key
// migrations/004_rename_ui_theme.sql: UPDATE settings SET key = 'ui.color_scheme' WHERE key = 'ui.theme';
fn color_scheme(&self) -> ColorScheme { self.get_typed("ui.color_scheme").unwrap_or_default() }
```

**Anti-Pattern — Plugin referenziert fremdes Plugin-Schema:**
```sql
-- klarvo-plugin-analytics/migrations/001_initial.sql — SCHLECHT
CREATE TABLE analytics_events (
    id INTEGER PRIMARY KEY,
    groq_request_id TEXT REFERENCES groq_metadata(id)  -- VERBOTEN: Fremd-Plugin-FK
);

-- RICHTIG: Abstraktion in Core ziehen, beide Plugins referenzieren Core-Tabelle
```

**Anti-Pattern — übersetzter String im `user_message`:**
```rust
// SCHLECHT — Sprache hardcoded, kein i18n-Resolve beim Display
Err(AppError {
    user_message: Some("Löschen fehlgeschlagen".into()),  // Deutsch-lock-in
    ..
})

// RICHTIG — i18n-Key, Frontend resolved
Err(AppError {
    user_message: Some("history.delete_failed".into()),
    ..
})
```

