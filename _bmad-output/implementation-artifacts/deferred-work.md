# Deferred Work

Items that were identified but not actioned. Each entry includes the source review and rationale.

## Deferred from: Epic-3 code review (2026-04-25)

- **F1 — `tauri-plugin-global-shortcut` Version-Pin spot-check** — Workspace `Cargo.toml` außerhalb des Review-Diff-Range; Spec verlangt exact-Pin analog ADR-0002. Externally verifizieren.
- **F2 — Cargo.lock `image`-Crate Supply-Chain-Notiz** — `image-png`-Feature zieht größeres Surface ein (transitiv via tauri/tray-icon). Pre-existing; ADR-Note nice-to-have.
- **F3 — `app.manage(Arc<SessionOrchestrator>)` doppeltes Arc-Wrapping** [`shells/windows/src-tauri/src/main.rs:878`] — Tauri::State liefert Arc-Semantik bereits; redundant aber funktional. Phase-2-Cleanup.
- **F4 — `drop(pipeline_task)` graceful-Shutdown** [`klarvo-shell-orchestrator/src/session.rs:208`] — Explizit als Phase-2-TODO in Story. *D5-Cross-Ref (commit 95a96e1):* 3-State-Lifecycle (`RecordingStarted`/`Stopped`/`Completed`) liegt jetzt als State-Machine-Foundation vor; Graceful-Shutdown kann auf `RecordingCompleted` warten oder `pipeline_task.abort()` rufen. F4 selbst (await/abort-on-exit) bleibt offen.
- **F5 — Story 3.11 AC-I Scope-Fence per-commit verification** — Combined-Diff verbirgt Per-Commit-Scope; via `git show 4603b87 --stat` separat verifizieren.
- **F6 — `setup_closure_types_compile` no-op assertion** [`shells/windows/src-tauri/src/main.rs:978-993`] — Compile-only Function-Definition; Standard-Rust-Pattern, kein echter Schaden.
- **F7 — `ShortcutState` non_exhaustive Robustness** [`shells/windows/src-tauri/src/hotkey.rs:631`] — Future-Plugin-Upgrade-Coverage; niedrige Priorität.
- ~~**F8 — `MockVadProvider` queue-size brittleness**~~ — **OBSOLET durch D1-Resolution (RmsVad-Refactor):** energy-basiert statt queue-basiert, kein Exhaustion-Pfad mehr.
- **F9 — `wait_for_delivery` busy-poll vs notify** [`klarvo-shell-orchestrator/tests/e2e_test.rs:204-215`] — Test-Ergonomie; gering bei 5s-SLA.
- ~~**F10 — TODO(de)-Prefix in production locale**~~ — **OBSOLET durch Story 4.3 (Translation-Pass).** Alle 13 Bestand-Keys haben jetzt finale deutsche Strings.

## Deferred from: Epic-4 code review (2026-04-25)

- **F11 — TOML-Type-Mismatch UX** [`shells/windows/src-tauri/src/config.rs:103-116`] — `parse_from_str` aliased Type-Mismatch (`ui_language = 42`) auf `error.config.missing` ("Configuration file not found"). Sauberer Fix bräuchte neuen Key `error.config.invalid_type` + Match auf TOML-Error-Patterns. Pre-existing aus Story 3.2-Branch-Logic; Phase-2-Settings-UI eliminiert das Problem strukturell.
- **F12 — `both_locale_files_valid_json_even_when_en_active` Regression-Guard** [`shells/windows/src-tauri/src/i18n.rs:189-197`] — Test prüft Happy-Path statt corrupt-DE-Regression. Echter Test bräuchte `load_from_strs(ui_language, en_json, de_json)`-Extraktion mit Inject-Fixture. `DE_JSON` ist aktuell `include_str!`-statisch.
- **F13 — Symmetric TODO-Marker-Test für `de.json`** [`shells/windows/src-tauri/src/i18n.rs::tests`] — `no_todo_markers_in_en_json` schützt nur EN-Master. Re-Introduction-Risiko niedrig (EN-Master + `de_json_covers_same_key_set` fangen primäre Drift); symmetrische Coverage wäre 5-Zeilen-Test.
