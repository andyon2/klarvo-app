---
name: debug-error
description: Analysiert einen Fehler (Build-Fehler, Runtime-Crash, Test-Failure) und schlaegt einen Fix vor. Aufrufen mit Fehler-Beschreibung oder Dateiname, z.B. "/debug-error cargo build failed" oder Fehler-Output einfach reinkopieren.
argument-hint: "[fehler-beschreibung oder error-output]"
allowed-tools: Read, Bash, Grep, Glob, WebSearch
context: fork
model: sonnet
---

Analysiere einen Fehler und finde die Ursache.

## Argumente

Aus `$ARGUMENTS` extrahiere die Fehler-Beschreibung oder den Error-Output.

## Vorgehensweise

1. **Fehler klassifizieren:**
   - Compile-Error (Rust / TypeScript)
   - Runtime-Error (Panic, Exception, Crash)
   - Test-Failure
   - Build-System-Error (Cargo, npm, Gradle, Tauri)
   - API-Error (HTTP-Status, Response-Fehler)

2. **Kontext sammeln:**
   - Relevante Quell-Dateien lesen (aus Error-Output die Dateien/Zeilen extrahieren)
   - `knowledge/platform-notes.md` auf bekannte Quirks pruefen
   - Bei unbekannten Fehlern: WebSearch

3. **Analyse:**
   - Was ist die ROOT CAUSE? (Nicht nur das Symptom)
   - Ist es ein lokales Problem (eine Datei) oder ein systemisches (Architektur)?
   - Gibt es verwandte Probleme, die auch gefixt werden sollten?

4. **Report:**

```
FEHLER-ANALYSE

Fehler: [Kurzbeschreibung]
Typ: [Compile | Runtime | Test | Build | API]
Root Cause: [Was genau das Problem verursacht]

Betroffene Dateien:
- [datei:zeile] -- [was dort falsch ist]

Fix-Vorschlag:
[Konkreter Code-Aenderungsvorschlag oder Schritt-fuer-Schritt-Anleitung]

Zustaendiger Agent: [rust-core | ui-dev | android-platform]

Verwandte Probleme: [Falls beim Debugging weitere Probleme aufgefallen sind]
```

5. **Wichtig:** Dieses Skill FIXT den Fehler NICHT selbst. Es analysiert nur und schlaegt einen Fix vor. Der Main-Agent entscheidet, wer den Fix implementiert.
