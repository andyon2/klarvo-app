# Projektstatus

## Aktueller Stand
Phase 2 (Integration) abgeschlossen. Dikta ist ein funktionsfaehiges MVP: Hotkey druecken -> Sprache aufnehmen -> transkribieren -> bereinigen -> ins aktive Fenster einfuegen. 78 Rust-Tests, Frontend und Backend kompilieren sauber. Settings und Dictionary werden persistent gespeichert.

## Abgeschlossene Aufgaben

### Phase 1 -- Foundation
- [x] Tauri v2 Projekt initialisiert (React 19 + TypeScript + Vite 7 + Tailwind CSS v4)
- [x] Audio-Capture Modul (cpal, WAV 16kHz mono 16-bit PCM, dedizierter Audio-Thread)
- [x] Groq Whisper STT Client (SttProvider Trait + GroqWhisper Impl, prompt-Parameter)
- [x] DeepSeek Cleanup Client (CleanupProvider Trait + DeepSeekCleanup Impl, 3 Styles)
- [x] Overlay-UI (State-Machine, Record-Button, Style-Picker, Status-Bar)
- [x] API-Provider-Docs recherchiert (Groq Whisper + DeepSeek Chat)

### Phase 2 -- Integration
- [x] End-to-End-Flow: Record -> Transcribe -> Cleanup -> Anzeige (3-Schritt-Pipeline mit Status-Updates)
- [x] API-Key-Management (Settings-Panel mit maskierten Keys)
- [x] Fehlerbehandlung im UI (API-Fehler, kein Mikrofon, etc.)
- [x] Globaler Hotkey (Ctrl+Shift+D, tauri-plugin-global-shortcut)
- [x] Text-Paste (arboard Clipboard + xdotool Ctrl+V Simulation)
- [x] Event-basierte Pipeline (dikta://state-changed fuer Frontend-Sync)
- [x] Settings-Persistenz (JSON config.json im App-Data-Dir)
- [x] Custom Dictionary (JSON dictionary.json, Terms in Groq-Prompt + DeepSeek-System-Prompt)

## Module (Rust)
```
src-tauri/src/
  audio/      -- Audio-Capture (cpal), WAV-Encoding, dedizierter Thread
  stt/        -- SttProvider Trait, GroqWhisper (multipart upload)
  llm/        -- CleanupProvider Trait, DeepSeekCleanup (3 Styles)
  paste/      -- PasteHandler Trait, Linux-Impl (arboard + xdotool)
  hotkey/     -- Pipeline-Orchestrierung, Event-Emitter
  config/     -- JSON-basierte Settings-Persistenz
  dictionary/ -- Custom-Woerterbuch (JSON)
```

## Offene Aufgaben (Phase 3+)
- [ ] Lokaler whisper.cpp Fallback (offline STT)
- [ ] Windows-spezifische Paste-Implementierung (SendInput statt xdotool)
- [ ] Hotkey konfigurierbar machen (UI)
- [ ] History (vergangene Diktate durchsuchen)
- [ ] System-Tray Integration
- [ ] Auto-Update
- [ ] Android IME (Phase 5)

## Entscheidungen
- [2026-03-06]: Tech-Stack: Tauri v2 + React/TS + Rust.
- [2026-03-06]: API-Strategie: Groq Whisper (STT) + DeepSeek (Cleanup) primaer, whisper.cpp als Fallback.
- [2026-03-06]: Entwicklung in WSL2 (Linux-Builds), Windows-Build spaeter ueber PowerShell.
- [2026-03-06]: Audio-Thread: cpal::Stream nicht Send auf Linux -- dedizierter OS-Thread mit Channel.
- [2026-03-06]: whisper-large-v3-turbo statt v3 (3x guenstiger, reicht fuer Diktat).
- [2026-03-06]: JSON statt SQLite fuer Config/Dictionary (MVP-Simplizitaet, <1000 Eintraege).
- [2026-03-06]: API-Keys verlassen Backend nie im Klartext (nur letzte 4 Zeichen maskiert).
- [2026-03-06]: Event-basierte Pipeline (dikta://state-changed) statt polling.

## Naechste Session
Phase 3: Lokaler whisper.cpp Fallback, Windows-Paste, oder direkt testen mit echten API-Keys. Zum Testen: `GROQ_API_KEY=... DEEPSEEK_API_KEY=... cargo tauri dev`
