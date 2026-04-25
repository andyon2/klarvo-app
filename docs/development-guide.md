# Klarvo — Entwicklungs-Guide

Generiert: 2026-04-13 | Projektversion: 0.5.0

---

## Voraussetzungen

### System
- **Node.js** (fuer Frontend-Build)
- **Rust** (stable, 2021 Edition)
- **Tauri CLI v2** (`npm run tauri`)

### Plattform-spezifisch

**Windows:**
- Visual Studio Build Tools (C++ Workload)
- CMake (fuer whisper-rs, llama-cpp-2 Bindgen)
- libclang (fuer Bindgen)

**Android:**
- Android SDK + NDK
- JDK 17+
- Gradle
- `aarch64-linux-android` Rust Target

**Linux (experimentell):**
- xdotool (fuer Paste-Simulation)
- ALSA/PulseAudio Dev-Libraries (fuer cpal)

## Installation

```bash
# Repository klonen
git clone <repo-url> klarvo
cd klarvo

# Frontend-Dependencies installieren
npm install

# Rust-Dependencies werden automatisch beim ersten Build geladen
```

## Entwicklung

### Frontend-Only (Browser-Preview)

```bash
npm run dev          # Vite Dev Server auf localhost:1420
npm run preview      # Preview auf localhost:1422 (ohne Tauri)
```

Im Browser-Preview-Modus liefert `tauri-commands.ts` Mock-Daten zurueck — erlaubt UI-Entwicklung ohne Backend.

### Vollstaendiger Desktop-Build

```bash
npm run tauri dev    # Startet Vite + Tauri Dev Build
```

### Production Build (Windows)

```powershell
# PowerShell-Script: Kill laufende Instanz, Build, Signierung
powershell.exe -ExecutionPolicy Bypass -File scripts/sync-and-build.ps1
```

Output: `src-tauri/target/release/klarvo.exe`

**Testen:** Direkt `klarvo.exe` starten (nicht ueber Installer, nicht ueber `tauri dev`).

### Android Build

```bash
# Kotlin-Quellen kopieren, Build, Signieren, Deploy
bash scripts/android-build.sh
```

Erfordert aktives Android-Geraet oder Emulator.

## Build-Befehle

| Befehl | Zweck |
|--------|-------|
| `npm run dev` | Vite Dev Server (Frontend) |
| `npm run build` | TypeScript Check + Vite Build |
| `npm run preview` | Frontend-Preview ohne Backend |
| `npm run tauri dev` | Full Desktop Dev Build |
| `npm run tauri build` | Production Desktop Build |
| `cargo test` | Rust Unit Tests |
| `cargo fmt --check` | Rust Format Check |
| `cargo clippy` | Rust Linter |

## Testen

### Rust Tests

```bash
cd src-tauri
cargo test                     # Alle Unit Tests
cargo test --test pi_security  # Prompt Injection Tests (ignoriert by default)
```

Test-Framework: Standard `#[cfg(test)]` Module + `insta` fuer Snapshot-Tests.

### Prompt Injection Tests

```bash
# Gegen spezifischen Provider testen
PI_PROVIDER=groq cargo test --test pi_security -- --ignored
PI_PROVIDER=deepseek cargo test --test pi_security -- --ignored
```

27 Testfaelle gegen 6 Injection-Surfaces (Arcanum PI Taxonomy).

### Frontend Tests

Kein dediziertes Test-Framework eingerichtet. UI-Tests erfolgen manuell oder via `npm run preview`.

## Umgebungsvariablen

Entwicklungs-Secrets in `.env` (in `.gitignore`):

```bash
GROQ_API_KEY=gsk_...
DEEPSEEK_API_KEY=sk-...
OPENAI_API_KEY=sk-...        # Optional
TURSO_URL=libsql://...       # Optional (fuer Sync)
TURSO_TOKEN=...              # Optional (fuer Sync)
TAURI_SIGNING_PRIVATE_KEY=...  # Fuer Auto-Updater Signierung
TAURI_SIGNING_PRIVATE_KEY_PASSWORD=...
```

Beispiel-Datei: `.env.example`

## Projekt-Konventionen

### Code-Sprache
- **Code:** Englisch (Variablen, Kommentare, Commits)
- **Kommunikation:** Deutsch (Team-Chat, Dokumentation)

### Commit-Konvention
Conventional Commits: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`

### Architektur-Regeln
- **Deep Modules:** Breites Interface, interne Komplexitaet verborgen
- **Vertical Slices:** Feature-Aenderungen schneiden durch alle Schichten
- **Platform Guards:** `#[cfg(desktop)]` / `#[cfg(mobile)]` / `#[cfg(windows)]`
- **Backward Search:** Vor Refactoring alle Consumer per grep pruefen
- **Characterization Tests:** Vor Refactoring bestehende Snapshots erstellen (insta)

### Task-Groesse
5-15 Minuten (max 35 Minuten). Groessere Tasks delegieren oder aufteilen.

## Konfigurationsdateien

| Datei | Zweck |
|-------|-------|
| `package.json` | npm Dependencies + Scripts |
| `tsconfig.json` | TypeScript (ES2020, strict, React-JSX) |
| `vite.config.ts` | Vite + React + Tailwind, Tauri-spezifische Ports |
| `src-tauri/Cargo.toml` | Rust Dependencies + Platform-Targets |
| `src-tauri/tauri.conf.json` | Tauri: Window, Bundle, Updater, Plugins |
| `src-tauri/capabilities/` | Tauri v2 Permission System |
| `src-tauri/.cargo/config.toml` | Cargo: Linker-Flags, /FORCE:MULTIPLE |

## Debugging

### Rust Backend
- Logging via `tauri-plugin-log` + `log` crate
- Log-Level konfigurierbar in Advanced Settings
- `println!` / `eprintln!` fuer temporaere Debug-Ausgaben

### Frontend
- Browser DevTools (F12 im Tauri-Fenster)
- Preview-Modus fuer isoliertes UI-Testing

### Android
- `KlarvoLogger` schreibt nach `{dataDir}/logs/klarvo.log`
- Logcat: `adb logcat -s Klarvo*`
- Rotierende Logs (2MB pro Datei, max 5 Dateien)

## Wichtige Hinweise

1. **Nie im falschen Verzeichnis arbeiten.** App-Code: `~/workspace/products/klarvo/`. Team-Repo: `~/workspace/teams/klarvo/`.

2. **Android umgeht Tauri IPC.** Kotlin macht direkte HTTP-Aufrufe und liest config.json vom Dateisystem. Aenderungen an der Rust-API-Schicht betreffen Android NICHT automatisch — Kotlin-Code muss separat aktualisiert werden.

3. **LLM-Prompts sind dupliziert** in Rust (`llm/mod.rs`) und Kotlin (`KlarvoApi.kt`). Nach Prompt-Aenderung BEIDE Dateien pruefen. Tool: `/sync-prompts` im Team-Repo.

4. **Testen via klarvo.exe**, nicht ueber Installer oder `tauri dev`. Das entspricht der User-Erfahrung.

5. **Pre-Commit-Hooks:** `cargo fmt --check`, `cargo clippy`, `cargo test` laufen automatisch.
