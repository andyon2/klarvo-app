# ADR-0009: Shell-Error-Bridge-Pattern (Async-Error-Propagation Core → Frontend)

**Status:** Proposed
**Date:** 2026-04-20

## Context

Klarvo v2 Phase-1-Hybrid-C-Error-Surfacing (Vor-Check-Pre-Decision) teilt Error-Propagation in zwei Pfade:

- **Sync Path (unchanged):** Tauri-Commands returnen `Result<T, AppError>`. Command-Rejection liefert den Error direkt an den JS-`invoke`-Caller. Arch-doc §Error-Shape :622-669 ist authoritative (post-ADR-0010-Target-State: 9-Variant `AppErrorKind`, `specta::Type`-derive, i18n-Key in `user_message`). Kein ADR nötig.

- **Async Path (dieser ADR):** Errors, die **außerhalb** einer Command-Invocation emergieren, brauchen einen eigenen Bridge-Mechanismus. Primäre Phase-1-Emit-Sites:
  - `CpalAudioSource`-Callback-Context (OS-Audio-Thread, cpal-stream-error mid-capture → kann nicht via `Result` propagieren; siehe ADR-0006 `CaptureHandle`-RAII).
  - Shell-Orchestrator nach `run_pipeline`/`output.deliver`/`keystore.get`-Result-Fail innerhalb des 7-Step-Hotkey-Cycles (ref `project_shell_session_lifecycle`; Frontend hat den Hotkey nicht via `invoke` ausgelöst, hat aber `listen` subskribiert).
  - Pipeline-Boot-Validation-Fail beim App-Init (FR28 `PipelineValidation`-Variant; erscheint vor jeder Command-Interaktion — Sonderfall, siehe Open-Question).

**Architecture-Conformance (NICHT Deviation):** Architecture-doc §Communication Patterns :683-686 mandatiert bereits das Split-Prinzip — **Channels für High-Frequency-Data, Events für State-Changes**. Errors sind diskrete State-Transitions (nicht kontinuierliche Datenströme), daher ist `tauri-specta::Event` die architecture-konforme Wahl. Dieser ADR dokumentiert *wie* der Event-Bridge konkret aufgebaut wird, nicht ob Events eingesetzt werden.

**Forward-Reference ADR-0010:** Event-Payload ist `AppError` im post-ADR-0010-Target-State (9-Variant `AppErrorKind` inkl. `PermissionDenied`/`PipelineValidation`/`KeyMissing`/`UpstreamUnavailable`; `specta::Type`-derive auf `AppError` UND `AppErrorKind`). Der ADR-0010-Code-Commit ist Pre-Flight für Bridge-Implementation.

**Scope-Fence:** Dieser ADR spezifiziert Bridge-**Pattern** (Event-Name, Trait-Shape, Resolve-Layer). NICHT Scope: Code-Implementation (Story-3.2+), Test-Infrastructure für Event-Emit, Frontend-Toast/Dialog-UI-Shape, Logging/Telemetrie-Layer (getrennt — siehe `project_no_remote_telemetry`), Android-Shell-Variante (Phase 3).

## Decision

**Hybrid-C mit vier Sub-Decisions.** Sync Command-Errors bleiben unverändert Result-basiert; Async-Errors fließen via ein einziges consolidated `tauri-specta::Event`. Core stellt eine narrow-scoped `ErrorEmitter`-Trait-Abstraktion für OS-Thread-Callback-Sites zur Verfügung; Shell-Orchestrator emittiert Result-Chain-Errors direkt.

### Sub-Decision 1: Single Consolidated `app.error`-Event

Ein einziger `tauri-specta::Event` transportiert alle Async-Error-Fälle. Payload = `AppError` (post-ADR-0010-Shape).

```rust
// shells/windows/src-tauri/src/error_bridge.rs  (Shell-Scope —
// tauri-specta-Dep darf NICHT in klarvo-core, weil Core portable
// für Phase-3-Android-Shell bleiben muss)

#[derive(Clone, Debug, Serialize, specta::Type, tauri_specta::Event)]
#[tauri_specta(event_name = "app.error")]
pub struct AppErrorEvent(pub AppError);
```

Rationale:
- **ADR-0010 SubDec-2 hat Unit-Variants gewählt** und strukturierte Per-Case-Zusatzfelder (`retry_attempts`, `skipped_samples`, `target_plugin_id`) explizit Phase-2+ deferred. Daraus folgt direkt: Phase-1 braucht keinen Per-Domain-Event-Split — es gibt keinen typed-Payload-Vorteil auszuschöpfen.
- **Event-Namenskonvention:** `app.error` matcht die `app.ready`-Präzedenz aus `reference_tauri_specta_rc24_event_name`. Dot-Notation ist G1-Lint-konform (ref ADR-0002 Amendment 1). Domain-Grouping-Info steckt im `AppError.user_message`-i18n-Key (z.B. `"error.stt.upstream_5xx"`, `"error.audio.dropout"`) — Frontend-Switch-Logik über `err.kind` + optional i18n-Key-Prefix-Dispatch.
- **Core-Side-Simplicity:** Ein Event-Emit-Site pro Error-Quelle; keine 4+ `tauri_specta::Event`-Deklarationen zu pflegen.
- **Wrapper-Struct (`AppErrorEvent(AppError)`)** statt direkt `AppError` als Event-Typ: gibt `tauri-specta` einen dedizierten Newtype-Identifier für Event-Binding-Codegen, hält `AppError` als reine Data-Type semantisch sauber (wird auch als Command-Result-Typ verwendet). Bindungs-Impl-Detail — Story-3.2 kann auch `impl tauri_specta::Event for AppError` direkt wählen wenn das in rc.24 stabil ist.
- **Scope-Asymmetrie (load-bearing):** `ErrorEmitter`-Trait lebt in `klarvo-core` (tauri-agnostisch, siehe SD-3). Der `AppErrorEvent`-Wrapper + `TauriErrorEmitter`-Impl leben in `shells/windows/src-tauri/` (Shell-Scope, tauri-specta-Dep). Diese Asymmetrie ist intentional: Core bleibt portable für Phase-3-Android-Shell, Shell stellt Tauri-spezifische Bindings.

### Sub-Decision 2: i18n-Resolve im Frontend (WebView-Layer)

`AppError.user_message: Option<String>` trägt den i18n-Key (z.B. `"error.permission.microphone"`, `"error.keystore.key_missing"`). Die **Frontend-WebView** resolved Keys zu lokalisierten Strings über den bestehenden JS-i18n-Stack. Rust-Shell nimmt **keine** Übersetzungs-Rolle wahr.

Rationale:
- **project_i18n_core_contract ist Core/Shell-Separation**, nicht Rust-Shell/WebView-Separation. Der Kontrakt sagt „Shells übersetzen" — Rust-Shell-side UND WebView-side sind beide „Shell". Wo genau innerhalb der Shell, bleibt pragmatische Implementation-Choice.
- **Frontend benötigt i18n-Infrastruktur ohnehin** für Non-Error-UI (Settings-Panel, Onboarding, Hotkey-Conflict-Messages, Pill-Bar-Labels). Error-Keys sind marginaler Zusatz, nutzen dieselben Locale-Assets.
- **Rust-Shell-side-i18n** wäre Duplikation des Resolve-Layers (fluent/rust-i18n zusätzlich zu i18next/o.ä. im Frontend) ohne kompensierenden Nutzen.
- **Phase-3-Android-Konsistenz:** Kotlin-Shell nutzt Android-native `strings.xml`-Resolve; Core emittiert Keys, Kotlin-View-Layer resolved. Gleiches Prinzip wie Windows-WebView — Shell-Native-Layer resolved, nicht Rust-Shell-Intermediary.

**Deferred Sub-Question (NICHT in diesem ADR):** Konkrete JS-i18n-Library-Choice (i18next vs. formatjs vs. leichtgewichtige Direct-Import-Lösung). Das ist ein separates Phase-1-i18n-ADR — die Bridge-Pattern-Decision ist davon unabhängig, weil der Wire-Vertrag nur „`user_message` ist Key-String" festlegt.

### Sub-Decision 3: `ErrorEmitter`-Trait in `klarvo-core` für OS-Thread-Callback-Sites

**Narrow-scoped Trait** in `klarvo-core`, injiziert als `Arc<dyn ErrorEmitter>` dort, wo Async-Errors NICHT über Result-Chain propagieren können.

```rust
// klarvo-core/src/error_emitter.rs  (empfohlen, separates Modul neben error.rs)

use crate::error::AppError;

pub trait ErrorEmitter: Send + Sync {
    /// Emit an async error to the Shell-side bridge.
    ///
    /// Sync signature (non-async, returns `()`): emit-failure is a Shell-internal
    /// telemetry concern handled inside the implementor (e.g., `tracing::error!`),
    /// not a caller concern. Implementors MUST NOT block the caller thread —
    /// relevant because this trait is called from OS-audio-callback contexts
    /// where blocking would risk sample drops (NFR2).
    fn emit(&self, error: AppError);
}
```

**Phase-1-Primary-Consumer ist ausschließlich die `CpalAudioSource`-Capture-Callback-Context** (cpal-OS-Thread kann Fehler nicht via Result propagieren). Alle anderen Phase-1-Error-Paths — Command-Handler, Pipeline-Executor (`run_pipeline`), `OutputTarget::deliver`, `KeyStore`-Lookup, `AudioSource::start` — nutzen Result-Chains und emittieren im Shell-Orchestrator-Loop direkt via `AppHandle::emit_all` (kein Trait-Detour, keine Trait-Dependency im Core-Executor-Constructor).

Die Trait-Abstraktion existiert, weil **Inversion-of-Control für Async-Callback-Sites notwendig** ist — der cpal-Callback läuft auf dem OS-Audio-Thread (non-tokio, non-Tauri-aware, ref `project_shell_runtime_model`), kann den `AppHandle` nicht direkt halten, braucht aber einen Typ-abstrakten Weg, Errors Shell-wärts zu pushen. NICHT als speculative Extensibility-Layer. Phase-2+-Konsumenten (z.B. Plugin-Background-Tasks mit Shell-Error-Surface) würden dem Trait natürlich folgen, aber dieser ADR stipuliert keine zukünftigen Consumer (premature-abstraction-guard per `feedback_premature_abstraction_guard`).

**Trait-Location:** `klarvo-core` — konsistent mit arch-doc §4 Error-Shape-Lokalisierung (AppError/PluginError im Core). `klarvo-audio-cpal` depends on `klarvo-core` (ADR-0006 Amendment 2), d.h. Trait-Surface ist accessible. Core hat keine Tauri-Deps; nur Shell-Impl wraps `AppHandle`. Empfehlung: eigene Modul-Datei `klarvo-core/src/error_emitter.rs` (trennt Data-Types von Behavior-Trait); ist Impl-Detail, nicht Decision-Block-Item.

**Shell-Impl-Skizze (Story-3.2-Scope, illustrativ):**

```rust
// shells/windows/src-tauri/src/error_bridge.rs
use klarvo_core::error::AppError;
use klarvo_core::error_emitter::ErrorEmitter;
use tauri::{AppHandle, Manager};

pub struct TauriErrorEmitter {
    app_handle: AppHandle,
}

impl ErrorEmitter for TauriErrorEmitter {
    fn emit(&self, error: AppError) {
        tracing::error!(kind = ?error.kind, message = %error.message, "async error");
        if let Err(e) = AppErrorEvent(error).emit(&self.app_handle) {
            tracing::error!(error = %e, "failed to emit app.error event to frontend");
        }
    }
}
```

### Sub-Decision 4: Pipeline-Boot-Validation-Error — IPC-Constraint dokumentieren, UX-Resolution zu Story-3.1-Pre-Flight deferren

Pipeline-Boot-Validation-Error (FR28, `AppErrorKind::PipelineValidation`) tritt beim App-Start auf, **bevor** `tauri-specta::Event`-Subscriber im Frontend aktiv sind. Die in Sub-Decision 1 gewählte Bridge (`app.error`-Event) allein löst das nicht — Fire-and-Forget eines Events vor Listener-Attachment verliert den Error silent.

**Drei UX-Resolution-Optionen** (Final-Resolution Story-3.1-Pre-Flight, NICHT hier):

- **(a) Pre-Event-Registration im Splash-Screen:** App bootet in einen Splash-UI-Zustand, Frontend registriert Listener, meldet Ready an Shell-Orchestrator, Core-Init läuft, bei Fail → Event-Emit an bereits-registrierten Listener.
- **(b) Native OS-Error-Dialog via Tauri-dialog-Plugin:** Boot-Fail → `tauri::dialog::MessageDialog` (synchron, native), App terminiert danach kontrolliert.
- **(c) Degraded-Mode mit Tray-Error-State + post-hoc Event-Emit:** App launched, Tray-Icon zeigt Error-State (FR20-Tray existiert bereits per Phase-1-Scope), Menu-Item „Show Error Details" öffnet Main-Window, Frontend-ready → Shell re-emits gepufferten Boot-Error via `app.error`.

**Soft-Recommendation (c)** — leveraged FR20-Tray-Infrastruktur, erlaubt partielle App-Funktion (Settings-Panel evtl. erreichbar, User kann Config fixen ohne Reinstall), konsistent mit NFR10-Wording „Plugin-Init-Failures zum Startup-Zeitpunkt werden vom Orchestrator als fatale Errors behandelt und führen zu kontrolliertem App-Beenden" NUR für Plugin-Init; PipelineValidation ist Manifest-Drift, konzeptuell recoverable durch User-Config-Fix.

**Story-3.1-Pre-Flight-Decision-Space** ist damit explizit gesetzt — Story plant nicht im luftleeren Raum.

## Alternatives Considered

### SD-1 Alternativen

**(B) Per-Domain-Events (`klarvo.stt.failed`, `klarvo.audio.dropout`, `klarvo.output.failed`, `klarvo.pipeline.validation_failed`).**

Fair-Argumentation: Bessere `tauri-specta`-Typing-Ergonomie — jedes Event hat domain-spezifischen Payload-Typ, Frontend-Listener sind narrow-typed. `listen<SttFailedEvent>('klarvo.stt.failed', ...)` mit Payload `{ error: AppError, retry_attempts: u32 }` ist ergonomischer als generic `listen<AppErrorEvent>`. Discoverability: „Welche Error-Events existieren?" → grep `#[tauri_specta::Event]` listet sie alle. Frontend kann domain-selektiv abonnieren.

Rejected: (i) Per-Case-Zusatzfelder sind per ADR-0010 SubDec-2 explizit Phase-2+ deferred — Phase-1-Payload ist in beiden Optionen `AppError`, d.h. typed-Vorteil existiert im aktuellen Scope noch nicht. (ii) 4+ Event-Deklarationen + Emit-Sites sind Maintenance-Overhead ohne Phase-1-Gegenwert. (iii) Phase-2+-Evolution-Pfad bleibt offen: Wenn Per-Case-Fields empirisch motiviert werden, können entweder `AppError` mit optionalen Feldern erweitert werden (Präzedenz: ADR-0010 `retry_after_ms`-Erwähnung) ODER Per-Domain-Events **neben** `app.error` eingeführt werden. Kein Rewrite.

**(C) Hybrid generic + selektive Per-Domain.**

Rejected: Zwei parallele Event-Patterns zu warten ist schlechter als eins. Ohne klares Kriterium („welche Errors Per-Domain, welche generic?") entsteht Drift-Vektor.

### SD-2 Alternativen

**(B) Rust-Shell-side Key-Resolve via `fluent`/`rust-i18n`, pre-translated String als zusätzliches `displayMessage`-Feld.**

Fair-Argumentation: Zentraler Resolve-Layer — sowohl tracing-Logs (Developer-Audience, englisch) als auch Frontend-Errors könnten in Rust-Shell resolved werden, einheitliches String-Handling. Tests könnten Translation-Keys mit Rust-Test-Harness verifizieren statt JS-Test-Harness.

Rejected: (i) Frontend hat i18n-Stack für Non-Error-UI ohnehin — Duplikation ist Fat-Rust-Shell-AntiPattern. (ii) Rust-Side-i18n-Libraries sind weniger mature als JS-i18n-Ecosystem (i18next/formatjs mit ICU-MessageFormat). (iii) Tracing-Logs bleiben englisch per `project_i18n_core_contract`, teilen keine Translation-Pipeline mit User-Messages.

**(C) Hybrid — Key im Event + optional pre-translated `displayMessage`.**

Rejected: Added-Complexity ohne klaren Konsumenten. Frontend resolved eh aus dem Key, `displayMessage` wäre dann nur Debug-Fallback-Stream — das gehört in tracing-Logs (Rust-Side), nicht in den Wire-Event.

### SD-3 Alternativen

**(B) Channel-to-Shell-Adapter (`mpsc::Sender<AppError>` im Core, Shell-consumer-Task).**

Fair-Argumentation: Kein Trait-Dep im Core; stdlib/tokio-primitives only. Explizite Backpressure-Kontrolle (bounded-channel-capacity). Entkoppelt Producer/Consumer noch deutlicher als Trait.

Rejected: (i) Channel-Capacity-Semantik für Errors ist awkward — bei capacity-exhaust entweder blockieren (schlecht im cpal-OS-Callback, NFR2-Risk) oder droppen (schlecht für Errors generell, jeder Error matters). (ii) Zusätzlicher tokio-spawn-Task auf Shell-Side nur um ein Event zu emittieren ist Overhead ohne Gegenwert. (iii) Direct-Method-Call (Trait) ist einfacher zu reason-aboutn als async-Channel-Fluss, besonders für seltene Error-Pfade.

**(C) `AudioEvent::Error(AppError)`-Variant auf existing broadcast-Channel (ADR-0006/0007-Pattern).**

Fair-Argumentation: Konsistenz mit existing Audio-Broadcast-Pattern. Keine neue Infrastruktur. Cleanes cpal-callback → AudioEvent::Error → Shell-consumer → emit.

Rejected: (i) Audio-spezifische Lösung, skaliert nicht für Phase-2+-Non-Audio-Async-Error-Sources. ADR-0009 positioniert als GENERAL Shell-Error-Bridge-Pattern, nicht Audio-Patch. (ii) `broadcast::channel` hat `Lagged(n)`-Lossy-Semantik bei slow-subscriber — für Audio-Samples akzeptabel (Backpressure-Policy per ADR-0007), für Errors **nicht** akzeptabel (jeder Error muss ankommen). (iii) Semantic-Mixing: Audio-Samples und Errors sind orthogonale State-Transitions; sie in einem Enum zu koppeln erhöht Kopplung zwischen Subsystemen.

### SD-4 Alternativen

**(a) Splash-Screen mit Pre-Event-Registration.**
Fair: Clean asynchrones Boot-Flow, keine verlorenen Events. Rejected-Risk: Zusätzliche Splash-UI-Komponente + Ready-Signal-Protokoll zwischen Frontend und Shell-Orchestrator — Phase-1-Komplexitätszuwachs.

**(b) Native OS-Error-Dialog + Panic.**
Fair: Simpelste Implementation, keine Frontend-Beteiligung nötig. Rejected-Risk: UX-jarring (native MessageBox passt nicht zum Klarvo-Tauri-Aesthetic); App kann nicht partiell recovern.

**(c) Degraded-Mode + Tray-Error-State + post-hoc Emit.** Soft-Recommendation (siehe Decision).

**Final-Resolution deferred** — alle drei Optionen sind architecture-konform; die UX-Dimensionen (Tray-Icon-Design, Splash-Screen-Scope, Native-Dialog-Styling) liegen außerhalb IPC-Boundary-Scope und gehören in Story-3.1-Pre-Flight.

## Consequences

**Positiv:**

- **Architecture-Conformance:** Event-Choice folgt arch §683-686 (Events für State-Changes, Channels für High-Frequency) — kein Deviation-ADR nötig, dieser ADR ist Pattern-Konkretisierung.
- **Hybrid-C Sync/Async-Split minimiert Complexity:** Der Großteil der Error-Pfade (Command-Handler, Pipeline-Executor-Result, Output-Deliver-Result, KeyStore-Lookup-Result) bleibt unverändert Result-basiert; Bridge-Infrastructure wird nur dort eingeführt, wo Result-Chain nicht funktioniert.
- **`ErrorEmitter`-Trait-Narrowing:** Justified-by-Necessity (cpal-OS-Thread-Inversion-of-Control), nicht speculative — respektiert `feedback_premature_abstraction_guard`.
- **Frontend-Switch-Ergonomie:** Ein Listener-Subscriber (`app.error`), Payload ist bekannte `AppError`-Shape — Shell-side-Toast/Dialog-UI-Layer kann generic bleiben.
- **i18n-Single-Resolve-Layer:** Keine Duplikation zwischen Rust-Shell und WebView; leveraged existing Frontend-i18n-Stack.
- **Phase-3-Android-Transferable:** Gleiches Pattern — Kotlin-Shell implementiert `ErrorEmitter` via JNI-Callback zum Android-UI-Layer; consolidated `app.error`-Event-Äquivalent ist Kotlin-Flow-Emission; i18n-Resolve im Kotlin-View-Layer via `strings.xml`. Kein Phase-3-Re-Design nötig.

**Negativ / akzeptierte Schulden:**

- **Neue Trait-Surface in `klarvo-core`** (`ErrorEmitter`) — minor-surface (ein `fn emit`), aber API-Commitment. `#[non_exhaustive]` auf AppError bleibt wichtig für Additions ohne Trait-Breaking.
- **`tauri-specta::Event`-Codegen-Dependency:** `AppErrorEvent`-Newtype erzeugt Binding in `shells/windows/src/bindings/`. G1-Lint muss `#[tauri_specta(event_name = "app.error")]` validieren (bereits Teil des G1-Scope per `reference_tauri_specta_rc24_event_name`).
- **Arch-Doc-Amendment erforderlich** — 1-2 Sätze in §IPC-Boundaries oder §Error-Shape-Ende, separater Companion-Commit analog ADR-0010-Pattern.
- **Boot-Error-UX bleibt offen** (SD-4 deferred) — akzeptabel, weil Story-3.1-Pre-Flight ohnehin Tray-Icon-State-Design macht; dieser ADR fixiert die IPC-Constraint damit Story nicht im luftleeren Raum plant.

**Epic-3-Story-Impacts:**

- **Story 3.1 (Tauri-Skeleton-Bootstrap + Tray-Icon):** Muss Pre-Flight die SD-4-Option ((a)/(b)/(c)) wählen. `TauriErrorEmitter`-Impl-Gerüst in `shells/windows/src-tauri/src/error_bridge.rs`. Tauri-managed-state hält den Emitter.
- **Story 3.2 (Error-Bridge-Implementation):** Direct-Scope dieses ADRs — `AppErrorEvent`-Newtype definieren, `AppErrorEvent.emit(&app_handle)` call-sites in Shell-Orchestrator, `TauriErrorEmitter`-Wiring in `AudioSource`-Constructor.
- **Story 3.3 (Tray-Icon + MIC-Permission, FR23):** Nutzt `AppErrorKind::PermissionDenied` (ADR-0010). Permission-Denied-Detection an cpal-device-enumeration → Construct `AppError { kind: PermissionDenied, user_message: Some("error.permission.microphone"), retryable: false, ... }` → Shell-Orchestrator emit.
- **Story 3.4 (Audio-Capture-Integration):** `CpalAudioSource`-Constructor nimmt `Arc<dyn ErrorEmitter>`; cpal-stream-error-Callback ruft `emitter.emit(AppError { kind: UpstreamUnavailable, user_message: Some("error.audio.stream_dropped"), retryable: true, ... })` oder analog.
- **Story 3.5 (Auto-Paste-Delivery, FR21):** Paste-Fail (Clipboard-Set oder `SendInput`-Fail) ist Shell-interner Error — Shell-Orchestrator emittiert direkt, braucht keinen `ErrorEmitter`-Detour.

**Phase-2+-Impacts:**

- Per-Case-strukturierte-Payload-Fields (FR29 `retry_attempts`, Audio-Dropout `skipped_samples`, etc.) können durch `AppError`-Extension (optionale Felder) ODER Per-Domain-Events-neben-`app.error` erweitert werden — beide Pfade offen, kein Rewrite.
- Plugin-Background-Tasks mit Shell-side-Error-Surface (z.B. Hintergrund-History-Sync) nutzen `ErrorEmitter` via Dependency-Injection — Trait ist bereit.

## Open Questions

- **SD-4-Final-Resolution (Boot-Error-UX):** (a) Splash-Pre-Reg vs (b) Native-Dialog vs (c) Degraded-Tray-Mode. Owner: Story-3.1-Pre-Flight. Soft-Recommendation (c).
- **AppErrorEvent vs direct-impl-Event:** `AppErrorEvent(AppError)`-Newtype oder `impl tauri_specta::Event for AppError` direkt? Impl-Detail, Story-3.2-Scope; abhängig von `tauri-specta`-rc.24-Binding-Codegen-Verhalten. Newtype-Pattern als Default wegen Separation-of-Concerns (AppError ist auch Command-Result-Typ).
- **JS-i18n-Library-Choice:** Separater Phase-1-i18n-ADR, orthogonal zu ADR-0009.
- **`Deserialize`-Derive auf `AppError`/`AppErrorEvent`:** Post-ADR-0010-Target-State deriviert `AppError` nur `Debug, Clone, Serialize, specta::Type` (kein `Deserialize`). `tauri-specta::Event` benötigt `Deserialize` möglicherweise nicht (Events sind emit-only Backend→Frontend; Deserialize wäre für Frontend→Backend-Reverse-Events). rc.24-API-Verifikation ist Story-3.2-Scope. Falls `Deserialize` für `tauri_specta::Event` required → minor Story-3.2-Impl-Commit ergänzt Derive auf `AppError` + `AppErrorKind` (beide Serde-Unit-Variant-kompatibel). **Nicht Blocker für ADR-0009-Decision.**

## Arch-Doc-Amendment

**Location:** `output/planning-artifacts/architecture.md` §Error-Shape-Ende (nach :669) oder §Communication Patterns (nach :686) — Delegate-Choice basierend auf Doc-Flow.

**Proposed Addition:**

> Async-Errors (emergent außerhalb von Command-Invocation-Contexten — z.B. cpal-Audio-Callback-Errors, Pipeline-Mid-Session-Errors, Pipeline-Boot-Validation-Errors) werden via dedicated `tauri_specta::Event` `app.error` mit `AppError`-Payload an das Frontend propagiert. Emit-Site ist der Shell-Orchestrator oder Core-`ErrorEmitter`-Trait-Impl (narrow-scoped für OS-Thread-Callback-Contexts wie `CpalAudioSource`). Ref ADR-0009.

**Companion-Commit:** `docs(architecture): document async-error-bridge per ADR-0009`.

## Cross-References

- `output/planning-artifacts/architecture.md` §3 IPC-Boundaries :251-258, §Event-Naming :432-452, §Error-Shape :622-674, §Communication Patterns :683-686
- `output/planning-artifacts/prd.md` FR19-23 (Windows-Shell), FR28-31 (Error-Handling), NFR10-11 (Reliability)
- `docs/adr/0002-tauri-specta-2-rc-acceptance.md` (Amendment 1: `#[tauri_specta(event_name = "...")]`-Syntax)
- `docs/adr/0006-audiosource-trait-signature.md` (CaptureHandle-RAII, cpal-OS-Thread)
- `docs/adr/0007-audio-buffer-backpressure-policy.md` (broadcast-Lossy-Semantik — Kontrast zu ADR-0009-Error-Lossless)
- `docs/adr/0008-shell-adapter-interface-shape.md` (Shell-Orchestrated-Post-Pipeline-Delivery)
- `docs/adr/0010-app-error-kind-extension.md` (AppError/PluginError Target-State, `specta::Type`-derive)
- `memory/project_shell_runtime_model` (Single-tokio + cpal-OS-Thread)
- `memory/project_shell_session_lifecycle` (7-Step-Hotkey-Cycle, Error-Emit-Sites)
- `memory/project_i18n_core_contract` (Core-Keys-only)
- `memory/project_no_remote_telemetry` (Local-Logs-only, kein Sentry)
- `memory/reference_tauri_specta_rc24_event_name` (Dot-Notation-G1-Lint)
- `memory/feedback_premature_abstraction_guard` (Trait-Narrow-Scope-Rationale)

## Next Actions

1. Write `docs/adr/0009-shell-error-bridge-pattern.md` (this file).
2. Separate companion-commit: arch-doc-amendment (§IPC-Boundaries oder §Error-Shape-Ende, 1-2 Sätze mit ADR-0009-Link).
3. Memory-file `project_shell_error_bridge_pattern.md` nach ADR-Closure (outside-commit).
4. Story-3.1-Pre-Flight: SD-4-Boot-Error-UX-Option wählen.
5. Story-3.2: Implementation des ADR-0009-Patterns.

---

## Amendment 1 — 2026-04-21: ErrorEmitter-Signature formalized

**Status:** Accepted

### Context

Sub-Decision 3 im Original-Decision-Block skizzierte `fn emit(&self, error: AppError)` als synchrone Trait-Signatur mit Full-AppError-Payload. Die Phase-1-Implementation in `klarvo-core/src/event/emitter.rs` (commit `213052d`, Story 1A.3) hat stattdessen:

```rust
#[async_trait]
pub trait ErrorEmitter: Send + Sync + 'static {
    async fn emit_error(&self, key: &str, ts_ms: u64);
}
```

Weitere Abweichung vom Entwurf: **Modul-Lokation.** Das Original-Decision-Dokument schlug `klarvo-core/src/error_emitter.rs` als eigenes Modul vor. Die Impl platzierte den Trait unter `klarvo-core/src/event/emitter.rs` (re-exported als `klarvo_core::event::ErrorEmitter`) — konsistent mit dem Event-Modul-Layout (ADR-0006/0007/0008-Pattern) und dem `klarvo_test_fixtures::MockErrorEmitter`-Gegenstück.

**Divergenz-Treiber (aus Impl-Erfahrung):**

- **`async` statt `sync`:** Frontend-Emit via `tauri::AppHandle::emit` ist `.await`-returning. Sync-Signatur würde `block_on` erzwingen → Deadlock-Risk in tokio-managed Runtime.
- **`key + ts_ms` statt `AppError`:** Core-Caller emittieren i18n-Keys (ref `memory/project_i18n_core_contract`); Full-`AppError`-Serialisierung wäre doppelter Payload (`message`-Field ist Log-Audience, nicht Frontend-Audience).
- **No Return-Value:** Advisory fire-and-forget per Rustdoc-Kontrakt — Emit-Failure darf Pipeline-Run nicht abbrechen.

### Amended Decision

Canonical Trait-Signatur (Phase 1 und vorwärts):

```rust
#[async_trait]
pub trait ErrorEmitter: Send + Sync + 'static {
    async fn emit_error(&self, key: &str, ts_ms: u64);
}
```

`app.error`-Frontend-Wire-Payload:

```json
{ "key": "error.<domain>.<variant>", "ts_ms": 12345 }
```

Frontend-i18n-Resolve erfolgt via Key-Lookup gegen die eigene i18n-Tabelle (per SD-2). `ts_ms` ist session-relative monotone Milliseconds vom Caller-Clock (ref ADR-0001/0003 Convention).

Die Original-Decision bleibt für die Hybrid-C-Architektur-Invariante gültig (sync-Results + async-`app.error`-Event); nur die Trait-Signatur wird konkretisiert.

**Modul-Lokation (Corrigendum):** Authoritativer Pfad: `klarvo-core/src/event/emitter.rs`. Import: `use klarvo_core::event::ErrorEmitter`. Original-Text referenzierte `klarvo-core/src/error_emitter.rs` — das ist nicht der Impl-Pfad.

### Impact

- `TauriErrorEmitter` (Story 3.8) implementiert diese Signatur; sein Event-Payload-Struct ist `AppErrorEventPayload { key: String, ts_ms: u64 }`.
- Core-Callsites (z.B. Orchestrator-Error-Paths in `on_release`) rufen `.emit_error(&e.user_message.unwrap_or_default(), clock.now_ms()).await`.
- Existing Rustdoc-Kontrakt auf `emit_error` (fire-and-forget, non-blocking) steht.
- `klarvo_test_fixtures::MockErrorEmitter` ist bereits konform: implementiert `async fn emit_error(&self, key: &str, ts_ms: u64)` und recorded `Vec<(String, u64)>`.

### Cross-References

- `klarvo-core/src/event/emitter.rs` — authoritative Trait-Definition (commit `213052d`)
- `klarvo-test-fixtures/src/event_bus_harness.rs` — `MockErrorEmitter`-Impl (konform)
- Story 3.3 AC-E — Orchestrator-Error-Emission-Sites
- Story 3.8 — `TauriErrorEmitter`-Impl
- `memory/project_i18n_core_contract` — Key-Only-Payload-Policy

---

## Amendment 2 — 2026-04-21: Primary-Consumer-Correction (Orchestrator, nicht CpalAudioSource)

**Status:** Accepted

### Context

Sub-Decision 3 im Original-Decision-Block (L81) behauptet:

> „Phase-1-Primary-Consumer ist ausschließlich die `CpalAudioSource`-Capture-Callback-Context"

und L198:

> „Story 3.4 (Audio-Capture-Integration): `CpalAudioSource`-Constructor nimmt `Arc<dyn ErrorEmitter>`"

Die Phase-1-Implementation in `klarvo-audio-cpal/src/source.rs` (commit `37b57c1`) ist jedoch ein Unit-Struct ohne ErrorEmitter-Injection:

```rust
pub struct CpalAudioSource;
```

Stream-Errors werden via `tracing::warn!` geloggt und triggern den Channel-Close-Flow (`*tx_slot.lock().unwrap() = None`), wodurch downstream `run_capture_session` einen `RecvError::Closed` empfängt und terminiert. Kein Cross-Thread-Error-Propagation via `ErrorEmitter` für cpal-Callback-Errors.

### Amended Decision

Phase-1-Primary-Consumer des `ErrorEmitter`-Traits ist der `SessionOrchestrator` (Story 3.3), nicht `CpalAudioSource`. Konkret:

- Orchestrator ruft `error_emitter.emit_error(...)` für:
  - `AudioSource::start`-Failure (Device unavailable, Config-Error)
  - `run_pipeline`-Result-Fail (STT-Error, Cleanup-Error)
  - `OutputTarget::deliver`-Fail
  - `PasteBackend::paste`-Fail
  - Output-Target-Registry-Lookup-Miss
- `CpalAudioSource` handhabt OS-Thread-Callback-Errors intern via `tracing::warn!` + Channel-Close. Kein ErrorEmitter-Detour; Phase-1-bewusste Design-Entscheidung aus der `klarvo-audio-cpal`-Impl-Phase.

### Rationale

- **Simplicity:** `CpalAudioSource` ohne DI-Dep hat minimale Konstruktor-Surface (Unit-Struct). Err-Surfacing via Channel-Close ist idempotent-sicher.
- **Downstream-Visibility:** Channel-Close propagiert natürlich als `RecvError::Closed` an `run_capture_session`, das seinerseits terminiert; der Orchestrator hält den Pipeline-Task-Handle und sieht die Termination.
- **Tracing-Sufficiency:** cpal-Stream-Errors sind Log-Audience-Ereignisse (Dev-Debug via Rolling-File + `project_no_remote_telemetry`). Kein User-facing Toast-Need für transient Stream-Drops (User merkt „keine Transkription" bereits).

### Phase-2+-Option (nicht aktiviert)

Wenn Phase-2 User-visible Stream-Error-UX verlangt (z. B. „Mikrofon-Verbindung verloren" Toast), kann `CpalAudioSource` um einen `Arc<dyn ErrorEmitter>`-DI-Field retrofit werden (additive Constructor-Change). Impl-Template würde dann in `Stream::build_input_stream` den Error-Callback um `emitter.emit_error("error.audio.stream_dropped", now).await` ergänzen. Das ist eine explizite Phase-2-Option, nicht Phase-1-Debt.

### Impact

- **Story 3.7 (CpalAudioSource-Wire-Up):** Factory-Signatur ist zero-arg `make_audio_source() -> Arc<Mutex<Box<dyn AudioSource>>>`. Kein `emitter`/`clock`-Parameter.
- **Story 3.3 (Orchestrator):** `ErrorEmitter`-Emit-Sites sind vollständig im Orchestrator (`on_press`/`on_release`-Flows + Pipeline-Task-Body).
- **Original L198 „Story 3.4":** zu lesen als „Story 3.7 — CpalAudioSource Wire-Up"; die konkrete Behauptung „`CpalAudioSource`-Constructor nimmt `Arc<dyn ErrorEmitter>`" ist supersedet durch diese Amendment.

### Cross-References

- `klarvo-audio-cpal/src/source.rs` — authoritative `CpalAudioSource`-Impl (commit `37b57c1`)
- Story 3.3 AC-E — Orchestrator-Error-Emission-Sites
- Story 3.7 Technical Notes §CpalAudioSource-Error-Model — Anchor dieser Amendment
- `memory/project_no_remote_telemetry` — tracing-Log-Audience-Policy
