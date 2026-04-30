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

## Deferred from: Epic-5 code review (2026-04-26)

- **F14 — verify-release rustup-vs-target-Distinktion** [`xtask/src/verify_release.rs:212-219`] — Bei fehlendem `rustup` wird "target not installed" gemeldet statt "rustup not found". Lower-priority — Dev-Experience.
- **F15 — verify-release Windows CRLF auf `rustup target list`** [`xtask/src/verify_release.rs:251-260`] — `lines()` handled CRLF, aber `trim()` fehlt explizit. False-Positive-Risk auf Windows-Hosts.
- **F16 — `cargo check` ≠ Build (kein NDK-Linker im Check)** [`xtask/src/verify_release.rs:222-247`] — Gate ist green ohne Link-Path-Validation. Story 5.4 Technical-Notes erwähnen das als bewusste CI-Speed-Wahl; Phase-2 könnte `--target`-Build ergänzen.
- **F17 — Shared `target/`-Dir Race bei concurrent xtask-Runs** [`xtask/src/verify_release.rs:222-247`] — Cross-Compile poisons incremental-state. CARGO_TARGET_DIR-Isolation wäre Phase-2.
- **F18 — xtask Cargo-Features baken Phase-1-Stages ein** [`xtask/Cargo.toml:11-13`] — `stage-passthrough/stt/cleanup` hardcoded; Phase-2 stage-llm würde nicht erkannt. Re-evaluieren bei Stage-Erweiterung.
- **F19 — G3-A `is_excluded_g3` Scope-Inkonsistenz** [`xtask/src/lint_events.rs:545`] — Excludiert `android/`, nicht `klarvo-bridge-jni` oder deeper-nested `target/`. Pre-existing Convention-Drift.
- **F20 — G3-C Heuristic-Edge-Cases** [`xtask/src/lint_events.rs:466-507`] — Nested wildcard-arms, `use PipelineStageType as Pst`-Aliase. Story 5.3 Technical-Notes flag das explizit als Heuristik-Toleranz.
- **F21 — G3-B Locale-JSON nested/BOM/CRLF/Duplicate-Keys** [`xtask/src/lint_events.rs:340-357`] — Aktuelle Locale-Files sind flat JSON ohne BOM. Future-format-shifts würden break.
- **F22 — G3-B Forward-Drift fängt nur Literal-Keys** [`xtask/src/lint_events.rs:149`] — `format!("error.{}.network", ...)` und Const-Imports werden nicht erfasst. Pre-existing-Limitation; Story 5.3 Technical-Notes-akzeptiert.
- **F23 — G3-A `extract_string_literal_from_expr` recurses ohne Path-Validation** [`xtask/src/lint_events.rs:355`] — `Some(SomeOtherCtor("x"))` würde "x" extrahieren. Niedrig: nur `String::from`/`Cow::*`/`.into()` kompiliert für `Option<Cow<'static, str>>`-Field. Defer-Patch: Path-Segment-Allowlist.
- **F24 — manifest-strict missing fixture-File silent inconsistency** [`xtask/src/manifest_strict.rs:75`] — `continue` ohne Counter-Decrement; `total = expected.len()`. Final-Count `N-1/N` ohne klaren Cause. Phase-2 sauberer Fail-Path.

## Deferred from: code review of story-2a-A4 (2026-04-29)

- **A4-D1 — `set_raw` emit-after-DB-write Reorder-Risk** [`klarvo-core/src/settings/mod.rs:170-180`] — Mutex-Drop vor Emit; theoretisch Multi-Thread-Setter Reorder-Race. In Single-User-UI nicht realistisch.
- **A4-D2 — Plugin-emit failure swallowed (warn + continue)** [`shells/windows/src-tauri/src/commands/settings.rs:67-69`] — Tauri-Emit-Failure rare; kein Retry, kein Backpressure-Signal an Frontend. MVP-akzeptabel.
- **A4-D3 — `SettingsChangedEvent.new_value` als plain String — keine Type-Info im Payload** [`klarvo-core/src/settings/mod.rs:34`] — Schema unterstützt 4 Typen (`string|i64|bool|json`); Event-Payload droppt Type. Aktuell alle 5 Core-Fields strings. Plugin-i64/bool ist Phase-2-B+.
- **A4-D4 — `type`-Spalte ist dekorativ — keine Roundtrip-Validation auf value** [`klarvo-core/src/settings/mod.rs:154-162`] — `get_raw` validiert nur dass `type` aus 4-Tuple ist, prüft nicht dass `value` zum `type` passt. Strings only aktuell; Future-Phase-Issue.
- **A4-D5 — Multi-Process SQLITE_BUSY Risk** [`klarvo-core/src/settings/mod.rs:74-86`] — Kein `PRAGMA journal_mode=WAL`, kein `PRAGMA busy_timeout`. Single-Instance-Annahme; bei zwei parallelen Klarvo-Prozessen → SQLITE_BUSY. Backlog Multi-Window/Tray-Doubleclick.
- **A4-D6 — Schema-Migration v1 syntax error blockt zukünftige Migrationen** [`klarvo-core/src/settings/migrations.rs:19-26`] — Pure Forward-Migrations ohne Down-Path und ohne Schema-Drift-Check. Theoretisch Future-Pfad-Blocker.
- **A4-D7 — AppError Display impl Debug-Repr; `#[non_exhaustive]` AppErrorKind serde** [`klarvo-core/src/error.rs`] — `Display` nutzt `{:?}: {}`; im Frontend bei rolling-update kann Frontend unbekannten `kind`-String empfangen. Backwards-compat / log-format-issue.
- **A4-D8 — Form-State-Race auf schnellen Edits (kein Dirty-Tracking, kein Unsaved-Warning)** [`shells/windows/src/index.html:415-430`] — User-Edits NACH Save-Click gehen verloren ohne Notice. Spec-AC-9 verlangt kein Unsaved-Warning. UX-Polish-Backlog.
- **A4-D9 — SettingsChangedEvent-Subscription fehlt komplett im Frontend** [`shells/windows/src/index.html:386-481`] — Backend emittet `settings.changed`, Bindings exportieren `events.settingsChanged`, aber Panel subscribed nirgendwo. Single-Window-Fall aktuell unkritisch; A8-Sub/C2/C3 holen das Listening nach.
- **A4-D10 — HTML-Panel selbst nicht i18n-übersetzt (`lang="en"`, "Klarvo Settings" hardcoded)** [`shells/windows/src/index.html:305,449`] — Panel ist Phase-2-A-Minimal-Implementation; volle i18n-Integration in Phase-2-B Vite+React-Migration.

## Deferred from: code review of story-2a-A4 (Pass-2, 2026-04-29)

- **P2-W1 — `CORE_PREFIXES` reserviert `"license."` + `"history."` ohne Doc-Comment** [`klarvo-core/src/settings/mod.rs:393`] — Premature Reservation; Plugin-Author bekommt confusing Reject ohne Erklärung. Doc-Update genügt; ADR-pending.
- **P2-W2 — `migration_does_not_block_when_only_plugin_rows_exist`-Test verifiziert nicht "block-detection"** [`klarvo-core/src/settings/mod.rs:899-912`] — Schwacher Assert (nur Core-Field-Equality); stronger Assert wäre `count(*)==6` (1 plugin + 5 sentinel). Test-Hardening, nicht code-bearing.
- **P2-W3 — Test-Boilerplate `Settings::in_memory(noop()).unwrap()` 20× repetition** [`klarvo-core/src/settings/mod.rs::tests`] — `fn fresh()`-Helper würde Lesbarkeit erhöhen; reine Test-Quality-Polish.
- **P2-W4 — `invoke("get_user_settings")`-Hang → Infinite-Spinner** [`shells/windows/src/index.html:271-289`] — Tauri-IPC ist lokal, sollte immer returnen; Timeout wäre defensive Ceremony. Optional Hardening.
- **P2-W5 — Power-Loss zwischen `BEGIN` und `COMMIT` + edited TOML zwischen Crashes** [`klarvo-core/src/settings/mod.rs::migrate_from_toml_if_needed`] — Extrem rare; SQLite-Atomicity hält den primären Pfad; Mitigation braucht Design-Pass.
- **P2-W6 — React ESM-Imports ohne Offline-Cache** [`shells/windows/src/index.html:184-185`] — First-Launch ohne Netz = blank Panel. Phase-2-B Vite+React-Migration löst strukturell.
- **P2-W7 — Story-1B `rusqlite_migration`-Präzedent in AC-1 unverifiziert** [Story-File AC-1] — Per `feedback_reviewer_external_fact_verification`: behauptetes externes Crate-Detail in Story-Statement braucht Source-Ref oder "zu verifizieren"-Markierung. Doc-Only, Memory-Hygiene.
- **P2-RC1 — SQLite-Hardening (WAL/busy_timeout/synchronous)** [`klarvo-core/src/settings/mod.rs::open`] — Pass-1 als A4-D5 deferred; Pass-2 (Edge Case Hunter E17) bestätigt zusätzlichen Cross-Process-Race auf `migrations.apply` zwischen Main-App und xtask-Binary. Verschmilzt mit A4-D5.
- **P2-RC2 — `settings.changed`-Listener fehlt im Frontend** [`shells/windows/src/index.html`] — Pass-1 als A4-D9 deferred; Pass-2 (E7) bestätigt: External-Writer-Stomp wird Foundation-Issue für A8-Sub/C2/C3. Verschmilzt mit A4-D9.
- **P2-RC3 — HTML `lang`-Attribut statisch `"en"`** [`shells/windows/src/index.html:129`] — Pass-1 als A4-D10 deferred; Pass-2 (BH#18) ergänzt a11y/Screen-Reader-Mispronunciation als zusätzliche User-Visible-Konsequenz. Verschmilzt mit A4-D10.
- **P2-RC4 — `type`-Spalte ohne `CHECK`-Constraint, Roundtrip-Validation fehlt** [`klarvo-core/src/settings/mod.rs::settings-schema`] — Pass-1 als A4-D4 deferred; Pass-2 (BH#5) verschärft: Schema-Level `CHECK(type IN ('string','i64','bool','json'))` würde Future-Phase-Bug strukturell verhindern statt code-only. Verschmilzt mit A4-D4 + Phase-2-B-Schema-Migration-Trigger.

### Pass-2 Skipped-Patches (Folge-Story-Scope, 2026-04-30)

Story 2.A.A4 bleibt nach Pass-2-Closure auf `done`. Die folgenden 4 Pass-2-Patches sind nicht unter A4 verbucht, sondern explizit als eigene Phase-2-A-Welle-3- oder Phase-2-B-Stories zu scopen — sie brauchen Spec-Arbeit (UX-Decision / Architectural-Trait-Extension / Error-Path-Discrimination / Semantic-Default-Choices), die unter Foundation-Closure-Scope nicht gehört.

- **P2-P1-deferred — Load-fail Data-loss: Form-UX-Redesign** [`shells/windows/src/index.html:271-289`] — Load-Fail-Pfad setzt FORM_DEFAULTS und erlaubt Save → reale DB-Werte werden mit Defaults gestompt. Echter Fix: Form `disabled` halten + explizites "Reload" anbieten + Save-Button hidden bis Load OK. Pass-1-P11-Intent (kein blank/undefined) muss umgekehrt werden — UX-Decision-Folge-Story.
- **P2-P8-deferred — `Settings::user_snapshot` single-mutex Bulk-Read** [`shells/windows/src-tauri/src/commands/settings.rs:127-137` + `klarvo-core/src/settings/mod.rs`] — `get_user_settings` nimmt 5× den Mutex, concurrent `set_*` zwischen den Lock-Cycles erzeugt torn read. Architectural-Extension: neuer `Settings::user_snapshot()`-Trait-Member der Mutex einmal hält + 5 SELECTs in einem Call macht; ripples durch Tauri-Command. Folge-Story für Phase-2-A-Welle-3 oder Phase-2-B (zusammen mit Pill-Bar-Foundation).
- **P2-P12-deferred — Corrupt-on-disk-DB Recovery-Pfad** [`klarvo-core/src/settings/mod.rs::open` + `main.rs::settings-fallback`] — Pass-1-P3 macht Two-Step-Fallback (file→in-memory) aber lässt korrupte File auf Disk shadowed. Echter Fix: `Connection::open`/`migrations::apply`-Err → File rename zu `settings.db.corrupted-<unix-ts>` + `tracing::error!`-Audit + dann in-memory. Discrimination welcher Error trigert Rename (corruption vs file-permission vs disk-full) braucht Decision.
- **P2-P21-deferred — Frontend-Reject + TomlMigrationSource semantic-fallback (D3-Resolution)** [`klarvo-core/src/settings/mod.rs:498-545` + `shells/windows/src/index.html:238-243,291-307`] — Code-Review Pass-2 D3 entschied Frontend-Authority für Locale/Output-Target-Validation. Implementation: Frontend hard-rejected unbekannte Werte vor Save (`langOptionsFor`-Helper aus Pass-1-P10 wird redundant), Migration bekommt semantic-fallback (Unknown locale → Default + warn). Zwei-System-Change mit Default-Choices (welcher Locale-Default? welcher Output-Target-Fallback?) — eigene Story.

### Pass-2 Dismissed (1, dokumentiert für Pass-3-Vermeidung)

- **P2-P19-dismissed — `Settings::in_memory` `pub` in Release** — Blind-Hunter-Blind-Spot ohne Projekt-Kontext. `in_memory` IST Production-Fallback (Pass-1-P3 Two-Step-Fallback in main.rs); cfg-test-Gating würde Boot-Resilience-Pfad brechen. Plugin-Authors können den Helper aufrufen, aber er greift auf eine flüchtige In-Memory-DB zu — kein Sicherheits-/State-Risiko. Sollte Pass-3-Reviewer das wieder flaggen: erste Antwort = "production fallback path".
- **P2-D1-resolved — CSP + SRI auf React ESM CDN-Imports** [`shells/windows/src/index.html:184-185`] — Resolved 2026-04-30 als defer (Option b). Phase-2-A intern, EA zurückgezogen, kein User-Release; SRI auf `esm.sh`-Sub-Imports (chained react→react-dom→scheduler) ist Halbsicherheit + false sense of security; CSP-Lockdown gegen `esm.sh` brüchig; saubere Lösung = Vendor-Bundling oder Vite-Migration in Phase-2-B. Investment in (a/c) wäre Wegwerf-Code. **Trigger zur Wieder-Aufnahme:** wenn Phase-2-A ungeplant Public-Release-Pfad bekommt ODER Phase-2-B Vite-Migration startet (dann strukturell durch Bundle-Inlining gelöst).
- **P2-D2-resolved — Sequential 5-await Save → Partial-State auf Mid-Fail** [`shells/windows/src/index.html:291-307`] — Resolved 2026-04-30 als defer (Option c). Mid-Fail-Trigger-Surface schrumpft mit P2-P2 (catch_unwind) + P2-RC1 (SQLite-Hardening) auf reine Validation-Fehler; Validation ist pro-Wert deterministisch → entweder alle 5 OK oder UI hat Garbage. Atomare `set_user_settings`-Batch-Command (Option a) ist Phase-2-B-Wert (Vite + Save-Status-pro-Field-UX) und sollte als eigene Story scoped werden, nicht als A4-Spec-Amendment. **Trigger zur Wieder-Aufnahme:** Phase-2-B Pill-Bar-UX-Mini-Pass oder External-Writer-Foundation für A8-Sub/C2/C3.
