# Klarvo v2 — Backlog (Phase-N+1-Single-Source-of-Truth)

**Status:** Living Document
**Bootstrapped:** 2026-04-21 (Phase-1-Closure Review)

## Zweck

Diese Datei ist die **einzige authoritative Liste aller Phase-N+1-Items** — alles, was im aktuellen Phase-Scope nicht enthalten ist, aber für eine spätere Phase vorgemerkt bleibt. Vor 2026-04-21 war diese Information verteilt über PRD-Frontmatter (`explicitlyOutOfScope`-Array), Product-Brief-Prose ("P1 (kurz nach MVP)", "P2 (Power-Features)", "DEFER / nicht in v2"), Distillate-Phase-Definitionen und Architecture-Tabellen ("Deferred auf P1/P2", "Phase-2+"). Scattered-Backlog ist Leck-Risiko.

Konvention: `memory/feedback_backlog_discipline`. Bei jedem Scope-Cut oder Review-Deferral wird ein Backlog-Entry sofort hier hinzugefügt, mit Source-Ref + Status.

## Struktur

Jedes Item hat:

- **Source**: Wo wurde es entschieden/vermerkt? (PRD-Frontmatter-Zeile, Brief-Section, ADR-Nummer, Phase-N-Review-Axis, etc.)
- **Description**: 1–2 Sätze Was + Motivation
- **Dependencies**: Was muss vorher geklärt/gemacht sein?
- **Status**: `Planned` | `UX-Spec-TODO` | `Blocked-by-<X>` | `Ready-for-Story-Writing` | `BLOCKER`

Gliederung nach Phasen. Innerhalb einer Phase nach groben Themenblöcken.

---

## Phase 2 — Windows daily usable

**Phase-Goal** (ref Product-Brief §Phasenplan, PRD Phase 1 Growth Features): Windows-Shell vollständig, alle Recording-Modi, zweite Hotkey-Slot, komplette Pill Bar, minimales Settings-Panel, zweiter STT-Plugin (Trait-Stability-Test), OS-Keystore als Release-Default.

### Audio-Cpal Precision & Correctness Hardening

- **Source**: Phase-1-Review Axis #2 (2026-04-21), `project_phase1_complete` §Carry-Over
- **Description**: **Drei** Items aus Story 2.5 / Phase-1-Closure-Review (`klarvo-audio-cpal`), die Phase-1 pragmatisch als akzeptabel eingeordnet wurden. (1) `ts_ms`-Stamping-at-First-Sample: aktuell approximiert im flush_chunks-Loop; exakte Sample-Count-basierte Timestamp-Derivation ist Phase-2-Tightening. (2) Resampler-Multi-Call-Steady-State-Test: `resampler_sample_count_correct`-Test wurde auf Range `[1, 342]` relaxed statt fixer Gleichheit; Multi-Call-Steady-State-Assertion ist nachzuholen. (3) Safety-Comment-Accuracy-Fix: Der existierende Comment in `klarvo-audio-cpal/src/source.rs:45-47` behauptet fälschlich „cpal::Stream is Send on all supported platforms" — cpal 0.15 markiert Stream via NotSendSyncAcrossAllPlatforms (PhantomData<*mut ()>) universell als !Send + !Sync; das unsafe impl Send/Sync ist load-bearing. Korrekter Safety-Reason: „CpalGuard wird nur beim Construction-Zeitpunkt in einen anderen Thread bewegt und nur vom Owning-Thread gedropped; Stream wird nie durch &-Reference mutiert" (Reviewer-Self-Finding aus Phase-1-Closure-Review 2026-04-21).
- **Dependencies**: Keine (kann parallel zu anderen Phase-2-Items laufen)
- **Status**: Ready-for-Story-Writing

### Audio-Capture-Config-Overrides via ShellConfig

- **Source**: `output/planning-artifacts/epics/epic-3/story-3.7-cpal-audiosource-wireup.md` Technical Notes §Phase-2-Expansion
- **Description**: User-konfigurierbare Audio-Settings (Sample-Rate, Channel-Count, Device-Selection) via `ShellConfig`. Phase-1 `CpalAudioSource` nutzt OS-determined defaults (`default_host().default_input_device().default_input_config()`); User-konfigurierbare Overrides sind legitimer Phase-2-Power-User-Feature. Settings-UI (Phase-2 separat) würde ShellConfig-Audio-Section editierbar machen.
- **Dependencies**: Story 3.2 ShellConfig-Shape-Extension (neue Felder: `audio.sample_rate`, `audio.channels`, optional `audio.device_id`); `klarvo-audio-cpal/src/source.rs` CaptureConfig-Param-Threading (aktuell hardcoded via `default_input_config()`). `CpalAudioSource` als Unit-Struct muss ggf. zu `pub struct CpalAudioSource { config: AudioConfig }` erweitert werden; impliziter Test-Suite-Update (Phase-2-Story-Scope).
- **Status**: Proposed

### Tray-Icon Extensions

- **Source**: Phase-1-Review Axis #4 Tray-Scope-Discussion (2026-04-21)
- **Description**: Tray-Icon (FR20) in Phase-1-MVP zeigt nur Recording/Idle-State. Phase-2-Erweiterungen: (a) Session-Statistik-Counter (Dictations heute / gesamt), (b) Post-Error-Restart-Onboarding-Hint (nach Boot-Error-Recovery), (c) Language-Switcher im Tray-Menu (ui_language vs. output_language).
- **Dependencies**: ADR-0009 SD-4 (Boot-Error-UX) muss entschieden sein — Restart-Onboarding-Hint ist Folge-Feature
- **Status**: Ready-for-Story-Writing (für alle drei Subfeatures)

### Floating Pill Bar

- **Source**: PRD L155 (`explicitlyOutOfScope`: "Floating Pill Bar with waveform/drag/shapes (Phase 2)"), PRD L179 (Umfang Phase 1: "keine Floating Pill Bar"), Product-Brief §Scope MVP-enthalten "komplette Floating Pill Bar (Windows)"
- **Description**: Persistente Floating-Window-UI während Recording, die Waveform visualisiert und per Drag positionierbar ist. UX-Spec unvollständig in aktueller Dokumentation — Shape-Definition (Pill vs. Bar vs. Other), Drag-Behavior-Konvention, Waveform-Rendering-Responsibility (Rust-getriggert per `memory`-Architecture §5), Auto-Hide-Logic (Release + Delay? User-Trigger?), Position-Persistence über Sessions hinweg.
- **Dependencies**: UX-Spec-Decisions (vor Story-Writing); evtl. separater UX-Arbeitsstrang
- **Status**: UX-Spec-TODO

### Toggle + AutoStop + Wait-and-Type Recording-Modi

- **Source**: PRD L153 (`explicitlyOutOfScope`: "Toggle / AutoStop / Wait-and-Type recording modes (Phase 2)")
- **Description**: Zusätzliche Recording-Modi neben Phase-1-Hold-to-Talk. Orchestrator-State-Machine (ADR-0012) muss um weitere State-Transitions erweitert werden.
- **Dependencies**: ADR-0012 implementiert (Phase-1-Epic-3), Settings-Panel (Phase-2-parallel für User-Switch)
- **Status**: Planned

### Second Hotkey-Slot

- **Source**: PRD L154 (`explicitlyOutOfScope`: "Second hotkey slot (Phase 2)"), Product-Brief §Scope MVP-enthalten "2 Slots (skaliert später auf 4–5)"
- **Description**: Zweiter user-konfigurierbarer Hotkey-Slot (z. B. für Toggle-Mode vs. Hold-Mode Parallelbetrieb). ADR-0011 erlaubt additive Slot-Registrierung ohne Plugin-Change. Offene Frage (Distillate §Offene Fragen): Hotkey-Slot-Skalierung MVP (2) → Post-MVP (4–5) Trigger-Bedingung.
- **Dependencies**: ADR-0011-Plugin (Phase-1-Epic-3), Settings-Panel (Phase-2)
- **Status**: Planned

### Minimales Settings-Panel

- **Source**: PRD L156 (`explicitlyOutOfScope`: "Settings UI (Phase 2)")
- **Description**: Erste UI für Hotkey-/Provider-/Language-Config statt `config.toml`-only (Phase-1-Default). Frontend-Scope: Tauri-WebView-Settings-Panel, React-State per Zustand, Form-Validierung via Zod.
- **Dependencies**: Settings-Schema-Revisit-Point (architecture.md §2) ggf. vorher — bei >20 Settings oder Composites Hybrid mit dedizierten Tabellen
- **Status**: Planned

### StylePicker (UI-Component)

- **Source**: Product-Brief §Scope MVP-enthalten UI/UX, PRD §Growth-Features Phase-2 "StylePicker"
- **Description**: UI-Selector für Cleanup-Style (Verbatim / Chat / Polished). Phase-1-Default ist Verbatim-only (`config.toml`); Picker wird relevant wenn Chat-Plugin (später) und Polished-Plugin (Phase 4) existieren.
- **Dependencies**: Chat-Plugin (unterhalb Phase 4), Settings-Panel
- **Status**: Planned

### History-Panel

- **Source**: Product-Brief §Scope MVP-enthalten UI/UX, PRD §Growth-Features Phase-2
- **Description**: UI für History-Einträge (Read/Delete/Clear-All). History-Write passiert bereits Phase-1 (7-Step-Topology-Step-7 äquivalent via OutputTarget-Wire-Up-Downstream-Plan). Panel = UI-Surface dafür.
- **Dependencies**: Settings-Panel / Main-Window (Phase-2), SQLite-History-Schema bereits Phase-0/1
- **Status**: Planned

### Return-Focus Feature

- **Source**: Product-Brief §Scope MVP-enthalten UI/UX, PRD §Growth-Features Phase-2
- **Description**: Nach Klarvo-Trigger kehrt Fokus zum ursprünglichen Foreground-App zurück (nicht zu Klarvo-UI). Windows-Shell-specifisches API-Wiring (GetForegroundWindow / SetForegroundWindow).
- **Dependencies**: Keine
- **Status**: Planned

### Zweiter STT-Plugin (Trait-Stability-Test)

- **Source**: PRD §Growth-Features Phase-2 "zweiter STT-Plugin (Trait-Stability-Test)", Distillate §Erfolgskriterien "Validierungs-Test: Wenn ein 2. STT-Plugin (z. B. Deepgram) in Phase 2 als reiner Trait-Impl einhängbar ist ohne Core-Trait-Änderung → Substrat trägt"
- **Description**: Working-Name `klarvo-plugin-deepgram` (oder anderer STT-Provider). Validiert `SttProvider`-Trait-Stability experimentell.
- **Dependencies**: Provider-Auswahl + BYOK-Key-Flow
- **Status**: Planned

### Windows-Toast-Notifications

- **Source**: Architecture L591 "Windows-Toast-Notifications → Phase 2+"
- **Description**: Native Windows-Toast-API-Integration statt WebView-Notification. Rollout abhängig von UX-Strategie.
- **Dependencies**: Keine
- **Status**: Planned

### Autostart + Notification-Area-Badge

- **Source**: PRD §Growth-Features Phase-2 (implicit über Brief P1 "Autostart")
- **Description**: Klarvo startet mit Windows-Login; Notification-Area zeigt Active-Mode-Badge.
- **Dependencies**: Registry-Key-Setup, Tray-Icon-Erweiterung (siehe "Tray-Icon Extensions")
- **Status**: Planned

### `cargo xtask set-key` (Keystore-CLI)

- **Source**: PRD §Deferred-to-Later-Phases "Phase 2: ... `cargo xtask set-key` (Keystore-CLI)"
- **Description**: CLI-Subcommand zum programmatic Setzen von API-Keys, Alternative zu Settings-UI für Power-User.
- **Dependencies**: OS-Keystore als Release-Default (Dep-Chain)
- **Status**: Planned

### Debug-Export-Zip (Settings-UI-gebunden)

- **Source**: PRD Section, explicit FR40 + Deferred-to-Later-Phases Phase 2, `memory/project_no_remote_telemetry`
- **Description**: User-triggered Zip-Export (Logs + redacted Config + Sys-Info) via Settings-Panel. `klarvo-core::telemetry::export`-Module-Stub existiert bereits Phase-1 (FR40); UI-Trigger + Redaction-Logic sind Phase-2.
- **Dependencies**: Settings-Panel
- **Status**: Planned

### Editor-Schema-Support für Pipeline-Manifest-TOML (VS-Code-JSON / Taplo-LSP)

- **Source**: PRD §Deferred-to-Later-Phases Phase-2 "Editor-Schema-Support für Manifest-TOML (VS-Code-JSON / Taplo-LSP)"
- **Description**: JSON-Schema für `pipeline.toml`, konsumierbar in VS-Code + Taplo-TOML-LSP. Phase-2-Trigger: Validation-Persona onboarded Plugin-Authors, braucht Editor-Support vor `cargo build`-Feedback.
- **Dependencies**: Plugin-Author-Persona aktiv (Phase-2-Timing)
- **Status**: Planned

### i18n-Coverage-Test durch Epic 5 FR34 Lint-Gate ersetzen

- **Source**: Story 4.4 AC-G (2026-04-25)
- **Description**: Story 4.4 manueller Coverage-Test durch Epic 5 FR34 Lint-Gate ersetzen, sobald G3 ausgerollt ist. `REQUIRED_KEYS` in `shells/windows/src-tauri/src/i18n.rs::tests` ist manuell gewartet; neue `error.*`-Konstanten in Core oder Plugins müssen manuell ergänzt werden. FR34 / Epic 5 G3 Lint-Gate (`cargo xtask lint-events`) soll das via AST-Parse automatisch extrahieren.
- **Dependencies**: Epic 5 FR34 (cargo xtask lint-events AST-Pass), G3-Gate-Rollout
- **Status**: Planned

### Windows-Compile-CI-Gate für klarvo-core windows-cfg + klarvo-windows-shell

- **Source**: Epic-3-Code-Review-Pass-Followup (2026-04-25, commit `0b5306e`)
- **Description**: Auf WSL/Linux überspringt cargo den Windows-cfg-Code in `klarvo-core/src/keystore/os/windows.rs` und `klarvo-windows-shell` ist hard `compile_error!`-gated für non-Windows. Konsequenz: 4 Compile-Errors blieben 4 Tage unentdeckt (2× pre-existing aus Story 1C.3 / Story 3.5, 2× Batch-B-Code-Review-Patches die nur Linux-verifiziert wurden). Phase-2-Item: CI-Gate G6 oder GitHub-Actions-Windows-Job, der `cargo check -p klarvo-windows-shell` auf jedem Push gegen master laufen lässt. Alternativ lokales `cargo xtask check-windows`-Subcommand das via cargo.exe + WSL-Interop oder remote Windows-Runner ausführt.
- **Dependencies**: GitHub-Actions-Setup (oder anderer CI-Provider mit Windows-Runner-Support); evtl. `CARGO_TARGET_DIR` auf Windows-Path-Pinning für WSL-Interop-Pfad
- **Status**: Planned

### Plugin-Author-Guide + externe Doc-Site

- **Source**: PRD §Deferred-to-Later-Phases Phase-2 "Plugin-Author-Guide, Pipeline-Authoring-Walkthrough, externe Doc-Site"
- **Description**: Dokumentation für Plugin-Authors (Sekundär-Persona aus Brief §Zielnutzer).
- **Dependencies**: Validation-Persona-Onboarding-Phase
- **Status**: Planned

### WASM-Plugin-Runtime (2+ explorativ)

- **Source**: PRD L517 "Kein JavaScript-/Python-/WASM-Plugin-Authoring Phase 1 (WASM-Runtime-Plugins deferred to Phase 2+, cf. memory/project_plugin_architecture.md)", Brief §Lösung "Eine WASM-Erweiterung für Third-Party-Plugins bleibt als v2.x-Option offen"
- **Description**: Zusätzlicher `WasmPluginLoader`, der dieselbe `PluginRegistry` füllt — ermöglicht Third-Party-Plugins ohne Cargo-Feature-Dance.
- **Dependencies**: Trait-Surface-Stability-Pass (Phase 2), WASM-Host-Choice (wasmtime, wasmer, etc.)
- **Status**: Planned (explorativ, könnte auch Phase 3+ werden)

### Live-Locale-Switch (Hot-Reload)

- **Source**: Story 4.5 `docs/sanity-tester-onboarding.md` (2026-04-25); Story 4.2 AC-E
- **Description**: `ui_language` (und ggf. die anderen Sprach-Achsen) beim laufenden Betrieb wechseln ohne App-Neustart. Phase-1-Constraint: Locale wird einmalig beim Boot aus `ShellConfig` geladen (`shells/windows/src-tauri/src/i18n.rs`); ein Hot-Reload-Pfad fehlt. Tray-Menu-Labels würden bei Locale-Wechsel live aktualisiert. Abhängig von Settings-Panel (UI-Trigger) oder Datei-Watcher auf `config.toml`.
- **Dependencies**: Minimales Settings-Panel (Phase-2) oder File-Watcher-Integration
- **Status**: Planned

### Signierter Installer / MSI-Distribution

- **Source**: Story 4.5 `docs/sanity-tester-onboarding.md` (2026-04-25)
- **Description**: Phase-1 hat keinen signierten Installer — Tester bekommen eine rohe `klarvo.exe` oder bauen selbst. Für Sanity-Tester ohne Rust-Toolchain ist das eine Hürde. Phase-2-Deliverable: signiertes MSI oder NSIS-Installer-Bundle (Tauri `tauri build --bundles msi` / `nsis`). Code-Signing-Zertifikat ist separater Dependency.
- **Dependencies**: Code-Signing-Zertifikat, Windows-Build-Pipeline (CI-Gate G6 oder lokal)
- **Status**: Planned

### Hotkey-Konflikt-Erkennung

- **Source**: Story 4.5 `docs/sanity-tester-onboarding.md` (2026-04-25)
- **Description**: Phase-1 emittiert bei Hotkey-Kollision `error.hotkey.registration_failed` als Toast — User muss den Konflikt selbst lösen und `config.toml` manuell anpassen. Phase-2-UX: (a) Beim Boot-Fehler direkt den konfliktierenden Prozess nennen (Windows `RegisterHotKey` liefert keinen Eigentümer — Workaround via `GetForegroundWindow` / Accessibility-API oder User-Hint im Toast); (b) Settings-Panel mit „Hotkey ändern"-Dialog, der sofort auf Registrierungsfehler reagiert.
- **Dependencies**: Minimales Settings-Panel (Phase-2)
- **Status**: Planned

---

## Phase 3 — Android daily usable

**Phase-Goal** (ref Product-Brief §Phasenplan, PRD §Growth-Features): Android-Shell mit allen Bubble-Zuständen, Gesten, AccessibilityService, JNI-Bridge produktiv (uniffi Control-Plane + raw jni Data-Plane), Android-v1-Import.

### Play-Store-Policy-Audit (AccessibilityService) — ⚠️ BLOCKER

- **Source**: `memory/project_play_store_phase3_blocker`, `memory/project_android_playstore_risk`, Architecture §7 :311 "Play Store als Primär-Distribution, Phase-3-Blocker. Vor Phase-3-Start: AccessibilityService-Policy-Audit als Pflicht-Deliverable"
- **Description**: Google-Play-Policy-Klärung für AccessibilityService-Usage. Deliverables: (1) Policy-Klärungs-Ticket mit Google Play Console Developer Support / Policy-Hotline; (2) Justification-Text (Diktat als Accessibility-Feature positionieren, RSI + motorische Einschränkungen als Sekundärgruppe); (3) Fallback-Plan bei Ablehnung → APK-Direct (Klarvo-Website) + F-Droid + UX-Anpassung für Sideload-Onboarding.
- **Dependencies**: Keine (kann parallel zur Windows-Phase-2-Arbeit starten)
- **Status**: **BLOCKER** — Phase 3 wird nicht mit nur funktionierender APK abgeschlossen, sondern erst mit Play-Submission ODER bestätigtem Fallback-Plan

### Android-Shell (native Kotlin + JNI-Bridge)

- **Source**: PRD L163 (`explicitlyOutOfScope`: "Android shell (Phase 3)"), Brief §Scope "Windows + Android parallel im MVP" (historisch — Phase-Plan revidiert: Phase 3 ist Android)
- **Description**: Native Kotlin-Shell + JNI-Dual-Surface (uniffi Control-Plane + raw jni Data-Plane, ref ADR-0003). `AndroidAudioSource`-Impl (AudioRecord-based). Alle fünf Android-Recording-Modi. Complete Bubble-UX (5 Zustände + Gesten).
- **Dependencies**: Play-Store-Policy-Audit BLOCKER (s. o.), ADR-0012 Orchestrator-Crate (implementiert Phase-1-Epic-3), JNI-Bridge-Productivity-Pfad (ADR-0003)
- **Status**: Blocked-by-Play-Store-Policy-Audit

### AccessibilityPasteBackend

- **Source**: ADR-0012 SD-2 (`PasteBackend`-Trait Android-Äquivalent), Brief §Lösung
- **Description**: Android-spezifische `PasteBackend`-Trait-Impl via AccessibilityService-Paste-Action. Einer der zwei Primary-Consumer der neuen `PasteBackend`-Trait, der Phase-1-Einführung rechtfertigt.
- **Dependencies**: ADR-0012 implementiert, Play-Store-Policy-Audit
- **Status**: Blocked-by-Play-Store-Policy-Audit

### Android-v1-Import

- **Source**: PRD §Growth-Features Phase-3 "Android-v1-Import"
- **Description**: Migrations-Writer (Phase-1 existiert CLI-Stub) bekommt Android-UI-Integration bzw. Android-analogen CLI-Path.
- **Dependencies**: Android-Shell
- **Status**: Planned

### Android-Runtime-Permission-Revoke Handling

- **Source**: ADR-0008 Amendment 1 Resolution Q2 Forward-Ref
- **Description**: AccessibilityService-Permission-Revoke durch User zur Runtime (nicht Startup-Init). Orchestrator-State-Transition + User-Reauthorization-Flow.
- **Dependencies**: Android-Shell, Play-Store-Policy-Audit
- **Status**: Planned

### Microsoft Store / Android-Side-Distribution-Policies

- **Source**: PRD L603 "Keine Store-Distribution Phase 1 (Microsoft Store Phase 2+, Play Store Phase 3+)"
- **Description**: Microsoft Store (Windows) und Play Store / F-Droid (Android) Distribution-Pipelines.
- **Dependencies**: Play-Store-Policy-Audit (für Android); Eigen-Scope für Microsoft Store
- **Status**: Planned

---

## Phase 4+ — MVP-Completion + Moat

**Phase-Goal** (ref Product-Brief §Phasenplan, PRD §Vision): Lizenz-System, Polished-Cleanup-Plugin (neu-gebaut), Onboarding-Flow, v1-Import-UI-Button, Settings-Polish, Vertikal-Nischen-Builds.

### Lizenz-System (HMAC + Trial + 30-Tage-Cache + 48h-Grace)

- **Source**: PRD L160 (`explicitlyOutOfScope`: "License system HMAC/Trial/Grace (Phase 4)"), Product-Brief §Scope MVP-enthalten License System
- **Description**: HMAC-Validation + Permanent + Trial + 30-Tage-Cache + 48h-Grace. Hardcoded + Obfuscated HMAC-Key im Binary (via `obfstr`-Crate). Lizenz-Cache in SQLite-Settings-Tabelle mit HMAC-Signature.
- **Dependencies**: Keine (Cross-Platform funktionierend)
- **Status**: Planned

### OS-Keystore als Release-Default

- **Source**: PRD L159 (`explicitlyOutOfScope`: "OS-Keystore as release default (Phase 4)"), Architecture §2 :247. Phase-Placement-Widerspruch 2026-04-21 zugunsten PRD aufgelöst (Andy-Call).
- **Description**: Plain-SQLite-KeyStore (`dev-plain-keystore`-Feature, Phase 1) wird als Release-Default durch WindowsCredentialManager-Impl + Android-Keystore-Impl ersetzt. OS-Keystore-Impl existiert seit Phase-0-Gate-Closure; Swap ist Feature-Gate-Toggle + Migration-Runner.
- **Dependencies**: Migration-Tooling
- **Status**: Planned

### Polished-Cleanup-Plugin (neu-gebaut, nicht v1-Port)

- **Source**: PRD L161 (`explicitlyOutOfScope`: "Polished cleanup plugin (Phase 4 — new-build, not v1-port)"), `memory/feedback_polished_designschwaeche`, Brief §Cleanup-Styles
- **Description**: Polished-Stil komplett neu konzipiert: "Filler weg, Grammatik korrekt, aber Stimme bleibt" — nicht "professionell umformuliert" wie v1. Neues Crate `klarvo-plugin-polished`. Explicit: v1-Polished nicht portieren (macht zu viel kaputt).
- **Dependencies**: Keine
- **Status**: Planned

### Onboarding-Flow

- **Source**: PRD L157 (`explicitlyOutOfScope`: "Onboarding flow (Phase 4)"), Brief §Onboarding-Default
- **Description**: Fresh-Install-Flow: Cloud-First Groq + DeepSeek als Default-Stack, BYOK-Schritt integriert, Ziel "Erstes erfolgreiches Diktat in < 2 min".
- **Dependencies**: Settings-Panel (Phase 2), Lizenz-System (für Trial-Start-Flow)
- **Status**: Planned

### v1-Import-UI-Button

- **Source**: PRD L158 (`explicitlyOutOfScope`: "v1-Import UI button (Phase 4)")
- **Description**: UI-Button im Onboarding bzw. Settings-Panel, der Migration (History + Dictionary + API-Keys + Hotkey-Config) in einem Klick triggert. Writer + CLI-Stub existieren seit Phase-1 (ADR-0004); UI-Wiring ist Phase-4.
- **Dependencies**: Onboarding-Flow
- **Status**: Planned

### Chat-Cleanup-Plugin

- **Source**: PRD L162 (`explicitlyOutOfScope`: "Chat cleanup plugin (later; only verbatim in Phase 1)")
- **Description**: Chat-Stil bleibt wie in v1 — reine Port-Story. Kein Re-Design wie bei Polished.
- **Dependencies**: Keine
- **Status**: Planned

### Vertikale Domain-Builds (Medical / Legal / Editorial / Accessibility)

- **Source**: PRD §Deferred-to-Later-Phases Phase-4+, Brief §Vision, Distillate §Go-to-Market
- **Description**: Cargo-Feature-Variants für Nischen-Märkte. Jede Variante mit eigener Plugin-Set, Dictionary, LLM-Endpoint-Config. Moat-Strategy gegen horizontale Wettbewerber (siehe Brief §Was Klarvo unterscheidet Punkt 1).
- **Dependencies**: Trait-Surface-Stability-Pass (Phase 2 validiert), Plugin-Author-Guide, konkrete Nischen-Markt-Identifikation (siehe "Konkrete Nischen-Markt-Ideen" unten)
- **Status**: Planned

### Windows-Shell-Extension (Explorer-Context-Menu)

- **Source**: Architecture L590 "Windows-Shell-Extension (z. B. Explorer-Context-Menu) → Phase 4+"
- **Description**: OS-Integration via Shell-Extension.
- **Dependencies**: Keine
- **Status**: Planned

### Lemon-Squeezy-Payment-Integration

- **Source**: Distillate §Lizenz-System "Lemon Squeezy (Payment-Integration) ist P1, nicht im MVP", Offene Fragen "Lemon-Squeezy-Integration-Timing: P1-Label ist grob"
- **Description**: Commercial-Tier-Payment via Lemon Squeezy. P1-Label aus Brief ist grob — welcher P1-Meilenstein löst es aus?
- **Dependencies**: Lizenz-System (Phase 4)
- **Status**: Blocked-by-Phase-Timing-Decision

---

## Post-MVP P1 (Early-Post-MVP, nach Phase 4)

### Auto-Turso-Sync

- **Source**: PRD L164 (`explicitlyOutOfScope`: "Turso cloud sync (P1 post-MVP)"), Architecture §2 :249 "Turso-Sync (P1): Batch alle 60s + On-Demand-Trigger"
- **Description**: Turso-basierter (libsql) Cross-Device-Sync für History + Settings. Whitelist-gesteuert.
- **Dependencies**: Auth-Flow für Turso, Conflict-Resolution-Policy
- **Status**: Planned

### OpenAI Whisper / LLM + Groq LLM

- **Source**: PRD §Post-MVP-P1, Brief §Scope P1
- **Description**: Zusätzliche STT + LLM-Provider neben Phase-1-Default (Groq Whisper + DeepSeek). Reine `SttProvider`- / `LlmProvider`-Impls.
- **Dependencies**: Keine
- **Status**: Planned

### Reformate (Email / Bullets / Summary)

- **Source**: PRD §Post-MVP-P1, Brief §Scope P1
- **Description**: Cleanup-Style-Varianten: Email-Format, Bullet-Points, Summary. Zusätzliche `CleanupStyle`-Plugins.
- **Dependencies**: Keine
- **Status**: Planned

### Whisper-Mode (Gain)

- **Source**: PRD §Post-MVP-P1
- **Description**: Audio-Gain-Adjustment für leise Umgebungen ("Whisper-Mode").
- **Dependencies**: Audio-Filter-Trait
- **Status**: Planned

### Stats-Panel + History-Search + Cost-Tracking

- **Source**: PRD §Post-MVP-P1
- **Description**: UI-Features. Cost-Tracking pro Provider (Kosten/Session + Summen).
- **Dependencies**: History-Panel (Phase 2), Settings-Panel
- **Status**: Planned

### Unlimitiertes Dictionary

- **Source**: PRD §Post-MVP-P1, Brief §Scope MVP-enthalten "Custom Dictionary (capped)"
- **Description**: Custom-Dictionary ohne Cap. Phase-1-MVP hat Cap (explizit).
- **Dependencies**: Keine
- **Status**: Planned

### Whisper-Model-Manager (small/medium)

- **Source**: PRD §Post-MVP-P1, Brief §Scope P1
- **Description**: Offline-Whisper-Modell-Download + Switching. Local-STT-Enablement.
- **Dependencies**: Local-Whisper-Enable (siehe "Local Whisper / Offline Mode" unten)
- **Status**: Planned

### Webhook-Integration

- **Source**: PRD §Post-MVP-P1, Brief §Vision Plugin-Ökosystem
- **Description**: OutputTarget-Plugin oder TextFilter-Plugin, das Dictation-Output an User-definiertes Webhook schickt.
- **Dependencies**: Keine
- **Status**: Planned

### Auto-Loop

- **Source**: PRD §Post-MVP-P1
- **Description**: Mehrere aufeinanderfolgende Dictations ohne Hotkey-Re-Trigger (Hotkey-Hold/Release-Loop).
- **Dependencies**: Orchestrator-State-Machine erweitert
- **Status**: Planned

### UI-Scale

- **Source**: PRD §Post-MVP-P1
- **Description**: Accessibility-Feature — skalierbare UI-Elemente (Pill Bar, Bubble).
- **Dependencies**: Keine
- **Status**: Planned

### Hot-Reload-Providers

- **Source**: PRD §Post-MVP-P1, Architecture §1 :236 "Plugin-Registration: Manuell in klarvo-core::PluginRegistry::bootstrap() via Cargo-Feature-gated cfg-Module"
- **Description**: Plugin-Re-Registration ohne App-Neustart. Post-MVP-Feature.
- **Dependencies**: Plugin-Lifecycle-Hooks im Core
- **Status**: Planned

### Local Whisper / Offline Mode

- **Source**: PRD L165 (`explicitlyOutOfScope`: "Local Whisper / offline mode (P1/P2)"), Brief §Scope P1/P2
- **Description**: Offline-Pfad: `whisper-rs` 0.16 + lokale Modelle. Infrastruktur via `SttProvider`-Trait bereits vorhanden.
- **Dependencies**: Whisper-Model-Manager
- **Status**: Planned

---

## Post-MVP P2 (Power-Features, später als P1)

### Anthropic + OpenRouter Provider

- **Source**: PRD §Post-MVP-P2, Brief §Scope P2
- **Description**: `LlmProvider`-Plugins für Anthropic und OpenRouter.
- **Dependencies**: Keine
- **Status**: Planned

### Provider-Model-Overrides

- **Source**: PRD §Post-MVP-P2, Brief §Scope P2
- **Description**: Model-ID-Override pro Provider (z. B. Groq Llama-70B statt Whisper-Large-v3).
- **Dependencies**: Settings-Panel
- **Status**: Planned

### Custom Prompts

- **Source**: PRD §Post-MVP-P2, Brief §Scope P2
- **Description**: User-definierte Cleanup-Prompts (Template-Overrides).
- **Dependencies**: Settings-Panel
- **Status**: Planned

### App-Profiles

- **Source**: PRD §Post-MVP-P2, Brief §Scope P2
- **Description**: Pro Target-App (Slack, VSCode, Notion, Mail) eigene Settings-Profile (anderer Cleanup-Style / Output-Language).
- **Dependencies**: Settings-Panel
- **Status**: Planned

### Command-Mode / Voice-Commands

- **Source**: PRD §Post-MVP-P2, Brief §Scope P2 + DEFER-Note "Voice-Commands in v1-Form — werden als natives Plugin neu konzipiert"
- **Description**: `VoiceCommandHandler`-Trait (Phase 0 Stub) bekommt Impl. Command-Hotkey-Slot (siehe "Second Hotkey-Slot") für Command-Mode.
- **Dependencies**: Second Hotkey-Slot, Trait-Stability-Pass
- **Status**: Planned

### Voice-Notes

- **Source**: PRD §Post-MVP-P2, Brief §Scope P2
- **Description**: Längere ungetrimmte Audio-Captures als eigenes Output-Ziel (Memo-Style).
- **Dependencies**: Keine
- **Status**: Planned

### Snippets

- **Source**: PRD §Post-MVP-P2
- **Description**: Textbausteine via Voice-Command.
- **Dependencies**: Voice-Commands
- **Status**: Planned

### Filler-Word-Analysis

- **Source**: PRD §Post-MVP-P2
- **Description**: Statistik über Filler-Wort-Usage (Coaching-Feature).
- **Dependencies**: Stats-Panel
- **Status**: Planned

### Local Whisper Large + GPU/CUDA

- **Source**: PRD §Post-MVP-P2, Architecture §6 :301 "GPU-Support (Windows): MVP/P1: CPU-only. CUDA via Cargo-Feature gpu-cuda in P2"
- **Description**: Large-Modell + GPU-Acceleration via CUDA-Feature.
- **Dependencies**: Whisper-Model-Manager, `gpu-cuda`-Cargo-Feature-Implementierung
- **Status**: Planned

### Alle Threshold-Configs

- **Source**: PRD §Post-MVP-P2
- **Description**: Fine-Tuning-Configs (RMS-Thresholds, Silence-Detection-Parameters, etc.) als User-Settings statt harte Defaults.
- **Dependencies**: Settings-Panel
- **Status**: Planned

### Offline-LLM

- **Source**: Architecture §6 :300 "Offline-LLM: Deferred auf P1/P2-Entscheidung. Vorläufige Präferenz: mistral.rs vs. llama-cpp-2"
- **Description**: Offline-LLM-Provider-Impl, `LlmProvider`-Trait bereits vorhanden. Library-Choice offen.
- **Dependencies**: P1/P2-Eval-Entscheidung (mistral.rs vs. llama-cpp-2)
- **Status**: Blocked-by-Eval-Decision

---

## Vision / 3-Jahre-Horizont (Brief §Vision)

Keine Story-Items, sondern Orientierungsrahmen:

- iOS-Shell (nach Android)
- macOS-Shell (nach iOS)
- Linux opportunistisch (kein explizites Ziel)
- Vollständig lebendiges Plugin-Ökosystem (First-Party für Notion, Todoist, Obsidian, Slack; WASM-basierte Third-Party)
- Accessibility-Leadership in RSI/Motor-Einschränkungs-Community

---

## Open Questions (aus Brief, nicht Phase-scoped)

Items, die Entscheidungs-Workflows brauchen, bevor sie Backlog-Items werden:

- **Konkrete Nischen-Markt-Ideen** (Brief §Offene Fragen): Andy hat Ideen, nicht im Brief verortet. Separate Arbeitsstränge.
- **Beta-Testing-Plan für Klarvo 1.0** (Brief §Offene Fragen): Soll es eine Beta-Phase geben, oder direkt Release? Keine Entscheidung.
- **Erste zahlende Nutzer** (Brief §Offene Fragen): Konkrete Quellen/Kanäle noch nicht definiert.
- **Team/Agent-Setup** (Brief §Offene Fragen): Bewusst nach Phase 0 vertagt — in Klarvo-v2-Kontext: revisit wenn Phase-2-Start ansteht.
- **Lemon-Squeezy-Integration-Timing** (Brief §Offene Fragen): P1-Label grob. Welcher P1-Meilenstein löst aus?
- **Hotkey-Slot-Skalierung-Trigger** (Brief §Offene Fragen): MVP hat 2, skaliert auf 4–5 — Trigger-Bedingung offen.

---

## Revision-Log

- **2026-04-21:** Bootstrap aus Phase-1-Closure-Review. Consolidation aus PRD-Frontmatter `explicitlyOutOfScope` (13 Items), Product-Brief Prose (P1/P2/DEFER-Listen), Distillate Phase-Definitionen, Architecture "Deferred"-Vermerke. Plus drei Review-Addenda: Audio-Cpal Precision & Correctness Hardening, Tray-Icon Extensions, Floating Pill Bar (UX-Spec-TODO).
- **2026-04-21 (Post-Review-Follow-up):** OS-Keystore Phase-Placement zugunsten PRD Phase 4 aufgelöst (Andy-Call). Audio-Cpal-Item ergänzt um AC-3 Safety-Comment-Accuracy-Fix (Reviewer-Self-Finding).
- **2026-04-21:** Audio-Capture-Config-Overrides added (Phase 2) — Source: Story 3.7 Technical-Notes. Welle-3-Review-Decision (Reviewer-approval).
