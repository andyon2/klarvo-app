# ADR-0011: Hotkey-Backend für Windows-Shell (Push-to-Talk)

**Status:** Proposed
**Date:** 2026-04-21

## Context

Epic 3 Windows-Shell-Integration braucht einen globalen Push-to-Talk-Hotkey, der Hold/Release emittiert — nicht nur Press. Die Hold-to-Talk-Semantik (`memory/project_shell_session_lifecycle`, 7-Step-Topology) startet die Recording-Session beim Press-Event (`broadcast::channel`, `CpalAudioSource::start`) und beendet sie beim Release-Event (Aggregator-Finalize + `run_pipeline`). Ohne verlässliche Release-Events gibt es kein Hold-to-Talk, sondern nur Toggle.

**Phase-1-Scope:** Windows-only (PRD §Phase-1 frontmatter, `sectionOverrides.drop.platform_support`). Android-Phase-3 löst das Problem über den AccessibilityService (separates Code-Path, `memory/project_play_store_phase3_blocker`). Dieser ADR entscheidet **nur** den Windows-Backend.

**Default-Hotkey:** `CommandOrControl+Shift+Space` (PRD frontmatter `scopeLock.hotkeyDefault`, user-configurable from Phase 2). Der Backend muss Modifier-Kombinationen korrekt binden.

**Decision-Drivers:**
- Hold/Release-Event-Support (nicht nur Press) — load-bearing für Push-to-Talk
- Fit mit existierendem Tauri-Runtime-Model (`memory/project_shell_runtime_model`: Single Tauri-managed tokio-Runtime)
- Maintenance-Burden (Solo-Dev, 3–5-Monate-MVP-Timeline)
- Test-Surface — kann Integration-Testing ohne echten Hotkey-Press passieren
- Ecosystem-Reife: Dependency-Alter, breaking-change-history, open-issue-queue

**Nicht entscheidungs-relevant für Phase 1:** Cross-platform-Parität (Android läuft über AccessibilityService separat; macOS/Linux sind Phase-3+ bzw. opportunistisch).

**Scope-Fence:** Dieser ADR entscheidet Backend-Wahl und Integration-Pattern. NICHT Scope: User-konfigurierbare Hotkey-Slots (Phase 2), Hotkey-Conflict-Detection-UX, SD-4-Boot-Error-UX-Option (separat in Story-3.1-Pre-Flight).

## Decision

**Gewählt: Option A — `tauri-plugin-global-shortcut` (offizielles Tauri v2 Plugin).**

### Sub-Decision 1: Plugin-Version

`tauri-plugin-global-shortcut` v2 (passend zu Tauri v2-RC-Stack aus ADR-0002). Konkrete Version wird in Story-3.1 gepinnt (current stable v2-Release, aligniert mit tauri-Core und tauri-specta-RC aus ADR-0002).

### Sub-Decision 2: Event-State-Dispatch-Pattern

```rust
// shells/windows/src-tauri/src/hotkey.rs (Skizze, Story-3.1-Scope)

use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
    match event.state() {
        ShortcutState::Pressed  => orchestrator.on_press(),
        ShortcutState::Released => orchestrator.on_release(),
    }
})?;
```

`ShortcutState::Pressed` vs. `ShortcutState::Released` ist die load-bearing API für Push-to-Talk. Beide Events werden explizit gematcht; kein Fall-through.

### Sub-Decision 3: Key-Repeat-Robustheit

Windows emittiert bei gedrücktem Hotkey Key-Repeat-Events. Der Orchestrator hält eine Session-State-Guard (ref `memory/project_shell_session_lifecycle`): wenn `on_press` gecallt wird während bereits eine Session aktiv ist, wird der Call als Repeat-Event verworfen (Idempotenz). Gleiches für `on_release` ohne aktive Session. Implementation-Detail der Orchestrator-Story (ADR-0012), nicht Plugin-Concern.

### Sub-Decision 4: Integration-Point

Registrierung in der Tauri-Setup-Phase (`shells/windows/src-tauri/src/main.rs` `.setup(..)`), nicht im Command-Handler. Plugin läuft auf Tauri-managed-tokio-Runtime (`memory/project_shell_runtime_model`) — kein zweiter Runtime. Callback dispatched an den Orchestrator (ADR-0012-Owner), der von `tauri::async_runtime::spawn` aus weiteragiert.

## Alternatives Considered

**(B) Raw `windows-rs` `RegisterHotKey` + Window-Message-Loop (WM_HOTKEY).**

Fair-Argumentation:
- Volle Kontrolle über Hold/Release-Dispatch und Key-Repeat-Verhalten (direkter WM_HOTKEY-Handler, manuelles GetMessage-Loop + Thread).
- Keine Plugin-Dependency → eine Indirection weniger, kein Plugin-Breaking-Change-Risiko.
- Direkt-Native: wenn das Plugin gegen Windows-API buggt, betroffen wir; wenn wir direkt gegen Windows-API gehen, debuggen wir unseren eigenen Code.

Rejected:
- **Eigen-Maintenance-Aufwand** für Message-Loop, Hotkey-Registration-Cleanup-on-App-Exit, MOD_NOREPEAT-Flag-Kombinatorik mit Modifier-Keys. Solo-Dev-Scope-Management (ref Phase-1-Timeline): ein Maintenance-Item weniger zählt.
- **Windows-only**: Kein Pfad für opportunistisches macOS/Linux. `tauri-plugin-global-shortcut` wäre re-usable falls Phase 3+ macOS-Shell (nicht aktuell geplant, aber Vision §Phase-4+).
- **WM_HOTKEY liefert keine expliziten Release-Events** — `RegisterHotKey` feuert nur bei Key-Down. Für Hold-to-Talk bräuchte es zusätzliche Low-Level-Keyboard-Hooks (`SetWindowsHookEx` WH_KEYBOARD_LL), die dann wieder Admin-Privileges-/UAC-Interaktionen hervorrufen. Ergo: Option B ist technisch **nicht einfacher**, nur anders komplex.
- **Testability: schwierig** — Message-Loop-Isolation für Integration-Tests erfordert Window-Handle-Mocking oder echte Fenster.

**(C) Third-Party-Crates (`rdev`, `global-hotkey`, `device_query`, u.a.).**

Fair-Argumentation:
- `rdev` bietet Low-Level-Keyboard-Hook mit echten Hold/Release-Events und arbeitet ohne Fenster-Handle.
- `global-hotkey` (von Tauri-Team vor Plugin-Era) ist eine minimale Registration-Crate, möglicherweise leichter als Plugin-Full-Stack.

Rejected:
- **`rdev`**: bekannte Interaktionsprobleme mit Tauri-Window-Focus (siehe tauri/tauri#14770 2025-2026), Linux benötigt X11-Display-Context (problematisch für Wayland — irrelevant für Windows-Phase-1, aber Alignment-Divergenz gegenüber Tauri-Mainline); zudem übersampelt Low-Level-Hooks den Keyboard-State für ein Feature, das wir so detailliert nicht brauchen.
- **`global-hotkey`**: duplicate ecosystem — `tauri-plugin-global-shortcut` wrappt es bereits mit Tauri-Runtime-Integration, Manager-Lifecycle-Handling und TS-Bindings. Direktverwendung verliert die Integration-Ergonomie ohne klaren Mehrwert für Phase 1.
- **Maintenance-Fragmentierung**: Third-Party-Crates haben inkonsistent Maintenance-Status (rdev hat Issue-Queue mit Input-Device-Breakage auf aktuellen Windows-Updates). Plugin-Stack ist unter Tauri-Team-Ownership — robustere Ecosystem-Allianz.

## Consequences

**Positiv:**
- **Hold/Release native:** `ShortcutState::Pressed`/`Released`-Split ist direkt die Semantik, die Push-to-Talk braucht — kein Custom-Low-Level-Hook nötig.
- **Integration-konform:** Plugin läuft unter Tauri-Runtime (`memory/project_shell_runtime_model` gewahrt), keine zweite Event-Loop, keine Thread-Pool-Fragmentierung.
- **Maintenance:** Tauri-Team-Ownership, parallele Release-Cadence mit Tauri-Core, bekannte Upgrade-Pfade für Phase-2+-Updates.
- **Test-Surface:** Orchestrator-Side kann `on_press`/`on_release`-Calls direkt aus Unit-Tests triggern (siehe ADR-0012 Option C) — Plugin-Emit ist synthetisierbar für Headless-Tests.
- **Cross-Platform-Option:** wenn Phase-4+ macOS-Shell ansteht, ist das Plugin re-usable. Android bleibt separat (AccessibilityService-Pfad), was konsistent mit bestehender Platform-Strategie ist.

**Negativ / akzeptierte Schulden:**
- **Plugin-Version-Drift:** tauri-plugin-global-shortcut folgt eigenem Release-Rhythmus — Story-3.1 pinnt Version (analog tauri-specta-RC-Pinning in ADR-0002). Upgrade-Gate beim Phase-2-Start.
- **Key-Repeat-Handling Orchestrator-Scope:** Plugin emittiert bei Windows-Key-Repeat weitere `Pressed`-Events. Orchestrator (ADR-0012) hält State-Guard gegen Duplicate-Press. Impl-Detail, keine Plugin-Limitation.
- **Bekannte macOS-Double-Fire-Issues** (tauri/tauri#10025, plugins-workspace#1748): **nicht Phase-1-relevant** (Windows-only); bei späterer macOS-Shell zu revisit. Forward-Marker in §Open Questions.

**Epic-3-Story-Impacts:**
- **Story 3.1 (Tauri-Skeleton-Bootstrap):** fügt `tauri-plugin-global-shortcut` zu `shells/windows/src-tauri/Cargo.toml` hinzu, ruft `.plugin(tauri_plugin_global_shortcut::Builder::new().build())` in `.setup(..)`.
- **Story 3.X (Hotkey-Wiring, AC-Shape noch per Epic-3-Story-Writing zu definieren):** Hotkey aus `config.toml` parsen (String wie `"CommandOrControl+Shift+Space"` → `Shortcut`), registrieren, `ShortcutState::Pressed`/`Released` an Orchestrator (ADR-0012) forwarden.
- **Story 3.X (Orchestrator-Error-Path):** wenn Hotkey-Registrierung beim Boot fehlschlägt (z. B. Shortcut bereits durch anderes System-Tool belegt), ergibt das einen `AppError { kind: PipelineValidation, user_message: "error.hotkey.registration_failed" }` (neue i18n-Key, Story-3.X-Scope) — propagiert via ADR-0009-Boot-Error-UX (SD-4-Resolution).

**Phase-2+-Impacts:**
- Zweiter Hotkey-Slot (`explicitlyOutOfScope` Phase-2) ist additiv registrierbar — Plugin erlaubt mehrere Shortcuts via `register_multiple` oder iterative `register`-Calls.
- Hotkey-Conflict-Detection (User-facing Warnung bei Bind-Failure) lebt in Settings-UI, Phase-2+.

## Open Questions

- **Key-Repeat-Guard-Implementation:** Orchestrator-Story-ACs definieren, ob State-Guard per `AtomicBool` oder per explizitem Session-State-Enum. Gehört zu ADR-0012-Implementation-Stories.
- **Hotkey-Parsing aus config.toml:** String-Format `"CommandOrControl+Shift+Space"` vs. strukturierter TOML-Block. Story-3.X-Scope. Plugin stellt `Shortcut::from_str` zur Verfügung — Default ist String.
- **macOS-Double-Fire-Forward-Marker:** bei Phase-4+-macOS-Shell revisit; aktuell Phase-1-Scope-Exclusion, kein Blocker.

## Cross-References

- `output/planning-artifacts/architecture.md` §8 Audio-Pipeline-Abstraktion (Integration-Downstream)
- `output/planning-artifacts/prd.md` FR12 (Hold-to-Talk), `scopeLock.hotkeyDefault`
- `docs/adr/0002-tauri-specta-2-rc-acceptance.md` (Version-Pinning-Präzedenz für RC-Plugins)
- `docs/adr/0009-shell-error-bridge-pattern.md` (Boot-Error-UX bei Hotkey-Registration-Fail)
- `memory/project_shell_runtime_model` (Single tokio-Runtime — Plugin muss darin leben)
- `memory/project_shell_session_lifecycle` (7-Step-Topology — Press/Release-Semantik)
- `memory/feedback_premature_abstraction_guard` (kein Plugin-Abstraction-Layer speculative)
- Tauri v2 Global-Shortcut-Plugin: https://v2.tauri.app/plugin/global-shortcut/

## Next Actions

1. Andy review + accept → Status `Proposed` → `Accepted`.
2. Story-3.1-Scope: Plugin zu `shells/windows/src-tauri/Cargo.toml` hinzufügen, `.plugin(..)` in `setup`.
3. Story-3.X (Hotkey-Wiring): Shortcut-Parsing + Registrierung + Dispatch-Pattern aus SD-2.
