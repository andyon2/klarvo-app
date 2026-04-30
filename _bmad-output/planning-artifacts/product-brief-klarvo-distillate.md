---
title: "Product Brief Distillate: Klarvo"
type: llm-distillate
source: "product-brief-klarvo.md"
created: "2026-04-17"
purpose: "Token-efficient context for downstream PRD creation, architecture spec, and story breakdown"
---

# Klarvo — Brief Distillate

Dichte Kontext-Sammlung für Downstream-Workflows (`/bmad-create-architecture`, `/bmad-create-prd`, `/bmad-create-epics-and-stories`). Strukturiert nach Themen, nicht nach Chronologie. Jeder Punkt ist standalone lesbar.

## Kernentscheidung & Kontext

- **Rebuild-Entscheidung: Clean Slate**, nicht Refactor von v1. Entschieden 2026-04-14, strategisch finalisiert 2026-04-17.
- **v1 bleibt als Referenz, wird archiviert, nicht mehr released.** Neue Version nach außen heißt „Klarvo 1.0" (keine v2-Nummer). Intern `klarvo-v2` als Repo/Branch.
- **Early Access wurde bewusst zurückgezogen.** Keine aktiven Tester, keinen Release-Druck. Tester sind informiert „noch viele Wochen".
- **Zeitliches Marktfenster-Signal**: Konkurrierende Diktat-Apps häufen sich in den letzten Wochen. Markt sättigt horizontal. Klarvos Antwort: vertikale Nischen via Cargo-Feature-Architektur, nicht Geschwindigkeit.
- **OS-Lizenz: PolyForm Noncommercial 1.0.0** bleibt (wie v1, entschieden 2026-03-27). Kein Änderungsbedarf.
- **Primärsprache des Gründers/Hauptnutzers: Deutsch.** Multi-Language ist First-Class, nicht Übersetzungs-Overlay.

## v1 Deep-Scan-Fakten (begründen Rebuild)

- **~85 % Android-Bypass**: Android-Kotlin nutzt nur ~15 % des Rust-Backends. HTTP-Calls direkt via `java.net.HttpURLConnection` zu Groq/DeepSeek/OpenAI, NICHT über Tauri Commands. JNI nur für Offline-STT (whisper-rs) und Offline-LLM (MNN).
- **~2.000 LOC dupliziert** zwischen `src-tauri/` und `android/kotlin-src/`: STT-Calls, LLM-Cleanup-Prompts, Sync-Logik, History-Schema.
- **44 % des Rust-Codes ist `#[cfg]`-plattform-fragmentiert.**
- **62 Tauri Commands**, davon ~40 plattform-spezifisch (Hotkey, Window, Updater).
- **Tauri-Mehrwert ist zu ~80 % Windows-only** (Hotkeys, Updater, Tray).
- **Security-Report v1**: 3 kritisch, 6 hoch (CSP deaktiviert, Test-Lizenzen in Prod, hardcoded Secrets in config.json).
- **Kein Frontend-Testframework in v1.**
- **Fazit der Analyse**: Architektur-Mismatch, nicht Code-Qualitätsproblem. Tauri v2 ist das falsche Framework für Multi-Plattform-Strategie.

## Architektur (siehe Architecture-Spec für Details)

### Kern-Prinzip
- **Shared Rust Core** (`klarvo-core` Crate): Pipeline, STT, LLM, History, Sync, Plugin-System — plattform-agnostisch, headless testbar.
- **Native Shells**: Tauri (Windows) → später Swift (iOS), Tauri oder nativ (macOS), Kotlin (Android). Shells hosten nur UI + plattform-spezifische Services (Hotkeys, Overlays, AccessibilityService).
- **Hybrid UI**: React WebView für App-UI (Settings, History, Onboarding) via Tauri/WebView; plattform-native Overlays (FloatingBar auf Desktop, Bubble auf Android, später MenuBar auf macOS).

### Plugin-System (Trait-basiert, compile-time)
- **~10 Traits** als Erweiterungspunkte: SttProvider, CleanupStyle, OutputTarget, VoiceCommandHandler, AudioFilter, TextFilter, u. a.
- **Zentrale `PluginRegistry`** in `klarvo-core` mit `register_*`-Methoden.
- **Plugin-Crates** (`klarvo-plugin-groq`, `klarvo-plugin-notion`, etc.) mit `init(&mut PluginRegistry)`-Funktion.
- **Cargo-Features** im Shell-Binary steuern Plugin-Set zur Build-Zeit. Lizenz-Gating (Free/Paid) und Nischen-Varianten (Medical/Legal/Accessibility) werden als Build-Feature umgesetzt, nicht Runtime-Limits.
- **Pipeline-Manifest in TOML** statt hardcoded:
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
- **Migration-Trait für Plugin-Daten**: `Plugin::migrations() -> Vec<Migration>`. Core sammelt + fährt idempotent.

### Datenmodell
- **Storage**: SQLite via `rusqlite` (wie v1). Cross-Platform bewährt.
- **Cloud-Sync**: Turso (libsql, SQLite-API-kompatibel). Schema bleibt.
- **Plugin-Daten**: Eigene Tabellen pro Plugin via Migration-Trait.
- **App-Konfiguration (Hybrid)**: System-Settings (~5 Felder: DB-Pfad, App-Lang) in TOML; User-Settings (Hotkeys, Provider, API-Keys, 40+ Felder) in SQLite-Tabelle `settings(key, value)`.
- **API-Keys in SQLite** (nicht JSON) ermöglicht v2.x-Verschlüsselung via Master-Password oder OS-Keystore ohne Format-Umstellung. Löst v1-TODO (Plain-Text-Keys in config.json).

### Abgelehnte Architektur-Alternativen (mit Begründung)
- **Dynamisch geladene .so/.dll-Plugins**: ABI-Stabilität cross-platform ist Albtraum.
- **WASM-Plugin-Layer ab Tag 0**: verzögert MVP um 2–4 Wochen; ~70 % der konkreten Use Cases sind First-Party und brauchen kein Sandboxing. WASM kommt in v2.x als zusätzlicher `WasmPluginLoader`, der dieselbe `PluginRegistry` füllt.
- **Separate Prozesse + IPC**: 1–10 ms Latenz pro Call ist im Diktat-Flow zu viel.
- **Pures nativ (kein WebView für App-UI)**: Aufwand zu hoch für Solo-Dev-Timeline.

## Plattform-Strategie

- **Windows + Android sind GLEICH wichtig — beide MVP, parallel ab Tag 1.**
- **iOS folgt nach MVP.** Reihenfolge danach: macOS.
- **Linux opportunistisch** (kein explizites Ziel).
- **Implikation für Phase 0**: Shared Rust Core wird sofort auf 2 Plattformen validiert. Aufwand initial höher. Braucht Trait-Boundaries, JNI-Bridge, Headless-Test-Infra ab Tag 1.

## MVP-Scope (~40–45 von 107 v1-Features)

### MVP-enthalten (Section-by-Section)

| Section | MVP-Inhalt |
|---------|-----------|
| Core Pipeline | STT, LLM, Auto-paste, Clipboard-Fallback, Insert-Send, History-Save, Min-Duration, Hallucination-Filter, Prompt-Stripping, Output-Language |
| Recording Modes | Hold, Toggle, AutoStop (Windows); alle Android-Modi (Tap-HOLD/TOGGLE, Long-Press PTT/AUTOSTOP) |
| Hotkey System | 2 Slots (skaliert später auf 4–5), Pause/Resume, ShortcutRecorder, Active-Mode-Badge |
| Text Processing | Verbatim (**neuer Default**), Chat, Polished (**NEU GEBAUT** — nicht aus v1 portiert), Auto-Capitalize |
| Audio | Device-Selection, RMS-Silence-Detection, Live-Audio-Events, WAV-Encoding |
| Providers | Groq Whisper, DeepSeek LLM, STT-Priority-List + Fallback, Live-API-Key-Validation, STT-Conditioning |
| UI/UX | Minimales Main-Settings, komplette Floating Pill Bar (Shape, Drag, Position, Waveform), Return-Focus, Tray, Onboarding-Stub, Quick Tips, StylePicker, History-Panel |
| History & Stats | Dictation History, Delete, Clear All |
| Android-spezifisch | ALLE 17 Features (Bubble mit 5 Zuständen, Gesten, AccessibilityService, Konfiguration) |
| Sync & Cloud | — (kommt in P1) |
| Dictionary | Custom Dictionary (capped) |
| Offline / Local Whisper | — (kommt in P1/P2) |
| License System | HMAC-Validation, Permanent, Trial, 30-Tage-Cache, 48h-Grace, Status-Display |
| Advanced / Power User | — (kommt in P1/P2) |

### P1 (kurz nach MVP)
Auto-Turso-Sync, OpenAI Whisper/LLM, Groq LLM, Reformate (Email/Bullets/Summary), Whisper-Mode (Gain), Stats-Panel, History-Search, Cost-Tracking, unlimitiertes Dictionary, Whisper-Model-Manager (small/medium), Webhook-Integration, Auto-Loop, UI-Scale, Autostart, Hot-Reload-Providers.

### P2 (Power-Features)
Anthropic, OpenRouter, Provider-Model-Overrides, Custom Prompts, App-Profiles, Command-Mode/Hotkey, Voice-Notes, Snippets, Filler-Word-Analysis, Local Whisper Large + GPU/CUDA, alle Threshold-Configs.

### DEFER / nicht in v2
- Live-Transcription-Preview (war in v1 deaktiviert).
- Integrations-Panel als zentrale Kachel-UI (Integrationen kommen als Plugins, nicht als Panel).
- Early-Adopter 60-Tage-Grace (v1-spezifisch).
- Voice-Commands in v1-Form — werden als natives Plugin neu konzipiert.

## Cleanup-Styles (wichtige Designänderung)

- **Verbatim wird neuer Default** (statt Polished wie in v1).
- **Polished wird vollständig neu gebaut**, nicht aus v1 portiert. v1-Polished „macht zu viel kaputt" (zu aggressives Umschreiben).
- **Designziel für neuen Polished**: „Filler weg, Grammatik korrekt, aber Stimme bleibt" — nicht „professionell umformuliert".
- **Chat bleibt wie in v1.**
- **Marketing/Onboarding-Implikation**: v1-Kommunikation verkauft Polished als Hero-Style. Für Klarvo 1.0 muss das komplett umgestellt werden: Verbatim ist Hero, Polished ist Option.

## Migration v1 → v2

- **Einmaliger Import-Button im v2-Onboarding** (nicht automatische stille Migration).
- **Migriert**: History, Custom Dictionary, API-Keys, Hotkey-Config, Cleanup-Style-Preferences.
- **Nicht migriert**: internes Debug-State, Lizenz-Cache (muss neu validiert werden).
- **Source-Pfade**: `klarvo.db` (SQLite) + `config.json` aus v1-Installationspfad.
- **Aufwand-Schätzung**: ~1–2 Tage Entwicklung, User-Wert hoch (Andy hat eigene v1-Nutzung + History).

## Onboarding-Default

- **Cloud-First** bei Fresh Install: Groq STT + DeepSeek LLM als Default-Stack.
- **BYOK** wird im Onboarding-Schritt abgefragt (nicht später).
- **Ziel**: „Erstes erfolgreiches Diktat in unter 2 Minuten" als Onboarding-Success-Metrik.
- **Offline ist P1/P2**, nicht im MVP. Privacy-Nutzer aktivieren Offline explizit, wenn verfügbar.

## Lizenz-System (MVP)

- **HMAC-Validation** + Permanent + Trial + 30-Tage-Cache + 48h-Grace direkt im MVP (Phase 4).
- **Lemon Squeezy (Payment-Integration) ist P1**, nicht im MVP.
- **Free/Paid-Gating** als Cargo-Feature, nicht als Runtime-Limit.

## Phasenplan (intern, alles bleibt MVP)

| Phase | Inhalt | Ziel |
|-------|--------|------|
| 0 | Rust Core + Pipeline (headless, Trait-Boundaries, JNI-Bridge, Test-Infra, Cargo-Workspace) | Architektur validiert, beide Plattformen ready |
| 1 | Windows-Shell mit Pipeline durchgehend (1 Mode, 1 Hotkey, kein UI-Polish) | Erstes Diktat funktioniert auf Windows |
| 2 | Windows-Shell vollständig (alle Modi, beide Hotkeys, komplette Pill Bar) | Windows daily usable |
| 3 | Android-Shell vollständig (alle Bubble-Zustände, Gesten, AccessibilityService) | Android daily usable |
| 4 | Lizenz-System + Settings-Polish + Cleanup-Style-Refactor (Polished neu) | MVP komplett |

- **Timeline konservativ**: 3–5 Monate Vollzeit (ursprünglich 2–4 ohne Puffer; bewusst realistischer).
- **Puffer-Bedarf**: JNI-Stolpersteine, Plattform-Überraschungen, Lernkurve Solo-Dev.
- **Team/Agent-Setup**: Nicht jetzt entschieden — operationale Frage nach Phase 0, wenn `klarvo-core` steht.

## Zielnutzer (Priorisierung)

1. **Primär**: Schreibender Power-User (Andy-Archetyp). Win + Android. Täglich große Textmengen. BYOK-bereit. Aha-Moment: erster langer Mail-Entwurf in 90 s statt 8 min.
2. **Sekundär**: Modularer Entwickler. Nutzt Plugin-Ökosystem + Pipeline-Manifest als Interface.
3. **Tertiär**: Institute / Organisationen (Kanzlei, Arztpraxis, Forschungsgruppe, Redaktion). Einmaliges Setup, Custom-Build via Cargo-Features mit domänen-spezifischem Dictionary / LLM-Endpoint.
4. **Signifikante Sekundärgruppe**: RSI- und motorisch eingeschränkte Nutzer. Nicht primärer Fokus, aber architektonisch first-class (AccessibilityService, Keyboard-Ergonomie, tiefes Dictionary). Wird in Kommunikation und Onboarding sichtbar.

**Bewusst nicht Zielgruppe**: Mass-Market-Normalnutzer ohne Tech-Affinität — BYOK-Reibung ist Akzeptanz-Filter, nicht Hürde, die entfernt werden soll.

## Go-to-Market (Markteintrittslogik)

- **Nischen-Strategie statt Massenmarkt**: Cargo-Feature-Architektur erzeugt Klarvo-Varianten (Medical / Legal / Science / Editorial / Accessibility) als echte Custom-Builds, nicht Settings-Toggles. Das ist horizontal nicht nachbaubar.
- **MVP-Launch-Kanäle**: informierte v1-Tester re-aktivieren (First-Wave-Kohorte), Product Hunt, Hacker News, Multi-Platform-Power-User-Communities (r/windows, Android-Foren).
- **Nischen-Anbahnung parallel zum MVP**: B2B/Institutional-Channel, domänen-spezifische Beratung / Partnerschaften. Konkrete Nischen-Ideen existieren (Andy), sind aber noch nicht im Brief verortet.
- **Pricing-Signal**: BYOK + PolyForm-NC signalisiert Ernsthaftigkeit, filtert auf qualifizierte Nutzer.

## Erfolgskriterien (messbar vs. qualitativ)

### Messbar (MVP-Abschluss)
- Pipeline durchgehend auf Windows + Android mit allen Recording-Modi + Pill Bar + Bubble komplett.
- Fresh Install → erstes Diktat in < 2 min.
- v1-Import migriert History + Dictionary + API-Keys + Hotkey-Config in einem Klick.
- Lizenz-System (HMAC, Trial, 30-Tage-Cache, 48h-Grace) funktioniert auf beiden Plattformen.

### Architektonisch
- 0 LOC Geschäftslogik dupliziert zwischen Rust-Core und Shells (v1-Baseline: ~2.000).
- `klarvo-core` headless testbar mit sinnvoller Test-Coverage.
- Neuer STT-Provider oder Cleanup-Style erfordert keine Shell-Änderungen.

### Qualitativ (bewusst weich, Mechanismus statt Metrik)
- Regressions-Disziplin: Feature-Entwicklung fühlt sich nicht mehr an wie Brand-Löschen. Operationalisiert durch Specs-vor-Code + headless Core + Review-Disziplin.
- Keine v1-Baseline für Regression-Rate existiert sauber, daher keine Ziel-Zahl.

## Technische Leitplanken (für Downstream-Workflows)

- **Rust-Workspace-Struktur** ist Voraussetzung ab Phase 0 (nicht optional).
- **Provider-Pattern aus v1 (sauber!)** ist Template für alle Trait-Implementierungen in v2.
- **Bei jeder Pipeline-Änderung**: erst Trait definieren, dann Plugin-Crate, dann `init()` registrieren. Keine Direktcalls in Pipeline.
- **Trait-Boundaries im Core**: null Tauri-spezifischer Code im Core. Jeder Tauri-Import im Core ist Architektur-Verletzung.
- **Plattform-Duplikat-Check**: Bei jeder STT/LLM/Config/History-Änderung in v1-Code explizit klären, ob Desktop+Android+beide betroffen sind (gilt während Parallelbetrieb v1/v2).

## Andy's Arbeitsstil (Collaboration-Context)

- **Specs vor Code**: Vor Implementierung klären — PRD / Story / Acceptance Criteria. Keine „mal schnell"-Implementierungen ohne Scope. (BMad ist explizit dafür installiert.)
- **Strategisches Gegenhalten wird respektiert**, wenn mit Fakten belegt (Deep-Scan-Zahlen waren überzeugend).
- **Pragmatisch, nicht nostalgisch**: EA zurückgezogen, Clean Slate gewählt. Keine falsche Rücksicht auf „aber das ist doch schon implementiert".
- **Parallel-Sessions**: `docs/rebuild-discussion.md` ist Shared-State zwischen parallelen Claude-Sessions. Bei Detailfragen dort zuerst nachschauen.

## Offene Fragen (für PRD-, Architektur- und GTM-Workflows)

- **Android AccessibilityService + Google-Play-Policy**: Play Store prüft zunehmend restriktiv. v1 war Sideload. Für Klarvo 1.0 irgendwann Thema. Status: **„weiß noch nicht"** — muss vor Android-Release geklärt werden (Phase 3 spätestens).
- **Konkrete Nischen-Markt-Ideen**: Andy hat neue Ideen für spezifische Zielgruppen / Nischen-Märkte, aber nicht im Brief verortet. Separate Arbeitsstränge.
- **Beta-Testing-Plan für Klarvo 1.0**: Soll es eine Beta-Phase geben, oder direkt Release? Keine Entscheidung.
- **Erste zahlende Nutzer**: Konkrete Quellen / Kanäle noch nicht definiert (v1-Tester-Reaktivierung ist erste Hypothese, mehr fehlt).
- **Team/Agent-Setup**: Bewusst nach Phase 0 vertagt — entscheidet sich basierend auf tatsächlichem Arbeitsfluss.
- **Hotkey-Slots**: MVP hat 2, skaliert später auf 4–5. Wann / unter welcher Bedingung?
- **Lemon-Squeezy-Integration-Timing**: P1-Label ist grob. Welcher P1-Meilenstein löst es aus?

## Schlüssel-Zitate (Entscheidungs-Kontext)

- Andy (2026-04-17) zur Klarvo-v1-Frustration: *„Ich war in den letzten ein, zwei Wochen mega unzufrieden mit meinem Klavo-Agenten. Er bekommt einfachste neue Features nicht hin, bringt ganz viele Dinge durcheinander, macht mehr kaputt als er es neu hinkriegt."*
- Andy (2026-04-17) zum Clean Slate: *„Wir koennen uns Zeit nehmen und in Ruhe alles vernuenftig modular aufbauen."*
- Andy (2026-04-17) zu Polished: *„Ich nutze Polished uebrigens nie. Ich nutze immer Verbatim. Polished macht zugleich zuviel kaputt!"*
- Andy (2026-04-17) zur Markt-Situation: Markt sättigt sich, ähnliche Apps popen hoch. Antwort: Nischen-Strategie via Cargo-Feature-Architektur, nicht horizontale Geschwindigkeits-Konkurrenz.
