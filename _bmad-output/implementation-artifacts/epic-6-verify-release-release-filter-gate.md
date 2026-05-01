---
name: Story 6.2 — verify_release — Sentinel-Replacement durch Release-Filter-Gate
epic: 6
story_number: "6.2"
status: backlog
dependencies:
  - "6-1-telemetry-logging-rolling-file"
---

# Story 6.2: `verify_release` — Sentinel-Replacement durch Release-Filter-Gate

Status: backlog

## Story

Als Release-Engineer / CI-Maintainer
möchte ich den Forcing-Sentinel `check_tracing_subscriber_sentinel` in `xtask/src/verify_release.rs` durch einen echten Release-Filter-Gate `check_tracing_release_filter` ersetzen,
damit `cargo xtask verify-release` den `tracing-subscriber` als erwartete Dependency akzeptiert UND mechanisch verifiziert, dass DEBUG/TRACE-Events im Release-Build gefiltert werden (NFR5 PII-Protection: Debug-Export-Zip darf keine sensiblen Request-Payloads enthalten).

## Kontext und Motivation

**Forcing-Sentinel-Pattern** (CI-Gate-Philosophy, `memory/feedback_ci_gate_philosophy.md`): Phase-0 hat in `xtask/src/verify_release.rs` einen Sentinel installiert, der fehlschlägt sobald `tracing-subscriber` als Dependency vorhanden ist. Sinn: wer `tracing-subscriber` hinzufügt, muss zwingend auch den echten Release-Filter-Check implementieren, statt den Subscriber stumm hinzuzufügen ohne PII-Protection.

**Story 6.1** fügt `tracing-subscriber` + `tracing-appender` zum Workspace hinzu und erstellt `klarvo-core::telemetry::logging` mit `RELEASE_MAX_LEVEL`-Konstante. **Damit feuert der Sentinel.** `cargo xtask verify-release --skip-cross-compile` exitiert nach Story 6.1 mit Code 1.

**Story 6.2** löst den Sentinel ein:
1. Sentinel-Funktion `check_tracing_subscriber_sentinel` löschen
2. Neue Funktion `check_tracing_release_filter` implementieren (2 Checks: Dep-Presence + Source-Sentinel)
3. `cargo xtask verify-release --skip-cross-compile` wieder grün

**Source-Grep-Ansatz** (statt AST-Parsing): Die `RELEASE_MAX_LEVEL`-Konstante in `klarvo-core/src/telemetry/logging.rs` dient als Drift-Sentinel. Sie wird mit `cfg(not(debug_assertions))` auf `LevelFilter::INFO` gesetzt. Wenn jemand diesen Filter entfernt oder lockert (z. B. `LevelFilter::DEBUG` im Release-Branch), schlägt der Source-Grep an. Das ist analog zum Pattern aus Story 5.6 (REQUIRED_KEYS-Drift-xtask) — kein dynamic linking, kein Macro-Expansion, kein AST — nur stable Token-Suche im Source.

## Acceptance Criteria

### AC-1: Sentinel-Funktion `check_tracing_subscriber_sentinel` gelöscht

**Given** `xtask/src/verify_release.rs` enthält `check_tracing_subscriber_sentinel` (Zeilen 190–207 vor Story 6.2),
**When** Story 6.2 committed ist,
**Then**:
- Funktion `check_tracing_subscriber_sentinel` ist vollständig entfernt
- Zugehörige Unit-Tests `tracing_subscriber_sentinel_absent_passes` + `tracing_subscriber_sentinel_present_fails_with_guidance` sind entfernt
- `run()`-Funktion ruft die alte Funktion nicht mehr (Compile würde sonst brechen)
- Module-doc (`//!` Header in `verify_release.rs`) entfernt den Sentinel-Eintrag (#2 in der bisherigen Liste) und ersetzt ihn durch die Beschreibung des neuen Release-Filter-Gates

### AC-2: Neue Funktion `check_tracing_release_filter` mit zwei Checks

**Given** `verify_release.rs` ist in der Form aus AC-1,
**Then** existiert eine neue Funktion mit dieser Signatur:
```rust
fn check_tracing_release_filter(metadata: &Metadata) -> Result<(), String>
```

**Check 1 — Dependency-Presence:**
```rust
let present = metadata.packages.iter().any(|p| p.name == "tracing-subscriber");
if !present {
    return Err(
        "`tracing-subscriber` missing from workspace dependencies. \
         Rolling-file logging (FR37) requires it. Add to root `Cargo.toml` \
         and reference via `klarvo-core/Cargo.toml`. Spec: architecture.md §4 Telemetrie."
            .into(),
    );
}
```

**Check 2 — Source-Sentinel im `logging.rs`:**
```rust
let root = locate_workspace_root().ok_or("could not locate workspace root")?;
let logging_src = root.join("klarvo-core/src/telemetry/logging.rs");
let content = std::fs::read_to_string(&logging_src)
    .map_err(|e| format!("cannot read {}: {e}", logging_src.display()))?;

let has_const = content.contains("RELEASE_MAX_LEVEL");
let has_release_cfg = content.contains("not(debug_assertions)");
let has_info_level = content.contains("LevelFilter::INFO");

if !(has_const && has_release_cfg && has_info_level) {
    return Err(
        "release-level filter sentinel not found in klarvo-core/src/telemetry/logging.rs. \
         Expected: `RELEASE_MAX_LEVEL` const gated by `cfg(not(debug_assertions))` \
         and set to `LevelFilter::INFO`. PII-Protection: DEBUG/TRACE must not reach \
         release builds (sensitive request payloads could leak into Debug-Export-Zip). \
         Spec: architecture.md §4a Release-Hardening + Story 6.1 AC-4."
            .into(),
    );
}
```

Beide Checks sind sequentiell — Check 1 fail-fast vor Check 2 (kein Read auf eine Datei wenn die Dep fehlt; ergibt keinen aussagekräftigen Error).

### AC-3: `run()`-Funktion ruft den neuen Gate

**Given** `run()` ruft bisher `check_tracing_subscriber_sentinel` (Zeilen 101–103),
**When** Story 6.2 committed ist,
**Then** ruft `run()` stattdessen `check_tracing_release_filter`:
```rust
if let Err(msg) = check_tracing_release_filter(&metadata) {
    failures.push(msg);
}
```

### AC-4: Module-doc-Header aktualisiert

**Given** `verify_release.rs` Module-doc hat aktuell den Sentinel-Eintrag (`//!   2. Sentinel: tracing-subscriber must NOT be...`, Zeilen 19–25),
**When** Story 6.2 committed ist,
**Then** ist der Eintrag ersetzt durch eine Beschreibung des neuen Filter-Gates:
```text
//!   2. Release-Filter-Gate: `tracing-subscriber` MUST be a resolved dependency
//!      AND `klarvo-core/src/telemetry/logging.rs` MUST contain a
//!      `RELEASE_MAX_LEVEL` const gated by `cfg(not(debug_assertions))` set to
//!      `LevelFilter::INFO`. Rationale: PII-Protection — DEBUG/TRACE events
//!      could leak request payloads into the Rolling-File-Log and the
//!      Debug-Export-Zip (memory `project_no_remote_telemetry.md`).
//!      Spec §4 Telemetrie + §4a Release-Hardening. Source-grep approach
//!      mirrors Story 5.6 REQUIRED_KEYS-drift gate.
```

### AC-5: Unit-Tests für neuen Gate

**Given** `verify_release.rs` Tests-Module (`#[cfg(test)] mod tests`),
**When** Story 6.2 committed ist,
**Then** existieren neue Tests für `check_tracing_release_filter`:

1. `release_filter_passes_when_dep_present_and_sentinel_in_source` — Mock-Metadata mit `tracing-subscriber`, echte `klarvo-core/src/telemetry/logging.rs` enthält die Sentinel-Tokens → `Ok(())`
2. `release_filter_fails_when_dep_missing` — Mock-Metadata ohne `tracing-subscriber` → `Err` enthält "missing from workspace dependencies"
3. `release_filter_fails_when_source_sentinel_missing_const` — synthetic logging.rs ohne `RELEASE_MAX_LEVEL` → `Err` enthält "release-level filter sentinel not found"

Hinweis: Test #1 nutzt die echte Source-Datei (lebt im selben Workspace via `locate_workspace_root`); Tests #2 + #3 müssen die Source-File-Lookup mocken oder die Funktion in zwei Hilfsfunktionen splitten (eine reine Token-Check-Funktion testbar mit `&str`-Input, eine I/O-Wrapper).

**Empfohlener Refactor zur Testbarkeit:**
```rust
fn release_filter_tokens_present(content: &str) -> bool {
    content.contains("RELEASE_MAX_LEVEL")
        && content.contains("not(debug_assertions)")
        && content.contains("LevelFilter::INFO")
}
```
Tests prüfen `release_filter_tokens_present` mit handgebauten Strings; `check_tracing_release_filter` ist dann der I/O-Wrapper darüber.

### AC-6: `cargo xtask verify-release` grün nach Story 6.2

**Given** Story 6.1 + Story 6.2 sind committed,
**When** `cargo xtask verify-release --skip-cross-compile` läuft,
**Then** exitiert mit Code 0 (alle Checks grün).

## Tasks / Subtasks

- [ ] Sentinel löschen (AC-1)
  - [ ] `check_tracing_subscriber_sentinel` Funktion entfernen
  - [ ] `tracing_subscriber_sentinel_*` Tests entfernen
  - [ ] Aufruf in `run()` entfernen (verhindert Compile-Fehler)
- [ ] Module-doc-Header aktualisieren (AC-4)
- [ ] Neue Funktion implementieren (AC-2 + AC-3)
  - [ ] `release_filter_tokens_present(content: &str) -> bool` Helper
  - [ ] `check_tracing_release_filter(metadata: &Metadata) -> Result<(), String>` mit Check 1 + Check 2
  - [ ] `run()`-Aufruf eintragen
- [ ] Unit-Tests (AC-5)
- [ ] Smoke-Run: `cargo xtask verify-release --skip-cross-compile` grün (AC-6)

## Dev Notes

### Source-Grep-Pattern (Story-5.6-Präzedenz)

`xtask/src/required_keys_drift.rs` (Story 5.6) liest Source-Files und prüft auf Token-Presence. Story 6.2 folgt demselben Pattern: keine `syn`-AST-Analyse, kein Macro-Expansion — nur `.contains(&str)` auf Datei-Inhalten. Das ist robust gegen Refactor-Noise (Whitespace, Reihenfolge), aber strict genug um Token-Drift zu erkennen.

### Warum diese 3 Tokens reichen

`RELEASE_MAX_LEVEL` + `not(debug_assertions)` + `LevelFilter::INFO` zusammen ergeben: es gibt eine cfg-gated Const die im Release-Build auf INFO klemmt. Schwächer (z. B. nur `LevelFilter::INFO`) wäre falsch-positiv (könnte irgendwo im File stehen, nicht im Release-Branch). Stärker (z. B. AST-Match) ist unverhältnismäßig für Phase 1.

### `locate_workspace_root()` ist bereits vorhanden

In `verify_release.rs` Zeilen 137–140:
```rust
fn locate_workspace_root() -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().map(Path::to_path_buf)
}
```
Direkt verwendbar. `Path::to_path_buf` macht es Owned, danach `root.join("klarvo-core/src/telemetry/logging.rs")`.

### `xtask`-Crate ist nicht vom `disallowed_methods`-Lint betroffen

Story 5.5 (Lint-Gate) scope ist `klarvo-core` + `klarvo-windows-shell` + `klarvo-shell-orchestrator`. `xtask` ist freier Boden — `.unwrap()` / `.expect()` in xtask-Tests sind erlaubt (für klare Test-Failures).

### Module-doc-Drift mit Story 6.1

Bei Story 6.1 wurde der Forcing-Sentinel im Module-doc bereits als "noch zu lösen" annotiert. In Story 6.2 wird der Eintrag final ausgetauscht. Wenn beide Stories Tag-an-Tag ausgeführt werden, ist die Drift minimal. Wenn dazwischen Tage liegen: das `cargo xtask verify-release` ist temporär rot (gewollter Forcing-Effekt).

### Project Structure Notes

Geänderte Dateien:
- `xtask/src/verify_release.rs` (Sentinel-Delete + neue Funktion + Tests + Module-doc-Update)

Keine neuen Dateien, keine Cargo-Dep-Änderungen.

### References

- [xtask/src/verify_release.rs:19-25] — aktueller Sentinel-Eintrag in Module-doc
- [xtask/src/verify_release.rs:101-103] — aktueller `run()`-Aufruf
- [xtask/src/verify_release.rs:137-140] — `locate_workspace_root()`
- [xtask/src/verify_release.rs:190-207] — alter Sentinel-Body (zu löschen)
- [xtask/src/verify_release.rs:375-390] — alte Sentinel-Tests (zu löschen)
- [architecture.md §4a Release-Hardening] — Verbose-Liste der verpflichtenden Checks; Filter ist 4. Punkt
- [memory/feedback_ci_gate_philosophy.md] — Forcing-Sentinel-Pattern
- [memory/project_no_remote_telemetry.md] — PII-Protection-Rationale für Filter
- Story 6.1 — etabliert `RELEASE_MAX_LEVEL`-Konstante (Source-Sentinel-Token)
- Story 5.6 (REQUIRED_KEYS-Drift) — Präzedenz für Source-Grep-Pattern in xtask

## Dev Agent Record

### Agent Model Used

claude-opus-4-7[1m] (create-story 2026-05-01)

### Debug Log References

### Completion Notes List

### File List
