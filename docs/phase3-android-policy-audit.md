# Play Store AccessibilityService Policy Audit

**Datum:** 2026-04-30
**Status:** AT-RISK
**Abhängige Phase:** Phase-3 (AccessibilityPasteBackend-Implementierung)
**Quellen verifiziert:** 2026-04-30

---

## Zusammenfassung

Klarvo's geplanter Use-Case (Hotkey → Dictation → Text-Injection via AccessibilityService) ist
unter der aktuellen Google Play Policy **grundsätzlich erlaubt**, fällt aber in eine Grauzone:
Die policy erlaubt explizit "deterministic, rule-based automation" (Trigger → Action) — was auf
Klarvo zutrifft — schließt aber gleichzeitig "automation tools" und "assistants" als nicht
qualifizierende Apps aus. Die Einordnung hängt davon ab, wie Google den Use-Case im
Play-Console-Declaration-Review bewertet. **Phase-3 darf nicht ohne erfolgreichen Review starten.**

---

## Policy-Quellen

### Quelle 1: Google Play AccessibilityService API Policy
URL: https://support.google.com/googleplay/android-developer/answer/10964491
Stand: 2026-04-30

Relevante Zitate:

> "Apps must be designed to help people with disabilities access their device or otherwise
> overcome challenges stemming from their disabilities" — Bedingung für `isAccessibilityTool="true"`

> "Voice-based input systems for motor impairments" — explizit als qualifizierende Accessibility-Tool-Kategorie genannt

> "Antivirus software, automation tools, assistants, monitoring apps, cleaners, password managers,
> and launchers" — explizit als NICHT qualifizierend gelistet

> "Any use of the Accessibility API that enables an app to autonomously initiate, plan, and execute
> actions or decisions is strictly prohibited."

> "Rule-based automation, where behavior follows a static, human-defined script (for example,
> 'If Trigger X occurs, perform Action Y')" — explizit ERLAUBT

> Seit 03.11.2021: Apps die API Level 31+ targeten, müssen eine Policy-Declaration im Play Console
> abschließen.

### Quelle 2: Android Developer Docs — AccessibilityService
URL: https://developer.android.com/guide/topics/ui/accessibility/service
Stand: 2026-04-30

> "An accessibility service is a specialized tool, not a standard way to make your app accessible."
> "Only build an accessibility service if you are creating a general-purpose assistive tool."

---

## Use-Case-Bewertung

### Erlaubter Scope (laut Policy)

- **Deterministic rule-based automation**: "If Trigger X → perform Action Y" — genau Klarvo's Modell:
  Hotkey (Trigger) → Text in fokussiertes Fenster einfügen (Action)
- **Voice-based input systems**: Diese Kategorie ist in der Policy als Qualifying-Accessibility-Tool
  gelistet — Klarvo könnte sich hier einordnen (besonders für Nutzer mit motorischen Einschränkungen)
- **Nicht-autonomes Verhalten**: Klarvo initiiert nie autonom — jede Aktion erfordert explizite
  Nutzeraktion (Hotkey-Press)

### Verbotener Scope (laut Policy)

- **Autonomous execution**: Apps die selbstständig Entscheidungen treffen und ausführen — trifft auf
  Klarvo nicht zu (keine Background-Aktionen ohne Nutzer-Trigger)
- **Automation tools / Assistants**: Explizit nicht qualifizierend — Klarvo läuft Risiko als
  "automation tool" eingestuft zu werden je nach Reviewer-Interpretation

### Klarvo-Einordnung

Klarvo sitzt zwischen zwei Policy-Kategorien:

| Perspektive | Einordnung | Implikation |
|---|---|---|
| "Voice dictation for motor impairments" | Qualifying Accessibility Tool | `isAccessibilityTool="true"` möglich, weniger Restriktionen |
| "Hotkey automation tool for power users" | Non-Qualifying (automation tool) | Disclosure-Pflicht + Play Console Declaration Required + höheres Rejection-Risiko |

**Empfehlung:** Klarvo sollte sich als "Voice-based input system" positionieren — diese Kategorie
ist policy-explizit qualifizierend. Das erfordert aber ein ehrliches Commitment: Die App muss
auch für Nutzer mit Einschränkungen sinnvoll nutzbar sein (nicht nur als Power-User-Tool vermarktet
werden).

---

## Deklarationspflichten

### In `accessibility_service_config.xml`

Pflicht-Felder für den Use-Case:

```xml
<accessibility-service
    android:accessibilityEventTypes="typeWindowStateChanged|typeViewFocused"
    android:accessibilityFeedbackType="feedbackGeneric"
    android:accessibilityFlags="flagDefault"
    android:canPerformGestures="true"
    android:description="@string/accessibility_service_description"
    android:notificationTimeout="100"
    android:settingsActivity=".SettingsActivity" />
```

Bei Claim als Accessibility Tool zusätzlich:
```xml
    android:isAccessibilityTool="true"
```

### In Play Console (seit 03.11.2021 Pflicht für API ≥ 31)

1. **Permission Declaration Form** ausfüllen: Core Functionality + Use Case erklären
2. **Video-Demo** einreichen: zeigt AccessibilityService-Nutzung in normalem App-Ablauf
3. **Review-Dauer**: "several weeks" laut Policy — Einplanen vor Phase-3-Release-Datum

### Prominent In-App Disclosure (wenn NICHT als Accessibility Tool)

Falls Klarvo `isAccessibilityTool="false"` deklariert, ist eine prominente In-App-Disclosure
Pflicht:
- Muss bei normaler App-Nutzung sichtbar sein (kein Verstecken im Menü)
- Beschreibt welche Daten via API zugegriffen werden
- Erfordert aktive Nutzer-Zustimmung (Tap/Checkbox)
- Darf NICHT nur in Privacy Policy erscheinen

---

## Alternativen-Analyse

### Option A: AccessibilityService (aktueller Plan)

**Vorteile:**
- Funktioniert transparent im Hintergrund
- Erkennt fokussiertes Fenster zuverlässig
- Kein sichtbares UI-Element nötig

**Nachteile:**
- Play Console Review erforderlich (Wochen)
- Rejection-Risiko wenn als "automation tool" eingestuft
- Strengere Policy-Überwachung ongoing

**Voraussetzungen für Approval:**
- `android:description` muss klar und ehrlich sein
- Video-Demo muss legalen Use-Case zeigen
- Positionierung als Voice-Input-Tool (nicht als Automatisierung)

### Option B: InputMethodService (IME)

**Vorteile:**
- Kein AccessibilityService, keine zugehörigen Policy-Risiken
- Play-Store-konform ohne Special Declaration
- Standard-Mechanismus für Text-Input in Android
- `commitText()` API ist der offizielle Weg für Text-Injection

**Nachteile:**
- Nutzer muss Klarvo als Eingabemethode aktiv setzen (System-Setting-Sprung)
- IME ist nur aktiv wenn ein Textfeld fokussiert ist (kein Hotkey aus beliebigem Kontext)
- Hotkey-Triggering aus dem IME-Kontext ist technisch komplexer
- Kann nicht sehen WELCHE App fokussiert ist (nur dass ein Textfeld aktiv ist)

**Fazit IME:** Technisch sauber, aber der ständige "Textfeld muss fokussiert sein"-Constraint
bricht Klarvo's Use-Case (Hotkey von überall). IME ist eine **Fallback-Option**, kein Drop-in.

### Option C: Clipboard + Notification (kein AccessibilityService)

Text in Zwischenablage + Toast/Notification "Text kopiert — jetzt einfügen":

**Vorteile:** Null Play Store Risiko
**Nachteile:** Zweistufiger Workflow, bricht UX-Kernversprechen (nahtloser Paste)

---

## Risiko-Einschätzung

### Rejection-Wahrscheinlichkeit

| Szenario | Wahrscheinlichkeit | Begründung |
|---|---|---|
| Approval als Voice-Input-Tool | ~60% | Use-Case passt in erlaubte Kategorie, deterministisch |
| Rejection als "automation tool" | ~30% | Reviewer-Ermessen, Policy-Grauzone |
| Prolonged Review (>4 Wochen) | ~50% | AccessibilityService-Submissions sind bekannt langsam |

### Suspension-Risk post-Approval

Gering, wenn:
- `accessibility_service_config.xml` genau deklariert was genutzt wird
- Keine zusätzlichen Accessibility-Events abonniert (nur Fokus-Detection)
- App tut nicht mehr als deklariert

Erhöht bei:
- Undokumentierte Event-Typen in der Config
- Hintergrund-Aktivität ohne Nutzer-Trigger
- Fehlende/unklare In-App-Disclosure

### Kritische Abhängigkeit: E1 (Windows CI) vs. F1 Timing

AccessibilityService-Review dauert Wochen. Wenn Phase-3 in ~2-3 Monaten geplant ist:
**Submission sollte spätestens 6-8 Wochen vor Phase-3-Start erfolgen** — d.h. idealer
Einreichungszeitpunkt ist **Mitte Juni 2026** wenn Phase-3 für August geplant ist.

---

## Einreichungs-Log

| Datum | Kanal | Status | Notizen |
|-------|-------|--------|---------|
| (ausstehend) | Google Play Developer Support | Nicht eingereicht | Policy-Audit abgeschlossen, Submission vorbereiten |

---

## Empfehlung für Phase-3

### Sofort-Maßnahmen (vor Phase-3-Start)

- [ ] **1. Positionierungs-Entscheidung treffen**: Klarvo als Accessibility-Tool positionieren
  (`isAccessibilityTool="true"`) oder als Non-Tool mit Disclosure? — Entscheidung durch Andy,
  beeinflusst Marketing + Store-Listing
- [ ] **2. Play Console Declaration einreichen**: Mit Video-Demo, ehrlichem Use-Case-Text,
  korrekter `accessibility_service_config.xml` — **mindestens 6 Wochen vor Phase-3-Release**
- [ ] **3. In-App Disclosure implementieren** (unabhängig von Positionierungs-Entscheidung):
  Beim ersten AccessibilityService-Enable-Prompt klar erklären was die App damit macht
- [ ] **4. `accessibility_service_config.xml` minimal halten**: Nur `typeWindowStateChanged` +
  `typeViewFocused` — keine `typeAllMask`

### Phase-3-Gate (bleibt wie gehabt)

**AccessibilityPasteBackend-Implementierung darf NICHT starten bevor:**
- [ ] Play Console Declaration **eingereicht** (nicht nur approved — früher Start erlaubt
  Code-Implementierung parallel zum Review), **UND**
- [ ] Positionierungs-Entscheidung (Option A/B/C) getroffen

### Fallback-Plan bei Rejection

Priorität 1: IME-basiertes Paste-Backend (technisch aufwendiger, aber Policy-sicher)
Priorität 2: Clipboard + Notification UX (Zero-Risk, aber degradierte UX)
Priorität 3: APK-Direct-Distribution ohne Play Store (Power-User-Pfad, kein Mainstream)

---

## Referenzen

- Google Play AccessibilityService Policy: https://support.google.com/googleplay/android-developer/answer/10964491
- Android AccessibilityService Developer Docs: https://developer.android.com/guide/topics/ui/accessibility/service
- Sensitive Permissions Declaration: https://support.google.com/googleplay/android-developer/answer/9214102
- Klarvo Memory: `memory/project_android_playstore_risk.md`, `memory/project_play_store_phase3_blocker.md`
