# Feature-Plan: VAD-basierte Silence Detection (Silero VAD v5)

## Übersicht

Ersetze die RMS-basierte Silence Detection durch Silero VAD v5 (ONNX-basiert).
Löst: Musik-Bleed, Bürolärm, Whisper-Halluzinationen.

## 6 Tasks in Reihenfolge

### Task 1: Characterization Tests (rust-core) ✅
Schreibe Tests die das aktuelle RMS-Verhalten festhalten (Golden Master).
Dateien: `src-tauri/src/audio/mod.rs`, `src-tauri/src/pipeline.rs`

### Task 2: Neues vad/ Modul bauen (rust-core) ✅
Isoliertes Modul: Silero VAD + Ring Buffer + 85Hz Highpass + Hysteresis State Machine.
Dateien: `src-tauri/src/vad/mod.rs` (neu), `Cargo.toml`
Crate: `voice_activity_detector = "0.2.1"` (nur Desktop)

### Task 3: Pipeline umschalten (rust-core) ✅
In pipeline.rs/audio/mod.rs: Alten RMS-Check durch SileroVad::feed() ersetzen.
Nur für AutoStop/Auto Modi. Audio-Level-Emitter bleibt.

### Task 4: Hallucination-Blocklist (rust-core) ⏳ OFFEN
Statische Liste bekannter Whisper-Phantomtexte. Nur kurze Texte (<=5 Worte) blocken.
Datei: `src-tauri/src/pipeline.rs`
**Wichtig fuer Voice Command Mode:** Phantom-Texte koennten als Commands fehlinterpretiert werden.

### Task 5: Android Integration (android-platform) ✅
android-vad:silero:2.0.10 einbinden, RMS-Loop in VoxlitAudioRecorder.kt ersetzen.
Gleiche Hysteresis-Logik wie Desktop.

### Task 6: Manueller Test (Andy) ⏳ OFFEN
Szenarien: Stille, Musik-Bleed, Bürolärm, kein Sprechen, Blocklist.
Idealerweise zusammen mit Voice Command Mode testen.

## Parameter (Defaults)

| Parameter | Wert |
|-----------|------|
| Onset threshold | 0.5 |
| Offset threshold | 0.35 |
| Hangover | ~608ms |
| Highpass cutoff | 85Hz |
| Energy floor | RMS < 0.001 |
| Silero frame size | 512 samples (32ms) |

## Risiken

- ONNX Runtime vergrößert Binary um ~1-2MB
- voice_activity_detector Thread-Safety prüfen (ggf. Mutex)
- android-vad braucht JitPack als Maven-Repo
- Hysteresis-Parameter brauchen evtl. Feintuning durch Andy
