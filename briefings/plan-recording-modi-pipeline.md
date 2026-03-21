# Feature-Plan: Recording-Modi Pipeline-Integration

## User Story

Als Voxlit-Nutzer möchte ich mit AutoStop/Auto-Modi sprechen und Voxlit hört automatisch auf zu
aufzunehmen, wenn ich eine Pause mache — damit ich die Hände frei behalte und kein Hotkey mehr
zum Stoppen nötig ist. Mit Insert+Send kann ich direkt in Chat-Feldern diktieren, ohne
nachher Enter drücken zu müssen.

## Betroffene Module

- `src-tauri/src/pipeline.rs`: Neue Hotkey-Handler für AutoStop und Auto. Insert+Send nach Paste.
- `src-tauri/src/commands/settings.rs`: `save_settings` um 3 neue Parameter erweitern. `get_settings` (SettingsView) entsprechend.
- `src-tauri/src/lib.rs`: SettingsView um `insertAndSend`, `autostopSilenceSecs`, `autoModeSilenceSecs` erweitern.
- `src/components/SettingsPanel.tsx`: Alle 4 Modi anzeigen, Silence-Slider, Insert+Send Toggle.

## Architektur-Entscheidungen (vor Umsetzung)

### Silence-Callback-Ansatz (bereits implementiert)
`AudioRecorder::set_silence_callback()` existiert bereits und ist genau das richtige Werkzeug:
- Wird VOR `start_recording()` installiert
- Recording-Thread überwacht RMS-Chunks (~66ms) auf dem cpal-Thread
- Bei Stille → `callback()` wird einmalig aufgerufen
- Callback wird als `Box<dyn Fn() + Send + 'static>` übergeben → kein Block des Recording-Threads

**Wichtig:** Der Callback läuft auf dem Recording-OS-Thread (cpal-Kontext), nicht im async-Runtime.
Für `stop_and_process_pipeline` brauchen wir `tauri::async_runtime::spawn` aus dem Callback heraus.
Das `AppHandle` ist `Clone` und `Send` → einfach clonen und in den spawn mitgeben.

### Auto-Mode Loop-Kontrolle
Auto-Mode braucht eine Loop-Zustandsvariable in `AppState`. Optionen:
- **AtomicBool in AppState** → einfachste Lösung, keine Lock-Contention
- Nach `stop_and_process_pipeline` prüfen: `if auto_loop_active → restart`

Ein zweiter Hotkey-Press im Auto-Mode muss die Loop beenden. Da `run_dictation_pipeline`
(Toggle-Handler) bereits `is_recording()` prüft, kann `AutoStop/Auto` denselben Einstiegspunkt
teilen mit einem `auto_loop_active`-Flag-Check.

---

## Tasks (in Reihenfolge)

### Task 1: Insert+Send nach Paste in pipeline.rs
- **Agent:** rust-core
- **Dateien:** `src-tauri/src/pipeline.rs`
- **Abhängigkeit:** keine
- **Beschreibung:** Direkt nach dem `paste_handler.paste()` Call in `stop_and_process_pipeline`
  (aktuell Zeile ~685): Config lesen (`config.insert_and_send`). Wenn `true` → Enter-Taste senden.
  Auf Windows via `SendInput` (VK_RETURN), plattform-abstrahiert hinter `#[cfg(target_os = "windows")]`.
  Kein eigenes Modul nötig — inline in pipeline.rs, nach dem Paste-Block, vor dem History-Save.
  Test: `test_insert_and_send_flag_is_read_from_config` — prüft dass der Config-Wert korrekt
  ausgelesen wird (Unit-Test ohne AppHandle, wie bestehende Pipeline-Tests).

### Task 2: `save_settings` und `SettingsView` erweitern
- **Agent:** rust-core
- **Dateien:** `src-tauri/src/commands/settings.rs`, `src-tauri/src/lib.rs`
- **Abhängigkeit:** keine (parallel zu Task 1)
- **Beschreibung:** Drei neue Parameter zu `save_settings` hinzufügen: `insert_and_send: Option<bool>`,
  `autostop_silence_secs: Option<f32>`, `auto_mode_silence_secs: Option<f32>`. Im `AppConfig`-Builder
  mit `unwrap_or(existing.X)` verarbeiten (identisches Muster wie `whisper_mode`, `autostart` etc.).
  `SettingsView` in `lib.rs` um dieselben drei Felder erweitern und in `get_settings()` befüllen.
  Tests: `test_save_settings_persists_recording_mode_fields` — rundet die drei Felder durch save/load.

### Task 3: AutoStop Hotkey-Handler in pipeline.rs
- **Agent:** rust-core
- **Dateien:** `src-tauri/src/pipeline.rs`
- **Abhängigkeit:** Task 1 (Pipeline-Logik muss klar sein), aber keine Code-Abhängigkeit
- **Beschreibung:** Neue async-Funktion `start_autostop_recording(handle: AppHandle)`:
  1. `start_recording_only(handle.clone()).await`
  2. Silence-Schwelle und Dauer aus Config lesen (`autostop_silence_secs`, `advanced.silence_threshold`)
  3. `state.recorder.set_silence_callback(secs, threshold, callback)` installieren, wobei `callback`
     einen `handle.clone()` captured und `tauri::async_runtime::spawn(stop_and_process_pipeline(h))`
     aufruft
  Im `register_hotkey` match-Block: `(HotkeyMode::AutoStop, ShortcutState::Pressed)` →
  `start_autostop_recording(h)`. `(HotkeyMode::AutoStop, ShortcutState::Released)` → kein op.
  Außerdem: ein zweiter Press während laufendem Recording (d.h. `is_recording() == true`) soll
  normal stoppen → in der Pressed-Branch: wenn `is_recording()` → `stop_and_process_pipeline`.
  Test: `test_autostop_handler_starts_silence_monitor` — prüft dass nach `start_autostop_recording`
  ein Silence-Callback installiert ist (via `recorder.silence_config.is_some()`).

### Task 4: Auto-Mode Loop-Infrastruktur in AppState
- **Agent:** rust-core
- **Dateien:** `src-tauri/src/lib.rs`, `src-tauri/src/pipeline.rs`
- **Abhängigkeit:** Task 3 (versteht das AutoStop-Muster)
- **Beschreibung:** `AppState` um `auto_loop_active: std::sync::atomic::AtomicBool` erweitern
  (initialisiert mit `false`). Neue async-Funktion `start_auto_recording(handle: AppHandle)`:
  identisch wie `start_autostop_recording`, aber der Silence-Callback setzt nach der Pipeline
  automatisch neu an:
  ```
  callback: Box::new(move || {
      tauri::async_runtime::spawn(async move {
          stop_and_process_pipeline(h.clone()).await;
          // Nach Processing: wenn auto_loop_active noch true → restart
          if state.auto_loop_active.load(Ordering::Relaxed) {
              start_auto_recording(h).await;
          }
      });
  })
  ```
  Im `register_hotkey` match:
  - `(HotkeyMode::Auto, ShortcutState::Pressed)` + `!is_recording()` →
    `auto_loop_active.store(true)`, dann `start_auto_recording(h)`
  - `(HotkeyMode::Auto, ShortcutState::Pressed)` + `is_recording()` →
    `auto_loop_active.store(false)`, dann `stop_and_process_pipeline(h)` (ohne Re-Start)
  Test: `test_auto_loop_flag_default_false` und `test_auto_loop_can_be_stopped`.

### Task 5: Frontend SettingsPanel — Alle 4 Modi + neue Settings
- **Agent:** ui-dev
- **Dateien:** `src/components/SettingsPanel.tsx`
- **Abhängigkeit:** Task 2 (SettingsView muss neue Felder haben)
- **Beschreibung:** Drei Änderungen im SettingsPanel:

  **5a — 4 Modi:** Den hartcodierten `["hold", "toggle"]`-Array (Zeile ~975) durch alle 4
  `HotkeyMode`-Varianten ersetzen: `["hold", "toggle", "autostop", "auto"]`. Labels/Tooltips:
  - hold: "Hold to record, release to process"
  - toggle: "Press to start, press again to stop"
  - autostop: "Press to start, stops automatically on silence"
  - auto: "Continuous — restarts after each silence gap"
  Die Beschreibungszeile darunter (Zeile ~993) entsprechend für alle 4 Modi.

  **5b — Silence-Slider:** Wenn `localHotkeyMode === "autostop" || "auto"` → Slider anzeigen.
  State: `localSilenceSecs` (number, 0.5–5.0, Schritt 0.5, Default aus `loadedSettings`).
  Für AutoStop aus `autostopSilenceSecs`, für Auto aus `autoModeSilenceSecs`.
  Ins `onSave`-Call einbinden.

  **5c — Insert+Send Toggle:** Neuer Toggle (Checkbox oder Switch) in der Hotkey-Sektion oder
  als eigener Abschnitt "Behaviour": Label "Insert & Send", Subtext "Send Enter after pasting
  (useful for chat apps)". State: `localInsertAndSend: boolean`.

  Alle drei neuen States in die Dirty-Check-Logik und in den `onSave`-Call einbinden.
  `onSave`-Signatur in den Props um 3 neue Parameter erweitern (oder als eigenes Objekt — mit
  rust-core absprechen wie die Frontend-zu-Backend-Übergabe aussieht).

### Task 6: App.tsx — onSave-Handler anpassen
- **Agent:** ui-dev
- **Dateien:** `src/App.tsx`
- **Abhängigkeit:** Task 2 (Backend-Command kennt neue Parameter), Task 5 (SettingsPanel sendet sie)
- **Beschreibung:** Im App.tsx-`handleSave`-Callback (der `save_settings` aufruft):
  die drei neuen Parameter `insertAndSend`, `autostopSilenceSecs`, `autoModeSilenceSecs` aus dem
  SettingsPanel entgegennehmen und an den `save_settings` Tauri-Command weiterleiten.
  Außerdem: `get_settings` Response (`SettingsView`) auf die neuen Felder mappen und an SettingsPanel
  als `loadedSettings`-Props weiterreichen.

---

## Implementierungs-Reihenfolge (empfohlen)

```
Task 1 + Task 2  →  Task 3  →  Task 4  →  Task 5 + Task 6
(parallel)           (sequentiell nach 1+2)
```

Tasks 1 und 2 haben keine Code-Abhängigkeiten untereinander und können parallel bearbeitet werden.
Task 3 und 4 bauen auf dem Muster aus Task 1 auf (Paste-Block verstehen), sind aber technisch
unabhängig. Tasks 5 und 6 brauchen die fertigen Signaturen aus Task 2.

---

## Testplan

- [ ] **AutoStop manuell:** Hotkey auf AutoStop stellen → Hotkey drücken → sprechen → aufhören
  zu sprechen → nach ~2 Sekunden muss Transkription automatisch starten
- [ ] **AutoStop abbrechen:** Während AutoStop-Recording → nochmal Hotkey drücken → Recording
  stoppt sofort und verarbeitet (kein Loop, kein Re-Start)
- [ ] **Auto-Loop:** Auto-Modus aktivieren → sprechen → Pause → Transkription kommt, danach
  wird sofort neu aufgenommen → zweiter Hotkey-Press → Loop endet nach aktuellem Diktat
- [ ] **Silence-Threshold Edge Case:** Sehr ruhige Umgebung → AutoStop darf nicht sofort nach
  1 Chunk feuern (Mindest-Recording-Zeit aus `min_recording_ms` beachten)
- [ ] **Insert+Send:** In einem Chat-Feld diktieren mit Insert+Send=true → Enter kommt nach
  dem Paste automatisch
- [ ] **Insert+Send off:** In Word/Notepad diktieren mit Insert+Send=false → kein Enter
- [ ] **Settings Roundtrip:** Neue Werte im UI einstellen → speichern → Settings neu öffnen →
  Werte müssen erhalten geblieben sein
- [ ] **Hold/Toggle unverändert:** Bestehende Modi müssen exakt wie zuvor funktionieren
- [ ] **Config-Migration:** Altes `config.json` ohne neue Felder → App startet, Defaults greifen
  (`insert_and_send=false`, Silence=2.0s)

---

## Risiken

**1. Silence-Callback feuert zu früh (Hintergrundgeräusche)**
Risiko: Threshold-Default zu hoch für laute Umgebungen.
Minderung: Slider in UI, guter Default (bestehender `advanced.silence_threshold`). Der
AutoStop-Handler sollte denselben `silence_threshold`-Wert aus AdvancedSettings nutzen — nicht
einen eigenen hardcodierten Wert.

**2. Auto-Loop Race Condition**
Risiko: `stop_and_process_pipeline` ist noch nicht fertig, wenn `start_auto_recording` den
nächsten Aufnahme-Zyklus startet.
Minderung: `start_auto_recording` ruft zuerst `start_recording_only` auf, das bei
`is_recording() == true` sofort returniert → kein Doppelstart möglich. Der Pipeline-Abschluss
(emit `done`) passiert, bevor der Restart-Spawn sich in der Callback-Chain durchläuft.

**3. Insert+Send in falschen Kontexten**
Risiko: In einer IDE oder Terminal wird Enter als "Befehl ausführen" interpretiert.
Minderung: Feature ist opt-in (`false` per Default). Nutzer müssen bewusst aktivieren.
Langfristig: Terminal-Detection (analog bestehende `capture_foreground_window_title`) als
mögliche Erweiterung im Backlog.

**4. AtomicBool vs. Mutex für auto_loop_active**
Risiko: Race zwischen "stop pressed" und "silence fired".
Minderung: AtomicBool mit `SeqCst` reicht — wir brauchen nur "ist Loop noch erwünscht" als
binäres Signal. Der worst case ist, dass ein einzelner Zyklus nach dem Stop-Press noch
abgeschlossen wird — das ist akzeptabel (kein Daten-Verlust, kein Crash).
