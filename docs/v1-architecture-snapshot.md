# Klarvo — Architektur

Generiert: 2026-04-13 | Projektversion: 0.5.0

---

## Architektur-Ueberblick

Klarvo ist eine **Tauri v2 Desktop/Mobile-App** mit drei Schichten:

```
┌─────────────────────────────────────────────────┐
│  React/TypeScript Frontend (src/)               │
│  Hooks → Komponenten → Tauri IPC Layer          │
├──────────────────────┬──────────────────────────┤
│  Tauri v2 IPC Bridge │  Android Kotlin Layer    │
│  (60+ Commands)      │  (Native Services)       │
├──────────────────────┴──────────────────────────┤
│  Rust Backend (src-tauri/src/)                  │
│  Audio → STT → LLM → Paste → History → Sync    │
└─────────────────────────────────────────────────┘
```

## Plattform-Architektur

### Desktop (Windows)
```
Hotkey Press
  → pipeline.rs: Orchestrierung
  → audio/: cpal Mikrofon-Capture (OS-Thread)
  → vad/: Silero VAD + Highpass-Filter (Silence Detection)
  → stt/: Groq/OpenAI API oder whisper-rs (lokal)
  → llm/: DeepSeek/Groq/OpenAI/Anthropic oder llama-cpp-2 (lokal)
  → paste/: arboard Clipboard + Win32 SendInput
  → history/: SQLite Speicherung
  → sync/: Turso HTTP Push (optional)
```

### Android
```
Bubble Touch Gesture
  → KlarvoOverlayService: Gesten-Routing, Pipeline
  → KlarvoAudioRecorder: AudioRecord 16kHz + Silero VAD
  → KlarvoApi: Direkte HTTP-Aufrufe (kein Tauri-Bridge)
  → LocalWhisperInference: JNI → Rust whisper-rs (optional)
  → LocalLlmInference: JNI → C++ MNN Qwen2.5 (optional)
  → KlarvoAccessibilityService: Paste in fokussiertes Textfeld
  → SQLite History (gleiche Schema wie Desktop)
```

**Wichtig:** Android umgeht den Tauri-IPC-Layer. Die Kotlin-Schicht macht direkte HTTP-Aufrufe und liest `config.json` vom Dateisystem. Nur fuer STT/LLM-Offline wird JNI nach Rust/C++ genutzt.

## Diktat-Pipeline (Datenfluss)

```
1. Audio-Capture
   Desktop: cpal → 16kHz mono PCM → WAV-Encoding
   Android: AudioRecord → 16kHz mono PCM → WAV-Encoding

2. Voice Activity Detection
   Desktop: Silero VAD (Rust, 85Hz Highpass) → Silence Callback
   Android: Silero VAD (Kotlin, Energy Gate + ONNX) → Silence Callback

3. Speech-to-Text
   Cloud:  Groq Whisper API / OpenAI Whisper API (multipart/form-data)
   Lokal:  whisper-rs (GGML, Desktop/Android via JNI)
   Prompt: Sprach-Hint + Dictionary-Begriffe (max 224 Tokens)
   Filter: Halluzinations-Erkennung (Echo-Detection)

4. LLM Text-Cleanup
   Cloud:  DeepSeek / Groq / OpenAI / Anthropic / OpenRouter
   Lokal:  llama-cpp-2 (Windows) / MNN Qwen2.5 (Android)
   Stile:  Polished (professionell), Verbatim (minimal), Chat (locker)
   Chunking: Texte >800 Zeichen werden parallel in Chunks verarbeitet
   Fallback: Automatischer Provider-Wechsel bei 429/5xx

5. Text-Einfuegung
   Desktop: arboard Clipboard → Focus-Restore → Win32 Ctrl+V
   Android: Clipboard → AccessibilityService.pasteIntoFocusedField()
   Fallback: ClipboardOnly-Modus wenn Focus-Restore fehlschlaegt

6. Persistierung
   SQLite: history.db (text, raw_text, style, language, uuid, synced)
   Turso: Cross-Device Sync (HTTP Pipeline API, UUID-Deduplizierung)
```

## State-Management

### Rust Backend (AppState)

Zentral in `lib.rs` als Tauri Managed State:

| Feld | Typ | Zweck |
|------|-----|-------|
| `recorder` | `Arc<AudioRecorder>` | Shared Audio-Capture |
| `stt_provider` | `RwLock<Arc<dyn SttProvider>>` | Hot-swappable STT |
| `cleanup_provider` | `RwLock<Arc<dyn CleanupProvider>>` | Hot-swappable LLM |
| `config` | `Mutex<AppConfig>` | 40+ Einstellungen |
| `dictionary` | `Mutex<Dictionary>` | Benutzer-Woerterbuch |
| `history_db` | `Mutex<Connection>` | SQLite-Verbindung |
| `license_status` | `Mutex<LicenseStatus>` | Lizenz-Cache |
| `last_recording` | `Mutex<Option<Vec<u8>>>` | Gepuffertes WAV |
| `prev_foreground_hwnd` | `Mutex<Option<isize>>` | Windows Focus-Restore |
| `hotkey_paused` | `AtomicBool` | Lock-free Flag |
| `auto_loop_active` | `AtomicBool` | Lock-free Flag |

**Thread-Safety-Pattern:**
- `Arc<T>` fuer shared read-only (AudioRecorder)
- `Mutex<T>` fuer exklusiven Zugriff (nie ueber await gehalten)
- `RwLock<Arc<dyn Trait>>` fuer Hot-Swap bei Settings-Aenderung
- `AtomicBool` fuer lock-free Flags

### React Frontend

**Kein Redux/Context.** Stattdessen Hook-Komposition:

| Hook | State-Bereich |
|------|--------------|
| `useRecording` | Recording-State-Machine, Transkription, Cleanup |
| `useSettings` | Einstellungen, Sprache, Provider, Dictionary |
| `usePanels` | Panel-Sichtbarkeit (Settings/History/Stats/Notes) |
| `useLicense` | Lizenzstatus, Validierung |
| `useUiScale` | UI-Skalierung (S/M/L) |
| `useQuickTip` | Onboarding-Tipps mit Trigger-Bedingungen |

`App.tsx` orchestriert alle Hooks und propagiert State via Props nach unten.

### Android

- `config.json` gelesen vom Dateisystem (geschrieben von Tauri/Rust)
- `SharedPreferences` fuer Bubble-Position, Always-Visible, Custom Blocklist
- SQLite `history.db` unabhaengig verwaltet (gleiche Schema fuer Sync)

## Modul-Architektur (Rust Backend)

```
lib.rs (AppState + Setup)
  ├── pipeline.rs         Orchestrierung (Provider-Aufloesung, Fallback)
  ├── audio/              Mikrofon-Capture
  │   └── mod.rs          cpal (Desktop) / Stub (Android)
  ├── stt/                Speech-to-Text
  │   └── mod.rs          SttProvider Trait + 3 Implementierungen
  ├── llm/                LLM Text-Cleanup
  │   └── mod.rs          CleanupProvider Trait + 5 Implementierungen
  ├── vad/                Voice Activity Detection
  │   └── mod.rs          Silero + Highpass + Hysterese
  ├── hotkey/             Pipeline-Events
  │   └── mod.rs          PipelineState Enum + Event Struct
  ├── paste/              Clipboard + Key Simulation
  │   └── mod.rs          PasteHandler Trait (Linux/Windows)
  ├── history/            SQLite History
  │   └── mod.rs          3 Tabellen: history, usage, tips_shown
  ├── sync/               Cross-Device Sync
  │   └── mod.rs          Turso HTTP Pipeline API
  ├── config/             Konfiguration
  │   └── mod.rs          JSON Laden/Speichern, 40+ Felder
  ├── dictionary/         Custom Dictionary
  │   └── mod.rs          Term-Management + Prompt-Building
  ├── license/            Lizenzvalidierung
  │   └── mod.rs          HMAC + Lemon Squeezy + Trial/Grace
  ├── voice_command/      Sprachbefehle (experimentell)
  │   └── mod.rs          VoiceCommandEngine + Pattern-Matching
  └── commands/           Tauri IPC Commands (60+)
      ├── recording.rs    Aufnahme, STT, Cleanup
      ├── settings.rs     Einstellungen, API-Keys
      ├── dictionary.rs   Dictionary CRUD
      ├── history.rs      History, Stats, Notizen
      ├── license.rs      Lizenz-Validierung
      ├── misc.rs         Profiles, Snippets, Sync, Bar
      ├── whisper.rs      Whisper-Modell-Download
      ├── llm_model.rs    LLM-Modell-Download
      ├── voice_command.rs Voice Command Toggle
      └── feedback.rs     Feedback-Webhook
```

## Provider-Abstraktion

### STT-Provider (Trait: `SttProvider`)
| Provider | Klasse | Endpoint | Modell |
|----------|--------|----------|--------|
| Groq | `GroqWhisper` | api.groq.com | whisper-large-v3-turbo |
| OpenAI | `OpenAiWhisper` | api.openai.com | whisper-1 |
| Lokal | `LocalWhisperProvider` | — | GGML (small/medium/large-v3) |

### LLM-Provider (Trait: `CleanupProvider`)
| Provider | Klasse | Endpoint | Modell |
|----------|--------|----------|--------|
| DeepSeek | `DeepSeekCleanup` | api.deepseek.com | deepseek-chat |
| Groq | `GroqCleanup` | api.groq.com | llama-3.3-70b-versatile |
| OpenAI | `OpenAiCleanup` | api.openai.com | gpt-4o-mini |
| Anthropic | `AnthropicCleanup` | api.anthropic.com | claude-* |
| OpenRouter | — | openrouter.ai | deepseek/deepseek-chat |
| Lokal | `LocalLlmCleanup` | — | llama.cpp (Windows) / MNN (Android) |

**Auto-Fallback:** Bei 429 (Rate Limit) oder 5xx wechselt die Pipeline automatisch zum naechsten verfuegbaren Provider.

## Datenbank-Schema

### SQLite (history.db)

```sql
-- Diktat-History
CREATE TABLE history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  text TEXT NOT NULL,
  raw_text TEXT,
  style TEXT NOT NULL DEFAULT 'polished',
  language TEXT NOT NULL DEFAULT '',
  is_note INTEGER NOT NULL DEFAULT 0,
  app_name TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  uuid TEXT,          -- v4 UUID fuer Sync-Deduplizierung
  device_id TEXT,
  synced INTEGER NOT NULL DEFAULT 0
);

-- Nutzungsstatistiken / Cost Tracking
CREATE TABLE usage (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  service TEXT NOT NULL,
  audio_duration_ms INTEGER,
  prompt_tokens INTEGER,
  completion_tokens INTEGER,
  estimated_cost_usd REAL NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Onboarding-Tipps (einmal pro Tipp)
CREATE TABLE tips_shown (
  tip_id TEXT PRIMARY KEY,
  shown_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### Konfiguration (config.json)

JSON-Datei in `{app_data_dir}/config.json` mit 40+ Feldern:
- API-Keys (Groq, DeepSeek, OpenAI, Anthropic, OpenRouter, Turso)
- Sprache, Cleanup-Stil, Output-Sprache
- Hotkey-Konfiguration (2 Slots mit je Shortcut + Modus + Auto-Send)
- Audio-Geraet, STT-Provider/Modell, LLM-Provider
- Lizenz-Daten (Key, Timestamps, LS Instance ID)
- Android-spezifisch: Bubble-Konfiguration (Tap/LongPress-Modi, Groesse, Opazitaet)
- Advanced Settings (LLM-Prompts, Temperaturen, Schwellwerte)

## Sicherheitsarchitektur

### Lizenz-System (Dual)
1. **HMAC-SHA256:** Offline-validierbar, `KLARVO-XXXX-XXXX-XXXX-XXXX` Format
2. **Lemon Squeezy:** Online-Aktivierung, Geraete-Limit (3), API-Validierung
3. **Trial:** 60 Tage (EA) → 14 Tage (Launch), danach 48h Grace Period
4. **Feature Gating:** 13 lizenzpflichtige Features via `LicensedFeature` Enum

### Prompt Injection Schutz
- **Sandwich Defense:** Kern-Instruktionen am Anfang UND Ende des System-Prompts
- **Output Sanitization:** ANSI, Bidi-Overrides, Null-Bytes, Zero-Width-Characters
- **Halluzinations-Erkennung:** `is_hallucination()` prueft ob STT den Prompt zurueckgibt
- **Testframework:** 27 Testfaelle gegen 6 Injection-Surfaces (Arcanum PI Taxonomy)

### API-Key-Sicherheit
- Keys als Plaintext in `config.json` (bekanntes TODO: Migration zu System Keystore)
- Masking im Frontend (nur letzte 4 Zeichen sichtbar)
- `.env` fuer Entwicklung, in `.gitignore`

### Android-Sicherheit
- Banking-App-Blocklist: Mandatory Hide (nicht deaktivierbar)
- Foreground Service mit Notification (transparent fuer Nutzer)
- AccessibilityService nur fuer Tastatur-Erkennung + Paste

## Plattform-Guards

Rust-seitige bedingte Kompilierung:

```rust
#[cfg(desktop)]          // Audio Capture, VAD, Hotkey, FloatingBar, Voice Command
#[cfg(target_os = "windows")]  // Win32 Paste, Registry Autostart, Tray, Local LLM
#[cfg(target_os = "android")]  // JNI Bridge, Audio Stub
#[cfg(target_os = "linux")]    // xdotool Paste
```

Frontend-seitige Erkennung:
```typescript
export const isMobile = /Android|iPhone|iPad/i.test(navigator.userAgent);
```

## Tauri IPC Commands (Auszug der wichtigsten)

| Kategorie | Commands | Modul |
|-----------|----------|-------|
| Recording | start/stop/cancel_recording, transcribe_audio, cleanup_text | recording.rs |
| Settings | get/save_settings, set_language, set_hotkey, validate_api_key | settings.rs |
| History | get_history, search_history, get_usage_stats, add_history_entry | history.rs |
| Dictionary | get/add/remove_dictionary_terms | dictionary.rs |
| License | validate_license, get_license_status, deactivate_license | license.rs |
| Models | get/download/delete_whisper_models, get/download_llm_model | whisper.rs, llm_model.rs |
| Misc | sync_history, save_bar_position, paste_snippet, send_feedback | misc.rs, feedback.rs |

Vollstaendige Liste: 60+ Commands (siehe Deep-Scan-Ergebnisse).
