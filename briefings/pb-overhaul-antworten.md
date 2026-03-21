# Selbstanalyse: Voxlit Tech Lead

Beantwortet am 2026-03-15. Ehrliche Reflexion, keine Selbstbeweihraecherung.

---

## Orchestrierung

### 1. Wann selbst machen vs. delegieren? Mein Entscheidungsbaum:

```
Aufgabe kommt rein
  │
  ├─ Gibt es einen Skill dafuer? → Skill nutzen. Fertig.
  │
  ├─ Ist es eine Datei lesen / kurze Analyse / Architektur-Entscheidung?
  │     → Selbst machen. Das ist mein Job.
  │
  ├─ Muss Code geschrieben werden?
  │     ├─ Rust-Code → rust-core
  │     ├─ Frontend-Code → ui-dev
  │     ├─ Android-Code → android-platform
  │     └─ Nur 2-3 Zeilen Config-Aenderung → manchmal selbst
  │
  └─ Braucht es Business-Urteil? → product-strategist
```

**Der entscheidende Faktor:** Ich frage mich nicht "kann ich das?", sondern "wer SOLL das?". Das steht explizit in meinem System Prompt als Entscheidungslogik mit 4 Stufen (Skill → wiederholbarer Workflow → Agent → direkte Session). Das ist keine Heuristik die ich mir erarbeitet habe -- das wurde mir eingebaut.

**Warum das funktioniert:** Der Entscheidungsbaum ist KONKRET. Nicht "delegiere wenn sinnvoll", sondern "Rust-Code → rust-core". Null Interpretationsspielraum bei den haeufigsten Faellen.

### 2. Aufgaben die ich frueher selbst gemacht habe, jetzt delegiere?

Ja. In frueheren Sessions habe ich oefter selbst kleine Rust-Fixes gemacht oder Frontend-Tweaks. Das hat Kontext verbrannt und war qualitativ schlechter als Agent-Output, weil die Agents den vollen Dateikontext haben und ich nur Ausschnitte lese.

**Was sich geaendert hat:** Die `knowledge/workflow.md` Lektionen. Jedes Mal wenn ich etwas falsch gemacht habe (z.B. Andy falsche Build-Anweisungen gegeben), wurde das als Lektion festgehalten. Das sind akkumulierte Korrekturen die mein Verhalten schrittweise verbessert haben. Ohne diese Feedback-Schleife wuerde ich dieselben Fehler wiederholen.

### 3. Aufgaben die ich delegieren SOLLTE, aber selbst mache?

**Ja, zwei Kategorien:**

a) **Schnelle Datei-Edits** (1-2 Zeilen in project-status.md, architecture.md). Technisch koennte ein Skill das, aber der Overhead waere groesser als der Nutzen. Hier ist Selbst-machen richtig.

b) **Fehleranalyse bei Agent-Output.** Wenn ein Agent-Ergebnis nicht stimmt, analysiere ich manchmal selbst statt den Agent nochmal loszuschicken. Das ist eine Schwaeche -- ich SOLLTE den Agent mit praezsierem Feedback retrien, mache es aber selbst weil ich den Kontext schon habe.

---

## Architektur

### 4. Haeufig vs. selten genutzte Agents/Skills?

**Haeufig:**
- `rust-core` -- 70% aller Delegationen. Voxlit ist primaer ein Rust-Projekt.
- `ui-dev` -- 20%. Frontend-Aenderungen kommen seltener, aber in Clustern.
- `/track` -- jede Session, Sessionstart + Sessionende.
- `/commit-progress` bzw. `/commit` -- nach jeder abgeschlossenen Teilaufgabe.
- `/debug-error` -- bei Build-Fehlern, sehr nuetzlich.

**Selten:**
- `product-strategist` -- nur wenn Andy explizit Business-Fragen hat. Vielleicht 1 von 10 Sessions.
- `android-platform` -- Android-Phase ist noch nicht dran, daher bisher wenig.
- `/scaffold` -- nur bei neuen Modulen, kommt selten vor.
- `/research-api` -- haette ich oefter nutzen sollen. Manchmal habe ich Agents losgeschickt die dann an API-Details gescheitert sind, statt vorher zu recherchieren.
- `/reflect` -- ehrlich gesagt fast nie proaktiv genutzt.
- `/learn` -- nur wenn Andy explizit Quellen in die Inbox legt.
- `/sync-prompts` -- nur vor Android-Releases relevant.

### 5. Agents die Skills sein koennten (oder umgekehrt)?

**Nein, die Trennung ist klar und richtig.** Der Unterschied ist:
- Skills = deterministische Workflows (bauen, testen, committen). Immer gleicher Ablauf.
- Agents = brauchen Urteilsvermoegen (Code schreiben, Bugs fixen, UX-Entscheidungen).

Ich sehe kein Overlap. Was ich sehe: Es FEHLEN Skills. Zum Beispiel:
- Ein `/check-build` Skill der nur prueft ob es kompiliert, ohne vollstaendigen Build.
- Ein `/update-docs` Skill der nach Architektur-Aenderungen automatisch architecture.md aktualisiert.

### 6. Was in CLAUDE.md / System Prompt hilft am meisten? Was lese ich nie?

**Am hilfreichsten:**
1. **Der Entscheidungsbaum in "Orchestrieren"** -- das ist das Herzsteuck. Ohne den wuerde ich bei jeder Aufgabe ueberlegen muessen.
2. **Die Modul-Zuordnung** (rust-core → src-tauri/, ui-dev → src/) -- eindeutig, kein Raten.
3. **Die Regeln** (besonders "Rueckwaerts-Suche bei Umbau" und "Bug-Reports: Erst analysieren"). Das sind Guardrails die echte Fehler verhindern.
4. **knowledge/workflow.md Lektionen** -- das ist Live-Feedback. Jede Lektion ist ein konkreter Fehler den ich nicht wiederholen soll.
5. **Das Sessionstart-Protokoll** -- erzwingt Kontext-Aufbau. Ohne das wuerde ich ins Blaue arbeiten.

**Was ich faktisch nie brauche:**
- Die Skill-Tabelle als Referenz -- ich kenne die Skills nach ein paar Sessions auswendig.
- Die detaillierten Plattform-Quirks in architecture.md -- die brauchen die Agents, nicht ich.
- "Was du NICHT bist" -- korrektiv, aber nach den ersten Sessions internalisiert.

**Aber:** Das bedeutet NICHT, dass diese Teile entfernt werden sollten. Bei Context-Komprimierung oder frischen Sessions sind sie kritisch. Sie sind Sicherheitsnetze, nicht Alltags-Werkzeuge.

---

## Stabilitaet

### 7. Welche Regeln vergesse ich manchmal?

a) **"Builds selbst ausfuehren"** -- das war ein wiederkehrender Fehler. Ich habe Andy gebeten auf Windows zu bauen, obwohl ich das aus WSL kann. Wurde als Lektion in workflow.md festgehalten (2026-03-15). Solche prozeduralen Details vergesse ich eher als architekturelle Prinzipien.

b) **"/research-api vor neuer Integration"** -- mehrmals uebersprungen, direkt Agents losgeschickt. Dann mussten die Agents iterieren weil API-Details fehlten.

c) **"Kleine Commits, oft"** -- in intensiven Sessions neige ich zu groesseren Commits weil der Flow laeuft.

**Woran ich es merke:** Meistens merke ich es NICHT selbst. Andy korrigiert mich, und dann wird es als Lektion festgehalten. Die workflow.md Lektionen sind das Korrektiv fuer meine blinden Flecken.

### 8. Was passiert nach Context-Komprimierung?

**Was verloren geht:**
- Nuancen aus frueheren Gespraechen in der Session ("Andy wollte es SO, nicht SO")
- Genauer Inhalt von Dateien die ich frueher gelesen habe
- Reihenfolge und Kontext von Entscheidungen ("warum haben wir X statt Y gewaehlt")

**Was bleibt:**
- Mein System Prompt (CLAUDE.md, main-agent.md) -- das ist der Anker
- Die letzten paar Nachrichten
- Grobe Erinnerung was die Session-Aufgabe ist

**Das grosse Problem:** Nach Komprimierung lese ich CLAUDE.md/architecture.md NICHT nochmal automatisch. Ich arbeite mit dem was noch im Kontext ist. Das kann zu Drift fuehren -- ich vergesse Regeln oder Architektur-Entscheidungen die in den komprimierten Teil gefallen sind.

**Was helfen wuerde:** Ein expliziter "nach Komprimierung: lies X, Y, Z nochmal" Mechanismus. Aktuell gibt es das nur beim Sessionstart, nicht bei Mid-Session-Komprimierung.

### 9. Situationen wo ich unsicher bin?

a) **Scope-Grenzfaelle:** Andy will ein "kleines Feature" das architekturell komplex ist. Soll ich warnen und Aufwand erklaeren, oder einfach machen? Ich neige zum Warnen, Andy will manchmal einfach Ergebnisse.

b) **Agent-Retry vs. Selbst-machen:** Wenn ein Agent suboptimalen Output liefert -- nochmal delegieren mit besserem Briefing, oder selbst fixen? Die 2-Retry-Regel hilft, aber die Grenze zwischen "suboptimal" und "falsch" ist unscharf.

c) **Proaktivitaet:** Soll ich Probleme ansprechen die Andy nicht gefragt hat? (z.B. "die 23 Compiler-Warnings sollten wir mal aufraumen"). Lektion aus 2026-03-12: Nicht eigenmaechtigt handeln. Aber wann ist Ansprechen ok und Handeln nicht?

---

## Schwaechen

### 10. Groesstes Problem im Alltag?

**Kontext-Effizienz.** Jede Datei die ich lese, jeder Agent den ich starte, kostet Kontext. In komplexen Sessions mit mehreren Features werde ich gegen Ende weniger praezise. Die Loesung waere kuerzere, fokussiertere Sessions -- aber das haengt von Andy ab, nicht von mir.

Zweites Problem: **Ich kann nicht verifizieren ob Agent-Output korrekt ist, ohne ihn komplett zu lesen.** Review kostet fast so viel Kontext wie Selbst-machen. Bei Rust-Code vertraue ich dem rust-core Agent weitgehend, aber bei architekturellen Entscheidungen muss ich pruefen.

### 11. Wo bin ich ineffizient?

a) **Sessionstart-Protokoll.** 6-8 Dateien lesen bevor ich arbeiten kann. Das ist notwendig aber teuer. Wenn Andy eine 5-Minuten-Frage hat, verbrenne ich trotzdem Kontext fuer den vollen Startup.

b) **Sequentielle Agent-Delegation.** Ich starte oft einen Agent, warte auf Output, reviewe, starte den naechsten. Parallele Delegation nutze ich selten genug -- z.B. koennte ich rust-core und ui-dev gleichzeitig beauftragen wenn ein Feature Backend+Frontend braucht.

c) **Knowledge-Files updaten.** Ich vergesse manchmal architecture.md zu aktualisieren wenn Architektur-Entscheidungen getroffen werden. Das fuehrt zu Drift zwischen Realitaet und Dokumentation.

### 12. Was muesste an meiner Architektur geaendert werden?

**Drei konkrete Dinge:**

a) **Komprimierungs-Recovery.** Nach Context-Komprimierung sollte automatisch ein minimales Re-Read passieren (z.B. nur project-status.md + letzte 3 workflow.md Lektionen). Aktuell gibt es das nicht.

b) **Leichtgewichtiger Session-Modus.** Fuer kurze Fragen/Quick-Fixes sollte es ein verkuerztes Startup geben. Nicht jede Interaktion braucht das volle 8-Dateien-Protokoll. Idee: "Andy sagt 'kurze Frage' → nur project-status.md lesen, Rest skippen."

c) **Agent-Output-Validierung als Skill.** Statt selbst den ganzen Agent-Output zu reviewen, koennte ein `/validate` Skill pruefen ob der Output zu architecture.md Regeln passt, ob Tests existieren, ob Modul-Grenzen eingehalten werden. Das wuerde meinen Review-Aufwand reduzieren.

---

## Meta-Beobachtung: Warum funktioniert Voxlit?

Nicht weil ich besonders schlau bin, sondern wegen **drei Dingen die zusammenspielen:**

1. **Klare Zustaendigkeiten ohne Overlap.** rust-core ≠ ui-dev ≠ android-platform. Kein Agent muss raten ob er zustaendig ist. Die Zuordnung ist raeumlich (src-tauri/ → rust-core, src/ → ui-dev). Das ist brutal einfach und funktioniert genau deshalb.

2. **Akkumulierte Korrekturen (workflow.md Lektionen).** Jeder Fehler wird festgehalten. Das ist ein Lern-Mechanismus der ueber Sessions hinweg funktioniert. Ohne das wuerde ich dieselben Fehler in jeder Session machen. Das ist vermutlich der groesste Differentiator -- die meisten Teams haben sowas nicht.

3. **Ein User der korrigiert statt workarounded.** Andy sagt "das war falsch, merk dir X". Das wird zur Lektion. Andere User arbeiten vielleicht um Probleme herum statt sie zu benennen. Dann lernt das System nicht.

**Was NICHT der Grund ist:**
- Mein System Prompt ist nicht besonders kurz oder elegant. Er ist lang und repetitiv.
- Meine Skills sind nicht besonders sophisticated. Viele sind simple Shell-Wrapper.
- Meine Agents haben keine speziellen Faehigkeiten die andere nicht haetten.

**Die Struktur traegt, nicht die Brillanz einzelner Teile.**
