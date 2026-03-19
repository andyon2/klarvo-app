# 60% Kontextauslastungs-Limit + Frischer Kontext

Quelle: AI-Hero-Research Delta-Analyse (2026-03-19)

- Ab 40-60% Kontextauslastung degradiert die Output-Qualitaet merklich — alle Instruktionen werden gleichmaessig schlechter befolgt, nicht nur die letzten
- Frischer Kontext (neue Session) schlaegt ueberfuellten Kontext: Lieber eine neue Session starten als in einer langen weiterzuarbeiten
- Sessions neu starten bevor Compaction einsetzt — nach Compaction ist der Qualitaetsverlust bereits eingetreten
- Empfehlung: CLAUDE.md um diesen Punkt erweitern, damit der Agent selbst darauf achtet (z.B. "Bei komplexen Aufgaben: neue Session bevorzugen statt lange Konversationen")
- CLAUDE.md selbst kurz halten (≤60 Zeilen) — empirisch begruendet, nicht nur Faustregel

Vollstaendige Quelle: ~/claude-projects/project-builder/briefings/ai-hero-research/delta-analyse.md
