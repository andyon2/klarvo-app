# TDD-Subagent-Pattern

Quelle: AI-Hero-Research Delta-Analyse (2026-03-19)

- Separate Subagents fuer Test-Schreiben, Implementierung und Refactoring verhindern Context Pollution
- Red-Green-Refactor mit getrennten Agent-Kontexten: Test-Agent schreibt failing Test → Implementierungs-Agent macht ihn gruen → Refactoring-Agent raeumt auf
- Vorteil: Jeder Agent hat frischen Kontext und klaren Fokus — vermeidet das Problem dass Test-Wissen die Implementierung beeinflusst (oder umgekehrt)
- Umsetzung: Claude Code Subagents (Agent-Tool) oder separate Sessions pro Phase
- Besonders wertvoll bei Rust wo Compile-Fehler + Test-Output den Kontext schnell fuellen

Vollstaendige Quelle: ~/claude-projects/project-builder/briefings/ai-hero-research/delta-analyse.md
