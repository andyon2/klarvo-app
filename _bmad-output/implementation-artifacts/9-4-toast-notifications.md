---
name: Story 9.4 — Toast Notifications
epic: 9
story_number: "9.4"
status: done
dependencies: []
---

# Story 9.4: Toast Notifications

Status: done

## Story

Als täglicher Klarvo-User
möchte ich eine native Windows-Toast-Benachrichtigung sehen, wenn mein Diktat erfolgreich eingefügt wurde oder ein Fehler während der Diktat-Session aufgetreten ist,
damit ich auch dann informiert bin, wenn das Klarvo-Settings-Fenster im Hintergrund oder minimiert ist.

## Kontext und Motivation

**Problem:** Das Settings-Fenster enthält bereits In-App-Toasts (div.toast im SettingsPanel/HistoryPanel), aber diese sind nur sichtbar, wenn das Fenster geöffnet ist. Beim normalen Arbeitsfluss ist das Settings-Fenster meist im Hintergrund. Nach einem Diktat (Recording → Pipeline → Paste) hat der User keine visuelle Bestätigung, dass der Text eingefügt wurde — besonders bei langsamen Groq-Responses (2-4s Latenz).

**Lösung:** Nativer OS-Toast via `tauri-plugin-notification`. Triggered aus dem Rust-Backend (EventBus-Subscriber), nicht aus dem Frontend — kein Tauri-Command, keine Frontend-Änderungen.

**Events:**
1. `Event::RecordingDelivered` → Erfolgs-Toast mit Text-Vorschau
2. `Event::ErrorEmitted` WÄHREND aktiver Recording-Session → Fehler-Toast (verhindert Boot-Error-Spam)

**Scope:** Reine Rust-Backend-Story. Keine Frontend-Änderungen außer 1 neuer i18n-Key in `en.json`/`de.json`. Kein neues Tauri-Command → `bindings-drift` bleibt clean.

**Architektur-Fit:** Muster identisch mit `EventMirror` und Tray-Subscriber (Step 12 in `main.rs`) — dritter unabhängiger EventBus-Subscriber (`event_bus_rx_notification`). Single-tokio-Runtime-Constraint (`memory/project_shell_runtime_model`) via `tauri::async_runtime::spawn`.

## Acceptance Criteria

**SCOPE-AMENDMENT 2026-05-03 (Code-Review-Closure, M1=C / M2=C / M3=C):**

Nach Review-Findings wurde Story 9.4 substantiell scope-reduziert. Original-Spec nahm an, `Event::RecordingDelivered` feuere bei jedem erfolgreichen Diktat und `Event::ErrorEmitted` werde auf den EventBus emittiert — beides verifiziert als nicht zutreffend. Konsequenzen:

- **AC-1 ist auf WaitAndType-Mode reduziert** (M2=C). Hold/Toggle/AutoStop-Pfade emittieren `RecordingDelivered` nicht; Toast erscheint dort nicht. Body-Text wurde auf "Dictation ready"/"Diktat bereit" angepasst (semantisch ehrlich, weil im WaitAndType-Mode der Text noch nicht gepastet ist).
- **AC-2 wurde komplett entfernt** (M1=C). `Event::ErrorEmitted` wird per ADR-0009 §SD-1 direkt an Frontend (`app.error`) geemitted, nicht auf den Bus. Der ErrorEmitted-Arm war Dead Code. Follow-Up-Story (Architektur-Spike) wird separat angelegt.
- **AC-1-Toast-Visibility ist gated auf signed-MSI** (M3=C). Ohne registriertes AppUserModelID (AUMID via Installer) liefert notify-rust auf Windows `Ok(())`, der Toast erscheint aber nicht. Runtime-Validierung folgt erst mit Story 2.A.C1 (signed-msi-installer).

### AC-1: Toast bei `RecordingDelivered` (WaitAndType-Mode)

**Given** der User dictiert in **WaitAndType-Mode** (`press_mode == RecordingMode::WaitAndType`) und die Pipeline emittiert `Event::RecordingDelivered { ts_ms, text }`,
**When** `NotificationService` diesen Event empfängt,
**Then**:

1. Ein nativer Windows-Toast wird angezeigt mit:
   - **Title:** `"Klarvo"`
   - **Body:** `"{label}: {preview}"` wobei:
     - `{label}` = i18n-Lookup von `notification.dictation.delivered` aus der aktuellen `SharedI18nTable`
     - `{preview}` = `text.chars().take(60).collect::<String>()` — Char-basiert, ohne Truncation-Ellipsis wenn ≤60 Chars, mit `…` wenn >60 Chars
   - Beispiel EN: `"Dictation ready: Hello, I'm writing to you regard…"`
   - Beispiel DE: `"Diktat bereit: Hallo, ich schreibe Ihnen bezügli…"`

2. Falls `tauri_plugin_notification` fehlschlägt: `tracing::warn!` + kein Panic (fail-soft).

3. **Visibility-Caveat:** Validierung der tatsächlichen Toast-Sichtbarkeit erfolgt erst nach Story 2.A.C1 (signed-MSI mit AUMID-Registrierung). In dev/portable Builds liefert `show()` `Ok(())`, der Toast wird aber vom Windows Action Center verworfen (kein registrierter AUMID). 2.A.C1 muss als Acceptance-Criterion ergänzen: "AUMID `com.klarvo.v2` wird vom Installer registriert, sodass Toasts aus 9.4 sichtbar werden."

### AC-2 (REMOVED): Fehler-Toast bei `ErrorEmitted`

~~Aus Story 9.4 entfernt 2026-05-03 (M1=C).~~ `Event::ErrorEmitted` wird per ADR-0009 nicht auf den EventBus emittiert; der ErrorEmitted-Arm im NotificationService war unerreichbar. Follow-Up-Story (Architektur-Spike: "Wie kommt `Event::ErrorEmitted` ohne ADR-0009-Verstoß auf den Bus?") wird separat per `bmad-create-story` angelegt.

### AC-3: i18n-Key `notification.dictation.delivered`

**Given** der neue i18n-Key noch nicht in `en.json`/`de.json` existiert,
**When** Story 9.4 committed ist,
**Then**:

**en.json addition:**
```json
"notification.dictation.delivered": "Dictation ready"
```

**de.json addition:**
```json
"notification.dictation.delivered": "Diktat bereit"
```

**Note:** `cargo xtask lint-events` muss nach diesem Add grün bleiben (Key ist orphan-allowlisted unter RUST-LOOKUP-Kategorie wegen `self.t()`-Method-Lookup, der vom G3-D-Visitor nicht erfasst wird).

### AC-4: `cargo check --target x86_64-pc-windows-gnu` clean

**Given** alle Story-Änderungen committed sind,
**When** `cargo check --target x86_64-pc-windows-gnu` ausgeführt wird (MinGW cross-compile, memory/feedback_windows_cross_compile_verify),
**Then** exitiert der Prozess mit Code 0.

**Hinweis:** `tauri-plugin-notification` muss für `x86_64-pc-windows-gnu` kompilieren. Falls der Crate für dieses Target Probleme hat, ist das ein Blocker — im Story-Review dokumentieren.

### AC-5: `cargo xtask bindings-drift` clean

**Given** keine neuen Tauri-Commands in Story 9.4 hinzugefügt werden,
**When** `cargo xtask bindings-drift` ausgeführt wird,
**Then** exitiert der Prozess mit Code 0 (kein diff).

**Sicherheits-Check:** `notification.rs` enthält KEINE `#[tauri::command]`-Annotationen. Alle Toast-Trigger sind EventBus-interne Subscriber — kein Frontend-IPC.

## Tasks / Subtasks

- [x] **T1: Cargo-Dep `tauri-plugin-notification` hinzufügen** (AC-1, AC-4)
  - [x] `Cargo.toml` (Workspace): `tauri-plugin-notification = "2"` in `[workspace.dependencies]`
  - [x] `shells/windows/src-tauri/Cargo.toml`: `tauri-plugin-notification = { workspace = true }` in `[dependencies]`
  - [x] `cargo check --target x86_64-pc-windows-gnu` → clean (1 pre-existing unused-import warning, kein Regression)

- [x] **T2: `notification.rs` Module erstellen** (AC-1)
  - [x] `shells/windows/src-tauri/src/notification.rs` angelegt; nach Code-Review (M1=C) auf reinen `RecordingDelivered`-Subscriber reduziert (kein in_session-Flag, kein ErrorEmitted-Arm)
  - [x] `pub mod notification;` in `shells/windows/src-tauri/src/lib.rs` hinzugefügt (nach `pub mod bridge;`)

- [x] **T3: Plugin registrieren + EventBus-Subscriber wiring in `main.rs`** (AC-1)
  - [x] `.plugin(tauri_plugin_notification::init())` als erstes Plugin auf `tauri::Builder::default()`
  - [x] `let event_bus_rx_notification = event_bus.subscribe();` (Step 12, neben tray + mirror)
  - [x] `notification_i18n = Arc::clone(&i18n_table)` VOR `app.manage(i18n_table)` eingefügt
  - [x] Step-12c nach EventMirror: `NotificationService::new(...).start(event_bus_rx_notification)`

- [x] **T4: i18n-Keys** (AC-3)
  - [x] `shells/windows/locales/en.json`: `"notification.dictation.delivered": "Dictation ready"` (post-review M2=C: WaitAndType-Semantik)
  - [x] `shells/windows/locales/de.json`: `"notification.dictation.delivered": "Diktat bereit"`

- [x] **T5: Verifikation** (AC-4, AC-5)
  - [x] `cargo check --target x86_64-pc-windows-gnu` → clean (5s cached build)
  - [x] `cargo xtask bindings-drift` → OK (index.ts in sync, kein neues Command)
  - [x] `cargo xtask lint-events` → OK (5 events scanned) — inkl. orphan-allowlist-Fix für 9.3-Schulden + notification.dictation.delivered (RUST-LOOKUP-Kategorie)

## Dev Notes

### NotificationService-Skelett

Neue Datei `shells/windows/src-tauri/src/notification.rs`:

```rust
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    RwLock,
};
use tokio::sync::broadcast;
use tauri_plugin_notification::NotificationExt;
use klarvo_core::event::Event;

use crate::i18n::I18nTable;

/// Subscribes to the [`EventBus`] and emits native OS toast notifications
/// for dictation-lifecycle events.
///
/// Two event types trigger notifications:
/// - [`Event::RecordingDelivered`] — always: "Dictation pasted: {preview}"
/// - [`Event::ErrorEmitted`] — only during an active recording session
///   (guard: `in_session` flag, set by `RecordingStarted`, cleared by `RecordingCompleted`).
///   Prevents boot-time errors (config parse, keystore) from generating OS toasts.
///
/// Generic over `R: tauri::Runtime` (idiomatic Tauri v2 — same pattern as
/// `EventMirror` and `TauriErrorEmitter`).
pub struct NotificationService<R: tauri::Runtime> {
    app_handle: tauri::AppHandle<R>,
    i18n: Arc<RwLock<I18nTable>>,
}

impl<R: tauri::Runtime> NotificationService<R> {
    pub fn new(handle: tauri::AppHandle<R>, i18n: Arc<RwLock<I18nTable>>) -> Self {
        Self { app_handle: handle, i18n }
    }

    /// Spawn a background task that drains `rx` and triggers OS notifications.
    /// Returns immediately; the task runs until the channel is closed.
    pub fn start(self, mut rx: broadcast::Receiver<Event>) {
        tauri::async_runtime::spawn(async move {
            let in_session = Arc::new(AtomicBool::new(false));
            loop {
                match rx.recv().await {
                    Ok(event) => self.handle(&event, &in_session),
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "NotificationService lagged; skipped events");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    fn handle(&self, event: &Event, in_session: &Arc<AtomicBool>) {
        match event {
            Event::RecordingStarted { .. } => {
                in_session.store(true, Ordering::Relaxed);
            }
            Event::RecordingCompleted { .. } => {
                in_session.store(false, Ordering::Relaxed);
            }
            Event::RecordingDelivered { text, .. } => {
                let label = self.t("notification.dictation.delivered");
                let preview: String = text.chars().take(60).collect();
                let suffix = if text.chars().count() > 60 { "…" } else { "" };
                let body = format!("{label}: {preview}{suffix}");
                self.show(&body);
            }
            Event::ErrorEmitted { error_key, .. } => {
                if in_session.load(Ordering::Relaxed) {
                    let body = self.t(error_key);
                    self.show(&body);
                }
            }
            _ => {}
        }
    }

    fn t(&self, key: &str) -> String {
        self.i18n
            .read()
            .ok()
            .and_then(|table| table.get(key).cloned())
            .unwrap_or_else(|| key.to_string())
    }

    fn show(&self, body: &str) {
        if let Err(e) = self.app_handle
            .notification()
            .builder()
            .title("Klarvo")
            .body(body)
            .show()
        {
            tracing::warn!(error = %e, "NotificationService: OS notification failed (fail-soft)");
        }
    }
}
```

### `main.rs` Wiring — Diff-Anleitung

**Vor dem `event_bus.subscribe()`-Block (Step 12):**
```rust
// EXISTING:
let event_bus_rx_tray = event_bus.subscribe();
let event_bus_rx_mirror = event_bus.subscribe();
// ADD:
let event_bus_rx_notification = event_bus.subscribe();
```

**VOR `app.manage(i18n_table)` (Step 10 Mitte):**
```rust
// EXISTING:
let boot_i18n = Arc::clone(&i18n_table);
app.manage(i18n_table);
// ADD (vor app.manage):
let notification_i18n = Arc::clone(&i18n_table);  // ADD this line
let boot_i18n = Arc::clone(&i18n_table);
app.manage(i18n_table);
```

**Step 12c — nach `EventMirror::new(...).start(event_bus_rx_mirror)` (Step 12b):**
```rust
// EXISTING Step 12b:
EventMirror::new(app.handle().clone()).start(event_bus_rx_mirror);

// ADD Step 12c:
// Step 12c: NotificationService — native OS toast on recording.delivered (AC-1)
// and on ErrorEmitted during an active recording session (AC-2).
klarvo_windows_shell::notification::NotificationService::new(
    app.handle().clone(),
    notification_i18n,
)
.start(event_bus_rx_notification);
```

**Plugin-Registrierung in `tauri::Builder::default()` (ganz oben im Builder-Chain):**
```rust
let app = tauri::Builder::default()
    // ADD before tauri_plugin_global_shortcut:
    .plugin(tauri_plugin_notification::init())
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    // ... rest unchanged
```

### `lib.rs` — `pub mod notification`

In `shells/windows/src-tauri/src/lib.rs` nach `pub mod bridge;`:
```rust
pub mod bridge;
pub mod notification;  // ADD
pub mod commands;
// ... rest unchanged
```

### `Cargo.toml` Workspace — Neue Dep

In `[workspace.dependencies]` (alphabetisch nach den anderen tauri-plugins):
```toml
tauri-plugin-notification = "2"
```

Hint für Dev: Prüfe `cargo search tauri-plugin-notification` für die aktuell stable 2.x Version und pinne ggf. auf eine Patch-Version (z.B. `"2.2.4"`), analog zu `tauri-plugin-global-shortcut = "2.3.1"`.

### Shell `Cargo.toml` — Plugin-Dep

In `shells/windows/src-tauri/Cargo.toml` unter `[dependencies]` nach `tauri-plugin-global-shortcut`:
```toml
tauri-plugin-notification = { workspace = true }
```

### Capabilities — kein Update nötig

`capabilities/default.json` muss NICHT geändert werden. Die `tauri-plugin-notification` Rust-API (`NotificationExt`) greift direkt auf den WinRT-Layer zu, ohne Frontend-Permission-Check. Die Capability-Permissions (`plugin:notification|allow-send-notification`) wären nur nötig, wenn das Frontend JavaScript-seitig Notifications triggern würde — das ist hier nicht der Fall.

Falls `cargo check` warnt, dass die Permission fehlt, kann `"notification:default"` zu den `permissions` in `default.json` hinzugefügt werden.

### SharedI18nTable und Live-Locale-Switch

`notification_i18n` ist ein Arc-Clone von `i18n_table`. Wenn `reload_locale` (Story 2.A.C3) die i18n-Tabelle via `RwLock::write()` ersetzt, sieht der `NotificationService` die neue Tabelle — weil alle Arc-Clones auf denselben `RwLock` zeigen. Kein extra Wiring nötig.

### WaitAndType-Mode und `RecordingDelivered`

In WaitAndType-Mode liefert die Pipeline Text via `RecordingDelivered`-Event (kein Paste). Der Toast zeigt "Dictation pasted" auch in diesem Mode — das ist eine bekannte Unschärfe in Phase 1 (die Unterscheidung liegt im Backend, nicht im Event-Payload). Deferred: wenn WaitAndType-Mode in Phase 2 eine eigene UI-Surface bekommt, kann ein separater Event-Typ oder ein `mode`-Field im Payload den Toast-Text differenzieren.

### Neue Dateien

- `shells/windows/src-tauri/src/notification.rs`

### Geänderte Dateien

- `Cargo.toml` (workspace) — neue Dep `tauri-plugin-notification`
- `shells/windows/src-tauri/Cargo.toml` — Dep
- `shells/windows/src-tauri/src/lib.rs` — `pub mod notification;`
- `shells/windows/src-tauri/src/main.rs` — Plugin init + 3. EventBus-Subscriber + `notification_i18n` Clone
- `shells/windows/locales/en.json` — 1 Key
- `shells/windows/locales/de.json` — 1 Key

### Cross-Compile-Hinweis

`tauri-plugin-notification` muss für `x86_64-pc-windows-gnu` (MinGW) kompilieren. Das Plugin nutzt WinRT-APIs. Falls MinGW-Link-Fehler auftreten (z.B. `libwindows.a` fehlend oder WinRT-Import-Lib), ist das ein Blocker — dann Tauri-Issue aufmachen und Story als blocked markieren. Alternative: native notification via `windows` crate direkt (Windows-Balloon-Tooltip + WinRT-Toast), aber das ist mehr Implementierungsaufwand.

### Regressions-Schutz

- **SettingsPanel In-App-Toast bleibt unverändert:** In-App-Toast (div.toast) in SettingsPanel/HistoryPanel und OS-Toast sind additiv — kein Conflict.
- **EventMirror unverändert:** 3. Subscriber teilt den Bus, nicht den EventMirror-Code.
- **Bindings-Drift:** Kein neues `#[tauri::command]` → kein Drift.
- **Tray-Subscriber unverändert:** Separate receiver, kein Conflict.

### References

- [architecture.md:591] — "Windows-Toast-Notifications → Phase 2+" (ursprünglich deferred; Story 9.4 zieht vor)
- [memory/project_shell_runtime_model] — Single tokio-Runtime; `tauri::async_runtime::spawn` für alle async Tasks
- [memory/project_shell_session_lifecycle] — per-Hotkey-Cycle 7-Step-Topology; RecordingStarted/Completed Anchors
- [memory/project_i18n_core_contract] — Core emittiert Keys, Shell übersetzt; `notification.rs` nutzt dieselbe I18nTable
- [memory/feedback_windows_cross_compile_verify] — Cross-Compile vor Story-Closure
- [shells/windows/src-tauri/src/bridge.rs] — EventMirror-Pattern (Vorlage für NotificationService)
- [shells/windows/src-tauri/src/main.rs:441-523] — Step 12 Area (Tray + EventMirror Wiring)
- [klarvo-core/src/event/bus.rs] — Event-Enum: RecordingStarted, RecordingCompleted, RecordingDelivered, ErrorEmitted
- [Story 9.3 Dev Notes §App.error-Listener] — In-App-Toast-Pattern (additiv zu OS-Toast, kein Conflict)
- [docs/backlog.md §Windows-Toast-Notifications] — Backlog-Entry, jetzt durch Story 9.4 consumed

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (dev-story 2026-05-03)

### Debug Log References

### Completion Notes List

- `tauri-plugin-notification = "2"` in Workspace-Cargo.toml + Shell-Cargo.toml. Cross-compile x86_64-pc-windows-gnu clean.
- `notification.rs`: `NotificationService<R>` mit `AtomicBool in_session`-Flag. RecordingStarted → true, RecordingCompleted → false. RecordingDelivered → OS-Toast (60-char Char-Preview). ErrorEmitted → OS-Toast nur wenn in_session.
- `main.rs`: `.plugin(tauri_plugin_notification::init())` als erstes Plugin. `notification_i18n = Arc::clone(&i18n_table)` vor manage(). Dritter EventBus-Subscriber `event_bus_rx_notification`. Step-12c nach EventMirror.
- 1 i18n-Key: `notification.dictation.delivered` in en.json + de.json.
- `orphan-allowlist.txt` um 9 Einträge erweitert: 7 Story-9.3-Schulden (FRONTEND-ONLY: history.* + settings.tab.*) + 1 neuer RUST-LOOKUP-Eintrag + neue Kategorie-Kommentare.
- Alle Gates grün: `lint-events OK`, `bindings-drift OK`, `manifest-strict 5/5`, cross-compile clean.

### File List

- `Cargo.toml` — workspace dep: `tauri-plugin-notification = "2"`
- `Cargo.lock` — updated (tauri-plugin-notification resolved)
- `shells/windows/src-tauri/Cargo.toml` — dep: `tauri-plugin-notification = { workspace = true }`
- `shells/windows/src-tauri/src/notification.rs` — NEW: NotificationService<R>
- `shells/windows/src-tauri/src/lib.rs` — `pub mod notification;` hinzugefügt
- `shells/windows/src-tauri/src/main.rs` — plugin-init + notification_i18n clone + event_bus_rx_notification + Step-12c
- `shells/windows/locales/en.json` — 1 neuer Key: notification.dictation.delivered
- `shells/windows/locales/de.json` — 1 neuer Key: notification.dictation.delivered
- `xtask/orphan-allowlist.txt` — 9 neue Einträge (7×FRONTEND-ONLY 9.3-Schulden + 1×RUST-LOOKUP + Kategorie-Kommentar)
- `_bmad-output/implementation-artifacts/9-4-toast-notifications.md` — story-file (status: review)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — 9.4: in-progress → review

### Change Log

- 2026-05-03: Story 9.4 implementiert — tauri-plugin-notification, NotificationService<R> (in_session-Guard, RecordingDelivered-Toast + session-scoped ErrorEmitted-Toast), 1 i18n-Key, orphan-allowlist-Fix für 9.3-Schulden. Status → review.
- 2026-05-03: Code-Review (3-layer: Blind Hunter / Edge Case Hunter / Acceptance Auditor). 2 Critical Decision-Needed (E1/E2 architektonische Spec-Lücken), 1 High Decision-Needed (E3 Runtime-Delivery), 2 Patches, 3 Defers, 5 Dismissed.
- 2026-05-03: Code-Review-Closure: M1=C (AC-2 entfernt), M2=C (WaitAndType-only-Scope), M3=C (Visibility-Caveat → 2.A.C1). 4 Patches applied: DP1 notification.rs gekürzt (in_session-Flag + ErrorEmitted-Arm + RecordingStarted/Completed-Arme entfernt) · DP3 i18n-Body korrigiert (Dictation ready / Diktat bereit) · DP4 AC-1 Visibility-Caveat dokumentiert + 2.A.C1 AC-6 hinzugefügt · P2 RwLock-Poison-Log. cargo check + lint-events + bindings-drift grün. AC-2-Follow-Up muss noch via bmad-create-story angelegt werden. Status → done.

### Review Findings

#### Decision-Needed (resolved 2026-05-03)

- [x] [Review][Decision] **M1 → Option C** — AC-2 aus Story 9.4 entfernt; `Event::ErrorEmitted`-Arm + `in_session`-Flag aus `notification.rs` entfernt. Follow-Up-Story für Error-Toast-Architektur muss separat per `bmad-create-story` angelegt werden (Architektur-Spike: ADR-0009-konformes Bus-Routing für `Event::ErrorEmitted`).
- [x] [Review][Decision] **M2 → Option C** — Story 9.4 auf WaitAndType-Mode-only scoped; Body-Text-Korrektur via i18n-Value-Update ("Dictation ready" / "Diktat bereit"). Hold/Toggle/AutoStop bekommen in dieser Story keinen Toast (deferred — braucht eigenes Event aus dem Hold-Erfolgspfad).
- [x] [Review][Decision] **M3 → Option C** — Toast-Visibility wird auf 2.A.C1 (signed-MSI mit AUMID-Registrierung) deferred; AC-1 enthält explizites Visibility-Caveat. 2.A.C1-Spec sollte ein zusätzliches AC bekommen ("AUMID `com.klarvo.v2` wird via Installer registriert").

#### Patch (applied 2026-05-03)

- [x] [Review][Patch] **DP1 — `notification.rs` auf reduzierten Scope kürzen** [shells/windows/src-tauri/src/notification.rs] — `Event::ErrorEmitted`/`RecordingStarted`/`RecordingCompleted`-Arme + `AtomicBool in_session` entfernt. Verbleibender Match: nur `Event::RecordingDelivered`. Doc-Kommentar amended (begründet die Scope-Beschränkung + verweist auf Follow-Up).
- [x] [Review][Patch] **DP3 — i18n-Body korrigiert auf WaitAndType-Semantik** [shells/windows/locales/en.json + de.json] — `"Dictation pasted"` → `"Dictation ready"`, `"Diktat eingefügt"` → `"Diktat bereit"`.
- [x] [Review][Patch] **DP4 — AC-1 Visibility-Caveat dokumentiert** [9-4-toast-notifications.md AC-1 Punkt 3] — Toast-Sichtbarkeit ist gated auf 2.A.C1-Installer-AUMID; Cross-Reference notiert.
- [x] [Review][Patch] **P2 — RwLock-Poisoning loggt jetzt explizit** [shells/windows/src-tauri/src/notification.rs `t()`] — `.read().ok()` ersetzt durch `match` mit `tracing::warn!(error = %e, key = key, ...)` im Err-Branch.
- [~] [Review][Patch] **DP2 — Follow-Up-Story für AC-2-Architektur** — NICHT in dieser Session angelegt. Anweisung an User: per `bmad-create-story` mit Spike-Anteil "Wie kommt `Event::ErrorEmitted` ohne ADR-0009-Verstoß auf den Bus?" anlegen. Vorschlag: Story 9.7 oder Epic-9-extension-Story.
- [x] [Review][Patch] **P1 — `Lagged` resync von `in_session`** — DISMISSED durch DP1 (`in_session`-Flag existiert nicht mehr).

#### Deferred (pre-existing / cosmetic)

- [x] [Review][Defer] **60-char Preview cuttet Grapheme-Cluster** [notification.rs:64-67] — char-basiert ist Spec-konform (Spec L45: "Char-basiert (kein UTF-16-Slice-Bug analog 9.3-F29)"). Cosmetic-Risk für Emoji/ZWJ am Boundary. Defer für `unicode-segmentation`-Upgrade später wenn Preview-Quality-Issue auftritt.
- [x] [Review][Defer] **`show()` blockiert Broadcast-Loop synchron** [notification.rs:88-99] — kein `spawn_blocking`. Unter normaler Toast-Frequenz <1ms-Latenz unkritisch; nur Risk wenn Win-Toast-COM-Call hängt. Defer auf Phase-3+-Hardening wenn beobachtbar.
- [x] [Review][Defer] **Keine Unit-Tests für `in_session`-Lifecycle** [notification.rs] — Doc-Comment verspricht `MockRuntime`-Test-Pfad, aber kein `#[cfg(test)] mod tests`. Pattern-konsistent mit `bridge.rs`/`tray.rs` (minimal-test). Defer als Follow-Up-Story wenn AC-2-Architektur (M1) entschieden ist — Tests müssen ohnehin angepasst werden.
- [x] [Review][Defer] **App-Exit-Shutdown emittiert kein `RecordingCompleted` → in_session-Leak latent** [klarvo-shell-orchestrator/src/session.rs:361-378] — Pre-existing aus Story 3.x; aktuell harmlos weil M1-Pfad dead. Coupled to M1-Resolution. Defer.

#### Dismissed (Noise / Spec-konform / Brand)

- "Klarvo"-Title hardcoded — Brand-Name, intentionale Ausnahme.
- ErrorEmitted body shows raw key on miss — Spec L60-61 explizit ("Falls Key nicht gefunden: error_key direkt anzeigen, kein Panic").
- Kein Coalescing rapider Delivered-Bursts — UX-Decision; Story-Scope = 1-Toast-pro-Diktat.
- `Arc<AtomicBool>` dead-weight im task-local-Scope — Nit, kein Bug.
- `char_count` walks string twice — Nit, perf vernachlässigbar.
