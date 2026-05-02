---
name: Story 6.2 — verify_release — Sentinel-Replacement durch Release-Filter-Gate
epic: 6
story_number: "6.2"
status: done
dependencies:
  - "6-1-telemetry-logging-rolling-file"
---

# Story 6.2: `verify_release` — Sentinel-Replacement durch Release-Filter-Gate

Status: done

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
//!   2. Release-Filter-Gate: `tracing-subscriber` MUST be a direct
//!      (non-transitive, non-dev) dependency of `klarvo-core`,
//!      AND `klarvo-core/src/telemetry/logging.rs` MUST contain a
//!      `RELEASE_MAX_LEVEL` const gated by `cfg(not(debug_assertions))` set to
//!      `LevelFilter::INFO`. Rationale: PII-Protection — DEBUG/TRACE events
//!      could leak request payloads into the Rolling-File-Log and the
//!      Debug-Export-Zip (memory `project_no_remote_telemetry.md`).
//!      Spec §4 Telemetrie + §4a Release-Hardening. The source-side check
//!      is a `syn`-AST walk over `logging.rs` (not token-grep), so disjoint
//!      tokens, comments, or weakened cfg-arms cannot bypass the gate.
```

### AC-5: Unit-Tests für neuen Gate

**Given** `verify_release.rs` Tests-Module (`#[cfg(test)] mod tests`),
**When** Story 6.2 committed ist,
**Then** existieren neue Tests für den Wrapper `check_tracing_release_filter` und seinen Pure-Input-Splitparter `check_tracing_release_filter_in`:

1. `release_filter_passes_when_dep_present_and_sentinel_in_source` — Mock-Metadata mit direktem `tracing-subscriber` Dep auf `klarvo-core` + Synthetic-Source mit korrektem Const → `Ok(())`
2. `release_filter_fails_when_dep_missing` — Mock-Metadata ohne `tracing-subscriber` → `Err` enthält "missing from workspace dependencies"
3. `release_filter_fails_when_source_sentinel_missing_const` — Mock-Metadata mit Dep + Synthetic-Source ohne `RELEASE_MAX_LEVEL` → `Err` enthält "release-level filter sentinel not found" (echter Wrapper-Err-Pfad, nicht nur Pure-Helper)

Zusätzlich (defensive Bypass-Resists für die strukturelle AST-Variante; siehe Dev-Notes-Amendment §AST-Shift):

- `release_filter_const_present_rejects_tokens_only_in_comment` — Tokens nur in Comments → `false`
- `release_filter_const_present_rejects_disjoint_items` — Tokens auf separaten Items → `false`
- `release_filter_const_present_rejects_weakened_filter` — `LevelFilter::TRACE` unter korrektem cfg → `false`
- `release_filter_const_present_rejects_inverted_cfg` — `cfg(debug_assertions)` statt `not(...)` → `false`
- `release_filter_fails_when_tracing_subscriber_only_transitive` — `tracing-appender` als direkter Dep, `tracing-subscriber` nur transitive → `Err`
- `release_filter_fails_when_tracing_subscriber_dev_only` — `kind: "dev"` direkt-deklariert → `Err`

**Refactor zur Testbarkeit (umgesetzt):**
- `release_filter_const_present(source: &str) -> bool` — `syn::parse_file` + Item-Walk auf `Item::Const` mit `cfg(not(debug_assertions))`-Attribut und `LevelFilter::INFO`-Initializer; eliminiert Token-Bypass-Pfade.
- `check_tracing_release_filter_in(metadata, source: &str) -> Result<(), String>` — pure-input, kombiniert direct-dep-check + struktureller Source-Check.
- `check_tracing_release_filter(metadata) -> Result<(), String>` — I/O-Wrapper, liest `klarvo-core/src/telemetry/logging.rs` und delegiert.

### AC-6: `cargo xtask verify-release` grün nach Story 6.2

**Given** Story 6.1 + Story 6.2 sind committed,
**When** `cargo xtask verify-release --skip-cross-compile` läuft,
**Then** exitiert mit Code 0 (alle Checks grün).

## Tasks / Subtasks

- [x] Sentinel löschen (AC-1)
  - [x] `check_tracing_subscriber_sentinel` Funktion entfernen
  - [x] `tracing_subscriber_sentinel_*` Tests entfernen
  - [x] Aufruf in `run()` entfernen (verhindert Compile-Fehler)
- [x] Module-doc-Header aktualisieren (AC-4)
- [x] Neue Funktion implementieren (AC-2 + AC-3)
  - [x] `release_filter_const_present(source: &str) -> bool` Helper (syn-AST-Walk; Code-Review-Patch P1)
  - [x] `check_tracing_release_filter_in(metadata, source: &str) -> Result<(), String>` Pure-Input-Variante (Code-Review-Patch P2)
  - [x] `check_klarvo_core_directly_depends_on_tracing_subscriber` Direct-Dep-Walk (Code-Review-Patch P3)
  - [x] `check_tracing_release_filter(metadata: &Metadata) -> Result<(), String>` I/O-Wrapper, ruft `_in`
  - [x] `run()`-Aufruf eintragen
- [x] Unit-Tests (AC-5 + Bypass-Resists aus Code-Review)
- [x] Smoke-Run: `cargo xtask verify-release --skip-cross-compile` grün (AC-6)
- [x] `fmt::layer()` Format-Config pinnen in `klarvo-core/src/telemetry/logging.rs` (W9-Defer aus 6.1-Code-Review)
  - [x] `with_target(true).with_thread_ids(false).with_thread_names(true)` zur fmt-Layer-Kette ergänzen (Thread-Names true: D2-Adjust aus Code-Review für Panic-Diagnose auf Audio-OS-Thread)
  - [x] Timestamp-Format empirisch prüfen (W7): Default-Timer (`SystemTime`) in tracing-subscriber 0.3.23 gibt bereits `YYYY-MM-DDTHH:MM:SS.ffffffZ` aus (UTC, RFC 3339). Kein `local-time`-Feature und kein `with_timer()`-Aufruf nötig.

### Review Findings

Aus `bmad-code-review` 2026-05-02 (Blind Hunter + Edge Case Hunter + Acceptance Auditor, parallel).

- [x] [Review][Decision] **D1 — Gate verifiziert Definition, nicht Anwendung** — **Resolution: Accept-as-deliberate-Trade-off.** Source-Sentinel ist explizite Architektur-Wahl (Story-5.6-Präzedenz, Phase-1-Scope); `logging.rs` ist klein/reviewbar, ein Subscriber-Refactor wäre nicht zu übersehen. Runtime-Smoke-Test (Release-Build + Execute + Rolling-File-Inspect) ist eine *andere* Architektur und wäre Scope-Creep. Rationale dokumentiert in Dev-Notes-Amendment §AST-Shift.
- [x] [Review][Decision] **D2 — W9 fmt-layer Trade-offs ohne AC-Coverage** — **Resolution: Adjust.** `with_thread_names(false)` → `with_thread_names(true)` für Panic-Diagnose (per `project_shell_runtime_model`: 2 distincte Threads — tokio-Runtime + cpal-Audio-OS-Thread; Audio-Thread-Panic ohne Thread-Name verliert Diagnose-Achse). `with_target(true)` und `with_thread_ids(false)` bleiben (Modul-Pfade sind Code-Identifier, null PII; Thread-IDs sind opake numerische OS-IDs ohne Diagnose-Wert wenn Names verfügbar).
- [x] [Review][Decision] **D3 — Spec-Re-Write + Status-Flip im selben Diff** — **Resolution: Split-Commit.** Working-Tree wird zerlegt — Spec-Erweiterung (W9/W7/W3-Defer-Sections + neue Tasks + File List + Frontmatter-Status `backlog → review`) als separater Commit *vor* Impl-Commit, gemäß `feedback_commit_hygiene` (Contract-before-Implementation-Split).
- [x] [Review][Patch] **P1 — Source-grep `release_filter_tokens_present` trivial bypassbar** — **Resolution: Replaced.** Token-grep durch `syn::parse_file` + AST-Item-Walk ersetzt (`release_filter_const_present`). Sechs Bypass-Resist-Tests (Comment-only, disjoint-Items, weakened-Filter, inverted-cfg, fully-qualified-path acceptance, canonical-form acceptance). Spec-L165 Source-Grep-Pattern-Block durch §AST-Shift Dev-Notes-Amendment ergänzt.
- [x] [Review][Patch] **P2 — AC-5 #3 Test exerziert Wrapper-Err-Pfad nicht** — **Resolution: Refactored.** Funktion in `check_tracing_release_filter_in(metadata, source: &str)` (pure) + `check_tracing_release_filter(metadata)` (I/O-Wrapper) gesplittet. AC-5 #1/#2/#3 Tests laufen jetzt alle gegen `_in`-Variante mit Synthetic-Source-Strings; AC-5 #3 prüft explizit Wrapper-Err-Substring `"release-level filter sentinel not found"`. Cross-crate-fs-Coupling eliminiert.
- [x] [Review][Patch] **P3 — Dep-Presence-Check matcht transitive Deps** — **Resolution: Direct-Dep-Walk.** `Metadata.Package` um `dependencies: Vec<Dependency>` erweitert (mit `kind: Option<String>`); `check_klarvo_core_directly_depends_on_tracing_subscriber` walkt klarvo-cores Package-Manifest-Deps und akzeptiert nur `kind: null` (normal). Bypass-Resists: `release_filter_fails_when_tracing_subscriber_only_transitive` + `_dev_only`.
- [x] [Review][Patch] **P4 — Spec-Doku W7-Widerspruch** — **Resolution: Cleaned.** Project Structure Notes + Dev Notes W7-Block auf empirisches Outcome aktualisiert (kein Cargo-Delta).
- [x] [Review][Defer] **W2 — Opaque I/O-Error Messages im Wrapper** [`xtask/src/verify_release.rs:208-211`] — `locate_workspace_root` returning None, `read_to_string` mit NotFound/InvalidData/permissions führen zu generischen `failures.push(msg)`, ununterscheidbar von echter PII-Regression. Fix wäre: Distinguish `ErrorKind::NotFound` (rename-guidance), `from_utf8_lossy` als Fallback, separater Test für None-Path. — deferred, niedrig, kein Blocker.
- [x] [Review][Defer] **W3 — W7 empirisches Artifact fehlt** [Spec L240 Debug Log References, leer] — Per `feedback_spike_rigor`: Spike-Claims brauchen Messwerte. Eine Log-Zeile aus dem empirischen Run als Beleg in Completion-Notes #7 oder Debug Log References einfügen. — deferred, doc-only.

**Dismissed (Noise):** sprint-status.yaml Whitespace-Alignment (kosmetisch), `failures.push` ohne Short-Circuit auf PII-Fail (existing run()-Pattern by-design), TOCTOU Source-Grep vs Binary (strukturell zum Source-Grep-Trade-off, durch Spec L165 explizit gewählt).

## Dev Notes

### AST-Shift (Code-Review-Amendment 2026-05-02)

**Original Plan:** Token-grep mit drei `.contains()`-Calls auf Datei-Inhalten (analog Story 5.6 REQUIRED_KEYS-Drift).

**Code-Review-Befund (Blind Hunter + Edge Case Hunter):** Token-grep ist trivial bypassbar — alle drei Tokens können in Comments stehen, auf disjunkten Items verteilt sein, oder mit invertiertem `cfg(debug_assertions)` auftauchen. Der "strict genug"-Claim aus dem Original-Plan war empirisch falsch.

**Resolution:** Strukturelle AST-Variante via `syn::parse_file` (Crate ist bereits Dep von xtask). `release_filter_const_present` walkt `file.items`, sucht ein `Item::Const` mit `ident == "RELEASE_MAX_LEVEL"`, prüft `cfg(not(debug_assertions))`-Attribut und `LevelFilter::INFO`-Initializer (last-2-Path-Segments-Match, akzeptiert sowohl `LevelFilter::INFO` als auch `tracing_subscriber::filter::LevelFilter::INFO`).

**Trade-off-Note (D1):** Der Gate verifiziert *Definition*, nicht *Anwendung*. `init_tracing` könnte den Subscriber löschen/ersetzen ohne dass der Gate feuert. Akzeptiert als deliberate Architektur-Wahl: `logging.rs` ist klein/reviewbar (~70 Zeilen), ein Subscriber-Refactor wäre beim Code-Review nicht zu übersehen. Defense-in-Depth via Runtime-Smoke-Test (Release-Build + Execute + Rolling-File-Inspect) wäre Scope-Creep für Phase 1; Re-evaluation falls echte Subscriber-Drift-Reports kommen.

### Warum diese 3 Strukturmerkmale reichen

`RELEASE_MAX_LEVEL`-Ident + `cfg(not(debug_assertions))`-Attr + `LevelFilter::INFO`-RHS auf *einem* Const-Item ergeben strukturell: es gibt eine cfg-gated Const die im Release-Build auf INFO klemmt. Disjoint-Token-Bypass ausgeschlossen durch Item-Walk; Comment-Bypass ausgeschlossen weil Comments keine syn-Items sind; weakened-Filter (`LevelFilter::TRACE`) ausgeschlossen durch Path-Match auf `INFO`.

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

### W9/W7/W3 — Defers aus 6.1-Code-Review

**W9 (fmt Format-Config-Pinning, `deferred-work.md §6.1-W9`):** `fmt::layer()` in `logging.rs:62-66` nutzt Defaults für `with_target`/`with_thread_*`/Timer. Defaults können zwischen Minor-Versionen driften. Explizit gepinnt (mit Code-Review-Adjust D2 für `with_thread_names`):
```rust
fmt::layer()
    .with_ansi(false)
    .with_target(true)         // qualifizierter Event-Pfad im Log
    .with_thread_ids(false)    // numerische OS-IDs ohne Diagnose-Wert wenn Names da
    .with_thread_names(true)   // Audio-OS-Thread vs tokio-Runtime-Panic-Diagnose (D2)
    .with_writer(non_blocking)
    .with_filter(RELEASE_MAX_LEVEL)
```
**D2-Rationale:** `project_shell_runtime_model` etabliert 2 distincte Threads (tokio-Runtime managed by Tauri + cpal-Audio-OS-Thread). Ein Panic auf dem Audio-Thread ohne Thread-Name verliert die Diagnose-Achse, die für die häufigsten Phase-1-Bugs (Audio-Capture-Glitches) load-bearing wäre.

**W7 (Timestamp-Format, `deferred-work.md §6.1-W7`):** **Outcome: kein Delta nötig.** Empirisch verifiziert (Debug-Build, Live-Log-Output): `tracing-subscriber 0.3.23` Default-Timer (`SystemTime`) emittiert bereits `YYYY-MM-DDTHH:MM:SS.ffffffZ` (UTC, RFC-3339-konform). Kein `local-time`-Feature, kein `with_timer()`-Aufruf, kein Cargo-Delta. Originalplan (`UtcTime::rfc_3339()` mit `local-time`-Feature) hätte den gleichen Output produziert — der Default macht's bereits.

**W3 (EnvFilter/RUST_LOG, `deferred-work.md §6.1-W3`):** Story 6.2 adressiert nur Release-Gate. `RELEASE_MAX_LEVEL` ist Hard-Ceiling über jedem zukünftigen `EnvFilter`-Layer (Layer-Stack-Reihenfolge matters). Defer zu Story 6.3 oder separater Story — nicht in 6.2 öffnen.

### Project Structure Notes

Geänderte Dateien:
- `xtask/src/verify_release.rs` (Sentinel-Delete + neue Funktionen + Bypass-Resist-Tests + Module-doc-Update; Code-Review-Refactor: AST-Walk + Direct-Dep-Check + Pure-Input-Splitparter)
- `klarvo-core/src/telemetry/logging.rs` (fmt::layer() Format-Config-Pinning W9-Defer; `with_thread_names(true)` per Code-Review D2)

Keine neuen Dateien. Keine Cargo-Dep-Änderung nötig (W7-Outcome: Default-Timer ist bereits RFC-3339).

### References

- `xtask/src/verify_release.rs` — Module-doc-Header (`//!   2. Release-Filter-Gate ...`), `run()`-Aufruf-Block, `locate_workspace_root()`, `release_filter_const_present` + `check_klarvo_core_directly_depends_on_tracing_subscriber` + `check_tracing_release_filter_in` + `check_tracing_release_filter`
- `klarvo-core/src/telemetry/logging.rs` — `RELEASE_MAX_LEVEL`-Const + `fmt::layer()`-Pinning
- [architecture.md §4a Release-Hardening] — Verbose-Liste der verpflichtenden Checks; Filter ist 4. Punkt
- [memory/feedback_ci_gate_philosophy.md] — Forcing-Sentinel-Pattern
- [memory/project_no_remote_telemetry.md] — PII-Protection-Rationale für Filter
- [memory/feedback_commit_hygiene.md] — Contract-before-Implementation-Split (D3)
- [memory/project_shell_runtime_model.md] — 2-Thread-Runtime-Modell (D2-Rationale)
- Story 6.1 — etabliert `RELEASE_MAX_LEVEL`-Konstante (Source-Sentinel-Token)
- Story 5.6 (REQUIRED_KEYS-Drift) — Original-Präzedenz für Source-Grep; in 6.2 zu syn-AST aufgewertet (siehe §AST-Shift)
- [deferred-work.md §6.1-W9] — fmt::layer() Format-Config-Pinning-Defer (adressed in diesem Story)
- [deferred-work.md §6.1-W7] — ISO-8601-Timestamp-Verifikation-Defer (adressed: kein Delta nötig)
- [deferred-work.md §6.1-W3] — EnvFilter/RUST_LOG-Defer (zu Story 6.3 oder eigene Story)
- [deferred-work.md §code review of story-6.2 (2026-05-02)] — W2 Opaque-IO-Errors + W3 W7-Empirical-Artifact

## Dev Agent Record

### Agent Model Used

claude-opus-4-7[1m] (create-story 2026-05-01; enriched 2026-05-02 — W9/W7/W3-Defer-Integration)

### Debug Log References

### Completion Notes List

- Sentinel-Funktion `check_tracing_subscriber_sentinel` + 2 zugehörige Tests entfernt (AC-1).
- Module-doc aktualisiert: Eintrag #2 beschreibt jetzt den echten Release-Filter-Gate (AC-4).
- `check_tracing_release_filter` implementiert: Check-1 direct-dep + Check-2 Source-Sentinel (AC-2, AC-3).
- `cargo xtask verify-release --skip-cross-compile` exitiert Code 0 (AC-6).
- W9: `with_target(true).with_thread_ids(false).with_thread_names(true)` zur fmt-Layer-Kette ergänzt (logging.rs). `thread_names(true)` per Code-Review D2 (Audio-OS-Thread-Panic-Diagnose).
- W7: Empirisch verifiziert — tracing-subscriber 0.3.23 `SystemTime`-Timer gibt `YYYY-MM-DDTHH:MM:SS.ffffffZ` (UTC RFC 3339) aus. Kein `local-time`-Feature und kein `with_timer()` nötig.

**Code-Review 2026-05-02 (4 Patches + 3 Decisions + 2 Defers):**
- P1 (Source-grep → AST-Walk): `release_filter_tokens_present` durch `release_filter_const_present` mit `syn::parse_file` ersetzt. Walkt `Item::Const` mit `cfg(not(debug_assertions))`-Attribut + `LevelFilter::INFO`-RHS. Eliminiert Comment-/Disjoint-Item-/Inverted-cfg-/Weakened-Filter-Bypass.
- P2 (Wrapper-Err-Pfad testbar): Funktion gesplittet in `check_tracing_release_filter` (I/O) + `check_tracing_release_filter_in(metadata, source: &str)` (pure). AC-5 #3 prüft jetzt Wrapper-Err-Substring direkt; Cross-crate-fs-Coupling im happy-path-Test eliminiert.
- P3 (Direct-Dep-Walk): `Metadata.Package` um `dependencies: Vec<Dependency>` erweitert; `check_klarvo_core_directly_depends_on_tracing_subscriber` walkt klarvo-cores Package-Manifest-Deps mit `kind: null`-Filter. Bypass-Resists für transitive + dev-only.
- P4 (Spec-Doku-Cleanup): W7-Widerspruch (Cargo-Delta "wenn nötig" vs "nicht nötig") aufgelöst.
- D1 (Definition-vs-Anwendung): Accept-as-deliberate-Trade-off; Rationale dokumentiert in Dev-Notes-§AST-Shift.
- D2 (W9 Thread-Names): Adjust → `with_thread_names(true)` für Audio-OS-Thread-Panic-Diagnose.
- D3 (Spec-Re-Write + Status-Flip): Split-Commit (Spec-Erweiterung Commit 1, Impl + Status + Review-Findings Commit 2) gemäß `feedback_commit_hygiene`.
- W2 (Opaque-IO-Errors) + W3 (W7-Empirical-Artifact): Defers in `deferred-work.md §code review of story-6.2 (2026-05-02)`.

Test-Coverage final: 55 Tests grün in xtask (vorher 49; +12 neue Bypass-Resist + Direct-Dep-Resist Tests, −6 alte Token-Level Tests).

### File List

- `xtask/src/verify_release.rs`
- `klarvo-core/src/telemetry/logging.rs`
