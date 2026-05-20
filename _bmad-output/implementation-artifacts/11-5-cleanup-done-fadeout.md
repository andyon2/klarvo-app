---
name: Story 11.5 — CleanupDone-Morph + FadeOut
epic: 11
story_number: "11.5"
status: review
dependencies:
  - "11.4"  # LP-state infra (.live-preview class, #lp-area, #lp-text, LivePreviewChunk event)
inputDocuments:
  - _bmad-output/planning-artifacts/ux-design-specification.md  # §2.5.4, §2.5.8, §C1
  - _bmad-output/planning-artifacts/epics.md                    # Epic 11 story-sequence
  - shells/windows/src/pill-bar.html                            # file being modified
  - shells/windows/src-tauri/src/overlay/pill_bar.rs            # file being modified
  - klarvo-core/src/event/bus.rs                                # Event enum — new variant
  - klarvo-shell-orchestrator/src/session.rs                    # emission site for CleanupDone
---

# Story 11.5: CleanupDone-Morph + FadeOut

Status: review

## Story

Als Klarvo-User
möchte ich nach dem Transkriptions-Ergebnis einen kurzen visuellen Erfolgs-Blitz sehen (success-edge border + Fade-Pulse auf dem Text), bevor die Pill Bar ausblendet —
damit ich bestätigtes Feedback bekomme, dass das Diktat erfolgreich abgeschlossen wurde.

Als Klarvo-Entwickler
möchte ich, dass `Event::CleanupDone` als Core-Daten-Contract existiert und die Pill Bar darauf reagiert,
damit Phase-2-Cleanup-Stages (non-Verbatim Filter) denselben Event-Pfad nutzen können ohne Shell-Änderungen.

## Context & Motivation

**State nach 11.4:** Die Pill Bar wechselt nach dem Loslassen der Taste in den LP-State (480×84, Text + Side-Strip), zeigt den STT-Text und bleibt so bis `pill_bar:fade_out` (getriggert durch `RecordingCompleted`). Es gibt keinen visuellen Übergang zwischen "STT-Text anzeigen" und "Fade-Out".

**Was 11.5 ergänzt:**
- `Event::CleanupDone { text, ts_ms }` als neues Core-Event
- Emission in `session.rs` zwischen LivePreviewChunk und Delivery — gleiche Positions-Invariante: vor Paste, damit Pill Bar aktualisiert ist bevor der Fokus wechselt
- In der Pill Bar: CleanupDone-State mit success-edge border, Abort-Button versteckt, Fade-Pulse-Animation (150ms) auf `#lp-text`
- Reset bei neuer Session

**Phase-1-Vereinfachung:** In Phase 1 (Verbatim) ist der Cleanup-Output identisch mit dem LP-Text. Die "Morph" ist daher inhaltlich eine No-Op — aber die CSS-Transition (Fade-Pulse + success border) tritt trotzdem auf. Der Benutzer sieht den kurzen Erfolgs-Blitz. Phase-2+-Stages (z.B. Polished) werden einen anderen Text produzieren; der Morph ist dann sichtbar inhaltlich.

**Reihenfolge im Session-Task (Phase 1):**
```
LivePreviewChunk { text: "..." }    ← LP-State zeigt STT-Text
CleanupDone     { text: "..." }    ← CleanupDone-State, Fade-Pulse, success border
→ Delivery (clipboard + focus + Ctrl+V)  ← kurz sichtbar während Paste
RecordingCompleted                  → schedule_fade_and_hide → pill_bar:fade_out
```

**Scope-Abgrenzung:**
- Error-State in Pill Bar → 11.6
- Echte Text-Transformation (non-Verbatim Cleanup) → Phase-2+
- Hold-Delay vor FadeOut (explizites Warten nach CleanupDone) → kein explizites Hold; RecordingCompleted kommt direkt nach Delivery
- Tauri-Window-Resize bei CleanupDone → NEIN, Pill bleibt auf 480×84 (LP-Größe)

## Acceptance Criteria

### AC-1: `Event::CleanupDone` im Core-Event-Bus

**Given** `klarvo-core/src/event/bus.rs`,
**then** existiert folgender neuer Variant in `pub enum Event`:

```rust
/// Cleanup pipeline stage completed; shell emits `pill_bar:cleanup_done` to
/// frontend for the CleanupDone-State morph.
///
/// Phase-1: emitted once after single-shot pipeline completes; `text` =
/// Verbatim-passthrough = raw STT output (identical to `LivePreviewChunk.text`).
/// Phase-2+: emitted once after the filter stage transforms the full STT output;
/// `text` is the filter-transformed result and differs from `LivePreviewChunk`.
///
/// `text` is NOT an i18n key — it is the cleaned transcript payload.
CleanupDone { text: String, ts_ms: u64 },
```

### AC-2: Emission in `klarvo-shell-orchestrator/src/session.rs`

**Given** `text_to_deliver` ist `Some(text)` (Pipeline hat Text produziert),
**when** die Session aktiv ist und der Text nicht leer ist (gleiche Guards wie LivePreviewChunk),
**then** wird **nach dem LivePreviewChunk-Block** (ab Zeile ~376) und **vor dem Delivery-Block** (ab Zeile ~385) emittiert:

```rust
if let Some(ref text) = text_to_deliver {
    if session_active && !text.is_empty() {
        event_bus.emit(Event::CleanupDone {
            text: text.clone(),
            ts_ms: clock.now_ms(),
        });
    }
}
```

**Ordering-Invariante:** CleanupDone muss vor `paste_backend.paste()` auf dem Bus ankommen. Die Emission ist sync (non-blocking broadcast), Delivery ist async danach — Reihenfolge durch sequenzielle Codeposition garantiert.

**Guard-Symmetrie:** Dieselben `session_active && !text.is_empty()` Guards wie LivePreviewChunk (gleiche EC-1/EC-2-Schutz aus 11.4-Pass-2).

### AC-3: `handle_event(CleanupDone)` in `pill_bar.rs`

**Given** `Event::CleanupDone { text, ts_ms }` eintrifft im PillBar-EventBus-Subscriber,
**then** (consistent mit LivePreviewChunk-Arm):

```rust
Event::CleanupDone { text, ts_ms } => {
    if let Some(_win) = app.get_webview_window(WINDOW_LABEL) {
        let _ = app.emit_to(WINDOW_LABEL, "pill_bar:cleanup_done", CleanupDonePayload { text, ts_ms });
    } else {
        tracing::debug!("CleanupDone dropped: pill-bar window not available");
    }
}
```

**Kein Resize** — Pill bleibt auf LP-Größe (480×84). Resize ist nur in `RecordingStarted` (Reset auf 320×48) und `LivePreviewChunk` (480×84) definiert.

**Neuer Struct in `pill_bar.rs`:**

```rust
#[derive(Debug, Clone, Serialize)]
struct CleanupDonePayload {
    text: String,
    ts_ms: u64,
}
```

**Match-Exhaustiveness:** `Event::CleanupDone` muss als eigener Match-Arm vor `_ => {}` stehen (analog LivePreviewChunk in 11.4).

### AC-4: CSS — `.cleanup-done` State + Fade-Pulse-Animation

**Given** `shells/windows/src/pill-bar.html`,
**then** existieren folgende CSS-Ergänzungen:

```css
/* Default: transparenter Border (verhindert Layout-Shift beim Übergang) */
#pill {
  border: 1px solid transparent;
}

/* CleanupDone: success-edge border (var(--klarvo-color-success) at 30% alpha) */
#pill.cleanup-done {
  border-color: color-mix(in srgb, var(--klarvo-color-success) 30%, transparent);
}

/* CleanupDone: kein Abort-Button */
#pill.cleanup-done #abort-btn { display: none; }

/* Fade-Pulse: ~150ms (var(--klarvo-timing-fast)) auf dem Text-Element */
@keyframes lp-morph-pulse {
  0%   { opacity: 1; }
  40%  { opacity: 0.15; }
  100% { opacity: 1; }
}
.lp-morph-pulse {
  animation: lp-morph-pulse var(--klarvo-timing-fast) ease;
}
```

**Token-Compliance:**
- `color-mix(in srgb, var(--klarvo-color-success) 30%, transparent)` statt hardcoded RGBA — CSS Level 5 `color-mix`, supported in Chromium ≥ 111 (Tauri WebView2 via Chromium >= 113)
- `var(--klarvo-timing-fast)` = 150ms aus tokens.css (design-tokens.toml `timing.fast`)

**LP-Area bleibt sichtbar:** `.cleanup-done` ergänzt `.live-preview` — LP-Area-Visibility wird durch `.live-preview` Regeln geregelt (unverändert aus 11.4). Kein Conflict.

**Cross-State FadeOut:** `#pill { transition: opacity var(--klarvo-timing-medium) ease-out; }` und `.fade-out { opacity: 0; }` gelten für alle States inkl. `.cleanup-done`. Kein zusätzlicher CSS-Code nötig.

### AC-5: JS — `pill_bar:cleanup_done` Listener

**Given** ein `pill_bar:cleanup_done`-Event mit Payload `{ text: string, ts_ms: number }` eintrifft,
**then**:
1. `#lp-text.textContent` wird auf `payload.text` gesetzt
2. Fade-Pulse-Animation wird retriggert (reflow-trick)
3. `#pill` bekommt `.cleanup-done` Klasse

```javascript
tauriEvent.listen('pill_bar:cleanup_done', (evt) => {
  const payload = evt?.payload;
  if (payload && typeof payload.text === 'string') {
    lpText.textContent = payload.text;
    // Retrigger animation (handles rapid successive calls)
    lpText.classList.remove('lp-morph-pulse');
    void lpText.offsetWidth;  // force reflow to restart CSS animation
    lpText.classList.add('lp-morph-pulse');
    pill.classList.add('cleanup-done');
  }
});
```

**Kein separater Enter-Handler nötig:** `.live-preview`-Klasse wurde bereits von `pill_bar:enter_live_preview` (11.4) gesetzt. CleanupDone-State setzt `.cleanup-done` additiv.

### AC-6: JS — Reset bei `pill_bar:show`

**Given** `pill_bar:show` eintrifft (neue Recording-Session startet),
**then** wird zusätzlich zu den bestehenden Reset-Aktionen (11.3/11.4) auch `.cleanup-done` entfernt:

```javascript
tauriEvent.listen('pill_bar:show', () => {
  pill.classList.remove('fade-out');
  pill.classList.remove('live-preview');
  pill.classList.remove('cleanup-done');   // ← NEU
  lpText.textContent = '';
  bins.fill(0);
  updateBars();
  updateSbars();
});
```

### AC-7: `cargo check` grün

`cargo check -p klarvo-core`, `cargo check -p klarvo-shell-orchestrator`, und `cargo check -p klarvo-windows-shell --lib` produzieren 0 Errors, 0 Warnings.

## Tasks / Subtasks

- [x] AC-1: `Event::CleanupDone` Variant in `klarvo-core/src/event/bus.rs`
  - [x] Variant + Rustdoc hinzufügen
  - [x] `cargo check -p klarvo-core` clean
- [x] AC-2: Emission in `session.rs`
  - [x] CleanupDone-Block nach LivePreviewChunk-Block, vor Delivery-Block (gleiche Guards)
  - [x] `cargo check -p klarvo-shell-orchestrator` clean
- [x] AC-3 + Struct: `handle_event(CleanupDone)` + `CleanupDonePayload` in `pill_bar.rs`
  - [x] Match-Arm vor `_ => {}`
  - [x] `CleanupDonePayload` Struct (Serialize, kein Deserialize)
  - [x] `cargo check -p klarvo-windows-shell --lib` clean (bridge.rs CleanupDone-Arm ebenfalls ergänzt)
- [x] AC-4: CSS in `pill-bar.html`
  - [x] `border: 1px solid transparent` auf `#pill`
  - [x] `.cleanup-done` border-color rule
  - [x] `.cleanup-done #abort-btn { display: none }`
  - [x] `@keyframes lp-morph-pulse` + `.lp-morph-pulse` class
- [x] AC-5: JS `pill_bar:cleanup_done` Listener in `pill-bar.html`
- [x] AC-6: `.cleanup-done` in `pill_bar:show` Reset in `pill-bar.html`
- [ ] Visuelle Verifikation (cargo tauri dev — golden path)

## Dev Notes

### Current State (after 11.4)

**`pill_bar.rs` handle_event match:**
- `RecordingStarted` → reset size to 320×48, show, emit `pill_bar:show`
- `RecordingCompleted` → `schedule_fade_and_hide` (emit `pill_bar:fade_out`, delay-hide)
- `AudioLevel` → ring-buffer update, emit `pill_bar:waveform_tick`
- `RecordingAborted` → `schedule_fade_and_hide`
- `LivePreviewChunk` → resize 480×84, emit `pill_bar:enter_live_preview`, emit `pill_bar:live_preview_chunk`
- `_ => {}` ← `CleanupDone` kommt hier als neuer Arm rein, VOR `_`

**`session.rs` emission sequence (lines ~364-456):**
```
1. LivePreviewChunk block (lines ~376-383) — session_active + !empty guard
2. [NEUE CleanupDone Emission HIER]
3. Delivery block (lines ~385-445) — clipboard + focus + paste
4. RecordingCompleted (line 456)
```

**`pill-bar.html` JS-State nach 11.4:**
- `pill_bar:show` → entfernt `.fade-out`, `.live-preview`; leert `#lp-text`
- `pill_bar:fade_out` → setzt `.fade-out`
- `pill_bar:enter_live_preview` → setzt `.live-preview`
- `pill_bar:live_preview_chunk` → updated `#lp-text`
- `pill_bar:waveform_tick` → updated bins + bars + sbars

**CSS-State nach 11.4:**
- `#pill` hat kein explizites `border` → 11.5 fügt `border: 1px solid transparent` hinzu (box-sizing: border-box, kein Layout-Impact)
- `.live-preview` zeigt `#lp-area`, versteckt `#waveform` und `#mode-badge`
- `.fade-out` → opacity 0

### Tauri Event-Naming-Convention

Nach dem Release-Build-Migration-Commit (`30630d3`) nutzt das Projekt **Colon-Separator** für Pill-Bar-Events: `pill_bar:show`, `pill_bar:fade_out`, `pill_bar:enter_live_preview`, `pill_bar:live_preview_chunk`, `pill_bar:waveform_tick`.

Das neue Event heißt: **`pill_bar:cleanup_done`** (Colon-Separator, kein Dot).

### CleanupDonePayload vs LivePreviewPayload

`CleanupDonePayload` ist strukturell identisch mit `LivePreviewPayload`. Trotzdem separater Struct — verschiedene semantische Bedeutung (final cleanup result vs. STT partial/full preview). Kein Refactoring zu shared type in 11.5 (Premature-Abstraction-Guard: erst bei Phase-2-Bedarf).

### color-mix Kompatibilität

Tauri v2 auf Windows nutzt WebView2 (Chromium-based). `color-mix(in srgb, ...)` ist ab Chromium 111 (März 2023) supported. Windows-10/11-WebView2-Auto-Updates garantieren modernes Chromium. Kein RGB-Fallback nötig.

### Reihenfolge-Detail: LP → CleanupDone in Phase 1

In Phase 1 feuern LivePreviewChunk und CleanupDone in derselben async task, nahezu gleichzeitig (kein await zwischen ihnen). Die JS-Events kommen sequenziell am WebView an (Tauri IPC-Channel ist ordered). Effekt:
- LP-State zeigt Text (von LivePreviewChunk)  
- CleanupDone: gleicher Text + Fade-Pulse + success border
- Delivery läuft (kurze Zeit sichtbar)
- Fade-Out folgt

In Phase 2 (chunked STT) wird LivePreviewChunk mehrfach während der Aufnahme gefeuert; CleanupDone einmalig nach Pipeline-Abschluss — dort ist der Morph inhaltlich sichtbar.

### `void lpText.offsetWidth` Reflow-Trick

Standard-Pattern zum Neustarten einer CSS-Animation auf demselben Element. Der Browser-Layout-Engine forciert einen Reflow, der den Animation-State zurücksetzt. Notwendig weil bei schnellen aufeinanderfolgenden Recordings die Animation sonst nicht neu startet (Element hat bereits `.lp-morph-pulse`).

### Verzeichnisstruktur nach Story

```
klarvo-core/src/event/
  bus.rs                     ← MODIFIED (CleanupDone Variant)
klarvo-shell-orchestrator/src/
  session.rs                 ← MODIFIED (CleanupDone emit zwischen LP und Delivery)
shells/windows/src/
  pill-bar.html              ← MODIFIED (CSS + JS cleanup-done state)
shells/windows/src-tauri/src/overlay/
  pill_bar.rs                ← MODIFIED (handle_event arm + CleanupDonePayload struct)
```

### References

- UX-Spec §2.5.4: Hold-and-morph, Fade-Pulse ~150ms [Source: ux-design-specification.md, line 501]
- UX-Spec §2.5.8: STT→Cleanup handoff, Fade-Pulse [Source: ux-design-specification.md, line 557]
- UX-Spec §C1: CleanupDone state — success-edge border at 30% alpha, no abort [Source: ux-design-specification.md, lines 998-1019]
- 11.4 Scope-Abgrenzung [Source: 11-4-live-preview-state.md, lines 43-44]
- 11.4 match-arm Pattern (LivePreviewChunk arm) [Source: pill_bar.rs:203-218]
- 11.4 Emission-Guards (session_active + !empty) [Source: session.rs:374-383]
- ADR-0007: emit ignoriert NoReceivers [Source: klarvo-core EventBus comment]
- Tauri Event-Naming Colon-Convention [Source: commit 30630d3]
- Premature-Abstraction-Guard [Source: memory/feedback_premature_abstraction_guard]

## Test Plan

1. **`cargo check` Dreifach**: `cargo check -p klarvo-core`, `cargo check -p klarvo-shell-orchestrator`, `cargo check -p klarvo-windows-shell --lib` → 0 Errors, 0 Warnings.

2. **Visueller Smoke-Test (`cargo tauri dev`):**
   a. Hotkey drücken + sprechen + loslassen.
   b. Pipeline läuft. Pill Bar wächst auf 480×84 (LP-State, Text sichtbar).
   c. **CleanupDone**: kurze Fade-Pulse (~150ms) auf Text, success-edge border (grünliches Leuchten) erscheint, Abort-Button verschwindet.
   d. Text wird in Zielfenster eingefügt.
   e. Pill Bar faded aus (300ms CSS-Transition).
   f. **Nächste Session**: Pill Bar erscheint neu im Recording-State — success-border weg, abort-btn sichtbar, LP-area hidden.

3. **Edge-Case: Kein Text (No-Speech):** Kein LivePreviewChunk, kein CleanupDone. Pill Bar bleibt im Recording-State und faded nach RecordingCompleted direkt aus (kein LP-State, kein CleanupDone-State). Visuell: Recording → FadeOut, kein success-border.

## Commit-Konvention

Empfohlen als zwei Commits:

```
feat(11.5): Event::CleanupDone + session emit + pill_bar.rs handler
feat(11.5): pill-bar CleanupDone CSS state + JS listener + show reset
```

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

- `bridge.rs` match war exhaustive → CleanupDone-Arm dort ebenfalls ergänzt (overlay-only, kein Mirror zur main WebView, analog LivePreviewChunk)

### Completion Notes List

- AC-1: `Event::CleanupDone { text, ts_ms }` mit vollständiger Rustdoc in `bus.rs` ergänzt
- AC-2: Emission zwischen LivePreviewChunk-Block und Delivery-Block in `session.rs`, gleiche `session_active && !text.is_empty()` Guards
- AC-3: `handle_event` Match-Arm + `CleanupDonePayload` Struct in `pill_bar.rs`; zusätzlich `bridge.rs` return-Arm für exhaustiveness
- AC-4: `border: 1px solid transparent` auf `#pill`, `.cleanup-done` success-border via `color-mix`, abort-btn hidden, `@keyframes lp-morph-pulse` + `.lp-morph-pulse`
- AC-5: `pill_bar:cleanup_done` JS-Listener mit reflow-trick für Animation-Retrigger
- AC-6: `pill.classList.remove('cleanup-done')` in `pill_bar:show` Reset ergänzt
- AC-7: Alle drei `cargo check` 0 Errors, 0 Warnings

### File List

- `klarvo-core/src/event/bus.rs` — `Event::CleanupDone` Variant hinzugefügt
- `klarvo-shell-orchestrator/src/session.rs` — CleanupDone emission zwischen LP und Delivery
- `shells/windows/src-tauri/src/overlay/pill_bar.rs` — handle_event Arm + CleanupDonePayload Struct
- `shells/windows/src-tauri/src/bridge.rs` — CleanupDone return-Arm für exhaustive match
- `shells/windows/src/pill-bar.html` — CSS cleanup-done state + JS listener + show-reset
