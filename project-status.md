# Projektstatus

## Aktueller Stand
Phase 5 in Arbeit. Dikta ist ein voll funktionsfaehiges Voice-Dictation-Tool mit Multi-Provider-Support, History, Stats, Voice Notes, Text Snippets, App-Context-Erkennung, Multi-Format-Output, Webhook-Export, System-Tray und Auto-Update-Infra. 178 Rust-Tests, Windows Release-Build laeuft.

## Abgeschlossene Aufgaben

### Phase 1 -- Foundation
- [x] Tauri v2 Projekt initialisiert (React 19 + TypeScript + Vite 7 + Tailwind CSS v4)
- [x] Audio-Capture Modul (cpal, WAV 16kHz mono 16-bit PCM, dedizierter Audio-Thread)
- [x] Groq Whisper STT Client (SttProvider Trait + GroqWhisper Impl, prompt-Parameter)
- [x] DeepSeek Cleanup Client (CleanupProvider Trait + DeepSeekCleanup Impl, 3 Styles)
- [x] Overlay-UI (State-Machine, Record-Button, Style-Picker, Status-Bar)
- [x] API-Provider-Docs recherchiert (Groq Whisper + DeepSeek Chat)

### Phase 2 -- Integration
- [x] End-to-End-Flow: Record -> Transcribe -> Cleanup -> Paste
- [x] API-Key-Management (Settings-Panel mit maskierten Keys)
- [x] Globaler Hotkey (konfigurierbar, Hold/Toggle Mode)
- [x] Text-Paste (Win32 SendInput + Clipboard)
- [x] Event-basierte Pipeline (dikta://state-changed)
- [x] Settings-Persistenz (JSON config.json)
- [x] Custom Dictionary (Groq-Prompt + LLM-System-Prompt)
- [x] Onboarding-Wizard

### Phase 3 -- Multi-Provider & Advanced Features
- [x] Multi-Provider STT: Groq Whisper, OpenAI Whisper (konfigurierbare Prioritaet)
- [x] Multi-Provider LLM: DeepSeek, OpenAI, Anthropic, Groq/Llama (konfigurierbare Prioritaet)
- [x] Provider Priority Drag & Drop UI
- [x] Command Mode (Ctrl+Shift+E: Text selektieren, Sprachbefehl geben)
- [x] Whisper Mode (Verstaerkung fuer leises Diktieren)
- [x] Live Preview (Echtzeit-Transkript waehrend Aufnahme)
- [x] History + Search (SQLite, Volltextsuche mit Highlighting)
- [x] Usage Statistics (Kosten-Tracking STT/LLM, Filler-Word-Analyse)
- [x] Chunked Parallel Cleanup (lange Texte parallel verarbeiten)
- [x] Raw-Text-Anzeige (Original vs. bereinigt)
- [x] Code-Switching DE/EN (verbesserte STT-Prompts)
- [x] App Profiles (Style/Language/Prompt pro App)

### Phase 4 -- Productivity Features
- [x] Live Translation (Output-Sprache konfigurierbar, 13 Sprachen)
- [x] Multi-Format Output (Email, Bullets, Summary + Reset-Button)
- [x] Filler Word Statistics (aufklappbar in Stats)
- [x] History Search mit Kontext-Highlighting
- [x] App-Context pro Diktat (App-Name in History gespeichert, durchsuchbar)
- [x] Voice Notes Mode (eigenes Panel, Aufnahme speichern statt pasten)
- [x] Text Snippets (Quick-Access Panel, Textbausteine ins aktive Fenster pasten)
- [x] Verbesserter STT-Prompt (Conditioning Text fuer alle Sprachen)

### Phase 5 -- Platform & Infra
- [x] Getrennte History-Suche (Text + App-Name als separate Felder)
- [x] Webhook/API Export (HTTP POST nach jedem Diktat, konfigurierbare URL)
- [x] System-Tray Integration (Minimize to Tray, Tray-Menue mit Settings/Quit)
- [x] Auto-Update Infra (tauri-plugin-updater, GitHub Releases Endpoint, Signing)
- [x] Floating Bar Window (Basis-Fenster am unteren Bildschirmrand)

## Module (Rust)
```
src-tauri/src/
  audio/      -- Audio-Capture (cpal), WAV-Encoding, Silence Detection
  stt/        -- SttProvider Trait, GroqWhisper, OpenAiWhisper
  llm/        -- CleanupProvider Trait, DeepSeek/OpenAI/Anthropic/Groq, Chunked Cleanup
  paste/      -- Win32 SendInput + Clipboard, Foreground Window Capture
  hotkey/     -- Pipeline-Orchestrierung, Event-Emitter, Command Mode
  config/     -- JSON Settings + App Profiles + Text Snippets + Webhook
  dictionary/ -- Custom-Woerterbuch (JSON)
  history/    -- SQLite History + Voice Notes + Usage Stats + Filler Analysis
```

## Offene Aufgaben (naechste Phasen)
- [ ] Lokaler whisper.cpp Fallback (offline STT)
- [ ] Android IME (Tauri v2 Mobile + Kotlin InputMethodService)
- [ ] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] Signing Keys generieren (Tauri Updater braucht Keypair)
- [ ] GitHub Releases CI/CD Pipeline fuer Auto-Update

## Entscheidungen
- [2026-03-06]: Tech-Stack: Tauri v2 + React/TS + Rust.
- [2026-03-06]: API-Strategie: Groq Whisper (STT) + DeepSeek (Cleanup) primaer, Fallback-Kette konfigurierbar.
- [2026-03-06]: Audio-Thread: cpal::Stream nicht Send auf Linux -- dedizierter OS-Thread mit Channel.
- [2026-03-06]: whisper-large-v3-turbo statt v3 (3x guenstiger, reicht fuer Diktat).
- [2026-03-06]: JSON fuer Config/Dictionary, SQLite fuer History/Stats.
- [2026-03-06]: API-Keys verlassen Backend nie im Klartext (nur letzte 4 Zeichen maskiert).
- [2026-03-06]: Event-basierte Pipeline (dikta://state-changed) statt polling.
- [2026-03-07]: Win32 SendInput fuer Paste statt xdotool. GetForegroundWindow fuer App-Context.
- [2026-03-07]: STT Conditioning Prompts pro Sprache fuer bessere Kurztext-Erkennung.
- [2026-03-07]: Webhook: fire-and-forget POST, blockiert nie die Pipeline.
- [2026-03-07]: Auto-Update: tauri-plugin-updater mit GitHub Releases Endpoint. Keys noch nicht generiert.
- [2026-03-07]: History-Suche getrennt nach Text und App-Name (AND-Verknuepfung).
