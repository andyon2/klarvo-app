# Projektstatus

## Aktueller Stand
Version 0.4.0. Voll funktionsfaehiges Voice-Dictation-Tool. 196 Rust-Tests (alle gruen). Windows + Android Builds stabil. GitHub Repo public mit Release v0.4.0. Rust-Test-Blocker aufgeloest.

## Naechste Sessions (in Reihenfolge, Business-priorisiert)

1. **Signing Keys + Auto-Update** → `briefings/plan-signing-auto-update.md`
   - Signing Keys generieren, Updater-Plugin konfigurieren, /release Skill anpassen
   - Grundvoraussetzung fuer vertrauenswuerdige Distribution

2. **License-Key-System** → [Briefing noch zu erstellen]
   - Open Core Modell: Free-Tier (Basis-Diktat) vs. Paid-Tier (EUR 29, alle Features)
   - Muss vor erstem Paid Release stehen
   - Details siehe `knowledge/product-strategy.md` Abschnitt Monetarisierung

3. **Offline whisper.cpp Fallback** → `briefings/plan-offline-whisper.md`
   - whisper-rs Integration, Model-Download, GPU-Detection, Fallback-Logik
   - DAS Differenzierungsmerkmal. Staerkster Kaufgrund.
   - Grosses Feature, 1-2 Sessions. Erstmal nur Windows.

4. **Onboarding/Polish** → [Briefing noch zu erstellen]
   - 3-Schritt-Setup: Install -> Hotkey -> Go
   - Erster Eindruck entscheidet bei Paid-Produkt

5. **Bubble Size/Opacity Controls** → `briefings/plan-bubble-appearance.md`
   - Presets statt Slider (Klein/Normal/Gross). Backend-Infrastruktur steht bereits.
   - Halbe Session, kann nebenbei passieren.

## Bekannte Bugs
- [windows] Signing Keys noch nicht generiert (Warnung bei jedem Build) → wird in Session 1 gefixt

## Backlog
- [ ] [shared] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter existiert)
- [ ] [windows] GitHub Releases CI/CD Pipeline (erst relevant wenn manueller Release nervt)

## Aenderungen Session 2026-03-09 (Session 4)

### Testing + Competitor Research
- [x] Rust-Test-Blocker gefixt: bubble_opacity/bubble_size in 4 Test-Stellen ergaenzt
- [x] cargo test kompiliert wieder, alle 196 Tests gruen
- [x] OpenWhispr Wettbewerbsanalyse durchgefuehrt:
  - Electron-basiert, kein Mobile-Support
  - MIT-Lizenz, 1640 Stars
  - Feature-Umfang aehnlich (Transcribe + Cleanup + Hotkey)
  - **Struktur-Entscheidung ausstehend:** Wo und wie Wettbewerbsanalysen in Projektdokumentation ablegen?

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
