---
name: Story 5.1 — `cargo xtask manifest-strict` Gate (FR32)
epic: 5
story_number: "5.1"
status: draft
dependencies: []
---

# Story 5.1: `cargo xtask manifest-strict` Gate

## Outcome

`cargo xtask manifest-strict` wird als production-grade Subcommand in `xtask/src/main.rs`
implementiert (ersetzt den Phase-0-Stub-Eintrag in der Help-Section). Der Subcommand konsumiert
`klarvo_core::manifest::parse_from_str` — den explizit nicht `#[cfg(test)]`-gegateten
Test-Injection-Entrypoint (Story 1B.2, `epics.md:637-643`) — und validiert eine kuratierte
Sammlung von Bad-Input-Manifests in `xtask/test-fixtures/manifest-strict/`. Jeder Fixture-Fall
wird als Harness-Test ausgefuehrt: bekannte schlechte Manifests muessen mit
`AppError::kind::PipelineValidation` failen, das korrekte Valid-Fixture muss parsen. Ziel ist
Pre-Commit-Mirror der Boot-Time-Executor-Strictness (Epic 1B FR6): Manifest-Autoren erhalten
lokalen Feedback-Loop, bevor das kaputte Manifest in einer Runtime-Crash-Situation landet.

Implementiert FR32. Gibt CI-Gate-Value gemaess `memory/feedback_ci_gate_philosophy.md` —
Preventive Enforcement mit Forcing-Sentinel.

## Acceptance Criteria

### AC-A — Subcommand-Registrierung und Dispatch

**Given** `xtask/src/main.rs` listet `manifest-strict` aktuell in der Help-Section als
geplanten Stub (nicht implementiert)
**When** Story 5.1 den Subcommand auf production-grade umsetzt
**Then**

- `xtask/src/main.rs` registriert `manifest-strict` im `match`-Dispatch-Arm:
  `Some("manifest-strict") => manifest_strict::run()`
- Ein neues Modul `xtask/src/manifest_strict.rs` (oder `xtask/src/manifest_strict/mod.rs`)
  traegt die Implementierung
- `fn run() -> ExitCode` gibt `ExitCode::SUCCESS` zurueck wenn alle Harness-Tests
  bestehen, `ExitCode::FAILURE` (Code 1) wenn mindestens ein Test failt
- Die Help-Ausgabe von `cargo xtask --help` nennt den Subcommand mit Kurzbeschreibung:
  `manifest-strict    Pre-commit gate: validates bad-input fixtures against parse_from_str (FR32)`
- `cargo xtask manifest-strict` laeuft headless ohne Filesystem-Seiteneffekte (keine
  temporaeren Dateien, kein `.cargo/` Schreiben)

### AC-B — Fixture-Layout und Valid-Manifest-Sentinel (Forcing-Sentinel-Pattern)

**Given** `memory/feedback_ci_gate_philosophy.md` mandatiert Forcing-Sentinel-Pattern —
mindestens ein Test, der gruen bleibt und bei deaktiviertem Lint fehlschlagen wuerde
**When** Story 5.1 das Fixture-Verzeichnis anlegt
**Then**

- Verzeichnis `xtask/test-fixtures/manifest-strict/` existiert mit folgenden Dateien:
  - `valid.toml` — ein korrektes Minimal-Manifest (`schema_version = 1`, mindestens
    eine bekannte Stage-Type); muss von `parse_from_str` erfolgreich geparst werden
  - `bad-unknown-stage.toml` — Manifest mit unbekanntem Stage-Type-String
  - `bad-missing-schema-version.toml` — Manifest ohne `schema_version`-Field
  - `bad-unsupported-schema-version.toml` — Manifest mit `schema_version = 99`
  - `bad-type-mismatch.toml` — Manifest mit type-inkompatiblen Stage-Paaren (z. B.
    Cleanup-Stage nach Passthrough, wo Cleanup `text`-Input erwartet aber
    Passthrough-Output-Context keinen garantierten `text`-Type liefert — konkretes
    Setup: Audio-Input + Cleanup-first, kein Stt-Stage davor)
  - `expected.toml` — Erwartungs-Tabelle: jede Fixture-Datei mit erwartetem Outcome
    (`ok` oder `err:<AppErrorKind-Variant>`) und optionalem `user_message_key`-Feld
- `valid.toml` ist der Forcing-Sentinel: wenn jemand `manifest_strict::run()` hardcoded
  `ExitCode::SUCCESS` zurueck gibt ohne echte Ausfuehrung, failt dieser Test (weil
  `valid.toml` durch `parse_from_str` bestanden werden muss und das einen echten Aufruf
  benoetigt); die Test-Ausgabe benennt `valid.toml` explizit als Forcing-Sentinel in
  einem Kommentar im Quellcode

### AC-C — Bad-Input-Szenario: Unbekannter Stage-Type (`error.pipeline.unknown_stage_type`)

**Given** Manifest `bad-unknown-stage.toml` referenziert einen Stage-Type-String
(z. B. `type = "transcription"`), der nicht im `PipelineStageType`-Serde-Enum registriert ist
(Epic 1B FR6, `epics.md:601`)
**When** der Harness `parse_from_str("bad-unknown-stage.toml")` aufruft
**Then**

- `parse_from_str` gibt `Err(AppError)` zurueck (kein Panic, kein `warn!+skip` —
  gemaess `memory/feedback_manifest_compile_contract.md`)
- `err.kind == AppErrorKind::PipelineValidation`
- `err.user_message == Some("error.pipeline.unknown_stage_type")` (Key aus
  `klarvo_core::manifest::keys::UNKNOWN_STAGE_TYPE`)
- Diese Validierung laeuft in **Pass-1 der Manifest-Parse (TOML-Parse-Layer)**: der
  Serde-`#[serde(tag = "type")]`-Enum-Match schlaegt fehl auf unbekannten Tag-Values
  — dies ist Compile-Time-Safety-Layer (Stage-Registry via Cargo-Features + Serde-Tag-Enum,
  `memory/project_manifest_boot_time_parse.md`)
- Der Harness benennt das AC in der Ausgabe: `[PASS] bad-unknown-stage: PipelineValidation
  (error.pipeline.unknown_stage_type)`

### AC-D — Bad-Input-Szenario: Fehlendes `schema_version`-Field

**Given** Manifest `bad-missing-schema-version.toml` hat keinen `schema_version`-Eintrag
(valide TOML-Syntax, aber fehlender Pflicht-Key)
**When** der Harness `parse_from_str("bad-missing-schema-version.toml")` aufruft
**Then**

- `parse_from_str` gibt `Err(AppError)` zurueck
- `err.kind == AppErrorKind::PipelineValidation`
- `err.user_message` ist `Some("error.pipeline.toml_parse_failure")` oder
  `Some("error.pipeline.schema_version_unsupported")` — beide Outcomes sind akzeptabel
  (Delegate-Choice abhaengig von Implementierungs-Path: serde-Deserialisierungsfehler
  vs. explizite Post-Parse-Validierung); der `expected.toml`-Eintrag dokumentiert welcher
  Key erwartet wird und darf bei Impl-Abweichung angepasst werden, solange `err.kind`
  korrekt ist
- Der Harness-Ausgabe: `[PASS] bad-missing-schema-version: PipelineValidation`

### AC-E — Bad-Input-Szenario: Nicht-unterstuetzte `schema_version`

**Given** Manifest `bad-unsupported-schema-version.toml` hat `schema_version = 99`
(TOML-Syntax korrekt, aber Version unbekannt)
**When** der Harness `parse_from_str("bad-unsupported-schema-version.toml")` aufruft
**Then**

- `parse_from_str` gibt `Err(AppError)` zurueck
- `err.kind == AppErrorKind::PipelineValidation`
- `err.user_message == Some("error.pipeline.schema_version_unsupported")`
  (Key aus `klarvo_core::manifest::keys::SCHEMA_VERSION_UNSUPPORTED`)
- Bestehende Unit-Tests in `klarvo-core/src/manifest.rs` (z. B. `schema_version_2_rejected`)
  decken diesen Fall bereits intern ab — der xtask-Harness validiert denselben Pfad
  end-to-end aus Harness-Binary-Perspektive
- Der Harness-Ausgabe: `[PASS] bad-unsupported-schema-version: PipelineValidation
  (error.pipeline.schema_version_unsupported)`

### AC-F — Bad-Input-Szenario: Type-Inkompatible Stage-Chain (`error.pipeline.stage_type_mismatch`)

**Given** Manifest `bad-type-mismatch.toml` konfiguriert eine Stage-Chain, in der der
Input-Type einer Stage nicht mit dem Output-Type der vorherigen Stage uebereinstimmt
(z. B. `Cleanup`-Stage als erste Stage bei Audio-Input — `Cleanup` erwartet `text`-Input,
aber das erste StageData ist `Audio`)

**Technischer Kontext (load-bearing):** Type-Chaining-Check ist **Runtime-Layer**, nicht
Parse-Layer (`memory/project_type_chaining_runtime_layer.md`, `epics.md:234-238`). Der
Executor's Boot-Time-Check-1 in `run_pipeline` uebernimmt diese Validierung — nicht
`parse_from_str`. Daher muss der FR32-Harness den **vollen Boot-Path** simulieren:
`parse_from_str` + `run_pipeline` (oder einen dedizierten Boot-Check-Entrypoint), um
Type-Mismatch zu fangen. Ein reiner `parse_from_str`-Aufruf genuegt fuer dieses Szenario
**nicht**.

**When** der Harness das Manifest laed und den Executor-Boot-Path durchlaueft
**Then**

- Der Harness ruft nach erfolgreichem `parse_from_str` den Executor-Boot-Check auf
  (entweder `run_pipeline` mit Dummy-Registry und Dummy-StageData, oder einen dedizierten
  `validate_manifest_boot_checks(manifest, registry)` Entrypoint falls Story-1B.5-Impl
  einen anbietet)
- Der Aufruf gibt `Err(AppError)` zurueck
- `err.kind == AppErrorKind::PipelineValidation`
- `err.user_message == Some("error.pipeline.stage_type_mismatch")`
  (Key aus `klarvo_core::pipeline::executor::keys::STAGE_TYPE_MISMATCH`)
- Falls `run_pipeline` keinen dedizierten Boot-Check-Entrypoint hat und eine
  vollstaendige Registry-Instanz benoetigt, darf der Harness eine leere
  `PluginRegistry::empty()` (oder Aequivalent) verwenden — der Type-Mismatch-Check
  laeuft vor dem Plugin-Lookup (Boot-Check-Ordering: Type-Chaining vor Plugin-Lookup,
  `memory/project_executor_stage_data_shape.md`)
- Der Harness-Ausgabe: `[PASS] bad-type-mismatch: PipelineValidation
  (error.pipeline.stage_type_mismatch)`
- **Cross-Story-Note:** Story 5.3 (FR34 `lint-events`) wird zusaetzlich den
  `match _ => ...`-No-Wildcard-Contract auf `PipelineStageType` statisch pruefen —
  das ist **nicht** 5.1-Scope (Verweis `epics.md:601`)

### AC-G — Ausgabe-Format und Exit-Codes

**Given** CI-Integration erfordert maschinenlesbare Exit-Codes und menschenlesbare
Diagnose-Ausgabe
**When** `cargo xtask manifest-strict` ausgefuehrt wird
**Then**

- Jeder Fixture-Fall wird auf `stderr` mit Prefix `[PASS]` oder `[FAIL]` ausgegeben
- Bei `[FAIL]` folgt: Fixture-Dateiname, erwartetes Outcome (aus `expected.toml`),
  tatstaechliches Outcome (Error-Kind + User-Message-Key oder `ok`)
- Abschliessende Zusammenfassung: `manifest-strict: N/M passed` auf `stderr`
- Exit-Code `0` wenn alle N Tests bestehen
- Exit-Code `1` wenn mindestens ein Test failt
- Exit-Code `2` bei internem Fehler (z. B. Fixture-Verzeichnis nicht auffindbar) —
  analog zu existierendem Pattern in `xtask/src/main.rs` (unbekannter Subcommand = Exit 2)
- Kein Farb-Escape-Sequence-Output (headless CI-kompatibel, kein `colored`/`termcolor`
  Dependency-Zusatz noetig)

### AC-H — CI-Gate-Integration und Forcing-Sentinel-Nachweis

**Given** `memory/feedback_ci_gate_philosophy.md` verlangt: Preventive Enforcement,
Forcing-Sentinel, Skip-by-Design, keine Stub-Checks
**When** Story 5.1 die Gate-Integration dokumentiert
**Then**

- `cargo xtask manifest-strict` schlaegt fehl, wenn `parse_from_str` fuer ein
  Bad-Input-Fixture kein `Err` zurueckgibt (Preventive Enforcement)
- `valid.toml` ist der Forcing-Sentinel: ein Harness, der ohne echte Ausfuehrung
  immer `ExitCode::SUCCESS` zurueckgibt, wuerde `valid.toml` als `ok`-Outcome nicht
  korrekt verifizieren koennen — dieser Test kann nur bei echter `parse_from_str`-
  Integration gruen sein
- **Skip-by-Design ist hier explizit nicht anwendbar:** es gibt keinen Known-Broken-
  Path, der Skip rechtfertigt; alle Fixture-Faelle muessen bestehen
- Kein Stub-Check: der Subcommand liest echte Fixture-Dateien und ruft echte Core-
  Entrypoints auf — kein Hard-Coded-`return ExitCode::SUCCESS`
- Ein `cargo xtask manifest-strict`-Aufruf in der CI-Pipeline (`.github/` oder
  Aequivalent) ist Empfehlung; die Story-AC erzwingt kein CI-Konfigurations-File-
  Aenderung (das ist Operations-Concern ausserhalb des Story-Scope), aber der
  Subcommand ist headless-CI-tauglich (AC-G)

## Technical Notes

### `parse_from_str` ist nicht `#[cfg(test)]`-gated — Cross-Epic-Consumer-Contract

Laut `epics.md:637-643` (Story 1B.2 AC) ist `parse_from_str` explizit **nicht**
`#[cfg(test)]`-gated, weil FR32-xtask-manifest-strict diesen Entrypoint aus einem
Harness-Binary konsumieren wird. Das Rustdoc auf `parse_from_str` dokumentiert:

> "Used by `cargo xtask manifest-strict` (Epic 5 FR32) to exercise bad-input scenarios
> at harness-compile-time — not a runtime user-facing API."

Story 5.1 loest diesen Forward-Contract ein. Kein Refactor an `manifest.rs` noetig
(der Cross-Epic-Consumer-Contract ist bereits Phase-1-Baseline).

### Zweischichtige FR6-Enforcement (Innovation-A)

FR6 ist zweischichtig (`epics.md:157`):
1. **Compile-Time-Layer:** Stage-Registry via Cargo-Features + `#[serde(tag = "type")]`-
   Enum — unbekannte Stage-Types sind bereits Serde-Parse-Failures
2. **Boot-Time-Layer:** Manifest-Match gegen Registry (Plugin-Lookup) + Type-Chaining-
   Executor-Check in `run_pipeline`

FR32 / Story 5.1 enforced primaar die **Boot-Time-Layer** auf xtask-Ebene. Die ACs
benennen beide Layers explizit: AC-C prueft Compile-Time-Layer-Manifestation (Serde-
Unknown-Stage), AC-F prueft Boot-Time-Layer-Manifestation (Executor-Type-Mismatch).

### Type-Chaining ist Runtime-Layer, nicht Parse-Layer

`memory/project_type_chaining_runtime_layer.md` + `epics.md:234-238`: der Type-Chaining-
Check sitzt im Executor (`run_pipeline` Boot-Time-Check-1), nicht in `parse_from_str`.
Ein reiner `parse_from_str`-Harness wuerde `bad-type-mismatch.toml` **nicht** fangen.
Story 5.1 muss den vollen Boot-Path (parse + Executor-Init) durchlaufen fuer AC-F.
Boot-Check-Ordering: Type-Chaining-Check (Check-1) laeuft **vor** Plugin-Lookup (Check-2),
daher ist eine leere Registry fuer den Type-Mismatch-Test ausreichend.

### Fixture-Format `expected.toml`

Empfohlenes Format:

```toml
[valid]
outcome = "ok"

[bad-unknown-stage]
outcome = "err"
error_kind = "PipelineValidation"
user_message_key = "error.pipeline.unknown_stage_type"

[bad-missing-schema-version]
outcome = "err"
error_kind = "PipelineValidation"
# user_message_key ist Delegate-Choice (toml_parse_failure oder schema_version_unsupported)

[bad-unsupported-schema-version]
outcome = "err"
error_kind = "PipelineValidation"
user_message_key = "error.pipeline.schema_version_unsupported"

[bad-type-mismatch]
outcome = "err"
error_kind = "PipelineValidation"
user_message_key = "error.pipeline.stage_type_mismatch"
```

Der Harness parst `expected.toml` und assertet jeden Fixture-Fall dagegen. Neue Fixtures
koennen durch Hinzufuegen einer Datei + Eintrag in `expected.toml` ergaenzt werden —
ohne Harness-Quellcode-Aenderung.

### Dependency auf `klarvo-core` Feature-Flags

`bad-type-mismatch.toml` benoetigt mindestens `stage-stt` oder `stage-cleanup` aktiviert,
damit ein Multi-Stage-Chain-Manifest ueberhaupt valide TOML ist. `xtask/Cargo.toml` muss
`klarvo-core` mit den gleichen Features wie `klarvo-windows-shell` referenzieren (oder
mindestens `stage-stt` + `stage-cleanup`), damit der Executor-Type-Chaining-Check
kompiliert. Delegate-Choice welche Features minimal benoetigt werden.

### Kein neuer Plugin-Not-Found-Fixture in 5.1-Scope

`error.pipeline.plugin_not_found` (Executor Boot-Check-2) wird indirekt durch den
Type-Mismatch-Harness-Path mitgetestet (leere Registry triggert nach Type-Chaining-Check
evtl. Plugin-Not-Found fuer valide Chains). Ein dedizierter `bad-plugin-not-found.toml`-
Fixture ist **optional** in 5.1 — wenn er eingefuegt wird, ist die Impl vollstaendiger,
aber er ist nicht AC-mandatory. Begruendung: Type-Chaining-Scenario ist die komplexere
Harness-Anforderung und hat hoeheren Spec-Wert.

## Dependencies

- Phase-1 complete (`memory/project_phase1_complete.md`) — `parse_from_str` und
  `run_pipeline` existieren als production-grade Entrypoints
- Story 1B.2 (`parse_from_str` nicht `#[cfg(test)]`-gated, Cross-Epic-Consumer-Contract):
  `epics.md:637-643`
- Story 1B.5 (Executor `run_pipeline`, Boot-Check-Ordering, Type-Chaining-Check):
  Boot-Path-Reuse fuer AC-F
- Story 4.4 (alle `error.pipeline.*`-Keys in Locale-Tables registriert) — keine
  neuen i18n-Keys in 5.1, aber Coverage-Voraussetzung ist erfuellt
- `memory/project_manifest_boot_time_parse.md` — Parse ist Boot-Time, nicht Compile-Time;
  Stage-Registry-Set-Ebene ist Compile-Time
- `memory/feedback_manifest_compile_contract.md` — kein `warn!+skip`, immer hart
  erroren bei unbekannten Stage-Types
- `memory/project_type_chaining_runtime_layer.md` — Type-Chaining sitzt im Executor,
  nicht im Parser; Harness muss vollen Boot-Path durchlaufen fuer AC-F
- `memory/feedback_ci_gate_philosophy.md` — Preventive Enforcement, Forcing-Sentinel,
  Skip-by-Design, keine Stub-Checks
- `memory/feedback_commit_hygiene.md` — xtask-Test-Fixtures unter `test-fixtures/`;
  Planning-Artifacts sofort committen

## Tasks/Subtasks

- [ ] Task 1 — Fixture-Verzeichnis und Fixture-Dateien anlegen (AC-B)
  - [ ] 1.1 `xtask/test-fixtures/manifest-strict/` Verzeichnis erstellen
  - [ ] 1.2 `valid.toml` — korrektes Minimal-Manifest mit `schema_version = 1` und
    bekannter Stage-Type (Passthrough oder Stt)
  - [ ] 1.3 `bad-unknown-stage.toml` — Manifest mit `type = "transcription"` oder
    anderem unbekannten Stage-Type-String
  - [ ] 1.4 `bad-missing-schema-version.toml` — valides TOML ohne `schema_version`-Field
  - [ ] 1.5 `bad-unsupported-schema-version.toml` — Manifest mit `schema_version = 99`
  - [ ] 1.6 `bad-type-mismatch.toml` — Manifest mit type-inkompatibler Chain (z. B.
    `Cleanup` als erste Stage, Audio-Input)
  - [ ] 1.7 `expected.toml` — Erwartungs-Tabelle fuer alle Fixtures

- [ ] Task 2 — Harness-Modul implementieren (AC-A, AC-C bis AC-G)
  - [ ] 2.1 `xtask/src/manifest_strict.rs` (oder `mod.rs`) anlegen mit `pub fn run() -> ExitCode`
  - [ ] 2.2 `expected.toml` parsen und Fixture-Liste aufbauen
  - [ ] 2.3 Fuer jedes Fixture: `parse_from_str` aufrufen, Ergebnis gegen Expected assertieren
  - [ ] 2.4 Fuer `bad-type-mismatch`: zusaetzlich Executor-Boot-Path aufrufen (leere Registry
    + Audio-StageData), Result gegen Expected assertieren (AC-F)
  - [ ] 2.5 `[PASS]`/`[FAIL]`-Ausgabe-Formatierung auf stderr (AC-G)
  - [ ] 2.6 Abschliessende Summary-Zeile `manifest-strict: N/M passed` auf stderr
  - [ ] 2.7 Forcing-Sentinel-Kommentar bei `valid.toml`-Test einbauen

- [ ] Task 3 — Subcommand in `xtask/src/main.rs` registrieren (AC-A)
  - [ ] 3.1 `mod manifest_strict;` hinzufuegen
  - [ ] 3.2 `Some("manifest-strict") => manifest_strict::run()` Dispatch-Arm
  - [ ] 3.3 Help-Text aktualisieren: `manifest-strict` mit Kurzbeschreibung

- [ ] Task 4 — `xtask/Cargo.toml` Dependency pruefen (Technical Notes)
  - [ ] 4.1 `klarvo-core` Dependency mit benoetigen Features (`stage-stt`, `stage-cleanup`)
    fuer Executor-Type-Chaining-Test sicherstellen
  - [ ] 4.2 Kein neuer externer Crate-Zusatz ausser bestehenden xtask-Dependencies

- [ ] Task 5 — Integration verifizieren
  - [ ] 5.1 `cargo xtask manifest-strict` headless ausfuehren, Exit-Code 0 bestaetigen
  - [ ] 5.2 Manuell einen Fixture-Fehler injizieren (z. B. `valid.toml` temporaer mit
    schema_version=99 beschaedigen), Exit-Code 1 bestaetigen, dann Revert
  - [ ] 5.3 `cargo build -p xtask` sauber kompiliert

## Dev Agent Record

### Completion Notes

_Leer (Story ist draft)_

### Story-Spec-Abweichung

_Leer (Story ist draft)_

## File List

_Leer (Story ist draft)_

## Change Log

_Leer (Story ist draft)_
