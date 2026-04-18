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
