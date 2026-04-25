---
name: Story 5.2 — `cargo xtask bindings-drift` (FR33)
epic: 5
story_number: "5.2"
status: draft
dependencies:
  - "3.1"
---

# Story 5.2: `cargo xtask bindings-drift`

## Outcome

`cargo xtask bindings-drift` vergleicht den deterministischen Output von
`generate-bindings` byte-genau gegen das committede
`shells/windows/src/bindings/index.ts`. Drift — d. h. Core-API-Änderung ohne
anschließendes Regenerieren + Committen der Bindings — wird fail-loud mit Exit-Code
!= 0 gemeldet; die Fehlermeldung enthält einen actionable Hinweis
(`run \`cargo xtask generate-bindings\` and commit`). In-Sync-Stand liefert Exit-Code 0.
Die Implementierung setzt einen kleinen Refactor in `xtask/src/generate_bindings.rs`
voraus, der `render_to_string()` und `write_to_disk()` trennt, sodass beide Subcommands
dieselbe Render-Logik teilen. `xtask/src/main.rs` wird um den neuen Subcommand und den
zugehörigen `print_help`-Eintrag ergänzt. Ein Forcing-Sentinel-Test weist nach, dass
künstlich erzeugter Drift mechanisch gefangen wird. FR33 ist damit vollständig geschlossen;
FR24 (Bindings-only-Consumption) wird mechanisch abgesichert: Shell-Drift bei Core-API-
Änderung ist lokal und in CI gleichermassen fatal.

## Acceptance Criteria

### AC-A — `generate_bindings.rs` Refactor: Render/Write getrennt

**Given** `xtask/src/generate_bindings.rs:run()` schreibt den TS-Output direkt nach
`shells/windows/src/bindings/index.ts` (ein einziger Schritt ohne Rückgabewert)
**When** Story 5.2 den Refactor durchführt
**Then**

- `generate_bindings.rs` exportiert eine neue reine Funktion
  `pub fn render() -> Result<String, Box<dyn std::error::Error>>`, die den
  TypeScript-Bindings-Content als `String` zurückgibt — ohne Datei-I/O
- Eine separate (crate-interne) Funktion übernimmt das Schreiben:
  `fn write_to_disk(content: &str) -> ExitCode`; sie schreibt nach
  `shells/windows/src/bindings/index.ts` wie bisher
- `generate_bindings::run()` ruft `render()` + `write_to_disk()` nacheinander auf;
  externes Verhalten (Exit-Code, Fehlermeldungen) ist identisch zum Vor-Refactor-Stand
- `cargo xtask generate-bindings` bleibt grün nach dem Refactor

### AC-B — Neuer Subcommand `bindings-drift` in `main.rs` registriert

**Given** `xtask/src/main.rs` kennt den Subcommand `bindings-drift` nicht, und
`print_help()` listet ihn weder als aktiv noch als Stub
**When** Story 5.2 den Subcommand hinzufügt
**Then**

- `main.rs` enthält den Match-Arm:
  ```rust
  Some("bindings-drift") => bindings_drift::run(),
  ```
- `main.rs` enthält `mod bindings_drift;`
- `print_help()` listet `bindings-drift` unter den aktiven Subcommands mit einem
  prägnanten Beschreibungstext, z. B.:
  ```
    bindings-drift      Drift-Check: failt wenn shells/windows/src/bindings/index.ts nicht synchron mit generate-bindings-Output ist
  ```
- `cargo xtask --help` gibt den aktualisierten Help-Text aus

### AC-C — Drift erkannt → Exit-Code != 0 + actionable Fehlermeldung

**Given** `shells/windows/src/bindings/index.ts` enthält einen committed Stand, der
vom aktuellen `generate-bindings`-Output abweicht (z. B. manuell bearbeitete Datei
oder Core-API-Änderung ohne Bindings-Regen)
**When** `cargo xtask bindings-drift` ausgeführt wird
**Then**

- Der Prozess terminiert mit Exit-Code != 0 (Empfehlung: Exit-Code 1)
- `stderr` enthält mindestens:
  - eine Zeile, die klar „bindings drift detected" o. ä. meldet
  - den Pfad zur Datei (`shells/windows/src/bindings/index.ts`)
  - die Handlungsaufforderung:
    `run \`cargo xtask generate-bindings\` and commit the updated bindings`
- Optional (Delegate-Choice): Ausgabe der Byte-Differenz-Größe oder eines unified Diff.
  `similar`-Crate ist kein Workspace-Bestandteil; falls Diff-Output gewünscht,
  muss die Dependency in `xtask/Cargo.toml` eingetragen werden. Einfache
  Byte-Identity-Meldung (`N bytes differ`) ist ausreichend für diesen AC.
- `stdout` bleibt leer (alle Diagnose-Ausgaben gehen nach `stderr`)

### AC-D — In-Sync-Stand → Exit-Code 0

**Given** `shells/windows/src/bindings/index.ts` ist byte-identisch mit dem aktuellen
`generate-bindings`-Output (d. h. keine Core-API-Änderung seit dem letzten Commit der
Datei)
**When** `cargo xtask bindings-drift` ausgeführt wird
**Then**

- Der Prozess terminiert mit Exit-Code 0
- `stderr` enthält keine Fehler- oder Warnmeldungen
- Optional ist eine knappe Bestätigung nach `stdout` erlaubt
  (z. B. `bindings up to date`), aber nicht verpflichtend

### AC-E — Format-Tolerance-Entscheidung: Byte-Identity

**Given** `generate-bindings` ruft `export-bindings`-Binary auf, das tauri-specta
deterministisch aufruft; der Output enthält keinen timestamps o. ä. non-deterministischen
Content
**When** Story 5.2 die Vergleichs-Strategie festlegt
**Then**

- Der Vergleich ist **byte-identisch** (String-Equality nach `render()` vs.
  `std::fs::read_to_string()` des committed File)
- Kein `prettier`- oder Whitespace-Normalisierungs-Schritt wird eingeführt; ein
  Reformat-Drift (z. B. durch `prettier`-Run ohne Regen) wäre selbst ein Bug und soll
  gefangen werden
- Diese Entscheidung ist im Code als `// byte-identity: see Story 5.2 AC-E`-Kommentar
  dokumentiert

### AC-F — `--fix`-Flag explizit ausgeschlossen (Backlog-Eintrag)

**Given** ein `--fix`-Flag (Auto-Rewrite des committed File) würde Dev-Ergonomics
verbessern
**When** Story 5.2 den Scope abgrenzt
**Then**

- `bindings-drift` akzeptiert kein `--fix`-Flag in dieser Story (Read-Only-Check)
- Ein Backlog-Eintrag in `docs/backlog.md` dokumentiert `--fix` als Phase-2-Optional:
  `bindings-drift --fix: auto-call generate-bindings + overwrite committed file (Phase-2 Dev-Ergonomics)`

### AC-G — Forcing-Sentinel-Test

**Given** Memory `feedback_ci_gate_philosophy.md` fordert Forcing-Sentinels für jeden
CI-Gate
**When** Story 5.2 den Test einführt
**Then**

- `xtask/tests/bindings_drift_test.rs` (oder `#[cfg(test)]`-Modul in
  `xtask/src/bindings_drift.rs`, Delegate-Choice) enthält mindestens zwei Tests:
  - **Test `in_sync_returns_ok`**: Ruft intern `render()` auf, vergleicht gegen
    den frischen Output — erwartet keine Differenz (testet die Basis-Logik)
  - **Test `artificial_drift_detected`**: Erzeugt eine Temp-Datei mit absichtlich
    modifiziertem Inhalt (z. B. zusätzliche Kommentarzeile), vergleicht gegen
    `render()`-Output — erwartet, dass die Vergleichs-Logik Drift meldet
    (d. h. Strings sind nicht gleich); RAII-Guard für Temp-File gemäss Memory
    `feedback_test_raii_cleanup_pattern.md`
- Die Tests laufen via `cargo test -p xtask` und blockieren CI bei Regressionen
- Test-Kommentar: `// Forcing sentinel: proves bindings-drift catches artificial drift
  (CI-gate-philosophy)`

### AC-H — CI-Lauffähigkeit

**Given** Memory `feedback_ci_gate_philosophy.md` erfordert, dass Gates fail-loud in CI
laufen
**When** `bindings-drift` in CI ausgeführt wird (z. B. in einem `bindings-drift`-Step)
**Then**

- Der Subcommand benötigt kein GUI / OS-Keystore / Audio-Hardware; er liest nur
  Dateien und ruft `cargo run --package klarvo-windows-shell --bin export-bindings` auf
- Die Dependency auf das `export-bindings`-Binary ist dokumentiert (Technical Notes):
  CI muss `klarvo-windows-shell` bauen können; cross-compile-Einschränkungen gelten
  (Windows-only-Crate, Linux-CI-Runner müsste cross-compile-Setup haben)
- Die Story liefert keinen CI-Workflow-YAML selbst (Delegate-Choice ob neues `.yml`
  oder Erweiterung des bestehenden `ci-bindings-drift.yml`); das ist Impl-Detail des
  Dev-Agents

## Technical Notes

### `generate_bindings::run()` Architektur vor und nach dem Refactor

Aktueller Stand (`xtask/src/generate_bindings.rs`): `run()` ruft
`cargo run --package klarvo-windows-shell --bin export-bindings` als Child-Process auf.
Der `export-bindings`-Binary schreibt direkt nach `shells/windows/src/bindings/index.ts`.

**Kernproblem für Drift-Check:** `run()` hat keinen Rückgabewert für den generierten
Content; der Output landet im Dateisystem, nicht im Speicher.

**Empfohlener Refactor-Ansatz:**
Der sauberste Ansatz ohne Breaking-Change am `export-bindings`-Binary ist eine
**Temp-File-Strategie** innerhalb von `xtask`:
1. `render()` ruft `export-bindings` mit einem alternativen Output-Pfad auf
   (z. B. via Env-Var oder CLI-Arg, falls das Binary das unterstützt),
   oder schreibt in eine temp-Datei und liest sie zurück
2. Falls `export-bindings` keinen konfigurierbaren Output-Pfad hat: `render()` ruft
   das Binary auf, liest den Schreib-Output (`shells/windows/src/bindings/index.ts`)
   zurück als String, und gibt diesen zurück — dann macht `write_to_disk()` nichts Neues
3. Alternative: `export-bindings` nach Tempfile kopieren, Vergleich, Restore. Das
   ist fragile; Temp-File-Ansatz ist robuster.

**Delegate-Decision:** Der Dev-Agent wählt den saubersten Ansatz basierend auf der
tatsächlichen Architektur des `export-bindings`-Binaries. Die Story-AC fordert nur die
funktionale Separation, nicht den exakten Mechanismus.

### Cross-Reference: ADR-0002 (tauri-specta rc.24)

ADR-0002 ist Load-Bearing für `generate-bindings` (Event-Name-Policy, Struct-Ident
etc.). Story 5.2 berührt ADR-0002 nicht; die Story ist format-agnostisch gegenüber
dem TS-Content. Event-Name-Policy und `#[tauri_specta(event_name)]`-Enforcement
sind Domain von Story 5.3 (FR34/G1-Lint).

### `similar`-Crate nicht im Workspace

`similar` ist weder in `xtask/Cargo.toml` noch im Workspace-Lock vorhanden.
Falls der Dev-Agent Diff-Output einbauen möchte (Bonus für Developer-DX), muss
`similar = "2"` als dev-dependency in `xtask/Cargo.toml` eingetragen werden.
Byte-Identity-Fehlermeldung (`committed: N bytes, generated: M bytes`) ist
als Minimalimplementation vollständig AC-konform.

### Deterministismus-Annahme

`generate-bindings` ruft `export-bindings` auf, das tauri-specta-rc.24 delegiert.
tauri-specta schreibt einen deterministischen Header-Kommentar
(`// This file has been generated by Tauri Specta. Do not edit this file manually.`)
ohne Timestamps. Der Output ist deterministisch genug für byte-identity-Vergleich.
Sollte sich das empirisch als falsch herausstellen (z. B. Hashmap-Ordering im
generierten Code), ist eine Sortier-Normalisierung als Story-Erweiterung zu behandeln,
nicht als Pre-emptive-Komplexität.

### `export-bindings`-Binary Verfügbarkeit in CI

`bindings-drift` hat dieselbe Build-Dependency wie `generate-bindings`:
das `export-bindings`-Binary in `klarvo-windows-shell`. Auf einem Linux-CI-Runner
erfordert das cross-compile-Setup für Windows; das bestehende `ci-bindings-drift.yml`
(falls vorhanden) löst das bereits. Der Dev-Agent prüft den Workflow-Stand und
ergänzt ggf.

## Dependencies

- Story 3.1 done — `shells/windows/src/bindings/index.ts` committed und als Baseline
  vorhanden (`memory/project_phase1_complete.md`)
- `xtask/src/generate_bindings.rs` — Load-Bearing Reuse-Target; Refactor (AC-A) ist
  Voraussetzung für `bindings-drift`-Implementierung
- `memory/feedback_ci_gate_philosophy.md` — Preventive Enforcement + Forcing-Sentinel
  (AC-G) + Skip-by-Design; Stub-Checks verboten
- `memory/feedback_commit_hygiene.md` — kein `git add .`; Contract-before-Implementation
- `memory/reference_tauri_specta_rc24_event_name.md` — Cross-Reference (ADR-0002);
  Event-Name-Policy ist FR34/G1-Domain (Story 5.3), nicht 5.2

## Tasks/Subtasks

- [ ] Task 1 — Refactor `generate_bindings.rs`: Render/Write trennen (AC-A)
  - [ ] 1.1 Architektur von `export-bindings`-Binary prüfen (unterstützt es
        konfigurierbaren Output-Pfad oder Stdout?)
  - [ ] 1.2 `pub fn render() -> Result<String, ...>` implementieren
  - [ ] 1.3 `fn write_to_disk(content: &str) -> ExitCode` implementieren
  - [ ] 1.4 `generate_bindings::run()` auf `render()` + `write_to_disk()` umstellen
  - [ ] 1.5 `cargo xtask generate-bindings` verifizieren (Output byte-identisch)
- [ ] Task 2 — `xtask/src/bindings_drift.rs` erstellen (AC-C / AC-D / AC-E)
  - [ ] 2.1 `pub fn run() -> ExitCode` implementieren
  - [ ] 2.2 `render()` aufrufen und Ergebnis gegen committed File vergleichen
        (byte-identity, Kommentar `// byte-identity: see Story 5.2 AC-E`)
  - [ ] 2.3 Drift-Pfad: `stderr`-Output mit Datei-Pfad + Handlungsaufforderung +
        Exit-Code 1
  - [ ] 2.4 In-Sync-Pfad: Exit-Code 0
- [ ] Task 3 — `main.rs` ergänzen (AC-B)
  - [ ] 3.1 `mod bindings_drift;` hinzufügen
  - [ ] 3.2 Match-Arm `Some("bindings-drift") => bindings_drift::run()` eintragen
  - [ ] 3.3 `print_help()` um `bindings-drift`-Zeile unter aktiven Subcommands ergänzen
  - [ ] 3.4 `cargo xtask --help` manuell verifizieren
- [ ] Task 4 — Forcing-Sentinel-Test (AC-G)
  - [ ] 4.1 `in_sync_returns_ok`-Test implementieren
  - [ ] 4.2 `artificial_drift_detected`-Test mit Temp-File + RAII-Guard implementieren
  - [ ] 4.3 `cargo test -p xtask` grün verifizieren
- [ ] Task 5 — Backlog-Eintrag für `--fix`-Flag (AC-F)
  - [ ] 5.1 `docs/backlog.md` um Phase-2-Eintrag ergänzen
- [ ] Task 6 — Abschluss-Verifizierung
  - [ ] 6.1 `cargo xtask bindings-drift` auf In-Sync-Stand → Exit-Code 0
  - [ ] 6.2 `cargo xtask bindings-drift` mit manuell modifiziertem
        `shells/windows/src/bindings/index.ts` → Exit-Code 1 + korrekte `stderr`-Ausgabe
  - [ ] 6.3 Modify + Restore von `bindings/index.ts` im Test-Flow dokumentieren
        (kein dauerhafter Schaden am committed File)

## Dev Agent Record

### Completion Notes

<!-- leer — Status: draft -->

### Story-Spec-Abweichungen

<!-- leer — Status: draft -->

## File List

<!-- leer — Status: draft -->

## Change Log

<!-- leer — Status: draft -->
