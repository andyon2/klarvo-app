# v1 → v2 Migration — Schema-Mapping, Fail-Modes, User-Flow

**Scope:** Windows-only (Klarvo v1 → Klarvo v2 auf derselben Maschine). Android-v1-Migration ist separate Phase-3-Achse.

**Strategie:** Parse-only (siehe [ADR-0004](../adr/0004-v1-to-v2-migration-strategy.md)). Dieses Dokument beschreibt das Bundle-Format + wie ein Phase-1-Writer Target-Stores befüllt.

## Source: v1-AppData-Layout

v1 liegt unter `%APPDATA%\com.klarvo.voice\` (Windows Roaming). Tauri-Identifier: `com.klarvo.voice` (siehe `reference_klarvo_v1_tauri_identifier.md`, v1-Quelle `src-tauri/tauri.conf.json:5`).

| Datei | Format | Source (v1-Code) |
|---|---|---|
| `history.db` | SQLite | `src-tauri/src/history/mod.rs` |
| `config.json` | JSON (camelCase) | `src-tauri/src/config/mod.rs` |
| `dictionary.json` | JSON | `src-tauri/src/dictionary/mod.rs` |

## Schema-Mapping

### 1. `history.db` — Tabelle `history`

v1-Columns (Reihenfolge wie in `row_to_entry`):

| v1-Column | Type | Bundle-Feld (`V1HistoryEntry`) | Kommentar |
|---|---|---|---|
| `id` | INTEGER PK | `id: i64` | — |
| `text` | TEXT NOT NULL | `text: String` | Cleaned text (nach Cleanup) |
| `raw_text` | TEXT | `raw_text: Option<String>` | Transcript vor Cleanup |
| `style` | TEXT NOT NULL DEFAULT 'polished' | `style: String` | `"polished" \| "verbatim" \| "chat"` — Phase-1-Mapping auf v2-Plugin-ID |
| `language` | TEXT NOT NULL DEFAULT '' | `language: String` | ISO-639-1 oder `""` (auto) |
| `is_note` | INTEGER NOT NULL DEFAULT 0 | `is_note: bool` | — |
| `app_name` | TEXT | `app_name: Option<String>` | Foreground-Window-Title |
| `created_at` | TEXT NOT NULL | `created_at: String` | ISO-8601 UTC |
| `uuid` | TEXT | `uuid: Option<String>` | v4 UUID (Sync-Deduplication) |
| `device_id` | TEXT | `device_id: Option<String>` | v1-Sync-Feld |
| `synced` | INTEGER NOT NULL DEFAULT 0 | — | **Dropped.** v1-Sync-Relikt. v2 hat kein Remote-Sync (`project_no_remote_telemetry.md`). |

**Phase-1-Writer-Hinweis:** `style` ist v1 ein String-Enum. v2 macht Cleanup-Style zu Plugin-IDs — Mapping-Table in Phase-1-Writer:

- `"polished"` → `klarvo-plugin-polished`
- `"verbatim"` → `klarvo-plugin-verbatim`
- `"chat"` → `klarvo-plugin-chat`

Unbekannte Werte → `imported_from_v1_style: <original>`-Field + Default-Plugin.

### 2. `history.db` — Tabelle `usage`

v1-Columns:

| v1-Column | Type | Bundle-Feld (`V1UsageEntry`) |
|---|---|---|
| `id` | INTEGER PK | `id: i64` |
| `service` | TEXT NOT NULL | `service: String` (`"groq_stt"`, `"deepseek_cleanup"`, ...) |
| `audio_duration_ms` | INTEGER | `audio_duration_ms: Option<i64>` (STT only) |
| `prompt_tokens` | INTEGER | `prompt_tokens: Option<i64>` (LLM only) |
| `completion_tokens` | INTEGER | `completion_tokens: Option<i64>` (LLM only) |
| `estimated_cost_usd` | REAL NOT NULL DEFAULT 0 | `estimated_cost_usd: f64` |
| `created_at` | TEXT NOT NULL | `created_at: String` |

**Phase-1-Writer-Hinweis:** v2 hat noch keine Usage-Stage (`ADR-0004 §3`). Bundle hält Usage preserved — Target-Mapping wird designt, wenn v2-Usage-Schema steht.

### 3. `history.db` — Tabelle `tips_shown`

**Dropped.** v1-Onboarding-Tip-Tracking ist UI-State, nicht User-Data. v2-Onboarding startet frisch.

### 4. `config.json` → Settings + Keys

`config.json` vermischt Settings und API-Keys. Der Import **splittet** beim Lesen:

**Keys (5 Felder → `V1ApiKeys`, `SecretString`):**

| v1-Feld | Bundle-Feld | Target (Phase 1) |
|---|---|---|
| `groqApiKey` | `groq: Option<SecretString>` | OS-Keystore `klarvo.groq` |
| `deepseekApiKey` | `deepseek: Option<SecretString>` | OS-Keystore `klarvo.deepseek` |
| `openaiApiKey` | `openai: Option<SecretString>` | OS-Keystore `klarvo.openai` |
| `anthropicApiKey` | `anthropic: Option<SecretString>` | OS-Keystore `klarvo.anthropic` |
| `openrouterApiKey` | `openrouter: Option<SecretString>` | OS-Keystore `klarvo.openrouter` |

Empty-String-Felder werden zu `None` normalisiert (nie „leerer Secret"). **Phase-1-Writer-Pflicht:** direkt in OS-Keystore, nie Zwischenstopp in Plain-SQLite.

**Settings (alle übrigen Felder → `V1Settings`):**

Der Bundle-Typ `V1Settings` hält ein `serde_json::Value`-Object der verbleibenden (keys-entfernten) `config.json`-Felder. Phase-1-Writer mapt gegen das finale v2-Settings-KV-Schema. Dieses ADR pinnt das v2-Settings-Schema **nicht** — der Phase-1-Writer verwendet die v1-Feldnamen als Matching-Key und entscheidet per Rename-Tabelle.

Grund für `serde_json::Value`-Hold: v1 hat ~80 Felder mit eigener Default-Behavior, Deprecated-Tombstones und Platform-conditional-Fields. Ein Phase-0-Typed-Schema dafür zu pflegen bindet Design-Kapazität, die in Phase 1 besser investiert ist. Bundle-Konsumenten bekommen eine explizite `keys_stripped: bool`-Invariante mit.

### 5. `dictionary.json`

v1-Format: `{ "terms": [String] }` — sortierte, de-duplizierte User-Term-Liste.

| v1-Feld | Bundle-Feld (`V1Dictionary`) |
|---|---|
| `terms` | `terms: Vec<String>` |

**Phase-1-Writer-Hinweis:** Dictionary ist laut architecture.md plugin-owned (Custom Dictionary Plugin). Writer ruft `Plugin<Dictionary>::import_terms(terms)`.

## Fail-Modes

Jede Sektion parst unabhängig. Bundle akkumuliert Warnings in `warnings: Vec<V1ImportWarning>`.

| Szenario | Verhalten | Warning-Kind |
|---|---|---|
| AppData-Root fehlt (kein `com.klarvo.voice`-Dir) | `load_from_path` returnt leeres Bundle + 1 Warning | `AppDataMissing { path }` |
| `history.db` fehlt | `history: None`, `usage: None` + Warning | `FileMissing { file: "history.db" }` |
| `history.db` korrupt / nicht lesbar | `history: None`, `usage: None` + Warning mit Fehlermeldung | `ParseError { file, detail }` |
| `config.json` fehlt | `settings: None`, `api_keys: V1ApiKeys::empty()` + Warning | `FileMissing { file: "config.json" }` |
| `config.json` invalid JSON | `settings: None`, `api_keys: V1ApiKeys::empty()` + Warning | `ParseError { file, detail }` |
| `dictionary.json` fehlt | `dictionary: None` + Warning | `FileMissing { file: "dictionary.json" }` |
| `dictionary.json` invalid JSON | `dictionary: None` + Warning | `ParseError { file, detail }` |

**Harter Fehler (Bundle-Produktion schlägt fehl):** nur wenn `resolve_default_v1_path()` `None` zurückgibt UND kein expliziter Pfad gegeben wurde. Dann kann der Import-Flow nicht starten.

Innerhalb von SQLite: wenn die Tabelle existiert aber einzelne Rows korrupt sind (z. B. NULL in NOT-NULL-Spalte nach manueller DB-Bearbeitung), wird die fehlerhafte Row übersprungen + pro Row eine `Warning::RowSkipped { table, row_id_or_offset, detail }`. Das hält den Import robust gegen Teil-Korruption.

## User-Flow (Phase-1-Writer, informativ)

```
1. App-Startup prüft: v1-AppData vorhanden + kein v2-Migration-Flag?
   → Settings-Flag `migration.v1_import_completed: bool`
2. Wenn ja: Migration-Dialog anzeigen — "Möchtest du deine v1-Daten übernehmen?"
3. User akzeptiert:
   a. klarvo_core::v1_import::load_default() → Result<V1ImportBundle, ImportError>
   b. bundle.warnings anzeigen, Scroll-Liste
   c. Writer:
      - bundle.history → v2-History-Store (INSERT mit uuid als Idempotency-Key)
      - bundle.usage → v2-Usage-Store (falls existiert, sonst preserve als-is)
      - bundle.dictionary → v2-Dictionary-Plugin
      - bundle.settings → v2-Settings-KV (per Rename-Tabelle)
      - bundle.api_keys → OS-Keystore (SecretString-expose nur innerhalb write-Aufruf)
   d. Settings-Flag `migration.v1_import_completed = true`
4. v1-Daten bleiben auf Disk (User-Delete separate UI-Aktion).
```

**Idempotency:** History-UUID-Column ist v2-Unique-Key. Re-run würde bei INSERT OR IGNORE keine Duplikate erzeugen. Settings + Dictionary sind „replace"-semantisch. Keys sind „replace" in OS-Keystore.

## Nicht-Ziele

- Cloud-Sync v1→v2 (v1 hatte rudimentäres Sync-Feld, v2 hat kein Remote-Sync).
- v2→v1 Downgrade (one-way).
- Android-v1-Migration (Phase 3, separates ADR).
- Automatisches v1-Uninstall.
- Background-Migration (Migration ist expliziter User-Trigger).

## Referenzen

- [ADR-0004](../adr/0004-v1-to-v2-migration-strategy.md) — Strategie-Entscheidungen
- `output/planning-artifacts/architecture.md` — Concern #13
- v1-Source: `src-tauri/src/{history,config,dictionary}/mod.rs` (read-only reference, nicht bearbeiten)
