# Klarvo Rebuild — Discussion State

Erstellt: 2026-04-14
Letztes Update: 2026-04-17
Status: **Grill-Session abgeschlossen — alle strategischen Fragen geklaert**
Naechster Schritt: `/bmad-product-brief` in frischem Context-Fenster fuer formales Artefakt, dann `/bmad-create-architecture` fuer technische Spec

---

## Ausgangslage

Andy moechte Klarvo neu aufsetzen statt zu refactoren. Parallel zu dieser Session laeuft ein weiteres Brainstorming. Dieses Dokument haelt den aktuellen Diskussionsstand fest, damit die parallele Session aufsetzen kann.

**Kontext:**
- Klarvo v0.5.0 existiert als Tauri 2 Desktop-App mit Android-Support
- Projektdokumentation wurde am 2026-04-13 via Deep Scan generiert (`docs/index.md`)
- Early Access wurde zurueckgezogen, keine aktiven Tester, kein Zeitdruck
- Idealer Moment fuer Rebuild

---

## Codebase-Analyse (Fakten aus Deep Scan)

Die bestehende Klarvo-Codebase zeigt:

### Was bruechig ist
- **Android umgeht Tauri IPC komplett**: Nur ~15% des Rust-Backends wird von Android genutzt
- **~2.000 LOC dupliziert** zwischen Rust (src-tauri/) und Kotlin (android/) — STT, LLM-Cleanup, Sync, History alles zweimal implementiert
- **44% des Rust-Codes ist plattform-bedingt** (`#[cfg]`) — Code fragmentiert
- **62 Tauri Commands**, davon ~40 plattform-spezifisch (Hotkey, Window, Updater)
- **Tauri's Mehrwert ist 80% Windows-only** (Hotkeys, Updater, Tray)
- **Security-Report**: 3 kritisch, 6 hoch (CSP deaktiviert, Test-Lizenzen in Prod, hardcoded Secrets)
- **Kein Frontend-Testframework**
- **Organisch gewachsen**: neue Features verursachen Bugs, starke Kopplung

### Was solide ist
- Provider-Abstraktion (Traits) ist sauber
- SQLite-Schema ist simpel und funktional
- Turso-Sync funktioniert
- 71 Features implementiert — Produkttiefe ist hoch
- Kern-Pipeline (Audio → STT → LLM → Paste) funktioniert

**Fazit der Analyse:** Das ist kein reines Code-Qualitaets-Problem, sondern ein **Architektur-Mismatch**. Tauri v2 ist das falsche Framework fuer die Multi-Plattform-Strategie.

---

## Bisher entschiedene Punkte (Grill-Session)

### Frage 1: Was ist bruechig?
**Antwort (Andy):**
- (A) Framework-Mismatch: ungeklaert, BMAD soll helfen → **Codebase-Analyse bestaetigt: Tauri v2 ist falsch fuer Multi-Plattform-Strategie**
- (B) **Code-Qualitaet: ja**, organisch gewachsen, viel Kopplung, neue Features verursachen staendig neue Bugs und Baustellen
- (C) **Produkt-Konzept: jain**, nicht fundamental anders, aber **Extension Openness** ist kritisch

### Frage 2: Was heisst "Extension Openness"?
**Antwort (Andy):**
- **(B) Platform-Erweiterung: ja**, macOS und iOS sind bereits angeplant
- **(C) Produkt-Pivot: ja**, staendig neue Feature-Ideen, Integrationen mit anderen Services (Notion, Todoist etc.) — entweder Diktat dorthin oder deren APIs in Klarvo nutzen
- Voice Commands, mehr Einstellmoeglichkeiten
- **Wichtig**: Architektonisch so gebaut (modular, clever), dass neue Features bestehendes nicht kaputt oder inkompatibel machen koennen

### Frage 3: Wie viel von v1 uebernehmen?
**Antwort (Andy):**
- **(A) Clean Slate** — nur Kernprodukt (Pipeline), alles andere neu designed und priorisiert
- Kein Zeitdruck, keine EA-Phase aktiv, keine Tester
- Alle Tester wurden informiert dass erste echte Version "noch viele Wochen" dauert
- "Wir koennen uns Zeit nehmen und in Ruhe alles vernuenftig modular aufbauen"

### Frage 4: Framework-Entscheidung (Ersatz fuer Tauri v2)?
**Antwort (Andy):**
- **(A) Shared Rust Core + native Shells**

**Architektur-Prinzip:**
- Plattform-agnostischer Rust-Kern als Library: STT, LLM, Pipeline, History, Sync, Plugin-System
- Native Shells rufen den Kern via FFI/JNI:
  - Desktop (Windows, macOS, Linux): Tauri oder Electron
  - Mobile (Android): Kotlin
  - Mobile (iOS): Swift
- **Ein Kern, mehrere Shells** — keine Duplikation mehr
- Geschaeftslogik (welchen Provider aufrufen, wie Text cleanen, wo speichern) lebt im Rust Core
- Plattform-spezifische Services (Overlay, AccessibilityService, Hotkeys) leben in den Shells

### Frage 5: Frontend-Strategie?
**Antwort (Andy):**
- **(C) Hybrid** — React fuer die App-UI (Settings, History, Onboarding) via WebView, plattform-native Overlays
- Overlays sind nativ: FloatingBar (Desktop), Bubble (Android), Menu Bar (macOS)
- Weniger Aufwand als voll-nativ, bessere UX als WebView-ueberall auf Mobile

### Frage 6: Plattform-Priorisierung (2026-04-17)
**Andy korrigiert urspruengliche Reihenfolge:**
- **Windows + Android sind GLEICH wichtig** — beide MVP, parallel ab Tag 1
- iOS und macOS folgen spaeter (Reihenfolge: iOS vor macOS)

**Architektur-Implikation:** Shared Rust Core wird sofort auf 2 Plattformen validiert (gut). Aber: Aufwand initial deutlich hoeher. Braucht von Tag 1:
- Klar definierte Trait-Boundaries im Core (kein Tauri-spezifischer Code)
- JNI-Bridge fuer Android sofort
- Headless-Test-Infrastruktur fuer Core
- Klares Shell-vs-Core-Split (Hotkeys/Overlays = Shell, Pipeline/Settings-Persistenz = Core)

### Frage 7: MVP-Scope (2026-04-17)
**Andy's Definition: ~40-45 von 107 Features sind MVP** (also nicht "Mini-MVP", sondern effektiv "v1 von Klarvo neu").

**Im MVP enthalten — Section-by-Section:**

| Section | MVP |
|---------|-----|
| Core Pipeline | STT, LLM, Auto-paste, Clipboard fallback, Insert-Send, History save, Min duration, Hallucination, Prompt stripping, Output language |
| Recording Modes | Hold, Toggle, AutoStop (W) + alle Android-Modi (Tap-HOLD/TOGGLE, Long-Press PTT/AUTOSTOP) |
| Hotkey System | 2 Slots (skaliert spaeter auf 4-5), Pause/Resume, ShortcutRecorder, Active mode badge |
| Text Processing | Verbatim (NEUER Default), Chat, Polished (NEU GEBAUT — siehe Frage 8), Auto-Capitalize |
| Audio | Device selection, RMS silence, Live audio events, WAV-Encoding |
| Providers | Groq Whisper, DeepSeek LLM, STT Priority List + Fallback, Live API key validation, STT conditioning |
| UI/UX | Main settings (minimal), Komplette Floating Pill Bar (Shape, Drag, Position, Waveform), Return-Focus, Tray, Onboarding (Stub), Quick Tips, StylePicker, History panel |
| History & Stats | Dictation history, Delete, Clear all |
| Android-Specific | Komplett alle 17 Features (Bubble mit 5 Zustaenden, Gesten, AccessibilityService, Konfiguration) |
| Sync & Cloud | — (kommt P1) |
| Dictionary | Custom dictionary (capped) |
| Offline / Local Whisper | — (kommt P1/P2) |
| License System | HMAC validation, Permanent, Trial, 30-day cache, 48h grace, Status display |
| Advanced / Power User | — (kommt P1/P2) |

**P1 (kurz nach MVP):** Auto Turso sync, OpenAI Whisper/LLM, Groq LLM, Reformate (Email/Bullets/Summary), Whisper Mode (Gain), Stats panel, History search, Cost tracking, Unlimited dictionary, Whisper Model Manager (small/medium), Webhook integration, Auto-Loop, UI scale, Autostart, Hot-reload providers.

**P2 (Power-Features):** Anthropic, OpenRouter, Provider model overrides, Custom prompts, App Profiles, Command Mode/Hotkey, Voice Notes, Snippets, Filler word analysis, Local Whisper Large + GPU/CUDA, alle Threshold-Configs.

**DEFER / nicht in v2:** Live transcription preview (war disabled), Integrations panel (war Placeholder), Early adopter 60-day grace (v1-only), Voice Commands (besser als Plugin neu denken).

### Frage 8: Cleanup-Styles & Polished-Bug (2026-04-17)
**Andy's Beobachtung:** *"Ich nutze Polished uebrigens nie. Ich nutze immer Verbatim. Polished macht zugleich zuviel kaputt!"*

**Entscheidung:**
- **Verbatim wird neuer Default** (statt Polished)
- **Polished wird in v2 neu gebaut** — nicht aus v1 portiert. Aktuelle Implementierung "macht zu viel kaputt" (zu aggressives Umschreiben).
- Chat bleibt wie ist
- Designziel fuer neuen Polished: "Filler weg, Grammatik korrekt, aber Stimme bleibt" — nicht "professionell umformuliert".

**Implikation:** In v1-Aussenkommunikation wird Polished als "Hero-Style" verkauft, real wird Verbatim genutzt. Marketing/Onboarding muessen angepasst werden.

### Frage 9: Lizenz-System im MVP (2026-04-17)
**Andy:** *"ja, direkt im MVP mit Lizenzsystem"*

- HMAC-Validierung, Trial, 30-day Cache, 48h Grace direkt im MVP
- Kein Lemon Squeezy im MVP (P1)
- Early Adopter 60-day Grace bleibt drin (DEFER) — v1-spezifisch

### Frage 10: Phasierung des MVP (2026-04-17)
**Entscheidung: Option A** — alles bleibt MVP, intern phasiert in Sub-Phasen.

| Phase | Inhalt | Ziel |
|-------|--------|------|
| 0 | Rust Core + Pipeline (headless, Trait-Boundaries, JNI-Bridge, Test-Infra) | Architektur validiert, beide Plattformen-ready |
| 1 | Windows-Shell mit Pipeline durchgehend (1 Mode, 1 Hotkey, kein UI-Polish) | Erstes Diktat funktioniert auf Windows |
| 2 | Windows-Shell vollstaendig (alle Modi, beide Hotkeys, komplette Pill Bar) | Windows daily usable |
| 3 | Android-Shell vollstaendig (alle Bubble-Zustaende, Gesten, AccessibilityService) | Android daily usable |
| 4 | Lizenz-System + Settings-Polish + Cleanup-Style-Refactor (Polished neu) | MVP komplett |

**Realistische Timeline (Andy bestaetigt):** 2-4 Monate Vollzeit-Arbeit. Andy: *"Ja, passt."*

### Frage 11: Plugin-Architektur-Design (2026-04-17)
**Entscheidung: Option A — Trait-basiert (compile-time) mit zentraler Registry, Pipeline-Manifest, Cargo-Features.**

**Begruendung:**
- ~70% der konkreten Use Cases (Provider, Notion, Todoist, Voice Commands) sind First-Party — brauchen kein WASM
- WASM-Host-API-Design wuerde MVP um 2-4 Wochen verzoegern
- Trait-basiert = null Latenz, type-safe, einfach
- WASM-Layer kann spaeter (v2.x) als zusaetzlicher `WasmPluginLoader` nachgeruestet werden, der dieselbe `PluginRegistry` fuellt

**Architektur-Komponenten:**

1. **Trait pro Erweiterungspunkt** (~10 Punkte: SttProvider, CleanupStyle, OutputTarget, VoiceCommandHandler, AudioFilter, TextFilter, etc.)
2. **Zentrale `PluginRegistry`** im Core mit `register_*`-Methoden
3. **Plugin-Crates** (`klarvo-plugin-groq`, `klarvo-plugin-notion`, etc.) mit `init(&mut PluginRegistry)`-Funktion
4. **Cargo-Features** im Shell-Binary steuern, welche Plugins gebaut werden (z.B. Free vs. Paid Build → Lizenz-Gating wird Build-Zeit-Sache)
5. **Pipeline-Manifest** in TOML statt hardcoded Pipeline:
   ```toml
   [pipeline]
   stages = [
     { type = "audio_capture" },
     { type = "vad", plugin = "silero" },
     { type = "stt", plugin = "groq" },
     { type = "text_filter", plugin = "hallucination_filter" },
     { type = "cleanup", plugin = "verbatim" },
     { type = "output", plugin = "paste" },
     { type = "history_save" },
   ]
   ```
   → Pipeline-Aenderungen ohne Code-Aenderung moeglich; Power-User koennen v2.x ihre eigene Pipeline definieren

**Implikation fuer Phase 0:**
- Trait-Definitionen sind erstes Artefakt im `klarvo-core`-Crate
- Provider-Pattern aus v1 (sauber!) wird zur Vorlage fuer alle Erweiterungspunkte
- Plugin-Discovery via Cargo-Features erfordert workspace-strukturiertes Cargo-Setup

### Frage 12: Datenmodell (2026-04-17)
**Entscheidung (Andy: "1. ja, A  2. A, 3. D"):**

1. **Storage-Engine:** SQLite via `rusqlite` (wie v1). Cross-Platform bewaehrt, simpel.
2. **Cloud-Sync:** Turso behalten (libsql ist SQLite-API-kompatibel, Schema bleibt).
3. **Plugin-Daten:** Eigene Tabellen pro Plugin via Migration-Trait:
   ```rust
   pub trait Plugin {
       fn id(&self) -> &str;
       fn migrations(&self) -> Vec<Migration> { vec![] }
   }
   pub struct Migration { pub version: u32, pub sql: &'static str }
   ```
   Core sammelt alle Plugin-Migrations beim Start, faehrt idempotent.
4. **App-Konfiguration:** Hybrid — System-Settings (DB-Pfad, App-Lang, ~5 Felder) in TOML, User-Settings (Hotkeys, Provider, API-Keys, 40+ Felder) in SQLite-Tabelle `settings(key, value)`.

**Bonus-Implikation:** API-Keys in SQLite ermoeglichen v2.x-Verschluesselung (Master-Password oder OS-Keystore) ohne Format-Umstellung. Loest das bekannte v1-TODO (Plain-Text-Keys in config.json).

### Frage 13: Migration v1→v2 (2026-04-17)
**Entscheidung: Option B — Einmalige Import-Funktion**

v2 bietet beim ersten Start einen „v1-Daten importieren"-Button:
- Liest `klarvo.db` + `config.json` vom v1-Installationspfad
- Migriert auf v2-Schema (History, Dictionary, API-Keys, Hotkey-Config)
- Ein-Klick-Prozess im Onboarding, nicht automatisch/still

**Begruendung:** Andy hat eigene Nutzung + History in v1, zusaetzlich ein paar Tester. Import-Aufwand ~1-2 Tage, User-Wert hoch. Stille Automatik wurde explizit verworfen (vermeidet „wo sind meine Daten?"-Fragen).

**Scope:** History, Dictionary, API-Keys, Hotkey-Config, Cleanup-Style-Preferences. NICHT migriert: internes Debug-State, Lizenz-Cache (muss neu validiert werden).

### Frage 14: Naming (2026-04-17)
**Entscheidung: Klarvo — keine v2-Versionsnummer nach aussen**

- Nach aussen: „Klarvo" (Marke bleibt)
- Nach aussen v2 wird als „Klarvo 1.0" released (erstmals stabile Version)
- Intern: `klarvo-v2` als Repo/Branch-Name
- v1-Codebase wird archiviert, nicht mehr released

### Frage 15: Offline/Cloud-Default (2026-04-17)
**Entscheidung: Cloud-First Default**

Bei Fresh Install default: Groq STT + DeepSeek LLM, BYOK im Onboarding.

**Begruendung:** Offline ist im MVP nicht drin (P1/P2). Cloud-First ermoeglicht „erstes erfolgreiches Diktat in 2 Minuten" — kritisch fuer Onboarding-Success. Privacy-Nutzer aktivieren Offline explizit.

### Frage 16: OS-Lizenz-Strategie (2026-04-17)
**Entscheidung: PolyForm Noncommercial 1.0.0 behalten**

Keine Aenderung zum v1-Stand (2026-03-27 entschieden). Lizenz-Modell ist ausgearbeitet, nicht der Schmerzpunkt.

### Frage 17: Team/Agenten-Setup (2026-04-17)
**Entscheidung: Jetzt nicht entscheiden — operationale Frage**

Nach Phase 0 (wenn `klarvo-core`-Crate steht) entscheiden, basierend auf tatsaechlichem Arbeitsfluss. BMad-Workflow liefert Strukturierung, Agenten-Struktur passt sich an.

---

## Status: Grill-Session abgeschlossen

Alle 17 strategischen Fragen sind beantwortet:
- Plattform-Strategie: Windows + Android parallel MVP, dann iOS, dann macOS
- Architektur: Shared Rust Core + native Shells, Hybrid UI
- MVP-Scope: ~40-45 von 107 Features, Phasenplan A
- Plugin-Architektur: Trait-basiert + Pipeline-Manifest + Cargo-Features
- Datenmodell: SQLite + Turso, eigene Plugin-Tabellen, Hybrid-Konfig
- Cleanup-Styles: Verbatim Default, Polished neu bauen
- Lizenz im MVP: HMAC + Trial + Cache + Grace
- Migration: Einmaliger Import-Button
- Naming: Klarvo (keine v2-Nummer nach aussen)
- Offline/Cloud: Cloud-First Default
- OS-Lizenz: PolyForm NC 1.0.0
- Team-Setup: erst nach Phase 0

**Nicht mehr offen** — alle urspruenglich 11 Fragen sind entweder geklaert oder operational-spaeter.

---

## Naechster Schritt — Uebergang zu BMad-Workflow

Dieses Dokument ist jetzt ein **abgeschlossener Strategie-Snapshot**. Der naechste Schritt ist die formale Ueberfuehrung in BMad-Artefakte:

1. **`/bmad-product-brief`** in frischem Context-Fenster — wandelt diese Entscheidungen in ein formales Produktkonzept-Dokument (`output/planning-artifacts/product-brief.md`)
2. **`/bmad-create-architecture`** — technische Architektur-Spec (Trait-Definitionen, Crate-Struktur, Shell-Interfaces)
3. **`/bmad-create-prd`** — PRD fuer Phase 0 (Rust Core + Pipeline, headless, Trait-Boundaries)
4. **`/bmad-create-epics-and-stories`** — Phase 0 in umsetzbare Stories

Dieses `rebuild-discussion.md` bleibt als Referenz — wenn Detail-Fragen in spaeteren Sessions aufkommen, kann man hier die urspruengliche Begruendung nachlesen.

---

## Zusammenfassung der Rebuild-Vision (final 2026-04-17)

Klarvo (v2 intern), gebaut auf:

1. **Shared Rust Core** (`klarvo-core`-Crate) — Geschaeftslogik plattform-agnostisch als Library, headless testbar
2. **Trait-basierte Plugin-Architektur** — ~10 Erweiterungspunkte (SttProvider, CleanupStyle, OutputTarget, VoiceCommandHandler, etc.), zentrale PluginRegistry, Pipeline-Manifest in TOML
3. **Native Shells**: Tauri (Windows), Kotlin (Android), spaeter Swift (iOS) und Tauri/native (macOS)
4. **Hybrid UI** — React WebView fuer App-UI, plattform-native Overlays (FloatingBar / Bubble / MenuBar)
5. **SQLite + Turso** — wie v1, plus eigene Plugin-Tabellen via Migration-Trait, Hybrid-Konfiguration (TOML System + SQLite User-Settings)
6. **Cargo-Workspace mit Features** — Plugin-Set zur Build-Zeit konfiguriert, Free/Paid-Gating als Build-Feature
7. **Multi-Platform-Reihenfolge**: Windows + Android parallel MVP → iOS → macOS

**Timeline:** 2-4 Monate Vollzeit, 5 interne Phasen (0-4)
**Migration:** Einmaliger Import-Button in v2-Onboarding (History, Dictionary, API-Keys, Hotkey-Config aus v1)
