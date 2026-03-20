# Characterization Tests vor Refactoring

Quelle: AI-Hero-Research Delta-Analyse (2026-03-19)

- Bestehendes Verhalten als Golden Master erfassen BEVOR der Agent refactored — verhindert versehentliche Verhaltensaenderungen
- Approval-Test-Ansatz: Aktuelles Output als "korrekt" snapshotten, dann gegen diesen Snapshot testen
- Besonders wichtig wenn AI bestehenden Code umstrukturiert — der Agent "versteht" implizites Verhalten nicht immer
- Umsetzung: Vor jedem groesseren Refactoring eine Test-Suite schreiben die das IST-Verhalten dokumentiert. Erst wenn alle Tests gruen sind, mit dem Refactoring beginnen
- Fuer Rust: `insta` Crate fuer Snapshot-Tests eignet sich gut als Characterization-Test-Tool

Vollstaendige Quelle: ~/claude-projects/project-builder/briefings/ai-hero-research/delta-analyse.md
