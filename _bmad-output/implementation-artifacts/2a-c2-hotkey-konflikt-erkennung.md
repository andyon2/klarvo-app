---
name: Story 2.A.C2 — Hotkey-Konflikt-Erkennung
phase: 2
wave: A
story_id: "2.A.C2"
status: done
dependencies:
  - "2.A.A4"
adr_refs:
  - docs/adr/0013-settings-persistence-schema.md
  - docs/adr/0011-hotkey-backend.md
  - docs/adr/0009-shell-error-bridge.md
source_ref: "_bmad-output/planning-artifacts/epics/epic-phase-2-a.md C2; ADR-0013 Sub-Decision 4"
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

---

## Tasks/Subtasks

- [x] AC-4: `AppErrorKind::HotkeyConflict` in `klarvo-core/src/error.rs` ergänzt
- [x] AC-3: i18n-Keys `error.hotkey.conflict` in `en.json` + `de.json` + `REQUIRED_KEYS`
- [x] AC-1/AC-2: `validate_hotkey_not_conflicting` (async, spawn_blocking, Win32-probe) in `hotkey.rs`
- [x] AC-1/AC-2: `reregister_hotkey` (unregister old + register new + Toast bei Fail) in `hotkey.rs`
- [x] AC-1/AC-2: `set_hotkey_slot1` auf async umgestellt, AppHandle ergänzt, Win32-Ablauf verdrahtet
- [x] `shortcut_dispatch_handler` extrahiert (kein Duplicate zwischen register + reregister)
- [x] Parse-Tests für `parse_combo_to_win32` in `hotkey.rs` (Linux-lauffähig)
- [x] Win32 round-trip-Test `win32_validation_uncontested_combo_succeeds` (Windows-only, CI-gated)
- [x] Alle Tests grün (20/20 lib, lint-events OK)

---

## File List

- `klarvo-core/src/error.rs` — `AppErrorKind::HotkeyConflict` variant
- `shells/windows/locales/en.json` — `error.hotkey.conflict` key
- `shells/windows/locales/de.json` — `error.hotkey.conflict` key
- `shells/windows/src-tauri/src/i18n.rs` — `error.hotkey.conflict` in REQUIRED_KEYS
- `shells/windows/src-tauri/src/hotkey.rs` — `validate_hotkey_not_conflicting`, `reregister_hotkey`, `parse_combo_to_win32`, `key_name_to_vk`, `shortcut_dispatch_handler`; Tests
- `shells/windows/src-tauri/src/commands/settings.rs` — `set_hotkey_slot1` async + AppHandle + Win32-Flow

---

## Dev Agent Record

### Completion Notes

**Datum:** 2026-05-01

**Implementierter Flow (AC-1 + AC-2):**
1. `set_hotkey_slot1` ist jetzt `async`, empfängt `tauri::AppHandle` (Tauri-injected).
2. Win32-Pre-Validation: `validate_hotkey_not_conflicting(&combo).await?` — läuft in `spawn_blocking`, ruft `RegisterHotKey(NULL, 0xBEEF, ...)` auf und returned sofort `HotkeyConflict`-Error wenn Win32 ablehnt, sonst `UnregisterHotKey` und `Ok(())`.
3. Old-Combo-Read vor Settings-Write (für Unregister in Schritt 4).
4. Settings-Write via `settings.set_hotkey_slot1_combo` (feuert `settings.changed` wie vorher).
5. `reregister_hotkey(&app, old, new)` — unregistered alten Shortcut (best-effort), registriert neuen mit identischem `shortcut_dispatch_handler` (AC-2). Fail → Toast `error.hotkey.registration_failed`, kein Settings-Rollback.

**Entscheidungen:**
- `#[allow(unused_variables)] app` + `#[cfg_attr(not(target_os = "windows"), allow(unused_variables))] let old_combo` — verhindert Warnings auf Linux-CI ohne die cfg-Gating-Semantik zu brechen.
- `shortcut_dispatch_handler()` extrahiert, damit `register_hotkey` (boot) und `reregister_hotkey` (settings-change) identische Dispatch-Logik ohne Duplikat haben.
- `parse_combo_to_win32` eigene Implementierung (Electron-Accelerator-Format → Win32 modifiers + VK); Scope: A–Z, 0–9, F1–F12, common nav/editing keys. Unbekannte Keys → `None` → `Validation`-Error (kein `HotkeyConflict`).
- Windows-Crate: `windows::Win32::UI::Input::KeyboardAndMouse` bereits mit `Win32_UI_Input_KeyboardAndMouse`-Feature im Cargo.toml aktiviert — keine neue Dependency nötig.

### Change Log

- 2026-05-01: Initial implementation — AC-1..AC-5 implementiert; alle Tests grün (20/20 lib, lint-events OK)
- 2026-05-01: Code-Review (3 Layer: Blind / Edge-Case / Acceptance-Auditor) — 16 Findings (4 decision-needed, 9 patch, 3 defer)
- 2026-05-01: Code-Review-Closure — 4 Decisions resolved (D1→P10 Skip-if-equal+Unregister-Old+Probe+Recovery, D2→P11 `Shortcut::from_str` Grammar-Gate, D3→Spec-Amendment, D4→P12 Re-Register-Old-Recovery), 12 Patches applied (P1 MOD_NOREPEAT, P2 AtomicI32, P3 RAII-Guard, P4 emit-on-parse-fail, P5 match instead of unwrap_or_default, P7 F12→F24, P8 `#[tokio::test]`, P9 cfg-gate, P10/P11/P12); 3 Defers in `deferred-work.md`; bindings regeneriert; alle Gates grün (20/20 lib, lint-events OK, manifest-strict OK, bindings-drift OK)

---

## Review Findings

> Quelle: `bmad-code-review` (3 parallele Layer), 2026-05-01.

### Decision-needed (resolved 2026-05-01)

- [x] **D1 → P10** (Option 4: Skip-if-equal als Fast-Path + unregister(old)→Probe→settings-write→register(new) + Re-register-Old als Recovery wenn Probe fehlschlägt).
- [x] **D2 → P11** (Option 1: `Shortcut::from_str` als Grammar-Gate VOR Probe; Win32-VK-Mapping bleibt lokal, Lücken im VK-Map werden zu sauberen `error.hotkey.parse_failed`).
- [x] **D3 → Spec-Amendment** (Option 1: Implementation bleibt; AC-1 Ast A liest sich Async-Bus-zentrisch, ADR-0009 Hybrid-C erlaubt aber Sync-Result-Path für User-Commands. Klarstellung im AC-1-Block unten.).
- [x] **D4 → P12** (Option 2: Bei `on_shortcut(new)`-Fail in `reregister_hotkey` → `gs.on_shortcut(old)` als Recovery; Settings stay neu, Hotkey stays alt; Toast-Wortlaut über i18n-Key klarstellen).

### AC-1 Spec-Amendment (2026-05-01, D3-Resolution)

> AC-1 Ast A bullet "Toast via ADR-0009 (`error_emitter.emit_error("error.hotkey.conflict", ts_ms)`)" liest sich Async-Bus-zentrisch.
> Per ADR-0009 Hybrid-C ist der **Sync-Result-Path** (Tauri-Command returnt `Err(AppError { user_message: Some("error.hotkey.conflict"), ... })`) der kanonische Pfad für User-initiierte Commands. Implementation in `validate_hotkey_not_conflicting` ist ADR-0009-konform; das Frontend resolved den i18n-Key vom Result-Error.
> Kein zusätzlicher `emit_error("error.hotkey.conflict")`-Call nötig — würde doppelte User-Surface erzeugen.

### Patch

- [x] [Review][Patch] **P1 — `MOD_NOREPEAT` zur Probe-Modifier-Maske hinzufügen** (BLOCKER, edge+blind) — `global-hotkey` registriert produktiv mit `MOD_NOREPEAT` (siehe `global-hotkey-0.7.0/src/platform_impl/windows/mod.rs:78`); Probe ohne diesen Flag kann Probe-/Runtime-Asymmetrie erzeugen. [`shells/windows/src-tauri/src/hotkey.rs:131`]

- [x] [Review][Patch] **P2 — Hardcoded ID `0xBEEF` durch Atomic-Counter oder Probe-Mutex ersetzen** (HIGH, blind+edge) — Zwei concurrent `set_hotkey_slot1`-Calls (User-Spam-Save oder Frontend-Doppel-Invoke) kollidieren auf demselben ID, zweiter Call kriegt false-positive `HotkeyConflict`. [`hotkey.rs:131,137`] Empfehlung: `static NEXT_PROBE_ID: AtomicI32 = AtomicI32::new(0xBEEF);` + `.fetch_add(1, Ordering::Relaxed)` pro Probe; oder `tokio::Mutex` um den ganzen Probe-Block.

- [x] [Review][Patch] **P3 — RAII-Scope-Guard für Probe-Registration** (HIGH, blind+edge) — Bei Panic zwischen `RegisterHotKey` und `UnregisterHotKey` (z. B. Tokio-Runtime-Shutdown) leakt die Test-Registration auf dem Blocking-Pool-Thread. Subsequente Probes mit demselben ID failed. [`hotkey.rs:130-138`] Empfehlung: `scopeguard::defer!` oder kleines `struct ProbeGuard { id: i32 } impl Drop` — passt auch zu Memory-Pattern `feedback_test_raii_cleanup_pattern`.

- [x] [Review][Patch] **P4 — `reregister_hotkey` Parse-Failure-Pfad: ADR-0009 lossless emit** (MEDIUM, auditor) — Wenn `Shortcut::from_str(new_combo)` nach erfolgreicher Probe doch fehlschlägt (z. B. Parser-Divergenz aus D2), läuft nur `tracing::warn!`, kein `app.error`-Toast. ADR-0009 SD-3 fordert Lossless-Async-Errors. [`hotkey.rs:85-91`] Fix: `emit_error("error.hotkey.parse_failed", ts_ms)` ergänzen analog zum boot-time `register_hotkey`.

- [x] [Review][Patch] **P5 — `unwrap_or_default()` auf `old_combo` durch `match` ersetzen** (MEDIUM, blind+edge) — Bei DB-Read-Error wird `old_combo = ""` → `Shortcut::from_str("")` failed silent → `unregister(old)` skipped → alter Hotkey bleibt registriert (parallel zu neuem). User merkt's nicht. [`commands/settings.rs:108`] Fix: `match settings.hotkey_slot1_combo()` mit `Err(e) => { tracing::warn!(...); String::new() }` ist ehrlicher; oder Fehler propagieren.

- [x] [Review][Patch] **P6 — Probe-Parse-Failure i18n-Key umrouten auf `error.hotkey.parse_failed`** (LOW, edge+auditor) — Aktuell `error.settings.validation` (REQUIRED_KEYS-Eintrag bestätigt vorhanden, i18n.rs:76); semantisch besser `error.hotkey.parse_failed` (existiert auch, i18n.rs:86) — User-Toast wird Domain-aligned. [`hotkey.rs:126`]

- [x] [Review][Patch] **P7 — Test-Combo `Ctrl+Shift+Alt+F12` ist auf vielen Dev-Maschinen besetzt (DevTools/NVIDIA/Screen-Capture)** (LOW, blind+edge) — `Ctrl+Shift+Alt+F24` oder ähnlich obskures Combo wählen. Auch CI-Risiko falls parallel-cargo-test mit `0xBEEF`-Kollision (siehe P2). [`hotkey.rs:317`]

- [x] [Review][Patch] **P8 — Test `win32_validation_uncontested_combo_succeeds` auf `#[tokio::test]` umstellen** (LOW, blind) — `tokio::runtime::Runtime::new().unwrap()` per-Test ist nicht-idiomatisch; kann mit anderen Tokio-Tests im selben File über die Blocking-Pool-Threads interferieren. [`hotkey.rs:316`]

- [x] [Review][Patch] **P9 — `#[allow(unused_variables)]` auf `app: AppHandle` cfg-gaten** (LOW, blind) — Aktuell unconditional, blendet zukünftige legitime Warnings auf Windows aus. Symmetrisch zu `old_combo`-Pattern (`#[cfg_attr(not(target_os = "windows"), allow(unused_variables))]`). [`commands/settings.rs:97`]

- [x] [Review][Patch] **P10 (D1-Resolution) — Skip-if-equal Fast-Path + unregister-old-then-probe-then-register-new mit Re-Register-Old-Recovery** (BLOCKER) — In `set_hotkey_slot1` (oder einer neuen `validate_hotkey_not_conflicting`-Surface): wenn `new_combo == old_combo` → Probe und Reregister überspringen (idempotent return Ok). Sonst: `gs.unregister(old)` → Probe (`RegisterHotKey`+`UnregisterHotKey`) → bei Probe-Fail `gs.on_shortcut(old, ...)` als Recovery + return `HotkeyConflict` → bei Probe-Erfolg Settings-Write + `gs.on_shortcut(new, ...)`. Coordiniert mit P12 (Re-Register-Old-Recovery auch im `on_shortcut(new)`-Fail-Pfad). [`hotkey.rs:115-155`, `commands/settings.rs:101-115`]

- [x] [Review][Patch] **P11 (D2-Resolution) — `Shortcut::from_str` als Grammar-Gate VOR Probe** (HIGH) — In `validate_hotkey_not_conflicting`: zuerst `Shortcut::from_str(&combo).map_err(|_| AppError { user_message: Some("error.hotkey.parse_failed"), ... })?`, dann `parse_combo_to_win32`. Probe lehnt damit alles ab, was Runtime ablehnt → kein Split-Brain. Lücken im lokalen `key_name_to_vk` werden zu `error.hotkey.parse_failed`-Validation-Errors statt false-positive `HotkeyConflict`. [`hotkey.rs:115-155,185-218,221-258`] Subsumiert P6 (i18n-Key-Routing) — die `error.settings.validation`-Site wird ohnehin durch diesen Patch obsolet.

- [x] [Review][Patch] **P12 (D4-Resolution) — Re-Register-Old als Recovery in `reregister_hotkey`** (HIGH) — Bei `gs.on_shortcut(new_shortcut, ...)`-Fail: `gs.on_shortcut(old_shortcut_parsed_above, ...)` als Best-Effort (mit gleichem `shortcut_dispatch_handler`); zusätzlich Toast-Wortlaut über i18n-Key klarstellen ("Hotkey-Update fehlgeschlagen, alter Hotkey bleibt aktiv"). Settings stay neu (AC-2 nicht verletzt), Hotkey stays alt (App bleibt funktional). [`hotkey.rs:78-103`] Neuer i18n-Key kann `error.hotkey.update_failed_old_active` heißen oder Bestand `error.hotkey.registration_failed` mit erweitertem Wortlaut.

### Deferred (pre-existing oder Out-of-Scope-Härtung)

- [x] [Review][Defer] **W1 — ADR-0013 §181 sagt „rolls back Settings-Mutation" — Spec/Impl nutzt aber Pre-Validation (kein Rollback)** [`docs/adr/0013-settings-persistence-schema.md:181`] — deferred, ADR-Amendment nötig (eigener Commit, ADR-Convention `feedback_adr_amendment_convention`).
- [x] [Review][Defer] **W2 — Negativ-Pfad-Test: assert dass auf Konflikt kein `settings.changed` fired** — deferred, control-flow-inspection ausreichend für Story-Closure; Regression-Test wäre Härtung.
- [x] [Review][Defer] **W3 — `state::<>()` panic-on-missing in `reregister_hotkey`** [`hotkey.rs:97-98`] — deferred, currently safe (Tauri-Commands fire post-`setup`); defensives `try_state` wäre Härtung wenn jemals pre-setup callable.

