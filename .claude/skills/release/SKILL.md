---
name: release
description: Bumpt die Version, baut beide Plattformen, synct nach voxlit-app, erstellt GitHub Release dort. Aufrufen mit Version-Bump-Typ, z.B. "/release minor" oder "/release patch" oder "/release 0.6.0".
argument-hint: "[minor | patch | major | X.Y.Z] -- Version bump type or explicit version"
allowed-tools: Read, Edit, Bash, Glob
context: fork
model: haiku
---

Erstelle einen vollstaendigen Release: Version bump, Build, Sync nach Public Repo, GitHub Release.

## Argumente

Aus `$ARGUMENTS` extrahiere den Bump-Typ oder die explizite Version:
- `patch` → 0.4.0 → 0.4.1
- `minor` → 0.4.0 → 0.5.0
- `major` → 0.4.0 → 1.0.0
- `X.Y.Z` → Explizite Version (z.B. `0.6.0`)

Falls kein Argument: Fehler melden. Nie raten.

## Zwei-Repo-Architektur

- `voxlit` (privat, `~/claude-projects/voxlit`): Arbeitsrepo. Code + Agents. Hier wird entwickelt und gebaut.
- `voxlit-app` (oeffentlich, `~/voxlit-app`): Produktcode-Mirror. Hier landen Releases fuer Nutzer.

Releases werden IMMER auf `voxlit-app` erstellt (--repo andyon2/voxlit-app).

## Vorgehensweise

### 1. Aktuelle Version lesen

Lies die Version aus `src-tauri/Cargo.toml` (Zeile `version = "..."`).

### 2. Neue Version berechnen

Wenn Bump-Typ (`patch`/`minor`/`major`):
- Parse die aktuelle Version als MAJOR.MINOR.PATCH
- Inkrementiere den entsprechenden Teil (bei minor/major werden niedrigere Teile auf 0 gesetzt)

Wenn explizite Version:
- Validiere Format X.Y.Z
- Pruefe dass die neue Version HOEHER ist als die aktuelle

### 3. Version in allen drei Dateien aktualisieren

Alle drei muessen synchron sein:

1. `src-tauri/Cargo.toml` → `version = "X.Y.Z"`
2. `src-tauri/tauri.conf.json` → `"version": "X.Y.Z"`
3. `package.json` → `"version": "X.Y.Z"`

Nutze `Edit` (nicht sed) fuer jede Datei. Ersetze NUR die alte Version durch die neue.

### 4. Beide Plattformen bauen

Windows:
```bash
cd /home/andyon2/claude-projects/voxlit && powershell.exe -Command "Get-Process voxlit -ErrorAction SilentlyContinue | Stop-Process -Force" 2>/dev/null; powershell.exe -ExecutionPolicy Bypass -File '\\wsl$\Ubuntu\home\andyon2\claude-projects\voxlit\scripts\sync-and-build.ps1' 2>&1
```

Android:
```bash
cd /home/andyon2/claude-projects/voxlit && bash scripts/android-build.sh 2>&1
```

Falls ein Build fehlschlaegt:
1. **Version-Bump revertern:** `git checkout -- src-tauri/Cargo.toml src-tauri/tauri.conf.json package.json`
2. Strukturierten Fehler melden (welcher Build, welcher Fehler, wahrscheinliche Ursache)
3. NICHT den Release erstellen, NICHT committen
4. Wenn nur EIN Build fehlschlaegt (z.B. Android OK, Windows nicht): Trotzdem komplett abbrechen. Kein partieller Release.

### 5. Installer signieren und latest.json generieren

Tauri's eingebauter Signer haengt auf Windows/WSL. Signing geschieht separat via rsign.

Das Build-Script (`sync-and-build.ps1`) ruft automatisch `scripts/sign-installer.sh` auf. Falls das
nicht geklappt hat oder die `.sig` fehlt, manuell signieren:

```bash
bash ~/claude-projects/voxlit/scripts/sign-installer.sh
```

Danach existieren:
- NSIS Installer: `/mnt/d/Apps/voxlit/src-tauri/target/release/bundle/nsis/Voxlit_X.Y.Z_x64-setup.exe`
- Signatur: `/mnt/d/Apps/voxlit/src-tauri/target/release/bundle/nsis/Voxlit_X.Y.Z_x64-setup.exe.sig`

Lies den Inhalt der `.sig`-Datei (das ist die Signatur als String, base64-kodiert).

Erstelle `/tmp/latest.json` mit diesem Inhalt:
```json
{
  "version": "X.Y.Z",
  "notes": "Release vX.Y.Z",
  "pub_date": "<aktuelles ISO-8601 Datum, z.B. 2026-03-09T12:00:00Z>",
  "platforms": {
    "windows-x86_64": {
      "signature": "<Inhalt der .sig-Datei>",
      "url": "https://github.com/andyon2/voxlit-app/releases/download/vX.Y.Z/Voxlit_X.Y.Z_x64-setup.exe"
    }
  }
}
```

Nutze `date -u +%Y-%m-%dT%H:%M:%SZ` fuer das Datum.

WICHTIG: Die URL zeigt auf `voxlit-app`, NICHT auf `voxlit`!

Falls die `.sig` Datei NICHT existiert: `bash scripts/sign-installer.sh` ausfuehren. Falls das auch fehlschlaegt (z.B. rsign nicht installiert): `cargo install rsign2` und erneut versuchen.

### 6. Git Commit + Push (privates Repo)

```bash
cd /home/andyon2/claude-projects/voxlit
git add src-tauri/Cargo.toml src-tauri/tauri.conf.json package.json
git commit -m "chore: bump version to X.Y.Z"
git push origin master
```

### 7. Public Repo sync + commit + push

```bash
cd /home/andyon2/claude-projects/voxlit && bash scripts/publish.sh
```

Falls publish.sh mit Marker-Warnung abbricht: STOPP. Dem Nutzer melden. Nicht weiter machen.

Falls OK:
```bash
cd /home/andyon2/voxlit-app
git commit -m "chore: sync release vX.Y.Z"
git push origin main
```

### 8. Release-Notes aus VOLLEM Changelog erstellen

**WICHTIG:** Release-Notes muessen ALLE Aenderungen seit dem letzten oeffentlichen Release enthalten, nicht nur die der aktuellen Session!

So sammelst du den vollstaendigen Changelog:
```bash
# Alle Feature/Fix-Commits seit dem letzten Tag (ohne chore/docs/PB):
git log v<LETZTE_VERSION>..HEAD --oneline --no-merges | grep -v "^\w\+ \[PB\]\|^\w\+ chore:\|^\w\+ docs:"
```

Gruppiere die Commits in:
- **Features** (feat:) — neue Funktionalitaet
- **Bug Fixes** (fix:) — Fehlerbehebungen
- **Improvements** — UX-Verbesserungen, Performance, etc.

**Filter:** Nur nutzerrelevante Aenderungen aufnehmen. Dev-Tooling (Preview Mode, Build-Scripts, Agent-Infrastruktur, Skill-Aenderungen) gehoert NICHT in Release-Notes. Nutzer interessiert nur, was sich fuer sie aendert.

**Nutzerperspektive:** Schreibe aus Sicht des Nutzers, nicht des Entwicklers. Wenn ein Feature im vorherigen Release noch nicht existierte, ist ein Bugfix daran kein "Fix" sondern Teil des neuen Features. Beispiel: "Trial-Keys zeigen jetzt korrekt das Ablaufdatum" ist falsch wenn Trial-Keys im letzten Release noch gar nicht existierten — dann gehoert es unter das neue Feature "Lizenzschluessel-System". Faustregel: Kannte der Nutzer das kaputte Verhalten? Nein → kein Bugfix erwaehnen.

Schreibe nutzerfreundliche Beschreibungen (nicht die Commit-Messages 1:1). Fasse zusammengehoerende Commits zusammen (z.B. mehrere Commits zu "offline whisper" → ein Punkt "Offline Speech-to-Text").

### 9. GitHub Release erstellen (auf voxlit-app!)

Pruefe dass alle Artefakte existieren und eine plausible Groesse haben (>1MB fuer Installer/APK).

```bash
gh release create vX.Y.Z \
  "/mnt/d/Apps/voxlit/src-tauri/target/release/bundle/nsis/Voxlit_X.Y.Z_x64-setup.exe" \
  "/mnt/d/Apps/voxlit/src-tauri/target/release/bundle/nsis/Voxlit_X.Y.Z_x64-setup.exe.sig#Voxlit_X.Y.Z_x64-setup.exe.sig" \
  "/tmp/latest.json" \
  "/mnt/d/Dropbox/App Development/voxlit/releases/vX.Y.Z/Voxlit-vX.Y.Z.apk" \
  --repo andyon2/voxlit-app \
  --title "Voxlit vX.Y.Z" \
  --notes "<Release-Notes aus Schritt 8, als HEREDOC>"
```

WICHTIG: `--repo andyon2/voxlit-app` (NICHT voxlit!)

### 9. Ergebnis melden

```
RELEASE ERSTELLT: vX.Y.Z

Version: X.Y.Z (vorher: A.B.C)
Windows Installer: Voxlit_X.Y.Z_x64-setup.exe ([Groesse])
Signatur: Voxlit_X.Y.Z_x64-setup.exe.sig
Updater Manifest: latest.json (signiert: ja/nein)
Android: Voxlit-vX.Y.Z.apk ([Groesse])
Public Repo: synced + pushed
Release: https://github.com/andyon2/voxlit-app/releases/tag/vX.Y.Z

Auto-Update: Nutzer mit v0.4.1+ bekommen Update-Benachrichtigung in den Settings.
Naechster Schritt: Release-Notes ergaenzen (gh release edit vX.Y.Z --repo andyon2/voxlit-app --notes "...")
```
