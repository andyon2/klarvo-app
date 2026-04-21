---
name: Story 3.5 — WinSendInputPasteBackend Shell-Impl
epic: 3
story_number: "3.5"
status: Draft
dependencies:
  - "3.4"
---

# Story 3.5: `WinSendInputPasteBackend` Shell-Impl

## Outcome

`shells/windows/src-tauri/src/paste.rs` implementiert `PasteBackend` via Win32 `SendInput`
(Ctrl+V-Key-Injection). Ein neuer i18n-Key `error.paste.send_input_failed` ist in
`locales/en.json` + `locales/de.json` registriert. Unit-Test über einen schmalen internen
Abstraction-Layer verifiziert die Ctrl+V-Sequence headless.

## Acceptance Criteria

### AC-A — Struct + Windows-Crate-Features

**Given** `shells/windows/src-tauri/Cargo.toml` listet das `windows`-Crate bereits für
KeyStore (Story 1C)  
**When** `WinSendInputPasteBackend` hinzugefügt wird  
**Then**

- `pub struct WinSendInputPasteBackend;` ist **stateless** — kein Struct-Field (kein
  Ressource-Lifecycle zu halten)
- `shells/windows/src-tauri/Cargo.toml` ergänzt im `windows`-Crate-Feature-Set:
  `Win32_UI_Input_KeyboardAndMouse` (enthält `SendInput`, `INPUT`, `KEYBDINPUT`,
  `INPUT_KEYBOARD`, `KEYBD_EVENT_FLAGS`, `VK_CONTROL`, `VK_V`, `KEYEVENTF_KEYUP`)
- `WinSendInputPasteBackend` lebt in `shells/windows/src-tauri/src/paste.rs`
- `paste.rs` ist innerhalb des `#[cfg(target_os = "windows")]`-Scopes des Crates (Story 3.1
  AC-E Gate gilt Crate-weit); ein separates File-Level-Gate ist nicht erforderlich, aber
  ein Rustdoc-Kommentar am Modul-Kopf expliziert:
  `// Windows-only: depends on Win32_UI_Input_KeyboardAndMouse, no non-Windows path.`

### AC-B — Impl-Shape: Ctrl+V via `SendInput`

**Given** `WinSendInputPasteBackend` existiert per AC-A  
**When** `PasteBackend::paste()` aufgerufen wird  
**Then**

- `impl PasteBackend for WinSendInputPasteBackend` implementiert:
  ```rust
  async fn paste(&self) -> Result<(), AppError>
  ```
- Der Body delegiert an eine interne sync-Funktion `fn send_ctrl_v() -> Result<(), u32>`
  (oder gleichwertig), die 4 `INPUT`-Structs als Array aufbaut:
  1. `VK_CONTROL` Key-Down
  2. `VK_V` Key-Down
  3. `VK_V` Key-Up (`KEYEVENTF_KEYUP`)
  4. `VK_CONTROL` Key-Up (`KEYEVENTF_KEYUP`)
- Der `unsafe`-Block ruft `SendInput(&inputs)` auf (windows-crate-API)
- `async fn paste` wrапpt den sync-Call via `tokio::task::spawn_blocking` **oder** als
  direkten sync-Call im async-Kontext — Delegate-Choice; spawn_blocking ist korrekteres
  async-Verhalten bei blocking-OS-Calls, aber SendInput ist typischerweise
  sub-millisecond; Wahl mit Rationale im Rustdoc

### AC-C — Error-Mapping: SendInput Return-Value

**Given** `SendInput` ein Return-Value `!= 4` liefert (OS-Fehler)  
**When** der Fehlerfall eintritt  
**Then**

- `SendInput` returniert die Anzahl der tatsächlich injizierten Events (erwartet: 4); bei
  abweichendem Return-Value:
  ```rust
  AppError {
      kind: AppErrorKind::Io,
      message: format!("SendInput returned {} (expected 4); GetLastError: {}", rc, last_error),
      user_message: Some("error.paste.send_input_failed".to_string()),
      retryable: false,
  }
  ```
- `GetLastError()` (windows-crate: `windows::Win32::Foundation::GetLastError`) wird im
  Fehlerfall aufgerufen und im `message`-Field (tech-audience) festgehalten
- Der neue i18n-Key `error.paste.send_input_failed` wird in `locales/en.json` +
  `locales/de.json` registriert (AC-G)

### AC-D — Focus-Window-Precondition: Lazy-Assumption

**Given** kein Fenster hat den Fokus wenn `paste()` aufgerufen wird  
**When** `SendInput` ausgeführt wird  
**Then**

- **Delegate-Choice: Lazy-Assumption (Option b) gewählt** — kein Pre-Check via
  `GetForegroundWindow`; wenn kein Fenster fokussiert ist, returniert `SendInput` 0
  (oder partial) → fällt in AC-C Error-Mapping
- Rationale (Rustdoc am Impl): `GetForegroundWindow` als Pre-Check wäre redundant, da
  `SendInput(0)` im NULL-Focus-Case bereits den fehlerhaften Return-Value liefert. Pre-Check
  würde eine Race-Condition nicht eliminieren (Fenster kann zwischen Check und SendInput
  den Fokus verlieren). Phase-2-UX-Option: Settings-UI-Overlay als immer-fokussiert
  Alternative für spezielle Workflows.
- Key `error.paste.send_input_failed` deckt beide Fälle (OS-Error + No-Focus) ab — aus
  User-Sicht ist die Ursache dieselbe (Paste nicht ausgeführt)

### AC-E — `unsafe`-Block-Rustdoc

**Given** der `unsafe`-Block für `SendInput`  
**When** Rustdoc auf dem Block (oder der umschließenden Funktion) geschrieben wird  
**Then**

- Rustdoc trägt mindestens drei explizite SAFETY-Invarianten:
  ```
  // SAFETY:
  // 1. Array-Pointer + Länge: `inputs.len()` und `inputs.as_ptr()` sind konsistent;
  //    `inputs` ist Stack-alloziert und lebt für die Dauer des SendInput-Calls.
  // 2. KEYBDINPUT-Struct-Felder sind valide: VK_CONTROL (0x11) und VK_V (0x56)
  //    sind bekannte, stabile Virtual-Key-Codes im gültigen u16-Range.
  // 3. Kein shared-mutable-State: WinSendInputPasteBackend ist stateless;
  //    concurrent calls produzieren unabhängige Key-Sequences (OS-serialized).
  ```
- Der `unsafe`-Block ist so klein wie möglich — nur der `SendInput`-Aufruf selbst ist
  `unsafe`, nicht die Array-Konstruktion

### AC-F — Unit-Test: Schmaler Internal-Abstraction-Layer

**Given** `SendInput` direkt zu testen bedeutet OS-Input-Injection  
**When** Unit-Tests implementiert werden  
**Then**

- **Option (a) — Schmaler interner Trait-Wrap (empfohlen):**
  Eine `#[cfg(test)]`-sichtbare Trait `trait InputSender { fn send(&self, inputs: &[INPUT]) -> u32 }`
  wird intern definiert. `WinSendInputPasteBackend.paste_impl()` delegiert an `&dyn InputSender`.
  `RealInputSender` wrапpt den `SendInput`-Call; `MockInputSender` recorded inputs + returniert
  konfigurierten Wert. `WinSendInputPasteBackend` hält im Test-Context einen `MockInputSender`.
  Test verifiziert: 4 INPUTs mit korrekter VK-Sequence (VK_CONTROL down, VK_V down, VK_V up,
  VK_CONTROL up)
- **Option (b) — `#[ignore]`-manueller Test als Fallback:**
  Falls (a) als over-engineered bewertet: ein `#[test] #[ignore]` Test mit Instruktion
  „Fokus auf ein Notepad-Fenster, dann `cargo test -- --ignored`", xtask-Anchor
  `cargo xtask smoke-test-paste`. Nicht als automatisierter Gate-Test
- Delegate wählt zwischen (a) und (b) mit Rationale in Technical Notes
- Mindestens ein Compile-Check-Test (kein `#[ignore]`) verifiziert, dass
  `Arc<dyn PasteBackend>` mit `WinSendInputPasteBackend` konstruierbar ist

### AC-G — i18n-Key-Registration

**Given** Story 3.1 AC-D hat `locales/en.json` + `locales/de.json` angelegt  
**When** diese Story committed wird  
**Then**

- `locales/en.json` bekommt mindestens:
  ```json
  "error.paste.send_input_failed": "Paste failed. The clipboard content could not be injected into the active window."
  ```
- `locales/de.json` bekommt den gleichen Key; Übersetzung Delegate-Choice oder
  TODO-Marker-Pattern (analog Story 3.2 AC-G)
- Beide Locale-Files bleiben valides JSON

## Technical Notes

### Stateless Struct Rationale

`WinSendInputPasteBackend` hält keine OS-Ressource. `SendInput` nimmt keine persistente
Handle-Verbindung. Kein `Drop`-Handler, kein Constructor-State. Das simplest mögliche
`impl`.

### i18n-Key-Präfix `error.paste.*`

Per `docs/shell-error-mapping.md` Evolution-Policy: `Io`-Kind → `error.io.*` normaler Weise,
aber Paste-Errors haben semantic-Domain-Präfix `error.paste.*` (Shell-Mapping-Tabelle
expliziert das als erweiterte Convention für Shell-spezifische Subsysteme). Dieser Präfix wurde
bereits in Story 3.4 AC-C als Forward-Reference im PasteBackend-Rustdoc verankert.

### Win32 SendInput API-Referenz

`SendInput` aus `windows::Win32::UI::Input::KeyboardAndMouse`. Dokumentation:
"Microsoft Win32 Keyboard and Mouse Input Functions" (MSDN). Die Funktion serialisiert
Key-Events in den OS-Keyboard-Input-Buffer — sie ist nicht garantiert synchron mit der
Empfangsseite, aber Sub-Millisecond in der Praxis für 4 Events.

### `spawn_blocking` vs. Direktaufruf

`SendInput` ist ein blocking-Syscall (kurz, aber nonetheless OS-call). Korrekte async-Praxis
wäre `tokio::task::spawn_blocking(|| send_ctrl_v())`. Da `SendInput` <1ms dauert, ist
ein direkter Call in einem tokio-Task vertretbar für Phase-1. Delegate dokumentiert die
Entscheidung im Rustdoc.

## Dependencies

- Story 3.4 (PasteBackend-Trait definiert in `klarvo-core`)
- ADR-0012 §SD-2 — WinSendInputPasteBackend als Shell-scoped PasteBackend-Impl
- `docs/shell-error-mapping.md` — `Io`-Kind → Toast-Treatment, `error.paste.*`-Präfix
- Story 3.1 AC-E — Windows-cfg-Gate (Crate-weit aktiv, kein doppeltes File-Gate)
- `memory/project_shell_session_lifecycle` — Step 7 (PasteBackend.paste) im 7-Step-Cycle
