# BMAD – Mensch-im-Step: Technische Verifikation

**Datum:** 2026-07-16 · **Referenz-Installation:** klarvo (`products/klarvo`), BMAD **6.6.1-next.2**
(`_bmad/bmm/config.yaml:3`) · **Vorarbeit:** `_bmad-output/tdd-in-bmad-2026-06-27.md`
**Zweck:** Entscheidungsgrundlage für ein Fremd-Team (kleines .NET-Dev-Team, Mono-Repo, heute nur
`bmad quick-dev`). Wunsch des Entscheiders: quick-dev behalten, aber Designvorgaben (von einer
nicht-technischen Autorin), Edge-Cases und Best Practices systematisch ins KI-Coden bekommen.

> **Legende:** `[V]` = VERIFIZIERT (Datei gelesen / Test gelaufen) · `[A]` = ANGENOMMEN/VERMUTET
> (plausibel, nicht direkt geprüft). Jede Aussage trägt genau ein Tag. Belege als `Datei:Zeile`.

---

## Fazit (5 Sätze)

1. **Mensch-im-Step geht — mit Auflagen:** `dev-story` ist auf das *Story-File* self-contained (es
   parst feste Abschnitte und fährt ausschließlich auf `Tasks / Subtasks`); ein von Hand
   geschriebenes Story-File ohne `create-story` wird strukturell akzeptiert, **aber** die
   technische Verdichtung (Dev Notes, Task-Zerlegung, Source-Refs, BDD-ACs) ist genau der
   anspruchsvolle Teil und nicht von einer nicht-technischen Autorin allein leistbar.
2. **Die Falle ist belegt (ja):** Jede Skill-Datei verdrahtet fix `_bmad/scripts/resolve_customization.py`,
   `_bmad/bmm/config.yaml` und die Merge-Kette `_bmad/custom/<skill>.toml`; die Basis-`customize.toml`
   trägt wörtlich „DO NOT EDIT — overwritten on every update“ — d.h. Eigenanbauten außerhalb der
   vier vorgesehenen Override-Schlüssel werden beim Update überschrieben.
3. **Die drei Wunsch-Elemente existieren im Vollpfad bereits als eigene Schichten** — Design →
   `create-ux-design`/PRD-Artefakte, Edge-Cases → Acceptance Criteria + Edge-Case-Hunter im Review,
   Best Practices → `project-context.md` + CLAUDE.md/Memory — und fließen zu genau definierten
   Zeitpunkten in den KI-Kontext (Aktivierung, Story-Erstellung, Dev-Zeit, Review-Zeit).
4. **Entscheidender Befund:** quick-dev *lädt diese Schichten heute schon*, wenn sie als Artefakte
   existieren (PRD/Architektur/UX in `planning_artifacts`, `project-context.md`, CLAUDE.md/Memory) —
   der Engpass ist nicht die quick-dev-Engine, sondern die **vorgelagerten, gepflegten Artefakte**
   und die Pro-Story-Verdichtung, die `create-story` leistet.
5. **Empfehlung in einem Satz:** Nicht „quick-dev + Eigenbau“ (Falle), sondern entweder
   (a) die Artefakt-Schichten sauber befüllen und quick-dev behalten, oder (b) den Original-Vollpfad
   fahren, wobei die nicht-technische Autorin *Design + Akzeptanz-Intent* besitzt und ein technischer
   Schritt (oder der `create-story`-Lauf) daraus das dev-fertige Story-File macht.

### Mapping-Tabelle (Kurzform, Details in Teil C)

| Wunsch-Element | Wohnort im Vollpfad | Wann im KI-Kontext | Deckt quick-dev das heute? |
|---|---|---|---|
| **Designvorgaben** | `create-ux-design` → `planning_artifacts/*ux*.md`, PRD | Story-Erstellung (in Dev Notes verdichtet) → Dev-Zeit | Teilweise: lädt `*ux*` selektiv / via epic-context `[V]` |
| **Edge-Cases** | Acceptance Criteria (BDD) + Dev Notes; Edge-Case-Hunter im Review | Story-Zeit (AC/Dev Notes) **und** Review-Zeit (Hunter) | Ja im Review (`step-04`) `[V]`; als *vorab geplant* nur über Spec |
| **Best Practices / Konventionen** | `project-context.md`, CLAUDE.md/Memory, `architecture.md` | Aktivierung (`persistent_facts`) + Dev-Zeit + Review-Zeit | Ja: quick-dev lädt project-context + CLAUDE.md + memory `[V]` |

---

## Überblick: die zwei Betriebsarten von BMAD

BMAD kennt zwei Wege von „Intent“ zu „gemergtem Code“. Sie teilen dieselben Bausteine (dieselben
drei adversarialen Review-Subagents, dasselbe `sprint-status.yaml`-Format, dieselbe `customize.toml`-
Mechanik) — sie unterscheiden sich in **Granularität** und **Persistenz**. Dieser Abschnitt gibt die
Landkarte; die Teile A/B/C zoomen in die Details. Alles hier ist `[V]` aus den Skill-Dateien gelesen.

### ① quick-dev — ein Skill, eine Session, Spec ad hoc

quick-dev (`bmad-quick-dev`) fährt den ganzen Bogen von Intent bis lokalem Commit in **einem** Skill-
Lauf. Zentrales Artefakt ist eine **Spec-Datei** (`spec-{slug}.md`), die der Workflow *selbst aus dem
Intent generiert* — keine vorgelagerten Planungs-Artefakte nötig, werden aber geladen, wenn vorhanden.

```
step-01 Clarify & Route  →  step-02 Plan  →  step-03 Implement  →  step-04 Review  →  step-05 Present
   Intent + Kontext         [HUMAN-GATE]       baseline_commit       3 Subagents        Review-Order
   laden, Route wählen      Approve/Edit       Subagent codet        + Loopback ↺       Status→done, Commit
```

- **step-01** — Intent prüfen; Kontext laden (Planungs-Artefakte, `project-context.md`, CLAUDE.md,
  Memory); VCS-Check; Multi-Goal-Check → Route one-shot / plan-code-review.
- **step-02 (Human-Gate)** — Codebase untersuchen; `spec-template` füllen; Token-Check;
  **Mensch: Approve/Edit** → Status `ready-for-dev`, `<frozen-after-approval>` gelockt.
- **step-03** — `baseline_commit`; Status `in-progress`; Subagent implementiert; Self-Check Tasks `[x]`.
- **step-04** — 3 adversariale Subagents (Blind / Edge-Case / Acceptance); klassifizieren
  (`intent_gap`/`bad_spec`/`patch`/`defer`/`reject`); **Loopback** zu step-02/03 bei Spec-/Intent-Fehlern
  (max. 5 Iterationen).
- **step-05** — Suggested Review Order; Status `done`; lokaler Commit; Editor öffnen.

**Kern:** eine Spec-Datei, ein Durchlauf, **ein** harter Human-Gate. Scope-Ziel: *ein* Goal,
900–1600 Tokens.

### ② Vollpfad — vier Skills, Artefakte persistieren, Board als Rückgrat

Zerlegt dieselbe Arbeit in **vier getrennte Skills** (typisch getrennte Sessions, idealerweise
unterschiedliche LLMs). Zentrales Artefakt ist ein **Story-File**, das durch die Skills wandert;
`sprint-status.yaml` trägt den Lifecycle-Status jeder Story.

```
sprint-planning  →  create-story        →  dev-story           →  code-review
(Vorstufe)          [HUMAN-GATE]            [HUMAN-GATE]            [HUMAN-GATE]
backlog-Stories     Story-File aus          fährt auf Tasks/        3 Subagents, Triage,
in sprint-status    epics/PRD/UX+Template    Subtasks, red-green    Findings→Story,
                    Status→ready-for-dev     Status→review          bei clean Status→done
```

- **sprint-planning** (Vorstufe) — erzeugt `sprint-status.yaml` mit `backlog`-Stories `[A]`
  (Skill nicht im Detail gelesen).
- **create-story (Human-Gate)** — nimmt erste `backlog`-Story; lädt epics/PRD/arch/UX + Vorgänger-Story
  + git; schreibt umfassendes Story-File; Status `ready-for-dev`.
- **dev-story (Human-Gate)** — liest Story-File; fährt auf `Tasks/Subtasks`; red-green-refactor;
  DoD-Checkliste; Status `review`.
- **code-review (Human-Gate)** — 3 adversariale Subagents; Triage (`decision_needed`/`patch`/`defer`/
  `dismiss`); Findings ins Story-File; bei clean → Status `done`, sprint-status-Sync.

**Kern:** getrennte Skills/Sessions, **persistente** Artefakte (Story-File wandert, Vorgänger werden
gelesen), `sprint-status.yaml` als zentraler Zustand, **mehrere** Human-Gates. Der Plan wird nicht aus
dem Intent generiert, sondern aus epics/PRD/UX *destilliert*.

### ③ Gegenüberstellung

| Dimension | quick-dev | Vollpfad (create-story → dev-story → code-review) |
|---|---|---|
| Skills / Sessions | 1 Skill, 1 Session | 4 Skills, typ. mehrere Sessions |
| Zentrales Artefakt | `spec-{slug}.md` (ad hoc, `spec-template`) | Story-File (`template.md`) + `sprint-status.yaml` |
| Woher der Plan? | aus dem Intent *generiert* (step-02) | aus epics/PRD/UX *destilliert* (create-story) |
| Planungs-Artefakte | optional (geladen wenn vorhanden) | zentral (epics/PRD/arch/UX) |
| Scope-Granularität | ein Goal, 900–1600 Tokens | Epic → Story-Zerlegung |
| Review | eingebaut (step-04) | eigener Skill (`code-review`) — gleiche 3 Subagents |
| Human-Gates | 1 (Spec-Approve) + Review-Entscheidungen | Board + Gate je Skill |
| Status-Tracking | optional (nur wenn `story_key` auflösbar) | `sprint-status.yaml` ist der Kern |
| Passt am besten für… | einzelne, klar umrissene Änderung / Bugfix | kontinuierlicher Epic-Fluss, mehrere Beteiligte |

> **Brücke zum Rest:** Der Wunsch des Zielteams (Design + Edge-Cases + Best Practices systematisch) ist
> die Stärke des **Vollpfads** — **Teil C** zeigt, in welcher Schicht jedes Element sitzt und wann es
> in den Kontext fließt; **Teil A** prüft, ob ein Mensch die `create-story`-Stufe ersetzen kann;
> **Teil B** erklärt, warum man die geteilten Bausteine nicht eigenmächtig umbauen sollte.

---

## Teil A — Mensch-im-Step (Kernfrage)

### A.1 Was erwartet `dev-story` genau? (Input-Kontrakt)

**Story-Findung** — drei Wege (`bmad-dev-story/SKILL.md:87–186`) `[V]`:
- **Expliziter Pfad** (`{{story_path}}` übergeben): Datei direkt lesen, `story_key` aus Dateiname
  ableiten, Sprint-Status-Discovery **komplett übersprungen** (`:88–92`).
- **Sprint-basiert:** erste Story in `sprint-status.yaml` mit Status `ready-for-dev`
  (Schlüsselmuster `nummer-nummer-name`, `:102–106`).
- **Ohne Sprint-Datei:** Scan von `implementation_artifacts` nach `*-*-*.md` mit Status
  `ready-for-dev` im File (`:152–186`).

**Geparste Pflicht-Abschnitte** (`:194`) `[V]`:
`Story, Acceptance Criteria, Tasks/Subtasks, Dev Notes, Dev Agent Record, File List, Change Log, Status`.

**Der load-bearing Treiber ist `Tasks / Subtasks`** `[V]`:
- „FOLLOW THE STORY FILE TASKS/SUBTASKS SEQUENCE EXACTLY AS WRITTEN — NO DEVIATION“ (`:298`)
- „NEVER implement anything not mapped to a specific task/subtask in the story file“ (`:322`)
- Findet die erste unerledigte Task `[ ]` (`:200`). **Gibt es keine unerledigte Task, springt der
  Workflow direkt zu Step 9 „Completion“** (`:202–204`) — d.h. eine Story ohne offene Tasks wird als
  „fertig“ markiert, ohne Code zu schreiben. Ohne `Tasks/Subtasks` passiert also nichts Sinnvolles.

**Weitere Kontext-Ladungen** `[V]`:
- Step 2 lädt `{project_context}` „for coding standards and project-wide patterns (if exists)“ (`:212`).
- Dev Notes werden für Architektur-/Vorgänger-Guidance geparst (`:196–198`, `:214–216`).

**HALT-Bedingungen** (`:205–206`) `[V]`: Story-File nicht lesbar → HALT; Task-Anforderung
mehrdeutig → nachfragen/HALT. **Status ist kein hartes Gate bei explizitem Pfad:** Step 4 gibt bei
unerwartetem Status nur „Continuing anyway…“ aus (`:282–286`) `[V]`.

**Kritischer struktureller Befund:** `dev-story` selbst lädt **keine** `epics.md`/`prd.md`/
`architecture.md` — es hat gar keine „Input Files“-Tabelle wie `create-story`. Es verlässt sich
darauf, dass diese Inhalte bereits als **Dev Notes im Story-File verdichtet** sind. `[V]`
(vgl. `create-story/SKILL.md:80–85` Input-Tabelle vs. `dev-story/SKILL.md` — keine solche Tabelle).
→ **Folge:** Fehlen die Planungs-Artefakte, *bricht `dev-story` nicht* — es degradiert die
*Qualität des Story-Files*, das `create-story` erzeugt. Ein Mensch, der ein gutes Story-File mit
belastbaren Dev Notes liefert, macht `dev-story` von den Planungs-Artefakten unabhängig.

### A.2 Was macht `create-story` ALLES? (die Seiteneffekte, die ein Mensch ersetzt)

Aus `bmad-create-story/SKILL.md` `[V]`:
1. **Story-Auswahl + Keys:** liest `sprint-status.yaml` komplett, nimmt die erste `backlog`-Story,
   leitet `epic_num`/`story_num`/`story_key`/`story_id` aus dem Schlüssel ab (`:130–219`).
2. **Epic-Statuslift:** ist es die erste Story eines Epics, wird `epic-N` von `backlog`→`in-progress`
   gehoben; bei Epic-Status `done` → HALT (`:164–186`).
3. **Artefakt-Analyse:** lädt `epics/prd/architecture/ux` über `discover-inputs.md`
   (`:248–253`, Input-Tabelle `:80–85`); Vorgänger-Story + `git log`-Intelligenz (`:263–283`).
4. **Story-Erzeugung aus `template.md`** (`:341–392`) mit fixer Abschnittsstruktur (s.u.).
5. **Statuswechsel:** setzt Story-`Status: ready-for-dev` (`:389`), aktualisiert
   `sprint-status.yaml` `backlog`→`ready-for-dev` + `last_updated` (`:398–407`).
6. **`persistent_facts`** lädt `file:{project-root}/**/project-context.md`
   (`bmad-create-story/customize.toml:33–35`).
7. **Selbst-Validierung** gegen `checklist.md` vor dem Finalisieren (`:395`).

**Pflicht-Template** (`bmad-create-story/template.md`) `[V]` — die Abschnitte, die ein Mensch
reproduzieren muss: `Status`, `## Story` (As a/I want/so that), `## Acceptance Criteria`,
`## Tasks / Subtasks` (mit `(AC: #)`-Referenzen), `## Dev Notes` (Architektur-Constraints, Source-Tree,
Testing-Standards, `### References` mit `[Source: docs/…#Section]`), `## Dev Agent Record`.

### A.3 Vorbedingungs-Artefakte von `create-story` (nicht von `dev-story`)

`discover-inputs.md` `[V]`: lädt `prd/architecture/ux/epics` (sharded oder whole), Strategie
`SELECTIVE_LOAD`. **Fehlt ein Artefakt, ist das kein Fehler** — `{pattern}_content` wird leer gesetzt
und dem User Nachlieferung angeboten (`discover-inputs.md:71–75`). D.h.: fehlende Planungs-Artefakte
schwächen die erzeugte Story, verhindern die Erzeugung aber nicht. `[V]`

### A.4 PRAKTISCHER TEST (Trockenlauf) — Minimal-Story von Hand

Ich habe von Hand ein Minimal-Story-File `9-99-demo-hand-story.md` (ohne `create-story`) geschrieben
mit: `Status: ready-for-dev`, `## Story`, `## Acceptance Criteria` (2× BDD), `## Tasks / Subtasks`
(2 unerledigte Tasks mit `(AC: #)`), `## Dev Notes` (Konvention + Testframework), `### References`,
`## Dev Agent Record`.

Strukturelle Prüfung gegen das, was `dev-story` Step 1 parst: `[V]`
- Alle sechs von `dev-story` geparsten Abschnittsköpfe PRESENT.
- Dateiname erfüllt `story_key`-Muster `nummer-nummer-name.md` → `9-99-…` (für sprint-lose Discovery
  und explizite Pfad-Übergabe ausreichend).
- 2 unerledigte `- [ ]`-Tasks vorhanden → der Treiber aus `dev-story:200` findet Arbeit.

**Ergebnis:** Das handgeschriebene File erfüllt strukturell **jeden** Abschnitt, auf den `dev-story`
zugreift, plus Dateinamens-Muster plus den Task-Treiber. **Marker: NICHT VOLL VERIFIZIERT** — dies ist
ein *struktureller Trockenlauf* (Datei-/Header-/Muster-Abgleich gegen die gelesene Workflow-Logik),
**kein** Live-`dev-story`-Lauf (ein echter LLM-Lauf war für diese Untersuchung zu teuer). Was der
Trockenlauf **nicht** beweist: dass die Task-Zerlegung inhaltlich implementierbar/eindeutig ist —
genau hier greift `dev-story:206` („ambiguous → ASK/HALT“).

### A.5 Robustheit der Kette bei human-authored Stories — wo es reißt

| Bruchstelle | Ursache | Was ein Mensch/Prozess herstellen muss |
|---|---|---|
| Story wird nicht gefunden | Dateiname nicht `n-n-name.md` **oder** Status ≠ `ready-for-dev` (bei Discovery) | Namens-/Status-Konvention exakt treffen — oder Pfad explizit übergeben `[V]` |
| „Nichts zu tun“ | keine unerledigte `Tasks/Subtasks` | mind. eine `- [ ]`-Task; Task = konkrete Aktion + Datei `[V]` |
| HALT „ambiguous“ | Task-Anforderung mehrdeutig | eindeutige, implementierbare Task-Zerlegung (technischer Akt) `[V]` |
| Schwache Umsetzung ohne HALT | leere/dünne Dev Notes (kein Architektur-/Konventions-Kontext) | Dev Notes mit Constraints + Source-Refs; ODER `project-context.md` vorhalten `[V]` |
| Sprint-Board driftet | `sprint-status.yaml` von Hand nicht mitgepflegt | Key-Format `n-n-name` reproduzieren, sonst Sprint-Discovery/Review-Auto-Done bricht `[V]` |

**Bewertung:** Die Kette ist bei human-authored Stories **robust auf der Mechanik-Ebene** (Parsing,
Discovery, Status) und **fragil auf der Inhalts-Ebene** (Dev Notes / Task-Zerlegung / eindeutige ACs).
Die Mechanik kann eine nicht-technische Person mit einer Vorlage treffen; die Inhalts-Ebene ist ein
technischer Autoren-Akt. → **Mensch-im-Step = JA, mit Auflagen** (Rollentrennung Design-Intent vs.
technische Story-Verdichtung).

---

## Teil B — Fallen-Beleg: „BMAD teilweise + Eigenes draufbauen“

Alle Belege strukturell, nicht meinungsbasiert.

**B.1 Fixe Verdrahtung in jeder Skill-Datei** `[V]`
Jede `SKILL.md` (create-story, dev-story, quick-dev, create-ux-design, …) hat einen identischen
„On Activation“-Block, der hart referenziert:
- `python3 {project-root}/_bmad/scripts/resolve_customization.py --skill {skill-root} --key workflow`
- `{project-root}/_bmad/bmm/config.yaml`
- Merge-Kette `customize.toml` (base) → `_bmad/custom/{skill-name}.toml` (team) →
  `_bmad/custom/{skill-name}.user.toml` (user), Regeln: „Scalars override, tables deep-merge,
  arrays … append“.
Beleg z.B. `bmad-dev-story/SKILL.md:28–38`, `bmad-create-story/SKILL.md:30–38`,
`bmad-quick-dev/SKILL.md:42–50`. → Wer diese Pfade/Formate ändert, bricht alle Skills gleichzeitig.

**B.2 Die Basis-Dateien werden beim Update überschrieben** `[V]`
`bmad-create-story/customize.toml:1` wörtlich: **„# DO NOT EDIT -- overwritten on every update.“**
Die einzige update-sichere Erweiterungsfläche sind die **vier Override-Schlüssel** in
`_bmad/custom/*.toml`: `activation_steps_prepend`, `activation_steps_append`, `persistent_facts`
(alle *append*), `on_complete` (Scalar) (`customize.toml:15–41`). `[V]`
Alles andere — `SKILL.md`-Step-Rümpfe, `template.md`, `checklist.md`, `discover-inputs.md`,
Step-Files — ist **keine** Merge-Fläche. `[A]` (stark begründet: es sind vom Installer erzeugte
Dateien, die Basis-`customize.toml` sagt explizit „overwritten on every update“; für die übrigen
installierten Dateien plausibel dasselbe Update-Verhalten, aber nicht per Datei-Kommentar bestätigt).

**B.3 Der sanktionierte, nicht-brüchige Seam existiert bereits** `[V]`
klarvo nutzt genau ihn: `bmad-create-story/customize.toml:33–35` lädt
`file:{project-root}/**/project-context.md` als `persistent_fact`; `bmad-quick-dev.user.toml:3–4`
injiziert eine Commit-Policy als `persistent_fact`. Das ist die vorgesehene Art, eigene
Standards/Guardrails „draufzubauen“ — **ohne** Basis-Dateien zu editieren.

**B.4 Geteilte Cross-File-Konventionen als Kopplung** `[V]`
Das `sprint-status.yaml`-Schlüsselformat `nummer-nummer-name` ist ein **geteilter Vertrag** zwischen
`sprint-planning`, `create-story` (`SKILL.md:156–160`), `dev-story` (`SKILL.md:102–106`),
`code-review` (`step-01:38`) und `quick-dev/sync-sprint-status.md:14`. Ein Eigenbau, der Story-Keys
anders bildet, entkoppelt das gesamte Board/Discovery/Review-Auto-Status. Ebenso sind
`planning_artifacts`/`implementation_artifacts` aus `config.yaml:7–8` von allen Skills referenziert.

**Beleg-Fazit B:** Die Falle ist **belegt**. „Teilweise nutzen + Eigenes draufbauen“ ist nur dann
nicht brüchig, wenn der Anbau strikt auf `_bmad/custom/*.toml` (vier Schlüssel, v.a. `persistent_facts`
mit `file:`-Refs) beschränkt bleibt. Jeder Anbau, der Step-Logik, Templates oder Schlüsselformate
verändert, ist update-brüchig und quer gekoppelt.

---

## Teil C — Gerüst-Mapping: Wo leben die drei Elemente, wann fließen sie in den Kontext

**C.1 Designvorgaben** `[V]`
- **Wohnort:** `bmad-create-ux-design` erzeugt UX-Spezifikationen nach `planning_artifacts/*ux*.md`
  (`bmad-create-ux-design/SKILL.md:1–3`, Output-Ort = `planning_artifacts`, `:… Load Config`);
  ergänzend PRD.
- **Kontext-Timing:** `create-story` lädt `ux` über die Input-Tabelle (`SKILL.md:84`) +
  `discover-inputs.md` und **verdichtet es in die Story-Dev-Notes** → `dev-story` liest Dev Notes zur
  Dev-Zeit. In quick-dev: Pfad A zieht UX in `epic-<N>-context.md`
  (`compile-epic-context.md:40–41`), Pfad B lädt `*ux*` selektiv (`step-01:73–78`).
- **Wichtig für den Wunsch:** `create-ux-design` ist explizit als **„facilitator working with a product
  stakeholder“** angelegt (`SKILL.md:6`) — das ist der natürliche Sitz für die **nicht-technische
  Autorin**: sie produziert das Design-Artefakt, das danach systematisch in jede Story fließt.

**C.2 Edge-Cases** `[V]`
- **Wohnort (geplant):** Acceptance Criteria in BDD-Form (`template.md:13–15`, „already BDD formatted“
  in `create-story/SKILL.md:261`) und Dev Notes „edge cases“. `dev-story` verlangt in Step 5
  „Handle error conditions and edge cases as specified in task/subtask“ (`:310`); die DoD-Checkliste
  prüft „Edge Cases Handled“ (`bmad-dev-story/checklist.md:38`).
- **Wohnort (entdeckt):** Der **Edge Case Hunter** ist ein eigener Review-Subagent — in `code-review`
  (`step-02:23`) **und** in quick-dev (`step-04-review.md:29`).
- **Kontext-Timing:** zweimal — als *geplante* ACs/Dev-Notes zur Story-Zeit **und** als *aktiv
  gejagte* Findings zur Review-Zeit. Die geplante Variante braucht ein sauberes AC-Artefakt; die
  gejagte hat quick-dev heute schon.

**C.3 Best Practices / Code-Konventionen** `[V]`
- **Wohnort:** `project-context.md` — in klarvo real befüllt mit **31 Regeln**
  (`_bmad-output/project-context.md` Frontmatter `rule_count: 31`, Abschnitte
  „Critical Implementation Rules“, „Anti-Patterns“); erzeugt via `bmad-generate-project-context`
  (`project-context-template.md`). Ergänzend CLAUDE.md/Memory und `architecture.md`.
- **Kontext-Timing (früheste + pervasivste Schicht):**
  - **Aktivierung:** `create-story` lädt `project-context.md` als `persistent_fact`
    (`customize.toml:33–35`).
  - **Dev-Zeit:** `dev-story` Step 2 lädt `{project_context}` (`SKILL.md:212`); quick-dev Step 4
    lädt `project_context` **+ CLAUDE.md + Memory** (`quick-dev/SKILL.md:68–69`).
  - **Review-Zeit:** Acceptance Auditor prüft gegen Spec + Kontext-Docs
    (`code-review/step-02:25–26`; quick-dev `step-04:30`).

**C.4 Kontrast — was deckt quick-dev heute schon ab, was braucht den Vollpfad?** `[V]`
- **Schon in quick-dev:** lädt `project-context.md`, CLAUDE.md, Memory (`SKILL.md:68–69`); Pfad A
  kompiliert `epic-<N>-context.md` aus PRD/Architektur/UX (`step-01:53–67`, `compile-epic-context.md`),
  Pfad B lädt PRD/Architektur/UX selektiv (`step-01:71–78`); erzeugt eine Spec mit BDD-ACs
  (`spec-template.md`, READY-Standard `SKILL.md:12–20`); fährt einen **dreifachen adversarialen
  Review** (Blind/Edge-Case/Acceptance) mit Loopback (`step-04-review.md:26–47`).
- **Nur im Vollpfad:** ein **durably authored, versioniertes** Design-Artefakt (`create-ux-design`),
  eine **pro-Story** technisch verdichtete Dev-Notes-/Task-Struktur mit Source-Refs (`create-story`),
  und ein Sprint-Board mit Epic-/Story-Lifecycle (`sprint-planning` → `create-story` → `dev-story` →
  `code-review`). quick-dev **generiert seine Spec ad hoc aus dem Intent** (`step-02-plan.md:14–16`) —
  es gibt darin **keinen festen Sitz**, an dem eine nicht-technische Autorin ein Design-Dokument
  einreicht, das *garantiert* jede Story speist. Genau dieser feste Sitz ist der Mehrwert des
  Vollpfads.

**Synthese C:** Die drei Wünsche mappen **fast vollständig** auf Artefakte, die quick-dev *bereits
konsumiert* — **sofern sie existieren**. Der fehlende Baustein ist nicht die Engine, sondern
(1) die gepflegten Upstream-Artefakte (UX/PRD/Architektur, `project-context.md`) und (2) die
Pro-Story-Verdichtung von `create-story`. Damit ist die These des Auftrags bestätigt: „quick-dev +
Eigenbau“ ist die Falle; die sauberen Alternativen sind **Artefakte befüllen + quick-dev behalten**
*oder* **Vollpfad mit menschlicher `create-story`-Autorschaft**.

---

## Teil D — Zweitbetrachtung: Reicht quick-dev mit Customization?

*Anlass: Challenge des Auftraggebers — der Report war von der Eingangs-These („das ist die Bestellung
des vollen Gerüsts“) gefärbt. Diese Rubrik betrachtet die Frage neu, nur auf Basis der verifizierten
Skill-Dateien. Teile A–C bleiben gültig; hier ändert sich die **Gewichtung**, nicht die Faktenlage.*

### D.1 Wo der Bias lag

Die in Teil B belegte Falle betrifft das **Umbauen von BMAD-Interna** (Step-Logik, Templates,
Key-Formate). Was das Zielteam braucht — *eigene Dokumente einspeisen* — ist kein „Eigenbau“ in diesem
Sinne, sondern der **sanktionierte Seam**: `persistent_facts` mit `file:`-Referenzen in
`_bmad/custom/bmad-quick-dev.toml` plus team-eigene Markdown-Dateien, die BMAD-Updates nicht berühren
(Existenzbeweis: `bmad-quick-dev.user.toml:3–4` in klarvo) `[V]`. Der ursprüngliche Fazit-Satz hat
„Customization“ und „Eigenbau“ zu nah aneinandergerückt.

### D.2 Was quick-dev für die drei Wünsche schon kann

1. **Intent-Dateien der Autorin werden nativ ingestiert** `[V]`: `step-01:25` („intent files, external
   docs, plans, descriptions → ingest it as starting intent“) und `:50`. Ihre ClickUp-Vorarbeiten, als
   Markdown exportiert, sind genau dieses Format. Sie muss **kein** dev-fertiges Story-File schreiben.
2. **Die technische Verdichtung macht die Maschine, nicht sie** `[V]`: `step-02` untersucht die
   Codebase und generiert die Spec; ein Mensch approved am Gate. **Wichtiger Punkt gegen die
   Eingangs-These:** Im Vollpfad wäre die Story-Verdichtung ihr Problem (Teil A: „mit Auflagen“); in
   quick-dev ist sie es nicht. quick-dev ist für eine nicht-technische Autorin *freundlicher* als
   Mensch-im-create-story-Step.
3. **Designvorgaben:** `design-guidelines.md` per `persistent_facts` → bei *jedem* Lauf geladen;
   zusätzlich Spec-Frontmatter `context:`-Liste, die der Acceptance Auditor lesen **muss**
   (`step-04:30`) `[V]`.
4. **Edge-Cases:** Edge-Case-Hunter eingebaut (`step-04:29`); geplante Edge-Cases via Intent-Datei in
   die Spec-ACs `[V]`.
5. **Best Practices:** `project-context.md` + CLAUDE.md + Memory bei Aktivierung (`SKILL.md:68–69`);
   `generate-project-context` existiert zum Erzeugen `[V]`.
6. **Kontinuität + Board gibt es auch:** Pfad A lädt die Vorgänger-Spec desselben Epics (Code Map,
   Design Notes; `step-01:67`) und synct `sprint-status.yaml` bei auflösbarem `story_key`
   (`sync-sprint-status.md`) — oben unterbelichtet `[V]`.

### D.3 Ehrliche Lücken von quick-dev-only

- **Scope-Deckel:** eine Spec = ein Goal, 900–1600 Tokens (`SKILL.md:24–28`); Multi-Goal-Check
  erzwingt Schnitte (`step-01:81–86`). Größere Features = mehrere Läufe; Kohärenz hängt an den
  Artefakten, nicht am Workflow `[V]`.
- **Ein einziges hartes Human-Gate** (Spec-Approve) statt Gate je Stufe — für ein kleines Team mit
  einem technischen Entscheider eher Feature als Mangel `[V]`.
- **Kein Erstellungs-Prozess für die Artefakte selbst:** quick-dev *konsumiert*
  design-guidelines/project-context, nichts erzwingt deren Pflege. Disziplin-, kein Tooling-Problem —
  der Vollpfad löst es auch nicht automatisch `[V]`.

### D.4 Einschätzung mit Bedingungen und Tripwires

**Zuversicht ~75–80 %, dass quick-dev + sanktionierte Customization reicht** — unter drei Bedingungen:

1. **Team-eigene Standards entstehen — kategorieoffen, autorin-frei:** ein Standards-Ordner
   (z.B. `docs/standards/`) per `persistent_facts`-Glob injiziert (neue Kategorie = neue Datei, Glob
   bleibt unverändert) plus `project-context.md` für Code-Konventionen (via
   `generate-project-context`). **Die Autorin schreibt ihre Story frei — kein Template, keine
   Pflegepflicht.** Die Intent-Klärung wandert als KI-Preflight zu ihr an den Schreibtisch
   (Mechanik + Pipeline im Folge-Report, s.u.).
2. **Ein technischer Mensch hält das Spec-Gate:** die Autorin liefert die Story als Intent-Datei,
   der Entwickler approved die generierte Spec (step-02 CHECKPOINT) und beantwortet Restfragen —
   dank Preflight wenige.
3. **Größere Features werden geschnitten** — oder das Team wächst in Pfad A hinein: ein leichtes
   epics-File genügt für epic-context-Kompilierung + Vorgänger-Kontinuität. Gradueller Mittelweg,
   kein Umstieg.

**Tripwires für den Vollpfad** (erst dann wechseln): mehrwöchige Feature-Züge mit 5+ zusammenhängenden
Stories werden zur Norm · mehrere Personen arbeiten parallel eine Story-Queue ab · die Spec-Gates
verlangen dem Entscheider zu viel Session-Präsenz ab (vorgefertigte Stories auf Halde gewünscht).

> **Revidierte Kernaussage:** Nicht „der Vollpfad ist die Bestellung“, sondern **„die Artefakte sind
> die Bestellung — quick-dev ist die ausreichende Engine dafür, und der Vollpfad ist die
> Eskalationsstufe, in die man hineinwachsen kann.“**
>
> **→ Weiterentwicklung:** Drei-Rollen-Modell (Autorin/Entwickler/Reviewer), Intent-Preflight bei der
> Autorin und die Glob-Mechanik im Detail: Folge-Report
> `bmad-quickdev-drei-rollen-2026-07-16.html`. Kern dort: Der einzige echte Umbau ist *zeitlich*,
> nicht technisch — die Intent-Klärung wandert vom Entwickler-Schreibtisch an den der Autorin (der
> Vollpfad löst das auch nicht: `create-story` ist „ZERO USER INTERVENTION“, `SKILL.md:17`).

### D.5 Rolle der CLAUDE.md in diesem Setup

Drei Schichten für Konventionen/Kontext, mit unterschiedlichem Träger und Reichweite:

| Schicht | Träger / Mechanik | Wer lädt sie wann | Reichweite |
|---|---|---|---|
| **CLAUDE.md** | *Runtime*-Mechanik (Claude Code lädt sie automatisch in jede Session) | Harness bei Session-Start `[V]` (beobachtet); quick-dev nennt sie zusätzlich explizit (`SKILL.md:69`) `[V]` — `create-story`/`dev-story` erwähnen sie **nicht** `[V]` | **Nur Claude Code.** BMAD unterstützt auch Copilot/Codex/Ollama (`step-01:63`) — dort existiert der Mechanismus nicht `[V]` |
| **project-context.md** | BMAD-nativ, runtime-agnostisch | `create-story` via `persistent_facts` (`customize.toml:33–35`) · `dev-story` Step 2 (`:212`) · quick-dev Aktivierung (`SKILL.md:68`) `[V]` | Alle Dev-Skills, jede Runtime — Teil des BMAD-Vertrags |
| **Eigene Dateien via `persistent_facts`** | `_bmad/custom/<skill>.toml`, `file:`-Globs | Bei Aktivierung des jeweiligen Skills `[V]` | Pro Skill steuerbar (z.B. design-guidelines nur in quick-dev + code-review) |

**Arbeitsteilung:**
- **CLAUDE.md dünn halten:** Verhaltens-/Prozessregeln (Commit-Politik, Sprache, Repo-Orientierung,
  „wo liegt was“) und *Pointer* auf die SSOTs — nicht die Code-Konventionen selbst.
- **Code-Konventionen/Testing-Regeln gehören nach `project-context.md`** — die Schicht, die im
  BMAD-Vertrag hängt und jede Runtime erreicht. **Nie doppelt** in CLAUDE.md und project-context.md
  pflegen (Drift: zwei Quellen, eine veraltet still).
- **Für das Zielteam konkret:** Mit Claude Code ist CLAUDE.md die bequeme Klammer über *alle* Arbeit
  (auch außerhalb der BMAD-Skills — Ad-hoc-Fragen, Debugging). Ist die Runtime offen/gemischt, gehört
  alles Tragende nach `project-context.md`, CLAUDE.md bleibt optionaler Komfort. `[A]` (Empfehlung,
  folgt aus den verifizierten Lade-Mechaniken)

---

## Versions-Caveats (was gilt für diese Installation, was kann abweichen)

- **Version:** Referenz ist **6.6.1-next.2** (`config.yaml:3`). Die Vorarbeit notiert, dass einige
  Dateien (z.B. der `dev-story` Step-5/6-Selbstwiderspruch) in `6.6.1-next.2`, `6.9.0` und `main`
  byte-identisch sind (`tdd-in-bmad-2026-06-27.md:82–86`) — **nicht** garantiert für alle hier
  zitierten Dateien. Das Zielteam sollte gegen seine eigene installierte Version prüfen. `[V]`
- **Custom-Overrides sind klarvo-spezifisch:** `bmad-quick-dev.user.toml` (keine Auto-Commits),
  `create-story` `persistent_facts` (`project-context.md`) etc. Das Zielteam hat andere/keine
  Overrides — Verhalten weicht entsprechend ab. `[V]`
- **Stack-Abhängigkeit:** klarvo = Tauri/Rust + native Kotlin/Android; Ziel = .NET-Mono-Repo. Die
  Workflow-Mechanik ist sprach-agnostisch (`dev-story` Step 7: „infer test framework from project
  structure“, `SKILL.md:336`). **Aber:** die TEA-Kette hat eine Stack-Auto-Detect-Falle, die einen
  Backend-Lauf an einem Frontend-Manifest anhält (`tdd-in-bmad-2026-06-27.md:44–49`). In einem
  Mono-Repo mit gemischten Manifesten (z.B. `.csproj` neben `package.json`) ist eine analoge
  Fehlklassifikation plausibel. `[A]`
- **Skill-Ablageform:** Die gelesenen Skills liegen unter `.claude/skills/` (installierte/compilierte
  Form). Ein anders installiertes Zielteam (anderer Agent/Runtime) kann die Skills abweichend ablegen;
  die `On Activation`-Referenzen auf `_bmad/…` bleiben aber der Vertrag. `[V]`

---

## Offene Fragen für das Gespräch mit dem Zielteam

**Nur das Team kann beantworten:**
1. **Rollen-Realität:** Kann/will die nicht-technische Autorin ein **Design-Artefakt**
   (`create-ux-design`-Output) besitzen — und wer übernimmt die **technische Story-Verdichtung**
   (Dev Notes, Task-Zerlegung, Source-Refs)? Ohne diese Rollentrennung kippt „Mensch-im-Step“.
2. **Volumen/Kadenz:** Wie viele Stories pro Woche? Der Vollpfad (sprint-planning → create-story →
   dev-story → code-review) lohnt bei kontinuierlichem Fluss; bei sporadischen Änderungen ist
   „Artefakte + quick-dev“ leichter.
3. **Sprint-Board erwünscht?** Wollen sie Epic-/Story-Lifecycle-Tracking (`sprint-status.yaml`) —
   oder reicht der spec-zentrierte quick-dev-Fluss ohne Board?
4. **Update-Politik:** Wie oft aktualisieren sie BMAD? Je häufiger, desto härter die Update-Falle bei
   Eigenanbauten — desto wichtiger, strikt auf `_bmad/custom/*.toml` zu bleiben.
5. **Design-Format der Autorin heute (ClickUp):** In welcher Form liegen die Vorarbeiten vor, und
   lassen sie sich 1:1 in ein `create-ux-design`-/PRD-Artefakt oder in `project-context.md`-Regeln
   überführen?

**Das Repo/diese Verifikation klärt bereits:**
- `dev-story` akzeptiert ein handgeschriebenes Story-File strukturell (A.4) — **die Mechanik ist kein
  Blocker**, die inhaltliche Verdichtung ist der Aufwand.
- Die Falle ist strukturell belegt (Teil B) — der einzig update-sichere Anbau-Seam ist
  `_bmad/custom/*.toml` `persistent_facts` mit `file:`-Referenzen.
- quick-dev lädt `project-context.md`/CLAUDE.md/Memory/Planungs-Artefakte bereits (C.4) — Best
  Practices und (Review-)Edge-Cases sind **heute schon abgedeckt**, wenn die Artefakte existieren;
  der ehrliche Zugewinn des Vollpfads ist der **feste Sitz für Design-Vorgaben** und die
  **pro-Story-Verdichtung**.

---

### Gelesene Belegdateien (Auszug)
`bmad-dev-story/{SKILL.md,checklist.md}` · `bmad-create-story/{SKILL.md,template.md,discover-inputs.md,checklist.md,customize.toml}` ·
`bmad-quick-dev/{SKILL.md,step-01-clarify-and-route.md,step-02-plan.md,step-04-review.md,compile-epic-context.md,sync-sprint-status.md}` ·
`bmad-code-review/steps/{step-01,step-02}` · `bmad-create-ux-design/SKILL.md` ·
`bmad-generate-project-context/project-context-template.md` · `_bmad/bmm/config.yaml` ·
`_bmad/custom/bmad-quick-dev.user.toml` · `_bmad-output/project-context.md` ·
`_bmad-output/tdd-in-bmad-2026-06-27.md`
