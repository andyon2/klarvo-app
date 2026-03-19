# Pre-Commit Hooks als AI-Guardrails

Quelle: AI-Hero-Research Delta-Analyse (2026-03-19)

- Pre-Commit Hooks sind zuverlaessiger als Instruktionen in CLAUDE.md — Hooks werden IMMER ausgefuehrt, Instruktionen werden mit steigendem Kontext schlechter befolgt
- Drei-Schichten-Modell: Prompts verduennen ueber die Session / Skills bleiben frisch pro Aufruf / Hooks sind extern und unbestechlich
- Empfohlene Hook-Patterns fuer Code-Projekte:
  - **Lint/Format-Check:** `cargo fmt --check && cargo clippy` vor jedem Commit — verhindert dass AI unformattierten Code committed
  - **Test-Runner:** `cargo test` als Pre-Commit — faengt Regressionen sofort ab
  - **Commit-Message-Validation:** Format-Pruefung (z.B. Conventional Commits)
- Hooks in `.claude/settings.json` konfigurieren (`hooks.pre_commit`), nicht in git hooks — so gelten sie spezifisch fuer Claude Code Sessions
- Kosten-Nutzen: Minimaler Aufwand, maximale Konsistenz — einmal einrichten, dauerhaft wirksam

Vollstaendige Quelle: ~/claude-projects/project-builder/briefings/ai-hero-research/delta-analyse.md
