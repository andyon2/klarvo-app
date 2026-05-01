# ADR-0014: Second STT Plugin — Whisper-Local

**Status:** Accepted
**Date:** 2026-05-01 (Stub Proposed; 5 Open Questions in derselben Session resolved → Accepted)
**Source:** Phase-2-A-Retrospektive AI-6; `_archive/phase-2-scope-lock.md` Pre-Story-Decision §168 (2nd-STT-Plugin-Choice — historisch).

## Context

Phase-2-B Story 2.B.B1 (Zweiter STT-Plugin) ist Substrate-Validation des Plugin-Trait-Surface — Beweis, dass `SttProvider`-Trait nicht versteckt Cloud-API-Annahmen kodiert (Brief §Erfolgskriterien). Phase-1 hat nur `klarvo-plugin-groq` (Cloud-HTTPS via Groq Whisper); ein zweiter, struktur-anders gearteter STT-Plugin ist nötig, um das Trait empirisch zu validieren.

Rahmenbedingungen:
- **Trait-Surface lock:** `SttProvider: PipelineStage<Input = AudioBuffer, Output = String>` ist Phase-1-Locked (`klarvo-core/src/traits/stt.rs`). Jede Trait-Änderung triggert Breaking-Change-Review.
- **Sample-Rate-Constraint:** Phase-1-Pipeline produziert 16 kHz mono f32 (`AudioBuffer.sample_rate`). Whisper.cpp erwartet exakt das.
- **AppError-Mapping:** STT-Plugins emittieren `AppError::kind` aus {`Network`, `Auth`, `RateLimit`, `UpstreamUnavailable`, `Fatal`} (`klarvo-core/src/traits/stt.rs:21`). Local-Plugin nutzt nur `Fatal` + neue/spezifische Varianten, falls nötig.
- **Plugin-Crate-Konvention:** `klarvo-plugins/klarvo-plugin-<name>/`, eigenes Cargo-Crate, Cargo-Feature-Gate über Pipeline-Executor (per `memory/project_manifest_boot_time_parse`).
- **No-P1-Konflikt:** Post-MVP-P1 enthält "Whisper-Modes" (scope-lock §58) — gemeint ist OpenAI-Whisper-Cloud-API. Lokales Whisper kollidiert dort nicht.
- **BYOK-Narrativ:** Klarvo's Marktpositionierung ist BYOK + Power-User (`memory/project_market_positioning`). Lokales Whisper passt strukturell — kein Account, kein Key, kein Netzwerk.

## Decision

**Gewählt: Whisper-Local als zweiter STT-Plugin**, in zwei Festlegungen:

1. **Crate-Name:** `klarvo-plugin-whisper-local`.
2. **Library:** `whisper-rs` (Rust-FFI zu `whisper.cpp` upstream).

**Rationale:**

- **Cloud-vs-Local-Differenzierung:** Groq ist HTTPS-Cloud-API mit Multipart-Upload + Auth + RateLimit-Surface. Whisper-Local ist Disk-Loaded-Model + In-Process-Inference ohne Network/Auth. Wenn `SttProvider`-Trait beide ohne Trait-Erweiterung trägt, ist Substrate-Validation strukturell bewiesen.
- **whisper.cpp ist State-of-the-Art für lokale Inference auf CPU/GPU:** Metal/CUDA/Vulkan-Backends, etablierte Model-File-Formate (ggml/gguf), aktive Maintenance.
- **`whisper-rs` ist die etablierte Rust-Anbindung:** v0.13+, FFI zu whisper.cpp, kompiliert sub-crate-statisch, keine Runtime-Dep auf System-Libraries (außer optional GPU-SDKs).
- **Reines Rust-native (`candle-whisper`/Mistral.rs) verworfen für jetzt:** Maturity-Risiko zu hoch für ein Substrate-Validation-Plugin. Whisper.cpp-Performance + Stability ist Bench-Ground-Truth.
- **Kein P1-OpenAI-Whisper-Cloud-Konflikt:** `klarvo-plugin-whisper-local` und ein hypothetisches späteres `klarvo-plugin-openai-whisper` (Cloud) sind orthogonale Crates — Naming-Trennung explizit gemacht.

## Consequences

**Positiv:**

- Substrate-Validation für `SttProvider`-Trait empirisch (Cloud + Local in einem Trait).
- Power-User-Targetgruppe (`memory/project_market_positioning`) bekommt offline-fähige STT-Option ohne Account/Key — strikt BYOK-konform.
- Plugin-Author-Onboarding hat zwei strukturell unterschiedliche Reference-Impls für Plugin-Author-Doku (Phase-2-B-nice-to-have C5, Phase-3+).
- Phase-3-Android: lokales Whisper auf Mobile ist Mid-Term-Plausibel (whisper.cpp läuft auf ARM mit reduzierter Model-Size); Cloud-only-Pfad würde diese Option verbauen.

**Negativ / Aufwand:**

- **Model-File-Distribution** ist neuer UX-Surface. Phase-1 hatte nur API-Keys (KeyStore-Trait). Model-Files sind hundert MB+, brauchen Pfad-Setting + ggf. Download-Mechanismus (Open Question OQ-2).
- **Whisper-Inference ist sync/blocking** — `process` muss in `tokio::task::spawn_blocking` gewrapt werden, damit der async Pipeline-Executor nicht blockiert.
- **GPU-Backend-Compile-Surface:** whisper-rs-Cargo-Features `cuda`/`metal`/`vulkan`/`hipblas` schalten unterschiedliche Builds. Phase-2-B muss eine Default-Auswahl treffen (Open Question OQ-4).
- **Erstmaliger Cargo-Build-Aufwand:** whisper-rs kompiliert C++-Quelle ein → CI-Build-Time steigt. E1 Windows-Compile-CI-Gate (Phase-2-A) muss timeout-Erweiterung bekommen.
- **Concurrent-Use:** Model-File ist groß (Default-Wahl pending). Mehrfach-Laden im Plugin ist nicht praktikabel; `Arc<Mutex<WhisperContext>>` ist die naheliegende Strukturlösung — Trait-Verträglichkeit zu prüfen (`PipelineStage` ist `&self`-basiert, also OK; Mutex-Contention im Single-Pipeline-Lauf irrelevant, weil pro Hotkey-Cycle nur ein Transcribe-Call).

## Decisions (Andy 2026-05-01, post Phase-2-A-Retro AI-6)

Die fünf in der Stub-Phase aufgeworfenen Open Questions wurden in derselben Session entschieden:

| ID | Topic | Decision | Note |
|----|-------|----------|------|
| **D-1** | Model-Distribution-Default | **User-supplied Path (BYO-Model)** | Download-on-demand als Phase-3-Onboarding-Polish-Trigger, nicht 2.B.B1-Substrate-Validation. Settings hat einen Pfad-Picker zum `.gguf`-File. |
| **D-2** | Default-Model-Size (Doku-Empfehlung) | **small (~500 MB)** | Empirisch begründet (Andy hat tiny/base getestet — „viel zu schlecht" auf Deutsch). Stub-Empfehlung war (b) base, in der Decision-Session überstimmt durch Power-User-Persona-Quality-Anspruch. tiny/base bleiben als Low-Resource-Fallback in der Doku-Tabelle. |
| **D-3** | GPU-Backend | **Compile-Feature-Flag, `cpu-only`-Default für CI** | Realistisch für 2.B.B1: nur cpu-only in E1 Windows-Compile-CI. CUDA/Metal/Vulkan als separate Release-Build-Targets erst wenn Hardware-konkret (Premature-Abstraction-Guard, `feedback_premature_abstraction_guard`). |
| **D-4** | Concurrent-Use-Locking | **`Arc<Mutex<WhisperContext>>`** | Pipeline pro Hotkey-Cycle Single-Concurrent-Call (`memory/project_shell_session_lifecycle`); Mutex-Contention strukturell irrelevant. RwLock kein Vorteil, weil `WhisperState::full(...)` mutiert. |
| **D-5** | Language-Hint-Source | **Explizit aus Settings-Output-Language-Achse** | `memory/project_i18n_three_axes` Axis 3 (Output-Language). whisper-rs-`set_language(...)` wird aus `settings.output.language` gespeist. Auto-Detect verworfen wegen Brittleness bei kurzen Utterances. |

**Effekt:** Der ADR ist `Accepted`. Story 2.B.B1 (Zweiter STT-Plugin) ist nicht mehr durch Architektur-Decisions blockiert — Implementations-Spec kann gegen diese 5 Festlegungen geschrieben werden.

**Folge-Items für 2.B.B1-Story-Spec:**
- Settings-Schema-Erweiterung um `whisper_local.model_path` (D-1) und Sicherstellung dass `output.language` von der Pipeline an den Plugin propagiert wird (D-5).
- E1-Workflow-Erweiterung um whisper-rs-Compile (D-3): timeout-Erhöhung wahrscheinlich nötig (whisper.cpp C++-Build).
- Onboarding-Doc-Tabelle Model-Sizes mit empfohlenem Default „small" + Low-Resource-Fallback-Hinweis (D-2).

## Trait-Conformance-Sketch

Skeleton-Pattern (zur Validierung des Substrate-Goals; nicht implementiert in diesem ADR):

```rust
pub struct WhisperLocal {
    ctx: Arc<Mutex<whisper_rs::WhisperContext>>,
    language: Option<String>,
}

#[async_trait]
impl PipelineStage for WhisperLocal {
    type Input = AudioBuffer;
    type Output = String;

    async fn process(&self, audio: AudioBuffer) -> Result<String, AppError> {
        let ctx = self.ctx.clone();
        let lang = self.language.clone();
        tokio::task::spawn_blocking(move || {
            let mut state = ctx.lock().expect("whisper-ctx mutex poisoned").create_state()?;
            // params from lang; samples from audio.samples
            state.full(...)?;
            // collect segments → String
        })
        .await
        .map_err(|e| AppError::fatal(...))?
    }
}

impl SttProvider for WhisperLocal {}
```

Validation-Goal: kein Member, der Cloud-spezifisch ist (kein `reqwest::Client`, kein `api_key`, kein `endpoint_url`); nur `WhisperContext` + `language`. Wenn der Trait das trägt → Substrate validiert.

## References

- `_archive/phase-2-scope-lock.md` §168 (Pre-Story-Decision-Item — historisch).
- `epic-phase-2-a-retro-2026-05-01.md` AI-6.
- `klarvo-core/src/traits/stt.rs` — Trait-Definition (Phase-1-Locked).
- `klarvo-plugins/klarvo-plugin-groq` — Cloud-Reference-Impl.
- `memory/project_market_positioning` — BYOK + Power-User-Persona.
- `memory/project_manifest_boot_time_parse` — Cargo-Feature-Gate-Modell.
- `memory/project_i18n_three_axes` — Output-Language-Axis (OQ-5).
- ADR-0006 — AudioSource-Trait + 16-kHz-mono-f32-Constraint.
