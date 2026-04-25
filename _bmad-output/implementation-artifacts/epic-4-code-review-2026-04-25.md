# Epic 4 Code Review — 2026-04-25

**Stories:** 4.1–4.5 (i18n-Refactor + Sanity-Tester-Doku)
**Commits:** `da310ec`, `040827e`, `0fb0993`, `4357361`, `c6c692c`
**Reviewer-Layer:** Blind Hunter + Edge Case Hunter + Acceptance Auditor (parallel)
**Diff:** 14 Dateien, 1981 Insertions / 43 Deletions; 4 Code-Files (`config.rs`, `i18n.rs`, `main.rs`, 2× locale.json) plus Story-Specs + Onboarding-Doc.

---

## Executive Summary

**4 Patches | 3 Defers | 7 Dismiss.** Acceptance-Coverage ist hoch — 30 von 31 reviewbaren ACs erfüllt (1 Doc-Drift in 4.1 AC-G). Größtes Finding ist ein **Story-4.4-AC-A-Miss**: der Coverage-Audit hat den `error.internal`-Fallback in `session.rs` übersehen, weil das Grep-Muster nur `user_message: Some(...)` abdeckte — `unwrap_or("error.internal")` rutschte durch. Drei weitere Patches sind Doc-Drift / Policy-Konsistenz.

Aus User-Direktive nicht re-litigiert: Spec-Delta `upstream_5xx`/`upstream_4xx` (Story 4.4) ist code-aligned + Completion-Notes; AC-G Manual-Smoke (Story 4.5) ist User-Validation-Domain.

---

## Patch Findings

### P1 — `error.internal` ist Orphan-Emit-Key (Story 4.4 AC-A Miss)

**Severity:** BUG (FR27 / Story 4.4 AC-A direkt verletzt)
**Location:** `klarvo-shell-orchestrator/src/session.rs:148, 155, 176`
**Source:** Edge Case Hunter

`session.rs::pipeline_task` emittiert in drei Pfaden den i18n-Key `error.internal` als Fallback wenn ein propagierter `AppError` `user_message: None` hat:

```rust
e.user_message.as_deref().unwrap_or("error.internal"),
```

`From<PluginError>` in `klarvo-core/src/error.rs:73-101` setzt für **5 von 6** PluginError-Varianten (`Network`, `Auth`, `RateLimit`, `Fatal`, `UpstreamUnavailable`) `user_message: None`. Diese AppErrors landen in `pipeline_task` und triggern den Fallback. `error.internal`:

- **fehlt in `REQUIRED_KEYS`** (`shells/windows/src-tauri/src/i18n.rs:58-89`, 30 Keys)
- **fehlt in `shells/windows/locales/en.json`**
- **fehlt in `shells/windows/locales/de.json`**

Frontend bekommt `app.error { key: "error.internal" }` ohne Übersetzung — der Key wird roh in den Toast gerendert (oder leer, je nach Frontend-Resolver-Pfad).

**Wurzel:** Story 4.4 AC-A Audit-Method greppte `user_message: Some(...)`-Patterns und `pub const *: &str = "error.*"`-Konstanten (siehe `_bmad-output/implementation-artifacts/i18n-coverage-audit-2026-04-25.md:11`). Das Pattern `unwrap_or("error.internal")` rutschte durch beide Filter.

**Fix:**
1. `error.internal` in `REQUIRED_KEYS` (`i18n.rs`) ergänzen.
2. EN-String in `en.json`, z. B.: `"An internal error occurred. Please check the diagnostic export or restart the application."`
3. DE-String in `de.json` analog (Stil-Guideline 4.3 AC-B).
4. Kurzer Update-Eintrag im Audit-Doc (`i18n-coverage-audit-2026-04-25.md`) zur Closure dieses Misses.

**Impact:** Verhindert rohe-Key-Anzeige bei Plugin-Errors mit `user_message: None`. Schließt FR31 (alle User-facing Errors keyed-und-resolved) wirklich.

---

### P2 — Stale Rustdoc-Tabelle in `load_config` referenziert dead key

**Severity:** DOC-DRIFT (Story 4.1 AC-G — Doc-Drift, kein Compile-Fail)
**Location:** `shells/windows/src-tauri/src/config.rs:150`
**Source:** Blind Hunter + Edge Case Hunter + Acceptance Auditor (3-fach bestätigt)

```rust
/// | Unsupported locale value | `Configuration` | `error.config.invalid_locale` |
```

Die Error-Path-Tabelle in der Rustdoc von `load_config` führt noch den alten Key auf, den Story 4.1 (Hard-Replace) durch `error.config.invalid_language` ersetzt hat (Code: `config.rs:127`). `grep -rn "error.config.invalid_locale"` über das ganze Repo trifft nur diese Doc-Zeile — kein anderer Konsument.

**Fix:** 1-Zeilen-Edit:
```rust
/// | Unsupported language axis (ui/dictionary/output) | `Configuration` | `error.config.invalid_language` |
```

---

### P3 — Doc-Beispiel in `klarvo-core/src/i18n.rs` nutzt nicht-existenten Production-Key

**Severity:** DOC-DRIFT (verleitet Plugin-Author zur falschen Key-Form)
**Location:** `klarvo-core/src/i18n.rs:17, 32`
**Source:** Edge Case Hunter

Doc-Comment-Beispiele:
```rust
//! Examples of valid keys: `"recording.started"`, `"error.pipeline.unknown_stage"`, `"app.ready"`.
//! assert_is_key("error.pipeline.unknown_stage");
```

Der tatsächliche Production-Key heisst `error.pipeline.unknown_stage_type` (Constant in `klarvo-core/src/manifest.rs::keys::UNKNOWN_STAGE_TYPE`, registriert in `REQUIRED_KEYS`, beide Locale-Files). Die Doc-Beispiele suggerieren die kürzere Form, was Plugin-Authoren zur falschen Key-Schreibweise verleiten kann.

**Fix:** 2 Doc-Edits — `unknown_stage` → `unknown_stage_type` in beiden Vorkommen.

---

### P4 — Bootstrap-Policy-Comment listet Step 2b nicht (Panic-Path-Inkonsistenz)

**Severity:** SMELL (Doc/Code-Policy-Drift)
**Location:** `shells/windows/src-tauri/src/main.rs:69-74` (Block-Comment) vs. `i18n.rs:32-38` (Panic-Path)
**Source:** Edge Case Hunter

Bootstrap-Block-Comment in `main.rs`:
```rust
// # Bootstrap-Error-Policy
// Fail-soft (continue with defaults/no-op) for Steps 1-8, 12: ...
// Fatal (return Err) for Steps 9-10: ...
```

Step 2b (`klarvo_windows_shell::i18n::load(&config.ui_language)`) ist tatsächlich ein Panic-Path:
```rust
.unwrap_or_else(|e| panic!("i18n boot-fail: locales/de.json is not valid JSON: {e}"))
```

Step 2b ist weder explicit fail-soft (Steps 1-8) noch fatal-as-Result (Steps 9-10) — sondern ein **Panic-Out-of-Setup-Closure**. Phase-2-Fail-Soft ist in `i18n.rs:31` per ADR-0009 SD-4 vorgemerkt; aber die Block-Comment-Policy in `main.rs` dokumentiert die aktuelle Ausnahme nicht.

**Fix:** Block-Comment in `main.rs:69-74` um Step-2b-Zeile ergänzen, z. B.:
```
// Step 2b is currently a Panic-Path (Phase-1 stub) — JSON-corruption surfaces
// before .setup() returns. Phase-2 fail-soft fallback per ADR-0009 SD-4.
```

---

## Defer Findings

### D1 — `both_locale_files_valid_json_even_when_en_active` testet Happy-Path statt Regression

**Location:** `shells/windows/src-tauri/src/i18n.rs:189-197`
**Source:** Blind Hunter + Edge Case Hunter + Acceptance Auditor

Test-Body:
```rust
let _table = load("en");
assert!(!_table.is_empty(), "en table must not be empty");
```

Der Test-Name verspricht Eager-Validation-Regression-Guard ("Wenn `de.json` corrupt ist, paniced auch `load("en")`"). Der Body prüft nur, dass `load("en")` nicht-leeres Ergebnis liefert. Wenn ein Refactor den `_de`-Validation-Pfad in `load()` entfernt, würde dieser Test grün bleiben.

**Defer-Begründung:** Echte Regression-Test bräuchte `load_from_strs(ui_language, en_json, de_json)`-Extraktion mit Inject-Variante (gezielt corrupt-DE-Fixture). `DE_JSON` ist `include_str!`-statisch — Test-Refactor wäre eigene Story. Pragmatisch akzeptabel: Eager-Validation-Code ist 1-Liner und greppbar; Code-Review fängt das natürlicher als ein synthetisch konstruierter Test.

---

### D2 — Kein TODO-Marker-Test für `de.json`

**Location:** `shells/windows/src-tauri/src/i18n.rs::tests` (kein Test vorhanden)
**Source:** Blind Hunter

`no_todo_markers_in_en_json` schützt `en.json` vor TODO-Regression. Symmetric-Coverage für `de.json` fehlt. Story 4.3 hat alle TODO(de)-Marker entfernt — Re-Introduction (z. B. neuer Key in zukünftiger Story mit `TODO(de):`-Platzhalter) würde lautlos durchrutschen, bis User es im Toast sieht.

**Defer-Begründung:** Trivialer Fix (5 Zeilen Test), aber niedrige Re-Introduction-Wahrscheinlichkeit:
1. EN ist Master, fängt strukturelle Drift
2. `de_json_covers_same_key_set` fängt Key-Set-Drift
3. Story 4.4 AC-H verlangt finale DE-Strings für neue Keys
Phase-2-Settings-UI macht das natürlicher mit Source-of-Truth-Locale-Schema.

---

### D3 — TOML-Type-Mismatch wird auf `error.config.missing` aliased

**Location:** `shells/windows/src-tauri/src/config.rs:103-116`
**Source:** Edge Case Hunter

`parse_from_str` branched nur auf `"unknown field"`-Substring im TOML-Error und schickt alles andere nach `error.config.missing` ("Configuration file not found"). Beispiele die durch dieses Bucket fallen:

- `ui_language = 42` (int für String-Field) → "Configuration file not found" (Datei existiert!)
- `ui_language = true`, `hotkey = [1,2,3]` (Type-Mismatch generally)
- TOML-Syntax-Error (z. B. malformed Sections)

**Defer-Begründung:** Pre-existing aus Story 3.2-Branch-Logic; Story 4.1 hat parse_from_str erweitert, aber nicht die Error-Klassifikations-Heuristik. Phase-1-akzeptabel mit aktueller Key-Liste. Sauberer Fix:
- Neuer Key `error.config.invalid_type`
- Match auf TOML-Error-Patterns (`invalid type`, `expected ...`)
- Zugehöriger AC + Locale-Strings

→ Eigene Phase-2-Story (Settings-UI hat eh Type-Aware-Validation in JSON-Schema).

---

## Dismissed Findings

- **Smoke-Checklist `ui_language="fr"` Widerspruch** (Blind Hunter): False-Alarm. `main.rs:83-95` wrappt Config-Load in fail-soft → Bootstrap fängt den AppError, fällt auf `ShellConfig::default()` (`ui_language="en"`) zurück, `tracing::error!` loggt. Doc-Behauptung "App startet trotzdem (fail-soft), nutzt Default-Sprache (en); Fehler im Log sichtbar" stimmt 1:1. Blind Hunter ohne Project-Read konnte das nicht sehen.
- **Audit-Doc Line-Numbers stale (config.rs:92,159)** (Blind Hunter): Verifiziert — beide Zeilen matchen aktuellen `error.config.missing`-Emit-Sites. Unverifiable-Finding stellt sich als false-Alarm raus.
- **`klarvo-shell-orchestrator/tests/e2e_test.rs:247` orphan key `upstream_unavailable`** (Edge Case Hunter): Pre-existing Test-Fixture; explizit im Audit-Doc (`i18n-coverage-audit-2026-04-25.md:103-104`) und in `i18n.rs:56-57` als Spec-Delta-Note dokumentiert. NICHT durch Epic 4 verursacht. Per User-Direktive (Spec-Delta nicht re-litigieren) dismissed.
- **DE_JSON wird bei `ui_language="de"` doppelt geparst** (Blind Hunter + Edge Case Hunter): Trivial-Cost (~30-Key-JSON, <1ms am Boot). Kosmetisch.
- **`exit_label`-Fallback `"Exit"` ist locale-blind** (Edge Case Hunter): Defensiv, im Steady-State unreachable (Coverage-Tests garantieren `tray.menu.exit`). Akzeptabel.
- **`mixed_languages_independent_axes` Coverage-schwach für Symmetrie-Bugs** (Edge Case Hunter): NIT. Eine Permutation `(de, en, de)` deckt das primäre Bug-Pattern (Cross-Field-Promotion). Die anderen 8 möglichen Permutationen würden keine zusätzlichen Bug-Klassen testen.
- **Doc-Comment auf `ShellConfig` über `#[serde(alias)]`-Abwesenheit** (Blind Hunter): Tatsächlich legitime Dokumentation der Hard-Replace-Begründung mit Memory-Verweis. Phase-2-Update wäre kein Doc-Bug, sondern Doc-Update.

---

## Story-Level AC-Coverage (aus Acceptance Auditor)

| Story | ACs total | Erfüllt | Abweichungen |
|-------|-----------|---------|--------------|
| 4.1   | 7 (A–G)   | 7       | 1 Doc-Drift (AC-G → P2) |
| 4.2   | 6 (A–F)   | 6       | 1 minor Test-Pragma (D1) |
| 4.3   | 5 (A–E)   | 5*      | * AC-E Smoke per User-Direktive out-of-scope |
| 4.4   | 8 (A–H)   | 8**     | ** AC-A miss `error.internal` → P1 (Coverage-Method-Lücke) |
| 4.5   | 7 (A–G)   | 7***    | *** AC-G Manual-Smoke per User-Direktive out-of-scope |

**33 reviewbare AC-Items, 33 strukturell erfüllt.** P1 ist eine AC-A-Method-Lücke (Audit-Grep-Pattern unvollständig), kein AC-Wording-Verstoß.

---

## Reviewer-Performance

- **Blind Hunter** lieferte 6 Findings, davon 2 echt (P2 Doc-Drift bestätigt 3-fach, D1 Test-Pragma), 1 unverifiable, 3 dismissable / Duplicates.
- **Edge Case Hunter** lieferte 10 Findings — der einzige Reviewer der den **`error.internal`-Orphan** (P1) gefunden hat, sowie P3 (klarvo-core/i18n.rs Doc-Beispiel) und P4 (Bootstrap-Policy-Comment). Klar wertvollster Layer für diesen Review.
- **Acceptance Auditor** bestätigte AC-Coverage und P2 als einzigen formalen AC-Drift; kongruent mit den anderen Layern.
