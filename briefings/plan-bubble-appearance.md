# Feature-Plan: Bubble Size/Opacity Controls (Android)

## Priorität: 3

## Ziel
Android-Nutzer können Größe und Transparenz der Floating Bubble anpassen. Vorheriger Versuch mit Slidern war buggy (Werte springen zurück, kein Live-Effekt). Neuer Ansatz nötig.

## UX-Entscheidung: Presets statt Slider

Slider waren problematisch weil:
- Werte wurden nicht live an den Service weitergegeben
- State-Sync zwischen React-WebView und Kotlin-Service ist fragil
- Feine Granularität (0.1-Schritte) bringt keinen echten Mehrwert

**Vorschlag: 3-4 Presets**
- **Klein** (0.75x, 80% opacity) — unauffällig
- **Normal** (1.0x, 85% opacity) — Standard
- **Groß** (1.3x, 90% opacity) — leichter zu treffen
- Optional: **Custom** mit numerischen Eingabefeldern statt Slidern

Presets sind ein einzelner Tap, kein Slider-Gefummel. Und der Wert wird atomar gespeichert.

## Betroffene Module
- `src/components/SettingsPanel.tsx` — Preset-Buttons (Mobile-only Sektion)
- `src-tauri/src/config/mod.rs` — bubble_size/bubble_opacity (Infrastruktur steht bereits!)
- `android/kotlin-src/com/dikta/voice/DiktaOverlayService.kt` — reloadBubbleAppearance() existiert bereits
- `android/kotlin-src/com/dikta/voice/DiktaApi.kt` — readConfig() liest bereits bubble_size/opacity

## Tasks

### Task 1: Preset-UI
- **Agent:** ui-dev
- **Dateien:** `src/components/SettingsPanel.tsx`
- **Beschreibung:** Mobile-only Sektion "Bubble-Größe" mit 3 Preset-Buttons (Klein/Normal/Groß). Jeder Button setzt bubble_size + bubble_opacity und speichert sofort. Aktiver Preset visuell hervorgehoben.

### Task 2: Live-Reload testen
- **Agent:** android-platform (direkte Session)
- **Beschreibung:** Sicherstellen dass reloadBubbleAppearance() bei IDLE-Transition zuverlässig die neuen Werte aus config.json liest und anwendet. Falls nicht: Event-basierte Lösung (Tauri-Event an Kotlin weiterleiten).

## Testplan
- [ ] Preset-Buttons erscheinen nur auf Mobile
- [ ] Tap auf Preset speichert Werte
- [ ] Bubble ändert Größe/Opacity nach nächster IDLE-Transition
- [ ] Preset bleibt nach App-Neustart erhalten

## Risiken
- Gering. Backend-Infrastruktur steht komplett, nur UI + Validierung.
