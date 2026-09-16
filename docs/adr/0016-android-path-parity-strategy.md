# ADR-0016: Android-Pfad-Paritäts-Strategie — Linie ziehen mit Wächter-Ausnahmeliste

**Status:** Accepted
**Date:** 2026-05-30

## Context

Das Robustheits-Audit (`docs/robustness-audit-2026-05-30.md`, §3) hat **14 verifizierte Rust↔Kotlin-Invarianten-Divergenzen** (DIV-01..14) gefunden. Hintergrund: ~2000 LOC Logik-Duplikat zwischen dem Tauri-IPC-Pfad (Desktop, Rust) und dem nativen Android-Pfad (Kotlin, `android/kotlin-src/com/klarvo/voice/`). Diese Duplikation war das **treibende Argument** für den v2-Rebuild. Seit dem Pivot 2026-05-29 ist v2 jedoch Blueprint-Referenz und **`v1-ship` ist das Produkt — Android inklusive**.

Die Befunde sind systematisch, nicht zufällig: Der Desktop härtet, Android pastet roh. Die Frage ist nicht „fixen wir 14 Bugs", sondern: alle Divergenzen in Kotlin nachziehen (= das Duplikat vertiefen, das wir loswerden wollten) oder eine Linie ziehen?

## Decision

**Linie ziehen — mit Wächter-Ausnahmeliste.** Keine pauschale Feature-Parität. Nur die Wächter-Klasse (Daten-Integrität / PI) wird gehärtet; der Rest wird als bewusste, dokumentierte Asymmetrie akzeptiert.

### Härten (Wächter — Daten-Integrität/PI, kein „Feature", sondern Notwehr)

| ID | Divergenz | Warum Notwehr |
|----|-----------|---------------|
| DIV-01 / DIV-05 | Halluzinations-Filter fehlt komplett auf Android | Whisper-Phantomtext (`"Untertitelung des ZDF"`, `"[Music]"`) wird ins fokussierte Feld **jeder** App gepastet **und** in History + Turso-Sync persistiert. Plattform-asymmetrische Datenintegritätsverletzung. |
| DIV-03 | Paste-Sanitisierung nicht auf allen Pfaden | Bidi-Override/Zero-Width-Zeichen aus Roh-Transkript ins Zielfeld bei fehlendem/fehlgeschlagenem Cleanup → Text-Spoofing-Risiko. |
| DIV-04 | Banking-/Sensitive-App-Blocklist greift nur für die Bubble, nicht für den Paste-Pfad | Laufende Pipeline pastet weiter in eine Banking-App, wenn die Aufnahme vor dem App-Wechsel startete. „Nicht abschaltbarer Schutz" schützt faktisch nur die Bubble-Sichtbarkeit. |
| DIV-02 | Min-Längen-/Stille-Vorfilter fehlt | Verhindert genau die Halluzinationen aus DIV-01 und spart BYOK-API-Kosten/Latenz bei Mini-Taps. |

### Bewusst akzeptiert (dokumentierte Asymmetrie, won't-fix-on-v1)

Feature-Parität mit marginalem Nutzen, die die Duplikation vertiefen würde — **keine Stories**, hier als bekannte Asymmetrie festgehalten, damit sie nicht als „Bug" re-gefiled wird:

- **DIV-06** Provider-Fallback bei 429/5xx (Android → sofort Raw-Degrade statt Zweit-Provider)
- **DIV-07** Output-Language / Inline-Übersetzung (Android liest `outputLanguage` nicht)
- **DIV-08** Dictionary-Anwendung auf STT/Whisper-Prompt (Android nur im LLM-Cleanup)
- **DIV-09** Local-Cleanup-System-Prompt-Vollständigkeit (Android verkürzt)
- **DIV-10** Command-Mode + PI-Defense (existiert auf Android nicht)
- **DIV-11/12/13/14** (low) — Prompt-Echo-Guard, Doppel-Start-Atomarität, Provider-Allowlist-Reject, VAD-Parameter. Akzeptiert; optional späteres Polish, nicht v1-ship-Scope.

**Rationale:** Wächter schützen den User unabhängig vom Feature-Gleichstand. Volle Parität würde genau die Duplikation vertiefen, aus der das Projekt heraus will, für marginalen Nutzen. Die langfristige Antwort auf die Duplikation ist v2 — nicht weitere v1-Android-Investition.

**Alternative verworfen:** *Volle Parität (alle 14 nach Kotlin portieren)* — vertieft das ~2000-LOC-Duplikat, das der ursprüngliche v2-Treiber war; schlechtes ROI auf einem Produkt, dessen strategische Dedup-Antwort woanders liegt.

## Consequences

**Positiv:**
- Android-User sind vor den realen Datenintegritäts-/PI-Lecks geschützt.
- Das Android-Epic schrumpft von 14 Divergenzen auf **~4 Wächter-Stories**.
- Keine neue Duplikat-Investition jenseits der Notwehr.

**Negativ:**
- Eine bewusste Funktions-Asymmetrie Desktop > Android bleibt bestehen (Übersetzung, Dictionary-on-STT, Provider-Fallback, Command-Mode). Muss in der UX/Doku kommuniziert werden, sonst latente Support-Last.
- Würde Android je gleichwertig werden sollen, ist das v2-Arbeit, kein v1-Inkrement.

**Mitigations:**
- Die akzeptierten Divergenzen sind hier **und** im Audit-Doc als known-asymmetry gelistet → kein Re-Filing als Bug.
- Die Wächter-Stories laufen Heavy-Track mit Test Architect `*risk` (Integrations-Regressionspotenzial zwischen Rust- und Kotlin-Pfad).

## Referenzen

- `docs/robustness-audit-2026-05-30.md` — §3 DIV-01..14
- Memory: Pivot 2026-05-29 (v1-Resume), Android-umgeht-Tauri-IPC (~85% Bypass, ~2000 LOC Duplikat)

## Next Action

1. Commit zusammen mit ADR-0015 + Audit-Doc.
2. Heavy-Track-Epic „Android Sicherheits-Wächter" (gated by dieser ADR): Stories für DIV-01/05, DIV-03, DIV-04, DIV-02 mit Test Architect `*risk`/`*design`.
3. DIV-06..14 erhalten **keine** Stories — sie sind durch diese ADR als akzeptierte Asymmetrie geschlossen.

---

## Amendment 1 (2026-06-10) — chirurgische Linienverschiebung nach A/B-Drift-Audit

> Ergänzt die Decision, ersetzt sie nicht. Punkt 3 der „Next Action" oben galt für die
> DIV-01..14-Liste des Robustheits-Audits; dieses Amendment re-adjudiziert sechs dieser
> Einträge auf Basis eines breiteren, verifizierten Audits.

**Auslöser:** `docs/cross-platform-drift-audit.md` (verifizierter A/B-Lauf Fable 5 vs Opus 4.8)
fand Divergenzen, die DIV-01..14 nicht adjudiziert hat — insbesondere **Kern-Output-Determinismus-Drift**
(H2 UTF-8-Bytes vs UTF-16-Code-Units beim Chunking; H1/H17 Auto-Stop-Energie-Gate liest `silenceThreshold`
nicht) und **settable-but-silently-dead Config-Keys** (surface-operable Traps). Die Originallinie wurde
auf einer unvollständigen Liste gezogen (C1 + H2 fand erst dieser Lauf).

**Reframe:** Die ursprüngliche Decision wog die Divergenzen als *Feature-Paritäts-ROI*. Für
*Kern-Output-Determinismus* und *Config-Contract-Integrität* gilt diese Abwägung nicht: dieselbe Config +
derselbe Input erzeugt nachweislich anderen Diktat-Output, bzw. ein gesetzter Nutzer-Wert verpufft still.
Das ist Notwehr an Datenintegrität/Erwartungstreue, kein Feature.

**Härten-Klasse erweitert (chirurgisch) — neu in Stories (Epic 7, `epics-cross-platform-parity.md`):**
- Kern-Output: H2 (Chunking-Längeneinheit), H13/L4 (Join/Operator), M8; H1/H17/Recall#1/M1-3/M4/L1
  (Auto-Stop-Gate + Pre-STT-Schwellen).
- STT-Konditionierung (**übersteuert DIV-08**): H3 + Recall#5; H9/H10/L3 (Model-/Temp-Reads).
- Output-Guards (**übersteuert DIV-11**): H6 (prompt-echo), H7 (fragment-strip).
- Routing-Contract-Hygiene: M9/M10/M11/M13/M16/L5.
- Gegenrichtung (Desktop ist die falsche Seite): H14 — Androids Whole-Word-Match nach Rust zurückportieren.
- Struktureller Wächter: Golden-Vektor-Paritäts-Netz (C1-proper) gegen künftige Drift.

**Weiterhin bewusst akzeptiert — aber → `docs/backlog.md`, NICHT hard-won't-fix:** Die ROI-Begründung der
Original-Decision bleibt für reine Feature-Ports gültig; sie wandern in den Backlog (sichtbar, nicht als Bug
re-gefiled, nicht verloren):
- **DIV-06** (H12 Provider-Fallback), **DIV-07** (H4 outputLanguage), **DIV-09** (H11 Local-Cleanup-Prompt),
  **DIV-10** (L7 Voice-Command), **DIV-13** (H16 OpenAI-STT), **DIV-14** (M5 VAD-Statemachine).
- Feature-Ports: C2 (Whisper-Mode), H5 (Anthropic), H8 (Mic-Wahl), H15 (Per-App-Profiles), M14 (Webhook),
  Recall#4 (Live-Preview), M6 (WAV-Float), M15 (Desktop-STT-Retry).

**Offene Decision:** M12 (Dictionary-in-Chat-Style) — Gegenrichtung, kanonische Seite ist Produktfrage.
Als `OPEN-DECISION` im Backlog geführt; in Story 7.6 zu lösen.

**Rationale-Erhalt:** Die Linie bewegt sich nur dort, wo Drift den *Kern-Output* verfälscht oder einen
*gesetzten Config-Wert still verschluckt*. Das ~2000-LOC-Duplikat wächst minimal und gezielt; die
strategische Dedup-Antwort bleibt v2.

**Quellen:** `docs/cross-platform-drift-audit.md`; `_bmad-output/planning-artifacts/sprint-change-proposal-2026-06-10.md`.

---

## Amendment 2 (2026-06-12) — STT-Pfad: Konsolidierung statt chirurgischem Kotlin-Port

> Ergänzt Decision + Amendment 1, ersetzt sie nicht. Betrifft NUR den STT-Pfad; für alle
> anderen Rows gilt Amendment 1 unverändert.

**Auslöser:** `docs/dictation-quality-android-vs-desktop-2026-06-12.md`. Die Prämisse
"Phone = local Whisper" ist FALSCH — beide Plattformen fahren denselben Groq-Engine
(`whisper-large-v3-turbo`; verifiziert: Phone-Config `sttProvider=groq`, Code-Dispatch,
46/46 Runtime-Läufe `provider=groq`). Der reale, **gestärkte** Befund ist die
Zwei-Strang-Drift: zwei getrennte STT-Request-Implementierungen (Rust `GroqWhisper` +
Kotlin `KlarvoApi.transcribe`) gegen denselben Endpoint mit nachweislich unterschiedlichen
Parametern — und Guard-Twins, die in **entgegengesetzte Richtungen** divergiert sind (Rust
hat den H14-Substring-Bug, Kotlin hat den Whole-Word-Fix).

**Linienverschiebung (nur STT-Pfad):** Für den STT-Request + dessen Output-Guards + den
Pre-STT-Silence-Filter gilt ab jetzt **Konsolidierung in den Rust-Kern via JNI**
(**ADR-0017**, Hard-Rule: geteilte STT-/Guard-Logik nur in Rust, Kotlin-Nachbau verboten),
NICHT mehr der per-row Kotlin-Port aus Amendment 1. Die betroffenen Epic-7-Rows (alt 7.3
STT-Conditioning H3/Recall#5/H9/H10/L3, alt 7.4 Output-Guards H6/H7, Silence-Teil von 7.2
Recall#1/M1-read, H14) verschmelzen in eine Konsolidierungs-Story (neu 7.3); die
Kotlin-Twins (`KlarvoApi.transcribe`/`buildMultipartBody`, `HallucinationFilter.kt`,
`SilencePreFilter.kt`) werden gelöscht. Dazu neue Halluzinations-Härtung aus dem
Evidence-Run: Stockphrase-Blocklist-Familie auch für Long-Clip-Trailing-Ghosts (≤8-Wort-Gate
fällt), Groq `verbose_json` + Konfidenz-Drop, Cleanup-no-invent.

**Unverändert (Amendment 1 gilt weiter):** Live-Autostop-VAD-Gate (7.2, geschmälert —
Realtime-Audio über JNI ist bewusst NICHT im Scope), Chunking (7.1) und LLM-Routing (7.5)
bleiben platform-lokale per-row-Parität. 7.6 schrumpft auf die offene M12-Decision (H14
wird von der Konsolidierung miterledigt). Das Golden-Vektor-Netz (7.7) bleibt und pinnt
zusätzlich den Shared-STT-Vertrag.

**Quellen:** `docs/dictation-quality-android-vs-desktop-2026-06-12.md`;
`_bmad-output/planning-artifacts/sprint-change-proposal-2026-06-12.md`; ADR-0017.

---

## Amendment 3 (2026-09-16) — Linie neu gezogen: v1 ist das einzige Produkt, die Asymmetrie-Liste bekommt Urteile

> Ergänzt Decision + Amendment 1/2, ersetzt sie nicht. Die Klassen-Logik (Wächter / Kern-Output /
> still verschluckte Config-Werte = Pflicht auf beiden Seiten; reiner Feature-Port = Backlog) bleibt.
> Neu ist: (a) die Prämisse der Original-Rationale wird zurückgezogen, (b) jede Zeile der
> akzeptierten Asymmetrie trägt jetzt ein explizites Urteil von Andi.

**Auslöser:** Andis Gefühl „Android hinkt hinterher" (Session 2026-09-16), gegen den heutigen Code
geprüft. Das Drift-Audit vom 2026-06-10 ist ein Schnappschuss und wurde nie wiederholt; seitdem
liefen Epic 7 (Re-Cut), 8, 9, 10, 11, 12.

**Prämisse zurückgezogen:** Die Original-Rationale sagt *„Die langfristige Antwort auf die
Duplikation ist v2 — nicht weitere v1-Android-Investition."* v2 ist seit dem Pivot 2026-05-29
Blueprint-Referenz, kein Bauziel. Damit hatte der Android-Rückstand kein Ablaufdatum und keinen
Eigentümer. Ab jetzt gilt: **v1-ship ist das einzige Produkt, Android inklusive.** Ein Feature-Port
wird nicht mehr mit „kommt in v2" abgelehnt, sondern pro Zeile nach Nutzen und Größe entschieden.

**Drei Befunde, die die Juni-Liste ändern:**
1. **Still erledigt:** DIV-06/H12 Provider-Fallback bei 429/5xx steht seit Epic 12 auch auf Android
   (`KlarvoApi.cleanupFallbackCandidates`, `isRetryable`-Spiegel in `KlarvoOverlayService`).
2. **Falsch klassifiziert:** C2 Whisper-Mode hat zwei Eingabefelder in den Advanced-Settings
   (`AdvancedSettingsPanel.tsx`, Threshold + Gain) ohne Plattform-Weiche. Die React-Settings laufen
   auch auf Android. Die Felder sind dort sichtbar und wirkungslos = Klasse „gesetzter Wert verpufft
   still" aus Amendment 1 = Pflicht, nicht Asymmetrie. Gleiches gilt für DIV-07/H4 `outputLanguage`
   (`LanguageContent.tsx`, kein `isDesktop`-Gate).
3. **Billiger geworden:** DIV-13/H16 OpenAI als STT-Anbieter. Seit ADR-0017 baut Rust den
   STT-Request für beide Plattformen (`stt/groq_jni.rs`). Ein Android-Port ist ein Provider-Schalter
   im Rust-Kern, kein Kotlin-Nachbau.

**Urteile (Andi, 2026-09-16) — Zeilen-IDs aus `docs/cross-platform-drift-audit.md`:**

| Zeile | Urteil | Größe | Bemerkung |
|---|---|---|---|
| DIV-06/H12 Provider-Fallback | **erledigt** | — | Epic 12; aus der Liste gestrichen |
| Recall#4 Live-Preview | **erledigt** | — | Epic 11 |
| M15 STT-Retry-Asymmetrie | **entschärft, gestrichen** | — | 12-1 Safety-Net + 12-2 Re-Process; Mechanismus bleibt verschieden, stiller Verlust ist weg |
| M6 WAV-Float | **gestrichen** | — | latent, beide Seiten PCM16 |
| C2 Whisper-Mode | **Pflicht, Stufe S** | S | die zwei Advanced-Felder auf Mobile verstecken. Stufe M (Gain in Kotlin) nur auf eigene Freigabe. Befund am Rande: der Ein/Aus-Schalter hat seit dem Settings-Drill-Down auf KEINER Plattform eine Render-Stelle (`void localWhisperMode`) → Desktop-UI-Lücke, siehe Backlog |
| DIV-07/H4 outputLanguage | **schließen** | S | ein Übersetzungs-Satz in `KlarvoApi.appendPromptExtensions`, Zwilling zu `llm/mod.rs` |
| M14 Webhook | **bleibt offen, gekoppelt** | S | Bedingung „Feld auf Mobile sichtbar" ist FALSCH: `webhookUrl` hat seit dem Drill-Down auf keiner Plattform ein UI-Feld (`void localWebhookUrl`). Android-Port erst, wenn das Desktop-Feld zurück ist |
| DIV-13/H16 OpenAI-STT | **schließen** | S–M | Provider-Schalter in Rust (`groq_jni.rs`), Kotlin reicht den Key durch |
| H5 Anthropic-Cleanup | **bleibt Asymmetrie** | M | Feld auf Mobile korrekt versteckt (7-9); kein Schaden |
| H15 Per-App-Profile | **schließen** | M | auf Android per Paket-Name statt Fenstertitel; `BankingGuard` kennt den Vordergrund bereits |
| H8 Mikrofon-Wahl | **bleibt Asymmetrie** | L | Regler korrekt versteckt (`isDesktop`); Bluetooth-Routing ist ein anderes Problem |
| DIV-10/L7 Command-Mode | **bleibt Asymmetrie** | L | — |
| DIV-09/H11 Local-Cleanup-Prompt | **schließen als Beifang** | S | `appendPromptExtensions` in `KlarvoApi.cleanupLocal` aufrufen, sobald jemand dort arbeitet |
| DIV-14/M5 VAD-Mechanismus | **bleibt Asymmetrie** | L | 7-2 hat Schwellen + Highpass angeglichen |
| Auto-Send auf Android (aus 7-9) | **NEIN, entschieden** | — | kein Auto-Send auf Android. `performEnter` bleibt ungenutzt. Kein „possible future story" mehr |
| 8-8-Zwilling Clipboard-Feedback | **offen bis Epic 9 weitergeht** | S–M | Backlog Epic 9 |
| 9-6 Tastatur-Einklappen | **gestrichen** | — | war seit 2026-06-16 geparkt (Preview liegt über der Tastatur); jetzt endgültig |
| 9-8 Long-Press-Menü | **umsetzen** | M | entparkt; der Blocker (DEBUG_SET_STATE „tot auf HyperOS") ist seit 2026-08-12 widerlegt |
| 8-6 Onboarding, 8-7 Fidelity | **wieder aufnehmen (Desktop)** | M | entparkt; 8-6 braucht den Erreichbarkeits-Vorlauf (Config-Wipe-Zustand) |
| 7-10 `copyToClipboard` ohne try/catch | **schließen** | S | Android sieht Clipboard-Fehler sonst nicht |
| 7-10 Q4 lokaler `catch` fügt Rohtext still ein | **schließen** | S | eigener Bug |
| 7-6 Leer-Term-Guard `is_empty` vs `isNullOrBlank` | **schließen** | S | trifft seit 7-6 auch Chat |
| Tote Desktop-Keys (`pasteDelayMs`, `logLevel`, `webhookHeaders`, `webhookTimeoutSecs`) | **pro Key**, offen | S | nicht als Block |

**Was „schließen" bedeutet:** Kandidat im Backlog mit Größe. **Kein** Bau ohne eigenen Schnitt und
Andis Go. Die S-Zeilen (outputLanguage, Local-Prompt-Beifang, drei Zwillings-Hygiene-Punkte,
Whisper-Felder verstecken) eignen sich als EINE Sammel-Story „Parity-Sweep 2026-09"; H15 und
DIV-13 sind eigene Stories; 9-8, 8-6, 8-7 sind bestehende Story-Nummern.

**Unverändert:** ADR-0017 (STT nur in Rust), das Golden-Vektor-Netz, die Klassen-Logik aus
Amendment 1. Das Drift-Audit soll als zweiter Lauf wiederholt werden (Methode wie 2026-06-10),
bevor die Sammel-Story geschnitten wird — Andis Punkt 1, noch nicht beauftragt.

**Quellen:** Session 2026-09-16 (Andi, Zeile-für-Zeile-Entscheid); `docs/backlog.md` Abschnitt
„DECIDED 2026-09-16 — Parity-Linie neu gezogen"; Code-Belege in den Zeilen oben.
