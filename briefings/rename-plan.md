# Rename-Plan: Dikta → Voxlit

Erstellt: 2026-03-21. Status: Geplant, noch nicht begonnen.

## Vorab-Entscheidungen (Andy)

| # | Frage | Entscheidung |
|---|-------|-------------|
| 1 | Repo-Verzeichnis `~/claude-projects/dikta/` → `~/claude-projects/voxlit/`? | **Ja** |
| 2 | Windows-Pfad `D:\Apps\dikta\` → `D:\Apps\voxlit\`? | **Ja** |
| 3 | GitHub-Repos umbenennen? | **`dikta` → `voxlit`, `dikta-public` → `voxlit-app`** |
| 4 | Keystore nur umbenennen oder neu generieren? | **Nur umbenennen** |
| 5 | AppData-Migration (alte Settings/History mitnehmen)? | **Nein, spaeter** |
| 6 | Social Preview / Logo fuer neues Repo? | **Bestehendes Logo weiterverwenden, Social Preview mit "Voxlit" Text neu erstellen** |

## Kritisch: License-Key-Format

Aktuell: `DIKTA-XXXX-XXXX-XXXX-XXXX` mit HMAC-Secret `b"dikta-license-v1"`.
Neu: `VOXLIT-XXXX-XXXX-XXXX-XXXX` mit `b"voxlit-license-v1"`.
Alle bestehenden DIKTA-Keys werden ungueltig. Pre-Launch unkritisch — keine bezahlenden Nutzer.
VOXLIT hat 6 Zeichen statt 5 → maxLength im Frontend von 25 auf 26 anpassen.

## Tasks (in Reihenfolge)

### Phase 1 — Parallel ausfuehrbar

**Task 1: Konfig-Dateien** (rust-core)
- `src-tauri/Cargo.toml`: `name = "dikta"` → `"voxlit"`, `name = "dikta_lib"` → `"voxlit_lib"`
- `package.json`: `"name": "dikta"` → `"voxlit"`
- `src-tauri/tauri.conf.json`: productName, identifier (`com.dikta.voice` → `com.voxlit.voice`), Window-title, Updater-Endpoint URL

**Task 2: Rust-Backend-Strings** (rust-core)
- `main.rs`: `dikta_lib::run()` → `voxlit_lib::run()`
- `lib.rs`: Window-Titel ("Dikta" → "Voxlit"), Tray-ID, Event-Praefix `dikta://` → `voxlit://`
- `hotkey/mod.rs`: EVENT_STATE_CHANGED
- `commands/whisper.rs`: 3 Event-Konstanten `dikta://model-download-*`
- `commands/settings.rs`: Registry-Value `"Dikta\0"` → `"Voxlit\0"`
- `commands/dictionary.rs`: User-facing Strings
- `pipeline.rs`: AppData-Pfad `com.dikta.voice`
- `stt/model_manager.rs`: Test-Pfade `/tmp/dikta-*`

**Task 3: License-Modul** (rust-core)
- `src-tauri/src/license/mod.rs`: HMAC-Secret, alle `"DIKTA"` Literale, Test-Keys

**Task 4: React-Frontend** (ui-dev)
- `src/tauri-commands.ts`: alle `"dikta://"` Events
- `src/components/SettingsPanel.tsx`: URL dikta.app → voxlit.app, License-Praefix, GitHub-Links, Footer
- `src/components/AdvancedSettingsPanel.tsx`: "Requires Dikta License"
- `src/FloatingBar.tsx`: DiktaLogo → VoxlitLogo, Events
- `src/Onboarding.tsx`: User-facing Strings
- `src/hooks/useQuickTip.ts`: localStorage-Key `"dikta_install_day"`

**Task 5: Android Kotlin-Quellen** (android-platform)
- Verzeichnis `android/kotlin-src/com/dikta/` → `com/voxlit/`
- Klassen: DiktaApi → VoxlitApi, DiktaOverlayService → VoxlitOverlayService, etc.
- Package-Deklarationen, Notification-Channel-ID, SharedPrefs, Action-Intents

### Phase 2 — Sequentiell nach Phase 1

**Task 6: Android gen/-Dateien** (android-platform)
- `build.gradle.kts`: namespace + applicationId
- `AndroidManifest.xml`: Service-Namen, Theme
- `strings.xml` (gen + android/res-values): app_name
- `themes.xml` (beide): Style-Name

**Task 7: Build-Skripte** (rust-core oder direkt)
- `scripts/android-build.sh`: Kotlin-Pfade, APK-Name, Keystore-Name
- `scripts/sign-installer.sh`: Pfade, Installer-Dateiname, Key-Pfad
- `scripts/publish.sh`: Excludes, Clone-URL, Scrub-Regex
- `scripts/sync-and-build.ps1`: $dst, Dropbox-Pfad, WSL-Pfad

**Task 8: Keystore umbenennen** (direkt)
- `mv dikta-debug.keystore voxlit-debug.keystore`
- Passwort im Script anpassen (dikta123 → voxlit123 oder beibehalten)

**Task 9: Resources + LICENSE** (direkt)
- `src-tauri/resources/README.txt`: App-Name, AppData-Pfad, GitHub-URL
- `src-tauri/resources/RELEASE-NOTES.txt`: Header
- `LICENSE`: "Licensed Work: Dikta" → "Voxlit"

**Task 10: README.md** (direkt)
- Alle "Dikta" → "Voxlit", GitHub-Links, Domain

### Phase 3 — Infrastruktur (nach Phase 2)

**Task 11: Agent-Infrastruktur + Knowledge** (direkt)
- `CLAUDE.md`, `main-agent.md`, `.claude/agents/*.md`, Skills
- `knowledge/architecture.md`: Repo-Architektur, Updater-Endpoint, AppData-Pfade
- `knowledge/product-strategy.md`, `project-status.md`
- Briefings aktualisieren oder archivieren

**Task 12: Verzeichnis- und Repo-Pfade** (direkt, ZULETZT)
- Repo-Verzeichnis umbenennen (wenn entschieden)
- Windows-Pfad umbenennen (wenn entschieden)
- GitHub-Repo umbenennen (wenn entschieden)
- Social Preview Image setzen
- sync-and-build.ps1 WSL-Pfade final anpassen

## Testplan

- [ ] `cargo check` nach Phase 1 Tasks 1-3
- [ ] `npm run build` nach Task 4
- [ ] `grep -r "dikta://" src/ src-tauri/src/` → 0 Treffer
- [ ] `grep -ri "dikta" src/ src-tauri/src/ android/kotlin-src/` → nur deutsches Wort "Diktat"
- [ ] `grep -r "dikta-public" .` → 0 Treffer
- [ ] License-Key: DIKTA-Key wird abgelehnt, VOXLIT-Key akzeptiert
- [ ] Windows-Build: Installer heisst `Voxlit_X.Y.Z_x64-setup.exe`
- [ ] Android-Build: Package `com.voxlit.voice` im Manifest

## Risiken

- **AppData-Migration:** Bestehende Nutzer verlieren Settings/History. Spaeter loesung, nicht jetzt.
- **gen/android teilweise generiert:** Nach `tauri android init` muessen Aenderungen re-applied werden.
- **Repo-Verzeichnis-Rename:** Bricht alle absoluten Pfade in Scripts. Deshalb ZULETZT machen.
- **APK-Signatur:** Keystore-Rename allein aendert nichts. Nur bei Neu-Generierung werden alte APKs inkompatibel.
