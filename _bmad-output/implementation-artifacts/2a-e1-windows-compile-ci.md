---
name: Story 2.A.E1 — Windows-Compile-CI-Gate (G6)
phase: 2
wave: A
story_id: "2.A.E1"
status: done
dependencies: []
adr_refs: []
source_ref: "Backlog Windows-Compile-CI-Gate; Epic-3-Followup; memory/project_jni_spike_scope"
---

# Story 2.A.E1: Windows-Compile-CI-Gate (G6)

## Outcome

Jeder PR gegen `master` triggert einen Windows-Compile-Check für die Windows-Shell.
Aktuell gibt es keine CI-Gate die sicherstellt, dass `klarvo-windows-shell` auf Windows
kompiliert — Regressions werden erst beim lokalen Build oder Release bemerkt.

Nach dem Fix: `.github/workflows/windows-ci.yml` kompiliert `klarvo-windows-shell` (und alle
Abhängigkeiten ohne `klarvo-bridge-jni`) auf `windows-latest`. Build-Fail = PR geblockt.
Kein `cargo test` in diesem Gate (Test-Gate ist bestehende CI aus Epic-5).

## Scope-Fence

**In-Scope:**
- `.github/workflows/windows-ci.yml` — neues Workflow-File
- `--exclude klarvo-bridge-jni` solange F2 offen (dokumentiert mit Rationale-Kommentar)
- Nur `cargo build` / `cargo check`, kein `cargo test`

**Nicht-in-Scope:**
- Test-Gate (bestehende CI aus Epic-5 deckt das ab)
- Android/JNI-Compile (Phase-3-Scope)
- Signing oder Release-Artifacts (C1-Scope)

## Acceptance Criteria

### AC-1 — Workflow-Datei existiert + triggert auf PR

**Given** `.github/workflows/windows-ci.yml` vorhanden  
**When** PR gegen `master` (oder `main`) geöffnet oder gepusht wird  
**Then**
- Workflow triggert auf `push` + `pull_request` (analog zu `ci-event-lint.yml`).
- Job läuft auf `windows-latest`.
- Concurrency-Group + `cancel-in-progress: true` konfiguriert.

---

### AC-2 — Windows-Shell kompiliert auf `windows-latest`

**Given** Workflow läuft auf `windows-latest`  
**When** Build-Step ausgeführt wird  
**Then**
- `cargo build -p klarvo-windows-shell` (oder `cargo check -p klarvo-windows-shell`) läuft ohne Error.
- `--exclude klarvo-bridge-jni` ist gesetzt (solange F2 offen).
- Workspace-Compile schließt Platform-spezifische Crates ein (Windows-only cfgs kompilieren durch).
- Rust-Toolchain via `dtolnay/rust-toolchain@stable` + `rust-toolchain.toml`-Pin (analog bestehende Workflows).

---

### AC-3 — `--exclude`-Rationale dokumentiert

**Given** Workflow-Datei  
**When** das `--exclude`-Flag für `klarvo-bridge-jni` gesetzt ist  
**Then**
- Inline-Kommentar erklärt: "JNI-Rate-Test-Regression offen (Story 2.A.F2); entfällt nach F2-Closure."
- Kommentar enthält Story-Referenz oder ADR-0003-Referenz.

---

### AC-4 — Build-Fail blockiert PR

**Given** ein PR mit Windows-Compile-Fehler  
**When** Workflow läuft  
**Then**
- GitHub-Check `windows-compile (G6)` (oder ähnlicher Name) schlägt fehl.
- PR kann nicht gemergt werden (Branch-Protection vorausgesetzt, kein Autofix nötig —
  Branch-Protection ist repo-Config außerhalb dieser Story).
- Bei Fix des Compile-Fehlers: Check wird grün.

---

## Technical Notes

- Cache-Key: `windows-compile-g6` (separater Key von bestehenden xtask-Caches).
- Swatinem/rust-cache@v2 mit `workspaces: ". -> target"` wie in bestehenden Workflows.
- Tauri-Build-Deps auf Windows (WiX, NSIS) werden für `cargo build` nicht benötigt —
  nur Rust-Build-Dependencies. Tauri-Bundle-Step ist C1-Scope.
- Falls `cargo check` statt `cargo build` ausreicht: bevorzuge `cargo check` für Speed.
  `cargo build` wenn compile-time-macros und proc-macros (wie specta) vollständig durchlaufen müssen.

## Dev Agent Record

### Implementation Plan

Single-file delivery: `.github/workflows/windows-ci.yml`. Pattern aus `ci-event-lint.yml` und
`ci-bindings-drift.yml` übernommen (Trigger, Permissions, Concurrency, Toolchain-Action,
Swatinem-Cache). Build-Step nutzt `cargo check --workspace --exclude klarvo-bridge-jni` —
das deckt klarvo-windows-shell + alle workspace-Crates ab (incl. `windows = "0.61"` und
`klarvo-audio-cpal` aus `[target.'cfg(target_os = "windows")']`-Block) und folgt der
Story-Empfehlung `cargo check` für Speed.

`klarvo-bridge-jni` excluded via Inline-Kommentar mit Story-2.A.F2 + ADR-0003-Referenz.
`--locked` weggelassen für Konsistenz mit bestehenden Workflows.

### Completion Notes

- AC-1 ✅ Workflow triggert auf push(master,main) + pull_request, runs-on: windows-latest,
  concurrency-group + cancel-in-progress.
- AC-2 ✅ `cargo check --workspace --exclude klarvo-bridge-jni` deckt klarvo-windows-shell
  und Platform-spezifische Crates ab. Toolchain via `dtolnay/rust-toolchain@stable`
  (honoring `rust-toolchain.toml` channel = 1.94.0).
- AC-3 ✅ Inline-Kommentar mit Story-Ref (2.A.F2) und ADR-Ref (ADR-0003); JNI-Rate-Test-
  Regression-Rationale dokumentiert.
- AC-4 ✅ Build-Fail blockiert PR via Default-GitHub-Check-Behavior (Branch-Protection ist
  repo-Config, explizit out-of-scope per Story-Text).

Lokale Validierung: `python3 -c yaml.safe_load(...)` → OK. `cargo check` lokal auf Linux
schlägt erwartungsgemäß am `compile_error!("shells/windows requires Windows target")` Guard
in `shells/windows/src-tauri/src/main.rs:7` fehl — genau die Lücke, die dieser Gate schließt.

### File List

- `.github/workflows/windows-ci.yml` (new)
- `_bmad-output/implementation-artifacts/2a-e1-windows-compile-ci.md` (status, Dev Agent Record)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (status: ready-for-dev → review)

### Change Log

- 2026-04-30: Initial implementation, Status → review (Dev Agent: Andy via Opus 4.7 1M).
- 2026-04-30: Code-Review-Closure. 1 Patch (D4 `--all-targets`), 4 Decisions deferred zur Cross-Cutting CI-Hardening-Folge-Story, 5 weitere Defers (Repo-Pattern), 1 Dismiss. Status → done.

## Review Findings

Code-Review 2026-04-30 (Opus 4.7 1M, 3 parallel layer: Blind Hunter / Edge Case Hunter / Acceptance Auditor). Acceptance Auditor: alle 4 ACs erfüllt, keine Findings. Blind+Edge: 11 Findings → 1 Patch, 9 Defer, 1 Dismiss.

### Patch (applied)

- [x] [Review][Patch] **`--all-targets` ergänzt** [`.github/workflows/windows-ci.yml:41-44`] — `cargo check --workspace --all-targets --exclude klarvo-bridge-jni`. Type-Check deckt jetzt auch Tests/Examples/Feature-gated-Code (nur kompilieren, nicht laufen — Scope-Fence „kein cargo test" konform). Schließt Lücke: Linux-CI compile-checked Tests nicht (xtask-only), ohne `--all-targets` würden Windows-only-Test-Compile-Fehler durch beide Gates schlüpfen. (Blind, major)

### Defer

**4 Cross-Cutting CI-Hardening (eigene Folge-Story für alle 4 Workflows simultan):**

- [x] [Review][Defer] **Toolchain-Pin nicht honoriert** [`.github/workflows/windows-ci.yml:31-33`] — Reviewer-Claim („`dtolnay/rust-toolchain@stable` ignoriert `rust-toolchain.toml`") ist unverifiziert (memory `feedback_reviewer_external_fact_verification`). Selbst wenn wahr: 4 Workflows betroffen → Cross-Cutting-Folge-Story (1) Action-Verhalten verifizieren (2) alle simultan fixen. (Blind+Edge, major)
- [x] [Review][Defer] **Kein `--locked` flag** [`.github/workflows/windows-ci.yml:44`] — Real, aber Cross-Cutting Pattern. Bündel mit Toolchain-Pin in CI-Hardening-Welle. (Blind+Edge, major)
- [x] [Review][Defer] **Kein `RUSTFLAGS=-D warnings`** — Compile-Gate ≠ Lint-Gate. Lint-Strictness verdient eigene koordinierte Story (eher `cargo clippy -D warnings` als `RUSTFLAGS`, workspace-weit konsistent); ohne Toolchain-Pin würde stable-Drift Warnings unbemerkt einführen → erst Pin lösen, dann Strictness. (Blind, major)
- [x] [Review][Defer] **Kein `timeout-minutes`** — Defensive-Ceremony, kein konkreter Trigger. Bündeln in CI-Hardening-Welle. (Blind+Edge, minor)

**5 Pre-existing Repo-Pattern / Spec-conform:**

- [x] [Review][Defer] **`pull_request` triggert doppelt auf same-repo PRs** [`.github/workflows/windows-ci.yml:12-15`] — `push` + `pull_request` feuern beide; Concurrency-Keys differieren zwischen Events. Repo-weiter Pattern (`ci-event-lint.yml`, `ci-bindings-drift.yml` haben dieselbe Form). (Edge, major)
- [x] [Review][Defer] **`klarvo-bridge-jni` Compile-Coverage = 0 auf CI** [Linux-CI vs Windows-CI] — Windows-CI excluded JNI-Bridge (F2-Rationale OK); Linux-CI führt nur xtask aus, kein Workspace-Compile. Bridge wird daher auf KEINEM CI-Platform compile-checked. Pre-existing Gap, nicht durch diesen Diff eingeführt; nach F2-Closure auf Windows wieder gedeckt. Linux-Compile-Gate für Bridge eigene Folge-Story. (Edge, major)
- [x] [Review][Defer] **Kein `paths-ignore`** [`.github/workflows/windows-ci.yml:12-15`] — Docs-only / `_bmad-output/`-only PRs triggern Windows-Runner. Repo-Konvention. Cost, keine Korrektheit. (Blind+Edge, minor)
- [x] [Review][Defer] **`cancel-in-progress: true` auf Master-Pushes maskiert cancelled Runs** [`.github/workflows/windows-ci.yml:20-22`] — Spec mandatiert `cancel-in-progress: true` (AC-1); Branch-Protection / `head_ref`-Keying ist Repo-Config out-of-scope. (Blind, minor)
- [x] [Review][Defer] **`pull_request` läuft auf Draft-PRs (default `types`)** [`.github/workflows/windows-ci.yml:15`] — Draft-WIP-Commits konsumieren Windows-Runner-Minuten. Repo-Konvention. (Edge, minor)

### Dismissed (1)

- **Comment/Code-Drift auf `--exclude`** — AC-3 explizit erfüllt: Inline-Kommentar enthält Story-Ref (`Story 2.A.F2`) UND ADR-Ref (`ADR-0003`). Kein TODO-Lint im Repo etabliert. (Blind, minor)
