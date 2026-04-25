# Epic-3 Code Review — 2026-04-25

**Scope:** Stories 3.3 (EventBus-Amendment), 3.6, 3.10, 3.11
**Commit-Range:** `378fe99..4603b87` (5 commits, 17 files, +661/-24 LOC)
**Layers:** Blind Hunter, Edge Case Hunter, Acceptance Auditor (alle 3 erfolgreich)

**Triage-Summary:** 5 decision-needed · 21 patch · 10 defer · 11 dismissed

---

## Decision-Needed (5)

- [ ] **[Review][Decision] D1 — Story 3.11: MockVadProvider statt RmsVad** — AC-B + Technical-Notes-§"Warum RmsVad" mandatieren explizit `RmsVad` mit synthetic-loud-Audio. Code (`tests/e2e_test.rs:147-151`) verwendet `MockVadProvider::with_decisions([SpeechStart, SpeechStart])`. Komplette Spec-Rationale (real-Signalpfad statt Mock-Logik testen) ist invalidiert. Optionen: (a) Spec amenden mit Begründung warum Mock OK ist, (b) Code refactoren auf RmsVad.

- [ ] **[Review][Decision] D2 — Story 3.10 AC-B Step 3: keystore-readiness fire-and-forget** — Spec mandatiert synchrones `verify_keystore_ready(...).await.unwrap_or_else(...)` im `.setup()`. Code wraps in `tauri::async_runtime::spawn(async move { ... })` (`main.rs:807-815`). Boot-Readiness wird damit *nach* Boot geprüft — fail-soft-Semantik bleibt erhalten, aber das "checked-at-boot"-Versprechen ist gebrochen. Optionen: (a) Spec amenden, (b) auf synchron umstellen.

- [ ] **[Review][Decision] D3 — Tray-PNG-Dateien byte-identisch** — `tray-idle.png` und `tray-recording.png` haben beide SHA `7fb34b5`. Recording-State-Indicator-Code im Tray switcht Icons (`main.rs:914-928`), aber visuell kein Unterschied. AC-F: "zwei Placeholder-PNGs (graues Mikro / rotes Mikro)". Optionen: (a) Distinct-Placeholder shippen, (b) Spec amenden auf "single placeholder, Phase-2 differenziert".

- [ ] **[Review][Decision] D4 — TauriErrorEmitter: managed-slot bypass in hotkey.rs** — Story 3.10 AC-D managed `Arc<dyn ErrorEmitter>` in tauri::State. `hotkey.rs:618` und `:642` bauen jeweils einen *neuen* `TauriErrorEmitter::new(handle)` statt den managed-Slot zu konsumieren. Funktional OK (stateless), aber inkonsistent zur AC-D-Rationale. Optionen: (a) refactor `register_hotkey` zur Konsum des managed-Slots, (b) AC-D amenden mit Notiz "emitter ist stateless, lokale Konstruktion akzeptabel".

- [ ] **[Review][Decision] D5 — RecordingStopped emittiert bevor pipeline_task drainiert** — `session.rs:202` emittiert `RecordingStopped` *vor* `drop(capture_handle)` und während pipeline_task noch läuft. Subscriber (Tray) sehen "idle" während Audio noch verarbeitet wird. Asymmetrisch zu RecordingStarted (das nach erfolgreichem `audio.start()` feuert). Optionen: (a) Event nach Pipeline-Completion verschieben, (b) Event in `RecordingStopRequested` umbenennen + neues `RecordingFinalized` einführen, (c) so lassen + dokumentieren.

---

## Patch (21)

### Hotkey + Error-Handling
- [ ] **[Review][Patch] P1 — Hotkey-Error-Context geschluckt durch `Err(_)`** [shells/windows/src-tauri/src/hotkey.rs:615, hotkey.rs:629] — Underlying parse/registration error nicht geloggt; User bekommt nur generischen i18n-Toast.
- [ ] **[Review][Patch] P2 — Zwei unabhängige `MonotonicClock`-Instanzen** [shells/windows/src-tauri/src/hotkey.rs:611, src/main.rs:838] — `register_hotkey` baut eigene Clock, divergente Epoch-Origin → `ts_ms` aus Hotkey-Errors inkomparable mit RecordingStarted-Events. Verstößt gegen `project_event_ts_ms_convention`. Fix: Clock aus main.rs threaden ODER in hotkey.rs entfernen falls unused.
- [ ] **[Review][Patch] P3 — Tray-Subscription-Loop exit auf Lagged** [shells/windows/src-tauri/src/main.rs:206-219] — `while let Ok(...) = rx.recv().await { ... }` exited silent auf `RecvError::Lagged`. Bei Burst > 64 verliert Tray Recording-State. Fix: explizite match-Arm + `continue` bei Lagged.

### Bootstrap-Fail-Soft (verstößt gegen `feedback_scaffold_fail_soft_pattern`)
- [ ] **[Review][Patch] P4 — `debug_assert!(app.manage(...))`** [shells/windows/src-tauri/src/main.rs:868-871, 887] — In Release-Builds ist die Assertion entfernt, side-effect läuft trotzdem; bei Duplikat-Registrierung scheitert manage() silent → Hotkey-State-Lookup panict später. Fix: Hard-Assert oder explizites if-not-Ok-AppError.
- [ ] **[Review][Patch] P5 — `bootstrap_smoke_test` body = `unimplemented!()`** [shells/windows/src-tauri/src/main.rs:970] — Verstößt gegen Phase-1-Fail-Soft-Pattern (Memory: scaffolds returnen structured AppError, nie `unimplemented!()`/`panic!`). `cargo test -- --include-ignored` panict. Fix: Body leeren oder structured AppError.
- [ ] **[Review][Patch] P9 — `expect("...valid PNG")` panict App-Start** [shells/windows/src-tauri/src/main.rs:899-902] — Korruptes Asset ⇒ App startet nicht. AC-F policy-konform: Tray ist fail-soft. Fix: Match + log + Tray-Skip.
- [ ] **[Review][Patch] P10 — `TrayIconBuilder::build()?` und `MenuBuilder::build()?` propagieren fatal** [shells/windows/src-tauri/src/main.rs:894-911] — `?`-Operator macht Tray-Setup-Failure boot-fatal, AC-F klassifiziert Tray aber als fail-soft. Fix: match + log + return Ok(()).

### Code-Hygiene
- [ ] **[Review][Patch] P6 — `event_bus.emit(...)` Result discarded** [klarvo-shell-orchestrator/src/session.rs:80, 89] — `broadcast::send` → `Err` wenn keine Receiver; nicht mal `tracing::trace!`. Fix: Result loggen.
- [ ] **[Review][Patch] P7 — Tray-Menü "Exit" hardcoded English** [shells/windows/src-tauri/src/main.rs:906] — Locale-Infrastructure existiert (de.json/en.json). Fix: i18n-Key `tray.menu.exit`.
- [ ] **[Review][Patch] P8 — `build_plugin_registry(_keystore: ...)` arg unused** [shells/windows/src-tauri/src/main.rs:751] — Parameter nur durch Kommentar als "Phase-2"-prepared markiert; misleading. Fix: Entfernen, in Epic-2 wieder einführen wenn Groq-Wire-Up landet.
- [ ] **[Review][Patch] P11 — `EventBus::new(64)` magic in 3 Sites** [main.rs:855, tests/e2e_test.rs:175, tests/session_tests.rs:65] — Fix: `pub const DEFAULT_EVENT_BUS_CAPACITY: usize = 64` aus `klarvo-core::events` exporten.
- [ ] **[Review][Patch] P14 — `#[cfg]`-Gate-Inconsistenz auf keystore-Modul** [shells/windows/src-tauri/src/lib.rs:705] — `keystore` cfg = `any(target_os="windows", feature="dev-plain-keystore")`, Siblings (`audio`/`hotkey`/`paste`) sind nur `target_os="windows"`. Non-Windows + dev-plain-keystore zieht `keystore` rein, Siblings fehlen → broken silent. Fix: cfg vereinheitlichen oder Siblings ebenfalls feature-gaten.
- [ ] **[Review][Patch] P17 — Tracing nutzt `error = %e.message` statt Display** [main.rs:803, 808, 820, 847] — Rename/Privacy-Change auf `AppError.message` brichst alle Sites. Fix: `error = %e` (impl Display required).
- [ ] **[Review][Patch] P19 — `if target.all_delivered().len() > 0`** [klarvo-shell-orchestrator/tests/e2e_test.rs:207] — Clippy `len_zero`. Fix: `!is_empty()`.
- [ ] **[Review][Patch] P20 — Stray Tray-Menu-ID silent ignoriert** [shells/windows/src-tauri/src/main.rs] — `match event.id.as_ref()` hat nur `"quit"`; unbekannte IDs (typo, Phase-2-Items) verschwinden. Fix: `_ => tracing::warn!(?id, "unknown menu id")`.

### Doc + Spec-Drift
- [ ] **[Review][Patch] P12 — AC-D Consumer-List + Step-Placement für event_bus** [shells/windows/src-tauri/src/main.rs:867-871, 887] — Rustdoc-Consumer-List (4 manages: orch/config/keystore/emitter) erwähnt event_bus nicht; `app.manage(event_bus)` liegt in Step-13-Block statt Step-11. Fix: Doc-Update + manage-Call in Step-11-Group.
- [ ] **[Review][Patch] P13 — Spec-Typo "Phase-2 default VAD" für RmsVad** [shells/windows/src-tauri/src/main.rs:833 Kommentar] — RmsVad ist Phase-1 default. Fix: Kommentar korrigieren.
- [ ] **[Review][Patch] P15 — Story 3.6 AC-A Rustdoc-Comment misplaced** [shells/windows/src-tauri/src/hotkey.rs:627 statt main.rs `.plugin(...)`] — Spec-Wording verlangt "ADR-0011 SD-4"-Comment am Plugin-Activation-Site. Fix: nach `.plugin(tauri_plugin_global_shortcut::Builder::new().build())` in main.rs:721 verschieben.
- [ ] **[Review][Patch] P16 — Spec-Text-Fix: `SystemClock` → `MonotonicClock`** [output/planning-artifacts/epics/epic-3/story-3.10*.md AC-C Step 7, story-3.11*.md AC-B] — `SystemClock` existiert nicht in klarvo-core; Code verwendet korrekt `MonotonicClock` (ADR-0001/0003 + memory). Fix: Spec-Text in beiden Stories nachziehen.
- [ ] **[Review][Patch] P18 — Locale-JSON ohne trailing newline** [shells/windows/locales/de.json, en.json] — Fix: newline anhängen.
- [ ] **[Review][Patch] P21 — Story 3.11 Helper-Naming** [klarvo-shell-orchestrator/tests/e2e_test.rs:122-181] — Spec mandatiert `make_test_orchestrator_real_pipeline`; Code hat `_with_handles` + `_with_custom_stt`. Fix: rename ODER Spec amenden.

---

## Defer (10)

- [x] **[Review][Defer] F1 — `tauri-plugin-global-shortcut` Version-Pin nicht im Diff** — Spec verlangt exact-Pin analog ADR-0002; workspace-Cargo.toml außerhalb des Review-Range. Spot-check externally.
- [x] **[Review][Defer] F2 — Cargo.lock zieht `image`-Crate transitiv ein** [Cargo.lock:25] — Größerer Supply-Chain-Surface durch `image-png` features. Pre-existing transitive dep aus tauri/tray-icon. ADR-Notiz wäre nice-to-have, nicht blockend.
- [x] **[Review][Defer] F3 — `app.manage(Arc<SessionOrchestrator>)` doppeltes Arc-Wrapping** [shells/windows/src-tauri/src/main.rs:878] — Tauri::State liefert bereits Arc-Semantik. Funktional korrekt; Phase-2-Cleanup.
- [x] **[Review][Defer] F4 — `drop(pipeline_task)` graceful-Shutdown** [klarvo-shell-orchestrator/src/session.rs:208] — Explizit als Phase-2-TODO markiert in der Story. *D5-Cross-Ref:* die 3-State-Lifecycle (`RecordingStarted`/`Stopped`/`Completed`) ist jetzt die State-Machine-Foundation, auf der Graceful-Shutdown aufsetzen kann (z.B. App-Exit-Handler kann auf `RecordingCompleted` warten oder pipeline_task abortn). F4 selbst (await/abort-on-exit) bleibt offen.
- [x] **[Review][Defer] F5 — Story 3.11 AC-I Scope-Fence per-commit verifizieren** — Combined-Diff zeigt Cross-Story-Files; muss via `git show 4603b87 --stat` einzeln geprüft werden.
- [x] **[Review][Defer] F6 — `setup_closure_types_compile` no-op assertion** [shells/windows/src-tauri/src/main.rs:978-993] — Compile-only Function-Definition; falsches Sicherheits-Gefühl, aber Standard-Rust-Pattern.
- [x] **[Review][Defer] F7 — Future `ShortcutState` non_exhaustive variants** [shells/windows/src-tauri/src/hotkey.rs:631] — Plugin-Upgrade-Robustness; nice-to-have.
- ~~**[Review][Defer] F8 — `MockVadProvider` queue-size brittleness**~~ — **OBSOLET durch D1-Resolution:** Andy entschied sich für RmsVad-Refactor (Code statt Spec amenden); RmsVad triggert energy-basiert, kein Queue-Exhaust mehr.
- [x] **[Review][Defer] F9 — `wait_for_delivery` busy-poll vs notify** [klarvo-shell-orchestrator/tests/e2e_test.rs:204-215] — Test-Ergonomie; CI-Slow-Risk gering bei 5s-SLA.
- [x] **[Review][Defer] F10 — TODO(de)-Prefix in production locale** [shells/windows/locales/de.json:9-10] — Etablierte Konvention aus Story 3.2 (User-facing aber bewusst gewählt als translation-pending-Marker).

---

## Dismissed (11) — false positives / noise

- Edge: Race zwischen callback und `app.manage` (manage runs sync in setup, vor Callbacks)
- Edge: Pressed/Released spawn-race (global_shortcut serialisiert + stray-release-guard greift)
- Edge: Win32 `set_icon` thread-affinity (Tauri-Tray-Handle ist Send/Sync)
- Edge: `tauri_plugin_global_shortcut::Builder::build()` failure-handling (Tauri-Lifecycle handles)
- Edge: `wait_for_error_or_timeout` no-panic (per AC-H spec-note)
- Edge: cycle-2 STT retry-assumption (spec definiert kein retry)
- Blind: `impl SttProvider {}` future-trait-breakage (overly cautious)
- Blind: TODO(de)-Prefix als doc-leak (etablierte Konvention; siehe F10)
- Blind: duplicate `_assert_state_bounds`-Test (minor)
- Blind: `setup_closure_types_compile` false sense of safety (Standard-Rust-Pattern)
- Blind: `include_bytes!` bypasses Tauri asset-packaging (Phase-2-Placeholder)

---

## Cross-Story Verified (kein Finding)

- EventBus-Injection-Amendment konsistent in allen 3 Construction-Sites (session_tests.rs, e2e_test.rs, main.rs Step 10)
- Zwei unabhängige Subscriber-Pattern (Tray + EventMirror) per AC-F/AC-G korrekt umgesetzt (`event_bus.subscribe()` 2× in main.rs:884-885)
- Story 3.6 ACs A-G implementiert (modulo P1, P2, P15)
- Story 3.10 ACs C/F/G/H/I implementiert (modulo D2, D3, D4, P4-P15)
- Story 3.11 ACs A/C/D/E/F/G/H implementiert (modulo D1, P21)
