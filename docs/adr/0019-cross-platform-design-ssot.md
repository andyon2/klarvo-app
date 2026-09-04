# ADR-0019: Cross-Platform Design Single-Source-of-Truth (Tokens · Farb-Semantik · Interaktions-Parität)

**Status:** Accepted
**Date:** 2026-06-16
**Canon-Erweiterung:** Bubble-Aufnahme-Zustand (Option A: teal + Amber-Puls-Ring + Send-Glyph) + danger=Abbrechen am 2026-06-16 in den Canon eingebaut (MANIFEST §In-repo extensions, Fingerprint `2bb99032…`).

## Context

Die App rendert dasselbe Design auf zwei Wegen, die **unabhängig** voneinander gepflegt werden:

- **Desktop (Windows):** React/Tailwind in `src/` *rendert den Canon direkt* — die `--k-*`-Tokens leben in `src/styles.css`, die Komponenten sind die Web-Umsetzung der Canon-HTML/CSS.
- **Android:** `android/kotlin-src/com/klarvo/voice/` *zeichnet den Canon von Hand nach* (Kotlin-`Canvas`, `FloatingBubbleView`/`ListeningPanelView`), weil es kein WebView ist. Jeder Farbwert, jede Geometrie, jede Geste wird abgeschrieben/neu codiert.

Daraus **zwei Drift-Flächen**, beide empirisch belegt:

1. **Token-Drift.** `KlarvoTheme.kt` ist eine handgetippte Kopie der `klarvo.css`-`--k-*`-Werte. Beweis: Story-9-5-Review-Fix **F6** korrigierte `AmberLine` von `0x4D…` (.30 α) auf `0x52…` (.32 α) — ein reiner Abschreibfehler gegen den Canon, der `rgba(233,162,76,0.32)` schon korrekt führte. Handkopierte Konstanten driften zwangsläufig.
2. **Verhaltens-/Farb-Semantik-Drift.** Es gibt **keine** geteilte Interaktions-/Semantik-Spezifikation. Beleg: das rote Danger-Quadrat ist auf **Desktop = Abbrechen** (`src/FloatingBar.tsx:167`, „Stop button … for canceling recording", ruft `cancelRecording`), auf **Android (9-5) = Senden** (`stopAndProcessRecording`). Dieselbe Farbe, gegensätzliche Bedeutung. Der Canon selbst hält in seiner Kritik fest (HTML Zeile ~206): *„Teal=Marke/Status und Orange=Aktivität sind charakteristisch, aber **nirgends als Regel festgehalten** … ohne Logik."* Und die Token-Rolle für danger ist `„Stop / Löschen / Fehler"` (HTML Zeile ~273) — „Stop" und „Löschen" zusammengeworfen, also nicht entscheidbar.

Das ist „shallow" im Sinne von *A Philosophy of Software Design*: jede Plattform reimplementiert, und die einzige geteilte Schnittstelle (der Canon) ist **nur visuell** und auf Android **nicht mechanisch erzwungen**, die Semantik gar nicht. Ein „deep module" wäre: eine schmale, geteilte Quelle (Tokens + Semantik + Interaktions-Spec), hinter der die plattformspezifische Pixel-Umsetzung versteckt liegt — und die nicht umgangen werden kann.

Verwandt: [ADR-0016](0016-android-path-parity-strategy.md) (Pfad-Parität), [ADR-0018](0018-android-bubble-rendering-tech.md) (Bubble = View+Canvas, der Grund, warum Android von Hand zeichnet), Memory `project_design_source_anchor` / `feedback_soll_anchor_external_approved_source`, Backlog `7-7` (Golden-Vector-Paritätsnetz).

## Decision

1. **Der Canon (`docs/design/overhaul/source/`) ist die einzige Quelle der Wahrheit** für (a) visuelle Tokens, (b) Farb-Semantik und (c) Interaktions-Spec — plattformübergreifend. Bisher galt das de facto nur für Desktop-Visuals; es wird auf Semantik + Interaktion + Android ausgeweitet. Kein Plattform-Code definiert eigene Design-Werte oder eigene Geste→Aktion-Bedeutungen.

2. **Visuelle Tokens werden generiert, nicht abgeschrieben.** Die Android-Token-Datei (`KlarvoTheme.kt`) wird per Codegen-Schritt aus den `--k-*`-Custom-Properties der `klarvo.css` erzeugt; Desktop konsumiert die CSS ohnehin direkt. **Kein handgetipptes Hex** in Plattform-Code. Damit kann die Token-Schicht strukturell nicht mehr driften (schließt die F6-Klasse von Fehlern).

3. **Die Farb-Semantik ist eine kodifizierte Regel** (deep statt „charakteristisch, aber ungeregelt" — schließt die Canon-eigene Kritik Zeile ~206):
   - **teal** = Marke / bereit / Verarbeitung / Erfolg / Fokus-Ring.
   - **amber** = live / Aufnahme aktiv (Tally-Light; nur während aufgenommen wird).
   - **danger (rot)** = **destruktiv: Abbrechen / Verwerfen / Löschen / Fehler** — **niemals** die primäre Bestätigen/Senden-Aktion.
   Diese Regel löst die `„Stop / Löschen"`-Doppeldeutigkeit der danger-Token-Rolle auf und korrigiert die Inversion: **rot = Abbrechen auf BEIDEN Plattformen.** Senden/Bestätigen ist eine eigene, nicht-rote Affordance (Desktop: teal-Haken; Android: Tippen auf die Bubble — siehe #4).

4. **Interaktions-Semantik ist single-source + paritätsgeprüft.** Das Geste→Aktion-Mapping und die Bestätigen/Abbrechen-Affordanzen werden einmal im Canon definiert und auf beiden Plattformen **bedeutungsgleich** umgesetzt. Für das Diktat konkret:
   - **Abbrechen** = die danger/rote Steuerung (Desktop: rotes Quadrat im Bar; Android: rotes Quadrat im Listening-Panel).
   - **Bestätigen/Senden** = die *primäre* Affordance: Desktop = teal-Haken; **Android = Tippen auf die Bubble.**
   - Die Android-Bubble braucht dafür einen **sichtbaren Aufnahme-Zustand mit Send-Affordance** (heute fehlt er — die Bubble bleibt `idle`, was „tippen = senden" nicht trägt). Der konkrete visuelle Aufnahme-Zustand der Bubble wird als Canon-Erweiterung spezifiziert (separater Design-Spec-Schritt, Provenance per #5) und ist Vorbedingung für die Neufassung von Story 9-5.
   - Parität wird erzwungen über eine geschriebene Spec + einen Review-Check jetzt; mechanisches Golden-Vector-Netz später (Backlog 7-7). Verhalten lässt sich nicht voll codegenerieren — die Regel + der Check sind der deep-module-Ersatz.

5. **Canon-Erweiterungen sind provenance-getrackt.** Der Canon ist als externes Design-Handoff ingestet (MANIFEST `sourceFingerprint`). Wird er **in-repo** erweitert (vs. externes Handoff), hält das MANIFEST das fest: neuer Fingerprint + Eintrag „in-repo erweitert durch ADR-0019, 2026-06-16, Abschnitt X". So liest `design-handoff-ingest` die Änderung als bewusste, datierte Erweiterung — nicht als unerkannte Drift. Der Canon bleibt der Anker; in-repo-Design-Entscheidungen sind erstklassig, aber protokolliert.

### Verworfene Alternativen

- **„Canon nur nachschärfen" (Cancel-Button als Feature in den Canon, sonst nichts).** Behebt das Symptom (ein Button), nicht die Ursache (zwei ungekoppelte Implementierungen). Die nächste Drift käme garantiert. Verworfen — genau der shallow-Patch, den dieses ADR vermeidet.
- **Android == Desktop pixel-/layout-identisch.** Falsch: Desktop hat eine andockende Pille, Android eine frei schwebende Bubble + Bottom-Panel. Geteilt wird **Bedeutung** (Tokens + Farb-Semantik + Geste→Aktion), nicht das Layout.
- **Voller Shared-UI-Layer (z.B. Compose Multiplatform / Tauri-Mobile).** Zu groß, [ADR-0018](0018-android-bubble-rendering-tech.md)/`project_tauri_mobile_rejected` haben native Kotlin bewusst gewählt. SSOT auf Token-/Spec-Ebene erreicht das Ziel ohne den Rewrite.

## Consequences

**Positiv**
- Token-Schicht kann strukturell nicht mehr driften (kein handgetipptes Hex; F6-Klasse erledigt).
- „Rot" bedeutet überall dasselbe (Abbrechen); die Desktop/Android-Inversion ist per Regel ausgeschlossen, nicht per Einzelfix.
- Die Canon-eigene Lücke (ungeregelte Semantik) ist geschlossen — der Canon wird von „nur visuell" zu „visuell + semantisch + interaktiv".
- Neue Oberflächen starten aus einer Quelle; Code-Review hat einen objektiven Maßstab (Token-Codegen-Diff + Semantik-Regel).

**Negativ / Kosten**
- Ein Token-Codegen-Schritt muss gebaut + im Build/CI verankert werden.
- Verhaltens-Parität lässt sich nicht voll mechanisieren — sie hängt an der geschriebenen Regel + Review-Disziplin, bis das Golden-Vector-Netz (7-7) existiert.
- Canon-Erweiterungen brauchen Provenance-Pflege (MANIFEST-Fingerprint) — bewusster Mehraufwand gegen stille Drift.

**Mitigations**
- Reihenfolge nach Hebel: (1) Token-Codegen (mechanisch, größter Hebel, billig), (2) Farb-Semantik-Regel + Bubble-Interaktions-Spec in den Canon, (3) Android 9-5 gegen die neue Spec neu fassen + Desktop-Paritäts-Check, (4) Golden-Vector-Netz später.
- Die `--k-*`-Custom-Properties sind bereits der faktische Token-Satz — der Codegen hat eine saubere, existierende Eingabe.

## Folgearbeiten (nicht Teil dieser Entscheidung, hier nur referenziert)

- Token-Codegen `klarvo.css` → `KlarvoTheme.kt` (eigene Story/Spike).
- Canon-Erweiterung: Bubble-Aufnahme-Zustand + „tap-to-send"-Affordance + danger=Abbrechen-Disambiguierung (Design-Spec, Provenance #5).
- Story 9-5 neu fassen gegen die erweiterte Spec (rotes Quadrat = Abbrechen, Bubble-Tap = Senden); der committete 9-5-Stand (`suppress-to-idle`, rot=Stop, ✗-Zusatz) wird dadurch abgelöst.
- Desktop-Paritäts-Check gegen die kodifizierte Semantik.

---

## Amendment 2026-06-17 — Modell B (Android-Aufnahme-Cluster)

**Status:** Accepted · **Trigger:** Andis Befunde aus echter Nutzung (Mockup-Review-Runde, 2026-06-17).
**Ändert:** Decision **§4** (Android-Aufnahme-Interaktion). **Unverändert:** §1–§3, §5 (Farb-Semantik, Token-Codegen, Provenance bleiben).

### Was §4 ursprünglich sagte
Senden = **Tippen auf die Bubble**; Abbrechen = rotes Quadrat im Panel; die Bubble braucht einen sichtbaren
*Aufnahme-Zustand mit Send-Affordance*.

### Befund (warum das nicht trägt)
In der Praxis entstehen daraus **zwei Probleme**: (1) **zwei konkurrierende bewegte Elemente** während der
Aufnahme — die pulsierende Bubble *und* die mitlaufende Live-Preview; (2) eine **asymmetrische Steuerung** —
Senden an der Bubble, Abbrechen im Panel = zwei getrennte Orte für eine zusammengehörige Entscheidung.

### Geänderte Entscheidung (§4′)
Während der Aufnahme gibt es **genau einen Interaktions-Ort**: einen **Steuer-Cluster am Bubble-Platz**
(die idle-Bubble wird beim Start der Aufnahme dadurch ersetzt, nicht angetippt):
- **Abschließen/Senden** = **Senden-Icon** (teal ➤) — die primäre, **nicht-rote** Affordance. **Ersetzt**
  „Tippen auf die Bubble". Ein **Haken (✓) ist hier falsch** (liest als „fertig") — der Haken ist dem
  `done`-Zustand vorbehalten.
- **Abbrechen** = **✗ (rot/danger)** im selben Cluster. Das rote Abbrechen **verlässt damit das Panel**.
- **Live-Cue** = mitlaufende **Waveform** (amber) **zwischen** Senden und Abbrechen im Cluster — die einzige
  Bewegung. Kein Puls-Ring, kein „REC"-Label.
- Das **Panel ist passiv**: nur Live-Text + Zeit (keine Waveform, kein roter Knopf). Damit konkurriert nichts
  mehr — Befund (1) ist aufgelöst.
- **`done`** = Erfolgs-**Grün + Haken**, klar abgesetzt von der teal idle-Bubble (rein visuelle
  Disambiguierung, keine Semantik-Änderung — Grün/Teal bleiben in der §3-Rolle „bereit/Erfolg").

**Konsistenz mit §3:** unverändert eingehalten — teal = bestätigen/Erfolg, amber = live, **danger/rot =
ausschließlich Abbrechen** (das ✗). Senden ist nicht-rot. Geändert hat sich nur die *Affordanz* des Sendens
(➤-Button statt Bubble-Tap), nicht die Farb-Semantik.

**Provenance (§5):** in-repo-Canon-Erweiterung. Canon-Fingerprint `2bb99032…` → **`b95f86f9b480b92c3375093bc2580d9f`**
(MANIFEST-Zeile 2026-06-17). Konkrete Surfaces im Canon: `.ab-cluster` / `.ab-cbtn.send` / `.hwave` /
`.ab-cbtn.cancel` (recording), passives `.ab-panel.rec`, grünes `.ab-bubble.done`. Abgesegneter Stand:
`docs/design/overhaul/mockup-bubble-preview-modelB.html` (Andi, 2026-06-17).

**Supersedes** in „Folgearbeiten" oben: *„Story 9-5 … Bubble-Tap = Senden"* — Senden ist jetzt der ➤-Button
im Cluster, nicht der Bubble-Tap. Die Story-Neufassung folgt dieser §4′.

### §4′-Addendum 2026-06-17 — transcribing (Variante B)

**Status:** Accepted · **Trigger:** Andis Pick aus `docs/design/overhaul/mockup-9-5-transcribing-done.html` (2026-06-17).

§4′ ließ den transcribing-Zustand (nach ➤ Senden, vor `done`) offen; der Canon-Artboard zeigte bis dahin
nur ein passives Panel mit Teal-Spinner und **leeren Dock**. Andi wählte **Variante B**: der **Dock-Platz
bleibt besetzt** — der Cluster kollabiert zu **einer teal Verarbeitungs-Bubble mit Spinner** (`.ab-bubble.proc`)
am selben Ort, an dem gerade ➤/✗ waren (Kontinuität; das Auge verliert den Ort nicht). Der Panel-Spinner
bleibt zusätzlich erhalten. **Konsistenz mit §3:** teal = Verarbeitung — amber bleibt in transcribing verboten.

**Provenance (§5):** in-repo-Canon-Erweiterung. Fingerprint `b95f86f9…` → **`efe726c6afa3cc92aff981a2e476e14c`**
(MANIFEST-Zeile 2026-06-17). Neue Surface: `.ab-bubble.proc` im transcribing-Artboard. **done-Grün (G1)** war
bereits Canon (`.ab-bubble.done` = `linear-gradient(150deg,#62E0A4,var(--k-success))`) — Andis Pick deckt sich,
keine Canon-Änderung. Abgesegneter Stand: `mockup-9-5-transcribing-done.html` (Andi, 2026-06-17).

### §4′-Amendment 2026-06-21 — 9-5 GATE Follow-ups #2 + #4

**Status:** Accepted · **Trigger:** Andis Picks am 9-5-GATE-4-Gate (siehe `docs/backlog.md` „Story 9-5 GATE-4 green"), abgesegnet via Render `docs/design/overhaul/mockup-9-5-followups-2-4.html` (Andi, 2026-06-21).
**Ändert:** verfeinert §4′ (Cluster-Geometrie + Gesten-Modus-Variante). **Unverändert:** §1–§3, §5 (Farb-Semantik, Token-Codegen, Provenance), die §4′-Kern-Entscheidung „ein Interaktions-Ort, Senden = ➤, rot = Abbrechen, Panel passiv" und das transcribing-Addendum.

**(#2) Cluster-Reihenfolge getauscht.** Der recording-Cluster ist jetzt **`[✗ Abbrechen (links) · Waveform · ➤ Senden (rechts)]`** statt zuvor `[➤ · Waveform · ✗]`. Grund: Daumen-Gewohnheit — die primäre **➤-Senden**-Affordanz gehört an den Bildschirm-Platz, an dem gerade die idle-„K"-Bubble getippt wurde (das rechte Dock-Ende), nicht das rote ✗. Die §4′-Regel „Waveform **zwischen** Senden und Abbrechen" bleibt erfüllt. Farb-Semantik unverändert (teal=Senden, amber=live, rot=Abbrechen).

**(#4) HOLD-Modus (Push-to-Talk) bekommt eine eigene Variante.** Der Tap/Toggle-Cluster passt nicht für **Hold**: dort **sendet Loslassen bereits** → ein separater ➤ ist redundant und ✗ unerreichbar (zum Tippen müsste man loslassen = sendet vorher). Stattdessen das vertraute Sprachnachricht-Modell:
- **halten = aufnehmen** · **loslassen = senden** · **wegziehen = abbrechen** (kein tippbares ➤/✗, solange der Finger hält).
- **hoch ziehen → 🔒 sperren** wandelt in den normalen Tap-Cluster `[✗ · Waveform · ➤]` (Reihenfolge aus #2), damit man loslassen kann, ohne zu senden.
- Live-Cue bleibt die amber Waveform; Halte-Ring amber. Tap/Toggle/Auto-Stop/Auto nutzen weiter den Cluster (§4′ unverändert).
- **Bezug 9-7 (Gesten-Modi):** eigene Build-Story; 9-7 wird **nicht** still erweitert.

**(#1-Anker, keine Geometrie-Änderung) Waveform ist RMS-getrieben.** Die Cluster-/HOLD-Waveform (`.hwave`) ist im Build ein **von der echten Stimm-Amplitude (RMS) getriebener** Live-Cue (wie Desktop), nicht die im Canon nur illustrative idle-Animation. Im Canon als CSS-Kommentar festgehalten; Realisierung = Build-Story **Follow-up #1** (`docs/backlog.md`).

> **Realisiert 2026-06-21 (Story 9-12, Andi real-device GATE-4 approved).** Der Android-Cluster-Waveform
> ist als **Desktop-Paritäts-Algorithmus** gebaut: eine **scrollende 20-tiefe Amplituden-Historie** (1:1 zu
> `src/FloatingBar.tsx`), kein synthetischer Cosinus-Sweep mehr. **Bei Stille flach/still** (alle Pegel 0 → Min-Balken)
> — das präzisiert die „nicht idle-Animation"-Regel: der frühere Eindruck „Balken frieren nie ein" ist **überholt**
> (Desktop friert bei Stille ein, Android jetzt auch). Verlauf: erst RMS-Feed kalibriert (`smoothedAmplitude`-Noise-Floor
> 0.04→0.012 war der echte Defekt, nicht die Zeichen-Formel), dann Stille-Kopplung, dann Desktop-Port. Feinschliff
> (Cross-Mic-Robustheit der Magic-Numbers, Panel-Waveform-Abgleich) geparkt → `docs/backlog.md`. Lehre: visuelle
> *Bewegung* ist NUR am echten Gerät verifizierbar (Emulator/Harness setzen Amplitude direkt, am Mikro vorbei).

**Provenance (§5):** in-repo-Canon-Erweiterung. Fingerprint `efe726c6…` → **`fc9ef7456700d19b8332dd2c34a43b8e`** (MANIFEST-Zeile 2026-06-21). Geänderte/neue Surfaces: recording-`.ab-cluster` (Reihenfolge), neu `.ab-holddock`/`.ab-holdstrip`/`.ab-slidehint`/`.ab-heldbub`/`.ab-lockchip` + Artboard-Sektion „Aufnahme · HOLD-Modus". Abgesegneter Stand: `mockup-9-5-followups-2-4.html` (Andi, 2026-06-21).

### Amendment 2026-06-26 — Android-Aufnahme-Steuerung: Mobile-Redesign („B-Sprache")

**Status:** Accepted · **Trigger:** Andis **erster echter Daumen-Test** des HOLD-Modus (Story 9-14) am echten Gerät: die §4′-/§4′-Amendment-Umsetzung fiel am Gerät durch — **zu klein, Finger verdeckt die UI beim Ziehen, „fühlt sich an wie ein Laptop-Feature, nicht wie ein Handy-Feature"** (Maschinen-Ebene war grün). Wurzel: der Canon wurde als Browser-Mockups im Laptop-Maßstab abgesegnet, nie am Daumen/Geräte-Maßstab. Mobile-Overlay-Rethink (Phase A) → neue Design-Sprache, **abgesegnet im Geräte-Maßstab (1080×2460 @ 2.75)** über belebtem Hintergrund.

**Ändert:** die **Android-MOBILE** Umsetzung der recording-Steuerung (der §4′-Klein-Cluster `[✗·Waveform·➤]` **und** die §4′-Amendment-HOLD-Slide-Variante). **Unverändert:** §1–§3, §5 (Farb-Semantik **teal=Senden · amber=live · rot=Abbrechen**, Token-Codegen, Provenance); **alle Desktop-Surfaces**; die HOLD-Kern-Intention (halten=aufnehmen · loslassen=senden · wegziehen=abbrechen · hochziehen=sperren) — nur **Surface, Geometrie, Größe und Feedback** ändern sich.

**Kern-Entscheidung (B-Sprache) — mobile-first, occlusion-bewusst, solide Flächen, große Targets:**
- **TAP-Aufnahme** (kurz tippen): zwei **große runde tappbare Ziele** — **Senden** (teal-Gradient, ➤) am **Dock/Daumen**, **Abbrechen** (dunkel + rot-Ring, ✕) auf der Gegenseite; dazwischen/oben ein ruhiger **Waveform-Chip** (amber, RMS-getrieben). **Ersetzt den `.ab-cluster`-Klein-Cluster.**
- **HOLD** (halten): **Daumen-Anker-Bubble** (teal, amber-Ring) am Dock + zwei große runde Ziele — **Sperren** (teal, Schloss-Icon, oben-zur-Display-Mitte) + **Abbrechen** (rot, ✗, weiter unten). **Das Ziel wächst + leuchtet, sobald der Finger drauf ist** (klarer Treffer-Cue); **Loslassen löst aus** (release-to-commit); **Zurückziehen vor dem Loslassen = Undo**. Loslassen ohne Ziel = **senden**.
- **Gesperrt** (nach Hochziehen-Sperren): wird zur **TAP-Surface** (Senden + Abbrechen, tappbar) — Loslassen sendet dann nicht mehr.
- **Dock-adaptiv:** Bei allen Andock-Positionen (rechts/links/oben/unten/frei) **spiegelt/dreht** sich die Anordnung; Cues wachsen **weg von der angedockten Kante und vom Daumen**, nie unter den Finger.
- **Größen-/Kontrast-Regeln:** Targets groß genug, dass sie neben dem Daumen sichtbar bleiben (Richtwert Ziel-Ø ≥ ~120dp, großzügiger Abstand); alles auf **blickdichten Flächen** — nie auf Transparenz verlassen (das war der „Lock auf transparentem Grund unlesbar"-Defekt).

**Bindendes Render (SOLL):** `docs/design/overhaul/mockup-mobile-hold-B-refined.html` (Ruhe + Treffer-Abbrechen) **+** `docs/design/overhaul/mockup-mobile-recording-states.html` (TAP-Aufnahme + HOLD-Treffer-Sperren + Dock-Spiegelung). Andi-approved 2026-06-26, gerendert @ 1080×2460 (Playwright, Geist-Fonts, belebter Hintergrund). Exakte Werte (Farben/Radii/Größen) = die CSS dieser Mockups.

**Superseded (für Android-MOBILE; Desktop unberührt):** die §4′-Amendment-Surfaces `.ab-cluster` (Klein-Cluster-Geometrie) + `.ab-holddock`/`.ab-holdstrip`/`.ab-slidehint`/`.ab-heldbub`/`.ab-lockchip` (Slide-Spur-HOLD). Im tracked Canon `Klarvo Design System.html` als SUPERSEDED markiert; ein Consumer liest die neue Wahrheit aus den beiden Mockups + diesem Amendment.

**Scope/Stories (nur geschrieben — Build folgt in frischer Session):** **9-14** neu gefasst (HOLD in B-Sprache); neue **9-15** (TAP-Surface-Re-Skin = ersetzt den 9-13-Klein-Cluster; liefert auch den Gesperrt-Zustand für 9-14). Transcribing/Done/Idle-Bubble + Live-Preview = **späterer Pass** (docs/backlog.md), bewusst nicht in diesem Amendment.

**Provenance:** bindendes Render = die zwei Mockups (eine Ebene über `source/`); tracked-Canon-`.ab-*`-Mobile-Surfaces als SUPERSEDED markiert (MANIFEST-Zeile 2026-06-26).

---

## Amendment 2026-07-01 — HOLD vereinfacht: ein Abbrechen-Button · Senden = Loslassen · kein Sperren (Andi-approved, device-tested)

**Kontext:** Die Zwei-Ziel-HOLD-Variante (Sperren + Abbrechen, B-refined, Amendment 2026-06-26) wurde gebaut (Story 9-14, commits `ce20bb0`/`c431ba5`) und **am echten Gerät verworfen**: (a) alles zu groß (feste Mockup-dp 82/112/148 — gleiche Provenienz-Falle wie 9-15s erster Build: Browser-Mockup ≠ Geräte-Maßstab), (b) das Anker-K springt in Größe+Position gegenüber der ~44dp Idle-Bubble, (c) **Design-Erkenntnis von Andi:** „Loslassen außerhalb von Abbrechen = senden" *ist* schon das Senden — es braucht **keinen** eigenen Senden-Button und keine zwei gleichberechtigten Ziele. Die Totzone zwischen zwei Zielen (Loslassen dort = unbeabsichtigtes Senden) verschwindet mit nur einem Ziel.

**Entscheidung (supersedet die HOLD-Sektion des 2026-06-26-Amendments):**
- **Halten** = aufnehmen. **Kleine Anker-Bubble** (K) am Dock, **= Idle-Bubble-Größe** (`bubbleSizeDp`, ~44dp responsive) — kein Größen-/Orts-Sprung; entkoppelt vom Button-Regler.
- **Loslassen** (überall außer auf Abbrechen) = **senden**. Kein Senden-Button, keine Totzone.
- **Ein** bewusster **Abbrechen-Button** (✗, rot, dunkle Ruhe-Fläche → wächst + leuchtet rot bei Treffer, „loslassen = abbrechen"). Größe **am `recordingButtonSizeDp`-Regler** (der erweitert wird: mehr + kleinere Stufen, nah an Idle-Größe).
- **Zurückziehen** vom Button vor dem Loslassen = Undo (nichts).
- **Kein Sperren** in HOLD (Freisprech-Aufnahme = TAP-Modus). Die gesamte Sperren-/Lock→TAP-Mechanik aus 9-14/9-15 entfällt für HOLD.
- **Dynamik (Teil des SOLL, diesmal bauen):** Ghost-Bubble folgt dem Finger · Origin-Bubble faded auf .32 beim Ziehen · Caption wechselt auf „Finger auf Abbrechen · loslassen löst aus".

**Bindendes Render (SOLL):** `docs/design/overhaul/mockup-mobile-hold-simple.html` (Frames `sRest` Ruhe + `sHit` Treffer), fingerprint `7e2829a5625c224fb2227cff53cefa70`, gerendert @ 1080×2460 (Playwright), Andi-approved 2026-07-01. **Supersedet** `mockup-mobile-hold-B-refined.html` (`bRest`/`bHit`) für HOLD.

**Unverändert:** TAP-Surface (9-15), Farb-Semantik (teal=Senden · amber=live · rot=Abbrechen), blickdichte Flächen, dock-adaptiv, kein `FLAG_NOT_TOUCHABLE`.

---

## Amendment 2026-07-01 #2 — non-HOLD-Aufnahme zurück zum Kompakt-Cluster (TAP-Surface verworfen, Andi-approved)

**Kontext:** Nach der Geräte-Abnahme von 9-14 (HOLD) + 9-15 (TAP) meldete Andi: die **großen TAP-Ziele mit Text** („Senden"/„tippen"/„Abbrechen") für die **non-HOLD-Modi** gefallen ihm nicht — er will **zurück zur Vorgänger-UI: kleine Symbole ohne Text, direkt links und rechts der Waveform** (der `.ab-cluster`-Klein-Cluster, der vor 9-15 live war). HOLD (Amendment #1 oben) bleibt unverändert.

**Entscheidung (supersedet die TAP-Surface aus dem 2026-06-26-Amendment — nur für non-HOLD):**
- **non-HOLD-Aufnahme** (TAP-Modi: kurz tippen, AUTO etc.) = **Kompakt-Cluster** `[✗ Abbrechen · amber Waveform · ➤ Senden]` — kleine Symbole **ohne Text**, feste Größe (`CLUSTER_VISUAL_W_DP=150 × CLUSTER_VISUAL_H_DP=52`), 1-D-X-Band-Touch-Zonen. Reihenfolge = post-9-13 (Abbrechen LINKS, Senden RECHTS).
- **Größen-Regler** (`recordingButtonSizeDp`): bleibt in den Settings, wirkt aber **nur noch auf den HOLD-Abbrechen-Button** — der Cluster ist fix (Andi-Entscheidung 2026-07-01).
- **HOLD** (Amendment #1): unverändert.

**Implementierung (reversibel, symmetrisch):** `drawRecordingCluster` (war seit 9-15 toter Code) wieder live via onDraw-Dispatch; `drawTapSurface` + 2D-Kreis-Touch-Zonen jetzt toter Code (behalten). JVM-Build + Unit-Tests grün; GATE-4 (Sicht/Touch am echten Gerät) = Andis Runde via `scripts/android-smoke.sh`.

**Superseded (nur Android non-HOLD):** die TAP-Surface (`.ztap`-Große-Kreise mit Labels, `mockup-mobile-recording-states.html` Frames `tapRight`/`tapLeft`) — der Kompakt-Cluster ist wieder Canon für non-HOLD. **Unverändert:** HOLD-Surface, Farb-Semantik, Waveform (RMS, 9-12), dock-Verhalten, kein `FLAG_NOT_TOUCHABLE`.

**Scope/Story:** **9-16** (non-HOLD-Cluster-Revert). Bindende Quelle = der historische `.ab-cluster` (git `e92f4f3`, pre-9-15) + dieses Amendment.

---

## Amendment 2026-09-04 — Rot bleibt zerstörerisch: `Undo` wird amber · der bestätigte Zustand ist eine benannte Ausnahme

**Status:** Accepted · **Trigger:** Epic-8-Retro 2026-09-03, Befund 1 („Canon and ADR-0019 disagree on red").
**Ändert:** die Regel nicht. Dieses Amendment hält eine Abweichung fest, benennt ihre Rücknahme und schreibt eine Ausnahme auf, die bisher nur im Canon stand.
**Berührte Decision-Abschnitte:** **§3** (Farb-Semantik) und **§5** (Provenance von Canon-Erweiterungen).

> **Nummern-Korrektur.** Die Retro und der Routing-Hook sagen, die amber-Entscheidung stelle „§5" wieder her.
> Das ist eine Verwechslung. Die Farb-Semantik-Regel steht in **§3**. §5 regelt die Provenance.
> Wer den Retro-Satz liest, meint §3. Beide Abschnitte sind hier berührt, aber wiederhergestellt wird §3.

### 1. Die Ausnahme: `.note .acts.responding` (bleibt gültig)

Story 8-8 baute die Aktions-Rückmeldung der Desktop-History. Die Aktionszeile `.note .acts` erscheint
normalerweise nur beim Hover (`.note:hover .acts`). Andi entschied am 2026-08-21 am echten Windows-Bildschirm
zwei Dinge, die von dieser Regel abweichen:

- **Der bestätigte Zustand überlebt das Verlassen der Karte.** Ohne diese Ausnahme sind die 1,5 s „Copied"
  nicht lesbar — der Zeiger wandert weiter, die Zeile verschwindet, die Bestätigung geht mit ihr. Der Canon
  schreibt das als `.note .acts.responding`.
- **`Delete` tritt zur Seite, solange die Karte antwortet** (`.acts.responding .act.del` → `opacity: 0` +
  `pointer-events: none`). Ein zerstörender Knopf darf nicht neben einer Erfolgsmeldung stehen, in die der
  Zeiger zielt. Das Ausblenden läuft über Deckkraft, **nicht** über `display` — die Zeile behält ihre Breite,
  die für 8-8 AC1 gemessene Geometrie bleibt gültig.

**Beide Punkte gelten weiter.** Sie berühren die Farb-Semantik nicht; sie regeln Sichtbarkeit und
Zeiger-Ziel. Dieses Amendment schreibt sie in das ADR, damit eine spätere Story sie nicht als Drift
„repariert". Provenance: MANIFEST-Zeile 2026-08-21, Fingerprint `028171af056a13030fe80adc54eae738`.

### 2. Die Episode: `Undo` wurde rot (2026-08-21) — und war damit gegen §3

Bei derselben Abnahme entschied Andi als dritten Punkt: `.note.deleted .undo` trägt `--k-danger` statt
`--k-teal`. Er las den „Deleted · Undo"-Streifen als Teil derselben Lösch-Handlung. Gebaut in `4a1a282`.

Das **widerspricht §3**. `Undo` ist kein zerstörendes Steuerelement, sondern das Gegenteil: die Rückhol-Affordanz.
Der Canon sagt es in eigenen Worten (Design-System-HTML, „Prinzipien"): *„Rot = zerstörerisch — Stop, Löschen,
Fehler — sonst nie."* Die MANIFEST-Zeile vom 2026-08-19 hatte den Streifen aus genau diesem Grund bewusst
**nicht** rot gefärbt; die 08-21-Zeile überstimmte diese Begründung, ohne §3 zu berühren.

Die Retro fand daraus eine konkrete Falle: **eine spätere Story liest §3, sieht Rot am `Undo` und
„repariert" die Farbe** — ohne zu wissen, dass ein Mensch sie so abgenommen hat. Die Quellen widersprachen
sich still.

### 3. Die Entscheidung (Andi, 2026-09-03): `Undo` wird amber

`.note.deleted .undo` trägt **`--k-amber` (`#E9A24C`)**, nicht `--k-danger`. Das überstimmt die Abnahme vom
2026-08-21 und **stellt §3 wieder her**: Rot bleibt dem zerstörenden Steuerelement vorbehalten.

Amber passt auch positiv, nicht nur als Ausweichfarbe: der Streifen ist ein befristeter, laufender Zustand
(6 s Frist, in der die App noch nicht wirklich löscht). Amber trägt in diesem Design genau die Bedeutung
„läuft gerade" (§3: *amber = live / aktiv*).

**Zustand heute — die Quellen weichen bewusst ab:**

| Quelle | Farbe des `Undo` | Warum |
|---|---|---|
| ADR-0019 (dieses Amendment) | amber | Entscheidung vom 2026-09-03. |
| Canon (`klarvo.css`, MANIFEST `028171af…`) | rot | Der Canon folgt erst, **nachdem** Andi amber am echten Windows-Bildschirm gesehen hat. |
| Code (`src/App.tsx`) | rot | Nicht gebaut. Eigener Follow-up. |

Diese Abweichung ist **beabsichtigt und befristet**, nicht Drift. Der Canon hält nur fest, was Andi gesehen hat
(§5 gilt unverändert: eine Canon-Erweiterung braucht Provenance, und Provenance braucht einen Blick auf den
echten Bildschirm). Der Follow-up steht in `docs/backlog.md` („Undo wird amber").

**Für die Story, die das baut:** ändere Code **und** Canon in einem Zug, lass Andi amber am Windows-Bildschirm
abnehmen, und trage dann eine neue MANIFEST-Zeile mit neuem Fingerprint nach. Erst danach stimmen alle drei
Spalten oben überein.

**Unverändert:** §1, §2, §4, §5. Die Farb-Semantik aus §3 selbst wird nicht geändert — sie wird angewandt.
