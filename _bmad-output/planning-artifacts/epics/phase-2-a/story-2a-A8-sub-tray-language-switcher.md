---
name: Story 2.A.A8-Sub — Tray-Language-Switcher
phase: 2
wave: A
story_id: "2.A.A8-Sub"
status: ready
dependencies:
  - "2.A.A4"
adr_refs:
  - docs/adr/0013-settings-persistence-schema.md
source_ref: "welle-2-dispatch-plan.md A8-Sub; ADR-0013 Sub-Decision 5"
---

# Story 2.A.A8-Sub: Tray-Language-Switcher

## Outcome

Das Tray-Menü zeigt ein Language-Submenu. Wenn User `ui.language` im Settings-Panel ändert
(via `set_ui_language`-Command aus A4), aktualisiert das Tray-Menü seine Labels live ohne
App-Neustart. Kein eigener Settings-Write durch A8-Sub — nur Reaktion auf `"settings.changed"`-Event.

`TauriSettingsEmitter` (A4) emittiert bei jedem Settings-Write `SettingsChangedEvent { key, new_value }`.
A8-Sub subscribed auf dieses Event im Backend (Tauri `app.listen()`), filtert auf key = `"ui.language"`
und rebuild den Tray-Menu.

## Scope-Fence

**In-Scope:**
- Tray-Menü: Language-Submenu mit den verfügbaren Locales (en/de)
- Backend: `app.listen("settings.changed", ...)` für key = `"ui.language"` → `tray.set_menu()`
- i18n-Keys: mindestens `tray.language_switcher.label` + Locale-Labels (z. B. `tray.language.en`, `tray.language.de`)
- Neue i18n-Keys in `en.json` + `de.json` eintragen

**Nicht-in-Scope:**
- Settings-Write (kein `set_ui_language`-Aufruf durch A8-Sub — nur Listen)
- Frontend-WebView-Locale-Reload → C3
- Second-Language-Axis-Switching (Dictionary-Language / Output-Language) — separates Feature

## Acceptance Criteria

### AC-1 — Language-Submenu im Tray-Menü bei Boot

**Given** Klarvo startet mit einer konfigurierten `ui.language` (z. B. `"de"`)  
**When** Tray-Icon initialisiert wird  
**Then**
- Tray-Menü enthält ein Language-Submenu (z. B. `"Sprache"` / `"Language"` — je nach boot-locale).
- Submenu zeigt die verfügbaren Locales: `"English"` + `"Deutsch"` (oder deren i18n-Labels).
- Aktuelle Locale ist im Submenu visuell erkennbar (z. B. via CheckMenuItem oder Label-Suffix `"✓"`).
- `tray.language_switcher.label` i18n-Key genutzt für den Submenu-Header.

---

### AC-2 — Event-Subscription: `settings.changed` + key-Filter

**Given** `TauriSettingsEmitter.emit_settings_changed("ui.language", "en")` aufgerufen  
**When** `"settings.changed"`-Event emittiert wird  
**Then**
- Backend-Listener filtert auf `key == "ui.language"`.
- Events mit anderem Key (z. B. `"hotkey.slot1.combo"`) werden ignoriert.
- Kein Deadlock, kein Panic im Listener.

---

### AC-3 — Live-Tray-Update nach Language-Wechsel

**Given** User wechselt `ui.language` von `"de"` auf `"en"` via Settings-Panel (A4)  
**When** `"settings.changed"` Event mit `key = "ui.language"`, `new_value = "en"` eingeht  
**Then**
- Tray-Menü wird mit neuer Locale rebuilt: Submenu-Header = englischer `tray.language_switcher.label`-Wert.
- Exit-Label und andere Tray-Labels werden ebenfalls zur neuen Locale aktualisiert
  (alle `tray.*`-Keys re-laden via i18n_table).
- `tray.set_menu(new_menu)` wird aufgerufen (kein App-Neustart).

---

### AC-4 — i18n-Keys registriert und Coverage-Gate-grün

**Given** `cargo xtask lint-events` nach der Story  
**When** G3-Sub-Lint B (Locale-Coverage) läuft  
**Then**
- `tray.language_switcher.label` in `en.json` + `de.json` vorhanden.
- Locale-spezifische Labels (z. B. `tray.language.en`, `tray.language.de`) in beiden Locales vorhanden.
- `cargo xtask lint-events` Exit 0 (kein new orphan key, kein missing key).

---

### AC-5 — Kein Settings-Write durch Tray-Language-Click (Scope-Fence)

**Given** User klickt einen Language-Eintrag im Tray-Submenu  
**When** Menu-Event-Handler läuft  
**Then**
- **Kein `set_ui_language`-Command-Aufruf durch A8-Sub's Menu-Event-Handler.**
  (A8-Sub ist nur reaktiv; aktives Switching via Settings-Panel ist A4/C3-Scope).
- Option A (einfach): Menu-Click triggert `app.emit("settings.changed", ...)`-simulierten Pfad
  via `set_ui_language`-Command.
- Option B (minimal): Menu-Click ist read-only / greyed-out in Phase-2-A; Language-Switch
  ausschließlich über Settings-Panel. Scope-Decision liegt bei Implementierung.
- Whichever Option gewählt: Scope-Fence ist dokumentiert im Commit-Message.

---

## Technical Notes

- `TrayIconBuilder::with_id("klarvo-tray")` (main.rs:334) erzeugt den tray. `tray.set_menu()`
  updated das Menu nach Boot.
- Tauri v2 `app.listen()` für global events: `app_handle.listen("settings.changed", |event| {...})`.
  Listener-Handle muss gespeichert werden (sonst wird er beim Drop entfernt).
- i18n_table ist aktuell als `State<HashMap<String, String>>` managed (main.rs:308).
  Um neue Tray-Labels bei Locale-Wechsel zu laden: i18n_table via State laden + neue Labels
  aus der neuen Locale-Datei lesen (oder i18n_table als `Arc<RwLock<HashMap>>` upgraden).
- Tauri-`CheckMenuItem` (`tauri::menu::CheckMenuItemBuilder`) für die aktuelle Locale markieren.
- Falls `i18n_table`-Update-Mechanismus für C3 designed wird: A8-Sub kann davon profitieren
  (shared reload-logic). Keine Voraussetzung für A8-Sub, aber koordinieren wenn C3 parallel.
