---
title: 'API-Key Debug-Redaction Guard'
type: 'refactor'
created: '2026-06-03'
status: 'done'
baseline_commit: '8a88554ff646054caf3af5f7c8cd951f729c4a5c'
context: ['{project-root}/_bmad-output/project-context.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `AppConfig` derives `Debug` (`config/mod.rs:450`) and holds 6 plaintext secrets — the 5 provider API keys (`groq/deepseek/openai/anthropic/openrouter`) plus `turso_token`. No code formats the config with `{:?}` today, so there is **no active leak** (audited 2026-06-03: no zip/log export exists, logs never print key values, the feedback webhook payload carries no keys/config). But the hook is open: any future `log::debug!("{cfg:?}")`, `dbg!()`, panic-with-config-in-context, or diagnostic dump would spill all 6 secrets in plaintext.

**Approach:** Replace the derived `Debug` on `AppConfig` with a manual impl that redacts the 6 secret fields, and add a regression test that fails if any secret value appears in `{:?}`/`{:#?}` — a forcing-sentinel against someone re-deriving `Debug` later. Serde, `Clone`, and `PartialEq` are left exactly as-is, so the `config.json` on-disk shape and the Android `KlarvoApi.kt` JSON reads are untouched.

## Boundaries & Constraints

**Always:**
- Redact ALL 6 secret fields: `groq_api_key`, `deepseek_api_key`, `openai_api_key`, `anthropic_api_key`, `openrouter_api_key`, `turso_token`.
- Preserve `Serialize`, `Deserialize`, `Clone`, `PartialEq` byte-for-byte — `config.json` round-trip and Android JSON reads must be unchanged.
- Use `.finish_non_exhaustive()` so any field not explicitly listed is omitted (safe-by-default for future secret fields) and the omission is visible as `..`.
- Debug may reveal whether a secret is set vs unset — never its value, and never a value-derived prefix/length.

**Ask First:**
- If you judge the heavier `Secret` newtype (see Design Notes) is worth it over a manual `Debug` impl, HALT and confirm before taking that path — it changes field types and touches assignment sites across multiple files.

**Never:**
- Do NOT change the on-disk JSON shape, field names, `#[serde(...)]` attributes, or `Default`.
- Do NOT touch the provider structs (`WhisperStt`, `OpenAiCompatibleCleanup`, `AnthropicCleanup`) — audited 2026-06-03: they hold `api_key: String` but derive no `Debug` and are never logged. Not a leak channel, out of scope.
- Do NOT build redaction for export/diagnostic paths — they don't exist (deferred slices in `deferred-work.md`).
- Do NOT add a keystore or move keys off disk (deferred).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Secret set | `groq_api_key = "gsk_live_SENTINEL"` | `{:?}` output excludes `"gsk_live_SENTINEL"`; field renders a redaction marker (e.g. `<set>`) | N/A |
| Secret empty | `turso_token = ""` | renders `<unset>`; no value emitted | N/A |
| Pretty-print | `{:#?}` over full config | same redaction as `{:?}` (one shared impl) | N/A |
| Non-secret field | `language = "de"` | unaffected — either shown verbatim or omitted via `..` | N/A |

</frozen-after-approval>

## Code Map

- `src-tauri/src/config/mod.rs:450` -- `AppConfig` `#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]`; drop `Debug`, add manual impl.
- `src-tauri/src/config/mod.rs:453-471` + `:~594` -- the 6 secret fields (`*_api_key` block + `turso_token`).
- `src-tauri/src/config/mod.rs` (`#[cfg(test)] mod ...`) -- home for the new regression test (inline, per project convention).

## Tasks & Acceptance

**Execution:**
- [x] `src-tauri/src/config/mod.rs` -- remove `Debug` from the `AppConfig` derive at :450 and add `impl std::fmt::Debug for AppConfig` using `f.debug_struct("AppConfig")`, rendering each of the 6 secret fields through a set/unset redaction helper, then `.finish_non_exhaustive()`. -- closes the latent Debug-leak at its only source.
- [x] `src-tauri/src/config/mod.rs` (test module) -- add `debug_redacts_secrets`: build `AppConfig::default()`, set the 6 secrets to distinct sentinel strings, assert `format!("{c:?}")` and `format!("{c:#?}")` contain none of the sentinels and do contain the redaction marker. -- forcing-sentinel: turns RED if `Debug` is re-derived or an un-redacted secret is added to the listed set.

**Acceptance Criteria:**
- Given an `AppConfig` with all 6 secret fields set to distinct known values, when formatted via `{:?}` and `{:#?}`, then none of those values appears anywhere in the output.
- Given the config persistence path, when a config is saved then reloaded, then the produced `config.json` is byte-identical to the pre-change output and the existing serde round-trip test still passes.
- Given `AppConfig` is used wherever a `Debug` bound is required, when the crate builds, then it compiles (the manual impl satisfies `Debug`) — `cargo check` and `cargo test` are green.

## Design Notes

Redaction marker shows configured-ness, never the value:

```rust
impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r = |s: &str| if s.is_empty() { "<unset>" } else { "<set>" };
        f.debug_struct("AppConfig")
            .field("groq_api_key", &r(&self.groq_api_key))
            // … deepseek / openai / anthropic / openrouter / turso_token …
            .field("language", &self.language) // example useful non-secret
            .finish_non_exhaustive()
    }
}
```

Alternative (heavier, more robust, NOT chosen by default): a `Secret(String)` newtype with `#[serde(transparent)]`, a redacting `Debug`, and `Deref<Target = str>` for the 6 fields. It auto-redacts any future secret field and removes `{}`-Display foot-guns, but changes the field types so assignment sites (env-load, `save_settings`/`merge_settings`, test fixtures) need `.into()`. All such breaks are compile-caught on Linux (pure-logic, no Windows-runtime surface), but it is a larger diff — take it only on human opt-in (Ask First).

## Verification

**Commands:**
- `cd src-tauri && cargo test debug_redacts_secrets` -- expected: pass.
- `cd src-tauri && cargo test` -- expected: green (serde/`Clone`/`PartialEq` round-trip tests unaffected).
- `cd src-tauri && cargo check` -- expected: compiles (no broken `Debug` consumer of `AppConfig`).

**Manual checks:**
- Confirm `AppConfig`'s `#[derive(...)]` no longer lists `Debug` and a manual `impl ... Debug for AppConfig` exists.
- Diff a `config.json` saved before vs after the change: the 6 secret fields are still present as plain JSON strings (proves the serde contract — and Android parity — is untouched).

## Suggested Review Order

- Entry point — the manual redacting `Debug`: secrets render `<set>`/`<unset>`, every other field hidden via `.finish_non_exhaustive()`.
  [`config/mod.rs:813`](../../src-tauri/src/config/mod.rs#L813)

- The derive change — `Debug` dropped; `Serialize`/`Deserialize`/`Clone`/`PartialEq` kept, so `config.json` + Android JSON stay byte-identical.
  [`config/mod.rs:450`](../../src-tauri/src/config/mod.rs#L450)

- Forcing-sentinel test — turns RED if any secret value appears in `{:?}`/`{:#?}` (verified RED via a manual inversion during review).
  [`config/mod.rs:1516`](../../src-tauri/src/config/mod.rs#L1516)
