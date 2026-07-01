# Story 11.1: Android Live-Preview — Machbarkeits-Benchmark (Spike)

Status: ready-for-dev

> **Neues Epic 11 — Cross-Platform Live-Preview.** Die Live-Cleanup-Preview-Box ist auf **Windows
> bereits voll implementiert und settings-abschaltbar** (Epics 5 + 6, beide `done`). Epic 11 bringt
> dasselbe Feature auf **Android**. Diese erste Story baut **nicht** das Feature — sie beantwortet die
> **benchmark-first Blocker-Frage**, die das Feature bisher „keine Story" sein ließ: Ist Androids
> Transkriptions-Pipeline schnell genug, dass roher Text bei einer Sprech-Pause *live* erscheint?
> Erst wenn diese Story **grün** ist, wird 11-2 (der eigentliche Android-Port) geschrieben. Fällt sie
> durch (rot), ist der Port so nicht sinnvoll und wir entscheiden neu (Architektur-Änderung oder Cut).

## Story

As the team deciding whether to port the desktop Live-Preview box to Android,
I want a measured, on-device latency figure for how fast raw transcript text becomes available after a speech pause,
so that we commit to (or reject) the Android port on evidence instead of hope — before building any preview UI.

## Kontext: warum das ein Blocker ist

Auf **Desktop** speist die Live-Preview aus einem **pausen-getriggerten Delta-Flush** (Story 5-1
`backend-pause-triggered-delta-flush-for-toggle-hold`): bei einer Pause wird das bisher Gesprochene
sofort roh angezeigt. Das fühlt sich live an, weil die Desktop-Transkription lokal/schnell ist.

Auf **Android** läuft STT über **Groq-Cloud in Chunks** (JNI → Rust → Groq `large-v3-turbo`, siehe
`reference_android_stt_is_groq_cloud`) — **nicht** lokal. Es gibt bekannte parked-Sorgen zur
Android-Transkriptions-Latenz und zur Waveform-Reaktions-Latenz vs. Desktop. Eine „Live"-Preview,
deren roher Text bei einer Pause erst nach mehreren Sekunden erscheint, ist ein totes Feature, egal wie
sauber der UI-Port ist. Deshalb: **erst messen, dann bauen.**

## Scope (Spike — messen, nicht bauen)

- **Instrumentieren**, nicht produkt-UI bauen: Miss die Latenz vom **Pause-Signal** bis zur
  **Verfügbarkeit des rohen Transkript-Texts** des gerade gesprochenen Segments auf dem echten Gerät.
- **Pause-Signal** = `KlarvoAudioRecorder.onSilenceDetected` (die vorhandene VAD-Stille-Erkennung;
  `silenceSecs`-Fenster). Das ist derselbe „Pause"-Begriff, den Desktop für den Delta-Flush nutzt.
- **Text-verfügbar-Zeitpunkt** = wenn das Transkript-Ergebnis des Segments aus dem Groq-Pfad zurückkommt
  (Rückgabe von `GroqSttBridge.nativeTranscribe`, orchestriert in `KlarvoOverlayService`).
- **Deliverable** = (a) die gemessene Latenz-Verteilung auf Andis echtem Gerät über mehrere realistische
  Aufnahmen (mind. ~5 Sprech-Pause-Zyklen, normale Netzbedingungen), und (b) eine schriftliche
  **Go/No-Go-Entscheidung** gegen die Schwelle, festgehalten in den Completion Notes + Memory/Backlog.
- Messung sichtbar/prüfbar machen: Log-Zeile mit Millisekunden (`KlarvoLogger`) bei jedem Zyklus, damit
  Andi den Wert am Gerät ohne Debugger ablesen kann.

**Hard scope boundaries:**
- **Keine** Preview-Box-UI, **kein** Settings-Toggle, **kein** Overlay-Layout-Change. Das ist 11-2+.
- **Keine** Änderung an der Transkriptions-/Chunking-/VAD-Logik selbst — nur Zeitmessung um sie herum.
  (Falls das Messen einen Produktions-Pfad anfasst, muss es hinter dem Debug-/Log-Pfad bleiben und das
  Laufzeitverhalten byte-gleich lassen.)
- **Keine** Desktop-Änderung. Desktop ist die Referenz, nicht das Ziel.
- `FLAG_NOT_TOUCHABLE` nie (steht hier nicht an, aber gilt projektweit für Overlays).

## Acceptance Criteria

**AC1 — Latenz wird auf echtem Gerät gemessen.** Given eine reale Aufnahme auf Andis echtem Android-Gerät
mit mindestens einer Sprech-Pause, When das Pause-Signal (`onSilenceDetected`) feuert und der rohe
Transkript-Text des Segments verfügbar wird, Then wird die verstrichene Zeit in Millisekunden gemessen und
per `KlarvoLogger` geloggt — für jeden Pause-Zyklus einzeln, am Gerät ablesbar.

**AC2 — Mehrere Zyklen, realistische Bedingungen.** Given mindestens ~5 Sprech-Pause-Zyklen über normale
Nutzung (nicht nur ein Einzelfall), When gemessen wird, Then liegt eine Verteilung vor (min/median/max),
nicht nur ein einzelner Wert — damit Ausreißer vs. typisches Verhalten unterscheidbar sind.

**AC3 — Go/No-Go gegen die Schwelle.** Given die gemessene Verteilung, When gegen die Schwelle bewertet wird
(**Andi-entschieden 2026-07-01: < 1 s ab Pause = „live"/grün; ≥ 1 s = tot/rot**), Then steht eine
schriftliche Entscheidung fest: **grün** → 11-2 (Android-Port) schreiben; **rot** → Port so nicht bauen,
Ursache benennen (Chunk-Größe? Netz-RTT? Groq-Verarbeitung?) und Alternativen (kleinere Chunks / anderer
Trigger / Cut) skizzieren. Entscheidung landet in den Completion Notes **und** im Backlog/Memory.

**AC4 — Kein Produktverhalten geändert.** Given der Build nach dem Spike, When Android normal aufnimmt,
Then verhält es sich exakt wie vorher (nur zusätzliche Log-Zeilen) — keine sichtbare UI-, Timing- oder
Transkriptions-Änderung.

## Tasks / Subtasks

- [ ] Task 1 — Messpunkte setzen (AC1)
  - [ ] Zeitstempel beim Feuern von `onSilenceDetected` in `KlarvoAudioRecorder.kt` erfassen
  - [ ] Zeitstempel bei Rückkehr des Segment-Transkripts im `KlarvoOverlayService`-Transkriptions-Pfad
        (um `GroqSttBridge.nativeTranscribe` / `transcribeWithRetry`) erfassen
  - [ ] Delta in ms pro Zyklus via `KlarvoLogger` mit eindeutigem Tag loggen (am Gerät greppbar)
- [ ] Task 2 — Auf echtem Gerät messen (AC2)
  - [ ] Build via `scripts/android-smoke.sh` (debug-signiert + `adb install -r`)
  - [ ] Andi nimmt ~5+ Sprech-Pause-Zyklen unter normalen Bedingungen auf; Logs via `adb logcat` ziehen
  - [ ] min/median/max aus den Log-Werten bilden
- [ ] Task 3 — Bewerten + entscheiden (AC3)
  - [ ] Verteilung gegen < 1 s bewerten
  - [ ] Go/No-Go schriftlich festhalten (Completion Notes + Backlog/Memory)
  - [ ] Bei rot: dominante Latenz-Quelle benennen + Alternativen skizzieren
- [ ] Task 4 — Regression-frei verifizieren (AC4)
  - [ ] Bestätigen, dass nur Logging hinzukam und der Produktions-Pfad unverändert ist

## Dev Notes

### Zu berührende Dateien (Android — nur Instrumentierung)

- `android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt` — VAD-Stille-Erkennung; `onSilenceDetected`
  ist das **Pause-Signal** (Start der Messung). `silenceSecs`/`requiredSilentFrames` definieren das
  Pause-Fenster. NICHT die VAD-Logik ändern — nur Zeitstempel abgreifen.
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — orchestriert Aufnahme→Transkription
  (`transcribeWithRetry`-Wrapper). Hier wird der rohe Text **verfügbar** (Ende der Messung).
- `android/kotlin-src/com/klarvo/voice/GroqSttBridge.kt` — JNI-Brücke `nativeTranscribe` → Rust → Groq-Cloud.
  Das ist der Latenz-Treiber (Netz-RTT + Groq-Verarbeitung), nicht lokale Rechenzeit.

### Referenz (nicht anfassen — nur Verständnis)

- Desktop Live-Preview-Mechanik = pausen-getriggerter Delta-Flush, Story 5-1
  (`5-1-backend-pause-triggered-delta-flush-for-toggle-hold`, done). Backend: `src-tauri/src/pipeline.rs`,
  `src-tauri/src/commands/recording.rs`. So sieht „roher Text bei Pause" auf der schnellen Plattform aus —
  das ist die Erfahrung, die 11-2 auf Android nachbauen soll, **wenn** dieser Benchmark grün ist.

### Testing / Verifikations-Symmetrie

- **GATE-4 = echtes Gerät, nie Emulator** (Android-Visual/Timing-Regel). Der Emulator misst weder echte
  Netz-RTT noch echte Groq-Latenz realistisch → die Zahl wäre wertlos.
- **Andi kann den Testzustand selbst herstellen**: aufnehmen → sprechen → Pause → Ergebnis. Der Wert ist
  am Gerät via Log ablesbar. Damit ist der Human-Gate sauber (kein „unmöglicher Test").
- Build/Geräte-Runde = `scripts/android-smoke.sh` / Shortcut „Klarvo Android Smoke".

### Dies ist eine Spike/Benchmark-Story

Der Liefergegenstand ist eine **Messung + Entscheidung**, kein Feature. Die DoD ist erfüllt, wenn die
Zahl steht, gegen < 1 s bewertet ist, und die Go/No-Go-Entscheidung schriftlich festgehalten wurde —
nicht wenn eine Preview-Box existiert.

### Project Structure Notes

- Neues Epic 11 ist frisch — kein Vorgänger-Story-Kontext innerhalb des Epics. Der einzige „Vorgänger"
  ist die Desktop-Implementierung in Epics 5/6 (andere Plattform, als Referenz oben verlinkt).
- Epic-11-Zeile + diese Story werden in `sprint-status.yaml` neu angelegt (bisher nicht vorhanden).

### References

- [Source: reference_android_stt_is_groq_cloud] — Android STT = Groq-Cloud `large-v3-turbo`, nicht lokal.
- [Source: _bmad-output/implementation-artifacts/5-1-backend-pause-triggered-delta-flush-for-toggle-hold.md] — Desktop-Pause-Flush-Referenz.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt] — `onSilenceDetected` Pause-Signal.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt] — Transkriptions-Orchestrierung / `transcribeWithRetry`.
- [Source: android/kotlin-src/com/klarvo/voice/GroqSttBridge.kt] — `nativeTranscribe` JNI → Groq.

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List
