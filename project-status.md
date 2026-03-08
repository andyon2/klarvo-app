# Projektstatus

## Aktueller Stand
Version 0.5.0. Voll funktionsfaehiges Voice-Dictation-Tool. 196 Rust-Tests. Windows + Android Builds laufen.

## Offene Tasks

### Bekannte Bugs
- [windows] Signing Keys noch nicht generiert (Warnung bei jedem Build)

### Backlog
- [ ] [android] Bubble Size/Opacity Controls (verschoben -- Slider-UX buggy, naechste Version)
- [ ] [shared] Lokaler whisper.cpp Fallback (offline STT)
- [ ] [shared] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] [windows] Signing Keys generieren (Tauri Updater braucht Keypair)
- [ ] [windows] GitHub Releases CI/CD Pipeline fuer Auto-Update
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter existiert)

## Aenderungen Session 2026-03-08 (Session 3)

### Windows -- Floating Bar Redesign
- [x] Idle State: Gruener Kreis → Duenne semi-transparente Pill (80x10px, kaum sichtbar)
- [x] Expanded State: Hoehe halbiert (36→18), Breite reduziert (220→164)
- [x] Waveform-Farbe: Rot → Weiches Blau (rgba(147,197,253,0.85))
- [x] Stop/Cancel-Button hinzugefuegt (rotes Quadrat-Icon, bricht Recording ab)
- [x] cancel_recording Tauri-Command implementiert (stoppt Recorder + emittiert idle)
- [x] Win32 Window-Region angepasst (idle=pill statt circle)
- [x] set_bar_shape aktualisiert: "circle"→"idle", neue Dimensionen

### Android -- Bubble Settings Cleanup
- [x] Bubble Appearance Slider aus Settings entfernt (buggy, Slider springen zurueck)
- [x] Config-Migration beibehalten (SharedPreferences → config.json mit Defaults)
- [x] BubbleSettingsMenu.kt geleert (Double-Tap-Menu war broken)

### Shared
- [x] bubble_size/bubble_opacity in AppConfig, SettingsView, saveSettings (Backend-Infrastruktur steht)

## Aenderungen Session 2026-03-08 (Session 2)

### Android
- [x] Waveform Noise Gate + Smoothing (0.04 Floor, 2.5x Amplify, 3-Sample Average)
- [x] Waveform Bars groesser (7dp breit, 5 Bars, 10% min, pow(0.6) Amplitude-Kurve)
- [x] Push-to-Talk: Bubble bleibt rund, skaliert 1.3x mit OvershootInterpolator (kein Bar-Expand)
- [x] Push-to-Talk: Bubble-Position fixiert waehrend Long-Press (kein Drag/Cancel)
- [x] Chunked Parallel Cleanup (4-Thread-Pool, ab 800 Zeichen, Sentence-Boundary-Split)
- [x] UI Size auf Mobile versteckt (CSS zoom glitcht in WebView)

### Shared (Windows + Android)
- [x] Collapsible Sections in SettingsPanel (Accordion: nur eine Sektion offen)
- [x] Save-Button-Fix: max-h + 48px Nav-Bar-Abzug auf Mobile
- [x] "Custom Prompt" umbenannt zu "Cleanup Instructions"
- [x] "Clear All" Button aus History entfernt
- [x] Clear-Button bei Cleanup Instructions Presets
- [x] "LLM Cleanup -- Base Prompts" Titel + Klarstellungs-Hint
- [x] Snippets ersetzt durch "Integrations" Platzhalter
- [x] Record-Button versteckt wenn Panel offen
- [x] MobileTextarea: Fullscreen-Popup fuer Textfelder auf Android
- [x] MobileTextarea: Safe-Area-Fix fuer Status-Bar (pt-9)
- [x] UI Size komplett entfernt (auch Desktop -- war buggy)
- [x] Font-Sizing: text-[10px] → text-[11px] global (bessere Lesbarkeit)
- [x] Accordion-Verhalten in Settings + Advanced Settings

### Meta
- [x] Projektdateien konsolidiert: 5 → 3 Dateien (project-status, architecture, MEMORY)
- [x] architecture.md komplett aktualisiert (Plattform-Split, Cross-Platform-Regeln)

## Modul-Referenz

### Rust (`src-tauri/src/`)
```
lib.rs       -- AppState, run(), invoke_handler
pipeline.rs  -- Hotkey pipeline: start_recording / stop_and_process
commands/    -- recording, settings, dictionary, history, misc
audio/       -- Audio-Capture (cpal, desktop-only), WAV-Encoding
stt/         -- SttProvider Trait, GroqWhisper, OpenAiWhisper
llm/         -- CleanupProvider Trait, DeepSeek/OpenAI/Anthropic/Groq
paste/       -- Win32 SendInput + Clipboard (desktop-only)
hotkey/      -- Pipeline-Orchestrierung, Event-Emitter, Command Mode
config/      -- JSON Settings + App Profiles + Text Snippets + Webhook
dictionary/  -- Custom-Woerterbuch (JSON)
history/     -- SQLite History + Voice Notes + Usage Stats + Filler Analysis
sync/        -- Cross-Device Sync via Turso HTTP API
```

### Frontend (`src/`)
```
App.tsx                          -- Hauptkomponente
FloatingBar.tsx                  -- Windows Floating Bar (thin idle pill + compact active pill)
components/SettingsPanel.tsx     -- Settings (Accordion-Sektionen)
components/AdvancedSettingsPanel.tsx
components/MobileTextarea.tsx    -- Fullscreen-Textarea-Popup (Android)
components/VoiceNotesPanel.tsx
components/icons.tsx, ui.tsx
hooks/useRecording.ts, useSettings.ts, usePanels.ts
```

### Android (`android/kotlin-src/com/dikta/voice/`)
```
DiktaOverlayService.kt      -- Foreground Service, Touch-Gesten, PTT
FloatingBubbleView.kt        -- Idle/Recording(Bar)/RecordingPTT/Processing
DiktaAudioRecorder.kt        -- Audio, WAV, Amplitude mit Noise Gate
BubbleSettingsMenu.kt         -- (geleert, Double-Tap war broken)
DiktaApi.kt                  -- STT + Chunked Cleanup + History-DB + Turso
DiktaAccessibilityService.kt -- Keyboard-Erkennung + Auto-Paste
MainActivity.kt              -- Permission-Flow
```
