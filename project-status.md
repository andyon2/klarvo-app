# Projektstatus

## Aktueller Stand
Phase 1 (Foundation) abgeschlossen. Tauri v2 Projekt steht, alle drei Backend-Module (Audio, STT, LLM) implementiert und getestet (23 Tests), Overlay-UI mit Recording-Controls und Style-Picker fertig. Alles kompiliert sauber (Frontend + Backend). Entwicklung laeuft in WSL2 mit Linux-Builds.

## Abgeschlossene Aufgaben (Phase 1)
- [x] Tauri v2 Projekt initialisiert (React 19 + TypeScript + Vite 7)
- [x] Rust-Abhaengigkeiten definiert (cpal, hound, reqwest, tokio, serde, thiserror, anyhow, async-trait)
- [x] Frontend-Abhaengigkeiten definiert (Tailwind CSS v4, React 19)
- [x] API-Provider-Docs vollstaendig recherchiert (Groq Whisper + DeepSeek Chat)
- [x] Audio-Capture Modul (cpal, WAV 16kHz mono 16-bit PCM, dedizierter Audio-Thread)
- [x] Groq Whisper STT Client (SttProvider Trait + GroqWhisper Impl)
- [x] DeepSeek Cleanup Client (CleanupProvider Trait + DeepSeekCleanup Impl, 3 Styles)
- [x] Minimale Overlay-UI (State-Machine, Record-Button, Style-Picker, Status-Bar)
- [x] Tauri-Commands registriert (start/stop recording, transcribe, cleanup, is_recording)

## Offene Aufgaben (Phase 2 -- Integration)
- [ ] End-to-End-Flow: Record -> Transcribe -> Cleanup -> Anzeige im UI
- [ ] API-Key-Management (Settings-UI oder .env)
- [ ] Fehlerbehandlung im UI (API-Fehler, kein Mikrofon, etc.)
- [ ] Globaler Hotkey (Push-to-Talk)
- [ ] Text-Paste in aktives Fenster (Clipboard + Simulate Ctrl+V)
- [ ] Settings-Persistenz (SQLite oder JSON)
- [ ] Custom Dictionary (SQLite)
- [ ] Lokaler whisper.cpp Fallback

## Entscheidungen
- [2026-03-06]: Tech-Stack: Tauri v2 + React/TS + Rust. Begruendung: Ein Codebase fuer Windows + Android, Rust fuer Performance, Web-Frontend fuer Flexibilitaet.
- [2026-03-06]: API-Strategie: Groq Whisper (STT) + DeepSeek (Cleanup) primaer, whisper.cpp lokal als Fallback.
- [2026-03-06]: GPU nur am Strom, Cloud-API auf Akku.
- [2026-03-06]: Android IME-Architektur (Tauri-Bridge vs. Native Kotlin) wird in Phase 5 entschieden.
- [2026-03-06]: Entwicklung in WSL2 (Linux-Builds), Windows-Build spaeter ueber PowerShell. Grund: Claude Code laeuft nur in WSL.
- [2026-03-06]: Audio-Thread-Architektur: cpal::Stream ist nicht Send auf Linux -- dedizierter OS-Thread mit Channel-Kommunikation.
- [2026-03-06]: whisper-large-v3-turbo statt v3 (3x guenstiger, minimal schlechtere Genauigkeit, reicht fuer Diktat).

## Naechste Session
Phase 2 starten: End-to-End-Flow verbinden (Record -> STT -> Cleanup -> UI), API-Key-Eingabe, Fehlerbehandlung. Danach Hotkey + Paste.
