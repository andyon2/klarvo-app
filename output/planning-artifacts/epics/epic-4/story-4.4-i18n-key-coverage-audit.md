---
name: Story 4.4 — i18n-Key-Coverage-Audit (FR28/FR30/FR31)
epic: 4
story_number: "4.4"
status: Draft
dependencies:
  - "4.1"
---

# Story 4.4: i18n-Key-Coverage-Audit

## Outcome

Jede `AppError`-Variante, die aus `klarvo-core` oder einem Phase-1-Plugin User-facing
emittiert wird, hat ein nicht-`None` `user_message`-Field, dessen i18n-Key in beiden
Locale-Tabellen (`locales/en.json`, `locales/de.json`) registriert ist. Ein Test-Suite
fail-loud-validiert die Coverage; ein Audit-Pass schließt Lücken (`error.keystore.key_missing`,
`error.pipeline.*`, `error.stt.*`, `error.audio.device_unavailable`,
`error.output.clipboard_unavailable`). Cleanup: alter `error.config.invalid_locale`-Key
wird entfernt (Story 4.1 hat den Successor `error.config.invalid_language` eingeführt).
Implementiert FR28 (`PipelineValidation` keyed), FR30 (`KeyMissing` keyed), FR31 (alle
User-Errors keyed). FR34/G3 Lint-Gate (Epic 5) baut auf dieser Coverage auf — diese Story
liefert den Greenfield-Stand.

## Acceptance Criteria

### AC-A — Key-Inventar aus Code-Emit-Sites

**Given** `klarvo-core` und Phase-1-Plugins emittieren `AppError` mit `user_message`-Keys
über Konstanten und String-Literale
**When** Story 4.4 das Inventar erstellt
**Then**

- Eine Audit-Liste wird erstellt (im Story-Implementation-Notes-Abschnitt der Story oder
  als `_bmad-output/implementation-artifacts/i18n-coverage-audit-2026-04-XX.md`) mit
  allen aktuell emittierten i18n-Keys aus:
  - `klarvo-core/src/error.rs` (z. B. `error.keystore.key_missing` aus
    `PluginError::KeyMissing → AppError`-Mapping)
  - `klarvo-core/src/manifest.rs` (`error.pipeline.toml_parse_failure`,
    `error.pipeline.schema_version_unsupported`, `error.pipeline.unknown_stage_type`)
  - `klarvo-core/src/pipeline/executor.rs` (`error.pipeline.plugin_not_found`,
    `error.pipeline.stage_type_mismatch`)
  - `klarvo-core/src/keystore/keys.rs` (`error.keystore.not_found`,
    `error.keystore.backend_unavailable`)
  - `klarvo-core/src/audio/keys.rs` (`error.audio.device_unavailable`,
    `error.audio.unsupported_format`)
  - `klarvo-core/src/output/keys.rs` (`error.output.target_not_found`,
    `error.output.clipboard_unavailable`)
  - `klarvo-plugins/klarvo-plugin-groq/src/lib.rs` (`error.stt.network`,
    `error.stt.timeout`, `error.stt.rate_limited`, `error.stt.auth_failed`,
    `error.stt.invalid_audio`, `error.stt.key_not_configured`,
    `error.stt.upstream_unavailable`)
  - `shells/windows/src-tauri/src/*.rs` (Shell-emit-Sites: `error.config.*`,
    `error.audio.start_failed`, `error.paste.send_input_failed`,
    `error.keystore.read_failed`, `error.hotkey.*`, `tray.menu.exit`)
- Das Inventar wird verglichen gegen den Bestand in `locales/en.json` (und `de.json`,
  identische Key-Set-Erwartung)
- Die Diff-Liste landet im Audit-Doc

### AC-B — Bekannte Gap geschlossen: `error.keystore.key_missing`

**Given** `klarvo-core/src/error.rs:106` emittiert `user_message: Some("error.keystore.key_missing".into())`
**When** Story 4.4 schließt die Lücke
**Then**

- `locales/en.json` enthält:
  ```json
  "error.keystore.key_missing": "API key missing for plugin. Please configure the key in application settings."
  ```
- `locales/de.json` enthält den deutschen Pendant (Stil-Guideline aus Story 4.3 AC-B):
  ```json
  "error.keystore.key_missing": "API-Schlüssel für Plugin fehlt. Bitte hinterlegen Sie den Schlüssel in den Einstellungen."
  ```
- Wording ist Delegate-Choice solange Stil-Guideline aus 4.3 AC-B eingehalten wird

### AC-C — Pipeline-Fehler-Keys registriert

**Given** `manifest.rs` und `pipeline/executor.rs` emittieren 5 Pipeline-Validation-Keys
(FR28: `PipelineValidation` keyed)
**When** Story 4.4 die Locale-Files ergänzt
**Then**

- `locales/en.json` + `locales/de.json` enthalten:
  - `error.pipeline.toml_parse_failure`
  - `error.pipeline.schema_version_unsupported`
  - `error.pipeline.unknown_stage_type`
  - `error.pipeline.plugin_not_found`
  - `error.pipeline.stage_type_mismatch`
- Wording-Vorschläge (Delegate-Choice):
  - EN `toml_parse_failure`: "Pipeline configuration is invalid. Please check pipeline-manifest.toml."
  - EN `schema_version_unsupported`: "Pipeline schema version is not supported by this build."
  - EN `unknown_stage_type`: "Pipeline references an unknown stage type. Please check pipeline-manifest.toml."
  - EN `plugin_not_found`: "Pipeline references a plugin that is not registered."
  - EN `stage_type_mismatch`: "Pipeline stage types are incompatible (output/input mismatch)."
- DE-Pendants nach 4.3-Stil-Guideline (formell, technische Begriffe englisch)

### AC-D — Audio + Output + STT-Plugin-Keys registriert

**Given** Phase-1 hat Keys aus Audio/Output-Layer + Groq-STT-Plugin, die in Locales
fehlen
**When** Story 4.4 die Locale-Files ergänzt
**Then**

- **Audio (2 Keys):**
  - `error.audio.device_unavailable`
  - `error.audio.unsupported_format`
- **Output (1 Key — `target_not_found` ist potentiell deckungsgleich mit
  `error.config.output_target_not_found` aus Story 3.3, aber distinkt im Code-Pfad):**
  - `error.output.target_not_found`
  - `error.output.clipboard_unavailable`
- **STT (Groq-Plugin, 7 Keys — FR29 + FR30 deckungsgleich keyed):**
  - `error.stt.network`
  - `error.stt.timeout`
  - `error.stt.rate_limited`
  - `error.stt.auth_failed`
  - `error.stt.invalid_audio`
  - `error.stt.key_not_configured`
  - `error.stt.upstream_unavailable`
- Jeder Key hat einen englischen + deutschen Wert in `en.json` / `de.json`
- Wording-Vorschläge sind Delegate-Choice; Stil-Guideline aus 4.3 AC-B gilt

### AC-E — Cleanup: `error.config.invalid_locale` entfernt

**Given** Story 4.1 hat `error.config.invalid_language` als Successor eingeführt;
`error.config.invalid_locale` wird seit 4.1 vom Code nicht mehr emittiert
**When** Story 4.4 aufräumt
**Then**

- `locales/en.json` und `locales/de.json` enthalten **nicht** mehr den Key
  `error.config.invalid_locale`
- `cargo build -p klarvo-windows-shell` bleibt grün — kein Code referenziert den
  alten Key (Story 4.1 hat den Code-Site auf `error.config.invalid_language` umgestellt)
- Coverage-Audit-Test (AC-F) schlägt fehl, falls jemand versehentlich noch eine
  Reference einbringt

### AC-F — Coverage-Audit-Test (Workspace-Level, fail-loud)

**Given** Coverage-Drift muss mechanisch verhindert werden bis FR34/G3 Lint-Gate (Epic 5)
es übernimmt
**When** Story 4.4 einen Test einführt
**Then**

- **Test-Lokation:** `shells/windows/src-tauri/tests/i18n_coverage_test.rs` (Integration-Test
  oder `#[cfg(test)]`-Modul in `i18n.rs`, Delegate-Choice). Begründung: Shell ist der
  Resolver-Owner; Test gehört auf Resolver-Seite
- **Test 1 — `en.json` ist Superset aller emittierten Keys:**
  - Liste aller im Code emittierten Keys ist zentral als Konstante im Test geführt:
    ```rust
    const REQUIRED_KEYS: &[&str] = &[
        "error.config.missing",
        "error.config.unknown_field",
        "error.config.invalid_language",
        "error.config.output_target_not_found",
        "error.audio.start_failed",
        "error.audio.device_unavailable",
        "error.audio.unsupported_format",
        "error.paste.send_input_failed",
        "error.keystore.read_failed",
        "error.keystore.not_found",
        "error.keystore.backend_unavailable",
        "error.keystore.key_missing",
        "error.hotkey.parse_failed",
        "error.hotkey.registration_failed",
        "error.pipeline.toml_parse_failure",
        "error.pipeline.schema_version_unsupported",
        "error.pipeline.unknown_stage_type",
        "error.pipeline.plugin_not_found",
        "error.pipeline.stage_type_mismatch",
        "error.output.target_not_found",
        "error.output.clipboard_unavailable",
        "error.stt.network",
        "error.stt.timeout",
        "error.stt.rate_limited",
        "error.stt.auth_failed",
        "error.stt.invalid_audio",
        "error.stt.key_not_configured",
        "error.stt.upstream_unavailable",
        "tray.menu.exit",
    ];
    ```
  - Der Test parsed `en.json` und assert: jeder Key aus `REQUIRED_KEYS` ist Eintrag der
    Tabelle, fail-loud bei Missing
- **Test 2 — `de.json` deckt gleiches Key-Set ab:**
  - Beide Tables werden parsed; assert `en_keys == de_keys` (als `BTreeSet<String>`).
    Fail-loud bei Asymmetrie (en hat Key, den de nicht hat — oder umgekehrt)
- **Test 3 — Kein verwaister Key:**
  - assert: `en.json` enthält **nur** Keys aus `REQUIRED_KEYS` (Whitelist-Check).
    Fail-loud, wenn jemand einen Key in `en.json` einträgt, der nicht im Code referenziert
    wird (z. B. alter `error.config.invalid_locale` nach Cleanup-Vergessen)
- **Test 4 — Keine TODO-Marker in `en.json`:**
  - assert: kein Wert in `en.json` startet mit `TODO`. (Englisch ist die Authoritative
    Master-Sprache; TODO-Marker dort sind ein Bug.) `de.json` darf TODO-Marker enthalten
    (Story 4.3 Smoke), bis 4.3 mergt
- Diese Tests laufen in `cargo test -p <shell-crate>` und blockieren CI bei Drift
- Test-Code dokumentiert in Comment, dass `REQUIRED_KEYS` per-Story manuell synchronisiert
  wird, **bis** FR34 / Epic 5 G3 Lint-Gate eine `cargo xtask lint-events`-Pass einführt,
  die das automatisch via AST-Parse macht. Diese Story liefert den Greenfield-Stand für
  die G3-Pass

### AC-G — Bekannte Schwäche dokumentiert

**Given** der Coverage-Test in AC-F basiert auf einer manuell gewarteten Konstanten-Liste
**When** ein Plugin-Author ein neues Key-Konstanten-Symbol einführt
**Then**

- Die Story-Doc enthält im Implementation-Notes-Abschnitt eine explizite Notiz:
  ```
  Known limitation: REQUIRED_KEYS is manually maintained. New key constants in core or
  plugins MUST be added to REQUIRED_KEYS in the same PR — there is no AST-based
  drift-detection until Epic 5 FR34 (cargo xtask lint-events). PR-Reviewer should ask
  for the locale-file diff alongside any new error.* constant.
  ```
- Backlog-Eintrag wird in `docs/backlog.md` ergänzt: „Story 4.4 manueller Coverage-Test
  durch Epic 5 FR34 Lint-Gate ersetzen, sobald G3 ausgerollt ist."

### AC-H — `de.json` parallel zu Story 4.3

**Given** Story 4.3 übersetzt den **Bestand** (Stand 2026-04-25, 12 Keys)
**When** Story 4.4 zusätzliche ~17 Keys hinzufügt
**Then**

- Story 4.4 trägt **selbst** die deutsche Übersetzung der neuen Keys mit ein —
  4.3 deckt sie nicht ab (siehe 4.3 AC-A explizit)
- Wording folgt 4.3 AC-B Stil-Guideline (formell „Sie", technische Begriffe englisch)
- Wenn 4.3 noch nicht gemergt ist, akzeptiert 4.4 dass der **Bestand** noch
  `TODO(de):`-Marker hat — neue Keys haben **keine** TODO-Marker (4.4 schreibt
  finale deutsche Strings, 4.3 räumt die Bestand-TODOs nach)

## Technical Notes

### Audit-Doc-Lokation

`_bmad-output/implementation-artifacts/i18n-coverage-audit-2026-04-XX.md` (Datum aus
Implementation-Datum) ist der Artefakt-Ablage-Ort gemäß Memory `feedback_commit_hygiene`
für Audit-Outputs. Die Doc enthält:
1. Inventar aller emittierten Keys mit Source-File-Pointer (`klarvo-core/src/...:LineNumber`)
2. Diff vs. `en.json`-Bestand
3. Closure-Liste (welche Keys neu hinzugefügt, welche entfernt)

### `REQUIRED_KEYS` als Single-Source vs. AST-Parse

Manuelle Konstanten-Liste ist Phase-1-Pragma. FR34/G3 (Epic 5) wird `cargo xtask lint-events`
einführen, das alle `pub const FOO: &str = "error.x.y"`-Patterns plus String-Literale in
`AppError { user_message: Some(...) }`-Position via `syn`-AST-Parse extrahiert. Bis dahin
ist die Konstanten-Liste der mechanische Gate; sie ist explizit als Übergangsstadium
markiert (AC-G).

### `error.output.target_not_found` vs. `error.config.output_target_not_found`

Beide Keys existieren parallel:
- `error.config.output_target_not_found` (Shell-config-Layer, Story 3.3) — gefeuert wenn
  config.toml einen `output_target_id` enthält, der nicht in der Registry ist
- `error.output.target_not_found` (Core-output-Layer, `output/keys.rs`) — gefeuert wenn
  Plugin-Code einen Output-Target nicht auflösen kann (z. B. Pipeline-Manifest hat invalid
  target_id)

Beide bleiben distinkt — verschiedene Code-Pfade, verschiedene User-Sichten. Story 4.4
dokumentiert das im Audit-Doc, mergt sie **nicht**.

### FR29 (Groq 5xx) ist FR30/FR31-deckungsgleich, kein Extra-Key

FR29 spezifiziert `UpstreamUnavailable` keyed; das deckt sich mit `error.stt.upstream_unavailable`
und `error.stt.timeout`/`error.stt.network` aus Groq-Plugin (AC-D). Kein zusätzlicher
generischer `error.upstream.*`-Key — Plugin-spezifisch ist präziser für User-Diagnose.

### Test-Crate vs. Shell-Crate-internal

Der Test in AC-F läuft im Shell-Crate (`shells/windows/src-tauri/`), weil:
1. Shell ist der Resolver-Owner (FR27 + ADR-0009 SD-2)
2. Locale-Files sind Shell-owned (`shells/windows/locales/`)
3. `klarvo-core` darf keine Locale-Files referenzieren (G3-Constraint —
   `memory/project_i18n_core_contract`)

Test-Lokation in der Shell macht G3-Boundary-Compliance natürlich.

## Dependencies

- Story 4.1 (`error.config.invalid_language` Successor existiert; `error.config.invalid_locale`
  ist deprecated)
- `klarvo-core/src/error.rs` (PluginError → AppError-Mapping)
- `memory/project_i18n_core_contract` — Core emittiert Keys, Shell resolved; Test in Shell
- `memory/feedback_ci_gate_philosophy` — Preventive Enforcement (Coverage-Test ist Phase-1
  Stub für G3-Lint-Gate)
- `memory/feedback_commit_hygiene` — Audit-Artefakt unter `_bmad-output/implementation-artifacts/`
- Epic 5 FR34 (forward-ref) — G3 Lint-Gate ersetzt manuelle `REQUIRED_KEYS`-Liste in Phase-1+
