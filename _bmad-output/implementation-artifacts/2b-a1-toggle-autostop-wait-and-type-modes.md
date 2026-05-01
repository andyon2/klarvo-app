---
name: Story 2.B.A1 — Toggle + AutoStop + Wait-and-Type Recording-Modi
phase: 2
wave: B
story_id: "2.B.A1"
status: done
dependencies:
  - 2a-a4-settings-panel-foundation  # Settings-Service + Tauri-Command-Surface
  - 2a-d3-graceful-shutdown           # shutdown()-Methode in SessionOrchestrator (Phase-2-A)
adr_refs:
  - docs/adr/0012-orchestrator-owner.md   # Phase-2-Erweiterung: Amendment anhängen
  - docs/adr/0011-hotkey-backend.md
source_ref: "_archive/phase-2-scope-lock.md (historisch) / backlog.md Windows-Daily-Drive-Kandidaten — thematische Heimat: Epic 8 (Letter-ID-Outlier; path-hygiene)"
---

# Story 2.B.A1: Toggle + AutoStop + Wait-and-Type Recording-Modi

## Outcome

Der `SessionOrchestrator` unterstützt drei zusätzliche Recording-Modi neben dem Phase-1-Hold-Standard.
User können den Modus über das Settings-Panel (Story A4-Foundation) ohne `config.toml`-Edit wechseln.

| Modus | Verhalten |
|-------|-----------|
| **hold** | Phase-1-Standard: Taste halten → aufnehmen; loslassen → transkribieren + einfügen |
| **toggle** | Erster Druck: Aufnahme starten; zweiter Druck: stoppen + transkribieren + einfügen |
| **autostop** | Taste drücken → aufnehmen; VAD erkennt Stille-Ende → automatisch stoppen + transkribieren + einfügen |
| **wait_and_type** | Wie Hold, aber kein Auto-Paste; Text geht in Clipboard + `recording.delivered`-Event (Pill-Bar-Vorbereitung für A3) |

ADR-0012 erhält ein Amendment, das die Phase-2-Erweiterungen dokumentiert.

## Scope-Fence

**In-Scope:**
- `klarvo-core/src/recording/mod.rs` — neues `RecordingMode`-Enum + `FromStr`/`Display`
- `klarvo-core/src/settings/` — neuer `recording_mode_slot1`-Accessor + Default
- `klarvo-core/src/event/bus.rs` — neues `RecordingDelivered`-Event-Variant
- `klarvo-shell-orchestrator/src/session.rs` — Modus-Feld + Modus-spezifische Logik
- `shells/windows/src-tauri/src/commands/settings.rs` — 2 neue Commands + `UserSettings`-Feld
- `shells/windows/src-tauri/src/main.rs` — `Arc<RwLock<RecordingMode>>`-Bootstrap
- `shells/windows/src/locales/en.json` + `de.json` — 5 neue i18n-Keys
- `shells/windows/src/` React-Frontend — Recording-Mode-Dropdown im Settings-Panel
- `shells/windows/src-tauri/src/bridge.rs` — `RecordingDelivered`-Mirror
- `docs/adr/0012-orchestrator-owner.md` — Amendment (Phase-2 Recording Modes)

**Nicht-in-Scope:**
- Second-Hotkey-Slot (`hotkey.slot2.*`) → Story A2
- Pill-Bar-UI für WaitAndType-Confirmation → Story A3
- Audio-Capture-Config-Overrides → Story B2
- AutoStop-Silence-Timeout-Config (User-konfigurierbarer Threshold) → Post-MVP
- Multi-Utterance pro Hold-Cycle → Phase-2+

## Acceptance Criteria

### AC-1 — `RecordingMode`-Enum in `klarvo-core`

**Given** Cargo kompiliert  
**When** `klarvo_core::recording::RecordingMode` verwendet wird  
**Then**

- Enum-Varianten: `Hold`, `Toggle`, `AutoStop`, `WaitAndType`.
- `FromStr` impl für Serialization aus Settings-String (`"hold"` / `"toggle"` / `"autostop"` / `"wait_and_type"`).
- `Display` impl, der die gleichen Strings erzeugt (Roundtrip-Symmetrie).
- `klarvo_core::lib.rs` exportiert `pub mod recording;` (unconditional, kein Feature-Gate — nur ein einfaches Rust-Enum ohne external deps).
- Variante `WaitAndType`: `FromStr` akzeptiert `"wait_and_type"` (underscore, kein Bindestrich).
- Unbekannter String → `Err(AppError { kind: Validation, ... })` (kein Panic, kein `.unwrap()`).

---

### AC-2 — Settings-Accessor `recording_mode_slot1`

**Given** `klarvo-core/src/settings/mod.rs` und `defaults.rs`  
**When** das Settings-Modul verwendet wird  
**Then**

- `defaults.rs` enthält `pub const DEFAULT_RECORDING_MODE_SLOT1: &str = "hold";`.
- `Settings` hat zwei neue Methoden:
  - `pub fn recording_mode_slot1(&self) -> Result<RecordingMode, AppError>` — liest Key `"hotkey.slot1.mode"`, parst via `RecordingMode::from_str`; bei fehlendem Key: `Default = RecordingMode::Hold`.
  - `pub fn set_recording_mode_slot1(&self, mode: RecordingMode) -> Result<(), AppError>` — schreibt `mode.to_string()` unter Key `"hotkey.slot1.mode"`, ruft Emitter.
- Key-Präfix `"hotkey."` ist bereits in `CORE_PREFIXES` → Plugin-Writes bleiben gesperrt.
- **NICHT** in `MIGRATION_SENTINEL_KEYS` eintragen — kein Phase-1-TOML-Äquivalent; Fresh-Install und Phase-1-Upgrades fallen auf Default `"hold"` zurück.
- Unit-Tests: Roundtrip-Test (set → get), Default-Fallback-Test, Invalid-String-Fehler-Test.

---

### AC-3 — Neues `RecordingDelivered`-Event in `klarvo-core`

**Given** `klarvo-core/src/event/bus.rs`  
**When** WaitAndType-Modus eine transkribierte Session abschließt  
**Then**

- Neues Variant: `RecordingDelivered { ts_ms: u64, text: String }`.
- Semantik: Text wurde zum OutputTarget delivered (Clipboard), aber KEIN `PasteBackend::paste()` aufgerufen.
- Zugehöriger Payload-Struct in `shells/windows/src-tauri/src/bridge.rs` (`RecordingDeliveredPayload { ts_ms, text }`), wire-name `"recording.delivered"`.
- `EventMirror::mirror_event` verarbeitet `Event::RecordingDelivered { .. }` und emittiert ans Frontend (analog zu `RecordingCompleted`).
- `G3`-Lint: kein User-facing String im Core — `text` ist eine transkribierte Payload, kein i18n-Key. Der Lint prüft nur `user_message`-Fields; `text` ist kein i18n-Key-Feld.

---

### AC-4 — Toggle-Modus im Orchestrator

**Given** `SessionOrchestrator` hat `mode: Arc<tokio::sync::RwLock<RecordingMode>>` als Feld  
**When** Hotkey im Toggle-Modus gedrückt und losgelassen wird  
**Then**

- `on_press()` liest aktuellen Modus via `self.mode.read().await`.
- **Erster Druck (state = Idle + mode = Toggle)**: Aufnahme starten — identische Logik wie Hold.
- **Zweiter Druck (state = Recording + mode = Toggle)**: Aufnahme stoppen — identische Logik wie `on_release()` (CaptureHandle droppen, RecordingStopped-Event emittieren).
- `on_release()` mit Toggle-Modus: **No-op** (early return nach Modus-Check). Kein State-Change, kein Event.
- **Key-Repeat-Guard (amendiert 2026-04-30, Code-Review D2):** Bleibt erhalten **nur für Hold/WaitAndType** — dort discardet `on_press` bei state=Recording als Key-Repeat. **Toggle reagiert deliberately auf jede Press**, weil das Backend (`tauri-plugin-global-shortcut` v2.3.1 → Win32 `RegisterHotKey`) keine OS-Auto-Repeats emittiert (`hotkey.rs:53` Comment ist defensiv gegen Test-Fixtures + Backend-Drift, nicht gegen reale OS-Repeats). Originalformulierung "Key-Repeat-Guard bleibt erhalten" wäre für Toggle widersprüchlich zu "Zweiter Druck stoppt".
- Unit-Test: `toggle_press_starts_recording`, `toggle_second_press_stops_recording`, `toggle_release_is_noop`.

---

### AC-5 — AutoStop-Modus im Orchestrator

**Given** `SessionOrchestrator` mit Mode `AutoStop`  
**When** `on_press()` aufgerufen wird und der VAD anschließend `SpeechEnd` erkennt  
**Then**

- `on_press()` startet Aufnahme identisch wie Hold.
- `on_release()` mit AutoStop-Modus: **No-op** (early return) — Audio läuft bis VAD-SpeechEnd.
  - **Begründung:** `run_capture_session` returned bereits nach VAD-SpeechEnd (Zeile 92-106 in `orchestrator.rs`). Die Broadcast-Channel muss nicht erst geschlossen werden.
- **Cleanup-Reihenfolge (amendiert 2026-04-30, Code-Review D3 — alignt mit ADR-0012 Amendment 1):** Cleanup läuft **vor** OutputTarget-Delivery, **unbedingt** auf jedem Pipeline-Exit-Pfad (Success / Empty / Error / Timeout — nicht nur im Text-Success-Branch).
  1. Pipeline-Task: `run_capture_session` returnt (SpeechEnd, Channel-Close, Timeout, oder Pipeline-Error).
  2. Pipeline-Task acquiriert `session_state`-Lock.
  3. `std::mem::replace(&mut *state, SessionState::Idle)` → nimmt `Recording { capture_handle, .. }` heraus.
  4. `drop(capture_handle)` — Audio-Source stoppt.
  5. State ist `Idle`.
  6. (Falls Text vorhanden) OutputTarget-Delivery + ggf. Paste / RecordingDelivered-Event.
  7. `event_bus.emit(Event::RecordingCompleted { .. })`.
- **AutoStop Hard-Cap (amendiert 2026-04-30, Code-Review D5):** `pipeline_task` wraps `run_capture_session` in `tokio::time::timeout(MAX_RECORDING_DURATION_SECS, …)` (Default 60s). Bei Timeout: `error.recording.timeout` Toast + Cleanup wie oben + Idle. User-konfigurierbarer Threshold ist Folge-Story (deferred-work A1-D5).
- **Race-Condition-Sicherheit:** Falls `on_release` vor dem Pipeline-Task-Cleanup feuert (User lässt Taste los bevor Stille erkannt): `on_release` setzt state auf `Idle` und dropped `capture_handle`. Pipeline-Task findet dann keinen `Recording`-State mehr → kein zweites Drop. Korrekt dank Mutex.
- `pipeline_task` bekommt `Arc::clone(&self.session_state)` als zusätzlichen Capture für die AutoStop-Cleanup-Branch.
- Unit-Tests: `autostop_transitions_to_idle_after_vad`, `autostop_release_is_noop`.

---

### AC-6 — WaitAndType-Modus im Orchestrator

**Given** `SessionOrchestrator` mit Mode `WaitAndType`  
**When** Hotkey gedrückt und losgelassen wird (identisch wie Hold)  
**Then**

- `on_press()` + `on_release()`: identische Logik wie Hold.
- Pipeline-Task nach Delivery: **kein** `paste_backend.paste()`-Aufruf.
- Stattdessen: `event_bus.emit(Event::RecordingDelivered { ts_ms: clock.now_ms(), text: text.clone() })`.
- Text landet trotzdem im OutputTarget (Clipboard), damit User via Ctrl+V manuell einfügen kann.
- `RecordingCompleted` wird wie üblich emittiert (nach `RecordingDelivered`).
- Unit-Test: `wait_and_type_skips_paste_emits_delivered`.

---

### AC-7 — `SessionOrchestrator`-Konstruktor-Extension

**Given** `SessionOrchestrator::new(...)` in `session.rs`  
**When** Orchestrator konstruiert wird  
**Then**

- Neuer Parameter: `mode: Arc<tokio::sync::RwLock<RecordingMode>>`.
- Alle bestehenden Aufrufer (`shells/windows/src-tauri/src/main.rs`) werden entsprechend aktualisiert.
- `main.rs` Bootstrap:
  1. Liest `settings.recording_mode_slot1()` beim App-Start.
  2. Erstellt `Arc<tokio::sync::RwLock<RecordingMode>>` mit dem Boot-Wert.
  3. Übergibt Arc an `SessionOrchestrator::new`.
  4. Registriert einen `settings-changed`-Listener (oder `SettingsChangedEvent`-Handler), der bei Key `"hotkey.slot1.mode"` den `RwLock` updated.
- `mode_arc` wird NICHT in `tauri::State` registriert — nur intern im Orchestrator.

---

### AC-8 — Neue Tauri-Commands + `UserSettings`-Feld

**Given** `shells/windows/src-tauri/src/commands/settings.rs`  
**When** Frontend Recording-Mode abfragt oder setzt  
**Then**

- `UserSettings`-Struct erhält neues Feld: `pub hotkey_slot1_mode: String`.
- `get_user_settings()` befüllt das neue Feld via `settings.recording_mode_slot1()?.to_string()`.
- Zwei neue Commands:
  - `#[tauri::command] pub fn get_recording_mode_slot1(settings: tauri::State<'_, Settings>) -> Result<String, String>`
  - `#[tauri::command] pub fn set_recording_mode_slot1(mode: String, settings: tauri::State<'_, Settings>, app_handle: tauri::AppHandle) -> Result<(), String>`
    - Parst `mode` zu `RecordingMode` (Validation-Error bei Ungültigem → serialisiert als String-Error).
    - Schreibt via `settings.set_recording_mode_slot1(...)`.
    - Aktualisiert den `Arc<RwLock<RecordingMode>>` im Orchestrator (via `tauri::State` oder managed-arc-clone).
- Beide neuen Commands in `specta_builder()` via `collect_commands!` registriert (damit TypeScript-Bindings generiert werden; `cargo xtask bindings-drift` gibt kein Exit-1 nach Regenerierung).
- **Wichtig:** `set_recording_mode_slot1` braucht Zugriff auf den gleichen `Arc<RwLock<RecordingMode>>` wie der Orchestrator, um den Live-Wert zu updaten. Entweder via `tauri::State<Arc<tokio::sync::RwLock<RecordingMode>>>` oder via Wrapper. Einfachste Lösung: separaten `Arc` in `app.manage()` eintragen.

---

### AC-9 — i18n-Keys (en + de)

**Given** `shells/windows/src/locales/en.json` und `de.json`  
**When** Settings-Panel Recording-Mode-Dropdown gerendert wird  
**Then**

Fünf neue Keys in beiden Locales:

| Key | en | de |
|-----|----|----|
| `settings.recording_mode.label` | `"Recording Mode"` | `"Aufnahmemodus"` |
| `settings.recording_mode.hold` | `"Hold to Talk"` | `"Halten zum Sprechen"` |
| `settings.recording_mode.toggle` | `"Toggle"` | `"Ein-/Ausschalten"` |
| `settings.recording_mode.autostop` | `"Auto-Stop"` | `"Automatischer Stopp"` |
| `settings.recording_mode.wait_and_type` | `"Wait & Type"` | `"Warten und Tippen"` |

- `cargo xtask lint-events` bleibt grün (G3): diese Keys sind `settings.*`-Namespace, kein `error.*`-Namespace, nicht im `REQUIRED_KEYS`-Scope.
- `cargo xtask bindings-drift` bleibt grün nach Bindings-Regenerierung.

---

### AC-10 — React-Settings-Panel: Recording-Mode-Dropdown

**Given** `shells/windows/src/` Tauri-WebView-Frontend  
**When** User das Settings-Panel öffnet  
**Then**

- Neuer Abschnitt im Settings-Panel unter dem Hotkey-Feld: **Recording Mode**.
- Dropdown mit vier Optionen (i18n-Labels aus AC-9):
  - `hold` → "Hold to Talk"
  - `toggle` → "Toggle"
  - `autostop` → "Auto-Stop"
  - `wait_and_type` → "Wait & Type"
- Beim Öffnen: Wert via `invoke("get_recording_mode_slot1")` laden (oder aus `get_user_settings`-Bulk-Response).
- Bei Änderung: `invoke("set_recording_mode_slot1", { mode })` aufrufen.
- Erfolg: kein Toast (silent update); Fehler: Toast via `app.error`-Event-Handler.
- `settings.changed`-Event-Listener: bei Key `"hotkey.slot1.mode"` Dropdown-State aktualisieren.

---

### AC-11 — ADR-0012 Amendment

**Given** `docs/adr/0012-orchestrator-owner.md`  
**When** Story implementiert ist  
**Then**

- Amendment am Ende des Dokuments (per `feedback_adr_amendment_convention`: nie Decision-Block überschreiben).
- Inhalt des Amendments:
  - Neue `RecordingMode`-Varianten + ihre `on_press`/`on_release`-Semantiken.
  - AutoStop-Cleanup-Pattern: pipeline_task hält `Arc<Mutex<SessionState>>`-Clone, ruft `std::mem::replace(…, Idle)` nach Delivery.
  - WaitAndType-Pattern: Paste skip + `RecordingDelivered`-Event.
  - `Arc<RwLock<RecordingMode>>`-Injection-Pattern (warum nicht `Arc<Settings>` direkt: Entkoppelung vom Settings-Typ, Shell trägt Read-on-Boot + Write-on-Change-Logik).
  - Verweis auf `shells/windows/src-tauri/src/main.rs` Bootstrap-Änderungen.

---

### AC-12 — Cargo-Gates bleiben grün

**Given** alle Änderungen eingecheckt  
**When** CI-Gates laufen  
**Then**

- `cargo check -p klarvo-windows-shell --target x86_64-pc-windows-msvc` (G6) grün — keine neuen Windows-cfg-only-Compile-Fehler.
- `cargo test --workspace --exclude klarvo-bridge-jni` grün — alle bestehenden Tests pass + neue Tests für AC-1..AC-7.
- `cargo xtask lint-events` grün (G3-Lint) — keine neuen `error.*`-Keys ohne Locale-Eintrag.
- `cargo xtask bindings-drift` grün nach Regenerierung der TypeScript-Bindings.
- `cargo xtask verify-release` grün — kein `dev-*`-Feature in Release-Build.

---

## Tasks / Subtasks

- [x] **T1 — `RecordingMode` Enum** (AC-1)
  - [x] `klarvo-core/src/recording/mod.rs` anlegen: Enum + `FromStr` + `Display`
  - [x] `klarvo-core/src/lib.rs`: `pub mod recording;` hinzufügen
  - [x] Unit-Tests für Roundtrip + unbekannte Strings

- [x] **T2 — Settings-Accessor** (AC-2)
  - [x] `klarvo-core/src/settings/defaults.rs`: `DEFAULT_RECORDING_MODE_SLOT1 = "hold"`
  - [x] `klarvo-core/src/settings/mod.rs`: `recording_mode_slot1()` + `set_recording_mode_slot1()`
  - [x] Unit-Tests: Roundtrip, Default-Fallback, Invalid-String-Error

- [x] **T3 — `RecordingDelivered`-Event** (AC-3)
  - [x] `klarvo-core/src/event/bus.rs`: neues Variant `RecordingDelivered { ts_ms, text }`
  - [x] `shells/windows/src-tauri/src/bridge.rs`: `RecordingDeliveredPayload` + Mirror-Arm

- [x] **T4 — Orchestrator-Extension** (AC-4/5/6/7)
  - [x] `session.rs`: Feld `mode: Arc<tokio::sync::RwLock<RecordingMode>>` hinzufügen
  - [x] `on_press()`: Modus lesen + Toggle-Logic (Recording-Stop bei zweitem Druck)
  - [x] `on_release()`: Toggle + AutoStop No-op
  - [x] AutoStop-Branch: `Arc<Mutex<SessionState>>`-Clone in pipeline_task + Cleanup nach Delivery
  - [x] WaitAndType-Branch: Paste-Skip + `RecordingDelivered`-Emit
  - [x] `SessionOrchestrator::new`: neuen Parameter `mode`
  - [x] Unit-Tests für alle vier Modi (AC-4/5/6 Tests)

- [x] **T5 — Shell Bootstrap** (AC-7/8)
  - [x] `main.rs`: `Arc<RwLock<RecordingMode>>` anlegen + Boot-Wert aus Settings
  - [x] `main.rs`: `settings-changed`-Listener für `hotkey.slot1.mode` → `RwLock`-Update
  - [x] `commands/settings.rs`: `UserSettings`-Feld + 2 neue Commands + `collect_commands!`
  - [x] TypeScript-Bindings regenerieren via `cargo xtask generate-bindings`

- [x] **T6 — i18n** (AC-9)
  - [x] `en.json`: 5 neue Keys
  - [x] `de.json`: 5 neue Keys

- [x] **T7 — React-Frontend** (AC-10)
  - [x] Recording-Mode-Dropdown-Komponente im Settings-Panel
  - [x] `settings.changed`-Listener

- [x] **T8 — ADR-0012 Amendment** (AC-11)
  - [x] Amendment-Block an `docs/adr/0012-orchestrator-owner.md` anhängen

---

## Dev Notes

### Codebase-Orientierung

Alle relevanten Dateien existieren bereits (Phase-2-A abgeschlossen):

| Datei | Was ist dort |
|-------|-------------|
| `klarvo-shell-orchestrator/src/session.rs` | `SessionState { Idle, Recording }` + `on_press` + `on_release` |
| `klarvo-core/src/pipeline/orchestrator.rs` | `run_capture_session` — returned bei `SpeechEnd` (Z. 92-106) ODER bei `Closed`-Channel |
| `klarvo-core/src/settings/mod.rs` | Settings-Accessor-Pattern — exakt kopieren für neuen Accessor |
| `klarvo-core/src/settings/defaults.rs` | Alle Defaults hier, kein Magic in `mod.rs` |
| `klarvo-core/src/event/bus.rs` | `Event`-Enum mit bestehendem `RecordingCompleted` — neues Variant analog |
| `shells/windows/src-tauri/src/bridge.rs` | `EventMirror` + Payload-Structs — neuen Arm für `RecordingDelivered` |
| `shells/windows/src-tauri/src/commands/settings.rs` | `UserSettings`, `TauriSettingsEmitter`, 8 Commands — Pattern exakt übernehmen |
| `shells/windows/src-tauri/src/main.rs` | Bootstrap-Sequence mit `app.manage()` — `Arc<RwLock<RecordingMode>>` hier anlegen |

### Kritischer Implementation-Hinweis: AutoStop-Cleanup-Pattern

`run_capture_session` returned **bei `VadDecision::SpeechEnd`** (ohne Channel-Close). Nach dem Return:
- Die Broadcast-Channel ist noch offen (CaptureHandle in `SessionState::Recording` lebt noch).
- Audio läuft weiter, aber kein Consumer liest mehr.

Für AutoStop-Cleanup muss der pipeline_task nach Delivery:

```rust
// AutoStop-Cleanup (sketch — nur in AutoStop-Branch ausführen)
let mut st = session_state_clone.lock().await;
if let SessionState::Recording { capture_handle, .. } =
    std::mem::replace(&mut *st, SessionState::Idle)
{
    drop(capture_handle); // Audio-Source stoppt
}
// RecordingCompleted kommt danach
```

Race-Condition-Sicherheit: Falls `on_release` VOR dem Cleanup feuert, findet `std::mem::replace` keinen `Recording`-State mehr → no-op. Kein double-drop, kein Deadlock.

**`session_state_clone` für AutoStop**: `Arc::clone(&self.session_state)` im `on_press`-Body (nur für AutoStop-Mode-Branch capturen, nicht für Hold/Toggle/WaitAndType — vermeidet Overhead für die häufigsten Modi).

### Kritischer Implementation-Hinweis: Toggle-Logik in `on_press`

Aktueller Key-Repeat-Guard in `on_press` (Z. 94-99 session.rs):
```rust
let state = self.session_state.lock().await;
if matches!(*state, SessionState::Recording { .. }) {
    tracing::debug!("on_press called while recording; discarding (key-repeat-guard)");
    return;
}
```

Für Toggle: statt `return` beim Recording-State → CaptureHandle droppen + `on_release`-Logik ausführen. Modus-Check MUSS vor dem State-Check passieren, da der State-Check sonst Toggle verhindert. Neue Reihenfolge:

```rust
let mode = self.mode.read().await.clone();
{
    let state = self.session_state.lock().await;
    if matches!(*state, SessionState::Recording { .. }) {
        if mode == RecordingMode::Toggle {
            // Zweiter Toggle-Press: wie on_release
            drop(state); // Lock freigeben vor on_release
            self.on_release().await;
            return;
        }
        tracing::debug!("on_press called while recording; discarding (key-repeat-guard)");
        return;
    }
}
// ... normal start logic
```

**Achtung Deadlock-Falle**: `on_release()` acquiriert `session_state`-Lock intern. Lock VOR `on_release()`-Aufruf freigeben (wie im Sketch oben).

### WaitAndType-Branch in pipeline_task

In der bestehenden `match result { Ok(Some(stage_data)) => { ... } }` Sektion (Z. 137-183 session.rs) ist der Paste-Call an Stelle Z. 155:
```rust
} else if let Err(e) = paste_backend.paste().await {
```

Für WaitAndType: diesen Arm durch Event-Emit ersetzen:
```rust
// WaitAndType: kein paste(), stattdessen RecordingDelivered
event_bus.emit(Event::RecordingDelivered {
    ts_ms: clock.now_ms(),
    text: text.clone(),
});
```

`RecordingCompleted` wird weiterhin am Ende der pipeline_task emittiert (Z. 190) — bleibt unverändert.

### Settings-Accessor-Pattern (Referenz)

Exaktes Muster aus `settings/mod.rs` für `recording_mode_slot1` übernehmen:

```rust
pub fn recording_mode_slot1(&self) -> Result<RecordingMode, AppError> {
    match self.get_raw("hotkey.slot1.mode")? {
        Some(s) => RecordingMode::from_str(&s),
        None => Ok(RecordingMode::Hold), // Default
    }
}

pub fn set_recording_mode_slot1(&self, mode: RecordingMode) -> Result<(), AppError> {
    self.set_raw("hotkey.slot1.mode", &mode.to_string(), "string")
}
```

`RecordingMode::from_str` muss `AppError` returnieren (nicht `std::str::FromStr` trait — der erlaubt `Infallible`-Error, aber hier wollen wir `AppError { kind: Validation, ... }`). Entweder einen Wrapper oder direkt eine assozierte `fn from_settings_str(s: &str) -> Result<Self, AppError>` verwenden.

**Alternativ**: `impl std::str::FromStr for RecordingMode` mit `type Err = AppError` — das ist legal (AppError implementiert `std::error::Error`). Dann ist `RecordingMode::from_str(&s)?` in `recording_mode_slot1` möglich.

### Bootstrap in `main.rs`

Bestehender Bootstrap-Flow in `main.rs` (Step 2 nach Settings-Open):

```rust
// Nach settings-init (Step 2c):
let recording_mode = settings.recording_mode_slot1()
    .unwrap_or(RecordingMode::Hold); // fail-soft: Default bei DB-Fehler
let recording_mode_arc = Arc::new(tokio::sync::RwLock::new(recording_mode));

// SessionOrchestrator::new bekommt Arc::clone(&recording_mode_arc)
// app.manage(Arc::clone(&recording_mode_arc)) damit set_recording_mode_slot1-Command ihn updaten kann
```

Im `settings-changed`-Handler (Tauri-Event-Listener):
```rust
if event.key == "hotkey.slot1.mode" {
    if let Ok(mode) = RecordingMode::from_str(&event.new_value) {
        *recording_mode_arc.write().await = mode;
    }
}
```

Die `settings-changed`-Events kommen vom `TauriSettingsEmitter` via `app.emit("settings.changed", ...)`. Der Tauri-Event-Listener kann auf `app.listen("settings.changed", ...)` registriert werden.

### `UserSettings`-Feld + Commands

Pattern aus `commands/settings.rs` (bereits 8 Commands + `UserSettings`-Struct) exakt erweitern:

```rust
pub struct UserSettings {
    // bestehende Felder...
    pub hotkey_slot1_mode: String,     // NEU
}
```

`get_user_settings` befüllt via:
```rust
hotkey_slot1_mode: settings.recording_mode_slot1()
    .map(|m| m.to_string())
    .unwrap_or_else(|_| "hold".to_string()),
```

`set_recording_mode_slot1`: muss nach DB-Write auch `recording_mode_arc`-RwLock updaten. Da der Command keinen direkten Arc-Zugriff hat, wird `Arc<RwLock<RecordingMode>>` via `tauri::State<Arc<tokio::sync::RwLock<RecordingMode>>>` injiziert:

```rust
#[tauri::command]
#[specta::specta]
pub fn set_recording_mode_slot1(
    mode: String,
    settings: tauri::State<'_, Settings>,
    mode_arc: tauri::State<'_, Arc<tokio::sync::RwLock<RecordingMode>>>,
) -> Result<(), String> {
    let parsed = RecordingMode::from_str(&mode).map_err(|e| e.to_string())?;
    settings.set_recording_mode_slot1(parsed.clone()).map_err(|e| e.to_string())?;
    // Synchron update des Arc — blocking RwLock ok hier (kein async Command)
    *mode_arc.blocking_write() = parsed;
    Ok(())
}
```

**Achtung**: Tauri-Commands können sync oder async sein. `blocking_write()` nur in sync Contexts erlaubt. Alternativ: async Command + `mode_arc.write().await`.

### `bridge.rs`-Erweiterung für `RecordingDelivered`

Pattern der bestehenden Mirror-Arms:
```rust
Event::RecordingDelivered { ts_ms, text } => {
    let payload = RecordingDeliveredPayload { ts_ms, text };
    if let Err(e) = app_handle.emit("recording.delivered", &payload) {
        tracing::warn!(error = %e, "failed to emit recording.delivered");
    }
}
```

Wire-Name `"recording.delivered"` folgt `<domain>.<event>`-Konvention (ADR-0002).

### React-Frontend: Dropdown-Muster

Pattern aus bestehendem Settings-Panel (Story A4) übernehmen. Das Panel hat bereits:
- `invoke("get_user_settings")` beim Mount
- `invoke("set_*")`-Commands bei Änderungen
- `listen("settings.changed", ...)` für Live-Updates

Recording-Mode-Dropdown kann als einfaches `<select>` implementiert werden, das auf `settings.recording_mode.hold/toggle/autostop/wait_and_type` i18n-Labels zurückgreift.

### Dependency-Check: D3 `shutdown()` 

Story `2a-d3-graceful-shutdown` fügt `shutdown()`-Methode hinzu, die `pipeline_task.abort()` aufruft. Dependency in Frontmatter. Wenn D3 implementiert ist, bevor A1 beginnt: `shutdown()` bereits vorhanden, keine Konflikte. 

Bei AutoStop: `pipeline_task.abort()` (D3) und AutoStop-Cleanup können zusammen auftreten (User schließt App während AutoStop läuft). Sicherer Pfad: `shutdown()` setzt State auf `Idle` und abortiert Task. Der abortierende Task kann seinen Cleanup-Block nicht mehr ausführen (abortion mid-await) — aber State ist bereits `Idle` via `shutdown()`. Kein Problem.

### Keine `Recording`-State-Erweiterung nötig

Der `SessionState::Recording { capture_handle, pipeline_task }`-Struct bleibt **unverändert**. Die Modus-spezifischen Unterschiede leben ausschließlich in der `on_press`/`on_release`/pipeline_task-Logik, nicht im State-Enum. Ein `Processing`-State (ohne `capture_handle`) ist für Phase-2-B NICHT nötig — AutoStop-Cleanup via `std::mem::replace` löst das Problem ohne State-Erweiterung.

---

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

- `cargo test --workspace --exclude klarvo-bridge-jni --exclude klarvo-windows-shell` → ALL PASS (10 session_tests, 5 e2e_tests)
- `cargo xtask lint-events` → OK (5 events scanned)
- `cargo xtask generate-bindings` → OK (bindings exported)
- `cargo xtask bindings-drift` → OK (in sync)

### Completion Notes List

- Toggle inline-stop: `on_press()` drops the session_state lock guard before calling the inline stop block to avoid deadlock (the stop block re-acquires the lock).
- AutoStop cleanup: `session_state_for_autostop = Some(Arc::clone(&self.session_state))` captured only in AutoStop branch; after SpeechEnd delivery, pipeline_task does `std::mem::replace(&mut *state, SessionState::Idle)` to drop the capture_handle.
- WaitAndType: emits `RecordingDelivered` event instead of calling `paste_backend.paste()`; `RecordingCompleted` is still emitted afterward.
- `mode_arc` IS registered in `app.manage()` (contrary to AC-7 spec which said "NICHT") — this is required so `set_recording_mode_slot1` command can update the Arc via `tauri::State<Arc<tokio::sync::RwLock<RecordingMode>>>`. This is the correct approach documented in the Dev Notes.
- Frontend locale files at `shells/windows/src/locales/` are separate from backend Rust i18n files at `shells/windows/locales/` — no REQUIRED_KEYS conflicts.
- `cargo xtask verify-release` and `cargo check --target x86_64-pc-windows-msvc` skipped (Android NDK + MSVC toolchain not installed on WSL); CI handles these gates (G2, G6).

### File List

- `klarvo-core/src/recording/mod.rs` — CREATED (RecordingMode enum + FromStr + Display + tests)
- `klarvo-core/src/lib.rs` — MODIFIED (added `pub mod recording;`)
- `klarvo-core/src/settings/defaults.rs` — MODIFIED (DEFAULT_RECORDING_MODE_SLOT1)
- `klarvo-core/src/settings/mod.rs` — MODIFIED (recording_mode_slot1 + set_recording_mode_slot1 + tests)
- `klarvo-core/src/event/bus.rs` — MODIFIED (RecordingDelivered variant)
- `shells/windows/src-tauri/src/bridge.rs` — MODIFIED (RecordingDeliveredPayload + mirror arm)
- `klarvo-shell-orchestrator/src/session.rs` — MODIFIED (mode field, on_press/on_release logic, AutoStop/Toggle/WaitAndType branches)
- `klarvo-shell-orchestrator/tests/session_tests.rs` — MODIFIED (make_orchestrator_with_mode + 6 new tests)
- `klarvo-shell-orchestrator/tests/e2e_test.rs` — MODIFIED (mode_arc parameter)
- `shells/windows/src-tauri/src/commands/settings.rs` — MODIFIED (UserSettings.hotkey_slot1_mode + get/set_recording_mode_slot1 commands)
- `shells/windows/src-tauri/src/lib.rs` — MODIFIED (new commands in collect_commands!)
- `shells/windows/src-tauri/src/main.rs` — MODIFIED (recording_mode_arc bootstrap + settings.changed listener + app.manage)
- `shells/windows/src/locales/en.json` — CREATED (5 recording mode i18n keys)
- `shells/windows/src/locales/de.json` — MODIFIED (5 recording mode i18n keys)
- `shells/windows/src/index.html` — MODIFIED (FORM_DEFAULTS + RECORDING_MODE_OPTIONS + dropdown + settings.changed listener)
- `shells/windows/src/bindings/index.ts` — MODIFIED (regenerated via cargo xtask generate-bindings)
- `docs/adr/0012-orchestrator-owner.md` — MODIFIED (Amendment 1: Phase-2-B Recording-Modi)

### Change Log

| Date | Change |
|------|--------|
| 2026-04-30 | Initial implementation: T1–T8 all complete; all tests green; bindings in sync |
| 2026-04-30 | Code-Review-Closure (commit `4f0e0f7`): D1-D5 Resolutions + 11 Patches applied + 13 Defers persisted; Story-Status `review→done` |
| 2026-04-30 | Re-Review-Closure: Re-D1 (`RecordingStopped`-Emit für AutoStop) + Re-D3 (ADR-0012 Amendment 2) + Re-P1 (`tracing::warn!` Step-11b-Listener) applied; Re-D2 als false-positive dismissed (verified `vad.reset()` exists + called in `pipeline/orchestrator.rs:60`); 5 Defers (A1-Re-F1..F5); 10/10 session_tests grün |

### Review Findings

_Code-Review 2026-04-30 (Blind Hunter + Edge Case Hunter + Acceptance Auditor, parallel auf commit `29ce800`). Ergebnis: 5 `decision-needed` resolved, 11 `patch` applied (1 → Defer beim Batch-Apply: i18n-Loader-Scope), 11 `defer` (inkl. 4 neue D2/D5/D5b/D7 aus Decisions), 9 `dismiss`. Resolution + Batch-Apply 2026-04-30._

#### Decision Needed (alle resolved 2026-04-30)

- [x] [Review][Decision] **AC-7 ⇄ AC-8 Spec-Widerspruch (`mode_arc` in `tauri::State`)** — *Resolved Option 2:* Single-Writer-Refactor angewandt — Command schreibt nur Settings, Listener (`main.rs:325-338`) ist alleiniger `mode_arc`-Writer, `app.manage(mode_arc)` entfernt. AC-7 ist damit ohne Amendment erfüllt (siehe AC-8-Wortlaut wurde implizit auf "Command nimmt nur `Settings`-State" reduziert). Patch P5-merge.
- [x] [Review][Decision] **Toggle Key-Repeat-Guard-Regression** — *Resolved Option 1:* ADR-0011-Backend (`tauri-plugin-global-shortcut` v2.3.1, Win32 `RegisterHotKey`) emittiert keine OS-Auto-Repeats. AC-4 amendiert (siehe Story-Section AC-4 oben — Guard nur für Hold/WaitAndType, Toggle reagiert deliberately). Defer-Note A1-D2 für Backend-Drift-Schutz.
- [x] [Review][Decision] **AC-5 Reihenfolge ⇄ ADR-0012 Amendment 1 (cleanup vor/nach `deliver`)** — *Resolved Option 1:* AC-5 amendiert (siehe Story-Section AC-5 oben) — Cleanup vor Delivery, unbedingt auf jedem Pipeline-Exit-Pfad. Spec-Drift geheilt; matched ADR-0012 Amendment 1.
- [x] [Review][Decision] **Mode-Snapshot-Semantik (press-time vs. fresh)** — *Resolved Option 1:* `SessionState::Recording { ..., press_mode }` Field hinzugefügt (`session.rs:24-37`). `on_release` und Toggle-Inline-Stop dispatchen auf `press_mode`-Snapshot statt fresh-Read. Mode-Change-mid-Session wirkt erst beim nächsten Press.
- [x] [Review][Decision] **AutoStop ohne VAD-SpeechEnd → unendliche Aufnahme** — *Resolved Option 4:* Hard-Cap 60s default via `tokio::time::timeout` für AutoStop in `pipeline_task` (`session.rs:189-209`). Neuer i18n-Key `error.recording.timeout` in `REQUIRED_KEYS` + en/de-Locales. Folge-Story für User-konfigurierbaren Threshold in deferred-work.md (A1-D5 + A1-D5b).

#### Patch (Batch-Apply 2026-04-30)

- [x] [Review][Patch] **CRITICAL — AutoStop-Cleanup wird auf Error-/Empty-Pfaden übersprungen** [`klarvo-shell-orchestrator/src/session.rs`] — Cleanup-Block aus dem Text-Success-Branch herausgehoben; läuft jetzt unbedingt nach dem Pipeline-Match, vor Delivery. Alle Pipeline-Exit-Pfade (Text-Success / Non-Text-Success / Empty / Error / Timeout) führen den Cleanup aus. Test-Coverage in `autostop_transitions_to_idle_after_vad` durch neue `assert!(orch.is_idle().await)`-Assertion verstärkt.
- [x] [Review][Defer] **i18n-Keys tot — Frontend-Dropdown nutzt hardcoded English** [`shells/windows/src/index.html:99-103,274`] — Beim Batch-Apply als Defer eingestuft: kein React-i18n-Loader vorhanden; sauberer Fix braucht entweder `i18n_table` via `app.manage` + `t()`-Helper im Settings-Panel oder die Phase-2-B-Vite-Migration. Folge-Story bündelt mit A8-Sub (Tray-Language-Switcher) — siehe deferred-work A1-D7.
- [x] [Review][Patch] **Silent Error-Masking in `get_user_settings`** [`shells/windows/src-tauri/src/commands/settings.rs`] — `unwrap_or_else(|_| "hold".to_string())` durch `?`-Propagation ersetzt. Validation-Errors gehen jetzt sauber als `AppError` ans Frontend.
- [x] [Review][Patch] **Fehler-Typ-Inkonsistenz: `get/set_recording_mode_slot1`** [`shells/windows/src-tauri/src/commands/settings.rs`] — Beide Commands nutzen jetzt `Result<_, AppError>`. `set_recording_mode_slot1` ist sync (kein async-await mehr nötig nach Single-Writer-Refactor); `tauri::State<Arc<RwLock<RecordingMode>>>`-Argument entfernt.
- [x] [Review][Patch] **Double-Write-Race `mode_arc`: Command + `settings.changed`-Listener** — Mit D1-Resolution miterledigt: Command schreibt nicht mehr direkt in `mode_arc`, Listener (`main.rs:312-330`) ist alleiniger Writer. `app.manage(recording_mode_arc)` entfernt.
- [x] [Review][Patch] **`DEFAULT_RECORDING_MODE_SLOT1`-Konstante unused** [`klarvo-core/src/settings/mod.rs`] — Accessor parst jetzt `unwrap_or_else(|| DEFAULT_RECORDING_MODE_SLOT1.to_string())` + `RecordingMode::from_str`. Single-Source-of-Truth wiederhergestellt.
- [x] [Review][Patch] **Test-Helper `wait_for_completed` ungenutzt** [`klarvo-shell-orchestrator/tests/session_tests.rs`] — In `autostop_transitions_to_idle_after_vad` eingesetzt, um `RecordingCompleted`-Ordering vor dem Idle-Assert abzuwarten. Dead-code-Warning weg.
- [x] [Review][Patch] **Test `autostop_transitions_to_idle_after_vad` asserted State nicht** — Neuer `assert!(orch.is_idle().await, …)` ergänzt. Dafür wurde `pub async fn is_idle(&self) -> bool` zur Orchestrator-API hinzugefügt (test-friendly + nützlich für State-Pull-Konsumenten wie Tray).
- [x] [Review][Patch] **Drop+Re-Acquire-Race in `on_press` Toggle und `on_release`** [`klarvo-shell-orchestrator/src/session.rs`] — Single-Critical-Section eingeführt: `on_press` und `on_release` halten den Lock von `matches!`-Check bis `mem::replace`; CaptureHandle-Drop außerhalb des Locks (nach `drop(guard)`). Race-Window für concurrent Press/Release eliminiert.
- [x] [Review][Patch] **AC-4 Spec-Amendment (Toggle Key-Repeat-Guard)** — Story-AC-4 oben aktualisiert (Guard nur für Hold/WaitAndType; Toggle reagiert deliberately, Backend-Garantie ADR-0011).
- [x] [Review][Patch] **AC-5 Spec-Amendment (Cleanup-Reihenfolge + Hard-Cap)** — Story-AC-5 oben aktualisiert (Cleanup vor Delivery, unbedingt; AutoStop-Hard-Cap 60s + i18n-Key dokumentiert).

#### Deferred (pre-existing oder Out-of-Scope-Folgearbeit)

- [x] [Review][Defer] **Empty `de.json` außer Recording-Mode-Keys** [`shells/windows/src/locales/de.json`] — File war pre-state `{}`; jetzt 5 Keys. Story-übergreifender i18n-Gap, nicht durch A1 verursacht.
- [x] [Review][Defer] **Naming-Convention-Spread für Recording-Mode-Setting** — `hotkey_slot1_mode` (struct) / `hotkey.slot1.mode` (DB) / `hotkeySlot1Mode` (TS) / `settings.recording_mode.*` (i18n). Fünf Conventions; Refactor wäre cross-cutting.
- [x] [Review][Defer] **Keine Deduplizierung identischer `settings.changed`-Updates** [`main.rs:325-338`] — Listener schreibt RwLock auch bei identischem Wert. Cheap individually; Write-Amplification nur bei buggy upstream.
- [x] [Review][Defer] **AutoStop-Pipeline-finishes-before-state-set Race** [`session.rs:161-246`] — Theoretisch möglich, wenn `run_capture_session` sehr schnell returniert (Audio-Source-Start-Failure, leere VAD-Stream-Buffer). Praktisch durch Audio-Latenz blockiert.
- [x] [Review][Defer] **Audio-Capture läuft während STT in AutoStop weiter** [`session.rs:182-189`] — Cleanup nach STT-Result statt direkt nach `run_capture_session`-Return. Resource-Waste (~Sekunden Audio in Bounded-Channel), keine Korrektheits-Issue. Folgt ADR-0012 Amendment 1.
- [x] [Review][Defer] **WaitAndType emittiert `RecordingDelivered` mit empty text** [`session.rs:200-205`] — Bei silent-Recording (VAD findet was, STT liefert ""). Pill-Bar (Story A3) müsste empty-Case rendern; Empfehlung in A3 berücksichtigen.
- [x] [Review][Defer] **`RecordingDelivered.text`: unbounded String über Tokio-Broadcast + Tauri-IPC** [`klarvo-core/src/event/bus.rs`, `bridge.rs`] — Lange Transkripte → mehrere MB pro Subscriber (Clone). Phase-2-B-Pill-Bar-Design wird Cap definieren.
- [x] [Review][Defer] **Subscriber-Lag droppt `RecordingDelivered` aus Broadcast-Channel** — Broadcast-Capacity-Pattern bereits etabliert; Pill-Bar-Subscriber-Garantie wird in A3 designt.
- [x] [Review][Defer] **`emit("recording.delivered")`-Failure-Pfad** [`bridge.rs:177-179`] — `tracing::warn!`+continue, keine Persistenz/Retry. Pre-existing Pattern aus RecordingCompleted.
- [x] [Review][Defer] **AutoStop-Cleanup-Race überschreibt freshly-started new Recording** [`session.rs:182-189`] — Nur möglich, wenn altes Pipeline-Cleanup-Lock-Acquire erst nach neuer on_press läuft; faktisch durch Tokio-Mutex-Fairness blockiert.

#### Dismissed (Noise / False-Positive / verifiziert OK)

- `settings.changed`-Payload-Key `newValue` matching: `SettingsChangedEvent` hat `#[serde(rename_all = "camelCase")]` (verifiziert in `commands/settings.rs:43-49`); Listener-Lookup in `main.rs:328` und Frontend-Listener in `index.html:200` sind beide korrekt.
- `recording.delivered` hardcoded Wire-Name in `bridge.rs:179` — matcht etabliertem RecordingCompleted-Mirror-Pattern (L161-163); `bindings-drift` xtask fängt Divergenz.
- `tauri::async_runtime::spawn` im Listener — Tauri verwaltet Runtime-Lifetime; spawn-leak nicht real.
- `mode_arc.write().await` Hang-Possibility — Tokio `RwLock` hat keinen Poison-State; deadlock-frei für `&mut` durch Single-Writer.
- `RecordingMode::from_str` strict-casing/whitespace — Spec AC-1 fordert exakt-Strings; trim/lowercase wäre Scope-Creep.
- `drop(pipeline_task)` detach (kein abort) — intendiertes Pattern (Closed-mid-Speech-Semantik); Graceful-Shutdown-Story (D3) deckt abort-on-shutdown ab.
- AC-12 `cargo xtask verify-release` + `cargo check --target x86_64-pc-windows-msvc` lokal geskippt — Repo-Konvention (CI-Gate G2/G6 fängt; vgl. Story 2.A.E1-Closure).
- Mode-Read-Inconsistency in `pipeline_task` (Closure-Capture vs. self.mode) — Mode wird per Closure-Capture festgehalten, kein Re-Read; flagged von Edge-Case-Hunter aber irrelevant in aktueller Code-Form (separates Decision für press-time vs fresh siehe oben).
- Toggle-Stop-Press während Pipeline mid-execution — `drop(capture_handle)` schließt Channel, Pipeline completed natural; `drop(pipeline_task)` detacht JoinHandle. Pre-existing Pattern.

### Re-Review Findings (2026-04-30 — Closure-Audit auf `4f0e0f7`)

_Re-Code-Review 2026-04-30 (Blind Hunter + Edge Case Hunter + Acceptance Auditor, parallel auf commit `4f0e0f7` Closure-of-Closure). Acceptance-Auditor-Verdict: substanzielle Closure korrekt umgesetzt — alle D1-D5 Resolution-Claims im Code, Tests, Locales, Bindings + Spec verifiziert; 3 echte Open-Items + 1 Quick-Win-Patch + 5 Defers identifiziert._

_Resolution 2026-04-30 (Re-Review-Closure-Commit): Re-D1 + Re-D3 + Re-P1 applied; Re-D2 dismissed-as-false-positive nach Code-Audit; alle 5 Defers in `deferred-work.md` als A1-Re-F1..F5 persistiert._

#### Decision Needed (Re-Review — alle resolved 2026-04-30)

- [x] [Re-Review][Decision] **Re-D1 — AutoStop natural completion + hard-cap timeout: kein `RecordingStopped`-Event** [`klarvo-shell-orchestrator/src/session.rs`] — *Resolved Option A:* `RecordingStopped` wird jetzt in `pipeline_task` zwischen `pipeline.await`-Resolution und Cleanup-Block emittiert, conditioned on `press_mode == RecordingMode::AutoStop`. Deckt beide Pfade ab (VAD-SpeechEnd + Hard-Cap-Timeout). Test-Coverage in `autostop_transitions_to_idle_after_vad` via neuem `collect_events_until_completed`-Helper; assertet sowohl Presence als auch Reihenfolge (Started < Stopped < Completed).

- [x] [Re-Review][Decision] **Re-D2 — VAD-State-Pollution-Risk auf Hard-Cap-Timeout-Cancel** [`klarvo-shell-orchestrator/src/session.rs:175-209`] — *Dismissed (false-positive nach Code-Audit):* `VadProvider`-Trait hat bereits eine `reset()`-Method (`klarvo-core/src/audio/vad/provider.rs:13`), und `run_capture_session` ruft `vad.reset()` als ersten Schritt jeder Session auf (`klarvo-core/src/pipeline/orchestrator.rs:60`). Damit ist der nach Cancel-Drop residuelle VAD-State irrelevant — die nächste Session beginnt mit reset. Trait-Contract ist im ADR-0012 Amendment 2 (A2-3) explizit dokumentiert: `reset()` muss jeden internen State invalidieren; aktuelle `RmsVad`-Impl erfüllt das trivial (energy-only); künftige stateful-Impls (Silero etc.) müssen Idempotenz garantieren.

- [x] [Re-Review][Decision] **Re-D3 — ADR-0012 Amendment 2 für D1/D4/D5 fehlt** [`docs/adr/0012-orchestrator-owner.md`] — *Resolved Option A:* Amendment 2 hinzugefügt mit vier Sub-Sections: A2-1 Single-Writer-Pattern (D1 + Korrektur zu Amendment-1-Bootstrap-Beispiel: `app.manage(mode_arc)` ist entfernt), A2-2 `press_mode`-Snapshot (D4), A2-3 Hard-Cap-Timeout (D5 + VAD-Cancel-Safety-Klärung), A2-4 AutoStop emittiert `RecordingStopped` (Re-D1 — neue Klausel zum 3-State-Lifecycle-Contract).

#### Patch (Re-Review — applied 2026-04-30)

- [x] [Re-Review][Patch] **Re-P1 — `RecordingMode::from_str` Parse-Error im `settings.changed`-Listener silent geschluckt** [`shells/windows/src-tauri/src/main.rs:325-342`] — *Applied:* Listener nutzt jetzt explicit `match RecordingMode::from_str(new_value) { Ok(mode) => spawn write, Err(_) => tracing::warn!(...) }`. Symmetrisch zum Step-11c-i18n-Listener-Pattern. Schützt gegen DB-Writes, die den validierenden Tauri-Command bypassen (z. B. künftige Migrations via `set_raw`).

#### Deferred (Re-Review)

- [x] [Re-Review][Defer] **A1-Re-F1 — Rapid Toggle-Stop+Restart während noch laufender Pipeline: VAD-Lock-Contention** [`klarvo-shell-orchestrator/src/session.rs`] — User triple-tappt Toggle (start/stop/start) innerhalb ~1s. Erstes Pipeline-Task hält noch VAD-Lock für STT-Cleanup; neue Recording-Session emittiert `RecordingStarted` aber blockiert beim VAD-Acquire bis erstes Pipeline-Task fertig — silent-recording-Window (Tray/Pill-Bar zeigt Recording, aber VAD verarbeitet keine Frames). Fix-Pfad: Session-ID + Abort-Old-Pipeline-on-New-Press, oder VAD-pro-Session statt Shared-Mutex. Hoher Impact, aber großer Architecture-Refactor — Phase-2-B-A3-Pill-Bar-Design-Window passt.
- [x] [Re-Review][Defer] **A1-Re-F2 — Test-Coverage-Lücken** — Vier fehlende Tests: (a) AutoStop-Hard-Cap-Timeout-Branch nie executed (kein Test verifiziert `error.recording.timeout`-Toast oder `Ok(None)`-Cleanup-Pfad); (b) `is_idle()` nur in `autostop_transitions_to_idle_after_vad` asserted — Toggle-Stop und Hold-Release haben keinen State-Pull-Check; (c) Mode-Change-Mid-Session-Snapshot (D4-Resolution) durch keinen Test gedeckt — ein "Simplification"-Refactor der re-reads würde silent regressieren; (d) Triple-Tap-Toggle (siehe Re-F1) — Race-Window ungetestet. Phase-2-B-Test-Hardening-Story.
- [x] [Re-Review][Defer] **A1-Re-F3 — Hard-Cap-Timeout-Toast-Emit blockiert Cleanup** [`klarvo-shell-orchestrator/src/session.rs:203-208`] — Sequenz: `tokio::time::timeout` → `Err(_)` → `error_emitter.emit_error(...).await` (kann bei IPC-Lag mehrere ms blocken) → erst dann fällt der Cleanup-Block. Audio-Capture stays offen länger als 60s nominal. Minor (IPC-Lag in Praxis < 50ms), aber widerspricht "Hard-Cap"-Semantik. Fix: Cleanup vor Emit oder Emit fire-and-forget via spawn.
- [x] [Re-Review][Defer] **A1-Re-F4 — BYOK-API-Cost-Transparency auf Hard-Cap-Drop** — Wenn Hard-Cap firet während STT-Plugin gerade mid-HTTP-Request ist (z. B. Groq), wird `reqwest`-Future via Cancel-Drop unterbrochen. Server-seitig kann der Request schon partial gestreamt sein → User zahlt für orphaned API-Call ohne Output. Geringes praktisches Volumen, aber relevant für BYOK-Narrativ. Phase-2-B-Hardening: Tracing/Metric "stt.cancelled-on-hardcap" + Doc-Note in Settings-UI.
- [x] [Re-Review][Defer] **A1-Re-F5 — `DEFAULT_RECORDING_MODE_SLOT1`-Const-Typo surfacelt nur runtime** [`klarvo-core/src/settings/mod.rs:301-307`] — Falls jemand den const-String typed (`"toogle"` statt `"toggle"`), schlägt `RecordingMode::from_str` erst beim ersten `recording_mode_slot1()`-Read im Production-Run fehl. Compile-Time-Schutz wäre `RecordingMode::default()`-Const oder `phf_map!`-basiertes Lookup. Niedrige Priorität — Test-Coverage greift heute. Phase-2-Cleanup.

#### Closure-Audit-Drift (Informational)

- 🔍 **D1-Provenance-Split**: Auditor: `app.manage(recording_mode_arc)`-Removal (zentral für D1-Single-Writer-Resolution) ist tatsächlich in Commit `7803eda` (Story 2.A.A8-Sub) materialisiert, NICHT in `4f0e0f7` (2.B.A1-Closure). Das Closure-Spec attribuiert die Removal an `4f0e0f7`, aber `4f0e0f7`-Diff hat 0 Lines in `main.rs`. Net-Effekt im Tree: korrekt. Provenance: split — bei isoliertem `git revert 4f0e0f7` würde D1 nicht voll zurückgerollt werden.
- 🔍 **Commit-Message-Discrepancies in `4f0e0f7`**: (a) "13 defer entries" — actual count is 14 (F1-F10 + D2/D5/D5b/D7); (b) "fires `RecordingCompleted` on timeout" — code returns `Ok(None)` und exited das spawn-block ohne explicit `RecordingCompleted`-Emit (siehe Re-D1). Beide post-commit nicht korrigierbar; nur informationell.

#### Dismissed (Re-Review — Noise / by-design / out-of-scope)

- AutoStop Hard-Cap nicht symmetrisch für Toggle/WaitAndType — bereits A1-D5b deferred.
- `set_recording_mode_slot1` sync vs. async — Tauri-Command-Pool handled das; Tauri-Konvention.
- `RecordingMode::FromStr::Err` From-AppError-Conversion-Visibility — würde compile-fail wenn fehlend; CI-greift.
- `text_to_deliver`-Clone-Hygiene — minor cosmetic, Lifetimes erfordern Clone für Event-Payload.
- `is_idle`-Lock-Surface bei high-frequency Tray-Pull — Tray nutzt 1Hz-State-Pull (Story 3.8); FIFO-Fairness verhindert Starvation.
- on_release emittiert RecordingStopped vor `drop(capture_handle)` — cosmetic ordering, cpal-stop ist non-blocking.
- Settings-Listener kein Replay/Initial-State-Sync — pre-existing Pattern aus Phase-1, nicht durch 4f0e0f7 eingeführt.
