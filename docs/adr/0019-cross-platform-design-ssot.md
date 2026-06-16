# ADR-0019: Cross-Platform Design Single-Source-of-Truth (Tokens · Farb-Semantik · Interaktions-Parität)

**Status:** Proposed
**Date:** 2026-06-16

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
