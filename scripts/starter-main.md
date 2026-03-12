Sessionstart:
1. git pull --ff-only
2. GitHub Issues abholen: gh issue list --repo andyon2/dikta-public --state open ausfuehren. Neue Issues (noch nicht in Inbox) in feedback/inbox.md unter "Neue Eintraege" eintragen. Duplikat-Check: Nur eintragen wenn GitHub #N nicht schon in der Datei steht.
3. Lies project-status.md -- dein Briefing wo das Projekt steht.
4. Lies feedback/inbox.md -- offenes Tester-Feedback? Neue Bugs? Wenn ja: kurz erwaehnen. Nicht eigenmaechtigt untersuchen oder Agenten losschicken. Andy entscheidet pro Eintrag was passiert.
5. Pruefe dispatches/inbox/ -- Dateien vorhanden? Sofort integrieren (Regel in CLAUDE.md aufnehmen, Knowledge aktualisieren, etc.), Datei nach dispatches/archive/ verschieben, Zeile in dispatches/log.md ergaenzen. Dann committen und pushen.
6. Lies knowledge/architecture.md -- die geltenden Tech-Entscheidungen.
6b. Lies knowledge/workflow.md -- wie Andy arbeitet, Build/Test-Wege, Lektionen.
7. Pruefe: Gibt es neue/geaenderte Dateien seit der letzten Session? (git status oder Datei-Timestamps)
8. Wenn eine Phase gerade laeuft: Pruefe, welche Tasks offen sind und schlage den naechsten Schritt vor.
