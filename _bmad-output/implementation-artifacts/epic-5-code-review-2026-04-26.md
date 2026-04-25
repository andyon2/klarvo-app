# Epic 5 Code Review — 2026-04-26

**Stories:** 5.1–5.4 (xtask-Subcommands: manifest-strict, bindings-drift, lint-events G3, verify-release hardening)
**Commits:** `8ca7a84`, `52cedad`, `d2e99f3`, `9605bc0`
**Reviewer-Layer:** Blind Hunter + Edge Case Hunter + Acceptance Auditor (parallel)
**Diff:** 20 Dateien, 1429 Insertions / 229 Deletions; xtask-Code (5 Subcommand-Module + main.rs), 5 Fixtures, 1 Shell-i18n-Patch, 4 Story-Specs, Backlog, Cargo.

**Persisted Spec-Deltas (no re-litigation):**
- KEY_REGEX-Pattern-Breiterung (Story 5.3) — accepted
- `dev-plain-keystore` Phase-0-Doppelung → Story 5.4 AC-B reduziert auf Doku+Sentinel — accepted

---

## Executive Summary

**1 Blocking | 7 Patches | 3 Decisions | 11 Defers | 11 Dismiss.**

Größtes Finding ist ein **Story-5.2-AC-G-Miss**: Beide Forcing-Sentinel-Tests in `bindings_drift::tests` rufen weder `bindings_drift::run()` noch die Vergleichs-Logik auf — `assert_eq!(content, content)` ist tautologisch, und `artificial_drift_detected` vergleicht zwei Test-interne String-Literale ohne die Production-Logik zu durchlaufen. Wenn `committed == generated` zu `committed != generated` invertiert würde, blieben beide Tests grün. Das verletzt die Forcing-Sentinel-Doktrin aus `feedback_ci_gate_philosophy` direkt — der Sentinel schützt das Gate nicht.

Zweitwichtigstes Finding (Patch P6): G3-Sub-Lint A erfasst **keine file-basierten `mod keys;`-Deklarationen**. Klarvo nutzt 4 solcher Module (`klarvo-core/src/{audio,output,keystore,v1_import}/keys.rs`); deren Key-Konstanten landen nie in `code_keys`, sodass G3-B Forward-Drift-Checks für diese Pfade keine Wirkung haben. Aktuell fällt nichts auf, weil die Keys manuell in `en.json` gepflegt werden — das Gate gibt aber falsche Sicherheit.

Drei **Decisions** drehen sich um Spec-vs-Implementation-Lücken, die ohne User-Entscheidung nicht patchbar sind:
- D1: `render()` ist nicht "rein ohne Datei-I/O" wie AC-A textlich verlangt (Amendment vs. Refactor zu Tempfile)
- D2: G3-A `in_keys_mod`-Restriction ist unannounced Spec-Narrowing per `feedback_kickoff_deltas_only` (Amendment vs. Restriction-Removal)
- D3: G3-A AC-B 3rd Position (`#[default = "..."]`-Attribute) nicht implementiert (Amendment vs. Implementation)

---

## Blocking Findings

### B1 — `bindings_drift` Forcing-Sentinel-Tests sind tautologisch (Story 5.2 AC-G Miss)

**Severity:** BLOCKING (FR33 / Story 5.2 AC-G direkt verletzt + Forcing-Sentinel-Doktrin aus `feedback_ci_gate_philosophy`)
**Location:** `xtask/src/bindings_drift.rs:73-100`
**Source:** Blind Hunter + Acceptance Auditor

```rust
#[test]
fn artificial_drift_detected() {
    let generated = "// ... export const foo = 'bar';";
    let mut temp = tempfile::NamedTempFile::new().expect("temp file");
    let modified = format!("{generated}\n// artificial drift\n");
    temp.write_all(modified.as_bytes())...;
    let committed = std::fs::read_to_string(temp.path())...;
    assert_ne!(committed, generated, ...);
}

#[test]
fn in_sync_returns_no_drift() {
    let content = "// same content on both sides";
    assert_eq!(content, content, "identical content = in-sync");
}
```

Beide Tests prüfen nur, dass zwei Test-eigene String-Literale (un-)gleich sind. Sie rufen weder `bindings_drift::run()` noch `generate_bindings::render()`. Wenn die Production-Comparison `committed == generated` zu `committed != generated` invertiert würde:
- `artificial_drift_detected` bliebe grün (zwei nicht-gleiche Strings sind weiterhin nicht gleich)
- `in_sync_returns_no_drift` bliebe grün (zwei gleiche Strings sind weiterhin gleich)
- Die Drift-Detection wäre kaputt; das Sentinel würde es nicht melden

AC-G Wortlaut (`story-5.2-bindings-drift-xtask.md:135-139`): *"Test `in_sync_returns_ok`: Ruft intern `render()` auf, vergleicht gegen den frischen Output — erwartet keine Differenz (testet die Basis-Logik)"*. Das wird so nicht ausgeführt.

**Fix-Optionen** (Patch — User-Klärung welche Variante):

1. **Recht-rein**: `bindings_drift::run()` aufteilen in `compare(committed: &str, generated: &str) -> ExitCode` + Wrapper, dann Test ruft `compare()` direkt auf. (Sauberster Forcing-Sentinel.)
2. **Pragmatisch**: Test ruft `bindings_drift::run()` auf nach Setup eines Tempfiles via `bindings_path_override` (env-var oder thread-local). Ohne Refactor schwer.
3. **Snapshot-Restore-Path testen**: Test ruft `run()` auf, modifiziert die committed-Datei vorher, verifiziert ExitCode + Restore. Braucht Test-Isolation (Repo-State-Modifikation).

**Empfehlung:** Variante 1 (`compare()`-Extract) — mechanisch, kein Test-Repo-State.

**Impact:** Schließt das Sentinel-Loch. Ohne Fix garantiert das Gate nicht, was es soll — exakt der Anti-Pattern, den FR34/FR33-Story explizit verbietet.

---

## Patch Findings

### P1 — `bindings_drift` Restore-Write swallows errors

**Severity:** IMPORTANT (Story 5.2 — silent data corruption auf Restore-Failure-Pfad)
**Location:** `xtask/src/bindings_drift.rs:41, 53`
**Source:** Blind Hunter + Edge Case Hunter + Acceptance Auditor (3-fach bestätigt)

```rust
let _ = generate_bindings::write_to_disk(&committed);
```

Auf beiden Restore-Pfaden (render-failure + drift-detected) wird das Result geschluckt. Wenn `index.ts` mid-check unwritable wird (Permission-Flip, FS-full, Windows-Lock), bleibt die Datei in undefiniertem Zustand und der User bekommt nur den ursprünglichen Drift/Render-Error — kein Hinweis auf den Restore-Failure.

**Fix:**
```rust
if let Err(e) = generate_bindings::write_to_disk(&committed) {
    eprintln!("WARN: bindings-drift restore failed — index.ts may be in regenerated state: {e}");
}
```

**Impact:** Sichtbarkeit auf seltene aber gefährliche Failure-Pfade; verhindert silent-corruption.

---

### P2 — `--skip-cross-compile` Flag-Scope leakt + unknown-Flag exit 0

**Severity:** IMPORTANT (CI-Gate-Verlässlichkeit; Story 5.4 AC-C / xtask UX)
**Location:** `xtask/src/main.rs:13, 25-29`
**Source:** Blind Hunter + Edge Case Hunter + Acceptance Auditor

Zwei Probleme im selben Parser-Block:

1. **Globale Flag-Erkennung:** `args.contains(&"--skip-cross-compile".to_string())` matched überall — `cargo xtask manifest-strict --skip-cross-compile` akzeptiert das Flag stillschweigend (heute no-op, morgen Footgun, falls andere Subcommands Flags bekommen).
2. **Unknown-Flag silent exit 0:** Der `Some(cmd) if cmd.starts_with("--")`-Arm fängt jeden ungültigen Flag-Namen (z.B. Tippfehler `--ksip-cross-compile`), druckt Help, exit 0. CI-Pipelines mit Tippfehlern bemerken den Misuse nicht.

**Fix:**
```rust
match args.first().map(String::as_str) {
    None | Some("--help") | Some("-h") => { print_help(); ExitCode::SUCCESS }
    Some("verify-release") => verify_release::run(skip_cross_compile),  // skip_cross_compile nur hier ausgewertet
    Some("generate-bindings") | Some("lint-events") | Some("manifest-strict") | Some("bindings-drift") => {
        // Subcommands ohne Flags — reject unknown args
        if args.iter().skip(1).any(|a| a.starts_with("--")) {
            eprintln!("xtask: subcommand '{cmd}' takes no flags");
            return ExitCode::from(2);
        }
        ...
    }
    Some(cmd) if cmd.starts_with("--") => {
        eprintln!("xtask: unknown flag '{cmd}'");
        ExitCode::from(2)
    }
    Some(cmd) => {
        eprintln!("xtask: unknown subcommand '{cmd}'");
        ExitCode::from(2)
    }
}
```

**Impact:** CI-Tippfehler werden zu Fail; `--skip-cross-compile` ist exklusiv für `verify-release`.

---

### P3 — `manifest_strict::kind_str` `_ => "Unknown"` ist Anti-Pattern (selbst-violiert die Doktrin)

**Severity:** IMPORTANT (Epic-5-G3-C-Doktrin selbst gebrochen; silent variant-drift)
**Location:** `xtask/src/manifest_strict.rs:200`
**Source:** Blind Hunter + Edge Case Hunter

Ironisch: Story 5.3 AC-E verbietet wildcard-`_`-Arms auf `PipelineStageType`. Hier ist das Match auf `AppErrorKind` (anderer Enum, anderes Crate, formal nicht im G3-C-Scope) — aber die selbe Falle: Wenn `klarvo-core` einen neuen `AppErrorKind` ergänzt, ergibt `kind_str` für Fixtures `"Unknown"`, der String-Vergleich in `check_result` schlägt mit nicht-aussagekräftigem `[FAIL]` fehl, nicht mit Compile-Error.

**Fix:**
```rust
fn kind_str(kind: &AppErrorKind) -> &'static str {
    match kind {
        AppErrorKind::PipelineValidation => "PipelineValidation",
        // ... (alle Varianten explizit)
        // KEIN wildcard-arm — neue Variante ⇒ Compile-Error
    }
}
```

`AppErrorKind` ist nicht `#[non_exhaustive]` ⇒ exhaustive match ist möglich. Wenn `non_exhaustive` später kommt, müsste `_ => panic!("unknown variant — extend kind_str")` (laut Forcing-Sentinel-Pattern; nicht silent fallback).

**Impact:** Schließt silent variant-drift; aligns xtask-Code mit der Doktrin, die das Subcommand selbst ausrollt.

---

### P4 — `manifest_strict::run_fixture` koppelt Test-Branch an Magic-String

**Severity:** IMPORTANT (Test-Harness-Robustheit)
**Location:** `xtask/src/manifest_strict.rs:96`
**Source:** Blind Hunter

```rust
fn run_fixture(name: &str, content: &str) -> Result<(), klarvo_core::AppError> {
    if name == "bad-type-mismatch" {
        // ... volle Executor-Boot-Path
    } else {
        parse_from_str(content).map(|_| ())
    }
}
```

Wenn die Fixture umbenannt wird (`bad-type-mismatch` → `bad-stage-types-incompatible`), fällt der Test stillschweigend auf den Parse-only-Pfad zurück und „passt" weiterhin (weil parse_from_str für die syntaktisch valide Fixture Ok zurückgibt) — ohne dass der Boot-Path je exerciert wird. Magic-String-Koppelung verstößt gegen die *Forcing-Sentinel-by-Design* aus `feedback_ci_gate_philosophy`.

**Fix:** Eine Markierung im `expected.toml` machen, z.B.:
```toml
[bad-type-mismatch]
outcome = "err"
mode = "boot"  # forces full Executor run; default = "parse"
error_kind = "PipelineValidation"
user_message_key = "error.pipeline.stage_type_mismatch"
```

`run_fixture` liest `mode` aus `FixtureExpected` und entscheidet datengetrieben.

**Impact:** Fixture-Renaming bricht Boot-Path-Test laut, nicht silent.

---

### P5 — `expected.toml` akzeptiert fehlende `error_kind`/`user_message_key` als Wildcard-Pass

**Severity:** IMPORTANT (Forcing-Sentinel-Pflicht: leere Felder degradieren Test zu no-op)
**Location:** `xtask/src/manifest_strict.rs:144-156`
**Source:** Blind Hunter

```rust
let kind_ok = expected.error_kind.as_deref().map(|k| ...).unwrap_or(true);
let msg_ok = expected.user_message_key.as_deref().map(|k| ...).unwrap_or(true);
```

Wenn ein Contributor eine neue `outcome = "err"`-Fixture ohne `error_kind` und ohne `user_message_key` einträgt, erlaubt der Harness *jeden* Error — passes silently. Validate-by-Construction:

**Fix:** Im `check_result` für `outcome = "err"` validieren, dass mindestens eines der beiden Felder gesetzt ist (besser: beide). Fixture-Author muss explizit deklarieren, was sie testet. Alternativ: TOML-Schema mit `#[serde(deny_unknown_fields)]` + Custom-Validation.

```rust
"err" => {
    if expected.error_kind.is_none() && expected.user_message_key.is_none() {
        eprintln!("[FAIL] {name}: 'err' outcome requires error_kind or user_message_key");
        return false;
    }
    // ... rest
}
```

**Impact:** Schließt das Wildcard-Loch; jede neue Fixture muss konkrete Erwartung haben.

---

### P6 — G3-A erfasst keine file-basierten `mod keys;`-Deklarationen

**Severity:** IMPORTANT (echtes Coverage-Loch — 4 betroffene Module in `klarvo-core`)
**Location:** `xtask/src/lint_events.rs:269-285, 311-322`
**Source:** Edge Case Hunter (E#14) — verifiziert via `grep "pub mod keys;"` in Code-Audit

Der `UserStringVisitor.in_keys_mod`-Flag wird nur in `visit_item_mod` gesetzt — und das feuert ausschließlich bei **inline** `mod keys { ... }`-Blöcken. File-basierte `mod keys;`-Deklarationen werden vom syn-Parser pro File separat geparst; das visited File ist selbst der `keys`-Modul-Inhalt, aber die AST hat keinen `ItemMod`-Node mit `ident = "keys"`. Resultat: const-Items in `keys.rs`-Files werden NIE in `code_keys` aufgenommen.

**Betroffen:**
- `klarvo-core/src/audio/keys.rs` (z.B. `DEVICE_UNAVAILABLE = "error.audio.device_unavailable"`, `UNSUPPORTED_FORMAT = ...`)
- `klarvo-core/src/output/keys.rs`
- `klarvo-core/src/keystore/keys.rs`
- `klarvo-core/src/v1_import/keys.rs`

Diese Keys sind heute *manuell* in `en.json` gepflegt (Story 4.4 AC-F `REQUIRED_KEYS`-Audit), aber G3-B Forward-Drift bietet null Schutz für diese Pfade.

**Fix-Optionen:**

1. **Filename-Heuristik** im Walker-Aufruf: Wenn das geparste File `keys.rs` heißt (oder Pfad endet auf `/keys.rs`), `in_keys_mod = true` als Initialwert setzen.
2. **Module-Resolution**: Vor `parse_file` lookup, ob ein Parent-`mod.rs`/`lib.rs` ein `pub mod keys;` deklariert. Aufwendig.

**Empfehlung:** Variante 1. Mechanisch in `walk_dir`-Loop: `let initial_in_keys = path.file_name() == Some("keys.rs");` und Visitor mit dieser Initialisierung starten.

**Impact:** Schließt G3-B Forward-Drift-Coverage für 4 file-basierte Key-Module; passt zur Klarvo-Konvention `pub mod keys;` aus `memory/project_keystore_trait_surface`.

---

### P7 — G3-B Violation-Message ohne file/line-Context (AC-D Wortlaut-Drift)

**Severity:** NIT (Dev-Experience; AC-D Beispiel deckt file:line, Impl droppt Origin)
**Location:** `xtask/src/lint_events.rs:1173-1175` (run_g3b_locale_cross_check Output)
**Source:** Acceptance Auditor

AC-D Beispiel-Output (`story-5.3 AC-D`):
```
key "error.stt.network" emitted in klarvo-plugins/klarvo-plugin-groq/src/lib.rs:42 but absent from en.json
```

Aktuelle Impl gibt:
```
key "error.stt.network" emitted in code but absent from en.json
```

`code_keys` ist `BTreeSet<String>` und droppt Origin-Info. Bei einem Drift-Violation muss der Dev manuell greppen, wo der Key herkommt.

**Fix:** `code_keys` zu `BTreeMap<String, Vec<(PathBuf, usize)>>` umstellen oder paralleles `code_key_origins`-Map mitführen. Bei Violation top-1 origin loggen.

**Impact:** Dev-Experience-Verbesserung; macht Gate-Failure direkt actionable.

---

## Decision-Needed Findings

### D1 — `bindings_drift::render()` ist nicht "rein ohne Datei-I/O" wie AC-A textlich fordert

**Source:** Blind Hunter (B#1) + Edge Case Hunter (E#7) + Acceptance Auditor

**Spec-Wortlaut (Story 5.2 AC-A, Lines 36-43):**
> `pub fn render() -> Result<String, ...>`, die den TS-Output **ohne Datei-I/O als String** zurückgibt.

**Implementation (`xtask/src/generate_bindings.rs`):**
`render()` ruft `cargo run --bin export-bindings` als Child-Process auf, das direkt nach `shells/windows/src/bindings/index.ts` schreibt — und liest die Datei dann zurück. Das ist real I/O, kein "reine Funktion". `bindings_drift::run()` snapshot-restored, was funktional korrekt ist, aber AC-A's literale Aussage stimmt nicht.

Story-File deklariert `Story-Spec-Abweichungen: None` — verstößt gegen `feedback_kickoff_deltas_only`.

Race-Risiko zusätzlich: Concurrent xtask-Invocations racen auf `index.ts` (E#7).

**Optionen:**
1. **Spec-Amendment**: AC-A umformulieren zu *"`render()` gibt den TS-Output als String zurück; etwaige Datei-I/O des darunterliegenden `export-bindings`-Binaries wird via Snapshot-Restore in `bindings_drift::run()` kompensiert"*. Dokumentiere im Story-File + Memory.
2. **Refactor**: `render()` schreibt zu Tempfile (z.B. `target/xtask-bindings/index.ts.tmp`), liest zurück, behält `index.ts` original. Erfordert `export-bindings`-Binary mit konfigurierbarem Output-Pfad — Story 5.2 Technical Notes (Lines 174-186) listet das als Architektur-Möglichkeit (`render()`-Optionen 1/2/3), aber "abhängig von export-bindings-Architektur".
3. **Alt-Path via env-var**: `KLARVO_BINDINGS_OUT=/tmp/...` an export-bindings durchreichen (falls supported); fallback Snapshot-Restore. Hybrid.

**Empfehlung-Frage:** Variante 1 (Amendment) ist günstig + ehrlich; Variante 2 ist sauber aber Tooling-Aufwand. Welche?

---

### D2 — G3-A `in_keys_mod`-Restriction ist undokumentiertes Spec-Narrowing

**Source:** Acceptance Auditor + Blind Hunter (B#10)

Diff fügt einen `in_keys_mod`-Flag (`xtask/src/lint_events.rs:269-285`) ein, der const-Collection NUR in `mod keys { ... }`-Blöcken erlaubt. Story 5.3 AC-B sagt nur *"alle gefundenen i18n-Keys"* — keine Mod-Restriction.

Die Persisted-Spec-Deltas im Audit-Briefing nennen die KEY_REGEX-Pattern-Breiterung als accepted, aber die `in_keys_mod`-Restriction NICHT. Das verletzt `feedback_kickoff_deltas_only` (Surprises sofort als Amendment/Memory persistieren) — Story-File deklariert `Story-Spec-Abweichungen: None`.

Plus: P6 (file-basierte `mod keys;` werden nicht erfasst) ist eine direkte Folge dieser Restriction.

**Optionen:**
1. **Amendment**: `in_keys_mod`-Restriction in AC-B als rationale Filter-Heuristik dokumentieren (Begründung: vermeidet false-positives wie `"com.klarvo.voice"`, `"config.json"` aus anderen Konstanten). Memory-Entry persistieren.
2. **Restriction-Removal**: `in_keys_mod` entfernen; alle `pub const … : &str = "..."` mit KEY_REGEX-Match einsammeln. Riskiert false-positive-Treffer auf nicht-i18n-Konstanten (Plugin-Identifier, Config-Pfade).
3. **Hybrid + P6-Fix**: Restriction behalten + P6 mit Filename-Heuristik schließen. Sauberster Effekt; kombinierbar mit Amendment für Restriction.

**Empfehlung-Frage:** Variante 3 ist die qualifizierteste — Restriction bleibt mit Begründung, file-basierte Module werden via Filename-Heuristik erfasst. Decision needed: Variante 1, 2 oder 3?

---

### D3 — G3-A AC-B 3rd Position (`#[default = "..."]`-Attribute) nicht implementiert

**Source:** Acceptance Auditor

AC-B (Story 5.3) listet drei positive-case-Positionen:
1. `AppError { user_message: Some(<lit>) }` — implementiert
2. Struct-Field-Default-Attributes `#[default = "..."]` auf Event-Structs — **nicht implementiert**
3. "Weitere direkte `user_message`-Zuweisungen via Literal" — implementiert (durch `ExprStruct`-Visit)

Aktuell keine Emit-Sites mit `#[default = "..."]` im Code, also keine Production-Lücke heute. Story-File sagt `Story-Spec-Abweichungen: None`. Bei zukünftigem Code-Pattern entsteht Lücke.

**Optionen:**
1. **Spec-Amendment**: Position 2 aus AC-B streichen — Begründung "kein Code-Pattern in Klarvo nutzt `#[default = "..."]`-Attribute". Story als Amendment + Memory-Entry.
2. **Implementation**: `visit_attribute` ergänzen, scan auf `default`-Attribute mit String-Literal-Argument. ~30 LOC.
3. **Defer**: Backlog-Eintrag, future-fix wenn Pattern auftritt. Risiko: silent escape-route.

**Empfehlung-Frage:** Variante 1 ist günstig (kein Code, kein Risiko). Variante 2 ist 30 LOC + Tests. Welche?

---

## Deferred Findings (Phase-2 oder pre-existing / dokumentierte Limitation)

- **F14 — verify-release rustup-vs-target-Distinktion** [`xtask/src/verify_release.rs:212-219`] — Bei fehlendem `rustup` wird "target not installed" gemeldet statt "rustup not found". Lower-priority — Dev-Experience.
- **F15 — verify-release Windows CRLF auf `rustup target list`** [`xtask/src/verify_release.rs:251-260`] — `lines()` handled CRLF, aber `trim()`-fehlt explizit. False-Positive-Risk auf Windows-Hosts.
- **F16 — `cargo check` ≠ Build (kein NDK-Linker im Check)** [`xtask/src/verify_release.rs:222-247`] — Gate ist green ohne Link-Path-Validation. Story 5.4 Technical-Notes erwähnen das als bewusste Wahl (CI-Speed). Dokumentiert.
- **F17 — Shared `target/`-Dir Race bei concurrent xtask-Runs** [`xtask/src/verify_release.rs:222-247`] — Cross-Compile poisons incremental-state. CARGO_TARGET_DIR-Isolation wäre Phase-2.
- **F18 — xtask Cargo-Features baken Phase-1-Stages ein** [`xtask/Cargo.toml:11-13`] — `stage-passthrough/stt/cleanup` hardcoded; Phase-2 stage-llm würde nicht erkannt. Re-evaluieren bei Stage-Erweiterung.
- **F19 — G3-A `is_excluded_g3` Scope-Inkonsistenz** [`xtask/src/lint_events.rs:545`] — Excludiert `android/`, nicht `klarvo-bridge-jni` oder deeper-nested `target/`. Pre-existing Convention-Drift.
- **F20 — G3-C Heuristic-Edge-Cases** [`xtask/src/lint_events.rs:466-507`] — Nested wildcard-arms, `use PipelineStageType as Pst`-Aliase. Story 5.3 Technical-Notes flag das explizit als Heuristik-Toleranz.
- **F21 — G3-B Locale-JSON nested/BOM/CRLF/Duplicate-Keys** [`xtask/src/lint_events.rs:340-357`] — Aktuelle Locale-Files sind flat JSON ohne BOM. Future-format-shifts würden break.
- **F22 — G3-B Forward-Drift fängt nur Literal-Keys** [`xtask/src/lint_events.rs:149`] — `format!("error.{}.network", ...)` und Const-Imports werden nicht erfasst. Pre-existing-Limitation; Story 5.3 Technical-Notes-akzeptiert.
- **F23 — G3-A `extract_string_literal_from_expr` recurses ohne Path-Validation** [`xtask/src/lint_events.rs:355`] — `Some(SomeOtherCtor("x"))` würde "x" extrahieren. Niedrig: nur `String::from`/`Cow::*`/`.into()` kompiliert für `Option<Cow<'static, str>>`-Field. Defer-Patch: Path-Segment-Allowlist.
- **F24 — manifest-strict missing fixture-File silent inconsistency** [`xtask/src/manifest_strict.rs:75`] — `continue` ohne Counter-Decrement; `total = expected.len()`. Final-Count `N-1/N` ohne klaren Cause. Phase-2 sauberer Fail-Path.

---

## Dismissed Findings

- AC-C uses `is_key`-Wrapper instead of `KEY_REGEX`-Constant directly — Spirit gewahrt (no duplication), `is_key` verwendet intern KEY_REGEX. Literal AC-C-Wortlaut-Toleranz.
- Help-Text Phase-0-Branding-Inconsistency (B#21) — Cosmetic.
- Self-attested test-counts in story-files (`26/26 green` vs `14 total tests`) (B#24) — Story-MD-Content; Counts beziehen sich auf unterschiedliche Test-Slices (Story-Tests vs. Cumulative).
- Tokio-Runtime `expect()` panic on Builder-Failure (B#17) — Resource-exhausted in CI: panic ist akzeptables Failure-Mode (non-zero exit signaled).
- Path-Traversal in fixture-name (E#21) — Fixtures sind dev-controlled; Threat-Model nicht anwendbar.
- AC-D fragile expected-key-Choice für `bad-missing-schema-version` (B#22) — AC-D explizit ODER-toleriert beide Keys; `expected.toml` Pick-by-Empirie ist legitim.
- Help-Text `Planned (stubs)` (re-checked) — Aktuelle Liste korrekt; manifest-strict + bindings-drift in Active-Section.
- `is_cfg_test_mod` substring "test"-Greedy-Match (B#9) — Edge-Case ohne real-world-Trigger; defer if needed.
- Concurrent xtask invocations on bindings_drift snapshot/restore race (E#7) — Subset von F17.
- AppError inside non-cfg-test test-helper module (E#16) — Workaround vorhanden (cfg(test) oder string-extraction); future-fix.
- `klarvo-plugins` non-Rust subdirs / symlinks (E#10) — Pre-existing walker behavior; conventions enforce flat plugin-layout.

---

## Files Referenced for Fixes

| Finding | File | Line |
|---------|------|------|
| B1 | `xtask/src/bindings_drift.rs` | 73-100 |
| P1 | `xtask/src/bindings_drift.rs` | 41, 53 |
| P2 | `xtask/src/main.rs` | 13, 25-29 |
| P3 | `xtask/src/manifest_strict.rs` | 200 |
| P4 | `xtask/src/manifest_strict.rs` | 96 |
| P5 | `xtask/src/manifest_strict.rs` | 144-156 |
| P6 | `xtask/src/lint_events.rs` | 269-285, 311-322 |
| P7 | `xtask/src/lint_events.rs` | 1173-1175 |
| D1 | `xtask/src/generate_bindings.rs`, AC-A wording | render() |
| D2 | `xtask/src/lint_events.rs`, AC-B wording | 269-322 |
| D3 | AC-B wording / `xtask/src/lint_events.rs` | n/a |
