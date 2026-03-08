# Dikta

Freie, selbst gehostete Alternative zu Wispr Flow -- Sprachdiktat mit KI-Text-Cleanup fuer Windows Desktop und Android.

Sprache in jedem Textfeld systemweit in bereinigten Text umwandeln. Kein Abo, keine Cloud-Abhaengigkeit, alles gehoert dem Nutzer.

## Quickstart

**Voraussetzungen:** Node.js, Rust/Cargo, Tauri v2 CLI

```bash
# Dependencies installieren
npm install

# Entwicklungsserver starten (Tauri + Vite)
npm run tauri dev

# Windows Release-Build
npm run tauri build
```

**Windows-Build aus WSL heraus** (sync + build auf Windows-Seite):

```powershell
powershell -ExecutionPolicy Bypass -File \\wsl$\Ubuntu\home\andyon2\dikta\scripts\sync-and-build.ps1
```

**API-Keys konfigurieren:** `.env` im Projektroot anlegen (siehe `.env.example`) oder ueber das Settings-Panel in der App.

## Team

### Agents (3) -- denken eigenstaendig

| Agent | Was er tut | Wie starten |
|-------|-----------|-------------|
| **rust-core** | Rust-Backend: Audio-Capture, STT-Pipeline, LLM-Client, Paste, Hotkey, Dictionary, Config. Alles in `src-tauri/`. | Wird vom Main-Agent delegiert |
| **ui-dev** | React-Frontend: Overlay, Settings, Dictionary-UI, Onboarding, Styling. Alles in `src/`. | Wird vom Main-Agent delegiert |
| **android-platform** | Android: IME (System-Keyboard), Tauri-Bridge, Permissions, Background-Services. Alles in `android/`. | Delegiert oder direkt via `scripts/android-platform` |

Alle Agents laufen auf Sonnet. Der Main-Agent (`main-agent.md`) ist der Tech Lead -- er plant, delegiert, reviewt, schreibt selbst keinen Code.

### Skills (8) -- fuehren Workflows aus

| Skill | Was er tut |
|-------|-----------|
| `/scaffold [typ] [name]` | Neues Rust-Modul, React-Component oder Android-Service aus Template erstellen |
| `/build [plattform]` | Fuer Windows oder Android bauen, strukturierte Fehlermeldung bei Problemen |
| `/run-tests [scope]` | Tests ausfuehren (rust / frontend / all), formatierter Report |
| `/research-api [thema]` | API-Docs recherchieren, Summary in `knowledge/` schreiben |
| `/lint-fix [scope]` | Linter + Formatter laufen lassen, Auto-Fix wo moeglich |
| `/plan-feature [beschreibung]` | Feature in Tasks zerlegen mit Dateien, Abhaengigkeiten, Agent-Zuweisung |
| `/commit-progress [beschreibung]` | Git-Commit mit Conventional-Commit-Message |
| `/debug-error [fehler]` | Fehler analysieren, Root Cause finden, Fix-Vorschlag mit zustaendigem Agent |

## Direkte Sessions

Der **android-platform** Agent kann als interaktive Session gestartet werden -- fuer iteratives Debugging (IME, Permissions, Android-Build-Fehler):

```bash
scripts/android-platform
```

Falls ein Briefing vom Main-Agent unter `briefings/android-platform-*.md` liegt, liest der Agent es automatisch.

## Tech-Stack

| Schicht | Technologie |
|---------|-------------|
| Desktop-Framework | Tauri v2 (Rust-Backend + Web-Frontend) |
| Frontend | React 19 + TypeScript + Tailwind CSS v4 |
| Backend | Rust (Audio, STT, LLM, Paste, Hotkey, Config) |
| Mobile | Tauri v2 Android + Kotlin fuer IME |
| STT | Groq Whisper API (primaer) + whisper.cpp (offline, geplant) |
| Text-Cleanup | DeepSeek API (primaer), OpenAI, Anthropic, Groq/Llama (konfigurierbar) |
| Speicherung | JSON (Config, Dictionary), SQLite (History, Stats) |
| Auto-Update | tauri-plugin-updater mit GitHub Releases Endpoint |

## Features (Stand Phase 5)

- **End-to-End Diktat-Pipeline:** Record -> Transcribe -> Cleanup -> Paste ins aktive Fenster
- **Multi-Provider STT:** Groq Whisper, OpenAI Whisper (konfigurierbare Prioritaet, Drag & Drop)
- **Multi-Provider LLM:** DeepSeek, OpenAI, Anthropic, Groq/Llama (konfigurierbare Prioritaet)
- **3 Schreibstile:** Polished (bereinigt), Verbatim (woertlich), Chat (locker)
- **Command Mode:** Text selektieren, Sprachbefehl geben (Ctrl+Shift+E)
- **Whisper Mode:** Verstaerkung fuer leises Diktieren
- **Live Preview:** Echtzeit-Transkript waehrend der Aufnahme
- **Live Translation:** Output-Sprache konfigurierbar (13 Sprachen)
- **Multi-Format Output:** Email, Bullets, Summary
- **Custom Dictionary:** Fachbegriffe, die STT und LLM beibehalten sollen
- **App Profiles:** Style/Language/Prompt pro App automatisch
- **History + Volltextsuche:** SQLite, nach Text und App-Name durchsuchbar
- **Voice Notes:** Aufnahmen speichern statt pasten
- **Text Snippets:** Textbausteine schnell ins aktive Fenster einfuegen
- **Usage Statistics:** Kosten-Tracking fuer STT/LLM, Fuellwort-Analyse
- **Webhook/API Export:** HTTP POST nach jedem Diktat
- **System-Tray:** Minimize to Tray, Tray-Menue
- **Auto-Update Infra:** tauri-plugin-updater + GitHub Releases (Signing-Keys noch ausstehend)
- **Globaler Hotkey:** Konfigurierbar, Hold-Modus oder Toggle-Modus
- **Onboarding-Wizard:** Ersteinrichtung fuer API-Keys und Einstellungen
- **178 Rust-Tests**

## Projektstruktur

```
dikta/
  CLAUDE.md                          # Projektkontext fuer Claude
  main-agent.md                      # System-Prompt: Tech Lead
  project-status.md                  # Zentraler Projektstatus (Sessionstart lesen!)
  package.json                       # v0.4.0, React 19 + Tauri v2
  vite.config.ts                     # Vite 7 + Tauri-Plugin
  tsconfig.json                      # TypeScript-Config
  .env.example                       # API-Key-Vorlage
  .claude/
    agents/
      rust-core.md                   # Rust-Backend-Entwickler
      ui-dev.md                      # Frontend-Entwickler
      android-platform.md            # Android-Plattform-Spezialist
    skills/
      scaffold/SKILL.md              # Modul/Component aus Template
      build-app/SKILL.md             # Plattform-Build
      run-tests/SKILL.md             # Tests + Report
      research-api/SKILL.md          # API-Recherche
      lint-fix/SKILL.md              # Linter + Formatter
      plan-feature/SKILL.md          # Feature-Planung
      commit-progress/SKILL.md       # Git-Commit
      debug-error/SKILL.md           # Fehler-Analyse
  src/                               # React-Frontend
    App.tsx                          # Haupt-App (Overlay, Settings, History, ...)
    FloatingBar.tsx                   # Floating Bar am Bildschirmrand
    Onboarding.tsx                   # Ersteinrichtungs-Wizard
    tauri-commands.ts                # Typisierte Tauri-IPC-Aufrufe
    media-recorder.ts                # Audio-Aufnahme (Browser-API-Fallback)
    platform.ts                      # Plattform-Erkennung
    types.ts                         # TypeScript-Interfaces
    styles.css                       # Tailwind + Custom Styles
  src-tauri/src/                     # Rust-Backend
    lib.rs                           # Tauri-Commands, AppState, Pipeline
    main.rs                          # Tauri-App-Entry
    audio/                           # Audio-Capture (cpal), WAV, Silence Detection
    stt/                             # SttProvider Trait, Groq Whisper, OpenAI Whisper
    llm/                             # CleanupProvider Trait, DeepSeek/OpenAI/Anthropic/Groq
    paste/                           # Text-Insertion (Win32 SendInput + Clipboard)
    hotkey/                          # Pipeline-Orchestrierung, Event-Emitter, Command Mode
    config/                          # JSON Settings + App Profiles + Snippets + Webhook
    dictionary/                      # Custom-Woerterbuch (JSON)
    history/                         # SQLite History + Voice Notes + Stats + Filler-Analyse
    sync/                            # Sync-Modul
  android/                           # Android-Plattformcode
    kotlin-src/                      # Kotlin-Quellcode (IME, Services)
    res-values/                      # Android-Ressourcen
    res-xml/                         # XML-Konfigurationen (IME-Registrierung)
  scripts/
    android-platform                 # Starter fuer direkte Android-Sessions
    sync-and-build.ps1               # WSL -> Windows Sync + Build (PowerShell)
  briefings/                         # Briefing-Dokumente fuer Agent-Sessions
  knowledge/
    architecture.md                  # Tech-Entscheidungen und Patterns
    api-providers.md                 # Groq + DeepSeek + weitere API-Details
    platform-notes.md                # Windows/Android-Quirks, Lessons Learned
    competitors.md                   # Wettbewerber-Analyse
    wispr-flow-android-ux.md         # Wispr Flow Android-UX-Recherche
  marketing/
    dikta-pitch.html                 # Pitch-Deck (HTML)
    dikta-pitch.pdf                  # Pitch-Deck (PDF)
  .tauri/                            # Tauri-Build-Konfiguration
  dist/                              # Build-Output
```

## Session-Workflow

**Start:** Der Main-Agent liest `project-status.md` (aktueller Stand, offene Tasks), dann `knowledge/architecture.md` (geltende Tech-Entscheidungen), dann `git status` fuer Aenderungen seit der letzten Session.

**Dispatch-Check:** Der Main-Agent prueft bei jedem Sessionstart `~/project-builder/dispatches.md` nach offenen Eintraegen fuer **dikta**. Falls vorhanden: Dispatch-Notiz lesen, zusammenfassen, fragen ob eingearbeitet werden soll. Verarbeitete Eintraege abhaken.

**Waehrend:** Der Main-Agent delegiert Aufgaben an die passenden Agents (rust-core, ui-dev, android-platform) oder nutzt Skills fuer Standardaufgaben. Architektur-Entscheidungen werden in `knowledge/architecture.md` dokumentiert. Plattform-Quirks landen in `knowledge/platform-notes.md`.

**Ende:** `project-status.md` wird aktualisiert (Stand, offene Aufgaben, naechste Session). Erledigte Aufgaben aelter als 2-3 Sessions werden entfernt. Architektur-Entscheidungen in Knowledge-Dateien festgehalten.

## Offene Aufgaben

- Lokaler whisper.cpp Fallback (offline STT)
- Android IME (Tauri v2 Mobile + Kotlin InputMethodService)
- VAD -- Voice Activity Detection (Auto-Start/Stopp)
- Signing Keys fuer Tauri Updater
- GitHub Releases CI/CD Pipeline fuer Auto-Update

## Regeln

1. **Code-Sprache ist Englisch.** Variablen, Funktionen, Kommentare, Commits.
2. **Architektur-Entscheidungen dokumentieren** in `knowledge/architecture.md` -- nicht nur "was", sondern "warum".
3. **API-Keys nicht im Code.** Immer ueber `.env` oder System-Keystore.
4. **Jedes neue Modul bekommt Tests.**
5. **Plattform-Code hinter Abstraktionen.** Nie `#[cfg(target_os)]` in der Business-Logik.
6. **Kleine Commits, oft.**
7. **Bei Unsicherheit: `/research-api` nutzen** statt zu raten.
