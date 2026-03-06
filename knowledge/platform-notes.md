# Plattform-Notizen -- Dikta

Hier werden plattformspezifische Quirks, Workarounds und Lessons Learned dokumentiert.

## Windows

### Text-Paste via SendInput
- Windows nutzt `SendInput` API fuer Ctrl+V Simulation
- Reihenfolge: Text in Clipboard -> SendInput(Ctrl Down, V Down, V Up, Ctrl Up)
- **Terminal-Erkennung noetig:** PowerShell, Windows Terminal, Git Bash nutzen Ctrl+Shift+V
- OpenWhispr hat einen nativen C-Binary dafuer (`windows-fast-paste`) -- als Referenz nutzen
- Alternative: `clipboard` Crate + `enigo` Crate fuer Keyboard-Simulation

### Globaler Hotkey
- `RegisterHotKey` Win32 API in einem separaten Thread
- Braucht ein unsichtbares Window fuer die Message-Loop
- Alternative: `global-hotkey` Crate (cross-platform, nutzt Tauri intern auch)

### GPU-Erkennung (Akku-Modus)
- `windows::System::Power::PowerManager` fuer Akku-Status
- Oder einfacher: `SYSTEM_POWER_STATUS` via Win32 API

## Android

### InputMethodService (IME)
- Der IME ist ein Service, kein Activity -- hat eigenen Lifecycle
- `onCreateInputView()` erstellt die Keyboard-UI
- `currentInputConnection.commitText(text, 1)` fuegt Text ein
- User muss Dikta manuell als System-Keyboard aktivieren (Settings -> Language & Input)
- Seit Android 11: `InputMethodManager.showInputMethodPicker()` kann Wechsel-Dialog zeigen

### Permissions
- `RECORD_AUDIO`: Muss zur Laufzeit angefragt werden (Android 6+)
- `POST_NOTIFICATIONS`: Muss zur Laufzeit angefragt werden (Android 13+)
- `FOREGROUND_SERVICE`: Manifest-Declaration reicht
- `FOREGROUND_SERVICE_MICROPHONE`: Noetig ab Android 14 fuer Mikrofon in Foreground Services

### Battery / Background
- Android killt Background-Services aggressiv (besonders Samsung, Xiaomi)
- ForegroundService mit Notification waehrend Aufnahme ist Pflicht
- Kein permanenter Service wenn nicht aufgenommen wird
- Doze-Mode: Kein Netzwerk moeglich -> STT/LLM-Calls scheitern -> Fallback benoetigt oder Doze-Whitelist

### Tauri v2 auf Android
- TODO: Recherchieren wie Tauri-Plugins aus nativem Kotlin-Code aufgerufen werden
- TODO: Testen ob Tauri-Android + IME zusammen funktioniert (IME ist ein Service, Tauri erwartet Activity)
- Das ist ein bekanntes Risiko -- moeglicherweise muss der IME komplett in Kotlin sein und nur die API-Calls machen

## Beide Plattformen

### Audio-Formate
- Groq akzeptiert: mp3, wav, webm, m4a (max 25MB)
- whisper.cpp erwartet: 16kHz 16-bit mono WAV
- -> Audio immer als 16kHz mono WAV aufnehmen (funktioniert fuer beide)
- Fuer Groq: WAV direkt schicken (unter 25MB bei normaler Diktat-Laenge)

### Latenz-Budget
- Ziel: <2s von Sprach-Ende bis Text erscheint
- Audio-Stop: ~50ms
- Upload zu Groq: ~200-500ms (abhaengig von Dateigroesse/Verbindung)
- Groq STT: ~500-1000ms (sehr schnell)
- DeepSeek Cleanup: ~500-1500ms
- Paste: ~50ms
- **Gesamt: ~1.3-3s** -- akzeptabel, aber eng. Optimierungspotenzial: Streaming, paralleles Processing
