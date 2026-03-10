# Projektstatus

## Aktueller Stand
Version 0.4.1. Voll funktionsfaehiges Voice-Dictation-Tool. 196 Rust-Tests (alle gruen). Windows + Android Builds stabil und signiert. Zwei-Repo-Setup: dikta (privat, Arbeitsrepo) + dikta-public (oeffentlich, Releases). Auto-Update-Infrastruktur steht (Updater-Endpoint zeigt auf dikta-public).

## Naechste Sessions (in Reihenfolge, Business-priorisiert)

1. **Offline whisper.cpp Fallback** → `briefings/plan-offline-whisper.md`
   - whisper-rs Integration, Model-Download, GPU-Detection, Fallback-Logik
   - DAS Differenzierungsmerkmal. Staerkster Kaufgrund.
   - Grosses Feature, 1-2 Sessions. Erstmal nur Windows.

2. **Onboarding/Polish** → [Briefing noch zu erstellen]
   - 3-Schritt-Setup: Install -> Hotkey -> Go
   - Erster Eindruck entscheidet bei Paid-Produkt

3. **Bubble Size/Opacity Controls** → `briefings/plan-bubble-appearance.md`
   - Presets statt Slider (Klein/Normal/Gross). Backend-Infrastruktur steht bereits.
   - Halbe Session, kann nebenbei passieren.

## Abgeschlossen
- [x] **License-Key-System** (v0.4.1) -- HMAC-Validierung, Feature-Gating, Open Core Modell
- [x] **Repo-Trennung** -- dikta (privat) + dikta-public (oeffentlich), publish.sh Script

## Bekannte Bugs
- Keine kritischen Bugs bekannt

## Backlog
- [ ] [shared] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter existiert)
- [ ] [windows] GitHub Releases CI/CD Pipeline (erst relevant wenn manueller Release nervt)

## Aenderungen Session 2026-03-10 (Housekeeping)

- [x] Repo-Trennung: dikta (privat) + dikta-public (oeffentlich)
- [x] publish.sh Script (rsync + Marker-Check gegen Agent-Daten-Leak)
- [x] dikta-tech-lead: --remote und --get-prompt Flags (fuer Claude Launcher)
- [x] project-status.md: License-Key als erledigt, Prioritaeten aktualisiert

## Aenderungen Session 2026-03-09b (Project-Builder-Ueberarbeitung)

Externe Ueberarbeitung durch Project Builder nach `/reflect`-Analyse:
- [x] CLAUDE.md: Skill-Tabelle vervollstaendigt (+/reflect, +/learn), Projektstruktur aktualisiert
- [x] feedback/inbox.md angelegt (Tester-Feedback-Inbox)
- [x] Sessionstart liest jetzt feedback/inbox.md (main-agent.md + Starter-Script)
- [x] scripts/rust-core Starter-Script erstellt (Direkt-Sessions fuer Rust-Debugging)
- [x] /release Recovery-Pfad dokumentiert (Version-Bump revertern bei Build-Fehler)
- [x] /learn Skill + sources/-Pipeline eingerichtet (sources/inbox → knowledge/ → archive)
- [x] main-agent.md Skill-Tabelle um /reflect und /learn ergaenzt
- [x] License-Key-System implementiert (HMAC-Validierung, Feature-Gating)

## Aenderungen Session 2026-03-09 (Session 5)

### Signing + Auto-Update + Release
- [x] Signing Keys generiert (~/.tauri/dikta.key), pubkey in tauri.conf.json
- [x] Build-Script: Liest Key direkt aus WSL-Keyfile, .env-Loader Bugfix
- [x] Signierter Windows-Build verifiziert (.exe + .exe.sig)
- [x] latest.json Updater-Manifest generiert
- [x] Release v0.4.1 veroeffentlicht (Windows signiert + Android APK + latest.json)
- [x] /release Skill aktualisiert (latest.json + Signatur-Artefakte)

### UI + Infra
- [x] About-Sektion in Settings (Version, Autor, GitHub-Link, MIT License) -- beide Plattformen
- [x] Android-Build: APK mit Versions-Postfix (Dikta-vX.Y.Z.apk statt Dikta.apk)
- [x] Product-Strategist Ergebnisse reviewed und committed

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
components/SettingsPanel.tsx     -- Settings (Accordion-Sektionen, About)
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
