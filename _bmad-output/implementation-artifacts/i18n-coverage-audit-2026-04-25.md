# i18n Key Coverage Audit — 2026-04-25

**Story:** 4.4 — i18n-Key-Coverage-Audit (FR28/FR30/FR31)
**Auditor:** Story-4.4-Impl-Agent
**Date:** 2026-04-25

---

## Audit Method

Grepped all `user_message: Some(...)` emit sites and `pub const *: &str = "error.*"` key
constants across `klarvo-core/`, `klarvo-plugins/`, and `shells/windows/src-tauri/src/`.

---

## Key Inventory (by source)

### `shells/windows/src-tauri/src/`

| Key | Source | Line |
|-----|--------|------|
| `error.config.missing` | config.rs | :92, :159 |
| `error.config.unknown_field` | config.rs | :106 (conditional) |
| `error.config.invalid_language` | config.rs | :127 |
| `error.audio.start_failed` | klarvo-shell-orchestrator/src/session.rs | :109 |
| `error.config.output_target_not_found` | klarvo-shell-orchestrator/src/session.rs | :164 |
| `error.paste.send_input_failed` | paste.rs | :74 |
| `error.keystore.read_failed` | keystore.rs | :67 |
| `error.hotkey.parse_failed` | hotkey.rs | :45 |
| `error.hotkey.registration_failed` | hotkey.rs | :68 |
| `tray.menu.exit` | main.rs | :185 (lookup) |

### `klarvo-core/src/`

| Key | Constant / Source | Line |
|-----|-------------------|------|
| `error.keystore.not_found` | keystore/keys.rs::KEY_NOT_FOUND | :11 |
| `error.keystore.backend_unavailable` | keystore/keys.rs::BACKEND_UNAVAILABLE | :15 |
| `error.keystore.key_missing` | error.rs | :106 (inline literal) |
| `error.pipeline.toml_parse_failure` | manifest.rs::keys::TOML_PARSE_FAILURE | :32 |
| `error.pipeline.schema_version_unsupported` | manifest.rs::keys::SCHEMA_VERSION_UNSUPPORTED | :34 |
| `error.pipeline.unknown_stage_type` | manifest.rs::keys::UNKNOWN_STAGE_TYPE | :36 |
| `error.pipeline.plugin_not_found` | pipeline/executor.rs::keys::PLUGIN_NOT_FOUND | :59 |
| `error.pipeline.stage_type_mismatch` | pipeline/executor.rs::keys::STAGE_TYPE_MISMATCH | :61 |
| `error.audio.device_unavailable` | audio/keys.rs::DEVICE_UNAVAILABLE | :5 |
| `error.audio.unsupported_format` | audio/keys.rs::UNSUPPORTED_FORMAT | :10 |
| `error.output.target_not_found` | output/keys.rs::TARGET_NOT_FOUND | :7 |
| `error.output.clipboard_unavailable` | output/keys.rs::CLIPBOARD_UNAVAILABLE | :8 |

### `klarvo-plugins/klarvo-plugin-groq/src/lib.rs`

| Key | Constant | Line |
|-----|----------|------|
| `error.stt.network` | keys::NETWORK | :52 |
| `error.stt.timeout` | keys::TIMEOUT | :53 |
| `error.stt.upstream_5xx` | keys::UPSTREAM_5XX | :54 |
| `error.stt.rate_limited` | keys::RATE_LIMITED | :55 |
| `error.stt.auth_failed` | keys::AUTH_FAILED | :56 |
| `error.stt.invalid_audio` | keys::INVALID_AUDIO | :57 |
| `error.stt.upstream_4xx` | keys::UPSTREAM_4XX | :58 |
| `error.stt.key_not_configured` | keys::KEY_NOT_CONFIGURED | :64 |

### `klarvo-plugins/klarvo-plugin-clipboard/src/lib.rs`

| Key | Source | Line |
|-----|--------|------|
| `error.output.clipboard_unavailable` | keys::CLIPBOARD_UNAVAILABLE | :33 |

---

## Delta vs. `locales/en.json` (pre-Story-4.4)

**Keys present in code but missing from en.json (before 4.4):**
- `error.keystore.key_missing`
- `error.pipeline.toml_parse_failure`
- `error.pipeline.schema_version_unsupported`
- `error.pipeline.unknown_stage_type`
- `error.pipeline.plugin_not_found`
- `error.pipeline.stage_type_mismatch`
- `error.audio.device_unavailable`
- `error.audio.unsupported_format`
- `error.output.target_not_found`
- `error.output.clipboard_unavailable`
- `error.stt.network`
- `error.stt.timeout`
- `error.stt.rate_limited`
- `error.stt.auth_failed`
- `error.stt.invalid_audio`
- `error.stt.key_not_configured`
- `error.stt.upstream_5xx`
- `error.stt.upstream_4xx`

**Orphan key in en.json removed by Story 4.4:**
- `error.config.invalid_locale` (successor: `error.config.invalid_language`, added Story 4.1)

---

## Story-Spec Delta Note

Story 4.4 AC-F REQUIRED_KEYS specified `error.stt.upstream_unavailable` (29 keys). Audit
found the actual Groq plugin uses `error.stt.upstream_5xx` (UPSTREAM_5XX) and
`error.stt.upstream_4xx` (UPSTREAM_4XX). REQUIRED_KEYS updated to 30 entries using
actual code values. The key `error.stt.upstream_unavailable` appears only in a test
fixture (klarvo-shell-orchestrator/tests/e2e_test.rs:247) as a mock — not a production emit.

---

## Closure

Story 4.4 adds 18 new keys to both locale files, removes 1 orphan (`error.config.invalid_locale`),
and introduces a 4-test coverage gate in `i18n::tests` (30 REQUIRED_KEYS). Gate is now green.

---

## Post-Audit Patch (Epic-4 Code-Review Follow-up, 2026-04-25)

The audit method (grep on `user_message: Some(...)` and `pub const *: &str = "error.*"`) missed
the `unwrap_or("error.internal")`-Fallback in `klarvo-shell-orchestrator/src/session.rs:148, 155, 176`.
`From<PluginError>` in `klarvo-core/src/error.rs:73-101` returns `user_message: None` for 5 of 6
variants, so this fallback is reached in production whenever the Groq plugin (or any future
plugin) propagates one of those errors without an explicit override.

**Closure:**
- `error.internal` added to `REQUIRED_KEYS` in `shells/windows/src-tauri/src/i18n.rs` (now 31 keys)
- EN string in `locales/en.json`
- DE string in `locales/de.json`
- Two Phase-2 backlog entries added (`docs/backlog.md`):
  1. PluginError-Variant-zu-i18n-Key-Mapping (root cause)
  2. Audit-Grep-Erweiterung für unwrap_or-Fallback-Keys (method gap for FR34 lint-gate)

Coverage gate is again green; Phase-1 surfaces a real (if generic) German/English string
instead of a raw key when those PluginError paths fire.
