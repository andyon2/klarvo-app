# Projektstatus

## Aktueller Stand
Version 0.4.0. Voll funktionsfaehiges Voice-Dictation-Tool. 196 Rust-Tests. Windows + Android Builds laufen. GitHub Repo public, erster Release veroeffentlicht.

## Naechste Sessions (in Reihenfolge)

1. **Signing Keys + Auto-Update** → `briefings/plan-signing-auto-update.md`
   - Signing Keys generieren, Updater-Plugin konfigurieren, /release Skill anpassen
   - Damit Tester automatisch Updates kriegen (nur Windows, Android bleibt APK-Sideload)

2. **Offline whisper.cpp Fallback** → `briefings/plan-offline-whisper.md`
   - whisper-rs Integration, Model-Download, GPU-Detection, Fallback-Logik
   - Grosses Feature, 1-2 Sessions. Erstmal nur Windows.

3. **Bubble Size/Opacity Controls** → `briefings/plan-bubble-appearance.md`
   - Presets statt Slider (Klein/Normal/Gross). Backend-Infrastruktur steht bereits.
   - Halbe Session, kann nebenbei passieren.

## Bekannte Bugs
- [windows] Signing Keys noch nicht generiert (Warnung bei jedem Build) → wird in Session 1 gefixt

## Backlog
- [ ] [shared] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter existiert)
- [ ] [windows] GitHub Releases CI/CD Pipeline (erst relevant wenn manueller Release nervt)

## Aenderungen Session 2026-03-08 (Session 3)

### Windows -- Floating Bar Redesign
- [x] Idle State: Gruener Kreis → Duenne semi-transparente Pill (80x10px, kaum sichtbar)
- [x] Expanded State: Hoehe halbiert (36→18), Breite reduziert (220→164)
- [x] Waveform-Farbe: Rot → Weiches Blau (rgba(147,197,253,0.85))
- [x] Stop/Cancel-Button hinzugefuegt (rotes Quadrat-Icon, bricht Recording ab)
- [x] cancel_recording Tauri-Command implementiert
- [x] Win32 Window-Region angepasst (idle=pill statt circle)

### Android -- Bubble Settings Cleanup
- [x] Bubble Appearance Slider aus Settings entfernt (buggy)
- [x] Config-Migration beibehalten (SharedPreferences → config.json)
- [x] BubbleSettingsMenu.kt geleert

### Infra -- GitHub + Release-Workflow
- [x] GitHub Repo erstellt (public): https://github.com/andyon2/dikta
- [x] Release v0.4.0 veroeffentlicht mit Windows-Installer + Android-APK
- [x] /release Skill erstellt (Version bump + Build + GitHub Release)
- [x] README neu geschrieben (tester-tauglich, deutsche Umlaute)
- [x] Social Preview erstellt und hochgeladen
- [x] Agent/Skill-Audit eingearbeitet (android-platform, ui-dev, build-app, etc.)

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
DiktaApi.kt                  -- STT + Chunked Cleanup + History-DB + Turso
DiktaAccessibilityService.kt -- Keyboard-Erkennung + Auto-Paste
MainActivity.kt              -- Permission-Flow
```
