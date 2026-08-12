# GATE-4 — Story 11-6 (Zeilenabstand als Appearance-Setting)

**Datum:** 2026-08-11 · **Branch:** `feat/11-6-line-spacing-setting` · **Range:** `86b5dca..HEAD`

## Verdikt

| Teil-Gate | Status | Grundlage |
|---|---|---|
| **GATE-4b — Android (AC-2 UI, AC-4 Render)** | **GRÜN** | Objektiv gemessen am echten Gerät, s. u. — danach revidiert, s. „Revision" |
| **GATE-4a — Windows (AC-3 GDI-Render)** | **GRÜN** | Von Andi am echten Release-Build abgenommen, 2026-08-11 |

> **Achtung, Reihenfolge:** die Messwerte weiter unten sind der Stand **vor** der Revision vom
> selben Tag. Die aktuell ausgelieferte Skala steht im Abschnitt „Revision" am Ende.

## Zielgerät

Redmi Note 12T Pro (`23054RA19C`, HyperOS), 1080×2460 @ density 440.
APK `Klarvo-v0.5.0-20260811-0912.apk` (Debug-Variante), installiert 2026-08-11.
**Freshness belegt:** `lastUpdateTime` 2026-07-09 13:44 (Basislinie) → 2026-08-11 11:12.

## Messverfahren (kein Augenmaß)

Screenshot → ffmpeg schneidet den Transkript-Bereich als 8-Bit-Graustufen aus → pro Bildzeile
werden Pixel > Helligkeit 110 gezählt → zusammenhängende Bänder ≥ 8 px sind Textzeilen → der
Abstand der Band-Schwerpunkte (gewichtet nach Textpixel-Masse) ist der Zeilenabstand.
Skript: `measure_pitch.py` (Session-Scratchpad).

Der Test ist **falsifizierbar** angelegt: Aus dem bei „Normal" gemessenen Basiswert wurden die
beiden anderen Stufen VOR der Messung vorhergesagt (`setLineSpacing(0f, mult)` skaliert linear),
und erst danach gemessen.

## Ergebnis — AC-4

| Stufe | Multiplikator | Vorhergesagt | Gemessen | Einzelabstände | Abweichung |
|---|---|---|---|---|---|
| Kompakt | 1,45 | 68,86 px | **68,75 px** | 68,9 / 68,6 | 0,16 % |
| Normal | 1,70 | (Basis) | **80,74 px** | 80,9 / 80,6 | — |
| Locker | 1,95 | 92,61 px | **92,72 px** | 92,8 / 92,6 | 0,12 % |

Gemessene Verhältnisse 0,8515 und 1,1484 gegen die Soll-Verhältnisse 0,8529 und 1,1471.
**Der konfigurierte Multiplikator erreicht den Android-Renderer mit unter 0,2 % Fehler.**

Schrittweite bei der ausgelieferten Voreinstellung (`previewFontSize` = „Klein"): **12 px pro
Stufe** — klar wahrnehmbar. Das war der Zweck der Verbreiterung auf ±0,30 em.

## Ergebnis — AC-2 (Android-Anteil)

- Die Kontrolle **„Zeilenabstand"** erscheint in Appearance direkt unter „Schriftgröße"
  (DESIGN DECISION 1) mit den Labels **Kompakt / Normal / Locker** (DESIGN DECISION 3).
  Beleg: `ist-settings-appearance-control.png`.
- **„Normal" ist vorausgewählt** — der No-op-Wert aus DESIGN DECISION 2. AC-1s
  Rückwärtskompatibilität am Gerät gesehen.
- Die Kontrolle ist bedienbar; der Wert überlebt „Save Settings" und wird vom Panel beim
  nächsten Anzeigen übernommen (jede Messung oben lief über einen echten Save).
- Nach dem Test auf „Normal" zurückgestellt, per Messung bestätigt (80,75 px).

## Was NICHT geprüft wurde — Rest für den Menschen

1. ~~**AC-3, Windows-GDI-Render.**~~ **Erledigt 2026-08-11:** Andi hat AC-2 + AC-3 am echten
   Release-Build abgenommen (`klarvo.exe`, gebaut 14:27, 44,1 MB).
2. ~~**R-D1-Restbeobachtung.**~~ **Erledigt:** kein Anschneiden von Umlaut-Punkten oder
   Unterlängen auf „Kompakt" — die Headroom-Anhebung 1,325 → 1,35 über Segoe UIs ≈1,330-em-Zelle
   hat gehalten.
3. **Ästhetisches Urteil.** → hat das Gate gekippt, siehe Revision.

## Revision 2026-08-11 — Android auf Desktop-Parität (Commit `117e244`)

Andis Augenschein widersprach dem grünen Messbefund — zu Recht. Die Messung belegte, dass der
Multiplikator den Renderer erreicht; sie sagte **nichts** darüber, ob die Skala richtig *liegt*.
Am Gerät saß jede Android-Stufe rund zwei Rasten lockerer als ihre gleichnamige Desktop-Stufe,
und Andis Konfiguration stand bereits auf `small`/`small` — es gab nichts Kleineres mehr.

**Ursache.** Genau die Annahme, die diese Story selbst als offen markiert hatte
(„to be confirmed at GATE-4"): `setLineSpacing(0f, mult)` multipliziert den *natürlichen
Zeilenkasten*, und der Kommentar schätzte ihn auf ~1,2×. Aus der Messung oben lässt er sich
exakt herausrechnen: 80,74 px ÷ 35,75 px (13 sp @ Dichte 440, font_scale 1,0) ÷ 1,70 = **1,3285**.
Die „per-Plattform-Normalisierung" (±0,25 Android gegen ±0,30 Desktop) normalisierte deshalb nicht.
Androids engste Stufe (effektiv 1,93 × Schriftgröße) entsprach fast exakt Desktops weitester (1,925).

**Änderung** (Android-only; Desktop war zu diesem Zeitpunkt bereits abgenommen):
Multiplikatoren werden als `desktop_wert / NATURAL_LINE_BOX` **abgeleitet** statt handgewählt, und
`FONT_PX_SP` geht auf Desktops 11/13/15 zurück (Story 11-3 hatte auf 13/15/18 skaliert).

**Nachmessung**, gleiches Verfahren, Vorhersage vorab im Skriptkopf notiert (`measure-revision.sh`):

| Stufe | Vorhergesagt | Gemessen | Abweichung |
|---|---|---|---|
| Kompakt | 40,84 px | **41,4 px** | +1,4 % |
| Normal | 49,16 px | **49,4 px** | +0,5 % |
| Locker | 58,23 px | **58,35 px** | +0,2 % |

Die Vorhersage unterstellte 11 sp und belegt damit die Schriftänderung mit: bei den alten 13 sp
hätte „Kompakt" bei 48,3 px landen müssen, nicht bei 41,4. Andi hat die Stufen am Gerät abgenommen.

**Lehre für den nächsten Gate.** Ein Messbefund kann vollständig korrekt und trotzdem am Ziel
vorbei sein. „Der konfigurierte Wert erreicht den Renderer" ist eine *Verdrahtungs*-Aussage;
„die Skala liegt richtig" ist eine *Design*-Aussage und braucht das menschliche Auge oder einen
plattformübergreifenden Soll-Vergleich. Hier war ein solcher Vergleich möglich und wurde nicht
gezogen — die Zahlen für beide Plattformen standen die ganze Zeit nebeneinander im Code.

## Messfallen, die diesen Lauf gekostet haben (für die nächste Sitzung)

1. **`monkey -p <pkg> -c LAUNCHER 1` startet nicht nur** — es injiziert danach einen *zufälligen*
   Tap und verlässt damit den Bildschirm, den man messen wollte. `am start -n <pkg>/.MainActivity`.
2. **`adb shell am broadcast --es transcript "text mit leerzeichen"`** — die Geräte-Shell zerlegt
   den Text in Argumente, `-p` erwischt dann ein Wort daraus als Paketnamen (`pkg=groessere`).
   Das gesamte Remote-Kommando quoten, Text in einfachen Anführungszeichen.
3. **Der Harness-State ist kleingeschrieben** (`recording`), nicht `RECORDING` — sonst
   `result=0` ohne jede Wirkung.
4. **`am force-stop` bringt das Onboarding zurück** („Enable Accessibility Access"), das das
   Panel verdeckt. Knopf per `uiautomator dump` finden und tippen, Koordinaten nie raten.
5. **Ein 4-Sekunden-Build ist von einem No-op nicht zu unterscheiden.** Nach jedem Kotlin-Build
   prüfen, ob die Änderung im *generierten* Baum steht (`grep` in
   `gen/android/app/src/main/java/...`), nicht nur im getrackten.

## Nebenbefunde (NICHT Story 11-6 — Backlog-Kandidaten)

1. **Der Debug-Harness war strukturell tot.** `DebugHarnessReceiver` wird einkompiliert, war
   aber in KEINEM Manifest deklariert → `am broadcast` lief auf `Enqueued: 0`. Ursache: das
   deklarierende `app/src/debug/AndroidManifest.xml` lag nur im gitignorierten, von
   `tauri android init` regenerierten `gen/android`-Baum und wurde von einer Regenerierung
   gelöscht. Für diesen Gate wiederhergestellt — **die Wiederherstellung ist aus demselben Grund
   wieder flüchtig**. Dauerhaft wird sie erst, wenn die Datei im versionierten `android/`-Baum
   wohnt und mitsynchronisiert wird.
2. **Der Contract ist an dieser Stelle falsch begründet.** `_bmad/custom/bmad-epic-conductor.toml`
   sagt, der Harness sei „dead on HyperOS (background restrictions)". Er war nie HyperOS-bedingt
   tot, sondern manifest-los. Der Satz führt die nächste Sitzung in die falsche Richtung.
3. **`scripts/android-build.sh` läuft auf powerhouse nicht durch.** Der Build gelingt, danach
   bricht das Skript beim Kopieren nach `/mnt/d/Dropbox/App Development/klarvo/releases/` ab —
   ein WSL-/Laptop-Pfad. Ebenso `scripts/android-emulator.sh`: die `klarvo-emu`-AVD existiert
   hier nicht (nur auf dem Laptop).
