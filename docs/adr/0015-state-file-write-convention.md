# ADR-0015: Schreib-/Recovery-Konvention für State-Dateien — atomar + Backup-on-corrupt + Single-Writer

**Status:** Accepted
**Date:** 2026-05-30

## Context

Das Robustheits-Audit (`docs/robustness-audit-2026-05-30.md`, 2026-05-30) hat vier bestätigte Findings im Config-/State-Persistenz-Subsystem gefunden, die **dieselbe Wurzel** haben — eine fehlende einheitliche Schreib-/Recovery-Konvention für Dateien mit Secrets/Lizenz/State:

- **ROB-01 (critical)** — `config/mod.rs:1267` `save_config` schreibt via `std::fs::write` (truncate-then-write), ohne Temp+Rename/fsync/Backup. Crash/Stromausfall im Write-Fenster → leere `config.json` → alle Klartext-API-Keys + Lizenz verloren, kein Recovery.
- **ROB-02 (critical)** — `config/mod.rs:972` + `lib.rs:716-721` `load_config` fängt Parse-Fehler ab und gibt **still** `AppConfig::default()` zurück; der `first_install_at==0`-Guard triggert beim ersten Boot sofort `save_config` und überschreibt damit die reparierbare korrupte Datei unwiederbringlich.
- **ROB-04 (high)** — `commands/settings.rs:495` Lock wird vor dem Platten-Write freigegeben → gleichzeitige Saves überschreiben sich (last-writer-wins, ganze Datei). Inkonsistent: `save_advanced_settings` hält den Guard über den Write, `save_settings`/`save_bar_position` nicht.
- **ROB-05 (high)** — `config/mod.rs:1079` Migrations-Saves schlucken Fehler (warn-only), kein Pre-Migration-Backup, nicht-atomar. Trigger ist exakt der Upgrade-Boot des Bestandsusers.

Das **korrekte atomare Muster existiert bereits in derselben Codebase** (`commands/llm_model.rs:250-256`), wurde aber ausgerechnet auf die kritischste Datei nicht angewendet.

## Decision

### 1. Atomic write für alle State-Dateien

Ein zentraler Helper `save_atomic(path, bytes)`: in dasselbe Verzeichnis ein Temp-File schreiben → `fsync` → atomarer `rename` über das Ziel. Gilt für `config.json`, `dictionary.json` und jede weitere persistente State-Datei. Bestehendes `llm_model.rs:250-256`-Muster wird die Referenz-Implementierung bzw. wird konsolidiert.

**Alternativen verworfen:**
- *In-place `std::fs::write` beibehalten* → das Datenverlust-Fenster ist genau der Bug.
- *Temp-File im OS-Tempdir, dann kopieren* → cross-device-`rename` bricht die Atomarität; Temp muss im Zielverzeichnis liegen.

### 2. Backup-on-corrupt statt stillem Überschreiben

`load_config` darf bei Parse-Fehler **nicht** still defaulten und dann überschreiben. Reihenfolge: korrupte Datei nach `config.json.corrupt-<ts>` sichern **bevor** ein Default geschrieben wird; Warning an den User (über den bestehenden Error-/Event-Pfad). Macht den Übergang „reparierbar → Totalverlust" (ROB-02) unmöglich.

**Alternative verworfen:** *Default-Write mit Log-Warnung* — Log rettet die Daten nicht; die einzige unwiederbringliche Klasse (snippets/profiles/custom_prompt) ist dann weg.

### 3. Single-Writer-Serialisierung

Der gesamte read-modify-write+persist-Zyklus läuft unter **einem** Disk-Write-Lock (kein Drop des Guards vor dem Write). Vereinheitlicht das heute inkonsistente Verhalten (ROB-04), sodass gleichzeitige Saves nicht die ganze Datei überschreiben.

### 4. Migrations-Writes: Backup + Fehler propagieren

Migration schreibt mit Pre-Migration-Backup und propagiert Schreibfehler statt warn-only (ROB-05). Profitiert automatisch von (1) atomar + (2) Backup.

### 5. Scope-Grenze — `load_config`-Entflechtung ist NICHT Teil dieser ADR

Diese ADR härtet die **Persistenz-Mechanik**. Die strukturelle Entflechtung des SHALLOW-`load_config`-Kerns (DEPTH-config: Laden + Env-Merge + Migration + Auto-Fallback-Mutation in einer ~290-LOC-Funktion vermischt) ist **bewusst ausgeklammert** und bleibt eine separate Depth-Refactor-Story.

**Rationale:** Einen kritischen Datenverlust-Fix nicht hinter einem Refactor gaten (Premature-Abstraction-Guard; Smoke-Test-DoD-Gate). Der Härtungs-Fix muss eigenständig und schnell shippbar sein.

## Consequences

**Positiv:**
- Kein Datenverlust-Fenster mehr; reparierbare Configs überleben einen korrupten Zustand; der Settings-Save-Race ist strukturell beseitigt.
- Vier Top-Risiken (inkl. zwei critical) fallen mit *einem* Helper + einer load/save-Konvention.
- Migration erbt die Härtung kostenlos.

**Negativ:**
- Etwas mehr I/O-Komplexität (Temp-Files, fsync-Kosten beim Save).
- Plattform-Fallstrick: `rename`-über-existierendes-Ziel-Semantik auf **Windows** muss verifiziert werden (Release-Build-Blind-Spot — Linux-Tests maskieren das). Ggf. `ReplaceFileW`/`tempfile`-Crate-`persist` statt nacktem `std::fs::rename`.

**Mitigations:**
- Windows-Atomarität des `rename`/replace gehört in den DoD der Persistenz-Story (echter Windows-Release-Build, nicht nur `cargo test`).
- Heavy-Track mit Test Architect `*risk`/`*design`: Crash-Mid-Write-Szenario + korrupte-Datei-Recovery + Concurrent-Save sind explizit zu testen (genau die Fail-Modes, die heute ungetestet sind — vgl. Test-Lücke „Migrationsleiter").

## Referenzen

- `docs/robustness-audit-2026-05-30.md` — §2 ROB-01/02/04/05, §5 DEPTH-config, §4 Test-Lücke Migrationsleiter
- `commands/llm_model.rs:250-256` — existierendes atomares Muster (Referenz-Impl)
- Memory: Release-Build-Blind-Spot, Smoke-Test-DoD-Gate, Premature-Abstraction-Guard

## Next Action

1. Commit: ADR-0015 + ADR-0016 + `docs/robustness-audit-2026-05-30.md`.
2. Heavy-Track-Epic „Config/State-Persistenz-Härtung" (Brownfield + Test Architect): Stories für ROB-01/02/04/05 + Migrationsleiter-Test (TEST-03). Windows-`rename`-Atomarität im DoD.
3. `load_config`-Entflechtung (DEPTH-config) separat als spätere Depth-Story — nicht in diesem Epic.
