# ADR-0004: v1→v2 Migration-Strategie — Parse-Only-Bundle, SecretString, Partial-Migration

**Status:** Accepted
**Date:** 2026-04-18

## Context

Concern #13 (architecture.md §v1→v2 Migration, Zeile 1256) fordert einen Einmal-Import-Pfad aus Klarvo v1 (`com.klarvo.voice`, `%APPDATA%\com.klarvo.voice\`) nach v2. v1 speichert in drei Files:

- `history.db` — SQLite mit `history`, `usage`, `tips_shown`
- `config.json` — `AppConfig` mit ~80 Feldern inkl. 5 Plain-String API-Keys (`groqApiKey`, `deepseekApiKey`, `openaiApiKey`, `anthropicApiKey`, `openrouterApiKey`)
- `dictionary.json` — `{ "terms": [String] }`

Phase-0-Status: v2 hat **kein** History-Target, **kein** Settings-Store, **kein** Dictionary-Store, **keine** OS-Keystore-Impl. Action-Item #10 (OS-Keystore) ist bewusst separate Achse. Die v2-Zielschemas werden in Phase 1 designt — Migration darf sie nicht vorab pinnen.

Gleichzeitig ist die Migration der Moment, in dem die v2-Security-Narrative („Keys gehören in OS-Keystore, nicht Plain-Storage" — `project_api_key_os_keystore_mvp.md`) enforcebar wird. Ein Plain-`String`-DTO würde die v1-Sünde wiederholen.

## Decision

### 1. Parse-Only-Bundle (keine v2-Stores schreiben)

`klarvo-core/src/v1_import/` parst v1-Daten und produziert ein `V1ImportBundle`-DTO im Speicher. **Keine Writes in v2-Stores** — die existieren in Phase 0 nicht. Phase-1-Task verdrahtet Bundle→Store-Writer, wenn die v2-Targets designt sind.

**Alternativen verworfen:**
- *v2-Target-Types parallel anlegen* → Scope-Creep, pinnt Phase-1-Design unter Migration-Druck.
- *Bundle + Keystore-Trait-Interface (ohne Impl) schon heute* → frisst Teile von Action-Item #10 (OS-Keystore-Design), verstößt gegen das Scope-Split.

### 2. API-Keys als `SecretString` im Bundle

Keys werden im Bundle in `SecretString` (via `secrecy`-Crate) gehalten, nicht als `String`. Invarianten:

- `Debug`/`Display`-Impl redacted durch `SecretString` automatisch → kein Tracing-Leak.
- Bundle darf nicht serialisiert werden (`#[derive(Serialize)]` NICHT auf `V1ImportBundle`).
- Phase-1-Writer MUSS Keys direkt in OS-Keystore schreiben. Plain-SQLite-Zwischenstopp ist verboten (`project_api_key_os_keystore_mvp.md`).

**Alternative verworfen:** Plain `String` mit „Comments-only"-Doku — unhaltbar bei Agent-Delegation, `secrecy`-Typen sind enforcebar.

### 3. Usage-Table: Import ins Bundle, Target-Mapping defer

v1's `usage`-Table (Cost-Tracking) wird ins Bundle übernommen (`V1UsageEntry`). v2-Usage-Schema existiert nicht — Mapping kommt mit Phase-1-Usage-Stage-Design. Kosten: 1 DTO-Feld, Preservation-Option bleibt offen.

**Architecture-Doc-Lücke:** Concern #13 listet „History, Dictionary, API-Keys, Hotkey-Config" explizit. Usage ist nicht genannt — dieses ADR schließt die Lücke: Usage ist First-Class-Bundle-Feld, aber Target-Mapping-Entscheidung ist Phase 1.

### 4. Partial-Migration-Semantik: Best-Effort per Sektion mit akkumulierten Warnings

Jede der vier Sektionen (Config, Keys, History, Dictionary) ist unabhängig partial-bar:

- Fehlt eine Datei → Sektion ist `None` im Bundle, Warning wird erfasst.
- Parse-Fehler einer Datei → Sektion ist `None`, Warning erfasst Datei-Pfad + Fehler.
- Der Import als Ganzes schlägt **nicht** fehl, solange mindestens eine Sektion lesbar war.
- Harter Fehler nur bei: AppData-Root existiert gar nicht **und** kein expliziter Pfad gegeben.

Rationale: v1-AppData kann korrupt sein (abgebrochener v1-Save, manuell editierte JSON, etc.). All-or-nothing würde bei einem kaputten File kompletten Import verhindern. Warnings werden in `V1ImportBundle.warnings: Vec<V1ImportWarning>` akkumuliert — Phase-1-Writer zeigt sie dem User.

**Fail-Modes sind explizit getestet** — Migration ohne Fail-Mode-Coverage ist exakt der v1-Silent-Assumption-Bug-Pattern, den wir vermeiden wollen.

### 5. Scope-Grenzen

- **Windows-only** (v1-AppData-Resolution via `com.klarvo.voice`). Android-v1-Migration ist Phase-3-Achse, separates ADR.
- **Keine OS-Keystore-Writes** — Action-Item #10.
- **Kein User-Prompt/UI-Flow** — Phase 1+.
- **Keine Änderungen in `src-tauri/`, `android/`, `src/`** — v1 ist nur Lese-Referenz (`project_v1_v2_coexistence.md`).

### 6. Pfad-Resolution: Split Production-Default + Parameterized

```rust
pub fn resolve_default_v1_path() -> Option<PathBuf>   // Windows: %APPDATA%\com.klarvo.voice\
pub fn load_from_path(appdata: &Path) -> V1ImportBundle  // Test-Injection
```

Standard-Pattern — Production nutzt Default-Resolver, Tests nutzen TempDir.

## Consequences

**Positiv:**
- Phase-0-Gate wird erfüllt ohne v2-Target-Schemas zu pinnen.
- API-Key-Security-Upgrade-Flow ist typed enforcebar (`SecretString`), nicht „per Doku".
- Fail-Mode-Coverage ab Tag 1 — Phase-1-Writer erbt Warnings-Channel, User-Sichtbarkeit kommt kostenlos.
- Usage-Preservation-Option bleibt offen, ohne Phase-1-Schema-Entscheidung vorzuziehen.

**Negativ:**
- Kein End-to-End-Roundtrip-Test in Phase 0 (Bundle wird nicht in Store geschrieben) — das vollständige v1→v2-User-Journey ist erst ab Phase 1 test-bar.
- `secrecy`-Crate ist zusätzliche Dep. Akzeptabel: gängige Crate (heavily-used), kleine Surface.
- Bundle-DTO-Schema ist Inter-Phase-Kontrakt — Phase-1-Writer darf Bundle-Felder voraussetzen. Breaking-Changes am Bundle = Migration-Behavior-Change.

**Mitigations:**
- Bundle-Felder sind explizit typisiert (keine `serde_json::Value`-Pass-through) → Phase-1-Schema-Evolution fängt compile-time-Breakage.
- Detail-Doc `docs/migration/v1-to-v2.md` dokumentiert Schema-Mapping-Tabellen für Phase-1-Writer.

## Referenzen

- `output/planning-artifacts/architecture.md` §Concern #13
- `docs/migration/v1-to-v2.md` (Schema-Mapping + Fail-Modes + User-Flow)
- `project_api_key_os_keystore_mvp.md` (Security-Narrative)
- `project_v1_v2_coexistence.md` (Repo-Struktur)
- `reference_klarvo_v1_tauri_identifier.md` (v1 = `com.klarvo.voice`)

## Next Action

1. Commit 1: Dieses ADR + `docs/migration/v1-to-v2.md`.
2. Commit 2: `klarvo-core/src/v1_import/` Module-Skelett + Unit-Tests.
3. Commit 3: `test-assets/v1-appdata/` Fixture + `klarvo-test-fixtures/src/v1_appdata.rs` Accessor + `klarvo-core/tests/v1_import.rs` Integration-Test (Happy + 3 Fail-Modes).
4. Phase-1-Item: Bundle→Store-Writer (wenn v2-Targets stehen) inkl. OS-Keystore-Write-Pfad (gating auf Action-Item #10).
