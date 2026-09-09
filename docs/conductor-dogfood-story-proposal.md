# Conductor-Dogfood — klarvo-Story-Vorschlag + Aufsetz-Notiz

**Von:** klarvo-Agent · **An:** Guardian / Andi · **Stand:** 2026-07-11
**Zweck:** Rückkanal zum Briefing `teams/guardian/briefings/conductor-dogfood-klarvo.md` —
die zwei Stories für den ersten Dogfood-/Eval-Baseline-Lauf des umgebauten
`bmad-story-conductor`, plus die Run-Parameter, die Guardian für die `run.sh`-L1-Verdrahtung braucht.

---

## TL;DR

| Niveau | Story | Sprache | Objektiver Orakel | Design-Gates |
|--------|-------|---------|-------------------|--------------|
| **anspruchslos** | **9-7-Follow-up** — Silence-Dauern swap-safe typisieren | Kotlin | Compiler + Inversions-Test (erweitert `RecordingModeSilenceSelectionTest.kt`) | keine |
| **anspruchsvoll** | **7.1** — Android Chunking-Parität | Kotlin | Golden-Vector-Output-Diff gegen Desktop-Referenz (`llm/mod.rs`) | design-arm (nur M8-Mini-Naht) |

Beide sind `backlog`/nicht ausgearbeitet (der `create-story`-Sitz hat echten Ground-Truth-Input),
dependency-frei baubar, **kein echtes Gerät nötig** — beide verifizieren auf **JVM-Unit-Test-Ebene**.

---

## Korrigiertes Auswahl-Kriterium (Andi, diese Session — an Guardian gemeldet)

Das Briefing riet, für die anspruchsvolle Story eine mit **Design-/Intent-Entscheidungen** zu nehmen,
damit das Gate-Divergenz-Signal sichtbar wird. **Andi hat das präzisiert/korrigiert:**

- Es gibt **zwei** Arten von Mensch-Gates. **Sorte 1** = legitimes Design-/Intent-Gate (erwartet,
  weitgehend modell-invariant, oft canon-verankert) = **Grundrauschen** für den Qualitätsvergleich.
  **Sorte 2** = pathologisches Gate: der Dev-Agent baut Mist / verheddert sich → der Mensch wird
  *früher als bei sauberer Arbeit* reingezogen (oder das Modell gatet **nicht**, wo es müsste, und
  walzt seinen Murks durch). **Sorte 2 ist der eigentliche Qualitäts-Tell.**
- Eine Story **absichtlich mit Sorte-1-Gates vollzuladen** maximiert das Grundrauschen und **ertränkt
  Sorte 2**. Richtig ist: **hoher Umsetzungs-Anspruch bei ruhigem Design-Hintergrund** — dann entsteht
  Sorte 2 nur bei echtem Straucheln und **hebt sich scharf ab**. Technischer Anspruch ≠ Design-Gate-Dichte.
- **Implikation für die Metrik (L0):** Sorte-1- und Sorte-2-Gates dürfen nicht in denselben Topf gezählt
  werden. Die Divergenz-Metrik sollte die *ungeplante* Gate-Abweichung messen, nicht die geplante.

7.1 überlebt dieses Kriterium: technisch hart, aber design-arm.

---

## Story A (anspruchslos) — 9-7-Follow-up: Silence-Dauern swap-safe typisieren

- **Quelle:** `docs/backlog.md` → „Story 9-7 follow-up — make Android silence-selection swap-safe".
- **Scope:** `KlarvoOverlayService.startRecording()` übergibt vier gleich-typisierte `Float`-Argumente
  (`tapSilenceSecs` / `longPressSilenceSecs` / `autostopSilenceSecs` / `autoModeSilenceSecs`). Ein
  Vertauschen zweier fällt nie auf (Prod-Defaults alle `2.0f`). **Fix (Variante a, bevorzugt):** distinkte
  Value-Class-Typen (`@JvmInline`), sodass ein vertauschtes Call-Site-Argument **nicht mehr kompiliert**.
- **Orakel:** Compiler (der by-construction-Fix) + ein **Inversions-Test** in der bereits existierenden
  `android/kotlin-test/com/klarvo/voice/RecordingModeSilenceSelectionTest.kt` — ein absichtlich
  vertauschtes Argument muss RED werden.
- **Warum anspruchslos:** klarer Scope, **null Design-Entscheidungen**, kleiner Blast-Radius. Testet, ob
  der Conductor die **reine Mechanik** (create → dev → review → fix → smoke) sauber durchtreibt.

## Story B (anspruchsvoll) — 7.1: Android Chunking-Parität (core output)

- **Quelle:** `_bmad-output/planning-artifacts/epics-cross-platform-parity.md` → Story 7.1 (Rows H2, H13, L4, M8).
- **Scope (voll spezifiziert mit Zeilen-Refs):** Kotlin-Chunk-Splitting in `KlarvoApi.kt` exakt an die
  Desktop-Rust-Referenz `llm/mod.rs` angleichen:
  - **H2** — Split-Indizes über **UTF-8-Byte-Länge** (wie `raw_text.len()`), nicht UTF-16 `text.length`.
  - **H13** — Join mit `\n`, nicht `\n\n`.
  - **L4** — Schwelle `< 400`, nicht `<= 400` (Off-by-one bei exakt 400).
  - **M8** — Chunk-Failure = Desktop-Abort-on-first-error-Semantik (**oder** dokumentierte, golden-vector-
    gelockte Divergenz — die *einzige* Intent-Naht der Story).
- **Orakel:** Golden-Vector-**Output-Diff** — deutsche Umlaut-Strings um die 400-Zeichen-Grenze, N-Chunk-
  Inputs; Kotlin-Output muss dem Desktop-Rust-Output byte-genau entsprechen. Die Story erzeugt ihren
  eigenen Orakel (Golden-Vectors sind Teil ihres Scopes) → self-contained.
- **Warum anspruchsvoll + design-arm:** Die UTF-8-Byte- vs. UTF-16-Länge-Subtilität (H2) ist genau die
  Falle, an der ein schwaches Modell entweder **orakel-sichtbar** scheitert oder sich verheddert und ein
  **Sorte-2-Gate** auslöst — vor ruhigem Design-Hintergrund gut sichtbar. Testet, ob die Orchestrierungs-
  Disziplin **unter technischer Last** hält.
- **Unabhängig:** Epic-7-Sequencing-Note (`epics-cross-platform-parity.md:52`): „7.1 is independent and can
  run in parallel". Braucht die Desktop-Referenz (existiert, gebaut), NICHT 7.3. **Nicht** 7.7 nehmen —
  die hängt an 7.1–7.6.

---

## Aufsetz-Notiz (Run-Parameter für Guardians `run.sh`-L1-Verdrahtung)

### Contract-Check ✅
`_bmad/custom/bmad-epic-conductor.toml` trägt alle Smoke-/Gate-Felder
(`smoke.command`, `smoke.proxy_surface`, `smoke.evidence_dir`, `visual_oracle.structural_method`)
plus einen sauberen `[tea]`-Deferred-Marker (off, bewusst). **Kein Degrade-Pfad — der Conductor läuft
mit vollem Contract.**

### Run-Params
| Feld | Wert |
|------|------|
| `repo` | `/home/andyon2/workspace/products/klarvo` |
| `build_cmd` / smoke | `scripts/android-smoke.sh` |
| Story-IDs | `9-7` (leicht) · `7.1` (schwer) — beide Kotlin, **eine** Test-/Build-Oberfläche |
| `baseRef` | **`v1-ship` @ `b29a13e`** (verifiziert: alle Ziele + Desktop-Referenz + Smoke-Script existieren dort — siehe unten) |
| Per-Seat-Modelle / Combos | **Andi klärt direkt mit Guardian** (welche Qwen-Größe je Sitz, wie viele Kombis) |

### „Bestanden" für diese (Logik-)Stories = der JVM-Test-Phase, nicht der Emulator-Drive
`scripts/android-smoke.sh` führt den **JVM-Golden-Vector-Lauf schon selbst aus**
(`./gradlew :app:testUniversalDebugUnitTest`, Zeile 187) — **bevor** der Emulator-Struktur-Drive kommt.
Für 9-7 und 7.1 ist **genau diese JVM-Test-Phase der objektive Orakel** (beide fügen Tests in
`android/kotlin-test/` hinzu). Die Emulator-Struktur-Phase (`dumpsys window`) bestätigt für Logik-Stories
nur „App baut + startet" — sie prüft die Chunking-/Typ-Korrektheit nicht.

→ **GATE-4 „passed" = JVM-Unit-Test-Phase grün.** Das ist ideal für den Eval-Kernzweck: **objektives
Pro-Sitz-Urteil ohne Andis Ästhetik-Verdikt.**

### Bewusster Tradeoff (benennen, nicht übersehen)
Der **visuelle / strukturelle Smoke-Pfad** (Overlay-Fenster-Orakel) bleibt in diesem Dogfood
**un-validiert** — beide Stories sind Logik, nicht Surface. Falls Guardian auch den Struktur-Orakel-Pfad
durch den umgebauten Conductor validieren will, ist das ein **separater, späterer Surface-Story-Lauf**
(sinnvoll **nach** der epic-conductor-Reconciliation). Achtung: Surface-Stories ziehen Ästhetik-Verdikte
(Andi-Gate) rein → kollidiert mit dem „keine Design-Gates einbauen"-Prinzip oben. Gehört auf Guardians
Tisch, nicht in den ersten Lauf.

### baseRef = `v1-ship` @ `b29a13e` (gepinnt, verifiziert)
Für Reproduzierbarkeit auf der kanonischen Linie gepinnt. **Verifiziert** — alle nötigen Dateien existieren
auf `v1-ship` @ `b29a13e`:
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` (7.1-Ziel) ✓
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` (9-7-Ziel) ✓
- `android/kotlin-test/com/klarvo/voice/RecordingModeSilenceSelectionTest.kt` (9-7-Test-Anker) ✓
- `src-tauri/src/llm/mod.rs` (7.1-Golden-Vector-Referenz) ✓
- `scripts/android-smoke.sh` ✓

Hinweis: Die Epic-11-Android-Preview-Arbeit (Branch `fix/11-3-android-preview-box`) ist **nicht** auf
`v1-ship` — für 9-7/7.1 irrelevant, deren Ziele liegen alle auf der kanonischen Linie.

---

## Erinnerungen / offene Kanten
- **Kein Epic-Nachtlauf** auf dem neuen Prozessmodell, bis Guardian den `bmad-epic-conductor`
  reconciliiert (Briefing-Kante). Dieser Dogfood = einzelne `bmad-story-conductor`-Läufe → nicht betroffen.
- Der eigentliche **L1-Conductor-Launch in `run.sh`** (`EVAL_GATE_LOG` + `RUN_ID/TASK/COMBO` + Per-Seat-
  Modelle + Wrapper-Pfad) fehlt noch — braucht die obigen Run-Params = jetzt geliefert.
