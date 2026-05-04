---
name: Story 9.6 — Pill Bar
epic: 9
story_number: "9.6"
status: ready-for-dev
dependencies:
  - "9-1-return-focus"
  - "9-2-history-backend"
  - "9-5-log-export-ui"
---

# Story 9.6: Pill Bar

Status: ready-for-dev

## Story

Als täglicher Klarvo-User
möchte ich eine schwebende transparente Pill-Bar sehen, die bei Hotkey-Press erscheint und eine Live-Waveform der laufenden Aufnahme anzeigt,
damit ich visuelles Feedback bekomme, ob die Aufnahme läuft — ohne mein aktives Fenster zu verlassen.

## Kontext und Motivation

**UX-Mini-Pass (accepted 2026-05-03):** Alle 4 Decisions sind accepted — diese Story ist der direkte Implementierungs-Step. Decisions-Dokument: `_bmad-output/planning-artifacts/pill-bar-ux-decisions.md`.

**Decisions-Summary:**
- **Shape:** Fixe Größe 320×48px, runde Ecken. YAGNI: kein adaptive Resize (Tauri-Window-Resize auf Win32 fragil).
- **Drag:** Nicht draggable. Feste Default-Position bottom-center. Kein Settings-Key.
- **Waveform:** Backend pre-computed bins — 64 amplitude-Bins × 50ms via `pill_bar.waveform_tick`-Event mit `Vec<f32>` + `ts_ms`. Frontend (Pill-Bar-WebView-Canvas) zeichnet nur die Bins.
- **Auto-Hide:** Show-on-Recording-only. Erscheint bei `RecordingStarted`-Event, verschwindet nach `RecordingCompleted` mit 300ms CSS-Fade-Out.

**Architektur-Kontext:** Die Pill-Bar ist eine zweite Tauri-WebView-Fensterin (`label = "pill-bar"`, transparent, no-deco, always-on-top), NICHT React. Der Comment im Architektur-Dokument "Native PillBar (nicht React!)" bedeutet: kein React-Component-Framework, kein Build-Step — nur Plain-HTML + Canvas. Das Fenster wird dynamisch via `WebviewWindowBuilder` in `.setup()` erzeugt (nicht statisch in `tauri.conf.json` — weil anfangs hidden und on-demand managed).

**Waveform-Datenquelle:** `CpalAudioSource` emittiert bereits `AudioEvent::Level { rms: f32, ts_ms }` per Audio-Chunk (~15.6Hz bei 1024 Samples / 16kHz). Diese werden via neuer `Event::AudioLevel`-Variante auf dem Core-EventBus weitergeleitet; der Pill-Bar-Subscriber akkumuliert sie in einem 64-Bin-RingBuffer und emittiert `pill_bar.waveform_tick` an die Pill-Bar-WebView.

**Deferred-Work-Bezug:** Story 9.1 hat `9.1-W1 (WaitAndType Restore-Timing-Race mit Pill-Bar)` als deferred abgelegt — dieser Scope-Note ist in den Dev Notes aufgegriffen.

**Session-Lifecycle-Anker (memory/project_shell_session_lifecycle):** Pill-Bar visualisiert Steps 1-7 der per-Hotkey-Cycle 7-Step-Topology:
- Step 1 (Hotkey Press) → `RecordingStarted` → Pill-Bar show
- Steps 2-4 (Audio Capture + VAD) → `AudioLevel` events → waveform
- Step 7 (Drop + cleanup) → `RecordingCompleted` → 300ms Fade → hide

## Acceptance Criteria

### AC-1: `Event::AudioLevel` in `klarvo-core/src/event/bus.rs`

**Given** `klarvo-core/src/event/bus.rs` enthält das `Event`-Enum ohne `#[non_exhaustive]`,
**When** AC-1 committed ist,
**Then**:

```rust
/// RMS audio level tap for Pill-Bar waveform (Shell-subscriber accumulates
/// into 64-bin ring buffer; not forwarded to main WebView by EventMirror).
/// `rms` is 0.0..=1.0 (same range as `AudioEvent::Level`).
AudioLevel { rms: f32, ts_ms: u64 },
```

Eingefügt **nach** `RecordingDelivered`.

**Alle** exhaustive `match event`-Stellen im Codebase müssen einen neuen Arm erhalten:
- `shells/windows/src-tauri/src/bridge.rs` `mirror_event()` → Arm `Event::AudioLevel { .. } => return,` (AudioLevel wird NICHT an die Haupt-WebView weitergeleitet — high-frequency, falscher Consumer).
- `shells/windows/src-tauri/src/main.rs` Tray-Subscriber-Task → `_ => {}` arm (bereits am Ende des `match event { ... }`-Blocks oder explizit `Event::AudioLevel { .. } => {}`).

`cargo check -p klarvo-core` → Exit 0.

### AC-2: Orchestrator emittiert `Event::AudioLevel` aus Audio-Level-Events

**Given** `klarvo-shell-orchestrator/src/session.rs` erstellt den AudioEvent-Broadcast-Channel `(tx, rx)` vor dem `CaptureConfig`-Build,
**When** AC-2 committed ist,
**Then**: Ein zweiter Subscriber `level_rx = tx.subscribe()` wird **vor** der `CaptureConfig`-Konstruktion angelegt (damit kein Event verpasst wird):

```rust
let (tx, rx) = tokio::sync::broadcast::channel::<AudioEvent>(DEFAULT_AUDIOEVENT_CAPACITY);
let level_rx = tx.subscribe();  // Pill-Bar level tap — before CaptureConfig consumes tx
let config = CaptureConfig { sample_rate: 16_000, channels: 1, events: tx };
```

Direkt nach dem `pipeline_task`-Spawn wird ein separater **Level-Tap-Task** gespawnt (lebt bis zum Session-Ende = bis der EventBus-Clone gedropt wird):

```rust
// Level-Tap-Task: forward AudioEvent::Level to EventBus as Event::AudioLevel.
// Task terminates when `level_rx` channel is closed (audio source dropped at session end).
let event_bus_level = Arc::clone(&event_bus);
let clock_level = Arc::clone(&clock);
tokio::spawn(async move {
    let mut rx = level_rx;
    loop {
        match rx.recv().await {
            Ok(AudioEvent::Level { rms, ts_ms }) => {
                event_bus_level.emit(Event::AudioLevel { rms, ts_ms });
            }
            Ok(AudioEvent::Samples { .. }) => {}  // not consumed here
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!(skipped = n, "level-tap lagged; skipped AudioLevel events");
            }
            Err(broadcast::error::RecvError::Closed) => break,
        }
    }
    let _ = clock_level; // keep alive
});
```

Der Task lässt `AudioEvent::Samples` unbeachtet (kein CPU-Overhead für Waveform-Berechnung auf Sample-Ebene).

`cargo check -p klarvo-shell-orchestrator` → Exit 0.

### AC-3: `overlay/` Modul-Scaffold

**Given** `shells/windows/src-tauri/src/` hat kein `overlay/`-Verzeichnis,
**When** AC-3 committed ist,
**Then** existieren:
- `shells/windows/src-tauri/src/overlay/mod.rs` — `pub mod pill_bar;`
- `shells/windows/src-tauri/src/overlay/pill_bar.rs` — Placeholder mit `PillBar`-Struct (Inhalt in AC-5)

Und `shells/windows/src-tauri/src/lib.rs` enthält (unter `pub mod tray;`):
```rust
#[cfg(target_os = "windows")]
pub mod overlay;
```

### AC-4: `pill-bar.html` — Transparente Overlay-Seite

**Given** `shells/windows/src/` enthält nur `index.html` und `bindings/`,
**When** AC-4 committed ist,
**Then** existiert `shells/windows/src/pill-bar.html`:

```html
<!doctype html>
<html>
<head>
  <meta charset="utf-8" />
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
    html, body {
      background: transparent;
      overflow: hidden;
      width: 320px; height: 48px;
    }
    #pill {
      width: 320px; height: 48px;
      background: rgba(13, 15, 20, 0.85);
      border-radius: 24px;
      display: flex; align-items: center; justify-content: center;
      opacity: 1;
      transition: opacity 300ms ease-out;
    }
    #pill.fade-out { opacity: 0; }
    canvas { display: block; }
  </style>
</head>
<body>
  <div id="pill">
    <canvas id="waveform" width="280" height="32"></canvas>
  </div>
  <script type="module">
    const tauriEvent = window.__TAURI__?.event ?? null;
    const pill = document.getElementById("pill");
    const canvas = document.getElementById("waveform");
    const ctx = canvas.getContext("2d");
    const BIN_COUNT = 64;

    // Ring buffer: current 64 bins (initialised to zero)
    let bins = new Float32Array(BIN_COUNT);

    function drawWaveform() {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      const barW = Math.floor(canvas.width / BIN_COUNT);
      const gap = 1;
      for (let i = 0; i < BIN_COUNT; i++) {
        const h = Math.max(2, bins[i] * canvas.height);
        const x = i * barW;
        const y = (canvas.height - h) / 2;
        const alpha = 0.5 + bins[i] * 0.5;
        ctx.fillStyle = `rgba(42, 195, 168, ${alpha})`;
        ctx.beginPath();
        ctx.roundRect(x, y, Math.max(1, barW - gap), h, 2);
        ctx.fill();
      }
    }

    // Initial idle state
    drawWaveform();

    if (tauriEvent) {
      // Waveform tick: update ring buffer + redraw
      tauriEvent.listen("pill_bar.waveform_tick", (evt) => {
        const payload = evt?.payload;
        if (payload && Array.isArray(payload.bins)) {
          const incoming = payload.bins;
          // Full ring-buffer replacement (backend sends complete 64-bin snapshot)
          for (let i = 0; i < BIN_COUNT && i < incoming.length; i++) {
            bins[i] = incoming[i];
          }
          drawWaveform();
        }
      });

      // Show: clear fade-out class (Rust already called show() before emitting)
      tauriEvent.listen("pill_bar.show", () => {
        pill.classList.remove("fade-out");
        bins.fill(0);
        drawWaveform();
      });

      // Fade-out: apply CSS transition; Rust calls hide() 300ms later
      tauriEvent.listen("pill_bar.fade_out", () => {
        pill.classList.add("fade-out");
      });
    }
  </script>
</body>
</html>
```

**Hinweise:**
- `background: transparent` auf `html, body` ist Pflicht — ohne das zeigt Tauri ein weißes Fenster, auch wenn `transparent: true` gesetzt ist.
- `ctx.roundRect()` ist in aktuellen Chromium-Versionen (Tauri WebView) verfügbar. Fallback: `ctx.fillRect()` wenn nicht verfügbar (nicht spec'd — dev-option).
- Keine externen Dependencies, kein Build-Step (passt zur aktuellen No-Vite-Phase).
- `bins` empfängt Full-Snapshot (64 Werte) — kein Client-seitiges Shifting nötig, da Backend den RingBuffer verwaltet.

### AC-5: `PillBar` in `overlay/pill_bar.rs`

**Pre-Flight-Finding (2026-05-04):** `transparent: true` erfordert in Tauri v2 zwingend einen Eintrag in `tauri.conf.json` — `.transparent(true)` im `WebviewWindowBuilder` alleine hat keine Wirkung (GitHub Issue #8308). Konsequenz: das Pill-Bar-Fenster wird **in `tauri.conf.json` deklariert** (AC-6a) und von Tauri beim App-Start automatisch erzeugt. `PillBar::new()` verwendet daher `get_webview_window()` + `set_position()` statt `WebviewWindowBuilder`. Der `WebviewWindowBuilder`-Import entfällt.

**Given** `overlay/mod.rs` existiert (AC-3) und `tauri.conf.json` enthält den pill-bar-Eintrag (AC-6a),
**When** AC-5 committed ist,
**Then** enthält `overlay/pill_bar.rs`:

```rust
use std::collections::VecDeque;
use tokio::sync::broadcast;
use tauri::{AppHandle, Emitter as _, LogicalPosition, Manager as _};
use klarvo_core::event::Event;

const WINDOW_LABEL: &str = "pill-bar";
const BIN_COUNT: usize = 64;

pub struct PillBar<R: tauri::Runtime> {
    app: AppHandle<R>,
}

impl<R: tauri::Runtime> PillBar<R> {
    /// Wire up the pill-bar window (declared in tauri.conf.json) and position it.
    ///
    /// Called once in .setup(). The window is already created by Tauri from conf;
    /// we only set the bottom-center position here. Fail-soft: if the window label
    /// is missing (misconfigured conf), returns Ok without crashing — waveform just
    /// won't show, but recording still works.
    pub fn new(app: &AppHandle<R>) -> tauri::Result<Self> {
        if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
            let (x, y) = pill_bar_position(app);
            let _ = win.set_position(LogicalPosition::new(x, y));
        } else {
            tracing::warn!("pill-bar window not found; check tauri.conf.json label");
        }
        Ok(Self { app: app.clone() })
    }

    /// Spawn the EventBus subscriber task that drives show/hide/waveform.
    ///
    /// Task terminates when the EventBus channel is closed (app shutdown).
    pub fn start(self, mut receiver: broadcast::Receiver<Event>) {
        let app = self.app;
        tauri::async_runtime::spawn(async move {
            let mut ring: VecDeque<f32> = VecDeque::from(vec![0.0f32; BIN_COUNT]);

            loop {
                match receiver.recv().await {
                    Ok(event) => handle_event(&app, &mut ring, event).await,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "PillBar lagged; skipped events");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }
}

/// Compute bottom-center logical position for the pill-bar window.
fn pill_bar_position<R: tauri::Runtime>(app: &AppHandle<R>) -> (f64, f64) {
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let scale = monitor.scale_factor();
        let size = monitor.size();
        let logical_w = size.width as f64 / scale;
        let logical_h = size.height as f64 / scale;
        let x = (logical_w - 320.0) / 2.0;
        let y = logical_h - 48.0 - 16.0; // 16px margin from taskbar
        (x, y)
    } else {
        (0.0, 0.0) // fallback: top-left (fail-soft)
    }
}

async fn handle_event<R: tauri::Runtime>(
    app: &AppHandle<R>,
    ring: &mut VecDeque<f32>,
    event: Event,
) {
    match event {
        Event::RecordingStarted { .. } => {
            // Reset ring, show window, signal WebView to clear fade-out state
            ring.iter_mut().for_each(|v| *v = 0.0);
            if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
                let _ = win.show();
            }
            let _ = app.emit_to(WINDOW_LABEL, "pill_bar.show", ());
        }
        Event::RecordingCompleted { .. } => {
            // Signal WebView to start CSS fade; hide window after 300ms
            let _ = app.emit_to(WINDOW_LABEL, "pill_bar.fade_out", ());
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                if let Some(win) = app_clone.get_webview_window(WINDOW_LABEL) {
                    let _ = win.hide();
                }
            });
        }
        Event::AudioLevel { rms, ts_ms } => {
            // Push new level into ring buffer, drop oldest
            ring.pop_front();
            ring.push_back(rms.clamp(0.0, 1.0));

            // Emit full snapshot to WebView
            let bins: Vec<f32> = ring.iter().copied().collect();
            let _ = app.emit_to(WINDOW_LABEL, "pill_bar.waveform_tick", WaveformPayload { bins, ts_ms });
        }
        _ => {}
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct WaveformPayload {
    bins: Vec<f32>,
    ts_ms: u64,
}
```

**Wichtig:** `PillBar` ist `#[cfg(target_os = "windows")]`-gated via `lib.rs`-Modul-Declaration (AC-3). Kein separates `cfg` in `pill_bar.rs` nötig.

### AC-6a: `tauri.conf.json` — Pill-Bar-Window-Eintrag

**Pre-Flight-Finding (2026-05-04):** `transparent: true` muss in `tauri.conf.json` stehen; Builder alleine reicht nicht.

**Given** `shells/windows/src-tauri/tauri.conf.json` enthält nur einen `main`-Window-Eintrag,
**When** AC-6a committed ist,
**Then** hat `app.windows` einen zweiten Eintrag:

```json
{
  "label": "pill-bar",
  "url": "pill-bar.html",
  "title": "Klarvo",
  "width": 320,
  "height": 48,
  "minWidth": 320,
  "minHeight": 48,
  "resizable": false,
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "focus": false,
  "visible": false,
  "fullscreen": false
}
```

Der `"url": "pill-bar.html"` Pfad ist relativ zu `frontendDist = "../src"` — Tauri löst ihn zu `shells/windows/src/pill-bar.html` auf.

### AC-6b: Wire-Up in `main.rs`

**Given** `main.rs` hat einen Step-12-Block für Tray + EventMirror + NotificationService,
**When** AC-6b committed ist,
**Then** wird nach den bestehenden `event_bus.subscribe()` Calls ein weiterer Receiver ergänzt:

```rust
let event_bus_rx_pill_bar = event_bus.subscribe();
```

Und nach `Step 12c` (NotificationService):

```rust
// Step 12d: PillBar — transparent overlay window for recording visualization.
// Window is declared in tauri.conf.json (transparent: true requires conf entry).
// Fail-soft: missing window label logs a warning; recording continues without overlay.
match klarvo_windows_shell::overlay::pill_bar::PillBar::new(app.handle()) {
    Ok(pill_bar) => pill_bar.start(event_bus_rx_pill_bar),
    Err(e) => {
        tracing::error!(error = %e, "pill-bar setup failed; continuing without overlay");
    }
}
```

**Import-Ergänzung in `main.rs`:** Kein separater Import nötig, da über vollqualifizierten Pfad referenziert.

### AC-7: Gates — Lint + Cross-Compile

**When** alle ACs committed sind,
**Then**:
- `cargo xtask lint-events` → Exit 0. **Keine** neuen i18n-Keys in Pill-Bar-Scope (UX-Decision: keine User-facing Strings im MVP — nur Icons/Canvas).
- `cargo check --target x86_64-pc-windows-gnu -p klarvo-windows-shell` → Exit 0.
- `cargo check -p klarvo-core` → Exit 0 (`AudioLevel`-Variant compiliert).
- `cargo check -p klarvo-shell-orchestrator` → Exit 0 (Level-Tap-Task compiliert).

## Tasks / Subtasks

- [ ] `Event::AudioLevel` in `klarvo-core/src/event/bus.rs` + exhaustive-match-Updates (AC-1)
  - [ ] Neue Variante `AudioLevel { rms: f32, ts_ms: u64 }` einfügen
  - [ ] `bridge.rs` — `Event::AudioLevel { .. } => return,` Arm hinzufügen
  - [ ] `main.rs` Tray-Subscriber — `_ => {}` Arm oder expliziter AudioLevel-Arm
- [ ] Orchestrator Level-Tap-Task (AC-2)
  - [ ] `level_rx = tx.subscribe()` vor `CaptureConfig` in `session.rs`
  - [ ] Level-Tap-Task spawnen (filtert auf `AudioEvent::Level`)
- [ ] `overlay/` Modul-Scaffold + `lib.rs`-Declaration (AC-3)
- [ ] `pill-bar.html` erstellen (AC-4)
  - [ ] Transparent background HTML/Canvas setup
  - [ ] `pill_bar.waveform_tick` listener + Canvas-Draw
  - [ ] `pill_bar.show` listener (reset bins)
  - [ ] `pill_bar.fade_out` listener (CSS transition)
- [ ] `tauri.conf.json` pill-bar Window-Eintrag mit `transparent: true` (AC-6a) — **VOR AC-5**
- [ ] `PillBar` Struct in `overlay/pill_bar.rs` (AC-5)
  - [ ] `PillBar::new()` — `get_webview_window()` + `set_position()` (kein WebviewWindowBuilder)
  - [ ] `pill_bar_position()` — bottom-center via `primary_monitor()`
  - [ ] `PillBar::start()` — EventBus subscriber task
  - [ ] `handle_event()` — RecordingStarted / RecordingCompleted / AudioLevel arms
  - [ ] `WaveformPayload` serde struct
- [ ] Wire-Up in `main.rs` (AC-6b)
  - [ ] `event_bus_rx_pill_bar` subscribe
  - [ ] Step 12d mit fail-soft
- [ ] Gates verifizieren (AC-7)
  - [ ] `cargo xtask lint-events` → Exit 0
  - [ ] `cargo check --target x86_64-pc-windows-gnu -p klarvo-windows-shell` → Exit 0
  - [ ] `cargo check -p klarvo-core` + `-p klarvo-shell-orchestrator` → Exit 0

## Dev Notes

### Tauri v2 WebviewWindowBuilder API

`WebviewWindowBuilder::new(app_handle, label, url)` — `label` ist der eindeutige Window-Identifier. `.build()` gibt `tauri::Result<WebviewWindow<R>>`. 

Relevante Builder-Methoden für transparent overlay:
- `.decorations(false)` — kein Titelbar/Border
- `.transparent(true)` — requires Tauri config `"transparent": true` pro Window ODER global via `tauri::Config::app.windows[n].transparent`
- `.always_on_top(true)` — `WS_EX_TOPMOST` auf Win32
- `.skip_taskbar(true)` — Fenster erscheint nicht in der Taskleiste
- `.focused(false)` — Fenster fokussiert sich beim Show nicht
- `.visible(false)` — initial hidden

**CRITICAL:** `transparent: true` auf Tauri-Fenstern erfordert auf Windows, dass DWM Composition aktiv ist (Windows 8+). Kein Workaround nötig — Klarvo-Target ist Win10+.

**CRITICAL:** Nach `.build()` ist das Fenster in der `main.rs` Setup-Closure sofort verfügbar via `app.get_webview_window("pill-bar")`. Der `PillBar`-Struct muss das Fenster nicht als Feld halten — `app.get_webview_window()` ist cheap (HashMap-Lookup im AppHandle-State).

### `emit_to` vs `emit`

`app.emit("event", payload)` → Alle Windows.
`app.emit_to(label, "event", payload)` → Nur dieses Window.

Pill-Bar-Events (`pill_bar.waveform_tick`, `pill_bar.show`, `pill_bar.fade_out`) **müssen** `emit_to(WINDOW_LABEL, ...)` verwenden — `emit()` würde die Events auch an die Haupt-WebView (`index.html`) schicken, was dort zu unbehandelten Event-Callbacks führt.

Beide Methoden sind auf `tauri::Emitter` trait — `use tauri::Emitter as _;` in `pill_bar.rs`.

### `AudioEvent::Level` Frequenz und Ring-Buffer-Semantik

`CpalAudioSource` emittiert `AudioEvent::Level` ~15.6× pro Sekunde (1024 Samples / 16000 Hz = 64ms per Chunk). Bei 64 Bins = ~4.1 Sekunden scrollende Waveform-History. Die Frequency ist leicht unter dem geplanten 20Hz-Target (Decisions Doc), aber für MVP-Visualisierung ausreichend.

**Ring Buffer:** Die `VecDeque<f32>` in `handle_event` ist per-Subscriber-Task-lokal (nicht Arc'd). Das ist korrekt — kein Shared-State benötigt.

**rms-Normalisierung:** `AudioEvent::Level { rms }` ist 0.0..=1.0 per `CpalAudioSource`-Contract. `rms.clamp(0.0, 1.0)` in `handle_event` ist eine Defensive-Measure für zukünftige AudioSource-Impls.

### Tauri `transparent` + Win32 — Pre-Flight-Finding

**CRITICAL (pre-flight 2026-05-04):** In Tauri v2 reicht `.transparent(true)` im `WebviewWindowBuilder` auf Windows NICHT. `transparent: true` muss zusätzlich in `tauri.conf.json` pro Window-Eintrag gesetzt sein — sonst zeigt das Fenster einen weißen Hintergrund (kein Compile-Fehler, stilles visuelles Bug). Quelle: [GitHub Issue #8308](https://github.com/tauri-apps/tauri/issues/8308), Tauri v2 Window-Customization-Docs.

**Konsequenz für diese Story:** Das Pill-Bar-Fenster wird in `tauri.conf.json` deklariert (AC-6a). `PillBar::new()` verwendet `get_webview_window()` statt `WebviewWindowBuilder` (AC-5 amended).

Tauri v2 nutzt intern `WS_EX_LAYERED` für transparente Fenster auf Windows. Zusätzlich gilt:
- `skip_taskbar: true` → `WS_EX_TOOLWINDOW` auf Win32. Verhindert Alt+Tab-Fokus. Gewünscht.
- `focus: false` in conf + `always_on_top: true` → Fenster stehlt keinen Fokus beim Show. Kritisch für WaitAndType-Flow.

### `pill-bar.html` Tauri-Availability Check

Die Pill-Bar-WebView läuft in einem separaten Fenster-Kontext von `index.html`. `window.__TAURI__` ist dennoch verfügbar (Tauri injiziert das in alle WebViews), aber `tauriEvent` ist defensiv geguardet: `window.__TAURI__?.event ?? null`. Bei `null` ist `listen()` nie aufgerufen — kein Crash im Dev-Build ohne Tauri-Runtime.

### `ctx.roundRect()` Kompatibilität

Chromium >= 99 (shipped im Tauri-Bundled WebView für Windows). `tauri.conf.json` setzt kein Minimum — akzeptabel, da Klarvo Win10+ (Chromium im WebView2 ist aktuell genug). Falls `ctx.roundRect` nicht verfügbar: `ctx.fillRect()` als stiller Fallback. Nicht spec'd — Dev-Entscheidung.

### Focus-Capture WaitAndType Race (9.1-W1)

`deferred-work.md §9.1-W1` dokumentiert: WaitAndType Restore-Timing-Race mit Pill-Bar. Das Pill-Bar-Fenster ist `.focused(false)` und `.skip_taskbar(true)` — es sollte den Fokus NICHT stehlen. Story 9.1's `WinFocusCapture::capture()` speichert das Foreground-Window vor dem Hotkey. Das `pill_bar.rs`-Show-Pfad ruft nur `win.show()` — kein `win.set_focus()`. Race-Window ist damit in MVP geschlossen. Falls doch ein Regression auftritt, liegt es an Win32-`SetForegroundWindow`-Verhalten auf `.always_on_top`-Fenstern — separater Bugfix.

### Exhaustive Match auf `Event` enum — alle Stellen

Mit `Event::AudioLevel` neu in Core gibt es 3 exhaustive `match event`-Stellen:
1. `bridge.rs:154` `mirror_event()` — Arm hinzufügen: `Event::AudioLevel { .. } => return,`
2. `main.rs:508` Tray-Subscriber — neuer Arm oder `_ => {}` (kein Tray-State-Change für AudioLevel)
3. `notification.rs` — nutzt `if let`, kein exhaustive match → kein Update nötig

`pill_bar.rs` `handle_event()` — hat `_ => {}` catch-all, ist already future-safe.

### Neue Dateien / Geänderte Dateien

**Neue Dateien:**
- `shells/windows/src-tauri/src/overlay/mod.rs`
- `shells/windows/src-tauri/src/overlay/pill_bar.rs`
- `shells/windows/src/pill-bar.html`

**Geänderte Dateien:**
- `shells/windows/src-tauri/tauri.conf.json` — pill-bar Window-Eintrag (AC-6a, **zuerst**)
- `klarvo-core/src/event/bus.rs` — `AudioLevel`-Variante
- `klarvo-shell-orchestrator/src/session.rs` — `level_rx` + Level-Tap-Task
- `shells/windows/src-tauri/src/lib.rs` — `pub mod overlay;`
- `shells/windows/src-tauri/src/bridge.rs` — `AudioLevel`-Arm
- `shells/windows/src-tauri/src/main.rs` — Tray-Subscriber `_ => {}` + Step 12d PillBar-Wireup

### References

- `_bmad-output/planning-artifacts/pill-bar-ux-decisions.md` — alle 4 Decisions + Cross-Cutting-Notes
- `klarvo-core/src/event/bus.rs:30-56` — `Event`-Enum (Ausgangspunkt für AC-1)
- `klarvo-core/src/audio/events.rs` — `AudioEvent::Level { rms, ts_ms }` Definition
- `klarvo-shell-orchestrator/src/session.rs:156-200` — Broadcast-Channel-Setup + Pipeline-Task
- `shells/windows/src-tauri/src/bridge.rs:153-184` — `EventMirror::mirror_event()` exhaustive match
- `shells/windows/src-tauri/src/main.rs:450-533` — Step 12 (Tray/EventMirror/Notification); Tray-Subscriber match line 508
- `shells/windows/src-tauri/src/main.rs:534-541` — Step 12c NotificationService (Step 12d kommt danach)
- `shells/windows/src-tauri/src/notification.rs` — `if let` Pattern (kein exhaustive match — kein Update nötig)
- `_bmad-output/implementation-artifacts/deferred-work.md §9.1-W1` — WaitAndType-Race-Hinweis
- `memory/project_shell_session_lifecycle` — 7-Step-Topology
- `memory/project_event_ts_ms_convention` — `ts_ms` auf `AudioLevel`-Variant
- `memory/project_event_routing_quirks` — `ErrorEmitted` geht direkt ans Frontend; `AudioLevel` geht zur Pill-Bar (analog)
- Architecture.md:292 — "Pill-Bar-State-Transitions Rust-getriggert (nicht JS)"
- Architecture.md:569 — "Pill-Bar selbst ist native Win-Overlay"

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (create-story 2026-05-04)

### Debug Log References

### Completion Notes List

### File List
