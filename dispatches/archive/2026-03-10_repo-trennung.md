# Repo-Trennung: dikta (privat) + dikta-public

Quelle: Project Builder Session (2026-03-10)

- `andyon2/dikta` ist jetzt **private** (war public). Dein Arbeitsrepo, Agent + Code.
- `andyon2/dikta-public` existiert als neues **public** Repo (leer, noch kein Commit).
- `scripts/publish.sh` liegt bereit (noch nicht committed). Synct Produktcode nach `~/dikta-public`, filtert Agent-Dateien raus.
- Initialer Commit in dikta-public steht noch aus. Deine Aufgabe: `publish.sh` committen, dann `cd ~/dikta-public && git commit && git push`.
- Starter-Script hat neue Flags: `--remote` und `--get-prompt` (fuer Claude Launcher). Auch noch nicht committed.
