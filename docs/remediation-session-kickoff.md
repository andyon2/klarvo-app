# Remediation — Session-Kickoff (START HERE)

**Zweck:** Diese Datei orientiert eine *frische* Session, die die Remediation-Planung fährt. Der gesamte Kontext liegt in durablen Artefakten auf der Platte — du brauchst **keine** vorherige Chat-Historie.

## 1. Lies zuerst — das ist deine Routing-Spec, NICHT neu herleiten

- **`docs/robustness-audit-2026-05-30.md`** — der adversarial verifizierte Audit (25 bestätigt, 2 umstritten, 6 widerlegt). **§0 Triage & Routing** ist die Track-Zuordnung (Heavy vs. quick-dev) + ID-Konvention (`ROB-` §2 · `DIV-` §3 · `TEST-` §4 · `DEPTH-` §5).
- **`docs/adr/0015-state-file-write-convention.md`** — *Accepted.* Gated ROB-01/02/04/05. `load_config`-Refactor ist bewusst NICHT im Scope.
- **`docs/adr/0016-android-path-parity-strategy.md`** — *Accepted.* Härten: DIV-01/05, DIV-03, DIV-04, DIV-02. DIV-06..14 sind bewusst akzeptierte Asymmetrie → **keine Stories**.

## 2. Was du tust

Fahre den **Brownfield-Track** von `bmad-create-epics-and-stories` über den Audit als Input. In BMM 6.7.1 ist Brownfield ein **Field-Type im scale-adaptiven Routing**, kein separater Command — wähle den Brownfield-Track. `discover_inputs` zieht das Audit-Doc automatisch aus `docs/`. Ergebnis: echte Epics/Stories (Heavy Track) + initiales `sprint-status.yaml` unter `_bmad-output/`.

### Heavy-Track-Epics — mit Test Architect (`*risk` / `*design` / `*trace`, weil Legacy-/Critical-Pfade)

- **Config/State-Persistenz-Härtung** [gated ADR-0015]: ROB-01/02/04/05 + Migrationsleiter-Test (TEST-03). DoD: Windows-`rename`-Atomarität echt verifizieren.
- **Android Sicherheits-Wächter** [gated ADR-0016]: DIV-01/05, DIV-03, DIV-04, DIV-02. NUR diese vier — DIV-06..14 sind per ADR geschlossen.
- **God-File-Depth**: DEPTH-config (inkl. der per ADR-0015 ausgeklammerten `load_config`-Entflechtung), DEPTH-pipeline.
- **Test-Integrität**: TEST-01..05 (falsche Sicherheit). VAD-Auto-Stop braucht erst eine Soll-Festlegung, dann echten Test.

### quick-dev Track — NICHT als Stories, via `bmad-quick-dev` (auto-verankert über `sync-sprint-status`)

- Pipeline Panic-/Drop-Safety: ROB-06/07/03/08/10
- Test-Proxy-Reparatur: TEST-06..10
- Low-sev Polish: ROB-11/15/16/17/18
- Contested (vor Fix re-evaluieren): ROB-12/13/14

## 3. BMM-Operatives — aus der Planungs-Session, damit es nicht verloren geht

- **quick-dev IST cross-session-getrackt:** `bmad-quick-dev` hat einen idempotenten `sync-sprint-status`-Step, der einen Status nie zurückregresst. Es gibt KEINE L1/L3-Tracking-Lücke — beide Tracks landen im selben `sprint-status.yaml`.
- **Watch-Point Closeout-Drift:** Der Sync hat dokumentierte Drift-Bugs an der Story-Close-/Epic-Close-Kante (Story-Status kann von `sprint-status.yaml` abweichen, Epic-Closeout läuft trotzdem durch). An jedem Story→done-Übergang kurz gegenchecken, dass das Ledger den Stand übernommen hat — nicht blind vertrauen.
- **Pfade (`_bmad/bmm/config.yaml`):** project_knowledge = `docs/`, output_folder = `_bmad-output/`, document_output_language = English (Audit-Doc ist bewusst Deutsch — Team-Sprache), communication_language = German.
- **Smoke-Test-DoD:** Surface-Stories (alles, was `shells/windows` oder `android/` berührt) brauchen einen echten Windows-Release-Build + manuellen Test im DoD — Linux `cargo test` reicht NICHT.

## 4. Was NICHT zu tun ist

- DIV-06..14 als Stories anlegen — per ADR-0016 als akzeptierte Asymmetrie geschlossen.
- `load_config` strukturell entflechten als Teil der Config-Härtung — per ADR-0015 separate Depth-Story.
- Die §0-Triage neu herleiten — sie ist die Spec.

## 5. Status-Gate

Beide Gate-ADRs sind Accepted → beide Heavy-Epics (Config-Härtung, Android-Wächter) sind entsperrt. God-File-Depth und Test-Integrität hängen an keinem Gate.
