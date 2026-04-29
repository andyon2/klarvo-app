# Phase-3 Android Play Store Policy Audit

**Status:** Pending-Google-Response  
**Erstellt:** 2026-04-29  
**Abhängige Phase:** Phase-3 (AccessibilityPasteBackend-Implementierung)

---

## Fragestellung

**Zulässigkeit von AccessibilityService für non-assistive use case unter aktueller Play-Store-Policy.**

Klarvo's Android-Implementation plant AccessibilityService zu nutzen für:
- Erkennung des Fokus-Fensters (aktive App beim Hotkey-Press)
- Paste-Operation in das fokussierte Fenster (Ctrl+V-Äquivalent via AccessibilityService)

Dies ist kein assistive-technology use case (Barrierefreiheit) — es ist ein Power-User-Workflow
(Hotkey → Voice → Paste). Google's Policy für AccessibilityService ist restriktiv und erfordert
in manchen Fällen Policy-Review oder Ausnahme-Genehmigung.

**Konkrete Einreichungsfrage an Google Developer Support:**

> "We plan to use Android's AccessibilityService exclusively to detect the currently focused
> window and programmatically paste transcribed text into it (equivalent to Ctrl+V). This is
> not an assistive technology feature — it's a keyboard shortcut replacement for power users.
> Is this use case permitted under the current Play Store AccessibilityService policy, and if
> so, what declaration is required in the app's accessibility declaration?"

---

## Einreichungs-Log

| Datum | Kanal | Status | Notizen |
|-------|-------|--------|---------|
| (ausstehend) | Google Developer Support | Nicht eingereicht | — |

---

## Eskalations-Pfad bei abgelehnter Policy

Priorität-1: **Alternativer Paste-Mechanismus**
- Input Method Service (IME) als Paste-Backend: komplexer, aber Play-Store-konform
- Limitation: IME muss aktiv sein, kein transparent-background-Setup

Priorität-2: **Direct APK Distribution (kein Play Store)**
- Klarvo als Sideload-APK ohne Play-Store-Distribution
- Target: Power-Users / Developer-Audience (akzeptiert Sideloading)
- Limitation: kein Mainstream-Reach, kein Auto-Update via Play

Priorität-3: **Phase-3 scope anpassen**
- Android-Phase nur für Accessibility-kompatible Features (z.B. nur Aufnahme + Core-Pipeline)
- Paste-Delivery über alternativen Mechanismus (Clipboard + Benachrichtigung)

---

## Phase-3-Dependency

**AccessibilityPasteBackend-Implementierung darf NICHT starten** bevor:
- [ ] Google-Response positiv, ODER
- [ ] Alternativer Paste-Mechanismus evaluiert und entschieden

Ref: `memory/project_play_store_phase3_blocker`

---

## Nächste Schritte

1. Google Developer Support kontaktieren (Ticket öffnen oder Developer Community Forum).
2. Response-Log oben aktualisieren sobald Antwort eingeht.
3. Bei positiver Response: Phase-3-Story für AccessibilityPasteBackend freischalten.
4. Bei negativer Response: Eskalations-Pfad-Entscheidung vor Phase-3-Kickoff.
