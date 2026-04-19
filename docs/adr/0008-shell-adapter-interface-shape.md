# ADR-0008: Shell-Delivery-Interface-Shape (OutputTarget-Trait, architecture-reconvergent)

**Status:** Proposed
**Date:** 2026-04-19

## Context

Epic 2 (End-to-End Dictation Pipeline, FR12-17) ends with:
- **FR17:** „`klarvo-core` delivers final Cleanup-Output an Shell-Adapter für Paste-Injection."
- **FR21** (Epic 3, Windows-Shell): „Windows-Shell performs Auto-Paste in active Foreground-Window via Clipboard-Set + simuliertes `Ctrl+V`."

Die Frage: **wie** wird der finale Cleanup-Text aus dem Core an die Shell transportiert? Welche Trait-/Interface-Shape nimmt der Delivery-Kontrakt?

Architecture.md §Plugin-System (Zeile 234) listet 7 Phase-0-Traits, darunter explizit **`OutputTarget`**. Architecture.md §Directory-Structure (Zeile 1035) listet `klarvo-plugins/klarvo-plugin-clipboard/` als OutputTarget-Reference-Implementation.

Phase-1-Status (Stand 2026-04-19, nach Epic-1A/1B/1C-Closure):
- Epic 1A (FR1-4) baute 4 Traits: `PipelineStage`, `SttProvider`, `CleanupStyle`, `VadProvider` — der **4-Trait-Data-Flow-Stability-Ring**.
- `OutputTarget` wurde in Epic 1A **nicht** eingeführt — lag außerhalb FR1-4-Scope. Das ist **nicht** aktive Architecture-Divergenz, sondern Phase-1-Scope-Narrowing: Phase-1-Trait-Surface ist **Subset** von Phase-0-Architecture-Plan, nicht Deviation.
- `KeyStore` (Epic 1C) ist separate Infrastructure-Trait-Category (per `memory/project_keystore_trait_surface`), **orthogonal** zum 4-Trait-Ring.

Decision-Space:
- **(a) OutputTarget-Trait jetzt in Phase 1 einführen** (architecture-reconvergent, `klarvo-plugin-clipboard/` als Reference-Impl)
- **(b) Dedicated `ShellAdapter`-Trait neu erfinden** (distinct von OutputTarget — begründungspflichtig gegen architecture.md per `memory/feedback_architecture_doc_authoritative`)
- **(c) Pipeline-Sink-Stage-Variant** (`PipelineStageType::Sink { plugin_id }` — Shell-Delivery als weitere Pipeline-Stage)

## Decision

**Gewählt: (a) OutputTarget-Trait jetzt in Phase 1 einführen.** Architecture-reconvergent — die Phase-1-Narrowing wird in Epic 2 wieder auf den architecture.md-Plan erweitert.

### Sub-Decisions

**1. Trait-Signatur.**

**Location:** `klarvo-core::traits::OutputTarget` (re-exported aus `klarvo-core/src/traits/output.rs`).

```rust
#[async_trait]
pub trait OutputTarget: Send + Sync + 'static {
    /// Deliver the final cleaned text to the target (clipboard, direct
    /// keystroke injection, file, network-endpoint, etc.).
    ///
    /// Returns `Ok(())` on success. On failure, returns `AppError` with
    /// an i18n-key `user_message` (Core emits keys, Shell resolves per
    /// memory/project_i18n_core_contract).
    async fn deliver(&self, text: &str) -> Result<(), AppError>;
}
```

**Invarianten:**
- **`text: &str`**, nicht `String`, nicht `SecretString` — Cleanup-Output ist nicht-sensitive per Phase-1-Policy (verbatim-Passthrough); später per Plugin-Contract falls Privacy-sensitive Outputs landen. Zero-Copy-Input, Target kann intern clonen wenn nötig.
- **`&self`**, nicht `&mut self` — OutputTarget-Impls sind idempotent-per-call und thread-safe (z. B. Clipboard-Set kann aus beliebigem Consumer-Task kommen). Ein Receiver-Buffer oder State (z. B. Retry-Counter) gehört nicht in die Trait-Surface, sondern in Impl-interne `Arc<Mutex<...>>` wenn benötigt.
- **`async fn`** — Windows-Clipboard-API kann synchron sein, aber Android-Equivalent (Accessibility-Service-Inject) und zukünftige Network-Targets sind inhärent async. `async_trait` für Object-Safety (`Box<dyn OutputTarget>`) in PluginRegistry.
- **`AppError`-Return mit i18n-Key** — Core darf keine User-facing-Strings enthalten (`memory/project_i18n_core_contract`). Impls emittieren Keys wie `"error.output.clipboard_unavailable"` — Shell resolved.

**2. OutputTarget ist Plugin-Contract-Trait, NICHT Infrastructure-Trait.**

Anders als `KeyStore` (Infrastructure, Single-Instance-per-Binary, kein Registry-Lookup) ist OutputTarget ein **Plugin-Contract**: mehrere Impls existieren parallel (Clipboard, Keystroke-Inject, File, …) und werden via Pipeline-Manifest / Shell-Config ausgewählt. Entsprechend:
- **PluginRegistry**-Slot analog zu SttProvider/CleanupStyle (Post-1B.5-Pattern per `memory/project_executor_stage_data_shape`):

```rust
// Additive Extension zur PluginRegistry:
pub struct PluginRegistry {
    stt: HashMap<String, Arc<dyn SttProvider>>,
    cleanup: HashMap<String, Arc<dyn CleanupStyle>>,
    output: HashMap<String, Arc<dyn OutputTarget>>,  // NEW
}

impl PluginRegistry {
    pub fn register_output(&mut self, id: impl Into<String>, plugin: Arc<dyn OutputTarget>);
    pub fn output(&self, id: &str) -> Option<Arc<dyn OutputTarget>>;
}
```
- Duplicate-ID-Register panicked (analog `register_stt`/`register_cleanup`).

**3. OutputTarget ist NICHT Teil des 4-Trait-Data-Flow-Stability-Rings.**

Der 4-Trait-Ring (`PipelineStage`/`SttProvider`/`CleanupStyle`/`VadProvider`) ist **Data-Flow-Traits** (Input → Output-Transforms innerhalb der Pipeline). OutputTarget ist **Terminal-Sink-Trait** (Pipeline-Exit zu Shell-Side-Effect). Gleiche Signatur-Stabilitäts-Disziplin gilt (nicht breaking in Phase 1+), aber **begrifflich separat** — der Ring bleibt 4, nicht 5.

Präzedenz-Konsistenz: PRD `triplecount` des Trait-Ring-Counts in Story-ACs + architecture-docs muss weiter „4 Traits" sagen. Phase-1-Amendment zu PRD FR1 NICHT erforderlich, weil OutputTarget **explizit außerhalb** des Rings kategorisiert ist.

**4. Pipeline-Dispatch: NICHT als PipelineStage-Variant, sondern als Executor-Terminal-Step.**

**Rejected-Alternative:** `PipelineStageType::Output { plugin_id }` als Stage-Type im Manifest.

**Grund (load-bearing):** `memory/project_executor_stage_data_shape` ist Phase-1-locked per Story-1B.5-Closure. Stage-Type-Enum-Extension würde Epic-1B-Reopening-Amendment erzwingen (Dispatch-Match muss forced-update-invariant preserve). Option (a) vermeidet das komplett — OutputTarget ist **nicht** Pipeline-Stage, sondern Post-Pipeline-Delivery-Step, den der Shell/Caller selbst orchestriert:

```rust
// Shell-wiring pattern (Story 2.4):
let result: StageData = run_pipeline(&manifest, &registry, input).await?;
let StageData::Text(final_text) = result else { return Err(...); };
let output = registry.output(&shell_config.output_target_id)
    .ok_or(AppError::plugin_not_found(...))?;
output.deliver(&final_text).await?;
```

Diese Trennung (Pipeline produces `StageData::Text`, Shell orchestriert Delivery via OutputTarget) ist **cleaner Separation-of-Concerns** als Shell-Delivery-in-Pipeline-Stage-gedrückt. Außerdem:
- Shell kann OutputTarget unabhängig von Pipeline-Re-Config switchen (z. B. User-Preference „Clipboard vs Direct-Keystroke" ist Shell-Config, kein Pipeline-Config).
- Pipeline-Manifest (für Phase-1 embedded-default) bleibt frei von Platform-Specific Outputs.

**5. Phase-1-Reference-Impl: `klarvo-plugin-clipboard` (Windows-Clipboard via `arboard`-Crate oder equivalent).**

Ein konkreter Impl-Crate in `klarvo-plugins/klarvo-plugin-clipboard/` (architecture.md:1035-konform). Setzt den Clipboard-Inhalt; das tatsächliche `Ctrl+V`-Simulieren (FR21) ist Shell-Responsibility (Windows-Shell-AutoPaste-Handler, nicht OutputTarget-intern). D. h. OutputTarget liefert Text an die Zwischenablage, Shell triggert Paste-Keystroke.

Rationale: Keystroke-Injection ist Platform-API-Access (SendInput auf Windows), gehört in Shell-Scope, nicht in einen Platform-neutralen Plugin-Crate. Phase-2-Optional: `klarvo-plugin-keystroke` (architecture.md:1036) als Direct-Keystroke-Output-Plugin — das wäre Platform-specific und würde das SendInput-Pattern kapseln.

## Alternatives Considered

**(b) Dedicated `ShellAdapter`-Trait neu erfinden.**
Rejected: Would be active Architecture-Divergence from architecture.md:234 + 1035 per `memory/feedback_architecture_doc_authoritative`. No empirical Duplication-Signal justifies a separate Trait (per `memory/feedback_premature_abstraction_guard` — factor-out erst bei provenen Duplication, nicht speculative). OutputTarget's Signature (`async fn deliver(&self, text: &str) -> Result<(), AppError>`) is simple and exhaustive enough for all known Phase-1/2-Targets (Clipboard, Keystroke, File, Network).

**(c) Pipeline-Sink-Stage-Variant (`PipelineStageType::Sink`).**
Rejected: Würde `memory/project_executor_stage_data_shape` Stage-Type-Enum-Variant-Extension erzwingen → Epic-1B-Closure-Amendment (StageData-Enum add `Unit`-Variant oder equivalent, Dispatch-Match alle Sites updaten). Epic 1B ist closed per Commit 104820e; Reopening ist signifikanter Prozess-Overhead vs. Nutzen. Zusätzlich konflatiert Sink-as-Stage die Semantik (Stage-Output-is-Input-for-next-Stage vs. Terminal-Side-Effect) — Pipeline-Stages sind Transform-Traits, Sinks sind Delivery-Traits, semantic-distinct.

## Consequences

**Positiv:**
- **Architecture-reconvergent:** Erweitert Phase-1-Trait-Surface zurück zum architecture.md-Plan, ohne aktive Divergenz zu erzeugen. Kein Architecture-Amendment nötig — architecture.md:234 + 1035 war bereits richtig, Phase-1 fügt lediglich den bisher nicht-implementierten Trait an.
- **Clean Separation-of-Concerns:** Pipeline produziert `StageData::Text`, Shell delivered via OutputTarget. Platform-specific-Delivery bleibt aus Pipeline-Manifest raus.
- **Phase-2-Extension-Ready:** `klarvo-plugin-keystroke` (architecture.md:1036) folgt demselben Trait — Additive Impl ohne Trait-Change.
- **PluginRegistry-Pattern konsistent:** Registry-Slot + Arc<dyn>-Collection folgt 1B.5-Pattern. Vertraute Discovery/Registration-Mechanik für Andy und zukünftige Reviewer.
- **Kein Epic-1B-Reopening:** `project_executor_stage_data_shape` bleibt Phase-1-locked. StageData-Enum bleibt 2-Variant (`Text`+`Audio`).

**Negativ / akzeptierte Schulden:**
- Shell-Caller muss Two-Step-Invocation machen (`run_pipeline` + `output.deliver`) statt Pipeline-Terminal-Dispatch. Das ist ~5 Zeilen Wiring-Code pro Shell, akzeptabel.
- OutputTarget-Impl-Wahl ist Shell-Config (nicht Pipeline-Manifest). Phase-2+-User, die „pro Pipeline anderes Output-Target" wollen, brauchen Pipeline-Config-Erweiterung — nicht Phase-1-Scope. Forward-Ref zu Epic 4.
- `arboard`-Crate-Choice für Clipboard-Reference-Impl ist nicht in ADR-0005-Stack (ADR-0005 regelt nur HTTPS-Client). Dependency-Policy für Clipboard-Crate ist Story-2.5-Implementation-Concern — kein separates ADR nötig es sei denn Andy sieht load-bearing Divergenz.

**Epic-2-Story-Impacts:**
- **Story 2.4 (End-to-End Headless Flow):** Integration-Test registriert Test-OutputTarget-Mock (in `klarvo-test-fixtures`, analog `InMemoryKeyStore`), wired Shell-Loop calls `output.deliver` nach `run_pipeline`-Return.
- **Story 2.5 (Clipboard-Reference-Impl):** Baut `klarvo-plugin-clipboard` Crate, Impl-via-`arboard` o.ä., Unit-Test via Test-Clipboard oder Mock-Layer.
- Epic 2 Manifest-Driven-Scope bleibt klar: Manifest drives Pipeline-Stages (STT, Cleanup), Shell drives OutputTarget-Wahl. Wenn User später config-driven OutputTarget-Selection will → Epic 4 Settings.

**Epic-3-Downstream-Impact (FR21 Windows-Auto-Paste):**
- Windows-Shell hat zwei Concerns: (i) Clipboard-Set via OutputTarget-Plugin-Invocation, (ii) `Ctrl+V`-Keystroke-Injection via Win32-SendInput. (i) ist Plugin-Concern, (ii) bleibt Shell-Native-Code. FR21-AC-Shape: Shell-Integration-Test asserted Clipboard-Content + Keystroke-Injected-Event.

**Forward-References Phase 2+:**
- `klarvo-plugin-keystroke` (architecture.md:1036) als Direct-Keystroke-Inject-Plugin (bypasses Clipboard).
- Pipeline-Config-driven OutputTarget-Selection → Epic 4.
- Android-OutputTarget-Impl (Phase 3) via Accessibility-Service.

## Open Questions (for Andy-review)

- **Q1:** Cleanup-Text-Sensitivity: Phase-1-Policy „verbatim nicht-sensitive" ist plausibel für Dev-Walking-Skeleton. Phase-2+ mit Polished/Chat-Styles könnte LLM-Outputs enthalten, die personenbezogene Daten aus Dictation-Context weiter-verarbeiten. Ob `text: &str` oder `SecretString` wird — ist load-bearing für Phase-2-API-Stability. Vorschlag: `&str` in Phase 1, Revisit bei Polished-Plugin-Einführung. Reviewer kann in Amendment fixieren oder Decision auf Epic 4 deferen.
- **Q2:** OutputTarget-Plugin-Init-Failures (z. B. Clipboard-System-Unavailable auf Server-Windows-Core): Shell-Startup-fatal (analog NFR10 Plugin-Init-Failures) oder Per-Dictation-Session-Retry? Vorschlag: Startup-fatal per NFR10, User sieht kontrolliertes Exit. Reviewer bestätigt.
- **Q3:** Soll es eine i18n-Key-Konvention `"error.output.<reason>"` geben (parallel zu `"error.keystore.<reason>"` aus Epic 1C)? Vorschlag: ja, Story 2.5 definiert initial-Keys (`"error.output.clipboard_unavailable"`, `"error.output.target_not_found"`). Reviewer bestätigt oder erweitert.

## Cross-References

- `output/planning-artifacts/architecture.md` §Plugin-System Zeile 234, §Directory-Structure Zeile 1035-1036
- `output/planning-artifacts/prd.md` FR17 (Shell-Delivery), FR21 (Windows-Auto-Paste)
- `memory/project_executor_stage_data_shape` (Runtime-Contract 1B.5 — StageData-Enum bleibt 2-Variant, PluginRegistry-Additive-Extension-Pattern)
- `memory/project_keystore_trait_surface` (Infrastructure-Trait-Precedent — OutputTarget ist NICHT Infrastructure, sondern Plugin-Contract distinct von 4-Trait-Ring)
- `memory/project_i18n_core_contract` (Core emits i18n-Keys, Shell resolves)
- `memory/feedback_architecture_doc_authoritative` (Korollar — Option (a) ist architecture-reconvergent, kein Abweichungs-ADR; Option (b)/(c) wären aktive Divergenzen)
- `memory/feedback_premature_abstraction_guard` (keine dedizierte Trait-Erfindung ohne proven Duplication)
- ADR-0004 (SecretString-Precedent für Q1-Review)
