# Design-Handoff-Brief — Android Bubble + Preview-Box (Evolution)

**Status:** DRAFT — Schwächen-Abschnitt von Andi auszufüllen, dann an den Design-Agenten.
**Erstellt:** 2026-06-17 · **Route:** B (Canon evolviert zuerst, dann Stories) · **Epic:** 9 (Android Visual Overhaul)

---

## 0. Wofür dieser Brief

Du (Design-Agent) lieferst ein **verbessertes HTML/CSS-Mockup** der Android-Bubble- und Preview-Surfaces.
Wir ingesten es danach als neuen verbindlichen Design-Canon (`design-handoff-ingest`), und erst gegen
diesen neuen Canon werden Implementierungs-Stories geschrieben. **Du definierst hier die Optik — nicht die
nachgelagerte Codeseite.**

Das ist eine **Evolution**, kein Neuanfang: Die Surfaces existieren im aktuellen Canon schon. Andi findet die
*Design-Sprache dieser Surfaces* an Stellen schwach (siehe §4). Deine Aufgabe ist, sie stärker zu machen —
**innerhalb** der unten festgelegten Leitplanken.

---

## 1. Nicht verhandelbar (Leitplanken)

Diese Punkte sind durch **ADR-0019** (Cross-Platform Design SSOT) gesetzt. Ein Mockup, das sie verletzt, ist
nicht ingest-fähig. Bitte **aus dem bestehenden Canon heraus** evolvieren, nicht von Null:

1. **Basis = aktueller Canon.** Geh von `docs/design/overhaul/source/Klarvo Design System.html` +
   `assets/klarvo.css` aus (Fingerprint `2bb99032…`). Du lieferst die **ganze** weiterentwickelte Datei
   zurück — inklusive aller heutigen Surfaces, nichts wegwerfen.
2. **Tokens: das `--k-*`-Set ist die Palette.** Keine freien Hex-Werte. Brauchst du einen neuen Wert,
   führ ihn als neues `--k-*`-Custom-Property ein (er wird später nach Android codegeneriert — handgetipptes
   Hex driftet, deshalb generiert).
3. **Farb-Semantik ist eine Regel, kein Geschmack:**
   - **teal** = Marke / bereit / Verarbeitung / Erfolg / Fokus-Ring
   - **amber** = live / Aufnahme aktiv (Tally-Light — nur *während* aufgenommen wird)
   - **danger/rot** = **destruktiv: Abbrechen / Verwerfen / Löschen / Fehler — NIEMALS Senden/Bestätigen**
4. **Interaktions-Semantik (Diktat):**
   - **Abbrechen** = die rote Steuerung (rotes Quadrat im Panel).
   - **Senden/Bestätigen** = primäre, **nicht-rote** Affordance. Auf Android: **Tippen auf die Bubble.**
     Die Bubble braucht dafür einen sichtbaren *Aufnahme-Zustand mit Send-Affordance*.
5. **Form-Konstanz:** Die Bubble ist ein **Squircle** (`border-radius:12px` auf ~40×40), **kein Kreis** und
   **kein Kreis↔Quadrat-Morph** über die Zustände. Form bleibt konstant; nur Füllung/Ring/Glyph wechseln.
6. **Android-Physik (gegenüber Desktop):**
   - System-Nav-Bar-Clearance einhalten (Panel sitzt über der Bottom-Nav).
   - Touch-Targets ≥ **48dp** (transparentes Padding zählt).
   - **Kein WebView-Backdrop-Blur** verfügbar → der „Glas-Ring" wird durch einen ~3–4dp Teal-Ring ersetzt.
     Entwirf so, dass kein echtes Blur nötig ist.
   - Die Bubble erscheint **nur bei fokussiertem Textfeld + offener Tastatur** (IME-Kontext). Denk den
     Tastatur-Kontext mit (Platzierung, Daumen-Reichweite).

> Diese Leitplanken sind genau die, die schon teuer gelernt wurden (Token-Drift, rot=Abbrechen-Inversion,
> Kreis-vs-Squircle-Drift in 9-3). Sie sind der Grund, warum wir *einen* Canon haben.

---

## 2. Scope

**In Scope (diese Surfaces evolvieren):**

| Canon-Selektor | Was es ist |
|---|---|
| `.ab-bubble.idle` | Bubble im Ruhezustand (Teal-Squircle, dunkles „K") |
| `.ab-bubble.recording` | Bubble während Aufnahme (Send-Affordance = „tippen zum Senden") |
| `.ab-bubble.done` | Bubble-Erfolgs-/Abschlusszustand |
| `.ab-panel.rec` | **Die „Preview-Box": Recording-Panel** — K-Icon, Amber-Waveform, Timer, rotes Abbrechen-Quadrat, **Live-Rohtext**, Footer |
| `.ab-panel` (clean) | Panel mit **bereinigtem** Text (Cleanup-Ergebnis) |
| Long-Press Quick-Menü | (HTML ~Zeile 789) — falls von Schwächen betroffen |

**Out of Scope:** Desktop-Surfaces (eigener Strang), Light-Theme (out per Design-Constraint D3),
reine Bugfixes/Funktionsfehler (das ist Code, nicht Design), Bubble-Rendering-Tech (steht: View+Canvas).

---

## 3. Baseline (das „Vorher", das du verbesserst)

Rendere die Ist-Canon-Surfaces für den Direktvergleich (oder nutze die beigelegten PNGs):

```
node ~/.claude/skills/design-handoff-ingest/render-surface.mjs \
  --html "docs/design/overhaul/source/Klarvo Design System.html" \
  --selector ".ab-panel.rec" --out /tmp/soll/ab-panel-rec.png --scale 3
```

Kurzbeschreibung der heutigen `.ab-panel.rec` („Preview-Box"): oben K-Squircle · Amber-Aufnahme-Indikator
(Punkt + 5 Waveform-Balken) · Timer `0:06` · rotes Abbrechen-Quadrat rechts. Darunter Live-**Rohtext** in
Monospace. Footer: „Tastatur pausiert · kehrt beim Einfügen zurück".

> Hinweis: Die *gebaute* Android-App kann zusätzlich von diesem Canon abweichen (Fidelity-Drift). Für diesen
> Brief zählt der **Canon** als Baseline — wir evolvieren die Design-Sprache, nicht den Build-Stand.

---

## 4. Befunde (Andi, 2026-06-17)

> **Wichtig:** ② (recording) und ④ (Panel) sind *keine* zwei unabhängigen Optik-Punkte — sie sind **eine
> Entscheidung**: der Interaktions-Ort während der Aufnahme. Die ist **ADR-0019-Level** (sie überstimmt §4
> „sichtbarer Aufnahme-Zustand *mit Send-Affordance in der Bubble*") und wird **vor** dem Handoff gelockt
> (→ §5). Die rein-visuellen Richtungen (② Farbe, ③ done) gelten modell-unabhängig.

**② Bubble — recording.** Das „Pfeil/Senden"-Icon vermittelt die falsche Botschaft; ein nur pulsierender Rand
macht nicht klar, dass gerade *aufgenommen* wird. Der Inhalt soll **Aufnahme** signalisieren, nicht Senden —
mitlaufende kleine Waveform (wie Preview) **oder** oranger Aufnahmekreis. Einfachste Variante: die idle-Bubble,
aber in **Orange pulsierend**.

**③ Bubble — done.** Hebt sich zu wenig vom idle-K ab (andere Form, aber gleiche Farbe). Wenn recording = das
**orange** K ist, dann done → **Standard-Grün + Haken**.

**④ Preview-Box / Recording-Panel — der Kern.** Aktuell **konkurrieren zwei Elemente** während der Aufnahme:
die pulsierende Bubble *und* die sich bewegende Live-Preview. Dazu eine unsinnige Asymmetrie: **Senden** =
Bubble drücken, **Abbrechen** = roter Knopf in der Preview — zwei getrennte Orte. **Es darf nur EINEN
Interaktions-Ort geben.** Zwei Auflösungen stehen im Raum → §5.

**⑤ Panel — clean.** Wenn die Interaktion an *einem* Ort gebündelt ist (z. B. an der Bubble), ist die Preview so
okay — **ohne** Interaktions-Elemente, **aber mit Zeitangabe**.

**⑥ Long-Press / ⑦ Diktat-Ende.** Kein eigener Long-Press-Befund. Das Diktat-Ende ist in ②/④ beantwortet (→ §5).

---

## 5. Die Kern-Entscheidung: EIN Interaktions-Ort (vor dem Handoff zu locken)

Andis Befund ④/⑤ verlangt: **genau ein** Ort für Bestätigen/Abbrechen während der Aufnahme — keine
konkurrierenden Animationen, keine Senden-hier/Abbrechen-dort-Asymmetrie. Das **überstimmt ADR-0019 §4**
(„Send-Affordance in der Bubble") und braucht ein **ADR-0019-Amendment**. Zwei Modelle stehen im Raum:

**Modell A — Panel ist das Cockpit (Bubble verschwindet während der Aufnahme).**
Sobald aufgenommen wird, blendet die Bubble aus. Das Panel trägt alles: Live-Text · Zeit · Aufnahme-Indikator
(Waveform/oranger Puls) · **✓ Senden (teal)** + **✗ Abbrechen (rot)** nebeneinander. Idle: Bubble da. Done:
Bubble grün + Haken. → Entspricht Andis „Haken-Icon in die Preview".

**Modell B — Bubble (bzw. ihr Platz) ist das Cockpit (Preview ist reine Anzeige).**
Die Preview zeigt nur Live-Text + Zeit (keine Waveform, kein roter Knopf). Am Platz der Bubble sitzt die
Steuerung: **✓ Senden** + **✗ Abbrechen** (z. B. die Bubble wird zu einem ✓/✗-Paar, orange akzentuiert).
→ Entspricht Andis „Interaktion in die Bubble / an den Platz der Bubble, Preview ohne Interaktion, mit Zeit".

> **ENTSCHEIDUNG: Modell B — gelockt 2026-06-17 (Andi).** Bubble/ihr Platz = Cockpit; Preview = reine Anzeige.
> Wird als **ADR-0019-Amendment** (`bmad-correct-course`) protokolliert (überstimmt §4 „Send-Affordance in der
> Bubble" → jetzt ✓/✗-Cluster am Bubble-Platz; das rote Abbrechen-Quadrat verlässt das Panel).

### 5a. Modell-B-Spezifikation für den Design-Agenten (Zustand für Zustand)

**idle** — unverändert: Teal-Gradient-Squircle, dunkles „K". Erscheint bei fokussiertem Feld + Tastatur.
Antippen startet die Aufnahme.

**recording** — die Bubble wird zum **Senden/Abbrechen-Steuer-Cluster** am selben Anker:
- **Abschließen/Senden** = **Senden-Icon** (teal, primär — Papierflieger; **kein Haken**: ✓ liest als „fertig"
  und ist dem done-Zustand vorbehalten) + **✗ Abbrechen** (rot/danger), nebeneinander, je ein Squircle, je ≥48dp.
- **Aufnahme-Sichtbarkeit (Befund ②, Andi 2026-06-17):** Der Live-Cue ist eine **mitlaufende Waveform**
  (Amber-Balken, wie früher in der Live-Preview), platziert **zwischen Senden und Abbrechen im Cluster**
  (Variante B aus dem Platzierungs-Review — als ein Bauteil, nicht freischwebend; **nicht** „REC"-Label,
  **nicht** Puls-Ring). Der Cluster-Ring ist ein ruhiger statischer Amber-Akzent; **die einzige Bewegung ist
  die Waveform** (das Panel bleibt ruhig — Andis Kernziel).
- Form-Vokabular bleibt Squircle (kein Kreis); die bewusste Änderung ggü. ADR-0019 C1 ist *ein Element → zwei
  Steuer-Elemente*, nicht die Form.

**done** — **Standard-Grün + Haken** (Befund ③), deutlich abgesetzt vom idle-Teal-K und vom orangen recording.
Kurze Bestätigung, dann zurück zu idle.

**Preview-Panel während recording (Befund ④/⑤)** — **reine Anzeige, keine Interaktion:**
- nur **Live-Text + Zeitangabe**. **Kein** rotes Abbrechen-Quadrat, **keine** Waveform (die lebt am Cluster),
  **kein** Senden-Button.
- Damit bewegt/pulsiert während der Aufnahme **nur** der Steuer-Cluster — die zwei konkurrierenden Elemente
  sind aufgelöst (Andis Kernbefund).
- raw→clean: der Text wird weiterhin roh angezeigt und beim Bereinigen ersetzt; das Panel bleibt passiv.

**Offen für den Design-Agenten (Ergonomie, nicht Semantik):** Daumen-Reichweite & Anker des ✓/✗-Clusters bei
offener/pausierter Tastatur; visueller Abstand Cluster↔Panel, damit beide klar getrennt lesbar sind.

> Entschieden mit Andi (2026-06-17, Mockup-Review-Runde 1): Live-Cue = **mitlaufende Waveform am Cluster**
> (nicht Orange-Puls/„REC"); Senden-Glyph = **Senden-Icon, kein Haken** (Haken nur für done). Mein früherer
> „keine-Waveform"-Default ist damit überholt — die Waveform lebt am Cluster, das Panel bleibt ruhig.

---

## 6. Lieferformat (damit der Ingest sauber läuft)

`design-handoff-ingest` konsumiert ein **HTML+CSS-Bundle** in derselben Struktur wie der aktuelle Canon:

- **Eine** „Klarvo Design System"-HTML-Datei (Render-Wahrheit) + `assets/klarvo.css` (Wert-Wahrheit).
- **Selektor-Namen beibehalten** (`.ab-bubble.idle/.recording/.done`, `.ab-panel`, `.ab-panel.rec`,
  `.ab-panel-text/-grip/-foot`, `.phone`-Frame …) — neue Surfaces mit konsistenten, sprechenden Klassen, damit
  `render-surface.mjs` jede Surface einzeln rendern kann.
- **Tokens nur in `klarvo.css`** als `--k-*`. Keine Inline-Hex.
- **Nichts regressieren:** die heutigen In-Repo-Erweiterungen müssen erhalten bleiben — `.ab-bubble.recording`
  (teal-Squircle + Amber-Puls-Ring + Send-Glyph), danger=Abbrechen, die Bubble-State-Sequenz.
- Ablegen als Bundle im (gitignored) Roh-Inbox-Verzeichnis `design-handoff/` — ich promote es von dort.

---

## 7. Was danach passiert (mein Teil)

1. `design-handoff-ingest` → neuer getrackter Canon unter `docs/design/overhaul/source/` + MANIFEST-Fingerprint
   + Provenance-Eintrag (ADR-0019 §5).
2. **ADR-0019-Amendment** für die in §5 getroffene Interaktions-Entscheidung (via `bmad-correct-course`).
3. `bmad-create-story` gegen den **neuen** Canon, gruppiert unter Epic 9 — jede Story mit objektivem visuellem
   Ziel; GATE-4 für Bubble-Optik = Andis Auge am echten Gerät.
