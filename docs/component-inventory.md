# Klarvo — Komponenteninventar

Generiert: 2026-04-13 | Projektversion: 0.5.0

---

## React-Komponenten (src/components/)

### Layout & Navigation

| Komponente | Datei | Zweck |
|------------|-------|-------|
| App | App.tsx | Haupt-Orchestrator: 6 Hooks, Panels, Recording-Button |
| SettingsPanel | SettingsPanel.tsx | Drill-Down Settings (Home → Kategorien) |
| SettingsHome | settings/SettingsHome.tsx | 8 Kategorien (Recording, AI, Appearance, ...) |
| SettingsRow | settings/SettingsRow.tsx | Wiederverwendbare Zeile mit Label + Aktion |
| SettingsSubPageHeader | settings/SettingsSubPageHeader.tsx | Zurueck-Navigation in Sub-Seiten |

### Settings Sub-Seiten

| Komponente | Datei | Einstellungen |
|------------|-------|---------------|
| RecordingAudioContent | settings/RecordingAudioContent.tsx | Mikrofon, STT-Provider/Modell, Custom Prompt, Whisper Mode |
| AiProvidersContent | settings/AiProvidersContent.tsx | API-Keys (5 Provider) mit Live-Validierung |
| AppearanceLanguageContent | settings/AppearanceLanguageContent.tsx | Sprache, Output-Sprache, UI-Scale (S/M/L) |
| ShortcutsContent | settings/ShortcutsContent.tsx | 2 Hotkey-Slots, Modi, Insert-and-Send, Silence-Dauer |
| DictionaryContent | settings/DictionaryContent.tsx | Custom Terms als Tags mit Add/Remove |
| LicenseSettings | settings/LicenseSettings.tsx | Lizenz-Eingabe, Status, Deaktivierung |
| AboutContent | settings/AboutContent.tsx | Version, Links, Credits |
| AdvancedSettingsPanel | AdvancedSettingsPanel.tsx | LLM-Prompts, Temperaturen, Schwellwerte, 30+ Felder |

### Feature-Panels

| Komponente | Datei | Zweck |
|------------|-------|-------|
| CostDashboard | CostDashboard.tsx | Nutzungsstatistiken (Kosten, Woerter, Diktate) |
| VoiceNotesPanel | VoiceNotesPanel.tsx | Sprachnotizen-Liste (EA: ausgeblendet) |
| SnippetsPanel | SnippetsPanel.tsx | Text-Snippets fuer Quick-Paste |
| FeedbackModal | FeedbackModal.tsx | In-App Feedback (Bug/Feature/Other) |

### Desktop-spezifisch

| Komponente | Datei | Zweck |
|------------|-------|-------|
| FloatingBar | FloatingBar.tsx | Transparente Pill-Statusanzeige mit Waveform |
| WhisperModelManager | WhisperModelManager.tsx | Offline-STT-Modell Download/Delete |
| LlmModelManager | LlmModelManager.tsx | Offline-LLM-Modell Download/Delete |

### Allgemein

| Komponente | Datei | Zweck |
|------------|-------|-------|
| Onboarding | Onboarding.tsx | Multi-Step Einrichtungs-Wizard (Cloud/Offline) |
| QuickTip | QuickTip.tsx | Toast mit Onboarding-Tipps |
| ThemeSwitcher | ThemeSwitcher.tsx | Dark/Light Toggle |
| PreviewComments | PreviewComments.tsx | Info-Box im Browser-Preview-Modus |
| MobileTextarea | MobileTextarea.tsx | Touch-optimierte Textarea |

### UI-Primitives (ui.tsx)

| Primitive | Zweck |
|-----------|-------|
| StatusDot | 2x2 Indikator (aktiv/inaktiv) |
| DictionaryTag | Inline-Tag mit Close-Button (Touch-optimiert) |
| FillerStatsChart | Horizontales Balkendiagramm (Top 10 Fuellwoerter) |
| HighlightedText | Such-Highlighting mit Kontext-Extraktion |
| StatCard | Info-Box mit Label + Wert + Sub-Text |

### Icons (icons.tsx)

SVG-Komponenten: MicIcon, StopIcon, SpinnerIcon, GearIcon, CloseIcon, LockIcon, FeedbackIcon u.a.

---

## React Hooks (src/hooks/)

| Hook | State-Bereich | Tauri-Commands |
|------|--------------|----------------|
| useRecording | Recording-State-Machine, Result-Text, Error | start/stop_recording, transcribe, cleanup |
| useSettings | Alle Einstellungen, Provider, Dictionary | get/save_settings, set_language/style/hotkey |
| usePanels | Panel-Sichtbarkeit (5 Panels) | — (Pure State) |
| useLicense | Lizenzstatus, Source, Validierung | get/validate/remove/deactivate_license |
| useUiScale | UI-Skalierung | getAdvancedSettings |
| useQuickTip | Onboarding-Tipps, Trigger-Logik | isTipShown, markTipShown |

---

## Rust-Module (src-tauri/src/)

### Core Pipeline

| Modul | Dateien | Zweck |
|-------|---------|-------|
| pipeline | pipeline.rs | End-to-End Diktat-Orchestrierung, Provider-Aufloesung, Fallback |
| audio | audio/mod.rs | Mikrofon-Capture (cpal Desktop / Stub Android) |
| stt | stt/mod.rs | SttProvider Trait: Groq, OpenAI, Local Whisper |
| llm | llm/mod.rs | CleanupProvider Trait: DeepSeek, Groq, OpenAI, Anthropic, Local |
| vad | vad/mod.rs | Silero VAD + Highpass-Filter, Dual-Threshold-Hysterese |
| paste | paste/mod.rs | PasteHandler Trait: Win32 SendInput, Linux xdotool |

### Daten & Konfiguration

| Modul | Dateien | Zweck |
|-------|---------|-------|
| history | history/mod.rs | SQLite: 3 Tabellen (history, usage, tips_shown) |
| sync | sync/mod.rs | Turso HTTP Pipeline API (Push/Pull, UUID-Dedup) |
| config | config/mod.rs | JSON: 40+ Felder, Hot-Reload bei Aenderung |
| dictionary | dictionary/mod.rs | Term-Liste, Prompt-Building, Limit-Pruefung |
| license | license/mod.rs | HMAC-SHA256, Lemon Squeezy API, Trial/Grace |

### Sonstiges

| Modul | Dateien | Zweck |
|-------|---------|-------|
| hotkey | hotkey/mod.rs | PipelineState Enum, PipelineEvent Struct |
| voice_command | voice_command/mod.rs | VoiceCommandEngine (experimentell, Desktop) |
| commands | commands/*.rs | 60+ Tauri IPC Command Handler |

---

## Android/Kotlin-Klassen (android/kotlin-src/)

### Services

| Klasse | Zweck | Typ |
|--------|-------|-----|
| KlarvoOverlayService | Bubble-Management, Pipeline, Gesten | Foreground Service |
| KlarvoAccessibilityService | Tastatur-Erkennung, Paste, Banking-Detection | Accessibility Service |

### Views

| Klasse | Zweck |
|--------|-------|
| FloatingBubbleView | Custom Canvas-Rendering: 4 States, Waveform, Touch-Zones |

### Audio & Inference

| Klasse | Zweck |
|--------|-------|
| KlarvoAudioRecorder | 16kHz PCM + Silero VAD (Energy Gate + ONNX) |
| LocalWhisperInference | JNI → Rust whisper-rs (Offline STT) |
| LocalLlmInference | JNI → C++ MNN Qwen2.5 (Offline LLM) |

### Infrastruktur

| Klasse | Zweck |
|--------|-------|
| KlarvoApi | HTTP-Client (STT, LLM, History, Turso Sync) |
| KlarvoLogger | Dual-Sink (Logcat + rotierende Datei, 2MB, max 5) |
| BankingAppBlocklist | 50+ Banking/Passwort-Apps (nicht deaktivierbar) |
| MainActivity | Permission-Sequenzierung (6 Schritte) |

---

## Design-System

### Tailwind Custom Properties (styles.css)

| Farbe | Variable | Verwendung |
|-------|----------|------------|
| Teal | `klarvo-primary` | Aufnahme, primaere Aktionen |
| Rot | `klarvo-danger` | Fehler, Abbruch |
| Orange | `klarvo-warning` | Verarbeitung, Warnungen |
| Text | `klarvo-text` | Haupttext |
| Muted | `klarvo-muted` | Sekundaertext |
| Surface | `klarvo-surface` | Kartenflaechen |
| Elevated | `klarvo-elevated` | Modale, Popups |
| Border | `klarvo-border` | Rahmen |
| Background | `klarvo-bg` | Hintergrund |

### Responsive Strategie

- Desktop: Kompakte Schrift (xs/sm), 20px Icons
- Mobile: Groessere Touch-Targets (32x32), sm/base Schrift, 24px Icons
- UI-Scale: 14px (S) / 16px (M) / 18px (L) via `document.documentElement.style.fontSize`
