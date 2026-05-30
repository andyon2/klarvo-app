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
