# Windows-Rebuild & Smoke — Session-Kickoff (START HERE)

**Zweck:** Orientiert eine Session auf der **Windows-Seite**, die einen frischen Build braucht und den
offenen Smoke-/DoD-Check fährt. Self-contained, wiederverwendbar. Pro Story nur **§3 Aktueller Auftrag**
aktualisieren.

## Topologie (so läuft der Build wirklich)

Quell-Repo lebt in **WSL** (`\\wsl$\Ubuntu\…\products\klarvo`, Branch `v1-ship`) — das ist die Source of
Truth. Der Windows-Build liest den **live WSL-Tree per robocopy** (nicht via git-pull von origin). origin
ist nur Backup/History.

## §1 Rebuild — der kanonische Weg

**Desktop-Verknüpfung „Klarvo Rebuild" doppelklicken.** Sie führt `sync-and-build.ps1` aus:
1. robocopy: aktueller WSL-Stand (`v1-ship`) → `D:\apps\klarvo`
2. `npm install` + `npx tauri build` (nativer Windows-Build, warmer Cache)
3. Signieren per WSL-`rsign` → Ablage in Dropbox

Fertige exe: **`D:\apps\klarvo\src-tauri\target\release\klarvo.exe`**. Fenster bleibt offen (`cmd /k`) →
komplettes Log sichtbar.

> **Signing-Falle (nicht neu erfinden):** Tauris eingebauter Signer **hängt** auf Windows/WSL. Das Script
> umgeht das bewusst (rsign nach dem Build). Kein eigenes Build-Script mit `.env`-Signing-Key bauen.
> `Klarvo Rebuild.lnk` = neu bauen · `klarvo.lnk` = App starten. Nicht verwechseln.

## §2 Schnell-Verifikation ohne vollen Build (für reine Rust-/NFR-W-Checks)

In `D:\apps\klarvo\src-tauri` auf nativer Windows-Toolchain:
```
cargo test --lib fs::
```
Trifft den echten Windows-Rename-Codepfad (`MoveFileExW`+`MOVEFILE_REPLACE_EXISTING`), den Linux nie
kompiliert. (Falls JNI-Bridge dazwischenfunkt: `--exclude klarvo-bridge-jni`.)

## §3 Aktueller Auftrag — Story 1.1 NFR-W Smoke (`save_atomic`)

Story-File: `_bmad-output/implementation-artifacts/1-1-atomic-state-file-writes-via-a-shared-save-atomic-helper.md`
(Status `review` → `done`, sobald grün). Frage: schreibt `save_atomic` auf echtem Windows atomar über
eine *existierende* Datei, ohne Leak, mit sauberem Fehler statt Panik? App schreibt nach
`%APPDATA%\com.klarvo.voice\` (`config.json`, `dictionary.json`).

**Kern (deterministisch):** `cargo test --lib fs::` (§2) → 4 Tests grün.

**App-Level (was Units nicht abdecken):**
1. **Normal + Force-Kill:** Key/Lizenz setzen → speichern → `config.json` vollständig & valides JSON,
   **keine** zufällig benannten `.tmp…`-Reste. App hart killen → neu öffnen → Daten intakt, genau *eine* `config.json`.
2. **Read-only-Ziel:** `attrib +R config.json` → speichern → muss **sauber fehlschlagen** (Fehler sichtbar,
   kein Crash, alte Datei intakt). Dann `attrib -R` → geht wieder.
3. **`dictionary.json`** analog.

**Ehrlich:** Das ~1 ms Crash-Fenster triffst du manuell nicht (Atomarität kommt vom OS-Rename). Der
realistische „locked"-Fall ist Defender-Real-Time-Scan (transient) — wichtig nur: Fehler wird *gemeldet*,
nicht verschluckt. → Read-only/locked einmal beobachten, dann als `#[cfg(windows)]`-Test einfrieren
(learn-then-encode), statt Dauer-Klick.

## §4 Zurückmelden

- `cargo test --lib fs::`-Ergebnis + App-Level 1–3.
- **Grün:** `sprint-status.yaml` → `1-1-…: done`, Story-Frontmatter `status: done`, committen+pushen
  (Frontmatter↔YAML in sync, Closeout-Drift-Check). **Sonst:** exakte Fehlersignatur melden, NICHT auf
  `done` flippen → `save_atomic` braucht Folge-Fix.
