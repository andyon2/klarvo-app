# ADR-0010: AppErrorKind-Extension (4 PRD-FR-Alignment Variants + Unavailable→UpstreamUnavailable Rename)

**Status:** Accepted
**Date:** 2026-04-20

## Context

PRD-FR23, FR28, FR29, FR30 reference konkrete `AppError::kind`-Variants:
- FR23: `AppError::kind::PermissionDenied` (MIC-Permission-denied, Windows-Shell-Bootstrap-Kontext)
- FR28: `AppError::kind::PipelineValidation` (Pipeline-Strict-Error bei unknown-stage-type — per `feedback_manifest_compile_contract`-Gate)
- FR29: `AppError::kind::UpstreamUnavailable` (Groq-5xx/Timeout/Network-Error)
- FR30: `AppError::kind::KeyMissing` + „Plugin-Identifier in Cause"

Architecture.md §Error-Shape :639-646 listet 6 generic Varianten: `Network | Auth | Validation | RateLimit | Internal | Unavailable`. Current Code (`klarvo-core/src/error.rs:31-38`, Epic-1-Closure-Commit) matched Arch-Doc.

**Gap:** 3 PRD-FR-Namen existieren nicht im Enum. 1 Variant (`Unavailable`) ist semantisch-unterspezifiziert gegenüber `UpstreamUnavailable` aus FR29.

**Reviewer-Grep-Verification (2026-04-20):**
- `AppErrorKind::Unavailable` hat exakt **eine** Production-Use-Site (`impl From<PluginError>` Arm für `PluginError::Unavailable`, `error.rs:75-80`).
- `PluginError::Unavailable(String)` ist Source-Variante, ebenfalls nur dort genutzt.
- 1 Beispiel-Referenz in `architecture.md:823` (Good-Pattern-Snippet).

Rename-Scope ist 2 Variant-Renames + 1 Match-Arm + 1 Arch-Doc-Snippet + 1 Table-Row. Kein Epic-1-Re-Spec nötig.

Timing: Epic 2 (Specs 2.3 + 2.6) ist commit-appended aber **noch nicht code-implementiert** (Workspace-Verify 2026-04-20). ADR-0010 ist **Epic-2-Pre-Flight-Resolution**, nicht Epic-3-Blocker — Code-Start profitiert von konsolidiertem ErrorKind-Surface.

## Decision

**Gewählt:** (A) Extend `AppErrorKind` mit 3 neuen Unit-Variants + Rename existing `Unavailable` → `UpstreamUnavailable`. Asymmetrische PluginError-Extension.

### Sub-Decisions

**1. Rename `AppErrorKind::Unavailable` → `AppErrorKind::UpstreamUnavailable`.**

Semantik-Alignment zu FR29-Wording. Kein Backward-Compat-Hack (keep-and-add) weil:
- Einzige bestehende Use-Site (`From<PluginError::Unavailable>`) ist genau Upstream-Context (Plugin-Backend-Unavailable). „Generic Unavailable" existiert im Code nicht; die Unterscheidung wäre artifizielle Retro-Classification.
- `#[non_exhaustive]` erlaubt Rename-as-Extension via Compile-Time-Error an allen Consumern; Reviewer-Grep zeigt keine Consumer.

Parallel-Rename `PluginError::Unavailable(String)` → `PluginError::UpstreamUnavailable(String)` für Semantik-Konsistenz an der Plugin-Boundary.

**2. Add 3 neue `AppErrorKind`-Unit-Variants: `PermissionDenied`, `PipelineValidation`, `KeyMissing`.**

Unit-Variants (nicht Struct-Variants) — preserviert Wire-flat-String-Kind per Arch-Doc :649. Zusatzdaten (plugin_id, offending-stage-type) landen in `AppError.message` (technisch, Logs) oder werden Phase-2+ über eigene struct-Felder (`retryAfterMs`-Präzedenz arch :653) ergänzt wenn empirisch motiviert.

```rust
#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AppErrorKind {
    /// Network-layer failure (TCP, DNS, TLS).
    /// Typical retryable=true.
    Network,
    /// Authentication/authorization rejection (401, 403).
    /// Typical retryable=false.
    Auth,
    /// Client-input validation failure. Distinct from `PipelineValidation`
    /// (boot-time manifest-strict-error).
    /// Typical retryable=false.
    Validation,
    /// Upstream rate-limit signal (429 + retry_after_ms).
    /// Typical retryable=true.
    RateLimit,
    /// Programmer-error, logic-bug, invariant-violation.
    /// Typical retryable=false.
    Internal,
    /// Upstream provider unavailable (5xx, timeout, connection-reset).
    /// Typical retryable=true.
    UpstreamUnavailable,
    /// OS-level permission denied (e.g., microphone, accessibility-service).
    /// Typical retryable=false — requires user-action at OS-level.
    PermissionDenied,
    /// Manifest strict-validation error at boot-time (unknown stage-type,
    /// type-mismatch). Distinct from `Validation` (runtime client-input).
    /// Typical retryable=false.
    PipelineValidation,
    /// KeyStore-lookup miss during plugin-init. Plugin-identifier lands
    /// in `AppError.message`.
    /// Typical retryable=false.
    KeyMissing,
}
```

**3. `PluginError` asymmetric extension: only `KeyMissing { plugin_id: String }` added.**

Rationale:
- **`PermissionDenied` is non-plugin-surface.** MIC-Permission-Check lives in Shell (Windows-cpal device-enumeration returns system-level-error; Android-Runtime-Permission via AccessibilityService). No plugin constructs PermissionDenied.
- **`PipelineValidation` is non-plugin-surface.** Manifest-Strict-Parse is Core-Executor-Boot-Time (`feedback_manifest_compile_contract`). No plugin constructs PipelineValidation; Core-Executor does, directly to AppError.
- **`KeyMissing` IS plugin-constructed.** Plugin-init (e.g., Groq-plugin) tries `keystore.get(slot)`, gets `None`, needs to signal upstream. PluginError-trait-boundary is right layer.
- **`UpstreamUnavailable` stays plugin-constructed** (renamed from `Unavailable`). No new PluginError-variant — just rename.

```rust
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PluginError {
    #[error("network: {0}")]
    Network(String),
    #[error("auth: {0}")]
    Auth(String),
    #[error("rate limited (retry after {retry_after_ms}ms)")]
    RateLimit { retry_after_ms: u64 },
    #[error("fatal: {0}")]
    Fatal(String),
    #[error("upstream unavailable: {0}")]
    UpstreamUnavailable(String),   // renamed from Unavailable
    #[error("key missing for plugin: {plugin_id}")]
    KeyMissing { plugin_id: String },   // new
}
```

**4. `From<PluginError> for AppError`-Extension.**

Extend match with rename + new arm:

```rust
PluginError::UpstreamUnavailable(msg) => AppError {
    kind: AppErrorKind::UpstreamUnavailable,
    message: msg,
    user_message: None,
    retryable: true,
},
PluginError::KeyMissing { plugin_id } => AppError {
    kind: AppErrorKind::KeyMissing,
    message: format!("key missing for plugin: {plugin_id}"),
    // KeyMissing maps 1:1 to universal i18n-key; other arms require
    // plugin-specific context at call-site (policy-divergence
    // documented in ADR-0010 SubDec-4).
    user_message: Some("error.keystore.key_missing".into()),
    retryable: false,
},
```

**`user_message: Some(...)` in KeyMissing-Arm** ist Policy-Divergenz zu anderen Arms: KeyMissing maps 1:1 zu bekanntem i18n-Key (registered per Epic-1C-convention `error.keystore.<reason>`, ref `project_keystore_trait_surface`), daher direkt im From-Impl gesetzt statt Post-Mapping-Mutation an Call-Sites. Andere Arms bleiben `user_message: None` weil Plugin-specific context an Construction-Site nötig ist.

**5. Retryable-Mapping: typical-default, nicht enforced.**

`retryable` ist per-Instance-Construction-Flag. Die folgende Table dokumentiert **typical defaults** für die Variants — Construction-Sites dürfen abweichen wenn Kontext es rechtfertigt.

| AppErrorKind | Typical retryable | Reasoning |
|--------------|-------------------|-----------|
| `Network` | `true` | transient TCP/DNS issues self-heal |
| `Auth` | `false` | credential-rotation required |
| `Validation` | `false` | client-input-fix needed |
| `RateLimit` | `true` | automatic after retry_after_ms |
| `Internal` | `false` | programmer bug, user-retry won't help |
| `UpstreamUnavailable` | `true` | NFR11-Retry-via-new-Hotkey (FR29) |
| `PermissionDenied` | `false` | OS-level user-action-required |
| `PipelineValidation` | `false` | manifest-fix + app-restart required |
| `KeyMissing` | `false` | user must set key via KeyStore-CLI / Settings-UI |

Rustdoc-lines per variant (Sub-Decision 2) reflect diese Defaults; ADR-Body ist authoritative Referenz. Arch-doc §Error-Shape :659-665 Mapping-Table mirrored die PluginError→AppError-relevanten-Zeilen (nur Plugin-surfaced-Variants).

## Alternatives Considered

**(B) Collapse-to-`user_message`-Discriminator.** FR23/28/29/30-Wording würde unter generic `Unavailable`/`Validation`/`Auth` mit Frontend-String-Matching auf `user_message.startsWith("error.permission.*")` gemapped.
Rejected: (i) PRD-FR-Wording nennt variant-names direkt → Collapse wäre silent-PRD-rewrite. (ii) Frontend-Switch auf `err.kind === 'permissionDenied'` ist cleaner als String-Prefix-Matching (arch-doc :652-Pattern). (iii) Scales schlecht: jede neue FR-spezifische-Fehlerklasse würde user_message-Key-Space überladen. (iv) Verliert typed-compile-time-guarantees in Rust-Match-Arms.

**(C) Two-level `kind` + `subkind: Option<String>`.** AppErrorKind bleibt 6-enum; `subkind` Stringly-typed für Narrowing.
Rejected: Stringly-typed Discriminators in Rust sind Anti-Pattern — exakt das Problem, das arch-doc :649 mit „flat String auf Wire, NICHT tagged Enum" vermeidet (aber im Rust-Code ist typed-Enum der natural-fit). Außerdem inkonsistent: Wire-shape wäre `{"kind":"unavailable","subkind":"upstream"}` statt `{"kind":"upstreamUnavailable"}` — die zweite Form ist sparsamer und Frontend-Switch-friendly.

**(D) Keep `Unavailable` as-is, add `UpstreamUnavailable` parallel.** Backward-compat durch zusätzliche Variant statt Rename.
Rejected: Artifizielle Semantic-Overlap — die einzige Use-Site von `Unavailable` ist bereits Upstream-Context. „Generic Unavailable" existiert im Code nicht, wäre speculative-future-use. Beide parallel führt zu Developer-Konfusion „wann welche?". Reviewer-Grep bestätigt Safe-Rename.

## Consequences

**Positiv:**
- **PRD-FR-Code-Alignment:** FR23/28/29/30-Wording matcht jetzt 1:1 zu Code-Identifiers. Keine Naming-Divergenz-Debt-Accumulation bei späteren Impl-Stories.
- **Frontend-Switch-Ergonomie:** Typed-Flat-Kind-String auf Wire bleibt (arch-doc :649). Frontend-Pattern-Match auf `err.kind === 'permissionDenied'` etc.
- **Typed-Rust-Match:** Plugin-Impls können `Result<T, PluginError>` mit ?-Operator chainen, Core-Code kann AppErrorKind pattern-matchen typed.
- **Asymmetrische-PluginError-Extension** respektiert trait-surface-Scope: Plugin-Boundary braucht nur was Plugins construct.
- **`#[non_exhaustive]` bereits gesetzt** — Extension ist by-contract-non-breaking, keine semver-major nötig.

**Negativ / akzeptierte Schulden:**
- **Arch-Doc-Amendment erforderlich** (§Error-Shape :639-646 enum + :659-665 mapping-table + :823 example-snippet). Committed als separater Companion-Commit `docs(architecture): extend error-kind variants per ADR-0010`.
- **Epic-1-Code-Touch:** `klarvo-core/src/error.rs:16+37+75-80` (PluginError + AppErrorKind + Match-Arm). Minimal-invasive, kein Epic-1-Re-Spec. Code-Change-Story: ADR-0010-Implementation kann als Epic-2-Pre-Flight-Code-Commit passieren, nicht als Epic-1-Amendment.
- **KeyMissing-Asymmetrie bei user_message-Default** (Sub-Decision 4) ist Policy-Inkonsistenz gegenüber anderen From-Arms. Dokumentiert + begründet; Revisit falls weitere From-Arms KeyMissing-analoge 1:1-key-mapping-pattern zeigen.
- **specta::Type-Derive-Drift-Fix bundled:** ADR-0010-Code-Commit bundelt arch-doc↔code-Drift-Fix — `specta::Type`-derive wird auf `AppErrorKind` hinzugefügt (arch-doc §Error-Shape :636-638 mandatiert es bereits, Code hatte's nicht). Minimal-invasiv, notwendig für ADR-0009-Error-Bridge-Wire-Up (typed tauri-specta-Event-Payload).

**Epic-2-Story-Impacts:**
- **Story 2.3 (Groq-Plugin KeyStore-Real-Wire-Up):** Nutzt neue `PluginError::KeyMissing { plugin_id }` bei Keystore-Miss. From-Impl setzt user_message automatisch.
- **Story 2.6 (FR29 Groq-Failure-Recovery + Retry-Surface):** Nutzt neue `AppErrorKind::UpstreamUnavailable` in Retry-Logic-Branch; Shell pattern-matcht `kind === 'upstreamUnavailable'` + `retryable === true`.

**Epic-3-Story-Impacts:**
- **Story 3.3 (Tray-Icon + MIC-Permission, FR23):** Shell construct `AppError { kind: PermissionDenied, user_message: Some("error.permission.microphone"), retryable: false, ... }` bei cpal-device-enumeration-fail (Windows mic-permission-revoked). Surfaced via ADR-0009-Error-Bridge.

**Phase-1-Later-Epic-Impacts:**
- **Epic 1B Manifest-Parser (bereits closed):** Emittiert `AppError { kind: PipelineValidation, ... }` bei Boot-time-strict-parse-fail. Code-Touch minimal: existing `ManifestError`-to-AppError-Mapping nutzt neue Variant.

**Forward-References Phase 2+:**
- Phase-3 Android-Shell: `PermissionDenied` auch für AccessibilityService-Permission-Revoke (per ADR-0008 Amendment 1 Resolution Q2 forward-ref).
- Phase-2+ eigene Detail-Felder (z.B. `permission_scope: String` für PermissionDenied, `missing_plugin_id: String` für KeyMissing) wenn empirisch motiviert — analog `retryAfterMs` arch :653. `#[non_exhaustive]` preserviert das.

## Cross-References

- `output/planning-artifacts/architecture.md` §Error-Shape :622-667, Good-Example :820-828
- `output/planning-artifacts/prd.md` FR23, FR28, FR29, FR30, NFR11
- `klarvo-core/src/error.rs` (current-state: 6-variant AppErrorKind, Unavailable as single-use-site)
- `memory/feedback_architecture_doc_authoritative` (architecture-doc gets amendment, not silent-PRD-rewrite)
- `memory/feedback_manifest_compile_contract` (PipelineValidation-surface origin)
- `memory/project_keystore_trait_surface` (KeyMissing + Epic-1C `error.keystore.<reason>`-naming convention)
- `memory/feedback_commit_hygiene` (commit-pair: ADR + arch-doc-edit als separate commits)

## Next Actions

1. Write `docs/adr/0010-app-error-kind-extension.md` as per this draft.
2. Edit `output/planning-artifacts/architecture.md` per arch-doc-diff (separate commit).
3. **Implementation-Commit** (code-touch in `klarvo-core/src/error.rs` für Rename + 3 Adds + `specta::Type`-derive) ist Epic-2-Pre-Flight separate story/commit, **NICHT** in diesem ADR-Session-Scope. Wer das fährt, entscheidet Andy post-ADR-Closure.
