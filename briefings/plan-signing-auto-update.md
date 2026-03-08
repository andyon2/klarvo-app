# Feature-Plan: Signing Keys + Auto-Update

## Priorität: 1 (nächste Session)

## Ziel
Tester bekommen Updates automatisch, ohne manuell neu installieren zu müssen. Windows-Build-Warnung bezüglich fehlender Signing Keys verschwindet.

## Betroffene Module
- `src-tauri/tauri.conf.json` — Updater-Plugin-Konfiguration, Signing Public Key
- `src-tauri/Cargo.toml` — tauri-plugin-updater Dependency
- `.env` — TAURI_SIGNING_PRIVATE_KEY
- `.claude/skills/release/SKILL.md` — Signing-Step einbauen
- Evtl. `src/` — Update-Check-UI (optional, Tray-Notification reicht)

## Tasks

### Task 1: Signing Keys generieren
- **Agent:** keiner (CLI-Befehl)
- **Befehl:** `npx tauri signer generate -w ~/.tauri/dikta.key`
- **Danach:** Private Key in `.env` als `TAURI_SIGNING_PRIVATE_KEY`, Public Key in `tauri.conf.json`
- **WICHTIG:** Private Key NIEMALS committen

### Task 2: Updater-Plugin konfigurieren
- **Agent:** rust-core
- **Dateien:** `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`
- **Beschreibung:** tauri-plugin-updater aktivieren. Endpoint auf GitHub Releases zeigen:
  `https://github.com/andyon2/dikta/releases/latest/download/latest.json`
- **Tauri v2 Docs:** https://v2.tauri.app/plugin/updater/

### Task 3: Update-Check im Backend
- **Agent:** rust-core
- **Dateien:** `src-tauri/src/lib.rs` oder neues Modul `src-tauri/src/updater.rs`
- **Beschreibung:** Beim App-Start prüfen ob Update verfügbar. Wenn ja: Tray-Notification oder Dialog. Auto-Install nach Bestätigung.

### Task 4: /release Skill anpassen
- **Agent:** keiner (Skill-Datei editieren)
- **Dateien:** `.claude/skills/release/SKILL.md`
- **Beschreibung:** Build-Schritt muss TAURI_SIGNING_PRIVATE_KEY aus .env laden. Release muss `latest.json` als Artefakt hochladen (Tauri Updater braucht das).

### Task 5: Erster signierter Release
- **Beschreibung:** `/release patch` (0.4.1) als Test. Prüfen ob Auto-Update bei installierter 0.4.0 funktioniert.

## Testplan
- [ ] Build läuft ohne Signing-Key-Warnung
- [ ] `latest.json` wird bei GitHub Release hochgeladen
- [ ] App mit v0.4.0 erkennt v0.4.1 als Update
- [ ] Update wird heruntergeladen und installiert (Windows)

## Risiken
- Tauri v2 Updater-Plugin hat sich seit letzter Recherche ggf. geändert → /research-api vorschalten
- Android hat keinen Auto-Updater (APK-Sideload kann nicht auto-updaten) → nur Windows
