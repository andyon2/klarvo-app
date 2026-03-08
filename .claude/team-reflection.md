# Dikta -- Team-Reflection

Stand: 2026-03-08

---

## 1. Wofuer bin ich zustaendig?

### Agents

| Agent | Was ich tue | Was ich NICHT tue |
|-------|-------------|------------------|
| **Tech Lead** (main-agent.md) | Architektur-Entscheidungen, Delegation, Review, Sessionstart/-ende-Protokoll, Kontext-Management, Skill-Auswahl | Code schreiben (ausser strategische 2-3-Nachrichten-Entscheidungen), Android debuggen, Frontend stylen, Rust compilieren |
| **rust-core** | Alles in `src-tauri/`: Audio-Capture (cpal), STT-Pipeline (Groq/OpenAI), LLM-Cleanup-Client, Text-Paste (Win32), Hotkey, Dictionary, Config, History, Sync | Android-Kotlin-Code, Frontend-React-Code, Build-Deployment |
| **ui-dev** | Alles in `src/`: FloatingBar, SettingsPanel, AdvancedSettings, MobileTextarea, VoiceNotes, Hooks, Tauri-IPC-Calls im Frontend | Rust-Backend, Android-Kotlin-Code, Build-Skripte |
| **android-platform** | Alles in `android/kotlin-src/`: DiktaOverlayService, FloatingBubbleView, DiktaAudioRecorder, DiktaApi, DiktaAccessibilityService, MainActivity; Android-Build-Workflow | Rust-Backend-Logik, React-Frontend-Code, Windows-spezifische Features |

### Skills

| Skill | Was ich tue | Was ich NICHT tue |
|-------|-------------|------------------|
| `/scaffold` | Leeres Modul/Komponente aus Template anlegen (Rust-Modul, React-Component, Android-Service) | Implementieren -- nur Boilerplate |
| `/build` | `tauri dev` oder `tauri android build` ausfuehren, Fehler strukturiert melden | Fehler beheben -- nur diagnostizieren |
| `/run-tests` | `cargo test` und `npm test` ausfuehren, Report formatieren | Tests schreiben oder Fehler fixen |
| `/research-api` | Docs recherchieren (WebSearch + WebFetch), Summary in `knowledge/` schreiben | Implementieren was recherchiert wurde |
| `/lint-fix` | `cargo fmt`, `clippy`, `prettier`, `eslint` ausfuehren, Auto-Fix wo moeglich | Code umstrukturieren -- nur Formatter-Level |
| `/plan-feature` | Feature in Tasks mit Agent-Zuweisung und Abhaengigkeiten zerlegen | Tasks ausfuehren oder entscheiden ob der Plan gut ist |
| `/commit-progress` | `git status`, `git diff`, konventionellen Commit erstellen | Code-Review vor dem Commit |
| `/debug-error` | Fehler klassifizieren, Root Cause finden, Fix vorschlagen | Fix selbst implementieren -- nur analysieren |
| `/reflect` (dieses Skill) | Team-Inventur, Schwachstellen-Analyse, Verbesserungsvorschlaege | Aenderungen selbst durchfuehren |

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
| Feature abgeschlossen | `project-status.md` aktualisieren, abgehakte Tasks loeschen | NEIN -- manuell, oft vergessen |
| Architektur-Entscheidung getroffen | `knowledge/architecture.md` aktualisieren | NEIN -- manuell, Disziplinfrage |
| Android-Quirk entdeckt | `knowledge/platform-notes.md` ergaenzen | NEIN -- manuell, bei android-platform-Sessions oft vergessen |
| API-Research | Summary in `knowledge/api-providers.md` oder `architecture.md` | Durch `/research-api` Skill semi-automatisch |
| Agent fertig | Tech Lead reviewt, gibt OK oder fordert Korrekturen | Manuell |
| Kotlin-Code geaendert | `scripts/android-build.sh` kopiert nach `gen/android/` | Semi-automatisch -- nur wenn Script genutzt wird |

### Was NICHT automatisch synchronisiert wird

1. **LLM-Prompts in Rust vs. Kotlin** -- `llm/mod.rs` und `DiktaApi.kt` enthalten denselben Prompt-Code. Kein Mechanismus sichert, dass beide synchron bleiben. Dieser Punkt ist in `architecture.md` als bekannter Trade-off dokumentiert, aber es gibt keine technische Absicherung.

2. **Agent-Wissen nach Architektur-Aenderungen** -- Wenn sich die Architektur aendert (z.B. "IME wurde durch Bubble ersetzt"), lesen delegierte Agents die aktuellen Projektdateien erst beim naechsten Auftrag. Es gibt keine Push-Benachrichtigung an Agents.

3. **platform-notes.md existiert nicht mehr** -- Die Datei ist in `CLAUDE.md`, `android-platform.md` und `rust-core.md` als Ziel fuer Plattform-Quirks referenziert. Sie existiert nicht (`ls knowledge/` zeigt: api-providers.md, architecture.md, competitors.md, wispr-flow-android-ux.md). Das ist eine tote Referenz in drei Dateien gleichzeitig.

4. **ui-dev.md referenziert Komponenten, die nicht existieren** -- `Overlay.tsx`, `Dictionary.tsx`, `StylePicker.tsx` stehen im ui-dev Agent-Prompt als Kern-Komponenten. Die tatsaechlichen Dateien heissen `FloatingBar.tsx`, `SettingsPanel.tsx` etc. Die Agent-Beschreibung ist mehrere Entwicklungsphasen alt.

---

## 3. Ziele und Organisation

### Kurz-/mittelfristige Ziele (aus project-status.md)

**Bekannte Bugs:**
- Windows Signing Keys nicht generiert (Warnung bei jedem Build)

**Backlog (kein priorisierter Zeitrahmen):**
- Android: Bubble Size/Opacity Controls (wegen Slider-Bug verschoben)
- Shared: Lokaler whisper.cpp Fallback (offline STT)
- Shared: Voice Activity Detection (Auto-Start/Stopp)
- Windows: Signing Keys + GitHub CI/CD fuer Auto-Update
- Shared: Integrationen (Notion, Todoist -- Platzhalter existiert)

### Langfristige Ziele (aus CLAUDE.md)

- Vollstaendige Wispr-Flow-Alternative
- Kein Abo, keine Cloud-Abhaengigkeit (offline-first via whisper.cpp)
- Sowohl Windows als auch Android

### Wie organisiert sich das Team dafuer?

Derzeit: Feature-by-Feature, reaktiv. Ein Feature wird besprochen, an den passenden Agent delegiert, committet. Es gibt keine Sprint-Planung, keine Priorisierungs-Sessions, keinen Meilenstein-Begriff.

Der Backlog ist eine flache Liste ohne Prioritaet, Abhaengigkeiten oder Zeitrahmen. Das ist fuer ein Soloprojekt okay -- aber wenn mehrere grosse Backlog-Items gleichzeitig angegangen werden, fehlt ein Koordinationsrahmen.

---

## 4. Schwachstellen und Luecken

### 4.1 Tote Referenz: platform-notes.md

**Problem:** `rust-core.md`, `android-platform.md` und `briefings/android-platform-research.md` referenzieren `knowledge/platform-notes.md`. Die Datei existiert nicht.

**Auswirkung:** Agents bekommen beim Lesen des Kontexts eine Lese-Fehlermeldung, oder sie ignorieren den Verweis. Plattform-Quirks, die eigentlich in dieser Datei landen sollten, landen nirgendwo -- oder in `architecture.md`, was dieses Dokument aufblaehlt.

**Moegliche Loesung:** Entweder `platform-notes.md` erstellen (mit dem Android-spezifischen Inhalt, der aktuell in `architecture.md` unter "Plattform-Quirks" steht), oder alle Referenzen auf `architecture.md` umbiegen.

### 4.2 ui-dev.md veraltet

**Problem:** Der ui-dev Agent beschreibt Komponenten (`Overlay.tsx`, `Settings.tsx`, `Dictionary.tsx`, `StylePicker.tsx`), die entweder nicht existieren oder anders heissen (`FloatingBar.tsx`, `SettingsPanel.tsx`, etc.). Er hat keine Kenntnis von `MobileTextarea`, `VoiceNotesPanel`, `usePanels`, `useRecording`.

**Auswirkung:** Wenn ui-dev beauftragt wird, orientiert er sich an einem falschen Datei-Modell. Er liest zwar tatsaechliche Dateien via Glob/Grep, aber seine interne Vorstellung ("Kern-Komponenten") stimmt nicht mit der Realitaet. Das fuehrt zu suboptimalen Entscheidungen ueber Komponenten-Grenzen.

**Moegliche Loesung:** ui-dev.md Kern-Komponenten-Liste an den tatsaechlichen `src/`-Baum angleichen.

### 4.3 android-platform.md beschreibt IME -- Projekt hat Bubble

**Problem:** Der android-platform Agent-Prompt widmet ~60% seines Inhalts dem InputMethodService (IME/Keyboard-Ansatz). Das Projekt hat diesen Ansatz zugunsten der Floating Bubble verworfen (Entscheidung: 2026-03-08). Die IME-Code-Beispiele im Agent sind Dead Code in der Beschreibung.

**Auswirkung:** Beim naechsten Android-Feature koennte der Agent irrtuemlicherweise einen IME-Ansatz vorschlagen, weil sein System-Prompt das als "das Herzstueck" bezeichnet. Das Risiko ist real bei komplexen Features (z.B. "Wie integrieren wir X in die Android-App?").

**Moegliche Loesung:** android-platform.md auf Bubble-Architektur aktualisieren. IME-Abschnitt deutlich kuerzen oder als "evaluiert, verworfen" markieren.

### 4.4 Prompt-Duplikation Rust/Kotlin ohne technische Absicherung

**Problem:** LLM-Cleanup-Prompts existieren in `src-tauri/src/llm/mod.rs` (Rust) und `android/kotlin-src/com/dikta/voice/DiktaApi.kt` (Kotlin). Bei jeder Prompt-Aenderung muessen beide manuell synchron gehalten werden. Das ist als bekannter Trade-off in architecture.md dokumentiert -- aber ohne Absicherung.

**Auswirkung:** In der letzten Session war "LLM Prompts dupliziert: Rust UND Kotlin -- bei Aenderungen BEIDE updaten!" bereits in MEMORY.md als wiederkehrende Falle. Das zeigt, dass dieser Fehler schon passiert ist.

**Moegliche Loesung:** Entweder ein Skill `/sync-prompts` der beide Dateien vergleicht und Diff anzeigt, oder die Prompts in eine gemeinsame JSON-Datei auslagern, die beide Seiten lesen.

### 4.5 /build Skill ist zu simpel fuer die tatsaechliche Build-Komplexitaet

**Problem:** Der `/build`-Skill kennt nur `npm run tauri build`. Der tatsaechliche Build-Workflow ist:
- Windows: PowerShell-Skript via WSL2 (`sync-and-build.ps1`), nicht direkt `tauri build`
- Android: `scripts/android-build.sh` (kopiert Kotlin-Quellen, signiert, deployt nach Dropbox)
- Direct `tauri android build` ohne das Script baut ohne die Kotlin-Quellen -- und das war ~8 fehlgeschlagene Debug-Versuche wert (laut MEMORY.md)

**Auswirkung:** Wer `/build android` nutzt, laeuft geradewegs in den bekannten Kotlin-Quelle-fehlen-Bug. Der Skill ist nicht falsch -- er ist gefaehrlich einfach.

**Moegliche Loesung:** `/build`-Skill auf die tatsaechlichen Build-Skripte umbiegen. `android` -> `scripts/android-build.sh`, `windows` -> `scripts/sync-and-build.ps1`.

### 4.6 /commit-progress macht kein Review vor dem Commit

**Problem:** Der `/commit-progress`-Skill staged und committet ohne zu pruefen ob Linter/Tests gruen sind. Er hat nur `Bash`-Tool-Zugriff (kein Read, kein Grep), kann also auch keine Dateien auf offensichtliche Fehler pruefen.

**Auswirkung:** Defekter Code kann committed werden. In der Praxis bei einem Soloprojekt mit regelmaessigen Commits kein kritisches Problem -- aber es widerspricht dem Anspruch aus CLAUDE.md ("Jedes neue Modul bekommt Tests. Kein Modul ohne mindestens einen Basis-Test.").

**Moegliche Loesung:** `/commit-progress` koennte optional `/run-tests` und `/lint-fix` vorschalten. Oder: Explizit dokumentieren dass der User selbst testen soll bevor er `/commit-progress` ausfuehrt.

### 4.7 Kein Session-Briefing-Mechanismus fuer Windows-Arbeit

**Problem:** Fuer Android existiert ein ausgereifter Direkt-Session-Mechanismus: `scripts/android-platform` starter, Briefings in `briefings/`, Hin-und-Her-Dialog dokumentiert.

Fuer Windows-spezifische iterative Arbeit (z.B. Win32 Paste-Debug, Hotkey-Probleme) gibt es nur den normalen Tech-Lead-Workflow -- kein direkter rust-core-Starter, kein Briefing-Pfad. Der Tech Lead muss alles im Haupt-Kontextfenster halten.

**Auswirkung:** Windows-Debugging verbraucht Tech-Lead-Kontextfenster. Das widerspricht dem "Kontext schuetzen"-Prinzip aus main-agent.md.

**Moegliche Loesung:** `scripts/dikta-tech-lead` (existiert bereits, neu in diesem Commit-Stand) koennte ein Aequivalent zu `scripts/android-platform` sein. Ein `scripts/rust-core` Starter fuer direkte Rust-Sessions wuerde den Luecke schliessen.

### 4.8 Keine strukturierte Kostentransparenz

**Problem:** Das Projekt nutzt bezahlte APIs (Groq, DeepSeek). Die Nutzung wird in SQLite History gespeichert (Filler-Analysis, Stats) -- aber ob ein Dashboard oder Report fuer API-Kosten existiert, ist unklar.

**Auswirkung:** Kein Ueberschreiten eines Budget-Limits merkbar, ausser man schaut manuell in die API-Dashboards. Kein Skill, kein Agent, kein Command ist dafuer zustaendig.

**Moegliche Loesung:** Entweder Tauri-Command `get_usage_stats` erweitern um Token-Kosten-Estimation, oder Hinweis dass Kostenkontrolle Sache des Users via API-Dashboard ist.

---

## 5. Ueberschneidungen und Ineffizienzen

### 5.1 `/build` und `/debug-error` ueberlappen bei Build-Fehlern

Wenn `/build` fehlschlaegt, liefert es bereits eine strukturierte Fehleranalyse ("Wahrscheinliche Ursache"). `/debug-error` macht dasselbe gruendlicher. In der Praxis: Wer `/build` nutzt und einen Fehler bekommt, ruft danach `/debug-error` mit demselben Error-Output auf. Das ist ein manueller Two-Step-Workflow.

**Vorschlag:** `/build` koennte bei Fehler automatisch den Error-Output an `/debug-error` weitergeben (wenn das technisch moeglich ist). Oder `/build` Fehlermeldungen sind so detailliert, dass `/debug-error` nicht mehr noetig ist.

### 5.2 Tech Lead und android-platform kennen beide die Android-Architektur

Der Tech Lead hat android-relevante Architektur-Entscheidungen in main-agent.md (Android IME-Verweis, Direkt-Session-Empfehlung). Der android-platform Agent hat dasselbe Wissen aus seinen eigenen Dateien. Wenn Andy direkt im android-platform Starter arbeitet, hat der Agent vollstaendigen Kontext ohne den Tech Lead. Das ist wie gedacht -- aber der Tech Lead behaelt veraltetes Android-Wissen in seinem System-Prompt.

**Vorschlag:** main-agent.md koennte android-platform-spezifische Implementierungsdetails rausloesen und nur auf die Agent-Datei verweisen.

### 5.3 `/scaffold` und Agent-Direkt-Implementation ueberlappen

`/scaffold` erzeugt ein leeres Template. Der zustaendige Agent wuerde direkt mit der Implementierung beginnen, wenn er beauftragt wird. In der Praxis: Der Tech Lead koennte direkt den Agent beauftragen mit "Erstelle Modul X und implementiere Y", und der Agent legt das Template selbst an.

`/scaffold` hat also nur Wert wenn: (a) das Template-Format wirklich standardisiert ist und sich lohnt zu forcen, oder (b) wenn ohne Implementierungsauftrag ein Placeholder gebraucht wird. Fuer ein kleines Team mit einem Developer-per-Layer ist `/scaffold` wahrscheinlich selten genutztes Overhead.

### 5.4 `/plan-feature` gibt Plan aus, schreibt ihn nicht

Der Plan wird ausgegeben ("nicht in Datei schreiben -- der Main-Agent entscheidet"), aber wenn der Tech Lead den Plan gut findet, gibt es keinen einfachen Weg ihn zu persistieren. Er landet im Chat und verschwindet. Wenn in der naechsten Session danach gefragt wird, ist er weg.

**Vorschlag:** `/plan-feature` koennte optional in `briefings/plan-[feature].md` schreiben, mit explizitem Flag. Oder der Tech Lead persistiert ihn manuell -- was aber selten passiert.

---

## 6. Streamlining-Vorschlaege

### Sofort umsetzbar (Quick Wins)

1. **`knowledge/platform-notes.md` erstellen** -- Entweder neudatei mit Android-Quirks (aus architecture.md "Plattform-Quirks"-Abschnitt hierher verschieben), oder alle drei Referenzen auf architecture.md umbiegen. Beseitigt tote Referenz in drei Dateien.

2. **`ui-dev.md` Kern-Komponenten-Liste aktualisieren** -- Tatysaechliche Dateien aus `src/` listen: FloatingBar, SettingsPanel, AdvancedSettingsPanel, MobileTextarea, VoiceNotesPanel, hooks/. Dauert 5 Minuten, spart dem Agent falsche Orientierung.

3. **`android-platform.md` IME-Abschnitt markieren** -- Den IME-Abschnitt mit einem Kommentar versehen: "EVALUIERT, VERWORFEN (2026-03-08). Aktueller Ansatz: Floating Bubble via DiktaOverlayService." Kein Loeschen, nur Kontext geben.

4. **`/build`-Skill auf tatsaechliche Skripte umbiegen** -- `android` -> `scripts/android-build.sh` statt direktem `tauri android build`. `windows` -> `scripts/sync-and-build.ps1`. Verhindert den bekannten Kotlin-fehlen-Bug.

5. **Referenz auf `platform-notes.md` in Agents durch Existierendes ersetzen** -- In rust-core.md und android-platform.md: Entweder auf `architecture.md` zeigen (wo Plattform-Quirks jetzt stehen) oder auf die neu erstellte platform-notes.md.

### Mittelfristig

6. **`/sync-prompts` Skill erstellen** -- Skill, der LLM-Prompt-Inhalte aus `src-tauri/src/llm/mod.rs` und `android/kotlin-src/com/dikta/voice/DiktaApi.kt` vergleicht und Diff ausgibt. Keine Automatisierung -- nur Sichtbarkeit. Verhindert stillen Drift.

7. **`/plan-feature` persistiert Output** -- Plan in `briefings/plan-[feature-slug].md` schreiben (optional, z.B. via `--save` Argument). Macht geplante aber noch nicht gestartete Features sichtbar.

8. **`scripts/rust-core` Starter** -- Analog zu `scripts/android-platform`: Direkte Session mit rust-core Agent fuer iteratives Rust-Debugging. Entlastet den Tech-Lead-Kontext bei Windows-Debugging.

9. **Backlog in project-status.md priorisieren** -- Aktueller Backlog: 6 Items, alle gleichwertig. Eine einfache Priorisierung (H/M/L oder Reihenfolge) wuerde klarstellen was in der naechsten Session angegangen werden soll.

---

## 7. Bewusst beibehalten

**Drei-Quellen-Wissenshierarchie (`project-status.md` / `knowledge/` / `CLAUDE.md`+Agents)**
Sinnvoll. Die Trennung zwischen "kurzlebiger Projektstatus", "stabiles gesammeltes Wissen" und "Teamstruktur" funktioniert gut. Keine Zusammenfuehrung empfohlen.

**android-platform Direkt-Session-Modus mit Briefing-System**
Sinnvoll. Android-Entwicklung ist iterativ und erfordert echten Dialog. Der Briefing-Pfad (`briefings/android-platform-*.md`) und der Script-Starter funktionieren. Die Grundidee "Tech Lead schreibt Briefing, Spezialist arbeitet es durch" ist solid.

**Skills als Haiku (operationell) vs. Agents als Sonnet (urteilend)**
Sinnvoll. Die Modell-Differenzierung (Haiku fuer build/lint/test/scaffold, Sonnet fuer research/plan/debug) ist kosteneffizient. Haiku-Skills fuer deterministische Tasks, Sonnet-Agents fuer Tasks die Urteilsvermoegen brauchen.

**`/debug-error` fixt nicht selbst, schlaegt nur Fix vor**
Sinnvoll. Analysieren und Implementieren trennen schuetzt vor voreiligen Fixes. Der Tech Lead entscheidet nach der Analyse, welcher Agent den Fix umsetzt. Das ist ein korrektes Kontrollprinzip.

**Konventionelle Commits via `/commit-progress`**
Sinnvoll. Kleine, haeufige Commits mit klarer Prafix-Konvention (feat/fix/refactor/docs/chore). Ergibt lesbare History ohne grossen Overhead.

**Keine Redux / kein komplexes State-Management im Frontend**
Sinnvoll fuer die aktuelle Projektgroesse. React Context + useReducer ist ausreichend. Das Hinzufuegen von Zustand oder Redux wuerde nichts loesen was jetzt ein Problem ist.

---

## Anhang: Dateistruktur

```
dikta/
  CLAUDE.md                          -- Projekt-Ueberblick, Regeln, Team-Tabelle [AKTUELL]
  main-agent.md                      -- Tech-Lead System-Prompt [AKTUELL]
  project-status.md                  -- Projektstatus, Backlog, Session-Changelog [AKTUELL]
  .claude/
    settings.json                    -- Tool-Permissions (alle erlaubt)
    settings.local.json              -- Lokale Overrides
    agents/
      rust-core.md                   -- Rust-Backend-Agent [AKTUELL, korrekt]
      ui-dev.md                      -- Frontend-Agent [VERALTET -- Komponenten-Namen falsch]
      android-platform.md            -- Android-Agent [VERALTET -- IME-Fokus, Bubble ignoriert]
    skills/
      scaffold/SKILL.md              -- Modul-Templates [OK, selten genutzt]
      build-app/SKILL.md             -- Build-Wrapper [GEFAEHRLICH -- nutzt nicht android-build.sh]
      run-tests/SKILL.md             -- Test-Runner [OK]
      research-api/SKILL.md          -- API-Recherche + Knowledge-Schreiber [OK, gut]
      lint-fix/SKILL.md              -- Linter/Formatter [OK]
      plan-feature/SKILL.md          -- Feature-Planer (Output nicht persistiert) [OK, Luecke]
      commit-progress/SKILL.md       -- Git-Commit [OK, kein Review vor Commit]
      debug-error/SKILL.md           -- Fehler-Analyse [OK]
      reflect/SKILL.md               -- Diese Analyse [NEU]
    worktrees/
      agent-ab3b1285/                -- Git-Worktree fuer Agent-Session (existiert noch)
  scripts/
    android-build.sh                 -- Kotlin-Copy + Build + Sign + Dropbox-Deploy [KRITISCH]
    android-platform                 -- Direkter Android-Session-Starter [OK]
    dikta-tech-lead                  -- Direkter Tech-Lead-Starter [NEU]
    sync-and-build.ps1               -- Windows-Build via PowerShell [OK]
  briefings/
    android-platform-research.md    -- Tauri-Android-Setup-Recherche (2026-03-07) [EXISTIERT]
    .gitkeep
  knowledge/
    architecture.md                  -- Tech-Entscheidungen, Plattform-Split [AKTUELL, gelegentlich aufgeblaehlt]
    api-providers.md                 -- Groq + DeepSeek API-Details [AKTUELL]
    competitors.md                   -- Konkurrenz-Analyse [EXISTIERT]
    wispr-flow-android-ux.md         -- Wispr-Flow-UX-Recherche [EXISTIERT]
    platform-notes.md                -- [FEHLT -- tote Referenz in 3 Dateien]
  src-tauri/                         -- Rust-Backend [AKTUELL]
  src/                               -- React-Frontend [AKTUELL]
  android/kotlin-src/                -- Kotlin-Quellen [AKTUELL, via android-build.sh deployt]
```
