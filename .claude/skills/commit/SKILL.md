---
name: commit
description: "Session-Commit: /track ausfuehren, dann committen und pushen. Standard fuer Sessionende."
---

Fuehre einen Session-Commit durch:

1. Rufe `/track` auf -- aktualisiert project-status.md mit dem Session-Fortschritt.
2. `git add -u` -- alle getrackten Aenderungen stagen.
3. `git diff --cached --stat` -- Ueberblick zeigen.
4. Commit-Message aus den Aenderungen ableiten (kurz, deutsch, beschreibend).
5. `git commit` mit der Message.
6. `git push`

Wenn nichts zu committen ist (keine staged changes nach Schritt 2), melde das und ueberspringe Schritt 3-6.
