# Feature-Plan: Onboarding-System + Kosten-Dashboard + Quick-Tip-System

## User Story

Als neuer Dikta-Nutzer moechte ich in wenigen Minuten vom Download bis zum ersten erfolgreichen Diktat geleitet werden, damit ich sofort produktiv bin ohne Dokumentation lesen zu muessen. Als Bestandsnutzer moechte ich meine API-Kosten im Blick haben und kontextuelle Tipps erhalten, wenn ich neue Features noch nicht kenne.

---

## Betroffene Module

- **`src/Onboarding.tsx`**: Vollstaendige Ueberarbeitung -- aktuell 3 flache Steps ohne Cloud/Offline-Weiche, kein Persistenz-Mechanismus, keine Android-Permissions-Steps, kein Test-Diktat.
- **`src/App.tsx`**: Onboarding-Trigger-Logik anpassen (aktuell: `isFirstRun` = alle Keys leer). Neu: JSON-State lesen. Quick-Tip-Hook einbinden. Dashboard-Tab oeffnen.
- **`src/components/CostDashboard.tsx`**: Neue Komponente. Zeigt aggregierte Kosten, Savings vs. Wispr Flow.
- **`src/components/QuickTip.tsx`**: Neue Komponente. Toast/Snackbar am unteren Rand.
- **`src-tauri/src/config/mod.rs`**: `AppConfig` um `onboarding`-Feld erweitern (`OnboardingState`-Struct).
- **`src-tauri/src/commands/settings.rs`**: Neue Commands: `get_onboarding_state`, `set_onboarding_state`, `validate_api_key`.
- **`src-tauri/src/history/mod.rs`**: `tips_shown`-Tabelle hinzufuegen. Migration. `get_tips_shown`, `mark_tip_shown`.
- **`src-tauri/src/commands/history.rs`**: Neue Commands fuer Tips-Tabelle.
- **`src-tauri/src/stt/model_manager.rs`**: Progress-Events pruefen -- sind bereits implementiert (`dikta://model-download-progress` etc.), kein Aenderungsbedarf.
- **`android/kotlin-src/com/dikta/voice/MainActivity.kt`**: Permissions-Chain ist bereits in `checkPermissionsAndStart()` -- kein Umbau. Webview-seitig Integration pruefen.

---

## Vorbedingungen / bestehende Infrastruktur

Was bereits existiert und wiederverwendet werden kann:

- Whisper-Download mit Progress-Events: `dikta://model-download-progress`, `dikta://model-download-complete`, `dikta://model-download-error` (in `commands/whisper.rs`)
- `WhisperModelManager.tsx`: fertige Download-UI-Komponente, kann im Onboarding eingebettet werden
- `UsageSummary`-Struct in Rust (`history/mod.rs`) und TypeScript (`types.ts`) mit `totalDictations`, `totalCostUsd`, `totalSttCostUsd`, `totalLlmCostUsd`, `totalAudioSeconds` -- deckt fast alles ab
- `get_usage_stats` Tauri-Command bereits vorhanden und im Frontend verkabelt
- Onboarding-Trigger in `App.tsx` via `isFirstRun()` -- muss auf State-basierte Logik umgestellt werden
- Android-Permissions-Flow in `MainActivity.kt` laeuft nativ (5 Steps) -- Webview kann Permissions nicht kontrollieren, muss angezeigt werden wenn User zurueck in die App kommt
- `platform.ts` mit `isMobile`/`isDesktop` Guards fuer Platform-Weichen

---

## Tasks (in Reihenfolge)

### Task 1: Onboarding-State in Config + Backend-Commands

- **Agent:** rust-core
- **Dateien:**
  - `src-tauri/src/config/mod.rs` (erweiternd -- `OnboardingState`-Struct + Feld in `AppConfig`)
  - `src-tauri/src/commands/settings.rs` (erweiternd -- 3 neue Commands)
  - `src-tauri/src/commands/mod.rs` (erweiternd -- Commands registrieren)
  - `src-tauri/src/lib.rs` (erweiternd -- Commands in `invoke_handler` eintragen)
- **Abhaengigkeit:** keine
- **Beschreibung:**

Fuege `OnboardingState` zu `AppConfig` hinzu. Das Struct wird zusammen mit der restlichen Config in `config.json` gespeichert (kein separates File):

```rust
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingState {
    pub completed: bool,
    pub skipped: bool,
    pub current_step: u8,       // 0 = not started
    pub mode: String,           // "cloud" | "offline" | ""
    pub language: String,       // ISO-639-1, e.g. "de", "" = not set
}
```

Fuege `#[serde(default)] pub onboarding: OnboardingState` zu `AppConfig` hinzu.

Implementiere 3 Tauri-Commands:

1. `get_onboarding_state() -> OnboardingState` -- liest aus AppState
2. `set_onboarding_state(state: OnboardingState) -> Result<(), String>` -- persistiert in config.json, gibt AppState-Lock zurueck
3. `validate_api_key(provider: String, key: String) -> Result<bool, String>` -- macht minimalen Test-Request an Provider-API (Groq: POST /openai/v1/audio/transcriptions mit leerem Dummy-WAV oder GET /openai/v1/models; DeepSeek/OpenRouter: POST mit 1-Token Prompt). Gibt `Ok(true)` zurueck wenn HTTP 200, `Ok(false)` bei 401, `Err(msg)` bei Netzwerkfehler.

Schreibe Unit-Tests fuer `OnboardingState` Default und Serde-Roundtrip.

---

### Task 2: tips_shown-Tabelle in SQLite + Backend-Commands

- **Agent:** rust-core
- **Dateien:**
  - `src-tauri/src/history/mod.rs` (erweiternd -- neue Tabelle + 2 Funktionen)
  - `src-tauri/src/commands/history.rs` (erweiternd -- 2 neue Commands)
  - `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` (erweiternd -- registrieren)
- **Abhaengigkeit:** keine
- **Beschreibung:**

Fuege im `open_db`-Migrations-Block die neue Tabelle hinzu:

```sql
CREATE TABLE IF NOT EXISTS tips_shown (
    tip_id     TEXT PRIMARY KEY,
    shown_at   TEXT NOT NULL DEFAULT (datetime('now'))
);
```

Implementiere:
- `fn is_tip_shown(conn: &Connection, tip_id: &str) -> Result<bool, HistoryError>`
- `fn mark_tip_shown(conn: &Connection, tip_id: &str) -> Result<(), HistoryError>`

Wrap als Tauri-Commands:
- `#[tauri::command] fn is_tip_shown(state, tip_id: String) -> bool`
- `#[tauri::command] fn mark_tip_shown(state, tip_id: String) -> Result<(), String>`

Schreibe Tests fuer is_tip_shown (neu/schon gezeigt) und mark_tip_shown (Idempotenz).

---

### Task 3: Onboarding-Wizard Frontend -- Kern-Scaffold

- **Agent:** ui-dev
- **Dateien:**
  - `src/Onboarding.tsx` (Vollrewrite -- bestehende 3 Steps werden ersetzt)
  - `src/tauri-commands.ts` (erweiternd -- 3 neue Command-Bindings)
  - `src/types.ts` (erweiternd -- `OnboardingState`-Interface)
- **Abhaengigkeit:** Task 1
- **Beschreibung:**

Rewrite `Onboarding.tsx` als State-Machine. Der Wizard-Flow haengt von `mode` ("cloud"|"offline") und `isMobile` (aus `platform.ts`) ab:

**Desktop Cloud-Pfad:** Step 0 Welcome → Step 1 Mode → Step 2 Language → Step 3 STT-Key → Step 4 LLM-Key → Step 5 Test-Diktat → Step 6 Done
**Desktop Offline-Pfad:** Step 0 Welcome → Step 1 Mode → Step 2 Language → Step 3 Modell-Download → Step 4 LLM-Key (optional) → Step 5 Test-Diktat → Step 6 Done
**Android Cloud-Pfad:** Permissions-Steps (Overlay, Mikrofon, Accessibility, Batterie) werden als Info-Cards VOR Step 2 Language eingeschoben (4 zusaetzliche Steps). Die tatsaechliche Permission-Anfrage laeuft nativ in MainActivity -- der Wizard informiert nur und wartet auf Rueckkehr.
**Android Offline-Pfad:** Modell-Download entfaellt (kein lokales Whisper auf Android) -- Pfad fuehrt zu Cloud.

Persistenz: Bei jedem Step-Wechsel `set_onboarding_state({currentStep, mode, language})` aufrufen. Beim App-Start: wenn `completed=false && !skipped`: `get_onboarding_state()` lesen, auf `currentStep` springen.

Skip-Button (oben rechts, klein, disabled auf Step 0): ruft `set_onboarding_state({...state, skipped: true, completed: true})` auf, schliesst Wizard.

Die bestehende `StepDots`-Komponente und das Fade-Transition-Pattern aus dem alten Code beibehalten.

In `tauri-commands.ts` hinzufuegen:
```typescript
export async function getOnboardingState(): Promise<OnboardingState>
export async function setOnboardingState(state: OnboardingState): Promise<void>
export async function validateApiKey(provider: string, key: string): Promise<boolean>
```

---

### Task 4: Step 0 Welcome + Step 1 Cloud/Offline-Weiche

- **Agent:** ui-dev
- **Dateien:** `src/Onboarding.tsx`
- **Abhaengigkeit:** Task 3
- **Beschreibung:**

**Step 0 (Welcome):** "Sprich. Dikta tippt." als grosse H1, darunter Untertitel. Animiertes Mic-Icon mit Pulse-Ring (CSS animation, kein External-Dependency). "Loslegen"-Button rechts unten. Optional: Skip-Link als kleiner Text-Link oben rechts ("Ich kenn mich aus →").

**Step 1 (Cloud/Offline-Weiche):** Zwei gleichgrosse Karten nebeneinander (CSS grid, 2 cols).
- **Cloud-Karte:** Icon Cloud, Titel "Cloud (empfohlen)", Bullet: "Beste Qualitaet", "API-Key benoetigt", "Groq kostenlos verfuegbar". Border smaragdgruen wenn selected.
- **Offline-Karte:** Icon Shield/Lock, Titel "Offline", Bullet: "Laeuft ohne Internet", "Privacy-First", "488 MB Download einmalig". Nur auf Desktop sichtbar (`isDesktop`). Auf Android: Offline-Karte ausgegraut mit Tooltip "Nicht verfuegbar auf Android".
- Klick auf Karte setzt `mode` und ruft `set_onboarding_state` auf.
- Kleiner Hinweis unter den Karten: "Du kannst jederzeit in den Einstellungen wechseln."

---

### Task 5: Step 2 Sprachauswahl + Android Permissions-Steps

- **Agent:** ui-dev
- **Dateien:** `src/Onboarding.tsx`
- **Abhaengigkeit:** Task 3
- **Beschreibung:**

**Step 2 (Sprache):** Dropdown mit Optionen: Deutsch (de), English (en), Auto-detect. Default: aus System-Locale ermitteln (`navigator.language` splitten auf '-', Fallback "de"). Beim Aendern sofort `set_onboarding_state({...state, language})`. "Weiter"-Button.

**Android Permissions-Steps (nur wenn `isMobile`):** Vier aufeinanderfolgende Info-Steps (je ein Step-Screen), die zwischen Step 1 Mode und Step 2 Language eingeschoben werden:

1. **Overlay-Permission:** Icon Bubble, Text "Dikta braucht Overlay-Berechtigung um ueber anderen Apps zu erscheinen.", Button "Berechtigung erteilen" → oeffnet Settings (via Tauri-Invoke oder direkter Android-Intent). Automatisch weiter wenn `canDrawOverlays` true (nach Return aus Settings -- pruefen via `on_resume`-aehnlichem Event oder einfach per Button "Ich habe es erteilt").
2. **Mikrofon-Permission:** Analoges Pattern. Icon Mic. Button "Erteilen" triggert native Runtime-Permission.
3. **Accessibility-Permission:** Icon Accessibility. Erklaerung warum. Button "Zu den Einstellungen".
4. **Batterie-Optimierung:** Icon Battery. "Verhindert, dass Android Dikta im Hintergrund stoppt." Button "Ausnahme hinzufuegen".

Da Tauri/Android-Permissions nicht direkt aus dem Webview steuerbar sind: Die Buttons zeigen eine Info-Card "Geh zu Einstellungen → Apps → Dikta → Berechtigungen" und warten auf manuellen "Weiter"-Button. Kein automatisches Pruefen -- zu komplex fuer Phase 1.

---

### Task 6: Cloud-Pfad -- STT-Key + LLM-Key Steps

- **Agent:** ui-dev
- **Dateien:** `src/Onboarding.tsx`
- **Abhaengigkeit:** Task 3, Task 1 (validate_api_key)
- **Beschreibung:**

**Step 3a (STT-Key, Cloud-Pfad):**
- Hervorgehobener Groq-Block (empfohlen, free tier) wie im alten Code.
- "Magic Link" Button "Kostenlosen Key holen" oeffnet `https://console.groq.com` via `openUrl`.
- Nach Eingabe: "Validieren"-Button triggert `validateApiKey("groq", key)`. Zeige Inline-Spinner waehrend Validierung. Bei Erfolg: gruenes Checkmark + "Key funktioniert!". Bei Fehler: rotes X + Fehlermeldung.
- "Weiter"-Button aktiv wenn Key eingegeben (Validierung nicht zwingend -- User kann auch ohne Valid-Check weiter).
- Collapsible "Andere Provider" fuer OpenAI (ohne Validierung in Phase 1).

**Step 4a (LLM-Key, Cloud-Pfad):**
- Ueberschrift: "Text-Bereinigung (optional)"
- Erklaerung: "Dikta nutzt ein Sprach-Modell um rohen Transkript-Text zu bereinigen. Optional -- ohne Key wird der rohe Text eingefuegt."
- DeepSeek empfohlen (Preis-Leistung), OpenRouter als Alternative.
- Skip-Button prominent: "Ueberspringen -- rohen Text nutzen"
- Analog zu STT-Key: Inline-Validierung wenn Key eingegeben.

---

### Task 7: Offline-Pfad -- Modell-Download Step

- **Agent:** ui-dev
- **Dateien:** `src/Onboarding.tsx`
- **Abhaengigkeit:** Task 3
- **Beschreibung:**

**Step 3b (Modell-Download, Offline-Pfad, nur Desktop):**
- Zeige "Whisper small (488 MB) -- kostenlos" als einzige Option (tiny/base entfernt, medium/large sind paid).
- "Jetzt herunterladen" Button triggert `downloadWhisperModel("small")` (existing Command).
- Progress-Bar konsumiert existierende Events `dikta://model-download-progress` und `dikta://model-download-complete`.
- Verwende `WhisperModelManager.tsx` NICHT direkt (zu viel UI-Overhead fuer Onboarding) -- implementiere minimale Progress-UI inline.
- Waehrend Download laeuft: nebeneinander optionale LLM-Key-Eingabe (DeepSeek/OpenRouter). Ueberschrift "Waehrend du wartest: Text-Bereinigung einrichten (optional)".
- Wenn Download fertig: Gruenes Checkmark, "Weiter"-Button aktiv.
- Wenn bereits heruntergeladen: direkt "Weiter" ohne Download.

---

### Task 8: Step Test-Diktat (Aha-Moment)

- **Agent:** ui-dev
- **Dateien:** `src/Onboarding.tsx`
- **Abhaengigkeit:** Task 3
- **Beschreibung:**

**Step 5 (Test-Diktat):**
- Ueberschrift: "Probiere es aus!"
- Grosser Record-Button (identisch zu `RecordButton` aus `App.tsx` -- als separate importierbare Komponente auslagern oder direkt rendern).
- Status-Text zeigt Pipeline-State (`idle` → `recording` → `transcribing` → `cleaning` → `done`).
- Event-Listener auf `dikta://state-changed` (wiederverwendbar via `useRecording`-Hook oder direktes Listen auf Tauri-Events).
- Wenn `done`-State: Zeige Ergebnis-Text in editierbarem Textfeld. Darunter: "Super! Dein Text wird in jedes Textfeld eingefuegt."
- "Weiter"-Button aktiv sobald mindestens einmal `done` erreicht (oder Skip "Spaeter ausprobieren").
- Hinweis auf Desktop: "Druecke Strg+Shift+D um zu diktieren" (platfformspezifisch via `isDesktop`).
- Hinweis auf Android: "Tippe auf die schwebende Blase um zu diktieren".

---

### Task 9: Step Done + Zusammenfassung + Re-Run in Settings

- **Agent:** ui-dev
- **Dateien:**
  - `src/Onboarding.tsx`
  - `src/App.tsx` (erweiternd -- Onboarding-Trigger-Logik)
  - `src/components/SettingsPanel.tsx` (erweiternd -- "Setup-Assistent starten" Button)
- **Abhaengigkeit:** Task 3
- **Beschreibung:**

**Step 6 (Fertig):**
- Gross: Checkmark-Animation (CSS, kein SVG-Library). "Du bist startklar!"
- Zusammenfassung als 2-3 Zeilen: Modus (Cloud mit Groq / Offline mit Whisper small), Sprache, LLM-Cleanup (aktiv/inaktiv).
- "Dikta starten"-Button: ruft `set_onboarding_state({...state, completed: true})` auf, schliesst Wizard, App ladet Settings.

**Onboarding-Trigger-Logik in `App.tsx`:**
- Ersetze `isFirstRun()`-Check durch `getOnboardingState()`. Zeige Wizard wenn `!completed && !skipped`.
- `isFirstRun()` Command kann erhalten bleiben als Legacy-Fallback fuer Nutzer die schon einen Key haben aber noch kein `onboarding.completed = true` in config -- in diesem Fall Wizard ueberspringen (Key vorhanden = schon eingerichtet).

**Re-Run in Settings:**
- In `SettingsPanel.tsx` unter "Allgemein" neuer Eintrag: "Setup-Assistent erneut starten" (kleiner Link-Button). Klick: `set_onboarding_state({completed: false, skipped: false, currentStep: 0, mode: '', language: ''})`, dann `setShowOnboarding(true)` in `App.tsx` via State.

---

### Task 10: Kosten-Dashboard Komponente

- **Agent:** ui-dev
- **Dateien:**
  - `src/components/CostDashboard.tsx` (neu)
  - `src/App.tsx` (erweiternd -- Dashboard-Tab oder Integration in Stats-Panel)
  - `src/tauri-commands.ts` (pruefen ob `getUsageStats()` alle benoetigen Felder liefert -- ja, tut es)
- **Abhaengigkeit:** keine (UsageSummary-Daten existieren bereits)
- **Beschreibung:**

Erstelle `CostDashboard.tsx` als eigenstaendige Komponente die `UsageSummary` als Prop erhaelt:

**Anzeige:**
- Statistik-Kacheln (wiederverwendbare `StatCard` aus `ui.tsx`):
  - "Diktate gesamt" (totalDictations)
  - "Gesprochene Zeit" (totalAudioSeconds formatiert als "Xm Ys")
  - "Woerter gesamt" (totalWords)
- Kosten-Sektion (Kacheln):
  - "STT-Kosten" (totalSttCostUsd, formatCost())
  - "LLM-Kosten" (totalLlmCostUsd)
  - "Gesamt" (totalCostUsd)
- Savings-Banner (smaragdgruener Hintergrund):
  - "Du sparst gegenueber Wispr Flow: $X.XX/Monat"
  - Berechnung: `wispr_monthly = 12.00`, `monthly_estimate = totalCostUsd / totalMonths` (wo totalMonths aus erster Diktat-Datum berechnet, Fallback 1). `savings = max(0, wispr_monthly - monthly_estimate)`.
  - Wenn savings <= 0: "Kein Vergleich moeglich (noch zu wenig Daten)"
- Fusszeile: "Kostenbasiert auf Provider-Preisen. Letzte Aktualisierung: [Datum]"

Provider-Kostentabelle (in `src/cost-rates.ts` als Konstante, nicht im Backend):
```typescript
// Versionsdatum: 2026-03-19
export const COST_RATES = {
  groq_stt: { per_minute: 0.0 }, // Groq STT ist aktuell kostenlos
  openai_stt: { per_minute: 0.006 },
  deepseek_llm: { per_1k_input: 0.00014, per_1k_output: 0.00028 },
  groq_llm: { per_1k_input: 0.0001, per_1k_output: 0.0001 },
  openrouter_llm: { per_1k_input: 0.0001, per_1k_output: 0.0001 }, // Mittelwert
};
```

Hinweis: Kosten werden bereits im Backend berechnet und in `usage`-Tabelle gespeichert (`estimated_cost_usd`). Die Frontend-Kostentabelle dient nur dem Display/Erklaerung, nicht der Neuberechnung.

Integration in `App.tsx`: Dashboard als neuer Sub-Tab in der bestehenden Stats-Ansicht (unter den Filler-Stats), oder als eigener Tab-Button in der Header-Leiste. Entscheide nach bestehendem Tab-Pattern -- bevorzuge Sub-Tab um keine neuen Icons benoetigen.

---

### Task 11: Quick-Tip-System

- **Agent:** ui-dev
- **Dateien:**
  - `src/components/QuickTip.tsx` (neu)
  - `src/hooks/useQuickTip.ts` (neu)
  - `src/App.tsx` (erweiternd -- Hook einbinden, Tip-Toast rendern)
  - `src/tauri-commands.ts` (erweiternd -- `isTipShown`, `markTipShown`)
- **Abhaengigkeit:** Task 2
- **Beschreibung:**

**Tip-Definitionen** (in `useQuickTip.ts` als Konstante):
```typescript
const TIPS = [
  { id: "cleanup-styles",    trigger: { dictations: 5  }, title: "Bereinigungsstile", text: "Probiere 'Chat' fuer lockere Nachrichten oder 'Verbatim' fuer unveraenderten Text.", action: { label: "Ausprobieren", panel: "settings" } },
  { id: "cleanup-instr",     trigger: { dictations: 10 }, title: "Eigene Anweisungen", text: "Unter Einstellungen kannst du dem KI-Modell eigene Stilanweisungen geben.", action: { label: "Einstellungen", panel: "settings" } },
  { id: "hotkey-change",     trigger: { dictations: 20 }, title: "Hotkey anpassen", text: "Du kannst den Diktat-Shortcut in den Einstellungen aendern.", action: { label: "Jetzt aendern", panel: "settings" } },
  { id: "cost-dashboard",    trigger: { days: 7        }, title: "Deine Kosten", text: "Schau dir an, wie viel du gegenueber Wispr Flow sparst.", action: { label: "Dashboard zeigen", panel: "stats" } },
  { id: "offline-mode",      trigger: { dictations: 50 }, title: "Offline-Modus", text: "Dikta kann auch ohne Internet funktionieren -- mit einem lokalen Whisper-Modell.", action: { label: "Einrichten", panel: "settings" } },
];
```

**`useQuickTip`-Hook:**
- Beim App-Start: `getUsageStats()` und `isTipShown(tipId)` fuer jeden Tip pruefen.
- Ersten passenden Tip der Sitzung ermitteln (Bedingung erfuellt + noch nicht gezeigt).
- Nach 3 Sekunden Delay anzeigen (nicht sofort nach App-Start).
- Max 1 Tip pro Session (State: `tipShownThisSession: boolean`).
- Gibt `{activeTip, dismissTip}` zurueck.

**`QuickTip`-Komponente:**
- Toast am unteren Rand (`fixed bottom-20 left-4 right-4 md:left-auto md:right-6 md:w-80`).
- Karte mit: Icon (bezogen auf Tip-Typ), Titel (bold), Text, Action-Button (emerald), Dismiss-X oben rechts.
- Enter-Animation: slide-up von unten (Tailwind `translate-y-4 → translate-y-0` + opacity).
- Dismiss: ruft `markTipShown(tipId)` auf, entfernt Karte.
- Action-Button: schliesst Tip und oeffnet angegeben Panel (callback nach oben via `onOpenPanel`).

---

### Task 12: Android-Integration pruefen + Smoke-Tests

- **Agent:** android-platform
- **Dateien:**
  - `android/kotlin-src/com/dikta/voice/MainActivity.kt` (lesend + ggf. kleiner Patch)
- **Abhaengigkeit:** Task 3-8
- **Beschreibung:**

Pruefen ob der neue Onboarding-Wizard auf Android korrekt gerendert wird:

1. **Safe-Area:** Onboarding-Vollbild nutzt `h-screen` -- pruefen ob Android-WebView unten abschneidet. Falls ja: `pb-[env(safe-area-inset-bottom,56px)]` oder fixes `pb-14` hinzufuegen.
2. **Permissions-Steps:** Die Android-Permission-Info-Cards sollen native Permissions nicht selbst triggern -- pruefen ob `isMobile`-Guard korrekt funktioniert.
3. **Test-Diktat Step:** `useRecording`-Hook auf Android pruefen. Android-Diktat laeuft ueber Kotlin-Overlay, nicht ueber den Tauri-Frontend-Record-Button. Deshalb Test-Diktat Step auf Android durch einfacheres "Tippe auf die Blase"-Hinweis-Screen ersetzen statt echten Record-Button (kann leicht verwirren wenn kein Pipeline-Event zurueckkommt).
4. **Offline-Step:** Sicherstellen dass Offline-Karte auf Android korrekt ausgegraut ist.

Keine groesseren Kotlin-Aenderungen erwartet -- Permissions-Flow laeuft bereits nativ in `MainActivity.kt`.

---

## Testplan

- [ ] **Onboarding Erstinstallation Desktop Cloud:** App starten ohne config.json -- Wizard erscheint auf Step 0. Durch alle Steps navigieren, Groq-Key eingeben, validieren. Test-Diktat ausfuehren. Done-Screen. App startet normal.
- [ ] **Onboarding Erstinstallation Desktop Offline:** Mode "Offline" waehlen, Modell-Download starten, Progress-Bar erscheint, Download completes, Wizard weiter.
- [ ] **Persistenz bei Abbruch:** Wizard auf Step 3 verlassen (App schliessen), neu starten -- Wizard oeffnet auf Step 3.
- [ ] **Skip fuer Power-User:** Skip-Button auf Step 1 klicken -- Wizard schliesst, App startet, Settings korrekt.
- [ ] **Re-Run aus Settings:** "Setup-Assistent starten" in Settings klicken -- Wizard oeffnet auf Step 0.
- [ ] **Bestehende Nutzer (Migration):** Config mit gesetztem Groq-Key aber ohne `onboarding`-Feld laden -- `isFirstRun()`-Fallback greift, Wizard wird NICHT gezeigt (Key vorhanden = eingerichtet).
- [ ] **API-Validierung:** Gueltigen Groq-Key eingeben → gruenes Checkmark. Ungueltigen Key → rotes X + Meldung. Offline (kein Netz) → Fehlermeldung "Netzwerkfehler".
- [ ] **Kosten-Dashboard:** Nach 5+ Diktaten: Dashboard oeffnen, alle Kacheln zeigen sinnvolle Werte. Savings-Berechnung korrekt.
- [ ] **Quick-Tip nach 5 Diktaten:** 5 Diktate ausfuehren, App neu starten, Tip erscheint nach 3s. Dismiss klicken. Erneut starten -- Tip erscheint nicht mehr.
- [ ] **Quick-Tip max 1 pro Session:** Bedingungen fuer 2 Tips erfuellt, nur 1 wird pro Session gezeigt.
- [ ] **Android Onboarding:** Permissions-Info-Steps erscheinen, Offline-Karte ausgegraut, Test-Diktat Step zeigt Hinweis-Text statt Record-Button.
- [ ] **SQLite ueberlept Update:** Usage-Tabelle und tips_shown-Tabelle sind nach Fake-Update (Daten loeschen, Migration erneut laufen) noch vorhanden.

---

## Risiken

| Risiko | Wahrscheinlichkeit | Umgang |
|--------|-------------------|--------|
| `validateApiKey` fuer Groq erfordert gueltiges Audio-File (kann kein leeres WAV senden) | Mittel | Stattdessen `GET /openai/v1/models` anfragen (Groq unterstuetzt diesen Endpoint); bei 401 = ungueltig |
| Android Test-Diktat Step: Record-Button triggert Tauri-Pipeline, aber Android-Nutzer diktieren ueber Kotlin-Overlay | Hoch | Step auf Android durch Info-Screen ersetzen (kein Record-Button) |
| Onboarding-Persistenz bei App-Crash auf Step 3: halber State gespeichert | Niedrig | `set_onboarding_state` nach jedem Step-Wechsel -- worst case: Step-Nummer stimmt nicht, User tippt "Zurueck" |
| `navigator.language` liefert auf Android WebView unerwartete Locale | Niedrig | Fallback auf "de" wenn Locale nicht in erlaubter Liste |
| Quick-Tip erscheint waehrend Diktat (schlecht) | Mittel | Tip-Anzeige nur wenn `recordingState === "idle"`. Im Hook pruefen. |
| Tips-Shown-Tabelle fehlt bei alten Nutzern (Migration) | Niedrig | `CREATE TABLE IF NOT EXISTS` in `open_db` -- idempotent, kein Problem |
