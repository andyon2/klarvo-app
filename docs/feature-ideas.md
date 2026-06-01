# Klarvo v1 — Feature-Ideen (Capture-Liste)

> Leichtgewichtige Sammelliste für noch nicht spezifizierte Ideen. **Kein Backlog, kein
> Commitment.** Jeder Eintrag ist Roh-Input für eine spätere PRD / einen Product-Brief,
> wenn die Feature-Phase ihn aufgreift (Pivot-Plan: Feature-Work via BMAD-Epic-Route).
> Status-Werte: `idea` (nur festgehalten) → `scoping` → `accepted` → `in PRD`.

---

## Live-Cleanup-Preview ("Wispr-Flow-Style Vorschaufenster")

- **Status:** `idea`
- **Erfasst:** 2026-05-30
- **Inspiration:** Wispr Flow — Overlay zeigt während/nach der Aufnahme das Transkript
  mit sichtbar durchgestrichenen Füllwörtern/Korrekturen ("um", "Friday I mean",
  "8:00 actually make that"), Label "CLEANING IT UP". Screenshot in Session-Kontext.
- **v2-Vorgänger:** war als "Epic 11 LivePreview" geplant, in der v1-Pivot-Memory als
  Feature-Kandidat **"Live-Overlay"** benannt — nie geshippt.

### Was es ist
Statt nur einer Waveform-Pill während der Aufnahme: ein Vorschau-Overlay, das den
**Cleanup visuell macht** — Verbatim-Transkript mit durchgestrichenen Entfernungen und
hervorgehobenen Ersetzungen, bevor der finale Text gepastet wird.

### Schlüssel-Erkenntnis: die Daten existieren bereits
Die Pipeline produziert Verbatim **und** Cleaned am selben Punkt:
`ProcessOutcome::Produced { cleaned_text, raw_text, .. }` (`src-tauri/src/pipeline.rs:943-952`).
Die Shell bekommt beide Strings zusammen. Das Feature ist im Kern: **diffe `raw_text`
gegen `cleaned_text` (Wort-Level) und rendere den Diff im bestehenden Overlay.** Kein
Pipeline-Umbau nötig.

### Zwei Versionen — Kosten unterscheiden sich massiv
- **(A) Post-Cleanup-Diff-Preview — ACHIEVABLE, billig.**
  Nach Stop, im Cleanup→Paste-Fenster: Word-Diff Verbatim↔Cleaned, im Overlay zeigen,
  dann pasten. Reuse vorhandener Daten.
- **(B) Echtes Streaming-Live-Preview während des Sprechens — TEUER, out-of-scope bis bewiesen.**
  Bräuchte Streaming-STT (Partial-Hypothesen) + inkrementelles Cleanup. v1 ist reines
  Batch (Record→Stop→STT→Cleanup→Paste). Das wäre ein Pipeline-Rewrite, kein Feature.

### Deep-Research-Befund (2026-05-30) — Wispr macht (A), und zeigt den Diff gar nicht
Recherche-Harness (5 Angles, 15 Quellen, adversarische Verifikation), hohe Konfidenz:
- **Wispr Flow nutzt (A): Post-Stop-Batch.** Eigener Eng-Blog + Baseten-Case-Study:
  sequenzielle Pipeline ASR<200ms → Llama-Cleanup<200ms → Netz<200ms, Ziel "innerhalb
  700ms **nachdem** der Nutzer aufhört". Help-Artikel wörtlich: *"Flow doesn't offer
  live or real-time transcription… processes your full audio first, then inserts the
  finished text."* → **(B) ist vom Tisch; kein Streaming-Rewrite nötig.**
- **ABER:** das "CLEANING IT UP"-Strikethrough-Overlay aus dem Screenshot ist in KEINER
  Quelle (offiziell o. Dritt) dokumentiert. Das echte Desktop-Produkt zeigt nur einen
  Spinner und pastet das fertige Ergebnis stumm. **Der Screenshot ist mit hoher
  Wahrscheinlichkeit eine Werbe-Animation, kein Alltags-UI.**
- **Implikation:** Wir würden den sichtbaren Diff nicht *kopieren*, sondern *neu bauen* —
  etwas, das der Marktführer nur bewirbt, aber für seinen Haupt-Flow bewusst NICHT zeigt
  (vermutl. weil ein Diff zwischen Stop und Paste die gefühlte Latenz erhöht + ablenkt).
  Offene strategische Frage: ist ein sichtbarer Diff ein Transparenz-Differenzierungs-
  merkmal (BYOK/Power-User-Narrativ) — oder Show-Reibung, die wir uns sparen sollten?
- Nuance: Wispr HAT serverseitig einen Streaming-ASR-Transport (gRPC/WebSocket, Partials)
  und zeigt auf iOS rohe Worte progressiv — aber das ist der Roh-Transkript-Pfad, NICHT
  der Cleanup-Diff. Der Diff wird erst im Post-Utterance-Pass aufgelöst.

### Visuelle Verifikation per Frame-Analyse (2026-05-30) — Diff ist Marketing, nicht Produkt
Nachdem Andy dem Text-only-Research misstraute (zurecht: die Demo-/Video-Angles waren
rausgefiltert), 4 offizielle Wispr-Videos frame-genau gescoutet (yt-scout), inkl. 2 echter
Screen-Recordings + Sub-Sekunden-Frames an den Übergängen:
- **"Watch Flow in Action"** (echtes macOS-Messages-Recording): leeres Feld während Sprache →
  fertiger sauberer Text am Stück. Aufnahme MIT Korrektur ("actually… 9am") → **kein Diff**.
- **"Live Demo feat. Woz"** (echtes Slack/Gmail): leeres Feld + Waveform-Pill → fertiger Text.
- **"Break up with your keyboard"** (stilisierter Ad), Sub-Sekunden-Übergang: leer + Pill im
  Verarbeitungs-Zustand → fertiges "This proposal is amazing." **Keine Durchstreichungen.**
- **iPhone-Ad**: stilisierte Sprechblasen-Overlays (Text erscheint beim Sprechen), aber
  ebenfalls KEIN Strikethrough-Cleanup.
- **Fazit:** Das "CLEANING IT UP"-Strikethrough-Overlay erscheint in KEINEM echten Produkt-UI.
  Signatur überall: Recording-Pill → kurzer Verarbeitungs-Zustand → fertiger sauberer Text.
- **ABER Andys Erinnerung ist nicht falsch:** der Screenshot ist ein echtes Wispr-**Ad-Creative**
  (WhatsApp-"Sydney"-Spot). Das Strikethrough-Device existiert — als **Marketing-Visualisierung**
  des Cleanup-Konzepts, nicht als ausgelieferte UI. Über Wisprs eigene Ads ist es nicht mal
  konsistent (4 von 4 gecheckten nutzen es NICHT).
- **Verschärfte Implikation:** Würden wir den sichtbaren Diff bauen, replizieren wir Wisprs
  *Werbeversprechen*, nicht ihr Produkt. Der Marktführer hat den sichtbaren Diff getestet
  (Ad-Creative) und bewusst NICHT ins Produkt genommen — vermutl. Latenz/Ablenkung. Zwei
  Lesarten: (a) Chance — liefern was sie nur bewerben (Transparenz-Differenzierung), oder
  (b) Warnung — sie haben's verworfen, aus gutem Grund. Bleibt billig (Version A), Daten da.
- Konfidenz hoch, aber nicht absolut: 4 Videos gecheckt (nicht der exakte Sydney-Ad, via
  iSpot.tv findbar); Frame-Sampling hat Grenzen, aber Sub-Sekunden-Übergänge + 2 echte
  Screen-Recordings schließen einen versteckten In-Produkt-Diff praktisch aus.

### Plattform-Machbarkeit
- **Windows:** leicht. `src/FloatingBar.tsx` ist ein Tauri-Webview (React) — Overlay
  erweitern, Diff in JS oder Rust berechnen, beide Strings via Event an die Bar.
- **Android:** machbar, aber dupliziert. `KlarvoOverlayService.kt` existiert schon (Bubble).
  Text-Anzeige via TextView/Compose. Diff-Logik müsste in Kotlin nachgebaut werden
  (Android umgeht Tauri-IPC ~85%, eigene Pipeline) — oder geteilte Rust-Logik via JNI.
- **"Cloud Web":** ⚠️ unklar — vermutlich Dictation-Artefakt; Klarvo ist Windows+Android,
  keine Web-Version. Mutmaßlich gemeint: "gilt auch im Cloud-Cleanup-Modus". **Zu klären.**

### Prior Art: zwei verschiedene "Live"-Apps — Frespr vs. soink (2026-05-30)
Andy verlinkte einen r/macapps-Post ("I built a live streaming Wispr Flow"). ⚠️ Korrektur:
Reddit war anti-bot-gewallt; ein Titel-Websearch führte zuerst fälschlich auf **Frespr** —
der verlinkte Post ist aber **soink**. Es sind DREI verschiedene Dinge, keins davon = das
Strikethrough-Device aus dem Screenshot:

**Frespr** (Salah Saleh, AGPL OSS, github.com/salah-saleh/frespr, gratis):
- LIVE = ROH-Transkript wort-für-wort im **Overlay** während des Sprechens.
- STT = **Google Gemini Live API** (gehostetes Streaming; Streaming gemietet, nicht gebaut). Cloud.
- Cleanup **post-stop**: *"AI post-processing step… then injects the final result."*
- → zeigt KEINEN Live-Cleanup-Diff; Live ist nur der Roh-Text.

**soink** (soink.ai, macOS 13.5+ only, Closed Beta, ASR/LLM nicht offengelegt):
- LIVE = Worte erscheinen **direkt im Textfeld** wort-für-wort (**System-Input-Layer/Keyboard**,
  KEIN Overlay, kein Paste). *"Words stream out as you speak — word by word, in real time."*
- *"AI polishes grammar and filler in the **background**"* — ob in-place-mutierend oder
  post-utterance ist NICHT offengelegt; **kein** Diff/Strikethrough erwähnt.
- Plus Voice-Editing ("change Tuesday to Wednesday") + Voice+Keyboard interleaved + Voice-Send.
- Maker: *"built on the system keyboard layer, not a regular app… over half a year"* — die
  harten Probleme lagen genau dort. = **anderes Produkt-Kategorie**, nicht "Overlay-Feature".

**Vier Tiers, sauber getrennt:**
- **(A) Post-Stop-Cleanup-Diff-Overlay** — billig, Daten in v1 da. Klarvo könnte das heute.
- **(A+) Live-ROH-Transkript-Overlay + Post-Stop-Cleanup** — = Frespr/Wispr-iOS. Braucht
  Streaming-STT-Provider auf dem Recording-Pfad (Gemini Live etc.); v1 ist heute Batch/Groq.
  Begrenzte Erweiterung, kein Rewrite; reibt am BYOK/Lokal-Narrativ (Dauer-Stream Cloud).
- **(B) Live-CLEANUP-Diff während Sprache** (inkrementelles LLM auf Partials, sichtbar) —
  baut NIEMAND, auch soink nicht. Das Screenshot-Device. Vaporware.
- **(C) Live-IME-Co-Writer** (= soink) — Voice als System-Input-Method, Text materialisiert
  im Feld, Background-Polish, Voice+Keyboard verschmolzen. **Keine Klarvo-Erweiterung, sondern
  Ersatz des record→process→paste-Kerns durch einen IME.** Auf Windows = TSF-Text-Service, auf
  Android = echte IME statt Accessibility/Paste. Massiver Rearchitektur, weit jenseits (B).

**Fazit:** Andys "Live-Cleanup in Videos"-Erinnerung = Wisprs Marketing-Strikethrough (Ad) +
Live-Roh-Transkript/IME-Apps (Frespr/soink). Der sichtbare Live-Cleanup-Diff selbst ist
Vaporware. Für Klarvo realistisch: (A) sofort. (A+) bewusste Streaming-Erweiterung. (C) wäre
ein neues Produkt, kein Feature.

### Code-Realität in v1 (2026-05-30) — "pure batch / Rewrite nötig" war zu absolut
Andy erinnerte einen Live-Feed in der Bar + "wir cleanen eh in chunks". Beides am Code belegt;
korrigiert meine frühere Überzeichnung:
- **Live-Raw-Preview existierte voll** und ist nur auskommentiert: `FloatingBar.tsx:389-405`
  + Backend-Command `transcribe_live_preview` (`commands/recording.rs:350`, noch registriert
  in `lib.rs:891`). **Grund der Deaktivierung im Code dokumentiert:** *"causes 10-20x Groq API
  quota usage with no meaningful UX benefit; waveform sufficient."* Mechanismus = Polling alle
  3s → `snapshot_wav()` des GANZEN Puffers → komplette Re-Transkription. D.h. ein
  Kosten-/UX-Problem (Text schreibt sich bei jedem Poll neu), KEINE Feasibility-Wand. Und exakt
  für LANGE Einsprechungen am schlimmsten (re-transkribiert die ganze wachsende Aufnahme).
- **`chunked_cleanup` existiert** (`llm/mod.rs:1297`): langes Transkript → split → Chunks
  **parallel** bereinigt, "reducing wall-clock time". ABER **post-stop** (chunkt den fertigen
  Text für parallele LLM-Calls), nicht chunk-as-you-speak. Latenz-Opt, kein Streaming.
- **Auto-Hotkey-Mode = continuous dictation**: "each silence gap triggers a transcription
  cycle" (`types.ts:9`, `auto_loop_active` in pipeline.rs). D.h. v1 segmentiert in Auto-Mode
  schon bei Sprechpausen und fährt pro Segment eine Transcribe(+Cleanup+Paste)-Schleife.
- **Folgerung:** v1 ist KEIN starrer Batch-Monolith. Für Andys Real-Bedarf (lange Einsprechung
  mitlesen) ist der billigste Pfad NICHT der deaktivierte Poll-Re-Transcribe, sondern die
  **Auto-Mode-Segmente in der Bar sichtbar machen, wie sie fertig werden** — phrase-für-phrase
  bei jeder Pause, jedes Segment nur 1× transkribiert, keine Re-Transcribe-Kosten, kein
  Jumpy-Rewrite. (A+) via echtem Streaming-STT bleibt die teurere Alternative.

### Sizing der gewählten Idee (2026-05-30) — "Auto-Mode-Phrasen-Feed in der Bar"
**Andy: "mir gefällt deine idee."** Bounded-Scoping (NICHT gebaut):
- Auto-Mode (`pipeline.rs:689 start_auto_recording`) ruft pro Sprechpause via Silence-Callback
  `stop_and_process_pipeline()` → das **emittiert bereits `state=done` mit `text`+`rawText`** an
  die Bar (gleicher Pfad wie der heutige done-pop; `StateChangedPayload`, `types.ts:12-19`).
- **Backend-Datenpfad existiert also schon.** Pro Segment kommt der fertige Text in der Bar an;
  sie verwirft ihn nur (done-pop → reset), statt zu akkumulieren.
- **Gap = reines Frontend:** Segment-Texte während einer Auto-Session **akkumulieren** + eine
  **erweiterte Bar-Ansicht** (scrollbarer Textbereich statt Pill). Kein neues STT, kein
  Streaming, keine Mehrkosten. Verzögerung = ein Beat pro Phrase (nach STT+Cleanup des Segments),
  nicht wort-für-wort — für "mitlesen bei langer Einsprechung" passend.
- **3 Entscheidungen für eine spätere Story:** (1) Feed zeigt `text` (cleaned) oder `rawText`
  (verbatim) — Andys Default ist Verbatim [[polished-designschwaeche]]? (2) bleibt der Feed nach
  Session-Ende stehen (Review) oder verschwindet? (3) Bar-Expand-UX (manuell/auto, Größe/Position
  — LP-Size-Infra war v2-Epic-11, in v1 neu in `FloatingBar.tsx`).
- **NICHT jetzt bauen.** Wenn Implementierung: via Feature-Route ([[v1-resume-pivot]] L3 —
  project-context.md → Story; Surface-Story → Windows-Release-Build-Smoke-Test im DoD
  [[smoke-test-dod-gate]]). Dies hier ist sized & geparkt.

### Offene Fragen
1. "Cloud Web" — was genau? (s.o.)
6. Wenn (A+) gewünscht: welcher Streaming-STT? Gemini Live (BYOK-Google, aber Dauer-Stream
   an Cloud, kein Offline) vs. Deepgram/AssemblyAI-Streaming vs. lokaler Streaming-Whisper.
   Spannungsfeld zu Klarvos BYOK/lokal-Narrativ explizit machen.
2. Anzeige-Timing: nur kurz vor Paste, oder bleibt das Overlay bis Bestätigung stehen
   (Wispr hat ✓/✗-Buttons im Screenshot → optionaler Confirm-Step statt Auto-Paste)?
3. Diff-Granularität: reicht Wort-Level-Diff, oder braucht es Phrasen-Korrekturen
   ("Friday → Thursday") als zusammenhängende Ersetzung?
4. Offline/Local-Cleanup-Modus: gibt es da überhaupt einen Cleanup-Diff zu zeigen, oder
   nur im LLM-Pfad? (Offline-Pfad pastet teils raw → kein Diff.)
5. Verhältnis zu "Confirm-before-Paste" — wird daraus implizit ein Edit-vor-Paste-Flow?

---

## Single-Instance-Lock (nur eine Klarvo-Instanz gleichzeitig)

- **Status:** `idea`
- **Erfasst:** 2026-06-01
- **Herkunft:** Gerettet aus einem verwaisten v2-Traceability-Artefakt
  (`phase-2-hardening-proposal.md` Item H1, urspr. 2026-05-04 via bmad-tea). Der Rest des
  Artefakts (v2-Coverage-Matrizen gegen abgebrochenen v2-Code) wurde verworfen
  ([[v1-resume-pivot]]); nur dieses Item ist v1-relevant.

### Was es ist
Verhindern, dass eine zweite Klarvo-Instanz parallel startet: beim Zweitstart das laufende
Fenster fokussieren und den neuen Prozess beenden — kein Doppelbetrieb.

### Warum das zählt — schließt ein Loch in Epic 1
Zwei Instanzen brechen genau die Garantien, die Epic 1 gerade eingezogen hat:
- **Story 1.3 (Single-Writer-Config-Serialisierung):** der Lock ist *prozessintern* — über
  zwei Prozesse hinweg greift er NICHT. Zwei Instanzen = zwei Writer auf dieselbe
  `config.json`, die 1.3-Serialisierung ist umgangen.
- **Story 1.5 / History-DB:** zwei Prozesse auf derselben SQLite-`history.db` → Lock-Conflicts.
- **Surface:** Double-Paste, Hotkey-Race auf dem Tray (beide greifen denselben globalen Hotkey).

Single-Instance ist damit kein reines Komfort-Feature, sondern die *prozessübergreifende*
Klammer um die prozessinternen Locks aus Epic 1.

### Code-Realität (2026-06-01)
Verifiziert: v1 hat **kein** Single-Instance-Handling (keine Logik, nichts in `Cargo.toml`).
v1 ist **Tauri 2** → `tauri-plugin-single-instance` **2.x** passt direkt (kein Custom-Mutex):
`single_instance::init` am Builder + Zweitstart-Fokus-Event an das laufende Fenster.

### Skizze für eine spätere Story
- Smoke: zwei Instanzen starten; die zweite beendet sich (`exit_code != 0`, loggt
  „already running"), die erste bleibt funktional.
- i18n-Key `app.single_instance.already_running` registrieren (G3 lint-events).
- Aufwand laut v2-Schätzung ~1 dev-day inkl. PR + manuellem Smoke. **Surface-Story →
  Windows-Release-Build-Smoke im DoD** [[smoke-test-dod-gate]].
- Scope zunächst **Desktop/Windows**; Android läuft als single Activity/Service, „zweite
  Instanz" ist dort anders gelagert.

### Offene Fragen
1. Zweitstart soll das Fenster ggf. aus dem Tray holen (falls „im Hintergrund weiterlaufen"
   geplant), nicht nur den Prozess killen.
