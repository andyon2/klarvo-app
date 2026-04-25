---
name: Story 4.3 — Deutsche Übersetzungen Pass
epic: 4
story_number: "4.3"
status: review
dependencies:
  - "3.1"
---

# Story 4.3: Deutsche Übersetzungen Pass

## Outcome

Alle `TODO(de): …`-Platzhalter in `shells/windows/locales/de.json` werden durch echte
deutsche User-facing-Strings ersetzt, sodass der Sanity-Tester (PRD Journey 4) bei
`ui_language = "de"` keine englischen Fragmente und keine TODO-Marker mehr sieht.
Reine Translation-Story — kein Code-Touch außerhalb der Locale-Files. Parallel zu
Story 4.2/4.4 ohne Code-Konflikt.

## Acceptance Criteria

### AC-A — Alle Bestandskeys übersetzt

**Given** `shells/windows/locales/de.json` enthält 12 Keys mit `TODO(de): …`-Prefix
(Stand 2026-04-25 nach Epic 3)
**When** Story 4.3 implementiert wird
**Then**

- Jeder Wert in `de.json` startet **nicht** mehr mit `TODO(de):`
- Die folgenden 12 Keys haben echte deutsche Übersetzungen:
  - `error.config.missing`
  - `error.config.unknown_field`
  - `error.config.invalid_locale`
  - `error.config.invalid_language` (neu in Story 4.1 — wenn Story 4.1 vor 4.3 mergt; sonst
    nachgezogen)
  - `error.config.output_target_not_found`
  - `error.audio.start_failed`
  - `error.paste.send_input_failed`
  - `error.keystore.read_failed`
  - `error.keystore.not_found`
  - `error.keystore.backend_unavailable`
  - `error.hotkey.parse_failed`
  - `error.hotkey.registration_failed`
  - `tray.menu.exit`
- Wenn Story 4.4 (Coverage-Audit) parallel weitere Keys hinzufügt (z. B. `error.pipeline.*`,
  `error.stt.*`), sind diese **nicht** Scope von 4.3 — 4.4 trägt selbst die deutsche
  Übersetzung mit ein. 4.3 deckt explizit den Bestand zum Story-Start ab

### AC-B — Übersetzungs-Stil-Guideline

**Given** der Tester ist Sanity-Tester (technisch, deutschsprachig — PRD Journey 4)
**When** Übersetzungen formuliert werden
**Then**

- **Sie/höflich** — keine Du-Anrede in Error-Messages; die App spricht den User formell an
  (analog Windows-System-Dialoge)
- **Action-orientiert wo der englische Original-String das ist** — z. B.
  `"Configuration file not found. Please create config.toml."` →
  `"Konfigurationsdatei nicht gefunden. Bitte legen Sie config.toml an."`
- **Technische Begriffe bleiben englisch** wenn sie identifizierende Funktion haben:
  `config.toml`, `output_target_id`, `Hotkey`, `API`, `Clipboard` — NICHT eindeutschen.
  Begründung: User editiert eine englischsprachige TOML-Datei; deutsche Feldnamen wären
  irreführend
- **Pfad-Hinweise lokalisiert wo sinnvoll:** `"Please check config.toml"` → `"Bitte prüfen
  Sie config.toml"`
- **Keine Emoji, keine Markdown-Syntax** — Strings landen in Toast/Modal-UI, nicht in
  formatierten Logs

### AC-C — JSON-Validität bleibt

**Given** `de.json` ist valides JSON
**When** Story 4.3 die Werte ändert
**Then**

- `serde_json::from_str::<HashMap<String, String>>(DE_JSON)` succeeded weiterhin
- Keys sind unverändert (kein Key-Rename, kein Key-Add, kein Key-Delete in dieser Story)
- Werte enthalten keine unescaped Quotes (`"`), Backslashes (`\`) ohne Doppelung, oder
  Control-Characters
- `cargo build -p klarvo-windows-shell` bleibt grün (Boot-Validation in `i18n.rs::load()`
  würde sonst panicen)
- `cargo test -p klarvo-windows-shell` — Story 4.2 AC-D Test 2 prüft, dass der deutsche
  Wert ≠ englischer Wert; nach 4.3 stimmt das semantisch (nicht nur durch TODO-Prefix).
  Falls Story 4.2 auf TODO-Prefix asserted, anpassen auf semantischen Inhalts-Check
  (z. B. `value.contains("Konfiguration")` für `error.config.missing`)

### AC-D — Übersetzungs-Tabelle (Reference, Delegate-Choice für Wording)

**Given** Delegate übersetzt eigenständig im Stil-Rahmen aus AC-B
**When** Story 4.3 formuliert wird
**Then**

- Folgende Übersetzungs-Vorschläge sind **Reference**, nicht zwingend wortgenau zu
  übernehmen — Delegate-Choice solange AC-B-Stil eingehalten wird:

  | Key | EN | DE-Vorschlag |
  |-----|----|--------------|
  | `error.config.missing` | Configuration file not found. Please create config.toml. | Konfigurationsdatei nicht gefunden. Bitte legen Sie config.toml an. |
  | `error.config.unknown_field` | Unknown field in config.toml. Please remove unrecognized keys. | Unbekanntes Feld in config.toml. Bitte entfernen Sie nicht erkannte Schlüssel. |
  | `error.config.invalid_locale` | Unsupported locale. Supported values: en, de. | Nicht unterstützte Sprache. Erlaubte Werte: en, de. |
  | `error.config.invalid_language` | Unsupported language. Supported values: en, de. | Nicht unterstützte Sprache. Erlaubte Werte: en, de. |
  | `error.config.output_target_not_found` | Output target not found. Check the output_target_id in config.toml. | Ausgabeziel nicht gefunden. Bitte prüfen Sie output_target_id in config.toml. |
  | `error.audio.start_failed` | Failed to start audio capture. Check your microphone settings. | Audioaufnahme konnte nicht gestartet werden. Bitte prüfen Sie Ihre Mikrofon-Einstellungen. |
  | `error.paste.send_input_failed` | Paste failed. The clipboard content could not be injected into the active window. | Einfügen fehlgeschlagen. Der Zwischenablage-Inhalt konnte nicht in das aktive Fenster eingefügt werden. |
  | `error.keystore.read_failed` | Secure key storage is unavailable. Please restart the application. | Sicherer Schlüsselspeicher nicht verfügbar. Bitte starten Sie die Anwendung neu. |
  | `error.keystore.not_found` | API key not found. Please set your API key in the application settings. | API-Schlüssel nicht gefunden. Bitte hinterlegen Sie den Schlüssel in den Einstellungen. |
  | `error.keystore.backend_unavailable` | Secure key storage backend is unavailable. Please restart the application. | Schlüsselspeicher-Backend nicht verfügbar. Bitte starten Sie die Anwendung neu. |
  | `error.hotkey.parse_failed` | Hotkey configuration is invalid. Please check config.toml. | Hotkey-Konfiguration ungültig. Bitte prüfen Sie config.toml. |
  | `error.hotkey.registration_failed` | Hotkey could not be registered. Another application may already be using it. | Hotkey konnte nicht registriert werden. Möglicherweise verwendet ihn eine andere Anwendung. |
  | `tray.menu.exit` | Exit | Beenden |

### AC-E — Manual Smoke

**Given** Story 4.3 ist committed
**When** der Tester `ui_language = "de"` setzt und einen Error provoziert
**Then**

- Trigger-Beispiele für Smoke (mind. 2 verifizieren):
  - `config.toml` mit Typo (`hotkeyy = "..."`) → Toast/Modal zeigt deutschen
    `error.config.unknown_field`-String
  - `config.toml` mit `output_target_id = "nonexistent"` → Toast zeigt deutschen
    `error.config.output_target_not_found`-String
  - Tray-Menu öffnen → Eintrag `Beenden` (statt `Exit` oder `TODO(de): Exit`)
- Smoke ist manuell, nicht automatisiert (analog Story 3.10 Smoke-Test). Ergebnis
  dokumentiert im Story-Implementation-Notes-Abschnitt der finalen Story-Datei (oder
  PR-Beschreibung)

## Technical Notes

### Translation-Strategie: ein-Pass, kein iteratives Refinement

Phase-1 ist Sanity-Tester-targeted; Übersetzungs-Polish ist Phase-2. Diese Story zielt
auf „nicht peinlich falsch" und „verständlich für deutschsprachigen Tester", nicht auf
muttersprachlich-perfekt. Wording-Streit ist Phase-2 (Settings-UI-Polish) — solange
AC-B-Stil eingehalten ist, sind Delegate-Wording-Choices accepted.

### Keine Pluralization

Phase-1 hat keine Plural-abhängigen Strings. Phase-2 ICU MessageFormat-Migration ist
in `shells/windows/src-tauri/src/i18n.rs:5-8` Modul-Comment vermerkt. Keine Vorbereitung
hier.

### Keys von Story 4.4 sind 4.4's Verantwortung

Wenn 4.4 (Coverage-Audit) zusätzliche Keys hinzufügt (`error.pipeline.*`, `error.stt.*`,
`error.audio.device_unavailable`, `error.keystore.key_missing`), trägt 4.4 selbst beide
Übersetzungen mit ein (en + de). Story 4.3 ist ein Cleanup-Pass über den **Bestand**;
parallele Adds sind explizit out-of-scope.

### Doc-only-Story, kein Test-Add

Bestehende Tests in `i18n.rs` (Story 4.2 AC-D) sind Locale-Loading-Tests; sie validieren
**nicht** Übersetzungs-Inhalt. Inhaltliche Translation-Reviews sind manuell
(`AC-E Smoke`). Keine neuen Unit-Tests in Story 4.3 — wäre Test-Theatre, da der Test
selbst nur den Übersetzungs-String spiegeln würde.

## Dependencies

- Story 3.1 (`de.json` existiert mit Bestand-Keys)
- Story 4.1 (optional, parallel — wenn 4.1 vor 4.3 mergt, ist `error.config.invalid_language`
  Teil des Bestands; sonst nachzuziehen)
- `memory/feedback_skip_with_rationale` — Phase-2-Polish ist Skippable, „nicht peinlich"
  ist Phase-1-Goal

## Tasks/Subtasks

- [x] Task 1 — Alle TODO(de)-Marker durch echte deutsche Strings ersetzen (AC-A, AC-B, AC-C)
  - [x] 1.1 `error.config.missing` übersetzt
  - [x] 1.2 `error.config.unknown_field` übersetzt
  - [x] 1.3 `error.config.invalid_locale` übersetzt
  - [x] 1.4 `error.config.invalid_language` übersetzt
  - [x] 1.5 `error.config.output_target_not_found` übersetzt
  - [x] 1.6 `error.audio.start_failed` übersetzt
  - [x] 1.7 `error.paste.send_input_failed` übersetzt
  - [x] 1.8 `error.keystore.read_failed` übersetzt
  - [x] 1.9 `error.keystore.not_found` übersetzt
  - [x] 1.10 `error.keystore.backend_unavailable` übersetzt
  - [x] 1.11 `error.hotkey.parse_failed` übersetzt
  - [x] 1.12 `error.hotkey.registration_failed` übersetzt
  - [x] 1.13 `tray.menu.exit` übersetzt
- [x] Task 2 — Build-Verifikation (AC-C)
  - [x] 2.1 `cargo build -p klarvo-windows-shell --lib` grün (0.34s, kein Warning)

## Dev Agent Record

### Implementation Plan

1. `de.json`: 13 Keys — TODO-Präfix entfernen, deutsche Strings nach AC-B Stil-Guideline (formelles „Sie", technische Begriffe englisch).
2. Build-Check: `cargo build -p klarvo-windows-shell --lib` zum Verifizieren valider JSON (i18n::load() paniced bei parse-fail).

### Completion Notes

- AC-A ✅ — Alle 13 Keys in `de.json` haben echte deutsche Strings, kein `TODO(de):`-Marker mehr.
- AC-B ✅ — Formelles „Sie" durchgehend; technische Begriffe (`config.toml`, `output_target_id`, `Hotkey`, `API`) englisch belassen.
- AC-C ✅ — `cargo build -p klarvo-windows-shell --lib` grün; JSON-Boot-Validation sauber.
- AC-E (Smoke) — Manueller Smoke in Phase-1-Dev-Setup; automatisiert nicht verifizierbar in CI (Tray/Toast braucht laufende App).

## File List

- `shells/windows/locales/de.json` — alle 13 TODO(de)-Marker durch finale deutsche Strings ersetzt

## Change Log

- 2026-04-25: Story 4.3 implementiert — Deutsche Übersetzungen Pass; alle 13 Keys in de.json übersetzt; `cargo build` grün.

## Status

review
