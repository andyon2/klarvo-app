# Architecture Decision Records (ADR)

Dieses Verzeichnis enthält Architecture Decision Records für Klarvo v2.

## Format

Pro ADR eine eigene Datei: `NNNN-kurztitel.md` mit aufsteigender Nummer.

Minimum-Struktur:

```
# ADR-NNNN: Titel

**Status:** Proposed | Accepted | Superseded by ADR-MMM | Deprecated
**Date:** YYYY-MM-DD
**Context:** (1-3 Sätze — welches Problem, welche Rahmenbedingungen)
**Decision:** (kurz und direkt, 1-3 Zeilen)
**Consequences:** (was folgt daraus, positiv und negativ)
```

Stubs mit `Status: Proposed` dürfen anfangs unvollständig sein — Vollständigkeit kommt spätestens bei `Accepted`.

## Referenz

`output/planning-artifacts/architecture.md` ist die authoritative Architektur-Quelle. ADRs dokumentieren Entscheidungen, die entweder (a) die Architektur-Quelle ergänzen oder (b) Abweichungen von ihr explizit machen.
