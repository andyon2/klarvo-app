# GATE-4 — Story 11-6 (Zeilenabstand als Appearance-Setting)

**Datum:** 2026-08-11 · **Branch:** `feat/11-6-line-spacing-setting` · **Range:** `86b5dca..HEAD`

## Verdikt

| Teil-Gate | Status | Grundlage |
|---|---|---|
| **GATE-4b — Android (AC-2 UI, AC-4 Render)** | **GRÜN** | Objektiv gemessen am echten Gerät, s. u. |
| **GATE-4a — Windows (AC-3 GDI-Render)** | **OFFEN** | Nicht unattended herstellbar: Live-Vorschau erscheint nur während echter Aufnahme (Mikrofon) |

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

1. **AC-3, Windows-GDI-Render.** Die native Vorschau (`native_preview.rs`) zeichnet nur während
   einer echten Aufnahme; ein Mikrofon ist von hier nicht bespielbar. Braucht einen echten
   Windows-Build (`scripts/sync-and-build.ps1`).
2. **R-D1-Restbeobachtung.** „Kompakt" = 1,35 liegt knapp über Segoe UIs natürlicher Zeilenzelle
   (≈1,330 em). `DrawTextW` zeichnet ohne `DT_NOCLIP` in ein exakt `line_h` hohes Rechteck
   (`native_preview.rs:770`). Beim Windows-Test gezielt auf **Umlaut-Punkte und Unterlängen**
   in der Stufe „Kompakt" achten.
3. **Ästhetisches Urteil.** Ob 12 px pro Stufe sich richtig *anfühlen*, ist Andis Auge — die
   Messung sagt nur, dass die Stufe wahrnehmbar ist.

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
