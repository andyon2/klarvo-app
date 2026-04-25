# Klarvo — Sanity-Tester Onboarding

_Zielgruppe: Nicht-Developer-Tester, die Klarvo auf einem Windows-PC manuell prüfen._
_Sprache: Deutsch — weil Journey 4 (Sanity-Tester) explizit deutschsprachig ist._
_Dev-Setup (cargo tauri dev, CI etc.) → [Entwicklungs-Guide](./development-guide.md)._

---

## Voraussetzungen

- **Windows 10 oder 11** (x64)
- Mikrofon vorhanden und als Standard-Aufnahmegerät eingestellt
- Kein vorheriger Klarvo-Zustand (`%APPDATA%\Klarvo\` leer oder nicht vorhanden)
- Build-Binärdatei von der zuständigen Person erhalten **oder** selbst gebaut (→ Schritt 1)

---

## Cold-Start-Pfad

### Schritt 1 — Installation / Binärdatei beschaffen

Phase-1 hat **keinen signierten Installer** (→ Backlog: _Signierter Installer / MSI-Distribution_).
Zwei Wege:

**Option A — Binary vom Entwickler erhalten**

```
klarvo.exe  (Einzeldatei, direkt ausführbar)
```

**Option B — Lokal bauen** (setzt Rust-Toolchain + Windows-Build-Tools voraus):

```powershell
# im Repo-Wurzelverzeichnis
cargo build -p klarvo-windows-shell --release
```

Ausgabepfad: `shells/windows/src-tauri/target/release/klarvo.exe`

> Hinweis: Das v1-Build-Skript `scripts/sync-and-build.ps1` baut die alte v1-Shell —
> für Phase-2-Tests bitte Option B oder eine vorgefertigte Binary verwenden.

---

### Schritt 2 — Erster App-Start (ohne config.toml)

1. `klarvo.exe` doppelklicken (kein Installer, kein Setup-Wizard)
2. Die App startet mit **Standard-Werten**:

| Einstellung | Phase-1-Default |
|---|---|
| `hotkey` | `CommandOrControl+Shift+Space` |
| `output_target_id` | `clipboard` |
| `ui_language` | `en` |
| `dictionary_language` | `en` |
| `output_language` | `en` |

Es erscheint **kein Splash-Screen und kein Hauptfenster** — Klarvo lebt im Tray.

---

### Schritt 3 — Tray-Icon prüfen

1. Tray-Icon (Systembereich, rechts unten in der Taskleiste) ist sichtbar → **Idle-State**
2. Rechtsklick auf das Icon → Kontextmenü zeigt:

```
Klarvo
──────
Exit
```

_(englisch, weil Default-Locale `en`)_

---

### Schritt 4 — Hotkey-Probe

Dieser Schritt prüft, dass die Aufnahme-Pipeline triggert.

> **Phase-1-Hinweis:** Die aktuelle Pipeline enthält noch keine Spracherkennung
> (Groq-STT-Plugin ist in Phase 1 noch nicht verdrahtet — siehe Backlog:
> _Groq-STT in Windows-Shell verdrahten_). Der Hotkey-Cycle zeigt trotzdem die
> korrekten Tray-Zustandsübergänge; es wird jedoch kein Text in die Zwischenablage
> geschrieben.

Ablauf:

1. Hotkey `Ctrl+Shift+Space` drücken und **halten**
   → Tray-Icon wechselt auf **Recording-State** (Aufnahme läuft)
2. Hotkey loslassen
   → Tray-Icon wechselt kurz auf **Processing-State**, dann zurück auf **Idle-State**
3. Zwischenablage: leer / unverändert (erwartetes Phase-1-Verhalten)

---

### Schritt 5 — config.toml anlegen

Datei manuell anlegen:

```
%APPDATA%\Klarvo\config.toml
```

_(falls das Verzeichnis `%APPDATA%\Klarvo\` noch nicht existiert: manuell anlegen)_

Inhalt:

```toml
hotkey = "CommandOrControl+Shift+Space"
output_target_id = "clipboard"
ui_language = "de"
dictionary_language = "de"
output_language = "de"
```

---

### Schritt 6 — App-Neustart, deutsche UI prüfen

1. Klarvo beenden: Rechtsklick im Tray → **Exit**
2. `klarvo.exe` erneut starten
3. Rechtsklick im Tray → Kontextmenü zeigt jetzt:

```
Klarvo
──────
Beenden
```

_(„Beenden" statt „Exit" — `ui_language = "de"` wurde geladen)_

---

### Schritt 7 — API-Key-Setup für Groq (Phase-1-Einschränkung)

> **Phase-1-Status:** Groq-STT ist noch nicht in die Windows-Shell verdrahtet.
> Der API-Key kann bereits hinterlegt werden, hat aber noch keinen Effekt auf
> das Diktat-Ergebnis.

**Release-Build** — Schlüssel im Windows Credential Manager speichern:

1. Windows-Taste → „Anmeldeinformationsverwaltung" öffnen
2. „Windows-Anmeldeinformationen" → „Generische Anmeldeinformationen hinzufügen"
3. Felder:
   - **Internetadresse oder Netzwerkadresse:** `klarvo/groq_api_key`
   - **Benutzername:** (beliebig, z. B. `klarvo`)
   - **Kennwort:** dein Groq-API-Key (`gsk_...`)

**Dev-Build** (mit Feature `dev-plain-keystore`) — SQLite-Datei:

```
%APPDATA%\Klarvo\keystore.db
```

_(wird von der App beim Start angelegt; direkte SQL-Bearbeitung nur für Entwickler)_

Vollständiger `cargo xtask set-key`-CLI-Subcommand ist Phase-2-Backlog →
[docs/backlog.md](./backlog.md) Sektion „Phase 2".

---

## Smoke-Checklist

Manuell abhaken. Mindestens die markierten (**Pflicht**) Items vor Story-4.5-Closure ausführen.

### Basis-Start

- [ ] **[Pflicht]** App startet ohne `config.toml` — kein Crash, Tray-Icon erscheint im Idle-State
- [ ] Tray-Kontextmenü zeigt englische Labels (`Exit`) bei `ui_language`-Default (`en`)

### Hotkey-Zyklus

- [ ] **[Pflicht]** Hotkey-Press (`Ctrl+Shift+Space`) → Tray-Icon wechselt auf Recording-State
- [ ] Hotkey-Release → Tray-Icon wechselt (kurze Processing-Phase) zurück auf Idle-State
- [ ] Kein App-Crash beim wiederholten Hotkey-Zyklus (3×)

### Lokalisierung

- [ ] **[Pflicht]** `config.toml` mit `ui_language = "de"` → Tray-Kontextmenü zeigt „Beenden"
- [ ] `config.toml` mit `ui_language = "fr"` → App startet trotzdem (fail-soft), nutzt Default-Sprache (`en`); Fehler im Log sichtbar, kein blockierender Crash

### Fehler-Nachrichten (deutsch)

- [ ] **[Pflicht]** Config mit Tippfehler-Schlüssel (z. B. `hotkeyy = "..."`) →
  Toast/Modal zeigt deutschen Text für `error.config.unknown_field`:
  _„Unbekanntes Feld in config.toml. Bitte entfernen Sie nicht erkannte Schlüssel."_

- [ ] Config mit `output_target_id = "doesnotexist"` →
  Toast zeigt `error.config.output_target_not_found`:
  _„Ausgabeziel nicht gefunden. Bitte prüfen Sie output_target_id in config.toml."_

- [ ] Hotkey belegt von anderer App (Konflikt) →
  Toast zeigt `error.hotkey.registration_failed`:
  _„Hotkey konnte nicht registriert werden. Möglicherweise verwendet ihn eine andere Anwendung."_

### Optional (wenn Groq verdrahtet)

- [ ] Groq-API-Key hinterlegt + Diktat → transkribierter Text erscheint in Zwischenablage
- [ ] Kein API-Key hinterlegt → Toast zeigt `error.stt.key_not_configured`:
  _„API-Schlüssel nicht konfiguriert. Bitte hinterlegen Sie Ihren Schlüssel in den Einstellungen."_

---

## Bekannte Phase-1-Einschränkungen

| Einschränkung | Backlog |
|---|---|
| Keine Settings-UI — Config wird manuell als TOML editiert | [Phase 2: Minimales Settings-Panel](./backlog.md#minimales-settings-panel) |
| Kein Live-Locale-Switch — `ui_language` wird nur beim App-Start gelesen (kein Hot-Reload) | [Phase 2: Live-Locale-Switch](./backlog.md#live-locale-switch-hot-reload) |
| Kein signierter Installer — lokaler Build oder direkte Binary-Distribution | [Phase 2: Signierter Installer / MSI-Distribution](./backlog.md#signierter-installer--msi-distribution) |
| Hotkey-Konflikte mit anderen Apps — User muss Hotkey in `config.toml` manuell anpassen | [Phase 2: Hotkey-Konflikt-Erkennung](./backlog.md#hotkey-konflikt-erkennung) |
| API-Key über Credential Manager / SQLite, kein UI-Setup-Wizard | [Phase 2: `cargo xtask set-key`](./backlog.md#cargo-xtask-set-key-keystore-cli) |
| Keine echte Spracherkennung in Phase-1 (Groq-Plugin noch nicht verdrahtet) | _Phase-2-Wiring (Backlog-Eintrag in Vorbereitung)_ |
| Kein Zip-Export für Logs / Debug-Daten | [Phase 2: Debug-Export-Zip](./backlog.md#debug-export-zip-settings-ui-gebunden) |

---

## Was du beim Bug-Report angeben solltest

1. **OS-Version**: `winver` in PowerShell ausführen (z. B. Windows 11 22H2 Build 22621)
2. **Klarvo-Build-Hash**: lässt sich aus dem Binary-Name oder dem Git-Commit des Builds ableiten
   _(Phase-1: vom Entwickler erfragen; automatischer Build-Hash in Settings-UI ist Phase-2)_
3. **Inhalt von `%APPDATA%\Klarvo\config.toml`** — API-Keys bitte schwärzen/weglassen
4. **Steps-to-Reproduce**: Konkrete Schritte, nicht nur das Symptom
5. **Tray-Zustand zum Zeitpunkt des Fehlers** (Idle / Recording / Processing)
6. **Log-Datei** (optional): Unter `%APPDATA%\Klarvo\logs\` (Phase-1-Status: Pfad ggf. noch nicht
   aktiv — beim Entwickler nachfragen). Automatischer Zip-Export via Settings-UI ist
   Phase-2-Backlog; kein Sentry/Remote-Telemetry (Design-Entscheidung, s. Architektur).

---

## Weitere Dokumentation

- [docs/index.md](./index.md) — Repo-Einstiegspunkt (alle Docs im Überblick)
- [docs/development-guide.md](./development-guide.md) — Dev-Setup (`cargo tauri dev`, Tests, CI)
  _(Persona-Abgrenzung: dort steht, wie Entwickler das Projekt bauen und testen — nicht dieser Guide)_
- [docs/backlog.md](./backlog.md) — Phase-2-Features die in Phase-1 fehlen
