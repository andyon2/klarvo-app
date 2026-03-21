---
name: lint-fix
description: Fuehrt Linter und Formatter ueber den Code aus. Auto-Fix wo moeglich. Aufrufen mit optionalem Scope, z.B. "/lint-fix" oder "/lint-fix rust" oder "/lint-fix frontend".
argument-hint: "[scope] -- all | rust | frontend (default: all)"
allowed-tools: Read, Bash, Glob
context: fork
model: haiku
---

Fuehre Linter und Formatter fuer Voxlit aus.

## Argumente

Aus `$ARGUMENTS` extrahiere den Scope: `all` (default) | `rust` | `frontend`

## Vorgehensweise

### Rust (scope = `rust` oder `all`)

```bash
cd /home/andyon2/claude-projects/voxlit

# Formatter
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check 2>&1
# Falls Aenderungen noetig:
cargo fmt --manifest-path src-tauri/Cargo.toml 2>&1

# Clippy (Linter)
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings 2>&1
```

### Frontend (scope = `frontend` oder `all`)

```bash
cd /home/andyon2/claude-projects/voxlit

# Prettier (Formatter)
npx prettier --check "src/**/*.{ts,tsx}" 2>&1
# Falls Aenderungen noetig:
npx prettier --write "src/**/*.{ts,tsx}" 2>&1

# ESLint (Linter)
npx eslint "src/**/*.{ts,tsx}" 2>&1
```

### Report

```
LINT REPORT -- Voxlit ([scope])

FORMATTER:
  Rust: [OK / n Dateien formatiert]
  Frontend: [OK / n Dateien formatiert]

LINTER:
  Clippy: [OK / n Warnungen / n Fehler]
  ESLint: [OK / n Warnungen / n Fehler]

PROBLEME (falls vorhanden):
  [Datei:Zeile] [Problem] -- [Auto-fixbar: Ja/Nein]

ZUSAMMENFASSUNG: [Alles sauber / n Probleme gefunden, davon m auto-gefixt]
```
