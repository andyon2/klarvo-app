# Dikta Feature Inventory

**Generated from source code audit — 2026-03-20**
**Version: 0.4.6**

All features extracted directly from source. Platform: W = Windows desktop, A = Android, B = Both. License: Free = available without license key, Paid = requires valid license key.

---

## 1. Core Pipeline

| Feature | Description | Platform | License | Source |
|---------|-------------|----------|---------|--------|
| STT transcription | Converts recorded audio to raw text via Whisper API or local model | B | Free | `pipeline.rs`, `commands/recording.rs` |
| LLM cleanup | Sends raw transcript to LLM for grammar/filler/style correction | B | Free | `pipeline.rs`, `llm/mod.rs` |
| Auto-paste | Pastes final result into the focused window immediately after processing | B | Free | `pipeline.rs`, `commands/recording.rs` |
| Clipboard fallback | Falls back to clipboard-only copy when direct paste fails, shown via amber pill state | W | Free | `pipeline.rs`, `FloatingBar.tsx` |
| Insert-and-Send | Presses Enter after paste to submit the text (e.g. in chat apps), per hotkey slot | W | Free | `pipeline.rs`, `commands/settings.rs` |
| History save | Stores every dictation result in local SQLite DB with metadata | B | Free | `pipeline.rs`, `commands/history.rs` |
| App Profile matching | Applies per-window-title cleanup style and custom prompt before processing | W | Paid | `pipeline.rs`, `commands/misc.rs` |
| Chunked LLM cleanup | Splits long transcripts at sentence boundaries and processes chunks in parallel | B | Free | `pipeline.rs`, `llm/mod.rs` |
| Hallucination detection | Strips Whisper hallucinations by detecting exact prompt echoes and word-overlap patterns | B | Free | `pipeline.rs` |
| Prompt fragment stripping | Removes leaked STT conditioning prompts from transcript before LLM cleanup | B | Free | `pipeline.rs` |
| Auto Turso sync | Fire-and-forget push of each dictation to Turso after save | B | Paid | `pipeline.rs`, `commands/misc.rs` |
| Webhook POST | POSTs dictation result JSON to a configurable URL after each recording | W | Free | `pipeline.rs`, `commands/settings.rs` |
| Output language / translation | Appends a translate instruction to the LLM cleanup call to produce output in a target language | B | Free | `pipeline.rs`, `llm/mod.rs` |
| Offline mode detection | Skips LLM cleanup when local STT provider is first in priority list (pure offline path) | W | Paid | `pipeline.rs`, `commands/recording.rs` |
| Min recording duration check | Discards recordings shorter than the configured minimum (default ~300ms) to avoid junk | B | Free | `pipeline.rs`, `config/mod.rs` |

## 2. Recording Modes

| Feature | Description | Platform | License | Source |
|---------|-------------|----------|---------|--------|
| Hold mode | Records while hotkey is held down, stops and processes on release | W | Free | `pipeline.rs` |
| Toggle mode | First press starts recording, second press stops and processes | W | Free | `pipeline.rs` |
| AutoStop mode | Starts recording on press, automatically stops after RMS-based silence is detected | W | Free | `pipeline.rs` |
| Auto (Loop) mode | Continuously re-starts recording after each result is processed until manually stopped | W | Free | `pipeline.rs` |
| Command Mode | Copies selected text via Ctrl+C, then records a voice command that rewrites the selection via LLM | W | Paid | `pipeline.rs`, `license/mod.rs` |
| Android Tap-HOLD | Single tap starts/stops recording (one of two configurable tap gesture modes) | A | Free | `DiktaOverlayService.kt` |
| Android Tap-TOGGLE | Single tap starts recording with AutoStop silence detection | A | Free | `DiktaOverlayService.kt` |
| Android Long-Press PTT | Long-press (>500ms) triggers push-to-talk with circular red bubble and scale animation | A | Free | `DiktaOverlayService.kt`, `FloatingBubbleView.kt` |
| Android Long-Press AUTOSTOP | Long-press starts AutoStop-mode recording | A | Free | `DiktaOverlayService.kt` |
| Android Auto-Loop | Automatically restarts recording after each result until stopped | A | Free | `DiktaOverlayService.kt` |

## 3. Hotkey System

| Feature | Description | Platform | License | Source |
|---------|-------------|----------|---------|--------|
| Dual hotkey slots | Two independent hotkeys (slot 0 + slot 1) each with their own mode, insert-and-send, and key binding | W | Free | `pipeline.rs`, `commands/settings.rs` |
| Hotkey slot 0 | Primary hotkey (default: user-configured), supports all four recording modes | W | Free | `pipeline.rs` |
| Hotkey slot 1 | Secondary hotkey (optional), independently configurable mode and insert-and-send | W | Free | `pipeline.rs` |
| Command hotkey | Separate hotkey for Command Mode (default: ctrl+shift+e), hold to record rewrite command | W | Paid | `pipeline.rs` |
| Hotkey pause/resume | Temporarily disables global hotkey listener (e.g. while recording shortcut in settings) | W | Free | `commands/settings.rs` |
| ShortcutRecorder UI | UI component that captures a new hotkey combination while pausing the active hotkey listener | W | Free | `SettingsPanel.tsx` |
| Active mode badge | Floating pill displays the current active slot's mode label, updates live via dikta://active-mode events | W | Free | `FloatingBar.tsx` |

## 4. Text Processing

| Feature | Description | Platform | License | Source |
|---------|-------------|----------|---------|--------|
| Polished style | LLM cleanup removing filler words, fixing grammar, producing professional prose | B | Free | `llm/mod.rs` |
| Verbatim style | Minimal LLM cleanup, preserves the speaker's original phrasing and word choice | B | Free | `llm/mod.rs` |
| Chat style | LLM cleanup targeting casual conversational tone for messaging contexts | B | Free | `llm/mod.rs` |
| Command Mode rewrite | Uses rewrite() method to rewrite selected text per the voice command | W | Paid | `llm/mod.rs`, `license/mod.rs` |
| Reformat: Email | Post-dictation LLM transform that restructures result as a professional email | B | Free | `llm/mod.rs`, `commands/settings.rs` |
| Reformat: Bullets | Post-dictation LLM transform that converts result into a bulleted list | B | Free | `llm/mod.rs`, `commands/settings.rs` |
| Reformat: Summary | Post-dictation LLM transform that condenses result into a brief summary | B | Free | `llm/mod.rs`, `commands/settings.rs` |
| Custom LLM system prompts | Per-style overrideable system prompts for power users | B | Paid | `config/mod.rs`, `commands/settings.rs`, `license/mod.rs` |
| Auto-capitalize | Automatically capitalizes the first letter of the pasted result | W | Free | `config/mod.rs` |
| App Profiles | Maps window titles to specific cleanup styles and custom prompts via regex/substring match | W | Paid | `commands/misc.rs`, `SettingsPanel.tsx`, `license/mod.rs` |

## 5. Audio

| Feature | Description | Platform | License | Source |
|---------|-------------|----------|---------|--------|
| System audio device selection | Lists and selects from all available audio input devices for recording | W | Free | `commands/recording.rs`, `SettingsPanel.tsx` |
| RMS silence detection | Computes RMS energy of audio frames to detect speech end in AutoStop/Auto modes | W | Free | `pipeline.rs`, `config/mod.rs` |
| Whisper Mode | Amplifies quiet microphone input by a configurable gain factor (default 3.0x) for quiet speakers | W | Paid | `pipeline.rs`, `config/mod.rs`, `license/mod.rs` |
| Live audio level events | Emits dikta://audio-level events at ~15 Hz during recording for waveform visualization | W | Free | `lib.rs` |
| WAV encoding (Android) | Encodes captured PCM short samples to 16kHz mono 16-bit WAV format for API submission | A | Free | `DiktaApi.kt` |
| Live transcription preview | Captures a WAV snapshot mid-recording and sends to STT for a partial preview (implemented, disabled) | W | Free | `commands/recording.rs` |

## 6. Providers

| Feature | Description | Platform | License | Source |
|---------|-------------|----------|---------|--------|
| Groq Whisper STT | Primary STT provider using Groq's Whisper API (whisper-large-v3-turbo) | B | Free | `stt/mod.rs`, `DiktaApi.kt` |
| OpenAI Whisper STT | Alternative STT provider using OpenAI's Whisper API (whisper-1) | W | Paid | `stt/mod.rs`, `license/mod.rs` |
| Local Whisper STT | Offline STT via whisper.cpp with GGML models (Windows only) | W | Paid | `stt/mod.rs`, `license/mod.rs` |
| STT priority list | Ordered list of STT providers with automatic fallback if primary fails | W | Free | `commands/recording.rs` |
| STT conditioning prompt | Builds Whisper conditioning prompt from language hint + dictionary terms | B | Free | `stt/mod.rs` |
| DeepSeek LLM cleanup | Default LLM cleanup provider (model: deepseek-chat, cheapest option) | B | Free | `llm/mod.rs` |
| OpenAI LLM cleanup | Alternative LLM provider (model: gpt-4o-mini) | B | Paid | `llm/mod.rs`, `license/mod.rs` |
| Anthropic LLM cleanup | Alternative LLM provider (model: claude-haiku-4-5-20251001); desktop only, not in UI | W | Paid | `llm/mod.rs`, `license/mod.rs` |
| Groq LLM cleanup | Alternative LLM provider via Groq (model: llama-3.3-70b-versatile) | B | Paid | `llm/mod.rs`, `DiktaApi.kt`, `license/mod.rs` |
| OpenRouter LLM cleanup | Alternative LLM provider via OpenRouter (any model) | B | Paid | `llm/mod.rs`, `DiktaApi.kt`, `license/mod.rs` |
| Live API key validation | Validates API keys against provider endpoints in real-time on settings save | W | Free | `commands/settings.rs` |
| Provider model overrides | Overrides the default model name for any LLM provider | B | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| Groq STT model selection | Allows selecting alternative Groq Whisper model variants | B | Free | `stt/mod.rs`, `config/mod.rs` |

## 7. UI / UX

| Feature | Description | Platform | License | Source |
|---------|-------------|----------|---------|--------|
| Main settings panel | Full-featured tabbed settings UI covering all configuration options | B | Free | `SettingsPanel.tsx` |
| Advanced settings panel | Collapsible power-user panel for STT prompts, LLM params, audio thresholds, paste behavior | B | Free | `AdvancedSettingsPanel.tsx` |
| Windows floating pill bar | Always-on-top transparent pill overlay showing recording/processing/done/error states | W | Free | `FloatingBar.tsx`, `lib.rs` |
| Floating bar pill shape | Pill/ellipse window region via SetWindowRgn Win32 API for shaped transparency | W | Free | `lib.rs`, `commands/misc.rs` |
| Floating bar drag | Manual mouse-event-based drag (Tauri's startDragging unreliable on transparent windows) | W | Free | `FloatingBar.tsx` |
| Floating bar position persistence | Saves and restores pill bar screen position across restarts | W | Free | `FloatingBar.tsx`, `commands/misc.rs` |
| Waveform animation | 5-bar real-time audio level waveform in the floating pill, driven by audio-level events | W | Free | `FloatingBar.tsx` |
| Return-to-Window focus | Restores focus to the originating window after settings panel closes | W | Free | `lib.rs` |
| Windows system tray | Tray icon with state-aware tooltip, Settings and Quit menu items | W | Free | `lib.rs` |
| Onboarding wizard | Multi-step first-run wizard with per-step persistence, auto-skipped for existing users | B | Free | `App.tsx`, `commands/settings.rs` |
| Quick tips system | Context-sensitive tips with per-tip shown tracking to avoid repetition | B | Free | `App.tsx`, `commands/history.rs` |
| StylePicker | Inline cleanup style picker (Polished/Verbatim/Chat) in main recording view | B | Free | `App.tsx` |
| ReformatButtons | Post-dictation quick-action buttons (Email / Bullets / Summary) | B | Free | `App.tsx` |
| History panel | Browsable list of past dictations with raw text toggle, copy and delete | B | Free | `App.tsx` |
| History search | Searches history by transcript text and/or source app name | B | Paid | `App.tsx`, `commands/history.rs`, `license/mod.rs` |
| Stats panel | Shows usage and cost statistics with Wispr Flow savings comparison | B | Paid | `App.tsx`, `CostDashboard.tsx`, `license/mod.rs` |
| Integrations panel | Placeholder panel for future desktop integrations (coming soon) | W | Free | `App.tsx` |
| App version + update check | Displays current app version and triggers GitHub release check | B | Free | `SettingsPanel.tsx` |

## 8. History & Statistics

| Feature | Description | Platform | License | Source |
|---------|-------------|----------|---------|--------|
| Dictation history | Stores each dictation with text, raw transcript, app name, duration, cost, timestamp | B | Free (50 cap) | `commands/history.rs`, `DiktaApi.kt` |
| Unlimited history | Removes the 50-entry cap to show full history list | B | Paid | `commands/history.rs`, `license/mod.rs` |
| History search | Full-text and app-name search across the entire history database | B | Paid | `commands/history.rs`, `license/mod.rs` |
| Delete history entry | Removes a single dictation from history | B | Free | `commands/history.rs` |
| Clear all history | Deletes all history entries | B | Free | `commands/history.rs` |
| Cost tracking dashboard | Tracks and displays STT and LLM cost per dictation, totals, and today's spend | B | Paid | `commands/history.rs`, `CostDashboard.tsx`, `license/mod.rs` |
| Wispr Flow savings estimate | Compares monthly Dikta cost against Wispr Flow's $12/month subscription | B | Paid | `CostDashboard.tsx` |
| Filler word analysis | Analyzes raw transcripts to identify and chart most common filler words | B | Paid | `commands/history.rs`, `license/mod.rs` |
| Voice Notes | Records and saves dictations as persistent notes instead of pasting | B | Paid | `commands/history.rs`, `VoiceNotesPanel.tsx`, `license/mod.rs` |
| Text Snippets | Named text snippets that can be pasted into the focused window on demand | W | Paid | `commands/misc.rs`, `SnippetsPanel.tsx`, `license/mod.rs` |

## 9. Android-Specific

| Feature | Description | Platform | License | Source |
|---------|-------------|----------|---------|--------|
| Floating bubble overlay | Persistent foreground service with WindowManager overlay bubble accessible from any app | A | Free | `DiktaOverlayService.kt`, `FloatingBubbleView.kt` |
| Bubble IDLE state | White circular bubble showing the app icon, serves as passive entry point | A | Free | `FloatingBubbleView.kt` |
| Bubble RECORDING state | Pill-shaped bar with cancel / waveform / confirm touch zones | A | Free | `FloatingBubbleView.kt` |
| Bubble RECORDING_PTT state | Circular red bubble that scales up 1.3x via OvershootInterpolator during push-to-talk | A | Free | `FloatingBubbleView.kt` |
| Bubble PROCESSING state | Amber circle with rotating arc spinner animation while STT/LLM runs | A | Free | `FloatingBubbleView.kt` |
| Bubble waveform animation | 5-bar animated waveform in RECORDING state with phase offsets and amplitude-driven heights | A | Free | `FloatingBubbleView.kt` |
| Bubble drag repositioning | Drag bubble to any screen position with 10dp threshold, position saved to SharedPreferences | A | Free | `DiktaOverlayService.kt` |
| Bubble size configuration | Adjustable bubble diameter from React settings | A | Free | `DiktaOverlayService.kt`, `DiktaApi.kt` |
| Bubble opacity configuration | Adjustable bubble transparency from React settings | A | Free | `DiktaOverlayService.kt`, `DiktaApi.kt` |
| Keyboard detection (AccessibilityService) | Primary method: tracks keyboard visibility via window state events | A | Free | `DiktaOverlayService.kt` |
| Keyboard detection (IMM reflection) | Fallback method: InputMethodManager reflection for unsupported devices | A | Free | `DiktaOverlayService.kt` |
| Bubble visibility modes | KEYBOARD_ONLY (default) vs ALWAYS_VISIBLE | A | Free | `DiktaOverlayService.kt` |
| Per-gesture mode configuration | Tap and long-press gestures each independently configurable | A | Free | `DiktaOverlayService.kt`, `DiktaApi.kt` |
| Per-gesture silence duration | Independent silence detection duration per tap and long-press gesture | A | Free | `DiktaApi.kt` |
| Notification mode indicator | Foreground notification displays current tap/long-press mode labels | A | Free | `DiktaOverlayService.kt` |
| AccessibilityService paste | Pastes transcribed text into focused input field via performAction | A | Free | `DiktaOverlayService.kt` |
| Android STT pipeline | Full pipeline: Groq STT -> LLM cleanup -> history save -> Turso push -> paste | A | Free | `DiktaOverlayService.kt`, `DiktaApi.kt` |
| Android chunked cleanup | Parallel chunk processing with same 800-char threshold, 4-thread pool | A | Free | `DiktaApi.kt` |
| config.json bridge | Android reads all settings from shared config.json written by React frontend | A | Free | `DiktaApi.kt` |

## 10. Sync & Cloud

| Feature | Description | Platform | License | Source |
|---------|-------------|----------|---------|--------|
| Turso cloud sync | Push/pull dictation history to Turso LibSQL cloud database | B | Paid | `commands/misc.rs`, `DiktaApi.kt`, `license/mod.rs` |
| UUID device ID | Per-device UUID used as sync partition key | B | Paid | `commands/misc.rs`, `SettingsPanel.tsx` |
| Sync now button | Manual on-demand sync trigger in settings UI | B | Paid | `SettingsPanel.tsx`, `commands/misc.rs` |
| Auto sync after dictation | Automatically pushes each new dictation to Turso as fire-and-forget | B | Paid | `pipeline.rs`, `DiktaOverlayService.kt` |
| 5-step push/pull sync | Full bidirectional sync: push local unsynced -> pull remote new -> mark synced | B | Paid | `commands/misc.rs` |
| Webhook integration | POSTs JSON payload with dictation result to user-configured URL | W | Free | `pipeline.rs`, `config/mod.rs` |
| Webhook headers | Custom HTTP headers attached to webhook POST request | W | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |

## 11. Dictionary

| Feature | Description | Platform | License | Source |
|---------|-------------|----------|---------|--------|
| Custom dictionary | List of terms injected into Whisper conditioning prompt for better accuracy | B | Free (capped) | `SettingsPanel.tsx`, `stt/mod.rs` |
| Unlimited dictionary | Removes the cap on dictionary term count | B | Paid | `license/mod.rs` |

## 12. Offline / Local Whisper

| Feature | Description | Platform | License | Source |
|---------|-------------|----------|---------|--------|
| Whisper Model Manager | UI to download, delete, and select offline GGML model files | W | Paid | `WhisperModelManager.tsx`, `license/mod.rs` |
| Small model (488 MB) | ggml-small model | W | Free | `WhisperModelManager.tsx` |
| Medium model (1.5 GB) | ggml-medium model, higher accuracy | W | Paid | `WhisperModelManager.tsx` |
| Large-v3 model (3.1 GB) | ggml-large-v3 model, highest accuracy | W | Paid | `WhisperModelManager.tsx` |
| Download with progress | Model download shows progress events | W | Paid | `WhisperModelManager.tsx` |
| GPU / CUDA acceleration | Enables CUDA GPU acceleration for local Whisper inference | W | Paid | `WhisperModelManager.tsx`, `config/mod.rs` |

## 13. License System

| Feature | Description | Platform | License | Source |
|---------|-------------|----------|---------|--------|
| License key validation | HMAC-SHA256 cryptographic validation of DIKTA-XXXX-XXXX-XXXX-XXXX key format | B | -- | `commands/license.rs`, `license/mod.rs` |
| Permanent license | License key type granting permanent access to all paid features | B | -- | `license/mod.rs` |
| Trial license | License key type with expiry date encoded as days since 2025-01-01 | B | -- | `license/mod.rs` |
| 30-day validation cache | License check result cached locally for 30 days | B | -- | `license/mod.rs`, `commands/license.rs` |
| 48-hour grace period | If cache expires and validation fails, grants 48h grace | B | -- | `license/mod.rs` |
| Early adopter 60-day grace | Extended grace period for users with keys obtained before grace system | B | -- | `license/mod.rs` |
| License status display | Shows current license state in settings | B | -- | `commands/license.rs`, `SettingsPanel.tsx` |

## 14. Advanced / Power User

| Feature | Description | Platform | License | Source |
|---------|-------------|----------|---------|--------|
| STT prompt overrides | Override Whisper conditioning prompt per language (de/en/auto) | B | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| STT temperature | Configures Whisper decoding temperature | B | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| LLM temperature | Configures LLM sampling temperature for cleanup | B | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| LLM max tokens | Configures maximum output tokens for LLM cleanup | B | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| Chunk threshold | Character threshold for text splitting (default 800) | B | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| Chunk target size | Target character size per chunk (default 600) | B | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| Silence threshold | RMS energy level below which audio is considered silence | W | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| Whisper mode threshold | RMS threshold below which Whisper Mode gain is applied | W | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| Whisper mode gain | Amplification multiplier for Whisper Mode (default 3.0x) | W | Paid | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| Min recording duration | Minimum ms of audio required to trigger STT (default ~300ms) | B | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| Paste delay | Milliseconds to wait before pasting after result is ready | W | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| Auto-paste toggle | Enables or disables automatic paste after dictation | W | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| Webhook timeout | HTTP timeout for webhook POST requests | W | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| Log level | Backend log verbosity (error/warn/info/debug/trace) | W | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| UI scale | Frontend zoom/scale factor | B | Free | `config/mod.rs`, `AdvancedSettingsPanel.tsx` |
| Windows autostart | Registers/removes Dikta from Windows startup | W | Free | `commands/settings.rs`, `SettingsPanel.tsx` |
| Hot-reload providers | Re-initializes providers on settings save without restart | W | Free | `commands/settings.rs` |

---

## USP-Analyse

Stand: 2026-03-20. Nur Features aus dem Code-Inventar, keine Spekulationen.

Bewertungs-Skala:
- **USP** -- Dikta hat es, Wispr Flow und andere nicht (oder deutlich schwaecher)
- **Staerke** -- Haben wenige Wettbewerber in dieser Form
- **Standard** -- Haben fast alle in dieser Kategorie
- **Nische** -- Nur fuer kleine Teilzielgruppe relevant

Quellen Wispr Flow: docs.wisprflow.ai, WebSearch Maerz 2026

| Feature-Bereich | Dikta | Wispr Flow | Andere Wettbewerber | Bewertung |
|-----------------|-------|------------|---------------------|-----------|
| **Dual Hotkey Slots** | 2 unabhaengige Slots mit je eigenem Modus, Insert-and-Send, Keybinding | 1 Hotkey (bis zu 4 Shortcuts fuer Command Mode, aber 1 Diktat-Slot) | Voice Type: 1, OpenWhispr: 1, Amical: 1 | **USP** |
| **Recording Modi (Breite)** | Hold, Toggle, AutoStop (RMS-silence), Auto-Loop -- alle 4 auf Windows + Android | Hold + Hands-Free (Toggle via Double-Tap) | Voice Type: Hold only; OpenWhispr: Hold/Toggle; Amical: Hold/Toggle | **Staerke** |
| **Android Floating Bubble** | Shipped: 5 Zustaende, Waveform, PTT-Animation, Drag, konfigurierbarer Tap + Long-Press | Shipped (Feb 2026), aehnliches UX-Pattern | Amical: Private Beta; Voice Type: kein Android; OpenWhispr: kein Android | **Standard** (nur vs. Wispr Flow gleichwertig; vs. Rest: Staerke) |
| **Android Per-Gesture-Konfiguration** | Tap und Long-Press jeweils unabhaengig konfigurierbar (Modus + Silence-Dauer) | Zwei Modi (Tap-Toggle / Hold), nicht per-Geste getrennt konfigurierbar | Keine anderen haben shipped Android | **USP** |
| **BYOK Multi-Provider STT + LLM** | Groq, OpenAI, lokales Whisper (STT) + DeepSeek, OpenAI, Anthropic, Groq, OpenRouter (LLM) -- alle mit eigenem API-Key | Kein BYOK. Proprietaeres Modell, kein Nutzer-API-Key, keine Provider-Wahl | Voice Type: BYOK fuer LLM (OpenAI/Groq), kein BYOK STT; Amical: BYOK STT + LLM; OpenWhispr: BYOK STT + LLM | **Staerke** (Breite der kombinierten STT+LLM Providerwahl ist USP-nah) |
| **Offline / Lokales Whisper** | whisper.cpp mit small/medium/large-v3, CUDA-Beschleunigung, Modell-Manager UI | Kein Offline-Modus, cloud-only | Voice Type: Offline (macOS-native, kein Whisper); Amical: Offline; OpenWhispr: Offline | **Staerke** (vs. Wispr Flow USP; vs. Feld Standard) |
| **App Profiles** | Mappt Window-Title (regex) auf eigenen Cleanup-Style + Custom Prompt | Context Awareness automatisch (liest aktive App, passt Ton an) -- kein manuelles Profil-System | OpenWhispr: Context-aware (auto, kein manuelles Profil); Amical: Context-aware | **Staerke** (manuelle Kontrolle vs. automatische Heuristik ist echter Unterschied) |
| **Command Mode (Text-Rewrite)** | Selektiert Text via Ctrl+C, nimmt Voice-Command auf, LLM-Rewrite der Selektion | Command Mode vorhanden (Highlight + Voice Command), ausgereift | Voice Type: Kein Command Mode; OpenWhispr: kein explizites Command Mode; Amical: unklar | **Standard** (Wispr Flow hat es auch, aber Dikta hat es ohne Abo) |
| **Post-Diktat Reformate** | Email / Bullets / Summary als One-Click-Buttons nach jedem Diktat | Keine vergleichbare Post-Processing UI nach dem Diktat gefunden | Keine anderen haben dieses explizite Post-Diktat-Transform-UI | **USP** |
| **Chunked LLM Cleanup (parallel)** | Lange Texte werden an Satzgrenzen gesplittet und parallel verarbeitet (Desktop + Android) | Verarbeitung waehrend Sprechen (Streaming), kein explizites Chunking | Amical: unklar; OpenWhispr: unklar | **Staerke** (macht lange Diktate praxistauglich) |
| **Hallucination Detection** | Erkennt und entfernt Whisper-Halluzinationen (Prompt-Echo, Word-Overlap-Patterns) | Proprietaeres Modell -- Problem tritt in dieser Form nicht auf | Kein anderer OSS-Player hat dokumentierte Hallucination-Detection | **USP** (technisch; Nutzer-sichtbarer Benefit: "keine seltsamen Textfragmente") |
| **Waveform-Animation (beide Plattformen)** | 5-Balken Echtzeit-Waveform in Windows Pill + Android Bubble, phasenversetzt | Waveform in Bubble vorhanden | Voice Type: keine Waveform; Amical: unklar; OpenWhispr: unklar | **Standard** (Wispr Flow hat es auch) |
| **Cost Tracking + Wispr Flow Savings** | Trackt STT+LLM-Kosten pro Diktat, zeigt Ersparnis ggue. Wispr Flow $12/mo | Kein Cost Tracking (Fixabo, Kosten nicht sichtbar) | Amical: kein Cost Tracking; Voice Type: kein Cost Tracking | **USP** (einziges Tool mit explizitem "Was habe ich gespart"-Dashboard) |
| **Filler-Word-Analyse** | Analysiert Raw-Transkripte auf haeufigste Fuelwoerter, Diagramm | Filler-Entfernung, aber kein Analyse-Dashboard | Kein anderer hat Filler-Word-Analyse als Feature | **USP** (Nischen-USP: interessant fuer Presenter/Coaches, nicht fuer alle) |
| **Cross-Device Sync (Turso)** | Bidirektionaler Sync ueber Turso LibSQL, fire-and-forget nach jedem Diktat | Cloud-native, Sync implizit (kein explizites Sync-Konzept noetig) | Amical: kein Sync; Voice Type: kein Sync; OpenWhispr: Notes-Sync vorhanden | **Staerke** (vs. OSS-Wettbewerber; vs. Wispr Flow kein Vorteil da cloud-native) |
| **Webhook Integration** | POST JSON-Payload an konfigurierbare URL nach jedem Diktat, Custom Headers | Kein Webhook, keine public API | OpenWhispr: kein Webhook; Amical: kein Webhook | **USP** (Power-User/Automator-Nische: Zapier-Workflows, n8n, eigene Backends) |
| **Whisper Mode (Gain-Amplification)** | Verstaerkt leises Mikrofon-Input per konfiguriertem Gain-Faktor | Whisper Mode vorhanden (erkennt Fluester-Sprache, andere Implementierung) | Voice Type: kein Whisper Mode; Amical: unklar | **Standard** (Name identisch, Implementierung unterschiedlich) |
| **Voice Notes** | Diktate als persistente Notizen speichern statt Paste | Kein explizites Voice Notes System gefunden | OpenWhispr: Notes-System vorhanden | **Staerke** (OpenWhispr hat es auch, Wispr Flow nicht) |
| **Text Snippets (Windows)** | Benannte Text-Snippets die per Diktat oder Klick eingefuegt werden | Snippets vorhanden (Voice-Shortcuts fuer Phrasen) | Voice Type: kein Snippets; Amical: unklar; OpenWhispr: kein Snippets | **Standard** (Wispr Flow hat es auch, aber Dikta hat es ohne Abo) |
| **Offline-Modus-Erkennung** | Erkennt automatisch wenn lokaler STT-Provider an erster Stelle steht, skippt LLM | Nicht relevant (kein Offline) | Amical: unklar; OpenWhispr: unklar | **USP** (technisch; macht den Offline-Workflow nahtlos) |
| **Insert-and-Send** | Drueckt Enter nach Paste (per Hotkey-Slot konfigurierbar) | Nicht explizit dokumentiert | Kein anderer hat das explizit als konfigurierbare Option | **Staerke** (kleines Feature, hoher Daily-Driver-Wert fuer Chat-User) |
| **Output-Sprache / Translation** | Instruiert LLM beim Cleanup in Zielsprache auszugeben (Live-Uebersetzung) | Multilingual (100+ Sprachen, automatische Erkennung), keine explizite Translation-Instruktion | OpenWhispr: 100+ Sprachen; Amical: 100+ Sprachen | **Staerke** (andere erkennen Sprache, Dikta kann aktiv uebersetzen) |
| **Advanced Power-User-Settings** | STT-Temp, LLM-Temp, Max-Tokens, Chunk-Threshold, Silence-Threshold, Paste-Delay -- alle konfigurierbar | Keine vergleichbaren Konfigurationsmoeglichkeiten (Black Box) | Amical: einige Parameter; OpenWhispr: einige Parameter | **Staerke** (vs. Wispr Flow USP; Power-User liebt das, Mainstream-User sieht es nicht) |
| **Einmalkauf-Preis EUR 29** | Einmalkauf, kein Abo | $12/mo ($144/Jahr) Abo | Voice Type: $19.99 einmalig; Amical/OpenWhispr: gratis | **USP** (Preis-Positionierung ist strategischer USP, kein Feature im engeren Sinne) |

---

### Top USPs fuer README

Die 8 Features die am staerksten differenzieren -- in Reihenfolge nach Kunden-Impact:

**1. Einmalkauf EUR 29 -- kein Abo**
Das staerkste Verkaufsargument gegen Wispr Flow ($144/Jahr). Zwei Saetze in der README, kein weiterer Kontext noetig: "Pay once. Own it forever."

**2. Cost Tracking mit Wispr Flow Savings**
Einziges Voice-Diktat-Tool das aktiv ausrechnet was du durch den Wechsel sparst. Ein Dashboard das "Du hast diesen Monat EUR 11.80 gespart vs. Wispr Flow" anzeigt ist ein kaufentscheidender Differenzierer -- zeigt den ROI buchstaeblich auf.

**3. Offline / Lokales Whisper mit GPU-Beschleunigung**
Kein Cloud-Zwang, keine Datenweitergabe, funktioniert ohne Internet. Kaufgrund fuer Privacy-Nutzer (Aerzte, Anwaelte, Journalisten) und Netzwerk-unabhaengige Workflows. Wispr Flow kann das nicht.

**4. Dual Hotkey Slots mit unabhaengigen Modi**
Kein anderer Wettbewerber hat zwei vollstaendig unabhaengige Diktat-Hotkeys. Slot 0 fuer Hold-Modus in Slack, Slot 1 fuer AutoStop in einem Dokument -- gleichzeitig aktiv, kein Hin-und-Her-Konfigurieren.

**5. Post-Diktat Reformate (Email / Bullets / Summary)**
Einmaliges Feature: Nach jedem Diktat erscheinen One-Click-Buttons um den Text direkt umzustrukturieren. Kein anderer Wettbewerber hat diesen Workflow. Stark fuer Wissensarbeiter die dasselbe Diktat in verschiedene Formate exportieren.

**6. BYOK fuer STT + LLM (volle Provider-Wahl)**
Bringt deinen eigenen Groq-, DeepSeek-, OpenAI- oder OpenRouter-Key. Kein Proxy, keine Marge, kein Vendor Lock-in. Wispr Flow hat das nicht (proprietaeres Modell, keine Key-Wahl). Starkes Argument fuer Tech-affine Zielgruppe.

**7. Webhook Integration**
Jedes Diktat-Ergebnis kann an eine eigene URL gepostet werden -- JSON-Payload, Custom Headers. Kein anderer Konkurrent bietet das. Oeffnet Dikta fuer Automations-Workflows (n8n, Zapier, eigene Backends) ohne zusaetzliche Infrastruktur.

**8. Android Floating Bubble mit per-Geste-Konfiguration**
Als einzige Open-Source-Alternative mit shipped Android-Support (Amical ist Beta, Wispr Flow ist Abo-only) bietet Dikta zusaetzlich: Tap und Long-Press sind unabhaengig konfigurierbar mit eigenem Modus und eigener Silence-Dauer. Wispr Flow hat nur einen gemeinsamen Modus pro Interaktionstyp.

---

## Discrepancies Found

~~1. Cleanup Styles: Fixed 2026-03-20. All three styles (Polished/Verbatim/Chat) are now Free in code.~~
~~2. Offline models: Fixed 2026-03-20. small+medium are Free, large-v3 is Paid in code.~~

All discrepancies resolved.
