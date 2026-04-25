---
name: Story 5.3 — `cargo xtask lint-events` G3-Erweiterung (FR34)
epic: 5
story_number: "5.3"
status: review
dependencies:
  - "4.4"
  - "1A.4"
  - "1B.1"
---

# Story 5.3: `cargo xtask lint-events` G3-Erweiterung

## Outcome

`cargo xtask lint-events` wird von einem Single-Pass-G1-Prüfer (Tauri-Event-Naming-Policy)
zu einem vollständigen G3-Lint-Gate mit drei Sub-Pässen erweitert:

1. **G1-Sub-Lint (bestehend, unverändert):** Alle `tauri_specta::Event`-Derives müssen
   ein explizites `#[tauri_specta(event_name = "...")]`-Attribut mit Dot-Notation tragen.
2. **G3-Sub-Lint A — User-Facing-String-Detection:** `klarvo-core` und
   `klarvo-plugins/*` werden per AST-Parse gescannt. Jeder String in einer
   `AppError { user_message: Some("...") }`-Position oder als Event-Field-Default muss
   ein valider i18n-Key gemäß `klarvo_core::i18n::KEY_REGEX` sein. Klartextstrings
   (z.B. `"Network error"`) failen laut.
3. **G3-Sub-Lint B — Locale-Cross-Validation:** Der Lint sammelt alle im Code gefundenen
   i18n-Keys und validiert sie bidirektional gegen `shells/windows/locales/en.json` und
   `shells/windows/locales/de.json`. Drift zwischen Code-Emit-Sites und Locale-Tables
   sowie Asymmetrien zwischen den Locale-Tables failen laut.
4. **G3-Sub-Lint C — Wildcard-Match-Detection:** `klarvo-core`-Source wird nach
   `match <expr> { ... _ => ... }`-Patterns gescannt; liegt ein `_`-Catch-All-Arm in
   einem Match auf einen `PipelineStageType`-Wert vor, schlägt der Lint fehl.

Nach Story 5.3 ersetzt G3-Sub-Lint B die manuelle `REQUIRED_KEYS`-Konstante aus Story 4.4
mechanisch. Die Konstante wird als obsolet dokumentiert; ihr eigentliches Entfernen ist
Scope einer separaten Phase-2-Cleanup-Story.

## Acceptance Criteria

### AC-A — G1-Sub-Lint bleibt unverändert erhalten

**Given** `xtask/src/lint_events.rs` implementiert den G1-Sub-Lint (Event-Name-Dot-Notation,
~290 Zeilen, Validierung-Pass G1) seit Phase-0
**When** Story 5.3 die drei neuen Sub-Pässe ergänzt
**Then**

- Der G1-Sub-Lint (`run_g1_event_name_check` o.ä. intern) bleibt als eigenständige
  Sub-Routine erhalten und wird weiterhin bei jedem `cargo xtask lint-events`-Aufruf
  ausgeführt
- Alle bestehenden G1-Tests in `xtask/src/lint_events.rs` bleiben grün; kein Refactor
  bricht bestehende Fixture-Assertions
- ADR-0002 + Amendment 1 (`reference_tauri_specta_rc24_event_name.md`) sind weiterhin
  die alleinige Policy-Quelle für G1; kein Duplikat der Naming-Regel in den neuen
  Sub-Pässen
- Exit-Code: `0` wenn alle Sub-Pässe pass, `1` wenn mindestens ein Sub-Pass mindestens
  eine Violation meldet

### AC-B — G3-Sub-Lint A: User-Facing-String-Detection

**Given** `klarvo-core` und `klarvo-plugins/*` dürfen per G3-Kontrakt
(`project_i18n_core_contract.md`) keine Klartext-User-Strings emittieren — ausschließlich
validierte i18n-Keys
**When** G3-Sub-Lint A eine Datei aus `klarvo-core/src/` oder `klarvo-plugins/*/src/`
per `syn`-AST-Parse scannt
**Then**

- **Positive Case (soll flaggen):** Findet der Lint einen String-Literal in einer der
  folgenden Positionen, der **nicht** dem Regex aus AC-C entspricht, wird eine Violation
  emittiert:
  - `AppError { user_message: Some(<string-literal>) }`-Struct-Init-Position
  - Struct-Field-Default-Attribute (`#[default = "..."]`) in Event-Structs
  - Weitere direkte `user_message`-Zuweisungen via Literal (z.B. in
    `let e = AppError { user_message: Some("Network error".into()), .. }`)
- **Negative Case (darf nicht flaggen):**
  - `user_message: None`-Felder
  - Strings in Test-Modulen (`#[cfg(test)]`-Blocks) — diese sind explizit ausgeschlossen
  - Strings in Kommentaren und `//!`-Doc-Kommentaren
  - Strings in `shells/`-Code (G1 ist dort Owner; kein Scope-Overlap)
- **Fixture-Anforderungen:** Das Lint-Modul enthält mindestens:
  - Eine Positive-Fixture-Funktion (Klartextstring in `user_message`-Position → Violation
    erwartet)
  - Eine Negative-Fixture-Funktion (valider i18n-Key in `user_message`-Position → kein
    Fehler)
- **Plugin-Scope-Begründung (in AC-Kommentar dokumentiert):** G3-Sub-Lint A scannt
  `klarvo-core/` UND `klarvo-plugins/*`, weil Plugin-Emit-Sites den Plugin-Boundary
  User-facing verlassen (Groq-STT-Errors, KeyMissing-Errors); der G3-Kontrakt gilt für
  den gesamten Nicht-Shell-Scope (FR30 + FR29)

### AC-C — Key-Format-Regex: Single-Source-Import aus `klarvo_core::i18n`

**Given** `klarvo_core::i18n::KEY_REGEX` ist seit Story 1A.4 die authoritative
Einzel-Quelle des Schlüsselformat-Regex
(`^[a-z][a-z0-9_]*(\.[a-z0-9_]+)+$`, explizit `pub const` in
`klarvo-core/src/i18n.rs:46`)
**When** der Lint einen String auf Key-Format prüft
**Then**

- `xtask/src/lint_events.rs` importiert `klarvo_core::i18n::KEY_REGEX` direkt als
  `extern crate`-Referenz (oder via `klarvo-core`-Dependency im `xtask/Cargo.toml`) —
  der Regex wird **nicht** im Lint-Modul dupliziert
- Kein eigener `const KEY_PATTERN: &str`-Block oder Inline-Regex-Literal im Lint-Code
  für die Key-Validierung; ausschließlich der Import der Core-Konstante
- Die Story-Doc (Technical Notes) dokumentiert das Regex-Pattern explizit als
  Referenz-Snapshot für Reviews (per `feedback_reference_block_discipline.md` — auch
  triviale Defaults müssen explizit stehen):
  ```
  KEY_REGEX = r"^[a-z][a-z0-9_]*(\.[a-z0-9_]+)+$"
  ```
  Dieses Pattern erlaubt: ASCII-Lowercase, Ziffern, Unterstriche in Segmenten;
  Dot-Notation obligatorisch (mind. ein `.`); Erster Buchstabe muss `[a-z]` sein;
  kein Kebab-Case (Bindestriche verboten)
- `cargo test -p xtask` bleibt grün nach Einführung der Core-Dependency

### AC-D — G3-Sub-Lint B: Locale-Cross-Validation

**Given** Story 4.4 hat eine manuelle `REQUIRED_KEYS`-Konstante in
`shells/windows/src-tauri/src/i18n.rs` als Übergangslösung eingeführt; deren
bekannte Schwäche ist im Backlog dokumentiert (Backlog-Eintrag aus Story 4.4 AC-G)
**When** G3-Sub-Lint B ausgeführt wird
**Then**

- **Schritt 1 — Key-Inventar aus Code:** Der Lint sammelt alle i18n-Keys, die in
  `klarvo-core/` und `klarvo-plugins/*` (gleiches Scope-Profil wie AC-B) als valide
  Schlüssel in `user_message`-Positionen vorkommen, in eine `BTreeSet<String>`
- **Schritt 2 — Forward-Drift-Check (Code → en.json):** Für jeden gesammelten
  Code-Key wird geprüft, ob er in `shells/windows/locales/en.json` als Top-Level-Eintrag
  existiert. Fehlende Keys erzeugen eine Violation:
  ```
  VIOLATION [locale-drift]: key "error.stt.network" emitted in
  klarvo-plugins/klarvo-plugin-groq/src/lib.rs:42 but absent from en.json
  ```
- **Schritt 3 — Symmetrie-Check (en.json ↔ de.json):** Die Key-Sets beider Locale-Tables
  werden verglichen. Asymmetrie (Key in en, nicht in de — oder umgekehrt) erzeugt eine
  Violation:
  ```
  VIOLATION [locale-asymmetry]: key "error.audio.unsupported_format" present in
  en.json but absent from de.json
  ```
- **Nicht geprüft:** Vollständige Coverage aller Shell-Emit-Sites (die liegen im
  `shells/`-Scope, außerhalb des G3-Scan-Bereichs) — Shell-seitige Coverage ist Aufgabe
  des manuellen Tests aus Story 4.4 AC-F, bis eine gesonderte Shell-G3-Extension
  eingeführt wird
- **Keine Whitelist-Check:** G3-Sub-Lint B prüft keinen „Kein-verwaister-Key"-Richtung
  (en.json enthält Keys, die der Code nicht kennt) — das ist Aufgabe des manuellen
  AC-F-Test 3 aus Story 4.4. Diese Nicht-Abdeckung muss im Technical-Notes dokumentiert
  sein
- **Fixture:** Mindestens ein Forcing-Sentinel-Fixture (`test_locale_drift_detected` o.ä.)
  der manuell eine Locale-Datei mit einem fehlenden Key simuliert und die Violation
  assertiert

### AC-E — G3-Sub-Lint C: Wildcard-Match-Detection auf `PipelineStageType`

**Given** Story 1B.1 dokumentiert (`klarvo-core/src/pipeline/stage.rs` Doc-Kommentar
und Enum-Doc) dass Consumers matching on `PipelineStageType` **keine** `_`-Wildcard-Arme
verwenden dürfen — exhaustive match ist verpflichtend, damit neue Variants eine
Compile-Error erzwingen
**When** G3-Sub-Lint C `klarvo-core/src/` scannt
**Then**

- **Heuristischer Ansatz (explizit dokumentiert):** Der Lint nutzt heuristisches Greppen
  auf AST-Ebene, nicht präzise Type-Resolution. Er sucht nach `syn::ExprMatch`-Knoten
  mit einem `_`-Arm, bei denen der Match-Scrutinee einen `PipelineStageType`-Ident
  (exakter String-Match auf den Typ-Namen in der Ausdrucks-Annotation oder als
  Varianten-Qualifizierer) enthält. Diese Heuristik ist für Phase-1 hinreichend; ein
  Kommentar im Code dokumentiert die Einschränkung: false-positives bei anderen Typen
  mit gleichem Ident-Namen sind theoretisch möglich, im aktuellen Workspace nicht
  vorhanden
- **Positive Case (soll flaggen):** Ein `match`-Block, der einen `PipelineStageType`-Wert
  matcht und einen `_ => ...`-Catch-All-Arm enthält, erzeugt eine Violation:
  ```
  VIOLATION [wildcard-match]: match on PipelineStageType at
  klarvo-core/src/pipeline/executor.rs:87 has a wildcard arm (`_`); use exhaustive match
  ```
- **Negative Cases (dürfen nicht flaggen):**
  - `match`-Blöcke auf anderen Enum-Typen mit `_`-Arm — nur `PipelineStageType`-Matches
    sind im Scope
  - `match`-Blöcke in Test-Modulen (`#[cfg(test)]`-Blocks)
  - Exhaustive `match`-Blöcke auf `PipelineStageType` ohne `_`-Arm
- **Fixture:** Mindestens ein Positive-Fixture (`test_wildcard_match_flagged`) und ein
  Negative-Fixture (`test_other_enum_wildcard_not_flagged`) als Inline-Tests im
  Lint-Modul

### AC-F — CI-Integration: Forcing-Sentinels und Fail-Loud

**Given** `feedback_ci_gate_philosophy.md` fordert Preventive-Enforcement +
Forcing-Sentinel + Skip-by-Design; Stub-Checks sind verboten
**When** `cargo xtask lint-events` im CI ausgeführt wird
**Then**

- Jeder der drei neuen Sub-Pässe (A/B/C) hat mindestens einen dedizierten
  Forcing-Sentinel: eine Fixture-Datei oder ein Inline-Test-Fall der explizit
  eine Violation erzeugt und den Exit-Code `1` assertiert. Fixtures dürfen nicht
  durch bedingte Compilation geskippt werden (`#[cfg(test)]`-Abschirmung der
  Fixture-Daten ist erlaubt; der Test-Assert selbst muss laufen)
- Alle Sub-Pässe laufen sequenziell in `run()`; Violations aus allen drei Pässen
  werden gesammelt und nach dem letzten Pass gemeinsam ausgegeben (kein early-exit
  nach erstem Sub-Pass-Failure, damit alle Violations auf einmal sichtbar sind)
- Der Exit-Code ist `1` wenn mindestens eine Violation über alle Pässe vorhanden ist,
  andernfalls `0`
- `cargo xtask lint-events` läuft in unter 30 Sekunden auf dem lokalen Dev-Rechner
  (kein Blocking-IO-Bottleneck durch naive vollständige Re-Parse jeder Datei; eine
  einzelne `walkdir`-Traversal genügt)
- Das CI-Skript (`.github/workflows/` o.ä.) bleibt unverändert, sofern
  `cargo xtask lint-events` bereits im CI-Gate-Step enthalten ist; andernfalls
  Hinweis in Technical Notes

### AC-G — Backlog-Closure: `REQUIRED_KEYS`-Konstante als obsolet markiert

**Given** Story 4.4 AC-G hat einen Backlog-Eintrag in `docs/backlog.md` angelegt:
„Story 4.4 manueller Coverage-Test durch Epic 5 FR34 Lint-Gate ersetzen, sobald G3
ausgerollt ist"; die `REQUIRED_KEYS`-Konstante in
`shells/windows/src-tauri/src/i18n.rs` ist manuell gewartet und driftet bei jedem
neuen Key-Emit ohne G3-Mechanismus
**When** Story 5.3 implementiert und grün ist
**Then**

- Der Backlog-Eintrag aus Story 4.4 AC-G in `docs/backlog.md` wird als geschlossen
  markiert (z.B. `[CLOSED 5.3]`-Suffix) — der G3-Sub-Lint B übernimmt die
  mechanische Key-Drift-Prüfung
- Die `REQUIRED_KEYS`-Konstante in `shells/windows/src-tauri/src/i18n.rs` wird
  **nicht** in Story 5.3 entfernt — Scope-Drift; der zugehörige manuelle Test
  (Story 4.4 AC-F) bleibt bis zur Ablösung aktiv
- Story 5.3 fügt einen Kommentar über der `REQUIRED_KEYS`-Konstante ein:
  ```rust
  // NOTE(5.3): G3-Sub-Lint B (cargo xtask lint-events) prüft seit Story 5.3
  // mechanisch, ob Code-Emit-Sites in klarvo-core + klarvo-plugins in en.json
  // registriert sind. Diese Konstante und die zugehörigen Tests sind eine
  // überlappende manuelle Absicherung; Entfernung in einer Phase-2-Cleanup-Story.
  ```
- In `docs/backlog.md` wird ein neuer Eintrag angelegt:
  `[Phase-2-Cleanup] REQUIRED_KEYS-Konstante und manuellen i18n-Coverage-Test aus
  shells/windows/src-tauri/src/i18n.rs entfernen; durch G3-Sub-Lint-B-Ausgabe ersetzt
  (Story 5.3). Entfernung erst nach Verifikation, dass G3-Sub-Lint alle Shell-Emit-Sites
  mitabdeckt oder ein Shell-G3-Pass eingeführt ist.`
- **Scope-Klarheit:** Das Entfernen des manuellen Tests ist ausdrücklich **nicht** Teil
  von Story 5.3 und wird als Phase-2-Cleanup-Story empfohlen (statt als Follow-Up-Subtask
  in 5.3), um Scope-Drift zu vermeiden

## Technical Notes

### Key-Format-Regex (Reference-Snapshot)

Per `feedback_reference_block_discipline.md` wird das Pattern hier explizit dokumentiert,
auch wenn es über `klarvo_core::i18n::KEY_REGEX` importiert wird:

```
KEY_REGEX = r"^[a-z][a-z0-9_]*(\.[a-z0-9_]+)+$"
```

Erlaubte Zeichen pro Segment: `[a-z0-9_]`. Erstes Zeichen: `[a-z]`.
Obligatorisch: mind. ein `.` (Dot-Notation). Verboten: Großbuchstaben, Bindestriche,
führende Ziffern oder Unterstriche, leere Segmente.

Dieses Pattern ist identisch mit `klarvo-core/src/i18n.rs:46 KEY_REGEX` — kein
Duplikat im Lint-Code.

### `klarvo-core`-Dependency in `xtask/Cargo.toml`

`xtask` muss `klarvo-core` als Dependency aufnehmen, um `KEY_REGEX` zu importieren.
Da `xtask` ein Workspace-Member ist, genügt ein `path`-Eintrag:

```toml
[dependencies]
klarvo-core = { path = "../klarvo-core" }
```

Achtung: `klarvo-core` hat Feature-Gates. Der Import von `klarvo_core::i18n::KEY_REGEX`
ist `default`-Feature-unabhängig (die `i18n`-Mod ist immer aktiv); kein extra
Feature-Flag nötig.

### Heuristischer Ansatz für G3-Sub-Lint C (Wildcard-Match-Detection)

Präzise Type-Resolution würde eine vollständige Semantic-Analyse (z.B. via
`rust-analyzer`-Bibliothek) erfordern — das ist für Phase-1 overkill. Die Heuristik:

1. Parse die `.rs`-Datei per `syn::parse_file`.
2. Traversiere alle `syn::ExprMatch`-Knoten.
3. Prüfe, ob der Scrutinee-Ausdruck den Ident-String `PipelineStageType` enthält
   (über `quote::ToTokens` oder rekursives Token-Walk).
4. Falls ja: prüfe ob einer der Match-Arme ein `syn::Pat::Wild`-Pattern (`_`) ist.
5. Falls ja: Violation.

Einschränkung (im Lint-Code-Kommentar zu dokumentieren): Ein anderer Typ mit dem Ident
`PipelineStageType` im gleichen Workspace würde fälschlicherweise geflaggt. Im
aktuellen Workspace (Phase-1) ist dieser Typ einmalig in
`klarvo-core/src/pipeline/stage.rs`. Kein ADR nötig; die Heuristik ist ausreichend
und die Einschränkung ist dokumentiert.

### G3-Sub-Lint B: Nicht abgedeckter Bereich (Shell-Emit-Sites)

G3-Sub-Lint B prüft Code-→-Locale-Drift nur für `klarvo-core/` und `klarvo-plugins/*`.
Shell-Emit-Sites (`shells/windows/src-tauri/src/*.rs`) liegen außerhalb des Scan-Scopes,
weil:

1. G3-Kontrakt (`project_i18n_core_contract.md`) definiert den Scope als
   Core + Plugins (Shell ist der Resolver, nicht der Emitter-Gate).
2. Shell-Emit-Sites werden durch den manuellen Test aus Story 4.4 AC-F
   (`REQUIRED_KEYS`) weiter abgesichert.

Diese Lücke ist bekannt und explizit akzeptiert für Phase-1. Eine zukünftige
Shell-G3-Extension würde den gleichen Sub-Lint-B-Mechanismus auf `shells/`-Code
ausweiten.

### `upstream_5xx` / `upstream_4xx` vs. `upstream_unavailable` (Story-4.4-Abweichung)

Story 4.4 Dev Agent Record dokumentiert: der tatsächliche Code in
`klarvo-plugin-groq/src/lib.rs` nutzt `error.stt.upstream_5xx` und
`error.stt.upstream_4xx` statt `error.stt.upstream_unavailable`. G3-Sub-Lint A/B
folgt dem Code als Source-of-Truth — der Lint-Scan extrahiert die tatsächlichen
String-Literals aus dem Code, nicht einer Spec-Liste. Keine manuelle Anpassung nötig.

### Modul-Struktur in `xtask/src/lint_events.rs`

Empfohlene interne Aufteilung:

```
pub fn run() -> ExitCode          // Orchestriert alle Sub-Pässe, sammelt Violations
fn run_g1_event_name_check(...)   // Bestand, unverändert
fn run_g3a_user_string_check(...) // Neu: User-Facing-String-Detection
fn run_g3b_locale_cross_check(...) // Neu: Locale-Cross-Validation
fn run_g3c_wildcard_match_check(...) // Neu: Wildcard-Match-Detection
```

Alle vier Sub-Funktionen geben `Vec<String>`-Violations zurück; `run()` aggregiert
und entscheidet den Exit-Code.

## Dependencies

- Story 4.4 (implementiert und committed; `REQUIRED_KEYS`-Stand + Backlog-Eintrag als
  Greenfield-Basis für G3-Sub-Lint-B-Ablösung — Memory `project_epic4_complete.md`)
- Story 1A.4 (implementiert: `klarvo_core::i18n::KEY_REGEX` als `pub const` in
  `klarvo-core/src/i18n.rs:46` — Single-Source für Key-Format-Regex)
- Story 1B.1 (implementiert: `PipelineStageType`-Enum in
  `klarvo-core/src/pipeline/stage.rs` mit explizitem No-Wildcard-Mandat in
  Doc-Kommentar — Target für G3-Sub-Lint-C)
- Keine Inter-Story-Deps zu 5.1, 5.2, 5.4 (parallele Stories in Epic 5)
- Memory `project_i18n_core_contract.md` — Core/Shell-Separation, definiert
  G3-Sub-Lint-A/B-Scope
- Memory `feedback_ci_gate_philosophy.md` — Preventive-Enforcement +
  Forcing-Sentinel-Pflicht (AC-F)
- Memory `feedback_reference_block_discipline.md` — Key-Format-Regex explizit
  dokumentieren (AC-C + Technical Notes)
- Memory `feedback_backlog_discipline.md` — Backlog-Closure für 4.4-Eintrag +
  neuer Phase-2-Cleanup-Eintrag (AC-G)
- Memory `reference_tauri_specta_rc24_event_name.md` — G1-Policy (ADR-0002 Amendment 1),
  Cross-Reference für AC-A

## Tasks/Subtasks

- [x] Task 1 — `klarvo-core`-Dependency in `xtask/Cargo.toml` hinzufügen (AC-C)
  - [x] 1.1 `klarvo-core = { path = "../klarvo-core" }` in `[dependencies]` eintragen
  - [x] 1.2 `cargo check -p xtask` verifizieren
- [x] Task 2 — G3-Sub-Lint A implementieren: User-Facing-String-Detection (AC-B)
  - [x] 2.1 `run_g3a_user_string_check()`-Funktion in `xtask/src/lint_events.rs`
  - [x] 2.2 AST-Traversal für `AppError { user_message: Some(<literal>) }`-Pattern
  - [x] 2.3 Key-Validierung via Import `klarvo_core::i18n::KEY_REGEX`
  - [x] 2.4 Test-Block-Exclusion (`#[cfg(test)]`-Scoping)
  - [x] 2.5 Positive-Fixture + Negative-Fixture als Inline-Tests
- [x] Task 3 — G3-Sub-Lint B implementieren: Locale-Cross-Validation (AC-D)
  - [x] 3.1 `run_g3b_locale_cross_check()`-Funktion
  - [x] 3.2 Key-Inventar aus Code-Scan (Ergebnis von Sub-Lint A) aggregieren
  - [x] 3.3 `en.json`-Parse + Forward-Drift-Check (Code-Keys ⊆ en.json-Keys)
  - [x] 3.4 Symmetrie-Check `en.json`-Keys == `de.json`-Keys (als `BTreeSet<String>`)
  - [x] 3.5 Forcing-Sentinel-Fixture für Locale-Drift-Detection
- [x] Task 4 — G3-Sub-Lint C implementieren: Wildcard-Match-Detection (AC-E)
  - [x] 4.1 `run_g3c_wildcard_match_check()`-Funktion
  - [x] 4.2 AST-Traversal für `syn::ExprMatch` mit `PipelineStageType`-Heuristik
  - [x] 4.3 `_`-Arm-Detection + Violation-Emit
  - [x] 4.4 Positive-Fixture (`PipelineStageType`-Match mit `_`) + Negative-Fixture
        (anderer Enum-Typ mit `_`)
- [x] Task 5 — `run()`-Orchestrierung anpassen (AC-F)
  - [x] 5.1 Alle vier Sub-Pässe in `run()` sequenziell aufrufen
  - [x] 5.2 Violations aggregieren (kein early-exit)
  - [x] 5.3 Exit-Code-Logik: `1` bei mindestens einer Violation
- [x] Task 6 — Backlog-Closure + Konstanten-Kommentar (AC-G)
  - [x] 6.1 `docs/backlog.md`: Story-4.4-Eintrag als `[CLOSED 5.3]` markieren
  - [x] 6.2 Kommentar über `REQUIRED_KEYS` in `shells/windows/src-tauri/src/i18n.rs`
        einfügen
  - [x] 6.3 `docs/backlog.md`: neuen Phase-2-Cleanup-Eintrag für
        REQUIRED_KEYS-Removal anlegen
- [x] Task 7 — Build + Tests verifizieren
  - [x] 7.1 `cargo test -p xtask` grün (alle G1 + G3-Fixtures)
  - [x] 7.2 `cargo xtask lint-events` auf aktuellem Workspace-Stand: Exit-Code `0`
  - [x] 7.3 `cargo build --workspace` grün nach `xtask/Cargo.toml`-Änderung

## Dev Agent Record

### Completion Notes

All three G3 sub-lints implemented in `xtask/src/lint_events.rs`:

- **G3-A** (`UserStringVisitor`): scans `user_message: Some(<literal>)` patterns and `mod keys { pub const }` blocks.
  `in_test_mod` flag suppresses test-module noise; `in_keys_mod` flag restricts key-constant collection to
  `mod keys {}` blocks only (prevents false positives from `v1_import` constants that match `KEY_REGEX` but
  are not i18n keys — e.g. `"com.klarvo.voice"`, `"config.json"`).
- **G3-B** (`run_g3b_locale_cross_check`): forward-drift (code_keys ⊆ en.json) + symmetry (en.json == de.json key-set).
- **G3-C** (`WildcardMatchVisitor`): detects `_` wildcard arms in `match` expressions on `PipelineStageType`.

14 total tests: G1 (6 unchanged), G3-A (3), G3-B (1 forcing-sentinel), G3-C (3), bindings (2).

**Fix applied**: initial `scan_g3a` test helper was missing `in_keys_mod: false` in initializer — compiler
error caught before first commit; fix applied immediately.

### Story-Spec-Abweichung

`upstream_5xx`/`upstream_4xx` vs `upstream_unavailable` discrepancy pre-documented in Story 4.4 Dev Agent
Record. G3 Sub-Lint A/B follows code as source-of-truth; no manual adjustment needed.

---

**AC-B — `in_keys_mod`-Restriction als rationale Filter-Heuristik (Amendment 2026-04-26)**

AC-B textual fordert die Sammlung *"alle gefundenen i18n-Keys"* als Code-Quelle für den G3-B Forward-Drift-Check. Die Implementation beschränkt die Const-Sammlung auf Konstanten innerhalb von `mod keys { … }`-Blöcken (inline-modules) — nicht jede `pub const … : &str = "…"` im Code wird gesammelt.

**Begründung:**
- KEY_REGEX `^[a-z][a-z0-9_]*(\.[a-z0-9_]+)+$` matched auch nicht-i18n-Konstanten wie `"com.klarvo.voice"` (Tauri-App-Identifier), `"config.json"` (Filename), `"shells.windows.locales"` (Path-Fragmente). Ohne Restriction würden diese als false-positives in `code_keys` landen und G3-B Forward-Drift-Violations für Pfade auslösen, die keine i18n-Keys sind.
- Die `mod keys { … }`-Convention ist in `klarvo-core` und `klarvo-plugins/*` etabliert (Precedent: `memory/project_keystore_trait_surface` 1C.1-pattern).

**Amended AC-B wording:** Die Const-Sammlung folgt der `mod keys { … }`-Convention. Konstanten außerhalb von `keys`-Modulen werden nicht erfasst, auch wenn sie `KEY_REGEX` matchen — diese Heuristik vermeidet false-positives auf Plugin-Identifier, Filenames und Path-Fragmente, die strukturell wie i18n-Keys aussehen.

**Folge-Patch P6 (Filename-Heuristik):** Initial wurde die Restriction ausschließlich über `visit_item_mod` mit `node.ident == "keys"` durchgesetzt. Das erfasste **nur inline** `mod keys { ... }`-Blöcke, nicht aber file-basierte `pub mod keys;`-Deklarationen, deren Inhalt in einer separaten `keys.rs` liegt. Klarvo nutzt 4 solcher Module in `klarvo-core/src/{audio,output,keystore,v1_import}/keys.rs`. P6 schließt die Lücke via Filename-Heuristik: Wenn die geparste Datei `keys.rs` heißt, wird `in_keys_mod` zu Beginn auf `true` gesetzt. Verifiziert über `g3a_file_based_keys_module_collected`-Test.

**Auslöser:** Code-Review 2026-04-26, Decision D2 (Variante 3 — Restriction behalten + Filename-Heuristik für file-basierte Module).

---

**AC-B Position 2 — `#[default = "..."]`-Attribute Implementation (D3I, 2026-04-26)**

AC-B listet drei positive-case-Positionen für G3-A; Position 2 (`#[default = "..."]` auf Struct-Fields/Enum-Variants) war initial nicht implementiert. D3I-Patch ergänzt `visit_field` + `visit_variant` in `UserStringVisitor` mit `extract_default_attribute_string`-Helper. Behandlung analog zu `user_message`-Position: `is_key`-Match → `code_keys`-Insert, sonst Violation. Verifiziert über vier neue Tests (`g3a_default_attr_*`).

**Auslöser:** Code-Review 2026-04-26, Decision D3 (Variante 2 — Implementation statt Spec-Streichung).

---

**P7 — File/Line-Origin in G3-B Violation-Messages (2026-04-26)**

AC-D Beispiel-Output zeigt file:line in der Violation-Message. Initial wurde nur der Key + en.json-Absence gelogged. P7 ergänzt die Origin-Info: `code_keys` ist jetzt `BTreeMap<String, (PathBuf, line)>`, G3-B Violation-Message zeigt `key "..." emitted in <repo-relative-path>:<line> but absent from en.json`. AC-D Wording bleibt unverändert.

## File List

- `xtask/src/lint_events.rs` — rewritten: added G3-A, G3-B, G3-C sub-lints; `in_keys_mod` false-positive fix; 14 tests
- `shells/windows/src-tauri/src/i18n.rs` — modified: NOTE(5.3) comment above `REQUIRED_KEYS` constant
- `docs/backlog.md` — modified: Story 4.4 entry `[CLOSED 5.3]`, new Phase-2-Cleanup + bindings-drift + Tauri-Bundle-Profile entries, Revision-Log update

## Change Log

- 2026-04-25: Story implemented. `cargo xtask lint-events` exits 0 (4 events scanned, no violations).
  `cargo test -p xtask` 26/26 green.
