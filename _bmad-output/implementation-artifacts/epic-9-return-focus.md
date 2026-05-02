---
name: Story 9.1 — Return-Focus nach Paste
epic: 9
story_number: "9.1"
status: ready-for-dev
dependencies: []
---

# Story 9.1: Return-Focus nach Paste

Status: ready-for-dev

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

**Invariante:** `restore()` wird nur aufgerufen wenn `deliver()` *erfolgreich* war. Bei `deliver()`-Fehler oder leerer Pipeline kein `restore()`.

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

**HWND-Konvertierung:** `HWND` in `windows 0.61` ist `HWND(pub isize)`. Cast: `hwnd.0 as u64` (capture) und `h as isize` (restore). Auf x86-64 Windows liegen HWND-Werte im 32-Bit-Range — kein Overflow-Risiko.

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

### AC-8: Regression-Test für Return-Focus-Dispatch

**Given** `klarvo-shell-orchestrator/tests/session_tests.rs`,
**When** ein neuer Test existiert,
**Then** verifiziert er:
- Nach erfolgreicher Delivery (Hold-Mode) wird `focus_capture.restore_count() == 1`
- `focus_capture.last_restored()` ist `Some(Some(42))` (der sentinel-Wert aus `MockFocusCapture::capture()`)
- Nach fehlgeschlagener Delivery (OutputTarget-Fehler) bleibt `restore_count() == 0`

Vorlage-Pattern: analoger Test zu `paste_called_after_successful_delivery` in `session_tests.rs`.

### AC-9: `disallowed_methods`-Lint-Compliance

**Given** `klarvo-shell-orchestrator` und `klarvo-core` haben `disallowed_methods = "deny"`,
**When** Story 9.1 committed ist,
**Then** enthält `focus.rs` (klarvo-core) **kein** `.unwrap()` / `.expect()` in Production-Code. `NullFocusCapture::restore` ignoriert Parameter ohne `unwrap`. `cargo clippy` grün.

## Tasks / Subtasks

- [ ] `FocusCapture`-Trait + `NullFocusCapture` (AC-1)
  - [ ] `klarvo-core/src/output/focus.rs` anlegen
  - [ ] `pub mod focus;` + Re-Export in `klarvo-core/src/output/mod.rs`
- [ ] `SessionOrchestrator`-Integration (AC-2)
  - [ ] `focus_capture: Arc<dyn FocusCapture>` als neues Feld in `SessionOrchestrator`
  - [ ] `new()` bekommt neuen Parameter (letzter, nach `mode`)
  - [ ] `on_press()`: `capture()` vor Recording-State-Check, als Closure-Variable in Task
  - [ ] Restore-Calls nach `deliver()` (Hold/Toggle/AutoStop) und nach WaitAndType-Delivery
- [ ] `WinFocusCapture` in Windows Shell (AC-3)
  - [ ] `shells/windows/src-tauri/src/focus.rs` anlegen
  - [ ] `#[cfg(target_os = "windows")] pub mod focus;` in `lib.rs`
- [ ] `Win32_UI_WindowsAndMessaging` Feature in `Cargo.toml` (AC-4)
- [ ] Windows Shell Wiring in `main.rs` (AC-5)
  - [ ] `Arc::new(WinFocusCapture)` in `SessionOrchestrator::new(...)` eintragen
- [ ] `MockFocusCapture` in `klarvo-test-fixtures` (AC-6)
  - [ ] `klarvo-test-fixtures/src/focus.rs` anlegen
  - [ ] Re-Export in `lib.rs`
- [ ] Test-Helper-Update + Regression-Tests (AC-7 + AC-8)
  - [ ] `make_orchestrator_with_mode` in `session_tests.rs` + `e2e_test.rs` updaten
  - [ ] Neuer Test `focus_restored_after_successful_delivery`
  - [ ] Neuer Test `focus_not_restored_on_deliver_error`
- [ ] `cargo clippy` + `cargo test` grün (AC-9)

## Dev Notes

### `HWND`-Wert als `u64` — Begründung

`windows 0.61` definiert `HWND(pub isize)`. Für das `FocusCapture`-Trait (platform-agnostisch in klarvo-core) darf keine `windows`-Crate-Dependency eingeschleppt werden. Daher opake `u64`-Kodierung:
- Capture: `hwnd.0 as u64` (isize → u64; auf x86-64 Windows hat HWND immer positiven Wert, kein Sign-Extension-Problem im 32-Bit-Value-Range)
- Restore: `h as isize` → `HWND(h as isize)` (sicher für alle validen Windows-HWND-Werte)
- `0` ist das Null-HWND (kein Fenster hat Fokus) — mappt auf `None`

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

### File List
