---
name: Story 9.4 — Toast Notifications
epic: 9
story_number: "9.4"
status: ready-for-dev
dependencies: []
---

# Story 9.4: Toast Notifications

Status: review

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

### AC-1: Erfolgs-Toast bei `RecordingDelivered`

**Given** `Event::RecordingDelivered { ts_ms, text }` auf dem EventBus geemitted wird,
**When** `NotificationService` diesen Event empfängt,
**Then**:

1. Ein nativer Windows-Toast wird angezeigt mit:
   - **Title:** `"Klarvo"`
   - **Body:** `"{label}: {preview}"` wobei:
     - `{label}` = i18n-Lookup von `notification.dictation.delivered` aus der aktuellen `SharedI18nTable`
     - `{preview}` = `text.chars().take(60).collect::<String>()` — Char-basiert (kein UTF-16-Slice-Bug analog 9.3-F29), ohne Truncation-Ellipsis wenn ≤60 Chars, mit `…` wenn >60 Chars
   - Beispiel EN: `"Dictation pasted: Hello, I'm writing to you reg…"`
   - Beispiel DE: `"Diktat eingefügt: Hallo, ich schreibe Ihnen bezi…"`

2. Falls `tauri_plugin_notification` fehlschlägt (Permission denied, OS-Fehler): `tracing::warn!` + kein Panic (fail-soft).

3. Der `in_session`-Flag wird nach `RecordingCompleted` zurückgesetzt (AC-2).

### AC-2: Fehler-Toast bei `ErrorEmitted` während Recording-Session

**Given** `Event::RecordingStarted` den `in_session`-Flag auf `true` gesetzt hat,
**When** `Event::ErrorEmitted { error_key, .. }` empfangen wird UND `in_session == true`,
**Then**:

1. Ein nativer Windows-Toast wird angezeigt mit:
   - **Title:** `"Klarvo"`
   - **Body:** i18n-Lookup von `error_key` aus der `SharedI18nTable`. Falls Key nicht gefunden: `error_key` direkt anzeigen (Key als Fallback, kein Panic).

2. `in_session`-Flag wird durch `ErrorEmitted` NICHT zurückgesetzt — kann mehrfach feuern.

3. `RecordingCompleted` setzt `in_session = false`.

**Given** `in_session == false`,
**When** `ErrorEmitted` empfangen wird (z.B. Boot-Error wie `error.config.parse_failed`),
**Then** wird KEIN OS-Toast angezeigt (verhindert Boot-Error-Spam).

**in_session-Lifecycle:**
```
Initial:             false
RecordingStarted  →  true
RecordingCompleted → false
ErrorEmitted       → unverändert (nur bedingt toast, kein state-change)
RecordingDelivered → unverändert (RecordingCompleted folgt danach)
```

### AC-3: i18n-Key `notification.dictation.delivered`

**Given** der neue i18n-Key noch nicht in `en.json`/`de.json` existiert,
**When** Story 9.4 committed ist,
**Then**:

**en.json addition:**
```json
"notification.dictation.delivered": "Dictation pasted"
```

**de.json addition:**
```json
"notification.dictation.delivered": "Diktat eingefügt"
```

**Note:** `cargo xtask required-keys-drift` muss nach diesem Add grün bleiben. Der neue Key ist NICHT im REQUIRED_KEYS-Set (nur Error-Keys und structural-Keys sind dort listed) — kein REQUIRED_KEYS-Update nötig.

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

- [x] **T2: `notification.rs` Module erstellen** (AC-1, AC-2)
  - [x] `shells/windows/src-tauri/src/notification.rs` angelegt (NotificationService<R> mit in_session AtomicBool)
  - [x] `pub mod notification;` in `shells/windows/src-tauri/src/lib.rs` hinzugefügt (nach `pub mod bridge;`)

- [x] **T3: Plugin registrieren + EventBus-Subscriber wiring in `main.rs`** (AC-1, AC-2)
  - [x] `.plugin(tauri_plugin_notification::init())` als erstes Plugin auf `tauri::Builder::default()`
  - [x] `let event_bus_rx_notification = event_bus.subscribe();` (Step 12, neben tray + mirror)
  - [x] `notification_i18n = Arc::clone(&i18n_table)` VOR `app.manage(i18n_table)` eingefügt
  - [x] Step-12c nach EventMirror: `NotificationService::new(...).start(event_bus_rx_notification)`

- [x] **T4: i18n-Keys** (AC-3)
  - [x] `shells/windows/locales/en.json`: `"notification.dictation.delivered": "Dictation pasted"`
  - [x] `shells/windows/locales/de.json`: `"notification.dictation.delivered": "Diktat eingefügt"`

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
