# Feature-Plan: Voice Command Mode

## User Story

Als Voxlit-Nutzer möchte ich Diktat vollständig hands-free steuern, damit ich meinen Computer
tippen lassen kann ohne die Hände zu bewegen — kein Hotkey nötig, einfach "Voxlit dictate"
sagen.

## Scope

Nur Desktop/Windows. Paid Feature (Command Mode ist bereits hinter License-Gate laut
`architecture.md` Abschnitt "Paid/Free Feature-Gates").

## Betroffene Module

- `src-tauri/src/audio/mod.rs`: Neuer Monitor-Modus (Mic dauerhaft offen, kein WAV-Sammeln)
- `src-tauri/src/voice_command/mod.rs`: Neues Modul — Command-Erkennung + Dispatch
- `src-tauri/src/config/mod.rs`: Neues Feld `voice_command_mode_enabled: bool`
- `src-tauri/src/lib.rs`: AppState-Feld `voice_command_monitor_active: AtomicBool`
- `src-tauri/src/pipeline.rs`: Start/Stop-Funktion für Voice Command Monitor
- `src-tauri/src/commands/settings.rs` oder `commands/misc.rs`: Toggle-Command für Frontend
- `src/FloatingBar.tsx` oder `src/App.tsx`: UI-Toggle (Tray-Menü + visueller Indicator)
- `src/components/SettingsPanel.tsx`: Settings-Toggle mit Paid-Gate

## Abhängigkeit: VAD-Overhaul (Task 4 + 6)

Task 4 (Hallucination-Blocklist) ist **Pflicht** vor diesem Plan — Phantom-Texte könnten
sonst Phantom-Commands auslösen. Task 6 (Manueller Test) kann parallel zur VCM-Implementierung
laufen.

---

## Tasks (in Reihenfolge)

### Task 1: Monitor-Modus in AudioRecorder

- **Agent:** rust-core
- **Dateien:** `src-tauri/src/audio/mod.rs`
- **Abhängigkeit:** keine (VAD-Overhaul Task 4 parallel)
- **Beschreibung:** Neuer Modus neben dem bestehenden Recording-Modus. `start_monitor(callback: MonitorCallback)` öffnet das Mic mit cpal, schreibt Samples **nicht** in einen WAV-Buffer, sondern ruft den Callback mit jedem PCM-Chunk auf (f32, 16 kHz). `stop_monitor()` schließt den Stream. Separate `monitor_session: Mutex<Option<MonitorSession>>` damit Monitor und normales Recording nicht kollidieren. Wenn normales Recording startet während Monitor aktiv, Monitor pausieren (samples verwerfen), nach Recording wieder aufnehmen. Nur `#[cfg(desktop)]`.

### Task 2: Neues Modul `voice_command`

- **Agent:** rust-core
- **Dateien:** `src-tauri/src/voice_command/mod.rs` (neu), `src-tauri/src/lib.rs` (mod-Deklaration)
- **Abhängigkeit:** Task 1 (Monitor-Callback), `vad/mod.rs` existiert bereits
- **Beschreibung:** Kapselt die gesamte Voice-Command-Erkennung. Öffentliches Interface: `VoiceCommandEngine::new()`, `VoiceCommandEngine::feed(pcm_chunk: &[f32]) -> Option<VoiceCommand>`. Intern: eigene `SileroVad`-Instanz (getrennt von der Recording-VAD), VAD-Trigger → PCM-Snippet akkumulieren (~1,5 Sek nach Onset), Snippet an whisper.cpp (local provider, small model) übergeben, Rohtext per `recognize_command(text: &str) -> Option<VoiceCommand>` matchen. Kein Groq/Cloud — ausschließlich lokal, kein API-Key-Risiko. Snippet wird **nicht** als WAV gespeichert, direkt als `Vec<f32>` an whisper-rs übergeben (API prüfen). `VoiceCommand` enum: `StartDictation`, `StopDictation`, `CancelDictation`, `SetStyle(CleanupStyle)`, `TurnOff`.

### Task 3: Fuzzy Keyword Matching in `voice_command`

- **Agent:** rust-core
- **Dateien:** `src-tauri/src/voice_command/mod.rs`
- **Abhängigkeit:** Task 2
- **Beschreibung:** `recognize_command(text: &str) -> Option<VoiceCommand>` implementieren. Strategie: Text lowercasen + trimmen. Trigger-Check: enthält der Text "voxlit" oder eine phonetische Variante? Varianten-Liste: `["voxlit", "vox lit", "foxlit", "fox lit", "foxy", "box lit", "woxlit"]`. Wenn kein Trigger → `None`. Wenn Trigger gefunden → Command-Keywords suchen: "start" | "dictate" | "diktat" → `StartDictation`, "stop" | "stopp" → `StopDictation`, "cancel" | "abbrechen" → `CancelDictation`, "polished" | "poliert" → `SetStyle(Polished)`, "verbatim" | "wörtlich" → `SetStyle(Verbatim)`, "chat" → `SetStyle(Chat)`, "off" | "aus" | "beenden" → `TurnOff`. Kein Keyword erkannt nach Trigger → `None` (Fehlauslösung ignorieren). Unit-Tests für jede Variante + Fuzzy-Cases.

### Task 4: Command-Dispatch in `pipeline.rs`

- **Agent:** rust-core
- **Dateien:** `src-tauri/src/pipeline.rs`, `src-tauri/src/lib.rs`
- **Abhängigkeit:** Task 2 + 3
- **Beschreibung:** Zwei neue Funktionen: `start_voice_command_monitor(handle: AppHandle)` und `stop_voice_command_monitor(handle: AppHandle)`. `start_voice_command_monitor` startet den Audio-Monitor-Modus (Task 1) mit einem Closure der `VoiceCommandEngine::feed()` aufruft. Wenn `feed()` ein `VoiceCommand` zurückgibt, Command-Handler aufrufen: `StartDictation` → `start_recording_for_hotkey_slot(&handle, 0)` (bestehende Funktion), `StopDictation` → `stop_and_process_for_slot(&handle, 0)`, `CancelDictation` → `cancel_recording()`, `SetStyle(s)` → Config schreiben + AppState-Provider neu aufbauen, `TurnOff` → `stop_voice_command_monitor()` + Config `voice_command_mode_enabled = false` + Event an Frontend. AppState bekommt `voice_command_monitor_active: AtomicBool`. Guard: Command nur dispatchen wenn kein Command in den letzten 2 Sekunden (Debounce, vermeidet Doppel-Trigger durch Nachhall).

### Task 5: Config + AppState-Erweiterung

- **Agent:** rust-core
- **Dateien:** `src-tauri/src/config/mod.rs`, `src-tauri/src/lib.rs`
- **Abhängigkeit:** Task 4
- **Beschreibung:** `AppConfig` bekommt `voice_command_mode_enabled: bool` (default: `false`). `SettingsView` bekommt dasselbe Feld. `AppState` bekommt `voice_command_monitor_active: AtomicBool`. In `lib.rs` `run()`: beim App-Start, falls `config.voice_command_mode_enabled == true`, `start_voice_command_monitor()` aufrufen (User hatte Modus aktiv bei letztem Beenden). Tray-Menü: neuen `MenuItem` "Voice Command: Ein/Aus" hinzufügen neben den bestehenden Tray-Items, Klick togglet den Monitor und speichert die Einstellung.

### Task 6: Tauri-Command für Frontend-Toggle

- **Agent:** rust-core
- **Dateien:** `src-tauri/src/commands/misc.rs` (oder neues `commands/voice_command.rs`), `src-tauri/src/commands/mod.rs`
- **Abhängigkeit:** Task 4 + 5
- **Beschreibung:** `#[tauri::command] async fn toggle_voice_command_mode(app: AppHandle) -> Result<bool, String>` — schaltet den Monitor ein/aus, persistiert in Config, gibt den neuen Boolean-State zurück. `#[tauri::command] fn get_voice_command_mode_active(state: State<AppState>) -> bool` — lesender Zugriff für das Frontend beim Laden. Beide Commands in `invoke_handler` registrieren.

### Task 7: LLM-Prompt-Erweiterung für Punctuation-Commands

- **Agent:** rust-core
- **Dateien:** `src-tauri/src/llm/mod.rs`
- **Abhängigkeit:** keine (unabhängig)
- **Beschreibung:** Alle drei Cleanup-Style-Prompts (Polished, Verbatim, Chat) um einen Abschnitt erweitern: "Wenn der Text gesprochene Satzzeichen enthält ('Punkt', 'Komma', 'Ausrufezeichen', 'Neuer Absatz', 'period', 'comma', 'new paragraph'), ersetze diese durch das entsprechende Satzzeichen." Achtung: Kotlin-Prompts in `android/kotlin-src/VoxlitApi.kt` synchron halten (Rule 2 aus CLAUDE.md: nach Änderung an Rust-Prompts immer auch Kotlin updaten, dann `/sync-prompts` aufrufen).

### Task 8: Frontend UI-Toggle

- **Agent:** ui-dev
- **Dateien:** `src/components/SettingsPanel.tsx`, `src/FloatingBar.tsx` (optional: Status-Indicator)
- **Abhängigkeit:** Task 6
- **Beschreibung:** Im SettingsPanel unter dem bestehenden Hotkey-Bereich: neuer Toggle "Voice Command Mode" mit Paid-Gate (wie Command Mode bereits). Toggle ruft `toggle_voice_command_mode()` auf, zeigt aktuellen State via `get_voice_command_mode_active()`. Warnung anzeigen wenn kein lokales Whisper-Modell installiert ist (Voice Command Mode braucht Local Provider). FloatingBar: visueller Indicator wenn Monitor aktiv ist — z.B. kleines Mic-Icon oder Puls-Animation im Idle-State. Kein eigener neuer State nötig, reicht als optischer Hinweis über ein Boolean-Prop.

---

## Testplan

- [ ] Unit-Tests `voice_command::recognize_command`: alle Commands + alle Trigger-Varianten + Nicht-Commands
- [ ] Unit-Test Debounce: zwei `VoiceCommand` in < 2 Sek → nur erster wird dispatched
- [ ] Unit-Test Monitor-Pause: `start_recording()` während Monitor aktiv → kein Sample-Leak in WAV
- [ ] Integration: Monitor starten → "Voxlit dictate" sagen → Recording startet → "Voxlit stop" → Pipeline läuft durch → Text eingefügt
- [ ] Edge Case: Stille nach Monitor-Start → kein Trigger (Hallucination-Blocklist greift)
- [ ] Edge Case: TV-Audio im Hintergrund → kein false positive (Hangover + Energie-Gate)
- [ ] Edge Case: "Voxlit off" → Monitor stoppt, Config gespeichert, nach Neustart kein Auto-Monitor
- [ ] Edge Case: Voice Command Mode ohne installiertes lokales Modell → sinnvolle Fehlermeldung im Frontend

## Risiken

- **Wake-Word-Erkennung:** "Voxlit" wird von Whisper als "Foxy", "Foxlit", "Box lit" etc. transkribiert. Mitigation: Varianten-Liste in Task 3 (erweiterbar). Andy sollte nach ersten Tests weitere Fehltranskriptionen zurückmelden.
- **Fehlauslösungen durch Umgebungsgeräusche:** TV, Gespräche triggern VAD → Whisper hört Phantomwort. Mitigation: Hallucination-Blocklist (Task 4 VAD-Overhaul), Debounce (Task 4 dieses Plans), Energie-Gate bereits im VAD.
- **Batterie-Drain:** Mic dauerhaft offen. Mitigation für späteren Schritt: GPU/Akku-Detection via `SYSTEM_POWER_STATUS` API (wie schon für lokales Whisper implementiert) — Monitor nur aktiv wenn am Strom. Für MVP: User-Entscheidung per Toggle.
- **Latenz Command-Erkennung:** whisper.cpp braucht 200-500ms für 1,5-Sek-Snippet. Mitigation: Snippet erst nach VAD-Hangover senden (Sprache abgeschlossen), dann Erkennung — subjektiv akzeptabel.
- **Monitor-Recording-Konflikt:** Wenn User Hotkey benutzt während Monitor lauscht → beide cpal-Streams gleichzeitig. Mitigation: Monitor in Task 1 explizit pausieren beim normalen Recording-Start.
- **whisper-rs API für Vec\<f32\>:** Prüfen ob `full()` oder `full_with_state()` direkt PCM-Samples akzeptiert ohne WAV-Schritt. Falls nicht: kleinen In-Memory WAV-Writer nutzen (wie in bestehendem audio/mod.rs).

## Technische Notizen

- `VoiceCommandEngine` braucht eigene `SileroVad`-Instanz, **nicht** die Recording-VAD teilen — unterschiedliche Konfigs (Command-VAD: niedrigerer Hangover ~300ms, weil Commands kurz sind).
- Lokales whisper.cpp ist Pflicht für Voice Command Mode. Wenn kein Modell installiert → Fehlermeldung und Toggle deaktivieren.
- Command-Erkennung läuft auf dem Monitor-Thread (nicht async), whisper.cpp-Aufruf ist synchron blocking — `spawn_blocking` in Tokio nötig wenn Dispatch async.
- Snippet-Akkumulation: Nach VAD-Onset akkumulieren bis VAD-Offset + Hangover (d.h. Sprache fertig). Max-Länge: 3 Sekunden (Commands sind kurz). Längere Aufnahme → verwerfen (kein Command, zu lang).
