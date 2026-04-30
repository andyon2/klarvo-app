---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
lastStep: 8
status: 'complete'
completedAt: '2026-04-18'
inputDocuments:
  - _bmad-output/planning-artifacts/product-brief-klarvo.md
  - _bmad-output/planning-artifacts/product-brief-klarvo-distillate.md
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

#### 4a. Release-Hardening (Validation-Patch G2)

**Problem:** „Test-Lizenzen in Prod" war eine v1-Original-Sünde. Ohne expliziten CI-Enforcement bleibt die Regel Wunschdenken — ein Agent aktiviert `dev-*`-Feature zur Debug-Zeit, vergisst das Abschalten, Release-Build geht durch.

**Entscheidung:** `cargo xtask verify-release` als Pflicht-Gate VOR jedem Release-Build. `.github/workflows/release.yml` ruft es als ersten Step. Fail = Release-Build wird gar nicht gestartet.

**Prüfpunkte (verpflichtend):**
- `test-license`-Feature ist NICHT aktiv
- `dev-plain-keystore` + alle `dev-*`-Features sind NICHT aktiv
- `obfstr`-Obfuscation-Key ist NICHT der Default-Placeholder (Compile-Time-Constant-Check)
- `tracing`-Subscriber-Config emittiert kein `DEBUG`- oder `TRACE`-Level im Release-Build — PII-Protection: Debug-Export-Zip könnte sonst sensible Request-Payloads enthalten
- Keine `#[cfg(debug_assertions)]`-Code-Paths in `--release`-Build (redundant mit Rust-Default, expliziter Check dokumentiert Intent)

**Enforcement-Direction:** `cargo xtask verify-release` ist authoritative; `release.yml` ruft es verpflichtend. Lokaler pre-push-Hook wäre quality-of-life, CI bleibt der eigentliche Gate (vgl. Step 5 Bindings-Drift-Pattern).

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

#### 8a. CaptureHandle — Opaque-Box Pattern

`AudioSource::start(..)` liefert einen `CaptureHandle`-RAII-Guard (ref ADR-0006 SubDec-4). Die konkrete Platform-Resource (cpal-Stream, zukünftiger Android-AudioRecord) soll jedoch nicht in `klarvo-core` durchschlagen — Core bleibt Platform-agnostisch. Das wird über einen Opaque-`Box<dyn Any + Send>` gelöst:

```rust
// klarvo-core/src/audio/source.rs
pub struct CaptureHandle {
    _guard: Box<dyn std::any::Any + Send>,
}

impl CaptureHandle {
    pub fn new<G: Send + 'static>(guard: G) -> Self {
        Self { _guard: Box::new(guard) }
    }
}

// Safety: CaptureHandle has no &self methods; _guard is only accessed at
// Drop time by the owning thread. Implementing Sync is sound because there
// is no way to observe shared mutable state through &CaptureHandle.
unsafe impl Sync for CaptureHandle {}
```

**Rationale.** Cross-crate Platform-Abstraktion ohne Generics auf der `AudioSource`-Trait-Signatur. Erlaubt `klarvo-audio-cpal`, `klarvo-test-fixtures` (MockAudioSource) und zukünftigem `klarvo-audio-android` je eigene konkrete Guard-Typen zu liefern, ohne dass `klarvo-core` cpal/JNI/AudioRecord als Dependency bekommt. Der `Any + Send`-vtable dispatched `Drop` für beliebige konkrete Guards korrekt — Drop-semantische Cleanup bleibt per-Impl definierbar.

**Live Consumer-Shapes.**
- `klarvo-audio-cpal::CpalGuard { _stream: cpal::Stream, tx_slot: Arc<Mutex<Option<broadcast::Sender<AudioEvent>>>> }` — Stream-Handle + Channel-Close-Slot.
- `klarvo-test-fixtures::MockAudioSource` — Oneshot-Sender-/Task-Handle für deterministisches Test-Teardown.

**`Sync`-Marker.** `Box<dyn Any + Send>` ist `Send` aber nicht automatisch `Sync`. Da der Guard nur beim Drop (Owning-Thread) berührt wird und es keine `&self`-Method auf `CaptureHandle` gibt, ist `unsafe impl Sync for CaptureHandle {}` sound — der Safety-Comment in `source.rs` dokumentiert das.

**Live-Code-Ref:** `klarvo-core/src/audio/source.rs:52-65`.

#### 8b. AudioError — Enum Shape & Abgrenzung zu PluginError

`AudioSource::start(..)` + Stream-interne Fehler werden als `AudioError` (Core-internes Enum, `#[non_exhaustive]`) modelliert:

```rust
// klarvo-core/src/audio/source.rs
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AudioError {
    #[error("audio device unavailable")]
    DeviceUnavailable,
    #[error("unsupported audio format")]
    UnsupportedFormat,
    #[error("capture interrupted: {msg}")]
    CaptureInterrupted { msg: String },
    #[error("resample failed: {msg}")]
    ResampleFailed { msg: String },
    #[error("device configuration error: {msg}")]
    DeviceConfigError { msg: String },
}
```

**Feldname-Rationale (`msg` statt `source`).** `thiserror` 2.x führt Auto-Source-Detection für Felder namens `source` durch und erwartet dort einen `dyn std::error::Error`-konformen Typ. `String` impl nicht `Error`, und eine Variante `CaptureInterrupted { source: String }` würde Compile-Errors produzieren. `msg` bypasst die Detection.

**Abgrenzung zu `PluginError`.** `AudioError` ist **Core-interne Fehlerart** — emergiert in `klarvo-core` + Platform-Impl-Crates (`klarvo-audio-cpal`, zukünftig `klarvo-audio-android`), deckt Device-Enumeration, Sample-Format-Probleme, Resampling-Failures und mid-session Capture-Interruption ab. `PluginError` (ref §1 Plugin-Error-Kontrakt) ist **Plugin-Boundary-Contract** — abstract Network/Auth/RateLimit/UpstreamUnavailable/KeyMissing aus externen Services. Beide mappen nach `AppError` via `From`-Impls; `AudioError` hat derzeit keinen `From`-Impl, weil Audio-Errors in Phase-1 ausschließlich über die `AudioSource::start(..)`-Result-Chain bzw. die `ErrorEmitter`-Brücke (ref ADR-0009) an die Shell propagieren und dort zum `AppError` konstruiert werden.

**Live-Code-Ref:** `klarvo-core/src/audio/source.rs:9-24`.

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
- **`klarvo-core` hat KEINE user-facing Strings** (Validation-Patch G3) — weder hardcoded in `.rs`-Konstanten, noch in Panic-Messages die ein User sehen könnte. Core emittiert ausschließlich i18n-Keys (via `AppError.user_message`, Event-Payloads) oder englische Developer-Messages (Panic-Kontext, Log-Felder — nicht für User-Anzeige gedacht). Übersetzung ist Shell-Aufgabe. Regel verhindert dass Phase-0-Agent eine deutsche Error-Message in Core hardcoded und das i18n-System später um Sonderbehandlung erweitert werden muss.
- **Translation-Assets-Location (Placeholder):** `shells/<platform>/locales/<lang>.<ext>`. Jede Shell hostet ihre Übersetzungen lokal. Extension (`.json` / `.ftl` / `.yml`) + Library-Wahl bleiben P1-ADR. Phase-0-Placeholder: leeres `shells/windows/src/locales/de.json` anlegen, damit spätere Library-Migration trivial ist. Android-äquivalent via `res/values/strings.xml` oder Library-spezifisch.
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
    UpstreamUnavailable,
    PermissionDenied,
    PipelineValidation,
    KeyMissing,
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
| `UpstreamUnavailable(_)` | `UpstreamUnavailable` | `true` |
| `KeyMissing { plugin_id }` | `KeyMissing` | `false` (+ `user_message: "error.keystore.key_missing"` gesetzt im From-Impl; plugin_id landet in `message`) |

`impl From<PluginError> for AppError` lebt in `klarvo-core` (nicht in Shells) — zentraler Mapping-Punkt, Shells konsumieren `AppError` ohne `PluginError` zu kennen.

**Asymmetrische Variants:** `PermissionDenied` + `PipelineValidation` emergieren außerhalb der Plugin-Boundary (Shell bzw. Core-Executor-Boot-Time) und haben keine `PluginError`-Counterpart. Ref ADR-0010.

**Async-Error-Bridge:** Async-Errors (emergent außerhalb von Command-Invocation-Contexten — z. B. cpal-Audio-Callback-Errors, Pipeline-Mid-Session-Errors, Pipeline-Boot-Validation-Errors) werden via dedicated `tauri_specta::Event` `app.error` mit `AppError`-Payload an das Frontend propagiert. Emit-Site ist der Shell-Orchestrator oder Core-`ErrorEmitter`-Trait-Impl (narrow-scoped für OS-Thread-Callback-Contexts wie `CpalAudioSource`). Ref ADR-0009.

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
| Event-Wire-Names | `cargo xtask lint-events` prüft auf jedem `specta::Event` das `#[specta(rename)]`-Attribut + dot-notation-Pattern (Validation-Patch G1) | Ja (CI-Gate) |
| Release-Hardening | `cargo xtask verify-release` — Test-Licenses, dev-Features, obfstr-Default-Key, DEBUG/TRACE-Subscriber off (Validation-Patch G2, siehe Step 4 §4a) | Ja (CI-Gate vor Release-Build) |
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
    kind: AppErrorKind::UpstreamUnavailable,
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

## Project Structure & Boundaries

### Step-4-Revision: VAD-Split

**Ergänzung zu Step 4 §1 (Plugin-System & Trait-Design):**

Die in Step 4 §1 formulierte Regel „VAD bleibt Core-intern" wird präzisiert zum Split:

- **Basis-VAD (RMS-based, Signal-Processing, keine ML-Deps): Core-intern** in `klarvo-core::audio::vad::rms`. Always-available Safety-Net — garantiert dass der Recording-Flow auch ohne jedes aktive Plugin funktioniert.
- **ML-basierte VAD-Impls (Silero ONNX, zukünftige Candle-Modelle): Plugins** via dediziertem `VadProvider`-Trait.

**Trait-Count aktualisiert:** 8 first-class Phase-0-Traits + 1 Stub — `SttProvider`, `LlmProvider`, `CleanupStyle`, `TextFilter`, `OutputTarget`, `AudioFilter`, `VadProvider` (neu), `PluginMigration`, plus `VoiceCommandHandler` als Stub.

**Rationale `VadProvider` dediziert statt `AudioFilter`-Extension:** Semantik-Mismatch — `AudioFilter` transformiert Samples (`samples_in → samples_out`), VAD emittiert Gate-Events (`is_speech: bool`, `speech_start_ms: u64`). Shoehorning pollutet `AudioFilter` für alle zukünftigen Filter-Impls. Trait-Signatur-Details werden im Phase-0-JNI-Spike-Zeitfenster finalisiert; leichte Präferenz liegt bei dediziertem Trait.

### Complete Project Directory Structure

```
klarvo/
├── Cargo.toml                        # [workspace] root, resolver v3, members list
├── Cargo.lock                        # Committed (SemVer safety: whisper-rs-sys, llama-cpp-2)
├── rust-toolchain.toml               # Pin Rust 2024 Edition channel + components
├── .cargo/config.toml                # Target-specific rustflags, xtask alias
├── .gitignore
├── .gitattributes                    # Git LFS rules für test-assets/audio/*.wav > 1MB
├── README.md
├── LICENSE                           # PolyForm Noncommercial 1.0.0
├── pipeline-manifest.toml            # Embedded-Default-Source: klarvo-core bindet via
│                                     # include_str!() zur Compile-Zeit ein. NICHT runtime-
│                                     # geladen. User-Override läuft als separater Loader-Pfad
│                                     # über User-Data-Dir (siehe Step 4 §1 + File Org unten).
├── justfile                          # Optional dev shortcuts (wraps xtask)
│
├── .github/workflows/
│   ├── ci-core.yml                   # Core + plugin unit tests (Linux)
│   ├── ci-windows.yml                # Tauri build + E2E (Windows runner)
│   ├── ci-android.yml                # Gradle build + unit tests (Linux runner + NDK)
│   ├── ci-bindings-drift.yml         # tauri-specta drift gate (authoritativ)
│   ├── ci-feature-lint.yml           # cargo xtask lint-features
│   ├── ci-event-lint.yml             # cargo xtask lint-events (Validation-Patch G1)
│   └── release.yml                   # Tauri-Updater + Play-Store-Upload; ruft
│                                     # cargo xtask verify-release VOR Build (Validation-Patch G2)
│
├── klarvo-core/                      # Shared Rust core (headless, testbar)
│   ├── Cargo.toml
│   ├── migrations/
│   │   ├── 001_schema_migrations.sql
│   │   ├── 002_settings_table.sql
│   │   ├── 003_histories_table.sql
│   │   └── 004_license_cache.sql
│   └── src/
│       ├── lib.rs                    # Public API surface (Core-API aus Step 4 §3)
│       ├── registry.rs               # PluginRegistry + bootstrap()
│       ├── manifest.rs               # Pipeline-Manifest: embed_default() via include_str!()
│       │                             # + load_user_override(path) für optionales User-TOML
│       ├── error.rs                  # PluginError, AppError, From-Impls (zentrales Mapping)
│       ├── migrations.rs             # Migration-Orchestrator + schema_migrations-Table
│       ├── traits/
│       │   ├── mod.rs
│       │   ├── stt.rs                # SttProvider
│       │   ├── llm.rs                # LlmProvider
│       │   ├── cleanup.rs            # CleanupStyle
│       │   ├── text_filter.rs       # TextFilter
│       │   ├── output.rs             # OutputTarget
│       │   ├── audio_filter.rs       # AudioFilter (sample-level transforms)
│       │   ├── vad.rs                # VadProvider (dediziert, Gate-Events)
│       │   ├── migration.rs          # PluginMigration
│       │   └── voice_command.rs      # VoiceCommandHandler (Stub, keine Impl Phase 0)
│       ├── pipeline/
│       │   ├── mod.rs
│       │   ├── state_machine.rs      # Idle→Recording→Processing→Output
│       │   └── orchestrator.rs       # Stage-Orchestration per Manifest
│       ├── audio/
│       │   ├── mod.rs
│       │   ├── source.rs             # AudioSource-Trait (Shell-Impls)
│       │   ├── events.rs             # AudioEvent enum (broadcast-Channel-Types)
│       │   ├── buffer.rs             # f32-Sample-Buffer-Primitives
│       │   ├── wav.rs                # hound-based WAV-Encoding
│       │   └── vad/
│       │       ├── mod.rs
│       │       └── rms.rs            # RMS-VAD (Safety-Net, ohne ML-Deps)
│       ├── recording/
│       │   ├── mod.rs
│       │   ├── modes.rs              # Hold, Toggle, AutoStop (Win) + 5 Android-Modes
│       │   └── state.rs              # Per-Session-Recording-State
│       ├── transcription/
│       │   ├── mod.rs
│       │   ├── priority.rs           # STT-Priority-List + Provider-Selection
│       │   └── fallback.rs           # Fallback-Chain-Execution
│       ├── cleanup/
│       │   └── mod.rs                # CleanupStyle-Orchestrator (nicht die Impls)
│       ├── history/
│       │   ├── mod.rs
│       │   ├── schema.rs
│       │   ├── queries.rs
│       │   └── retention.rs
│       ├── dictionary/
│       │   ├── mod.rs
│       │   └── schema.rs
│       ├── settings/
│       │   ├── mod.rs
│       │   ├── accessor.rs           # Typed Accessor-Layer (ui_language(), etc.)
│       │   ├── system.rs             # TOML-System-Settings (~5 Felder)
│       │   └── hybrid.rs             # System-vs-User-Resolution
│       ├── keystore/
│       │   ├── mod.rs
│       │   ├── trait_def.rs          # KeyStore-Trait
│       │   ├── plain_sqlite.rs       # #[cfg(feature = "dev-plain-keystore")]
│       │   └── os/
│       │       ├── mod.rs
│       │       ├── windows.rs        # #[cfg(target_os = "windows")], windows-rs
│       │       └── android.rs        # #[cfg(target_os = "android")], jni-crate
│       │                             # → ruft Android-Platform-APIs (siehe JNI-Dep-Einschub
│       │                             # in Component-Boundaries unten)
│       ├── license/
│       │   ├── mod.rs
│       │   ├── validator.rs          # HMAC-Validation
│       │   ├── cache.rs              # 30-Tage-Cache + 48h-Grace
│       │   └── obfuscation.rs        # obfstr-based Key-Obfuscation
│       ├── hotkey/
│       │   └── mod.rs                # Slot-Abstraktion + Pause/Resume + ShortcutRecorder +
│       │                             # Active-Mode-Badge. Phase-0-Agent-Call: bei >300 LOC
│       │                             # in mod.rs + slots.rs + registration.rs splitten.
│       ├── sync/                     # Turso-Sync P1-Stub (deferred-Crate-Kandidat)
│       │   ├── mod.rs
│       │   └── noop.rs               # MVP-NoOp-Impl
│       ├── ipc/                      # Shared IPC-DTOs (camelCase-Serde-Tags)
│       │   ├── mod.rs
│       │   ├── commands.rs
│       │   └── events.rs
│       ├── telemetry/
│       │   ├── mod.rs
│       │   ├── logging.rs            # tracing-subscriber + rolling file appender
│       │   └── export.rs             # User-triggered Debug-Zip
│       └── v1_import/
│           ├── mod.rs
│           ├── config.rs             # Plain-JSON → typed Settings
│           ├── keys.rs               # Plain-JSON Keys → OS-Keystore direkt
│           ├── history.rs            # v1-SQLite → v2-Schema
│           └── dictionary.rs
│
├── klarvo-plugins/                   # Alle Plugin-Crates folgen dem Skeleton aus Step 5
│   ├── klarvo-plugin-groq/           # STT-Provider (Groq Whisper)
│   ├── klarvo-plugin-deepseek/       # LLM-Provider (Cleanup-Backend)
│   ├── klarvo-plugin-verbatim/       # CleanupStyle (Default)
│   ├── klarvo-plugin-chat/           # CleanupStyle
│   ├── klarvo-plugin-polished/       # CleanupStyle (neu für v2)
│   ├── klarvo-plugin-clipboard/      # OutputTarget: Clipboard
│   ├── klarvo-plugin-keystroke/      # OutputTarget: Direct Keystroke Injection
│   └── klarvo-plugin-vad-silero/     # VadProvider: Silero ONNX
│
├── klarvo-bridge-jni/                # JNI-Bridge: Core-API → Kotlin-Shell
│   │                                 # (Linux-CI-buildable, NDK nur beim Final-Link)
│   ├── Cargo.toml
│   ├── build.rs                      # uniffi-bindgen
│   ├── uniffi.toml                   # uniffi-Config (Control-Plane-Scope)
│   └── src/
│       ├── lib.rs                    # uniffi-Scaffolding
│       ├── commands.rs               # Control-Plane (uniffi)
│       └── streams.rs                # Data-Plane (raw-jni + Callbacks)
│
├── klarvo-test-fixtures/             # Dev-Crate (publish = false)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── audio.rs                  # fn speech_de_short() -> Vec<f32>
│       └── mocks/
│           ├── mod.rs
│           ├── stt.rs
│           ├── llm.rs
│           ├── output.rs
│           └── vad.rs
│
├── shells/
│   ├── windows/                      # Tauri v2 + React
│   │   ├── src-tauri/
│   │   │   ├── Cargo.toml
│   │   │   ├── tauri.conf.json       # identifier: "de.klarvo.windows", CSP strikt
│   │   │   ├── build.rs
│   │   │   ├── capabilities/default.json
│   │   │   ├── icons/
│   │   │   └── src/
│   │   │       ├── main.rs           # Tauri-Entry, klarvo-core::bootstrap()
│   │   │       ├── commands.rs       # #[tauri::command]-Wrapper (KEIN Bridge-Crate —
│   │   │       │                     # asymmetric vs JNI by design; siehe Boundaries)
│   │   │       ├── events.rs         # Tauri-Channel-Emitters
│   │   │       ├── hotkey.rs         # Windows global-shortcut integration
│   │   │       ├── overlay/
│   │   │       │   ├── mod.rs
│   │   │       │   ├── pill_bar.rs   # Native PillBar (nicht React!)
│   │   │       │   └── tray.rs
│   │   │       ├── audio_source.rs   # cpal-based AudioSource-Impl
│   │   │       └── bindings.rs       # tauri-specta Type-Collection-Entry
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   ├── vite.config.ts
│   │   ├── tailwind.config.ts
│   │   ├── index.html
│   │   └── src/                      # React-Frontend (feature-based, Step 5)
│   │       ├── main.tsx
│   │       ├── App.tsx
│   │       ├── features/
│   │       │   ├── settings/
│   │       │   ├── history/
│   │       │   ├── onboarding/
│   │       │   ├── transcription/    # WebView-Seite only; PillBar ist native
│   │       │   └── hotkeys/
│   │       ├── components/ui/
│   │       ├── lib/
│   │       │   ├── i18n.ts           # i18n-Key-Resolver (Library P1-ADR)
│   │       │   ├── format.ts
│   │       │   └── errors.ts
│   │       ├── stores/
│   │       │   ├── app.ts
│   │       │   └── theme.ts
│   │       └── bindings/             # tauri-specta-generated, committed
│   │           ├── commands.ts
│   │           ├── events.ts
│   │           └── types.ts
│   │
│   └── android/                      # Kotlin + Jetpack Compose
│       ├── app/
│       │   ├── build.gradle.kts      # namespace = "de.klarvo.android"
│       │   └── src/main/
│       │       ├── AndroidManifest.xml
│       │       ├── kotlin/de/klarvo/android/
│       │       │   ├── KlarvoApplication.kt
│       │       │   ├── bridge/
│       │       │   │   ├── Core.kt               # uniffi-generated Wrapper
│       │       │   │   ├── Events.kt             # callbackFlow-Adapter
│       │       │   │   └── StreamListener.kt     # raw-jni-Callbacks
│       │       │   ├── features/
│       │       │   │   ├── bubble/               # Floating-Overlay (17 Features)
│       │       │   │   ├── settings/
│       │       │   │   ├── history/
│       │       │   │   └── onboarding/
│       │       │   ├── service/
│       │       │   │   ├── AccessibilityService.kt
│       │       │   │   └── InputMethodService.kt
│       │       │   ├── audio/
│       │       │   │   └── AndroidAudioSource.kt # AudioRecord-based
│       │       │   └── ui/theme/
│       │       └── res/
│       ├── build.gradle.kts
│       ├── settings.gradle.kts
│       ├── gradle.properties
│       └── keystore/                 # gitignored (Signing-Keys)
│
├── xtask/                            # Build-Orchestration
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── build_all.rs
│       │   ├── test_core.rs
│       │   ├── ci.rs
│       │   ├── generate_bindings.rs  # tauri-specta Regen + Drift-Check
│       │   ├── new_plugin.rs         # Plugin-Skeleton-Generator (Phase-0 nice-to-have)
│       │   ├── lint_features.rs      # Cargo-Feature-Naming-Convention-Enforcer
│       │   ├── lint_events.rs        # Specta-Event-Rename-Convention-Enforcer (G1)
│       │   └── verify_release.rs     # Release-Hardening-Gate (G2, ruft gegen alle Checks aus §4a)
│       └── templates/
│           └── plugin_skeleton/
│
├── test-assets/                      # Shared Binary-Test-Assets (Git LFS > 1MB)
│   ├── audio/
│   │   ├── speech-de-short.wav
│   │   ├── speech-en-short.wav
│   │   ├── silence-2s.wav
│   │   ├── noise-only.wav
│   │   └── README.md
│   ├── cleanup-golden/
│   │   ├── verbatim-case-01.json
│   │   └── polished-case-01.json
│   └── dictionary/
│       └── sample-dictionary.json
│
├── docs/                             # Project-Documentation (v1-brownfield + v2-ADRs)
│   ├── index.md                      # v1-Entry-Point, Links auf v1-snapshot
│   ├── v1-architecture-snapshot.md   # v1-Brownfield-Referenz (umbenannt 2026-04-17)
│   ├── component-inventory.md        # v1-Scan-Output
│   ├── development-guide.md          # v1; wird in Phase-0 v2-rewritten
│   ├── rebuild-discussion.md         # Shared-State zwischen parallelen Sessions
│   ├── project-overview.md           # v1-Scan-Output
│   ├── source-tree-analysis.md       # v1-Scan-Output
│   ├── project-scan-report.json      # Frozen v1-Scan-Manifest
│   ├── adr/                          # Architecture Decision Records (neu, Phase 0+)
│   │   └── README.md
│   └── phase-0-checklist.md          # Generated nach Step 8
│
└── output/
    ├── planning-artifacts/            # BMad-Workflow-Outputs (tracked)
    │   ├── product-brief-klarvo.md
    │   ├── product-brief-klarvo-distillate.md
    │   └── architecture.md           # Single-Source v2-Architecture (dieses Doc)
    └── implementation-artifacts/     # Phase-1+, Sub-Path-Policy beim ersten Commit
```

### Architectural Boundaries

**IPC Boundaries:**
- **Tauri (Rust↔React):** Commands + Channels (v2). Alle Boundary-Types leben in `klarvo-core/src/ipc/`, TS-Bindings via tauri-specta auto-generiert → `shells/windows/src/bindings/`. Drift-Check in CI als authoritative Gate.
- **JNI (Rust↔Kotlin):** Dual-Surface (Step 4 §3). Control-Plane via uniffi in `klarvo-bridge-jni/src/commands.rs`. Data-Plane via raw-jni in `klarvo-bridge-jni/src/streams.rs`. Kotlin-Side in `shells/android/.../bridge/` mit `callbackFlow`-Adaptern.
- **Asymmetrie Tauri ↔ JNI ist Absicht:** kein `klarvo-bridge-tauri`-Crate. Tauri-Commands sind triviale Wrapper in `shells/windows/src-tauri/src/commands.rs` direkt. JNI braucht eine Abstraktionsschicht, Tauri nicht. Agents verletzen diese Asymmetrie nicht aus Symmetrie-Gefühl.
- **Plugin-Registration:** compile-time via `klarvo-core::registry::bootstrap()` + feature-gated `#[cfg]`-Module. Kein dynamisches Loading.

**Component Boundaries (Dependency-Direction):**
- `klarvo-core` ← `klarvo-plugins/*` (Plugins depend on Core)
- `klarvo-core` ← `klarvo-bridge-jni` ← `shells/android`
- `klarvo-core` ← `shells/windows/src-tauri`
- **`klarvo-core` importiert NIE aus `shells/*`, `klarvo-bridge-jni`, oder `klarvo-plugins/*`** (nur Trait-Objekte über `PluginRegistry`)
- `klarvo-test-fixtures` ← Core + Plugins (dev-dependency, nur in Tests)
- Frontend No-Horizontal-Imports zwischen Features (enforced via `eslint-plugin-boundaries` ab Phase 1)
- Android-Compose-Features mirror die gleiche Regel

**Einschub — JNI-Deps in `klarvo-core` (zwei Rollen, nicht verwechseln):**

`klarvo-core` hat die `jni`-Crate als **conditional-dependency** (`#[cfg(target_os = "android")]`-gated). Das ist KEIN Widerspruch zur „Core importiert nie aus `klarvo-bridge-jni`"-Regel — die beiden Nutzungen sind orthogonal:

| Rolle | Location | Richtung | Zweck |
|-------|----------|----------|-------|
| **Bridge-Crate** (`klarvo-bridge-jni`) | separates Crate | **Core-API → Kotlin** (inbound für Kotlin-Shell) | Kotlin-Shell spricht Core via uniffi/raw-jni an |
| **Platform-API-Access** (`klarvo-core/src/keystore/os/android.rs`) | Core intern, conditional-dep | **Core → Android-OS** (outbound zu OS) | Core ruft Android-Keystore-System-API direkt |

Beide nutzen `jni`, spielen aber unterschiedliche Rollen. „Core hat kein JNI" ist falsch und führt zu kaputter Keystore-Impl. „Core importiert nicht aus `klarvo-bridge-jni`" bleibt richtig.

Vergleichbar auf Windows-Seite: `klarvo-core/src/keystore/os/windows.rs` nutzt `windows-rs` (`#[cfg(target_os = "windows")]`-gated) für Windows-Credential-Manager-Zugriff. Selbes Muster, andere Platform.

**Data Boundaries:**
- SQLite-DB wird nur über `klarvo-core` geöffnet; Shells haben KEINEN direkten DB-Zugriff
- Plugin-Migrations plugin-owned, aber zentrale `schema_migrations`-Tracking-Table in Core (Composite-PK `(plugin_id, version)`)
- OS-Keystore-Zugriff ausschließlich via `KeyStore`-Trait — raw-OS-Credential-APIs nie von Plugins oder Shells gerufen
- Turso-Sync (P1): whitelist-gesteuert (histories, ausgewählte settings) via `klarvo-core::sync`

### Requirements to Structure Mapping

**FR-Cluster aus Step 2 → File-Level:**

| # | Cluster | Primary Location | Shell-Impls / Sekundär |
|---|---------|------------------|------------------------|
| 1 | Core Pipeline | `klarvo-core/src/{pipeline,traits,transcription,cleanup}/` | — |
| 2 | Recording Modes | `klarvo-core/src/recording/` | `shells/windows/src-tauri/src/hotkey.rs`, `shells/android/.../service/AccessibilityService.kt` |
| 3 | Hotkey System | `klarvo-core/src/hotkey/mod.rs` | `shells/windows/src-tauri/src/hotkey.rs` (global-shortcut), `shells/android/.../service/AccessibilityService.kt` |
| 4 | Text Processing | `klarvo-plugins/klarvo-plugin-{verbatim,chat,polished}/` | `klarvo-core/src/cleanup/mod.rs` (Orchestrator) |
| 5 | Audio | `klarvo-core/src/audio/` + `klarvo-plugins/klarvo-plugin-vad-silero/` | `shells/windows/src-tauri/src/audio_source.rs` (cpal), `shells/android/.../audio/AndroidAudioSource.kt` (AudioRecord) |
| 6 | Providers | `klarvo-plugins/klarvo-plugin-{groq,deepseek,...}/` | `klarvo-core/src/transcription/{priority,fallback}.rs` |
| 7 | UI | `shells/windows/src/features/*`, `shells/windows/src-tauri/src/overlay/` (native PillBar), `shells/android/.../features/*`, `shells/android/.../features/bubble/`, `shells/android/.../service/` | — |
| 8 | History & Stats | `klarvo-core/src/history/` | `shells/*/features/history/` |
| 9 | Dictionary | `klarvo-core/src/dictionary/` | `shells/*/features/settings/` (Edit-UI) |
| 10 | Lizenz-System | `klarvo-core/src/license/` | `shells/*/features/{settings,onboarding}/` |

**Cross-Cutting Concerns (Step 2, 15 Concerns) → File-Level:**

| # | Concern | Primary Location |
|---|---------|------------------|
| 1 | Plugin-Registry & Lifecycle | `klarvo-core/src/registry.rs` |
| 2 | Pipeline-Manifest | `klarvo-core/src/manifest.rs`, `pipeline-manifest.toml` (embedded-default via `include_str!`) |
| 3 | Plugin-Migrations | `klarvo-core/src/migrations.rs`, `klarvo-core/src/traits/migration.rs`, `<crate>/migrations/` |
| 4 | Konfigurations-Hybrid | `klarvo-core/src/settings/{system,hybrid,accessor}.rs` |
| 5 | API-Key-Storage-Evolution | `klarvo-core/src/keystore/` (Trait + Plain + OS-Impls) |
| 6 | Error-Handling & Fallback | `klarvo-core/src/error.rs`, `klarvo-core/src/transcription/fallback.rs` |
| 7 | Cross-Platform Audio | `klarvo-core/src/audio/` + Shell-AudioSource-Impls |
| 8 | Cross-Platform Hotkey | `klarvo-core/src/hotkey/` + Shell-Impls |
| 9 | Observability | `klarvo-core/src/telemetry/` (Logging + Export) |
| 10 | Licensing-Integration | `klarvo-core/src/license/` + Cargo-Feature-Gating |
| 11 | Security Baseline | `shells/windows/src-tauri/tauri.conf.json` (CSP), `klarvo-core/src/keystore/`, CI-Checks |
| 12 | Testability | `klarvo-test-fixtures/`, `test-assets/`, `xtask::test_core` |
| 13 | v1 → v2 Migration | `klarvo-core/src/v1_import/` + Sub-Task v1-Identifier-Check (Step 5 Platform-IDs) |
| 14 | Update/Release | Tauri-Updater (Win), Play-Store (Android), `.github/workflows/release.yml` |
| 15 | i18n (drei Achsen) | `klarvo-core/src/settings/` (Sprach-Settings), `shells/*/lib/i18n.*` (UI-Resolver), Plugin-Configs (Output-Language) |

### Integration Points

**Internal Communication (Rust-intern):**
- `tokio::sync::broadcast` für Events (multi-consumer)
- `tokio::sync::mpsc` für Command-Queues
- Direkte Funktionsaufrufe über Trait-Objekte im `PluginRegistry`
- Serialisierung passiert ausschließlich an IPC-Boundary (Tauri-Commands, JNI-Bridge) — nicht intern

**External Integrations:**
- **Cloud-AI-Provider:** Groq, DeepSeek, OpenAI, Anthropic, OpenRouter via HTTPS (BYOK, Keys aus OS-Keystore); jeder Provider in eigenem Plugin-Crate
- **Windows:** `global-shortcut`-Crate (Hotkeys), Windows Credential Manager via `windows-rs` (Keystore), Tauri Updater API
- **Android:** Android-Keystore via JNI (conditional-dep in `klarvo-core`), AccessibilityService-API, InputMethodService, Play-Core-Update-API (falls Play-Release)
- **Turso (P1):** `libsql`-Client für Cloud-Sync
- **Local-AI (P1/P2):** `whisper-rs` (Offline-STT), `mistral.rs`/`llama-cpp-2` TBD (Offline-LLM)

**Data Flow:**
`AudioSource (Shell) → broadcast::Channel → VadProvider::gate → AudioFilter-Chain → SttProvider → CleanupStyle → OutputTarget → History-Persistence → IPC-Event-Emission`

Pipeline-Orchestrator (`klarvo-core/src/pipeline/orchestrator.rs`) liest Manifest (embedded oder User-Override) und dispatched Stages über Plugin-Registry.

### File Organization Patterns

**Configuration:**
- **TOML im Workspace-Root** (nicht runtime-modifizierbar, compile-time-kritisch): `Cargo.toml`, `rust-toolchain.toml`, `pipeline-manifest.toml` (letzteres wird von `klarvo-core::manifest` via `include_str!()` zur Compile-Zeit eingebettet; siehe Manifest-Kontrakt unten)
- **Tauri-Config:** `shells/windows/src-tauri/tauri.conf.json`
- **Per-Plugin-Configs:** `src/config.rs` im Plugin-Crate (Schema), User-Values in Settings-KV-Tabelle
- **User-Settings:** SQLite-KV + System-TOML (~5 Felder), Hybrid-Resolution via `klarvo-core::settings::hybrid`

**Manifest-Kontrakt (explizit dokumentiert, nicht Agent-Ermessen):**
- `pipeline-manifest.toml` am Workspace-Root ist die **Embedded-Default-Quelle**
- Core bindet sie via `include_str!("../../pipeline-manifest.toml")` zur **Compile-Zeit** ein — der Inhalt ist Teil des Binaries, nicht runtime-geladen
- User-Override (optional, Pro-Feature-Gate möglich) lebt im User-Data-Dir (`%APPDATA%\de.klarvo.windows\pipeline.toml` / Android-Equivalent) und wird zusätzlich zur Laufzeit geladen wenn vorhanden
- **Kein „read-from-working-dir"-Fallback**: Agents dürfen den Loader NICHT auf „wenn keine Datei, suche in cwd" umbauen — das bricht den Deterministik-Kontrakt und erzeugt Prod-Bugs die in Dev nie auftauchen
- Validation läuft in beiden Fällen (Embedded + User-Override) durch denselben Parser in `klarvo-core::manifest::validate()`

**Source Code:**
- Crate-per-Concern in `klarvo-core` (Sub-Module unter `src/`)
- Ein Crate pro Plugin
- Shell-Code physisch separiert unter `shells/<platform>/`
- Utility-Crates auf Root-Level (`klarvo-bridge-jni`, `klarvo-test-fixtures`, `xtask`)

**Test Code:**
- Unit: `#[cfg(test)]` co-located mit Source
- Integration: `<crate>/tests/*.rs`
- Shared Rust-Fixtures: `klarvo-test-fixtures` (Code) + `test-assets/` (Binaries)
- Snapshot-Tests (`insta`): `.snap`-Files vom Crate gemanaged, nicht in `test-assets/`

**Asset Organization:**
- Test-Assets: `test-assets/` (Root, cross-crate)
- Runtime-Shell-Assets: `shells/<platform>/assets/` oder plattform-übliche Locations (`src-tauri/icons/`, Android `res/`)
- Audio > 1MB via Git LFS (`.gitattributes`)

### Development Workflow Integration

**Development Server:**
- Windows-Shell: `cd shells/windows && npm run tauri dev`
- Android-Shell: Android Studio `app`-Module-Run oder `./gradlew :app:installDebug`
- Core-Unit-Tests headless: `cargo test -p klarvo-core` oder `cargo xtask test-core`
- Plugin-Integration-Tests (hinter Feature-Gate): `cargo test -p klarvo-plugin-groq --features integration-tests`

**Build Process:**
- `cargo xtask build-all` orchestriert Core + Plugins + Windows-Shell + Android-Shell
- `cargo xtask ci` fährt lokal die CI-Matrix (Core-Tests + Lint + Feature-Lint + Bindings-Drift)
- `cargo xtask generate-bindings` regeneriert tauri-specta-TS-Types
- `cargo xtask new-plugin <name>` generiert Plugin-Skeleton (Phase-0-Utility)
- `cargo xtask lint-events` prüft Specta-Event-Wire-Names gegen dot-notation-Konvention (G1-Gate)
- `cargo xtask verify-release` Release-Hardening-Gate, wird von `release.yml` vor Build gerufen (G2)

**Deployment:**
- Windows-Installer: `cargo tauri build` (via xtask) → MSI/EXE, signed mit Cert (MVP self-signed)
- Android-Bundle: `./gradlew :app:bundleRelease` → AAB für Play-Store (Primär) oder APK (Fallback)
- Release-Pipeline: `.github/workflows/release.yml` orchestriert beide Targets + Tauri-Updater-Upload + Play-Console-Upload (wenn API konfiguriert)

### Repo-Preparation (bereits in dieser Session ausgeführt)

- `docs/architecture.md` (v1-Brownfield-Scan) → `docs/v1-architecture-snapshot.md` umbenannt (2026-04-17)
- 4 Cross-Refs aktualisiert (`docs/index.md`, `docs/project-overview.md`, 2× Frontmatter in `output/planning-artifacts/`)
- `docs/` + `output/planning-artifacts/` erstmalig git-tracked (Commit `cbf9138`, 11 Files, 2641 Insertions)
- **Offen als Phase-0-Prep:** v1-Windows-Tauri-Identifier in v1-Repo gegenchecken (Migration-Pfad-Findung für Einmal-Import; Sub-Task Concern #13). Memory: `project_phase0_action_items.md`.

## Architecture Validation Results

### Coherence Validation ✅

**Decision Compatibility:**
- Rust 2024 Edition × Tauri v2.10.x × tokio × rusqlite × whisper-rs × cpal × windows-rs × `jni` × `uniffi` × tauri-specta/specta 2.x — alle auf dem April-2026-Stand kompatibel. Versionen pinned in Cargo.lock (commited).
- `jni`-Crate in zwei orthogonalen Rollen (`klarvo-bridge-jni` exposes Core-API zu Kotlin; `klarvo-core/keystore/os/android.rs` ruft Android-OS-APIs) — in Step 6 Component-Boundaries klargestellt.
- `chrono::DateTime<Utc>` × Serde × i64-Millis-Wire — Standard-Pattern via `chrono/serde`-Feature.

**Pattern Consistency:**
- **Initial detectierter Widerspruch:** Event-Naming-Konvention (`recording.started`, dot-notation) vs. Specta-Events-Default-Wire-Name (Type-Name). Specta hat explizite `#[specta(rename)]`-Attribute, aber kein Compile-Error bei Vergessen → Drift-Vektor.
- **Auflösung:** `cargo xtask lint-events` als CI-Gate (Validation-Patch G1). Prüft dass jede `specta::Event`-Struct das `#[specta(rename = "<dot-notation>")]`-Attribut trägt und dem `<domain>.<event>`-Pattern folgt. Inkonsistenz strukturell eliminiert.

**Structure Alignment:**
- Project-Tree (Step 6) mappt alle FR-Cluster und Cross-Cutting-Concerns auf konkrete File-Locations.
- Dependency-Direction-Regeln (Core importiert nie aus Shells/Bridge/Plugins) sind mit `jni`-Dual-Rolle-Klarstellung konsistent.
- Feature-basiertes Frontend + No-Horizontal-Feature-Imports spiegelt sich in Plugin-Crate-Skeleton (eigener Namespace pro Plugin).

### Requirements Coverage Validation ✅

**FR-Cluster-Coverage (Step 2 → Step 6 Mapping):**
- Alle **10 FR-Cluster** haben Primary-Location + Shell-Impls dokumentiert.
- Keine Cluster ohne architektonische Unterstützung.

**NFR-Coverage (11 NFRs):**

| NFR | Status | Anmerkung |
|-----|--------|-----------|
| Plattform-Parität (0 LOC Duplikat) | ✅ | Shared-Core + Plugin-Architektur erzwingt strukturell |
| Testbarkeit (headless Core) | ✅ | `klarvo-test-fixtures` + `xtask test-core` |
| Latenz (kein IPC im Diktat-Flow) | ✅ | Compile-time Plugins, gleicher Prozess |
| Modularität (Provider ohne Shell-Change) | ✅ | Plugin-Skeleton + Cargo-Feature-Gate |
| Sicherheit (keine v1-Sünden portiert) | ✅ | Release-Hardening-Gate (§4a, G2-Patch) erzwingt |
| Privacy/Sovereignty (BYOK, keine Telemetry) | ✅ | OS-Keystore, Local-Logs, User-triggered-Export |
| Multi-Language (3 unabhängige Achsen) | ✅ | Settings-Separation + Core-ohne-User-Strings-Regel (G3-Patch) |
| Lizenzierung (Free/Paid via Cargo-Features) | ✅ | license-Modul + Feature-Gating |
| Onboarding (<2min first-success) | ✅* | Architektur enabled; UX-Benchmark ist Phase-0-QA-Messung (nicht Architektur-Gap) |
| Migration (v1 → v2 Einmal-Import) | ✅ | `klarvo-core/src/v1_import/` + Identifier-Sub-Task (#13) |
| Regressions-Disziplin | ✅ | Headless-Core + CI-Matrix + Pattern-Enforcement + Release-Hardening-Gate |

**Cross-Cutting-Concerns:** 15/15 mit Primary-Location gemappt (Step 6 Tabelle).

### Implementation Readiness Validation ✅

**Decision Completeness:**
- Kritische Entscheidungen dokumentiert mit Versionen (Rust 2024, Tauri v2, whisper-rs 0.16, Specta 2.x)
- Offene Entscheidungen explizit als „Open Decisions" oder „Deferred auf P1/P2" markiert (Offline-LLM, exakte Trait-Signatur für VadProvider im JNI-Spike)
- Open-Verification-Todos im Doc (llama-cpp-2-Version, whisper-rs-SemVer-Pinning)

**Structure Completeness:**
- File-Level-Tree in Step 6 (keine „`...`"-Platzhalter an wichtigen Stellen)
- Alle Crate-Grenzen + Shell-Grenzen benannt
- Integration-Points (IPC, Component, Data) explizit

**Pattern Completeness:**
- Reference-Block für Defaults
- Strittige Cluster (A/B/C) in Step 5 mit Rationale
- Enforcement-Matrix inkl. G1/G2-Gates
- Good-Examples + Anti-Patterns vorhanden

### Gap Analysis Results

**Important Gaps (alle 3 in dieser Validierung gepatcht):**

| # | Gap | Patch | Location |
|---|-----|-------|----------|
| G1 | Specta-Event-Rename-Drift-Gate fehlt | `cargo xtask lint-events` Subcommand + CI-Job | Step 5 Enforcement-Matrix, Step 6 xtask/workflows |
| G2 | Release-Hardening-Enforcement fehlt | `cargo xtask verify-release` Subcommand + release.yml-Integration | Step 4 §4a (neu), Step 5 Enforcement-Matrix, Step 6 xtask/workflows |
| G3 | i18n-Core-no-user-strings + Asset-Path | Reference-Block-Ergänzung + Placeholder-Pfad | Step 5 Reference-Block i18n |

**Minor Gaps (Phase-0-Agent-Calls, nicht blocking):**

| # | Gap | Handhabung |
|---|-----|------------|
| G4 | ADR-Template-Format in `docs/adr/` | Standard-ADR-Template (Status / Context / Decision / Consequences / Date / Number) als Phase-0-Agent-Call; `docs/adr/_template.md` wird beim ersten ADR erstellt |
| G5 | Workspace-Feature-Propagation-Beispiel | Wird bei erster Multi-Plugin-Aktivierung in Phase 0 konkretisiert |
| G6 | Cargo.lock-Update-Policy | Solo-Dev-Kontext → kein akuter Formalismus nötig |

**Keine Critical Gaps.** Phase-0 ist startfähig.

### Validation Issues Addressed

Alle Important-Gaps (G1/G2/G3) wurden **inline in die betroffenen Steps gepatcht** — Korrekturen stehen in den authoritativen Sektionen (Step 4 §4a, Step 5 Reference-Block, Step 5 Enforcement-Matrix, Step 6 xtask/workflows), nicht als externer Fix-Block. Zukünftige Agents lesen damit die korrekte Version ohne zwischen Original und Correction springen zu müssen.

Minor-Gaps sind hier dokumentiert + werden in `project_phase0_action_items.md` als Agent-Calls gepflegt.

### Architecture Completeness Checklist

**✅ Requirements Analysis**
- [x] Project-Kontext analysiert (10 FR-Cluster, 11 NFRs, 15 Cross-Cutting-Concerns)
- [x] Scale/Complexity: High (Multi-Plattform-nativ + Plugin-System + BYOK + Lizenz-System)
- [x] Technical Constraints: Rust-Workspace, compile-time Plugins, Hybrid-UI, PolyForm-NC, Dual-Surface-JNI
- [x] Cross-Cutting Concerns: alle 15 mit File-Level-Mapping

**✅ Architectural Decisions**
- [x] 8 Decision-Kategorien in Step 4 als Tabellen
- [x] Tech-Stack vollständig spezifiziert (Rust 2024, Tauri v2, React 19, TS 5.8, Tailwind 4, Kotlin 2.x, Compose)
- [x] Integration-Patterns: Dual-Surface-JNI (uniffi + raw-jni) + Tauri direkt
- [x] Performance: compile-time Plugins, broadcast-Channels, i64-Millis
- [x] Security-Baseline mit Release-Hardening-Gate (§4a)
- [x] Vier Andy-Revisionen als Beschlüsse (JNI Dual-Surface, OS-Keystore ab MVP, keine Remote-Telemetry, Play-Store Phase-3-Blocker)
- [x] VAD-Split (Step-4-Revision in Step 6): Basis-RMS in Core, ML-VAD als Plugin

**✅ Implementation Patterns**
- [x] Reference-Block für Defaults (Rust/SQL/TS/Kotlin/TOML/Cargo/Tests/Errors/Logging/i18n)
- [x] Naming-Patterns: Events (dot-notation), Settings-Keys (namespaced), Platform-IDs (symmetric)
- [x] Struktur-Patterns: Frontend feature-based, Plugin-Skeleton, tauri-specta-Bindings committed
- [x] Format-Patterns: JSON camelCase via Serde, Date/Time i64 Millis UTC, AppError flat-kind + `From<PluginError>`
- [x] Communication-Patterns: IPC-Commands + Events + State-Updates
- [x] Process-Patterns: drei Migration-Arten, 4-state Loading-States
- [x] Enforcement-Matrix inkl. lint-events + verify-release
- [x] Good-Examples + Anti-Patterns

**✅ Project Structure**
- [x] Complete File-Level-Tree (Step 6)
- [x] Dependency-Direction dokumentiert (Core ← Plugins ← Bridge ← Shells)
- [x] JNI-Dual-Rolle klargestellt (Bridge vs. Platform-API-Access)
- [x] FR-Cluster → File Mapping (10/10)
- [x] Concern → File Mapping (15/15)
- [x] Integration-Points: Tauri, JNI, DB-Ownership, OS-APIs
- [x] Manifest-Kontrakt: embedded-default via include_str! + optional User-Override

### Architecture Readiness Assessment

**Overall Status:** READY FOR IMPLEMENTATION

**Confidence Level:** **High** — alle 15 Cross-Cutting-Concerns mit File-Level-Mapping, alle NFRs abgedeckt, alle Important-Gaps gepatcht, keine Critical-Gaps.

**Key Strengths:**
- Clean-Slate-Architektur adressiert v1-Architektur-Schulden strukturell: Android-Tauri-Bypass → Shared-Core + Dual-Surface-JNI · Plain-Key-Storage → OS-Keystore ab MVP · CSP-off → strict CSP · Test-Licenses → Release-Hardening-Gate
- Plugin-Architektur + Cargo-Feature-Gating macht Free/Paid/Nischen-Builds zur Build-Zeit-Entscheidung
- Dual-Surface-JNI (uniffi Control-Plane + raw-jni Data-Plane) — pragmatisch, testbar auf Linux-CI, respektiert uniffi-Stream-Limitationen
- Comprehensive Patterns-Block (Step 5) mit Reference-Defaults verhindert Agent-Drift bei Solo-Dev-Multi-Session-Arbeit
- Three-Way-Migration-System (Schema + Settings-Key-Rename + Settings-Value-Semantic) fängt alle realistischen Schema-Evolution-Fälle ab
- CI-Enforcement-Matrix (Bindings-Drift, Feature-Lint, Event-Lint, Verify-Release) macht kritische Invarianten unumgehbar

**Areas for Future Enhancement:**
- WASM-Plugin-Loader für Third-Party-Plugins (v2.x)
- Offline-LLM-Entscheidung `mistral.rs` vs `llama-cpp-2` (P1/P2)
- Turso-Sync-Implementierung (P1-Feature, Stub vorhanden)
- i18n-Library-Wahl + exakte File-Extension bei zweiter UI-Sprache (P1-ADR)
- VadProvider-Trait-Signatur-Finalisierung im Phase-0-JNI-Spike-Zeitfenster
- `klarvo-macros` + `klarvo-sync` als separate Crates wenn Trigger-Bedingungen erfüllt (Auto-Settings-Accessor-Macros, heavy Turso-Deps)

### Implementation Handoff

**AI Agent Guidelines:**
- Architektur-Entscheidungen aus Steps 1–6 + Validation-Patches aus Step 7 sind **verbindlich** — keine Freestyle-Variation
- Reference-Block (Step 5) ist Pflichtlektüre vor erstem Code-Commit
- Plugin-Crates mechanisch via `cargo xtask new-plugin <name>` erzeugen (wenn verfügbar) — manuelle Kopie nur in Ausnahme
- Bei Unsicherheit: ADR in `docs/adr/` schreiben statt ad-hoc entscheiden
- Memory-Referenzen nutzen: `project_phase0_action_items.md` für Phase-0-Start-Checkliste
- Neue Event-Structs: `#[specta(rename = "<domain>.<event>")]` setzen, lokaler `cargo xtask lint-events`-Run vor Commit
- Release-Hardening: `cargo xtask verify-release` manuell vor jedem Release-PR fahren (CI erzwingt auch, aber lokale Feedback-Loop ist schneller)

**First Implementation Priority (Phase 0, synthetisiert aus Step 3 + Step 6 + Validation-Patches):**
1. Cargo-Workspace + `klarvo-core` + `xtask`-Crate-Skelett
2. Core-Traits (8 + VoiceCommand-Stub) + `PluginRegistry::bootstrap()`
3. `KeyStore`-Trait + beide Impls (Plain-SQLite hinter `dev-plain-keystore`-Feature, OS-Keystore für Release)
4. Migration-Tooling (Orchestrator + `schema_migrations`-Tabelle + Plugin-Migration-Trait)
5. `AudioSource`-Trait + Core-interne Pipeline-State-Machine + RMS-VAD
6. **JNI-Spike (1 Tag Gate):** Audio-Level-Meter end-to-end; entscheidet uniffi-Commit + VadProvider-Trait-Signatur
7. Pipeline-Manifest-TOML-Parser + Embedded-Default via `include_str!()` + User-Override-Loader
8. Erste Plugin-Crate-Skelette: `klarvo-plugin-groq`, `klarvo-plugin-verbatim`
9. xtask-Subcommands: `new-plugin`, `lint-features`, `lint-events`, `verify-release`, `generate-bindings`, `test-core`, `ci`
10. Headless-Test-Harness + `klarvo-test-fixtures` + erste Mock-Impls
11. v1-Windows-Tauri-Identifier-Check (für v1→v2-Migration-Pfad)
12. tauri-specta-Integration + erste Bindings-Generation (Drift-Gate aktiv)
13. CI-Pipelines (`ci-core.yml`, `ci-feature-lint.yml`, `ci-event-lint.yml`, `ci-bindings-drift.yml`)

**Phase-0-Gates (müssen erfüllt sein bevor Phase 1 beginnt):**
- JNI-Spike-Ergebnis committed (uniffi vs. raw-jni-only final)
- Release-Hardening-Gate grün (verify-release passt für ersten Release-Build)
- Bindings-Drift-Gate aktiv in CI
- v1→v2-Migration-Pfad dokumentiert + Integration-Test grün
- Mindestens 1 Plugin (z. B. `groq`) end-to-end lauffähig via headless Core-Test

