---
name: Story 9.5 — Log-Export-UI
epic: 9
story_number: "9.5"
status: done
dependencies:
  - "9-3-history-panel"
  - "6-3-panic-hook-export-stub"
---

# Story 9.5: Log-Export-UI

Status: done

## Story

Als täglicher Klarvo-User / Entwickler
möchte ich in den Settings einen "Export Debug Log"-Button, der ein Diagnose-Zip am Download-Pfad ablegt,
damit ich Logs ohne manuelle Dateisystem-Navigation teilen kann, wenn ich einen Bug melde.

## Kontext und Motivation

**Foundation (Story 6.3):** `klarvo-core::telemetry::export::export_debug_zip` existiert bereits als Fail-Soft-Stub — er returnt immer `AppError::Configuration` mit Key `error.telemetry.export.unimplemented`. Story 9.5 ersetzt den Stub durch eine echte Implementierung und verdrahtet ihn mit einem Tauri-Command + Settings-UI-Button.

**PRD FR40:** "full UI-triggered Zip-Generation (Debug-Export) is deferred to Phase 2." Dieser Story ist genau dieser Phase-2-Schritt.

**NFR5-Constraint:** Kein Audio, kein Transkriptions-Text im Export. Nur: Rolling-File-Logs + Sys-Info-Text. Die Config (`pipeline.toml`) enthält keine API-Keys (die sind im KeyStore) — sie kann optional hinzugefügt werden, aber ist kein Pflicht-Scope dieser Story.

**Scope-Grenze:** Kein File-Save-Dialog (Phase-2-B). Fixer Output-Pfad: `%USERPROFILE%\Downloads\klarvo-debug-{unix_timestamp}.zip`, Fallback `%TEMP%\klarvo-debug-{unix_timestamp}.zip`. Kein "Redacted Config" über sys-info hinaus in dieser Story.

## Acceptance Criteria

### AC-1: `AppErrorKind::ExportFailed` in `klarvo-core/src/error.rs`

**Given** `klarvo-core/src/error.rs` enthält `AppErrorKind` Enum,
**When** AC-1 committed ist,
**Then** ist ein neuer Variant eingefügt (nach `HotkeyConflict`):
```rust
/// Export of the debug-zip failed (I/O, zip-creation, etc.).
/// Typical retryable=false.
ExportFailed,
```
`#[non_exhaustive]` auf dem Enum ist weiterhin vorhanden. Keine anderen `error.rs`-Änderungen.

### AC-2: `zip = { version = "2", features = ["deflate"] }` in `klarvo-core/Cargo.toml`

**Given** `klarvo-core/Cargo.toml` hat keinen `zip`-Eintrag,
**When** AC-2 committed ist,
**Then**:
```toml
zip = { version = "2", features = ["deflate"] }
```
Unter `[dependencies]` eingefügt. Kein Workspace-dep — `zip` ist Core-spezifisch.

### AC-3: Reale `export_debug_zip`-Implementierung in `klarvo-core/src/telemetry/export.rs`

**Given** `klarvo-core/src/telemetry/export.rs` hat den Phase-1-Stub,
**When** AC-3 committed ist,
**Then** ist die Funktion **vollständig ersetzt** durch:

```rust
/// Erzeugt ein Debug-Export-Zip am gegebenen Pfad.
///
/// Enthält: sysinfo.txt + alle Dateien aus log_dir.
/// NFR5: KEINE Audio- oder Transkriptions-Daten.
pub fn export_debug_zip(log_dir: &Path, out_path: &Path) -> Result<(), AppError> {
    // ... (echte Implementierung — siehe Dev Notes für exakte zip-2.x-API)
}
```

**Zip-Inhalt:**
1. `sysinfo.txt` — einzeiliges Text-File mit:
   ```
   klarvo_version: {env!("CARGO_PKG_VERSION")}
   os: {std::env::consts::OS}
   arch: {std::env::consts::ARCH}
   exported_at: {SystemTime::now().duration_since(UNIX_EPOCH) in Sekunden}
   ```
2. `logs/` — alle Dateien aus `log_dir` (falls `log_dir.exists()`). Kein Rekurs in Sub-Dirs.
3. Falls `log_dir` nicht existiert: nur `sysinfo.txt` im Zip (kein Error).

**Error-Mapping:** Jeder IO- oder Zip-Fehler → `AppError { kind: AppErrorKind::ExportFailed, message: format!("{e}"), user_message: Some("error.telemetry.export.failed".into()), retryable: false }`.

**Signature-Break:** `_out_path: &Path` (stub) → `log_dir: &Path, out_path: &Path` (real). Alle Caller sind Shell-seitig (neu in AC-5) — kein bestehender Aufrufer außer dem Unit-Test im selben File.

**Test-Update:** Der bestehende Test `export_debug_zip_returns_unimplemented_error` wird **ersetzt** durch:
- `export_debug_zip_writes_sysinfo_txt` — erstellt Zip in `tempdir`, prüft dass `sysinfo.txt` im Zip vorhanden
- `export_debug_zip_empty_log_dir_ok` — `log_dir` zeigt auf nicht-existierendes Dir → `Ok(())`
- `export_debug_zip_creates_parent_dir_if_needed` — `out_path` unter nicht-existierendem Subdir → dir wird angelegt

**Altes i18n-Key-Entfernen:** Die Zeile `user_message: Some("error.telemetry.export.unimplemented".into())` wird **komplett entfernt** — dieser Key-Verweis darf nicht mehr im Code stehen (sonst failing lint).

### AC-4: i18n-Keys aktualisieren (Locale-Files + Lint)

**Given** beide Locale-Files enthalten `error.telemetry.export.unimplemented`,
**When** AC-4 committed ist,
**Then**:

**Entfernen** aus `shells/windows/locales/de.json` und `shells/windows/locales/en.json`:
```json
"error.telemetry.export.unimplemented": "..."
```

**Hinzufügen** zu beiden Locale-Files:
| Key | DE | EN |
|-----|----|----|
| `error.telemetry.export.failed` | `"Export fehlgeschlagen. Prüfen Sie den Logs-Ordner manuell."` | `"Export failed. Check the logs folder manually."` |
| `error.telemetry.export.in_progress` | `"Export läuft bereits. Bitte warten."` | `"Export already in progress. Please wait."` |

Die beiden neuen Keys müssen in **beiden** Locale-Files existieren (Symmetrie-Test in `i18n.rs::de_json_covers_same_key_set` schlägt sonst fehl).

`cargo xtask lint-events` → **Exit 0** nach AC-3 + AC-4 zusammen.

### AC-5: `ExportState` + Tauri-Command `export_debug_zip_cmd`

**Given** `shells/windows/src-tauri/src/commands/` enthält `history.rs`, `settings.rs` und `mod.rs`,
**When** AC-5 committed ist,
**Then** existiert `shells/windows/src-tauri/src/commands/telemetry.rs`:

```rust
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::State;
use klarvo_core::error::{AppError, AppErrorKind};
use klarvo_core::telemetry::export::export_debug_zip;

pub struct ExportState {
    pub log_dir: PathBuf,
    pub in_progress: Arc<AtomicBool>,
}

#[tauri::command]
#[specta::specta]
pub async fn export_debug_zip_cmd(
    state: State<'_, ExportState>,
) -> Result<String, AppError> {
    // Single-flight guard
    if state.in_progress.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
        return Err(AppError {
            kind: AppErrorKind::ExportFailed,
            message: "export already in progress".into(),
            user_message: Some("error.telemetry.export.in_progress".into()),
            retryable: false,
        });
    }

    let result = (|| {
        let out_path = resolve_export_path();
        export_debug_zip(&state.log_dir, &out_path)?;
        Ok(out_path.to_string_lossy().into_owned())
    })();

    state.in_progress.store(false, Ordering::SeqCst);
    result
}

fn resolve_export_path() -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = format!("klarvo-debug-{ts}.zip");
    std::env::var("USERPROFILE")
        .ok()
        .map(|h| PathBuf::from(h).join("Downloads").join(&name))
        .filter(|p| p.parent().map(|d| d.exists()).unwrap_or(false))
        .unwrap_or_else(|| std::env::temp_dir().join(name))
}
```

Und `shells/windows/src-tauri/src/commands/mod.rs` erhält:
```rust
pub mod telemetry;
```

### AC-6: Command in `specta_builder()` registrieren + Managed State in `main.rs`

**Given** `shells/windows/src-tauri/src/lib.rs` hat `specta_builder()`, `main.rs` hat `app.manage(...)`-Block,
**When** AC-6 committed ist,
**Then**:

**`lib.rs`** — `collect_commands![]` erhält Eintrag:
```rust
// Story 9.5: Debug-Export command
export_debug_zip_cmd,
```

**`lib.rs`** Imports erweitern:
```rust
use crate::commands::telemetry::export_debug_zip_cmd;
```

**`main.rs`** — `app.manage()`-Block erhält (nach dem bestehenden `app.manage(orch)`-Block):
```rust
// Story 9.5: ExportState — log_dir muss identisch mit dem oben verwendeten Pfad sein
debug_assert!(app.manage(klarvo_windows_shell::commands::telemetry::ExportState {
    log_dir: log_dir.clone(),
    in_progress: Arc::new(std::sync::atomic::AtomicBool::new(false)),
}));
```

**Wichtig:** `log_dir` ist in `main.rs` bereits vor dem `tauri::Builder` als `PathBuf` deklariert. Da der `setup`-Closure `move |app|` nutzt, muss `log_dir.clone()` **vor** dem Closure stehen oder `log_dir` wird per Clone in den Closure gegeben. Pattern: analog zur bestehenden Settings-DB-Path-Übergabe.

`cargo build --target x86_64-pc-windows-gnu -p klarvo-windows-shell` kompiliert ohne Errors.

### AC-7: Settings-Panel "Export Debug Log" Button in `index.html`

**Given** `SettingsPanel` in `shells/windows/src/index.html` hat einen `saving`-State und einen `handleSave`-Button,
**When** AC-7 committed ist,
**Then** hat `SettingsPanel` einen separaten `exporting`-State + Export-Button unterhalb der `.actions`-Zeile:

```javascript
const [exporting, setExporting] = useState(false);

const handleExport = useCallback(async () => {
  setExporting(true);
  setToast(null);
  try {
    const path = await invoke("export_debug_zip_cmd");
    setToast({ kind: "ok", msg: `Exported to: ${path}` });
  } catch (e) {
    setToast(errorToToast(e));
  } finally {
    setExporting(false);
  }
}, []);
```

Der Button ist **separat** vom Submit-Button (kein `type="submit"`):
```javascript
h("div", { className: "actions", style: { marginTop: 12 } },
  h("button", {
    type: "button",
    className: "btn-secondary",   // neue Klasse, siehe unten
    onClick: handleExport,
    disabled: exporting || saving || loading,
  },
    exporting
      ? h("span", { style: { display: "flex", alignItems: "center", gap: 6 } },
          h("div", { className: "spinner", style: { width: 14, height: 14 } }),
          "Exporting…"
        )
      : "Export Debug Log"
  )
)
```

**CSS** — `.btn-secondary` hinzufügen (analog zu `button`-Base-Style, aber ohne Primary-Akzent):
```css
.btn-secondary {
  background: #1e2535; color: #c0c8d8; border: 1px solid #2a3040;
  padding: 8px 16px; border-radius: 6px; font-size: 13px;
  cursor: pointer; transition: background 0.15s;
}
.btn-secondary:hover:not(:disabled) { background: #252d40; }
.btn-secondary:disabled { opacity: 0.5; cursor: default; }
```

Das Export-Toast (Success oder Error) nutzt denselben `toast`-State wie der Settings-Save-Toast — der Benutzer sieht also das zuletzt ausgeführte Feedback.

### AC-8: `cargo xtask lint-events` grün + Windows-Cross-Compile grün

**When** alle ACs committed sind,
**Then**:
- `cargo xtask lint-events` → Exit 0, **kein** `[locale-orphan]` Violation
- `cargo check --target x86_64-pc-windows-gnu -p klarvo-windows-shell` → Exit 0 (MinGW-w64 analog zu Story 9.1 Pattern)
- `cargo test -p klarvo-core` → Exit 0 (neue Export-Tests grün)

## Tasks / Subtasks

- [x] `AppErrorKind::ExportFailed` in `error.rs` (AC-1)
- [x] `zip = "2"` Dep in `klarvo-core/Cargo.toml` (AC-2)
- [x] Reale `export_debug_zip`-Implementierung + Tests (AC-3)
  - [x] Stub-Implementierung ersetzen, neue Signatur (`log_dir`, `out_path`)
  - [x] `sysinfo.txt` + Log-Files in Zip
  - [x] Error-Mapping zu `AppErrorKind::ExportFailed`
  - [x] Alte Test ersetzen + 3 neue Tests
- [x] i18n-Keys aktualisieren (AC-4)
  - [x] `error.telemetry.export.unimplemented` aus beiden Locale-Files entfernen
  - [x] `error.telemetry.export.failed` + `error.telemetry.export.in_progress` hinzufügen
- [x] `ExportState` + Tauri-Command `export_debug_zip_cmd` (AC-5)
  - [x] `commands/telemetry.rs` anlegen
  - [x] `commands/mod.rs` erweitern
- [x] Wiring in `lib.rs` + `main.rs` (AC-6)
  - [x] `collect_commands![]` erweitern
  - [x] `app.manage(ExportState { ... })` in setup-Closure
- [x] Settings-Panel-Button in `index.html` (AC-7)
  - [x] `exporting`-State + `handleExport`-Callback
  - [x] Button-Rendering mit Spinner + Disabled-State
  - [x] `.btn-secondary`-CSS-Klasse
- [x] Lint + Cross-Compile verifizieren (AC-8)
  - [x] `cargo xtask lint-events` → Exit 0
  - [x] `cargo check --target x86_64-pc-windows-gnu -p klarvo-windows-shell` → Exit 0
  - [x] `cargo test -p klarvo-core` → Exit 0

## Dev Notes

### `zip` 2.x API — Exakte Verwendung

`zip` 2.x hat eine von 1.x abweichende `FileOptions`-API:

```rust
use std::io::{Read, Write};
use zip::{ZipWriter, write::SimpleFileOptions, CompressionMethod};

let file = std::fs::File::create(out_path)
    .map_err(|e| make_export_err(format!("create file: {e}")))?;
let mut zip = ZipWriter::new(file);
let opts = SimpleFileOptions::default()
    .compression_method(CompressionMethod::Deflated);

// Eintrag starten und schreiben:
zip.start_file("sysinfo.txt", opts)
    .map_err(|e| make_export_err(format!("zip entry: {e}")))?;
write!(zip, "klarvo_version: {}\n...", env!("CARGO_PKG_VERSION"))
    .map_err(|e| make_export_err(format!("zip write: {e}")))?;

// Log-Files:
zip.start_file(format!("logs/{filename}"), opts)
    .map_err(...)?;
zip.write_all(&buf).map_err(...)?;

zip.finish().map_err(|e| make_export_err(format!("zip finish: {e}")))?;
```

Helper:
```rust
fn make_export_err(msg: String) -> AppError {
    AppError {
        kind: AppErrorKind::ExportFailed,
        message: msg,
        user_message: Some("error.telemetry.export.failed".into()),
        retryable: false,
    }
}
```

**`zip::ZipWriter` implements `Write`** — nach `start_file()` gehen `write!()` / `write_all()` direkt in den aktuellen Eintrag.

### `log_dir` in `main.rs` — Closure-Capture-Pattern

`log_dir` ist in `main.rs` als `PathBuf` vor dem `tauri::Builder` deklariert. Der `setup`-Closure nutzt `move`. Da `log_dir` nach dem `init_tracing(&log_dir)`-Call nicht mehr per `&`-Referenz gebraucht wird, kann er in den Closure gemovt werden:

```rust
// NACH init_tracing (die &log_dir-Referenz ist weg):
let log_dir_for_export = log_dir.clone(); // oder direkt in den Closure movet

tauri::Builder::default()
    .setup(move |app| {
        // ...
        debug_assert!(app.manage(ExportState {
            log_dir: log_dir_for_export,
            in_progress: Arc::new(AtomicBool::new(false)),
        }));
```

### Single-Flight Guard

`compare_exchange(false, true, SeqCst, SeqCst)` — `Err` bedeutet, der Bool war schon `true` (anderes Export in progress). Das ist die sichere Variante gegenüber `swap`.

Nach dem `result`-Closure **immer** `store(false)` — auch bei Panic. Da `tokio::task`-Panics aber normalerweise via `JoinHandle`-Err propagieren, ist ein explizites RAII-Guard hier unnötig für Phase 2; der AtomicBool-Store im `finally`-äquivalenten Code reicht.

### Frontend-Patterns (index.html)

- `handleExport` nutzt `useCallback(async () => {...}, [])` — leere Dependency-Array OK, da kein Form-State gebraucht wird
- `type="button"` ist wichtig — verhindert, dass Enter im Form-Input den Export triggert
- `disabled: exporting || saving || loading` — gegenseitige Sperre mit Save-Operation
- Der Export-Button teilt den `toast`-State mit dem Save-Button — das ist gewollt (ein `toast`-State pro Panel, kein separater)
- Kein `mountedRef`-Check nötig: `setExporting(false)` und `setToast(...)` in `finally` sind synchron nach dem Await, Component wird nicht unmountet während Export läuft (Settings-Panel bleibt offen)

### NFR5-Compliance

**Kein** `session.rs`-, `pipeline.rs`-, audio-buffer-bezogener Code in `export_debug_zip`. Nur Dateien aus `log_dir` lesen (die Rolling-File-Log-Files — kein Audio-Content). Die `tracing`-Events im Rolling-Log enthalten per NFR5 ebenfalls keinen Audio/Text-Content (das ist eine Core-Invariante, keine Story-9.5-Aufgabe).

### `AppErrorKind::ExportFailed` vs. `AppErrorKind::Io`

Der Stub-Code hatte `TODO: introduce dedicated ExportFailed variant`. Story 9.5 führt ihn ein. Der bestehende `Io`-Variant bleibt für Clipboard/Paste-Fehler — `ExportFailed` ist semantisch klarer für die Export-UI (ermöglicht spezifischeres Frontend-Handling in Phase 3+).

### `cargo xtask lint-events` — Scan-Scope

Der Scanner liest nur `.rs`-Files in `klarvo-core/src/`, `shells/windows/src-tauri/src/` und klarvo-plugins. `index.html` wird NICHT gescannt — deshalb müssen alle Frontend-Only-Keys in `orphan-allowlist.txt`.

Neu in Story 9.5 verwendete Keys:
- `error.telemetry.export.failed` — Rust-Emit-Site: `klarvo-core/src/telemetry/export.rs` + `commands/telemetry.rs`
- `error.telemetry.export.in_progress` — Rust-Emit-Site: `shells/windows/src-tauri/src/commands/telemetry.rs`

Beide werden vom Scanner mechanisch erkannt (String-Literal in `Some("...")`-Context). Kein Allowlist-Eintrag nötig.

### Project Structure Notes

**Neue Dateien:**
- `shells/windows/src-tauri/src/commands/telemetry.rs`

**Geänderte Dateien:**
- `klarvo-core/src/error.rs` — `AppErrorKind::ExportFailed` hinzufügen
- `klarvo-core/src/telemetry/export.rs` — Stub durch Implementierung ersetzen
- `klarvo-core/Cargo.toml` — `zip` dep hinzufügen
- `shells/windows/src-tauri/src/commands/mod.rs` — `pub mod telemetry;`
- `shells/windows/src-tauri/src/lib.rs` — `export_debug_zip_cmd` in `collect_commands![]`
- `shells/windows/src-tauri/src/main.rs` — `ExportState` managed state
- `shells/windows/locales/de.json` — Key-Swap (remove unimplemented, add failed + in_progress)
- `shells/windows/locales/en.json` — Key-Swap (idem)
- `shells/windows/src/index.html` — Export-Button in SettingsPanel

### References

- `klarvo-core/src/telemetry/export.rs` — Stub-Vorlage + Phase-2-Impl-Hinweise im Datei-Header
- `klarvo-core/src/telemetry/logging.rs` — `init_tracing` → log_dir-Pattern
- `shells/windows/src-tauri/src/commands/history.rs` — Command-Pattern + `HistoryStoreState`-Vorlage für `ExportState`
- `shells/windows/src-tauri/src/lib.rs:51-75` — `specta_builder()` + `collect_commands![]`
- `shells/windows/src-tauri/src/main.rs:19-22` — log_dir-Deklaration (Zeilen 19-22)
- `shells/windows/src-tauri/src/main.rs:331-358` — Managed-State-Block (Muster für neuen `debug_assert!(app.manage(...))`)
- `shells/windows/src/index.html:277-351` — `SettingsPanel`-Render inkl. `handleSave`-Pattern + `.actions`-Div
- `shells/windows/src/index.html:98-103` — `errorToToast`-Helper
- `_bmad-output/implementation-artifacts/deferred-work.md §6.3-W2` — Single-Flight-Guard-Hintergrund
- `memory/project_no_remote_telemetry.md` — NFR5-Constraint-Begründung
- `prd.md FR40` — Scope-Definition (Phase-2-Zip-UI)
- `prd.md NFR5` — "Audio-Daten und Transkriptions-Text werden NICHT im Rolling-File-Log persistiert"
- `architecture.md §9 Observability` — `klarvo-core/src/telemetry/` als Primary Location

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (create-story 2026-05-03)

### Debug Log References

### Completion Notes List

- AC-1: `AppErrorKind::ExportFailed` nach `HotkeyConflict` in `klarvo-core/src/error.rs` eingefügt
- AC-2: `zip = { version = "2", features = ["deflate"] }` + `tempfile = "3"` (dev) in `klarvo-core/Cargo.toml`
- AC-3: Stub vollständig durch echte Implementierung ersetzt; neue Signatur `(log_dir, out_path)`; `make_export_err` helper; Parent-Dir-Anlegen; Log-Files werden per `read_dir` gesammelt (non-recursive); 3 neue Tests grün (sysinfo_txt, empty_log_dir_ok, creates_parent_dir)
- AC-4: `error.telemetry.export.unimplemented` aus beiden Locale-Files entfernt; `error.telemetry.export.failed` + `error.telemetry.export.in_progress` hinzugefügt
- AC-5: `commands/telemetry.rs` mit `ExportState` + `export_debug_zip_cmd` angelegt; `commands/mod.rs` erweitert
- AC-6: `export_debug_zip_cmd` in `collect_commands![]` (lib.rs) + `app.manage(ExportState { log_dir, in_progress })` in setup-Closure (main.rs); `log_dir_for_export = log_dir.clone()` vor dem Closure
- AC-7: `exporting`-State, `handleExport`-Callback, Export-Button mit Spinner, `.btn-secondary`-CSS in `index.html`
- AC-8: `cargo xtask lint-events` → Exit 0 (5 events); `cargo check --target x86_64-pc-windows-gnu` → Exit 0; `cargo test -p klarvo-core` → 114 tests OK

### File List

- klarvo-core/src/error.rs
- klarvo-core/src/telemetry/export.rs
- klarvo-core/Cargo.toml
- shells/windows/locales/de.json
- shells/windows/locales/en.json
- shells/windows/src-tauri/src/commands/telemetry.rs (neu)
- shells/windows/src-tauri/src/commands/mod.rs
- shells/windows/src-tauri/src/lib.rs
- shells/windows/src-tauri/src/main.rs
- shells/windows/src/index.html

### Review Findings (2026-05-04)

Run via `bmad-code-review`-Skill mit drei parallelen Reviewern (Blind Hunter / Edge Case Hunter / Acceptance Auditor). Triage: 4 decision-needed, 10 patches, 8 deferred, 8 dismissed.

**Decisions (resolved 2026-05-04):**

- [x] [Review][Decision] D1=A — NFR5 Allowlist `*.log` (whitelist). Resolved → **P11**.
- [x] [Review][Decision] D2=A — Version als Param an `export_debug_zip` durchreichen (Signature-Break + Spec-Amend). Resolved → **P12**.
- [x] [Review][Decision] D3=B — Write-to-tempfile + atomic rename. Resolved → **P13**.
- [x] [Review][Decision] D4=B — `retryable=false` bleibt bis Frontend-Retry-UX-Story. Accepted as-is, kein Patch.

**Patches (alle applied 2026-05-04):**

- [x] [Review][Patch] P1 (BLOCKING) — `bindings/index.ts` regeneriert via `cargo xtask generate-bindings`; `bindings-drift`-Gate green. [`shells/windows/src/bindings/index.ts:42`]
- [x] [Review][Patch] P2 (BLOCKING) — RAII-`InProgressGuard(Drop)` in `commands/telemetry.rs`; Single-Flight-Reset auch bei Panic im Body. [`shells/windows/src-tauri/src/commands/telemetry.rs:14-21`]
- [x] [Review][Patch] P3 (BLOCKING) — `zip = { version = "2", default-features = false, features = ["deflate"] }` — aes/bzip2/lzma/xz/zstd raus. [`klarvo-core/Cargo.toml:31`]
- [x] [Review][Patch] P4 (BLOCKING) — Save-Button mutual exclusion: `disabled: saving || exporting` + `onSubmit`-Guard `if (saving || loading || exporting) return;`. [`shells/windows/src/index.html:346,358`]
- [x] [Review][Patch] P5 — `tokio::task::spawn_blocking` umschließt `export_debug_zip` im Tauri-Command; Runtime-Thread frei. [`shells/windows/src-tauri/src/commands/telemetry.rs:43-55`]
- [x] [Review][Patch] P6 — i18n-Keys `settings.export.{button,in_progress,success}` in en.json/de.json + `orphan-allowlist.txt`-Einträge. JS bleibt hardcoded-EN-konsistent mit `settings.tab.label`-Pattern (Phase-2-B Vite-Migration löst Resolver auf). [`shells/windows/locales/{en,de}.json`, `xtask/orphan-allowlist.txt`]
- [x] [Review][Patch] P7 — `std::io::copy(&mut file, &mut zip)` ersetzt `std::fs::read` — bounded memory. [`klarvo-core/src/telemetry/export.rs:96-101`]
- [x] [Review][Patch] P8 — `std::fs::File::open(&path)` mit `Err → continue`-Branch; concurrent-Rotation überspringt Single-File statt Abort. [`klarvo-core/src/telemetry/export.rs:88-91`]
- [x] [Review][Patch] P9 — Test umbenannt zu `export_debug_zip_nonexistent_log_dir_ok` + neuer Test `export_debug_zip_only_packs_log_extension` (NFR5-Allowlist) + `export_debug_zip_no_partial_on_failure`. [`klarvo-core/src/telemetry/export.rs:tests`]
- [x] [Review][Patch] P10 — DISMISSED nach Verification: Project-Pattern hat `tempfile = "3"` direct in 3 Cargo.tomls (`src-tauri`, `klarvo-bridge-jni`, `klarvo-core`); Story 9.5 folgt der Konvention. Blind-Hunter-Annahme stimmt nicht.
- [x] [Review][Patch] P11 (D1=A) — NFR5-Allowlist `path.extension() == Some("log")` + Doc-Comment-Update + Test `export_debug_zip_only_packs_log_extension` (assertet `.wav`/`.db` nicht im Zip). [`klarvo-core/src/telemetry/export.rs:81-83`]
- [x] [Review][Patch] P12 (D2=A) — `export_debug_zip(log_dir, out_path, app_version: &str)` Signature-Break; Caller im Shell übergibt `env!("CARGO_PKG_VERSION")` aus `klarvo-windows-shell`. AC-3-Amendment: sysinfo-`klarvo_version` zeigt User-sichtbare App-Version. [`klarvo-core/src/telemetry/export.rs:35-39,63`, `shells/windows/src-tauri/src/commands/telemetry.rs:47`]
- [x] [Review][Patch] P13 (D3=B) — `tempfile::NamedTempFile::new_in(parent)` + `persist(out_path)`; jeder `?`-Bail räumt das Tempfile auf, kein halb-geschriebenes Zip in Downloads. Test `export_debug_zip_no_partial_on_failure` deckt den Persist-Fail-Path. [`klarvo-core/src/telemetry/export.rs:50-52,116-118`]

**Deferred (siehe `_bmad-output/implementation-artifacts/deferred-work.md`):**

- [x] [Review][Defer] DF1 — Error-Path-Tests für `make_export_err` fehlen [`klarvo-core/src/telemetry/export.rs:tests`] — alle 3 Tests sind happy-path; failure-injection-Helpers brauchen separate Story.
- [x] [Review][Defer] DF2 — Component-unmount-Race in `handleExport` [`shells/windows/src/index.html:304-315`] — SettingsPanel unmountet heute nicht während Export; theoretisch bis Settings-Tab Route wird.
- [x] [Review][Defer] DF3 — Subdirs in `log_dir` werden silently übersprungen [`klarvo-core/src/telemetry/export.rs:63-65`] — Spec mandatet non-recursive; relevant erst wenn rolling-appender Subdir-Layout produziert.
- [x] [Review][Defer] DF4 — `USERPROFILE` → Downloads-ist-eine-Datei-statt-Dir [`shells/windows/src-tauri/src/commands/telemetry.rs:46`] — extreme corner; `temp_dir`-Fallback würde im selben Szenario auch nicht greifen.
- [x] [Review][Defer] DF5 — `debug_assert!(app.manage(...))` droppt Result in Release [`shells/windows/src-tauri/src/main.rs:344-358`] — projektweites Pre-Existing-Pattern (7+ Sites); separater Refactor.
- [x] [Review][Defer] DF6 — `exported_at: 0` Fallback versteckt Clock-Errors [`klarvo-core/src/telemetry/export.rs:44`] — minor; Bug-Report mit `exported_at:0` als Diagnose-Signal akzeptabel.
- [x] [Review][Defer] DF7 — AC-Coverage für `tempfile`-Dev-Dep fehlt [`spec AC-2`] — Spec-Amendment kosmetisch; im Closure-Commit-Body erwähnen reicht.
- [x] [Review][Defer] DF8 — Spec-Prosa "einzeiliges Text-File" widerspricht 4-Zeilen-Format [`spec L72`] — Spec-Cleanup, code stimmt.

**Dismissed:** B1 (out_path nicht user-controlled), B2 (AtomicBool-unused — false-positive Blind-only), B3 (telemetry.rs body invisible — diff-only-Artefakt; File ist im WT untracked, Reviewer mit Repo-Access hat ihn verifiziert), B6 (zip-slip via `file_name()` — `Path::file_name()` strippt Separators per Definition), B13 (in_progress-Key dead — false-positive; Key wird in `commands/telemetry.rs:22` emittiert), B20 (sprint-status self-flip — dieser Review IST der Signoff), E13 (out_path bare-filename — bereits durch `!parent.as_os_str().is_empty()` geguardet), E14 (env-divergence init_tracing↔clone — theoretisch).


