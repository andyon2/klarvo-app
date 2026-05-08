---
name: Story 11.4 — LivePreview-State (Text + Side-Strip-Waveform + Live-Update)
epic: 11
story_number: "11.4"
status: done
dependencies:
  - "11.3"  # LP-size-infra (480×84), enter_live_preview JS-listener + .live-preview class, LIVE_PREVIEW_WIDTH/HEIGHT constants
inputDocuments:
  - _bmad-output/planning-artifacts/ux-design-specification.md  # §2.5.8, §C1 (LP anatomy: text-area + side-strip 8 bars)
  - _bmad-output/planning-artifacts/epics.md                    # Epic 11 story-sequence + scope note
  - shells/windows/src/pill-bar.html                            # file being modified — read before touching
  - shells/windows/src-tauri/src/overlay/pill_bar.rs            # file being modified — read before touching
  - klarvo-core/src/event/bus.rs                                # Event enum — new variant goes here
  - klarvo-shell-orchestrator/src/session.rs                    # emission site for LivePreviewChunk
---

# Story 11.4: LivePreview-State — Text + Side-Strip-Waveform + Live-Update

Status: done

## Story

Als Klarvo-User
möchte ich nach dem Loslassen der Aufnahme-Taste das transkribierte Wort sehen, bevor es eingefügt wird —
damit ich ein visuelles Feedback habe, was erkannt wurde.

Als Klarvo-Entwickler
möchte ich, dass `Event::LivePreviewChunk` als Core-Daten-Contract existiert und die Pill Bar darauf reagiert,
damit Phase-2-Chunked-STT den gleichen Event-Pfad nutzen kann ohne Shell-Änderungen.

## Context & Motivation

**Phase-1-Vereinfachung:** §2.5.8 beschreibt langfristig "every ~3-5 s a chunk while capture continues". In Phase 1 läuft die Pipeline single-shot (Audio-Capture → STT → Verbatim → Deliver). `Event::LivePreviewChunk` wird nach Pipeline-Abschluss emittiert — der Text ist das vollständige STT-Ergebnis (= Verbatim-Passthrough). Echtes Chunking ist Phase-2+-Scope. Die Infrastruktur (Event-Contract, Shell-Handler, JS-Listener) ist identisch.

**Dependency-Präzisierung:** Story 11.3 hat folgende Infrastruktur geliefert:
- `LIVE_PREVIEW_WIDTH = 480.0`, `LIVE_PREVIEW_HEIGHT = 84.0` als Konstanten in `pill_bar.rs`
- JS-Listener für `pill_bar.enter_live_preview` → fügt `.live-preview` zu `#pill` hinzu
- CSS: `#pill.live-preview #mode-badge { display: none; }`
- `dev_pill_bar_enter_live_preview` Tauri-Command als LP-Test-Trigger
- `pill_bar.show`-Handler entfernt `.live-preview` bei neuer Session

Story 11.4 ergänzt auf dieser Basis den eigentlichen **Inhalt**: Text-Area + Side-Strip-Waveform.

**Scope-Abgrenzung zu 11.5:** Story 11.5 implementiert den CleanupDone-Morph (Text morpht auf Cleanup-Ergebnis, success-edge border). In 11.4 zeigt die Pill Bar den LivePreview-Text bis zum `pill_bar.fade_out`-Event (kein Morph, kein Success-Border). Das ist bewusst, weil in Phase 1 STT-Output = Cleanup-Output (Verbatim). Der Code-Pfad für den Morph gehört in 11.5.

## Scope — Was IN dieser Story ist

| Item | Datei | Typ |
|---|---|---|
| `Event::LivePreviewChunk { text: String, ts_ms: u64 }` Variant | klarvo-core/src/event/bus.rs | MODIFY |
| Emit `LivePreviewChunk` in session pipeline task (nach Text-Extraktion, vor Delivery) | klarvo-shell-orchestrator/src/session.rs | MODIFY |
| `handle_event(LivePreviewChunk)` — LP-Resize + `enter_live_preview` + `live_preview_chunk` emit | shells/windows/src-tauri/src/overlay/pill_bar.rs | MODIFY |
| `LivePreviewPayload { text: String, ts_ms: u64 }` Struct | shells/windows/src-tauri/src/overlay/pill_bar.rs | NEW |
| `#lp-text` Element (Text-Area) + `#side-strip` Container (8 Bars) | shells/windows/src/pill-bar.html | MODIFY |
| CSS: `.live-preview` Layout-Regeln (text-area + side-strip sichtbar, `#waveform` hidden) | shells/windows/src/pill-bar.html | MODIFY |
| JS: `pill_bar.live_preview_chunk` Listener → `#lp-text` update | shells/windows/src/pill-bar.html | MODIFY |
| JS: `pill_bar.waveform_tick` in LP-Mode → Side-Strip 8 Bars updaten | shells/windows/src/pill-bar.html | MODIFY |

## Scope — Was NICHT in dieser Story ist

- Echtes Chunked-STT (Emission während Aufnahme läuft) → Phase-2+
- CleanupDone-Morph (Text → Cleanup-Ergebnis, success-border) → 11.5
- Dynamisches Window-Wachstum mit Text-Content (Wispr-Flow-Pattern "grows to max-size") → Post-MVP (§2.5.8 Open §6)
- Error-State in Pill Bar → 11.6
- `pill_bar.exit_live_preview` Event → nicht nötig, `pill_bar.show` (11.3) setzt `.live-preview` zurück

## Acceptance Criteria

### AC-1: `Event::LivePreviewChunk` im Core-Event-Bus

**Given** `klarvo-core/src/event/bus.rs`,
**then** existiert folgender neuer Variant in `pub enum Event`:

```rust
/// STT-partial-result (Phase-1: full result after single-shot pipeline; Phase-2+: chunked).
/// Shell emits `pill_bar.live_preview_chunk` to frontend on this event.
/// `text` is the raw STT output — not an i18n key.
LivePreviewChunk { text: String, ts_ms: u64 },
```

**Rustdoc-Anmerkung:** "Phase-1: emitted once after single-shot pipeline completes (text = Verbatim-passthrough = final STT output). Phase-2+: emitted per chunk while recording continues."

### AC-2: Emission in `klarvo-shell-orchestrator/src/session.rs`

**Given** `text_to_deliver` ist `Some(text)` (Pipeline hat Text produziert),
**when** der Pipeline-Task diesen Wert extrahiert hat (`match result { Ok(Some(StageData::Text(t))) => Some(t), … }`),
**then** wird **vor dem Delivery-Block** (vor `registry.output(...)`) emittiert:

```rust
if let Some(ref text) = text_to_deliver {
    event_bus.emit(Event::LivePreviewChunk {
        text: text.clone(),
        ts_ms: clock.now_ms(),
    });
}
```

**Reihenfolge-Invariante:** LivePreviewChunk muss vor `paste_backend.paste()` auf dem Bus ankommen, damit die Pill Bar den Text zeigen kann, bevor der Fokus wechselt. Die Emission ist sync (non-blocking broadcast), der Delivery-Block danach ist async — die Reihenfolge ist durch die sequenzielle Codeposition garantiert.

**Fail-soft:** Kein separater Fehler-Pfad für den Emit — `EventBus::emit` ignoriert `SendError::Receivers` per ADR-0007 (kein Subscriber = kein Fehler).

### AC-3: `handle_event(LivePreviewChunk)` in `pill_bar.rs`

**Given** `Event::LivePreviewChunk { text, ts_ms }` eintrifft im PillBar-EventBus-Subscriber,
**then** (in Reihenfolge):
1. Window wird auf `LIVE_PREVIEW_WIDTH × LIVE_PREVIEW_HEIGHT` (480×84) resized — fail-soft (warn!).
2. `pill_bar.enter_live_preview` wird an `WINDOW_LABEL`-WebView emittiert.
3. `pill_bar.live_preview_chunk` wird an `WINDOW_LABEL`-WebView emittiert mit Payload `LivePreviewPayload { text, ts_ms }`.

**Implementierung (Skizze):**

```rust
Event::LivePreviewChunk { text, ts_ms } => {
    if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
        if let Err(e) = win.set_size(LogicalSize::new(LIVE_PREVIEW_WIDTH, LIVE_PREVIEW_HEIGHT)) {
            tracing::warn!(error = %e, "pill-bar LP resize failed");
        }
    }
    let _ = app.emit_to(WINDOW_LABEL, "pill_bar.enter_live_preview", ());
    let _ = app.emit_to(WINDOW_LABEL, "pill_bar.live_preview_chunk", LivePreviewPayload { text, ts_ms });
}
```

**Neuer Struct in `pill_bar.rs`:**

```rust
#[derive(Debug, Clone, Serialize)]
struct LivePreviewPayload {
    text: String,
    ts_ms: u64,
}
```

**Match-Exhaustiveness:** `Event::LivePreviewChunk` muss aus dem `_ => {}` Wildcard-Arm herausgezogen werden — als eigener Arm in `handle_event`s `match event { … }` Block.

### AC-4: HTML — Neue Elemente (`#lp-text` + `#side-strip`)

**Given** `shells/windows/src/pill-bar.html`,
**then** existieren folgende neue Elemente **innerhalb** `<div id="pill">`, **nach** `#abort-btn`:

```html
<!-- Live-Preview content: wired by Story 11.4 -->
<div id="lp-area">
  <div id="lp-text" role="status" aria-live="polite"></div>
  <div id="side-strip">
    <div class="sbar" data-idx="0"></div>
    <div class="sbar" data-idx="1"></div>
    <div class="sbar" data-idx="2"></div>
    <div class="sbar" data-idx="3"></div>
    <div class="sbar" data-idx="4"></div>
    <div class="sbar" data-idx="5"></div>
    <div class="sbar" data-idx="6"></div>
    <div class="sbar" data-idx="7"></div>
  </div>
</div>
```

**Wichtig:** `#lp-area` ist im Recording-State (`#pill` ohne `.live-preview`) **nicht sichtbar** (`display: none`). In LP-State erscheint er.

### AC-5: CSS — `.live-preview` Layout-Regeln

**Given** `.live-preview`-Klasse auf `#pill`,
**then** gelten folgende CSS-Regeln:

```css
/* Recording-State: lp-area ausblenden */
#lp-area { display: none; }

/* Live-Preview: waveform ausblenden, lp-area einblenden */
#pill.live-preview #waveform { display: none; }
#pill.live-preview #lp-area {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 8px;
  min-width: 0;
  overflow: hidden;
}

/* Text-Area: füllt verfügbaren Raum, overflow hidden + ellipsis */
#lp-text {
  flex: 1;
  font-family: system-ui, -apple-system, sans-serif;
  font-size: 13px;
  font-weight: 400;
  color: var(--klarvo-color-text-primary);
  line-height: 1.4;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  user-select: none;
}

/* Side-Strip: 8 schmale Bars rechts */
#side-strip {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
  height: 28px;
}
.sbar {
  width: 3px;
  border-radius: var(--klarvo-radius-pill);
  background: var(--klarvo-color-action);
  opacity: 0.7;
  height: 4px;
  min-height: 4px;
  max-height: 28px;
  transition: height 40ms linear;
}
```

**Token-Compliance:** Alle Farben via `var(--klarvo-*)` — kein hardcoded HEX.

**Visuelle Invariante (Recording-State):** Das Erscheinungsbild ist nach 11.4 im Recording-State **identisch** zum Status nach 11.3 — `#lp-area` ist `display: none`, `#waveform` und `#mode-badge` sind unverändert sichtbar.

### AC-6: JS — `pill_bar.live_preview_chunk` Listener

**Given** ein `pill_bar.live_preview_chunk`-Event mit Payload `{ text: string, ts_ms: number }` eintrifft,
**then**:
1. `#lp-text` wird auf `payload.text` gesetzt.
2. (Die `.live-preview`-Klasse wurde bereits von `pill_bar.enter_live_preview` gesetzt, der kurz davor feuert.)

```javascript
const lpText = document.getElementById('lp-text');
const sbars  = Array.from(document.querySelectorAll('.sbar'));
const SBAR_COUNT = 8;

tauriEvent.listen('pill_bar.live_preview_chunk', (evt) => {
  const payload = evt?.payload;
  if (payload && typeof payload.text === 'string') {
    lpText.textContent = payload.text;
  }
});
```

**Kein separater `enter_live_preview`-Handler nötig:** der Listener aus 11.3 setzt bereits die `.live-preview`-Klasse, die CSS-Regeln übernehmen das Layout-Switching.

### AC-7: JS — Side-Strip 8 Bars via `pill_bar.waveform_tick`

**Given** `pill_bar.waveform_tick` eintrifft (wird **weiterhin** während LP emittiert, da `Event::AudioLevel` bei laufendem Level-Tap-Task feuert — der Level-Tap-Task läuft bis `RecordingCompleted`),
**then** werden **sowohl** die 5 Haupt-Bars (`#waveform .bar`) **als auch** die 8 Side-Strip-Bars (`#side-strip .sbar`) aktualisiert.

**Side-Strip-Mapping (8 Bars auf 64 Bins):**

```javascript
function updateSbars() {
  for (let i = 0; i < SBAR_COUNT; i++) {
    const binIdx = Math.round(i / (SBAR_COUNT - 1) * (BIN_COUNT - 1));
    const amplitude = Math.max(0.08, bins[binIdx] ?? 0);
    const heightPx = Math.round(amplitude * 28);
    sbars[i].style.height = `${heightPx}px`;
  }
}
```

Im bestehenden `waveform_tick`-Handler wird `updateSbars()` **nach** `updateBars()` aufgerufen:

```javascript
tauriEvent.listen('pill_bar.waveform_tick', (evt) => {
  const payload = evt?.payload;
  if (payload && Array.isArray(payload.bins)) {
    // ... bestehender bins-Update Code (unverändert) ...
    updateBars();
    updateSbars();  // ← NEU
  }
});
```

**Hinweis:** Im Recording-State (kein `.live-preview`) sind die `#side-strip`-Bars per CSS unsichtbar (`#lp-area { display: none }`). Das DOM-Update ist billig, kein Conditional nötig.

### AC-8: JS — Reset bei `pill_bar.show`

**Given** `pill_bar.show` eintrifft (neue Recording-Session startet),
**then** wird `#lp-text` geleert (zusätzlich zu den bestehenden Reset-Aktionen aus 11.3):

```javascript
tauriEvent.listen('pill_bar.show', () => {
  pill.classList.remove('fade-out');
  pill.classList.remove('live-preview');
  lpText.textContent = '';      // ← NEU
  bins.fill(0);
  updateBars();
  updateSbars();                // ← NEU (Side-Strip auf Null setzen)
});
```

### AC-9: `cargo check` grün

`cargo check -p klarvo-core`, `cargo check -p klarvo-shell-orchestrator`, und `cargo check -p klarvo-windows-shell --lib` produzieren 0 Errors, 0 Warnings.

## Technical Notes & Dev Guardrails

### Event-Bus-Match exhaustiveness

`handle_event` in `pill_bar.rs` hat aktuell einen Wildcard-Arm `_ => {}`. `LivePreviewChunk` ist ein neuer Variant — er muss als **eigener Match-Arm** vor `_ => {}` stehen:

```rust
Event::LivePreviewChunk { text, ts_ms } => {
    // AC-3
}
_ => {}  // alle anderen Events
```

Falls in Zukunft weitere Event-Variants kommen, bleibt der Wildcard-Arm intentional (nicht alle Core-Events sind für die Pill Bar relevant).

### Level-Tap-Task läuft während LP

Der `level_tap_task` (der `AudioEvent::Level` → `Event::AudioLevel` auf den Bus forwarded) läuft bis die `RecordingHandle` gedropt wird (bei `on_release`) und der Broadcast-Channel sich schließt. Das passiert **nach** `run_capture_session` returned — also kann `Event::AudioLevel` noch einige Frames nach der `LivePreviewChunk`-Emission ankommen. Das ist erwünscht: die Side-Strip-Waveform zeigt bis zum Fade-Out die letzten Audio-Level.

### `emit_to` Reihenfolge in AC-3

`app.emit_to(WINDOW_LABEL, "pill_bar.enter_live_preview", ())` und danach `app.emit_to(WINDOW_LABEL, "pill_bar.live_preview_chunk", payload)`. Die JS-Reihenfolge ist garantiert (single-threaded JS, Events werden sequenziell dispatched). Der `enter_live_preview`-Handler setzt `.live-preview`; der `live_preview_chunk`-Handler setzt den Text. Damit ist beim Text-Update das Layout bereits umgeschaltet.

### `text_to_deliver` Lifetime in session.rs

Die Emission in AC-2 borgt `text` via `ref text` — cloning ist nötig, weil `event_bus.emit(...)` Ownership des Events benötigt und `text_to_deliver` noch für den Delivery-Block danach benötigt wird:

```rust
// Korrekt:
if let Some(ref text) = text_to_deliver {
    event_bus.emit(Event::LivePreviewChunk { text: text.clone(), ts_ms: clock.now_ms() });
}
// Danach: text_to_deliver wird für Delivery-Block genutzt (unverändert)
```

### `Serialize` auf `LivePreviewPayload`

`LivePreviewPayload` benötigt `#[derive(Debug, Clone, Serialize)]` (analog `WaveformPayload` in derselben Datei). Kein `Deserialize` nötig — wird nur von Rust nach JS serialisiert, nie zurück.

### `white-space: nowrap` + `text-overflow: ellipsis` für lp-text

Die Pill Bar ist 480px breit — bei kurzen Diktaten ist das ausreichend. Längere Texte werden abgeschnitten (Ellipsis). Dynamisches Wachstum (§2.5.8 Open §6) ist Post-MVP. Kein Scroll-Mechanismus in 11.4.

### Verzeichnisstruktur nach Story

```
klarvo-core/src/event/
  bus.rs                     ← MODIFIED (LivePreviewChunk Variant)
klarvo-shell-orchestrator/src/
  session.rs                 ← MODIFIED (LivePreviewChunk emit vor delivery)
shells/windows/src/
  pill-bar.html              ← MODIFIED (#lp-area + CSS + JS listeners)
shells/windows/src-tauri/src/overlay/
  pill_bar.rs                ← MODIFIED (handle_event arm + LivePreviewPayload struct)
```

## Test Plan

1. **`cargo check -p klarvo-core`**, **`cargo check -p klarvo-shell-orchestrator`**, **`cargo check -p klarvo-windows-shell --lib`** → 0 Errors, 0 Warnings.
2. **Visueller Smoke-Test (`cargo tauri dev`):**
   a. Hotkey drücken → Pill Bar erscheint im Recording-State (320×48), 5 Waveform-Bars + "Hold"-Badge sichtbar, `#lp-area` unsichtbar.
   b. Sprechen + Hotkey loslassen → Pipeline läuft.
   c. Nach Pipeline-Completion: Pill Bar wächst auf 480×84. `#lp-area` erscheint. `#lp-text` zeigt das transkribierte Wort. `#waveform` ist ausgeblendet, `#mode-badge` ausgeblendet.
   d. `#side-strip` zeigt 8 Bars (letzte Audio-Level aus Level-Tap-Task).
   e. Text wird in das Zielfenster eingefügt.
   f. Pill Bar faded aus (bestehende 300ms Fade-Out-Logik aus 11.1/9.6).
   g. Nächste Session: Pill Bar kehrt zu Recording-State zurück (`.live-preview` entfernt, `#lp-text` leer, 320×48).

## Commit-Konvention

Empfohlen als zwei Commits:

```
feat(11.4): Event::LivePreviewChunk + session emit + pill_bar.rs handler
feat(11.4): pill-bar LP text + side-strip waveform — HTML/CSS/JS
```

## Review Findings

### Pass 1 (2026-05-08, 3 Layer: Blind Hunter ✓, Acceptance Auditor ✓, Edge Case Hunter ✗ failed)

0 Patches, 0 Decisions, 3 Defers, ~14 dismissed.

- [x] [Review][Defer] LP-Emit vor Delivery-Success-Check — Text wird angezeigt, auch wenn Paste später fehlschlägt [`klarvo-shell-orchestrator/src/session.rs:341-349`] — deferred, by-design (Phase-1 Verbatim-passthrough; Error-State + Fail-Visualisierung kommt in Story 11.6)
- [x] [Review][Defer] `emit_to`-Ordering `enter_live_preview` → `live_preview_chunk` formal nicht garantiert [`shells/windows/src-tauri/src/overlay/pill_bar.rs:208-210`] — deferred, Tauri-Per-Window-IPC-Channel ist sequenziell in Praxis; Spec-Tech-Note dokumentiert Annahme. Bei Bruch = einmaliger Frame-Glitch
- [x] [Review][Defer] Resize-vor-Class-Swap kann One-Frame Layout-Flash erzeugen (Recording-Layout in 480×84 vor `.live-preview` greift) [`shells/windows/src-tauri/src/overlay/pill_bar.rs:204-210`] — deferred, Visual-Polish; Fix erfordert Order-Inversion (CSS-Klasse vor Resize) und damit Spec-Deviation. Im Smoke-Test verifizieren

**Auditor-Outcome (Pass 1):** Alle 9 ACs (AC-1 bis AC-9) sind 1:1 spec-konform implementiert. Zwei Out-of-Scope-Touches (bridge.rs EventMirror-Filter, initial `updateSbars()` Boot-Call) sind defensiv-notwendig bzw. konsistent.

**Blind-Hunter-Outcome (Pass 1):** 17 Findings, 3 als Defer eskaliert, Rest dismissed.

**Edge-Case-Hunter-Outcome (Pass 1):** Skill-Invocation hat den Subagent nach Skill-Tool-Call vorzeitig terminiert (1-3 Tool-Uses, kein Output). Erkanntes Subagent-Skill-Limitation. Pass-2 daher manuell vom Coordinator durchgeführt.

### Pass 2 (2026-05-08, manueller Edge-Case-Walk Achsen A–G)

3 Patches applied, 0 Decisions, 0 zusätzliche Defers (die 3 Pass-1-Defers durch Walk bestätigt).

- [x] [Review][Patch] EC-1 Empty-Text-Guard — `text_to_deliver = Some("")` triggert sonst LP-Resize + leere Anzeige [`klarvo-shell-orchestrator/src/session.rs:344-360`] — fixed: `if session_active && !text.is_empty()` gate vor `event_bus.emit`
- [x] [Review][Patch] EC-2 Stale-Session-LP-Suppression — Toggle stop-then-start race konnte stale LP-Text vom alten pipeline_task auf neue Pill-Bar laden [`klarvo-shell-orchestrator/src/session.rs:264-272 + 344-360`] — fixed: `session_counter: Arc<AtomicU64>` auf `SessionOrchestrator`, snapshot via `fetch_add(1)` vor `pipeline_task`-spawn, self-filter im Task vor LP-emit. Delivery-Pfad bleibt unberührt (User bekommt Transcript trotzdem)
- [x] [Review][Patch] EC-3 emit_to inside if-let-window — defensive Konsistenz mit RecordingStarted-Arm [`shells/windows/src-tauri/src/overlay/pill_bar.rs:203-216`] — fixed: alle window-related side-effects inside `if let Some(win) = app.get_webview_window(...)`, `tracing::debug!` auf else-branch

**Achsen-Sweep:** A (extreme inputs) — A1 patched, A2-A7 clean. B (race conditions) — B1/B2/B5 confirm Pass-1-Defers, B6/B7 patched (EC-2). C (state transitions) — C5 patched (EC-3), C1-C4 clean. D (DOM/CSS) — clean. E (failure semantics) — clean (alle fail-soft konsistent). F (cross-file consistency) — F1/F2 are pre-existing project-wide patterns, dismissed. G (spec-boundary) — clean.

**Compile-Verify:** `cargo check -p klarvo-core -p klarvo-shell-orchestrator` clean; `cargo check -p klarvo-windows-shell --lib` clean (host); 27/27 orchestrator-Tests grün. Cross-compile (`--target x86_64-pc-windows-gnu`) blocked durch pre-existing whisper-rs-sys C-header-Issue (nicht durch 11.4-Patches induziert).
