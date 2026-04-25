---
name: Story 5.4 — `xtask verify-release` Hardening (FR35)
epic: 5
story_number: "5.4"
status: draft
dependencies:
  - "phase-0-complete"
---

# Story 5.4: `xtask verify-release` Hardening

## Outcome

`cargo xtask verify-release` wird um zwei zusätzliche Release-Hardening-Invarianten
erweitert: (1) `dev-plain-keystore`-Feature-Off-in-Release-Lint (Security-Theater-Gate)
und (2) `aarch64-linux-android` Cross-Compile-Check für `klarvo-core` und
pure-Rust-Plugins. Die Phase-0-Baseline (forbidden-features + tracing-subscriber-sentinel)
bleibt vollständig erhalten. Ein Forcing-Sentinel-Fixture (Unit-Test mit künstlicher
Metadata-Fixture) sichert den `dev-plain-keystore`-Check mechanisch ab; der Cross-Compile-Check
failt-loud bei fehlendem Target mit actionable Message. Play-Store-spezifisches Hardening
(AccessibilityService-Manifest-Audit) ist explizit Phase-3-Scope, NICHT 5.4.

## Acceptance Criteria

### AC-A — Phase-0-Behavior bleibt erhalten

**Given** `xtask/src/verify_release.rs` implementiert in Phase-0 die Checks
`check_forbidden_features` (forbidden: `test-license`, `dev-*`-Prefix) sowie
`check_tracing_subscriber_sentinel`
**When** Story 5.4 die Erweiterungen einbaut
**Then**

- Beide Phase-0-Checks bleiben im Code erhalten und werden weiterhin aufgerufen
- Die bestehenden Unit-Tests in `verify_release.rs::tests` (10+ Tests, z. B.
  `is_forbidden_feature_matches_spec`, `forbidden_features_flags_test_license`,
  `tracing_subscriber_sentinel_present_fails_with_guidance`) bleiben alle grün
- Das Verhalten von `cargo xtask verify-release` ohne Verletzungen ist identisch zum
  Phase-0-Stand: OK-Ausgabe mit Package- und Node-Count
- `cargo test -p xtask` bleibt grün nach allen Änderungen in dieser Story

### AC-B — `dev-plain-keystore`-Feature-Off-in-Release-Lint

**Given** `dev-plain-keystore` ist ein Cargo-Feature in `klarvo-keystore` (Memory
`project_keystore_trait_surface.md`), das für Dev-Convenience Plain-SQLite-Storage
aktiviert; in einem Production-Binary führt es zu Security-Theater (Memory
`project_api_key_os_keystore_mvp.md`)
**When** `cargo xtask verify-release` ausgeführt wird
**Then**

- Der bestehende `check_forbidden_features`-Check in Phase-0 deckt `dev-plain-keystore`
  bereits ab (Pattern: `name.starts_with("dev-")`); Story 5.4 **verifiziert** diese
  Coverage und **dokumentiert** sie explizit in einem rustdoc-Kommentar in
  `verify_release.rs` als geprüfte Invariante
- Zusätzlich wird ein dedizierter Unit-Test `dev_plain_keystore_is_caught_by_forbidden_check`
  als Forcing-Sentinel-Fixture eingeführt:
  ```rust
  #[test]
  fn dev_plain_keystore_is_caught_by_forbidden_check() {
      // Sentinel: explizit für Security-Theater-Gate aus Story 5.4.
      // Wenn dev-plain-keystore als Cargo-Feature jemals umbenannt wird,
      // MUSS dieser Test angepasst werden + Memory project_keystore_trait_surface.md
      // aktualisiert werden.
      let m = fixture(
          &[],
          vec![node("klarvo-keystore 0.0.1", &["default", "dev-plain-keystore"])],
      );
      let v = check_forbidden_features(&m);
      assert_eq!(v.len(), 1);
      assert!(v[0].contains("dev-plain-keystore"));
      assert!(v[0].contains("klarvo-keystore"));
  }
  ```
- Das Fixture verwendet eine Metadata-Struktur mit `dev-plain-keystore`-Feature aktiv;
  der Test failt, wenn der Check dieses Feature durchlässt
- Ein TODO-Kommentar im Quellcode verweist auf die Deferred-Item-Erweiterung:
  Tauri-Bundle-Detection (Profile-spezifische Feature-Auflösung via
  `cargo metadata --features`/`--no-default-features`) ist Phase-2-Backlog — aktuell
  wird Default-Feature-Resolution geprüft

### AC-C — `aarch64-linux-android` Cross-Compile-Check

**Given** Phase-1-MVP-Scope ist Win + Android parallel (Memory
`project_klarvo_v2_rebuild.md`); CI-Integration für Cross-Compile wurde explizit in
Epic 5 deferred; `klarvo-core` ist pure-Rust ohne NDK-Abhängigkeiten
**When** `cargo xtask verify-release` ausgeführt wird
**Then**

- `verify_release.rs` führt einen neuen Check `check_android_cross_compile` aus, der
  `cargo check --target aarch64-linux-android -p klarvo-core` als Subprocess startet
- Scope der geprüften Crates: `klarvo-core` + alle `klarvo-plugin-*`-Crates, die keine
  OS-spezifischen Native-Dependencies haben (Heuristik: kein `build.rs`, kein
  `links`-Field in Cargo.toml); Plugin-Author-Verantwortung für NDK-spezifische Plugins
  wird separat geregelt
- Wenn das Target `aarch64-linux-android` nicht installiert ist, failt der Subcommand
  mit Exit-Code 1 und einer actionable Message:
  ```
  xtask verify-release: FAIL — aarch64-linux-android target not installed.
    Run: rustup target add aarch64-linux-android
    Note: pass --skip-cross-compile to skip this check in local-dev environments.
  ```
- Ein `--skip-cross-compile`-Flag wird am CLI-Parser von `verify-release` akzeptiert
  (in `main.rs` als Argument weitergereicht); wenn gesetzt, wird der Check übersprungen
  mit einer Warn-Ausgabe:
  ```
  xtask verify-release: WARNING — cross-compile check skipped via --skip-cross-compile.
    This flag must NOT be set in CI release pipelines.
  ```
- CI-Release-Pipelines übergeben `--skip-cross-compile` **nicht**; lokale Dev-Workflows
  dürfen es bei fehlendem NDK-Setup nutzen
- NDK-Installation ist **nicht** xtask-Verantwortung; der Check prüft nur, ob
  `rustup target list --installed` das Target enthält, bevor der
  `cargo check`-Subprocess gestartet wird (fail-fast mit klarer Message vor dem
  potentiell langsamen Cargo-Invocation)
- Play-Store-spezifisches Hardening (AccessibilityService-Manifest-Audit) ist
  **Phase-3-Scope** (Memory `project_play_store_phase3_blocker.md`), nicht 5.4;
  ein TODO-Kommentar im Code verweist darauf

### AC-D — CI-Integration: Fail-Loud + Forcing-Sentinel

**Given** CI-Gate-Philosophy mandatiert Preventive Enforcement + kein Skip-by-Default
(Memory `feedback_ci_gate_philosophy.md`)
**When** ein Release-Build in CI läuft
**Then**

- `cargo xtask verify-release` gibt Exit-Code 1 bei jeder Verletzung (bestehend aus
  Phase-0 + den neuen 5.4-Checks), Exit-Code 0 nur wenn alle Checks grün
- Alle Violations werden in einer aggregierten Fehlerliste ausgegeben (bestehende
  Aggregations-Logik aus Phase-0 bleibt erhalten — alle Checks laufen durch, keine
  Early-Exit bei erster Verletzung)
- Der Forcing-Sentinel aus AC-B (`dev_plain_keystore_is_caught_by_forbidden_check`)
  ist ein Unit-Test im xtask-Crate; er scheitert, wenn der Check versehentlich
  entfernt oder generalisiert wird
- Für den Cross-Compile-Check (AC-C) existiert kein analoger Unit-Test-Sentinel (da
  er echte `cargo check`-Invocations benötigt); stattdessen enthält die Rustdoc-Section
  in AC-E den Hinweis, dass der Cross-Compile-Check in CI mit installierten Targets
  verifiziert werden muss
- `cargo test -p xtask` deckt alle Unit-Tests ab; Cross-Compile-Failures sind
  Integration-Tests in CI, nicht lokale Unit-Tests

### AC-E — Documentation: Rustdoc-Invarianten-Comment-Block

**Given** `verify_release.rs` hat in Phase-0 einen rustdoc-Kommentar-Block, der die
implementierten Checks auflistet (Zeilen 1–33 in Phase-0-Baseline)
**When** Story 5.4 die neuen Checks einführt
**Then**

- Der rustdoc-Kommentar-Block in `verify_release.rs` wird aktualisiert und listet
  **alle** enforced Invarianten in einer nummerierten Liste:
  1. Forbidden Cargo features (Phase-0: `test-license`, `dev-*`) — inklusive
     explizitem Verweis auf `dev-plain-keystore` als Security-Theater-Gate (5.4)
  2. `tracing-subscriber`-Sentinel (Phase-0)
  3. `dev-plain-keystore`-Feature-Off-in-Release — explizite Nennung mit Source-Ref
     `project_keystore_trait_surface.md` + `project_api_key_os_keystore_mvp.md` (5.4)
  4. `aarch64-linux-android` Cross-Compile-Check für `klarvo-core` + pure-Rust-Plugins
     — mit Verweis auf `project_klarvo_v2_rebuild.md` Phase-1-Scope (5.4)
- Deferred-TODO-Block bleibt erhalten und wird um Phase-3-Item erweitert:
  - `TODO(phase-3): AccessibilityService-Manifest-Audit für Play Store.
    Prerequisite: Policy-Audit abgeschlossen. Spec: memory project_play_store_phase3_blocker.md.`
- Das `--skip-cross-compile`-Flag ist im Kommentar-Block dokumentiert mit explizitem
  Hinweis „MUST NOT be set in CI release pipelines"

## Technical Notes

### Phase-0-Check-Coverage für `dev-plain-keystore`

Der Phase-0-`check_forbidden_features`-Check deckt `dev-plain-keystore` bereits ab
(`name.starts_with("dev-")` — verifiziert durch existierenden Test
`is_forbidden_feature_matches_spec`). Story 5.4 fügt **keinen separaten Check** hinzu,
sondern einen Forcing-Sentinel-Test und explizite Dokumentation. Diese Unterscheidung
ist wichtig: der Implementierungs-Aufwand liegt hauptsächlich in AC-C (neuer
Cross-Compile-Check), AC-D (Sentinel-Test) und AC-E (Rustdoc-Update).

### `cargo metadata`-Parse vs. Cargo.toml-Text-Grep

Detection via `cargo metadata` (bestehende Infrastruktur in `load_metadata()`) ist
robust gegen Workspace-Reorg und Rename-Refactors. Text-Grep auf Cargo.toml-Dateien
ist brittle. Die bestehende `check_forbidden_features`-Funktion nutzt bereits
Cargo-Metadata — 5.4 erweitert diese Infrastruktur, nicht ersetzt sie.

### `--skip-cross-compile`-Flag-Design

`main.rs` muss den Flag durchreichen: `verify_release::run()` wird auf
`verify_release::run(skip_cross_compile: bool)` oder ähnliches angepasst. Alternativ:
`verify_release::run()` liest den Flag selbst aus `std::env::args()`. Delegate-Choice,
aber die Flag-Semantik (CI setzt ihn nie, lokale Dev darf) muss in der Rustdoc und im
Warn-Output klar sein.

### Cross-Compile-Scope: `klarvo-core` + pure-Rust-Plugins

Für Phase-1 ist `klarvo-core` das primäre Target. `klarvo-plugin-groq` ist ebenfalls
pure-Rust (reqwest + rustls, per ADR-0005). Die Liste der zu prüfenden Crates kann als
Konstante im Check geführt werden:
```rust
const ANDROID_CHECK_CRATES: &[&str] = &[
    "klarvo-core",
    "klarvo-plugin-groq",
    // Erweiterung: neue pure-Rust-Plugins hier eintragen
];
```
Plugin-Author-Verantwortung für NDK-spezifische Plugins (z. B. zukünftige Audio-HAL-Plugins)
wird separat geregelt — 5.4 deckt nur Core + Default-Plugins aus Phase-1.

### Target-Installations-Prüfung vor `cargo check`

`rustup target list --installed` als Vorprüfung verhindert, dass ein langsamer
`cargo check`-Subprocess mit kryptischer Fehlermeldung startet. Die Vorprüfung
gibt eine actionable Message und failt sofort (kein Silent-Fail, kein Skip-by-Default —
CI-Gate-Philosophy).

### Tauri-Bundle-Detection ist Phase-2-Backlog

Die Definition von „Release" für die Feature-Detection ist aktuell Default-Resolution
von `cargo metadata`. Tauri-Bundle aktiviert intern spezifische Feature-Flags; eine
Profile-spezifische Prüfung (`--profile release` vs. Development-Profile) ist sinnvoll
aber außerhalb Phase-1-Scope. Backlog-Eintrag wird in `docs/backlog.md` eingefügt.

## Dependencies

- `project_phase0_complete.md` — Phase-0-Baseline mit `verify_release.rs`, alle 5 Gates
  grün; Code-Reuse der bestehenden `load_metadata`/`check_forbidden_features`-Infrastruktur
- `project_keystore_trait_surface.md` — Feature-Gate `dev-plain-keystore` Source-of-Truth;
  Feature-Name muss mit Forcing-Sentinel-Test synchron bleiben
- `project_api_key_os_keystore_mvp.md` — Security-Theater-Begründung für
  `dev-plain-keystore`-Off-in-Release
- `project_klarvo_v2_rebuild.md` — Win+Android-Phase-1-Scope; Begründung für
  `aarch64-linux-android`-Cross-Compile als Release-Invariante
- `feedback_ci_gate_philosophy.md` — Preventive Enforcement + Fail-Loud + kein
  Skip-by-Default (außer explizit mit `--skip-cross-compile` dokumentiert)
- `project_play_store_phase3_blocker.md` — Forward-Reference; AccessibilityService-Policy
  ist Phase-3, NICHT 5.4-Scope
- Keine Inter-Story-Deps zu 5.1/5.2/5.3

## Tasks/Subtasks

- [ ] Task 1 — Phase-0-Behavior-Verifikation (AC-A)
  - [ ] 1.1 `cargo test -p xtask` lokal ausführen, alle bestehenden Tests bestätigen grün
  - [ ] 1.2 Prüfen dass `check_forbidden_features` + `check_tracing_subscriber_sentinel`
        unverändert im Call-Stack von `run()` bleiben

- [ ] Task 2 — Forcing-Sentinel-Test für `dev-plain-keystore` (AC-B)
  - [ ] 2.1 Unit-Test `dev_plain_keystore_is_caught_by_forbidden_check` in
        `verify_release.rs::tests` hinzufügen
  - [ ] 2.2 TODO-Kommentar für Tauri-Bundle-Detection (Phase-2-Backlog) in `run()` einfügen

- [ ] Task 3 — Cross-Compile-Check `aarch64-linux-android` (AC-C)
  - [ ] 3.1 `check_android_cross_compile(skip: bool) -> Vec<String>` implementieren
        (Target-Prüfung via `rustup target list --installed` als Vorprüfung)
  - [ ] 3.2 `cargo check --target aarch64-linux-android -p klarvo-core` (und weitere
        Crates aus `ANDROID_CHECK_CRATES`) als Subprocess ausführen
  - [ ] 3.3 Fail-Loud mit actionable Message wenn Target fehlt (ohne `--skip-cross-compile`)
  - [ ] 3.4 Warn-Output wenn `--skip-cross-compile` gesetzt
  - [ ] 3.5 `ANDROID_CHECK_CRATES`-Konstante mit `klarvo-core` + `klarvo-plugin-groq`
        definieren; Comment-Block für Plugin-Author-Erweiterungsanleitung

- [ ] Task 4 — CLI-Flag `--skip-cross-compile` (AC-C)
  - [ ] 4.1 `main.rs`-Routing anpassen: `--skip-cross-compile`-Flag parsen und an
        `verify_release::run(...)` weitergeben (Signatur-Änderung oder Env-Var-Alternative)
  - [ ] 4.2 `print_help()` in `main.rs` aktualisieren mit Flag-Dokumentation

- [ ] Task 5 — Aggregations-Logik sicherstellen (AC-D)
  - [ ] 5.1 `check_android_cross_compile`-Ergebnis in `failures`-Vec in `run()` integrieren
        (kein Early-Exit, alle Checks laufen durch)
  - [ ] 5.2 `cargo test -p xtask` nach allen Änderungen bestätigen grün

- [ ] Task 6 — Rustdoc-Update (AC-E)
  - [ ] 6.1 `//!`-Kommentar-Block in `verify_release.rs` mit allen 4 Invarianten
        aktualisieren
  - [ ] 6.2 Deferred-TODO-Block um Phase-3-Item
        `AccessibilityService-Manifest-Audit` erweitern
  - [ ] 6.3 `--skip-cross-compile`-Semantik in Kommentar dokumentieren

- [ ] Task 7 — Backlog-Eintrag (Technical Notes — Tauri-Bundle-Detection)
  - [ ] 7.1 `docs/backlog.md` Eintrag: „Story 5.4 — Tauri-Bundle-Profile-spezifische
        Feature-Detection als Erweiterung von `verify-release` (aktuell: Default-Resolution);
        Phase-2-Backlog"

## Dev Agent Record

### Completion Notes

<!-- Wird von Dev-Agent ausgefüllt -->

### Story-Spec-Abweichung

<!-- Wird von Dev-Agent ausgefüllt -->

## File List

<!-- Wird von Dev-Agent ausgefüllt -->

## Change Log

<!-- Wird von Dev-Agent ausgefüllt -->
