---
name: Story 3.9 — Windows-Keystore Boot + Readiness-Check
epic: 3
story_number: "3.9"
status: Draft
dependencies:
  - "3.1"
  - "3.2"
---

# Story 3.9: Windows-Keystore Boot + Readiness-Check

## Outcome

`shells/windows/src-tauri/src/keystore.rs` liefert (a) eine Factory `make_keystore()`
die `WindowsKeystore` (`klarvo_core::keystore::os::WindowsKeystore`) bzw.
`PlainSqliteKeyStore` (Feature-Gate) wrapped, und (b) einen Boot-Check
`verify_keystore_ready(keystore: &dyn KeyStore) -> Result<(), AppError>` der
OS-Credential-Manager-Accessibility prüft. Per-Plugin-Key-Lookup (Epic-2/1C-Scope)
geschieht lazily im `PluginRegistry` — diese Story scopet nur Shell-Infrastructure.
Zwei neue i18n-Keys und drei Unit-Tests (ohne echten Windows-Credential-Manager) sind enthalten.

## Acceptance Criteria

### AC-A — Factory-Function mit Feature-Gate

**Given** `klarvo-core` hat `keystore::os::WindowsKeystore` (Release-Default) und
`keystore::PlainSqliteKeyStore` (Feature-Gate `dev-plain-keystore`)  
**When** `make_keystore()` implementiert wird  
**Then**

- Factory-Shape mit Feature-Gate:
  ```rust
  #[cfg(feature = "dev-plain-keystore")]
  pub fn make_keystore() -> Arc<dyn KeyStore> {
      Arc::new(klarvo_core::keystore::PlainSqliteKeyStore::open_or_create(
          &default_keystore_path(),
      ).expect("PlainSqliteKeyStore init failed in dev mode"))
  }

  #[cfg(not(feature = "dev-plain-keystore"))]
  pub fn make_keystore() -> Arc<dyn KeyStore> {
      Arc::new(klarvo_core::keystore::os::WindowsKeystore::new("klarvo"))
  }
  ```
  Delegate verifiziert `PlainSqliteKeyStore`-Constructor-Signatur aus
  `klarvo-core/src/keystore/plain_sqlite.rs` und passt ggf. an
- `"klarvo"` ist der `app_id`-Namespace für `WindowsKeystore` (alle Keys werden als
  `"klarvo/<key>"` im Credential-Manager gespeichert)
- Rustdoc expliziert Feature-Gate-Branch:
  ```
  /// Returns `PlainSqliteKeyStore` when `dev-plain-keystore` feature is active
  /// (dev/test builds). Returns `WindowsKeystore` otherwise (release default).
  /// Phase-4-Release-Default-Swap semantics: see klarvo-core/src/keystore/mod.rs.
  ```

### AC-B — Boot-Readiness-Check

**Given** `make_keystore()` eine `Arc<dyn KeyStore>`-Instanz geliefert hat  
**When** `verify_keystore_ready(keystore: &dyn KeyStore) -> Result<(), AppError>` aufgerufen
wird  
**Then**

- Die Funktion führt einen Probe-Lookup durch:
  ```rust
  pub async fn verify_keystore_ready(keystore: &dyn KeyStore) -> Result<(), AppError> {
      match keystore.get("klarvo_bootstrap_probe").await {
          Ok(_) => Ok(()),  // Probe key exists somehow — keystore is definitely functional
          Err(e) if e.user_message.as_deref() == Some(klarvo_core::keystore::keys::KEY_NOT_FOUND) => {
              Ok(())  // Expected: probe key absent → keystore accessible but key not stored
          }
          Err(_) => Err(AppError {
              kind: AppErrorKind::Io,
              message: "keystore boot-readiness probe failed".to_string(),
              user_message: Some("error.keystore.read_failed".to_string()),
              retryable: false,
          }),
      }
  }
  ```
- `"klarvo_bootstrap_probe"` ist ein reservierter Key-String, der nie als echter API-Key
  registriert wird. Rustdoc dokumentiert diese Invariante
- `KEY_NOT_FOUND` = `"error.keystore.not_found"` (aus `klarvo_core::keystore::keys`);
  dieser Wert zeigt an: Keystore ist funktional, Key existiert nicht — das ist der
  erwartete Happy-Path des Probes

### AC-C — Missing-Per-Plugin-Key-Flow (Scope-Fence)

**Given** Story 3.9 prüft nur Boot-Readiness  
**When** ein Plugin beim Init einen fehlenden API-Key findet  
**Then**

- Per-Plugin-Key-Lookup ist **nicht** Story-3.9-Scope
- Plugin-Init (Epic-2/1C-Committed) führt `keystore.get("<plugin_key>").await` durch und
  returniert `AppError { kind: KeyMissing, user_message: Some("error.keystore.not_found") }`
  wenn der Key fehlt
- Story 3.9 stellt nur sicher, dass der Keystore beim Bootstrap funktional ist;
  der `KeyMissing`-Path bei per-Plugin-Lookup existiert bereits im `PluginRegistry`-Init-Code
- Rustdoc auf `verify_keystore_ready` expliziert die Scope-Fence:
  `// Checks infrastructure readiness only. Per-plugin key presence is Plugin-Init-scope.`

### AC-D — Integration-Point (Forward-Reference)

**Given** `make_keystore()` und `verify_keystore_ready()` sind implementiert  
**When** Rustdoc auf beiden Funktionen steht  
**Then**

- `make_keystore` Rustdoc enthält:
  ```
  /// Story 3.10 (Bootstrap-Integration) calls:
  ///   let keystore = make_keystore();
  ///   if let Err(e) = verify_keystore_ready(keystore.as_ref()).await {
  ///       error_emitter.emit_error(&e.user_message.unwrap_or_default(), clock.now_ms()).await;
  ///   }
  ///   app.manage(keystore);
  ```
- Forward-Reference auf Story 3.10 als einziger Nutzer dieser Factory

### AC-E — Error-Treatment-Mapping

**Given** `verify_keystore_ready` einen Fehler detektiert  
**When** der Fehler propagiert wird  
**Then**

- Keystore-Boot-Failure → `AppErrorKind::Io` → `error.keystore.read_failed` →
  **Toast** (per `docs/shell-error-mapping.md` `Io`-Kind-Mapping)
- Rationale (Technical Notes): ephemer; User-Remedy ist App-Restart. Toast statt Modal
  weil kein User-actionable Konfigurationsschritt nötig ist
- Per-Plugin-`KeyMissing` (fehlender API-Key) → `AppErrorKind::KeyMissing` → Modal:
  das ist Epic-2/1C-Committed-Code, nicht Story-3.9-Scope
- `docs/shell-error-mapping.md` `Io`-Kind-Row ist authoritative Quelle für
  Toast-Treatment-Entscheidung

### AC-F — Unit-Tests

**Given** `keystore.rs` ist implementiert  
**When** `cargo test -p <windows-shell-crate>` (oder analog) ausgeführt wird  
**Then**

- **Test 1 — Factory-returns-boxed-trait-object (Compile-Check):**
  ```rust
  #[test]
  fn make_keystore_returns_arc_dyn_keystore() {
      let _ks: Arc<dyn KeyStore> = make_keystore();
      // Compile-check: return type is Arc<dyn KeyStore>-compatible.
      // Runtime behavior is feature-dependent (dev vs. release).
  }
  ```
- **Test 2 — verify_keystore_ready happy-path (InMemoryKeyStore):**
  ```rust
  #[tokio::test]
  async fn verify_keystore_ready_happy_path_probe_key_absent() {
      let ks = klarvo_test_fixtures::InMemoryKeyStore::empty();
      let result = verify_keystore_ready(&ks).await;
      assert!(result.is_ok(), "empty InMemoryKeyStore should be 'ready'");
  }
  ```
  `InMemoryKeyStore.get("klarvo_bootstrap_probe")` returniert `Err` mit `KEY_NOT_FOUND`
  → `verify_keystore_ready` gibt `Ok(())`
- **Test 3 — verify_keystore_ready IO-failure (FailingKeyStore):**
  ```rust
  #[tokio::test]
  async fn verify_keystore_ready_io_failure_maps_to_apperror_io() {
      let ks = klarvo_test_fixtures::FailingKeyStore::with_error(AppError {
          kind: AppErrorKind::KeyMissing,
          message: "backend unreachable".to_string(),
          user_message: Some(klarvo_core::keystore::keys::BACKEND_UNAVAILABLE.to_string()),
          retryable: false,
      });
      let err = verify_keystore_ready(&ks).await.unwrap_err();
      assert!(matches!(err.kind, AppErrorKind::Io));
      assert_eq!(err.user_message.as_deref(), Some("error.keystore.read_failed"));
  }
  ```
  Falls `FailingKeyStore` noch nicht in `klarvo-test-fixtures` existiert: Delegate
  erweitert `klarvo-test-fixtures/src/keystore.rs` mit:
  ```rust
  pub struct FailingKeyStore { error: AppError }
  impl FailingKeyStore {
      pub fn with_error(error: AppError) -> Self { Self { error } }
  }
  #[async_trait]
  impl KeyStore for FailingKeyStore {
      async fn get(&self, _key: &str) -> Result<SecretString, AppError> { Err(self.error.clone()) }
      async fn set(&self, _k: &str, _v: SecretString) -> Result<(), AppError> { Err(self.error.clone()) }
      async fn delete(&self, _key: &str) -> Result<(), AppError> { Err(self.error.clone()) }
  }
  ```
  `AppError` muss `Clone` ableiten (prüfen; falls nicht: Delegate-Choice zwischen Clone-Add
  und `Arc<AppError>`-Wrapping im FailingKeyStore)
- Alle Tests nutzen `#[tokio::test]` (async) und laufen ohne echten Windows-Credential-Manager

### AC-G — i18n-Keys

**Given** Story 3.1 AC-D hat `locales/en.json` + `locales/de.json` angelegt  
**When** diese Story committed wird  
**Then**

- `locales/en.json` bekommt mindestens:
  ```json
  {
    "error.keystore.read_failed": "Secure key storage is unavailable. Please restart the application."
  }
  ```
- `locales/de.json` bekommt den gleichen Key; Übersetzung Delegate-Choice oder
  TODO-Marker-Pattern analog Story 3.2 AC-G
- **Verifizierung existierender Keys:** `error.keystore.not_found` und
  `error.keystore.backend_unavailable` sind in `klarvo_core::keystore::keys` als Rust-Konstanten
  definiert, aber möglicherweise noch nicht in `locales/en.json`/`locales/de.json` eingetragen.
  Delegate prüft + trägt fehlende Keys nach. Wichtig: der Schlüssel `error.keystore.key_missing`
  existiert **nicht** im Core — authoritativ ist `error.keystore.not_found` (Konstante `KEY_NOT_FOUND`
  in `klarvo_core::keystore::keys`).
- Beide Locale-Files bleiben valides JSON nach der Ergänzung

## Technical Notes

### Feature-Gate-Model

Per `memory/project_keystore_trait_surface` + `klarvo-core/src/keystore/mod.rs`:

- `#[cfg(feature = "dev-plain-keystore")]` → `PlainSqliteKeyStore` (Security-Theater,
  nur für Dev; explizites Feature-Flag verhindert versehentliche Release-Nutzung)
- `#[cfg(not(feature = "dev-plain-keystore"))]` → `WindowsKeystore::new("klarvo")` (Release)
- Phase-4-Release-Default-Swap-Semantik ist bereits im Core-Keystore-Modul dokumentiert

### verify_keystore_ready: Funktionaler Probe, nicht Sensitivity-Probe

`"klarvo_bootstrap_probe"` ist kein echter API-Key-Identifier. Der Probe nutzt einen
reservierten String der nie als echter Key registriert wird. `KEY_NOT_FOUND`-Return
(`"error.keystore.not_found"`) bedeutet: Keystore ist erreichbar und hat einen validen
Query-Path verarbeitet — Readiness ist bestätigt. Nur Backend-Accessibility-Fehler
(`BACKEND_UNAVAILABLE` oder andere) indizieren echten Failure.

### Warum Toast (nicht Modal) für Keystore-IO-Fail beim Boot

Aus `docs/shell-error-mapping.md` `Io`-Kind → Toast: Keystore-Boot-Fail ist ephemer
(OS-Credential-Manager-Service temporär nicht erreichbar, Race bei App-Start). App-Restart
löst das Problem in der Regel. Modal wäre für nicht-automatisch-recoverable Config-Issues
(z.B. falscher API-Key, korruptes Config-File) — Keystore-IO-Fail fällt nicht in diese Klasse.
Per-Plugin-`KeyMissing` (Nutzer hat API-Key nicht eingetragen) → Modal ist Epic-1C/2-Scope.

### AppError Clone-Anforderung für FailingKeyStore

Der Test-3-Fixture `FailingKeyStore` speichert eine `AppError`-Instanz und returned Clone
bei jedem `get`/`set`/`delete`. Prüfen ob `AppError` `#[derive(Clone)]` hat. Falls nicht,
gibt es zwei Optionen: (a) Clone zu `AppError` in `klarvo-core/src/error.rs` hinzufügen
(additive Change, akzeptabler Scope für Test-Ergonomie), (b) `FailingKeyStore` speichert
eine Factory-Closure statt `AppError`-Clone.

### Forward-Reference auf Phase-2 Auto-Recovery

Settings-UI (Phase 2+) könnte einen Re-Init-Button anbieten der `make_keystore()` nochmals
aufruft und `verify_keystore_ready` ausführt ohne App-Restart. Aktuell Phase-1: App-Restart
ist der einzige Recover-Path.

## Dependencies

- Story 3.1 — Tauri-Skeleton (Crate-Setup, `locales/`-Files)
- Story 3.2 — `ShellConfig` (Pfad-Resolution für `PlainSqliteKeyStore` ggf. nötig)
- `klarvo-core/src/keystore/os/windows.rs` — `WindowsKeystore`-Impl (authoritative)
- `klarvo-core/src/keystore/mod.rs` — Feature-Gate-Model (`dev-plain-keystore`)
- `klarvo-core/src/keystore/keys.rs` — `KEY_NOT_FOUND`, `BACKEND_UNAVAILABLE` Konstanten
- `klarvo-test-fixtures/src/keystore.rs` — `InMemoryKeyStore` (Test 2); `FailingKeyStore` (Test 3, ggf. neu)
- ADR-0012 §SD-4 — Error-Path-Integration (Orchestrator nutzt `ErrorEmitter` für Keystore-Boot-Fail)
- `docs/shell-error-mapping.md` — `Io`-Kind → Toast-Treatment
- `memory/project_keystore_trait_surface` — Epic-1C-Scope (Feature-Gate, Trait-Surface)
