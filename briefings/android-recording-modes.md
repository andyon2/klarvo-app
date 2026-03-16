# Briefing: Android Recording-Modi (Tasks 2-4)

## Kontext

Die Windows-Seite hat ein Dual-Hotkey-System mit 4 Recording-Modi (Hold, Toggle, AutoStop, Auto). Android soll dieselben Modi für die Floating Bubble bekommen.

**Task 1 (Config) ist bereits erledigt:** `bubble_recording_mode` existiert in `AppConfig` (Rust) und `DiktaApi.Config` (Kotlin). Default: `"hold"`.

## Was zu tun ist

### Task 2: State-Machine-Erweiterung in DiktaOverlayService

**Datei:** `android/kotlin-src/com/dikta/voice/DiktaOverlayService.kt`

Neues `RecordingMode`-Enum (privat in der Klasse):
```kotlin
private enum class RecordingMode { HOLD, TOGGLE, AUTOSTOP, AUTO }
```

Aktiven Modus aus Config laden (beim Service-Start und nach jedem IDLE-Return):
```kotlin
private var recordingMode = RecordingMode.HOLD

private fun loadRecordingMode() {
    val config = DiktaApi.readConfig(this)
    recordingMode = when (config.bubbleRecordingMode) {
        "toggle" -> RecordingMode.TOGGLE
        "autostop" -> RecordingMode.AUTOSTOP
        "auto" -> RecordingMode.AUTO
        else -> RecordingMode.HOLD
    }
}
```

Touch-Handling anpassen (`handleTap()` / `handleTouch()`):

| Modus | Single-Tap (IDLE) | Single-Tap (RECORDING) | Long-Press (IDLE) |
|-------|-------------------|------------------------|-------------------|
| HOLD | Bar expand (bestehend) | Stop + Process | → PTT (bestehend) |
| TOGGLE | Start Recording (roter Kreis) | Stop + Process | → Mode-Picker |
| AUTOSTOP | Start Recording | Stop + Process (manuell) | → Mode-Picker |
| AUTO | Start Loop | Stop Loop + Process | → Mode-Picker |

**Wichtig:** In HOLD-Modus bleibt Long-Press = PTT (bestehendes Verhalten). In allen anderen Modi öffnet Long-Press den Mode-Picker (Task 3).

### Task 3: Mode-Picker-Overlay

**Dateien:** `DiktaOverlayService.kt`, `FloatingBubbleView.kt`

Long-Press im IDLE-State (nicht-HOLD-Modi) zeigt ein Overlay mit 4 Buttons:
- Hold / Toggle / Auto Stop / Auto
- Auswahl → `config.json` updaten, Modus setzen, Badge aktualisieren
- Dismiss bei Tap außerhalb

**Mode-Badge** im Bubble-IDLE-Kreis: Kleiner Buchstabe unten rechts (H/T/A/L).

Config-Write in Kotlin:
```kotlin
private fun saveRecordingMode(mode: RecordingMode) {
    val configFile = File(dataDir, "config.json")
    val json = if (configFile.exists()) JSONObject(configFile.readText()) else JSONObject()
    json.put("bubble_recording_mode", mode.name.lowercase())
    val tmp = File(dataDir, "config.json.tmp")
    tmp.writeText(json.toString(2))
    tmp.renameTo(configFile) // atomic auf den meisten Filesystems
}
```

### Task 4: AutoStop-Silence-Detection

**Dateien:** `DiktaOverlayService.kt`, `DiktaAudioRecorder.kt`

`DiktaAudioRecorder` hat bereits Amplitude-Callbacks. Erweitern um:
```kotlin
var onSilenceDetected: (() -> Unit)? = null
private var silentChunks = 0
private val silenceThreshold = 0.03f
private val requiredSilentChunks = 30 // ~2s bei 15Hz Chunks
```

In der Amplitude-Berechnung: Wenn Amplitude < threshold für N Chunks → Callback feuern.

In `DiktaOverlayService`:
- AUTOSTOP: Bei Silence → `stopAndProcessRecording()`
- AUTO: Bei Silence → `stopAndProcessRecording()`, dann `startRecording()`
- Sicherheitsnetz für AUTO: Max 10 Loops oder 5 Minuten

## Referenz: Windows-Implementierung

Die Desktop-Pendants stehen in:
- `src-tauri/src/pipeline.rs` — `register_hotkey()` (Zeile ~1115), State Machine
- `src-tauri/src/audio/mod.rs` — Silence Detection mit RMS-Threshold
- `src-tauri/src/config/mod.rs` — `HotkeyMode` Enum, `silence_threshold` Config

## Testplan

- [ ] HOLD-Regression: Tap → Bar, Long-Press → PTT (identisch zu jetzt)
- [ ] TOGGLE: Tap → roter Kreis, Tap → Stop+Process
- [ ] AUTOSTOP: Tap → Aufnahme, 2s Stille → Auto-Stop
- [ ] AUTO: Tap → Loop, Tap → Loop-Stop
- [ ] Mode-Picker: Long-Press → Overlay, Auswahl → Badge + Config
- [ ] Persistenz: Modus überlebt App-Neustart
- [ ] Edge: Long-Press während RECORDING → kein Picker

## Start

```bash
scripts/dikta android
```
Dann dieses Briefing lesen und mit Task 2 anfangen.
