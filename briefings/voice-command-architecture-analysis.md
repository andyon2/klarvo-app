# Voice Command Architecture — Analyse (2026-03-21)

## Kontext

Voice Command Mode funktioniert fuer Modus-Start ("Klarvo toggle" wird erkannt und dispatcht).
Aber: Waehrend eines laufenden Diktats koennen keine Sprachbefehle erkannt werden.

## Root Cause: Monitor-Pause waehrend Recording

In `audio/mod.rs` Zeile 279-284 wird der Voice Command Monitor pausiert sobald
ein Recording startet:

```rust
// Pause the monitor so both streams don't fight over the same samples.
if let Ok(mon) = self.monitor_session.lock() {
    if let Some(ref session) = *mon {
        session.paused.store(true, Ordering::Relaxed);
    }
}
```

Der Monitor resumed erst nach `stop_recording_with_gain`. Waehrend des Diktats
ist der Voice Command Engine blind.

## Auswirkungen

| Befehl | Wann | Funktioniert? |
|--------|------|---------------|
| Klarvo toggle/auto-stop/full auto | Kein Recording aktiv | Ja |
| Klarvo stop/cancel | Recording laeuft | Nein (Monitor pausiert) |
| Klarvo off/aus | Kein Recording aktiv | Ja |
| Klarvo off/aus | Recording laeuft | Nein (Monitor pausiert) |

## Bug 2: AutoStop-Stille greift nicht

`start_autostop_recording` installiert einen Silence-Callback vor dem Recording.
Muss geprueft werden ob:
- Config-Werte korrekt sind (autostop_silence_secs, silence_threshold)
- Der Callback tatsaechlich feuert
- `stop_and_process_pipeline` im Callback-Kontext funktioniert

## Bug 4: Lifecycle beim Toggling

Wenn ein voice-command-getriggertes Recording laeuft und der User Voice Command
Mode per UI ausschaltet:
- `toggle_voice_command_mode` stoppt den Monitor, aber NICHT das aktive Recording
- Naechstes Einschalten: Neuer Monitor startet, altes Recording laeuft noch
- State wird inkonsistent

Fix: `toggle_voice_command_mode` muss auch aktive Recordings stoppen/canceln
bevor der Monitor gestoppt wird.

## Architektur-Optionen fuer Befehle waehrend Diktat

### Option A: Pragmatisch (aktueller Plan)
Voice Commands = Modus-Auswahl + Start. Stoppen nur per Hotkey/UI/Stille.
Kein Umbau noetig. AutoStop ist der natuerlichste Modus fuer Voice Command User.

### Option B: Single-Stream (aufwendig)
Ein Audio-Stream speist GLEICHZEITIG Recording-Buffer UND Voice Command Engine.
Kein Pausieren. Aber: Engine hoert das Diktat mit → False Positives moeglich
("Das Meeting ist halt..." → "halt" = StopDictation).
Mitigation: Strengeres Trigger-Matching (nur exakter Match am Satzende),
oder: Befehle muessen ein Delay nach dem Trigger-Wort haben.

### Option C: Zwei Mikrofone / Zwei Streams (hardware-abhaengig)
Monitor und Recording auf separaten Devices. Nicht realistisch fuer die meisten User.

## Referenz: Wie machen es andere?

Windows Voice Access und Windows Speech Recognition loesen das auf OS-Ebene:
- Exklusiver Zugriff auf Audio-Pipeline
- Grammar-basierte Command-Erkennung parallel zu Free-Form Dictation
- Der Recognizer selbst unterscheidet zwischen Command und Diktat
- Nicht mit User-Space cpal + externer API (Groq) vergleichbar

## Entscheidung

Offen. Andy will Option B nicht ausschliessen, bevor er versteht wie Windows-Apps
das technisch loesen.
