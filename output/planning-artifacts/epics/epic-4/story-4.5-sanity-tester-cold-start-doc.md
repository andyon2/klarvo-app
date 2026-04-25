---
name: Story 4.5 — Sanity-Tester Cold-Start Doku (PRD Journey 4)
epic: 4
story_number: "4.5"
status: Draft
dependencies:
  - "4.1"
  - "4.2"
  - "4.3"
  - "4.4"
---

# Story 4.5: Sanity-Tester Cold-Start Doku

## Outcome

Ein dokumentierter Sanity-Tester-Onboarding-Pfad (PRD Journey 4) beschreibt
Schritt-für-Schritt: Frischer Windows-PC ohne `%APPDATA%\Klarvo\` → App-Start mit Defaults
→ User legt minimale `config.toml` mit `ui_language = "de"` an → User registriert
Groq-API-Key → erster Dictation-Cycle → User sieht deutsche Errors bei Fehlkonfiguration.
Reine Doku-Story ohne Code-Touch (gemäß User-Approval 2026-04-25). Smoke-Checklist
wird Teil von `docs/development-guide.md` oder einer neuen
`docs/sanity-tester-onboarding.md`.

## Acceptance Criteria

### AC-A — Doku-Lokation

**Given** das Repo hat `docs/development-guide.md` als Dev-Onboarding-Doc
**When** Story 4.5 die Sanity-Tester-Doku verfasst
**Then**

- Neue Datei: `docs/sanity-tester-onboarding.md`. Begründung: Sanity-Tester ≠ Developer;
  Mischung in `development-guide.md` würde Persona-Achsen vermischen
  (`memory/project_phase1_trait_narrowing` — Persona-Trennung als Architektur-Prinzip)
- Datei wird in `docs/index.md` referenziert (kurzer Eintrag unter „Onboarding" oder
  „Operations"-Sektion, je nach existierender Struktur)
- Sprache der Doku: **deutsch**, da Zielgruppe der deutschsprachige Sanity-Tester ist
  (PRD Journey 4 deckt explizit den deutschsprachigen Onboarding-Pfad ab)

### AC-B — Cold-Start-Pfad dokumentiert

**Given** ein Tester startet auf einem frischen Windows-Rechner ohne Klarvo-State
**When** die Doku den Pfad führt
**Then**

- Schritt 1 — Installation: Verweis auf Build-Artefakt-Bezugspfad (oder explizite
  Notiz, dass der Tester ein lokales `cargo tauri build` ausführt — Phase-1 hat keinen
  signed Installer, das ist Backlog)
- Schritt 2 — Erster App-Start: Beschreibt was passiert ohne `config.toml` (App startet
  mit Defaults: `hotkey = "CommandOrControl+Shift+Space"`, `output_target_id = "clipboard"`,
  alle drei Languages = `en`)
- Schritt 3 — Verifikation Tray-Icon: Tray-Icon ist sichtbar (Idle-State); Klick auf
  Tray → Menu zeigt „Klarvo" + „Exit" (englisch, da Default-Locale)
- Schritt 4 — Hotkey-Probe: User drückt Hotkey kurz; Tray-Icon wechselt auf Recording-State
  und zurück (sichtbar, dass die Pipeline triggert — auch ohne API-Key, weil Phase-1
  die Verbatim-Plugin-Default-Pipeline nutzt, ref `klarvo-windows-shell::main::build_plugin_registry`)
- Schritt 5 — Config-File anlegen: Tester legt manuell `%APPDATA%\Klarvo\config.toml` an
  mit Beispiel-Inhalt:
  ```toml
  hotkey = "CommandOrControl+Shift+Space"
  output_target_id = "clipboard"
  ui_language = "de"
  dictionary_language = "de"
  output_language = "de"
  ```
- Schritt 6 — App-Restart, Verifikation deutsche UI: Nach Restart zeigt Tray-Menu
  „Beenden"; bei provoziertem Error (z. B. Config-Typo) zeigt Toast/Modal deutschen Text
- Schritt 7 — API-Key-Setup für Groq-Pipeline: Verweis auf Phase-1-API-Key-Setup-Path
  (xtask oder manueller Credential-Manager-Eintrag — Delegate konsultiert
  `memory/project_keystore_trait_surface` und `memory/project_api_key_os_keystore_mvp`
  für aktuellen Stand). Wenn Phase-1 nur einen Stub-Path hat, dokumentiert die Doku
  das ehrlich („Phase-1: API-Key-Setup ist Developer-Tool-Pfad — siehe `xtask` oder
  Setup-Skript"); kein Fake einer Settings-UI, die nicht existiert

### AC-C — Smoke-Checklist (Verifikations-Liste)

**Given** der Tester soll prüfen, dass die App in einem definierten Zustand ist
**When** die Doku eine Checkliste anhängt
**Then**

- Checkliste mit Pass/Fail-Items, manuell abhakbar:
  - [ ] App startet ohne `config.toml` (kein Crash, Tray-Icon erscheint)
  - [ ] Tray-Menu zeigt englische Labels bei `ui_language` Default (`en`)
  - [ ] Hotkey-Press wechselt Tray-Icon-State auf Recording (rot/aktiv)
  - [ ] Hotkey-Release wechselt Tray-Icon-State auf Idle zurück (nach kurzer
    Processing-Phase)
  - [ ] Config mit `ui_language = "de"` führt zu deutschem Tray-Menu („Beenden")
  - [ ] Config mit Typo (z. B. `hotkeyy = "..."`) → Toast/Modal zeigt deutschen
    `error.config.unknown_field`-Text
  - [ ] Config mit `output_target_id = "doesnotexist"` → Toast zeigt deutschen
    `error.config.output_target_not_found`-Text
  - [ ] Config mit `ui_language = "fr"` → App startet trotzdem (fail-soft) und nutzt
    Default-Sprache; Error wird im Log sichtbar (nicht blockierend für App-Start)
- Optional: zusätzliche Items für Groq-Wire-Up, wenn Phase-1 das bereits enthält
  (`memory/project_phase1_complete` referenziert Epic 2 — Delegate prüft, ob Groq-Plugin
  aktiv ist und Smoke-Path testbar ist)

### AC-D — Bekannte Gaps benannt

**Given** Phase-1 ist nicht feature-komplett aus Tester-Sicht
**When** die Doku ehrlich dokumentiert
**Then**

- Sektion „Bekannte Phase-1-Einschränkungen" listet:
  - Keine Settings-UI — Config wird manuell editiert (Phase-2-Backlog)
  - Kein Live-Locale-Switch — Sprache wird nur beim Boot gelesen (Phase-2-Backlog,
    Story 4.2 AC-E)
  - Kein signed Installer — lokaler Build oder Dev-Mode-Distribution
  - Hotkey-Konflikte mit anderen Apps — User muss config selbst anpassen, kein
    Auto-Detect
  - API-Key wird über Credential-Manager gespeichert; kein UI-Setup-Wizard. Verweis
    auf Backlog-Eintrag wenn vorhanden
- Sektion „Was du beim Bug-Report angeben solltest" mit Items:
  - OS-Version, Klarvo-Build-Hash
  - Inhalt von `%APPDATA%\Klarvo\config.toml` (mit redacted API-Keys, falls vorhanden —
    Memory-Hinweis: keine Keys in Logs)
  - Steps-to-Reproduce
  - Optional Trace-Log (`memory/project_no_remote_telemetry` — Local-Logs,
    User-triggered Zip-Export wenn vorhanden, sonst Phase-2-Backlog)

### AC-E — Cross-Reference zu existierender Doku

**Given** das Repo hat bereits `docs/development-guide.md`, `docs/index.md` und
`docs/backlog.md`
**When** Story 4.5 die Doku verfasst
**Then**

- `docs/sanity-tester-onboarding.md` verweist explizit auf:
  - `docs/index.md` als Repo-Einstiegspunkt
  - `docs/development-guide.md` für Dev-Setup (klare Persona-Abgrenzung: dort steht
    `cargo tauri dev`, hier nicht)
  - `docs/backlog.md` für „Phase-2-Features, die in Phase-1 fehlen" — kein Duplikat
    der Backlog-Items, nur Verweis auf Backlog-Section-Anchor
- `docs/index.md` enthält neuen Eintrag, der auf
  `docs/sanity-tester-onboarding.md` zeigt. Eintrag-Wording ist Delegate-Choice;
  Beispiel: `- [Sanity-Tester Onboarding](sanity-tester-onboarding.md) — erste
  Schritte für nicht-Developer-Tester (deutsch)`

### AC-F — Kein Code-Touch

**Given** Story 4.5 ist explizit Doc-only (User-Approval 2026-04-25)
**When** der Implementation-PR aufgebaut wird
**Then**

- Keine Änderungen an `*.rs`, `*.ts`, `*.tsx`, `Cargo.toml`, `package.json`, oder
  ähnlichem Code-Artefakt
- Keine neuen Tests
- Erlaubte Touches: `docs/sanity-tester-onboarding.md` (neu), `docs/index.md`
  (Eintrag), optional `docs/backlog.md` (wenn 4.5 beim Verfassen Lücken aufdeckt
  und Phase-2-Items hinzugefügt werden)
- Diff-Review prüft: keine `*.rs`-Datei im Patch (`git diff --stat -- '*.rs'` ist
  leer)

### AC-G — Manual-Smoke-Run im Story-Closure

**Given** die Doku ist verfasst
**When** Story 4.5 als Done markiert werden soll
**Then**

- Eine Person folgt der Doku end-to-end auf einem (idealerweise frischen oder
  bereinigten) Windows-Setup. Reicht für Phase-1 ein bereinigter `%APPDATA%\Klarvo\`-Ordner
  ohne separate Maschine
- Mindestens drei Smoke-Checklist-Items (AC-C) werden tatsächlich ausgeführt:
  - First-Start ohne Config → englischer Default-State
  - Config mit `ui_language = "de"` → Tray-Menu „Beenden"
  - Config mit Typo → deutscher Error-Toast
- Ergebnis (Pass/Fail mit Notizen) wird in der Story-Implementation-Notes oder PR-Body
  vermerkt. Bei Fail: Doku wird angepasst bevor Story als Done markiert wird

## Technical Notes

### Doku-Sprache deutsch — bewusste Entscheidung

PRD Journey 4 (Sanity-Tester) ist explizit deutschsprachig adressiert; eine englische
Onboarding-Doku wäre Persona-Mismatch. `docs/development-guide.md` und andere
Dev-Docs bleiben englisch (Developer-Persona ist multi-lingual). Die Sprach-Achse
des Dokuments folgt der Persona-Achse — analog `memory/project_i18n_three_axes`
auf Doku-Ebene.

### Doc-only-Begründung

User hat 2026-04-25 explizit „doc-only" approved. Begründung: Coverage-Test (Story 4.4)
deckt mechanisch alle i18n-Keys ab; Story 4.5 ist Persona-Onboarding, nicht
Test-Coverage. Code-Test einer Cold-Start-Pfads würde voraussetzen, dass eine
Test-Harness `%APPDATA%\Klarvo\` löschen kann — RAII-Guard-Pattern mit echtem OS-State
(`memory/feedback_test_raii_cleanup_pattern`) wäre möglich, ist aber für Persona-Doku
overengineered.

### Verlinkung statt Duplikation

Die Doku verlinkt Backlog-Items statt sie zu duplizieren. Begründung
`memory/feedback_backlog_discipline`: `docs/backlog.md` ist Single-Source-of-Truth.
Wenn 4.5 beim Verfassen entdeckt, dass ein Phase-2-Feature noch nicht im Backlog ist
(z. B. „Settings-UI" oder „API-Key-Setup-Wizard"), wird der Backlog-Eintrag in
derselben PR mit angefügt — mit Source-Ref `Source: Story 4.5 sanity-tester-onboarding.md`.

### Story-Reihenfolge: 4.5 nach 4.1–4.4

4.5 dokumentiert Verhalten, das von 4.1–4.4 hergestellt wird. Ein Sanity-Tester-Doku, die
auf nicht-existierender deutscher Übersetzung basiert (vor 4.3) oder fehlenden i18n-Keys
(vor 4.4), würde Tester direkt in Bugs schicken. Daher hartes Dependency-Constraint
auf alle vier Vorgänger-Stories — auch wenn 4.5 selbst Doc-only ist.

## Dependencies

- Story 4.1 (3-Achsen-Schema in `ShellConfig` — Doc benutzt das in Schritt 5)
- Story 4.2 (Locale-aware Loading — Doc verlässt sich darauf, dass `ui_language = "de"`
  funktioniert)
- Story 4.3 (Deutsche Übersetzungen vorhanden — Doc verlässt sich darauf, dass Tester
  echte Strings sieht, keine TODOs)
- Story 4.4 (Coverage-Audit — Doc verlässt sich darauf, dass alle i18n-Keys auflösen,
  keine Raw-Keys im Toast erscheinen)
- `memory/project_no_remote_telemetry` — Bug-Report-Section verlinkt auf Local-Logs,
  nicht Sentry/Telemetry
- `memory/feedback_backlog_discipline` — keine Backlog-Duplikate, nur Verweise
- PRD Journey 4 — Source der Persona-Definition
