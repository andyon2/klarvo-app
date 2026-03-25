# Feature-Plan: i18n System (Deutsch + Englisch)

## Ziel
App-UI in Deutsch und Englisch bedienbar machen. Technische Begriffe (STT, API Key, Speech-to-Text) bleiben in beiden Sprachen Englisch.

## Architektur-Entscheidungen
- **Library:** react-i18next mit JSON-Dateien pro Sprache (`en.json`, `de.json`)
- **Namespace:** Single namespace `translation` (App ist nicht gross genug fuer mehrere)
- **Desktop:** Expliziter Language Switcher in Settings (DE/EN Dropdown)
- **Android:** Folgt System-Locale (kein eigener Switcher, Android laedt automatisch `values-de/strings.xml`)
- **Rust-Backend:** Errors bleiben Englisch. Frontend mappt haeufigste Error-Patterns auf i18n-Keys, unbekannte Errors werden im Original angezeigt
- **Default:** Englisch (`en`)

## Tasks

### Task 1: i18n-Infrastruktur aufsetzen
- **Agent:** ui-dev
- **Dateien:** package.json, src/i18n/index.ts, src/i18n/locales/en.json, src/i18n/locales/de.json, src/main.tsx
- **Beschreibung:** i18next + react-i18next installieren, Init-Datei anlegen. Sprache wird aus Config gelesen (kein Browser-Detector). Fallback: en.

### Task 2: uiLanguage in Config + Rust
- **Agent:** rust-core
- **Dateien:** src/types.ts, src-tauri/src/config.rs, get/save_settings Commands
- **Beschreibung:** Neues Feld `ui_language: String` (Default "en") in Config-Struct. Serde-Default fuer Backward-Compatibility.

### Task 3: Language Switcher in Settings
- **Agent:** ui-dev
- **Dateien:** src/components/SettingsPanel.tsx, Settings-Hooks
- **Beschreibung:** Dropdown [English | Deutsch] in General Settings. Sofortiger Wechsel via i18n.changeLanguage(), persistiert in Config.

### Task 4: Alle Komponenten auf i18n-Keys umstellen
- **Agent:** ui-dev (ggf. 2 Sub-Tasks wenn >35min)
- **Dateien:** App.tsx, Onboarding.tsx, FloatingBar.tsx, SettingsPanel.tsx, AdvancedSettingsPanel.tsx, WhisperModelManager.tsx, VoiceNotesPanel.tsx, SnippetsPanel.tsx, CostDashboard.tsx, QuickTip.tsx, en.json, de.json
- **Beschreibung:** Alle hardcodierten Strings durch t("key") ersetzen. Keys hierarchisch: settings.hotkey_label, onboarding.welcome_title. Error-Mapping fuer Rust-Backend-Errors in App.tsx.

### Task 5: Android Strings internationalisieren (parallel)
- **Agent:** android-platform
- **Dateien:** android/res-values/strings.xml, android/res-values-de/strings.xml (neu), VoxlitOverlayService.kt
- **Beschreibung:** Alle hardcodierten Toast/Notification-Strings nach strings.xml auslagern. Deutsche values-de/strings.xml anlegen.

## Testplan
- [ ] Sprachenwechsel wechselt UI sofort ohne Reload
- [ ] Sprache bleibt nach App-Neustart erhalten
- [ ] Alle sichtbaren Texte in gewaehlter Sprache
- [ ] Fallback bei fehlender uiLanguage in Config = Englisch
- [ ] Android: System-Sprache Deutsch = deutsche Toasts
- [ ] Rust-Fehler werden im Frontend uebersetzt (haeufigste Patterns)
- [ ] Kein Key in en.json ohne Pendant in de.json (Lint-Check)

## Risiken
- Task 4 ist gross (~30+ Strings allein im Onboarding). Bei Zeitueberschreitung aufteilen.
- Rust-Errors sind freie Strings, nicht typisiert. Vollstaendige Uebersetzung nicht moeglich.
- Preview-Mode: i18n-Init darf nicht auf Tauri warten. Init mit Default "en", async nachladen.
