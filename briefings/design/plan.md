# UI Redesign Plan -- Voxlit Desktop + Android

Erstellt: 2026-03-21
Status: Research + Plan (kein Code)

---

## 1. Research-Zusammenfassung

### Was haben hochwertige Desktop-Tools gemeinsam?

**Wispr Flow (direkter Konkurrent)**
- Links-Sidebar-Navigation auf Desktop: "Styles" als eigener Sidebar-Punkt, nicht ein Tab unter vielen
- Dark-first: #1A1A1A Hintergrund, Blau (#4d65ff) als primärer Akzent, Cream/Off-White für Text
- Floating Bubble auf Mobile/Android: kleines schwebendes Element, verschwindet wenn kein Textfeld aktiv
- Klare Trennung: Recording-Interface (minimalistisch, immer sichtbar) vs. Settings (separates Panel/Fenster)
- Key Principle: Das Diktat-Interface darf niemals mit dem Settings-Interface konkurrieren

**Raycast (Settings-Organisation)**
- Klare Hierarchie in Settings: General / Appearance / Extensions / Advanced / About
- Sidebar-Navigation mit rechtem Detail-Panel -- nicht Tabs in einer Zeile
- Settings sind klar kategorisiert: häufig genutzte Optionen oben, Experten-Optionen hinten
- Principle: Settings sind einmal eingerichtet und dann vergessen -- Discoverability durch Kategorien, nicht durch Menge

**Linear (visuelle Hierarchie + Dark Mode)**
- Sidebar ist bewusst gedimmt (niedrigere Luminanz als Content-Bereich), damit der Arbeitsbereich dominiert
- LCH-Farbsystem statt HSL: 3 Kern-Variablen (base, accent, contrast) generieren das komplette Scheme
- Sidebar: 224px Breite, klare vertikale Rhythmus, Icons und Labels horizontal ausgerichtet
- "Warmes Grau" statt "kühles Grau" für weniger Sättigung, nicht muddy -- wichtiger Hinweis für unser Navy-Schema

**Notion (Settings-Modal-Pattern)**
- Settings als Modal (nicht als Panel im Hauptfenster): deutlicher Fokus-Wechsel signalisiert "du bist jetzt in Settings"
- Sidebar innerhalb des Modals: ~220px breit, Kategorien mit Icons + Labels
- Dark Mode nutzt #2F3438 (Content), #373C3F (Sidebar), #3F4448 (Hover) -- subtile Differenz, nicht schwarz
- Kategorien gruppiert nach Funktion, nicht nach technischer Zugehörigkeit

**Arc Browser (Sidebar-Tiefe)**
- Die gesamte Navigation lebt in der Sidebar -- keine Top-Nav-Leiste
- Sidebar ist kontextual: zeigt was relevant ist, versteckt den Rest
- Theming ist space-spezifisch (jeder Bereich hat eigene Farbe) -- zeigt wie Farbe Navigation verstärkt

**1Password (Settings-Hierarchie)**
- Account-Auswahl oben in Sidebar, dann Settings darunter -- User hat klaren Kontext
- Appearance/Theme als dedizierte Settings-Kategorie
- Keine Icon-Zeile ohne Labels -- jeder Sidebar-Punkt hat Icon + Label

### Kernlektionen für Voxlit

1. **Icon-only-Navigation (aktueller Stand) ist ein Anti-Pattern.** Alle untersuchten Apps nutzen Icons + Labels in der Navigation. 6-7 nackte Icons in einer Zeile ist nicht intuitiv -- auch erfahrene User müssen raten.

2. **Settings gehören in ein Modal, nicht in ein Accordion im Hauptfenster.** Das heutige Pattern (alle Panels klappen sich im selben 480px-Fenster auf) verschwendet Platz und macht das Fenster zu einer Endlos-Scrollseite.

3. **Das Fenster ist für ein Tool zu schmal.** 480px Portrait ist ein Handy-Formfaktor. Raycast, Notion, 1Password -- alle nutzen Landschaft-orientierte Fenster für Settings (mindestens 640px breit). Das gibt Platz für Sidebar + Content nebeneinander.

4. **Farbe als Navigationshilfe.** Linear und Arc nutzen Farbe nicht dekorativ, sondern funktional: aktiver Zustand, Hierarchie, Hover-States. Unser aktuelles Emerald ist funktional, aber passt nicht zur Brand.

5. **Recording-State muss immer sichtbar bleiben.** Wispr Flow zeigt den Recording-Status getrennt vom Settings-Interface. Bei uns ist er im Header -- wenn Settings offen ist, verschwindet er.

---

## 2. Neue Farbpalette

### Prinzip

Die Social Preview definiert das Brand-Schema:
- Deep Navy als Hintergrund-Gradient (nicht Zinc/Gray)
- Orange/Amber + Teal/Cyan als Logo-Akzente
- Weiße Schrift mit guter Lesbarkeit

Das Logo hat zwei Kreise: **Amber** (#FBBF24) und **Teal/Cyan** (#38BDF8). Diese werden die primären funktionalen Akzentfarben -- Emerald verschwindet.

### Semantische Farbrollen

| Rolle | Aktuell (Zinc/Emerald) | Neu (Navy/Brand) | Hex |
|-------|------------------------|------------------|-----|
| App-Hintergrund | #0a0a0c | Deep Navy | `#080B14` |
| Surface (Karten, Panels) | #18181b | Navy Surface | `#0D1220` |
| Elevated Surface (Modals) | -- | Navy Elevated | `#111827` |
| Border subtil | #27272a | Navy Border | `#1E2A3D` |
| Border aktiv | -- | Navy Border Active | `#2D3F5C` |
| Text primär | #fafafa | White | `#F1F5F9` |
| Text sekundär | #71717a | Muted | `#94A3B8` |
| Text gedimmt | -- | Dim | `#64748B` |
| Primary Accent (Recording, Active) | #10b981 (emerald) | Teal/Cyan | `#22D3EE` |
| Primary Accent Background | emerald-500/15 | Teal BG | `rgba(34,211,238,0.12)` |
| Secondary Accent (Processing, Warning) | #f59e0b (amber) | Amber | `#FBBF24` |
| Secondary Accent Background | amber-500/15 | Amber BG | `rgba(251,191,36,0.12)` |
| Danger | #ef4444 | Red (unverandert) | `#EF4444` |
| Success/Done | #34d399 | Teal heller | `#2DD4BF` |

### Tailwind CSS Theme Override (Tailwind v4 @theme Syntax)

```css
@theme {
  --color-vx-bg:         #080B14;
  --color-vx-surface:    #0D1220;
  --color-vx-elevated:   #111827;
  --color-vx-border:     #1E2A3D;
  --color-vx-border-active: #2D3F5C;
  --color-vx-text:       #F1F5F9;
  --color-vx-muted:      #94A3B8;
  --color-vx-dim:        #64748B;
  --color-vx-teal:       #22D3EE;
  --color-vx-teal-bg:    rgba(34,211,238,0.12);
  --color-vx-amber:      #FBBF24;
  --color-vx-amber-bg:   rgba(251,191,36,0.12);
  --color-vx-danger:     #EF4444;
  --color-vx-success:    #2DD4BF;
}
```

### Anwendungsbeispiele

- **Recording aktiv:** Teal-Ring, teal waveform, `rgba(34,211,238,0.12)` Background
- **Processing:** Amber-Spinner, amber text
- **Done:** Teal-Check (#2DD4BF)
- **Error:** Red (#EF4444) unverandert
- **Aktiver Nav-Item:** Teal Text + `rgba(34,211,238,0.08)` Background
- **Hover:** `rgba(255,255,255,0.04)` Background, Text zu `#F1F5F9`
- **FloatingBar:** `rgba(8,11,20,0.96)` Hintergrund, Teal/Amber Akzente je nach State

### Kontrastcheck (WCAG AA: mindestens 4.5:1 auf Text)

- #F1F5F9 auf #080B14: ~18:1 (WCAG AAA)
- #22D3EE auf #080B14: ~9.4:1 (WCAG AAA)
- #FBBF24 auf #080B14: ~8.1:1 (WCAG AAA)
- #94A3B8 auf #080B14: ~5.6:1 (WCAG AA)
- #64748B auf #080B14: ~3.8:1 (nur fuer dekorative/Hint-Texte, keine kritische Info)

---

## 3. Layout-Konzept

### 3.1 Desktop -- Hauptfenster

**Aktuelles Problem:** 480x720px Portrait. Settings, Advanced, History, Stats, Notes, Integrations klappen alle als Accordions im selben Fenster auf. Das ergibt ein Infinite-Scroll-Chaos.

**Neuer Ansatz: Zwei-Fenster-Modell**

**Fenster 1: Recording View (klein, immer präsent)**
- Dimensionen: `480x220px` (deutlich kleiner als heute)
- Position: Frei positionierbar (wie heute), Default: rechte Seite, vertikal zentriert
- Inhalt: Logo, Recording-Button, Style-Picker, letztes Transkriptionsergebnis, Status-Zeile
- Navigation: Nur EIN Settings-Button (Zahnrad) -- öffnet Settings-Modal
- Sichtbar: Immer, wenn App läuft (kein Verstecken ausser beim Tray-Minimize)

**Fenster 2: Settings/Dashboard Modal (mittelgroß, on-demand)**
- Dimensionen: `780x560px` (Landscape-Orientierung)
- Öffnet: Zahnrad-Button im Recording View oder System-Tray
- Schließt: Escape, X-Button, Klick außerhalb
- Layout: Sidebar (linke 200px) + Content (rechte 580px)
- Schließt sich nach Save automatisch (optional: konfigurierbar)

**Recording View Layout (480x220px):**
```
┌─────────────────────────────────────────┐
│  [Logo]  Voxlit            [⚙]          │  <- 40px Header
├─────────────────────────────────────────┤
│                                         │
│      [ Polished | Verbatim | Chat ]     │  <- Style-Picker 36px
│                                         │
│  ┌─────────────────────────────────┐    │
│  │  Letztes Ergebnis (truncated)   │    │  <- Transkript-Preview 60px
│  └─────────────────────────────────┘    │
│                                         │
│  [●] Idle  |  Alt+Shift+D              │  <- Status-Zeile 28px
└─────────────────────────────────────────┘
```

**Settings-Modal Layout (780x560px):**
```
┌────────────────────────────────────────────────────────────────┐
│  Voxlit Settings                                          [X]  │  <- 44px Header
├──────────────┬─────────────────────────────────────────────────┤
│              │                                                 │
│  Recording   │                                                 │
│  ----------  │   [Content-Bereich fuer aktive Kategorie]      │
│  Dictation   │                                                 │
│  ----------  │   Wechselt vollständig je nach Sidebar-Auswahl  │
│  History     │                                                 │
│  Stats       │                                                 │
│  ----------  │                                                 │
│  Dictionary  │                                                 │
│  Snippets    │                                                 │
│  ----------  │                                                 │
│  Advanced    │                                                 │
│  Updates     │                                                 │
│              │                                                 │
└──────────────┴─────────────────────────────────────────────────┘
  200px sidebar        580px content
```

**Alternativer Ansatz (falls Zwei-Fenster zu komplex): Breiteres Einzelfenster**
- Wenn das Zwei-Fenster-Modell zu viel Umbau bedeutet (neue Tauri-Window + IPC):
- Hauptfenster auf `720x560px` Landscape erweitern
- Linke Sidebar (200px) immer sichtbar, Content rechts (520px)
- Recording-View als "Home"-Eintrag in der Sidebar
- Dieser Ansatz erfordert weniger Backend-Änderungen

**Empfehlung:** Erst Einzelfenster mit Sidebar (weniger Risiko), Zwei-Fenster als Phase 2.

### 3.2 Android -- Hauptfenster

Android-Layout bleibt konzeptuell gleich (Floating Bubble als primäres Interface, Tauri-App als Settings-Zugang), aber profitiert von der neuen Farbpalette und Navigation.

**Was sich ändert:**
- Farbpalette: Navy statt Zinc/Gray
- Touch-Targets: Bleiben identisch (aktuell korrekt)
- Navigation: Bottom-Tab-Bar statt Header-Icons (besser für einhandige Bedienung)

**Bottom-Tab-Bar (Android, ersetze Header-Icons):**
```
┌─────────────────────────────────────────┐
│                                         │
│   [Recording View -- Hauptinhalt]       │
│                                         │
├─────────────────────────────────────────┤
│  [🎙 Record] [📖 History] [⚙ Settings] │  <- 56px Bottom Tab Bar
└─────────────────────────────────────────┘
```

Nur 3 Bottom-Tabs auf Mobile statt 6-7 Icons: **Record** (Home), **History**, **Settings**. Alles andere (Stats, Notes, Advanced, Dictionary) lebt verschachtelt unter Settings.

### 3.3 FloatingBar (Desktop, bleibt)

Der FloatingBar ist bereits gut -- er ist minimal, schnell und unsichtbar im Idle-State. Kleine Verbesserungen:

- Hintergrund: `rgba(8,11,20,0.96)` statt `rgba(15,15,18,0.95)` (passender Navy-Ton)
- Waveform-Farbe: `#22D3EE` (Teal statt Blau `#93c5fd`)
- Keine strukturellen Änderungen nötig -- der FloatingBar ist richtig designed

---

## 4. Menü-Umstrukturierung

### 4.1 Aktuelle Struktur (problematisch)

**In einem Header: 6-7 Buttons ohne Labels**
- Zahnrad (Settings)
- Uhr/History
- Balken/Stats
- Noten-Icon (Voice Notes)
- Stern/Integrations (Desktop only)
- Sliders/Advanced Settings

**In Settings-Panel:** API Keys, Language, Hotkey, Audio Device, Cleanup Style, Cleanup Instructions, Output Language, STT Prompts, Profiles

**In Advanced Settings:** STT Provider, LLM Provider, Model Selection, Auto-Paste, Auto-Stop, Dictionary, Whisper Mode, Command Mode, Snippets, Webhooks, Updates

**Probleme:**
- "Settings" und "Advanced Settings" ist eine künstliche Trennung -- der Nutzer weiß nicht was wo ist
- Stats und Voice Notes haben denselben visuellen Rang wie API-Key-Settings
- Integrations ist ein Placeholder ohne Inhalt aber mit eigenem Icon-Button
- Dictionary ist in Advanced Settings, obwohl es eine häufig genutzte Funktion ist

### 4.2 Neue Struktur -- Sidebar-Kategorien

**Sidebar-Gruppen (mit Icons + Labels):**

```
──── DIKTAT ──────────────────
  Aufnehmen          [home view]
  Geschichte         [history]
  Statistiken        [stats]

──── EINSTELLUNGEN ───────────
  API-Keys           [keys]
  Sprache & Stil     [language]
  Tastenkürzel       [hotkey]
  Schreibstil        [style, cleanup instructions]

──── WÖRTERBUCH & SNIPPETS ──
  Wörterbuch         [dictionary]
  Snippets           [snippets -- paid]

──── PROVIDER ────────────────
  STT-Provider       [stt: groq/openai/whisper]
  KI-Provider        [llm: deepseek/groq/openai/openrouter]
  Offline-Modell     [whisper.cpp -- desktop]

──── ERWEITERT ───────────────
  Aufnahme           [audio device, auto-stop, silence threshold]
  STT-Feinabstimmung [stt prompts, temperature]
  KI-Feinabstimmung  [llm prompts, model params]
  Automatisierung    [webhooks, command mode, snippets triggers]
  Synchronisation    [turso sync]

──── APP ─────────────────────
  Updates            [desktop only]
  Lizenz             [license status]
  Info               [version, links]
```

### 4.3 Vor/Nach Vergleich

| Alt | Neu | Warum |
|-----|-----|-------|
| "Settings" Panel | "API-Keys", "Sprache & Stil", "Tastenkürzel" | Konkret, nicht abstrakt |
| "Advanced Settings" Panel | Aufgeteilt auf "Provider", "Erweitert", "App" | Klare Verantwortlichkeit |
| Icon-only Header-Navigation | Sidebar mit Icon + Label | Discoverability |
| API Keys + Hotkey + Audio in einem Panel | Drei separate Sidebar-Punkte | Einzelverantwortung |
| Dictionary in Advanced | Eigene Gruppe "Wörterbuch & Snippets" | Dictionary ist kein Advanced-Feature |
| Integrations (Placeholder) | Verschwindet bis es Inhalt hat | Kein leerer Platz in der Nav |
| STT Prompts in Advanced | Unter "STT-Feinabstimmung" | Klarer Kontext |
| Stats als Header-Icon | Unter "Diktat" Gruppe in Sidebar | Gehört zum Diktat-Workflow |
| Voice Notes als Header-Icon | Unter "Diktat" Gruppe | Gehört zum Diktat-Workflow |

### 4.4 Mobile-spezifisch (Vereinfachung)

Android nutzt eine 3-Tab Bottom-Bar. Die Settings-Hierarchie ist tiefer verschachtelt:

```
Bottom Tab: Settings
  ├── API-Keys
  ├── Sprache & Stil
  ├── Tastenkürzel (nicht auf Android)
  ├── Provider
  │   ├── STT-Provider
  │   └── KI-Provider
  ├── Wörterbuch
  ├── Snippets (paid)
  └── Erweitert
      ├── Aufnahme
      ├── STT-Feinabstimmung
      └── KI-Feinabstimmung
```

---

## 5. Priorisierte Umsetzungsschritte

### Priorisierung nach Aufwand/Impact

**Phase 1: Farbe (geringer Aufwand, hoher Impact)**
Kein struktureller Umbau. Nur `styles.css` @theme Block ändern und alle `emerald-`/`zinc-` Klassen durch die neuen `vx-teal-`/`vx-amber-`/`vx-navy-` Tokens ersetzen.

Umfang: ~2-3 Stunden reine Textersetzung. Hauptrisiko: übersehene Farbwerte in Inline-Styles (FloatingBar nutzt Inline-Styles, kein Tailwind).

Dateien:
- `/home/andyon2/claude-projects/voxlit/src/styles.css` -- @theme Block
- `/home/andyon2/claude-projects/voxlit/src/App.tsx` -- alle `emerald-`/`zinc-` Klassen
- `/home/andyon2/claude-projects/voxlit/src/components/SettingsPanel.tsx` -- alle emerald Klassen
- `/home/andyon2/claude-projects/voxlit/src/components/AdvancedSettingsPanel.tsx`
- `/home/andyon2/claude-projects/voxlit/src/components/ui.tsx` -- INPUT_CLS, LABEL_CLS, SECTION_TITLE_CLS
- `/home/andyon2/claude-projects/voxlit/src/FloatingBar.tsx` -- Inline-Style Hex-Werte

**Phase 2: Fenster-Dimensionen anpassen (mittlerer Aufwand)**
Hauptfenster von 480x720 Portrait auf 720x540 Landscape umstellen. Kein Sidebar-Umbau nötig -- das Layout passt sich durch Tailwind-Responsivität an. Wichtig: Android-Layout darf nicht brechen (isMobile Guard prüfen).

Umfang: ~1-2 Stunden. Risiko: Android WebView bricht wenn Landscape-Layout angenommen wird.

Dateien:
- `/home/andyon2/claude-projects/voxlit/src-tauri/tauri.conf.json` -- width/height
- `/home/andyon2/claude-projects/voxlit/src/App.tsx` -- Layout-Klassen prüfen

**Phase 3: Android Bottom-Tab-Navigation (mittlerer Aufwand)**
Die aktuelle Header-Icon-Zeile durch eine Bottom-Tab-Bar ersetzen. Nur auf Mobile (isMobile Guard). 3 Tabs: Record / History / Settings.

Umfang: ~3-4 Stunden. Risiko: Bestehendes Panel-State-Management in usePanels.ts muss angepasst werden.

Dateien:
- `/home/andyon2/claude-projects/voxlit/src/App.tsx` -- Rendering-Logik
- `/home/andyon2/claude-projects/voxlit/src/hooks/usePanels.ts` -- Panel-State

**Phase 4: Desktop Sidebar-Navigation (höherer Aufwand)**
Das Herzstück des Redesigns. Die horizontale Icon-Zeile im Header durch eine vertikale Sidebar ersetzen. Nur auf Desktop. Setzt Phase 2 (breiteres Fenster) voraus.

Umfang: ~6-8 Stunden. Risiko: Viel interaktiver State, Panel-Transitions, möglicherweise Tauri-Window-Resize-Logic.

Neue Datei: `/home/andyon2/claude-projects/voxlit/src/components/Sidebar.tsx`
Geänderte Dateien:
- `/home/andyon2/claude-projects/voxlit/src/App.tsx` -- Haupt-Layout
- `/home/andyon2/claude-projects/voxlit/src/hooks/usePanels.ts` -- Navigation-State

**Phase 5: Menü-Gruppen umstrukturieren (höherer Aufwand)**
Setzt Phase 4 voraus. Die aktuellen SettingsPanel + AdvancedSettingsPanel in die neue Sidebar-Struktur überführen. Inhalte werden auf neue Kategorien verteilt.

Umfang: ~8-12 Stunden. Risiko: Viel Props-Drilling muss aufgelöst werden. Möglicherweise ist ein Settings-Context sinnvoll.

### Empfohlene Reihenfolge

1. **Phase 1 zuerst** -- visuell sofortiger Impact, minimales Risiko. Farbe ist unabhängig von Struktur.
2. **Phase 3 danach** -- Android-Navigation ist heute ein echtes UX-Problem. Kein Fenster-Resize nötig.
3. **Phase 2 + 4 zusammen** -- Desktop-Fenster und Sidebar hängen zusammen, in einem Zug umsetzen.
4. **Phase 5 zuletzt** -- struktureller Umbau, braucht stabiles Fundament.

---

## 6. Offene Fragen und Risiken

**Frage 1: Zwei-Fenster vs. Ein-Fenster auf Desktop?**
Wispr Flow nutzt ein separates Settings-Fenster. Das ist sauberer (Recording-View immer sichtbar), aber erfordert ein zweites Tauri-Window mit eigenem IPC-Kanal. Empfehlung: Erst mit Einzelfenster (Sidebar) starten, Zwei-Fenster als optionales Upgrade wenn Nutzer danach fragen.

**Frage 2: Fenster-Größe auf kleinen Displays?**
720x540 setzt voraus dass das Display mindestens 720px breit ist. Auf 1280x720 (minimale Desktop-Auflösung) nimmt das Fenster 56% der Breite ein -- akzeptabel. Auf 1920x1080 sind es 38% -- gut. MinWidth sollte bei 640px bleiben.

**Frage 3: Sidebar auf Android?**
Nein. Android bekommt Bottom-Tabs, keine Sidebar. Sidebar ist ein Desktop-Pattern. Auf schmalen Screens (360px) ist eine 200px-Sidebar nicht nutzbar.

**Frage 4: Settings-Context nötig?**
Aktuell fließen Settings als Props durch App.tsx → SettingsPanel. Bei Sidebar-Navigation müssen Settings in jedem Sub-Panel verfügbar sein. Ein Settings-Context wäre sauberer -- aber das ist Phase 5 Problem, nicht Phase 1-3.

---

## Quellen

Research-Grundlage für dieses Briefing:
- [Raycast Settings Documentation](https://manual.raycast.com/preferences)
- [Linear UI Redesign](https://linear.app/now/how-we-redesigned-the-linear-ui)
- [Wispr Flow Feature Overview](https://wisprflow.ai/)
- [Wispr Flow Style Setup](https://docs.wisprflow.ai/articles/2368263928-how-to-setup-flow-styles)
- [Notion Sidebar UI Breakdown](https://medium.com/@quickmasum/ui-breakdown-of-notions-sidebar-2121364ec78d)
- [Sidebar Design Best Practices](https://uxplanet.org/best-ux-practices-for-designing-a-sidebar-9174ee0ecaa2)
- [Linear Dark Mode & Design System](https://linear.app/now/behind-the-latest-design-refresh)
- [Dark Mode Design Systems Guide](https://medium.com/design-bootcamp/dark-mode-design-systems-a-practical-guide-13bc67e43774)
- [Navy Blue Color Pairing Guide](https://www.media.io/colors/navy-blue-color.html)
