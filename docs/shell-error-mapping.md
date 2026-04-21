# Shell-Error-Mapping — AppErrorKind → i18n-Key → UI-Treatment

**Status:** Reference
**Date:** 2026-04-21

## Zweck

Konkretisiert den Mapping-Kontrakt zwischen `klarvo_core::error::AppError.kind` (ref ADR-0010), dem i18n-Key in `AppError.user_message` (Core emittiert Keys, Shell resolved — ref `memory/project_i18n_core_contract`), und der UI-Treatment-Klasse, die die Windows-Shell für jede Kind-Klasse anwendet. Schließt die Spec-Lücke aus ADR-0009 (der das Hybrid-C-Pattern definiert, aber keine konkrete Kind→Treatment-Tabelle enthält).

Diese Tabelle ist Reference-Doc, kein ADR-Amendment — weil Tabellen-Inhalt zu invasiv für ADR-Integration wäre und weil die Mapping-Matrix mit Epic-3-Impl-Stories iterativ additive Einträge bekommt (neue i18n-Keys per Story).

## Scope

- **In Scope:** Phase-1-Windows-Shell, AppErrorKind post-ADR-0010 (9 Varianten), i18n-Key-Präfixe, UI-Treatment-Legend.
- **Out of Scope:** Android-Shell-Äquivalent (Phase 3 — `strings.xml`-Resolve, Kotlin-View-Layer-Treatments), Pipeline-Boot-Validation-Error-UX-Dimension (ADR-0009 SD-4 — Splash vs. Native-Dialog vs. Degraded-Tray bleibt Story-3.1-Pre-Flight), Toast/Modal/Banner-Stil-Tokens (Design-System, Phase 2+).

## UI-Treatment-Legend

- **Toast** — Non-blocking Notification (3–5 s Auto-Dismiss), bei automatisch-recoverbaren oder User-Retry-freundlichen Fehlern. Frontend-Stack: Tauri-WebView-Notification-Komponente (Library-Choice P1-ADR).
- **Modal** — Blockiert Workflow, Dismiss nur durch explizite User-Action. Bei nicht-automatisch-recoverbaren Fehlern, die User-Intervention brauchen (Key falsch, Config korrupt).
- **Banner** — Persistenter UI-Banner (z. B. im Hauptfenster-Kopfbereich), reflektiert degradierten Zustand über mehrere Sessions hinweg. Bei längerfristig degraded State (Offline, Rate-Limit länger als transient).
- **Tray** — Tray-Icon-State-Change (FR20-Tray) + optional Tray-Tooltip. Bei stillen Hintergrund-Fehlern, die User nicht sofort unterbrechen müssen.
- **Log-Only** — Kein UI-Treatment; nur in den Rolling-File-Log (user-triggered Debug-Export-Zip per `memory/project_no_remote_telemetry`). Bei Internal-/Programmer-Errors, die keinen User-actionable Content haben.

**Kombination erlaubt:** Ein Error kann z. B. `Tray + Toast` triggern (Recording-Tray-Icon wechselt + Toast informiert). Die Tabellen-Spalte listet die *primäre* Treatment-Klasse; Impl-Story kann additiv kombinieren.

## Mapping-Tabelle

Source für `AppErrorKind`-Varianten: `klarvo-core/src/error.rs` (post-ADR-0010, 9 Unit-Variants).

| Kind | i18n-Key-Präfix | Primary UI-Treatment | retryable-Default | Notes |
|------|-----------------|----------------------|--------------------|-------|
| `Network` | `error.network.*` | Toast | `true` | Auto-Retry falls transient. Beispiel-Keys: `error.network.offline`, `error.network.timeout`, `error.network.dns_fail`. FR29-Context: bei `UpstreamUnavailable`-Spezialisierung dorthin, nicht hier. |
| `Auth` | `error.auth.*` | Modal | `false` | User muss Credential korrigieren. Typisch Groq/OpenAI-401/403. Modal leitet zum Settings-Panel (Phase 2) bzw. in Phase 1 zur `config.toml`-Hinweis-Section. |
| `Validation` | `error.validation.*` | Toast | `false` | Client-Input-Fix notwendig (z. B. Hotkey-String invalid). Dauer-kurz, non-blocking. Distinkt von `PipelineValidation` (Boot-Time). |
| `RateLimit` | `error.rate_limit.*` | Banner | `true` | Banner zeigt Retry-Countdown (wenn `retryAfterMs` verfügbar, Phase-2+-Feld). Toast kann additiv für ersten Hit gezeigt werden. |
| `Internal` | `error.internal.*` | Log-Only | `false` | Programmer-Error. User kann nichts tun; Kein UI-Noise. Log-Trace ist Dev-Audience via Debug-Export-Zip. Ausnahme: wenn fatal für Session → additiv Toast `error.internal.session_aborted`. |
| `UpstreamUnavailable` | `error.upstream.*` | Toast | `true` | FR29-Context (Groq-5xx, Timeout, Connection-Reset). User-Prompt „retry via new Hotkey-Press" (NFR11). Bei wiederholten Hits → Banner-Eskalation (Story-Scope). |
| `Configuration` | `error.config.*` | Modal | `false` | OS-Config-Fail (z. B. Output-Target-ID in Shell-Config unbekannt). User muss `config.toml` editieren + Restart. |
| `Io` | `error.io.*` | Toast | `false` | OS-IO-Fail (Clipboard-Set, File-Access, KeyStore-IO). Ephemer; User-Retry via neuen Hotkey. |
| `PermissionDenied` | `error.permission.*` | Modal | `false` | FR23-Context (MIC-Permission, Accessibility-Service). Modal muss User zu OS-Settings leiten (Phase-1 Phase 1: statische Instruktion; Phase 2+: Deep-Link-Button). Beispiel-Key: `error.permission.microphone`. |
| `PipelineValidation` | `error.pipeline.*` | Modal | `false` | Boot-Time Manifest-Strict-Error (FR28, ref `feedback_manifest_compile_contract`). SD-4-Resolution (ADR-0009) entscheidet Modal vs. Splash-Pre-Reg vs. Native-Dialog — aktueller Soft-Recommendation (c) Degraded-Tray mit Post-hoc-Emit + Modal-Open-on-Tray-Click. Beispiel-Key: `error.pipeline.unknown_stage`. |
| `KeyMissing` | `error.keystore.*` | Modal | `false` | User muss Key via `cargo xtask set-key` (Phase 2+) oder `config.toml`-Flow (Phase 1) setzen. From-Impl in `klarvo-core::error` setzt `user_message: Some("error.keystore.key_missing")` automatisch (ADR-0010 SD-4). Plugin-Identifier steckt im `AppError.message`-Feld (technisch, Log-Audience). |

## Cross-Referenzen

- ADR-0009: Shell-Error-Bridge-Pattern (Hybrid-C, `app.error`-Event, ErrorEmitter-Trait-Scope) — dieser Doc konkretisiert das Treatment-Konzept, das ADR-0009 offenläßt.
- ADR-0010: AppErrorKind-Extension — authoritativer Enum-Variant-Katalog.
- `output/planning-artifacts/architecture.md` §Error-Shape :622-675 — Wire-Format + Serde-Kontrakt.
- `memory/project_i18n_core_contract` — Core emittiert Keys, Shell resolved.
- `memory/project_no_remote_telemetry` — Log-Only-Treatment lebt im Rolling-File + Debug-Export-Zip.

## Evolution-Policy

- Neue i18n-Keys pro Epic-3-Impl-Story werden via Story-AC unter dem passenden Präfix registriert (additive Extension der *-Spalte, keine Matrix-Row-Restrukturierung).
- Neue `AppErrorKind`-Varianten (via `#[non_exhaustive]`-Extension, ADR-0010-Pattern): Matrix-Row wird in demselben Commit hinzugefügt wie der Enum-Variant-Add (Contract-before-Implementation, ref `memory/feedback_commit_hygiene`).
- Treatment-Class-Wechsel für existierende Kind (z. B. `RateLimit` → Modal statt Banner): braucht Begründung in Story-Description oder Mini-ADR, keine stille Matrix-Revision.
