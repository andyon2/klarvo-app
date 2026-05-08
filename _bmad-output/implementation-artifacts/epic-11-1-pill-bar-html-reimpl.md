---
name: Story 11.1 — Pill Bar HTML Re-Implementation
epic: 11
story_number: "11.1"
status: review
dependencies:
  - "9-6-pill-bar"  # Pill-Bar-Overlay-Infrastructure (PillBar Rust-Struct, tauri.conf.json, EventBus-Subscriber-Task)
inputDocuments:
  - _bmad-output/planning-artifacts/ux-design-specification.md  # §2.5.2 Visual Contract
  - _bmad-output/planning-artifacts/pill-bar-ux-decisions.md    # accepted 2026-05-03
  - docs/backlog.md                                              # "Pill Bar HTML Re-Implementation" entry
  - src/FloatingBar.tsx                                          # v1 visual reference (BAR_COUNT=5, KlarvoLogo, StopButton)
---

# Story 11.1: Pill Bar HTML Re-Implementation

Status: review

## Story

Als täglicher Klarvo-User
möchte ich, dass die Pill-Bar 5 Pill-shaped Waveform-Bars (wie in v1), ein K-Logo links und einen roten Abort-Button rechts zeigt,
damit ich die vertraute v1-Optik bekomme und eine Recording-Abbruch-Möglichkeit habe — ohne die Tastatur loslassen zu müssen.

## Context & Motivation

**UX-Spec §2.5.2 V1-Visual-Continuity-Decision (2026-05-07):**
Die aktuelle `shells/windows/src/pill-bar.html` rendert 64 vertikale Balken (64-bin Canvas-Waveform). Die UX-Spec fordert explizit V1-Kontinuität: 5 Pill-shaped Bars (verbatim aus v1 `src/FloatingBar.tsx`, `BAR_COUNT=5`, `borderRadius:9999`). Diese visuelle Identität ist das Erkennungsmerkmal der App.

**Pre-Decisions bleiben unverändert** (`pill-bar-ux-decisions.md`, accepted 2026-05-03):
- Shape: Fixe Größe 320×48px (UNVERÄNDERT)
- Drag: Nicht draggable (UNVERÄNDERT)
- Waveform: Backend sendet 64 bins via `pill_bar.waveform_tick` (UNVERÄNDERT — nur Frontend-Rendering ändert sich)
- Auto-Hide: Show-on-Recording-only (UNVERÄNDERT)

**Abort Button (UX-Spec §2.5 Abort Affordance):**
Der Abort-Button (roter Square) ist in v1 (`FloatingBar.tsx::StopButton`) vorhanden. v2 hatte ihn noch nicht. Die UX-Spec sagt: "Abort discards the audio buffer, skips Processing/Delivery, and transitions directly back to Idle. No paste." Dies erfordert neues Backend (`Event::RecordingAborted` + `cancel_recording` Tauri-Command + `SessionOrchestrator::cancel_recording`).

**Story-Scope:**
- Wave-1 (diese Story): Visueller Overhaul + Abort-Button-Backend
- Wave-2 (Story 11.2): Floating Pill Bar (Drag, Position-Persistence, mode-dependent Größe ~480×84 für Live-Preview)
- Deferred: Mode-Badge im Pill-Bar (erfordert Event-Erweiterung für aktiven Mode), Abort-Button-Disable während Processing-Phase

## Acceptance Criteria

### AC-1: `Event::RecordingAborted` in `klarvo-core/src/event/bus.rs`

**Given** `klarvo-core/src/event/bus.rs` enthält das `Event`-Enum ohne `#[non_exhaustive]`,
**When** AC-1 committed ist,
**Then**:

Neuer Variant **nach** `RecordingCompleted`:
```rust
/// User aborted the recording session via the Pill-Bar abort button.
/// Audio buffer is discarded; pipeline task is hard-cancelled (no STT call,
/// no paste). Pill-Bar fades out on this event identically to RecordingCompleted.
RecordingAborted { ts_ms: u64 },
```

**Alle** exhaustiven `match event`-Stellen erhalten einen neuen Arm:

1. `shells/windows/src-tauri/src/bridge.rs` `mirror_event()` — letzter Arm vor `Event::AudioLevel`:
   ```rust
   // RecordingAborted: pill-bar fades via its own subscriber; main WebView has no consumer yet.
   Event::RecordingAborted { .. } => return,
   ```
   ⚠️ Diese `match`-Expression hat KEINEN Wildcard-Arm — neuer Variant erzeugt sonst Compile-Error.

2. `shells/windows/src-tauri/src/overlay/pill_bar.rs` `handle_event()` — explizit vor `_ => {}` (damit der Wildcard-Arm nicht stille übernimmt):
   ```rust
   Event::RecordingAborted { .. } => {
       // Fade out identically to RecordingCompleted.
       // No epoch bump needed — abort ends the current session, does not start a new one.
       let _ = app.emit_to(WINDOW_LABEL, "pill_bar.fade_out", ());
       let app_clone = app.clone();
       let epoch_snapshot = fade_epoch.load(Ordering::SeqCst);
       let epoch_clone = Arc::clone(fade_epoch);
       tauri::async_runtime::spawn(async move {
           tokio::time::sleep(std::time::Duration::from_millis(FADE_OUT_MS)).await;
           if epoch_clone.load(Ordering::SeqCst) != epoch_snapshot {
               return; // New recording started within the 300ms fade window — no-op.
           }
           if let Some(win) = app_clone.get_webview_window(WINDOW_LABEL) {
               if let Err(e) = win.hide() {
                   tracing::warn!(error = %e, "pill-bar hide failed (abort path)");
               }
           }
       });
   }
   ```

3. `main.rs` Tray-Subscriber — hat bereits `Ok(_) => {}` als letzten Arm → kein Change nötig, aber add explicit arm for clarity:
   ```rust
   Ok(Event::RecordingAborted { .. }) => {
       // Recording aborted — return tray icon to idle state immediately.
       let _ = tray_handle.set_icon(Some(idle_icon_tray.clone()));
   }
   ```
   Einfügen VOR dem `Ok(_) => {}` catch-all.

`cargo check -p klarvo-core` → Exit 0.

### AC-2: `SessionOrchestrator::cancel_recording` in `klarvo-shell-orchestrator/src/session.rs`

**Given** `klarvo-shell-orchestrator/src/session.rs` enthält `SessionOrchestrator` mit `session_state: Mutex<SessionState>`,
**When** AC-2 committed ist,
**Then** existiert eine neue `pub async fn cancel_recording(&self)`:

```rust
/// Abort the current recording session from the UI (Pill-Bar abort button).
///
/// Differs from `shutdown`: emits `Event::RecordingAborted` after teardown
/// (so the Pill-Bar overlay fades). Does NOT emit `RecordingStopped` (no
/// meaningful audio boundary crossed). Idempotent: second call observes Idle
/// and no-ops.
///
/// Pipeline task is hard-cancelled (abort, not graceful drop) — audio buffer
/// is discarded without STT call or paste (per UX-Spec §2.5 Abort Affordance).
pub async fn cancel_recording(&self) {
    let mut state = self.session_state.lock().await;
    let prev = std::mem::replace(&mut *state, SessionState::Idle);
    drop(state); // release lock before async work
    if let SessionState::Recording {
        capture_handle,
        pipeline_task,
        level_tap_task,
        ..
    } = prev {
        pipeline_task.abort();
        level_tap_task.abort();
        let _ = pipeline_task.await; // JoinError::is_cancelled() expected
        let _ = level_tap_task.await;
        drop(capture_handle);
        self.event_bus.emit(Event::RecordingAborted { ts_ms: self.clock.now_ms() });
    }
}
```

Beachte die Import-Zeile — `Event::RecordingAborted` ist bereits in scope via `use klarvo_core::event::Event`.

`cargo check -p klarvo-shell-orchestrator` → Exit 0.

### AC-3: `commands/recording.rs` + lib.rs-Registration

**Given** `shells/windows/src-tauri/src/commands/` enthält `mod.rs`, `history.rs`, `settings.rs`, `telemetry.rs`,
**When** AC-3 committed ist,
**Then**:

(a) `shells/windows/src-tauri/src/commands/mod.rs` enthält neu:
```rust
pub mod recording;
```

(b) Neue Datei `shells/windows/src-tauri/src/commands/recording.rs`:
```rust
//! Recording control commands (Story 11.1).

use klarvo_shell_orchestrator::SessionOrchestrator;

/// Abort the current recording session (Pill-Bar abort button).
///
/// No-op if no recording is active. Returns immediately — the orchestrator's
/// async teardown (pipeline_task.abort, level_tap_task.abort) completes
/// concurrently; the Pill-Bar fades via `Event::RecordingAborted` on the
/// EventBus subscriber.
#[tauri::command]
#[specta::specta]
pub async fn cancel_recording(
    orch: tauri::State<'_, SessionOrchestrator>,
) -> Result<(), ()> {
    orch.cancel_recording().await;
    Ok(())
}
```

(c) `shells/windows/src-tauri/src/lib.rs`:
- Neue Import-Zeile (neben den anderen `commands::*` imports):
  ```rust
  use commands::recording::cancel_recording;
  ```
- In `specta_builder()` → `collect_commands![...]` Block, neuer Eintrag:
  ```rust
  // Story 11.1: Pill-Bar abort button
  cancel_recording,
  ```

`cargo check -p klarvo-windows-shell` → Exit 0.

### AC-4: `capabilities/pill-bar.json` — Invoke-Permission für Pill-Bar-Window

**Given** `shells/windows/src-tauri/capabilities/default.json` gilt nur für `windows: ["main"]` und das Pill-Bar-Window kann damit keine Tauri-Commands invoken,
**When** AC-4 committed ist,
**Then** existiert `shells/windows/src-tauri/capabilities/pill-bar.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "pill-bar",
  "description": "Pill-bar overlay capability — IPC + event access for cancel_recording invoke",
  "windows": ["pill-bar"],
  "permissions": [
    "core:default"
  ]
}
```

**Hintergrund:** Tauri v2 requires an explicit capability for each window to call any command via `invoke`. `core:default` grants standard IPC access (same as the main window). Scoped granularity (only `cancel_recording`) requires a custom plugin permission — deferred to later as MVP overhead.

`cargo tauri dev` → Pill-Bar-Window kann `cancel_recording` invokeн ohne Security-Error.

### AC-5: `pill-bar.html` — Visueller Overhaul

**Given** `shells/windows/src/pill-bar.html` rendert 64 vertikale Canvas-Balken ohne K-Logo oder Abort-Button,
**When** AC-5 committed ist,
**Then** ersetzt `shells/windows/src/pill-bar.html` den Canvas-Block durch folgendes Layout und Script:

**Layout (innerhalb `<body>`):**
```html
<div id="pill">
  <!-- K-Logo: verbatim aus v1 KlarvoLogo component (FloatingBar.tsx) -->
  <div id="k-logo">K</div>

  <!-- 5 Pill-shaped Waveform-Bars (v1 BAR_COUNT=5, borderRadius:9999) -->
  <div id="waveform">
    <div class="bar" data-idx="0"></div>
    <div class="bar" data-idx="1"></div>
    <div class="bar" data-idx="2"></div>
    <div class="bar" data-idx="3"></div>
    <div class="bar" data-idx="4"></div>
  </div>

  <!-- Abort button: verbatim aus v1 StopButton component -->
  <button id="abort-btn" title="Cancel recording" aria-label="Cancel recording">
    <div id="abort-square"></div>
  </button>
</div>
```

**CSS (vollständiger Ersatz):**
```css
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
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 10px;
  opacity: 1;
  transition: opacity 300ms ease-out;
}
#pill.fade-out { opacity: 0; }

/* K-Logo: verbatim v1 KlarvoLogo (24×24 rounded, Teal #14B8A6, white K) */
#k-logo {
  width: 24px; height: 24px;
  border-radius: 6px;
  background: #14B8A6;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 14px;
  color: #fff;
  line-height: 1;
  font-family: system-ui, -apple-system, sans-serif;
  user-select: none;
}

/* 5-bar waveform container */
#waveform {
  display: flex;
  align-items: center;
  gap: 3px;
  flex: 1;
  height: 28px;
  min-width: 0;
}

/* Individual pill-shaped bars — Trust-Anchor Teal #14B8A6 (v1 verbatim) */
.bar {
  flex: 1;
  border-radius: 9999px;
  background: #14B8A6;
  opacity: 0.85;
  height: 4px;      /* Updated via JS on each waveform_tick */
  min-height: 4px;
  max-height: 28px;
  transition: height 40ms linear;
}

/* Abort button: v1 StopButton — red square on transparent button */
#abort-btn {
  width: 22px; height: 22px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  cursor: pointer;
  border-radius: 2px;
  padding: 0;
}
#abort-btn:hover #abort-square {
  opacity: 1;
}
#abort-square {
  width: 10px; height: 10px;
  border-radius: 1px;
  background: rgba(248, 113, 113, 0.9);  /* v1: rgba(248,113,113,0.9) */
  opacity: 0.85;
}
```

**JavaScript (vollständiger Ersatz des `<script type="module">` Blocks):**
```javascript
const tauriCore  = window.__TAURI__?.core  ?? null;
const tauriEvent = window.__TAURI__?.event ?? null;

const pill      = document.getElementById('pill');
const abortBtn  = document.getElementById('abort-btn');
const bars      = Array.from(document.querySelectorAll('.bar'));

const BIN_COUNT = 64;
const BAR_COUNT = 5;

// bins: full 64-element snapshot from the last waveform_tick
let bins = new Float32Array(BIN_COUNT);

// Map 64 bins → 5 bars by sampling evenly-spaced indices (matches v1's approach
// from FloatingBar.tsx: levelIdx = Math.round(i/(BAR_COUNT-1)*(levels.length-1)))
function updateBars() {
  for (let i = 0; i < BAR_COUNT; i++) {
    const binIdx = Math.round(i / (BAR_COUNT - 1) * (BIN_COUNT - 1));
    const amplitude = Math.max(0.12, bins[binIdx] ?? 0);
    const heightPx = Math.round(amplitude * 28);
    bars[i].style.height = `${heightPx}px`;
  }
}

// Initial idle state: all bars at minimum height
updateBars();

// Abort button: invoke cancel_recording Tauri command
abortBtn.addEventListener('click', () => {
  if (!tauriCore) return;
  tauriCore.invoke('cancel_recording').catch((e) => {
    console.error('[pill-bar] cancel_recording failed:', e);
  });
});

if (tauriEvent) {
  // Waveform tick: update bins + redraw 5 bars
  tauriEvent.listen('pill_bar.waveform_tick', (evt) => {
    const payload = evt?.payload;
    if (payload && Array.isArray(payload.bins)) {
      const incoming = payload.bins;
      const limit = Math.min(BIN_COUNT, incoming.length);
      for (let i = 0; i < limit; i++) {
        const v = incoming[i];
        bins[i] = Number.isFinite(v) ? v : 0;
      }
      for (let i = limit; i < BIN_COUNT; i++) {
        bins[i] = 0;
      }
      updateBars();
    }
  });

  // Show: clear fade-out class, reset bars
  tauriEvent.listen('pill_bar.show', () => {
    pill.classList.remove('fade-out');
    bins.fill(0);
    updateBars();
  });

  // Fade-out: CSS transition; Rust hides window after 300ms (FADE_OUT_MS constant)
  tauriEvent.listen('pill_bar.fade_out', () => {
    pill.classList.add('fade-out');
  });
}
```

**Hinweise für den Dev-Agent:**

- `transition: height 40ms linear` auf `.bar` macht schnelle Updates (20 Hz waveform_tick) weich ohne Lag. Kein `will-change: height` (overkill für 5 Elemente).
- `window.__TAURI__?.core` ist die korrekte Tauri v2 Invoke-API in plain HTML (kein `@tauri-apps/api` Import). Gleiche Pattern wie `shells/windows/src/index.html` (Zeile 91–96).
- `window.__TAURI__?.event` für `listen()` — identisch zum bisherigen pill-bar.html Code.
- Das bisherige `<canvas>` Element und alle canvas-bezogenen Variablen (`ctx`, `canvas`, `drawWaveform`) werden vollständig entfernt.
- Der `<!doctype html>` Header, `<meta charset="utf-8">` und die transparente `html, body`-Basis bleiben erhalten.

### AC-6: TypeScript Bindings regenerieren

**Given** `cancel_recording` ist ein neuer `#[specta::specta]`-Command der dem Bindings-Export bekannt gemacht werden muss,
**When** AC-6 committed ist,
**Then**:

```bash
cargo xtask generate-bindings
```

→ `shells/windows/src/bindings/` enthält aktualisierte `.gen.ts` Datei mit `cancelRecording` Export.

`cargo xtask bindings-drift` → Exit 0 (kein Drift mehr).

**Hinweis:** Das Pill-Bar HTML selbst nutzt `tauriCore.invoke('cancel_recording')` direkt (raw string), nicht den generierten TS-Wrapper. Die Bindings-Regenerierung ist trotzdem nötig, damit `xtask bindings-drift` nicht feuert.

## Dev Notes

### Datei-Übersicht (was wird geändert / neu erstellt)

| Datei | Änderung |
|-------|----------|
| `klarvo-core/src/event/bus.rs` | NEW: `Event::RecordingAborted { ts_ms }` variant nach `RecordingCompleted` |
| `shells/windows/src-tauri/src/bridge.rs` | UPDATE: `Event::RecordingAborted { .. } => return,` arm in `mirror_event()` |
| `shells/windows/src-tauri/src/overlay/pill_bar.rs` | UPDATE: `Event::RecordingAborted` arm in `handle_event()` |
| `shells/windows/src-tauri/src/main.rs` | UPDATE: `Ok(Event::RecordingAborted { .. })` arm im Tray-Subscriber |
| `klarvo-shell-orchestrator/src/session.rs` | NEW: `pub async fn cancel_recording(&self)` |
| `shells/windows/src-tauri/src/commands/mod.rs` | UPDATE: `pub mod recording;` |
| `shells/windows/src-tauri/src/commands/recording.rs` | NEW: `cancel_recording` Tauri-Command |
| `shells/windows/src-tauri/src/lib.rs` | UPDATE: import + collect_commands! Eintrag |
| `shells/windows/src-tauri/capabilities/pill-bar.json` | NEW: Capability für pill-bar Window |
| `shells/windows/src/pill-bar.html` | UPDATE: vollständiger visueller Overhaul |
| `shells/windows/src/bindings/*.gen.ts` | UPDATE: `cargo xtask generate-bindings` |

### Kritische Reihenfolge

1. Zuerst `bus.rs` (Event-Variant) — sonst Compile-Error in allen nachfolgenden Crates
2. Dann `bridge.rs` — exhaustive match, bricht ohne neuen Arm
3. Dann `overlay/pill_bar.rs`, `session.rs`, `commands/recording.rs`, `lib.rs` — Reihenfolge egal
4. Dann `capabilities/pill-bar.json` — kein Compile-Impact, aber ohne JSON fehlt die Invoke-Permission
5. Dann `pill-bar.html` — reines Frontend, kein Rust-Impact
6. Zuletzt `cargo xtask generate-bindings` (AC-6)

### bridge.rs exhaustive match — ACHTUNG

`bridge.rs mirror_event()` hat KEINEN `_ => {}` Wildcard-Arm. Das ist Absicht (Compile-Zeit-Sicherheit). Neue Varianten MÜSSEN explizit eingetragen werden. `RecordingAborted` → `return` (nicht forwarden an Main-WebView; der Pill-Bar-Subscriber reagiert selbst über den EventBus).

### Tray-Subscriber — Idle-Return beim Abort

Im Tray-Subscriber (main.rs) setzen wir beim Abort den Icon auf `idle_icon_tray` zurück (AC-1 Punkt 3). Der Tray zeigt bei `RecordingStopped` + `RecordingCompleted` den normalen Flow — beim Abort sollte der Tray ebenfalls auf Idle zurück, damit kein Recording-Icon hängen bleibt.

### pill_bar.rs — Fade-Epoch beim Abort

Der `fade_epoch` Zähler wird NUR bei `RecordingStarted` erhöht (neue Session startet). Im Abort-Pfad (`RecordingAborted`) kein Erhöhen — die Abort-Session endet, startet aber keine neue. Race-Safety: Wenn der User unmittelbar nach Abort erneut den Hotkey drückt, erhöht `RecordingStarted` die Epoch — die Abort-Fade-Task sieht Mismatch und no-ops korrekt.

### cancel_recording Command — Return Type

`Result<(), ()>` ist gewählt weil:
- kein Fehler-Case möglich (idempotent, no-op wenn Idle)
- `()` implementiert `specta::Type`
- `String`-Error wäre falsche Semantik (kein tatsächlicher Fehler-Zustand)

### pill-bar.html — Invoke-API in Tauri v2

```javascript
const tauriCore = window.__TAURI__?.core ?? null;
tauriCore.invoke('cancel_recording')  // korrekt in Tauri v2 plain HTML
```

⚠️ NICHT `window.__TAURI__?.invoke()` (das ist Tauri v1 API).
⚠️ NICHT `window.__TAURI_INTERNALS__.invoke()` (internes, undokumentiertes API).

Gleiche Pattern in `shells/windows/src/index.html` Zeile 91–96 verifiziert.

### Capabilities — Warum core:default für pill-bar?

Tauri v2 ACL: ohne Capability kann ein Window KEINE Commands invokeн. `core:default` gibt das gleiche Basis-IPC wie das Main-Window. Feingranulare Berechtigung (nur `cancel_recording`) erfordert einen Custom-Plugin-Permission-Eintrag — MVP-Overhead, deferred zu Story 11.2 oder später. Pill-Bar ist ein lokales Overlay ohne Netz-Zugriff, Risk-Level akzeptabel.

### 64→5 Bins Mapping

Das Backend sendet 64 Amplitude-Bins pro `pill_bar.waveform_tick`. Die 5 Bars samplen gleichmäßig verteilte Indizes:
- Bar 0 → bin[0]
- Bar 1 → bin[16]  (rundet Math.round(1/4 * 63) = 16)
- Bar 2 → bin[32]  (rundet Math.round(2/4 * 63) = 32)
- Bar 3 → bin[48]  (rundet Math.round(3/4 * 63) = 47)
- Bar 4 → bin[63]

Das entspricht v1's Sampling-Pattern (`levelIdx = Math.round(i/(BAR_COUNT-1)*(levels.length-1))`). Kein averaging — direct sampling ist responsiver.

### Kein `#[non_exhaustive]` auf `Event` Enum

Per AC-1 Story 9.6: das `Event`-Enum hat explizit KEIN `#[non_exhaustive]`. Das ist eine Design-Entscheidung für Compile-Zeit-Vollständigkeit. Jedes neue Variant erfordert Updates an allen exhaustiven match-Sites.

## Deferred (nicht in dieser Story)

| Item | Grund | Heimat |
|------|-------|--------|
| Abort-Button verstecken während Processing-Phase | Erfordert neues Event für Pipeline-Stage-Visibility im Pill-Bar | Story 11.2 oder separates Deferred-Work |
| Mode-Badge im Pill-Bar (Hold/Toggle/Auto-Label) | Erfordert Event mit aktivem Mode-Snapshot | Story 11.2 |
| Floating Pill Bar (Drag, Position-Persistence) | Mode-dependent Größe (~480×84 Live-Preview) | Story 11.2 |
| Feingranulare Capability (nur cancel_recording) | Custom-Plugin-Permission-Overhead | Story 11.2 oder later |
| `cargo check --target x86_64-pc-windows-gnu` vor Closure | Per `feedback_windows_cross_compile_verify` Memory | Dev-Agent-Pflicht vor PR |

## Test Plan

1. **`cargo check -p klarvo-core`** → 0 nach AC-1
2. **`cargo check -p klarvo-shell-orchestrator`** → 0 nach AC-2
3. **`cargo check -p klarvo-windows-shell`** → 0 nach AC-3
4. **`cargo check --target x86_64-pc-windows-gnu -p klarvo-windows-shell`** (MinGW cross-compile) → 0 — pflicht per `feedback_windows_cross_compile_verify`
5. **`cargo xtask generate-bindings`** → Exit 0, Bindings-Datei aktualisiert
6. **`cargo xtask bindings-drift`** → Exit 0
7. **Manuell (Windows Runtime):** `cargo tauri dev` → Pill-Bar zeigt K-Logo + 5 Pill-Bars + Abort-Button während Recording
8. **Manuell:** Abort-Button click → Recording stoppt sofort, Pill-Bar faded out, kein Paste
9. **Manuell:** Abort-Button während Idle (kein Recording) → kein Crash (Orchestrator no-op)
10. **Manuell:** Hotkey-Press → Abort → sofort erneuter Hotkey-Press → neue Recording startet korrekt (Epoch-Race kein Problem)

## Dev Agent Record

### Completion Notes

All 6 ACs implemented in one session (2026-05-08):

- **AC-1**: `Event::RecordingAborted { ts_ms: u64 }` added to `bus.rs` after `RecordingCompleted`. All three exhaustive match sites updated: `bridge.rs` (return — no main WebView consumer), `pill_bar.rs` (fade-out path identical to RecordingCompleted, no epoch bump), `main.rs` tray-subscriber (idle icon restore, inserted before `Ok(_) => {}` catch-all).
- **AC-2**: `cancel_recording()` added to `SessionOrchestrator` in `session.rs`. Hard-aborts pipeline_task + level_tap_task, drops capture_handle, emits `Event::RecordingAborted`. Idempotent (Idle no-op). Pattern mirrors `shutdown()` except it emits RecordingAborted instead of nothing.
- **AC-3**: `commands/recording.rs` created with `cancel_recording` Tauri command (`Result<(), ()>` return type). `mod.rs` and `lib.rs` updated with import + collect_commands! entry.
- **AC-4**: `capabilities/pill-bar.json` created with `core:default` permission for the `pill-bar` window.
- **AC-5**: `pill-bar.html` fully replaced — Canvas/ctx/drawWaveform removed; 5 pill-shaped divs + K-Logo + Abort-Button added. JS uses `window.__TAURI__?.core` for `invoke` (Tauri v2 pattern). 64→5 bin mapping via Math.round sampling.
- **AC-6**: `cargo xtask generate-bindings` run → `cancelRecording` exported. `cargo xtask bindings-drift` → OK.

**Cross-compile note**: `cargo check --target x86_64-pc-windows-gnu` fails on `whisper-rs-sys` (pre-existing MinGW/Linux-bindgen size mismatch, confirmed on baseline). `klarvo-core` and `klarvo-shell-orchestrator` pass MinGW cross-compile cleanly. `klarvo-windows-shell --lib` passes on Linux. MinGW failure predates Story 11.1.

## File List

- `klarvo-core/src/event/bus.rs` — NEW variant `RecordingAborted { ts_ms: u64 }`
- `shells/windows/src-tauri/src/bridge.rs` — NEW arm `Event::RecordingAborted { .. } => return`
- `shells/windows/src-tauri/src/overlay/pill_bar.rs` — NEW arm `Event::RecordingAborted` with fade-out logic
- `shells/windows/src-tauri/src/main.rs` — NEW arm `Ok(Event::RecordingAborted { .. })` for tray idle-restore
- `klarvo-shell-orchestrator/src/session.rs` — NEW method `cancel_recording()`
- `shells/windows/src-tauri/src/commands/mod.rs` — NEW `pub mod recording`
- `shells/windows/src-tauri/src/commands/recording.rs` — NEW file: `cancel_recording` Tauri command
- `shells/windows/src-tauri/src/lib.rs` — NEW import + `cancel_recording` in `collect_commands!`
- `shells/windows/src-tauri/capabilities/pill-bar.json` — NEW capability file
- `shells/windows/src/pill-bar.html` — REPLACED: Canvas → 5 pill-bars + K-Logo + Abort-Button
- `shells/windows/src/bindings/index.ts` — UPDATED: `cancelRecording` export added

## Change Log

- 2026-05-08: Story 11.1 implementation — `Event::RecordingAborted` backend + `cancel_recording` Tauri command + `capabilities/pill-bar.json` + Pill-Bar visual overhaul (5 pill-shaped bars, K-Logo, Abort-Button). Bindings regenerated.
