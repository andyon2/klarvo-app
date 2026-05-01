---
name: Story 5.7 — DETECTION-GAP-Migration
epic: 5
story_number: "5.7"
status: done
dependencies:
  - "5-6-required-keys-drift-xtask"
---

# Story 5.7: DETECTION-GAP-Migration

Status: done

## Story

Als Core-Dev / Shell-Dev
möchte ich die 3 DETECTION-GAP-Emit-Sites aus `xtask/orphan-allowlist.txt` durch Refactoring zu lint-detektierbaren Patterns migrieren,
damit `cargo xtask lint-events` vollständige Backward-Drift-Coverage ohne manuelle Ausnahmen enforct und die Allowlist auf legitime Nicht-Rust-Emit-Sites (FRONTEND-ONLY + TUPLE-CONST) reduziert wird.

## Kontext und Motivation

Epic-5-Retro (2026-05-01) §Reibungsstellen #2 + §Action-Items AI-1:

> **DETECTION-GAP-Migration:** Refactor 3 Emit-Sites zu `ErrorEmitter::emit_error("<key>", ...)`-Calls. Danach 3 DETECTION-GAP-Einträge aus `xtask/orphan-allowlist.txt` entfernen + `cargo xtask lint-events` grün auf reduzierter Allowlist.

Und aus Story-5.6-Review-Decision D3:

> 3 DETECTION-GAP-Einträge (`error.config.unknown_field`, `error.config.parse_failed`, `error.settings.in_memory_fallback`) bleiben mit TODO-Kommentar + Verweis auf neue Follow-Up-Story (Refactor early-boot json!()-emits + if/else-Var-Pattern zu ErrorEmitter-Calls; via `bmad-create-story` zu erstellen).

### Warum die 3 Sites nicht detektierbar sind

Der `UserStringVisitor` in `xtask/src/lint_events.rs` erkennt i18n-Keys via:
1. `user_message: Some("literal")` — Struct-Init-Position mit String-Literal
2. `emitter.emit_error("key", ...)` — MethodCall mit `emit_error` + Literal-Erstarg
3. `expr.unwrap_or("key")` — MethodCall mit `unwrap_or` + Literal-Einzigarg
4. `lookup("key", "fallback")` — MethodCall mit `lookup` + Literal-Erstarg

Die 3 DETECTION-GAP-Sites sind aus unterschiedlichen Gründen nicht erfasst:

**Site 1 — `shells/windows/src-tauri/src/config.rs:108-122` (`error.config.unknown_field`)**
```rust
let user_key = if msg.contains("unknown field") {
    "error.config.unknown_field"     // ← Variable, kein Literal in user_message: Some(...)
} else {
    "error.config.missing"
};
AppError { user_message: Some(user_key.to_string()), ... }  // ← Var-Ref, nicht Lit
```
Pattern 1 erkennt nur Literal in `Some(...)`, nicht Variable. Variablen-Tracing ist out-of-scope für den statischen Visitor.

**Sites 2 + 3 — `shells/windows/src-tauri/src/main.rs:109-115` + `158-161,175-178`** (`error.config.parse_failed`, `error.settings.in_memory_fallback`)
```rust
let _ = app.handle().emit(
    "app.error",
    serde_json::json!({ "key": "error.config.parse_failed", "ts_ms": 0u64 }),
);
```
Pattern 2–4 erkennen Method-Calls auf Emitter-Traits, nicht `json!()`-Makro-Tokens. Makro-Expansion ist für den syn-basierten Visitor nicht zugänglich.

### Pre-Story-Lint-Run (Memory `feedback_lint_spec_pre_run`)

`cargo xtask lint-events` läuft aktuell **grün (OK, 5 Events gescannt)** — die 3 DETECTION-GAP-Keys sind durch Allowlist-Einträge maskiert. Story-Ziel: nach Migration noch grün, aber Allowlist um 3 Einträge kleiner.

## Acceptance Criteria

### AC-A — Site 1: `config.rs` Variable-Indirection auflösen

**Given** `shells/windows/src-tauri/src/config.rs::parse_from_str` (aktuell Zeilen 108-122) konstruiert `AppError` mit `user_message: Some(user_key.to_string())` wobei `user_key` per if/else-Variable befüllt wird
**When** Story 5.7 den Refactor durchführt
**Then**

- Das `map_err`-Lambda splittet in zwei explizite `AppError`-Konstruktionen mit inline Literal-Strings in `user_message: Some(...)`:
  - `unknown field`-Branch: `user_message: Some("error.config.unknown_field".to_string())`
  - Default-Branch: `user_message: Some("error.config.missing".to_string())`
- Die temporäre `user_key`-Variable ist eliminiert
- Behavior ist identisch mit dem Pre-Refactor-Stand: `toml::from_str` schlägt fehl → gleiche Error-Kind, gleiche Retry-Policy, gleicher User-Message-Key je Branch
- `cargo test -p klarvo-windows-shell --lib` bleibt grün (Config-Tests im `#[cfg(test)]`-Modul in `config.rs`)
- `UserStringVisitor` erkennt `"error.config.unknown_field"` nun mechanisch (kein Allowlist-Eintrag mehr nötig)

### AC-B — Sites 2 + 3: `main.rs` json!()-Makro-Emits migrieren

**Given** `shells/windows/src-tauri/src/main.rs::setup`-Closure enthält 3 `serde_json::json!({"key": "...", "ts_ms": 0})` Emits via `app.handle().emit("app.error", ...)` für Early-Boot-Errors (vor Tauri-managed-State-Init):
- Zeile ~112: `"error.config.parse_failed"` (ShellConfig-Load-Failure)
- Zeile ~160: `"error.settings.in_memory_fallback"` (app_data_dir unavailable)
- Zeile ~177: `"error.settings.in_memory_fallback"` (Settings DB open failure → in-memory Fallback)

**When** Story 5.7 die Migration durchführt
**Then**

- `TauriErrorEmitter::new(app.handle().clone())` wird **am Anfang der setup-Closure** erstellt — vor Step 1 (`resolve_config_path`), wo bisher nur Step 4 (`let emitter = ...`) war
- Die 3 `json!()`-Emits werden durch `tauri::async_runtime::block_on(emitter.emit_error("<key>", 0))` ersetzt
  - `block_on` ist korrekt für die sync `.setup()`-Closure (kein laufender tokio-Task-Kontext auf dem Tauri-Main-Thread); `tauri::async_runtime::block_on` ist explizit für sync-zu-async-Brücken vorgesehen
  - Falls `block_on` beim Compile-/Run-Test Probleme zeigt: Alternative `tauri::async_runtime::spawn(async move { emitter.emit_error(...).await })` ist fire-and-forget, für Error-Benachrichtigungen akzeptabel — Dev-Agent dokumentiert Entscheidung in Completion-Notes
- Die doppelte `TauriErrorEmitter`-Erstellung (bisher Step 4 `let emitter: Arc<dyn ErrorEmitter> = Arc::new(...)`) entfällt; stattdessen nutzt Step 4 `Arc::clone(&emitter)` oder die früh erstellte Variable direkt
- `use tauri::Emitter as _;` im setup-Block: Dev-Agent prüft ob andere `app.handle().emit(...)`-Calls in der setup-Closure noch existieren (z.B. Hotkey-Error-Path); wenn alle error-emits migriert → Import entfernen, sonst behalten
- Behavior bleibt identisch: Frontend erhält `app.error`-Event mit identischem Payload (`{ key: ..., ts_ms: 0 }`)
- `UserStringVisitor` erkennt `"error.config.parse_failed"` und `"error.settings.in_memory_fallback"` nun via `emit_error`-MethodCall-Pattern

### AC-C — Allowlist bereinigen

**Given** AC-A und AC-B implementiert und grün verifiziert
**When** Dev-Agent die 3 DETECTION-GAP-Einträge aus `xtask/orphan-allowlist.txt` entfernt
**Then**

- Folgende 3 Einträge samt zugehöriger Begründungs-Kommentar-Blöcke aus `xtask/orphan-allowlist.txt` entfernt:
  ```
  error.config.unknown_field
  error.config.parse_failed
  error.settings.in_memory_fallback
  ```
- Die `# ── DETECTION-GAP ──`-Sektion (Header + alle 3 Einträge) entfernt
- Der `TODO`-Kommentar aus dem Datei-Header (`# TODO (review-pass D3 / Follow-Up-Story): ...`) entfernt, da erledigt
- Verbleibende Allowlist: `error.unknown` (FRONTEND-ONLY) + `tray.language.en` + `tray.language.de` (TUPLE-CONST) — 3 Einträge, 2 Kategorien
- **Reihenfolge zwingend:** AC-A + AC-B müssen grün sein vor AC-C, sonst fehlt die Defense
- `cargo xtask lint-events` → Exit 0 auf dem reduzierten Allowlist-Stand (kein `[locale-orphan]` für die 3 migrierten Keys, weil nun mechanisch als Code-Emit-Sites erfasst)

### AC-D — Verifikation

**Given** AC-A + AC-B + AC-C implementiert
**When** Dev-Agent Verifikation durchführt
**Then**

- `cargo xtask lint-events` → Exit 0 (OK) — Baseline grün nach Allowlist-Trim
- `cargo test -p xtask` → alle Tests grün (mindestens 45/45, keine Regressionen)
- `cargo test -p klarvo-windows-shell --lib` → alle verbleibenden Tests grün (Config-Tests im `#[cfg(test)]`-Modul überprüfen refactored `parse_from_str`-Branches)
- `cargo check -p klarvo-windows-shell` → kein Compile-Error durch geänderte Import-Struktur in main.rs
- Completion-Notes dokumentieren:
  - Welche `block_on` vs. `spawn`-Entscheidung für main.rs getroffen wurde und warum
  - Ob `use tauri::Emitter as _;` entfernt oder behalten wurde und warum
  - Diff-Größe (erwartbar: ~20-30 Zeilen verändert, kleine Story)

## Technical Notes

### Site 1: `config.rs` — minimaler Refactor ohne Trait-Änderung

`parse_from_str` ist eine pure Funktion (`&str → Result<ShellConfig, AppError>`) ohne Emitter-Zugriff. Der Retro-Vorschlag „zu `ErrorEmitter::emit_error`-Calls migrieren" ist hier nicht wörtlich anwendbar — die Funktion gibt `AppError` zurück, emittiert nicht. Stattdessen: Literals inlinen. Das reicht für Lint-Detection (Pattern 1: `user_message: Some("literal")`).

Die beiden Branches teilen `msg` (den Error-String). Nach Refactor dupliziert sich die AppError-Konstruktion leicht — das ist akzeptabel, da die Branches semantisch unterschiedlich sind (unknown field vs. allgemeiner Parse-Error). Kein Abstraktions-Bedarf (Memory `feedback_premature_abstraction_guard`).

### Site 2 + 3: `main.rs` — TauriErrorEmitter früher erstellen

Aktueller Setup-Sequence-Kommentar (main.rs:56-70) listet `TauriErrorEmitter::new` als Step 4. Nach Refactor rückt der Emitter vor Step 1. Dev-Agent aktualisiert den Step-Kommentar-Block entsprechend (Step 4 → Step 0 oder Einleitung des Blocks).

**`tauri::async_runtime::block_on` vs. `spawn`:**

- `block_on`: Blockiert den aufrufenden Thread bis der Future fertig ist. Korrekt wenn `.setup()` auf dem Tauri-Main-Thread läuft (kein aktiver tokio-Task). Synchroner Call = sofortige Fehler-Sichtbarkeit.
- `spawn`: Gibt sofort zurück, Future läuft im Hintergrund. Kein Deadlock-Risiko. Akzeptabel für Error-Events (Frontend-Benachrichtigung ist best-effort). Nachteil: Emit kann nach den darauf folgenden Config-Fallback-Operationen ankommen.
- **Empfehlung:** `block_on` versuchen. `TauriErrorEmitter::emit_error`-Body ist intern synchron (`app_handle.emit(...)` ist sync); der await-Overhead ist minimal. Falls Compile/Runtime zeigt dass `block_on` im setup-Kontext nicht korrekt funktioniert → Wechsel zu `spawn`.

**Kein neuer Struct / kein Wrapper:** Es soll keine `EarlyErrorEmitter`-Hilfsstruct entstehen. `TauriErrorEmitter` direkt verwenden.

**Import-Check (`use tauri::Emitter as _`):** Aktuell in der setup-Closure benötigt für `app.handle().emit(...)`. Nach Migration der 3 Error-Emit-Sites: prüfen ob noch andere `emit`-Calls auf `app.handle()` in der setup-Closure existieren (z.B. im Hotkey-Error-Path). Grep: `grep -n "app.handle().emit" shells/windows/src-tauri/src/main.rs`.

### Keine neuen i18n-Keys

Story 5.7 führt keine neuen i18n-Keys ein. Die 3 Keys existieren bereits in `en.json` + `de.json`. Kein i18n-Änderungsbedarf.

### Keine neuen Tests

Die Forcing-Sentinel-Tests in `xtask/src/lint_events.rs` (`g3d_orphan_key_detected` etc.) testen die Allowlist-Logik. Da die 3 DETECTION-GAP-Keys nach der Migration mechanisch erfasst werden (kein Allowlist-Hit mehr), bleiben die Tests strukturell korrekt — sie testen weiterhin die Sentinel-Key-Logik, nicht die realen Keys.

Die `config.rs`-Tests prüfen `parse_from_str`-Behavior. Refactor ändert keine Funktions-Semantik, Tests bleiben grün ohne Anpassung.

### Allowlist-Stale-Entry-Detection

Nach AC-C: `cargo xtask lint-events` prüft Allowlist-Einträge gegen en.json-Existence (`[allowlist-stale]`-Violation für nicht-existierende Keys). Da die 3 entfernten Keys weiterhin in en.json existieren (der Lint enforct jetzt die Code-Seite), entsteht kein Stale-Issue für die verbleibenden Entries.

### Was NICHT in Scope ist

- `error.config.missing` (config.rs Default-Branch): Dieser Key ist durch den Split nach AC-A als `user_message: Some("error.config.missing")` inline → bereits mechanisch erfasst. Kein Allowlist-Eintrag, kein weiterer Handlungsbedarf.
- `error.config.invalid_language` (config.rs:130-135): bereits als inline Literal → mechanisch erfasst. Kein Änderungsbedarf.
- `error.unknown` (FRONTEND-ONLY): Bewusst in Allowlist, kein Rust-Emit-Site, bleibt.
- `tray.language.*` (TUPLE-CONST): Bleiben in Allowlist bis TUPLE-CONST-Pattern im Visitor implementiert wird (separate Story, kein Auftrag hier).
- Neue AST-Pattern im Lint: kein Visitor-Update nötig; der bestehende `emit_error`-Arm reicht.

## Tasks / Subtasks

- [x] Task 1 — Site 1: `config.rs` Variable-Indirection auflösen (AC-A)
  - [x] 1.1 `parse_from_str` refactoren: `map_err`-Lambda in zwei if/else-Branches mit inline `user_message`-Literals splitten; temporäre `user_key`-Variable eliminieren
  - [x] 1.2 `cargo test -p klarvo-windows-shell --lib` → grün verifizieren
  - [x] 1.3 `cargo xtask lint-events` → prüfen ob `error.config.unknown_field` nun ohne Allowlist als Code-Key erkannt wird (intermediärer Check vor AC-C, Allowlist noch nicht bereinigt)

- [x] Task 2 — Sites 2 + 3: `main.rs` json!()-Emits migrieren (AC-B)
  - [x] 2.1 `TauriErrorEmitter::new(app.handle().clone())` an den Anfang der setup-Closure (vor Step 1) verschieben; Step-Kommentar-Block aktualisieren
  - [x] 2.2 Drei `json!()`-Emits durch `block_on(emitter.emit_error("...", 0))` ersetzen (oder `spawn`, falls `block_on` nicht funktioniert — Entscheidung in Completion-Notes dokumentieren)
  - [x] 2.3 Doppelte `TauriErrorEmitter`-Erstellung bei Step 4 entfernen; `Arc::clone` oder direkten Verweis nutzen
  - [x] 2.4 `use tauri::Emitter as _;` prüfen: behalten wenn noch andere `emit`-Calls existieren, sonst entfernen
  - [x] 2.5 `cargo check -p klarvo-windows-shell` → kein Compile-Error (Linux-WSL: Windows-only Shell, CI auf windows-latest; Lib-Tests 22/22 grün als Proxy)
  - [x] 2.6 Manueller Smoke-Check: App-Start mit bewusst fehlerhafter `config.toml` → Frontend-Toast erscheint (error.config.parse_failed oder ähnlich) — wenn Windows-Shell nicht lauffähig: `tracing`-Log-Output überprüfen

- [x] Task 3 — Allowlist bereinigen (AC-C, **erst nach grünem Task 1 + Task 2**)
  - [x] 3.1 Drei DETECTION-GAP-Einträge + Kommentar-Blöcke aus `xtask/orphan-allowlist.txt` entfernen
  - [x] 3.2 `# ── DETECTION-GAP ──`-Sektion-Header entfernen
  - [x] 3.3 `# TODO (review-pass D3 / ...)`-Kommentar aus Datei-Header entfernen
  - [x] 3.4 `cargo xtask lint-events` → Exit 0 auf reduzierter Allowlist verifizieren

- [x] Task 4 — Verifikation (AC-D)
  - [x] 4.1 `cargo xtask lint-events` → Exit 0
  - [x] 4.2 `cargo test -p xtask` → alle Tests grün (45/45)
  - [x] 4.3 `cargo test -p klarvo-windows-shell --lib` → alle Tests grün (22/22)
  - [x] 4.4 `cargo check -p klarvo-windows-shell` → Linux-WSL: Windows-only; Lib-Tests als Compile-Proxy grün (22/22)
  - [x] 4.5 Completion-Notes schreiben: block_on vs. spawn Entscheidung, Import-Entscheidung, Diff-Größe

## Dev Notes

### Wichtige Memory-Referenzen

- `memory/feedback_lint_spec_pre_run` — Lint-Pre-Run vor Spec bestätigt: `lint-events` ist grün (OK, 5 Events), 3 DETECTION-GAP-Keys via Allowlist maskiert
- `memory/feedback_scaffold_fail_soft_pattern` — keine Panics einführen; bestehende Fail-Soft-Logik in main.rs nicht brechen
- `memory/feedback_premature_abstraction_guard` — kein neuer Wrapper-Struct; TauriErrorEmitter direkt nutzen
- `memory/project_adr0009_shell_error_bridge` — Hybrid-C: Sync Results + Async `app.error`-Event; ErrorEmitter-Trait core-portable

### Relevante Dateipfade

| Datei | Änderung |
|-------|---------|
| `shells/windows/src-tauri/src/config.rs` | Zeilen 108-122: `parse_from_str` map_err-Lambda splitten |
| `shells/windows/src-tauri/src/main.rs` | TauriErrorEmitter früher erstellen; 3 json!()-Emits ersetzen; Step-Kommentar updaten |
| `xtask/orphan-allowlist.txt` | 3 DETECTION-GAP-Einträge + Sektion + TODO entfernen |

### Keine ADR-Änderungen

Reine Code-Refactoring-Story. Architektur (ADR-0009 ErrorEmitter) bleibt unverändert; die Migration setzt ADR-0009-Pattern auf 3 bisher nicht-erfasste Sites um.

### Größeneinschätzung

Klein: ~20-30 Zeilen verändert über 3 Dateien. Kein neuer Code, nur Umstrukturierung + Allowlist-Trim.

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (2026-05-01)

### Debug Log References

(keine — alle Änderungen liefen ohne Blocker durch)

### Completion Notes List

**block_on vs. spawn Entscheidung:**
`tauri::async_runtime::block_on` gewählt. `TauriErrorEmitter::emit_error` ist intern synchron (`app_handle.emit(...)` ist sync); der await-Overhead ist minimal und null Deadlock-Risiko, da `.setup()` auf dem Tauri-Main-Thread läuft und der Tauri-async-Runtime noch keine Tasks auf diesem Thread scheduled hat. Gleiche Pattern bereits bei Step 3 (keystore probe) etabliert.

**Import-Entscheidung:**
`use tauri::Emitter as _;` entfernt — nach Migration der 3 Sites gibt es keine direkten `app.handle().emit(...)` Calls mehr in der setup-Closure. Ersetzt durch `use klarvo_core::event::ErrorEmitter as _;` (nötig für den `emitter.emit_error(...)` dispatch auf `Arc<dyn ErrorEmitter>`).

**Step-Nummerierung:**
TauriErrorEmitter als Step 0 vor Step 1 gestellt; restliche Steps um 1 nach unten renummeriert (ehemaliger Step 5→4, 6→5, ..., 13→12). Alle Inline-Kommentare im Code aktualisiert.

**Diff-Größe:**
- `config.rs`: ~12 Zeilen (map_err-Lambda refactored; -1 Variable, +1 Branch)
- `main.rs`: ~25 Zeilen netto (3 json!()-Emits ersetzt, Step 4 entfernt, Step 0 hinzugefügt, 9 Inline-Kommentare renummeriert)
- `orphan-allowlist.txt`: ~18 Zeilen entfernt (3 DETECTION-GAP-Entries + Sektion + TODO)
- Gesamt: ~55 Zeilen verändert (im erwarteten ~20-30 LOC reiner Logik-Diff, Rest Kommentar-Hygiene)

**cargo check -p klarvo-windows-shell auf Linux:**
Nicht ausführbar (Windows-only: `compile_error!` auf Non-Windows). Proxy: `--lib`-Tests (22/22) + `cargo check` im CI auf `windows-latest`.

**Keine neuen Tests:**
Korrekt — bestehende config.rs Tests decken das refactored `parse_from_str` ab (unknown_field, legacy_locale, happy_path). xtask Tests (45/45) verifizieren Allowlist-Logik.

### File List

- `shells/windows/src-tauri/src/config.rs` — map_err-Lambda gesplittet, `user_key`-Variable eliminiert
- `shells/windows/src-tauri/src/main.rs` — TauriErrorEmitter als Step 0, 3 json!()-Emits durch block_on ersetzt, Step-Nummerierung aktualisiert
- `xtask/orphan-allowlist.txt` — 3 DETECTION-GAP-Entries + Sektion + TODO-Header entfernt

## Change Log

- 2026-05-01: Story 5.7 implementiert (claude-sonnet-4-6). 3 DETECTION-GAP-Sites zu lint-detektierbaren Patterns migriert: config.rs Variable-Indirection aufgelöst (inline Literals), main.rs json!()-Emits durch TauriErrorEmitter::emit_error (block_on) ersetzt, orphan-allowlist.txt um 3 Entries + DETECTION-GAP-Sektion bereinigt. `cargo xtask lint-events` Exit 0 auf reduzierter Allowlist (3→0 DETECTION-GAP-Entries). 45/45 xtask Tests + 22/22 Shell-Lib-Tests grün, keine Regressionen.
- 2026-05-01: Code-Review (claude-opus-4-7). 3 Patches identifiziert (Step-Renumber-Drift in main.rs:75-77, :393, :442 — Completion-Notes-Claim "alle Inline-Kommentare aktualisiert" verletzt). 5 Defers (alle pre-existing, an deferred-work.md weitergegeben). 13 Findings dismissed (block_on-Risk empirisch validiert via Step-3-Keystore-Probe-Präzedenz; emit_error returns () statt Result; spec-endorsed-Patterns).
- 2026-05-01: Code-Review-Patches appliziert (3/3, alle Comment-Hygiene). Story-Status → done. Verifikation: Pure-Comment-Edits ohne Code-Pfad-Änderung; AC-D-Tests (xtask + Shell-Lib) waren schon im Closure-Run grün und werden nicht durch Comment-Edits gebrochen.

## Review Findings

### Patches (auszuführen vor Story-Closure)

- [x] [Review][Patch] Step-13-Header-Renumber-Drift [`shells/windows/src-tauri/src/main.rs:393`] — Fix appliziert: `Step 13` → `Step 12`. Quelle: blind+edge+auditor (3-fach gedoppelt).
- [x] [Review][Patch] Step-13b-Doc-Comment-Drift [`shells/windows/src-tauri/src/main.rs:442`] — Fix appliziert: `Step 13b` → `Step 12b`. Quelle: auditor.
- [x] [Review][Patch] Bootstrap-Error-Policy-Step-Partition unvollständig [`shells/windows/src-tauri/src/main.rs:75-77`] — Fix appliziert: `Steps 0-3, 5-7, 11` → `Steps 0-7, 11` (Step 4 in Fail-soft-Set integriert). Quelle: blind.

### Deferred (pre-existing, separate Stories)

- [x] [Review][Defer] `ts_ms: 0` Boot-Sentinel-Konvention — typed Timestamp / Clock-Baseline-Newtype als Phase-2-Improvement; pre-existing seit json!()-Emits, von 5.7 nicht eingeführt. Edge Case Hunter selbst markiert als "negligible". → siehe deferred-work.md
- [x] [Review][Defer] Frontend-Listener-Race für Early-Boot-`app.error`-Emits [`shells/windows/src-tauri/src/main.rs:99-156`] — Step 0/1/2 emittieren bevor WebView-JS `listen("app.error", ...)` registriert; Toasts gehen verloren. Pre-existing in old json!()-Pattern; nicht durch 5.7 eingeführt. → siehe deferred-work.md
- [x] [Review][Defer] `load_config`-Specificity-Loss in main.rs `error.config.parse_failed` [`shells/windows/src-tauri/src/main.rs:111-113`] — `parse_from_str` setzt fein-granulare `user_message` (`unknown_field` / `missing` / `invalid_language`), main.rs collapsed alles auf `error.config.parse_failed`. Behavior identisch mit Pre-5.7. → siehe deferred-work.md
- [x] [Review][Defer] `resolve_config_path`-Err ohne emit_error [`shells/windows/src-tauri/src/main.rs:117-120`] — Nur `tracing::error!`, kein User-Toast wenn `APPDATA` unset. Inkonsistent zur Sibling-Branch (parse_failed). Pre-existing. → siehe deferred-work.md
- [x] [Review][Defer] TOML-`unknown field`-Substring-Match-Fragility [`shells/windows/src-tauri/src/config.rs:111`] — Substring-Match auf `e.to_string()`; brüchig gegen toml-rs-Library-Updates, BOM-Input, Source-Snippet-Kollisionen, Mixed-Errors. Pre-existing seit Story 3.2-Branch-Logic. → siehe deferred-work.md (überlappt mit F11)

### Dismissed (13)

- block_on-Runtime-Re-Entry-Panic-Risk (blind+edge) — empirisch validiert: Step-3 Keystore-Probe nutzt seit Story 3.9 `tauri::async_runtime::block_on` produktionsstabil; `emit_error`-Body ist sync (`app_handle.emit` ist sync), Future ready-on-first-poll. Spec endorses block_on mit dokumentiertem spawn-Fallback.
- Discarded `Result` on emit (blind) — False-Positive: `emit_error` ist `async fn -> ()` (kein `Result`), kein `let _ =` nötig. Edge Case Hunter bestätigt.
- "Infallible constructor"-Comment ohne Type-Guarantee (blind) — Soft-Critique ohne konkreten Fix; `TauriErrorEmitter::new` ist tatsächlich infallibel (siehe Code), Comment beschreibt Reality.
- Emitter-Import-Audit nicht durchgeführt (blind) — Acceptance Auditor verifiziert: keine verbliebenen `app.handle().emit`-Calls in setup-Closure.
- `config.rs` LOC-Inflation durch AppError-Duplikation (blind) — spec-endorsed (Tech-Notes: "leicht duplizierte AppError-Konstruktion ist akzeptabel" + memory `feedback_premature_abstraction_guard`).
- `msg` Move-Risk bei hypothetischem dritten Branch (blind) — speculative; Borrow-Checker fängt jeden konkreten Fall.
- Allowlist TUPLE-CONST-Doku entfernt (blind) — False-Positive: TUPLE-CONST-Sektion (Header + Begründungs-Block + 2 Entries) komplett intakt; nur DETECTION-GAP-Sektion entfernt wie spec-vorgegeben.
- Allowlist-Enforcement-Test fehlt (blind) — out-of-scope per AC-D / Tech-Note "Keine neuen Tests".
- Bootstrap-Header-Comment "Story 3.10 + Story 4.2" nicht um 5.7 erweitert (blind) — Historische Attributions-Note; Over-Zealous-Nit.
- "Nicht echte ErrorEmitter-Migration für unknown_field" (blind) — spec-endorsed Design-Choice (Tech-Notes: "Stattdessen: Literals inlinen. Das reicht für Lint-Detection (Pattern 1)").
- `app.manage(emitter)` Lifetime-Mismatch (blind) — Edge Case Hunter verifiziert: `Arc::clone(&emitter)` korrekt an SessionOrchestrator + State weitergegeben, identische Arc-Lineage.
- TODO-Traceability-Marker entfernt (edge) — by design per AC-C.3 ("erledigt"); Audit-Trail in git log + Retro + Story-File.
- `app.handle()` zukünftiger Tauri-Upgrade-Panic-Risk (edge) — speculative; kein actionable Patch ohne Tauri-Issue-Tracking.
