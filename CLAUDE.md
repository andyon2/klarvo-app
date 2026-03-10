# Dikta

Eine freie, selbst gehostete Alternative zu Wispr Flow. Sprachdiktat mit KI-Text-Cleanup fuer Windows Desktop und Android.

## Ziel

Ein voll funktionsfaehiges Voice-Dictation-Tool bauen, das:
- Sprache in jedem Textfeld systemweit in Text umwandelt (Windows + Android)
- Rohen Transkript-Text per LLM bereinigt (Fuellwoerter, Grammatik, Stil)
- Sowohl online (Groq/DeepSeek API) als auch offline (whisper.cpp lokal) funktioniert
- Komplett dem Nutzer gehoert -- kein Abo, keine Cloud-Abhaengigkeit

## Ich (Andy)

Koordiniert das Projekt, trifft finale Entscheidungen. Wenig Erfahrung mit Android-Entwicklung. Hat einen Windows-Laptop mit GPU (nutzt GPU nur am Strom, nicht auf Akku). Bevorzugt guenstige Cloud-APIs (Groq, DeepSeek) fuer den Normalbetrieb.

## Kommunikation

- Sprache: Deutsch im Chat, Englisch im Code (Variablen, Kommentare, Commits)
- Ton: Direkt, kein Blabla. Technische Details sind willkommen.
- Bei Unsicherheiten: Lieber kurz rueckfragen als falsch abbiegen.

## Tech-Stack

- **Desktop-Framework:** Tauri v2 (Rust-Backend + Web-Frontend)
- **Frontend:** React + TypeScript + Tailwind CSS
- **Backend:** Rust (Audio-Capture, STT-Pipeline, LLM-Client, Hotkey, Paste)
- **Mobile:** Tauri v2 Android-Support + Kotlin (Floating Bubble Overlay, DiktaApi, AccessibilityService)
- **STT:** Groq Whisper API (primaer) + whisper.cpp (offline Fallback)
- **Text-Cleanup:** DeepSeek API (primaer, guenstig)
- **Speicherung:** SQLite fuer Dictionary, Settings, History

## Projektstruktur

```
dikta/
  CLAUDE.md                    -- Dieses Dokument (Projektkontext)
  main-agent.md                -- System Prompt fuer den Tech-Lead
  project-status.md            -- Zentraler Projektstatus (bei jedem Sessionstart lesen!)
  .claude/
    agents/
      rust-core.md             -- Rust-Backend-Entwickler
      ui-dev.md                -- Frontend-Entwickler
      android-platform.md      -- Android-Plattform-Spezialist
    skills/
      scaffold/SKILL.md        -- Neues Modul/Component aus Template erstellen
      build-app/SKILL.md       -- Fuer Zielplattform bauen
      run-tests/SKILL.md       -- Tests ausfuehren + Report
      research-api/SKILL.md    -- API-Docs recherchieren + Summary schreiben
      lint-fix/SKILL.md        -- Linter + Formatter laufen lassen
      plan-feature/SKILL.md    -- Feature in Tasks zerlegen
      commit-progress/SKILL.md -- Git-Commit mit konventioneller Message
      debug-error/SKILL.md     -- Fehler analysieren + Fix vorschlagen
      sync-prompts/SKILL.md    -- LLM-Prompts in Rust/Kotlin vergleichen
      release/SKILL.md         -- Version bump + Build + GitHub Release
      track/SKILL.md           -- Projektstatus lesen/aktualisieren
  scripts/
    dikta-tech-lead            -- Starter fuer den Tech Lead (Main-Agent)
    android-platform           -- Starter fuer direkte Android-Sessions
    product-strategist         -- Starter fuer direkte Strategie-Sessions
    rust-core                  -- Starter fuer direkte Rust-Backend-Sessions
  briefings/                   -- Briefing-Dokumente fuer direkte Agent-Sessions
  feedback/
    inbox.md                   -- Tester-Feedback (Bugs, Feature-Wuensche, UX-Probleme)
  sources/
    inbox/                     -- Neue Wissensquellen hier ablegen
    archive/                   -- Verarbeitete Quellen
    log.md                     -- Integrationslog
  knowledge/
    architecture.md            -- Tech-Entscheidungen, Patterns, Modulstruktur
    api-providers.md           -- Groq + DeepSeek API-Details
    competitors.md             -- Wettbewerbsanalysen (Voice Type, Wispr Flow, OpenWhispr)
    product-strategy.md        -- Positionierung, Zielgruppe, Pricing, Differenzierung
    wispr-flow-android-ux.md   -- Wispr Flow Android-UX-Recherche
    (platform-notes.md entfernt -- Quirks stehen in architecture.md Abschnitt "Plattform-Quirks")
  src-tauri/                   -- Rust-Backend (Tauri)
  src/                         -- React-Frontend
  android/                     -- Android-Plattformcode
```

Zentrale Dateien:
- `project-status.md` -- Projektstatus (via `/track` Skill bei Sessionstart gelesen, bei Sessionende aktualisiert)
- `feedback/inbox.md` -- Tester-Feedback (bei jedem Sessionstart pruefen)
- `briefings/` -- Briefing-Dokumente fuer direkte Agent-Sessions
- `knowledge/` -- Gesammeltes Wissen ueber APIs, Plattform-Quirks, Architektur
- `sources/` -- Wissensquellen-Pipeline (inbox → verarbeiten → archive)

## Agenten

| Agent | Aufgabe | Modell | Modus |
|-------|---------|--------|-------|
| rust-core | Rust-Backend: Audio, STT, LLM-Client, Paste, Hotkey, Dictionary | sonnet | delegiert + direkt |
| ui-dev | React-Frontend: Overlay, Settings, Dictionary-UI, Styles | sonnet | delegiert |
| android-platform | Android: Floating Bubble, AccessibilityService, Kotlin-native Audio/API, Permissions | sonnet | delegiert + direkt |
| product-strategist | Positionierung, Monetarisierung, Roadmap-Priorisierung (Business-Sicht), Wettbewerbs-Strategie | sonnet | delegiert + direkt |

## Skills

| Skill | Aufgabe | Kontext |
|-------|---------|---------|
| /scaffold | Neues Modul/Component aus Template erstellen | fork (haiku) |
| /build | Fuer Zielplattform bauen, Fehler melden | fork (haiku) |
| /run-tests | Tests ausfuehren, Ergebnisse formatieren | fork (haiku) |
| /research-api | API-Docs recherchieren, Summary in knowledge/ schreiben | fork (sonnet) |
| /lint-fix | Linter + Formatter, Auto-Fix | fork (haiku) |
| /plan-feature | Feature in Tasks mit Dateien + Agent-Zuweisung zerlegen | fork (sonnet) |
| /commit-progress | Git-Commit mit konventioneller Message | fork (haiku) |
| /debug-error | Fehler-Output analysieren, Ursache finden, Fix vorschlagen | fork (sonnet) |
| /sync-prompts | LLM-Prompts in Rust und Kotlin auf Drift vergleichen | fork (haiku) |
| /release | Version bump + Build + publish.sh sync + GitHub Release auf dikta-public | fork (haiku) |
| /track | Aktualisiert project-status.md mit Session-Fortschritt | inline |
| /reflect | Team-Selbstanalyse erstellen | fork (sonnet) |
| /learn | Neue Wissensquellen in knowledge/ integrieren | fork (sonnet) |

## Regeln

1. **Code-Sprache ist Englisch.** Variablen, Funktionen, Kommentare, Commits -- alles Englisch.
2. **Jede Architektur-Entscheidung wird in `knowledge/architecture.md` dokumentiert.** Nicht nur "was", sondern "warum".
3. **API-Keys kommen NICHT in den Code.** Immer ueber `.env` oder System-Keystore.
4. **Jedes neue Modul bekommt Tests.** Kein Modul ohne mindestens einen Basis-Test.
5. **Plattform-spezifischer Code wird hinter Abstraktionen versteckt.** Nie `#[cfg(target_os)]` quer durch die Business-Logik.
6. **Kleine Commits, oft.** Lieber 10 kleine als 1 riesiger Commit.
7. **Bei Unsicherheit: `/research-api` nutzen** statt zu raten. Lieber 2 Minuten recherchieren als 20 Minuten debuggen.
