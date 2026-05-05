---
name: Story 8.1 — Second Hotkey-Slot
epic: 8
story_number: "8.1"
status: review
dependencies:
  - "2b-a1-toggle-autostop-wait-and-type-modes"
adr_refs:
  - docs/adr/0011-hotkey-backend.md
  - docs/adr/0012-orchestrator-owner.md
---

# Story 8.1: Second Hotkey-Slot

Status: ready-for-dev

## Story

Als Klarvo Power-User
möchte ich einen zweiten Hotkey-Slot mit eigenem Recording-Mode im Settings-Panel konfigurieren,
damit ich z.B. Hold-to-Talk und Toggle parallel betreiben kann ohne config.toml zu editieren.

## Kontext und Motivation

**ADR-0011 Phase-2+-Impact (Accepted):** "Zweiter Hotkey-Slot ist additiv registrierbar — Plugin erlaubt mehrere Shortcuts via `register_multiple` oder iterative `register`-Calls." Kein Plugin-Change nötig.

**Story 2.B.A1 Foundation:** `RecordingMode`-Enum, Toggle/AutoStop/WaitAndType-Orchestrator, `recording_mode_slot1`-Setting, Mode-Dropdown im Settings-Panel sind alle done. Story 8.1 erweitert das etablierte Pattern auf einen zweiten Slot.

**Entscheidungen (aus Step-1-Workflow, 2026-05-05):**

| ID | Entscheidung |
|----|---|
| D-1 | Mutual-Exclusion: Slot-2-Press während Slot-1-Recording → silently discarded (analog ADR-0011 Key-Repeat-Guard). Implizit durch bestehenden `SessionState`-Guard. |
| D-2 | Conflict-Detection: Inline-Fehlermeldung im Settings-Panel + Save-Button disabled + Backend-Guard bei `set_hotkey_slot2`-Command. Slot2 == Slot1 → nicht speicherbar. |
| D-3 | Slot-2-Optionality: Leeres Feld = kein zweiter Hotkey registriert. `hotkey.slot2.combo` nullable (kein Default). |

## Acceptance Criteria

### AC-1: Core Settings — `hotkey.slot2.combo` + `hotkey.slot2.mode`

**Given** `klarvo-core/src/settings/mod.rs` hat `hotkey.slot1.combo` + `hotkey.slot1.mode` als Keys,
**When** AC-1 committed ist,
**Then**:

Neue Accessor-Methoden in `Settings`:

```rust
/// Returns `Ok(None)` when no slot-2 hotkey is configured (D-3).
pub fn hotkey_slot2_combo(&self) -> Result<Option<String>, AppError> {
    self.get_raw("hotkey.slot2.combo")
}

pub fn set_hotkey_slot2_combo(&self, val: &str) -> Result<(), AppError> {
    validate_setting_value("hotkey.slot2.combo", val)?;
    self.set_raw("hotkey.slot2.combo", val, "string")
}

/// Clear slot-2 combo (sets key absent / deleted from DB).
pub fn clear_hotkey_slot2_combo(&self) -> Result<(), AppError> {
    self.delete_raw("hotkey.slot2.combo")
}

/// Returns `RecordingMode::Hold` when `hotkey.slot2.mode` is not set.
/// Only meaningful when `hotkey_slot2_combo()` returns `Some`.
pub fn recording_mode_slot2(&self) -> Result<RecordingMode, AppError> {
    match self.get_raw("hotkey.slot2.mode")? {
        Some(raw) => RecordingMode::from_str(&raw),
        None => Ok(RecordingMode::Hold),
    }
}

pub fn set_recording_mode_slot2(&self, mode: RecordingMode) -> Result<(), AppError> {
    self.set_raw("hotkey.slot2.mode", &mode.to_string(), "string")
}
```

**WICHTIG `delete_raw`:** Falls `Settings` noch keine `delete_raw`-Methode hat, muss diese ergänzt werden:

```rust
pub fn delete_raw(&self, key: &str) -> Result<(), AppError> {
    let db = self.db.lock().map_err(|_| internal_err("settings lock poisoned"))?;
    db.execute("DELETE FROM settings WHERE key = ?1", [key])
        .map_err(|e| settings_err(format!("delete_raw({key}): {e}")))?;
    Ok(())
}
```

**`validate_setting_value`:** `"hotkey.slot2.combo"` fällt bereits unter das `"hotkey."`-Prefix in `CORE_PREFIXES` — kein neuer Eintrag nötig.

**Schema:** Kein neues DB-Migration-Script nötig. Die Keys werden lazy angelegt (INSERT OR REPLACE bei `set_raw`). `hotkey.slot2.combo` und `hotkey.slot2.mode` sind NICHT in `MIGRATION_SENTINEL_KEYS` — sie sind optional (D-3).

`cargo test -p klarvo-core` → Exit 0.

---

### AC-2: `HotkeySlot`-Enum + Orchestrator `on_press(slot)` / `on_release(slot)`

**Given** `klarvo-core/src/recording.rs` enthält `RecordingMode`,
**When** AC-2 committed ist,
**Then**:

**Neues Enum in `klarvo-core/src/recording.rs`:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeySlot {
    One,
    Two,
}
```

**Orchestrator — neues Feld + Konstruktor-Parameter:**

In `klarvo-shell-orchestrator/src/session.rs`:

```rust
pub struct SessionOrchestrator {
    // ... bestehende Felder ...
    mode: Arc<tokio::sync::RwLock<RecordingMode>>,       // Slot 1 — bereits vorhanden
    mode_slot2: Arc<tokio::sync::RwLock<RecordingMode>>, // Slot 2 — NEU
    // ...
}
```

Konstruktor-Extension (analog `mode`):

```rust
pub fn new(
    // ... bestehende Parameter ...
    mode: Arc<tokio::sync::RwLock<RecordingMode>>,
    mode_slot2: Arc<tokio::sync::RwLock<RecordingMode>>, // NEU
    // ...
) -> Arc<Self> { ... }
```

**`on_press` + `on_release` — Signatur-Extension:**

```rust
pub async fn on_press(&self, slot: HotkeySlot) {
    // Mode-Lookup per Slot:
    let press_mode = match slot {
        HotkeySlot::One => self.mode.read().await.clone(),
        HotkeySlot::Two => self.mode_slot2.read().await.clone(),
    };
    // Rest des Bodys unverändert — bestehender SessionState-Guard
    // behandelt "already recording" identisch für beide Slots (D-1).
}

pub async fn on_release(&self, slot: HotkeySlot) {
    // slot-Parameter wird im Body NICHT für die Mode-Lookup verwendet:
    // press_mode ist im SessionState::Recording { press_mode } gespeichert.
    // Der Parameter ist für semantische Klarheit + spätere Tracing-Logs.
    // Body im Wesentlichen unverändert.
    let _ = slot; // suppress unused-var wenn kein Tracing
}
```

**Mutual-Exclusion (D-1) — IMPLIZIT durch bestehenden Guard:**

Der existierende Guard in `on_press`:

```rust
if let SessionState::Recording { press_mode, .. } = *state {
    // ... discard as key-repeat
}
```

greift für beide Slots — kein zusätzlicher Code nötig.

**ADR-0012 Amendment 2:** Am Ende von `docs/adr/0012-orchestrator-owner.md` anhängen:

```markdown
## Amendment 2 — HotkeySlot-Enum (Story 8.1, 2026-05-05)

`HotkeySlot { One, Two }` in `klarvo-core/src/recording.rs` eingeführt.
`on_press(slot: HotkeySlot)` / `on_release(slot: HotkeySlot)` erweitern die Signatur.
Mode-Lookup via `self.mode` (Slot::One) bzw. `self.mode_slot2` (Slot::Two).
Mutual-Exclusion (D-1): bestehender `SessionState`-Guard discarded Slot-2-Press
während Slot-1-Recording transparent — kein neuer Code.
```

`cargo check --workspace --exclude klarvo-windows-shell` → Exit 0.

---

### AC-3: Shell — Conditional Slot-2-Registration (Startup)

**Given** `shells/windows/src-tauri/src/main.rs` registriert Slot-1-Hotkey in `.setup(..)`,
**When** AC-3 committed ist,
**Then**:

Nach der bestehenden Slot-1-Registrierung in `.setup(..)`:

```rust
// --- Slot-2 Hotkey (conditional, D-3) ---
let slot2_combo = settings.hotkey_slot2_combo().unwrap_or_else(|e| {
    tracing::warn!(error = %e, "hotkey_slot2_combo read failed; slot 2 not registered");
    None
});

if let Some(ref combo2) = slot2_combo {
    let slot1_combo = settings.hotkey_slot1_combo().unwrap_or_default();
    if combo2 == &slot1_combo {
        // D-2 Backend-Guard: identical combos → soft-fail, no registration
        tracing::warn!(
            combo = %combo2,
            "hotkey slot-2 combo identical to slot-1; slot 2 not registered"
        );
    } else {
        register_hotkey_slot2(&app, combo2, Arc::clone(&orchestrator));
    }
} else {
    tracing::debug!("hotkey slot-2 not configured; skipping registration (D-3)");
}
```

**Neue Funktion `register_hotkey_slot2` in `shells/windows/src-tauri/src/hotkey.rs`:**

```rust
/// Register the optional second push-to-talk hotkey.
/// Fails soft (warn + return) on parse or registration error — app remains
/// functional with slot 1 only (D-3). No error event emitted to frontend.
pub fn register_hotkey_slot2<R: tauri::Runtime>(
    app: &tauri::App<R>,
    combo: &str,
    orchestrator: Arc<SessionOrchestrator>,
) {
    let shortcut = match Shortcut::from_str(combo) {
        Ok(s) => s,
        Err(_) => {
            tracing::warn!(combo, "hotkey slot-2 combo parse failed; slot 2 not registered");
            return;
        }
    };

    if let Err(e) = app
        .global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            let orc = Arc::clone(&orchestrator);
            tauri::async_runtime::spawn(async move {
                match event.state() {
                    ShortcutState::Pressed  => orc.on_press(HotkeySlot::Two).await,
                    ShortcutState::Released => orc.on_release(HotkeySlot::Two).await,
                }
            });
        })
    {
        tracing::warn!(error = %e, combo, "hotkey slot-2 registration failed; slot 2 not active");
    } else {
        tracing::info!(combo, "hotkey slot-2 registered");
    }
}
```

Imports die ergänzt werden müssen: `use klarvo_core::recording::HotkeySlot;`

`cargo check --workspace --exclude klarvo-windows-shell` → Exit 0.

---

### AC-4: Tauri Commands + `UserSettings`-Extension

**Given** `shells/windows/src-tauri/src/commands/settings.rs` enthält `get_user_settings`, `set_hotkey_slot1`, `set_recording_mode_slot1`,
**When** AC-4 committed ist,
**Then**:

**`UserSettings`-Extension (in `klarvo-core/src/settings/mod.rs`):**

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct UserSettings {
    // ... bestehende Felder ...
    pub hotkey_slot2_combo: Option<String>,  // NEU: None wenn nicht konfiguriert
    pub hotkey_slot2_mode: String,           // NEU: "hold" wenn nicht gesetzt
}
```

**`get_user_settings` — Extension:**

```rust
pub fn get_user_settings(
    settings: tauri::State<Arc<Settings>>,
) -> Result<UserSettings, AppError> {
    Ok(UserSettings {
        // ... bestehende Felder ...
        hotkey_slot2_combo: settings.hotkey_slot2_combo()?,
        hotkey_slot2_mode: settings.recording_mode_slot2()?.to_string(),
    })
}
```

**Neue Commands:**

```rust
/// Set or clear the slot-2 hotkey combo.
/// `None` → clears the combo (slot 2 becomes inactive on next reboot).
/// `Some(combo)` → validates grammar + Win32 conflict + Slot-1 conflict before
///   writing to DB. Re-registration happens at next app start (no live re-register
///   to keep Boot-Registration as Single-Source-of-Truth — see Dev Notes).
#[tauri::command]
#[specta::specta]
pub async fn set_hotkey_slot2(
    combo: Option<String>,
    settings: tauri::State<'_, Arc<Settings>>,
) -> Result<(), AppError> {
    match combo {
        None => settings.clear_hotkey_slot2_combo()?,
        Some(ref new_combo) => {
            // D-2 Backend-Guard: same combo as slot 1
            let slot1 = settings.hotkey_slot1_combo().unwrap_or_default();
            if new_combo == &slot1 {
                return Err(AppError {
                    kind: AppErrorKind::Configuration,
                    message: format!("hotkey slot-2 combo identical to slot-1: {new_combo}"),
                    user_message: Some("error.settings.hotkey.slot_conflict".into()),
                    retryable: false,
                });
            }
            // Grammar gate (Shortcut::from_str) — reuse existing parse-error key
            Shortcut::from_str(new_combo).map_err(|_| AppError {
                kind: AppErrorKind::Configuration,
                message: format!("invalid hotkey combo: {new_combo}"),
                user_message: Some("error.hotkey.parse_failed".into()),
                retryable: false,
            })?;
            settings.set_hotkey_slot2_combo(new_combo)?;
        }
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_recording_mode_slot2(
    mode: String,
    settings: tauri::State<Arc<Settings>>,
) -> Result<(), AppError> {
    let parsed = RecordingMode::from_str(&mode)?;
    settings.set_recording_mode_slot2(parsed)
}
```

**Command-Registration** in `main.rs` `generate_handler![...]` + `collect_commands![...]` — analog slot1.

**Dev Note — kein Live-Re-Register:** Slot-2-Re-Registration nach Settings-Write ist absichtlich auf "next boot" verschoben. Die Komplexität von Live-Re-Register (Unregister-altes-Slot2 + Register-neues-Slot2 + Atomic-Recovery) ist derselbe Aufwand wie Slot-1 (`hotkey.rs:reregister_hotkey`), aber für ein optionales Feature ohne sofortigen UX-Benefit. Nutzer sieht Settings-Save-Confirmation + "effective on restart"-Hinweis in UI.

`cargo xtask bindings-drift` → Exit 0.

---

### AC-5: i18n — 4 neue Keys

**Given** `shells/windows/locales/en.json` + `de.json` existieren,
**When** AC-5 committed ist,
**Then**:

**`en.json`** — nach den bestehenden `settings.hotkey.*`-Keys:

```json
  "settings.hotkey.slot2.label": "Second hotkey",
  "settings.hotkey.slot2.placeholder": "Not set — click to configure",
  "settings.recording_mode.slot2.label": "Slot 2 mode",
  "error.settings.hotkey.slot_conflict": "This shortcut is already used by slot 1."
```

**`de.json`** — analog:

```json
  "settings.hotkey.slot2.label": "Zweiter Hotkey",
  "settings.hotkey.slot2.placeholder": "Nicht gesetzt — zum Konfigurieren klicken",
  "settings.recording_mode.slot2.label": "Slot 2 Modus",
  "error.settings.hotkey.slot_conflict": "Diese Tastenkombination wird bereits von Slot 1 verwendet."
```

`cargo xtask lint-events` → Exit 0.

**ACHTUNG Orphan-Check:** `error.settings.hotkey.slot_conflict` hat eine Emit-Site in `set_hotkey_slot2` (Rust-Command). Die anderen 3 Keys sind UI-only (Frontend-Strings) — sie kommen in `orphan-allowlist.txt` falls der Scanner sie nicht über Frontend findet. Vor Commit prüfen ob `lint-events` sie als Orphan meldet; wenn ja, in Allowlist eintragen.

---

### AC-6: Settings Panel — Slot-2-UI + Conflict-Validation

**Given** das Settings-Panel hat Hotkey-Slot-1-Section und Recording-Mode-Slot-1-Dropdown,
**When** AC-6 committed ist,
**Then** ist unterhalb der Slot-1-Section ein neuer Slot-2-Block sichtbar:

**UI-Elemente:**

```
[Zweiter Hotkey]  [_____________ Nicht gesetzt _____________]  [Clear-Button (×)]
[Slot 2 Modus]   [Dropdown: Hold / Toggle / AutoStop / WaitAndType]  ← disabled wenn Slot-2-Feld leer
```

**Conflict-Validation (D-2):**

```
Wenn slot2_input.value === slot1_combo:
  → Rotes Inline-Label unterhalb: "error.settings.hotkey.slot_conflict"
  → Save-Button disabled
  → slot2_input bekommt aria-invalid="true"

Wenn slot2_input.value !== slot1_combo (oder leer):
  → Kein Fehler, Save-Button enabled (sofern alle anderen Felder valide)
```

**"Effective on restart"-Hinweis:** Wenn Slot-2-Combo verändert (und gespeichert) wurde, erscheint ein einmaliger Info-Hinweis direkt unter dem Feld: `settings.hotkey.slot2.restart_hint` ("Active after restart" / "Aktiv nach Neustart"). Key muss in AC-5 ebenfalls ergänzt werden.

**WICHTIG:** Das Konflikt-Signal muss beim `onBlur`-Event (Fokusverlust) ausgewertet werden, nicht erst beim Save-Click, damit Nutzer sofort Feedback kriegt.

`cargo xtask bindings-drift` → Exit 0 (TypeScript sieht neue UserSettings-Felder).

---

### AC-7: Unit Tests in `klarvo-core`

**Given** AC-1 + AC-2 committed,
**When** AC-7 committed ist,
**Then** in `klarvo-core/src/settings/mod.rs` im `#[cfg(test)]`-Block:

```rust
#[test]
fn hotkey_slot2_combo_returns_none_when_not_set() {
    let s = Settings::in_memory(noop()).unwrap();
    assert_eq!(s.hotkey_slot2_combo().unwrap(), None);
}

#[test]
fn hotkey_slot2_combo_roundtrip() {
    let s = Settings::in_memory(noop()).unwrap();
    s.set_hotkey_slot2_combo("F9").unwrap();
    assert_eq!(s.hotkey_slot2_combo().unwrap(), Some("F9".to_string()));
    s.clear_hotkey_slot2_combo().unwrap();
    assert_eq!(s.hotkey_slot2_combo().unwrap(), None);
}

#[test]
fn recording_mode_slot2_defaults_to_hold_when_not_set() {
    let s = Settings::in_memory(noop()).unwrap();
    assert_eq!(s.recording_mode_slot2().unwrap(), RecordingMode::Hold);
}

#[test]
fn recording_mode_slot2_roundtrip() {
    let s = Settings::in_memory(noop()).unwrap();
    s.set_recording_mode_slot2(RecordingMode::Toggle).unwrap();
    assert_eq!(s.recording_mode_slot2().unwrap(), RecordingMode::Toggle);
}
```

In `klarvo-shell-orchestrator/tests/` — Mutual-Exclusion-Test (AC-2 / D-1):

```rust
#[tokio::test]
async fn slot2_press_discarded_when_slot1_recording() {
    // Orchestrator mit Slot-1 in Recording bringen, dann on_press(HotkeySlot::Two) callen.
    // Assert: SessionState bleibt Recording (kein zweites Recording gestartet).
    // Pattern: analog bestehender key_repeat_guard-Tests.
}
```

`cargo test -p klarvo-core -p klarvo-shell-orchestrator` → Exit 0.

---

## Tasks / Subtasks

- [x] AC-1: `hotkey_slot2_combo()` + `recording_mode_slot2()` Accessors + `delete_raw()` (falls fehlend) — `klarvo-core`
- [x] AC-2: `HotkeySlot`-Enum in `recording.rs` + `on_press(slot)` / `on_release(slot)` + `mode_slot2`-Feld + ADR-0012 Amendment 2
- [x] AC-3: `register_hotkey_slot2()` in `hotkey.rs` + conditional Boot-Registration in `main.rs`
- [x] AC-4: `UserSettings`-Extension + `set_hotkey_slot2` + `set_recording_mode_slot2` Commands + Command-Registration
  - [x] `cargo xtask bindings-drift` grün
- [x] AC-5: i18n-Keys in `en.json` + `de.json` (4 Keys + `restart_hint`)
  - [x] `cargo xtask lint-events` grün (Orphan-Allowlist prüfen)
- [x] AC-6: Settings Panel Slot-2-Block + Conflict-Validation + `onBlur`-Trigger + Restart-Hint
- [x] AC-7: Unit Tests (`klarvo-core` 4 Tests + `klarvo-shell-orchestrator` Mutual-Exclusion-Test)
- [x] `cargo check --workspace --exclude klarvo-windows-shell` → Exit 0

## Dev Notes

### Kritische Constraints

**1. `delete_raw`:** Die Settings-API hat möglicherweise keine `delete_raw`-Methode. Vor Implementation prüfen ob `Settings::delete_raw` oder ein Equivalent (`set_raw` mit leerem Value? DELETE-SQL direkt?) existiert. Wenn nicht: implementieren wie in AC-1 beschrieben.

**2. Live-Re-Register nicht in Scope:** Slot-2-Combo-Änderung gilt ab dem nächsten App-Start. Das ist eine bewusste Scope-Entscheidung (D-3-Extension). Wenn Andy das unbefriedigend findet, ist Live-Re-Register ein separater Story-Follow-up analog `reregister_hotkey` für Slot 1.

**3. `HotkeySlot` im Shell-Orchestrator-Test:** Der Mutual-Exclusion-Test braucht einen Orchestrator mit simulierter Recording-Session. Pattern aus bestehenden `e2e_test.rs`-Tests übernehmen.

**4. `set_hotkey_slot2(None)`-Command:** Tauri-specta serialisiert `Option<String>` als `string | null` in TypeScript. Das Frontend sendet `null` für Clear. Im Rust-Command ist `combo: Option<String>` korrekt.

**5. Orphan-Allowlist für Frontend-Only-Keys:** `settings.hotkey.slot2.label`, `settings.hotkey.slot2.placeholder`, `settings.recording_mode.slot2.label`, `settings.hotkey.slot2.restart_hint` haben keine Rust-Emit-Sites. Falls `lint-events` sie als Orphans meldet, in `xtask/orphan-allowlist.txt` eintragen (analog anderen UI-only-Keys).

**6. `UserSettings` lebt in `klarvo-core`:** Der `#[derive(specta::Type)]` auf `UserSettings` braucht alle neuen Felder korrekt mit `serde::Serialize`/`Deserialize`. `Option<String>` ist in specta direkt supported.

### Referenz-Implementierungen

| Aspect | Slot 1 (Referenz) | Slot 2 (Story 8.1) |
|--------|-------------------|---------------------|
| Settings-Key | `hotkey.slot1.combo` | `hotkey.slot2.combo` |
| Mode-Key | `hotkey.slot1.mode` | `hotkey.slot2.mode` |
| Combo-Accessor | `hotkey_slot1_combo() -> Result<String>` (hat Default) | `hotkey_slot2_combo() -> Result<Option<String>>` (nullable) |
| Mode-Accessor | `recording_mode_slot1()` | `recording_mode_slot2()` |
| Shell-Register | `register_hotkey()` | `register_hotkey_slot2()` (soft-fail) |
| Commands | `set_hotkey_slot1`, `set_recording_mode_slot1` | `set_hotkey_slot2`, `set_recording_mode_slot2` |
| on_press | `on_press()` → `on_press(HotkeySlot::One)` | `on_press(HotkeySlot::Two)` |

### Dateien die berührt werden

**UPDATE:**
- `klarvo-core/src/settings/mod.rs` — neue Accessors + `UserSettings`-Extension + (optional) `delete_raw`
- `klarvo-core/src/recording.rs` — `HotkeySlot`-Enum
- `klarvo-shell-orchestrator/src/session.rs` — `mode_slot2`-Feld + `on_press(slot)` / `on_release(slot)`
- `shells/windows/src-tauri/src/hotkey.rs` — `register_hotkey_slot2()`
- `shells/windows/src-tauri/src/main.rs` — conditional Slot-2-Boot-Registration + imports
- `shells/windows/src-tauri/src/commands/settings.rs` — neue Commands + `get_user_settings`-Extension
- `shells/windows/locales/en.json` — 5 neue Keys (4 + restart_hint)
- `shells/windows/locales/de.json` — 5 neue Keys
- `docs/adr/0012-orchestrator-owner.md` — Amendment 2

**NICHT ändern:**
- `klarvo-core/src/settings/migrations.rs` — keine Schema-Migration nötig (KV-Store, lazy)
- `pipeline-manifest.toml` — kein Touch

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

Keine Blocker. `delete_raw` war noch nicht vorhanden → implementiert. `shortcut_dispatch_handler` brauchte `slot: HotkeySlot`-Parameter + `move`-Closure wegen Borrow-Semantik. Bindings-Drift nach generate-bindings aufgelöst.

### Completion Notes List

- AC-1: `hotkey_slot2_combo`, `set_hotkey_slot2_combo`, `clear_hotkey_slot2_combo`, `recording_mode_slot2`, `set_recording_mode_slot2`, `delete_raw` in `klarvo-core/src/settings/mod.rs` implementiert. 4 Unit-Tests grün.
- AC-2: `HotkeySlot { One, Two }` in `klarvo-core/src/recording/mod.rs`. `on_press(slot)` / `on_release(slot)` + `mode_slot2`-Feld in Orchestrator. ADR-0012 Amendment 2 angehängt. Alle Test-Call-Sites auf `HotkeySlot::One` migriert.
- AC-3: `register_hotkey_slot2()` in `hotkey.rs` (soft-fail). `shortcut_dispatch_handler` mit `slot`-Parameter. Conditional Slot-2-Boot-Registration + D-2-Guard + settings.changed-Listener für `hotkey.slot2.mode` in `main.rs`.
- AC-4: `UserSettings` um `hotkey_slot2_combo: Option<String>` + `hotkey_slot2_mode: String` erweitert. `set_hotkey_slot2` (mit D-2-Guard + Grammar-Gate) + `set_recording_mode_slot2` Commands registriert. Bindings nach `generate-bindings` regeneriert.
- AC-5: 5 Keys in `en.json` + `de.json` (4 + `restart_hint`). 4 Frontend-only-Keys in `orphan-allowlist.txt`. `lint-events` OK.
- AC-6: Settings-Panel-Slot-2-Block mit Text-Input, Clear-Button (×), Conflict-Inline-Error, Restart-Hint, Mode-Dropdown (disabled wenn Combo leer). `onBlur`-Trigger für Conflict-Check. Save-Button disabled bei `slot2Conflict`.
- AC-7: 4 Unit-Tests in `klarvo-core`, 1 Mutual-Exclusion-Test (`slot2_press_discarded_when_slot1_recording`) in `klarvo-shell-orchestrator`. 23 Orchestrator-Tests grün.

### File List

- `klarvo-core/src/recording/mod.rs` (modified — HotkeySlot enum)
- `klarvo-core/src/settings/mod.rs` (modified — slot2 accessors, delete_raw, 4 unit tests)
- `klarvo-shell-orchestrator/src/session.rs` (modified — mode_slot2 field, on_press/on_release slot param)
- `shells/windows/src-tauri/src/hotkey.rs` (modified — shortcut_dispatch_handler slot param, register_hotkey_slot2)
- `shells/windows/src-tauri/src/main.rs` (modified — slot2 boot registration, mode_arc_slot2, settings.changed listener)
- `shells/windows/src-tauri/src/commands/settings.rs` (modified — UserSettings extension, set_hotkey_slot2, set_recording_mode_slot2)
- `shells/windows/src-tauri/src/lib.rs` (modified — command registration)
- `shells/windows/locales/en.json` (modified — 5 new keys)
- `shells/windows/locales/de.json` (modified — 5 new keys)
- `shells/windows/src/index.html` (modified — slot2 UI block)
- `shells/windows/src/bindings/index.ts` (modified — regenerated bindings)
- `xtask/orphan-allowlist.txt` (modified — 4 frontend-only keys)
- `docs/adr/0012-orchestrator-owner.md` (modified — Amendment 2)
- `klarvo-shell-orchestrator/tests/session_tests.rs` (modified — HotkeySlot::One migration + mutual-exclusion test)
- `klarvo-shell-orchestrator/tests/e2e_test.rs` (modified — HotkeySlot::One migration + mode_arc_slot2)

### Change Log

- 2026-05-05: Story 8.1 implementiert — Second Hotkey-Slot (AC-1..AC-7). HotkeySlot-Enum, Settings-Accessors, Shell-Registration, Tauri-Commands, i18n-Keys, Settings-Panel-UI, Tests.
