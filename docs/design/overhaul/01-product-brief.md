# 01 — Was Klarvo ist (Produkt-Kontext)

## In einem Satz
Klarvo ist eine Diktier-/Spracheingabe-App: Hotkey drücken → sprechen → der von einem LLM
bereinigte Text landet direkt im Zielfeld (Editor, Browser, Chat, Code).

## Zielgruppe (bewusst NICHT Mass-Market)
- **Power-User** (der Archetyp des Gründers selbst)
- **Entwickler** — modular, scriptbar, BYOK
- **Institute/Organisationen** — Kanzlei, Arztpraxis, Forschungsgruppe, Redaktion;
  einmaliges Setup, dann produktiv
- **Sekundär & architektonisch first-class:** Nutzer mit RSI / motorischen Einschränkungen

Bewusst **keine** Zielgruppe: tech-ferne Normalnutzer. Reibung (z.B. eigenen API-Key
hinterlegen) ist ein **Akzeptanz-Filter**, kein Defekt. Das UI darf Ernsthaftigkeit und
Präzision ausstrahlen — nicht verspielt, nicht "consumer-cute".

## Positionierung
Vertikale Nischen-Plattform, nicht horizontaler Massenmarkt-Konkurrent. Differenzierung ist
architektonisch (BYOK, lokal-first, Cargo-Feature-Varianten für Klarvo Medical/Legal/Science/…),
nicht Feature-Geschwindigkeit. Das visuelle Design soll dieses Ernsthaftigkeits-Signal tragen.

## Werte, die das Design transportieren soll
- **Lokal-first & vertrauenswürdig** — keine Telemetrie, Daten bleiben beim Nutzer (BYOK).
- **Ruhig & präzise** — ein Werkzeug, das man stundenlang neben der echten Arbeit laufen
  lässt. Es darf nie um Aufmerksamkeit konkurrieren.
- **Dicht, aber nicht überladen** — Power-User vertragen (und wollen) Informationsdichte,
  aber sauber hierarchisiert.

## Der Kern-Loop (damit der Designer den Kontext der FloatingBar versteht)
1. Nutzer arbeitet in irgendeiner App. Die **FloatingBar** schwebt unaufdringlich am
   Bildschirmrand.
2. Hotkey (`Shift+Alt+Y`, Toggle) → Bar geht in **Recording**-State, zeigt Live-Waveform.
3. Optional: **Live-Cleanup-Preview** zeigt schon während des Sprechens den bereinigten Text.
4. Hotkey erneut → Transkription + LLM-Cleanup → Text wird ins zuvor fokussierte Feld
   eingefügt. Bar geht zurück in **idle**.
5. **History/Voice-Notes** im Main-Window halten vergangene Diktate durchsuchbar.

> Die FloatingBar ist das, was der Nutzer **die meiste Zeit sieht**. Sie ist die
> wichtigste einzelne Designfläche.

## Markenelemente (Ist-Zustand)
- Name: **Klarvo**. Logo aktuell: teal "K" auf dunklem, abgerundetem Quadrat (`#14B8A6`).
- Theme: Dark, Notion-artig. Teal als Primärfarbe, warmes Orange als Akzent/Aktivität.
- Ton: nüchtern, technisch, vertrauenswürdig.
