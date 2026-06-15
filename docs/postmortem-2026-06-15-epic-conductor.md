# Postmortem + Redesign-Spec — Epic-Conductor (2026-06-15)

**Status:** Lauf gestoppt durch Andi mitten in Epic 9 (nach 9-4/9-5). Kein Weiterlauf bis die unten
beschriebenen Nähte gefixt sind. Dies ist Analyse + Spec, **kein** Umbau.

## Was passiert ist (Evidenz)

1. **Worker committen trotz Sole-Committer-Regel — wiederholt, beide Stories.**
   - 9-4: `18187ff` (19:22), `56b9cae` (19:22), `ffa3a0b` (19:31), `5752867` (19:46) — Worker.
   - 9-5: `6ce8fe5` (22:06), `5a18465` (22:06), `b34f260` (22:21) — Worker.
   - Alle committet als `Andi <anolte@gmx.com>` (die Git-Identität, unter der bmad-Worker laufen).
   - Inkonsistente Behandlung: bei 9-4 blieben die 4 Worker-Commits in der History (Subagent meldete
     `workerCommits=[]`, weil sie *unter* seinem baseRef lagen); bei 9-5 wurden sie per `reset --soft`
     gefaltet. Zwei Stories, zwei verschiedene History-Formen.

2. **Worker sind auf das ECHTE Handy entkommen — Ursache der kaputten App.**
   - `com.klarvo.voice` auf `100.112.41.70:5555`: `flags=[ DEBUGGABLE ]`, `firstInstallTime=19:37`,
     `lastUpdateTime=22:00`. Beide Zeiten fallen in die Worker-Aktivität (9-4-Ära bzw. 9-5-Lauf).
   - Der Conductor hat ausschließlich auf `emulator-5554` gezielt. Die Installs kamen von bmad-Workern,
     die `scripts/android-smoke.sh` benutzten — das per Default `KLARVO_ADB_TARGET=100.112.41.70:5555`
     (Tailscale-Handy) ansteuert und installiert. Die Emulator-Strategie hat nur *meine* Befehle
     isoliert, nicht die der Worker.

3. **Keine persistente, überprüfbare GATE-4-Evidenz.**
   - Screenshots (`/tmp/ist-9-5-*.png`, `/tmp/soll-9-5-*.png`) liegen nur in `/tmp`, ephemer, nicht
     pro Story indiziert, nie vorgelegt. Für einen Nacht-Lauf existiert kein Review-Trail.

4. **Keine Run-Isolation / kein Lock.**
   - 9-4 wurde 19:22–19:46 committet, obwohl der Conductor zu dieser Zeit nachweislich im
     Emulator-Setup war und keinen 9-4-Lauf gestartet hatte. Erklärung: entweder lief der von Andi
     abgebrochene *erste* Spawn als Subagent weiter, oder ein paralleler/früherer Lauf fasste dasselbe
     Repo an. Der Conductor hat kein Lockfile und prüft nicht, ob HEAD sich nur durch *seine* Commits
     bewegt → solche Fremdmutationen sind von innen unsichtbar.

## Root Cause (gemeinsam)

Der Conductor versucht Eigenschaften **per Prosa-Direktive an Subagenten** durchzusetzen
(„nicht committen", „nur Emulator"), aber die darunterliegenden bmad-Skills (dev-story, code-review,
deren Smoke-Schritt) haben **eingebaute `git commit`- und `android-smoke.sh`-Schritte**. Eine Bitte
gegen den eingebauten Ablauf eines Skills ist nicht bindend. Garantien müssen **mechanisch** sein
(Umgebung, Hooks, Erreichbarkeit), nicht per Anweisung.

---

## Redesign-Spec — die vier Nähte

### Naht 1 — Commit-Ownership: mechanisch statt per Bitte
**Entscheidung:** Nicht länger „Worker dürfen nicht committen" als Prosa. Stattdessen zwei Lagen:
- **(Gürtel) Akzeptieren + normalisieren, konsistent.** Worker *dürfen* committen; die **Story-Conductor-
  Ebene** (näher an den Workern, wie Andi erwartet) faltet nach JEDEM Worker-Schritt sofort
  (`git log -1` Check → wenn Worker committet hat, in den Arbeitsbaum zurückfalten) und liefert dem
  Epic-Conductor *einen sauberen Zustand*. Der Epic-Conductor wird zum **Sole-Finalizer** (ein
  Commit/Story), nicht Sole-Committer-der-gegen-Worker-kämpft. Immer falten — nie wie bei 9-4 stehen
  lassen. (Deckt sich mit Memory `feedback_conductor_dev_story_commit_variance` — wurde nicht befolgt.)
- **(Hosenträger) Optionaler `pre-commit`-Hook** beim Run-Start, der Commits während des Laufs
  blockt (non-zero), wenn man HART verhindern will. Risiko: bmad-Workflows können bei Commit-Fehler
  loopen/abbrechen → erst testen, ob die Skills das vertragen, sonst nur Gürtel.

### Naht 2 — Test-Isolation: das echte Gerät physisch unerreichbar machen
**Entscheidung:** Verlassen sich NICHT darauf, dass Worker den Emulator-Wrapper benutzen.
- **(Gürtel) Echtes Gerät beim Run-Start abkoppeln:** `adb disconnect 100.112.41.70:5555`; am Run-Ende
  wieder verbinden. Während des Laufs ist nur `emulator-5554` erreichbar → ein Worker, der
  `android-smoke.sh` ruft, KANN das Handy nicht treffen (Auto-Connect schlägt fehl, Install bricht ab).
- **(Hosenträger) Env-Guard in `android-smoke.sh`:** bei `KLARVO_CONDUCTOR=1` jedes nicht-Emulator-
  Ziel ablehnen (Abbruch mit klarer Meldung), und Default-Target auf `emulator-5554` ziehen.
- **Folge fürs Handy:** das echte Gerät bleibt bewusst *Andis Morgen-Gate* — aber Deployment dorthin
  passiert nur durch eine **explizite, separate** Handlung (s. „Morgen-Gate-Artefakt"), nie durch einen
  Worker mitten im Lauf.

### Naht 3 — Evidenz-Persistenz: ein überprüfbarer Trail pro Story
**Entscheidung:** GATE-4 schreibt dauerhafte, indizierte Artefakte.
- Pro Story: `_bmad-output/implementation-artifacts/gate4-evidence/<story>/` mit
  `soll-<state>.png`, `ist-<state>.png`, optional `diff-<state>.png` und `verdict.md`
  (State, pass/fail, Pixel-/Sicht-Notiz, welcher Canon-Selector).
- Story-Change-Log verlinkt den Evidenz-Ordner. Der **Final Report** des Conductors listet pro Story
  den Ordner + Decisions-Ledger.
- Run-Ebene: `gate4-evidence/RUN-<datum>.md` mit Links/Thumbnails aller States + Ledger — das öffnet
  Andi morgens und sieht *sofort*, was jeder Emulator-Test ergeben hat.

### Naht 4 — Run-Isolation: Lock + HEAD-Wächter
**Entscheidung:**
- **Lockfile** beim Run-Start (`.conductor-lock` mit PID/Start-SHA/Zeit); Abbruch wenn vorhanden →
  keine überlappenden Läufe.
- **HEAD-Wächter:** Conductor merkt sich nach jedem eigenen Commit den erwarteten HEAD; vor dem
  nächsten Schritt prüfen, dass HEAD == erwartet. Bewegte sich HEAD fremd → **halten** und melden,
  nicht blind weiter.
- **Abgebrochene Spawns:** klären, ob ein per User-Interrupt „rejected" Agent-Spawn Kinder
  hinterlassen kann, die weiterlaufen. Wenn ja: Kill-/Reap-Schritt beim Interrupt.

### Naht 5 — Emulator-False-Green (der schwerste Befund, am echten Gerät bewiesen)
**Beobachtung (2026-06-15, finaler 9-5-Build `50d1d7f` auf echtem Xiaomi/MIUI):**
- idle-Bubble: rendert sauber. ABER:
- recording-State: **kaputt** — eine verirrte rote Recording-Pille schwebt MITTEN im Screen über
  fremdem App-Inhalt, zusätzlich zu einem dünnen Panel-Streifen unten. Wahrscheinliche Ursache: das
  9-5-Listening-Panel (separates Overlay-Fenster) UND die alte RECORDING-Leiste der
  `FloatingBubbleView` sind GLEICHZEITIG offen → zwei Fenster für denselben Zustand.
- **Derselbe Zustand wurde vom Emulator als „GATE-4 GRÜN" gemeldet.** Das ist der kanonische
  False-Green: die unbeaufsichtigte Verifikation lief auf einer Maschine, die sich (a) anders rendert
  als das Zielgerät und (b) MIUI-Overlay-Verhalten (mehrere Fenster, Positionierung) gar nicht zeigt.
- **Zusatz:** der 9-4-Harness (`DEBUG_SET_STATE`-Broadcast) ist auf dem echten MIUI **tot** — der
  Broadcast wird enqueued, erreicht den Manifest-Receiver aber nie (MIUI-Background-Restriktionen).
  Er funktioniert NUR auf dem Emulator. D.h. das Verifikations-Werkzeug selbst läuft nicht dort, wo
  es zählt.

**Konsequenz (Grundsatz-Entscheidung nötig):** Eine **visuelle/Overlay-Epic ist nicht ehrlich
voll-unbeaufsichtigt verifizierbar.** Der Emulator taugt für Compile/Logik/Install-ohne-Prompt —
**nie als visuelles Orakel** für Surface-Arbeit. Optionen:
- **(A) Visuelle Epics laufen ATTENDED:** Conductor macht den mechanischen Teil (create/dev/
  code-review/commit), aber GATE-4-Sichtprüfung = Andis echtes Gerät, kein Emulator-Grün.
- **(B) Conductor nur für nicht-visuelle Epics; Surface-Stories interaktiv/manuell.**
- In beiden Fällen: **Emulator-Grün darf nie als „verifiziert" in einen Report/Commit** für eine
  Surface-Story. Wenn Emulator benutzt wird, muss der Report ihn als „nur Emulator, real-device
  ausstehend" kennzeichnen (ehrliche Herabstufung, E7-Posture).
- **Harness real-device-fähig machen:** Broadcast-Receiver ersetzen durch einen Trigger, den MIUI
  nicht blockt (z.B. eine Debug-Activity, ein In-App-Debug-Menü, oder ein File-/ContentProvider-
  Poll) — sonst ist Andis Morgen-Gate (selbst States erreichen) nicht bedienbar.

### Bonus — Morgen-Gate-Artefakt
Am Run-Ende EINE bewusste, dokumentierte Aktion: den finalen Branch-Stand als Debug-APK bauen und
(nur dann, mit Zustimmung / als expliziter letzter Schritt) auf das echte Gerät installieren — plus
die Evidenz-Sammlung — damit Andi morgens ein konsistentes Telefon UND den Screenshot-Trail hat.
Nie durch einen Worker mitten im Lauf.

## 9-5-Defekt-Spec (real-device, am echten MIUI bewiesen) — für sauberen Fix nächste Session

**Symptom:** Im RECORDING-State erscheinen ZWEI Fenster — das Listening-Panel (unten) UND die
RECORDING-Form der `FloatingBubbleView` (HOLD-tap: rote Leiste; PTT/circular: Kreis) an der
edge-gesnappten Bubble-Position, schwebend über fremdem App-Inhalt.

**Warum kein Einzeiler (die Fallen, alle verifiziert im Code):**
- `showListeningPanel`/`hideListeningPanel` sind der gemeinsame Chokepoint — `hideBubble()` dort wäre
  naheliegend, ABER:
- **PTT-Touch-Kollision:** In Push-to-Talk (`longPressMode=HOLD`, `pushToTalkActive=true`) stoppt das
  Loslassen das Recording über den Touch-Stream der Bubble (`handleTouch` → `ACTION_UP` →
  `stopAndProcessRecording`, KlarvoOverlayService ~Zeile 1029-1032). `hideBubble()` = `removeView` →
  ACTION_UP wird nie zugestellt → **PTT kann nicht mehr stoppen.** Die Bubble-WINDOW muss während
  des Recordings für den Touch leben.
- **`alpha=0`-Falle:** Window behalten + `bubbleView.alpha = 0f` würde Touch erhalten und das Visual
  verstecken — aber `setState()` setzt `bubbleView.alpha = 1.0f` bei JEDEM State-Wechsel (Zeile ~1775)
  → bei RECORDING→TRANSCRIBING käme der Glitch zurück. Bräuchte ein dediziertes
  `bubbleSuppressedForPanel`-Flag, das `setState`/`onDraw` respektieren.
- **Cancel-Affordance:** Die alte HOLD-tap-Leiste hatte Cancel(✗)+Confirm(✓)-Zonen auf der Bubble.
  Das Panel hat nur Stop(=Confirm). Wird die Bubble-Leiste entfernt, fehlt **Cancel** → gehört aufs
  Panel (zweiter Button) als Teil des Fixes.

**Richtiger Fix (Skizze, real-device zu verifizieren — Harness ist auf MIUI TOT, also manuell durch
Andi: PTT halten+loslassen, HOLD-tap, TOGGLE — je recording→transcribing→done):**
Bubble-WINDOW während Panel-Surface erhalten (Touch lebt), Bubble-VISUAL unterdrücken via Flag, das
`setState` respektiert; Cancel-Button aufs Panel; bei DONE Bubble-Visual zurück (Check) → IDLE.
Pro Modus (HOLD-tap / PTT / TOGGLE / AUTOSTOP / AUTO) einzeln auf dem echten Gerät durchspielen.

## Sofort-Reste dieses Laufs (offen)
- **Handy:** trägt einen halbfertigen 9-4/9-5-Debug-Build → Rescue-Entscheidung offen (deinstallieren /
  letzten guten Stand neu bauen / finalen 9-5-Stand bewusst bauen+inspizieren).
- **9-5 Architektur-Notiz:** Listening-Panel als separates `TYPE_APPLICATION_OVERLAY`-Fenster
  (statt in `FloatingBubbleView`, abweichend von ADR-0018-Wortlaut) — relevant für 9-6/9-8;
  MIUI-Overlay-Quirks sind ein wahrscheinlicher Mitverursacher der Handy-Glitches.
- **History:** 9-4 trägt 4 ungefaltete Worker-Commits, 9-5 ist gefaltet — inkonsistent (aber nicht kaputt).
- Sprint-status: 9-4 + 9-5 stehen auf `done`; Rest Epic 9 (9-6…9-9) `backlog`.
