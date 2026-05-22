# ADR-0013: Settings-Persistence-Schema für Phase-2-Settings-Panel

**Status:** Accepted
**Date:** 2026-04-26
**Accepted:** 2026-04-27 (Open-Questions-Resolution Q1–Q5, see Amendment 1 + §Resolved Questions)

## Context

Phase-2 Settings-Panel (`_bmad-output/planning-artifacts/_archive/phase-2-scope-lock.md` Phase-2-A Item A4 — historisch) braucht einen User-editable-Persistence-Layer für Settings. Phase-1 hat keinen — alle User-Settings leben in `%APPDATA%\Klarvo\config.toml` (5 Felder, hand-edited), API-Keys liegen im OS-Keystore / `dev-plain-keystore` (KeyStore-Trait, Story 1C.x).

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

**Accepted 2026-04-27.** Alle 5 Sub-Decisions sind final; Q1–Q5-Resolutions stehen inline in §Resolved Questions, Trace-Summary in §Amendment 1. Story-2.A.4 unblocked.

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

**Decision: flat `settings(key, value, type)` für Phase-2-A und Phase-2-B; Composite-Promote als spätere Phase-2-B-Story-Decision falls A2-Slot-Anzahl auf 4–5 skaliert.**

- **Phase-2-A** Settings-Panel implementiert flat `settings(key, value, type)` mit dot-namespaced Keys. Hotkey ist `hotkey.slot1.combo` + `hotkey.slot1.mode` (zwei Felder, kein Triple → flat reicht).
- **Phase-2-B** A2 Second-Hotkey-Slot bleibt initial flat (`hotkey.slot2.combo` + `hotkey.slot2.mode`). Composite-Promote zu dedizierter `hotkey_slots(slot_id, combo, mode, active)`-Tabelle ist eigene Mini-Migration-Story und wird *nur* dann eröffnet, wenn die Slot-Anzahl auf 4–5 skaliert (siehe Brief §Open Question Hotkey-Slot-Skalierungs-Trigger) ODER wenn Phase-3-Android-Bubble-State-Triple (`bubble.position` + `bubble.size` + `bubble.opacity`) den Promote-Schwellenwert erreicht.
- **Bewusst akzeptiert:** flat-Keys mit Slot-Index im Key-Pfad sind redundant gegenüber dedizierter Slots-Tabelle, aber bei MVP-Slot-Cap = 2 ist die Redundanz minimal und der Migration-Schritt zu Composite ist später additiv (kein Schema-Break, nur Daten-Move + Read-Path-Wechsel).

### Sub-Decision 3: Migration-Path Phase-1-TOML → Phase-2-SQLite

**Decision: Hard-Cut + One-Shot-Migration on First-Phase-2-Boot.**

Beim ersten Boot eines Phase-2-Builds:
1. Detect: `settings`-Tabelle leer + `config.toml` existiert.
2. Lese aktuellen `ShellConfig` aus TOML.
3. Schreibe User-Layer-Felder (siehe Sub-Decision-1-Tabelle) in SQLite.
4. Lasse System-Layer-Felder (z.B. `ui_language` als Default) im TOML.
5. Idempotent: nach erstem Boot wird TOML nur noch für System-Layer-Felder gelesen.

**Begründung gegen "TOML als alleiniger System-Layer + leere SQLite":** Phase-1-User-Edits am `config.toml` (insb. `hotkey`-Override) müssen erhalten bleiben — sonst bricht Daily-Drive bei Phase-1→Phase-2-Upgrade.

**Begründung gegen "Dual-Read-Period":** Komplexitäts-Anstieg ohne klaren Nutzen; Phase-1 hat keine aktiven Tester (`memory/project_ea_withdrawn`), Hard-Cut ist legitim.

### Sub-Decision 4: Settings-Mutation-API-Surface

**Decision: typed Tauri-Commands pro Core-Namespace + Generic-Fallback für `plugins.<id>.*`.**

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

**Decision: tauri-emit Push-Event `settings-changed` mit Key-Prefix-Filter.**

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
- **Format-Mutability-Window bis Phase-4 (Q5-Resolution-Schuld):** Phase-2-Schema ist *nicht* als v1-Import-Target stabilisiert. Zwischen Phase-2-Acceptance (2026-04-27) und Phase-4-v1-Import-Story darf das Schema-Format weiter mutieren — Key-Renames, Type-Promotions, Composite-Promotes (siehe Sub-Decision 2) sind erlaubt ohne v1-Compat-Break-Sorge. Konsequenz: ADR-0004 (v1→v2-Migration, Phase 4) wird einen eigenen Transformations-Layer brauchen, der v1-`config.json`-Schema in das *zu-Phase-4-stabilisierte* Phase-2-Schema mappt, nicht in das *2026-04-27-Schema*. Mitigation: Phase-4-Story-Eröffnung muss explizit das aktuelle Schema (zu Phase-4-Zeitpunkt) als Mapping-Target lesen, nicht ein gefrorenes Snapshot. Risiko: wenn zwischen Phase-2 und Phase-4 viele Mutations laufen (>5 Schema-Migrations), wird der v1-Import-Mapper komplexer als heute angenommen — diese Schuld wird bewusst akzeptiert, weil v1-User-Anzahl (`memory/project_ea_withdrawn`) klein ist und Phase-2-Iteration-Speed vor Phase-4-Mapping-Simplicity priorisiert wird.

**Story-Impacts (Phase-2-A):**

- **Story A4 (Settings-Panel):** Erstellt SQLite-`settings`-Tabelle-Migration, implementiert `Settings`-Service-Layer (klarvo-core) + Tauri-Commands (Sub-Decision 4) + Migration (Sub-Decision 3) + Notify-Event (Sub-Decision 5).
- **Story A8-Sub (Tray-Language-Switcher):** Listent auf `settings-changed`-Event, ruft `tray.set_menu(...)`.
- **Story C3 (Live-Locale-Switch):** Frontend listent auf `settings-changed` für `ui.language`-Key.
- **Story C2 (Hotkey-Konflikt-Erkennung):** Schreibt `hotkey.slot1.combo` über Settings-API; bei `RegisterHotKey`-Fail rollt Settings-Mutation zurück.

**Story-Impacts (Phase-2-B):**

- **Story A2 (Second-Hotkey-Slot):** Triggert Composite-Promote-Decision (Sub-Decision 2). Entweder eigene `hotkey_slots`-Tabelle oder weiter flat. ADR-Update Pflicht.
- **Story A1 (Recording-Modi):** Schreibt `hotkey.slot1.mode` über Settings-API.
- **Story B2 (Audio-Capture-Config-Overrides):** `audio.sample_rate`, `audio.channels`, `audio.device_id` über Settings-API.

## Resolved Questions

Alle 5 Open-Questions des Proposed-State (2026-04-26) wurden 2026-04-27 von Andy entschieden. Hier die Q-zu-A-Trace; Sub-Decision-Body ist entsprechend de-hedged.

### Q1 — Composite-Threshold-Trigger-Form (→ Sub-Decision 2)

- **Frage:** Phase-2-A flat oder pre-emptiv `hotkey_slots`-Tabelle?
- **Resolution (2026-04-27):** **flat.** Phase-2-A bleibt bei flat-Keys mit dot-namespacing. Phase-2-B-A2 erweitert flat um `hotkey.slot2.*` ohne Composite-Promote. Promote-Trigger ist Slot-Skalierung auf 4–5 oder Phase-3-Bubble-Triple — eigene Mini-Migration-Story zu dem Zeitpunkt.
- **Begründung der Wahl:** Pre-emptiver Composite ist Premature-Abstraction (`memory/feedback_premature_abstraction_guard`) — bei MVP-Slot-Cap = 2 ist die flat-Redundanz minimal, Migration zu Composite ist später additiv (kein Schema-Break, nur Daten-Move).

### Q2 — Migration-Path Aggressivität (→ Sub-Decision 3)

- **Frage:** Hard-Cut + One-Shot-Migration, oder Dual-Read-Period?
- **Resolution (2026-04-27):** **Hard-Cut.** Erster Phase-2-Boot detected leere `settings`-Tabelle + existierende `config.toml`, schreibt User-Layer-Felder in SQLite, danach ist TOML System-Layer-only. Idempotent + transaktional.
- **Begründung der Wahl:** Phase-1 hat keine aktiven Tester (`memory/project_ea_withdrawn`); Dual-Read-Period erhöht Komplexität ohne Nutzwert. Hard-Cut ist legitim und reduziert Read-Path-Drift in Phase-2-Code.

### Q3 — Settings-Save-API-Surface-Granularität (→ Sub-Decision 4)

- **Frage:** Typed pro Namespace, oder generic-only?
- **Resolution (2026-04-27):** **typed-pro-Namespace + generic-Fallback** für `plugins.<id>.*`. Core-Namespaces (`hotkey.*`, `ui.*`, `audio.*`, `app.*`) bekommen typed Tauri-Commands; Plugin-Namespaces nutzen generischen `set_plugin_setting(plugin_id, key, value)`.
- **Begründung der Wahl:** Frontend-Type-Safety via tauri-specta-Bindings für Core-Settings; Generic-Fallback vermeidet pro-Plugin-Command-Bloat und respektiert Plugin-Author-API-Contract (Plugins kennen ihren eigenen Namespace, Frontend muss generic dispatchen).

### Q4 — Notify-Mechanismus-Wahl (→ Sub-Decision 5)

- **Frage:** Push-Event oder Pull-State?
- **Resolution (2026-04-27):** **Push-Event** via `tauri::AppHandle::emit("settings-changed", ...)`. Frontend listent + reagiert reaktiv; Tray-Language-Switcher (A8-Sub) listent ebenfalls.
- **Begründung der Wahl:** Pull-State braucht Frontend-Polling oder Tauri-Subscribe-Layer; Push-Event ist Tauri-idiomatic, hat Phase-1-Präzedenz (ADR-0009 `app.error`-Event), und macht C3 Live-Locale-Switch ohne zusätzliche Infrastruktur möglich.

### Q5 — Phase-4-v1-Import-Schema-Stability-Erwartung (→ Consequences §Negativ)

- **Frage:** Phase-2-Schema als v1-Import-Target stabilisieren (Format-Lock), oder Format-Mutability bis Phase 4 offen?
- **Resolution (2026-04-27):** **Format-Mutability bis Phase 4 offen.** Schema darf zwischen Phase-2-Acceptance und Phase-4-v1-Import-Story weiter mutieren (Key-Renames, Type-Promotions, Composite-Promotes). Phase-4-v1-Import-Story muss das *zu-Phase-4-Zeitpunkt-aktuelle* Schema als Mapping-Target lesen, nicht ein 2026-04-27-Snapshot.
- **Begründung der Wahl:** v1-User-Anzahl ist klein (`memory/project_ea_withdrawn`); Phase-2-Iteration-Speed wird über Phase-4-Mapping-Simplicity priorisiert. Schuld explizit dokumentiert in §Consequences §Negativ als "Format-Mutability-Window bis Phase-4".

## Cross-References

- `output/planning-artifacts/architecture.md` §2 :245 (Config-Hybrid Decision-Source), §2 :247 (KeyStore-Layer-Trennung), §L520 (Namespace-Convention), §L536 (Typed-Accessor-Layer-Mandate)
- `_bmad-output/planning-artifacts/_archive/phase-2-scope-lock.md` Phase-2-A Item A4 (historisch — blockte durch dieses ADR)
- `docs/adr/0004-v1-to-v2-migration-strategy.md` (Phase-4 v1-Import, Q5-Cross-Ref)
- `docs/adr/0011-hotkey-backend.md` (Hotkey-Foundation Phase-1, additiv erweitert in Q1-Composite-Threshold)
- `docs/adr/0012-orchestrator-owner.md` (Orchestrator-Owner, konsumiert Settings-API in Phase 2-B A1)
- `shells/windows/src-tauri/src/config.rs` (Phase-1-`ShellConfig`-Ist-Stand, Migration-Source)
- `docs/backlog.md` "Minimales Settings-Panel" + "Live-Locale-Switch" + "Hotkey-Konflikt-Erkennung" (Phase-2-A-Items)
- `memory/feedback_premature_abstraction_guard` (Begründung für Mini-Pass statt Inline-Decision; auch Q1-flat-Resolution)
- `memory/feedback_adr_amendment_convention` (Acceptance-Commit-Hygiene: separater Commit, Decision-Block-Preservation, Memory-Update extern)
- `memory/project_phase1_complete` (Phase-1-Closure-Snapshot, Audit-Matrix-Stories abgeschlossen)
- `memory/project_klarvo_v2_rebuild` (Phasenplan-Memory, Phase-2-Scope-Konsistenz)
- `memory/project_ea_withdrawn` (Q2-Hard-Cut + Q5-Format-Mutability-Begründung: kleine v1-User-Anzahl)

## Next Actions

1. ✅ Andy → Q1–Q5 beantwortet 2026-04-27 (siehe §Resolved Questions + Amendment 1).
2. ✅ ADR-Status `Proposed` → `Accepted` mit eingebauten Decisions.
3. Story-2.A.4 (Settings-Panel) eröffnen — ADR-0013-Decisions sind dort load-bearing.
4. Bei Phase-2-B A2-Story (Second-Hotkey-Slot): Q1-flat-Resolution beibehalten *außer* Slot-Skalierung-Trigger feuert; dann Composite-Promote als eigene Mini-Migration-Story.
5. Bei Phase-4-v1-Import-Story (ADR-0004): Schema-Mapping-Target = zu-Phase-4-Zeitpunkt-aktuelles Schema (nicht 2026-04-27-Snapshot) — Q5-Schuld lesen + adressieren.

---

## Amendment 1 — 2026-04-27 — Open-Questions-Resolution + Acceptance

**Trigger:** Andy beantwortet alle 5 Open Questions des Proposed-State (2026-04-26).

**Geändert:**

- Header: `Status: Proposed` → `Accepted`; `Accepted: 2026-04-27` ergänzt.
- §Decision-Intro: Status-Block-Sentence umformuliert (Proposed-Vorschlag-Hedging → Accepted-Final).
- Sub-Decision 2: `Decision (Vorschlag)` → `Decision`; flat-Keys explizit als Resolution dokumentiert; Composite-Promote-Trigger-Bedingung präzisiert.
- Sub-Decisions 3, 4, 5: Header-Hedging `Vorschlag:` → `Decision:` entfernt; Body-Inhalt unverändert (Q-Resolutions deckungsgleich mit Vorschlägen).
- §Consequences §Negativ: neue Bullet "Format-Mutability-Window bis Phase-4 (Q5-Resolution-Schuld)" — explizite akzeptierte Schuld mit Mitigation + Risiko-Anker für Phase-4-v1-Import-Story.
- §Open Questions → §Resolved Questions: Q1–Q5 mit Inline-Resolution + Begründung der Wahl umgeschrieben.
- §Cross-References: `feedback_adr_amendment_convention` + `project_ea_withdrawn` ergänzt.
- §Next Actions: Q1–Q5-Pendings abgehakt; neuer Action-Item für ADR-0004-Phase-4-Story (Q5-Schuld-Adressierung).

**Nicht geändert (per `feedback_adr_amendment_convention` Decision-Block-Preservation):**

- Sub-Decision-Bodies (außer Hedging-Header) und Alternatives-Considered-Sections — der ursprüngliche Decision-Pfad samt verworfener Alternativen bleibt traceable.
- Context-Section + Decision-Drivers + Scope-Fence — die ursprüngliche Problemformulierung (2026-04-26) bleibt unverändert.

**Memory-Update (außerhalb dieses Commits):** `memory/project_phase2_scope_lock` aktualisieren, sodass ADR-0013-Status auf Accepted gewechselt ist und A4 unblocked.

---

## Amendment 2 — 2026-04-30 — Event-Name Dot-Notation (SD-5 Naming-Korrektur)

**Trigger:** Code-Review Pass-2 von Story 2.A.A4 (2026-04-30) flaggt Diskrepanz zwischen ADR-Wortlaut und Implementation.

**Geändert (SD-5 / Sub-Decision 5 — Settings-Change-Notification):**

- Event-Name in allen Code-Bezugnahmen von kebab-case `"settings-changed"` auf dot-notation `"settings.changed"`.
- Sub-Decision-5-Body Codeblock: `app.emit("settings-changed", ...)` → `app.emit("settings.changed", ...)`.
- Listener-Beschreibung: `listen<SettingsChangedEvent>("settings-changed", ...)` → `listen<SettingsChangedEvent>("settings.changed", ...)`.
- Cross-References Story-A8-Sub + Story-C3: `settings-changed` → `settings.changed`.
- Resolved-Questions Q5-Resolution-Block: gleicher Rename.

**Begründung:** `reference_tauri_specta_rc24_event_name`-Konvention + G1-Lint-Standard (FR34, Story 5.3) mandaten Dot-Notation für Cross-Layer-Event-Identifier (vgl. `app.error`, `app.ready`). Kebab-Case im ursprünglichen ADR war ungeprüfter Wortlaut-Drift zwischen ADR-Authoring (2026-04-27) und etablierter Naming-Convention. Code (`commands/settings.rs` + `bindings/index.ts`) folgt seit Initial-Implementation der Dot-Notation; ADR zieht jetzt nach.

**Nicht geändert:** Payload-Schema `SettingsChangedEvent { key, new_value }` (Form unverändert), Subscription-Mechanik, Filter-Empfehlungen, Concurrency-Modell. Funktional ist die Korrektur rein deklarativ (Event-Identifier-String).

**Cross-Refs:** Story 2.A.A4 §Spec-Deviations „Event-Name `settings.changed` (Dot-Notation) statt `settings-changed`"; Code-Review Pass-2 Patch P2-P16.

---

## Amendment 3 — 2026-05-01 — Hotkey-Konflikt-Modell: Pre-Validation statt Settings-Rollback (Story-C2-Wortlaut-Drift)

**Trigger:** Phase-2-A-Retrospektive 2026-05-01 (`epic-phase-2-a-retro-2026-05-01.md` AI-4) flaggt Diskrepanz zwischen ADR-Wortlaut §181 und Story-2.A.C2-Implementation. Defer aus C2-Code-Review-Closure (`2a-c2-hotkey-konflikt-erkennung.md` Defer-Item W1) wird hier resolved.

**Geändert (§181 / Story-Impacts Phase-2-A — Story C2):**

Alter Wortlaut (2026-04-26 Authoring):

> **Story C2 (Hotkey-Konflikt-Erkennung):** Schreibt `hotkey.slot1.combo` über Settings-API; bei `RegisterHotKey`-Fail rollt Settings-Mutation zurück.

Neuer Wortlaut (Implementation-aligned):

> **Story C2 (Hotkey-Konflikt-Erkennung):** Pre-Validation-Modell — Settings-Write erst nach erfolgreicher `RegisterHotKey`-Probe. `set_hotkey_slot1` sequenziert: (1) Skip-if-equal-Fast-Path falls `new_combo == old_combo`; (2) Grammar-Gate via `Shortcut::from_str` vor Probe; (3) `unregister(old)`; (4) Probe (`RegisterHotKey` + sofortiges `UnregisterHotKey` auf AtomicI32-Probe-ID, RAII-Guard); (5) bei Probe-Fail Re-Register-Old als Recovery + Return `HotkeyConflict`; (6) bei Probe-Erfolg Settings-Write + `register(new)`. Falls `register(new)` post-Settings-Write fehlschlägt: Re-Register-Old als Recovery + Toast `error.hotkey.update_failed_old_active`; **Settings bleiben neu, Hotkey bleibt alt** — kein Settings-Rollback (zu komplex bei async Win32-Failure-Modes).

**Begründung:** Pre-Validation ist die strukturell einfachere Topologie — Settings-Mutation passiert *nur* nach bewiesener Hotkey-Akquirierbarkeit, statt nach optimistischem Write mit fehleranfälligem Async-Rollback. Win32 `RegisterHotKey` ist Thread-spezifisch (Message-Queue des calling Thread, vgl. C2-Story §115); Probe-Pattern entkoppelt Akquise-Test vom Persist-Write. Recovery-Pfad bei post-Write-Register-Fail nutzt Re-Register-Old statt Settings-Rollback, weil (a) Settings-Schema schon mutiert ist und transaktionaler Rollback Cross-Layer-Coordination bräuchte (SQLite-Tx + Win32-State), (b) User-facing Verhalten ("alter Hotkey aktiv, Settings zeigen neuen") via Toast eindeutig kommunizierbar ist.

**Nicht geändert:** Settings-Service-API-Surface (Sub-Decision 4 typed-pro-Namespace), Settings-Change-Notification (Sub-Decision 5 Push-Event, vgl. Amendment 2), SQLite-Schema (Sub-Decision 1 flat-Keys mit dot-namespacing), Migration-Path (Sub-Decision 3 Hard-Cut).

**Phase-2-B-Implikation:** 2.B.A2 (Second-Hotkey-Slot) erbt Pre-Validation-Modell für Slot-2-Akquisition: identische Sequenz mit `hotkey.slot2.combo`. AtomicI32-Probe-ID-Counter ist global (ein Counter für beide Slots) — Concurrent-Probe-Sicherheit gilt cross-slot. Story-Spec für 2.B.A2 muss explizit auf diesen Amendment-Block referenzieren.

**Cross-Refs:** Story 2.A.C2 §Code-Review-Closure (Patches P10/P11/P12 + Defer W1); `epic-phase-2-a-retro-2026-05-01.md` AI-4; `feedback_adr_amendment_convention` (Amendment-Convention).

## Amendment 3 — 2026-05-09: event-name `settings.changed` → `settings:changed`

Per ADR-0002 Amendment 2 (Tauri 2.10 `IllegalEventName`), the wire-name `"settings.changed"` mandated by Amendment 2 above is migrated to `"settings:changed"` (commit `30630d3`). The Tauri runtime rejects event names containing `.`; the migration applies to backend `app.emit*` / `app.listen` call-sites, the `SettingsChangedEvent` specta-derive `event_name`-attribute (`commands/settings.rs`), and frontend `tauriEvent.listen(...)` consumers (`shells/windows/src/index.html`).

**Out of scope:** the SQLite-key dot-namespacing (Sub-Decision 1 — `hotkey.slot1.combo`, `ui.language`, etc.) is unaffected. Those are payload data inside the `SettingsChangedEvent`, not Tauri event names.

## Amendment 4 — 2026-05-22: `audio.device_id` defer superseded by `audio.input_device`

**Line 187 reconciliation (Story 12.3):** The Story-B2-defer placeholder `audio.device_id` at line 187 is superseded. The implemented key is **`audio.input_device`**, matching the `architecture.md:520` namespace reservation.

**Value type: device NAME, not ID.** cpal exposes devices as `Device` objects without stable platform IDs. The closest Windows analog is the `IMMDevice::GetId()` endpoint-GUID, which changes on unplug/replug, driver updates, and audio-stack resets. v1 used device names (`audio_device: Option<String>` in `src-tauri/src/config/mod.rs:531`); v1 users (= Andy) have not reported confusion. Name-collisions (two devices with identical names) are rare enough to accept v1-equivalent silent-pick-first behaviour.

**Semantics:** `None` means "use OS-default device" (key absent from DB); `Some(name)` means "use this named device, fall back to OS-default if not found" (logged at WARN in `klarvo-audio-cpal`).

**Accessor surface:** `Settings::audio_input_device()` → `Result<Option<String>, AppError>`; `Settings::set_audio_input_device(val: Option<String>)` — `None` deletes the key, `Some(name)` upserts with type `"string"`. Pattern mirrors `hotkey_slot2_combo()` / `clear_hotkey_slot2_combo()`.

**Remaining line-187 defers:** `audio.sample_rate` and `audio.channels` overrides (sample-rate/channels configuration) are still deferred to Phase-2+ as originally noted.
