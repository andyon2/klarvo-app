---
name: Story 11.3 — Floating Pill Bar (Drag + Position-Persistence + Size + Mode-Badge)
epic: 11
story_number: "11.3"
status: backlog
dependencies:
  - "11.2"  # design-token foundation done; pill-bar.html uses var(--klarvo-*), tokens.css committed
inputDocuments:
  - _bmad-output/planning-artifacts/ux-design-specification.md  # §C1, §2.5.7, line 1488 (position memory)
  - _bmad-output/planning-artifacts/epics.md                    # Epic 11 full definition + story-sequence
  - shells/windows/src/pill-bar.html                            # file being modified — read before touching
  - shells/windows/src-tauri/src/overlay/pill_bar.rs            # file being modified — read before touching
  - klarvo-core/src/settings/mod.rs                             # file being modified — read before touching
  - docs/adr/0013-settings-persistence-schema.md                # position storage pattern (ui.* namespace)
---

# Story 11.3: Floating Pill Bar — Drag, Position-Persistence, Mode-Dependent Size, Mode-Badge

Status: backlog

## Story

Als Klarvo-User
möchte ich die Pill Bar durch Ziehen auf dem Bildschirm repositionieren können — und sie beim nächsten Start an der letzten Position wiederfinden —
damit ich sie dauerhaft aus meinem Arbeitsbereich heraushalten kann.

Als Klarvo-Entwickler
möchte ich, dass die Pill Bar einen Mode-Badge zeigt und die Infrastruktur für Live-Preview-Größe (480×84) bereitstellt,
damit Story 11.4 LivePreview-Inhalt ohne Größen-Refactor ergänzen kann.

## Context & Motivation

**Drag-Entscheidung:** Epic-11-Scope-Expansion 2026-05-08 overridet Drag=A aus `pill-bar-ux-decisions.md` (2026-05-03). UX-Spec §C1 "Drag: not supported (Step-2 pre-decision)" ist damit stale — die UX-Spec line 1488 gewinnt: "last user-dragged screen position persists per monitor signature". Die neue Entscheidung ist explizit von Andy bestätigt.

**Mode-Badge:** §C1 Anatomie mandatiert `[K-logo 28px] [abort-square 22px] [waveform 5-pill-bars] [mode-badge 'Hold']` für den Recording-State. Der Badge ist in 11.1 aus dem Scope gefallen (waveform-first-implementation); jetzt nachgeholt.

**Mode-Dependent Size:** §C1 nennt 480×84 für den Live-Preview-State (vs. 320×48 für Recording). 11.3 etabliert die Infrastruktur (CSS + Rust-Window-Resize + Event-Listener); 11.4 feuert den tatsächlichen Pipeline-Trigger.

**Prerequisite für Story 11.4:** 11.4 hängt davon ab, dass `pill_bar.enter_live_preview` bereits in JS konsumiert wird und das Window auf 480×84 greifen kann. 11.3 muss done+committed sein, bevor 11.4 startet.

## Scope — Was IN dieser Story ist

| Item | Datei | Typ |
|---|---|---|
| `data-tauri-drag-region` auf `#pill` + `cursor: move` | shells/windows/src/pill-bar.html | MODIFY |
| `html, body, #pill` auf `width: 100%; height: 100%` (dynamic-fill) | shells/windows/src/pill-bar.html | MODIFY |
| `#mode-badge` Element + CSS (static "Hold") | shells/windows/src/pill-bar.html | MODIFY |
| `pill_bar.enter_live_preview` JS-Listener + `.live-preview`-Klasse | shells/windows/src/pill-bar.html | MODIFY |
| `pill_bar_position()` + `set_pill_bar_position()` typed-Accessors | klarvo-core/src/settings/mod.rs | MODIFY |
| `PillBar::new()` — `WindowEvent::Moved`-Listener (debounced save) | shells/windows/src-tauri/src/overlay/pill_bar.rs | MODIFY |
| `handle_event(RecordingStarted)` — Position-Restore + Window-Reset | shells/windows/src-tauri/src/overlay/pill_bar.rs | MODIFY |
| `LIVE_PREVIEW_WIDTH/HEIGHT`-Konstanten + Window-Resize in RecordingStarted | shells/windows/src-tauri/src/overlay/pill_bar.rs | MODIFY |
| `dev_pill_bar_enter_live_preview` Tauri-Command (LP-Test-Trigger) | shells/windows/src-tauri/src/overlay/pill_bar.rs | NEW |
| `dev_pill_bar_enter_live_preview` in `specta_builder()` registrieren | shells/windows/src-tauri/src/lib.rs | MODIFY |

## Scope — Was NICHT in dieser Story ist

- LivePreview-Textinhalt (Text-Area, Side-Strip-Waveform mit 8 Bars) → 11.4
- Pipeline-seitiger Trigger für `pill_bar.enter_live_preview` (Core-Event) → 11.4
- Per-Monitor-Positions-Signaturen (multi-monitor-aware persistence) → Post-MVP
- Mode-Badge dynamischer Text für Modi außer "Hold" → wenn weitere Modi implementiert werden
- `pill_bar.exit_live_preview`-Event (Rückkehr zu 320×48 durch RecordingStarted-Reset, kein extra-Event nötig)

## Acceptance Criteria

### AC-1: Pill Bar ist draggable

**Given** die Recording-Session läuft (Pill Bar sichtbar),
**when** der User klickt-und-zieht den Pill-Bar-Body (außerhalb des Abort-Buttons),
**then** folgt das Window der Maus-Position.

**Given** der User klickt (kein Drag) auf den Abort-Button,
**then** feuert der Abort-Button seinen Click-Handler (keine Drag-Interferenz).

**Implementierungsanforderungen:**
1. `#pill`-Div bekommt das Attribut `data-tauri-drag-region` (kein Wert-Attribut — Tauri v2 erkennt die Presence).
2. CSS auf `#pill` ergänzt `cursor: move`.
3. `#abort-btn { cursor: pointer; }` bleibt bestehen (überschreibt `cursor: move` des Parents).
4. kein neuer Tauri-Command nötig — `data-tauri-drag-region` ist Tauri-v2-nativ.

### AC-2: Mode-Badge zeigt "Hold"

**Given** die Pill Bar ist sichtbar (Recording-State),
**then** ist rechts vom `#waveform` ein `<span id="mode-badge">Hold</span>` sichtbar.

**CSS-Anforderungen:**
```css
#mode-badge {
  font-family: system-ui, -apple-system, sans-serif;
  font-size: 10px;
  font-weight: 500;
  color: var(--klarvo-color-surface-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  flex-shrink: 0;
  user-select: none;
}
```

**Given** die Pill Bar ist im `.live-preview`-State (`#pill` hat Klasse `live-preview`),
**then** ist `#mode-badge` ausgeblendet (`display: none`).

**i18n-Anmerkung:** "Hold" ist ein Shell-seitiger HTML-String, nicht Core-seitig — außerhalb des G3-Lint-Scope. Lokalisierung deferred (analog zu `aria-label="Cancel recording"` in 11.1).

### AC-3: HTML/Body füllt Window dynamisch

**Given** das pill-bar-Window hat irgendeine Größe (320×48 oder 480×84),
**then** füllen `html, body` und `#pill` das gesamte Window (`width: 100%; height: 100%`).

**Konkrete Änderungen in `pill-bar.html`:**

| Vorher | Nachher |
|---|---|
| `html, body { width: 320px; height: 48px; }` | `html, body { width: 100%; height: 100%; }` |
| `#pill { width: 320px; height: 48px; ... }` | `#pill { width: 100%; height: 100%; ... }` (andere Properties bleiben) |

**Visuelle Invariante:** Beim 320×48-Window (Recording-State) ist das Erscheinungsbild identisch zum Status nach 11.2 — keine sichtbare Änderung.

### AC-4: Live-Preview-Größen-Infrastruktur

**Given** `pill_bar.enter_live_preview`-Event vom Rust-Backend eintrifft (via `dev_pill_bar_enter_live_preview`-Command oder zukünftigen 11.4-Trigger),
**then** gilt:

1. JS fügt Klasse `live-preview` zu `#pill` hinzu.
2. `#mode-badge` ist ausgeblendet (Regel aus AC-2).
3. Das pill-bar-Window ist auf 480×84 logical-px resized (durch Rust, vor dem Event-Emit).
4. Layout-Container (480×84) ist bereit für 11.4-Inhalt (Text-Area, Side-Strip-Waveform).

**Given** eine neue Recording-Session startet (`Event::RecordingStarted` → `pill_bar.show`),
**then** entfernt der JS-`pill_bar.show`-Handler die Klasse `live-preview` von `#pill` (frischer Recording-State).

**CSS für `.live-preview` (Layout-Ready-Zustand für 11.4):**
```css
/* Live-Preview layout — content wired by Story 11.4 */
#pill.live-preview #mode-badge { display: none; }
/* Waveform und weitere Anpassungen: durch 11.4 ergänzt */
```

**`pill_bar.rs` Konstanten:**
```rust
const LIVE_PREVIEW_WIDTH: f64 = 480.0;
const LIVE_PREVIEW_HEIGHT: f64 = 84.0;
```

### AC-5: `dev_pill_bar_enter_live_preview` Tauri-Command

**Given** der Command `dev_pill_bar_enter_live_preview` wird invoked (via DevTools-Console oder Test),
**then**:
1. Das pill-bar-Window wird auf `LIVE_PREVIEW_WIDTH × LIVE_PREVIEW_HEIGHT` (480×84 logical-px) resized.
2. Das Event `pill_bar.enter_live_preview` wird an das `pill-bar`-WebView emitted.
3. Return: `Ok(())`.

**Command-Implementierung (Skizze):**
```rust
#[tauri::command]
#[specta::specta]
pub async fn dev_pill_bar_enter_live_preview<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
        win.set_size(tauri::LogicalSize::new(LIVE_PREVIEW_WIDTH, LIVE_PREVIEW_HEIGHT))
            .map_err(|e| e.to_string())?;
        app.emit_to(WINDOW_LABEL, "pill_bar.enter_live_preview", ())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

**Registrierung in `lib.rs`:**
```rust
// lib.rs specta_builder collect_commands!
dev_pill_bar_enter_live_preview,
```

Mit Kommentar `// Story 11.3: LP-resize test trigger; wired to pipeline in 11.4`.

### AC-6: Typed Accessors für Pill-Bar-Position in `klarvo-core`

**Given** `klarvo-core/src/settings/mod.rs`,
**then** existieren folgende neuen typed Accessors (analog zu `ui_language()` / `set_ui_language()`):

```rust
/// Stored position of the pill-bar window (logical pixels, last user-dragged position).
/// Returns `None` when no position is saved (first run, fallback to bottom-center).
pub fn pill_bar_position(&self) -> Result<Option<(f64, f64)>, AppError> {
    let x_str = self.get_raw("ui.pill_bar.position_x")?;
    let y_str = self.get_raw("ui.pill_bar.position_y")?;
    match (x_str, y_str) {
        (Some(xs), Some(ys)) => {
            let x: f64 = xs.parse().unwrap_or(f64::NAN);
            let y: f64 = ys.parse().unwrap_or(f64::NAN);
            if x.is_finite() && y.is_finite() {
                Ok(Some((x, y)))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

/// Persist the pill-bar's logical-pixel position. Skips validate_setting_value since
/// values are programmatically generated (not user-supplied) and always valid floats.
pub fn set_pill_bar_position(&self, x: f64, y: f64) -> Result<(), AppError> {
    self.set_raw("ui.pill_bar.position_x", &x.to_string(), "string")?;
    self.set_raw("ui.pill_bar.position_y", &y.to_string(), "string")
}
```

**Koordinatensystem:** logische Pixel (f64). Konversion von/zu physical erfolgt in `pill_bar.rs` (Shell-Concern, nicht Core-Concern).

**Keys:** `ui.pill_bar.position_x`, `ui.pill_bar.position_y` — Prefix `ui.` ist in `CORE_PREFIXES` registriert, kein weiterer Change nötig.

**Unit-Test:** Mindestens ein Test für round-trip save+read analog zu `set_ui_language_persists_and_emitter_receives_event`.

### AC-7: Position-Save bei Drag (debounced)

**Given** der User zieht die Pill Bar (Window bewegt sich),
**when** ~300ms nach dem letzten `WindowEvent::Moved` vergangen sind,
**then** ist `ui.pill_bar.position_x` / `ui.pill_bar.position_y` mit der neuen logischen Position in Settings gespeichert.

**Fail-soft:** Ein Settings-Schreib-Fehler loggt `tracing::warn!` und crasht nicht.

**Implementierungsanforderungen:**
1. `PillBar::new()` registriert einen `on_window_event`-Listener auf dem `pill-bar`-Window.
2. Listener-Closure captured: `app.clone()` (für `app.state::<Settings>()` und `app.primary_monitor()`), `Arc<AtomicBool>` als "save-pending"-Flag.
3. Bei `WindowEvent::Moved(phys_pos)`: konvertiere zu logical via `app.primary_monitor()?.scale_factor()` (default 1.0 bei Monitor-Fehler). Setze pending-Flag. Spawne debounce-Task (nur wenn Flag vorher `false` war).
4. Debounce-Task: `tokio::time::sleep(Duration::from_millis(300))`, lese aktuellste Position aus `Arc<Mutex<Option<(f64,f64)>>>`, clearé pending-Flag, call `settings.set_pill_bar_position(x, y)`.
5. `PillBar`-Struct erhält kein `Settings`-Feld — Zugriff via `app.state::<Settings>()` in der Closure und im Spawn-Task.

### AC-8: Position-Restore bei RecordingStarted

**Given** `Event::RecordingStarted` eintrifft,
**then** (Reihenfolge einhalten):
1. `win.set_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))` — Window auf 320×48 zurücksetzen (falls vorherige Session LP-Größe hatte).
2. Position aus Settings lesen:
   - `settings.pill_bar_position()` gibt `Some((x, y))`: `win.set_position(LogicalPosition::new(x, y))`.
   - `settings.pill_bar_position()` gibt `None` (erster Start): `pill_bar_position(app)` (bestehende Bottom-Center-Berechnung, unverändert).
3. `win.show()`.
4. `app.emit_to(WINDOW_LABEL, "pill_bar.show", ())`.

**Fail-soft:** Jeder Fehler (Settings-Lesen, `set_size`, `set_position`) loggt `tracing::warn!` und setzt den jeweiligen Schritt aus — Window erscheint trotzdem.

**Hinweis:** `app.state::<Settings>()` ist im `handle_event`-Context verfügbar über `app: &AppHandle<R>`. Settings ist via `app.manage(settings)` in `main.rs` registriert.

### AC-9: `cargo check -p klarvo-windows-shell --lib` und `cargo check -p klarvo-core` grün

Keine neuen Compiler-Warnings durch diese Story.

**Windows Cross-Compile:** `pill-bar.html`-Änderungen benötigen kein Cross-Compile. `pill_bar.rs`-Änderungen sind Windows-only (`#[cfg(target_os = "windows")]`). `cargo check -p klarvo-windows-shell --lib` auf Linux mit MinGW-Target ist ausreichend (bestehende Baseline, nicht neu getriggert durch diese Story).

## Technical Notes & Dev Guardrails

### `data-tauri-drag-region` — Tauri v2 Behavior

In Tauri v2 initiiert `data-tauri-drag-region` auf einem Element Window-Drag bei `mousedown`+Maus-Bewegung. Reine Clicks (mousedown → mouseup ohne Bewegung) propagieren normal zu Child-Elementen. `#abort-btn` als `<button>` verhindert Drag-Aktivierung bei Clicks — kein zusätzlicher JS-Handler nötig.

**Kein `draggable` HTML-Attribut verwenden** — das ist für HTML5-Drag-and-Drop, nicht Tauri-Window-Drag.

### `WindowEvent::Moved` in PillBar::new()

```rust
// In PillBar::new() nach window-lookup:
let app_for_drag = app.clone();
let pending_pos: Arc<Mutex<Option<(f64, f64)>>> = Arc::new(Mutex::new(None));
let save_pending = Arc::new(AtomicBool::new(false));

if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
    let pending_pos_clone = Arc::clone(&pending_pos);
    let save_pending_clone = Arc::clone(&save_pending);
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::Moved(phys_pos) = event {
            let scale = app_for_drag.primary_monitor()
                .ok().flatten()
                .map(|m| m.scale_factor())
                .unwrap_or(1.0);
            let lx = phys_pos.x as f64 / scale;
            let ly = phys_pos.y as f64 / scale;
            {
                let mut guard = pending_pos_clone.lock().unwrap();
                *guard = Some((lx, ly));
            }
            if save_pending_clone.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
                let app2 = app_for_drag.clone();
                let pp = Arc::clone(&pending_pos_clone);
                let flag = Arc::clone(&save_pending_clone);
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                    flag.store(false, Ordering::SeqCst);
                    let pos = pp.lock().unwrap().take();
                    if let Some((x, y)) = pos {
                        if let Err(e) = app2.state::<Settings>().set_pill_bar_position(x, y) {
                            tracing::warn!(error = %e, "pill-bar position save failed");
                        }
                    }
                });
            }
        }
    });
}
```

`AtomicBool::compare_exchange` statt `AtomicBool::fetch_or` für korrekte "nur ein Spawn"-Semantik.

### `handle_event(RecordingStarted)` — vollständige neue Reihenfolge

```rust
Event::RecordingStarted { .. } => {
    fade_epoch.fetch_add(1, Ordering::SeqCst);
    ring.iter_mut().for_each(|v| *v = 0.0);
    if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
        // 1. Reset to recording size (in case previous session was in LP mode)
        if let Err(e) = win.set_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT)) {
            tracing::warn!(error = %e, "pill-bar size reset failed");
        }
        // 2. Restore position (or fall back to bottom-center)
        let (x, y) = match app.state::<Settings>().pill_bar_position() {
            Ok(Some((x, y))) => (x, y),
            _ => pill_bar_position(app),  // existing bottom-center calc
        };
        if let Err(e) = win.set_position(LogicalPosition::new(x, y)) {
            tracing::warn!(error = %e, "pill-bar set_position failed");
        }
        // 3. Show
        if let Err(e) = win.show() {
            tracing::warn!(error = %e, "pill-bar show failed");
        }
    }
    let _ = app.emit_to(WINDOW_LABEL, "pill_bar.show", ());
}
```

Die bestehende `pill_bar_position()` Funktion (gibt logische Bottom-Center-Koordinaten zurück) bleibt **unverändert** als Fallback.

### `pill_bar.show` JS-Handler — `.live-preview` entfernen

```javascript
tauriEvent.listen('pill_bar.show', () => {
    pill.classList.remove('fade-out');
    pill.classList.remove('live-preview');   // ← NEU
    bins.fill(0);
    updateBars();
});
```

### `pill_bar.enter_live_preview` JS-Handler

```javascript
tauriEvent.listen('pill_bar.enter_live_preview', () => {
    pill.classList.add('live-preview');
});
```

### settings/mod.rs — Import check

`set_pill_bar_position` verwendet `set_raw()` direkt (ohne `validate_setting_value`): Float-Strings sind programmatisch erzeugt und niemals leer/kontroll-char/oversized. Diese Ausnahme ist als Kommentar dokumentiert.

### Verzeichnisstruktur nach Story

```
shells/windows/src/
  pill-bar.html          ← MODIFIED (drag-region, dynamic-size, mode-badge, LP-listener)
klarvo-core/src/settings/
  mod.rs                 ← MODIFIED (pill_bar_position, set_pill_bar_position + test)
shells/windows/src-tauri/src/
  overlay/pill_bar.rs    ← MODIFIED (Moved-listener, AC-4 resize, AC-8 restore, dev command)
  lib.rs                 ← MODIFIED (dev_pill_bar_enter_live_preview in collect_commands!)
```

## Test Plan

1. **`cargo check -p klarvo-core`** und **`cargo check -p klarvo-windows-shell --lib`** → 0 Errors, 0 Warnings.
2. **Settings-Unit-Test** (neu): `pill_bar_position` round-trip: set → get → same values.
3. **Visueller Smoke-Test (cargo tauri dev):**
   a. Hotkey: Pill Bar erscheint bei 320×48, zeigt "Hold"-Badge rechts der Waveform.
   b. Drag: Pill Bar lässt sich über den Desktop ziehen; Abort-Button feuert weiterhin.
   c. Nach 300ms Stop: `ui.pill_bar.position_x/y` sind in der Settings-DB gespeichert (prüfbar via sqlite3-CLI).
   d. App neu starten → Hotkey: Pill Bar erscheint an der letzten Position (nicht bottom-center).
   e. DevTools-Console: `window.__TAURI__.core.invoke('dev_pill_bar_enter_live_preview')` → Window wächst auf 480×84, Mode-Badge verschwindet.
   f. Nächste Recording-Session: Pill Bar kehrt zu 320×48 zurück, Mode-Badge wieder sichtbar.

## Commit-Konvention

Empfohlen als zwei Commits:

```
feat(11.3): pill-bar settings accessors — ui.pill_bar.position_x/y + round-trip test
feat(11.3): floating pill bar — drag + position-persistence + LP-size-infra + mode-badge
```
