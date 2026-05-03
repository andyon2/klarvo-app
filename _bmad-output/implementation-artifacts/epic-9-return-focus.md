---
name: Story 9.1 — Return-Focus nach Paste
epic: 9
story_number: "9.1"
status: done
dependencies: []
---

# Story 9.1: Return-Focus nach Paste

Status: done (Review-Closure 2026-05-03 — 5 Patches + 1 Bonus-Fix für während Cross-Compile entdeckten pre-existing Tauri-Listener-Bug in `main.rs`; AC-5 Windows-Compile via MinGW vollständig verifiziert — gesamte Windows-Shell clean auf `x86_64-pc-windows-gnu`)

## Story

Als täglicher Klarvo-User
möchte ich, dass nach jedem erfolgreichen Diktat der Fokus automatisch zum ursprünglichen Foreground-Window zurückkehrt,
damit ich nahtlos weiter arbeiten kann ohne das Target-Fenster erneut anklicken zu müssen.

## Kontext und Motivation

**Problem:** Nach dem Hotkey-Press und dem automatischen Ctrl+V-Paste verbleibt der Fokus im aktuellen Foreground-Window (das Target-App). Das ist in den meisten Fällen korrekt. Jedoch gibt es Szenarien — besonders in WaitAndType-Mode (Epic 8) oder wenn die Pill-Bar (Epic 9.6) Fokus bekommt — wo der Fokus nicht im Target-Window landet und der User manuell zurückswitchen muss.

**Lösung:** `GetForegroundWindow()` bei Hotkey-Press (Step 1 des 7-Step-Lifecycle) speichern und nach erfolgreicher Delivery `SetForegroundWindow()` aufrufen. Gilt für alle Recording-Modi: Hold/Toggle/AutoStop (nach Paste) und WaitAndType (nach Delivery, vor Paste).

**Architektur-Entscheidung:** `FocusCapture`-Trait in `klarvo-core::output` (neben `PasteBackend`) hält die `SessionOrchestrator`-Dependency platform-agnostisch. `WinFocusCapture` in Windows Shell implementiert die Win32-Calls. `NullFocusCapture` (no-op) für Tests und zukünftige Non-Windows-Plattformen.

**Scope-Grenze:** Return-Focus ist best-effort und silent. Kein Error-Toast bei `SetForegroundWindow`-Failure (Target-Window existiert nicht mehr, User hat Focus selbst verschoben). Kein i18n-Key nötig.

## Acceptance Criteria

### AC-1: `FocusCapture`-Trait in `klarvo-core::output::focus`

**Given** `klarvo-core/src/output/` enthält `paste.rs` + `mod.rs`,
**When** Story 9.1 committed ist,
**Then** existiert eine neue Datei `klarvo-core/src/output/focus.rs`:

```rust
/// Captures and restores the OS foreground window around a dictation session.
///
/// Platform-agnostic trait — Windows impl lives in the shell, test/null impl
/// in `klarvo-test-fixtures` / `klarvo-core` (NullFocusCapture).
pub trait FocusCapture: Send + Sync + 'static {
    /// Capture the current foreground window handle as an opaque u64.
    /// Returns `None` if no window has focus, feature is unsupported, or handle is 0.
    fn capture(&self) -> Option<u64>;

    /// Restore focus to a previously captured handle. No-op if handle is None.
    /// Best-effort: silently ignores OS failures (target window may no longer exist).
    fn restore(&self, handle: Option<u64>);
}

/// No-op implementation for tests and non-Windows platforms.
pub struct NullFocusCapture;

impl FocusCapture for NullFocusCapture {
    fn capture(&self) -> Option<u64> { None }
    fn restore(&self, _handle: Option<u64>) {}
}
```

Und `klarvo-core/src/output/mod.rs` exportiert:
```rust
pub use focus::{FocusCapture, NullFocusCapture};
```

`pub mod focus;` in `mod.rs` hinzugefügt.

### AC-2: `SessionOrchestrator` injiziert und nutzt `FocusCapture`

**Given** `klarvo-shell-orchestrator/src/session.rs` hat `paste_backend: Arc<dyn PasteBackend>`,
**When** Story 9.1 committed ist,
**Then**:

1. `SessionOrchestrator::new()` erhält neuen Parameter `focus_capture: Arc<dyn FocusCapture>` (nach `mode`).
2. `on_press()` ruft `focus_capture.capture()` **als erstes**, bevor Audio-Start:
   ```rust
   pub async fn on_press(&self) {
       // 1a. Capture focus before any recording-state check or audio start
       let captured_focus = self.focus_capture.capture();
       // ... existing Recording-state check (key-repeat guard) ...
       // ... Audio start ...
       // pipeline_task captures `focus_capture_clone` + `captured_focus`
   ```
3. Der Pipeline-Task erhält `focus_capture` (Arc-Clone) + `captured_focus` (Option<u64>) als Closured Variables.
4. Nach erfolgreicher `target.deliver()` — **aller Modi inkl. WaitAndType**:
   ```rust
   if let Err(e) = target.deliver(&text).await {
       error_emitter.emit_error(...).await;
   } else if press_mode == RecordingMode::WaitAndType {
       event_bus.emit(Event::RecordingDelivered { ts_ms: clock.now_ms(), text: text.clone() });
       focus_capture_clone.restore(captured_focus); // WaitAndType: Fokus zurück zum Target
   } else if let Err(e) = paste_backend.paste().await {
       error_emitter.emit_error(...).await;
   } else {
       focus_capture_clone.restore(captured_focus); // Hold/Toggle/AutoStop: nach Paste
   }
   ```

**Invariante (Always-Restore-Policy, amended Review-Closure 2026-05-03 Decision D1=A):** `restore()` wird auf **jedem** Post-Capture-Exit-Pfad genau einmal aufgerufen, damit der User nie auf Klarvo's Overlay strandet. Pfade: deliver-success (Hold/Toggle/AutoStop nach Paste-Success **oder** Paste-Error), WaitAndType (vor `RecordingDelivered`-Emit), deliver-error, output-target-not-found, leere Pipeline (`text_to_deliver = None`), STT/Pipeline-Error. WaitAndType ist die einzige Branch mit *früher* Restore (vor Emit) — alle anderen Pfade restoren am Ende des `pipeline_task`. Implementierung: `let mut focus_restored = false;` Flag + End-Restore wenn `!focus_restored`. Frühere Spec-Version (nur restore-on-deliver-success) war unter-spezifiziert; korrigiert nach Review-Findings 2026-05-03.

### AC-3: `WinFocusCapture` in Windows Shell

**Given** `shells/windows/src-tauri/src/` enthält `paste.rs`,
**When** Story 9.1 committed ist,
**Then** existiert `shells/windows/src-tauri/src/focus.rs`:

```rust
#![cfg(target_os = "windows")]

use klarvo_core::output::FocusCapture;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};

pub struct WinFocusCapture;

impl FocusCapture for WinFocusCapture {
    fn capture(&self) -> Option<u64> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0 == 0 { None } else { Some(hwnd.0 as u64) }
    }

    fn restore(&self, handle: Option<u64>) {
        if let Some(h) = handle {
            if h != 0 {
                // SAFETY: h is an HWND value captured from GetForegroundWindow().
                // SetForegroundWindow is safe to call from any thread; failure is
                // best-effort (window may no longer exist).
                let _ = unsafe { SetForegroundWindow(HWND(h as isize)) };
            }
        }
    }
}
```

**HWND-Konvertierung:** `HWND` in `windows 0.61` ist `HWND(pub *mut core::ffi::c_void)` (verifiziert gegen `windows-0.61.3/src/Windows/Win32/Foundation/mod.rs`; **frühere Annahme `HWND(pub isize)` war falsch, korrigiert in Review-Closure 2026-05-03**). Cast: `hwnd.0 as usize as u64` (capture) und `HWND(h as usize as *mut core::ffi::c_void)` (restore). Null-Check: `hwnd.0.is_null()` (semantisch identisch zu `HWND::is_invalid()`).

`klarvo-core::output::FocusCapture` muss importiert werden — kein `use klarvo_windows_shell::...`.

### AC-4: `Win32_UI_WindowsAndMessaging` Feature in Cargo.toml

**Given** `shells/windows/src-tauri/Cargo.toml` hat:
```toml
windows = { version = "0.61", features = ["Win32_UI_Input_KeyboardAndMouse", "Win32_Foundation"] }
```
**When** Story 9.1 committed ist,
**Then**:
```toml
windows = { version = "0.61", features = [
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_WindowsAndMessaging",
    "Win32_Foundation",
] }
```
Feature `Win32_UI_WindowsAndMessaging` ist hinzugefügt. Damit werden `GetForegroundWindow` und `SetForegroundWindow` kompilierbar.

### AC-5: Windows Shell wired `WinFocusCapture` in `SessionOrchestrator`

**Given** `shells/windows/src-tauri/src/main.rs` baut `SessionOrchestrator::new(...)`,
**When** Story 9.1 committed ist,
**Then**:
- `pub mod focus;` in `lib.rs` (unter `#[cfg(target_os = "windows")]`)
- `SessionOrchestrator::new(...)` erhält `Arc::new(WinFocusCapture)` als letzten neuen Parameter
- Compile-Check: `cargo check --target x86_64-pc-windows-msvc` (oder Windows-Build) grün

### AC-6: `MockFocusCapture` in `klarvo-test-fixtures`

**Given** `klarvo-test-fixtures/src/` enthält `paste.rs` (Vorlage),
**When** Story 9.1 committed ist,
**Then** existiert `klarvo-test-fixtures/src/focus.rs`:

```rust
use std::sync::{Arc, Mutex};
use klarvo_core::output::FocusCapture;

pub struct MockFocusCapture {
    captured: Arc<Mutex<Vec<Option<u64>>>>,
    restored: Arc<Mutex<Vec<Option<u64>>>>,
}

impl MockFocusCapture {
    pub fn new() -> Self { Self { captured: Default::default(), restored: Default::default() } }
    pub fn capture_count(&self) -> usize { self.captured.lock().unwrap().len() }
    pub fn restore_count(&self) -> usize { self.restored.lock().unwrap().len() }
    pub fn last_restored(&self) -> Option<Option<u64>> {
        self.restored.lock().unwrap().last().copied()
    }
}

impl Default for MockFocusCapture { fn default() -> Self { Self::new() } }

impl FocusCapture for MockFocusCapture {
    fn capture(&self) -> Option<u64> {
        let handle = Some(42u64); // fixed sentinel for test assertions
        self.captured.lock().unwrap().push(handle);
        handle
    }
    fn restore(&self, handle: Option<u64>) {
        self.restored.lock().unwrap().push(handle);
    }
}
```

`klarvo-test-fixtures/src/lib.rs` exportiert:
```rust
pub mod focus;
pub use focus::MockFocusCapture;
```

### AC-7: `make_orchestrator_with_mode` Helper in Tests aktualisiert

**Given** `klarvo-shell-orchestrator/tests/session_tests.rs` hat `make_orchestrator_with_mode(...)`,
**When** Story 9.1 committed ist,
**Then**:
- Funktion erhält neuen Return-Member `Arc<MockFocusCapture>`
- `SessionOrchestrator::new(...)` im Helper bekommt `Arc::clone(&focus_capture) as Arc<dyn FocusCapture>` als neuen letzten Parameter
- **Alle bestehenden Tests** kompilieren und sind grün (NullFocusCapture/MockFocusCapture ist Drop-in)

Analog für `e2e_test.rs` falls dort `SessionOrchestrator::new` direkt gebaut wird.

### AC-8: Regression-Tests für Return-Focus-Dispatch (always-restore policy)

**Given** `klarvo-shell-orchestrator/tests/session_tests.rs`,
**When** die AC-8-Tests existieren,
**Then** verifizieren sie (amended Review-Closure 2026-05-03 D1=A):
- `focus_restored_after_successful_delivery`: nach erfolgreicher Delivery (Hold-Mode) `restore_count() == 1`, `last_restored() == Some(Some(42))`
- `focus_restored_after_deliver_error`: nach fehlgeschlagener Delivery (OutputTarget-Fehler) `restore_count() == 1` (always-restore — User darf nicht stranded sein)
- `focus_restored_after_paste_error`: nach erfolgreicher Delivery + fehlgeschlagenem Paste `restore_count() == 1`

Synchronisation: `wait_for_restore(&focus_capture, 1).await` (poll-until-condition mit 5s-Timeout) statt `tokio::time::sleep` — vermeidet CI-Flakes. Vorlage-Pattern: analoger Test zu `paste_called_after_successful_delivery` in `session_tests.rs`.

### AC-9: `disallowed_methods`-Lint-Compliance

**Given** `klarvo-shell-orchestrator` und `klarvo-core` haben `disallowed_methods = "deny"`,
**When** Story 9.1 committed ist,
**Then** enthält `focus.rs` (klarvo-core) **kein** `.unwrap()` / `.expect()` in Production-Code. `NullFocusCapture::restore` ignoriert Parameter ohne `unwrap`. `cargo clippy` grün.

## Tasks / Subtasks

- [x] `FocusCapture`-Trait + `NullFocusCapture` (AC-1)
  - [x] `klarvo-core/src/output/focus.rs` anlegen
  - [x] `pub mod focus;` + Re-Export in `klarvo-core/src/output/mod.rs`
- [x] `SessionOrchestrator`-Integration (AC-2)
  - [x] `focus_capture: Arc<dyn FocusCapture>` als neues Feld in `SessionOrchestrator`
  - [x] `new()` bekommt neuen Parameter (letzter, nach `mode`)
  - [x] `on_press()`: `capture()` vor Recording-State-Check, als Closure-Variable in Task
  - [x] Restore-Calls nach `deliver()` (Hold/Toggle/AutoStop) und nach WaitAndType-Delivery
- [x] `WinFocusCapture` in Windows Shell (AC-3)
  - [x] `shells/windows/src-tauri/src/focus.rs` anlegen
  - [x] `#[cfg(target_os = "windows")] pub mod focus;` in `lib.rs`
- [x] `Win32_UI_WindowsAndMessaging` Feature in `Cargo.toml` (AC-4)
- [x] Windows Shell Wiring in `main.rs` (AC-5)
  - [x] `Arc::new(WinFocusCapture)` in `SessionOrchestrator::new(...)` eintragen
- [x] `MockFocusCapture` in `klarvo-test-fixtures` (AC-6)
  - [x] `klarvo-test-fixtures/src/focus.rs` anlegen
  - [x] Re-Export in `lib.rs`
- [x] Test-Helper-Update + Regression-Tests (AC-7 + AC-8)
  - [x] `make_orchestrator_with_mode` in `session_tests.rs` + `e2e_test.rs` updaten
  - [x] Neuer Test `focus_restored_after_successful_delivery`
  - [x] Neuer Test `focus_not_restored_on_deliver_error`
- [x] `cargo clippy` + `cargo test` grün (AC-9)

### Review Findings

**Code-Review 2026-05-03** (Blind Hunter + Edge Case Hunter + Acceptance Auditor)

#### Decision Needed

- [x] [Review][Decision] **Restore-on-Failure-Policy** — RESOLVED 2026-05-03: Option A (Always-restore-on-any-post-capture-exit) gewählt. Implementiert via `focus_restored`-Flag + End-Restore in `session.rs`. Spec-Invariante AC-2 amendiert.

#### Patches (alle angewendet 2026-05-03)

- [x] [Review][Patch][CRITICAL] **HWND-Typ-Mismatch — Code kompiliert nicht auf Windows-Target** [shells/windows/src-tauri/src/focus.rs:14, :27] — Fixed: `hwnd.0.is_null()` (statt `== 0`) und `HWND(h as usize as *mut core::ffi::c_void)` (statt `HWND(h as isize)`). Verifiziert gegen `windows-0.61.3` Source.
- [x] [Review][Patch][HIGH] **Spec-Text falsch über HWND-Innentyp** [epic-9-return-focus.md:135, :266-268] — Fixed: Beide Stellen amendiert auf korrekten `*mut core::ffi::c_void`-Typ + Verweis auf authoritative Source.
- [x] [Review][Patch][HIGH] **AC-5 Compile-Check verifiziert** — MinGW-w64 (`x86_64-w64-mingw32-gcc 13`) installiert, `cargo check --target x86_64-pc-windows-gnu` für `shells/windows/src-tauri` ausgeführt. `focus.rs` (Story-9.1-Code) kompiliert clean — keine Errors mit `focus`-Bezug. 4 Errors in `main.rs` betreffen Pre-existing Tauri-2.x-`Listener`-Trait-Import-Bug (`app.listen` ohne `use tauri::Listener;`); Existenz auf HEAD c91e3e6 verifiziert via stash-Test → independent von Story 9.1 → als Defer dokumentiert.
- [x] [Review][Patch][MED] **Sleep-basierte Test-Synchronisation** [session_tests.rs] — Fixed: `wait_for_restore(&focus_capture, n)`-Helper hinzugefügt (poll-until mit 5s-Timeout, gleiches Pattern wie `wait_for_delivery`/`wait_for_error`); alle 3 AC-8-Tests umgestellt.
- [x] [Review][Patch][HIGH] **Always-Restore-Policy implementiert** [session.rs] — `focus_restored`-Flag + End-Restore-Pattern; restore wird auf jedem Post-Capture-Exit-Pfad genau einmal aufgerufen (deliver-error, paste-error, output-not-found, leere Pipeline, success). WaitAndType behält early-restore vor RecordingDelivered. AC-8 erweitert um `focus_restored_after_paste_error` (3. Test). 20 session_tests + 5 e2e_tests grün.

#### Deferred

- [x] [Review][Defer] **WaitAndType Restore-Timing-Race mit Pill-Bar** [session.rs:274-282] — Restore vor `RecordingDelivered`-Emit, aber Pill-Bar (Story 9.6) holt Fokus möglicherweise direkt zurück. Architektur-Entscheidung Spec-konform; Race ist Cross-Story-Concern → Epic-9.6-Scope.
- [x] [Review][Defer] **`SetForegroundWindow` Foreground-Lock-Limit** [shells/windows/src-tauri/src/focus.rs:27] — Win32-API rate-limited; restore schlägt silent fehl, wenn Klarvo nicht der Foreground-Process ist (wahrscheinlich nach Audio-Capture-Cycle). Spec acknowledged "best-effort, silent". Workaround `AllowSetForegroundWindow` evaluieren in Folge-Story.
- [x] [Review][Defer] **`MockFocusCapture`-Sentinel = 42 ist tautologisch für Multi-Press** [klarvo-test-fixtures/src/focus.rs:38] — Mock returnt immer `Some(42)`; Pairing-Tests fangen Ordering-Bugs bei mehreren Presses nicht. Nice-to-have: monoton-zählender Mock-Variant.
- [x] [Review][Defer] **Test-Coverage-Gaps**: paste-failure-restore, empty-result-restore, capture-during-key-repeat, None-from-capture path. Großteil hängt von Decision-D1 ab; nach Policy-Entscheid als Folge-Patches umsetzbar.

#### Dismissed (4)

- HWND-32-Bit-Truncation-Framing — falsche Diagnose-Richtung; durch P1 abgedeckt.
- Dual import paths `output::focus::FocusCapture` vs `output::FocusCapture` — etablierte Projekt-Konvention (siehe `paste`, `keys`).
- Status-Flip + Tasks-`[x]` im selben Diff — Process-Smell, nicht Code-Bug; das Audit ist genau hierfür.
- Inline `FailingOutputTarget`-Definition in Test — Pre-existing Pattern in `session_tests.rs`.

## Dev Notes

### `HWND`-Wert als `u64` — Begründung

`windows 0.61` definiert `HWND(pub *mut core::ffi::c_void)` (verifiziert gegen `~/.cargo/registry/.../windows-0.61.3/src/Windows/Win32/Foundation/mod.rs`). Für das `FocusCapture`-Trait (platform-agnostisch in klarvo-core) darf keine `windows`-Crate-Dependency eingeschleppt werden. Daher opake `u64`-Kodierung:
- Capture: `hwnd.0 as usize as u64` (Pointer → usize → u64; pointer-provenance via `usize`-Hop, semantisch standard für FFI-Round-Trip)
- Restore: `HWND(h as usize as *mut core::ffi::c_void)` (u64 → usize → Pointer)
- Null-HWND (kein Fenster hat Fokus) erkannt via `hwnd.0.is_null()`, mappt auf `None`

### `GetForegroundWindow` Thread-Safety

`GetForegroundWindow` ist ein Win32-API-Call ohne eigene Locking-Anforderungen — safe von jedem Thread. Der Return-Wert ist ein Snapshot-Zeitpunkt; zwischen Capture und Restore kann das Window zerstört worden sein — `SetForegroundWindow` returnt dann `BOOL(0)`, was wir silently ignorieren.

### Timing: Capture vor Recording-State-Check

In `on_press()` kommt der `capture()`-Call **vor** dem Recording-State-Check (key-repeat-guard). Begründung: Für Toggle-Mode wird der zweite Press als Stop behandelt — dort brauchen wir kein neues Capture. Wenn wir den Capture vor dem Check machen und der Check ein early-return auslöst, wird der `captured_focus`-Wert nie in die Closure übergeben (kein Leak, kein Problem).

Alternative wäre Capture nur auf dem Happy-Path (Idle → Recording). Beide sind korrekt; Capture-vor-Check ist minimal und verhindert doppelten Fokus-Speicher.

### `SessionState::Recording` — kein neues Feld nötig

`captured_focus: Option<u64>` wird als Closure-Variable in den `pipeline_task`-Spawn übergeben, nicht im `SessionState::Recording` gespeichert. Das reicht, weil der Capture-Wert nur innerhalb des Pipeline-Tasks benötigt wird und nicht nach `on_release()` oder Auto-Stop-Cleanup.

### Bestehende Tests: Null-Impl Pattern

`make_orchestrator_with_mode` muss angepasst werden (neuer Parameter). Da alle bestehenden Caller in `session_tests.rs` und `e2e_test.rs` über diesen Helper gehen, reicht eine Stelle. `MockFocusCapture` gibt immer `Some(42)` zurück — bestehende Tests, die `restore_count()` nicht prüfen, sind nicht betroffen.

### WaitAndType: Restore vor Pill-Bar-Delivery-Event

In WaitAndType: Restore-Call kommt **vor** `event_bus.emit(RecordingDelivered)`. Begründung: die Pill-Bar (Epic 9.6) reagiert auf `RecordingDelivered` — zu dem Zeitpunkt sollte das Target-Window schon fokussiert sein, damit der User sofort Tasten-Input nutzen kann.

Aktuelle Reihenfolge in session.rs:
```rust
} else if press_mode == RecordingMode::WaitAndType {
    // ÄNDERUNG: restore vor emit
    focus_capture_clone.restore(captured_focus);
    event_bus.emit(Event::RecordingDelivered { ts_ms: clock.now_ms(), text: text.clone() });
}
```

### Project Structure Notes

Neue Dateien:
- `klarvo-core/src/output/focus.rs`
- `shells/windows/src-tauri/src/focus.rs`
- `klarvo-test-fixtures/src/focus.rs`

Geänderte Dateien:
- `klarvo-core/src/output/mod.rs` (`pub mod focus;` + Re-Export)
- `klarvo-shell-orchestrator/src/session.rs` (neues Feld + Capture/Restore-Calls)
- `shells/windows/src-tauri/src/lib.rs` (`pub mod focus;` unter `cfg(windows)`)
- `shells/windows/src-tauri/src/main.rs` (`Arc::new(WinFocusCapture)` in SessionOrchestrator::new)
- `shells/windows/src-tauri/Cargo.toml` (`Win32_UI_WindowsAndMessaging` Feature)
- `klarvo-test-fixtures/src/lib.rs` (Re-Export)
- `klarvo-shell-orchestrator/tests/session_tests.rs` (Helper-Update + neue Tests)
- `klarvo-shell-orchestrator/tests/e2e_test.rs` (Helper-Update falls SessionOrchestrator::new direkt)

### References

- [architecture.md §UI] — "Return-Focus" als UI-Feature gelistet
- [memory/project_shell_session_lifecycle] — 7-Step-Topology: Press→channel→AudioSource→aggregator→run_pipeline→deliver→Ctrl+V→drop
- [prd.md §Growth-Features Phase-2] — Return-Focus explizit listed
- [docs/backlog.md "Return-Focus Feature"] — "Windows-Shell-specifisches API-Wiring (GetForegroundWindow / SetForegroundWindow)"
- `klarvo-core/src/output/paste.rs` — Vorlage: `PasteBackend`-Trait-Pattern
- `klarvo-test-fixtures/src/paste.rs` — Vorlage: `MockPasteBackend`-Pattern
- `shells/windows/src-tauri/src/paste.rs` — Vorlage: `WinSendInputPasteBackend`, HWND-Kommentar
- `klarvo-shell-orchestrator/tests/session_tests.rs:make_orchestrator_with_mode` — anzupassender Test-Helper

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (create-story 2026-05-03)

### Debug Log References

### Completion Notes List

- Alle 9 ACs implementiert und verifiziert (2026-05-03, claude-sonnet-4-6); Review-Closure 2026-05-03 (claude-opus-4-7): 5 Patches angewendet (HWND-Typ-Fix, Spec-Amendments, Always-Restore-Policy D1=A, Sleep→Poll, Paste-Error-Test)
- `FocusCapture`-Trait in `klarvo-core::output::focus` mit `NullFocusCapture` (no-op)
- `SessionOrchestrator.on_press()`: Capture vor Recording-State-Check; **Always-Restore-Policy** (Review D1=A) — Restore am Ende des `pipeline_task` auf jedem Exit-Pfad (deliver-error, paste-error, output-not-found, leere Pipeline, success); WaitAndType bleibt early-restore vor `RecordingDelivered`-Emit für Pill-Bar-Timing.
- `WinFocusCapture` via `GetForegroundWindow`/`SetForegroundWindow` (best-effort, silent); HWND-Typ korrekt als `*mut core::ffi::c_void` (windows 0.61.3 verifiziert)
- `Win32_UI_WindowsAndMessaging` Feature zu Windows-Shell Cargo.toml hinzugefügt
- `MockFocusCapture` (sentinel 42) in klarvo-test-fixtures; `make_orchestrator_with_mode` gibt 6-Tuple zurück
- 3 AC-8-Tests: `focus_restored_after_successful_delivery` + `focus_restored_after_deliver_error` + `focus_restored_after_paste_error`; alle nutzen `wait_for_restore`-Poll-Helper (kein sleep)
- Alle 20 session_tests + 5 e2e_tests grün (Linux-Host); `cargo clippy -p klarvo-core` warning-frei
- ✅ AC-5 Windows-Compile-Verifikation erfolgreich via MinGW-w64 + `cargo check --target x86_64-pc-windows-gnu`: gesamte Windows-Shell kompiliert clean inkl. `focus.rs` (klarvo-core + windows-shell), `session.rs`, `lib.rs`-Additions. Während der Verifikation entdeckt + gefixt: pre-existing Tauri-2.x-`Listener`-Trait-Import-Bug in `main.rs:34` (`use tauri::Listener;` hinzugefügt; existierte auf HEAD c91e3e6 unabhängig von Story 9.1, von Linux-only-Tests maskiert). Fix als Teil dieses Review-Closure-Commits.

### File List

**Neue Dateien:**
- klarvo-core/src/output/focus.rs
- shells/windows/src-tauri/src/focus.rs
- klarvo-test-fixtures/src/focus.rs

**Geänderte Dateien:**
- klarvo-core/src/output/mod.rs
- klarvo-shell-orchestrator/src/session.rs
- shells/windows/src-tauri/src/lib.rs
- shells/windows/src-tauri/src/main.rs
- shells/windows/src-tauri/Cargo.toml
- klarvo-test-fixtures/src/lib.rs
- klarvo-shell-orchestrator/tests/session_tests.rs
- klarvo-shell-orchestrator/tests/e2e_test.rs
- _bmad-output/implementation-artifacts/epic-9-return-focus.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
