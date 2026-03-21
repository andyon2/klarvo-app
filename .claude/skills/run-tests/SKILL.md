---
name: run-tests
description: Fuehrt Tests aus und formatiert die Ergebnisse als Report. Aufrufen mit Scope, z.B. "/run-tests all" oder "/run-tests rust" oder "/run-tests frontend".
argument-hint: "[scope] -- all | rust | frontend | [modulname]"
allowed-tools: Read, Bash, Glob
context: fork
model: haiku
---

Fuehre Tests fuer Voxlit aus und erstelle einen strukturierten Report.

## Argumente

Aus `$ARGUMENTS` extrahiere den Scope: `all` | `rust` | `frontend` | ein spezifischer Modulname

## Vorgehensweise

### Rust-Tests (scope = `rust` oder `all` oder ein Rust-Modulname)

```bash
cd /home/andyon2/claude-projects/voxlit

# Alle Rust-Tests oder spezifisches Modul
cargo test --manifest-path src-tauri/Cargo.toml [--lib modulname] 2>&1
```

### Frontend-Tests (scope = `frontend` oder `all`)

```bash
cd /home/andyon2/claude-projects/voxlit && npm test 2>&1
```

### Report-Format

```
TEST REPORT -- Voxlit ([scope])
Datum: [aktuelles Datum]

ZUSAMMENFASSUNG:
  Bestanden: [n]
  Fehlgeschlagen: [n]
  Uebersprungen: [n]
  Gesamt: [n]

FEHLGESCHLAGENE TESTS:
  [testname]: [Fehlermeldung -- kurz]
    -> Datei: [pfad:zeile]
    -> Moegliche Ursache: [1-Satz-Einschaetzung]

ALLE BESTANDEN? [Ja/Nein]

EMPFEHLUNG: [Was als naechstes tun bei Fehlern, oder "Bereit fuer Commit" bei Erfolg]
```

Wenn alle Tests bestehen, halte den Report kurz (nur Zusammenfassung + "Bereit fuer Commit").
