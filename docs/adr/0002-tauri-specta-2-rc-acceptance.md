# ADR-0002: tauri-specta 2.0.0-rc.24 als Codegen-Layer akzeptieren (trotz RC-Status)

**Status:** Proposed
**Date:** 2026-04-18

## Context

Architecture-Doc Step 4 §3 + Step 5 Naming-Patterns +`ci-bindings-drift.yml` + `xtask lint-events` (Validation-Patch G1) setzen `tauri-specta` als authoritative Codegen-Boundary voraus.

Pre-Flight-Live-Check (2026-04-18):
- Crates.io: tauri-specta stable-Tag ist `1.0.2` (2023-05-18). 2.x ist seit 2023 in RC-Status.
- Aktuellster RC: `2.0.0-rc.24`, pinned gegen Tauri v2 (stable) + specta `=2.0.0-rc.24`
- Events (emit/listen mit Type-Safety) sind in 2.x unterstützt und für Klarvos Architektur (Audio-Level, Transcription-Progress) zwingend nötig
- Tauri 2 ist stable seit Oktober 2024, ökosystem-mäßig etabliert
- tauri-specta 1.x unterstützt Tauri v2 NICHT — kein Stable-Fallback existiert

## Decision

Klarvo v2 verwendet **`tauri-specta = "=2.0.0-rc.24"`** (exakt gepinnt) + `specta = "=2.0.0-rc.24"` in Phase 0. Cargo.lock committed. Upgrades nur nach expliziter Review, nicht automatisch.

Rationale RC-Akzeptanz:
- Kein Stable-Alternative für Tauri-v2-Codegen
- Community-Usage ist breit, API für Commands + Events ist stabil zwischen RC-Versionen
- Drift-Gate in CI (`ci-bindings-drift.yml`) kompensiert Codegen-Instabilität — wenn Upgrade breaking wird, fail-fast

## Consequences

**Positiv:**
- Type-Safety über Tauri-IPC-Boundary (Commands + Events) ist Phase-0-ready
- `xtask lint-events` (G1 Validation-Patch) hat definierte API-Oberfläche
- Ökosystem-Alignment mit Tauri-v2-Mainstream

**Negativ:**
- Dependency auf RC-Release ist Policy-Risiko (Supply-Chain-Auditor bei B2B-Kunden könnte fragen — nicht MVP-Problem, aber später Thema)
- Stable-2.0-Release wird früher oder später Breaking-Changes bringen — beim Upgrade ggf. Migration-Aufwand

**Mitigations:**
- Exakter Version-Pin (`=2.0.0-rc.24`) verhindert Drift bei `cargo update`
- Cargo.lock committed + Dependabot-Alert auf breaking changes
- CI-Drift-Gate erkennt silent breaking-change bei Regen

## Smoke-Test Plan (Phase-0 Erst-Nachweis)

Minimal-Setup zum Drift-Gate-Bootstrapping:
1. 1 `#[tauri::command]` + Argument-Type über `collect_commands!` registrieren
2. 1 Event-Type mit `#[specta(rename = "recording.test")]` registrieren
3. `xtask generate-bindings` läuft, Output in `shells/windows/src/bindings/` committen
4. `xtask lint-events` prüft Rename-Attribut (G1-Gate)
5. CI-Job `ci-bindings-drift.yml` läuft Regen + `git diff --exit-code`

Einordnung im Phase-0-Flow: nach Cargo-Workspace-Init und Core-Traits, bevor erste "echte" Plugin-Commands.

## Next Action

Beim Scaffold in Phase 0 gegen `2.0.0-rc.24` pinnen. Bei stable-2.0-Release: separates Upgrade-ADR, nicht silent mergen.

## Amendment 1 — 2026-04-18: rc.24 event-attribute syntax correction

**Finding:** `#[specta(rename)]` removed on event containers in rc.24.

**Correct:** `#[tauri_specta(event_name = "app.ready")]`.

**Default without attribute:** `struct_name.to_kebab_case()` (NOT dot-notation).

**Policy unchanged:** every event must carry explicit dot-notation name.

**G1-Lint target updated accordingly:** scan `#[derive(..., tauri_specta::Event, ...)]` structs, require `#[tauri_specta(event_name = "…")]` with `.` in value.

**Source of finding:** Smoke-Test compile error in `tauri-specta-macros-2.0.0-rc.24`; confirmed in macro source `tauri-specta-macros/src/lib.rs:41-44` (fallback `ident.to_string().to_kebab_case()` when `event_name` attribute absent).

**Updated Smoke-Test-Plan Schritt 2 (supersedes original §Smoke-Test-Plan):**
> 1 Event-Type mit `#[tauri_specta(event_name = "recording.test")]` (or similar dot-notation) registrieren. `#[serde(rename)]` erfüllt die Anforderung NICHT — betrifft nur Payload-Feld-Keys, nicht die Event-NAME-Konstante.

## Amendment 2 — 2026-05-09: Tauri 2.10 `IllegalEventName` — migration from dot- to colon-notation

**Finding:** Tauri 2.10.x rejects event names containing `.` at runtime with `IllegalEventName`. Permitted characters are `[a-zA-Z0-9-/:_]`. The first ever Windows release-build of v2 (2026-05-08, prior to this amendment) panicked on the very first `app.listen("settings.changed", ...)` call inside the Tauri `setup` closure. Linux `cargo check` and unit tests do not exercise the runtime event-name validator, so the breakage was masked.

**Correction supersedes Amendment 1's "dot-notation" policy:** every event must carry an explicit colon-notation name `<domain>:<event>`.

**Migrated wire-names** (commit `30630d3`, 2026-05-09):
- `app.error` → `app:error`
- `app.ready` → `app:ready`
- `recording.{started,stopped,completed,delivered}` → `recording:{...}`
- `pipeline.{stage_started,stage_completed}` → `pipeline:{...}`
- `settings.changed` → `settings:changed`
- `pill_bar.{show,fade_out,waveform_tick,enter_live_preview,live_preview_chunk}` → `pill_bar:{...}`

**Out of scope for this amendment:** Error-keys carried as event payload (e.g. `error.config.parse_failed`, `error.hotkey.parse_failed`, `error.audio.start_failed`) are payload data, **not Tauri event names** — they remain in dot-notation. The migration touches only the wire-name constants passed to `app.emit*` / `app.listen` / `#[tauri_specta(event_name = ...)]`.

**G1-Lint target update:** the `xtask lint-events` rule that previously required `.` in `event_name = "…"` values must now require `:` instead (validation regex must reject `.` in event names; payload keys are unaffected because they don't go through the Tauri-event-name validator).

**Cross-references:**
- ADR-0009 §Implementation registered `app.error`; the AppErrorEvent specta-derive on `bridge.rs` now carries `event_name = "app:error"`.
- ADR-0013 Amendment 2 mandated dot-notation for `settings.changed`; that amendment is itself superseded by this one.

**Source of finding:** `panic.message=called Result::unwrap() on an Err value: IllegalEventName("settings.changed") at tauri-2.10.3/src/app.rs:1114` during first Windows release-build smoke-test (logfile `%APPDATA%\Klarvo\logs\klarvo.2026-05-08.log`).
