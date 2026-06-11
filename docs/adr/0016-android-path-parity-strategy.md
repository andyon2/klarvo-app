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
