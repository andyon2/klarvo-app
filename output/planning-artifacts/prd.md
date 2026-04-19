---
stepsCompleted: ['step-01-init', 'step-02-discovery', 'step-02b-vision', 'step-02c-executive-summary', 'step-03-success', 'step-04-journeys', 'step-05-domain', 'step-06-innovation', 'step-07-project-type', 'step-08-scoping', 'step-09-functional', 'step-10-nonfunctional', 'step-11-polish', 'step-12-complete']
status: complete
completedAt: '2026-04-19'
inputDocuments:
  - output/planning-artifacts/product-brief-klarvo.md
  - output/planning-artifacts/product-brief-klarvo-distillate.md
  - output/planning-artifacts/architecture.md
  - docs/index.md
  - docs/project-overview.md
  - docs/rebuild-discussion.md
  - docs/adr/README.md
  - docs/adr/0001-vad-provider-trait-signature.md
  - docs/adr/0002-tauri-specta-2-rc-acceptance.md
  - docs/adr/0003-jni-spike-outcome.md
  - docs/adr/0004-v1-to-v2-migration-strategy.md
  - docs/migration/v1-to-v2.md
  - memory/MEMORY.md
  - memory/feedback_polished_designschwaeche.md
  - memory/project_i18n_three_axes.md
  - memory/project_i18n_core_contract.md
  - memory/project_api_key_os_keystore_mvp.md
  - memory/project_no_remote_telemetry.md
  - memory/project_market_positioning.md
  - memory/project_klarvo_v2_rebuild.md
documentCounts:
  briefCount: 2
  researchCount: 0
  brainstormingCount: 1
  projectDocsCount: 10
workflowType: 'prd'
scopePhase: 'phase-1'
classification:
  projectTypePrimary: 'developer_tool'
  projectTypePrimaryRationale: 'Bulk der Phase-1-Arbeit ist Library-API + Plugin-Contracts (klarvo-core, Plugin-Traits, Pipeline-Executor, Manifest-as-Compile-Contract). Shell ist Konsument, nicht Kern. Verhindert v1-Failure-Mode (desktop_app-Thinking: UI-Features vor Core-Kontrakten → 71 Features aufgeblasen, 85% Android-Bypass).'
  projectTypeSecondary: 'desktop_app'
  projectTypeSecondaryRationale: 'Windows-Shell als erster Konsument (Tray-Icon, Hotkey-Capture, Auto-Paste via win32). Explizit dünn gehalten — Shell-Surface ist Adapter-Layer, kein Feature-Set.'
  domain: 'developer_productivity'
  domainRationale: 'Custom string. BYOK + Local-Only + Privacy-First-Positioning + Plugin-Extensibility adressieren Power-User/Dev-Persona direkt. Privacy/BYOK gehören in NFR+Compliance, NICHT in Domain — sauberer Split. Accessibility/RSI ist Secondary-Persona, nicht Domain.'
  complexity: 'high'
  complexityDeliveryScopeNote: 'thin-slice walking-skeleton'
  complexityRationale: 'Architektur-Substrat ist high: JNI-Dual-Surface (uniffi Control-Plane + raw jni Data-Plane), Trait-Pipeline-Executor mit Manifest-as-Compile-Contract, 3-Achsen-i18n-Separation (ui/dict/output_language), ts_ms-Konvention session-relative monotone, OS-Keystore-Abstraktion (Impl-Swap ab Phase 4), tauri-specta rc.24 Event-Kontrakte mit Drift-Gate, real-time Audio-Pipeline mit VAD+RMS. Scope ist medium (1 Plattform, 1 Mode, 1 Hotkey, 1 Nutzer), aber diese Architektur-Entscheidungen lassen sich nachträglich nicht billig nachziehen — falsches Medium-Labeling würde Test-Strategy, Gate-Kriterien und PRD-Priorisierung miskalibrieren.'
  projectContext: 'greenfield'
  projectContextAnnotation: 'Single Brownfield-Touchpoint: v1→v2-Import (Writer + CLI-Invocation) als isolierter Epic. v1-Code lebt in workspace-excludes (src/, src-tauri/, android/ NICHT Teil des v2-Workspace), v2 baut neu in crates/klarvo-core/, shells/windows/, plugins/*. Projektweite Brownfield-Haltung wird explizit abgelehnt (würde jeder Story unnötigen Kompatibilitäts-Overhead vererben).'
personaTiering:
  phase1Target:
    tier: 'dogfooding-prototype'
    audience: 'Andy + 1-2 interne Sanity-Tester'
    acceptedFriction: 'Kein Onboarding, config.toml-only Konfiguration, toleriert Rough-Edges, kennt Rust-Build-Toolchain'
    prdImplication: 'PRD schreibt für diese Persona. User-Acquisition-, Onboarding-, Metrics-Sections sind Phase-1-Overkill — explizit als deferred-to-Phase-2 markieren.'
  phase2And3Target:
    tier: 'validation-persona'
    audience: 'Power-User extern (Android-Phase 3 inkl.)'
    requirements: 'Stabile Plugin-API, Doku, minimales Settings-UI, Onboarding-Flow, Pill-Bar/Bubble UX'
    prdImplication: 'NICHT Ziel dieses PRDs. In späterem Phase-2/3-PRD addressieren.'
  phase4Target:
    tier: 'moat-persona'
    audience: 'Vertical-Niche-User (Medical/Legal/Accessibility/Editorial via Cargo-Feature-Variants)'
    requirements: 'Domain-Dictionary-Format, Feature-Gate-Granularität, Niche-Plugin-APIs'
    prdImplication: 'NICHT Ziel dieses PRDs. Phase-4+-PRD oder separates Moat-PRD.'
sectionOverrides:
  drop:
    - section: 'platform_support'
      reason: 'Windows-only bis Phase 3; Cross-Platform-Support ist Phase-3+-Thema'
    - section: 'update_strategy'
      reason: 'Zip-Replace reicht für Dogfooding-Prototype; Auto-Updater ist P1-Thema nach MVP'
    - section: 'offline_capabilities'
      reason: 'BYOK-Cloud-ASR (Groq) ist Phase-1-Default; Offline/Local-Whisper ist Nicht-Ziel Phase 1, P1/P2-Thema'
  add:
    - section: 'core_library_contract'
      content: 'Plugin-Traits-Definitionen, Pipeline-Manifest-TOML-Schema, Event-Bus, Manifest-Parser; Compile-Contracts enforced durch Executor (hart erroren auf unbekannte Stage-Types — ref memory/feedback_manifest_compile_contract.md)'
    - section: 'plugin_contract_stories'
      content: 'klarvo-plugin-groq + klarvo-plugin-verbatim sind Trait-Impls gegen Core-API. Story-AC muss Trait-Compliance + Headless-Core-Test enthalten, nicht UI-Verifikation.'
    - section: 'shell_surface_thin'
      content: 'Windows-Shell ist Adapter-Layer (Tray-Icon, Hotkey-Capture, Clipboard-Paste). Stories hier sind Adapter-Stories, nicht Feature-Stories.'
    - section: 'headless_testability'
      content: 'Jede Core-Story braucht AC: „läuft in headless integration test ohne Shell". Keine UI-only-validierbaren Features in Phase 1.'
    - section: 'bindings_drift_gate'
      content: 'Shell konsumiert tauri-specta rc.24 generierte Bindings. Story-AC muss Drift-Gate referenzieren (Phase-0-Gate aus architecture.md, ADR-0002).'
competitiveContext:
  purpose: 'Market-Context-Block für PRD-Intro (nicht Teil Classification, aber Pflicht-Input für Step-02b Vision)'
  cloudDictationPremium:
    competitors: ['Wispr Flow', 'Superwhisper', 'MacWhisper']
    positioning: 'Besetzen Cloud-Dictation-Premium mit OpenAI/Groq-Keys. Klarvos BYOK ist Counter-Positioning gegen Vendor-Lock-in, nicht Feature-Parität.'
  legalMedicalVerticals:
    competitors: ['Dragon Professional', 'Philips SpeechLive']
    positioning: 'Besetzen Legal/Medical proprietär, teuer, windows-only. Klarvos Cargo-Feature-Strategie ist gegen diese Inkumbenten gerichtet — erklärt Architektur-Überinvestition (10 Plugin-Traits im Walking-Skeleton).'
  accessibilityRsi:
    competitor: 'Talon Voice'
    positioning: 'Talon = Voice-Coding-DSL (Programmierung per Stimme). Klarvo = Dictation-first mit Accessibility-Respekt. Abgrenzung wichtig, damit RSI-Persona im PRD nicht weichgespült wird.'
visionInsights:
  phase1Framing: 'Substrat-Validierung für Phasen 2-4. Nicht Mini-MVP, sondern thin-slice-Beweis dass v2-Architektur unter realer Nutzung trägt.'
  coreDifferentiatorPrinciple: |
    v2 baut Failure-Modes in die Architektur selbst, nicht in die Disziplin der Contributors.
    CI-Gates, headless ACs, Manifest-as-Compile-Contract, Strict-Error-on-Unknown-Stage, Bindings-Drift-Gate sind alle Manifestationen desselben Satzes: wenn Disziplin slipped, bricht Kompilation/Test. Mechanisch erzwungene Invariante, nicht freiwillige Policy.
    (CI-Gates sind eins von vier Instrumenten, nicht DAS Instrument. Specs-first-Ergebnis ist Symptom, nicht Prinzip.)
  successAnchorsPhase1:
    principle: 'Architektur-seitige Messlatte, nicht verhaltensseitig. Dogfooding-Disziplin sagt nichts über Substrat-Tragfähigkeit.'
    rejectedAnchor: '14-30 Tage ohne v1-Rückfall (misst Dogfooding-Disziplin, nicht Substrat)'
    composite: 'Alle drei Anker müssen grün sein'
    anchor1TraitStability: |
      Zwischen Phase-1-MVP und Phase-1-End keine breaking changes an den publizierten Plugin-Traits (SttProvider, CleanupStyle, VadProvider, PipelineStage).
      Validierungs-Test: Wenn ein 2. STT-Plugin (z.B. Deepgram) in Phase 2 als reiner Trait-Impl einhängbar ist ohne Core-Trait-Änderung → Substrat trägt.
    anchor2HeadlessCoverage: |
      Jede Phase-1-Story hat einen grünen headless integration test.
      Test-Suite stays green für gesamte Dogfooding-Dauer.
      Production-Bugs müssen reproduzierbar headless sein, bevor Fix gemerged wird — nicht "im UI gefixt, läuft wieder".
    anchor3CiGateLoadBearing: |
      Die 5 Phase-0-Gates + Phase-1-Extensions (manifest-strict, bindings-drift, lint-events, i18n-no-user-strings-in-core) feuern während Phase 1 mindestens einmal echt (nicht nur bei Grün-Commits).
      Wenn kein Gate in 3+ Wochen Use je gefired hat → Gates sind Ceremonial, keine echten Contracts.
    qualitativeSecondary: 'Andy nutzt es für produktive Arbeit, nicht nur für Tests. (Ohne Tag-Zahl — nicht härtet, nur Qualitäts-Ergänzung.)'
  phase1ExperiencePolicy:
    framing: 'Bewusst aesthetic-free. Phase 1 darf sich rough anfühlen — das ist der Punkt.'
    noDemoMoment: 'Kein 90-Sekunden-Mail-Draft-Zielbild, kein UX-Narrative. Würde Step-3 Draft-Details automatisch UX-Polish-Stories reinziehen, die explizit in Scope-Lock ausgeschlossen sind.'
    canonicalReferenceWorkflow:
      purpose: 'Integration-Test-Anker, NICHT Demo'
      flow: 'Hotkey → Groq STT → Verbatim-Cleanup → Auto-Paste'
      targetApps: ['Notion-Web (Browser)', 'VSCode', 'Windows-Mail-Client']
      role: 'Acceptance-Anker für Pipeline-E2E-Story, keine UX-Vision'
    latencyCriteria: 'Kommen aus VAD-Split-Gate + NFR-Sections, nicht aus Vision'
  strategicContextBlockInPrd:
    include: true
    sectionTitle: 'Strategic Context (v2-Vision — not Phase-1 Scope)'
    clearlyFlaggedAsNotScope: true
    content:
      - 'Superwhisper-für-den-Rest-der-Welt'
      - 'Vertikale-Nischen-Moat via Cargo-Feature-Variants'
      - 'BYOK-als-Counter-Positioning gegen Vendor-Lock-in-Cloud-Dictation'
    purposeInPrd: 'Kalibrierungsreferenz für zukünftige Reader (Andy in Phase 2-4, Delegate-Agents, externe Leser). Macht nachvollziehbar, warum bestimmte Phase-1-Entscheidungen überproportional hart aufschlagen — z.B. Plugin-Trait-Surface-Stabilität (Voraussetzung für Vertical-Feature-Variants) oder Manifest-as-Compile-Contract (Voraussetzung für stable-extension-boundary in Phase 2+).'
    framingForReader: 'Diese Entscheidung kostet Phase-1-Einfachheit, zahlt aber in Phase-4-Strategie ein.'
    scopeGuardrail: 'Alle Items in diesem Block sind v2-Gesamt-Vision, nicht Phase-1-Scope. Phase-1-Stories, -ACs und -Success-Anker beziehen sich ausschließlich auf Substrat-Validierung.'
scopeLock:
  intent: 'Dev-internal Walking Skeleton for Andy + 1-2 sanity testers; NOT end-user-ready'
  hotkeySlots: 1
  recordingModes: ['hold-to-talk']
  hotkeyDefault: 'CommandOrControl+Shift+Space (placeholder, user-configurable from Phase 2)'
  hotkeyForbidden: ['CapsLock']
  hotkeySource: 'config.toml'
  apiKeyStorage: 'PlainSqliteKeyStore behind dev-plain-keystore Cargo feature (no OS-Keystore in Phase 1 release build)'
  pillBar: 'none (tray icon only, shows recording/idle)'
  settingsUI: 'none (config.toml only)'
  onboarding: 'none (Phase 4)'
  v1Migration: 'writer + CLI stub only (no UI button, that is Phase 4)'
  cleanupDefault: 'verbatim (single plugin; chat + polished are later phases)'
  i18nAxes:
    ui_language: 'config.toml key, default de'
    dictionary_language: 'config.toml key, default de'
    output_language: 'config.toml key, default de'
  i18nShellContract: 'User-strings (tray tooltips, error toasts, system notifications) resolved via key-lookup against shells/windows/src/locales/de.json — NEVER hardcoded. Single-locale bundle is fine for Phase 1, but key-lookup infrastructure must exist so Phase 2 is not a refactor. Ref memory/project_i18n_core_contract.md'
  telemetry: 'none (tracing to rolling local file only; no Sentry; ref memory/project_no_remote_telemetry.md)'
  targetAudience: 'Andy self + 1-2 internal sanity testers; NO public release artifact, NO license gating, NO Android'
  explicitlyOutOfScope:
    - 'Toggle / AutoStop / Wait-and-Type recording modes (Phase 2)'
    - 'Second hotkey slot (Phase 2)'
    - 'Floating Pill Bar with waveform/drag/shapes (Phase 2)'
    - 'Settings UI (Phase 2)'
    - 'Onboarding flow (Phase 4)'
    - 'v1-Import UI button (Phase 4)'
    - 'OS-Keystore as release default (Phase 4)'
    - 'License system HMAC/Trial/Grace (Phase 4)'
    - 'Polished cleanup plugin (Phase 4 — new-build, not v1-port)'
    - 'Chat cleanup plugin (later; only verbatim in Phase 1)'
    - 'Android shell (Phase 3)'
    - 'Turso cloud sync (P1 post-MVP)'
    - 'Local Whisper / offline mode (P1/P2)'
---

# Product Requirements Document - klarvo

**Author:** Andy
**Date:** 2026-04-19

**Scope:** Phase 1 only — "first successful dictation on Windows" walking skeleton. See `scopeLock` in frontmatter for exact boundary. Anything not enumerated in `scopeLock` is either out-of-scope or pending step-2+ discussion.

## Executive Summary

Dieses PRD definiert **Phase 1 des Klarvo-v2-Rebuilds** — ein Dev-internes Walking Skeleton, das die Tragfähigkeit der in Phase 0 festgezurrten v2-Architektur (Shared Rust Core, Plugin-Traits, Pipeline-Manifest, Native Shells) unter realer Nutzung validiert, bevor Phasen 2–4 auf dieses Substrat investiert werden.

**Zielgruppe Phase 1:** Andy (Rebuild-Initiator) + 1–2 interne Sanity-Tester. Explizit **keine Endnutzer**, kein öffentlicher Release. Konfiguration läuft ausschließlich über `config.toml`; UI-Polish (Settings-Panel, Floating Pill Bar, Onboarding, Lizenz-System) ist Phase-2+-Arbeit.

**Das gelöste Problem (Phase-1-spezifisch):** Klarvo v1 ist architektonisch an Framework-Mismatch (Tauri+React für Multi-Plattform) und Disziplin-Erosion (~85 % Android-Bypass, ~2 000 LOC duplizierter Pipeline-Code, 71 organisch gewachsene Features) gescheitert. Phase 0 hat die neue Architektur konzeptionell + infrastrukturell gelegt (5 Gates grün, 2026-04-18). Phase 1 ist der erste Moment, in dem Architektur-Versprechen und tatsächliche End-to-End-Nutzung aufeinandertreffen. Ohne diese Validierung sind alle weiteren Phasen Spekulation.

**Umfang Phase 1 (thin-slice):** Windows-Tauri-Shell hostet eine End-to-End-Pipeline aus Audio-Capture → RMS-VAD → Groq-Whisper-STT → Verbatim-LLM-Cleanup → Auto-Paste → History-Save. Hold-to-talk auf einem konfigurierbaren Hotkey (`config.toml`). Zwei erste Plugin-Crates (`klarvo-plugin-groq`, `klarvo-plugin-verbatim`) implementieren die neu definierten Core-Traits. v1→v2-Datenmigration existiert als Writer + CLI-Subcommand; parse-only Bundle mit `SecretString`-Keys (ADR-0004); der UI-Button folgt in Phase 4.

### What Makes This Special

**Kernprinzip (v1/v2-Differenzierung):** *v2 baut Failure-Modes in die Architektur selbst, nicht in die Disziplin der Contributors.* CI-Gates, headless Acceptance-Criteria, Manifest-as-Compile-Contract, Strict-Error-on-Unknown-Stage und Bindings-Drift-Gate sind Manifestationen desselben Satzes: wenn Disziplin slipped, bricht Kompilation oder Test. Nicht eine freiwillige Policy, die man vergisst — eine mechanisch erzwungene Invariante.

**Konkrete Manifestationen in Phase 1:**
- **Manifest-as-Compile-Contract:** Der Pipeline-Executor bricht hart auf unbekannte Stage-Types. `warn!+skip` ist verboten — falsche Konfiguration bricht Boot.
- **Headless-Testability-Pflicht:** Jede Phase-1-Story hat ein AC „läuft in headless integration test ohne Shell". Production-Bugs müssen headless reproduzierbar sein, bevor Fix merged — kein „im UI gefixt, läuft wieder".
- **Bindings-Drift-Gate:** `cargo xtask generate-bindings && git diff --exit-code` in CI. TypeScript-Bindings driften mechanisch nicht mehr von Rust-Source (ADR-0002, tauri-specta rc.24).
- **i18n-Core-no-user-strings:** `klarvo-core` emittiert nur i18n-Keys. User-Strings in Core-Crates = CI-Reject. Shells übersetzen gegen `locales/<lang>.<ext>`.

**Abgrenzung zur v1-Vergangenheit:** v1 scheiterte nicht an einzelnen Features, sondern daran, dass „specs-first" und „keine Code-Duplikation" **Wunschzustände** waren statt erzwungene Eigenschaften. Phase 1 macht diesen Unterschied zur Substrat-Frage: die Architektur selbst verhindert Drift.

## Project Classification

- **Project Type (Primary):** `developer_tool` — Bulk der Phase-1-Arbeit liegt in Core-Library-Kontrakten (`crates/klarvo-core/`: Pipeline-Executor, Plugin-Traits, Manifest-Parser). Die Shell ist dünner Konsument der Core-API, nicht das Produkt.
- **Project Type (Secondary):** `desktop_app` — Windows-Tauri-Shell hostet Tray-Icon, Hotkey-Capture und Clipboard-Paste. Explizit als Adapter-Layer gehalten, kein Feature-Set.
- **Domain:** `developer_productivity` (custom) — BYOK, Local-Only, Plugin-Extensibility adressieren Power-User und modulare Entwickler direkt. Privacy und BYOK gehören in NFR+Compliance, nicht in Domain. Accessibility/RSI ist Secondary-Persona.
- **Complexity:** `high` mit Delivery-Scope-Note *thin-slice walking-skeleton*. Nicht medium: JNI-Dual-Surface (uniffi + raw jni), Trait-Pipeline-Executor, 3-Achsen-i18n, `ts_ms`-Konvention, OS-Keystore-Abstraktion und tauri-specta-Drift-Gate sind strukturelle Entscheidungen, die in Phase 1 bereits final dimensioniert werden müssen — Scope ist medium (1 Plattform, 1 Mode, 1 Nutzer), Architektur-Substrat ist high.
- **Project Context:** `greenfield` mit einer expliziten Brownfield-Annotation — v1 lebt in `workspace-excludes` (`src/`, `src-tauri/`, `android/` sind nicht Teil des v2-Workspace); v2 baut neu in `crates/klarvo-core/`, `shells/windows/`, `plugins/*`. Einziger Brownfield-Touchpoint ist der v1→v2-Migrations-Writer als isolierter Epic, nicht als projektweite Brownfield-Haltung.
- **Phase-1-Persona-Tier:** `dogfooding-prototype` — Andy + 1–2 Sanity-Tester. Kein Onboarding, `config.toml`-only, toleriert Rough-Edges. Validation-Persona (Phase 2/3) und Moat-Persona (Phase 4+) sind explizit **nicht** Ziel dieses PRDs.

## Strategic Context (v2-Vision — not Phase-1 Scope)

Dieser Block ist **Kalibrierungsreferenz für zukünftige Reader** (Andy selbst in Phase 2–4, Delegate-Agents, externe Leser). Er skizziert die v2-Gesamt-Vision, gegen die Phase-1-Entscheidungen kalibriert wurden — und macht nachvollziehbar, warum bestimmte Phase-1-Entscheidungen überproportional hart aufschlagen.

- **„Superwhisper für den Rest der Welt":** plattform-unabhängiges, privatsphäre-freundliches, hackbares Diktat für Windows + Android + später iOS/macOS. Drei Jahre nach Klarvo 1.0 die Standard-Antwort außerhalb des Apple-Ökosystems.
- **Vertikale-Nischen-Moat via Cargo-Feature-Variants:** Klarvo Medical, Legal, Editorial, Accessibility als echte Custom-Builds mit domänen-spezifischen Plugins und Dictionaries — horizontal nicht nachbaubar. Begründet Plugin-Trait-Surface-Stabilität als Phase-1-Pflicht, nicht nur Phase-4-Feature.
- **BYOK als Counter-Positioning:** gegenüber Cloud-Dictation-Premium-Vendoren (Wispr Flow / Superwhisper / MacWhisper) und proprietären Vertical-Incumbents (Dragon / Philips). BYOK ist Akzeptanz-Filter, nicht Hürde.

**Framing für den Reader:** Diese Vision kostet Phase-1-Einfachheit (Plugin-Trait-Surface-Stabilität, Manifest-as-Compile-Contract, 3-Achsen-i18n-Separation im Walking-Skeleton) und zahlt in Phase-4-Strategie ein (stable-extension-boundary, Vertical-Feature-Variants, Vendor-Independence).

**Scope-Guardrail:** Alle Items in diesem Block sind v2-Gesamt-Vision, nicht Phase-1-Scope. Phase-1-Stories, Akzeptanzkriterien und Success-Anker beziehen sich ausschließlich auf Substrat-Validierung wie in Executive Summary + Classification definiert.

## Success Criteria

### User Success

Phase 1 adressiert die `dogfooding-prototype`-Persona — Andy + 1–2 interne Sanity-Tester. „User Success" ist hier **qualitativ und Use-basiert**, keine Adoption-/Retention-Metriken.

- **Primärer User-Erfolg:** Andy nutzt Klarvo Phase 1 für produktive Arbeit (Mails, Code-Kommentare, Notion-Einträge) — nicht nur zum Testen der Pipeline. Kein Rückfall auf Microsoft Voice Typing oder v1 ist qualitatives Sekundärsignal, kein hartes Kriterium (Dogfooding-Disziplin misst nicht Substrat-Tragfähigkeit).
- **Sanity-Tester-Erfolg:** 1–2 interne Tester können die Groq+Verbatim-Pipeline anhand einer kurzen Setup-Doku (README-Abschnitt oder Äquivalent) via `config.toml` in Betrieb nehmen. Friction-Points beim Setup werden als Doku-Tasks zurückgespielt, nicht als Tester-Fails gewertet — die Metrik misst, ob der `config.toml`-only-Pfad tragfähig genug ist, dass er dokumentierbar ist; nicht, ob Tester ihn autonom navigieren können. Hand-Holding durch Andy ist im Phase-1-Modell ausdrücklich erlaubt.
- **Canonical Reference-Workflow grün:** Hotkey → Groq STT → Verbatim-Cleanup → Auto-Paste funktioniert end-to-end gegen drei reale Target-Apps: Notion-Web (Browser), VSCode, Windows-Mail-Client. Acceptance-Anker für die Pipeline-E2E-Story.

### Program Success

Phase 1 hat **keine Business-Metriken** im klassischen Sinn (kein Release, keine User-Acquisition, kein Revenue). Stattdessen: **Program-Success** — was muss Phase 1 für die Gesamt-Roadmap liefern, damit Phase 2 überhaupt starten kann?

- **Phase-Transition-Readiness:** Alle drei Architektur-Anker (siehe Technical Success) sind grün. Ohne grüne Anker gibt es keine Phase-2-Freigabe.
- **Architektur-Investitions-Validierung:** Die Phase-0-Investition in Cargo-Workspace, Core-Traits, JNI-Spike, xtask-Subcommands, Bindings-Drift-Gate, Manifest-as-Compile-Contract hat sich unter realer End-to-End-Nutzung bewährt. Wenn Phase 1 auf einer dieser Entscheidungen scheitert, wird **vor** Phase 2 per ADR korrigiert, nicht später unter Zeitdruck.
- **Regressions-Disziplin-Proof:** Feature-Entwicklung in Phase 1 fühlt sich nicht mehr an wie in v1 („Brand-Löschen"). Qualitatives Sekundärsignal, operationalisiert durch die drei harten Anker.

### Technical Success

Die drei zusammengesetzten Architektur-Anker aus der Vision-Phase — **alle drei müssen grün sein**:

**Anker 1 — Trait-Stability:**
Zwischen Phase-1-MVP und Phase-1-End keine breaking changes an den publizierten Plugin-Traits (`SttProvider`, `CleanupStyle`, `VadProvider`, `PipelineStage`). Validierungs-Test am Phase-2-Start: ein zweiter STT-Plugin (z. B. Deepgram) muss als reiner Trait-Impl einhängbar sein **ohne Core-Trait-Änderung**. Wenn das erfordert, einen Trait zu erweitern → Trait-Design war nicht stable genug → zurück ans Zeichenbrett.

**Anker 2 — Headless-Coverage:**
Jede Phase-1-Story hat einen grünen headless integration test gegen `klarvo-core` (ohne Shell). Die Test-Suite bleibt grün für die gesamte Dogfooding-Dauer. Production-Bugs müssen headless reproduzierbar sein, bevor Fix gemerged wird — explizit **kein** „im UI gefixt, läuft wieder".

**Anker 3 — CI-Gate-Load-Bearing-Signal:**
Die 5 Phase-0-Gates — JNI-Spike, verify-release, Bindings-Drift, v1→v2-Migration-Test, Plugin-Verbatim E2E — plus Phase-1-Extensions (`manifest-strict`, `bindings-drift`, `lint-events`, `i18n-no-user-strings-in-core`) feuern während der Phase-1-Dauer **mindestens einmal real** (nicht nur bei Grün-Commits). Wenn in 3+ Wochen aktiver Entwicklung kein Gate je rot geworden ist, sind die Gates Ceremonial statt echte Contracts — dann ist Gate-Design zu überprüfen, nicht die Entwicklung zu loben.

### Measurable Outcomes

- **Trait-Stability-Pass:** Zweiter STT-Plugin (Working-Name: `klarvo-plugin-deepgram`) in Phase 2 einhängbar als reiner Trait-Impl ohne Core-Trait-Änderung. Messpunkt: Phase-2-Start.
- **Headless-Coverage:** 100 % aller gemergeten Phase-1-Stories haben AC „headless integration test grün" dokumentiert und verifiziert.
- **CI-Gate-Fires:** ≥ 1 CI-Gate-Failure dokumentiert während Phase-1-Dauer (Proof, dass die Gates echte Contracts sind, nicht Dekoration).
- **Canonical Reference-Workflow:** E2E-Test Hotkey → Groq STT → Verbatim-Cleanup → Auto-Paste grün gegen alle drei Target-Apps (Notion-Web, VSCode, Windows-Mail-Client).
- **Latenz-Kriterien:** aus VAD-Split-Gate (ADR-0001) + NFR-Sektion (kommt Step 10). In Success-Criteria nicht dupliziert, nur referenziert.

## Product Scope

### MVP — Minimum Viable Product (= Phase-1-Scope)

Dieser Phase-1-MVP ist das **Walking Skeleton**, wie im Frontmatter `scopeLock` festgezurrt. Zusammenfassung:

- **Pipeline:** Windows-Tauri-Shell hostet Audio-Capture → RMS-VAD → Groq-Whisper-STT (via `klarvo-plugin-groq`) → Verbatim-LLM-Cleanup (via `klarvo-plugin-verbatim`) → Auto-Paste → History-Save.
- **Input:** 1 Hotkey (Hold-to-talk) über `config.toml` konfiguriert. Default-Placeholder `CommandOrControl+Shift+Space`. **Nicht** CapsLock.
- **Konfiguration:** `config.toml`-only — inklusive drei i18n-Achsen-Keys (`ui_language`, `dictionary_language`, `output_language`, alle default `de`).
- **API-Keys:** `PlainSqliteKeyStore` hinter `dev-plain-keystore` Cargo-Feature. OS-Keystore-Impl existiert aus Phase 0, wird aber erst in Phase 2+ Release-Default.
- **UI:** Tray-Icon mit Recording/Idle-Status. **Keine** Floating Pill Bar, **keine** Settings-UI, **kein** Onboarding.
- **i18n-Shell-Kontrakt:** User-Strings (Tray-Tooltips, Error-Toasts, System-Notifications) via Key-Lookup gegen `shells/windows/src/locales/de.json`. Single-Locale-Bundle reicht, aber Key-Lookup-Infrastruktur steht ab Phase 1.
- **v1→v2-Datenmigration:** Writer + `cargo xtask import-v1`-Subcommand (kein UI-Button). Bundle mit `SecretString`-Keys (ADR-0004).
- **Nicht-Telemetrie:** `tracing` + `tracing-subscriber` → Rolling-File im User-Data-Dir. Kein Sentry, kein Remote-Endpoint.

### Growth Features (Post-MVP = Phase 2–3)

Kommen **nach** Phase-1-Freigabe (alle drei Anker grün). Nicht Ziel dieses PRDs.

- **Phase 2 — Windows daily usable:** Toggle-Mode + AutoStop, 2. Hotkey-Slot, komplette Floating Pill Bar (Shape, Drag, Position, Waveform), minimales Settings-Panel, Return-Focus, StylePicker, History-Panel, zweiter STT-Plugin (Trait-Stability-Test), OS-Keystore als Release-Default.
- **Phase 3 — Android daily usable:** Android-Shell mit allen Bubble-Zuständen, Gesten, AccessibilityService, JNI-Bridge produktiv (uniffi Control-Plane + raw jni Data-Plane), Android-v1-Import.

### Vision (Future = Phase 4+ und Post-MVP)

Nicht Ziel dieses PRDs. Verweise Phase-Plan + Strategic-Context-Block.

- **Phase 4 — MVP komplett:** Lizenz-System (HMAC, Trial, 30-Tage-Cache, 48h-Grace), Polished-Cleanup-Plugin (neu gebaut, **nicht** aus v1 portiert), Onboarding-Flow, v1-Import-UI-Button, Settings-Polish.
- **Post-MVP P1:** Auto-Turso-Sync, OpenAI/Groq-LLM, Reformate (Email/Bullets/Summary), Stats-Panel, History-Search, Cost-Tracking, Whisper-Model-Manager, Autostart, Hot-Reload-Providers.
- **Post-MVP P2:** Anthropic, OpenRouter, Custom Prompts, App-Profiles, Command-Mode, Voice-Notes, Local-Whisper-Large + GPU/CUDA, alle Threshold-Configs.
- **Vision (3 Jahre):** Alle vier großen Plattformen (Windows, Android, iOS, macOS) ausgeliefert. Vertikale Nischen-Varianten (Klarvo Medical/Legal/Editorial/Accessibility) als echte Custom-Builds. WASM-Plugin-Layer für Third-Party-Extensions. Accessibility-Leadership in RSI/Motor-Einschränkungs-Community.

## User Journeys

**Scope-Note:** Dieses PRD adressiert ausschließlich den Phase-1-Persona-Tier `dogfooding-prototype` (Andy + 1–2 interne Sanity-Tester). Admin/Operations-, Support- und API-Consumer-Journeys sind für diese Persona nicht existent und werden in späteren Phase-PRDs (Phase 2/3/4) addressiert. Die vier unten beschriebenen Journeys decken die realen Phase-1-Interaktionsflächen vollständig ab.

---

### Journey 1 — Andy: Core Dictation Workflow (Happy Path)

**Szene:** Andy sitzt am Windows-Laptop, schreibt einen Mail-Entwurf in einem der drei canonical Target-Apps (Notion-Web im Browser, VSCode, Windows-Mail-Client). Statt zu tippen, drückt er und hält den konfigurierten Hotkey (`CommandOrControl+Shift+Space` aus `config.toml`).

**Ablauf (architektur-geerdet, nicht UX):**
1. **Hotkey-Capture:** Windows-Shell registriert Hold-Event, signalisiert `klarvo-core` via tauri-specta-Event (`hotkey.held`).
2. **Pipeline-Start:** Core liest Pipeline-Manifest (`pipeline.toml`, embedded-default via `include_str!()`), instanziiert Stages: `audio_capture` → `vad:rms` → `stt:groq` → `cleanup:verbatim` → `output:paste` → `history_save`.
3. **Recording:** `cpal`-Audio-Stream beginnt, RMS-VAD emittiert `is_speech: true`, Tray-Icon wechselt zu „Recording" via Shell-Key-Lookup (`de.json` → `tray.tooltip.recording`).
4. **Release:** Andy lässt Hotkey los, Shell sendet `hotkey.released`, Core schließt Audio-Stream.
5. **STT:** `klarvo-plugin-groq` POSTet Audio-Chunks an Groq-Whisper-API (API-Key aus `PlainSqliteKeyStore` hinter `dev-plain-keystore`-Feature). Core emittiert `ts_ms`-monotone Events (`stt.started` / `stt.completed`).
6. **Cleanup:** `klarvo-plugin-verbatim` rendert Prompt gegen DeepSeek-API (oder konfigurierten LLM-Endpoint), erhält cleaned Text. Output-Language aus `config.toml` wird in Prompt-Template injiziert.
7. **Paste:** Core delegiert an Shell via `output.paste_requested`-Event mit Text-Payload. Shell ruft `arboard` + `SendInput`-Paste-Sequence gegen fokussiertes Target-App.
8. **History:** Core schreibt Entry in SQLite (`history(id, text, raw_text, style, language, app_name, created_at, uuid, device_id)`).

**Erfolgs-Signal:** Text erscheint im Target-App-Editor, History-Entry persistiert, keine ERROR-Logs. Headless-Integration-Test reproduziert alle Schritte außer Paste (Shell-Adapter).

**Capabilities, die diese Journey verlangt:**
- `AudioSource`-Trait + RMS-VAD-Impl
- `SttProvider`-Trait + `klarvo-plugin-groq`-Impl
- `CleanupStyle`-Trait + `klarvo-plugin-verbatim`-Impl
- `OutputTarget`-Trait + Shell-Clipboard-Adapter
- Pipeline-Manifest-Parser mit Embedded-Default
- Event-Bus mit `ts_ms`-Konvention
- History-Storage (SQLite-Migration-Trait)
- Hotkey-Capture Shell-Adapter mit tauri-specta-Event-Kontrakt

---

### Journey 2 — Andy: Pipeline Failure Recovery (Edge Case)

**Szene:** Andy drückt Hotkey, spricht — aber der Pipeline-Flow bricht an einer der vier Architecture-Invarianten. Drei Failure-Modes, die Phase-1 explizit durchhalten muss.

**Failure-Mode A: Unknown Pipeline Stage.**
Andy hat `pipeline.toml` manuell editiert und einen Stage-Type `polished` eingetragen — aber `klarvo-plugin-polished` existiert in Phase 1 nicht (Polished-Plugin ist Phase 4). Der Pipeline-Executor **bricht hart** beim Boot mit Error `unknown_stage: polished (available: verbatim)`. `warn!+skip` ist verboten (`feedback_manifest_compile_contract`). Andy sieht Error-Toast via Shell-Key-Lookup (`de.json` → `error.pipeline.unknown_stage`), korrigiert `pipeline.toml`, restartet. Pipeline läuft.

**Failure-Mode B: Groq-API-Failure.**
Andy diktiert, Groq-API antwortet mit 503. `klarvo-plugin-groq` emittiert `AppError { kind: NetworkError, user_message: Some("error.network.offline"), retryable: true }`. Core propagiert Error zur Shell, Shell zeigt Toast (i18n-Key-Lookup), keine Paste. History-Entry wird nicht geschrieben. Andy wartet 30 Sekunden, wiederholt — funktioniert.

**Failure-Mode C: Keystore-Miss.**
Andy hat `dev-plain-keystore`-Feature aktiviert, aber vergessen, `groq_api_key` in der Keystore-SQLite zu persistieren. `klarvo-plugin-groq::init()` ruft `KeyStore::retrieve("groq_api_key")`, erhält `None`. Plugin-Registration scheitert, Core loggt `ERROR: plugin groq failed init: keystore_miss`, Pipeline-Executor rejected Manifest beim Boot. Andy sieht Error-Toast (`error.plugin.init_failed`), schaut ins Rolling-File-Log, führt `cargo xtask import-v1` aus oder setzt Key manuell. Neustart. Läuft.

**Erfolgs-Signal:** In allen drei Fällen bleibt die Test-Suite grün (der Failure-Mode ist als `#[test]`-Case reproduzierbar, nicht nur in Production). User-facing Error-Messages kommen aus Shell-i18n, nicht aus Core-Strings.

**Capabilities, die diese Journey verlangt:**
- Strict-Error-on-Unknown-Stage im Pipeline-Executor (kein `warn!+skip`)
- `AppError { kind, message, user_message: Option<I18nKey>, retryable }`-Schema
- i18n-Key-Lookup in Shell gegen `de.json`
- Rolling-File-Log mit `tracing` + `tracing-subscriber`
- Keystore-Miss-Handling im `init()`-Pfad jedes Plugins
- `AppError`-Propagation Core→Shell via tauri-specta-Event

---

### Journey 3 — Andy: Dev-Iteration Loop (Config-/Plugin-Change mit Headless-First)

**Szene:** Andy will einen neuen Verbatim-Prompt-Variant testen — er vermutet, dass der aktuelle Prompt zu viel Filler durchlässt. Er arbeitet im dogfooding-Modus: editiert, testet headless, erst dann dogfooded er live.

**Ablauf:**
1. **Branch öffnen:** Neuer Git-Branch `verbatim-prompt-v2`.
2. **Prompt editieren:** `plugins/klarvo-plugin-verbatim/src/prompt.rs` angepasst.
3. **Headless-Test zuerst:** `cargo test -p klarvo-plugin-verbatim --features test-fixtures` gegen `klarvo-test-fixtures`-Audio-Samples. Test grün → Architektur-Anker 2 erfüllt (Headless-Coverage).
4. **CI-Gate-Check:** `cargo xtask ci` lokal — `lint-events`, `manifest-strict`, `bindings-drift`, `i18n-no-user-strings-in-core` feuern. Alle grün.
5. **Dogfood:** Build, Start, Hotkey, diktieren. Andy beobachtet neuen Verbatim-Output in Notion-Web. Wenn nicht überzeugend: zurück zu Schritt 2.
6. **Merge:** Commit + PR + Merge zu `main`. CI feuert alle Phase-1-Gates erneut.

**Failure-Sub-Case (CI-Gate-Fire als Proof):** Andy vergisst, in seinem Event-Struct `#[tauri_specta(event_name)]` zu setzen (alte `#[specta(rename)]`-Syntax aus der rc.24-Event-Rename-Memory, `reference_tauri_specta_rc24_event_name`). `cargo xtask lint-events` in CI schlägt fehl, PR blockiert. Andy fixt den Rename, CI grün, Merge. **Genau diese Art Gate-Fire ist Architektur-Anker 3-Ziel** — Proof, dass die Gates echte Contracts sind.

**Erfolgs-Signal:** Iteration-Zyklus < 5 min zwischen Edit und Dogfood. Keine UI-only-Bugfixes. CI-Gates fangen Drift vor Merge.

**Capabilities, die diese Journey verlangt:**
- `cargo xtask ci`-Orchestrator mit allen Phase-0/1-Gates
- `klarvo-test-fixtures`-Crate mit Audio-Samples + Mock-Providers
- `lint-events`-Subcommand enforced tauri-specta-Event-Namen-Regel
- `manifest-strict`-Subcommand valididiert Pipeline-TOML gegen Plugin-Registry
- Bindings-Drift-Gate in CI (`generate-bindings && git diff --exit-code`)

---

### Journey 4 — Sanity-Tester: Setup mit Hand-Holding

**Szene:** Ein interner Tester (z. B. ein Bekannter aus Andy's Umfeld) erhält von Andy den `klarvo-v2`-Windows-Binary-Zip + einen kurzen README-Abschnitt „Phase-1-Setup". Tester ist tech-affin (kennt `config.toml` aus anderen Rust-Projekten), aber hat Klarvo noch nie benutzt.

**Ablauf:**
1. **Zip entpacken**, `klarvo.exe` + `pipeline.toml` + `config.toml.example` + `README-phase1.md`.
2. **README liest:** „(1) Kopiere `config.toml.example` zu `config.toml`. (2) Setze deine Groq-API-Key entweder via `cargo xtask import-v1` (wenn du Klarvo-v1 hattest) oder manuell via `sqlite3 keystore.db 'INSERT ...'`. (3) Setze deinen Hotkey in `[hotkey]` section. (4) Starte `klarvo.exe`."
3. **Friction-Point 1:** Tester weiß nicht, wo er den Groq-Key herbekommt. Er fragt Andy per Chat. Andy schickt Link zu Groq-Signup + Key-Creation-Flow. **Friction-Point-Response:** Andy fügt den Link zu `README-phase1.md` hinzu. Das ist eine Doku-Task, kein Tester-Fail — Phase-1-Persona-Tier hat Hand-Holding explizit erlaubt.
4. **Friction-Point 2:** Tester hat Hotkey-Konflikt mit OBS Studio auf `CommandOrControl+Shift+Space`. Er ändert zu `F9` in `config.toml`. Funktioniert.
5. **Erste Dictation:** Tester drückt `F9`, spricht „Hallo Welt", lässt los. Text erscheint in seinem Text-Editor. Tester meldet an Andy zurück: „läuft, aber `F9` war nicht im Default."
6. **Doku-Update:** Andy fügt zu README hinzu „Typische Hotkey-Alternativen, wenn `CommandOrControl+Shift+Space` belegt ist: `F9`, `F10`, `CommandOrControl+Alt+Space`."

**Erfolgs-Signal:** Friction-Points werden als Doku-Tasks zurückgespielt, nicht als Feature-Requests. Der `config.toml`-only-Pfad ist tragfähig genug, dass er dokumentierbar ist. Hand-Holding durch Andy ist **Modell**, nicht Umgehung.

**Capabilities, die diese Journey verlangt:**
- `config.toml.example`-Template im Release-Artefakt (verify-release-Gate)
- `README-phase1.md` mit schrittweiser Setup-Anleitung
- Klare Hotkey-Alternative-Syntax in `config.toml`
- `cargo xtask import-v1`-Subcommand für v1-Keystore-Migration
- (Implizit) Tester haben Direkt-Kontakt zu Andy — **keine Support-Infrastruktur nötig**

---

### Journey Requirements Summary

Die vier Phase-1-Journeys decken die folgenden Capability-Cluster ab. Jeder Cluster wird in Step-9 Functional Requirements und Step-10 NFRs konkret spezifiziert.

**Core-Library-Kontrakte:**
- Pipeline-Manifest-Parser mit Embedded-Default + User-Override-Loader
- Pipeline-Executor mit Strict-Error-on-Unknown-Stage
- 8 first-class Plugin-Traits (`SttProvider`, `LlmProvider`/`CleanupStyle`, `TextFilter`, `OutputTarget`, `AudioFilter`, `VadProvider`, `AudioSource`, `PluginMigration`) + `VoiceCommandHandler`-Stub
- `PluginRegistry::bootstrap()` mit `init()`-Contract pro Plugin
- `KeyStore`-Trait mit `PlainSqliteKeyStore`-Impl hinter `dev-plain-keystore`-Feature
- `AppError { kind, message, user_message: Option<I18nKey>, retryable }`-Schema
- Event-Bus mit `ts_ms`-Konvention (session-relative monotone)
- SQLite-History-Storage via Plugin-Migration-Trait

**Plugin-Crate-Kontrakte (Phase-1-Deliverables):**
- `klarvo-plugin-groq` — `SttProvider`-Impl mit Groq-Whisper-API
- `klarvo-plugin-verbatim` — `CleanupStyle`-Impl mit Prompt-Template + LLM-API-Call

**Shell-Surface (dünn, Windows):**
- Tray-Icon mit Recording/Idle-State + i18n-Key-Lookup gegen `de.json`
- Hotkey-Capture (Hold-Detection) mit tauri-specta-Event-Kontrakten
- Clipboard-Paste (`arboard` + `SendInput`)
- Error-Toast-UI mit i18n-Key-Lookup
- tauri-specta rc.24 Event-Name-Konvention mit `#[tauri_specta(event_name)]`

**Developer-Tooling (xtask):**

`cargo xtask ci`-Orchestrator als Umbrella-Command.

*Phase-0-etabliert (carried into Phase 1):*
- `generate-bindings` mit Git-Diff-Exit-Code-Gate (ADR-0002)
- `verify-release` für Release-Artefakt-Validierung
- `import-v1` für v1→v2-Bundle-Import (ADR-0004)

*Phase-1-neu:*
- `lint-events` — enforct tauri-specta-Event-Namen-Konvention
- `manifest-strict` — validiert Pipeline-TOML gegen Plugin-Registry
- `i18n-no-user-strings-in-core` — verhindert Core-Strings
- `test-core`, `test-fixtures` — Orchestrator für Headless-Suite

**Operational:**
- **Rolling-File-Log** in `%APPDATA%/klarvo/logs/` (max 10 MB, 5 Rotations) — Phase 1 definit, via `tracing` + `tracing-subscriber`. Dogfooding-Tester greifen direkt auf das Verzeichnis zu, kein UI-Trigger nötig.
- **User-triggered Debug-Export-Zip** (Logs + redacted Config + Sys-Info) via Settings-Panel — **Phase 2** (an Settings-UI gebunden). Export-Library-Code (`klarvo-core/src/telemetry/export.rs`) kann bereits Phase 1 stubbed existieren, falls Sentinel-Gate-Pattern aus `feedback_ci_gate_philosophy` sinnvoll — entschieden in Step 9: FR40 — `telemetry::export`-Module-Stub Phase 1.
- Panic-Hook in denselben Stream als `level=ERROR`
- `README-phase1.md` mit Setup-Anleitung + Hotkey-Alternative-Syntax
- `config.toml.example` im Release-Artefakt

**Explizit NICHT Capability in Phase 1:**
- Settings-UI, Pill Bar, Onboarding-Flow (Phase 2/4)
- Admin-/Operator-/Support-Interfaces (kein Persona-Tier)
- API-Consumer-Endpoints (keine externen Consumers)
- Update-Mechanismus (Zip-Replace reicht)
- Cross-Platform (Windows-only bis Phase 3)
- OS-Keystore als Release-Default (Phase 4)
- Lizenz-System (Phase 4)
- Cloud-Sync (P1 post-MVP)
- `cargo xtask set-key`-Subcommand zur Keystore-Key-Pflege über CLI (Phase-2-Candidate, falls Journey 4 als zu rough empfunden wird — aktuell bewusst hart gescoped auf `sqlite3`-Manual-Fallback)

**Amendment (Step 11 consistency correction, 2026-04-19):** Die ursprüngliche Auflistung von „8 first-class Plugin-Traits + `VoiceCommandHandler`-Stub" stammt aus dem `architecture.md`-Trait-Inventory und reflektiert den architektonischen Gesamtplan. Für Phase-1-Trait-Stability-Contract-Zwecke sind nur die vier Core-Traits `SttProvider`, `CleanupStyle`, `VadProvider` und `PipelineStage` load-bearing (siehe §Success Criteria.Anker-1, §Functional Requirements FR1–4, §Consolidated Digest.In-Scope). Die übrigen in `architecture.md` konzipierten Traits (`LlmProvider`, `TextFilter`, `OutputTarget`, `AudioFilter`, `AudioSource`, `PluginMigration`, `VoiceCommandHandler`) existieren als architektonische Erweiterungsfläche, sind aber **nicht** Phase-1-Stability-Anker — ihre Signaturen können Phase-2/3/4 evolvieren, ohne Phase-1-Success-Kriterien zu verletzen.

## Domain-Specific Requirements

### Skip-Rationale

Für den Phase-1-Persona-Tier `dogfooding-prototype` und die Custom-Domain `developer_productivity` greift keine externe Regulatorik (HIPAA, FDA, PCI-DSS, GDPR-Processor-Rolle, ITAR). Die BYOK-Architektur-Entscheidung (Brief + `memory/project_no_remote_telemetry.md`) führt dazu, dass Klarvo als Software-Distributor auftritt, nicht als Daten-Processor — API-Key-gestützte Cloud-Calls laufen direkt zwischen User-Gerät und User-gewähltem Provider (Groq, DeepSeek etc.).

Dev-Tool-spezifische Anforderungen (Compile-Contracts, Headless-Testability, Shell-Adapter-Thinness, Bindings-Drift-Gate) sind nicht hier, sondern via `classification.sectionOverrides.add` geroutet und werden in Step 9 Functional Requirements konkret spezifiziert. Privacy/BYOK/Local-Only/3-Achsen-i18n sind NFR-Stoff (Step 10).

### Forward-Looking Policy-Risks (deferred to later phases)

- **AccessibilityService-Policy (Phase-3-Blocker):** Google Play Store bewertet AccessibilityService-Usage seit 2024 verschärft. Klarvo-Android (Phase 3) nutzt AccessibilityService für Global-Hotkey + System-weites Paste — eine Policy-Audit gegen Play-Store-Richtlinien ist Pflicht vor Phase-3-Start, nicht erst am Release-Ende. Referenzen: `memory/project_play_store_phase3_blocker.md`, `memory/project_android_playstore_risk.md`. Konsequenz für Phase 1: keine, aber das Risiko ist hier verankert, damit es bei Phase-3-PRD-Start nicht neu entdeckt werden muss.
- **Windows-MIC-Permission (Phase 1):** OS-Level-Prompt beim ersten Audio-Capture-Zugriff — kein app-spezifischer Consent-Flow nötig. Betriebssystem übernimmt. Relevanz für Phase 1: `klarvo-plugin-groq` + Audio-Capture-Trait müssen MIC-permission-denied als `AppError::kind::PermissionDenied` behandeln, mit `user_message: "error.permission.microphone"`.
- **Zukünftige Domain-Varianten (Phase 4+ Moat):** Vertikale Nischen-Builds (Medical, Legal, Editorial) könnten domain-spezifische Regulatorik aktivieren (z. B. HIPAA-Processor-Rolle bei Medical-BYOK-Ausnahme, DPA-Pflichten bei EU-Legal-Einsatz). Diese werden in Phase-4-PRD adressiert, nicht hier. Vermerkt als Strategic-Context-Anker.

## Innovation & Novel Patterns

### Innovation Axis A: Manifest-as-Compile-Contract

**Detected Innovation:**
Audio-Pipeline als TOML-Manifest deklariert, beim `cargo build` hart gegen Rust-Types der Plugin-Traits aufgelöst. Executor erroriert auf unknown stage-types (nicht `warn!+skip`, explizite Entscheidung in `memory/feedback_manifest_compile_contract.md`). Gegenmodell zum Runtime-Plugin-Registry-Pattern (typisch für Dev-Tools mit Extension-Mechanismen). Referenz: `classification.sectionOverrides.add.core_library_contract`, `memory/project_plugin_architecture.md` (Trait-basiert compile-time, Pipeline-Manifest TOML).

**Market Context & Competitive Landscape:**
Dictation-Tools operieren entweder mit fixen Pipelines (Dragon) oder laufzeit-konfigurierbaren „Features" via Settings-UI (Wispr Flow, Superwhisper, MacWhisper) — keiner exposet Pipeline-Komposition als Compile-Contract. Extension-Mechanismen in Dev-Tool-Space generell (VS Code Extensions, Obsidian Plugins) nutzen Runtime-Registry mit Runtime-Failure-Modes. Manifest-as-Compile-Contract ist von Build-System-Patterns (Bazel-BUILD, Cargo-Workspace, Nix-flakes) inspiriert, aber angewandt auf Audio-Pipeline-Composition ist das ein eigenständiges Paradigm für die Dictation-Domäne.

**Validation Approach:**
(i) `cargo xtask lint-events` / Manifest-Parser-Gate: unknown stage-type in Manifest → `cargo build` schlägt fehl (CI-Gate, Phase-0-etabliert).
(ii) Type-Mismatch zwischen Stages (z. B. `AudioFrame`-Output → `Text`-Input-erwartender Next-Stage) → Executor-Compile-Error via Trait-Generics.
(iii) Integration-Test: Manifest-Parser-Roundtrip (parse → validate → bundle-check) als Headless-Test ohne Shell.

**Risk Mitigation:**
- **Risk:** Compile-Time-Safety verstärkt Plugin-Dev-Friction (Compile-Zyklus statt Hot-Reload). **Mitigation:** Phase-1-Persona `dogfooding-prototype` toleriert das explizit (`personaTiering.phase1Target.acceptedFriction`). WASM-Runtime-Plugins später geplant (`memory/project_plugin_architecture.md` „WASM später") — dann Runtime-Contract mit Schema-Validation am Load-Time, nicht Hot-Patchable.
- **Risk:** Manifest-Schema-Drift zwischen Releases bricht User-Manifests. **Mitigation:** Manifest-Schema-Versionierung im TOML-Header, Core validiert Version vor Resolution.

### Innovation Axis B: Headless-Core-Contract

**Detected Innovation:**
`klarvo-core` ist UI-frei — kein User-facing String im Core (Events emittieren i18n-Keys, nicht Text; Errors transportieren `kind` + `user_message`-Key). Shell (Windows-Tray) ist reiner Adapter: Hotkey-Capture + Clipboard-Paste + Bindings-Konsumtion. Jede Core-Story-AC enthält „läuft in headless integration test ohne Shell". CI-Gate G3 („core emits no user strings", `memory/project_phase0_action_items.md`) erzwingt das mechanisch. Referenz: `classification.sectionOverrides.add.headless_testability` + `.shell_surface_thin`, `memory/project_i18n_core_contract.md`.

**Market Context & Competitive Landscape:**
Dictation-Tool-Default ist UI-first: Wispr Flow, Superwhisper, MacWhisper, Dragon bauen Features direkt in die UI ein — kein separabler Core. Konsequenz: Cross-Platform-Portability ist teuer, was Klarvo v1 empirisch belegt hat (85% Android-IPC-Bypass, ~2000 LOC duplizierte Business-Logic zwischen Tauri-Shell und Android-Native-Pfad, `memory/project_android_bypass.md`). Headless-Core-Contract überträgt klassische Library-Dev-Hygiene (Unix-Pipe-Philosophie, „tools nicht apps", CLI-Tooling-Style à la ripgrep/fd) auf die Dictation-Domäne — ungewöhnlich für Audio/ASR-Produkte.

**Validation Approach:**
(i) **CI-Gate G3** (Phase-0-etabliert): `cargo xtask lint-events` scanned Core-Emissions (Event-Payloads, Error-User-Messages) auf User-Strings → CI fail bei Literal-Match außerhalb erlaubter i18n-Key-Shapes.
(ii) **Headless-Integration-Test-Suite:** jede Core-Story enthält mindestens ein Test-Case ohne Shell-Mount (AC-Template in `sectionOverrides.add.headless_testability`).
(iii) **Bindings-Drift-Gate** (`sectionOverrides.add.bindings_drift_gate`, ADR-0002): tauri-specta rc.24 generierte TS-Types werden in CI gegen Shell-Konsumtion gecheckt — Shell darf nicht divergieren.

**Risk Mitigation:**
- **Risk:** Shell-Agenten späterer Phasen (Phase 2/3/4) könnten Core-i18n-Keys duplizieren oder umgehen, Key-Drift zwischen Shell-Surfaces entsteht. **Mitigation:** Core emittiert i18n-KEYS, Shells übersetzen (`memory/project_i18n_core_contract.md`); G3-Gate erzwingt Core-Emission-Shape; 3-Achsen-i18n-Separation (`memory/project_i18n_three_axes.md`) verhindert Konflation von UI-Language mit Output-Language.
- **Risk:** Dogma-Drift — „headless everywhere" könnte UX-only-Konzepte in Phase 2+ (z. B. Pill-Bar-Animation-Timing) behindern. **Mitigation:** Phase-1-Scope-Lock verbietet UI-Features explizit; Phase-2-PRD kann Shell-only-Stories separat labelen, ohne Headless-Contract im Core aufzuweichen.

### Scope Note: BYOK-as-Filter-Positioning (deliberately not listed as innovation)

BYOK-als-Filter-Positionierung wird hier nicht als Innovation gelistet, sondern als strategische Positionierung im Strategic-Context-Block nach Classification geführt — siehe dort sowie `memory/project_market_positioning.md`, `memory/project_no_remote_telemetry.md`, `memory/project_api_key_os_keystore_mvp.md`. Innovation-Claims in diesem PRD beschränken sich auf technische + methodologische Novelty, nicht auf Positioning-Stances.

## Developer-Tool-Specific Requirements (Primary) + Desktop-App-Specific (Secondary)

### Language & Toolchain Matrix

- **Rust** (stable, MSRV aus `architecture.md`): Core-Crates (`klarvo-core`, `klarvo-plugin-*`), Shell-Backend (`shells/windows/`), xtask-Subcommands.
- **TypeScript** (via `tauri-specta` rc.24 generierte TS-Bindings, ADR-0002): Shell-Frontend (Windows-Tray-Renderer). TS wird generiert, nicht manuell gepflegt — Bindings-Drift-Gate erzwingt Einhaltung (`sectionOverrides.add.bindings_drift_gate`).
- **Plugin-Autoren** (Phase 1 = Andy): Rust-Build-Toolchain erforderlich (`personaTiering.phase1Target.acceptedFriction`). Kein JavaScript-/Python-/WASM-Plugin-Authoring Phase 1 (WASM-Runtime-Plugins deferred to Phase 2+, cf. `memory/project_plugin_architecture.md`).

### Installation & Distribution

- **Einziger Phase-1-Installation-Pfad:** `cargo build` aus Source. Kein Pre-Built-Binary, kein Windows-Installer (MSI/MSIX), kein Store-Release.
- **Kein `crates.io`-Publish** für `klarvo-*` — privater Cargo-Workspace, keine externen Consumer.
- **Update-Mechanismus:** Zip-Replace (sectionOverrides.drop.update_strategy). Auto-Updater Phase 2+.

### Core API Surface

Publizierte Plugin-Traits mit Trait-Stability-Garantie (`successAnchorsPhase1.anchor1TraitStability`):
- `SttProvider` — Speech-to-Text-Plugin-Contract
- `CleanupStyle` — Output-Post-Processing-Contract
- `VadProvider` — Voice-Activity-Detection-Contract (ADR-0001)
- `PipelineStage` — Pipeline-Stage-Base-Trait

Weitere Core-APIs:
- **Pipeline-Manifest-TOML-Schema:** Input-Format für Compile-Contract-Resolution. Version im TOML-Header, Manifest-Parser validiert Version vor Resolution.
- **Event-Bus-API** (Core→Shell): `tauri-specta`-generierte Event-Typen als stable API. Core emittiert i18n-Keys, nicht Text (`memory/project_i18n_core_contract.md`).
- **Error-API:** `AppError { kind: AppErrorKind, user_message: I18nKey, cause: Option<Box<dyn Error>> }`. `kind`-Enum ist stable API.
- **Config-API:** `config.toml`-Parser + Default-Overlay. Phase-1-einziger Config-Mechanismus — kein Settings-UI, keine CLI-Args für Config-Overrides außer Diagnostics-Flags.

Referenzen: `sectionOverrides.add.core_library_contract`, ADR-0001, `memory/project_event_ts_ms_convention.md`.

### Reference Plugin Examples

- **`klarvo-plugin-groq`** — `SttProvider`-Trait-Impl gegen Groq-Whisper-Cloud-ASR. Canonical-Reference für STT-Trait-Implementation. Enthält MIC-Permission-Handling (`AppError::kind::PermissionDenied`).
- **`klarvo-plugin-verbatim`** — `CleanupStyle`-Trait-Impl für Verbatim-Output (pass-through + minimale Normalisierung). **Kein Polished-Style** Phase 1 (cf. `memory/feedback_polished_designschwaeche.md`). Canonical-Reference für Cleanup-Trait.
- **Rolle:** De-facto-Examples für Plugin-Author-Learning (kein separater Plugin-Author-Guide Phase 1).
- **AC pro Reference-Plugin:** Trait-Compliance-Tests + Headless-Integration-Tests (`sectionOverrides.add.plugin_contract_stories` + `.headless_testability`).

### V1 → V2 Migration Path

- **Scope Phase 1:** Parse v1-AppData (SQLite-History + `config.toml`) → V2-Writer in v2-AppData-Layout. Modul `v1_import` bereits in Phase 0 angelegt.
- **Trigger:** CLI-Invocation (`cargo xtask import-v1` o. ä.), **nicht** UI-getriggered Phase 1.
- **Nicht-Ziele:**
  - **Settings-Migration:** v1-Polished-Mode wird in v2 **nicht** migriert (v2 baut Polished neu, cf. `memory/feedback_polished_designschwaeche.md`). Verbatim-Settings migrieren 1:1.
  - **API-Key-Migration:** v1 Plain-SQLite → v2 Dev-Feature-Plain-Keystore oder OS-Keystore. User muss Key neu eingeben (Security-Hygiene, `memory/project_api_key_os_keystore_mvp.md`).
- Details: ADR-0004, `docs/migration/v1-to-v2.md`. v1-Tauri-Identifier: `com.klarvo.voice` (`memory/reference_klarvo_v1_tauri_identifier.md`).

### Documentation Surface

**rustdoc auf publizierten Traits ist Phase-1-Anforderung** (nicht nice-to-have). Jede Trait-Methode braucht Doc-Kommentar mit:
- **Intent** (was die Methode semantisch leistet)
- **Contract-Conditions** — Input-Format-Erwartung (z. B. Audio-Frame-Sample-Rate, Channel-Count), Return-Typ-Semantik, Error-Conditions, `ts_ms`-Bezug (session-relative monotone, cf. `memory/project_event_ts_ms_convention.md`)

**Begründung:** Innovation-Claims aus Step 6 (Manifest-as-Compile-Contract, Headless-Core-Contract) verlangen, dass Reader die Trait-Intent ohne externes Doc-Artefakt rekonstruieren können. Leere `pub trait SttProvider {}` mit implizitem „schau dir `klarvo-plugin-groq` an" reicht nicht — das ist Reverse-Engineering-by-Example, nicht Contract-Documentation.

**Phase-1-Scope:**
- rustdoc auf publizierten Traits + Event-/Error-/Config-APIs.
- Manifest-TOML-Schema-Doku als Rustdoc-Modul-Doc auf Manifest-Parser-Crate (kein separates `docs/`-Artefakt).

**Out-of-Scope Phase 1:**
- rustdoc-Coverage als CI-Gate (Qualitätsnorm + Reviewer-Erwartung in PRs, aber kein mechanisches Enforcement Phase 1).
- Plugin-Author-Guide, Pipeline-Authoring-Walkthrough, externe Doc-Site → Phase 2 (Validation-Persona-Onboarding).

### IDE / Editor Support

- **Phase 1:** `rust-analyzer` als Dev-Support für Plugin-Autoren. Keine Klarvo-spezifische IDE-Extension.
- **Manifest-TOML-Validation** läuft via `cargo build` (`manifest-strict`-xtask-Gate). Compile-Error-Feedback ist Phase-1-Validation-UX — konsistent mit `dogfooding-prototype`-Persona-Akzeptanz-Schwelle.
- **Deferred-to-Phase-2 (explizit, nicht stillschweigend):** VS-Code-JSON-Schema-Extension oder Taplo-TOML-LSP-Schema für Pipeline-Manifest-TOML. Phase-2-Trigger: Validation-Persona onboarded Plugin-Authors, braucht Editor-Support **vor** `cargo build`-Feedback.

### Windows System Integration

- **Tray-Icon** (Win32-API / Tauri-Tray-Plugin): Recording/Idle-State-Kommunikation per Icon-Swap. Deckt State-Visualisierung für Phase-1-Scope ab.
- **Global-Hotkey-Registration** (Win32-API): Single-Hotkey aus `config.toml`, Default `CommandOrControl+Shift+Space` (Placeholder, nicht CapsLock — explizit rejected in Scope-Lock).
- **Clipboard-Paste-Injection** (Win32-API): Auto-Paste via `SetClipboardData` + simuliertes `Ctrl+V`. Target-Apps canonisch (aus `visionInsights.canonicalReferenceWorkflow`): Notion-Web, VSCode, Windows-Mail-Client — **als Integration-Test-Anker**, nicht als Feature-Matrix-Zusicherung.
- **Single-Instance-Lock IN-scope:** Named Mutex via Windows-API oder `tauri-plugin-single-instance`. **AC:** Zweite Klarvo-Instanz beim Startversuch terminiert mit Log-Message (kein Popup — Dogfooding-Persona), bestehende Instanz bleibt funktional. **Begründung:** Konkurrierende Instanzen mit demselben Global-Hotkey führen zu „manchmal wirkt der Hotkey, manchmal nicht"-Verhalten, das Debug-Zeit frisst — selbst im Dogfooding nicht tolerabel.
- **MIC-Permission-Handling** (Forward-Looking-Risk aus Step 5): OS-Level-Prompt beim ersten Audio-Capture. `klarvo-plugin-groq` + Audio-Capture-Trait müssen permission-denied als `AppError::kind::PermissionDenied` mit `user_message: "error.permission.microphone"` behandeln.

**Out-of-Scope Phase 1:**
- **Autostart / Registry-Run-Key** → Phase-2-Settings-UI. Phase 1 = manueller Start.
- **Notification-Area-Badge-Overlays** → Phase-2-Pill-Bar übernimmt Recording-Visualisierung. Tray-Icon-State-Swap reicht Phase 1.
- **Windows-Shell-Extension** (z. B. Explorer-Context-Menu) → Phase 4+.
- **Windows-Toast-Notifications** → Phase 2+.

### Explicitly Skipped Sections (Audit Trail)

Transparenter Audit-Trail für Section-Skips, damit Phase-2/3-PRD-Authors nachvollziehen können, welche Sections gedropped wurden und warum.

| Section | Quelle | Skip-Grund |
|---|---|---|
| `platform_support` | desktop_app CSV | `sectionOverrides.drop`: Windows-only Phase 1, macOS/Linux non-goal, Android Phase 3. |
| `update_strategy` | desktop_app CSV | `sectionOverrides.drop`: Zip-Replace reicht für Dogfooding, Auto-Updater Phase 2+. |
| `offline_capabilities` | desktop_app CSV | `sectionOverrides.drop`: BYOK-Cloud-ASR Default, Offline/Local-Whisper P1/P2-Thema. |
| `visual_design` | developer_tool CSV `skip_sections` | UI-visueller Polish ist non-goal Phase 1, konsistent mit `dogfooding-prototype`-Tier. |
| `store_compliance` | developer_tool CSV `skip_sections` | Keine Store-Distribution Phase 1 (Microsoft Store Phase 2+, Play Store Phase 3+). |
| `web_seo` | desktop_app CSV `skip_sections` | Nicht anwendbar — Klarvo ist Desktop-App ohne Web-Surface. |
| `mobile_features` | desktop_app CSV `skip_sections` | Nicht anwendbar Phase 1 — Android ist Phase 3. |

## Project Scoping & Phased Development (Consolidated Digest)

Dieses Dokument konsolidiert Scoping-Entscheidungen, die über Frontmatter und Body-Sektionen verteilt sind, als Single-Page-Entry-Point für Phase-2/3/4-PRD-Authors, Sprint-Planner und Delegate-Agents. Keine neuen Behauptungen — alle Bullets sind Cross-References zu bereits committed Inhalten.

### In-Scope (Phase 1 — dogfooding-prototype)

- **Core-Library-Kontrakte** (Plugin-Traits `SttProvider`/`CleanupStyle`/`VadProvider`/`PipelineStage`, Pipeline-Manifest-Parser + Executor, Event-Bus, Error-API, Config-API) — §Core API Surface, `sectionOverrides.add.core_library_contract`, ADR-0001
- **Reference-Plugin-Crates** (`klarvo-plugin-groq`, `klarvo-plugin-verbatim`) — §Reference Plugin Examples, `sectionOverrides.add.plugin_contract_stories`
- **Canonical Reference-Workflow** (Hotkey → Groq-STT → Verbatim-Cleanup → Auto-Paste in Notion-Web / VSCode / Windows-Mail-Client als Integration-Test-Anker) — §Executive Summary, §Journey 1, `visionInsights.canonicalReferenceWorkflow`
- **Windows-Shell als Adapter** (Tray-Icon-State-Swap, Global-Hotkey, Clipboard-Paste, Single-Instance-Lock, MIC-Permission-Handling) — §Windows System Integration, `sectionOverrides.add.shell_surface_thin`
- **Compile-Contract-Enforcement** (Manifest-strict-Gate, Bindings-Drift-Gate, G3-i18n-core-no-user-strings, Trait-Stability) — §Innovation A+B, §Success Criteria Anker 1–3, `sectionOverrides.add.bindings_drift_gate` + `.headless_testability`
- **rustdoc auf publizierten Traits** als Qualitätsnorm (Intent + Contract-Conditions, kein CI-Gate Phase 1) — §Documentation Surface
- **v1→v2-Import** als isolierter Brownfield-Epic (CLI-invoked, kein UI-Pfad Phase 1) — §V1→V2 Migration Path, ADR-0004, `docs/migration/v1-to-v2.md`
- **3-Achsen-i18n** (ui/dictionary/output-language) in `config.toml` — `scopeLock`, `memory/project_i18n_three_axes.md`
- **Dev-Feature-Plain-Keystore** (OS-Keystore-Abstraktion vorbereitet, Release-Default ab Phase 4) — `scopeLock`, `memory/project_api_key_os_keystore_mvp.md`
- **Rolling-File-Log** (kein Debug-Export-Zip Phase 1, kein Remote-Reporting) — §Journey Requirements Summary, `memory/project_no_remote_telemetry.md`

### Deferred-to-Later-Phases

- **Phase 2** (validation-persona): Settings-UI / Pill-Bar / Onboarding-Flow, Plugin-Author-Guide + externe Doc-Site, Editor-Schema-Support für Manifest-TOML (VS-Code-JSON / Taplo-LSP), Autostart + Notification-Area-Badge + Windows-Toast, `cargo xtask set-key` (Keystore-CLI), Debug-Export-Zip (Settings-UI-gebunden) — §Product Scope.Growth Features, `personaTiering.phase2And3Target`
- **Phase 3**: Android-Port (native Kotlin + JNI-Dual-Surface, Tauri Mobile rejected), **AccessibilityService-Policy-Audit als Phase-3-Blocker** — ADR-0003, `memory/project_jni_dual_surface.md`, `memory/project_tauri_mobile_rejected.md`, §Domain Requirements.Forward-Looking-Risks
- **Phase 4+** (moat-persona): OS-Keystore als Release-Default (Plain-Keystore-Dev-Feature deaktiviert), Lizenz-System, vertikale Domain-Builds (Medical/Legal/Editorial/Accessibility via Cargo-Feature-Variants) — §Product Scope.Vision, `personaTiering.phase4Target`

### Out-of-Scope (Explicit Non-Goals)

- **Cross-Platform beyond Windows** Phase 1 (macOS/Linux non-goal, Android ist Phase 3) — `sectionOverrides.drop.platform_support`
- **Offline-ASR / Local-Whisper** (BYOK-Cloud-ASR ist Phase-1-Default) — `sectionOverrides.drop.offline_capabilities`
- **Auto-Updater** (Zip-Replace reicht Dogfooding) — `sectionOverrides.drop.update_strategy`
- **Admin-/Operator-/Support-Interfaces, externe API-Consumer-Endpoints** (kein Persona-Tier, keine externen Consumers) — §Executive Summary, §User Journeys (keine Admin-/Support-Journey)
- **Remote-Telemetry / Sentry-artiges Reporting** (widerspricht BYOK-Narrativ) — `memory/project_no_remote_telemetry.md`
- **UI-visueller-Polish, Store-Distribution, Web-Surface, Mobile-Features** (CSV skip_sections: `visual_design`, `store_compliance`, `web_seo`, `mobile_features`) — §Developer-Tool-Specific Requirements.Explicitly Skipped Sections
- **Polished-Style Cleanup** (v1-Regression) — `memory/feedback_polished_designschwaeche.md`

### Risk-Index

| Risk | Phase | Where Addressed |
|---|---|---|
| Trait-Stability (Plugin-API-breaking-changes) | 1→2 | §Innovation A, §Success Criteria Anker 1 |
| Strict-Error bei Unknown Pipeline Stage | 1 | §Journey 2 Failure-Mode-A, `memory/feedback_manifest_compile_contract.md` |
| Groq-API-Failure-Recovery | 1 | §Journey 2 Failure-Mode-B |
| Keystore-Miss beim Plugin-Init | 1 | §Journey 2 Failure-Mode-C |
| Headless-Coverage-Drift | 1 | §Success Criteria Anker 2, G3-Gate |
| CI-Gates werden Ceremonial | 1 | §Success Criteria Anker 3 |
| AccessibilityService-Play-Store-Policy | 3 | §Domain Requirements Forward-Looking-Risks |
| Vertical-Niche-Domain-Regulatorik | 4+ | §Domain Requirements Forward-Looking-Risks |
| Markt-/Resource-Risks | n/a | Nicht anwendbar: dogfooding-prototype, Andy solo |

## Functional Requirements

Dieses Kapitel ist **THE CAPABILITY CONTRACT** für Phase 1. UX-Designer, Architekten und Epic-Breakdown referenzieren exklusiv diese Liste — Features außerhalb dieser Liste existieren nicht in Phase-1-Scope. Gliederung nach Capability-Area, nicht nach Technology-Layer. Actor-Mix: `klarvo-core`/Windows-Shell/System für Library-Contracts, Andy/User für Direct-Interactions.

### A. Core Library & Plugin Composition

- **FR1:** `klarvo-core` exposes the `SttProvider`-Trait for Speech-to-Text-Transkription of audio-streams.
- **FR2:** `klarvo-core` exposes the `CleanupStyle`-Trait for Post-Processing of transcribed text.
- **FR3:** `klarvo-core` exposes the `VadProvider`-Trait for Voice-Activity-Detection on audio-streams.
- **FR4:** `klarvo-core` exposes the `PipelineStage`-Base-Trait for all Pipeline-Stage-Implementations.
- **FR5:** `klarvo-core` parses Pipeline-Manifest-TOML files into resolved Pipeline-Definitions at Boot-Time.
- **FR6:** `klarvo-core`'s Pipeline-Executor erroriert zur Boot-Zeit mit hartem Fehler (`AppError::kind::PipelineValidation`) bei unbekannten Stage-Types oder Type-Mismatches zwischen Stages. `warn!+skip` ist verboten (`memory/feedback_manifest_compile_contract.md`). Die Menge erlaubter Stage-Types wird zur Compile-Zeit durch Cargo-Features und `#[serde(tag = "type")]`-Enum-Variants bestimmt.
- **FR7:** `klarvo-core` exposes an Event-Bus-API emitting typed events via tauri-specta-generated bindings.
- **FR8:** `klarvo-core` emittiert Events und Errors ausschließlich als i18n-Keys, niemals als User-facing Strings (G3-enforced).
- **FR9:** `klarvo-core` exposes the `AppError`-Struktur mit `kind`-Enum, `user_message`-i18n-Key und Cause-Chain.
- **FR10:** Plugin-Crates (`klarvo-plugin-groq`, `klarvo-plugin-verbatim`) implementieren Core-Traits als Reference-Implementations.
- **FR11:** Pipeline-Manifest-TOML supportet Schema-Version-Header; Manifest-Parser validiert Schema-Version vor Type-Resolution.

### B. Audio Capture & Pipeline Execution (Canonical Reference-Workflow)

- **FR12:** User can initiate audio-capture via Global-Hotkey (Hold-to-Talk).
- **FR13:** `klarvo-core` captures audio from Windows-Microphone-Input during Hotkey-Hold.
- **FR14:** `klarvo-core` runs VAD on captured audio-stream during capture.
- **FR15:** `klarvo-core` sendet captured audio an konfiguriertes STT-Plugin zur Transkription.
- **FR16:** `klarvo-core` passes STT-Output durch konfiguriertes CleanupStyle-Plugin.
- **FR17:** `klarvo-core` delivers final Cleanup-Output an Shell-Adapter für Paste-Injection.
- **FR18:** Pipeline-Execution is fully exercisable without Shell-Dependency (headless-capable).

### C. Windows Shell Adapter

- **FR19:** Windows-Shell registriert Global-Hotkey aus `config.toml` (Default `CommandOrControl+Shift+Space`) via Win32-API.
- **FR20:** Windows-Shell displays Tray-Icon und swapped State zwischen Idle und Recording.
- **FR21:** Windows-Shell performs Auto-Paste in active Foreground-Window via Clipboard-Set + simuliertes `Ctrl+V`.
- **FR22:** Windows-Shell enforces Single-Instance-Lock via Named-Mutex; zweiter Startversuch terminiert mit Log-Entry (kein Popup), bestehende Instanz bleibt funktional.
- **FR23:** Windows-Shell handled MIC-Permission-denied als `AppError::kind::PermissionDenied` mit `user_message` `"error.permission.microphone"`.
- **FR24:** Windows-Shell consumiert ausschließlich tauri-specta-generierte TypeScript-Bindings; kein Ad-hoc-Core-Access.

### D. Configuration & Internationalization

- **FR25:** System loads User-Configuration aus `config.toml` at startup; kein Settings-UI, kein Runtime-Config-Override außer Diagnostics-Flags.
- **FR26:** System supports drei unabhängige i18n-Achsen: UI-Language (Shell-Strings), Dictionary-Language (Plugin-Dictionary-Lookups), Output-Language (Cleanup-Target-Language).
- **FR27:** Shell resolves i18n-Keys (emittiert durch Core) gegen Shell-owned Translation-Tables (z. B. `de.json`).

### E. Error Handling & Failure Recovery

- **FR28:** Pipeline-Strict-Error bei unknown-stage-type propagiert als `AppError::kind::PipelineValidation` mit actionable `user_message`-Key.
- **FR29:** Groq-API-Failures (5xx, Timeout, Network-Error) surfacen als `AppError::kind::UpstreamUnavailable` mit `user_message`-Key und gelogged Cause.
- **FR30:** Keystore-Miss bei Plugin-Init surfaced als `AppError::kind::KeyMissing` mit `user_message`-Key und Plugin-Identifier in Cause.
- **FR31:** Alle user-facing Errors werden als i18n-Keys emittiert; Shell resolves zu localized Strings at Display-Time.

### F. Developer Tooling & Gate-Enforcement

- **FR32:** `cargo xtask manifest-strict` fails Build bei Pipeline-Manifest-Verletzungen (unknown oder type-incompatible stages).
- **FR33:** `cargo xtask bindings-drift` fails Build wenn Shell Core-APIs konsumiert, die nicht in tauri-specta-generierten Bindings existieren.
- **FR34:** `cargo xtask lint-events` (G3) fails Build bei literalen User-facing Strings in Core-emittierten Events/Errors.
- **FR35:** `cargo xtask verify-release` (G2) enforced Release-Hardening-Invarianten (Phase-0-etabliert).
- **FR36:** Publizierte Core-Traits und Core-APIs sind via rustdoc dokumentiert mit Intent + Contract-Conditions (Quality-Norm, kein CI-Gate Phase 1).

### G. Observability & Diagnostics

- **FR37:** System schreibt structured Log-Einträge in Rolling-File-Log bei konfigurierbarer Verbosity.
- **FR38:** System transmittiert keine Telemetry, Errors oder Usage-Data an Remote-Endpoints.
- **FR39:** Uncaught Panics in `klarvo-core` werden als `level=ERROR` tracing-Events geloggt und landen im Rolling-File-Log (nicht Rust-Default-Stderr-Trace).
- **FR40:** `klarvo-core` exposes a `telemetry::export`-Module-Stub in Phase 1; full UI-triggered Zip-Generation (Debug-Export) is deferred to Phase 2.

### H. V1→V2 Data Migration

- **FR41:** Andy can invoke v1→v2-Import via CLI-Subcommand (`cargo xtask import-v1` oder Äquivalent).
- **FR42:** v1→v2-Import reads v1-AppData (SQLite-History + `config.toml`) und writes v2-AppData-Layout preserving Dictation-History-Records.
- **FR43:** v1→v2-Import migriert WEDER API-Keys (Security-Hygiene — User re-enters) NOCH Polished-Mode-Settings (v2 baut Polished neu in späterer Phase).

### I. Security & Key Management

- **FR44:** `klarvo-core` exposes a `KeyStore`-Abstraction-Trait für API-Key-Retrieval und -Storage, konsumiert von allen STT/LLM-Plugins at Init-Time.
- **FR45:** Phase-1-Default-Implementation ist `PlainSqliteKeyStore`, gated behind `dev-keystore`-Cargo-Feature; nicht enabled in Release-Builds.
- **FR46:** OS-Keystore-Implementations (Windows-Credential-Manager, macOS-Keychain, Linux-Secret-Service) sind als Scaffolds prepared für Phase-4-Release-Default-Swap, ohne `KeyStore`-Trait-Signature-Änderungen.

## Non-Functional Requirements

### Performance

- **NFR1:** End-to-End-Latency Hotkey-Release → Clipboard-Paste wird im Rolling-File-Log mit `ts_ms` erfasst und als Observable für Dogfooding-Regression-Detection exposed (keine harte SLA-Grenze Phase 1).
- **NFR2:** Audio-Capture-Thread droppt keine Samples während Hold-to-Talk, unabhängig von Downstream-Processing-Latency.
- **NFR3:** Alle Core-Events nutzen session-relative monotone `ts_ms` (Caller-Clock; nicht Wall-Clock, nicht Sample-Count) — ref ADR-0001/0003, `memory/project_event_ts_ms_convention.md`.

### Security & Privacy

- **NFR4:** Der `dev-plain-keystore`-Default ist explizit Security-Theater — eine Windows-ACL-Restriktion auf Current-User (Read/Write) mitigiert casual-access durch andere OS-User, schützt aber nicht gegen privileged-process-read oder Disk-Backup-Extraction. Echte API-Key-Protection kommt via OS-Keystore-Impl (Phase-4-Release-Default). Phase-1-Nutzer sind informiert, dass lokale Keys nicht als produktions-sicher behandelt werden sollen. Ref `memory/project_api_key_os_keystore_mvp.md`.
- **NFR5:** Audio-Daten und Transkriptions-Text werden NICHT im Rolling-File-Log persistiert; Log enthält nur Metadata (Event-Types, Error-Keys, Latency-Metrics).
- **NFR6:** System führt keine Outbound-Network-Calls außer zu user-konfigurierten Upstream-Providern (Groq, zukünftige LLM-APIs via BYOK). Kein Telemetry, kein Auto-Update-Check, keine Crash-Reports. Ref `memory/project_no_remote_telemetry.md`.

### Compliance

- **NFR7:** Klarvo agiert NICHT als Daten-Processor für GDPR-Zwecke — User ist Controller für Upstream-Provider-Usage via eigener API-Key-Account. Kein Klarvo-Backend, keine Data-Processing-Agreements erforderlich. (BYOK-Positioning-Consequence auf Compliance-Ebene; Template-Hook für Phase-4-Moat-Persona-Domain-Erweiterung wie Medical/Legal.)

### Compatibility & Integration

- **NFR8:** Windows 10 und Windows 11 sind supported Target-OS Phase 1; Windows 7/8/8.1 sind non-goal.
- **NFR9:** STT-Provider-Kompatibilität ist Phase-1-limited auf Groq-Whisper-Cloud-API; Trait-Stability (`SttProvider`) ermöglicht Phase-2-Einhängung alternativer Provider (Deepgram, Azure, etc.) ohne Trait-Signature-Änderung.

### Reliability

- **NFR10:** Runtime-Failures in plugin-dispatched Pipeline-Stages werden als `AppError` propagiert und im Rolling-File-Log erfasst; subsequent Hotkey-Triggers bleiben funktional. Plugin-Init-Failures zum Startup-Zeitpunkt werden vom Orchestrator als fatale Errors behandelt und führen zu kontrolliertem App-Beenden mit spezifischem Exit-Code (kein silent-crash).
- **NFR11:** Klarvo recovered graceful von Groq-API-Failures: User kann Hotkey erneut triggern nach Upstream-Error ohne App-Neustart.
