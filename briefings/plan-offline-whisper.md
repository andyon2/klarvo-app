# Feature-Plan: Offline whisper.cpp Fallback

## Ziel
Voxlit kann ohne Internet und ohne API-Keys transkribieren. whisper.cpp lokal via whisper-rs. Automatischer Fallback wenn Cloud-API fehlschlaegt und "local" in der STT-Priority-Liste steht.

## Scope
- Nur Windows (erstmal). Android ist ein eigenes Thema (NDK-Build).
- Kein CUDA im Default-Build. CUDA als optionales Cargo-Feature.

## Tasks

### Session 1: Backend-Kern

**Task 1: Recherche whisper-rs** → `/research-api whisper-rs`
- API (0.x vs 1.x), CUDA-Feature-Flags, GGML-Model-Format, Build-Anforderungen
- Latenz tiny/base/small auf CPU vs GPU
- Ergebnis in `knowledge/api-providers.md`

**Task 2: Cargo.toml** → rust-core
- `whisper-rs` unter `[target.'cfg(windows)'.dependencies]`
- Feature `local-whisper-cuda` optional
- Android-Build darf nicht brechen

**Task 3: LocalWhisperProvider** → rust-core
- `src-tauri/src/stt/local_whisper.rs` (neu)
- Implementiert `SttProvider`-Trait
- WAV → PCM-f32 (hound vorhanden) → whisper_rs transcribe
- `SttError::LocalWhisper(String)` Variante
- `#[cfg(windows)]`, mindestens 2 Unit-Tests

**Task 4: Config-Erweiterung** → rust-core
- `local_whisper_model: String` (default "base")
- `local_whisper_gpu: bool` (default true)
- Beide mit `#[serde(default)]`, in SettingsResponse integrieren

**Task 5: Fallback in Pipeline** → rust-core
- `resolve_stt_provider()` erkennt `"local"` als Provider-ID
- Bei Cloud-STT-Fehler (Request/ApiError 401/429): automatisch naechsten Provider aus Priority-Liste nehmen
- Kein separates `stt_fallback_to_local` Flag — STT-Priority-Liste reicht

### Session 2: UI + Model-Download

**Task 6: Model-Manager** → rust-core
- `src-tauri/src/stt/model_manager.rs` (neu)
- `list_available_models()`, `download_model()`, `model_path()`
- Chunked HTTP-Download, Progress-Events `voxlit://model-download-progress`
- Atomic rename (temp → final nur bei vollstaendigem Download)
- Tauri-Commands: `get_local_model_status`, `download_local_model`

**Task 7: Settings-UI** → ui-dev
- "Offline Transcription" Sektion (nur Desktop via `isDesktop`)
- Model-Auswahl (tiny 75MB / base 142MB / small 466MB)
- Download-Button + Fortschrittsbalken
- Status-Badge "Bereit" / "Nicht installiert"
- GPU-Checkbox

**Task 8: STT-Priority-UI** → ui-dev
- `"local"` als Option in AdvancedSettings STT-Provider-Liste
- Label: "Local (whisper.cpp)"
- Nur auf Desktop sichtbar

## Testplan
- [ ] Transkription ohne Internet (tiny-Model, CPU)
- [ ] Model-Download + Fortschrittsbalken + Badge
- [ ] Abgebrochener Download: kein korruptes Model
- [ ] Cloud-Key ungueltig → Pipeline faellt auf Local (wenn in Priority-Liste)
- [ ] Android-Build bricht nicht
- [ ] Unit-Tests: Provider-Konstruktion, leeres Audio

## Risiken
| Risiko | Gegenmassnahme |
|--------|----------------|
| whisper-rs Build-Probleme (cmake) | Task 1 klaert das; ggf. pre-built oder pure-Rust-Alternative |
| Binary-Groesse (CUDA-Libs) | CUDA optional, Standard ohne |
| Download-Abbruch | Atomic rename, kein Resume fuer MVP |
| Compile-Zeit | Nur Windows-Target betroffen |
