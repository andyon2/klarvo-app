# Windows-Rebuild & Smoke — Session-Kickoff (START HERE)

**Zweck:** Orientiert eine *frische* Session auf der **Windows-Seite**, die einen Release-Build von
`v1-ship` zieht und den aktuell offenen Smoke-/DoD-Check fährt. Self-contained — keine Chat-Historie
nötig. Wiederverwendbar: bei jeder Persistenz-/Surface-Story, deren DoD einen echten Windows-Build
verlangt (NFR-W / NFR-Smoke). Den Abschnitt **§4 Aktueller Auftrag** pro Story aktualisieren.

## Topologie (warum es diesen Schritt gibt)

Quell-Repo lebt in WSL (Linux-FS); der Windows-Build läuft in deinem Windows-Klon von
`github.com/andyon2/klarvo-app`. Sync läuft über `origin` (GitHub) per push/pull. Tauri-Runtime-Bugs,
Windows-only-Codepfade und `rename`/`MoveFileExW`-Semantik tauchen **nur** im echten Windows-Build auf —
Linux `cargo test` maskiert sie (Release-Build-Blind-Spot).

## §1 Sync — den richtigen Stand holen

In deinem Windows-Build-Klon:

```
git fetch origin
git checkout v1-ship
git pull --ff-only origin v1-ship
git log --oneline -3
```

**Erwartung:** HEAD enthält `c1ffa79` (`fix(config): atomic state-file writes via shared save_atomic
helper`). Wenn der Commit fehlt → Stand stimmt nicht, NICHT weiterbauen.

## §2 Build

Toolchain: Rust (MSVC-Toolchain), Node + npm. Tauri-CLI v2 ist über npm verdrahtet.

- **Voller Release-Build (.exe/Installer):**
  ```
  npm install
  npm run tauri build
  ```
  `beforeBuildCommand` baut zuerst das Vite/TS-Frontend (`npm run build`), dann den Rust-Release +
  Bundle. Artefakte: `src-tauri/target/release/Klarvo.exe` und Installer unter
  `src-tauri/target/release/bundle/` (NSIS/MSI, `targets: "all"`).

- **Schnell-Verifikation ohne vollen Build** (reicht für den NFR-W-Kern unten — hier am wichtigsten):
  ```
  cd src-tauri
  cargo test --lib fs::
  ```
  Das führt den **echten Windows-Rename-Codepfad** (`MoveFileExW`+`MOVEFILE_REPLACE_EXISTING`) aus, den
  Linux nie kompiliert. (Falls der JNI-Bridge-Build dazwischenfunkt: `--exclude klarvo-bridge-jni` an
  Workspace-Test-Aufrufe; `--lib` im `src-tauri`-Package sollte den Bridge-Crate aber gar nicht ziehen.)

## §3 Wo die App schreibt

`%APPDATA%\com.klarvo.voice\` → `config.json`, `dictionary.json`. (= `C:\Users\<du>\AppData\Roaming\com.klarvo.voice\`.)

## §4 Aktueller Auftrag — Story 1.1 NFR-W Smoke (`save_atomic`)

Story-File: `_bmad-output/implementation-artifacts/1-1-atomic-state-file-writes-via-a-shared-save-atomic-helper.md`
(Status `review` → wird `done`, sobald dieser Smoke grün ist). Prüft: schreibt `save_atomic` auf echtem
Windows atomar über eine *existierende* Datei, ohne Leak, mit sauberem Fehler statt Panik?

**Kern (deterministisch, höchster Wert):** `cargo test --lib fs::` auf der nativen Windows-Toolchain →
alle 4 Tests grün (Replace-over-existing, kein Temp-Leak, Fehler-bei-fehlendem-Parent, Happy-Path).

**App-Level (was die Unit-Tests nicht abdecken):**
1. **Normal + Force-Kill:** API-Key + Lizenz setzen → speichern → in `com.klarvo.voice\` prüfen:
   `config.json` vollständig & valides JSON, **keine** zufällig benannten `.tmp…`-Reste. App per
   Task-Manager hart killen → neu öffnen → Key/Lizenz intakt, genau *eine* `config.json`.
2. **Read-only-Ziel:** `attrib +R config.json` → in der App speichern → muss **sauber fehlschlagen**
   (Fehler sichtbar, kein Crash, alte Datei intakt). Danach `attrib -R` → speichern geht wieder.
3. **`dictionary.json`** analog: Wörterbuch-Wort hinzufügen → persistiert, kein Temp-Leak.

**Fehler-Signaturen:** Streu-`.tmp`-Dateien sammeln sich an (Leak) · `config.json` 0 Byte/abgeschnitten
(Atomarität verletzt) · Key/Lizenz weg nach Kill (Datenverlust) · App crasht statt Fehler (read-only Fall).

**Ehrlich:** Das ~1 ms „Mid-Write-Crash"-Fenster triffst du manuell nicht — musst du nicht, Atomarität
kommt vom OS-Rename. Der realistische „locked"-Fall ist Defender-Real-Time-Scan (transient) — wichtig ist
nur, dass ein dadurch fehlgeschlagener Save *gemeldet* statt verschluckt wird.

## §5 Zurückmelden

- `cargo test --lib fs::`-Ergebnis (Output-Tail) + Build-Erfolg/-Fehler.
- App-Level-Checks 1–3: bestanden / Fehler-Signatur.
- **Wenn alles grün:** `_bmad-output/implementation-artifacts/sprint-status.yaml` →
  `1-1-…: done`, Story-File-Frontmatter `status: done`, committen + pushen (Frontmatter↔YAML in sync,
  Closeout-Drift-Check). **Wenn nicht:** exakte Fehlersignatur melden, NICHT auf `done` flippen — dann
  braucht `save_atomic` einen Folge-Fix (z. B. read-only/locked-Fehlerbehandlung).
