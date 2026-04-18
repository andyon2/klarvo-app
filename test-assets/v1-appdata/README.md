# v1 AppData Fixture

Sanitized snapshot of a Klarvo v1 `%APPDATA%\com.klarvo.voice\` directory,
used by `klarvo-core` v1-import integration tests via
[`klarvo-test-fixtures`](../../klarvo-test-fixtures/).

**No real user data.** All texts are synthetic, UUIDs are `fixture-0000-NNNN`,
API keys contain the substring `FIXTURE_NOT_REAL_` and are not valid against
any service.

## Contents

- `history.db` — SQLite matching v1 schema. 3 rows in `history`, 2 rows in
  `usage`, `tips_shown` empty.
- `config.json` — representative settings; three keys set
  (`groq`, `deepseek`, `openrouter`), two empty (`openai`, `anthropic`).
- `dictionary.json` — 4 sample terms.

## Regenerating

`history.db` was generated with the sqlite3 CLI (see commit history for the
exact INSERT statements). JSON files are plain text and may be edited with any
editor — keep them in sync with the assertions in
`klarvo-core/tests/v1_import.rs`.
