---
name: Story 10.1 — Whisper-Local STT-Plugin
epic: 10
story_number: "10.1"
status: done
dependencies:
  - "6-1-telemetry-logging-rolling-file"
  - "2a-a4-settings-panel-foundation"
adr_refs:
  - docs/adr/0014-second-stt-plugin-whisper-local.md
---

# Story 10.1: Whisper-Local STT-Plugin

Status: done

## Story

Als Klarvo Power-User
möchte ich diktieren ohne API-Key oder Internetverbindung, indem ich ein lokales `whisper.cpp`-Modell (.gguf) auf meinem Gerät bereitstelle,
damit ich offline-taugliches und BYOK-konformes Voice-Dictation habe — und um das `SttProvider`-Trait als Cloud-agnostisches Substrat empirisch zu validieren.

## Kontext und Motivation

**Substrate-Validation-Ziel (Epic 10 Primär-Goal):** Der `SttProvider`-Trait muss ein zweites, strukturell anderes Plugin tragen, ohne Core-Änderung. `klarvo-plugin-groq` ist Cloud-HTTPS mit Auth + RateLimit-Surface. `klarvo-plugin-whisper-local` ist Disk-Loaded-Model + In-Process-Inference ohne Network/Auth. Wenn beide denselben Trait erfüllen → Substrate validiert.

**ADR-0014 Decisions (alle final, kein offenes OQ):**

| ID | Decision |
|----|----------|
| D-1 | BYO-Model: User liefert `.gguf`-Pfad via `plugins.whisper-local.model_path` in Settings-SQLite (Plugin-Setting-API). Kein Auto-Download in Story 10.1. |
| D-2 | Empfohlene Model-Größe: `small` (~500 MB). `tiny`/`base` sind Low-Resource-Fallback. Doku-Tabelle im Plugin-Rustdoc. |
| D-3 | CPU-only-Default für CI. GPU via separate Cargo-Features — nicht in Story 10.1. |
| D-4 | `Arc<Mutex<WhisperContext>>` für Concurrent-Use (Mutex-Contention strukturell irrelevant: Single-Pipeline-Cycle per Hotkey). |
| D-5 | Language-Hint explizit aus `settings.output_language()` (Axis 3 per `memory/project_i18n_three_axes`). Auto-Detect verworfen (Brittleness). |

**ADR-0014 Folge-Items für diese Story:**
- Settings-Schema-Erweiterung via Plugin-Setting-API (D-1) — kein ShellConfig-TOML-Feld
- `output_language` propagation vom Shell-Bootstrap zum Plugin-Konstruktor (D-5)
- E1 Windows-CI-Gate: whisper-rs kompiliert C++ (whisper.cpp) → Timeout-Erweiterung + exclude-strategy
- Onboarding-Doku: Model-Größen-Tabelle mit D-2-Empfehlung

**Wichtig — Groq-Registration-Status:** `build_plugin_registry()` registriert aktuell NUR verbatim (Groq ist kompiliert aber nicht registriert). Nur verbatim ist in `klarvo_plugin_verbatim::register(&mut registry)` drin. Groq-Registration ist separates Story-Scope (Wire-Up-Story für Phase-2-B). Whisper-local wird parallel zu verbatim conditional registriert.

**Manifest-Status:** `pipeline-manifest.toml` hat weiterhin `type = "passthrough"`. Das Manifest wird in Story 10.1 NICHT auf `stt` umgestellt — das ist eine bewusste User-Entscheidung. Plugin-Registration allein reicht für Substrate-Validation (Trait-Konformanz ist Compile-Time-Check; Registry-Registration + Boot-Time-Check ist Run-Time-Validation).

## Acceptance Criteria

### AC-1: `klarvo-plugins/klarvo-plugin-whisper-local` in Workspace-Members

**Given** `Cargo.toml` (workspace root) enthält `workspace.members`,
**When** AC-1 committed ist,
**Then**:

```toml
# in workspace.members [...]:
"klarvo-plugins/klarvo-plugin-whisper-local",
```

Eingefügt nach `klarvo-plugin-vad-silero` (alphabetisch korrekt nach vorhandenen Members).

`cargo check --workspace` schlägt an diesem Punkt NOCH NICHT fehl (AC-2 liefert die Dateien).

---

### AC-2: Plugin-Crate `Cargo.toml` + Scaffold

**Given** Verzeichnis `klarvo-plugins/klarvo-plugin-whisper-local/` existiert nicht,
**When** AC-2 committed ist,
**Then**:

**`klarvo-plugins/klarvo-plugin-whisper-local/Cargo.toml`:**

```toml
[package]
name = "klarvo-plugin-whisper-local"
version.workspace = true
edition.workspace = true
license.workspace = true
publish.workspace = true

[dependencies]
klarvo-core = { path = "../../klarvo-core" }
whisper-rs = { version = "0.13", default-features = false }
async-trait = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
```

**Warum `whisper-rs = "0.13", default-features = false`:**
- D-3: CPU-only Default. `default-features = false` schaltet GPU-Backends (CUDA/Metal/Vulkan) aus, die System-Libraries brauchen.
- `whisper-rs` 0.13+ compiles whisper.cpp statisch via C++ build — kein `default-features = false` schaltet die Kernkompilierung aus, aber GPU-Backend-Features werden verhindert.

**Hinweis an Dev-Agent:** `whisper-rs` hat folgende relevante Features: `metal`, `cuda`, `opencl`, `hipblas`. Keines davon in Story 10.1 aktivieren. Die minimale CPU-only-Build funktioniert ohne explizite Feature-Aktivierung wenn `default-features = false`.

---

### AC-3: `WhisperLocal` Struct + `load()` Konstruktor

**Given** `klarvo-plugins/klarvo-plugin-whisper-local/src/lib.rs` neu erstellt,
**When** AC-3 committed ist,
**Then** enthält die Datei:

**Plugin-ID Konstante:**
```rust
pub const ID: &str = "whisper-local";
```

**i18n Error-Keys-Modul:**
```rust
pub mod keys {
    pub const MODEL_NOT_FOUND: &str = "error.stt.local.model_not_found";
    pub const LOAD_FAILED: &str = "error.stt.local.load_failed";
    pub const INFERENCE_FAILED: &str = "error.stt.local.inference_failed";
}
```

**Struct-Definition:**
```rust
use std::sync::{Arc, Mutex};
use whisper_rs::WhisperContext;

pub struct WhisperLocal {
    ctx: Arc<Mutex<WhisperContext>>,
    language: Option<String>,
}
```

**Konstruktor:**
```rust
impl WhisperLocal {
    /// Load a whisper.cpp model from `model_path`. `language` is an ISO-639-1 code
    /// (e.g. "de", "en") sourced from `settings.output_language()` (ADR-0014 D-5).
    /// Pass `None` only if language is genuinely unknown — whisper.cpp auto-detect
    /// is disabled in Klarvo per D-5 (brittleness on short utterances).
    ///
    /// # Errors
    ///
    /// - `error.stt.local.model_not_found` — `model_path` does not exist on disk.
    /// - `error.stt.local.load_failed` — whisper.cpp context creation failed
    ///   (corrupt file, unsupported format, OOM).
    pub fn load(model_path: &std::path::Path, language: Option<String>) -> Result<Self, AppError> {
        if !model_path.exists() {
            debug_assert!(klarvo_core::i18n::is_key(keys::MODEL_NOT_FOUND));
            return Err(AppError {
                kind: AppErrorKind::Configuration,
                message: format!("whisper-local: model not found: {}", model_path.display()),
                user_message: Some(keys::MODEL_NOT_FOUND.to_string()),
                retryable: false,
            });
        }

        let path_str = model_path.to_str().ok_or_else(|| AppError {
            kind: AppErrorKind::Configuration,
            message: format!("whisper-local: model path is not valid UTF-8: {}", model_path.display()),
            user_message: Some(keys::LOAD_FAILED.to_string()),
            retryable: false,
        })?;

        let ctx = WhisperContext::new_with_params(
            path_str,
            whisper_rs::WhisperContextParameters::default(),
        )
        .map_err(|e| {
            debug_assert!(klarvo_core::i18n::is_key(keys::LOAD_FAILED));
            AppError {
                kind: AppErrorKind::Configuration,
                message: format!("whisper-local: context load failed: {e:?}"),
                user_message: Some(keys::LOAD_FAILED.to_string()),
                retryable: false,
            }
        })?;

        Ok(Self {
            ctx: Arc::new(Mutex::new(ctx)),
            language,
        })
    }
}
```

**Imports am Dateiheader:**
```rust
use klarvo_core::error::{AppError, AppErrorKind};
use klarvo_core::i18n;
use klarvo_core::pipeline::PipelineStage;
use klarvo_core::traits::SttProvider;
use async_trait::async_trait;
```

**WICHTIG — whisper-rs API-Verifikation:** Vor Implementierung `cargo doc -p whisper-rs` oder `cargo doc --open` aufrufen um die tatsächliche API-Surface von v0.13 zu verifizieren. Besonders:
- `WhisperContext::new_with_params(path: &str, params: WhisperContextParameters)` — Signatur prüfen
- `WhisperContextParameters::default()` — ob verfügbar
- `WhisperContext::create_state()` — Rückgabetyp
- `WhisperState::full(params: FullParams, samples: &[f32])` — Signatur

Falls die API von dieser Sketch-Implementierung abweicht, die tatsächliche API verwenden.

---

### AC-4: `PipelineStage` Implementierung via `spawn_blocking`

**Given** `WhisperLocal` aus AC-3 existiert,
**When** AC-4 committed ist,
**Then**:

```rust
#[async_trait]
impl PipelineStage for WhisperLocal {
    type Input = klarvo_core::audio::AudioBuffer;
    type Output = String;

    async fn process(&self, audio: klarvo_core::audio::AudioBuffer) -> Result<String, AppError> {
        let ctx = Arc::clone(&self.ctx);
        let lang = self.language.clone();

        tokio::task::spawn_blocking(move || {
            let guard = ctx.lock().map_err(|_| AppError {
                kind: AppErrorKind::Fatal,
                message: "whisper-local: mutex poisoned".to_string(),
                user_message: Some(keys::INFERENCE_FAILED.to_string()),
                retryable: false,
            })?;

            let mut state = guard.create_state().map_err(|e| {
                debug_assert!(i18n::is_key(keys::INFERENCE_FAILED));
                AppError {
                    kind: AppErrorKind::Fatal,
                    message: format!("whisper-local: create_state failed: {e:?}"),
                    user_message: Some(keys::INFERENCE_FAILED.to_string()),
                    retryable: false,
                }
            })?;

            let mut params = whisper_rs::FullParams::new(
                whisper_rs::SamplingStrategy::Greedy { best_of: 0 },
            );
            if let Some(ref lang_code) = lang {
                params.set_language(Some(lang_code.as_str()));
            }
            // Suppress console output from whisper.cpp (noisy on some builds)
            params.set_print_special(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_timestamps(false);

            state.full(params, &audio.samples).map_err(|e| AppError {
                kind: AppErrorKind::Fatal,
                message: format!("whisper-local: inference failed: {e:?}"),
                user_message: Some(keys::INFERENCE_FAILED.to_string()),
                retryable: false,
            })?;

            let n_segments = state.full_n_segments().map_err(|e| AppError {
                kind: AppErrorKind::Fatal,
                message: format!("whisper-local: full_n_segments failed: {e:?}"),
                user_message: Some(keys::INFERENCE_FAILED.to_string()),
                retryable: false,
            })?;

            // Pure verbatim aggregation (D1 review-decision 2026-05-05): preserve
            // whisper.cpp's native segment whitespace. No per-segment trim and no
            // manual ' '-insertion — klarvo's default is verbatim per
            // `memory/feedback_polished_designschwaeche`.
            let mut result = String::new();
            let mut successful_segments: i32 = 0;
            for i in 0..n_segments {
                match state.full_get_segment_text(i) {
                    Ok(seg) => {
                        successful_segments += 1;
                        result.push_str(&seg);
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "klarvo.stt.whisper_local",
                            segment = i,
                            error = ?e,
                            "segment text extraction failed; skipping"
                        );
                    }
                }
            }

            // Distinguish "real silence" from "all segments errored":
            if n_segments > 0 && successful_segments == 0 {
                debug_assert!(i18n::is_key(keys::INFERENCE_FAILED));
                return Err(AppError {
                    kind: AppErrorKind::Internal,
                    message: format!(
                        "whisper-local: all {n_segments} segments failed text extraction"
                    ),
                    user_message: Some(keys::INFERENCE_FAILED.to_string()),
                    retryable: false,
                });
            }

            Ok::<String, AppError>(result)
        })
        .await
        .map_err(|join_err| AppError {
            kind: AppErrorKind::Fatal,
            message: format!("whisper-local: spawn_blocking panic: {join_err}"),
            user_message: Some(keys::INFERENCE_FAILED.to_string()),
            retryable: false,
        })?
    }

    fn stage_type(&self) -> &'static str {
        "stt"
    }
}
```

**WICHTIG — `WhisperState` Send-Bound:**
`spawn_blocking` erfordert `'static + Send` für die Closure. `ctx: Arc<Mutex<WhisperContext>>` ist `Send` weil `Arc<Mutex<T>>` Send ist wenn T: Send. `WhisperContext` muss `Send` implementieren — whisper-rs 0.13 hat `unsafe impl Send for WhisperContext {}`. Wenn der Compiler `Send`-Fehler meldet, in den whisper-rs Release-Notes schauen ob sich das geändert hat.

**WICHTIG — `FullParams` Lifetime:**
`FullParams` in einigen whisper-rs-Versionen hat Lifetime-Parameter (z.B. `FullParams<'a, 'b>`). In diesem Fall MUSS `FullParams` innerhalb der Closure konstruiert werden (nicht außerhalb und in die Closure gemoved), da die Closure `'static` sein muss. Die obige Implementierung konstruiert `params` bereits innerhalb der Closure.

---

### AC-5: `SttProvider` Blanket-Impl

**Given** AC-4 kompiliert,
**When** AC-5 committed ist,
**Then** in `lib.rs`:

```rust
impl SttProvider for WhisperLocal {}
```

`cargo check -p klarvo-plugin-whisper-local` → Exit 0.

---

### AC-6: i18n-Keys in Locale-Dateien

**Given** `shells/windows/locales/en.json` und `de.json` existieren,
**When** AC-6 committed ist,
**Then**:

**`shells/windows/locales/en.json`** — nach den bestehenden `error.stt.*`-Keys (nach `error.stt.upstream_4xx`):
```json
  "error.stt.local.model_not_found": "Whisper model not found. Please check the model path in settings.",
  "error.stt.local.load_failed": "Failed to load Whisper model. The file may be corrupt or in an unsupported format.",
  "error.stt.local.inference_failed": "Local speech recognition failed. Please try again."
```

**`shells/windows/locales/de.json`** — analog:
```json
  "error.stt.local.model_not_found": "Whisper-Modell nicht gefunden. Bitte Modellpfad in den Einstellungen prüfen.",
  "error.stt.local.load_failed": "Whisper-Modell konnte nicht geladen werden. Die Datei ist möglicherweise beschädigt oder hat ein nicht unterstütztes Format.",
  "error.stt.local.inference_failed": "Lokale Spracherkennung fehlgeschlagen. Bitte erneut versuchen."
```

**`cargo xtask lint-events`** nach AC-6 muss grün sein (neue Keys haben Rust-Emit-Sites in `klarvo-plugin-whisper-local/src/lib.rs`).

**ACHTUNG Orphan-Check:** Neue Keys werden NICHT in `xtask/orphan-allowlist.txt` eingetragen — sie HABEN Rust-Emit-Sites im Plugin. Wenn `lint-events` trotzdem einen Orphan-Fehler meldet, ist der Scan-Pfad zu prüfen (Scanner muss `klarvo-plugins/` scannen — per Epic-5-Retro-Notiz deckt der Scan `klarvo-core/src/`, `shells/windows/src-tauri/src/` UND `klarvo-plugins/`).

---

### AC-7: `build_plugin_registry()` — Settings-lesen + Whisper-Conditional-Registration

**Given** `shells/windows/src-tauri/src/main.rs` enthält `build_plugin_registry()` ohne Parameter,
**When** AC-7 committed ist,
**Then**:

**Signatur-Änderung:**
```rust
fn build_plugin_registry(
    settings: &klarvo_core::settings::Settings,
    output_language: &str,
) -> klarvo_core::registry::PluginRegistry {
```

**Body:**
```rust
{
    let mut registry = klarvo_core::registry::bootstrap();
    klarvo_plugin_verbatim::register(&mut registry);

    // Whisper-local: conditional on model_path plugin-setting.
    // If not configured → no-op (Groq or other STT plugins take over via manifest).
    // If configured but load fails → warn + skip; pipeline falls through to manifest error.
    match settings.get_plugin_setting("whisper-local", "model_path") {
        Ok(Some(model_path_str)) => {
            let model_path = std::path::Path::new(&model_path_str);
            match klarvo_plugin_whisper_local::WhisperLocal::load(
                model_path,
                Some(output_language.to_string()),
            ) {
                Ok(plugin) => {
                    registry.register_stt(
                        klarvo_plugin_whisper_local::ID,
                        std::sync::Arc::new(plugin),
                    );
                    tracing::info!(
                        target: "klarvo.bootstrap",
                        model_path = %model_path.display(),
                        language = output_language,
                        "whisper-local: plugin registered"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        target: "klarvo.bootstrap",
                        error = %e,
                        "whisper-local: model load failed; plugin NOT registered"
                    );
                }
            }
        }
        Ok(None) => {
            tracing::debug!(
                target: "klarvo.bootstrap",
                "whisper-local: no model_path configured; plugin not registered"
            );
        }
        Err(e) => {
            tracing::warn!(
                target: "klarvo.bootstrap",
                error = %e,
                "whisper-local: settings read failed; plugin not registered"
            );
        }
    }

    registry
}
```

**Aufruf-Seite (Step 8 in `.setup()`):**
```rust
// Vor dem bestehenden build_plugin_registry()-Aufruf:
let output_language = settings
    .output_language()
    .unwrap_or_else(|_| "en".to_string());

let registry = Arc::new(build_plugin_registry(&settings, &output_language));
```

**WICHTIG — Settings-Ownership:** `settings` ist zu diesem Zeitpunkt noch NICHT via `app.manage()` transferiert (das passiert in Step 10). Daher ist `&settings` borrow hier gültig. Der `app.manage(settings)` Call in Step 10 konsumiert danach den owned `settings`. Das ist der korrekte Reihenfolge-Constraint.

**USE-Deklaration ergänzen:**
```rust
use klarvo_plugin_whisper_local; // in den bestehenden use-Block einfügen
```

Das `klarvo-plugin-whisper-local`-Crate muss als Dependency in `shells/windows/src-tauri/Cargo.toml` stehen:
```toml
klarvo-plugin-whisper-local = { path = "../../../klarvo-plugins/klarvo-plugin-whisper-local" }
```

---

### AC-8: `windows-ci.yml` — whisper-rs C++-Build-Concern

**Given** `.github/workflows/windows-ci.yml` enthält `cargo check --workspace --all-targets`,
**When** AC-8 committed ist,
**Then**:

**Problem:** `cargo check` auf `klarvo-plugin-whisper-local` löst `build.rs` von `whisper-rs` aus, das whisper.cpp C++ kompiliert. Das dauert 10-20 Minuten auf einem Fresh-Runner. `Swatinem/rust-cache@v2` cached die Artifacts, aber der erste Run ist langsam.

**Lösung (2-Teil):**

**Teil 1:** `cargo check --workspace --all-targets` → `cargo check --workspace --all-targets --exclude klarvo-plugin-whisper-local`

```yaml
- name: cargo check --workspace --all-targets (excl. whisper-local C++ build)
  # klarvo-plugin-whisper-local excluded: whisper-rs compiles whisper.cpp C++ (~15 min
  # first build). Checked separately below with extended timeout.
  run: cargo check --workspace --all-targets --exclude klarvo-plugin-whisper-local
```

**Teil 2:** Neuen Step nach dem Workspace-Check:
```yaml
- name: cargo check -p klarvo-plugin-whisper-local (whisper-rs C++ build; cached on 2nd run)
  # First run: ~15 min C++ compile. Subsequent runs: <1 min from rust-cache.
  # timeout-minutes is on job level; this step uses the shared cache.
  run: cargo check -p klarvo-plugin-whisper-local
  timeout-minutes: 30
```

**Job-Level Timeout:** Am Job `windows-compile:` ergänzen:
```yaml
jobs:
  windows-compile:
    name: cargo check --workspace (windows-latest)
    runs-on: windows-latest
    timeout-minutes: 90  # Extended for whisper.cpp first-build; cached on 2nd run
```

**Cache-Key:** In `Swatinem/rust-cache@v2` den `shared-key` von `windows-compile-g6` auf `windows-compile-g6-v2` ändern (Cache-Bust nach whisper-rs-Hinzufügung):
```yaml
with:
  workspaces: ". -> target"
  shared-key: windows-compile-g6-v2
```

---

### AC-9: Unit Tests in `klarvo-plugin-whisper-local/src/lib.rs`

**Given** AC-3 bis AC-5 kompilieren,
**When** AC-9 committed ist,
**Then** am Ende von `lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time: WhisperLocal implements SttProvider + PipelineStage.
    /// If this test file compiles, the trait bounds are satisfied.
    #[allow(dead_code)]
    fn _assert_trait_bounds(plugin: WhisperLocal) {
        // SttProvider: PipelineStage<Input = AudioBuffer, Output = String>
        let _: &dyn klarvo_core::traits::SttProvider = &plugin;
    }

    #[test]
    fn load_rejects_nonexistent_model_path() {
        let result = WhisperLocal::load(
            std::path::Path::new("/does/not/exist/model.gguf"),
            Some("en".to_string()),
        );
        let err = result.expect_err("must fail for nonexistent path");
        assert_eq!(
            err.user_message.as_deref(),
            Some(keys::MODEL_NOT_FOUND)
        );
        assert!(matches!(err.kind, klarvo_core::error::AppErrorKind::Configuration));
    }

    #[test]
    fn plugin_id_constant() {
        assert_eq!(ID, "whisper-local");
    }

    #[test]
    fn i18n_keys_are_valid() {
        klarvo_core::i18n::assert_is_key(keys::MODEL_NOT_FOUND);
        klarvo_core::i18n::assert_is_key(keys::LOAD_FAILED);
        klarvo_core::i18n::assert_is_key(keys::INFERENCE_FAILED);
    }
}
```

`cargo test -p klarvo-plugin-whisper-local` → Exit 0. Tests laufen headless ohne Modell-File (kein echtes whisper.cpp involviert in diesen Tests).

---

### AC-10: Onboarding-Doku im Plugin-Rustdoc

**Given** `lib.rs` hat kein Module-Level-Doc,
**When** AC-10 committed ist,
**Then** am Dateianfang:

```rust
//! `klarvo-plugin-whisper-local` — Local Whisper.cpp STT-Provider.
//!
//! Implements `SttProvider` backed by `whisper-rs` (Rust FFI to `whisper.cpp`).
//!
//! # Substrate-Validation Role
//!
//! This is the second STT-Plugin alongside `klarvo-plugin-groq`. Where Groq uses
//! HTTPS-Cloud-API + Auth + RateLimit, Whisper-Local uses Disk-Loaded-Model +
//! In-Process-Inference. Both satisfy `SttProvider` without trait changes —
//! proving the trait carries both Cloud and Local implementations (ADR-0014).
//!
//! # Configuration
//!
//! Set the model path in Klarvo Settings (Plugin-Setting API):
//!   `plugins.whisper-local.model_path` = `/path/to/model.gguf`
//!
//! # Recommended Model Sizes (ADR-0014 D-2)
//!
//! | Model | Size | Quality | Note |
//! |-------|------|---------|------|
//! | `small` | ~500 MB | **Recommended** | Best quality/size tradeoff for German/English |
//! | `base` | ~150 MB | Low-Resource Fallback | Acceptable for short EN utterances |
//! | `tiny` | ~75 MB | Not recommended | Quality too low for production use (tested by Andy) |
//!
//! Models: download `.gguf` files from `ggerganov/whisper.cpp` releases on GitHub.
//!
//! # Thread Safety
//!
//! `WhisperContext` is wrapped in `Arc<Mutex<_>>` (ADR-0014 D-4).
//! `PipelineStage::process` dispatches inference to a blocking thread pool via
//! `tokio::task::spawn_blocking` — the async executor is never blocked.
//!
//! # Language Hint
//!
//! The `language` parameter (ADR-0014 D-5) is sourced from `settings.output_language()`
//! (i18n Axis 3 per `memory/project_i18n_three_axes`). Pass `None` only if no language
//! axis is configured.
```

## Tasks / Subtasks

- [x] AC-1: Workspace `Cargo.toml` — add `klarvo-plugin-whisper-local` to members
- [x] AC-2: Plugin `Cargo.toml` scaffold + `src/lib.rs` (empty stub that compiles)
  - [x] Verify whisper-rs 0.13 actual API via source read (libclang not available on WSL)
- [x] AC-3: `WhisperLocal` struct + `load()` constructor (error paths first)
- [x] AC-4: `PipelineStage::process` via `spawn_blocking`
  - [x] Verify `WhisperContext: Send` in whisper-rs 0.13 (confirmed via source: unsafe impl Send for WhisperInnerContext)
  - [x] Verify `FullParams` lifetime situation (confirmed: 'a, 'b params; constructed inside closure)
- [x] AC-5: `impl SttProvider for WhisperLocal {}` + `cargo check -p klarvo-plugin-whisper-local` ✓
- [x] AC-6: i18n keys in `en.json` + `de.json` + `cargo xtask lint-events` grün ✓
- [x] AC-7: `build_plugin_registry()` Signatur + Whisper-Conditional-Registration
  - [x] `klarvo-plugin-whisper-local` dep in `shells/windows/src-tauri/Cargo.toml`
  - [x] `output_language` read + pass to function
  - [x] `cargo check --workspace --exclude klarvo-windows-shell` ✓ (windows-shell has Linux compile_error guard; native Windows compile via CI)
- [x] AC-8: `windows-ci.yml` — exclude + dedizierter Step + Job-Timeout + Cache-Bust
- [x] AC-9: Unit-Tests (kein Model-File nötig)
- [x] AC-10: Module-Level-Rustdoc mit Onboarding-Doku

### Review Findings

_From `bmad-code-review` 2026-05-05 (3 layers: Blind Hunter / Edge Case Hunter / Acceptance Auditor; 22 raw findings → 19 unique after dedup; 9 dismissed as spec-explicit / cosmetic / verified-justified)._

- [x] [Review][Patch] Segment-Text-Aggregation auf pure verbatim umstellen [lib.rs:173-208] — D1 resolved: `result.push_str(&seg)` ohne `trim()` / ohne `' '`-Separator. AC-4-Spec-Block in dieser Datei nachgezogen. [Source: Blind F-1; D1 user-decision 2026-05-05]

- [x] [Review][Patch] Boot-time `output_language()` Err-arm logged silently [shells/windows/src-tauri/src/main.rs:353-364] — `match` mit `tracing::warn!`-Fallback statt `unwrap_or_else`.
- [x] [Review][Patch] Windows-CI step 2 fehlt `--all-targets` [.github/workflows/windows-ci.yml:43-46] — `--all-targets` ergänzt; `#[cfg(test)]`-Tests werden jetzt auf MSVC typecheckt.
- [x] [Review][Patch] Empty-transcription Ok("") nicht unterscheidbar von "alle Segmente erroren" [lib.rs:198-208] — `successful_segments`-Counter; `n_segments > 0 && successful_segments == 0` → `AppError(Internal, INFERENCE_FAILED)`.
- [x] [Review][Patch] AC-7 `use klarvo_plugin_whisper_local;` nicht im use-Block ergänzt [shells/windows/src-tauri/src/main.rs:32] — `use klarvo_plugin_whisper_local::WhisperLocal;` (substantive form; bare-crate-Import würde `clippy::single_component_path_imports` auf `-D warnings`-CI-Gate triggern). Call-Site `WhisperLocal::load(...)` entsprechend gekürzt.

- [x] [Review][Defer] Mutex-Poisoning brickt Plugin permanent [lib.rs:1325-1330] — deferred, FFI-Panic ist seltener Failure-Mode; Process-Restart recovert; `parking_lot::Mutex` als follow-up wenn empirisch nötig
- [x] [Review][Defer] Language-Input nicht validiert/normalisiert (z.B. `"de-DE"` / leerer String) [main.rs ↔ lib.rs:1346] — deferred, heute strukturell von `validate_setting_value` gesichert; Phase-2+-Migration-Risk only
- [x] [Review][Defer] Long-Inference nicht cancellable bei App-Shutdown [lib.rs:1320-1401] — deferred, architektural; whisper.cpp `abort_callback`-Wiring + `CancellationToken` ist eigene Story
- [x] [Review][Defer] Sample-Rate-Guard fehlt (16kHz hardcoded) [lib.rs:1355] — deferred, ADR-0006 SD-2 fixt heute auf 16kHz; Phase-2+ wenn variable Sample-Rate eingeführt wird
- [x] [Review][Defer] `spawn_blocking` JoinError-Panic-Payload nicht extrahiert [lib.rs:1394-1400] — deferred, Diagnostic-Quality only; `join_err.into_panic()`-Downcast als follow-up

## Dev Notes

### Kritische Constraints

**1. `whisper-rs` 0.13 API muss verifiziert werden.** Das ADR-Sketch ist intentionell als Approximation markiert. Vor der Implementierung `cargo add whisper-rs@0.13 --no-default-features` in einem Scratch-Crate und dann `cargo doc` aufrufen. Die tatsächliche API kann von der Sketch-Implementierung abweichen.

**2. `FullParams` hat Lifetime-Parameter in einigen whisper-rs-Versionen.** In diesem Fall kann `FullParams` nicht außerhalb der `spawn_blocking`-Closure konstruiert und rüber-gemoved werden (Closure muss `'static` sein). Die Lösung: `FullParams` innerhalb der Closure konstruieren (wie in AC-4 gezeigt).

**3. `WhisperContext` Send-Bound.** whisper-rs 0.13 hat `unsafe impl Send for WhisperContext {}`. Falls das in der installieren Version nicht der Fall ist, schlägt `Arc<Mutex<WhisperContext>>` als `Arc<Mutex<T>>: Send` fehl. Dann eine ADR-0014-Amendment nötig.

**4. Windows-Cross-Compile-Check.** whisper-rs kompiliert C++ — `cargo check -p klarvo-plugin-whisper-local` auf Linux macht die Rust-Analyse, aber die C++ Build-Step läuft auch. Das ist OK für die dev-loop. Für Windows-only-Bugs: `cargo check --target x86_64-pc-windows-gnu -p klarvo-plugin-whisper-local` (per `memory/feedback_windows_cross_compile_verify`).

**5. `cargo xtask lint-events` nach AC-6.** Der Scanner deckt `klarvo-plugins/` ab. Neue Keys müssen durch `debug_assert!(i18n::is_key(keys::XYZ))` an einem der drei Emit-Sites im Plugin erreichbar sein. Kein Eintrag in `orphan-allowlist.txt` nötig.

**6. Settings `validate_setting_value` und leere Pfade.** `settings.get_plugin_setting` returnt `Ok(None)` für fehlende Keys (kein Fehler). Für eine gesetzte aber leere value returniert der Settings-Service `Ok(None)` (leere Strings werden als fehlend behandelt per `get_raw` impl). Das Pfad-Validierung in `WhisperLocal::load` prüft `model_path.exists()` nach dem UTF-8-Check.

### Referenz-Implementierung: klarvo-plugin-groq

Pattern-Vorlage für diese Story:

| Aspect | Groq | Whisper-Local |
|--------|------|---------------|
| `pub const ID` | `"groq"` | `"whisper-local"` |
| `pub mod keys {}` | 7 Keys | 3 Keys |
| Konstruktor | `new(key_store: Arc<dyn KeyStore>)` | `load(model_path: &Path, language: Option<String>)` |
| Fehler-Kategorien | Network/Auth/RateLimit/5xx | Config/Fatal |
| Runtime | async reqwest | spawn_blocking + whisper-rs sync |
| `stage_type()` | `"stt"` | `"stt"` |
| `impl SttProvider for X {}` | ✓ | ✓ (Substrate-Validation) |

Kein `register()` Funktion nötig (Groq hat auch keine — Registration passiert im Shell-Bootstrap via `registry.register_stt(ID, Arc::new(...))`).

### Dateien die berührt werden

**NEU:**
- `klarvo-plugins/klarvo-plugin-whisper-local/Cargo.toml`
- `klarvo-plugins/klarvo-plugin-whisper-local/src/lib.rs`

**UPDATE:**
- `Cargo.toml` (workspace root) — `workspace.members` ergänzen
- `shells/windows/src-tauri/Cargo.toml` — dependency `klarvo-plugin-whisper-local`
- `shells/windows/src-tauri/src/main.rs` — `build_plugin_registry()` Signatur + Body + `output_language` read
- `shells/windows/locales/en.json` — 3 neue Keys
- `shells/windows/locales/de.json` — 3 neue Keys
- `.github/workflows/windows-ci.yml` — exclude + dedizierter Step + timeout

**NICHT ändern:**
- `pipeline-manifest.toml` — bleibt `type = "passthrough"` (User-Entscheidung)
- `klarvo-core` irgendwelche Dateien — Core-Änderung widerspricht Substrate-Validation-Goal
- `klarvo-plugin-groq` — kein Touch

### Vorherige Story Learnings (Story 9.6 Code Review)

Aus der Pill-Bar-Closure:
- Rebase-Disziplin: Lines, die per Rebase entfernt werden, sind verloren (`memory/feedback_rebase_restoration_discipline`)
- Windows-Cross-Compile-Verify vor Story-Closure (`memory/feedback_windows_cross_compile_verify`)
- Kein `git add .` — immer spezifische Files (`memory/feedback_commit_hygiene`)

### Project Structure Notes

```
klarvo/
├── Cargo.toml                             # workspace — AC-1
├── pipeline-manifest.toml                 # NICHT ÄNDERN
├── klarvo-plugins/
│   ├── klarvo-plugin-groq/                # Referenz-Pattern
│   └── klarvo-plugin-whisper-local/       # NEU (AC-2)
│       ├── Cargo.toml
│       └── src/lib.rs
├── shells/windows/
│   ├── src-tauri/
│   │   ├── Cargo.toml                     # dep whisper-local (AC-7)
│   │   └── src/main.rs                    # build_plugin_registry() (AC-7)
│   └── locales/
│       ├── en.json                        # 3 Keys (AC-6)
│       └── de.json                        # 3 Keys (AC-6)
└── .github/workflows/windows-ci.yml       # AC-8
```

### References

- ADR-0014: `docs/adr/0014-second-stt-plugin-whisper-local.md` (Accepted, alle 5 Decisions D-1..D-5)
- Referenz-Plugin: `klarvo-plugins/klarvo-plugin-groq/src/lib.rs`
- SttProvider Trait: `klarvo-core/src/traits/stt.rs`
- Plugin-Registry: `klarvo-core/src/registry.rs` (`register_stt`, `stt` getter)
- Settings-API: `klarvo-core/src/settings/mod.rs` (`get_plugin_setting`, `output_language`)
- Windows-Shell-Bootstrap: `shells/windows/src-tauri/src/main.rs` (Step 2c–Step 8)
- i18n-Linting: `xtask/src/lint_events.rs` (G3-Sub-Lint D Scanner-Pfade)
- CI: `.github/workflows/windows-ci.yml`
- Workspace: `Cargo.toml` root

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

### Completion Notes List

- API-Deviation D1: `AppErrorKind::Fatal` in the AC spec does not exist in `klarvo-core`. All inference-path errors use `AppErrorKind::Internal` (invariant violation) instead.
- API-Verification: whisper-rs API read directly from crate source. Key findings: `WhisperContext::new_with_params(path: &str, params: WhisperContextParameters)` ✓; `WhisperState::full` returns `Result<c_int, WhisperError>` (not `Result<(), ...>` as sketched); `FullParams<'a, 'b>` has two lifetime params (constructed inside `spawn_blocking` closure); `WhisperInnerContext: Send + Sync` (unsafe impl).
- Test-Deviation D2: `Result::expect_err` requires `T: Debug`; `WhisperContext` (the wrapper) does not derive `Debug`. Replaced `result.expect_err(...)` with explicit `match` in `load_rejects_nonexistent_model_path`.
- Verifications: `cargo check -p klarvo-plugin-whisper-local` ✓; `cargo test -p klarvo-plugin-whisper-local` ✓ (3/3); `cargo xtask lint-events` ✓; `cargo check --workspace --exclude klarvo-windows-shell` ✓.
- Local Linux dev requires `libclang-dev` + `cmake` + g++ for whisper.cpp compilation. Windows CI (windows-latest) has MSVC toolchain natively.
- Windows cross-compile from Linux (`x86_64-pc-windows-gnu`) blocked by host-libclang/MinGW header mismatch in whisper-rs-sys bindgen — known crate issue, irrelevant for native Windows CI.

### File List

- `klarvo-plugins/klarvo-plugin-whisper-local/Cargo.toml` (new)
- `klarvo-plugins/klarvo-plugin-whisper-local/src/lib.rs` (new)
- `Cargo.toml` (AC-1: member added)
- `shells/windows/src-tauri/Cargo.toml` (AC-7: dep added)
- `shells/windows/src-tauri/src/main.rs` (AC-7: build_plugin_registry updated)
- `shells/windows/locales/en.json` (AC-6: 3 keys)
- `shells/windows/locales/de.json` (AC-6: 3 keys)
- `.github/workflows/windows-ci.yml` (AC-8: exclude + step + timeout + cache-bust)
