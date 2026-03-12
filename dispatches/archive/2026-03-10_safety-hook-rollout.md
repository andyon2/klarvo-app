# Safety-Hook + Permission-Migration

Quelle: Project Builder Session (2026-03-10)

- Globaler Safety-Hook aktiv (`~/.claude/hooks/safety-hook.sh`). Gilt fuer alle Projekte auf allen Maschinen (WSL + Hetzner).
- Destruktive Befehle (rm -rf, git push --force, git reset --hard) sind hart blockiert. .env-Zugriff und Commits in fremden Repos loesen eine User-Abfrage aus.
- Lokale Permissions in `.claude/settings.json` entfernt (waren redundant). Globale Settings decken das ab.
- Dein System Prompt hat ein neues Kontextschutz-Prinzip: Bei Aufgaben die nicht vom bisherigen Session-Kontext profitieren, neue Session vorschlagen.
