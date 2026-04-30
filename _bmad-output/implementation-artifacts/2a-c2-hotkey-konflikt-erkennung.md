---
name: Story 2.A.C2 — Hotkey-Konflikt-Erkennung
phase: 2
wave: A
story_id: "2.A.C2"
status: ready
dependencies:
  - "2.A.A4"
adr_refs:
  - docs/adr/0013-settings-persistence-schema.md
  - docs/adr/0011-hotkey-backend.md
  - docs/adr/0009-shell-error-bridge.md
source_ref: "welle-2-dispatch-plan.md C2; ADR-0013 Sub-Decision 4"
---

# Story 2.A.C2: Hotkey-Konflikt-Erkennung

## Outcome

`set_hotkey_slot1`-Command (A4) wird um Win32-Konflikt-Validierung erweitert.
Aktuell schreibt der Command den neuen Hotkey direkt in SQLite, ohne zu prüfen ob der Combo
bereits von einer anderen App registriert ist. Bei Konflikt: kein Settings-Write, kein Toast-Update,
User bleibt ratlos.

Nach dem Fix: Command versucht `RegisterHotKey` (Win32) bevor er in Settings schreibt.
Bei HRESULT-Fail: `AppErrorKind::HotkeyConflict` → Toast via ADR-0009-Mechanismus.
Bei Erfolg: Sofortiges `UnregisterHotKey` (wir testeten nur), dann Settings-Write + Re-Register
des eigentlichen Hotkey via Hotkey-Backend (ADR-0011).

## Scope-Fence

**In-Scope:**
- `shells/windows/src-tauri/src/commands/settings.rs` — `set_hotkey_slot1`-Handler erweitern
- Win32 Pre-Validation: `RegisterHotKey` → Fail → `AppErrorKind::HotkeyConflict`
- i18n-Key: `error.hotkey.conflict`
- `AppErrorKind::HotkeyConflict`-Variant hinzufügen (oder re-use `HotkeyRegistration`)
- Hotkey-Backend-Update nach erfolgreichem Settings-Write (unregister old, register new)

**Nicht-in-Scope:**
- Second-Hotkey-Slot (`hotkey.slot2.combo`) — Phase-2-B A2
- Konflikt-Quelle identifizieren (Win32 liefert keinen Eigentümer — Workaround-Hint in Toast)
- `error.hotkey.registration_failed` (boot-time) — bereits Phase-1; C2 ergänzt nur den settings-change-path

## Acceptance Criteria

### AC-1 — Pre-Validation: `RegisterHotKey` vor Settings-Write

**Given** User setzt einen neuen Hotkey via `set_hotkey_slot1`-Command  
**When** `RegisterHotKey(NULL, virtual_key, modifiers)` aufgerufen wird  
**Then** zwei Äste:

**Ast A — Konflikt (RegisterHotKey schlägt fehl):**
- Settings-Write (`settings.set_hotkey_slot1_combo(...)`) wird NICHT ausgeführt.
- `settings.changed`-Event wird NICHT emittiert.
- `AppError { kind: AppErrorKind::HotkeyConflict, user_message: Some("error.hotkey.conflict"), ... }` wird returniert.
- Toast via ADR-0009 (`error_emitter.emit_error("error.hotkey.conflict", ts_ms)`) erscheint.

**Ast B — Kein Konflikt (RegisterHotKey erfolgreich):**
- `UnregisterHotKey(NULL, ...)` sofort nach dem Test-Aufruf (kein Halten).
- Settings-Write führt durch.
- `settings.changed`-Event wird emittiert.

---

### AC-2 — Hotkey-Backend-Update nach erfolgreichem Write

**Given** Settings-Write erfolgreich + `settings.changed` emittiert  
**When** Hotkey-Backend (ADR-0011 / `tauri-plugin-global-shortcut`) den alten Hotkey noch hält  
**Then**
- Alter Hotkey wird via Hotkey-Backend unregistriert.
- Neuer Hotkey wird via Hotkey-Backend registriert.
- Bei Registration-Fail durch Hotkey-Backend (parallel zu UnregisterHotKey): Settings-Write
  NICHT rückgängig machen (zu komplexer Rollback) — Toast mit `error.hotkey.registration_failed`
  statt HotkeyConflict (anderer Fehlerfall).

---

### AC-3 — i18n-Key `error.hotkey.conflict` registriert

**Given** `cargo xtask lint-events` nach der Story  
**When** G3-Sub-Lint B (Locale-Coverage) läuft  
**Then**
- `error.hotkey.conflict` in `en.json` + `de.json` vorhanden.
- Key ist semantisch korrekt: z. B. `"en": "This keyboard shortcut is already in use by another application."`.
- `cargo xtask lint-events` Exit 0.

---

### AC-4 — `AppErrorKind::HotkeyConflict` variant oder reuse

**Given** `klarvo-core/src/error.rs` `AppErrorKind`-Enum  
**When** Conflict-Fehler erzeugt wird  
**Then**
- Entweder: neues `HotkeyConflict`-Variant existiert.
- Oder: `HotkeyRegistration`-Variant (Phase-1, falls vorhanden) wird wiederverwendet.
- In beiden Fällen: `user_message: Some("error.hotkey.conflict".into())` gesetzt
  (kein generisches `"error.internal"`-Fallback für diesen Fall).

---

### AC-5 — Kein Conflict-Error bei valider Combo

**Given** Hotkey-Combo `"ctrl+shift+alt+v"` (unrealistisch konfliktreich, aber testbar:
verwende eine Combo, die aktuell von keiner anderen App gehalten wird)  
**When** `set_hotkey_slot1`-Command aufgerufen  
**Then**
- Settings-Write erfolgreich.
- `settings.changed`-Event mit `key = "hotkey.slot1.combo"`, `new_value = "ctrl+shift+alt+v"`.
- Kein `error.hotkey.conflict`-Toast.

---

## Technical Notes

- Win32 `RegisterHotKey`: `winapi::um::winuser::RegisterHotKey(hwnd, id, modifiers, vk)`.
  Für Test-Validation: `hwnd = NULL`, `id = 0xBEEF` (temporary ID).
  Bei Fail: `GetLastError()` = `ERROR_HOTKEY_ALREADY_REGISTERED (1409)`.
- Parsing vom Hotkey-Combo-String (z. B. `"ctrl+shift+v"`) zu `(modifiers, vk)`:
  Entweder via `tauri-plugin-global-shortcut`'s interne Parsing-Logik (wenn zugänglich),
  oder eigene Parsing-Funktion im Shell-Code.
- `RegisterHotKey` auf NULL-HWND ist Thread-spezifisch (Message-Queue des calling Thread).
  Async-Context in Tauri-Command: prüfen ob Win32-Aufruf auf Tauri's async-Runtime-Thread
  korrekt funktioniert. Falls nötig: `tokio::task::spawn_blocking`.
- Vorhandener Hotkey-Backend (ADR-0011): `hotkey.rs` registriert den Recording-Hotkey via
  `tauri-plugin-global-shortcut`. Nach Settings-Write: `shortcut.unregister(old)` + `register(new)`.
  Source: `shells/windows/src-tauri/src/hotkey.rs`.
