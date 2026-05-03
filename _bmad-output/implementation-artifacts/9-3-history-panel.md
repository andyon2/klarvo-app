---
name: Story 9.3 — History Panel
epic: 9
story_number: "9.3"
status: done
dependencies:
  - 9-2-history-backend
---

# Story 9.3: History Panel

Status: review

## Story

Als täglicher Klarvo-User
möchte ich im Settings-Window eine History-Tab sehen, die meine letzten Diktate auflistet und mir erlaubt, einzelne Einträge oder den gesamten Verlauf zu löschen,
damit ich frühere Diktat-Ergebnisse nachschlagen und die lokale History pflegen kann.

## Kontext und Motivation

**Problem:** Story 9.2 hat das History-Backend (SQLite + Tauri-Commands) implementiert. Die Commands `get_history`, `delete_history_entry`, `clear_history` sind registriert, aber noch nicht im Frontend konsumierbar: weder ist das TypeScript-Bindings-File regeneriert, noch existiert eine UI-Surface.

**Scope:** Reine Frontend-Story. Keine Rust-Änderungen. Alles in `shells/windows/src/index.html` (React 19 ESM CDN, kein Build-Step, Phase-2-A-Minimal-Panel). Zusätzlich: Bindings-Regen + 7 neue i18n-Keys.

**Architektur-Entscheidung:** Tab-basiertes State-Routing (architecture.md:290) — kein React Router, State-Variable `activeTab: 'settings' | 'history'`. Kein Feature-Slice-Split (der kommt mit Phase-2-B Vite-Migration). Alles inline im Single-HTML-File, analog zu SettingsPanel.

**Scope-Grenze:** Nur Read + Delete + Clear All. Keine Suche, keine Paginierung (100 Einträge reichen Phase-1), kein Edit, kein Export-UI (Epic 9.5 macht Log-Export). `app_name`/`raw_text` sind Phase-1 `None` → nicht anzeigen.

## Acceptance Criteria

### AC-1: Bindings regeneriert — History-Commands in `bindings/index.ts`

**Given** `shells/windows/src/bindings/index.ts` enthält noch NICHT `getHistory`, `deleteHistoryEntry`, `clearHistory` und den Typ `HistoryEntryDto`,
**When** `cargo xtask generate-bindings` ausgeführt wird,
**Then** ist `shells/windows/src/bindings/index.ts` regeneriert und enthält:

```typescript
// Commands (camelCase, typedError-wrapped):
commands.getHistory: (limit?: number) => Promise<{status:"ok"; data:HistoryEntryDto[]} | {status:"error"; error:AppError}>
commands.deleteHistoryEntry: (id: number) => Promise<...>
commands.clearHistory: () => Promise<...>

// Type:
export type HistoryEntryDto = {
  id: number,
  text: string,
  style: string,
  language: string,
  createdAt: string,    // ISO-8601 UTC — serde rename_all = "camelCase" von created_at
  pluginId: string | null,
  outputLanguage: string | null,
}
```

`cargo xtask bindings-drift` läuft danach clean (kein diff).

**Hinweis zu `reload_locale`:** `reload_locale` ist in `lib.rs collect_commands!` registriert aber noch nicht im committed `bindings/index.ts` — Bindings-Regen fügt es ebenfalls hinzu. Das ist erwünschtes Verhalten, kein Bug.

### AC-2: Tab-Routing — Settings-Tab und History-Tab

**Given** `shells/windows/src/index.html` hat nur eine `SettingsPanel`-Komponente und kein Tab-Routing,
**When** Story 9.3 committed ist,
**Then**:

1. Der Root-Render (`createRoot(...).render(...)`) rendert nicht mehr direkt `SettingsPanel`, sondern `App`.
2. `App`-Komponente hält `const [activeTab, setActiveTab] = useState('settings')` als React State.
3. Eine Tab-Bar wird gerendert mit zwei Tabs: "Settings" und "History". Aktiver Tab ist optisch hervorgehoben (border-bottom oder background-difference mit `#2ac3a8` als Active-Color).
4. Bei `activeTab === 'settings'`: `SettingsPanel` wird gerendert (unverändert).
5. Bei `activeTab === 'history'`: `HistoryPanel` wird gerendert (AC-3).
6. Default beim App-Start: `'settings'` (Settings-Tab aktiv).
7. `h1`-Titel "Klarvo Settings" wird entfernt — Tab-Bar ersetzt die Funktion (kein redundantes Heading über dem Tab-Inhalt).

**CSS-Anforderung (inline im `<style>`-Block):**
```css
.tabs { display: flex; gap: 0; margin-bottom: 20px; border-bottom: 1px solid #2a3040; }
.tab-btn {
  padding: 8px 16px; background: none; border: none; border-bottom: 2px solid transparent;
  color: #8090a0; font-size: 14px; font-weight: 500; cursor: pointer;
  margin-bottom: -1px; transition: color 0.15s, border-color 0.15s;
}
.tab-btn:hover { color: #e0e4ef; }
.tab-btn.active { color: #2ac3a8; border-bottom-color: #2ac3a8; }
```

### AC-3: `HistoryPanel` Komponente — Load on Mount

**Given** `HistoryPanel` im gleichen `<script type="module">`-Block wie `SettingsPanel` definiert ist,
**When** der History-Tab aktiviert wird (= `HistoryPanel` mounted),
**Then**:

1. `HistoryPanel` hält den State:
   ```js
   const [loadState, setLoadState] = useState({ status: 'idle' });
   // status: 'idle' | 'loading' | 'success' | 'error'
   // Bei success: { status: 'success', entries: [...] }
   // Bei error: { status: 'error', error: AppError }
   ```
2. `useEffect(loadHistory, [])` feuert einmal beim Mounten.
3. `loadHistory()` ruft `invoke("get_history", { limit: 100 })` (via raw `tauriCore.invoke`, analog zu SettingsPanel-Pattern) und setzt State entsprechend.
4. Während Load: Spinner + "Loading history…" (analog zu SettingsPanel Loading-Row).
5. Bei Error: Toast-Komponente zeigt `error.userMessage ?? "error.unknown"` (analog zu `errorToToast`-Pattern aus SettingsPanel).
6. Bei Success: Liste der Entries oder Empty State (AC-4/AC-7).

**4-State-Typ** (architecture.md:812): `idle / loading / success / error` — KEIN bool-Flag.

### AC-4: Entry-List-Rendering

**Given** `loadState.status === 'success'` und `loadState.entries.length > 0`,
**When** der History-Tab sichtbar ist,
**Then** wird jeder `HistoryEntryDto` als Zeile gerendert:

- **Timestamp:** `entry.createdAt` formatiert via `new Date(entry.createdAt).toLocaleString()` (lokale Zeitzone, kein custom Format nötig). Darstellung: grau (`#8090a0`), klein (`font-size: 12px`).
- **Text-Preview:** `entry.text` — wenn länger als 120 Zeichen: auf 120 Zeichen truncieren + `"…"` anhängen. Darstellung: weiß (`#e0e4ef`), `font-size: 13px`.
- **Style-Badge:** `entry.style` — kleines Label hinter dem Timestamp (z.B. `"verbatim"`, `"groq-cleanup"`). `font-size: 11px`, `color: #2ac3a8` (brand). Wenn `style === "verbatim"` → Badge weglassen (Default, kein visuelles Rauschen).
- **Delete-Button:** Pro Entry ein Button mit Text "Delete" (AC-8 für i18n-Key). `class="btn-danger"` (neue CSS-Class). Beim Click: `deleteEntry(entry.id)` (AC-5).

**Neue CSS-Class:**
```css
.entry-row {
  display: flex; align-items: flex-start; gap: 10px;
  padding: 10px 0; border-bottom: 1px solid #1a1e28;
}
.entry-meta { font-size: 12px; color: #8090a0; flex-shrink: 0; width: 130px; }
.entry-text { flex: 1; font-size: 13px; color: #e0e4ef; word-break: break-word; }
.btn-danger {
  padding: 4px 10px; border-radius: 6px; border: 1px solid #3a1e1e;
  background: #2e1414; color: #ff7369; font-size: 12px; cursor: pointer;
  flex-shrink: 0; transition: opacity 0.15s;
}
.btn-danger:hover:not(:disabled) { opacity: 0.75; }
.btn-danger:disabled { opacity: 0.5; cursor: default; }
```

Einträge in Render-Reihenfolge wie vom Backend geliefert (neueste zuerst — `ORDER BY id DESC` in `SqliteHistoryStore.list()`).

### AC-5: Entry löschen

**Given** User klickt "Delete" auf einem History-Entry,
**When** `delete_history_entry({ id })` returned `{ status: "ok" }`,
**Then**:
- Entry wird aus dem lokalen `loadState.entries`-Array entfernt (optimistic-removal nach Erfolg, NICHT vor dem API-Call).
- Kein Reload-from-Backend (lokale State-Mutation reicht).
- Der Delete-Button des betreffenden Entries ist disabled während der Call läuft (per-entry loading-flag).

**When** `delete_history_entry` returned `{ status: "error" }`,
**Then**:
- Toast-Anzeige mit `error.userMessage ?? "error.unknown"` (identisch zu SettingsPanel `errorToToast`-Pattern).
- Entry bleibt in der Liste.

**Per-entry Disable-Pattern:**
```js
const [deletingId, setDeletingId] = useState(null);
// Beim Click: setDeletingId(id), nach Completion: setDeletingId(null)
// Render: disabled={deletingId === entry.id}
```

### AC-6: Clear All

**Given** `loadState.status === 'success'` und `loadState.entries.length > 0`,
**When** User auf "Clear All"-Button klickt,
**Then**:
1. `window.confirm("Clear all dictation history? This cannot be undone.")` wird aufgerufen.
2. Wenn User abbricht: nichts passiert.
3. Wenn User bestätigt: `clear_history()` wird aufgerufen.
4. Bei Erfolg: `loadState.entries` wird auf `[]` gesetzt (keine Reload-from-Backend).
5. Bei Error: Toast analog zu AC-5.

"Clear All"-Button ist disabled während `clearHistory`-Call läuft (`const [clearing, setClearing] = useState(false)`).

"Clear All"-Button wird NOT angezeigt wenn `loadState.entries.length === 0` (kein leerer Clearing-Button).

### AC-7: Empty State

**Given** `loadState.status === 'success'` und `loadState.entries.length === 0`,
**When** der History-Tab sichtbar ist,
**Then** wird ein Empty-State-Block gerendert:

```html
<!-- Rendered via createElement: -->
<div class="empty-state">
  <p>No dictations yet.</p>
  <p class="empty-hint">Start dictating with your hotkey — entries appear here.</p>
</div>
```

```css
.empty-state { text-align: center; padding: 40px 20px; color: #8090a0; }
.empty-state p { font-size: 14px; margin-bottom: 8px; }
.empty-hint { font-size: 12px; opacity: 0.7; }
```

### AC-8: i18n-Keys

**Given** die neuen i18n-Schlüssel für die History-Panel-UI noch nicht in `en.json`/`de.json` existieren,
**When** Story 9.3 committed ist,
**Then** sind folgende Keys in beiden Locale-Files vorhanden:

**en.json additions:**
```json
"settings.tab.label": "Settings",
"history.tab.label": "History",
"history.empty_state.title": "No dictations yet",
"history.empty_state.hint": "Start dictating with your hotkey — entries appear here.",
"history.entry.delete_label": "Delete",
"history.clear_all.label": "Clear All",
"history.clear_all.confirm": "Clear all dictation history? This cannot be undone."
```

**de.json additions:**
```json
"settings.tab.label": "Einstellungen",
"history.tab.label": "Verlauf",
"history.empty_state.title": "Noch keine Diktate",
"history.empty_state.hint": "Starten Sie eine Diktat-Session mit Ihrem Hotkey — Einträge erscheinen hier.",
"history.entry.delete_label": "Löschen",
"history.clear_all.label": "Alles löschen",
"history.clear_all.confirm": "Gesamten Diktatverlauf löschen? Dies kann nicht rückgängig gemacht werden."
```

**Hinweis:** Die aktuelle `index.html` verwendet i18n-Keys noch nicht für UI-Strings (Phase-2-B-Migration bringt vollen i18n-Resolver, architecture.md Kommentar: "Translation lands with the toolchain rebuild"). Die Keys sind für die Zukunft vorbereitet; die tatsächlichen UI-Strings in `index.html` sind hardcoded-Englisch — konsistent mit dem bestehenden SettingsPanel-Pattern (`"Save Settings"`, `"Loading…"` sind ebenfalls hardcoded).

### AC-9: Bestehende SettingsPanel-Funktionalität unverändert

**Given** `SettingsPanel` bisher direkt gemountet war,
**When** Story 9.3 `App`-Wrapper einführt,
**Then** ist die `SettingsPanel`-Komponente **inhaltlich unverändert** (kein Refactoring ihrer Internals, kein Entfernen von Props, kein Umbau der Form-Fields oder Event-Handler). Sie wird nur in `App` ein-gewrapped statt direkt gemountet.

**Regressions-Check:** `app.error`-Listener, `settings.changed`-Listener, `reload_locale`-Invoke, Save-Button — alle weiterhin funktional.

### AC-10: Bindings-Drift-Gate grün

**Given** alle Änderungen committed sind,
**When** `cargo xtask bindings-drift` ausgeführt wird,
**Then** exitiert der Prozess mit Code 0 (kein Diff zwischen generiertem und committed `bindings/index.ts`).

**Wichtig:** Story 9.3 macht KEINE Rust-Änderungen. `cargo check --target x86_64-pc-windows-gnu` muss zwar nicht explizit laufen (nur Shell-JS-Änderungen), aber `bindings-drift` ist der relevante Gate hier.

## Tasks / Subtasks

- [x] **AC-1: Bindings regenerieren** (AC-1, AC-10)
  - [x] `cargo xtask generate-bindings` ausführen
  - [x] `bindings/index.ts` committen (enthält nun `getHistory`, `deleteHistoryEntry`, `clearHistory`, `HistoryEntryDto`, und auch `reloadLocale` als Bonus-Fix des bestehenden Drifts)
  - [x] `cargo xtask bindings-drift` prüfen → clean

- [x] **AC-2: Tab-Routing** (AC-2)
  - [x] CSS-Classes `.tabs`, `.tab-btn`, `.tab-btn.active` zum `<style>`-Block hinzufügen
  - [x] `App`-Komponente mit `activeTab`-State + Tab-Bar implementieren
  - [x] `createRoot(...).render(h(App, null))` statt direktem SettingsPanel-Render

- [x] **AC-3+4+5+6+7: HistoryPanel-Komponente** (AC-3, AC-4, AC-5, AC-6, AC-7)
  - [x] `HistoryPanel`-Funktion im `<script type="module">`-Block definieren (nach SettingsPanel-Definition)
  - [x] `loadState`-State (4-state) + `useEffect(loadHistory, [])` mit `invoke("get_history", { limit: 100 })`
  - [x] Loading-Spinner-Row (aus SettingsPanel wiederverwenden: `loading-row` + `spinner`)
  - [x] Error-Toast via `errorToToast`-Funktion (aus SettingsPanel wiederverwenden)
  - [x] Entry-List mit `.entry-row`, `.entry-meta`, `.entry-text` CSS
  - [x] Per-entry Delete-Button (`deletingId`-State, `btn-danger`-Class)
  - [x] Clear-All-Button mit `window.confirm()` Guard + `clearing`-State
  - [x] Empty State mit `.empty-state`, `.empty-hint`-CSS

- [x] **AC-8: i18n-Keys** (AC-8)
  - [x] 7 neue Keys in `shells/windows/locales/en.json`
  - [x] 7 neue Keys in `shells/windows/locales/de.json`

- [x] **AC-9: Regression-Check** (AC-9)
  - [x] SettingsPanel inhaltlich unverändert (diff-verify: nur `h1` + `createRoot`-Zeile entfernt)
  - [x] `app.error`-Listener, `settings.changed`-Listener, Save-Flow strukturell unverändert

### Review Findings

- [x] [Review][Patch] **P1 (CRITICAL): `invoke()` result-shape mismatch — HistoryPanel non-functional** [`shells/windows/src/index.html:366-378, 387-396, 409-415`] — Der lokale `invoke()`-Wrapper (Zeile 87–90) gibt das Raw-Tauri-Result zurück (Resolve = direkter Daten-Wert, Reject = AppError-throw). Das bestehende SettingsPanel-Pattern nutzt `.then((s) => s.field).catch((e) => setToast(errorToToast(e)))`. HistoryPanel prüft aber `result && result.status === "ok"` — das Feld `status` existiert nie, also greift IMMER der Error-Branch: jedes erfolgreiche `get_history` zeigt einen Error-Toast statt der Liste; jeder erfolgreiche `delete_history_entry` und `clear_history` triggert den Error-Toast statt das State-Update. **Fixed:** `loadHistory`/`deleteEntry`/`clearHistory` auf `.then((entries) => …).catch((e) => …)` umgeschrieben analog zu SettingsPanel `get_user_settings`.
- [x] [Review][Patch] **P2 (MAJOR): Async-Cancellation fehlt — setState auf unmounted HistoryPanel** [`shells/windows/src/index.html:362-381, 383-402, 404-422`] — Tab-Switch von History → Settings unmounted die HistoryPanel-Komponente; jede in-flight `loadHistory`/`deleteEntry`/`handleClearAll`-Promise schreibt nach Resolve in unmounted State (React-Warning + Memory-Leak). **Fixed:** `mountedRef = useRef(true)` mit Cleanup-Effect; alle setState-Aufrufe in async-Pfaden gegen `mountedRef.current` geguarded.
- [x] [Review][Patch] **P3 (MINOR): Render-Branch error+toast strukturell convoluted** [`shells/windows/src/index.html:268-279`] — Ternary-Kette + trailing-OR mit defensivem `loadState.status !== "error"`-Guard rendert Toast in zwei verschiedenen Code-Pfaden. **Fixed:** dedizierter Toast-Slot oben in der Komponente, unabhängig vom `loadState.status`; loadState-Branching enthält nur noch loading + success.
- [x] [Review][Patch] **P4 (MINOR): Empty-State Punkt am Satzende abweichend von Spec/i18n-Key** [`shells/windows/src/index.html:228-232`] — Render-String war `"No dictations yet."` (mit Punkt). Spec AC-7 + i18n-Key `history.empty_state.title` definieren `"No dictations yet"` (ohne Punkt). **Fixed:** Punkt entfernt.
- [x] [Review][Patch] **P5 (NIT): Funktion `handleClearAll` vs. spec-prescribed `clearHistory`** [`shells/windows/src/index.html:404`] — architecture.md:754 mandatiert verb-first-Action-Naming. **Fixed:** `handleClearAll` → `clearHistory` (Funktionsdefinition + onClick-Referenz).
- [x] [Review][Defer] **F25: Tabs ohne ARIA-Tablist-Semantik** [`shells/windows/src/index.html:288-298`] — `<button>` mit className-Toggle, kein `role="tablist"/"tab"`, kein `aria-selected`, keine Arrow-Key-Navigation. Spec adressiert Accessibility nicht; Phase-1-Polish — deferred.
- [x] [Review][Defer] **F26: Kein Retry-Button im Error-State** [`shells/windows/src/index.html:267-273`] — Wenn `loadHistory` fehlschlägt, ist der Error-State terminal bis Tab-Switch. UX-Polish — spec-out-of-scope.
- [x] [Review][Defer] **F27: `toLocaleString()` ignoriert ui.language-Setting** [`shells/windows/src/index.html:247`] — Date-Format folgt Browser-Default-Locale, nicht der UI-Sprachwahl. Phase-1 ohne i18n-Resolver akzeptabel — deferred bis Phase-2-B.
- [x] [Review][Defer] **F28: `.btn-danger` ohne visuelle Hierarchie für „Clear All" vs. Per-Row-Delete** [`shells/windows/src/index.html` CSS-Block + Render] — Bulk-Destruktiv und Per-Row-Destruktiv haben identisches Styling und stehen kanten-nah → Misclick-Risiko. UX-Polish — deferred.
- [x] [Review][Defer] **F29: UTF-16-Slice schneidet Surrogate Pairs (Emojis)** [`shells/windows/src/index.html:243-245`] — `text.slice(0, 120)` operiert auf Code-Units; Emoji am 120er-Index ergibt Replacement-Char. Spec ohne Grapheme-Mandat — deferred.
- [x] [Review][Defer] **F30: Kein Live-Update bei neuem Diktat (`history.appended`-Listener)** — Während Panel offen ist und User diktiert, taucht der neue Eintrag erst nach Tab-Switch auf. Spec ohne Live-Update-Mandat — Phase-2-Backlog.

## Dev Notes

### Die wichtigste Tatsache: Single HTML File, kein Build-Step

`shells/windows/src/index.html` ist die **einzige Datei** für das Frontend-UI. Es gibt keine `src/features/`, keine `src/components/`, keine TypeScript-Kompilierung für dieses File. Die Architektur plant einen Wechsel zu Vite+React in Phase-2-B — bis dahin: alles inline, `createElement as h`, ESM CDN.

Kein Versuch, jetzt eine Features-Ordnerstruktur oder Datei-Splits einzuführen. Das ist Premature-Abstraction (memory `feedback_premature_abstraction_guard`). Einfach alles im `<script type="module">`-Block.

### Pattern-Vorlagen aus SettingsPanel

| Pattern | Vorlage in `index.html` | Ziel in Story 9.3 |
|---------|------------------------|-------------------|
| `invoke(cmd, args)` | Zeile 64–67 | identisch nutzen: `invoke("get_history", { limit: 100 })` |
| `errorToToast(e)` | Zeile 75–80 | identisch nutzen für History-Errors |
| `loading-row` + `spinner` | Zeile 292–296 | identisch in HistoryPanel |
| `toast` State | Zeile 123 | eigener `toast`-State in HistoryPanel |
| Cancellation-Flag-Pattern | Zeile 139–166 | für History-Listener falls nötig (wahrscheinlich nicht) |
| `useState`, `useEffect`, `useCallback` | imports Zeile 54 | bereits importiert, einfach nutzen |

### `HistoryEntryDto` Feldnamen (nach serde camelCase)

Rust-Feldname → JS-Feldname nach `#[serde(rename_all = "camelCase")]`:
- `id` → `id` (number/i64)
- `text` → `text` (string)
- `style` → `style` (string — z.B. `"verbatim"`, `"groq-cleanup"`)
- `language` → `language` (string — Phase-1 immer `""`)
- `created_at` → `createdAt` (string ISO-8601 UTC)
- `plugin_id` → `pluginId` (string | null)
- `output_language` → `outputLanguage` (string | null)

`new Date(entry.createdAt).toLocaleString()` liefert lokale Zeitzone-Darstellung. Kein `chrono`, kein `moment.js`.

### Warum `invoke()` direkt statt `bindings/index.ts` importieren

`index.html` ist kein TypeScript-Projekt und hat keinen Build-Step. `bindings/index.ts` ist COMMITTED als Typ-Referenz und CI-Gate-Artefakt, aber wird vom HTML NICHT importiert. Das ist das bestehende Phase-2-A-Pattern. `reload_locale` im HTML wurde ebenfalls direkt via `invoke()` aufgerufen, obwohl es nicht in den Bindings war — und das war kein Bug (nur pre-existing Drift, den AC-1 behebt).

### `window.confirm()` in Tauri-WebView

`window.confirm()` ist in der Tauri-WebView-Umgebung auf Windows verfügbar (standard Chromium-Behavior). Es rendert den Browser-nativen Confirm-Dialog — kein Custom-Modal nötig für Phase-1.

### CSS-Farb-Palette (Referenz aus bestehendem `<style>`)

| Token | Wert |
|-------|------|
| Background | `#0d0f14` |
| Surface | `#1a1e28` |
| Border | `#2a3040` |
| Text primary | `#e0e4ef` |
| Text secondary | `#8090a0` |
| Brand / Active | `#2ac3a8` |
| Error | `#ff7369` |
| Error background | `#2e1414` |
| Error border | `#ff736930` |

### Tab-Routing Implementierung (concrete Guide)

```js
// Direkt unterhalb der SettingsPanel-Funktion:
function HistoryPanel() {
  // ... (AC-3)
}

function App() {
  const [activeTab, setActiveTab] = useState('settings');
  return h('div', null,
    // Tab bar
    h('div', { className: 'tabs' },
      h('button', {
        className: `tab-btn ${activeTab === 'settings' ? 'active' : ''}`,
        onClick: () => setActiveTab('settings'),
      }, 'Settings'),
      h('button', {
        className: `tab-btn ${activeTab === 'history' ? 'active' : ''}`,
        onClick: () => setActiveTab('history'),
      }, 'History'),
    ),
    // Active panel
    activeTab === 'settings' ? h(SettingsPanel, null) : h(HistoryPanel, null),
  );
}

createRoot(document.getElementById('root')).render(h(App, null));
```

### `get_history` Aufruf-Parameter

```js
invoke("get_history", { limit: 100 })
```

- `limit: 100` ist hard-coded in Phase-1 (zeigt die 100 neuesten Einträge).
- Default ohne Parameter würde `MAX_LIST_LIMIT = 1000` zurückgeben — zu viel für ein Panel.
- Kein Paginierungs-UI in Phase-1 (`backlog.md` kandidiert als Post-MVP).

### Delete: Optimistic vs. Confirmed Removal

Story 9.3 nutzt **confirmed removal** (NICHT optimistic): erst nach API-Erfolg wird der Entry aus dem lokalen State entfernt. Begründung: History ist Read-only-Panel, keine Performance-kritische Live-Liste. Optimistic-Removal bei Delete-Failure würde Entry "wiederauftauchen" lassen — für ein Settings-artiges Panel unnötig verwirrend.

### App.error-Listener: Kein doppelter Listener

`SettingsPanel` hat bereits einen `app.error`-Listener. `HistoryPanel` soll KEINEN eigenen `app.error`-Listener hinzufügen — nur einen eigenen `toast`-State für lokale Command-Errors. Globale Backend-Errors landen im SettingsPanel-Listener (der bleibt aktiv solange App läuft). Das ist eine bekannte Einschränkung Phase-1 (der Listener lebt in der Komponente, nicht auf App-Level) — kein Fix in dieser Story.

### Neue Dateien

Keine.

### Geänderte Dateien

- `shells/windows/src/index.html` — Tab-Routing (App-Komponente) + HistoryPanel-Komponente + CSS-Additions
- `shells/windows/src/bindings/index.ts` — regeneriert via `cargo xtask generate-bindings`
- `shells/windows/locales/en.json` — 7 neue i18n-Keys
- `shells/windows/locales/de.json` — 7 neue i18n-Keys

### Regressions-Schutz

- **SettingsPanel inhaltlich unverändert:** `h1`-Titel entfällt, aber sonst zero-touch.
- **`app.error`-Listener in SettingsPanel:** bleibt registriert, weil SettingsPanel immer live ist (Tab-Switch unmountet nicht wirklich — React-State bleibt, solange App läuft; ohne explicit Tab-Unmounting bleibt auch der Listener aktiv).
- **bindings-drift-Gate:** MUSS nach Bindings-Regen grün sein (AC-10).
- **`cargo xtask lint-events` (G3):** Keine Core-Änderungen in Story 9.3, kein Risk.
- **`cargo xtask verify-release` (G2):** Keine Rust-Änderungen, kein Risk.

### References

- [architecture.md:290] — Tab-basiertes State-Routing (kein React Router), Settings/History/Onboarding als Panels
- [architecture.md:482] — `history.empty_state.title` als Beispiel-Key-Name (Namespace-Norm)
- [architecture.md:483] — Namespace matched Feature-Folder: `history.*` → `features/history/`
- [architecture.md:812–818] — 4-State AsyncState-Union + P1-Extension `refreshing`-State (noch nicht nötig)
- [architecture.md:555–570] — `shells/windows/src/features/history/` als geplante Phase-2-B-Location (NICHT jetzt anlegen)
- [architecture.md:754] — Action-Naming: `loadHistory`, `clearHistory` (verb-first)
- [shells/windows/src/index.html] — ESM CDN React, `invoke()`-Pattern, `errorToToast()`, `loading-row`, `spinner`-CSS, Tab-CSS-Palette
- [shells/windows/src-tauri/src/commands/history.rs] — `HistoryEntryDto`-Felder, `MAX_LIST_LIMIT = 1000`, `get_history`-Signatur
- [docs/backlog.md §History-Panel] — "Panel = UI-Surface für Read/Delete/Clear All"
- [memory/feedback_premature_abstraction_guard] — kein File-Split jetzt; inline bleibt
- [Story 9.2 Dev Notes §Pattern-Vorlagen] — SettingsPanel als Vorlage-Referenz
- [Story 9.2 AC-7] — `HistoryEntryDto` exakte Typen + camelCase-Mapping

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (create-story + dev-story 2026-05-03)

### Debug Log References

### Completion Notes List

- Bindings regeneriert: `getHistory`, `deleteHistoryEntry`, `clearHistory`, `HistoryEntryDto`, `reloadLocale` (Bonus-Drift-Fix) → `bindings-drift` clean.
- Tab-Routing: `App`-Wrapper mit `activeTab`-State, Tab-Bar CSS (`.tabs`, `.tab-btn`, `.tab-btn.active`). `h1` entfernt.
- `HistoryPanel`: 4-state `loadState`, `useEffect(loadHistory, [])` mit `limit: 100`, Entry-List mit per-entry Delete + `deletingId`, Clear-All mit `window.confirm()`, Empty State. Alle CSS-Classes per Spec.
- 7 neue i18n-Keys in `en.json` + `de.json`.
- SettingsPanel-Internals zero-touch (diff-verified: nur `h1` + letztes `createRoot`-Call entfernt).

### File List

- `shells/windows/src/index.html`
- `shells/windows/src/bindings/index.ts`
- `shells/windows/locales/en.json`
- `shells/windows/locales/de.json`
- `_bmad-output/implementation-artifacts/9-3-history-panel.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-05-03: Story 9.3 implementiert — Tab-Routing (App-Wrapper), HistoryPanel-Komponente (Load/Delete/ClearAll/EmptyState), Bindings-Regen, 7 i18n-Keys. Status → review.
