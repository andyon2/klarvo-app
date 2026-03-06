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
  dictionary/ -- Custom-Woerterbuch (SQLite)
  config/     -- Settings-Persistenz (SQLite oder JSON)
```

Jedes Modul exponiert seine Funktionalitaet via Tauri-Commands an das Frontend.

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
