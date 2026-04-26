# ADR-0013: Settings-Persistence-Schema für Phase-2-Settings-Panel

**Status:** Proposed
**Date:** 2026-04-26

## Context

Phase-2 Settings-Panel (`output/planning-artifacts/phase-2-scope-lock.md` Phase-2-A Item A4) braucht einen User-editable-Persistence-Layer für Settings. Phase-1 hat keinen — alle User-Settings leben in `%APPDATA%\Klarvo\config.toml` (5 Felder, hand-edited), API-Keys liegen im OS-Keystore / `dev-plain-keystore` (KeyStore-Trait, Story 1C.x).

**Phase-1-Ist-Stand:**

- `shells/windows/src-tauri/src/config.rs` definiert `ShellConfig` mit 5 Feldern: `hotkey`, `output_target_id`, `ui_language`, `dictionary_language`, `output_language` (`#[serde(deny_unknown_fields)]`).
- `klarvo-core` hat keinen Settings-SQLite-Layer (architecture.md §2 :245 mandatet einen, ist aber Phase-2-Trigger).
- `architecture.md` L520 dokumentiert Namespace-Konvention: `app.*`, `audio.*`, `hotkey.*`, `ui.*`, `license.*`, `history.*`, `plugins.<id>.*`.
- `architecture.md` L536 mandatet **Typed-Accessor-Layer** (`settings.ui_language()`, nicht `settings.get_string("app.ui_language")` in Feature-Code).

**Phase-2-Trigger (warum jetzt und nicht inline-im-Story):**

- Settings-Panel braucht Persistenz mit Schema-Decision.
- 2nd-Hotkey-Slot (A2) ist Composite-Triple-Kandidat (`hotkey.slot1.combo` + `hotkey.slot1.mode` + `hotkey.slot1.active`) — `architecture.md` §2 mandatet Composite-Threshold-Check als ADR-Trigger.
- Cross-Platform-Implikation: `klarvo-core` ist Phase-3-Android-shared. Schema-Wahl heute prägt Phase-3-Android-Settings-Surface.
- Per `feedback_premature_abstraction_guard`: kein Speculation-Layer — diese Decision ist nicht speculative, sondern *bereits-zwei-Konsumenten-vorhanden* (Windows-Settings-Panel A4 + zukünftige Android-Settings-Surface in Phase 3).

**Architectural-Mandat (existing baseline aus `architecture.md` L245):**

> **Config-Hybrid:** System-TOML (~5 Felder: DB-Pfad, App-Lang-Default, Telemetry-Flag, Dev-Mode) + User-SQLite-Tabelle `settings(key, value, type)`. Plain-JSON aus v1 abgeschafft.

> **Composite-Key-Revisit-Point:** wenn `hotkey.slot1.combo` + `hotkey.slot1.mode` + `hotkey.slot1.active` als Triple auftreten, ist das der Trigger für dedizierte `hotkey_slots`-Tabelle (architecture.md L530).

Dieser ADR konkretisiert die Architektur-Mandate auf Phase-2-Story-Granularität und beantwortet die offenen Sub-Fragen, die `architecture.md` bewusst auf Phase-N-Trigger verschoben hat.

**Decision-Drivers:**
- Schema-Stability gegen Phase-3-Android-Settings-Surface
- Migration-Path von Phase-1-`ShellConfig.toml` zu Phase-2-Persistenz (kein User-Data-Loss)
- Settings-Save-API-Surface als Tauri-Command-Vertrag (frontend-erkennbar)
- Live-Locale-Switch (C3) braucht Settings-Mutation-Notify-Mechanismus
- Composite-Threshold (Hotkey-Slot-Triple) — Phase-2-A vs. Phase-2-B Decision

**Scope-Fence:** Dieser ADR entscheidet Schema-Shape + Migration-Path + API-Surface + Notify-Mechanism. NICHT Scope: Settings-UI-Component-Tree (Story-2.A.4-Scope), v1-Import-Schema-Mapping (Phase-4 / ADR-0004), Plugin-eigene Migrations-Hooks (existing Plugin-Trait, Phase-1).

## Decision

**Status: Proposed.** Sub-Decisions sind als Vorschlag formuliert; finale Form nach User-Decision auf Open Questions.

### Sub-Decision 1: Layer-Split — System-TOML + User-SQLite (architecture.md-bestätigt)

System-TOML (`%APPDATA%\Klarvo\config.toml`) bleibt der **System-Layer** mit ~5 Feldern, die der Shell vor `klarvo-core`-Init bekannt sein müssen (App-Lang-Default, DB-Pfad, Dev-Mode-Flag, Telemetry-Flag). User-Layer wandert in SQLite-Tabelle `settings(key TEXT PRIMARY KEY, value TEXT, type TEXT)` im `klarvo-core`-DB-File.

**Phase-1-Felder-Triage (was wandert wohin):**

| Phase-1-Field (`ShellConfig`) | Phase-2-Layer | Begründung |
|--------------------------------|---------------|------------|
| `hotkey` | User-SQLite (`hotkey.slot1.combo`) | User-editable, Settings-UI-Surface |
| `output_target_id` | User-SQLite (`app.output_target_id`) | User-editable |
| `ui_language` | System-TOML (default) + User-SQLite (override) | Brief-i18n-3-Achsen: Default in TOML (Boot-Time), Override per User-Switch |
| `dictionary_language` | User-SQLite (`app.dictionary_language`) | User-editable |
| `output_language` | User-SQLite (`app.output_language`) | User-editable |

System-TOML-Phase-2-Surface (final): `db_path`, `ui_language_default`, `dev_mode`, `telemetry_local_logs_enabled`, ggf. `config_schema_version`.

### Sub-Decision 2: Composite-Threshold-Resolution für Phase-2-A vs. Phase-2-B

`hotkey.slot1.combo` ist Phase-1-Solo-Field (kein Triple). Phase-2-A ergänzt `hotkey.slot1.mode` (Toggle/AutoStop/Hold via A1) — wird zum Triple, sobald `hotkey.slot1.active` (boolean enable/disable) hinzukommt. Phase-2-B Story A2 (Second-Hotkey-Slot) verdoppelt das auf 6 Felder (slot1 + slot2).

**Decision (Vorschlag):**

- **Phase-2-A** Settings-Panel implementiert flat `settings(key, value, type)` mit dot-namespaced Keys. Hotkey ist `hotkey.slot1.combo` + `hotkey.slot1.mode` (zwei Felder, kein Triple → flat reicht).
- **Phase-2-B** A2 Second-Hotkey-Slot triggert Composite-Promote zu dedizierter `hotkey_slots(slot_id, combo, mode, active)`-Tabelle als eigene Mini-Migration-Story (oder bewusste Beibehaltung von flat-Keys, falls Slot-Anzahl auf 2 capped).
- **Trigger für Promote:** entweder A2-Story (wenn Slot-Anzahl auf 4–5 skaliert wird, siehe Brief Open Question) oder Phase-3-Android-Surface (wenn Bubble-State-Triple `bubble.position` + `bubble.size` + `bubble.opacity` analoge Promote-Trigger ist).

### Sub-Decision 3: Migration-Path Phase-1-TOML → Phase-2-SQLite

**Vorschlag: Hard-Cut + One-Shot-Migration on First-Phase-2-Boot.**

Beim ersten Boot eines Phase-2-Builds:
1. Detect: `settings`-Tabelle leer + `config.toml` existiert.
2. Lese aktuellen `ShellConfig` aus TOML.
3. Schreibe User-Layer-Felder (siehe Sub-Decision-1-Tabelle) in SQLite.
4. Lasse System-Layer-Felder (z.B. `ui_language` als Default) im TOML.
5. Idempotent: nach erstem Boot wird TOML nur noch für System-Layer-Felder gelesen.

**Begründung gegen "TOML als alleiniger System-Layer + leere SQLite":** Phase-1-User-Edits am `config.toml` (insb. `hotkey`-Override) müssen erhalten bleiben — sonst bricht Daily-Drive bei Phase-1→Phase-2-Upgrade.

**Begründung gegen "Dual-Read-Period":** Komplexitäts-Anstieg ohne klaren Nutzen; Phase-1 hat keine aktiven Tester (`memory/project_ea_withdrawn`), Hard-Cut ist legitim.

### Sub-Decision 4: Settings-Mutation-API-Surface

**Vorschlag: typed Tauri-Commands pro Settings-Group + Generic-Fallback.**

```rust
// shells/windows/src-tauri/src/commands/settings.rs (Skizze, Story-2.A.4-Scope)

#[tauri::command]
async fn set_hotkey_slot1(combo: String, state: State<'_, Arc<Settings>>) -> Result<(), AppError> { ... }

#[tauri::command]
async fn set_ui_language(lang: String, state: State<'_, Arc<Settings>>) -> Result<(), AppError> { ... }

// Generic fallback for plugin-namespaced keys
#[tauri::command]
async fn set_plugin_setting(plugin_id: String, key: String, value: String) -> Result<(), AppError> { ... }
```

Typed-Commands für Core-Namespaces (`hotkey.*`, `ui.*`, `audio.*`, `app.*`); generischer `set_plugin_setting` für `plugins.<id>.*`. Konsistent mit `architecture.md` L536 Typed-Accessor-Layer.

**Alternative:** generischer `set_setting(key, value, type)` für alles. Verworfen wegen tauri-specta-Bindings-Surface (typed-Commands geben Frontend-Type-Safety, generic-Set verliert das).

### Sub-Decision 5: Live-Mutation-Notify-Mechanismus (für C3 Live-Locale-Switch)

**Vorschlag: tauri-emit-Event `settings-changed` mit Key-Prefix-Filter.**

```rust
// On settings.set("ui.language", "de")
app.emit("settings-changed", SettingsChangedEvent {
    key: "ui.language".into(),
    new_value: "de".into(),
})?;
```

Frontend-Subscriber listenen via `listen<SettingsChangedEvent>("settings-changed", ...)` und re-rendern Locale-abhängige Komponenten. Tray-Language-Switcher (A8-Sub) listent ebenfalls und ruft `tray::set_menu_text(...)` auf.

**Alternative A:** `tauri::State<Arc<RwLock<Settings>>>` mit Pull-Read pro Render. Verworfen wegen fehlender Reactive-Re-Render-Trigger im Frontend.

**Alternative B:** broadcast-channel im klarvo-core, Shell-Wrapper bridged zu Tauri-Event. Verworfen wegen Komplexitäts-Overhead in Phase-2-A; broadcast-pattern ist Phase-1 nur für Audio-Hot-Path etabliert (`memory/project_shell_runtime_model`), nicht für seltene Settings-Mutations.

## Alternatives Considered

### (B) Settings ausschließlich in TOML, kein SQLite-Layer in Phase 2

Fair-Argumentation:
- Einfacher: ein einziger Persistenz-Layer.
- Phase-1-Code muss nicht großflächig auf SQLite-Lookups umgestellt werden.

Rejected:
- **architecture.md L245 mandatet** SQLite-User-Layer als Phase-2-Trigger. Beibehaltung von TOML-only verschiebt das Problem auf Phase-3 oder Phase-4, mit dann größerem Migrations-Schritt.
- **40+ User-Settings** (Brief §rebuild-discussion Frage 12 Antwort 4) in TOML wird fragil bei concurrent-Mutation aus Settings-Panel.
- **Plugin-Settings** (`plugins.<id>.*`) brauchen Prefix-Query-Support für Plugin-Uninstall (`architecture.md` L531) — TOML hat keinen nativen Prefix-Query.

### (C) Komplette settings-Tabelle ohne System-TOML (nur SQLite)

Fair-Argumentation:
- Single-Source-of-Truth, keine Layer-Sync-Probleme.

Rejected:
- **Boot-Sequence-Problem:** Vor SQLite-Init muss die Shell wissen, wo das DB-File liegt (chicken-egg). System-TOML ist Bootstrap-Layer.
- **Dev-Mode-Flag** muss vor klarvo-core-Init lesbar sein (steuert KeyStore-Selection: `dev-plain-keystore` vs. OS-Keystore).
- `architecture.md` L245 explizit: System-TOML hat 5 Felder, nicht 0.

### (D) Settings als JSON-File in `%APPDATA%`, kein SQLite

Fair-Argumentation:
- v1-Pattern (`config.json`) — Migrations-Tooling existiert teilweise.

Rejected:
- **architecture.md** mandatet ausdrücklich SQLite (L245: "Plain-JSON aus v1 abgeschafft").
- Keine atomic-Mutations bei concurrent-Frontend-Edits (Settings-Panel + Live-Locale-Switch).
- Keine Migration-Trait-Integration mit Plugin-Migrations (`memory/project_phase1_complete` 1B.4 / Plugin-Trait-Migration-Mechanism).

## Consequences

**Positiv:**

- **Architektur-konform:** Befolgt `architecture.md` §2 :245 ohne Abweichung; ADR ist konkretisierender Mini-Pass, nicht Deviation.
- **Phase-3-Cross-Platform-ready:** SQLite-Settings-Tabelle lebt in `klarvo-core`; Android-Shell (Phase 3) nutzt dieselbe Schema, Settings-UI ist platform-spezifisch aber Persistenz-Layer ist shared.
- **Plugin-Settings-Ergonomie:** Prefix-Query (`plugins.groq.%`) erlaubt sauberes Plugin-Uninstall; konsistent mit Plugin-Trait-Migrations.
- **Live-Locale-Switch (C3) entkoppelt:** tauri-emit + Frontend-listen ist ein Pattern, das auch für andere settings-changed-Reaktionen (z.B. Hotkey-Live-Re-Bind) skaliert.

**Negativ / akzeptierte Schulden:**

- **Migration-Step-Friction:** Phase-1→Phase-2-Upgrade triggert one-shot-Migration. Wenn die fehlschlägt, ist User-Daten-State unklar. Mitigation: Migration ist idempotent + transaktional + write-only (TOML wird nicht gelöscht, sondern für System-Layer-Felder weitergenutzt).
- **Composite-Promote-Future-Work:** Phase-2-B A2 (Second-Hotkey-Slot) MUSS Promote-Decision treffen — nicht nachgeholt, nicht ignoriert. ADR-Update beim Trigger.
- **Typed-Accessor-Boilerplate:** Pro Phase-2-Setting wird ein `Settings::ui_language()` / `Settings::set_ui_language(...)` benötigt. Mitigation: macro-based Code-Gen ist Phase-2+-Polish, nicht Phase-2-A-Blocker.

**Story-Impacts (Phase-2-A):**

- **Story A4 (Settings-Panel):** Erstellt SQLite-`settings`-Tabelle-Migration, implementiert `Settings`-Service-Layer (klarvo-core) + Tauri-Commands (Sub-Decision 4) + Migration (Sub-Decision 3) + Notify-Event (Sub-Decision 5).
- **Story A8-Sub (Tray-Language-Switcher):** Listent auf `settings-changed`-Event, ruft `tray.set_menu(...)`.
- **Story C3 (Live-Locale-Switch):** Frontend listent auf `settings-changed` für `ui.language`-Key.
- **Story C2 (Hotkey-Konflikt-Erkennung):** Schreibt `hotkey.slot1.combo` über Settings-API; bei `RegisterHotKey`-Fail rollt Settings-Mutation zurück.

**Story-Impacts (Phase-2-B):**

- **Story A2 (Second-Hotkey-Slot):** Triggert Composite-Promote-Decision (Sub-Decision 2). Entweder eigene `hotkey_slots`-Tabelle oder weiter flat. ADR-Update Pflicht.
- **Story A1 (Recording-Modi):** Schreibt `hotkey.slot1.mode` über Settings-API.
- **Story B2 (Audio-Capture-Config-Overrides):** `audio.sample_rate`, `audio.channels`, `audio.device_id` über Settings-API.

## Open Questions

Diese MUSS Andy beantworten, bevor `Status: Proposed` → `Status: Accepted` wechselt und Story-A4 beginnen kann.

### Q1 — Composite-Threshold-Trigger-Form

Sub-Decision 2 Vorschlag: flat-Keys in Phase-2-A, Composite-Promote bei A2 (Second-Hotkey-Slot). Alternative: bereits in Phase-2-A `hotkey_slots`-Tabelle anlegen (Future-Proof). **Frage:** Phase-2-A flat oder pre-emptiv `hotkey_slots`-Tabelle?

### Q2 — Migration-Path Aggressivität

Sub-Decision 3 Vorschlag: Hard-Cut + One-Shot-Migration (TOML User-Felder → SQLite, TOML bleibt System-Layer-only nach Migration). **Frage:** Hard-Cut OK, oder wollen wir Dual-Read-Period (z.B. ersten Phase-2-Boot beide Layers lesen, danach Hard-Switch)?

### Q3 — Settings-Save-API-Surface-Granularität

Sub-Decision 4 Vorschlag: typed-Commands pro Core-Namespace + generic für `plugins.*`. Alternative: generic-only (`set_setting(key, value, type)`) — Frontend-Type-Safety geht verloren, dafür weniger tauri-specta-Bindings-Drift. **Frage:** Typed pro Namespace, oder generic-only?

### Q4 — Notify-Mechanismus-Wahl

Sub-Decision 5 Vorschlag: tauri-emit `settings-changed`-Event. Alternative: Pull-based via `tauri::State<Arc<RwLock<Settings>>>` (Frontend pollt bei Re-Render). **Frage:** Push-Event oder Pull-State?

### Q5 — Phase-4-v1-Import-Schema-Stability-Erwartung

ADR-0004 (v1→v2-Migration) ist Phase-4. Wenn dieses ADR-0013-Schema in Phase-2 stabilisiert wird, kann v1-Import-Story (Phase 4) direkt darauf zielen — ODER muss das v1-Import-Schema-Mapping eigene Transformations-Layer einführen, weil Phase-2-Schema sich noch ändert? **Frage:** Soll Phase-2-Schema explizit als v1-Import-Target stabilisiert werden (Format-Lock), oder bleibt Format-Mutability-Window bis Phase-4 offen?

## Cross-References

- `output/planning-artifacts/architecture.md` §2 :245 (Config-Hybrid Decision-Source), §2 :247 (KeyStore-Layer-Trennung), §L520 (Namespace-Convention), §L536 (Typed-Accessor-Layer-Mandate)
- `output/planning-artifacts/phase-2-scope-lock.md` Phase-2-A Item A4 (blockt durch dieses ADR)
- `docs/adr/0004-v1-to-v2-migration-strategy.md` (Phase-4 v1-Import, Q5-Cross-Ref)
- `docs/adr/0011-hotkey-backend.md` (Hotkey-Foundation Phase-1, additiv erweitert in Q1-Composite-Threshold)
- `docs/adr/0012-orchestrator-owner.md` (Orchestrator-Owner, konsumiert Settings-API in Phase 2-B A1)
- `shells/windows/src-tauri/src/config.rs` (Phase-1-`ShellConfig`-Ist-Stand, Migration-Source)
- `docs/backlog.md` "Minimales Settings-Panel" + "Live-Locale-Switch" + "Hotkey-Konflikt-Erkennung" (Phase-2-A-Items)
- `memory/feedback_premature_abstraction_guard` (Begründung für Mini-Pass statt Inline-Decision)
- `memory/project_phase1_complete` (Phase-1-Closure-Snapshot, Audit-Matrix-Stories abgeschlossen)
- `memory/project_klarvo_v2_rebuild` (Phasenplan-Memory, Phase-2-Scope-Konsistenz)

## Next Actions

1. Andy review + accept → 5 Open Questions beantworten.
2. ADR-Status `Proposed` → `Accepted` mit eingebauten Decisions.
3. Story-2.A.4 (Settings-Panel) eröffnen — ADR-0013-Decisions sind dort load-bearing.
4. Bei Phase-2-B A2-Story (Second-Hotkey-Slot): Composite-Promote-Decision-Re-Visit → ADR-Update oder Beibehaltung Begründung.
