# Architektur-Entscheidungen -- Dikta

## Grundlegende Architektur

### Framework: Tauri v2
- **Entscheidung:** Tauri v2 als Desktop- und Mobile-Framework
- **Warum:** Ein Codebase fuer Windows + Android. Rust-Backend fuer Performance-kritische Teile (Audio, STT). Web-Frontend (React) fuer UI. Kleine Binaries (kein Electron-Bloat). Tauri v2 hat stabilen Mobile-Support.
- **Trade-off:** Weniger Ecosystem als Electron, aber deutlich schlanker. Tauri-Android ist juenger als Desktop -- moegliche Quirks zu erwarten.

### Frontend: React + TypeScript + Tailwind
- **Entscheidung:** React mit TypeScript fuer das Web-Frontend im Tauri-Fenster
- **Warum:** Groesstes Ecosystem, beste LLM-Code-Qualitaet, TypeScript fuer Typsicherheit. Tailwind fuer schnelles Styling ohne CSS-Dateien.
- **Trade-off:** React ist schwerer als Svelte/Solid, aber bei dieser kleinen App irrelevant.

### Backend: Rust
- **Entscheidung:** Gesamte Business-Logik in Rust (Tauri-Backend)
- **Warum:** Direkte Integration mit whisper.cpp (via whisper-rs), niedrige Latenz fuer Audio-Processing, native OS-API-Zugriffe (Hotkeys, Paste, Clipboard).

## Modul-Architektur (Rust)

```
src-tauri/src/
  audio/      -- Audio-Capture (cpal), Buffer-Management
  stt/        -- Speech-to-Text (Trait: GroqWhisper + LocalWhisper)
  llm/        -- Text-Cleanup (Trait: DeepSeek + erweiterbar)
  paste/      -- Text-Insertion (plattformspezifisch hinter Trait)
  hotkey/     -- Globaler Hotkey (plattformspezifisch hinter Trait)
  dictionary/ -- Custom-Woerterbuch (JSON-Datei, kein SQLite)
  config/     -- Settings-Persistenz (JSON-Datei)
```

Jedes Modul exponiert seine Funktionalitaet via Tauri-Commands an das Frontend.

### Config-Persistenz (`config/`)
- **Format:** JSON (`{app_data_dir}/config.json`), nicht SQLite
- **Warum JSON statt SQLite (MVP):** Eine flache Settings-Struktur braucht kein relationales Schema. JSON ist human-editable und hat null Setup-Overhead.
- **Env-Var-Fallback:** Falls API-Keys in config.json fehlen, werden `GROQ_API_KEY` / `DEEPSEEK_API_KEY` aus der Prozessumgebung gelesen. Ermoeglicht `.env`-basierte Entwicklung ohne GUI.
- **API-Keys auf Disk:** Plaintext im user-owned app-data-dir. Zukunft: Windows Credential Manager.

### Dictionary-Persistenz (`dictionary/`)
- **Format:** JSON (`{app_data_dir}/dictionary.json`)
- **Warum JSON statt SQLite (MVP):** Eine einfache String-Liste braucht keine Datenbank.
- **Duplikat-Pruefung:** Case-insensitiv beim Hinzufuegen, case-sensitiv beim Entfernen.
- **Pipeline-Integration:**
  1. STT: `terms_as_prompt()` -> Groq `prompt`-Parameter (verbessert Whisper-Erkennung von Fachwoertern, max 224 Token)
  2. LLM: `terms_as_list()` -> DeepSeek System-Prompt (LLM bewahrt die exakte Schreibweise)

### AppState-Struktur
- `config: Mutex<AppConfig>` -- alle persistierten Settings inkl. API-Keys
- `dictionary: Mutex<Dictionary>` -- User-Wortliste
- `app_data_dir: PathBuf` -- Pfad fuer Datei-I/O
- `stt_provider: RwLock<Arc<dyn SttProvider>>` -- hot-swappbar bei Key-Aenderung
- `cleanup_provider: RwLock<Arc<dyn CleanupProvider>>` -- hot-swappbar

### API-Key-Sicherheit im Frontend
- `get_settings()` gibt nur maskierte Keys zurueck: `"****{last4}"` (z.B. `"****1234"`)
- Volle Keys verlassen das Backend nie Richtung Frontend
- `get_api_key_status()` gibt nur `bool` zurueck (fuer einfache "konfiguriert"-Anzeige)

## API-Strategie

### STT (Speech-to-Text)
- **Primaer:** Groq Whisper API (schnell, guenstig, gute Qualitaet)
- **Fallback:** Lokales whisper.cpp via whisper-rs (offline, GPU optional)
- **Auto-Modus:** Cloud wenn online + API-Key vorhanden, sonst lokal

### Text-Cleanup (LLM)
- **Primaer:** DeepSeek API (guenstig, gute Qualitaet fuer Text-Cleanup)
- **System-Prompt pro Stil:**
  - Polished: Bereinige Fuellwoerter, korrigiere Grammatik, formatiere professionell
  - Verbatim: Nur Satzzeichen und offensichtliche Fehler
  - Chat: Kurz, locker, Emojis erlaubt

### GPU-Strategie
- Lokal whisper.cpp: GPU wenn am Strom, CPU wenn auf Akku
- Erkennung via Windows Power-Status API oder manueller Toggle in Settings

## Plattform-Abstraktionen

Plattformspezifischer Code wird hinter Traits versteckt:

```rust
pub trait PasteHandler: Send + Sync {
    fn paste(&self, text: &str) -> Result<()>;
}

pub trait HotkeyManager: Send + Sync {
    fn register(&self, key: KeyCombo, callback: Box<dyn Fn() + Send>) -> Result<()>;
    fn unregister(&self, key: KeyCombo) -> Result<()>;
}
```

Implementierungen:
- Windows: `WindowsPasteHandler`, `WindowsHotkeyManager`
- Android: `AndroidPasteHandler` (via InputConnection), `AndroidHotkeyManager` (via IME)

## Android-Architektur

### IME (InputMethodService)
- Dikta registriert sich als System-Keyboard
- Minimale UI: Grosser Speak-Button + Style-Auswahl
- Audio wird aufgenommen -> an Rust-Backend geschickt (Tauri Plugin Bridge) -> STT -> Cleanup -> Text eingefuegt via InputConnection

### Entscheidung offen: Tauri-Bridge vs. Native Kotlin
- Option A: IME nutzt Tauri Plugin Bridge -> Rust-Backend (Code-Sharing, aber Latenz?)
- Option B: IME nutzt APIs direkt aus Kotlin (schneller, aber Code-Duplikation)
- Entscheidung wird in Phase 5 (Android) getroffen nach Prototyp.
