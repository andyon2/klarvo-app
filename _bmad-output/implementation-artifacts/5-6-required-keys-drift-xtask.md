---
name: Story 5.6 — REQUIRED_KEYS-Drift-Detection xtask
epic: 5
story_number: "5.6"
status: ready-for-dev
dependencies:
  - "5-3-lint-events-g3-extension"
  - "4.4"
---

# Story 5.6: REQUIRED_KEYS-Drift-Detection xtask

Status: ready-for-dev

## Story

Als Core-Dev / Shell-Dev
möchte ich einen mechanischen xtask-Gate, der **Backward-Drift** zwischen Locale-Files (`en.json` / `de.json`) und Code-Emit-Sites (klarvo-core + klarvo-plugins + klarvo-shell-orchestrator + shells/windows/src-tauri) detektiert,
damit die manuelle `REQUIRED_KEYS`-Whitelist in `shells/windows/src-tauri/src/i18n.rs` obsolet wird und der A4-Drift-Pattern (neuer Locale-Key ohne Code-Emit-Site bzw. ohne Whitelist-Update) nicht mehr durchrutschen kann.

## Kontext und Motivation

Phase-2-A-Retro Reibungsstelle 3 (authoritative Quelle: `_bmad-output/implementation-artifacts/epic-phase-2-a-retro-2026-05-01.md` §Reibungsstellen):

> A4-P2-P11 fügte `error.unknown` in JSON, vergaß Konstante in `i18n.rs::REQUIRED_KEYS` → Test rot bis A8-Sub das nachzog. D2 musste 3 weitere fehlende Keys aus A4 nachpflegen. **G3-Lint catched anderes — Drift-Mechanismus ist nicht xtask-abgedeckt.**

### Lücke gegenüber Story 5.3 (G3-Sub-Lint B)

Story 5.3 implementiert in `xtask/src/lint_events.rs::run_g3b_locale_cross_check`:
- **Forward-Drift** (Code → en.json): Key emittiert in `klarvo-core` / `klarvo-plugins`, fehlt in en.json → Violation
- **Symmetrie** (en.json ↔ de.json): asymmetrische Key-Sets → Violation

Story 5.3 AC-D dokumentiert explizit als out-of-scope:

> **Keine Whitelist-Check:** G3-Sub-Lint B prüft keinen „Kein-verwaister-Key"-Richtung (en.json enthält Keys, die der Code nicht kennt) — das ist Aufgabe des manuellen AC-F-Test 3 aus Story 4.4. Diese Nicht-Abdeckung muss im Technical-Notes dokumentiert sein

Plus: G3-Sub-Lint B scannt nur `klarvo-core/src/` und `klarvo-plugins/*/src/`. **Shell-Code** (`klarvo-shell-orchestrator/src/session.rs:156,205,234,262,275,284`, `shells/windows/src-tauri/src/{paste,bridge}.rs`) emittiert ebenfalls i18n-Keys, ist aber im G3-B-Scope nicht enthalten (G1 ist dort Owner — aber G1 prüft Event-Naming, keine Key-Drift).

### Aktuelle Defense (manuell, brüchig)

`shells/windows/src-tauri/src/i18n.rs:72-116` enthält die `const REQUIRED_KEYS: &[&str]`-Whitelist mit ~37 Einträgen. Drei Tests darauf:
- `en_json_covers_all_required_keys` (en.json muss alle REQUIRED_KEYS enthalten)
- `de_json_covers_same_key_set` (en ↔ de Symmetrie — überlappt mit G3-B)
- `no_orphan_keys_in_en_json` (en.json darf keine Keys haben, die nicht in REQUIRED_KEYS stehen)

Schwächen:
- Whitelist wird manuell gepflegt — A4-P2-P11 hat genau das vergessen
- Tests laufen nur `cargo test -p klarvo-windows-shell` → Linux-CI ohne Cross-Compile catched es nicht
- Doppelmaintenance: REQUIRED_KEYS-Konstante + Code-Konstanten in `mod keys`-Blöcken

Story 5.3 Inline-Note (`shells/windows/src-tauri/src/i18n.rs:54-71`) verspricht Cleanup in einer „Phase-2-Cleanup-Story" — Story 5.6 ist diese Story.

## Acceptance Criteria

### AC-A — G3-Sub-Lint B Scope-Erweiterung um Shell-Code

**Given** `xtask/src/lint_events.rs::run_g3a_user_string_check` scannt aktuell nur `klarvo-core/src/` und `klarvo-plugins/*` (siehe `scan_roots`-Variable)
**When** Story 5.6 die Scope-Erweiterung umsetzt
**Then**

- Ein neues Sub-Pass-Modul (`run_g3d_orphan_check` o.ä. — siehe Implementation-Notes für Naming-Discussion) UND/ODER eine erweiterte Code-Key-Sammlung scannt zusätzlich:
  - `klarvo-shell-orchestrator/src/`
  - `shells/windows/src-tauri/src/`
- Bekannte Shell-Emit-Sites die durch die Erweiterung sichtbar werden müssen (Inventar aus Grep, **nicht abschließend**):
  - `klarvo-shell-orchestrator/src/session.rs:156` → `"error.audio.start_failed"`
  - `klarvo-shell-orchestrator/src/session.rs:205` → `"error.recording.timeout"`
  - `klarvo-shell-orchestrator/src/session.rs:234,262,275` → `"error.internal"` (`unwrap_or`-Fallback-Pattern)
  - `klarvo-shell-orchestrator/src/session.rs:284` → `"error.config.output_target_not_found"`
  - `shells/windows/src-tauri/src/paste.rs:74` → `"error.paste.send_input_failed"` (in `user_message: Some(...)`-Position)
- Test-Module (`#[cfg(test)]`) bleiben ausgeschlossen — gleiche Heuristik wie G3-A
- **Pattern-Erkennung erweitert** über reine `user_message: Some(...)`-Positionen hinaus:
  - `unwrap_or("<key>")`-Calls auf `Option<&str>` / `Option<String>` (Orchestrator-Pattern)
  - `emit_error("<key>", ...)`-Calls auf `ErrorEmitter`-Trait (ADR-0009-Pattern)
  - Begründung: Shell-Code emittiert Keys nicht durch Struct-Init, sondern durch Trait-Method-Call mit String-Literal als erstem Argument

### AC-B — Backward-Orphan-Check (Locale → Code)

**Given** en.json kann Keys enthalten, die kein Code-Emit-Site referenziert (Beispiel A4-P2-P11: `error.unknown` in JSON, kein Konsumer in Rust-Code)
**When** xtask `lint-events` läuft
**Then**

- Nach Sammlung aller Code-Keys aus dem erweiterten Scope (AC-A) wird die Backward-Drift-Richtung geprüft:
  - Für jeden Key in en.json, der **nicht** in der Code-Key-`BTreeSet` vorkommt → Violation:
    ```
    VIOLATION [locale-orphan]: key "error.unknown" present in en.json but no Rust emit-site found in klarvo-core/, klarvo-plugins/, klarvo-shell-orchestrator/, shells/windows/src-tauri/
    ```
- **Whitelist-Mechanismus für legitime Orphans:** Manche Keys werden nur via TypeScript-Frontend referenziert (Frontend-Only-Fallbacks, zukünftige Settings-UI-Strings) und haben keinen Rust-Emit-Site. Solche Keys werden in `xtask/orphan-allowlist.txt` (Workspace-Root) erfasst:
  - Datei-Format: ein Key pro Zeile, `#`-Kommentar-Lines erlaubt
  - **Initial-Inhalt (verifiziert per `grep -rn "error\.unknown" shells/windows/src/`):**
    ```
    # Frontend-Only-Keys ohne Rust-Emit-Site.
    # Format: ein Key pro Zeile, # = Kommentar.
    # Jeder Eintrag braucht einen Begründungs-Kommentar mit Datei:Zeile-Referenz.

    # error.unknown: Frontend-Fallback in shells/windows/src/index.html:79,148
    # für unbekannte Error-Payloads aus app.error-Event. Kein Rust-Emit-Site geplant.
    error.unknown
    ```
  - Lint liest die Datei, normalisiert Whitespace + Skip-Comment-Lines, prüft Orphan-Keys gegen die Allowlist, ignoriert Match-Hits
- **Symmetrie-Check zu de.json bleibt bei G3-B** — kein Duplikat in 5.6, der bestehende Code in `run_g3b_locale_cross_check` Step 2 deckt das ab
- **Forcing-Sentinel (analog Story 5.3 AC-F):** Inline-Test im Lint-Modul, der eine en.json mit einem Test-Orphan-Key (`"test.orphan.sentinel"`) konstruiert (in-memory, nicht auf Disk), die Code-Key-Sammlung leer simuliert und assertiert dass der Lint exit-1 mit `[locale-orphan]`-Violation meldet

### AC-C — REQUIRED_KEYS-Konstante entfernen + Tests retiren

**Given** AC-A und AC-B mechanisch decken, was die manuelle Whitelist abdeckt
**When** Story 5.6 die Cleanup-Promise aus Story 5.3 (`shells/windows/src-tauri/src/i18n.rs:54-71`) einlöst
**Then**

- `const REQUIRED_KEYS: &[&str]` (aktuell `shells/windows/src-tauri/src/i18n.rs:72-116`) entfernt
- Die folgenden Tests entfernt:
  - `en_json_covers_all_required_keys` (Zeile 118-131) — von G3-B Forward-Drift abgedeckt
  - `no_orphan_keys_in_en_json` (Zeile 147-161) — von Story-5.6-AC-B abgedeckt
- **Behalten:** `de_json_covers_same_key_set` (Zeile 133-145) — duplikativ zu G3-B Symmetrie-Check, aber als shell-lokaler Smoke-Test bei `cargo test -p klarvo-windows-shell` weiterhin sinnvoll als Schnell-Catch (Trade-off: kleiner Doppel-Test vs. potentielle CI-Lücke wenn xtask-Lint mal nicht läuft)
- Die Doc-Notes auf Zeile 54-71 (Manual-Maintenance-Hinweis + Audit-Source-Referenz) entfernt, ggf. durch Kurzhinweis ersetzt: `// i18n-Drift wird durch xtask lint-events (Story 5.3 + 5.6) mechanisch enforct.`
- `cargo test -p klarvo-windows-shell` (oder Cross-Target / CI) bleibt grün ohne die entfernten Tests
- ⚠️ **Verifikations-Reihenfolge:** Dev-Agent muss zuerst AC-A + AC-B implementieren und grün laufen lassen, **bevor** AC-C ausgeführt wird — sonst entfällt die Defense ohne Ersatz

### AC-D — Forcing-Sentinels (Skip-by-Design)

**Given** `feedback_ci_gate_philosophy.md` fordert Forcing-Sentinels für jeden neuen Sub-Pass; Stub-Checks sind verboten
**When** Story 5.6 die neuen Lint-Pässe ergänzt
**Then**

- Mindestens zwei Inline-Tests in `xtask/src/lint_events.rs` (Test-Modul):
  1. `g3d_orphan_key_detected` (oder analoger Name) — konstruiert in-memory eine Code-Key-Menge ohne `"test.orphan.sentinel"` UND ein en.json-Mapping mit `"test.orphan.sentinel"` → Violation erwartet
  2. `g3d_orphan_allowlist_skips_match` — gleiche Setup, aber Allowlist enthält `"test.orphan.sentinel"` → keine Violation, Lint-Pass-OK
- Tests laufen unconditionally bei `cargo test -p xtask` (kein `#[ignore]`, kein env-Skip)
- Falls Code-Key-Sammlung als getrennte Funktion refactored wird (`collect_code_keys(scope: &[PathBuf]) -> CodeKeys`), zusätzlicher Test:
  - `g3d_collect_code_keys_finds_emit_error_calls` — verifiziert dass `emit_error("error.foo", ...)` und `unwrap_or("error.bar")` als Code-Keys eingesammelt werden (Pattern aus AC-A)

### AC-E — Verifikation

**Given** alle Implementations + Cleanups eingebaut
**When** Dev-Agent die Verifikation läuft
**Then**

- `cargo xtask lint-events` → Exit 0 auf aktuellem `master`-State (en.json + de.json müssen sauber sein nach Cleanup)
- `cargo test -p xtask` → alle Tests grün inklusive neuer Forcing-Sentinels
- `cargo test -p klarvo-windows-shell` (Linux-Cross-Target, oder CI auf Windows) → alle Tests grün, **ohne** die entfernten REQUIRED_KEYS-Tests
- `xtask/orphan-allowlist.txt` existiert (kann initial leer + nur Kommentare sein)
- **CI-Verifikation:** AC-E.5 — `windows-compile-ci.yml` (oder die generelle `lint-events`-CI-Pipeline) ruft `cargo xtask lint-events` auf einer ungeänderten `master`-Variante UND auf einer Test-Branch mit künstlichem Orphan-Key (Forcing-Sentinel-Pattern). Erwarteter CI-Exit: grün auf master, rot auf Sentinel-Branch — verifiziert via temporär gecommittetem Sentinel-Key (revert vor Merge der 5.6).

## Technical Notes

### Implementation-Strategie: Erweiterung von `lint-events`, nicht neuer Subcommand

Die Retro-AI-3-Beschreibung („xtask: REQUIRED_KEYS-Drift-Detection (parse JSON-Locale-Files, diff gegen `i18n.rs::REQUIRED_KEYS`)") suggeriert einen separaten xtask-Subcommand. Story 5.6 wählt stattdessen die Erweiterung von `xtask lint-events` aus drei Gründen:

1. **Single-Source-of-Truth statt zwei Drift-Detektoren:** G3-Sub-Lint B (Story 5.3) sammelt bereits Code-Keys aus klarvo-core + klarvo-plugins. AC-A erweitert diese Sammlung statt sie zu duplizieren.
2. **Konsistenz mit Story 5.3 Cleanup-Promise:** Story 5.3 dokumentiert explizit Phase-2-Cleanup-Story, die G3-B um Whitelist-Check erweitert; das passt strukturell in `lint-events`.
3. **CI-Setup wiederverwendet:** `cargo xtask lint-events` ist bereits in `windows-compile-ci.yml` etabliert — kein neuer CI-Step nötig.

Sollte Dev-Agent während Implementation feststellen, dass die Erweiterung `lint-events` zu groß macht (z.B. >1500 Zeilen Single-File), kann ein `xtask/src/lint_locale.rs`-Module-Split vorgenommen werden — aber als interner Code-Split, nicht als separater Subcommand.

### Naming der neuen Sub-Lint-Stage

Zur Wahl:
- **`G3-Sub-Lint D` (Empfehlung):** Setzt die Naming-Konvention von Story 5.3 (G3-A/B/C) fort. Klar als ergänzender G3-Pass erkennbar.
- **`G4`:** Würde signalisieren dass es ein neues Validation-Tier ist — nicht passend, da die Funktion semantisch zu G3 (i18n-Kontrakt-Enforcement) gehört.

Empfehlung: **G3-D**. Naming wird in Code-Doc-Comments (`xtask/src/lint_events.rs` Top-Level-Doc) und Dev-Agent-Completion-Notes konsistent verwendet.

### Pattern-Erweiterung: `emit_error` und `unwrap_or` Erkennung

G3-A erkennt aktuell `user_message: Some(<lit>)`-Struct-Init-Position und `#[default = "<lit>"]`-Attribute. Story 5.6 muss zwei zusätzliche Patterns erkennen:

**Pattern 1: `Trait::method("<key>", ...)`** (Trait-Method-Call mit Literal-Erstargument)

Beispiel: `self.error_emitter.emit_error("error.audio.start_failed", clock.now_ms()).await`
- AST-Form: `Expr::MethodCall { method: Ident("emit_error"), args: [Expr::Lit(Str(...)), ...] }`
- Heuristik: Method-Name-Allowlist (`emit_error`, plus zukünftige Trait-Methods aus ADR-0009) + erstes Argument ist String-Literal
- Begründung-Limitation in Code-Doc: Lint matcht auf Method-Name (Heuristik), nicht auf Type-Resolution; theoretischer False-Positive bei anderem Trait mit gleichnamiger Method ist im Phase-1-Workspace unwahrscheinlich

**Pattern 2: `<expr>.unwrap_or("<key>")`**

Beispiel: `e.user_message.as_deref().unwrap_or("error.internal")`
- AST-Form: `Expr::MethodCall { method: Ident("unwrap_or"), args: [Expr::Lit(Str(...))] }`
- Heuristik: Method-Name `unwrap_or` + einziges Argument ist String-Literal das KEY_REGEX erfüllt
- ⚠️ **False-Positive-Risiko:** `unwrap_or` wird häufig auch außerhalb von i18n-Kontexten genutzt (z.B. `.unwrap_or("default value")`). Filter via `is_key()`-Validierung (KEY_REGEX) mitigiert das — Strings die nicht dem Key-Format entsprechen werden ignoriert
- Implementation-Note: AST-Visit auf `Expr::MethodCall` muss in Visitor-Struct (analog `UserStringVisitor`) angereichert werden

### Allowlist-File: warum Datei statt Code-Konstante

Die Orphan-Allowlist (`xtask/orphan-allowlist.txt`) ist als separates File ausgelegt, nicht als Rust-Konstante in `lint_events.rs`, weil:

1. **PR-Diff-Surface:** Eine Eintragung in die Allowlist ist Review-Surface, der Reviewer kann den Eintrag prüfen ohne Rust-Code zu öffnen
2. **Format-Disziplin:** Whitespace-Format + Comment-Lines machen die Datei selbst-dokumentierend (jede Zeile rechtfertigt sich)
3. **Konsistenz mit anderen xtask-Konfigs:** ähnliches Pattern wie `clippy.toml` (Story 5.5)

Initial-Inhalt (Stand 2026-05-01, verifiziert per Grep): ein Eintrag — `error.unknown` (Frontend-Fallback in `shells/windows/src/index.html:79,148`). Künftige Frontend-Only-Keys (z.B. Settings-UI-Strings für Story Epic 9) werden mit Begründung als Comment-Block über dem Eintrag hinzugefügt.

**Pre-Story-Verifikation für Dev-Agent (analog AC-E.5):** Vor Implementation prüfen, ob seit 2026-05-01 weitere Frontend-Only-Keys hinzugekommen sind. Suche-Pattern: `grep -rn "<key>" shells/windows/src/`. Aktuell bekannte Frontend-Konsumenten: `index.html`, später `src/main.ts`. Wenn neue Frontend-Only-Keys gefunden werden: Allowlist erweitern statt Lint-Failure.

### Was Story 5.6 NICHT macht (Out-of-Scope)

- **Frontend-Key-Coverage:** TypeScript-Code in `shells/windows/src/` wird nicht gescannt. Frontend hat eigene i18n-Library (P1-ADR pending) und eigene Key-Resolution-Pipeline. Frontend-Drift ist Out-of-Scope für 5.6.
- **Plugin-spezifische Keys:** Plugin-spezifische i18n-Files (falls Plugins eigene Locale-Resources mitbringen) — aktuell kein Plugin tut das, künftiger Erweiterungspunkt für Story Epic 10+.
- **Android-Shell:** `android/`-Code wird vom xtask aktuell ausgeschlossen (`is_excluded_g3` filtert `android`). Android-i18n-Drift wird in Phase-3 (Android-Submission-Window) separat adressiert — siehe Memory `project_android_playstore_risk`.
- **Build-Time-Resolution-Validation:** Format-String-Placeholder in Locale-Werten (z.B. `"{count} files"`) werden nicht gegen Resolver-Calls validiert. Out-of-Scope.

### Wichtige Memory-Referenzen für Dev-Agent

- `memory/project_i18n_three_axes` — UI-Language-Achse, betrifft en/de-Symmetrie
- `memory/project_i18n_core_contract` — Core/Shell-Separation, Core hat keine User-Strings
- `memory/feedback_ci_gate_philosophy` — Forcing-Sentinel-Pflicht, Skip-by-Design
- `memory/feedback_reference_block_discipline` — auch trivial-erscheinende Defaults explizit dokumentieren (Allowlist-Format)
- `memory/feedback_scaffold_fail_soft_pattern` — keine `todo!()`/`unimplemented!()`-Lücken in xtask-Pass
- `memory/feedback_premature_abstraction_guard` — wenn Code-Key-Sammlung neu strukturiert wird, nur factor-out wenn proven Re-Use (≥2 Konsumenten)

## Tasks / Subtasks

- [ ] Task 1 — Code-Key-Scope-Erweiterung (AC-A)
  - [ ] 1.1 `run_g3a_user_string_check` (oder neue Funktion `collect_code_keys`) erweitern um `klarvo-shell-orchestrator/src/` und `shells/windows/src-tauri/src/` als Scan-Roots
  - [ ] 1.2 Visitor erweitern: `Expr::MethodCall` mit Method-Name `emit_error` + erstes Arg = Literal → in `code_keys` aufnehmen wenn `is_key()`
  - [ ] 1.3 Visitor erweitern: `Expr::MethodCall` mit Method-Name `unwrap_or` + einziges Arg = Literal → in `code_keys` aufnehmen wenn `is_key()`
  - [ ] 1.4 Test-Module (`#[cfg(test)]`) bleiben ausgeschlossen via bestehender `in_test_mod`-Heuristik

- [ ] Task 2 — Orphan-Check-Logic + Allowlist (AC-B)
  - [ ] 2.1 Neuer Sub-Pass `run_g3d_orphan_check(workspace_root, code_keys)` in `xtask/src/lint_events.rs`
  - [ ] 2.2 Lädt `en.json`, sammelt en-Keys, prüft jedes en-Key gegen `code_keys`-Set; nicht-vorhandene Keys → Violation
  - [ ] 2.3 Lädt `xtask/orphan-allowlist.txt`, normalisiert Whitespace, skippt Comment-Lines (`#`-prefix); Allowlist-Hits werden vor Violation-Push übersprungen
  - [ ] 2.4 Allowlist-File `xtask/orphan-allowlist.txt` initial mit Header-Comment + `error.unknown`-Eintrag (Frontend-Only, siehe AC-B Initial-Inhalt) anlegen
  - [ ] 2.4a Pre-Verifikation: `grep -rn "<frontend-only-key>" shells/windows/src/` für jeden in en.json gelisteten Key OHNE Rust-Emit-Site → ggf. zusätzliche Allowlist-Einträge mit Begründung
  - [ ] 2.5 Sub-Pass in `run()`-Funktion sequenziell nach G3-C aufrufen, Violations aggregieren

- [ ] Task 3 — Forcing-Sentinels (AC-D)
  - [ ] 3.1 Inline-Test `g3d_orphan_key_detected`: konstruiert leere `code_keys`, en-Table mit `{"test.orphan.sentinel": "..."}`, leere Allowlist; assertiert eine `[locale-orphan]`-Violation enthaltend `"test.orphan.sentinel"`
  - [ ] 3.2 Inline-Test `g3d_orphan_allowlist_skips_match`: gleiche Inputs, aber Allowlist enthält `"test.orphan.sentinel"`; assertiert keine Violation
  - [ ] 3.3 Inline-Test `g3d_collect_code_keys_finds_emit_error_calls`: Source-String mit `emitter.emit_error("error.foo", ts)` + `e.user_message.as_deref().unwrap_or("error.bar")`; assertiert dass `code_keys` beide enthält

- [ ] Task 4 — REQUIRED_KEYS-Cleanup (AC-C, **erst nach grünem AC-A + AC-B**)
  - [ ] 4.1 `const REQUIRED_KEYS: &[&str]` aus `shells/windows/src-tauri/src/i18n.rs:72-116` entfernen
  - [ ] 4.2 Test `en_json_covers_all_required_keys` (Zeile 118-131) entfernen
  - [ ] 4.3 Test `no_orphan_keys_in_en_json` (Zeile 147-161) entfernen
  - [ ] 4.4 Doc-Block Zeile 54-71 entfernen, durch Kurz-Verweis ersetzen: `// i18n-Drift wird durch 'cargo xtask lint-events' (Stories 5.3 + 5.6) mechanisch enforct.`
  - [ ] 4.5 Test `de_json_covers_same_key_set` bleibt erhalten (shell-lokaler Smoke-Test, Trade-off-begründet)

- [ ] Task 5 — Verifikation (AC-E)
  - [ ] 5.1 `cargo xtask lint-events` → Exit 0 auf master nach Cleanup (en.json + de.json müssen sauber sein)
  - [ ] 5.2 `cargo test -p xtask` → grün inklusive 3 neuer Sentinels
  - [ ] 5.3 `cargo test -p klarvo-windows-shell` (Linux-Cross-Target oder CI auf Windows) → grün ohne entfernte Tests
  - [ ] 5.4 CI-Verifikation per temporärem Sentinel-Key (revert vor Merge): rot auf Sentinel-Branch, grün auf master nach Revert
  - [ ] 5.5 Completion-Notes dokumentieren: Listings der entfernten Tests + Diff-Größe der i18n.rs + Allowlist-File-Pfad

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
