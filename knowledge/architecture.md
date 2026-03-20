# Architektur -- Dikta

## Tech-Stack

| Schicht | Technologie | Warum |
|---------|-------------|-------|
| Framework | Tauri v2 | Ein Codebase Win+Android, Rust-Backend, kleine Binaries |
| Frontend | React 19 + TypeScript + Tailwind v4 | Groesstes Ecosystem, Typsicherheit, schnelles Styling |
| Backend | Rust | whisper.cpp-Integration, niedrige Latenz, native OS-APIs |
| Mobile Native | Kotlin | Overlay-Service, AudioRecord, AccessibilityService -- braucht Android-APIs |
| STT | Groq Whisper API (primaer), OpenAI Whisper (Fallback) | Schnell, guenstig |
| LLM Cleanup | DeepSeek (primaer), OpenAI/Anthropic/Groq (Fallback) | DeepSeek ist guenstigster |
| Persistenz | JSON (Config, Dictionary), SQLite (History, Stats) | JSON fuer flache Daten, SQLite fuer relationale |
| Sync | Turso HTTP API | Lokale SQLite bleibt, Turso fuer Push/Pull, UUID als PK remote |

## Plattform-Architektur

### Was wo laeuft

```
                    ┌─────────────────────────┐
                    │   Shared Frontend (src/) │
                    │   React + TypeScript     │
                    └──────────┬──────────────┘
                               │ invoke()
              ┌────────────────┴────────────────┐
              │                                 │
    ┌─────────▼──────────┐           ┌──────────▼─────────┐
    │  Desktop (Windows)  │           │  Mobile (Android)   │
    │  Rust Backend       │           │  Rust Backend       │
    │  + cpal Audio       │           │  (STT/LLM/History)  │
    │  + Win32 Paste      │           │                     │
    │  + Global Hotkey    │           │  Kotlin Services:   │
    │  + System Tray      │           │  + Overlay Bubble   │
    │  + Updater          │           │  + AudioRecord      │
    └─────────────────────┘           │  + Accessibility    │
                                      │  + DiktaApi (HTTP)  │
                                      └─────────────────────┘
```

### Desktop-only (hinter `#[cfg(desktop)]` oder `isDesktop`)
- **Rust:** cpal Audio-Capture, arboard Clipboard, Win32 SendInput Paste, global-shortcut, tray-icon, updater, Floating Bar Window
- **Frontend:** Hotkey-Recorder, Audio-Device-Picker, Whisper Mode, Command Mode, Snippets-Panel, Webhook-Config, Updates-Sektion, UI Size, Footer-Hotkey-Anzeige

### Mobile-only (hinter `#[cfg(mobile)]` oder `isMobile`)
- **Rust:** Stub-Implementierungen fuer Audio (no-op), Paste (no-op)
- **Frontend:** Safe-Area-Padding, Touch-Target-Groessen, MediaRecorder (WebAudio API), Android-Back-Button
- **Kotlin:** Gesamter Overlay-Service, Bubble-UI, AudioRecord, AccessibilityService, Permission-Flow, DiktaApi (HTTP-Calls direkt)

### Shared (beide Plattformen)
- **Rust:** STT-Provider, LLM-Provider, Config, Dictionary, History, Sync, Pipeline-Logik
- **Frontend:** Settings-Panel, Advanced-Settings, VoiceNotes, History-Ansicht, Recording-State-Hooks

### Cross-Platform-Regeln
1. **Shared-First:** Jede Aenderung an `src/` betrifft BEIDE Plattformen
2. **Build-Reihenfolge:** Windows zuerst testen (`tauri dev`), dann Android (`scripts/android-build.sh`)
3. **Platform Guards:** `isDesktop`/`isMobile` im Frontend, `#[cfg(desktop)]`/`#[cfg(mobile)]` in Rust
4. **Android WebView:** `env(safe-area-inset-bottom)` gibt 0 zurueck -- nie darauf verlassen, feste px-Werte nutzen

## Design-Prinzipien

### Deep Modules + Vertical Slices (2026-03-19)
- Code nach Features organisieren statt nach technischen Schichten — AI arbeitet praeziser wenn zusammengehoeriger Code beieinander liegt
- Deep Modules (Ousterhout): Einfache Interfaces, komplexe Implementierung dahinter — reduziert die Anzahl Dateien die gleichzeitig im Kontext sein muessen
- AI-Genauigkeit: ~60% bei tight coupling → ~95% bei sauberen Modulen (Thoughtworks Radar)
- Fuer Rust/Tauri: Trait-basierte Module mit klaren Boundaries. Jedes Feature als eigenes Modul mit definiertem Public Interface
- Tracer Bullets: Bei neuen Features zuerst minimale End-to-End-Implementierung (UI → Backend → Persistence), dann ausbauen

## Modul-Grenzen

### Pipeline-Flow (Desktop)
```
Hotkey → Audio-Capture (cpal) → WAV → STT (Groq/OpenAI) → Raw Text → LLM Cleanup → Paste (SendInput)
```

### Pipeline-Flow (Android)
```
Bubble-Tap → AudioRecord (Kotlin) → WAV → STT (Kotlin HTTP) → Raw Text → LLM Cleanup (Kotlin HTTP) → AccessibilityService Paste
```

### Schluessel-Entscheidungen

**Android: Floating Bubble statt IME (2026-03-08)**
- Overlay-Service mit Floating Bubble als primaerer Ansatz
- Kein System-Keyboard noetig -- Bubble erscheint ueber jeder App
- Gesten: Single-Tap=Record, Long-Press=Push-to-Talk, Double-Tap=Settings

**Android: Native Kotlin statt Tauri-Bridge (2026-03-08)**
- DiktaApi.kt macht HTTP-Calls direkt (Groq, DeepSeek, Turso)
- Kein Tauri-Bridge-Overhead, weniger Latenz
- Trade-off: Prompt-Logik ist in Rust UND Kotlin dupliziert -- bei Aenderungen BEIDE updaten!

**Keyboard-Detection: AccessibilityService (2026-03-08)**
- TYPE_INPUT_METHOD Window-Events erkennen ob Tastatur sichtbar ist
- Funktioniert system-weit (nicht nur in-app)
- Xiaomi: "restricted settings" umgehbar via ADB Security Settings

**Config: JSON fuer Settings, SQLite fuer History**
- Config/Dictionary: JSON in `{app_data_dir}/` -- human-editable, kein Setup
- History/Stats: SQLite -- relationale Queries, Volltextsuche
- Android liest config.json aus `context.dataDir` (identischer Pfad wie Tauri)

**API-Key-Sicherheit**
- Keys verlassen Backend nie im Klartext (nur `****{last4}` maskiert)
- Env-Var-Fallback: `GROQ_API_KEY`, `DEEPSEEK_API_KEY` aus `.env`

**Audio: 16kHz mono WAV**
- Funktioniert fuer Groq API (max 25MB) und whisper.cpp
- Desktop: cpal mit dediziertem OS-Thread (Stream nicht Send)
- Android: AudioRecord API, PCM sammeln, bei Stop zu WAV konvertieren

**Sync: Turso HTTP API**
- Lokale SQLite bleibt, Turso fuer Push/Pull
- UUID als Primary Key remote (kein AUTOINCREMENT)
- DB-Lock nie ueber async await halten (rusqlite Connection nicht Send)
- Android: pusht nach jedem Diktat via DiktaApi.pushToTurso()

**Event-basierte Pipeline**
- `dikta://state-changed` Events statt Polling
- States: idle → recording → transcribing → cleaning → idle

**Paid/Free Feature-Gates (2026-03-10)**
- Free: Alle Cleanup-Stile (Polished/Verbatim/Chat), alle Provider, Cleanup Instructions, Offline small+medium Modelle, Dictionary (max 20 Eintraege), Basis-Statistiken
- Paid: Whisper large-v3 Modell, unbegrenztes Dictionary, Snippets, Command Mode, Cross-Device Sync, Webhooks, Integrations, erweiterte Stats, Voice Notes, Whisper Mode
- Offline-Gate ist ein Modell-Gate, kein Feature-Gate: small+medium sind free, large-v3 ist paid. Kein harter "Offline gesperrt"-Moment.
- tiny und base entfernt — Qualitaet zu niedrig fuer ein Produkt.

**LLM Cleanup: Drei Stile**
- Polished: Fuellwoerter bereinigen, Grammatik, professionell formatieren
- Verbatim: Nur Satzzeichen und offensichtliche Fehler
- Chat: Kurz, locker, Emojis erlaubt
- Prompts muessen in Rust (llm/mod.rs) UND Kotlin (DiktaApi.kt) synchron gehalten werden!

**Custom Prompts: Zwei Stufen, kein Overlap**
- "Cleanup Instructions" (Settings): Zusaetzliche LLM-Anweisungen ("formelles Deutsch", "keine Aufzaehlungen")
- "STT Prompts" (Advanced Settings): Whisper Conditioning Text pro Sprache (verbessert Erkennung)
- Unterschiedliche Pipeline-Stufen: STT-Prompt → Transkription, Cleanup Instructions → LLM-Bereinigung

## Repository-Architektur

**Zwei-Repo-Setup (2026-03-10)**
- `andyon2/dikta` (privat): Arbeitsrepo. Gesamter Code + Agent-Infrastruktur (CLAUDE.md, main-agent.md, .claude/, knowledge/, briefings/, feedback/, sources/, scripts/).
- `andyon2/dikta-public` (oeffentlich): Produktcode-Mirror fuer Nutzer. Kein Agent-Zeug.
- `scripts/publish.sh`: rsync-basierter Sync mit Exclude-Liste + Marker-Check (verhindert Agent-Daten-Leak).
- **Releases:** Immer auf dikta-public (`gh release create --repo andyon2/dikta-public`).
- **Updater-Endpoint:** `https://github.com/andyon2/dikta-public/releases/latest/download/latest.json`
- **Taegliche Arbeit:** Nur in dikta (privat). Commit + Push geht nur hierhin. dikta-public wird nur bei Releases via publish.sh aktualisiert.

## Build-Anforderungen whisper-rs

- **CMake:** Muss auf Windows im PATH sein (whisper.cpp wird beim Build kompiliert)
- **LLVM/Clang:** Bindgen braucht libclang.dll. **LLVM 18.1.8 nutzen, NICHT 22+!** Clang 22 generiert kaputte Struct-Bindings (Codeberg whisper-rs #268).
- **Install:** `https://github.com/llvm/llvm-project/releases/tag/llvmorg-18.1.8` → `LLVM-18.1.8-win64.exe`, "Add to PATH" anhaeken.
- **Build-Script:** `sync-and-build.ps1` setzt `BINDGEN_EXTRA_CLANG_ARGS=--target=x86_64-pc-windows-msvc`

## Plattform-Quirks

### Windows
- **Paste:** Win32 SendInput fuer Ctrl+V. Terminals brauchen Ctrl+Shift+V (Terminal-Erkennung noetig)
- **Hotkey:** `global-hotkey` Crate (Tauri-integriert), braucht Message-Loop
- **GPU:** Fuer lokales whisper.cpp: GPU am Strom, CPU auf Akku (SYSTEM_POWER_STATUS API)

### Android
- **Permissions:** RECORD_AUDIO + POST_NOTIFICATIONS (Runtime), FOREGROUND_SERVICE_MICROPHONE (Android 14+)
- **Background-Killing:** ForegroundService mit Notification Pflicht. Xiaomi/Samsung besonders aggressiv
- **Overlay:** SYSTEM_ALERT_WINDOW, TYPE_APPLICATION_OVERLAY (API 26+), in onResume() pruefen
- **Touch:** rawX/rawY statt x/y fuer Drag. 10dp Tap-vs-Drag Schwelle
- **Kotlin-Dateien:** Persistent in `android/kotlin-src/`, werden via `scripts/android-build.sh` nach `gen/android/` kopiert
- **WebView:** `env(safe-area-inset-bottom)` gibt 0 zurueck. Feste 56px Padding + max-h Abzuege nutzen
- **Accessibility:** FLAG_RETRIEVE_INTERACTIVE_WINDOWS + packageNames=null fuer system-weite Events

### Android Build
- JDK 17, NDK via SDK Manager, Build-Tools 34.0.0, Rust targets: aarch64-linux-android
- WSL2: `ADB_SERVER_SOCKET=tcp:$WSL_HOST:5037` fuer ADB-Zugriff
- Build: `scripts/android-build.sh` (kopiert Kotlin, baut, signiert, deployt nach Dropbox)
- Tauri Plugins desktop-only: opener, global-shortcut, updater, tray-icon (mit cfg-Guards!)
