---
name: Story 3.2 — Shell-Config Parsing + Validation
epic: 3
story_number: "3.2"
status: Draft
dependencies:
  - "3.1"
---

# Story 3.2: Shell-Config Parsing + Validation

## Outcome

`shells/windows/src-tauri/src/config.rs` parsed `config.toml` aus dem
Windows-Convention-Pfad (`%APPDATA%\Klarvo\config.toml`) in eine typisierte `ShellConfig`-Struct.
Unbekannte Felder werden mit `AppError::Configuration` abgelehnt (strict-parse analog
`feedback_manifest_compile_contract`). Drei neue i18n-Keys in `locales/en.json` +
`locales/de.json` registriert.

## Acceptance Criteria

### AC-A — Config-File-Path-Resolution

**Given** die Windows-Shell sucht nach `config.toml` in einem OS-konformen Verzeichnis  
**When** `resolve_config_path() -> PathBuf` aufgerufen wird  
**Then**

- Die Funktion liefert `%APPDATA%\Klarvo\config.toml` zurück, wobei `%APPDATA%` via
  `std::env::var("APPDATA")` oder `windows::Win32::UI::Shell::SHGetKnownFolderPath`
  (Delegate-Choice) aufgelöst wird
- Wenn `APPDATA` nicht gesetzt ist → `AppError { kind: Configuration,
  message: "APPDATA environment variable not set", user_message:
  Some("error.config.missing"), retryable: false }`
- Der Pfad ist ein reiner Resolver — er legt das Verzeichnis **nicht** an und öffnet die Datei
  **nicht**
- `cfg(target_os = "windows")`-Gate gilt für die gesamte `config.rs`-Datei; die Funktion
  kompiliert nur auf Windows (analog Story 3.1 AC-E)

### AC-B — `ShellConfig`-Struct-Shape + Serde-Deserialize

**Given** eine valide `config.toml` im Pfad aus AC-A  
**When** `load_config(path: &Path) -> Result<ShellConfig, AppError>` aufgerufen wird und die
Datei existiert  
**Then**

- `ShellConfig` ist deklariert als:
  ```rust
  #[derive(Debug, Clone, serde::Deserialize)]
  #[serde(deny_unknown_fields)]
  pub struct ShellConfig {
      #[serde(default = "ShellConfig::default_hotkey")]
      pub hotkey: String,
      #[serde(default = "ShellConfig::default_output_target")]
      pub output_target_id: String,
      #[serde(default = "ShellConfig::default_locale")]
      pub locale: String,
  }
  ```
  mit `default_hotkey() -> "CommandOrControl+Shift+Space"` (PRD `scopeLock.hotkeyDefault`),
  `default_output_target() -> "clipboard"`, `default_locale() -> "en"`
- Ein komplett leeres `config.toml` (`{}` oder leer) ist valide und returniert `ShellConfig`
  mit allen Defaults
- `load_config` liest die Datei via `std::fs::read_to_string`, parsed via `toml::from_str`
- Erfolg returniert `Ok(ShellConfig { ... })`
- Das Locale-Feld akzeptiert nur `"en"` oder `"de"` — Validation kommt in AC-C

### AC-C — Validation-Errors: Unknown Fields + Invalid Locale

**Given** eine `config.toml` mit ungültigem Inhalt  
**When** `load_config` aufgerufen wird  
**Then**

- **Unknown Field:** `config.toml` enthält ein Feld, das nicht in `ShellConfig` definiert ist
  → `#[serde(deny_unknown_fields)]` löst `toml::de::Error` aus → `load_config` mappt auf
  `AppError { kind: Configuration, message: "<toml-error-string>",
  user_message: Some("error.config.unknown_field"), retryable: false }`
- **Invalid Locale:** `locale = "fr"` (oder anderer nicht-`en`/`de`-Wert) → Post-Parse-Validation
  in `load_config` nach erfolgreichem Serde-Deserialize prüft
  `matches!(config.locale.as_str(), "en" | "de")`; bei Fail →
  `AppError { kind: Configuration, message: format!("unsupported locale: {}", config.locale),
  user_message: Some("error.config.invalid_locale"), retryable: false }`
- **Missing File:** `config.toml` existiert nicht → `std::io::Error::NotFound` wird auf
  `AppError { kind: Configuration, message: "<path> not found",
  user_message: Some("error.config.missing"), retryable: false }` gemappt. Das ist kein
  Fatal-Error — User legt Config selbst an (AC-E)
- **Corrupt TOML (Parse-Error, nicht Unknown-Field):** `toml::de::Error` ohne Unknown-Field-Cause
  → `AppError { kind: Configuration, user_message: Some("error.config.missing"), ... }` (gleicher
  Key wie Missing-File, da aus User-Sicht „Config nicht nutzbar"). Delegate-Choice: separater Key
  `error.config.parse_error` ist akzeptiert, wenn er in den Locale-Files registriert wird

### AC-D — `load_config`-Funktion-Shape

**Given** `config.rs` ist implementiert  
**When** Consumer `load_config(&path)` aufruft  
**Then**

- Signatur: `pub fn load_config(path: &std::path::Path) -> Result<ShellConfig, AppError>`
- Der Funktionskörper ist rein sync (kein `async`) — TOML-Parse ist kein I/O-Block-Risiko im
  Maßstab dieser Anwendung; Tauri-Setup-Phase ist sync
- Rustdoc auf `load_config` beschreibt:
  1. Path-Convention (`%APPDATA%\Klarvo\config.toml`)
  2. Fehlerfälle (Missing-File, Unknown-Field, Invalid-Locale) mit jeweiligem `AppErrorKind`
  3. Forward-Reference: `// Story 3.10 wires ShellConfig into tauri::State<Arc<ShellConfig>>`

### AC-E — No-Auto-Create Policy

**Given** `config.toml` fehlt  
**When** `load_config` aufgerufen wird  
**Then**

- Die Datei wird **nicht** automatisch erstellt; die Funktion returniert
  `Err(AppError { kind: Configuration, ... })` (AC-C Missing-File-Path)
- Rustdoc auf `load_config` enthält expliziten Kommentar:
  `// Phase-2: Settings-UI creates config.toml on first-run via xtask or settings-save.`
  `// Phase-1: user creates config.toml manually. Missing file is not auto-generated.`
- Die Development-Guide (`docs/development-guide.md`) oder ein `README.md` in
  `shells/windows/` dokumentiert, wie der User eine minimal-Config anlegt. Delegate-Choice
  ob inline-Rustdoc oder externes Dokument ausreicht

### AC-F — Unit-Tests (in-memory TOML-Strings)

**Given** `config.rs` ist implementiert  
**When** `cargo test -p <windows-shell-crate>` ausgeführt wird (oder separate
`shells/windows/src-tauri/tests/config_test.rs`)  
**Then**

- **Test 1 — Happy-Path minimal:** `load_config` mit in-memory-TOML via `toml::from_str`
  (oder temporärem File-Fixture) mit leerem `{}` returniert `Ok(ShellConfig)` mit Defaults
- **Test 2 — Happy-Path explicit:** `hotkey = "CommandOrControl+Shift+Space"\n
  output_target_id = "clipboard"\nlocale = "en"` → Ok mit korrekten Werten
- **Test 3 — Unknown-Field rejected:** `hotkey = "X"\nunknown_key = "Y"` → `Err` mit
  `AppErrorKind::Configuration` und `user_message == Some("error.config.unknown_field")`
- **Test 4 — Invalid-Locale rejected:** `locale = "fr"` → `Err` mit
  `user_message == Some("error.config.invalid_locale")`
- **Test 5 — Missing-File:** nicht-existenter Pfad → `Err` mit `AppErrorKind::Configuration`
  und `user_message == Some("error.config.missing")`
- Tests operieren auf in-memory-TOML-Strings via `toml::from_str` direkt wo möglich
  (vermeidet echte Filesystem-Interaktion in den meisten Cases)

### AC-G — i18n-Key-Registration

**Given** Story 3.1 AC-D hat `locales/en.json` + `locales/de.json` als leere `{}` angelegt  
**When** diese Story committed wird  
**Then**

- `locales/en.json` enthält mindestens:
  ```json
  {
    "error.config.missing": "Configuration file not found. Please create config.toml.",
    "error.config.unknown_field": "Unknown field in config.toml. Please remove unrecognized keys.",
    "error.config.invalid_locale": "Unsupported locale. Supported values: en, de."
  }
  ```
- `locales/de.json` enthält die gleichen Keys; deutsche Übersetzung ist Delegate-Choice;
  Fallback auf englische Texte mit TODO-Marker ist akzeptiert:
  `"error.config.missing": "TODO(de): Configuration file not found."`
- Beide Locale-Files bleiben valides JSON nach der Ergänzung

## Technical Notes

### `deny_unknown_fields` Rationale

Analog `feedback_manifest_compile_contract`: kein silent-merge unbekannter Felder. Ein User,
der `hotkeyy = "..."` (Typo) in die Config schreibt, soll sofort einen klaren Error bekommen,
nicht schweigend den Default-Wert verwenden. Verhindert Drift zwischen Config-Spec und
tatsächlicher Nutzung.

### Locale: `String` mit Whitelist-Check

Phase-1 hat genau 2 Locales (`en`/`de`). Enum-Machinery (rename-all, `From<Locale> for String`)
für 2 Varianten ist premature-abstraction (`feedback_premature_abstraction_guard`). Whitelist-Check
in AC-C deckt die Type-Safety-Lücke ausreichend ab. Phase-2 Post-MVP-P2-Backlog enthält
„weitere UI-Languages" — dann natürlicher Enum-Upgrade-Moment, nicht vorher.

### `ShellConfig` in Tauri-managed-State

`app.manage(Arc::new(config))` kommt in Story 3.10 (Bootstrap-Integration), nicht hier.
Diese Story definiert nur Parsing + Validation. Der Rustdoc-Forward-Reference-Kommentar in
`load_config` verankert das.

### Windows-Path-Resolution

`std::env::var("APPDATA")` ist die einfachste portable Methode. Für
Production-Robustheit wäre `windows::Win32::UI::Shell::SHGetKnownFolderPath` mit
`FOLDERID_RoamingAppData` präziser, aber in Phase-1 ist `env::var` ausreichend.
`APPDATA` ist auf Windows-10+ immer gesetzt für normale User-Sessions.

## Dependencies

- Story 3.1 (Crate-Setup + `locales/`-Dateien existieren)
- `feedback_manifest_compile_contract` — strict-parse, kein `warn!+skip`
- ADR-0009 §SD-2 — i18n-Resolve im Frontend (Shell liefert Keys, nicht übersetzten Text)
- `docs/shell-error-mapping.md` — `Configuration`-Kind → Modal-Treatment
- `memory/project_i18n_core_contract` — Core emittiert Keys, Shell resolved
