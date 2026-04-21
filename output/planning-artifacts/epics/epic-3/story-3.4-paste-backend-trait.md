---
name: Story 3.4 — PasteBackend Core-Trait + MockPasteBackend
epic: 3
story_number: "3.4"
status: Draft
dependencies: []
---

# Story 3.4: PasteBackend Core-Trait + MockPasteBackend

## Outcome

Neues Core-Trait `PasteBackend` in `klarvo-core/src/output/paste.rs` mit Signatur
`async fn paste(&self) -> Result<(), AppError>`. `MockPasteBackend` in `klarvo-test-fixtures`
instrumentiert „paste called N times" und ist als Test-Double in `SessionOrchestrator`-Unit-Tests
einsetzbar (ADR-0012 §Sub-Decision 5 Testability-Contract).

## Acceptance Criteria

### AC-A — Trait-Shape in `klarvo-core`

**Given** `klarvo-core` enthält noch kein `PasteBackend`-Trait  
**When** `klarvo-core/src/output/paste.rs` angelegt wird  
**Then**

- Das Trait ist via `#[async_trait::async_trait]` deklariert mit exakter Signatur:
  ```rust
  #[async_trait]
  pub trait PasteBackend: Send + Sync {
      async fn paste(&self) -> Result<(), AppError>;
  }
  ```
- `Send + Sync` sind explizite Supertraits (nicht nur implizit über `async_trait`), sodass
  `Arc<dyn PasteBackend>` ohne zusätzliche Bounds möglich ist
- Das Trait hat **kein Text-Argument**: PasteBackend triggert ausschließlich die Paste-Aktion
  (typischerweise Ctrl+V / SendInput-Key-Injection). Das Clipboard-Setzen ist separate
  `OutputTarget`-Responsibility (Step 6 vs. Step 7 im 7-Step-Hotkey-Cycle per
  `project_shell_session_lifecycle`)
- Rustdoc auf dem Trait erklärt die Trennung von OutputTarget (clipboard-set) und PasteBackend
  (paste-trigger) mit einem Beispiel-Kommentar:
  `/// Step 6 (OutputTarget::deliver): sets clipboard content.`
  `/// Step 7 (PasteBackend::paste): triggers Ctrl+V injection into focused window.`

### AC-B — Modul-Placement + Public-Export

**Given** `klarvo-core/src/output/` existiert (von Epic-1/2 Stories)  
**When** das neue Modul hinzugefügt wird  
**Then**

- Trait lebt in `klarvo-core/src/output/paste.rs`
- `klarvo-core/src/output/mod.rs` re-exportiert: `pub mod paste; pub use paste::PasteBackend;`
- Öffentlicher Pfad ist `klarvo_core::output::PasteBackend` (verifiziert via Compile-Test
  in AC-F)
- Rustdoc auf dem Modul (`paste.rs`) enthält einen Second-Consumer-Rationale-Abschnitt:
  ```
  /// # Second-Consumer Rationale
  ///
  /// Primary consumer: `shells/windows/src-tauri/src/paste.rs::WinSendInputPasteBackend`
  /// (Story 3.5) — Win32 `SendInput` key-injection.
  ///
  /// Phase-3 consumer: `shells/android/.../AccessibilityPasteBackend` —
  /// Android AccessibilityService paste-action.
  ///
  /// Two concrete platform implementations justify introducing this abstraction per
  /// `feedback_premature_abstraction_guard` (second-consumer requirement).
  ```

### AC-C — Expected `AppError`-Kinds (Rustdoc-Forward-Reference)

**Given** die Trait-Signatur returniert `Result<(), AppError>`  
**When** Rustdoc auf `paste()` geschrieben wird  
**Then**

- Das Rustdoc auf `async fn paste` listet die erwarteten `AppErrorKind`-Varianten:
  ```
  /// # Error Variants
  ///
  /// Implementations are expected to return:
  /// - `AppErrorKind::Io` — Win32 SendInput failure or OS clipboard-access failure.
  /// - `AppErrorKind::PermissionDenied` — Android AccessibilityService denied or
  ///   not enabled (Phase-3).
  /// - `AppErrorKind::Configuration` — no target window focused or paste-target
  ///   not resolvable.
  ///
  /// i18n-key prefix: `error.paste.*` (per `docs/shell-error-mapping.md` Evolution-Policy).
  /// Concrete keys are registered by implementation stories (Story 3.5 for Windows,
  /// Phase-3 for Android) — not in this trait-definition story.
  ```
- Diese Story registriert **keine neuen i18n-Keys** in `klarvo-core` (nur Forward-Reference im
  Rustdoc). Implementation-Stories (3.5 / Phase-3) fügen konkrete Keys hinzu gemäß
  `docs/shell-error-mapping.md` Evolution-Policy

### AC-D — `MockPasteBackend` in `klarvo-test-fixtures`

**Given** `klarvo-test-fixtures` existiert mit anderen Mock-Typen (MockAudioSource, etc.)  
**When** `MockPasteBackend` hinzugefügt wird  
**Then**

- `klarvo-test-fixtures` hat eine neue Datei `src/paste.rs` (oder gleichwertiges Modul)
  mit `pub struct MockPasteBackend`
- Die Struct enthält:
  1. Einen internen `Arc<Mutex<Vec<()>>>` Call-History-Tracker (für thread-safe Call-Count)
  2. Einen konfigurierbaren Return-Wert: `fn with_result(result: Result<(), AppError>) -> Self`
     (Builder-Style); Default bei `MockPasteBackend::new()` ist `Ok(())`
- Query-API:
  - `pub fn call_count(&self) -> usize` — Anzahl der bisher registrierten `paste()`-Calls
  - `pub fn was_called(&self) -> bool` — äquivalent zu `call_count() > 0`
- `impl PasteBackend for MockPasteBackend`:
  ```rust
  async fn paste(&self) -> Result<(), AppError> {
      self.calls.lock().unwrap().push(());
      self.result.clone()
  }
  ```
  wobei `result` das konfigurierte Return-Value ist (via `with_result`)
- `MockPasteBackend` ist `pub use`-d aus `klarvo-test-fixtures` Root (oder aus
  `klarvo_test_fixtures::paste::MockPasteBackend`)
- `klarvo-test-fixtures/Cargo.toml` listet `klarvo-core` als Dep (für `PasteBackend`-Trait +
  `AppError`-Typ)

### AC-E — `MockPasteBackend` Unit-Tests

**Given** `MockPasteBackend` ist implementiert per AC-D  
**When** Unit-Tests in `klarvo-test-fixtures/tests/paste_mock.rs` ausgeführt werden  
**Then**

- **Test 1 — Default-happy-path:**
  ```
  Given: MockPasteBackend::new() (default Ok-return)
  When: paste() einmal aufgerufen
  Then: Rückgabe ist Ok(()); call_count() == 1; was_called() == true
  ```
- **Test 2 — Configured-to-fail:**
  ```
  Given: MockPasteBackend::new().with_result(Err(AppError { kind: AppErrorKind::Io, ... }))
  When: paste() aufgerufen
  Then: Rückgabe ist Err mit AppErrorKind::Io; call_count() == 1
  ```
- **Test 3 — Multiple calls:**
  ```
  Given: MockPasteBackend::new()
  When: paste() dreimal aufgerufen
  Then: call_count() == 3
  ```
- Alle Tests laufen ohne Tauri-App-Instance, ohne OS-Audio-Device, ohne echtes Clipboard —
  reiner headless Rust-Test

### AC-F — `klarvo-core` Trait-Object-Compat-Check

**Given** `PasteBackend` ist deklariert per AC-A, `MockPasteBackend` per AC-D  
**When** `cargo test -p klarvo-core` ausgeführt wird  
**Then**

- Ein Unit-Test in `klarvo-core/tests/paste_trait.rs` (oder inline im Modul) verifiziert
  Trait-Object-Kompatibilität:
  ```rust
  #[test]
  fn paste_backend_is_object_safe_and_arc_compatible() {
      // Verifies Send + Sync bounds and dyn-compatibility at compile time.
      let mock = klarvo_test_fixtures::MockPasteBackend::new();
      let _: Arc<dyn klarvo_core::output::PasteBackend> = Arc::new(mock);
  }
  ```
- Der Test kompiliert und ist grün (kein Runtime-Assert nötig — Compile-Zeit-Check reicht)
- `klarvo-core/Cargo.toml` listet `klarvo-test-fixtures` als `[dev-dependencies]` (nur für
  diesen Test), falls noch nicht vorhanden

## Technical Notes

### Design-Anchor: ADR-0012

`PasteBackend`-Trait-Shape kommt direkt aus ADR-0012 §Sub-Decision 2 (API-Surface Code-Sketch):
```rust
paste_backend: Arc<dyn PasteBackend>
```
und §Sub-Decision 5 (Testability-Contract: `MockPasteBackend` als Test-Double für
`SessionOrchestrator`-Unit-Tests).

### Async-Signatur-Rationale

`async fn paste` statt sync: Tauri's `async_runtime::spawn` dispatcht den Paste-Call
off-hot-path. `SendInput` ist theoretisch synchron-schnell, aber die Async-Signatur hält
`SessionOrchestrator`-Consumer-Interface einheitlich mit `AudioSource::start` (ebenfalls async,
ADR-0006). Zukünftige Implementations können reale async-Operationen enthalten (z.B.
Android-Accessibility-IPC).

### Second-Consumer-Validation

Per `feedback_premature_abstraction_guard`: factor-out nur bei proven Second-Consumer.
Die zwei konkreten Consumer sind:
1. `WinSendInputPasteBackend` (Story 3.5) — Windows Win32 SendInput
2. `AccessibilityPasteBackend` (Phase-3, `shells/android/`) — Android AccessibilityService

Beide sind im Phase-Plan explizit verankert. Die Trait-Einführung ist gerechtfertigt.

### `MockPasteBackend` Result-Typ

`result`-Field im Mock ist `Result<(), AppError>`. Da `AppError` kein `Copy` ist, muss das
Field beim `paste()`-Call geklont werden: `self.result.clone()`. `AppError` deriviert bereits
`Clone` in `klarvo-core/src/error.rs:21` (commit 178fdd8 ADR-0010 Impl-Commit) — keine
Prerequisite-Arbeit in Story 3.4.

### i18n-Key-Policy

Diese Story registriert keine neuen i18n-Keys — sie definiert nur den Key-Präfix im Rustdoc
(`error.paste.*`). Konkrete Keys wie `error.paste.send_input_failed` oder
`error.paste.no_target_window` werden von Story 3.5 (Windows-Impl) registriert. Konform mit
`project_i18n_core_contract`: Core emittiert Keys, Shells lösen auf.

## Dependencies

- Keine Story-Dependencies (Welle-1, dependency-free)
- ADR-0012 §Sub-Decision 2 — PasteBackend API-Surface
- ADR-0012 §Sub-Decision 5 — Testability-Contract (MockPasteBackend-Usage-Pattern)
- `feedback_premature_abstraction_guard` — Second-Consumer-Rationale für Trait-Einführung
- `project_shell_session_lifecycle` — Step 6 (OutputTarget) vs. Step 7 (PasteBackend) Trennung
- `docs/shell-error-mapping.md` — i18n-Key-Präfix-Konvention + Evolution-Policy
