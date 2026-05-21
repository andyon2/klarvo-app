---
story: 12.2
status: ready-for-dev
epic: 12
inputDocuments:
  - _bmad-output/implementation-artifacts/epic-12-1-production-pipeline-wire-up.md
  - memory/feedback_release_build_blind_spot.md
  - memory/feedback_windows_cross_compile_verify.md
  - shells/windows/src-tauri/src/main.rs (lines 14-25)
  - klarvo-core/src/telemetry/logging.rs
  - klarvo-core/src/keystore/os/windows.rs
---

# Story 12.2: Pipeline Smoke-Test Diagnose-Recovery

Status: **ready-for-dev**

## Story

As a Klarvo developer,
I want a deterministic, synchron-schreibendes Diagnose-Logging, das **vor** der `tracing-appender`-Initialisierung greift,
so that ich Boot-Pfade auf Windows-Release-Builds debuggen kann, ohne dass ein silent-failing Subscriber alle Diagnose-Info schluckt.

As a Klarvo user,
I want, dass Story 12.1 ("Production Pipeline Wire-Up") nicht nur Spec-/Code-Review-done, sondern auch **funktional smoke-getestet** ist (gesprochenes Wort erscheint als Text per Ctrl+V),
so that der Production-Pipeline-Wire-Up am Ende auch wirklich produktiv ist.

## Context

Story 12.1 ist Spec-Done + Code-Review-Done (2026-05-21). Der funktionale Smoke-Test ist **nicht erfolgreich**. 6 Bug-Diagnose-Commits über den Tag haben jeweils echte Windows-Release-Build-Bug-Klassen freigelegt, aber das End-Resultat bleibt: gesprochenes Wort führt nicht zu Text auf dem Bildschirm.

**Aktuelles Blocker-Symptom:** `%APPDATA%\Klarvo\logs\klarvo.YYYY-MM-DD.log` ist 0 Bytes nach jeder Test-Session, auch nach sauberem Tray-Quit, auch nach dem `WorkerGuard`-Drop-Fix (`249c8c1`). Ohne Logs ist keine weitere Diagnose des STT/Keystore/Network-Pfades möglich.

**Heutige Commits (Carry-Over):**
- `b004fec` — feat(12.1): production pipeline wire-up — manifest + groq registry
- `7f15d1d` — docs(11.5): code-review closure
- `3eba851` — chore(groq): log reqwest error detail for transport-layer diagnostics
- `da39ae3` — fix(keystore): defensive encoding detection in `WindowsKeystore::get()` (UTF-16-LE detection)
- `249c8c1` — fix(tracing): drop `WorkerGuard` in `RunEvent::Exit` to flush log buffer
- `6368c5c` — chore(clippy): replace manual modulo check with `.is_multiple_of()` (CI green)

**Offene Hypothesen für 0-Byte-Log:**
1. `init_tracing` returns `None` silent — `set_global_default` returnt `Err` (z.B. weil ein anderer Subscriber zuvor gesetzt wurde oder durch Tauri-Plugin-Konflikt)
2. `tracing-appender` DAILY rotation generiert einen anderen Pfad als erwartet (UTC- vs. Local-Date-Skew zur Mitternacht, Filename-Pattern-Detail)
3. Klarvo crasht **vor** `init_tracing` (Zeile 22 in `main.rs`) — vor jeglicher Tracing-Init
4. `WorkerGuard`-Drop greift, aber der `non_blocking`-Worker-Thread war zwischenzeitlich aus anderem Grund gestoppt (Panic im Worker, OOM, etc.)

**Davor unmaskierte InvalidHeaderValue-Hypothese (vom Tracing-Bug verdeckt):** Vor dem 0-Byte-Log-Symptom war der Fehler `InvalidHeaderValue` im Groq-Plugin. Theorie war: `cmdkey` speichert API-Keys als UTF-16-LE, `WindowsKeystore::get()` las als UTF-8. Fix in `da39ae3` adressiert diese Theorie defensiv. **Ob der Fix funktioniert, ist nicht verifiziert** — Tracing-Bug verdeckt das.

## Acceptance Criteria

**AC-1 — Pre-Tracing File-Marker:**
Vor jeglichem Aufruf von `init_tracing` schreibt `main()` synchron eine Boot-Marker-Datei (`%APPDATA%\Klarvo\diag\boot-marker.txt`) mit aktuellem Timestamp + `"main() reached"`. Sync `std::fs::write`, kein Buffer, kein Tracing. Wenn diese Datei nach einem Klarvo-Run **fehlt**, dann ist `main()` selbst nicht erreicht (Linker-Issue, DLL-Missing, etc.) — eindeutiges Signal.

**AC-2 — Boot-Stage-Marker an strategischen Punkten:**
Pre-Tracing-Marker werden an mindestens 4 Stellen geschrieben (Append-Mode in dieselbe Datei oder eine separate Stage-pro-Datei):
- `Stage 0: main() entered`
- `Stage 1: log_dir resolved = <path>` (post-APPDATA-Resolution)
- `Stage 2: init_tracing called, returned Some/None`
- `Stage 3: install_panic_hook done`
- `Stage 4: Tauri app.build() entered`
- `Stage 5: Tauri app.run() entered`

Jeder Marker enthält Timestamp und kurze Diagnostic-Info (z.B. `init_tracing returned None` mit Error-String falls verfügbar).

**AC-3 — Smoke-Test re-run nach Diagnose-Setup:**
Nach Build + Install der Diagnose-Marker: Smoke-Test (Hotkey-Press → sprechen → Hotkey-Release → 5 Sek warten → Tray-Quit) durchführen. **Erwartete Outputs:**
- Pre-Tracing-Marker-Datei existiert mit allen Stages bis zum tatsächlichen Crash- oder Hang-Punkt
- Reguläres tracing-Log (`%APPDATA%\Klarvo\logs\klarvo.YYYY-MM-DD.log`) enthält Inhalt **falls** `init_tracing` Success returned

**AC-4 — Diagnose-Bericht im Story-File:**
Story-File wird am Ende erweitert um Section `## Diagnostic Findings` mit:
- Welche Stages erreicht wurden (aus Marker-Datei)
- Tatsächliches Failure-Symptom (aus Tracing-Log, falls verfügbar)
- Decision: ist Folge-Bug ein Code-Fix in dieser Story, oder Folge-Story?

**AC-5 — Functional Smoke-Test verifiziert:**
Audio → STT → Cleanup → Paste-Pipeline funktioniert end-to-end. Gesprochenes Wort erscheint als Text im Zielfenster (Notepad oder beliebige Text-Eingabe). Bei AC-5-Pass: Diagnostic-Marker-Code wird entweder entfernt (wenn purely temporary) oder als permanentes Boot-Tracing-Feature eingebaut (Entscheidung in Story).

**AC-6 — Sprint-Status-Annotation für 12.1:**
Sprint-Status `12-1-production-pipeline-wire-up` Kommentar wird ergänzt: `(functional verification via 12.2)`. 12.1 bleibt `done` (Spec + Code-Review-Done), aber der Functional-Verification-Debt ist sichtbar.

## Approach

**Phase 1 — Diagnostic Setup (AC-1 + AC-2):**
- Helper-Funktion `klarvo_core::telemetry::diag::write_boot_marker(stage: &str, detail: &str)` mit synchron `OpenOptions::new().append(true).create(true).open(&marker_path)` + `writeln!`. Kein Tracing-Crate-Use, kein Buffer.
- Call-Sites in `shells/windows/src-tauri/src/main.rs` an den AC-2-Stellen.

**Phase 2 — Diagnose (AC-3):**
- Build + Smoke-Test. Marker-Datei lesen.
- Wenn Stages 0-2 da, aber 3+ fehlen: `install_panic_hook` crasht (sehr unwahrscheinlich) oder Hypothese 1 (silent init_tracing fail) ist real.
- Wenn nur Stage 0: Tracing-Init bricht main() ab (Hypothese 1 oder Argument-Resolution-Crash).
- Wenn alle Stages da, aber Tracing-Log leer: Tracing-Subscriber-Setup ist das Problem (Hypothese 1 oder 2).

**Phase 3 — Fix (AC-4 + AC-5):**
- Je nach Phase-2-Befund entweder Inline-Fix in 12.2 (wenn Scope passt) oder Folge-Story.
- Bei Tracing-Subscriber-Problem: prüfe ob ein Tauri-Plugin oder ein anderes Crate vorher `set_global_default` ruft (z.B. via `try_init` statt `set_global_default`).
- Bei UTC-Date-Skew: `tracing-appender::rolling::RollingFileAppender::builder().date_format(...)` mit lokaler Zeitzone konfigurieren (falls verfügbar) oder Suffix-Pattern korrigieren.

## Risks

- **Diagnose-Marker selbst silent-failt**: Wenn `OpenOptions::open` failed (z.B. `%APPDATA%` nicht resolvabel), gibt das auch keine Marker. Mitigation: Marker-Helper writet bei `open()`-Fail in `std::env::temp_dir()` als Fallback, mit `eprintln!` als letztem Mittel.
- **Story zieht sich:** Wenn Diagnose-Befund auf einen tiefen Bug zeigt (z.B. Tauri-internal Subscriber-Konflikt), kann das eine eigene Story werden. AC-4 macht diese Entscheidung explizit.

## Non-Goals

- Klarvo um ein generelles Boot-Telemetrie-Feature erweitern. AC-5-Entscheidung explizit: nur wenn nützlich für späteren Bug-Bedarf.
- Andere Pipeline-Stage-Plugins integrieren (DeepSeek, Polished). Bleibt aus Story-12.1-Scope.
- Tracing-Library wechseln. Wenn `tracing-appender` Bug ist, dann config-fix oder Bypass-helper, nicht Library-Swap.

## Out-of-Scope (Carry-Over)

- 12.1-DF1, 12.1-DF2 (siehe `deferred-work.md`)
- 11.5-DF1, 11.5-DF2 (siehe `deferred-work.md`)
- Pipeline-Performance-Profiling
- Logging-Library-Migration

## Definition of Done

- [ ] AC-1: Pre-Tracing-Marker schreibt synchron
- [ ] AC-2: 4+ Boot-Stage-Marker an `main.rs`-Stellen
- [ ] AC-3: Smoke-Test mit Diagnose durchgeführt, Outputs gesichert
- [ ] AC-4: `## Diagnostic Findings` in Story-File ergänzt
- [ ] AC-5: Audio → STT → Text-Paste funktioniert; Diagnose-Code-Entscheidung dokumentiert
- [ ] AC-6: Sprint-Status 12.1-Annotation ergänzt
- [ ] Windows-Cross-Compile-Check vor Story-Closure (`cargo check --target x86_64-pc-windows-gnu` — siehe memory `feedback_windows_cross_compile_verify`)
- [ ] Clippy-Gate grün auf Windows-CI
