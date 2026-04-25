# Klarvo — Source-Tree-Analyse

Generiert: 2026-04-13 | Projektversion: 0.5.0

---

## Verzeichnisbaum

```
klarvo/
├── src/                          # React/TypeScript Frontend
│   ├── main.tsx                  # Entry Point — entscheidet App vs. FloatingBar
│   ├── App.tsx                   # Haupt-Orchestrator (Hooks, Panels, Recording)
│   ├── FloatingBar.tsx           # Desktop: Transparente Pill-Statusanzeige
│   ├── Onboarding.tsx            # Multi-Step Einrichtungs-Wizard
│   ├── tauri-commands.ts         # IPC-Layer: 60+ Tauri-Command-Wrapper
│   ├── types.ts                  # Zentrale TypeScript-Typen
│   ├── platform.ts               # Desktop/Mobile-Erkennung (User-Agent)
│   ├── media-recorder.ts         # Browser-Audio-Aufnahme (Mobile-Fallback)
│   ├── styles.css                # Globale CSS + Tailwind-Imports
│   ├── App.css                   # App-spezifische Styles
│   ├── components/
│   │   ├── SettingsPanel.tsx      # Settings-Navigation (Drill-Down)
│   │   ├── AdvancedSettingsPanel.tsx # Power-User-Einstellungen
│   │   ├── CostDashboard.tsx      # Nutzungsstatistiken (Kosten, Woerter)
│   │   ├── WhisperModelManager.tsx # Offline-STT-Modell-Download (Desktop)
│   │   ├── LlmModelManager.tsx    # Offline-LLM-Modell-Download (Desktop)
│   │   ├── FeedbackModal.tsx      # In-App Feedback-Formular
│   │   ├── VoiceNotesPanel.tsx    # Sprachnotizen-Liste
│   │   ├── SnippetsPanel.tsx      # Text-Snippets fuer Quick-Paste
│   │   ├── ThemeSwitcher.tsx      # Dark/Light Theme Toggle
│   │   ├── QuickTip.tsx           # Toast-Benachrichtigung (Onboarding-Tipps)
│   │   ├── PreviewComments.tsx    # Info-Box fuer Browser-Preview-Modus
│   │   ├── MobileTextarea.tsx     # Touch-optimierte Textarea
│   │   ├── icons.tsx              # SVG-Icon-Komponenten
│   │   ├── ui.tsx                 # Wiederverwendbare UI-Primitives
│   │   └── settings/
│   │       ├── SettingsHome.tsx           # Kategorie-Liste
│   │       ├── RecordingAudioContent.tsx  # Mikrofon, STT-Modell, Provider
│   │       ├── AiProvidersContent.tsx     # API-Key-Verwaltung (5 Provider)
│   │       ├── AppearanceLanguageContent.tsx # Sprache, UI-Scale
│   │       ├── ShortcutsContent.tsx       # Hotkey-Konfiguration (2 Slots)
│   │       ├── DictionaryContent.tsx      # Custom Dictionary
│   │       ├── LicenseSettings.tsx        # Lizenz-Aktivierung
│   │       ├── AboutContent.tsx           # Versioninfo, Links
│   │       ├── SettingsRow.tsx            # Wiederverwendbare Zeile
│   │       ├── SettingsSubPageHeader.tsx  # Zurueck-Navigation
│   │       └── types.ts                  # Settings-spezifische Typen
│   └── hooks/
│       ├── useRecording.ts        # Recording-State-Machine + Pipeline
│       ├── useSettings.ts         # Einstellungen laden/speichern
│       ├── usePanels.ts           # Panel-Sichtbarkeit (Settings/History/Stats)
│       ├── useLicense.ts          # Lizenzstatus + Validierung
│       ├── useUiScale.ts          # UI-Skalierung (S/M/L)
│       └── useQuickTip.ts         # Onboarding-Tipps mit Trigger-Logik
│
├── src-tauri/                    # Rust/Tauri Backend
│   ├── src/
│   │   ├── main.rs               # Tauri Entry Point (Desktop)
│   │   ├── lib.rs                # AppState, Setup, Command-Registrierung
│   │   ├── pipeline.rs           # End-to-End Diktat-Pipeline-Orchestrierung
│   │   ├── test_helpers.rs       # Test-Utilities
│   │   ├── audio/                # Mikrofon-Capture (cpal Desktop, Stub Android)
│   │   ├── stt/                  # Speech-to-Text (Groq, OpenAI, Local Whisper)
│   │   ├── llm/                  # LLM Text-Cleanup (DeepSeek, Groq, OpenAI, Anthropic, Local)
│   │   ├── vad/                  # Voice Activity Detection (Silero + Highpass-Filter)
│   │   ├── hotkey/               # Pipeline-Events und State-Machine-Typen
│   │   ├── paste/                # Clipboard + Tastensimulation (Linux/Windows)
│   │   ├── history/              # SQLite-Datenbank (History, Usage, Tips)
│   │   ├── sync/                 # Cross-Device Sync (Turso HTTP API)
│   │   ├── config/               # JSON-Konfiguration (40+ Felder)
│   │   ├── dictionary/           # Benutzer-Woerterbuch
│   │   ├── license/              # Lizenzvalidierung (HMAC + Lemon Squeezy)
│   │   ├── voice_command/        # Sprachbefehl-Modus (Desktop, experimentell)
│   │   ├── commands/             # Tauri IPC Commands (60+)
│   │   │   ├── recording.rs      #   Aufnahme, STT, Cleanup
│   │   │   ├── settings.rs       #   Einstellungen, API-Keys, Hotkeys
│   │   │   ├── dictionary.rs     #   Dictionary CRUD
│   │   │   ├── history.rs        #   History, Stats, Notizen
│   │   │   ├── license.rs        #   Lizenz-Validierung
│   │   │   ├── misc.rs           #   Profiles, Snippets, Sync, Bar
│   │   │   ├── whisper.rs        #   Whisper-Modell-Management
│   │   │   ├── llm_model.rs      #   LLM-Modell-Management
│   │   │   ├── voice_command.rs  #   Voice Command Toggle
│   │   │   └── feedback.rs       #   Feedback-Webhook
│   │   └── snapshots/            # Insta-Test-Snapshots
│   ├── Cargo.toml                # Rust Dependencies + Platform-Targets
│   ├── Cargo.lock                # Dependency Lockfile
│   ├── build.rs                  # Tauri Build Script
│   ├── tauri.conf.json           # Tauri-Konfiguration (Window, Bundle, Updater)
│   ├── capabilities/             # Tauri v2 Capability Permissions
│   │   ├── default.json          #   Basis-Rechte
│   │   └── desktop.json          #   Desktop-spezifische Rechte
│   ├── gen/                      # Generierter Code
│   │   ├── android/              #   Android-Projekt (Gradle, Manifest)
│   │   └── schemas/              #   Tauri-Schemas
│   ├── resources/                # Bundled Resources
│   │   ├── README.txt            #   Mitgeliefertes README
│   │   └── RELEASE-NOTES.txt    #   Release Notes
│   ├── icons/                    # App-Icons (alle Groessen)
│   └── tests/
│       ├── pi_security.rs        # Prompt-Injection-Sicherheitstests
│       └── pi_security/          # PI-Test-Framework (Arcanum Taxonomy)
│
├── android/                      # Android/Kotlin Native Layer
│   ├── kotlin-src/com/klarvo/voice/
│   │   ├── MainActivity.kt               # Entry Point + Permission-Sequenzierung
│   │   ├── KlarvoOverlayService.kt        # Foreground Service (Bubble + Pipeline)
│   │   ├── KlarvoAccessibilityService.kt  # Systemweite Tastatur-Erkennung + Paste
│   │   ├── FloatingBubbleView.kt          # Custom View (Canvas-Rendering, 4 States)
│   │   ├── KlarvoAudioRecorder.kt         # 16kHz PCM + Silero VAD
│   │   ├── KlarvoApi.kt                   # HTTP-Client (STT, LLM, History, Sync)
│   │   ├── LocalWhisperInference.kt       # JNI-Bridge → Rust whisper-rs
│   │   ├── LocalLlmInference.kt           # JNI-Bridge → C++ MNN
│   │   ├── KlarvoLogger.kt               # Dual-Sink Logger (Logcat + File)
│   │   ├── BankingAppBlocklist.kt         # Banking-App-Erkennung (Mandatory Hide)
│   │   └── BubbleSettingsMenu.kt          # Platzhalter (Settings via React)
│   ├── jni/
│   │   ├── CMakeLists.txt                 # MNN JNI Build-Config
│   │   └── klarvo_llm_jni.cpp             # C++ JNI Wrapper fuer MNN LLM
│   ├── res-values/strings.xml             # Android-Strings
│   └── res-xml/accessibility_service_config.xml  # A11y-Service-Konfiguration
│
├── scripts/                      # Build- und Deploy-Skripte
│   ├── android-build.sh          #   Kotlin-Copy, Build, Sign, Deploy
│   ├── avx2-portable.cmake       #   CMake-Config fuer AVX2-Kompatibilitaet
│   └── setup-ssh-server.ps1      #   SSH-Server Setup (Dev)
│
├── marketing/                    # Marketing-Assets
│   ├── notion-cover-klarvo*.html/png  # Notion-Cover-Generatoren
│   ├── social-preview-klarvo*.html/png # GitHub/Social-Preview
│   └── dikta-pitch.pdf           # Pitch-Deck
│
├── public/                       # Statische Web-Assets
│   ├── favicon.png
│   ├── tauri.svg
│   └── vite.svg
│
├── docs/                         # Generierte Projektdokumentation
├── package.json                  # npm Dependencies + Scripts
├── package-lock.json             # npm Lockfile
├── tsconfig.json                 # TypeScript-Konfiguration
├── tsconfig.node.json            # TS-Config fuer Node/Vite
├── vite.config.ts                # Vite Build-Config + Tailwind Plugin
├── index.html                    # SPA Entry HTML
├── README.md                     # Produkt-Beschreibung
├── pre-launch-checklist.md       # v1.0 Launch-Planung
├── security-report.txt           # Security-Audit Report
├── LICENSE                       # BSL 1.1
├── latest.json                   # Auto-Updater Manifest
└── social-preview.png            # GitHub Social Preview
```

## Kritische Verzeichnisse

| Verzeichnis | Zweck | Plattform |
|-------------|-------|-----------|
| `src/` | React/TypeScript UI — alle Benutzer-Interaktionen | Alle |
| `src/hooks/` | State-Management — 6 Domain-spezifische Hooks | Alle |
| `src/tauri-commands.ts` | IPC-Schnittstelle zum Rust-Backend (60+ Commands) | Alle |
| `src-tauri/src/` | Rust-Backend — Audio, STT, LLM, Pipeline | Desktop + Android |
| `src-tauri/src/commands/` | Tauri IPC Command Handler | Desktop + Android |
| `src-tauri/src/pipeline.rs` | Diktat-Pipeline-Orchestrierung | Desktop |
| `android/kotlin-src/` | Android-nativer Layer — Bubble, Audio, API | Android |
| `android/jni/` | C++ JNI-Bridge fuer MNN LLM-Inference | Android |
| `src-tauri/tests/` | Sicherheitstests (Prompt Injection) | Test |
| `scripts/` | Build- und Deploy-Automatisierung | Alle |

## Entry Points

| Entry Point | Datei | Beschreibung |
|-------------|-------|--------------|
| Web/Desktop UI | `src/main.tsx` | React Root — entscheidet App vs. FloatingBar |
| Tauri Desktop | `src-tauri/src/main.rs` | Rust Entry Point |
| Tauri Library | `src-tauri/src/lib.rs` | AppState Setup + Command-Registrierung |
| Android | `android/.../MainActivity.kt` | Permission-Chain + Service-Start |
| Overlay | `android/.../KlarvoOverlayService.kt` | Foreground Service + Pipeline |
| Build | `vite.config.ts` | Vite + React + Tailwind Build |
