# Story 1.5: Migration-ladder regression test — history-DB `open_db()`

Status: done

## Story

As a klarvo maintainer,
I want the real schema-migration ladder exercised by a test,
so that a regression in the v1-DB upgrade path (which today has only false safety) fails CI instead
of silently corrupting an existing user's history.

## Acceptance Criteria

**AC-1 — Real ladder is called, not bypassed**
Given the existing test helper `mem_db()` (`history/mod.rs:517-550`) builds the END schema
directly and bypasses the real `open_db()` migration ladder,
When a new test `test_open_db_runs_migration_ladder` runs,
Then it constructs an OLD pre-migration schema (see Dev Notes for exact DDL), inserts at least one
existing row, and calls the REAL `open_db()` on that database file — NOT `mem_db()`.

**AC-2 — Post-migration schema is complete**
Given the OLD schema is missing `uuid`, `device_id`, and `synced` columns,
When `open_db()` returns,
Then all three columns exist on the `history` table (verifiable via `PRAGMA table_info(history)`
or a SELECT probe).

**AC-3 — UUID backfill ran**
Given the pre-existing row has no uuid (NULL),
When `open_db()` returns,
Then the row's `uuid` column is non-NULL and is a valid UUID v4 string (non-empty, 36 chars,
matches `xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`).

**AC-4 — Unique index on uuid exists**
Given the uuid migration includes `CREATE UNIQUE INDEX IF NOT EXISTS idx_history_uuid ON history(uuid)`,
When `open_db()` returns,
Then the index exists (verifiable via `PRAGMA index_list(history)` or a duplicate-insert probe).

**AC-5 — Test is non-tautological (capable of catching a real regression)**
Given a second variant test `test_open_db_migration_ladder_is_non_tautological`,
When it creates the same OLD schema but calls `open_db()` only after deliberately omitting the uuid
migration step from the scheme — i.e., the test DOES NOT call open_db but instead asserts that
the OLD schema has no uuid column,
Then it PASSES (confirming the old schema truly lacks uuid before migration).
*Rationale: this guards against the test accidentally asserting on the post-CREATE-TABLE schema
rather than on post-migration state.*

**AC-6 — No new test-only pub surface added**
Given the story adds only tests,
When the implementation is complete,
Then no new `pub` or `pub(crate)` items are introduced; `open_db` is already `pub` and
accessible from the test module without change.

## Tasks / Subtasks

- [x] Task 1: Write `test_open_db_runs_migration_ladder` (AC-1, AC-2, AC-3, AC-4)
  - [x] 1.1 Create a `tempfile::TempDir`, build OLD schema DDL in `{tempdir}/history.db`
  - [x] 1.2 Insert one row with bare minimum columns (text, style, language)
  - [x] 1.3 Drop the setup connection to release the file lock
  - [x] 1.4 Call `open_db(tempdir.path())` — assert returns `Ok`
  - [x] 1.5 Assert uuid, device_id, synced columns exist via SELECT probe
  - [x] 1.6 Assert the inserted row's `uuid` is non-NULL and 36 chars
  - [x] 1.7 Assert `idx_history_uuid` index exists via `PRAGMA index_list(history)`

- [x] Task 2: Write `test_open_db_migration_ladder_is_non_tautological` (AC-5)
  - [x] 2.1 Open `Connection::open_in_memory()`, create OLD schema
  - [x] 2.2 Assert that `uuid` column is NOT present (probe SELECT returns Err)
  - [x] 2.3 No call to `open_db`; this test is a baseline guard only

- [x] Task 3: Run and verify (AC-6)
  - [x] 3.1 `cargo test history` — 26/26 history tests pass (26 = 24 pre-existing + 2 new), 539 total / 0 fail
  - [x] 3.2 `cargo clippy -- -D warnings` — clean (0 new warnings in history/mod.rs)

## Dev Notes

### Critical constraint: `open_db` requires a real file, not in-memory

`open_db(app_data_dir: &Path)` calls `Connection::open(path)` on a file path
(`history/mod.rs:113-114`). There is no in-memory override. The test MUST use `tempfile::TempDir`
to create a real temporary directory; `tempfile` is already a dev-dependency in `Cargo.toml`.

Pattern (verified from Story 1.2 which used TempDir for config backup tests):

```rust
let dir = tempfile::TempDir::new().unwrap();
// ... set up old schema in dir.path() ...
let conn = open_db(dir.path()).unwrap();
// assertions ...
// dir drops at end of test, cleaning up the file
```

### OLD pre-migration schema DDL

The "old" schema is the state of `history.db` before ANY of the five migrations in `open_db`
ran. Based on reading `history/mod.rs:113-186`, the original CREATE TABLE only established:

```sql
CREATE TABLE history (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    text       TEXT NOT NULL,
    raw_text   TEXT,
    style      TEXT NOT NULL DEFAULT 'polished',
    language   TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_history_created_at ON history(created_at DESC);
```

Key: columns NOT present in this OLD schema (and therefore migration targets):
- `is_note` — added by migration 1 (`ALTER TABLE history ADD COLUMN is_note ...`)
- `app_name` — added by migration 2
- `uuid` — added by migration 3 (+ UUID backfill + unique index) ← most critical
- `device_id` — added by migration 4
- `synced` — added by migration 5

IMPORTANT: Do NOT use `CREATE TABLE IF NOT EXISTS` in the setup helper — the old DB would have
created the table unconditionally at the time. Using `CREATE TABLE` (without IF NOT EXISTS) in
the setup prevents accidentally creating the new schema.

### Migration ladder (`open_db` lines 137–186)

```
open_db()
  ├─ CREATE TABLE IF NOT EXISTS history (base columns incl. is_note, app_name) [line 117]
  ├─ Migration 1: ADD COLUMN is_note  [if SELECT is_note fails]  [line 137]
  ├─ Migration 2: ADD COLUMN app_name [if SELECT app_name fails] [line 143]
  ├─ Migration 3: ADD COLUMN uuid     [if SELECT uuid fails]     [line 149]
  │   ├─ UUID backfill: SELECT id WHERE uuid IS NULL, UPDATE each row [line 153-162]
  │   └─ CREATE UNIQUE INDEX idx_history_uuid [line 163]
  ├─ Migration 4: ADD COLUMN device_id [if SELECT device_id fails] [line 167]
  └─ Migration 5: ADD COLUMN synced    [if SELECT synced fails]    [line 173]
```

Because the OLD schema has a `history` table, `CREATE TABLE IF NOT EXISTS` is a no-op (table
exists). Migrations 1–5 ALL fire because the OLD schema lacks all five columns.

### Asserting column existence

Use `PRAGMA table_info(history)` which returns rows with columns `cid, name, type, notnull, dflt_value, pk`:

```rust
let has_uuid = conn
    .prepare("SELECT uuid FROM history LIMIT 0")
    .is_ok();
assert!(has_uuid, "uuid column must exist after migration");
```

This mirrors the exact pattern used in `open_db` itself — consistent, zero additional API surface.

### Asserting the unique index

```rust
let mut stmt = conn.prepare("PRAGMA index_list(history)").unwrap();
let index_names: Vec<String> = stmt
    .query_map([], |row| row.get::<_, String>(1))
    .unwrap()
    .filter_map(|r| r.ok())
    .collect();
assert!(
    index_names.iter().any(|n| n == "idx_history_uuid"),
    "idx_history_uuid must exist after migration"
);
```

### UUID format assertion

```rust
let uuid_val: Option<String> = conn
    .query_row("SELECT uuid FROM history WHERE id = ?1", [row_id], |r| r.get(0))
    .unwrap();
let uuid_str = uuid_val.expect("uuid must not be NULL after backfill");
assert_eq!(uuid_str.len(), 36, "uuid must be 36-char v4 format");
// Basic hyphen-position check (not full regex to keep deps zero):
assert_eq!(&uuid_str[8..9], "-");
assert_eq!(&uuid_str[13..14], "-");
assert_eq!(&uuid_str[18..19], "-");
assert_eq!(&uuid_str[23..24], "-");
```

### AC-5: Non-tautology guard test

This test does NOT call `open_db`. It creates an in-memory DB, builds the OLD schema, and
asserts that uuid is absent. This proves that the main test's pre-condition is genuine:

```rust
#[test]
fn test_open_db_migration_ladder_is_non_tautological() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            raw_text TEXT,
            style TEXT NOT NULL DEFAULT 'polished',
            language TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );"
    ).unwrap();
    // uuid column must NOT exist in the old schema
    let has_uuid = conn.prepare("SELECT uuid FROM history LIMIT 0").is_ok();
    assert!(!has_uuid, "old schema must not have uuid — pre-condition guard");
}
```

### No new pub surface

`open_db` is already `pub`. Tests live inside `mod tests { use super::*; }`. No visibility
changes needed.

### Insertion helper for setup

To insert a row into the OLD schema (which has no uuid/device_id/synced columns), use bare SQL
rather than `add_entry` (which would fail because `add_entry` inserts uuid). Example:

```rust
conn.execute(
    "INSERT INTO history (text, style, language) VALUES (?1, ?2, ?3)",
    ["Test entry", "verbatim", "en"],
).unwrap();
let row_id = conn.last_insert_rowid();
```

### Files to modify

- `src-tauri/src/history/mod.rs` — add 2 new `#[test]` functions inside `mod tests`
  (lines ~855 = end of file, after the last test). No other file touched.

### Project Structure Notes

- All changes are in `src-tauri/src/history/mod.rs` only.
- `tempfile` is already in `[dev-dependencies]` in `src-tauri/Cargo.toml` — no Cargo change.
- `rusqlite = { version = "0.32", features = ["bundled"] }` already in dependencies.
- Tests live in the existing `#[cfg(test)] mod tests { ... }` block (line ~514).

### References

- `open_db()`: `src-tauri/src/history/mod.rs:112-186`
- `mem_db()` (existing helper): `src-tauri/src/history/mod.rs:517-550`
- `add_entry()`: `src-tauri/src/history/mod.rs:201-228`
- TEST-03: `docs/robustness-audit-2026-05-30.md` §4, row 3
- ADR-0015 (Next-Action #2 — TEST-03 lives in Epic 1, NOT Epic 3): `docs/adr/0015-state-file-write-convention.md`
- Epics: `_bmad-output/planning-artifacts/epics.md` §Story 1.5 (line ~381)
- Story 1.4 (context: migration hardening covered on config side): `_bmad-output/implementation-artifacts/1-4-*.md`
- tempfile TempDir pattern: Story 1.2 dev notes

### Anti-patterns to avoid

- **Do NOT use `mem_db()` in the new test.** The entire point is to call the real `open_db()` on
  a real file, not bypass it.
- **Do NOT call `add_entry()` on the pre-migration connection.** `add_entry` expects uuid/device_id
  columns. Insert with bare SQL on the old-schema connection.
- **Do NOT forget to drop the setup connection before calling `open_db()`.** rusqlite's bundled
  SQLite (WAL mode or default journal mode) allows only one writer at a time. Drop the setup
  `Connection` (or let it go out of scope in a block) before calling `open_db(dir.path())`.
- **Do NOT assert on migration details inside `test_open_db_migration_ladder_is_non_tautological`.**
  That test is a pure pre-condition guard; any `open_db` call would defeat its purpose.

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (create-story + dev-story, 2026-05-31)

### Debug Log References

### Completion Notes List

- Added `test_open_db_runs_migration_ladder`: uses `tempfile::TempDir` to create a real `history.db`
  with the 6-column OLD schema (no is_note/app_name/uuid/device_id/synced), inserts a bare-SQL row,
  drops the setup connection, then calls `open_db(dir.path())`. Asserts: uuid/device_id/synced columns
  exist (SELECT probe), existing row uuid is non-NULL 36-char v4, `idx_history_uuid` index exists
  (PRAGMA index_list). All 5 migration steps fire because the OLD schema table already exists
  (CREATE TABLE IF NOT EXISTS is a no-op).
- Added `test_open_db_migration_ladder_is_non_tautological`: in-memory only, builds OLD schema,
  asserts uuid column is absent. Confirms pre-condition is genuine — the main test truly starts
  from a pre-migration state.
- No production code changes. No new pub surface. Cargo.toml unchanged (tempfile already dev-dep).
- 539 lib tests / 0 fail (+2 from 537). Clippy clean.

### File List

- `src-tauri/src/history/mod.rs` — 2 tests added in `mod tests` block

## Review Findings

Code review 2026-05-31 (3 adversarial layers: Blind Hunter / Edge Case Hunter / Acceptance Auditor, Opus 4.8). Tests verified green first-hand: `cargo test --lib history` → 26 passed / 0 failed, 539 total. All 6 ACs satisfied as written; findings below strengthen the regression value of the new tests (the story's stated purpose: "fail CI instead of silently corrupting an existing user's history").

- [x] [Review][Patch] AC-4 verifies the index NAME but not that it is UNIQUE [src-tauri/src/history/mod.rs:927-937] — `PRAGMA index_list` returns the `unique` flag in column index 2; the test reads only column 1 (name). A regression that recreated `idx_history_uuid` as a non-unique index would pass. Since the index is the dedup mechanism for cross-device sync, this is in the story's silent-corruption threat model. Fix (unambiguous): assert the `unique` flag (col 2) == 1, or — stronger, and AC-4 explicitly offers it — a duplicate-uuid INSERT probe that must fail (this also kills the "constant-UUID backfill invisible with one row" concern).
- [x] [Review][Patch] Pre-existing row data-survival and row count not asserted [src-tauri/src/history/mod.rs:895-925] — the test checks only the `uuid` of the row by id; it never asserts `text`/`style`/`language` survived nor that `COUNT(*) == 1`. A migration that dropped/recreated the table (or duplicated rows) but left a valid-UUID row at `row_id` would pass — exactly the "silently corrupting an existing user's history" failure the story exists to catch. Fix: assert `text == "Migration test entry"` (+ style/language) and `COUNT(*) == 1` after `open_db()`.
- [x] [Review][Patch] Non-tautology guard tests a re-declared copy, not the migrated file-DB [src-tauri/src/history/mod.rs:940-962] — `test_open_db_migration_ladder_is_non_tautological` re-declares its own OLD-schema string; if the main test's OLD DDL ever drifts, the guard still passes (coupled-by-convention, not construction). The OLD-schema DDL is also duplicated verbatim across both tests. Fix: extract a shared `const OLD_SCHEMA_DDL: &str` used by both, and add an inline precondition assertion in the main test (before `open_db()`) that the file-DB lacks `uuid`. The separate guard test stays (AC-5 mandates it). NOTE: this strengthens beyond AC-5 as written.
- [x] [Review][Patch] Migrations 1 & 2 (is_note, app_name) unasserted; comment claims "all five must be present" [src-tauri/src/history/mod.rs:898-908] — the doc comment states all five migrated columns must exist, but only uuid/device_id/synced are probed; is_note and app_name have zero post-migration assertions (AC-2 literally requires only the three, so this is not an AC violation). Fix: add `SELECT is_note`/`SELECT app_name` existence probes (makes the comment honest and covers migrations 1–2).

**Resolution:** all 4 patches applied 2026-05-31. Shared `OLD_SCHEMA_DDL` const + inline file-DB precondition assert (F3); 5-column existence loop + COUNT/data-survival assertions (F2, F4); UNIQUE-flag check via `PRAGMA index_list` col 2 (F1). Re-verified: `cargo test --lib history` → 26 passed / 0 failed (539 total); clippy clean on `history/mod.rs`.

### Dismissed (9, recorded for audit)

- Byte-slicing `&uuid_str[8..9]` panic risk — verified safe: `len()==36` check precedes the slices; source is `Uuid::new_v4()` (ASCII-guaranteed). (Blind + Edge)
- OLD schema omits is_note/app_name but ladder still adds them — verified safe: base `CREATE TABLE IF NOT EXISTS` is a no-op on the existing table; the five ALTERs fire. (Edge, `mod.rs:108-117`)
- UUID backfill targets the pre-existing row — verified safe: backfill runs `WHERE uuid IS NULL` after the ALTER; `row_id` lookup is meaningful. (Edge, `mod.rs:152-162`)
- Residual `-wal`/`-shm` / lock contention — verified safe: `open_db()` uses default DELETE journal, no sidecars; setup conn dropped before `open_db()`. (Edge)
- Imports (`params!`, `Connection`, `Uuid`, `tempfile`) resolve — verified safe via `use super::*`. (Edge)
- Determinism — verified safe: TempDir per run, single row, only structural UUID shape asserted. (Edge)
- `query_row().unwrap()` panics instead of clean assert-fail if row vanished — noise: the test still fails correctly; diagnostic-only, and the row-survival patch (F2) surfaces it cleanly. (Blind)
- No second-open idempotency check — out of scope: story targets old→new upgrade; ladder idempotency is verified-safe by construction. Capture as a separate test-coverage story if desired. (Edge)
- AC-3 does not assert the v4 version nibble at position 14 — justified: Dev Notes explicitly relaxed full-regex "to keep deps zero"; production source is `Uuid::new_v4()`. AC text and Dev Notes diverge; code follows Dev Notes. (Auditor)
