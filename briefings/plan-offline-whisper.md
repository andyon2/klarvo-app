# Feature-Plan: Offline whisper.cpp Fallback

## Priorität: 2

## Ziel
Dikta kann ohne Internet-Verbindung und ohne API-Keys transkribieren. Nutzt whisper.cpp lokal über die whisper-rs Crate. Automatischer Fallback wenn Cloud-API nicht erreichbar.

## Betroffene Module
- `src-tauri/src/stt/` — Neuer LocalWhisperProvider
- `src-tauri/Cargo.toml` — whisper-rs Dependency
- `src-tauri/src/config/` — Model-Pfad, GPU-Einstellung
- `src/components/SettingsPanel.tsx` — Model-Download-UI, Offline-Toggle
- `src/components/AdvancedSettingsPanel.tsx` — STT Priority Liste (Whisper Local einfügen)

## Tasks

### Task 1: Recherche whisper-rs
- **Skill:** /research-api whisper-rs
- **Beschreibung:** Aktuelle API, GPU-Support (CUDA/Vulkan), Model-Formate, Performance-Benchmarks. Ergebnis in knowledge/api-providers.md.

### Task 2: whisper-rs Integration
- **Agent:** rust-core
- **Dateien:** `src-tauri/src/stt/local_whisper.rs` (neu), `src-tauri/src/stt/mod.rs`
- **Beschreibung:** Neuer `LocalWhisperProvider` der das SttProvider-Trait implementiert. Model laden, Audio transkribieren, Sprache erkennen. GPU nutzen wenn verfügbar (CUDA), sonst CPU.
- **Abhängigkeit:** Task 1

### Task 3: Model-Management
- **Agent:** rust-core
- **Dateien:** `src-tauri/src/stt/model_manager.rs` (neu)
- **Beschreibung:** Model-Download von HuggingFace. Fortschritts-Events an Frontend emittieren. Models in App-Data-Dir speichern. Verschiedene Größen: tiny (75MB, schnell), base (142MB, gut), small (466MB, besser).
- **Abhängigkeit:** Task 2

### Task 4: Fallback-Logik
- **Agent:** rust-core
- **Dateien:** `src-tauri/src/stt/mod.rs`, `src-tauri/src/pipeline.rs`
- **Beschreibung:** STT Priority Liste erweitern. Wenn Cloud-Provider fehlschlägt (Timeout, kein Key, kein Internet) → automatisch auf Local Whisper fallen. Konfigurierbar: "cloud-first", "local-first", "local-only".

### Task 5: Settings-UI
- **Agent:** ui-dev
- **Dateien:** `src/components/SettingsPanel.tsx`, `src/components/AdvancedSettingsPanel.tsx`
- **Beschreibung:** Model-Download-Button mit Fortschrittsbalken. Offline-Modus-Toggle. Model-Größe wählen (tiny/base/small). GPU-Status anzeigen.
- **Abhängigkeit:** Task 3

### Task 6: GPU-Detection
- **Agent:** rust-core
- **Beschreibung:** Erkennen ob CUDA verfügbar. Andys Laptop hat GPU aber nutzt sie nur am Strom — ggf. Battery-Detection einbauen (Windows Power API).
- **Optional, kann nachgezogen werden**

## Testplan
- [ ] Transkription ohne Internet funktioniert
- [ ] Model-Download mit Fortschrittsanzeige
- [ ] Fallback Cloud → Local wenn API nicht erreichbar
- [ ] GPU-Beschleunigung wenn CUDA verfügbar
- [ ] Verschiedene Model-Größen wählbar

## Risiken
- whisper-rs Binary-Größe: Kann die App deutlich vergrößern (CUDA-Libs)
- Model-Download: Große Dateien (75MB-466MB), braucht gute UX
- Android: whisper.cpp auf Android ist ein eigenes Thema (NDK-Build). Erstmal nur Windows.
- Compile-Zeit: whisper-rs mit CUDA-Support kann den Build deutlich verlangsamen
