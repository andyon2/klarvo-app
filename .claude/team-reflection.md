# Voxlit -- Team-Reflection

Stand: 2026-03-09

---

## 1. Wofuer bin ich zustaendig?

### Agents

| Agent | Was ich tue | Was ich NICHT tue |
|-------|-------------|------------------|
| **Tech Lead** (main-agent.md) | Architektur-Entscheidungen, Delegation, Review, Sessionstart/-ende-Protokoll, Kontext-Management, Skill-Auswahl, Dispatches pruefen | Code schreiben (ausser strategische 2-3-Nachrichten-Entscheidungen), Android debuggen, Frontend stylen, Rust compilieren |
| **rust-core** | Alles in `src-tauri/`: Audio-Capture (cpal), STT-Pipeline (Groq/OpenAI), LLM-Cleanup-Client, Text-Paste (Win32), Hotkey, Dictionary, Config, History, Sync | Android-Kotlin-Code, Frontend-React-Code, Build-Deployment |
| **ui-dev** | Alles in `src/`: FloatingBar, SettingsPanel, AdvancedSettingsPanel, MobileTextarea, VoiceNotesPanel, SnippetsPanel, Hooks, Tauri-IPC-Calls im Frontend, Onboarding | Rust-Backend, Android-Kotlin-Code, Build-Skripte |
| **android-platform** | Alles in `android/kotlin-src/`: VoxlitOverlayService, FloatingBubbleView, VoxlitAudioRecorder, VoxlitApi, VoxlitAccessibilityService, MainActivity; Android-Build-Workflow | Rust-Backend-Logik, React-Frontend-Code, Windows-spezifische Features |
| **product-strategist** | Positionierung, Monetarisierung, Roadmap-Priorisierung aus Marktsicht, Wettbewerbs-Strategie, Release-Scoping | Tech-Entscheidungen, Architektur, Code |

### Skills

| Skill | Was ich tue | Was ich NICHT tue |
|-------|-------------|------------------|
| `/scaffold` | Leeres Modul/Komponente aus Template anlegen (Rust-Modul, React-Component, Android-Service) | Implementieren -- nur Boilerplate |
| `/build` | `scripts/sync-and-build.ps1` (Windows) oder `scripts/android-build.sh` (Android) ausfuehren, Fehler strukturiert melden | Fehler beheben -- nur diagnostizieren |
| `/run-tests` | `cargo test` und `npm test` ausfuehren, Report formatieren | Tests schreiben oder Fehler fixen |
| `/research-api` | Docs recherchieren (WebSearch + WebFetch), Summary in `knowledge/` schreiben | Implementieren was recherchiert wurde |
| `/lint-fix` | `cargo fmt`, `clippy`, `prettier`, `eslint` ausfuehren, Auto-Fix wo moeglich | Code umstrukturieren -- nur Formatter-Level |
| `/plan-feature` | Feature in Tasks mit Agent-Zuweisung und Abhaengigkeiten zerlegen; optional `--save` in briefings/ | Tasks ausfuehren oder entscheiden ob der Plan gut ist |
| `/commit-progress` | `git status`, `git diff`, konventionellen Commit erstellen | Code-Review vor dem Commit, Tests pruefen |
| `/debug-error` | Fehler klassifizieren, Root Cause finden, Fix vorschlagen | Fix selbst implementieren -- nur analysieren |
| `/sync-prompts` | LLM-Prompts in `llm/mod.rs` (Rust) vs. `VoxlitApi.kt` (Kotlin) vergleichen, Drift sichtbar machen | Drift beheben -- nur diagnostizieren |
| `/release` | Version bump in 3 Dateien, beide Plattformen bauen, `latest.json` generieren, GitHub Release erstellen | Post-Release-Marketing, App-Store-Publishing |
| `/track` | `project-status.md` nach Session aktualisieren, Karteileichen bereinigen, max 50 Zeilen halten | Ausfuehrliche Dokumentation, Code-Aenderungen |
| `/reflect` (dieses Skill) | Team-Inventur, Schwachstellen-Analyse, Verbesserungsvorschlaege, schreibt nach `.claude/team-reflection.md` | Aenderungen selbst durchfuehren |

---

## 2. Synchronisation

### Drei-Quellen-Hierarchie

```
project-status.md     -- "Was ist JETZT los?"  (kurzlebig, session-granular)
knowledge/            -- "Was haben wir gelernt?" (stabil, wachsend)
CLAUDE.md / Agents    -- "Wer tut was wie?" (selten geaendert, strukturgebend)
```

### Konkrete Trigger

| Trigger | Aktion | Automatisch? |
|---------|--------|-------------|
| Sessionstart | Tech Lead liest `project-status.md` + `knowledge/architecture.md` | Manuell (via System-Prompt-Instruktion) |
| Sessionstart | Tech Lead prueft `dispatches/inbox/` auf neue Dispatch-Dateien | Manuell (via System-Prompt-Instruktion) |
| Feature abgeschlossen | `/track` ausfuehren: `project-status.md` aktualisieren | NEIN -- manuell per Konvention, kann vergessen werden |
| Architektur-Entscheidung getroffen | `knowledge/architecture.md` aktualisieren | NEIN -- manuell, Disziplinfrage |
| API-Research | Summary in `knowledge/api-providers.md` oder `architecture.md` | Durch `/research-api` Skill semi-automatisch |
| Agent fertig | Tech Lead reviewt, gibt OK oder fordert Korrekturen | Manuell |
| Kotlin-Code geaendert | `scripts/android-build.sh` kopiert nach `gen/android/` | Semi-automatisch -- nur wenn Script genutzt wird |
| LLM-Prompts geaendert | `/sync-prompts` ausfuehren, beide Dateien manuell synchronisieren | NEIN -- kein Mechanismus zwingt dazu |

### Was NICHT automatisch synchronisiert wird

1. **LLM-Prompts in Rust vs. Kotlin** -- `llm/mod.rs` und `VoxlitApi.kt` enthalten denselben Prompt-Code. `/sync-prompts` macht Drift sichtbar, aber es gibt keine automatische Absicherung. In MEMORY.md explizit als wiederkehrende Falle dokumentiert.

2. **Agent-Wissen nach Architektur-Aenderungen** -- Agents lesen Projektdateien erst beim naechsten Auftrag. Es gibt keine Push-Benachrichtigung.

3. **`/reflect` und `/track` sind nicht in CLAUDE.md Skill-Tabelle registriert** -- `/sync-prompts`, `/release` und `/reflect` fehlen in der Skill-Tabelle in `CLAUDE.md`. Wer nur `CLAUDE.md` liest (z.B. ein neuer Assistent), sieht diese Skills nicht. Nur `main-agent.md` hat die vollstaendige Skill-Tabelle.

---

## 3. Ziele und Organisation

### Kurz-/mittelfristige Ziele (aus project-status.md, Stand 2026-03-09)

Aktueller Stand: Version 0.4.1, Windows + Android signiert, GitHub Release v0.4.1, Auto-Update-Infrastruktur steht.

**Priorisierte naechste Schritte:**
1. License-Key-System (Open Core: Free vs. Paid EUR 29) -- kein Briefing existiert noch
2. Offline whisper.cpp Fallback -- Briefing existiert (`briefings/plan-offline-whisper.md`)
3. Onboarding/Polish -- kein Briefing existiert noch
4. Bubble Size/Opacity Controls (Presets) -- Briefing existiert (`briefings/plan-bubble-appearance.md`)

**Backlog (niedrig priorisiert):**
- VAD (Voice Activity Detection)
- Notion/Todoist Integrationen
- GitHub CI/CD Pipeline

### Wie organisiert sich das Team dafuer?

Reaktiv, feature-by-feature. Ein Feature wird besprochen, an den passenden Agent delegiert, committet. Es gibt keine Sprint-Planung, keine Meilensteine. Der Backlog ist eine flat list ohne Gewichtung oder Abhaengigkeits-Graph.

Fuer die naechste Business-kritische Phase (License-Key-System vor erstem Paid Release) fehlt ein vollstaendiger Plan. Das Briefing wurde noch nicht erstellt -- das ist eine echte Luecke zwischen Strategie und Ausfuehrung.

---

## 4. Schwachstellen und Luecken

### 4.1 CLAUDE.md Skill-Tabelle ist unvollstaendig

**Problem:** `CLAUDE.md` listet 8 Skills in der Tabelle: `/scaffold`, `/build`, `/run-tests`, `/research-api`, `/lint-fix`, `/plan-feature`, `/commit-progress`, `/debug-error`. Tatsaechlich existieren 12 Skills: zusaetzlich `/sync-prompts`, `/release`, `/track`, `/reflect`. Diese vier fehlen in der CLAUDE.md Skill-Tabelle komplett.

**Auswirkung:** Wer CLAUDE.md als Orientierung nutzt (neue Assistenten, Andy wenn er nachschaut), sieht ein unvollstaendiges Bild. `/track` ist besonders kritisch -- es ist der Sessionende-Skill der in main-agent.md explizit genutzt wird, fehlt aber in CLAUDE.md.

**Moegliche Loesung:** CLAUDE.md Skill-Tabelle um die vier fehlenden Skills ergaenzen.

### 4.2 /commit-progress hat nur Bash-Tool, macht kein Review

**Problem:** Der `/commit-progress`-Skill staged und committet ohne zu pruefen ob Linter/Tests gruen sind. Tool-Liste: nur `Bash`. Er kann keine Dateien lesen, keine Patterns suchen, keinen Code-Review machen.

**Auswirkung:** Defekter Code kann committed werden. Im Soloprojekt kein kritisches Problem -- aber der Skill hat keine Defense gegen "vergessen zu testen".

**Moegliche Loesung:** Explizit in der Skill-Beschreibung dokumentieren: "User soll vor dem Aufruf selbst /run-tests und /lint-fix ausfuehren." Oder: `/commit-progress` ruft optional `/run-tests` vor dem Commit auf.

### 4.3 Kein rust-core Direkt-Session-Starter

**Problem:** Fuer Android und Product-Strategist gibt es direkte Session-Starter (`scripts/android-platform`, `scripts/product-strategist`). Fuer Windows-spezifische Rust-Arbeit (Win32 Paste-Debug, Hotkey-Probleme, Audio-Pipeline-Tuning) gibt es nur den normalen Tech-Lead-Workflow. Der Tech Lead muss alles im Haupt-Kontextfenster halten.

**Auswirkung:** Windows-Rust-Debugging verbraucht Tech-Lead-Kontext, widerspricht dem "Kontext schuetzen"-Prinzip aus main-agent.md. Beim naechsten grossen Rust-Debugging (z.B. whisper.cpp Integration) wird das spaetestens spaerbar.

**Moegliche Loesung:** `scripts/rust-core` Starter analog zu `scripts/android-platform` erstellen.

### 4.4 License-Key-Briefing fehlt, obwohl erste Prioritaet

**Problem:** `project-status.md` listet das License-Key-System als erste Prioritaet ("Muss vor erstem Paid Release stehen"). Das Briefing (`briefings/plan-license-key.md`) existiert jedoch nicht. Die Planung fuer das kritischste naechste Feature wurde nicht gemacht.

**Auswirkung:** In der naechsten Session muss die Planung on-the-fly stattfinden, ohne vorbereitetem Kontext. Das kostet Zeit und Kontext, die vermeidbar waerenwaeren.

**Moegliche Loesung:** `/plan-feature license key system --save` ausfuehren um Briefing zu erstellen. Oder: product-strategist direct session fuer Requirements, dann Tech Lead fuer technischen Plan.

### 4.5 product-strategist fehlt in CLAUDE.md Direkt-Modus-Beschreibung

**Problem:** CLAUDE.md listet in der Agenten-Tabelle `product-strategist` mit "delegiert + direkt" -- aber die Tabelle in main-agent.md (die der Tech Lead liest) hat dieselbe Abkuerzung. Kein Ort beschreibt explizit den Briefing-Pfad fuer product-strategist analog zu android-platform. android-platform.md hat einen expliziten Abschnitt "Interaktionsmodi" -- product-strategist.md hat diesen Abschnitt auch, aber die Integration in den Tech-Lead-Workflow ist nicht symmetrisch zur Android-Seite.

**Auswirkung:** Geringes Risiko. Der Prozess funktioniert -- aber der Onboarding-Overhead ist etwas hoeher.

### 4.6 /release Skill kann nicht fehlschlagen ohne kompletten Abbruch zu triggern

**Problem:** Der `/release`-Skill baut Windows UND Android sequentiell. Wenn Windows erfolgreich ist und Android fehlschlaegt (oder umgekehrt), gibt es keinen dokumentierten Recovery-Pfad. Der Skill sagt "Falls ein Build fehlschlaegt: Abbrechen" -- aber der Windows-Build hat dann bereits Artefakte erzeugt und die Version wurde bereits gebumpt.

**Auswirkung:** Partieller Release-Zustand: Version gebumpt, ein Build fertig, kein GitHub Release. Recovery ist manuell und undokumentiert.

**Moegliche Loesung:** Im Skill dokumentieren: "Wenn ein Build fehlschlaegt, versionsbump revertern und Status melden." Oder: Beide Builds vor dem Version-Bump ausfuehren.

### 4.7 Keine strukturierte Kostentransparenz fuer API-Nutzung

**Problem:** Das Projekt nutzt bezahlte APIs (Groq, DeepSeek). Usage wird in SQLite History gespeichert -- aber es gibt keinen Tauri-Command, Skill oder Agent der Kosten-Estimation liefert.

**Auswirkung:** Kein Ueberschreiten eines Budget-Limits merkbar ausser via API-Dashboard. Fuer ein Produkt das in Richtung kommerzieller Nutzung geht (Open Core) ist das relevant: Wenn Nutzer auf eigene API-Keys angewiesen sind, sollten sie wissen was sie ausgeben.

**Moegliche Loesung:** Token-Count und Kosten-Estimation in `get_usage_stats` Command ergaenzen. Niedrige Prioritaet bis erstes Paid Release.

---

## 5. Ueberschneidungen und Ineffizienzen

### 5.1 `/build` und `/debug-error` ueberlappen bei Build-Fehlern

`/build` gibt bei Fehler bereits strukturierte Analyse mit "Wahrscheinliche Ursache" aus. `/debug-error` macht dasselbe gruendlicher. In der Praxis: Wer `/build` nutzt und einen Fehler bekommt, ruft danach haeufig `/debug-error` mit demselben Error-Output auf. Das ist ein manueller Two-Step.

**Status:** Bekannt, akzeptiert. Die Trennung (diagnostizieren vs. implementieren) ist sinnvoll. Overhead ist gering.

### 5.2 `/scaffold` und Agent-Direkt-Implementation

`/scaffold` erzeugt leere Templates. Ein direkt beauftragter Agent wuerde das Template als ersten Schritt selbst anlegen. `/scaffold` hat Wert wenn: standardisiertes Template-Format erzwungen werden soll, oder Placeholder ohne Implementierungsauftrag gebraucht wird.

**Status:** Selten genutzter Skill. Kein Quick-Win bei der Bereinigung -- der Overhead ist gering. Behalten als optionales Tool.

### 5.3 CLAUDE.md Skill-Tabelle vs. main-agent.md Skill-Tabelle

Beide Dateien haben eine Skill-Tabelle. `main-agent.md` hat die vollstaendige mit "Wann nutzen"-Spalte. `CLAUDE.md` hat eine gekuerzte Version, die jetzt auch veraltet ist (4 Skills fehlen). Bei jeder Skill-Aenderung muss man in zwei Dateien aktualisieren.

**Vorschlag:** CLAUDE.md nur noch auf main-agent.md verweisen fuer die vollstaendige Skill-Liste. Oder: CLAUDE.md Tabelle automatisch generieren statt manuell pflegen.

### 5.4 Drei Dateien beschreiben das Team, nicht eine

Team-Struktur steht in: `CLAUDE.md` (Tabellen), `main-agent.md` (ausfuehrliche Orchestrierungslogik), `agents/*.md` (individuelle Agent-Prompts). Ein Aussenstehender muss alle drei lesen um das Team zu verstehen.

**Status:** Akzeptierbar. Die drei Schichten haben verschiedene Zwecke (User-Ueberblick, Tech-Lead-Instruktion, Agent-Instruktion). Keine Redundanz eliminieren -- nur konsistent halten.

---

## 6. Streamlining-Vorschlaege

### Sofort umsetzbar (Quick Wins)

1. **CLAUDE.md Skill-Tabelle um 4 fehlende Skills ergaenzen** -- `/sync-prompts`, `/release`, `/track`, `/reflect` fehlen in der Tabelle. 5 Minuten Arbeit, behebt eine echte Informations-Luecke.

2. **License-Key-Briefing erstellen** -- `/plan-feature license key system --save` ausfuehren oder tech lead direkt beauftragen. Das kritischste naechste Feature hat kein Briefing.

3. **`/commit-progress` Nutzungs-Hinweis ergaenzen** -- Explizit in der Skill-Beschreibung: "Soll erst nach /run-tests und /lint-fix aufgerufen werden." Keine technische Aenderung, nur dokumentarische Klarheit.

### Mittelfristig

4. **`scripts/rust-core` Starter erstellen** -- Analoges Skript zu `scripts/android-platform`. Wichtig vor der whisper.cpp Integration (Offline-Fallback) -- das wird iteratives Rust-Debugging erfordern. Ohne direkten Starter landet alles im Tech-Lead-Kontext.

5. **`/release` Recovery-Pfad dokumentieren** -- Was tun wenn ein Build im mehrstufigen Release-Prozess fehlschlaegt? Versionsbump revertern? Partial-Release als Issue? Konkrete Anleitung im Skill-Prompt ergaenzen.

6. **Backlog in project-status.md mit Abhaengigkeiten versehen** -- VAD und whisper.cpp sind technische Abhaengigkeiten voneinander (whisper.cpp sollte vor VAD kommen). Notion/Todoist Integrationen sind erst relevant nach License-Key-System. Eine simple Reihenfolge-Angabe spart Diskussionszeit.

---

## 7. Bewusst beibehalten

**Drei-Quellen-Wissenshierarchie (`project-status.md` / `knowledge/` / `CLAUDE.md`+Agents)**
Sinnvoll. Die Trennung zwischen kurzlebigem Projektstatus, stabilem Wissen und Teamstruktur funktioniert gut. Keine Zusammenfuehrung empfohlen.

**android-platform Direkt-Session-Modus mit Briefing-System**
Sinnvoll. Android-Entwicklung erfordert echten iterativen Dialog. Der Briefing-Pfad (`briefings/android-platform-*.md`) und der Script-Starter sind das richtige Pattern. Es sollte auf rust-core ausgedehnt werden (Quick Win 4).

**Skills als Haiku (operationell) vs. Agents als Sonnet (urteilend)**
Sinnvoll. Die Modell-Differenzierung (Haiku fuer build/lint/test/scaffold, Sonnet fuer research/plan/debug/agents) ist kosteneffizient. Haiku-Skills fuer deterministische Tasks, Sonnet fuer Urteilsvermoegen.

**`/debug-error` fixt nicht selbst, schlaegt nur Fix vor**
Sinnvoll. Analysieren und Implementieren trennen schuetzt vor voreiligen Fixes. Der Tech Lead entscheidet nach der Analyse.

**`/sync-prompts` als reines Diagnose-Tool**
Sinnvoll. Prompt-Sync automatisch zu machen (z.B. via Pre-Commit-Hook) waere fragil. Sichtbarmachen und manuell entscheiden ist das richtige Pattern fuer Prompt-Engineering.

**android-platform.md IME als "EVALUIERT, VERWORFEN" markiert**
Gut geloest. Der Abschnitt bleibt als Entscheidungsdokumentation (warum nicht), ist aber klar als verworfen markiert.

**Native Kotlin VoxlitApi statt Tauri-Bridge**
Richtige Entscheidung beibehalten. Weniger Latenz, direkter HTTP-Stack. Trade-off (Prompt-Duplikation) ist akzeptiert und durch `/sync-prompts` adressiert.

---

## Anhang: Dateistruktur

```
voxlit/
  CLAUDE.md                          -- Projekt-Ueberblick, Regeln, Team-Tabelle [SKILL-TABELLE UNVOLLSTAENDIG -- 4 Skills fehlen]
  main-agent.md                      -- Tech-Lead System-Prompt [VOLLSTAENDIG, aktuell]
  project-status.md                  -- Projektstatus, Backlog, Session-Changelog [AKTUELL, v0.4.1]
  .claude/
    settings.json                    -- Tool-Permissions
    settings.local.json              -- Lokale Overrides
    team-reflection.md               -- Diese Datei [AKTUELL, 2026-03-09]
    agents/
      rust-core.md                   -- Rust-Backend-Agent [AKTUELL]
      ui-dev.md                      -- Frontend-Agent [AKTUELL -- Komponenten-Liste korrigiert]
      android-platform.md            -- Android-Agent [AKTUELL -- IME als verworfen markiert]
      product-strategist.md          -- Strategie-Agent [AKTUELL]
    skills/
      scaffold/SKILL.md              -- Modul-Templates [OK, selten genutzt]
      build-app/SKILL.md             -- Build-Wrapper [KORRIGIERT -- nutzt jetzt android-build.sh]
      run-tests/SKILL.md             -- Test-Runner [OK]
      research-api/SKILL.md          -- API-Recherche + Knowledge-Schreiber [OK]
      lint-fix/SKILL.md              -- Linter/Formatter [OK]
      plan-feature/SKILL.md          -- Feature-Planer, --save persistiert in briefings/ [OK]
      commit-progress/SKILL.md       -- Git-Commit [LUECKE: kein Pre-Commit-Check]
      debug-error/SKILL.md           -- Fehler-Analyse [OK]
      sync-prompts/SKILL.md          -- Prompt-Drift-Detektion [OK, fehlt in CLAUDE.md Tabelle]
      release/SKILL.md               -- Version + Build + GitHub Release [OK, fehlt in CLAUDE.md Tabelle]
      track/SKILL.md                 -- Status-Update [OK, fehlt in CLAUDE.md Tabelle]
      reflect/SKILL.md               -- Team-Reflection [OK, fehlt in CLAUDE.md Tabelle]
    worktrees/                       -- Git-Worktrees fuer Agent-Sessions
  scripts/
    android-build.sh                 -- Kotlin-Copy + Build + Sign + Dropbox-Deploy [KRITISCH]
    android-platform                 -- Direkter Android-Session-Starter [OK]
    voxlit-tech-lead                  -- Direkter Tech-Lead-Starter [OK]
    product-strategist               -- Direkter Strategie-Session-Starter [OK]
    sync-and-build.ps1               -- Windows-Build via PowerShell [OK]
    [FEHLT: rust-core]               -- Direkter Rust-Session-Starter [LUECKE]
  briefings/
    android-platform-research.md    -- Tauri-Android-Setup-Recherche [EXISTIERT]
    plan-signing-auto-update.md      -- Signing + Auto-Update Plan [EXISTIERT, ERLEDIGT]
    plan-offline-whisper.md          -- Offline Whisper Plan [EXISTIERT, naechste Session]
    plan-bubble-appearance.md        -- Bubble Presets Plan [EXISTIERT]
    [FEHLT: plan-license-key.md]     -- License-Key-System Plan [LUECKE -- erste Prioritaet]
  knowledge/
    architecture.md                  -- Tech-Entscheidungen, Plattform-Split [AKTUELL]
    api-providers.md                 -- Groq + DeepSeek API-Details [AKTUELL]
    competitors.md                   -- Konkurrenz-Analyse [EXISTIERT]
    product-strategy.md              -- Positionierung, Pricing, Differenzierung [EXISTIERT]
    wispr-flow-android-ux.md         -- Wispr-Flow-UX-Recherche [EXISTIERT]
  src-tauri/                         -- Rust-Backend [AKTUELL]
  src/                               -- React-Frontend [AKTUELL]
  android/kotlin-src/                -- Kotlin-Quellen [AKTUELL, via android-build.sh deployt]
```
