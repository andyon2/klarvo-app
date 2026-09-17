# Entscheidungsblatt — Parity-Linie über Audit #2 (2026-09-16)

**Zweck:** Andi zieht die Linie je Zeile über `docs/cross-platform-drift-audit-2026-09-16.md`. Dieses Blatt
übersetzt die 78 Audit-Zeilen in Entscheidungen. Es misst nicht neu. Jede Aussage hier steht mit
Code-Beleg im Audit unter derselben `D-*`-ID.
**Interaktive Fassung (Andi hakt ab):** https://claude.ai/code/artifact/fb5706a5-5b46-4050-8432-47b00c8c156a — Antworten liegen in der Artefakt-DB (`sheet/parity-2026-09-16`).
**Beantwortet:** 2026-09-17 per Artefakt → ADR-0016 Amendment 4 (`301d19d`). Dieses Blatt ist damit abgeschlossen.
**Ergebnis:** ADR-0016 **Amendment 4** (eigener Commit), dann `docs/backlog.md`, `sprint-status.yaml`,
Routing-Hook. Gebaut wird nichts vor einem eigenen Schnitt und Andis Go.

## So antwortest du

1. Beantworte zuerst die drei Grundsatzfragen **G1–G3** mit dem Buchstaben. Sie entscheiden ~18 Zeilen auf einmal.
2. Gehe dann **Block A bis G** durch. Pro Block reicht „Block X: deine Vorschläge". Abweichungen nennst du mit Zeilen-ID, zum Beispiel „C9: pasteDelayMs verdrahten".
3. Nutze genau vier Urteile: **schließen** · **Asymmetrie** (bleibt, wird in der ADR-Liste dokumentiert) · **streichen** · **offen** (Produktfrage, geparkt mit benannter Frage).
4. Bei „schließen" gilt mein Vorschlag zu **Form** und **Richtung**, wenn du nichts anderes schreibst. Form = **Gate** (Regler auf Android verstecken) / **Port** (nachbauen) / **Fix**. Richtung = die Plattform, deren Verhalten das Ziel ist.
5. Zeilen mit 🤖 kannst du nicht selbst am Gerät prüfen. Mit „deine Vorschläge" stufst du den Human-Gate dort bewusst auf „nur agent-verifiziert" herunter. Willst du das für eine Zeile nicht, schreib „Herstellbarkeit mitbauen" dazu.
6. Du musst weder Code noch das Audit lesen. Wenn du eine Zeile nachlesen willst: im Audit nach der `D-*`-ID suchen.

Test-Zeichen: 📱 = du am Android-Gerät · 🖥️ = du am Windows-Rechner · 🤖 = nur Agent (Unit-Test, Golden-Vektor, Log).

---

## Drei Grundsatzfragen

**G1 — Lizenz. Soll der Desktop die Lizenz in der Diktat-Pipeline durchsetzen?**
Heute: Android setzt durch, Desktop nicht (D-C1). Beide definieren „Free-Tier" verschieden (D-H1).
- **a)** Ja, wie Android. Paywall auf beiden Seiten. → D-C1 schließen (Desktop, M), D-H1 eine Definition, A5 + A6 schließen.
- **b)** Offen, bis Veröffentlichung und Lizenzwahl entschieden sind. Zustand bleibt und wird als bekannte Asymmetrie dokumentiert. Nur A1 und A2 werden gefixt.
- **c)** Nein. Android-Durchsetzung abschalten, kein Paywall nirgends. (Verwirft gebaute Lizenz-Logik.)
- Bei **a)** zusätzlich: Free-Tier = **nur Groq** (Android heute) oder **DeepSeek + Groq** (Desktop heute)? Schreib „G1a-Groq" oder „G1a-DeepSeek".
- Kein Vorschlag von mir. Das ist eine Produktfrage, verwandt mit der offenen Veröffentlichungsfrage vom 2026-09-04.

**G2 — Offline. Was verspricht „Offline"?**
- **a)** Kein Byte verlässt das Gerät. STT lokal ⇒ Cleanup lokal oder gar nicht. (Desktop-Hotkey-Semantik.) → E2 schließen auf Android + Rust-In-App-Button, S.
- **b)** Nur STT ist lokal. Cleanup läuft mit dem eingestellten Anbieter, auch Cloud. (Android-Semantik.) → Desktop ändert seine Offline-Regel.
- **Vorschlag: a).** E1 (Preview lädt offline hoch) ist unabhängig davon Pflicht.

**G3 — Android Offline-Angebot.**
- **G3a Offline-STT auf Android.** Heute läuft nur `ggml-small`, ohne Wörterbuch, ohne Ghost-Strip (D-H8, D-H13).
  - **a)** Verstecken (Gate S). B5, C2, D7, D8 entfallen.
  - **b)** Behalten und richtig machen: C2 Port S, B5 S–M, D7 S, D8 S.
  - **Vorschlag: a)**, solange du Offline-STT am Handy nicht nutzt.
- **G3b Lokales Cleanup auf Android.** Kann heute nicht laufen und meldet Rohtext als Erfolg (D-H18).
  - **a)** Jetzt verstecken (Gate S): „Local (Offline)"-Cleanup + Modell-Manager auf Android weg. Bau = eigene spätere Entscheidung (L: native Bibliothek, Build, Modellpfad).
  - **b)** Jetzt bauen (L).
  - **Vorschlag: a).** Unabhängig davon Pflicht: ein Fehlschlag wird nie als Erfolg gemeldet (D1, S).

---

## Bereits entschieden (Amendment 3), Audit bestätigt — keine neue Frage

- **C3** D-H14 outputLanguage → schließen S.
- **C6** D-H17 Auto-Send → Gate S (kein Auto-Send auf Android).
- **C8** D-L1 Whisper-Felder → Gate S.
- **D-H11** OpenAI als STT-Anbieter → schließen S–M, Provider-Schalter in Rust.
- **D-L13** Wörterbuch-Leer-Term-Guard → schließen S.
- **D6** D-M12 `copyToClipboard` ohne try/catch → schließen S.
- Webhook (M14) bleibt offen, gekoppelt an das fehlende Desktop-Feld. `voiceCommandEnabled` bleibt ausgeschlossen (kein Regler).

## Drei Amendment-3-Urteile, die das Audit korrigiert

- **H5 Anthropic** „Feld auf Mobile korrekt versteckt" ist falsch. Key-Feld und Modell-Feld rendern auf Android (D-M17) → **C7**.
- **M15 STT-Retry** „stiller Verlust ist weg" ist nicht ganz richtig. Nach einem lokalen STT-Fehler liegt die Aufnahme auf Android ohne History-Eintrag im pending-Ordner und wird nach 7 Tagen gelöscht (D-M8) → **D7**.
- **7-10 Clipboard** bestätigt, plus Desktop-Seite: die Pill sagt „In Clipboard" für Text, der nirgends ist (D-M12) → **D6**.

---

## Block A — Lizenz-Nebenzeilen

- **A1** D-C2 — Unlizenziert + „Offline"-Cleanup: Android schickt jedes Transkript an Groq, Desktop bleibt lokal. → **schließen S**, Android nimmt `local` von der Lizenz-Überschreibung aus. 🤖
- **A2** D-H2 — Android-UI zeigt die gewählten Anbieter, die Runtime nutzt still Groq. → **schließen S**: Anbieter-Regler mit `isPaid` sperren, die Sperre nennt die Anbieter. 📱
- **A3** D-M23 + D-L30 + D-L7 — Die Paywall bewirbt Snippets, Cross-Device-Sync, Command-Mode. Keins hat eine UI, auf keiner Plattform. Das Sync-Gate greift nirgends. → **offen, gekoppelt an G1**. Sofort-S: die beworbene Liste auf existierende Features kürzen. 🖥️📱
- **A4** D-L31 + D-L32 — Ein Bibliotheks-Ladefehler macht einen lizenzierten Nutzer zu „No API keys configured"; toter zweiter Lizenz-Einstieg. → **Sweep-Hygiene S**. 🤖
- **A5** D-M19 — Desktop: `config.json` löschen = neue 14-Tage-Trial. Android nutzt die OS-Installzeit. → **gekoppelt an G1** (a: schließen S, Richtung Android; b: Asymmetrie). 🤖
- **A6** D-L9 — Desktop erkennt eine neu aktivierte Lizenz erst nach Neustart. → **gekoppelt an G1**. 🖥️

## Block B — Wächter und Kern-Output (Pflicht-Klasse, du bestätigst Richtung)

- **B1** D-H3 — Desktop hat keinen Banking-/Passwort-App-Schutz. Android hat ihn, speichert aber History und Turso, bevor der Guard den Paste blockt. → **schließen**: Desktop Blocklist per Prozessname **M**; Android Reihenfolge fixen **S**. 🖥️📱
- **B2** D-H5 + D-H6 + D-M9 — Android: Echo- und Fragment-Guard sehen das Wörterbuch. Kurze Sätze aus Wörterbuch-Wörtern werden verworfen. Ein Begriff ≥ 10 Zeichen wird aus jedem Transkript gelöscht. Reihenfolge invertiert. → **schließen S** in Rust `groq_jni.rs`, Richtung Desktop. 📱 Test: Wörterbuch „Klarvo, Kubernetes", sag „Klarvo und Kubernetes."
- **B3** D-H7 — Ghost-Strip: Desktop nur nach dem Cleanup und verwirft ein Transkript mit End-Ghost ganz. Android nur vor dem Guard, ein vom LLM „reparierter" Ghost bleibt. → **schließen S**: beide Seiten machen beides. 🤖
- **B4** D-H4 — Android gibt die Cleanup-Anweisung als Whisper-Prompt. Die drei STT-Prompt-Felder sind auf Android editierbar und tot. → **schließen S**: Kotlin liest `advanced.sttPrompt*` und reicht sie durch; `customPrompt` geht nur ans LLM. 📱 Test: Preset „Technical" setzen, gleiches Audio.
- **B5** D-H8 — Offline-STT auf Android ohne Wörterbuch und Ghost-Strip. → **gekoppelt an G3a**. 🤖
- **B6** D-M10 + D-L19 + D-L21 — VAD: Android fest 0,5 statt Hysterese 0,5/0,35; Hangover ein Frame (32 ms) früher. → D-M10/D-L19 **Asymmetrie** (Amendment 3 bestätigt); D-L21 **Sweep S**. 🤖

## Block C — Regler auf Android sichtbar, dort tot (Pflicht: Gate oder Port)

- **C1** D-H12 sttModel — Modellwahl wirkt nur auf Desktop; die JNI kann es schon. → **Port S** (Kotlin liest den Key, reicht durch). 🤖
- **C2** D-H13 localWhisperModel — Android fest `small`; der Standard-Download der UI ist auf Android unbrauchbar. → **gekoppelt an G3a** (a: Gate; b: Port S). 📱
- **C3** D-H14 outputLanguage — Übersetzung nur Desktop. → **schließen S**, Amendment 3 bestätigt. 📱
- **C4** D-H15 Profile — Profile zünden auf Android nie (Amendment 3: schließen M, bestätigt). **Neu:** `profiles[].language` wirkt auf keiner Seite, die UI verspricht es. → Sprache pro Profil verdrahten (S) **oder** das Versprechen aus dem Text streichen (S)? **Vorschlag: streichen.** 📱
- **C5** D-H16 Stille-Dauer-Slider — Der Android-Slider ändert nichts; wirksam ist immer der Desktop-Default 2,0 s. → **schließen S**, Richtung: der Regler wirkt. 📱
- **C6** D-H17 Auto-Send — Schalter rendert auf Android, tut nichts. → **Gate S**, Amendment 3 bestätigt. 📱
- **C7** D-M17 Anthropic — Key-Feld + Modell-Feld rendern auf Android ohne Verbraucher (Amendment 3 irrte). → **Gate S**. 📱
- **C8** D-L1 Whisper-Felder — zwei Advanced-Felder auf beiden Seiten sichtbar, Schalter nirgends. → **Gate S**, Amendment 3 bestätigt. 📱
- **C9** D-L2 pasteDelayMs · D-L3 logLevel — auf **beiden** Seiten tot, beide Regler sichtbar. → **pro Key.** Vorschlag: beide Regler **entfernen** (S). Alternative: Desktop verdrahten + Android-Gate (S je Key). 🖥️📱
- **C10** D-L4 previewBgBlur — Der Desktop-Slider bewegt nur die Vorschau im Settings-Fenster, nie das Overlay (seit Epic 10). → **entfernen S**. Alternative: in `native_preview` verdrahten (M). 🖥️
- **C11** D-L6 — Keys ohne Regler auf beiden Seiten: `voiceNotesHotkey`, `bubbleSize`, `bubbleOpacity`, `bubbleRecordingMode`, `webhookHeaders`, `webhookTimeoutSecs`. → **Sweep S**: aus der Config entfernen. 🤖
- **C12** D-L27 — History-Filter „App…" ist auf Android immer leer; Android speichert keinen App-Namen. → **Port S** (Paketname aus dem a11y-Service) oder Gate S. **Vorschlag: Port.** 📱
- **C13** D-M15 — Das Statistik-Panel zählt Android-Diktate mit 0 s und 0,00 $. → **Port M** (Kotlin schreibt Usage-Zeilen) oder **Gate S**. **Vorschlag: Gate jetzt, Port als Kandidat.** 📱

## Block D — Stiller Verlust / stilles Nichtstun (Pflicht-Klasse)

- **D1** D-H18 — Android Local-Cleanup kann nicht laufen, meldet Rohtext als Erfolg. → **gekoppelt an G3b**. Unabhängig: Fehlschlag ⇒ Zwischenablage + Ursache (S). 📱
- **D2** D-H19 — Leere LLM-Antwort: Android fügt ein leeres Feld ein und speichert „" in der History. → **schließen S** (Zwilling zu Desktop). 🤖
- **D3** D-M16 — Abgeschnittene LLM-Antwort: Android fügt den halben Satz ein. → **schließen S**. 🤖
- **D4** D-H20 — Kein Eingabefeld fokussiert: Android zeigt den Haken, nichts erscheint, der Text liegt unbemerkt in der Zwischenablage. → **schließen S**, Richtung Desktop („In Zwischenablage"-Hinweis). 📱
- **D5** D-M24 — Android zeigt den Erfolgs-Haken bei fehlgeschlagenem Cleanup. → **schließen S** (Pill = Statuslicht, wie 7-10). 📱
- **D6** D-M12 — Clipboard-Fehler: Android ohne try/catch (Crash); Desktop sagt „In Clipboard" für Text, der nirgends ist. → **schließen S** auf beiden Seiten. 🤖
- **D7** D-M8 — Android: Aufnahme nach lokalem STT-Fehler ohne History-Eintrag, nach 7 Tagen gelöscht. → **gekoppelt an G3a** (b: schließen S). 🤖
- **D8** D-M7 — Groq 429 erreicht das lokale Sicherheitsnetz nur auf Desktop. → **gekoppelt an G3a**. 🤖
- **D9** D-M5 + D-M6 — Leeres STT-Ergebnis: Android verbrennt 3 Groq-Aufrufe und ~7 s; Retry-Budget 3 vs. 1. → D-M5 **schließen S** (nicht wiederholbar markieren); D-M6 **Asymmetrie**. 🤖
- **D10** D-M2 — Kaputte Anbieter-Antwort bei kurzem Diktat: Android ohne Fallback. → **Sweep S**. 🤖
- **D11** D-M14 — Nichts erkannt: Desktop stumm (Absicht „wie Wispr Flow"), Android zeigt Toast. → **Asymmetrie** (Design). Willst du Desktop-Hinweise, dann schließen S. 🖥️
- **D12** D-M13 — „Erneut verarbeiten" auf Android läuft mit Desktop-Regeln; live vs. Retry kann abweichen. → **Asymmetrie**, dokumentieren. 🤖

## Block E — Offline-Privatsphäre

- **E1** D-H9 — Android „Offline" + Live-Preview lädt jede Pause zu Groq hoch, auch ohne Groq-Key. → **schließen S**, Pflicht, unabhängig von G2. 🤖
- **E2** D-H10 + D-M20 + D-M21 — „Offline"-STT + Cloud-Cleanup: Desktop ohne Netz, Android sendet an DeepSeek. Der In-App-Button hat eine dritte Regel. Eine gemischte Config liefert drei Ergebnisse. → **gekoppelt an G2**. 📱 Test: Offline wählen, mit Füllwörtern diktieren. Bleiben die „ähm"?

## Block F — Sync / History / Metriken (Turso hat keine UI, nur config.json)

- **F1** D-M3 — Android markiert Zeilen als synchronisiert bei HTTP 200, ohne die Antwort zu lesen. → **schließen S**. 🤖
- **F2** D-M4 — Desktop pusht nur den Live-Eintrag und legt keine Tabelle an; Android pusht alles. → **gekoppelt an A3**. Vorschlag: offen. 🤖
- **F3** D-L25 — `http://`-Turso-URL: Desktop lehnt ab, Android sendet den Token im Klartext. → **Sweep S**. 🤖
- **F4** D-L24 · D-L23 · D-L26 — Datumsformat, Pull-Richtung, Ressourcen. → D-L24 **Sweep S**; D-L23/D-L26 **keine Aktion**, dokumentieren. 🤖
- **F5** D-L28 · D-L29 — Feedback-Metriken: zwei nicht-atomare Schreiber (ADR-0015-Klasse); die Zähler bedeuten je Seite etwas anderes. → D-L28 **Sweep S**; D-L29 **Asymmetrie**, dokumentieren. 🤖

## Block G — Zwillings-Hygiene (alle S, kein Produkturteil)

Vorschlag: eine Hygiene-Liste im Sweep, kein Einzel-Urteil. „Block G: deine Vorschläge" reicht.

- **Fix S:** D-M1 Config-Load-Leiter (Richtung: 12-1-Leiter) · **D-M11** unbekannter `cleanupStyle` setzt auf Desktop die ganze Config zurück (Richtung Android: still Polished) · D-M18 leer-vs-blank-Prädikate · D-M22 lokales Cleanup ohne Chunking (latent, mit G3b) · D-L5 deviceId · D-L10 Trim · D-L11 alphanumerisch · D-L12 HTTP-2xx · D-L13 (Amendment 3) · D-L14 Sanitize-Passthrough · D-L15 Blank-Tabellen · D-L16 Clamp · D-L32 toter Einstieg · drei falsche Docstrings (§8 des Audits).
- **Nur dokumentieren (unerreichbar oder nur Anzeige):** D-L8 · D-L17 · D-L18 · D-L20 · D-L22.

---

## Schnitt-Form (Vorschlag, entscheidest du nach den Urteilen)

- **Sammel-Story „Parity-Sweep 2026-09"** = alle S-Zeilen aus B–G. Golden-Vektoren für B2, B3, D2, D3 in `test-fixtures/`; `Adr0017BoundaryGuardTest` erweitern.
- **Eigene Stories:** B1 Banking-Guard Desktop (M) · C4 Profile Android (M) · D-H11 OpenAI-STT (S–M) · bei G1a: Desktop-Lizenz-Gate (M) · bei G3b-b: Android Local-Cleanup (L).
- **Bestehende Kandidaten unverändert:** 9-8, 8-6, 8-7.
