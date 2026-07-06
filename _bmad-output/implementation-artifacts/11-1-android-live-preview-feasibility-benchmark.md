# Story 11.1: Android Live-Preview — Machbarkeits-Benchmark (Spike)

Status: done

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

- [x] Task 1 — Messpunkte setzen (AC1)
  - [x] Zeitstempel beim Feuern von `onSilenceDetected` in `KlarvoAudioRecorder.kt` erfassen
  - [x] Zeitstempel bei Rückkehr des Segment-Transkripts im `KlarvoOverlayService`-Transkriptions-Pfad
        (um `GroqSttBridge.nativeTranscribe` / `transcribeWithRetry`) erfassen
  - [x] Delta in ms pro Zyklus via `KlarvoLogger` mit eindeutigem Tag loggen (am Gerät greppbar)
- [x] Task 2 — Auf echtem Gerät messen (AC2)
  - [x] Build via `scripts/android-smoke.sh` (debug-signiert + `adb install -r`)
  - [x] Andi nimmt Sprech-Pause-Zyklen unter normalen Bedingungen auf; Logs via `adb logcat` ziehen (4 Zyklen, 2026-07-01)
  - [x] min/median/max aus den Log-Werten bilden
- [x] Task 3 — Bewerten + entscheiden (AC3)
  - [x] Verteilung gegen < 1 s bewerten
  - [x] Go/No-Go schriftlich festhalten (Completion Notes + Backlog/Memory) — **GO/grün**
  - [x] Bei rot: dominante Latenz-Quelle benennen + Alternativen skizzieren (n/a — grün)
- [x] Task 4 — Regression-frei verifizieren (AC4)
  - [x] Bestätigen, dass nur Logging hinzukam und der Produktions-Pfad unverändert ist

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

Claude Sonnet 5 (bmad-dev-story)

### Debug Log References

- `scripts/android-smoke.sh` run (2026-07-01): 24 JVM unit tests, 0 failures; fresh debug APK
  built and installed on Andi's real device (`100.112.41.70:5555` via Tailscale pin), versionName
  `0.5.0` verified on-device (AI-1 gate passed). Log output not reproduced here (see terminal
  history), just the summary line: `SMOKE BUILD OK v0.5.0`.

### Completion Notes List

- **Instrumentation implemented (Task 1, AC1) — done.** Two timestamp points + one delta log:
  - `KlarvoAudioRecorder.kt` (`processVadFrame`, right before `onSilenceDetected?.invoke()`):
    logs `[benchmark-11-1] onSilenceDetected fired at <epoch-ms>` — the pause-signal instant.
  - `KlarvoOverlayService.kt`: `onSilenceTriggered()` captures `pauseSignalMs =
    System.currentTimeMillis()` right at pause-signal receipt, threads it through
    `stopAndProcessRecording(pauseSignalMs)` → `processAudio(wavBytes, pauseSignalMs)`. At the
    exact point the raw transcript returns (`tStt`, right after the
    `transcribeWithRetry`/`GroqSttBridge.nativeTranscribe` call, before the hallucination filter),
    logs `[benchmark-11-1] pause-to-text=<delta>ms` when `pauseSignalMs != null`.
  - Only fires for **pause-triggered** stops (AUTOSTOP/AUTO modes, i.e. real `onSilenceDetected`
    events) — manual taps/releases (✓ button, push-to-talk release) pass `pauseSignalMs = null`
    and are silently skipped, matching AC1's "Pause-Signal" definition exactly.
  - Both `stopAndProcessRecording` and `processAudio` gained an **optional, default-`null`**
    parameter — every pre-existing call site (manual stop paths, lines ~1292/1379 in
    `KlarvoOverlayService.kt`) is unchanged and behaves byte-identically (AC4).
- **Build + install (Task 2, first subtask) — done.** `scripts/android-smoke.sh` ran clean:
  KlarvoTheme.kt drift-gate ok, 17 Kotlin sources + 6 fonts + 10 test sources synced, 24 JVM unit
  tests green, fresh debug APK built (2s incremental) and installed on the real device via the
  pinned Tailscale adb target; on-device `versionName` matched the build (AI-1 gate).
- **Regression-free (Task 4, AC4) — verified.** `git diff --stat` on the two touched files shows
  only the timestamp/log additions plus the two default-`null` optional parameters described
  above — no VAD/chunking/silence-detection logic, no config keys, no UI, and no timing behavior
  changed. All 24 pre-existing JVM unit tests still pass unchanged.
- **BLOCKED on a human action — Task 2's real-recording subtask, Task 3 (Go/No-Go).** The
  benchmark's whole point is a genuine on-device, real-network measurement of Andi speaking with
  natural pauses (AC1/AC2 require "eine reale Aufnahme ... mit mindestens einer Sprech-Pause" /
  "~5 Sprech-Pause-Zyklen über normale Nutzung"). I have no microphone/voice input into the real
  device — this is explicitly designed in the story as Andi's action ("Andi nimmt ~5+
  Sprech-Pause-Zyklen auf"), matching the project's Verifikations-Symmetrie rule (a test step
  goes to Andi only when *he* can establish the test state himself, which he can here: open the
  app, use an AUTOSTOP/AUTO-mode gesture, speak a few sentences with pauses between them). The
  fresh APK with the instrumentation is already installed and ready on his device — nothing else
  needs to be built first.
  - **What Andi needs to do:** open Klarvo, use an AUTOSTOP or AUTO-mode gesture (the ones that
    auto-stop/auto-loop on silence — check current bubble-gesture config in Settings if unsure
    which gesture maps to which mode), speak ~5+ short sentences with a natural pause after each,
    then pull the log: `adb -s 100.112.41.70:5555 logcat -d | grep "benchmark-11-1"`. Each cycle
    yields one `pause-to-text=<ms>` line.
  - **Task 3 (Go/No-Go) is written but not executable without that data** — min/median/max
    against the Andi-decided <1s / ≥1s threshold, plus (if red) naming the dominant latency
    source (chunk size vs. network RTT vs. Groq processing) is deferred to whoever runs the
    device session next, using the log tag above. Story stays `in-progress` until that happens.

### File List

- `android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt` — pause-signal timestamp log at
  `onSilenceDetected` fire point (instrumentation only).
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — `pauseSignalMs` threaded through
  `onSilenceTriggered` → `stopAndProcessRecording` → `processAudio`; pause-to-text delta logged
  once the raw transcript returns (instrumentation only, default-null param preserves all other
  call sites unchanged).

### Measurement result + Go/No-Go (2026-07-01, Andi real device via Tailscale, fixed build `5443f87`)

4 pause-cycles, all clean (every `stt=` 366–528 ms, none ≥ 2 s → no retry contamination):

| cycle | pause-to-text | stt (Groq) |
|-------|--------------|-----------|
| 1 | 999 ms | 528 ms |
| 2 | 712 ms | 366 ms |
| 3 | 819 ms | 458 ms |
| 4 | 753 ms | 431 ms |

**min 712 · median 786 · max 999 · mean 821 ms.** All under Andi's < 1 s threshold (AC3).

**DECISION = GO (green).** The Groq path is fast enough for a live-feeling preview. Caveats
recorded: max (999 ms) sits at the 1 s edge, ~half the latency is Groq network RTT (366–528 ms),
so a weak signal / a retry can push a single sample > 1 s — the Android preview must tolerate an
occasional slow sample. Measured over Tailscale, n=4.

**Follow-on architecture decision (Andi, 2026-07-01) — feeds 11-2, NOT part of this spike's code:**
the Android preview will mirror the desktop **Groq delta-STT** approach (`delta_snapshot_wav()` →
each pause STTs only new audio; total ≈ 2× Groq audio-seconds per dictation, *not* N×). A
local-on-device model for preview was considered and **deferred** — it would need its own
on-device latency benchmark (this Groq measurement does not apply) plus activating Android's
dormant local-whisper. See backlog / memory.

### Change Log

| Date | Change |
|------|--------|
| 2026-07-01 | Instrumentation built (dev `8d32a28`). Code-review (3 adversarial reviewers) → 3 confirmed findings on benchmark-number trustworthiness fixed (`5443f87`): retry-visibility (`stt=` annotation), blank-transcript guard, monotonic clock (`SystemClock.elapsedRealtime`). AC4 clean (log-only, manual call sites byte-identical). |
| 2026-07-01 | Andi measured on real device (n=4): median 786 ms, all < 1 s → **Go/No-Go = GO**. Architecture for 11-2 chosen: Groq delta-STT (~2×), local-model preview deferred. Story → done. |
