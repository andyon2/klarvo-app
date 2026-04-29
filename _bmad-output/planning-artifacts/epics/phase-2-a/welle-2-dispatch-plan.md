---
title: Phase-2-A Welle-2 Parallel-Dispatch-Plan
phase: 2
wave: A
status: In-Progress (Story-Files vollständig, Impl-Dispatch ausstehend)
created: 2026-04-28
---

# Phase-2-A Welle-2 — Parallel-Dispatch-Plan

## Welle-Topologie (approved 2026-04-28)

```
Welle 1:  A4 (allein, Foundation)
               │
               ▼
Welle 2:  ┌── A8-Sub ──┐
          │   C2        │  (A4-abhängig, parallel untereinander)
          │   C3        │
          └────────────┘
          ┌── D2 ───────┐
          │   D3        │  (dependency-frei, sofort startbar)
          │   E1        │
          │   F2        │
          └────────────┘
          ┌── C1 ───────┐  (extern-async: Build startet sofort,
          └────────────┘   Signing wartet auf Code-Signing-Cert)

F1: kein Story-File. → docs/phase3-android-policy-audit.md + Backlog-Update.
```

**Trigger für Welle-2-Start:** A4 committed + grün in CI.  
**E1-Note:** `--exclude klarvo-bridge-jni` in CI solange F2 offen. F2-Closure entfernt das Flag
im selben Commit als JNI-Fix (kein Dual-Action-Race).

---

## Stream A — A4-abhängige Stories (nach A4-Implementierung, parallel)

AC-Writing-Session für A8-Sub / C2 / C3 als nächster `bmad-create-story-acs`-Step.
Gemeinsamer Trigger: A4-Story merged + `SettingsChangedEvent`-Typ + 8 Tauri-Commands
in Bindings vorhanden.

### 2.A.A8-Sub — Tray-Language-Switcher

**ADR-Refs:** ADR-0013 Sub-Decision 5 (`settings-changed`-Event-Shape)  
**Kickoff-Kontext:**
- Hört auf `settings-changed`-Event (key = `"ui.language"`).
- Ruft `tray.set_menu(...)` auf bei Match.
- Kein eigener Settings-Write — nur Reaktion auf emittiertes Event.
- `TauriSettingsEmitter` ist in A4 implementiert; A8-Sub konsumiert das Frontend-seitig.
- Tray-Menü-Structure: bestehendes Tray-Menü (Story-3.x) erweitern um Language-Submenu.
- i18n-Keys für Tray-Labels (mindestens `tray.language_switcher.label`) registrieren.

**Scope-Fence:** Kein neuer Settings-Write-Path. Nur Event-Listen + Tray-Update.

---

### 2.A.C2 — Hotkey-Konflikt-Erkennung

**ADR-Refs:** ADR-0013 Sub-Decision 4 (`set_hotkey_slot1`-Command),
ADR-0011 (Hotkey-Backend-Foundation)  
**Kickoff-Kontext:**
- Überschreibt / ergänzt den `set_hotkey_slot1`-Handler aus A4.
- Vor Settings-Write: `RegisterHotKey` (Win32) versuchen. Bei `HRESULT`-Fail:
  Settings-Mutation NICHT durchführen (kein Write, kein Emit).
- Fehler-Response: `AppError` mit spezifischem Kind (z.B. `AppErrorKind::HotkeyConflict`)
  → Toast via ADR-0009-Mechanismus.
- Conflict-Feedback-i18n-Key: `error.hotkey.conflict` (oder ähnlich, AC-Writing bestimmt Key).
- A4's generischer Error-Toast wird durch C2's spezifischen Key ersetzt (kein Breaking-Change,
  nur Key-Erweiterung).

**Scope-Fence:** Nur `hotkey.slot1.combo`-Field. Second-Hotkey-Slot (Phase-2-B A2) out-of-scope.

---

### 2.A.C3 — Live-Locale-Switch

**ADR-Refs:** ADR-0013 Sub-Decision 5, `project_i18n_three_axes`, `project_i18n_core_contract`  
**Kickoff-Kontext:**
- Frontend-seitig: `listen<SettingsChangedEvent>("settings-changed", ...)` für key = `"ui.language"`.
- Bei Match: i18n-Locale neu laden + alle locale-abhängigen Komponenten re-rendern
  (ohne App-Neustart).
- Tray-Seite: A8-Sub handled den Tray-Update; C3 handled nur WebView-Frontend-Komponenten.
- i18n-Loading-Layer ist aus Epic-4 (Story-4.2); C3 ruft deren Reload-API auf.
- 3-Achsen-Klarstellung: C3 switcht UI-Language (Axis 1). Dictionary-Language (Axis 2) +
  Output-Language (Axis 3) werden über Settings-Panel gespeichert; deren Live-Reload
  ist Pipeline-seitig (out-of-scope für C3).

**Scope-Fence:** Nur `ui.language`-Achse. Keine Dictionary/Output-Language-Hot-Reload.

---

## Stream B — Dependency-freie Stories (sofort startbar, parallel)

Diese Stories brauchen keine weiteren ACs-Sessions — Scope ist klar genug für direkte
Implementation-Dispatch mit den unten stehenden Story-Files.

### 2.A.D2 — Arc-Wrapping-Duplikat-Fix (Carry-Over F3)

→ Story-File: `story-2a-D2-arc-wrapping-fix.md` (schreiben + committen vor Dispatch)

**Source:** `_bmad-output/implementation-artifacts/deferred-work.md` F3 / Epic-3-Code-Review  
**Scope:** Entferne doppeltes `Arc<_>`-Wrapping in `klarvo-core` Audio-Pipeline-Pfad.
Structural-Cleanup only; kein API-Change, kein Behavior-Change. `cargo test -p klarvo-core`
muss grün bleiben.  
**Touch-Boundary:** `klarvo-core` Audio-Module (keine Überschneidung mit D3-Scope).

---

### 2.A.D3 — Graceful-Shutdown `pipeline_task.abort()` (Carry-Over F4)

→ Story-File: `story-2a-D3-graceful-shutdown.md`

**Source:** `_bmad-output/implementation-artifacts/deferred-work.md` F4 / Epic-3-Review,
`memory/project_shell_session_lifecycle` (7-Step-Topology, Step-7-Drop)  
**Scope:** Session-Teardown ruft `pipeline_task.abort()` mit `RecordingCompleted`-Guard
(Story-3.5-Precedent). Shutdown ist deterministisch; kein Panic-on-Drop bei laufender Pipeline.  
**Touch-Boundary:** Session-Lifecycle in Windows-Shell / `SessionOrchestrator`
(keine Überschneidung mit D2-Audio-Module).

---

### 2.A.E1 — Windows-Compile-CI-Gate (G6)

→ Story-File: `story-2a-E1-windows-compile-ci.md`

**Source:** Backlog `"Windows-Compile-CI-Gate"` / Epic-3-Followup  
**Scope:** GitHub-Actions-Workflow `.github/workflows/windows-ci.yml`. Kompiliert
Windows-Shell (`cargo build -p klarvo-windows-shell` oder äquivalent) auf jedem PR gegen
master. Build-Fail = PR geblockt.  
**--exclude-Flag:** `--exclude klarvo-bridge-jni` bleibt drin solange F2 offen;
Commit-Message dokumentiert Rationale (ref: `memory/project_jni_spike_scope`).
F2-Closure-Commit entfernt das Flag.  
**Keine Tests nötig:** Reine Compile-Validierung; kein `cargo test` in diesem Gate
(Test-Gate ist bestehende CI-Infrastruktur aus Epic-5).

---

### 2.A.F2 — JNI-Rate-Test-Regression-Triage

→ Story-File: `story-2a-F2-jni-regression-triage.md`

**Source:** `memory/project_jni_spike_scope` (2026-04-20-Regression),
`docs/adr/0003-jni-spike.md`, `klarvo-bridge-jni`-Crate  
**Scope:** Root-Cause der 20-Hz-Regression (Event-Emission nicht reproduzierbar auf HEAD).
Triage-Arbeit:
1. Reproduziere Fehler auf aktuellem HEAD.
2. Bisect: welcher Commit zwischen Spike-Baseline und HEAD bricht den Test?
3. Fix ODER Defer-Entscheidung.

**Zwei akzeptable Outcomes:**
- **Fix:** Commit mit grünem Rate-Test + ADR-0003-Amendment; danach E1 `--exclude` entfernen.
- **Defer:** Commit mit Triage-Report (`docs/jni-regression-triage.md`): Root-Cause,
  Defer-Begründung, Backlog-Phase-3-Update. `--exclude`-Flag in E1 bleibt dokumentiert.

**Scope-Fence:** Triage-only in Phase-2-A. Kein Full-JNI-Bridge-Rewrite — das ist Phase-3.
Per `feedback_spike_rigor`: Messwerte dokumentieren (200/200 events bei 20 Hz war Spike-Baseline
per ADR-0003).

---

### 2.A.C1 — Signierter MSI-Installer

→ Story-File: `story-2a-C1-signed-msi-installer.md`

**Source:** Backlog `"Signierter Installer / MSI-Distribution"`  
**Scope:** MSI-Build-Integration in CI (NSIS oder WiX-Toolset, je nach Tauri-v2-Default).
Signing-Step via CI-Secret (`WINDOWS_SIGNING_CERT`, `WINDOWS_SIGNING_PASSWORD`).

**Zwei parallele Tracks:**
- **Build-Track (sofort):** CI-Workflow für unsigned-MSI (dev-fallback ohne Cert). Lokal testbar.
- **Signing-Track (extern-warten):** Sobald Code-Signing-Cert beschafft, CI-Secrets hinterlegen;
  Signing-Step aktivieren.

**Erfolgs-Kriterium Phase-2-A:** Unsigned-MSI-Build in CI grün. Signierter Build nach
Cert-Beschaffung als Follow-Up (kein Phase-2-A-Blocker für tägliche Engineering-Arbeit).  
**Tester-Outcome:** Signiertes MSI ohne SmartScreen-Block ist das finale Ziel für externen Tester
(phaseable: unsigned für frühe Validation akzeptabel mit Workaround-Note im Onboarding-Doc).

---

## F1 — Play-Store-Policy-Audit (kein Story-File)

**Action:** `docs/phase3-android-policy-audit.md` anlegen + Backlog-Phase-3-Eintrag
`"Play-Store-Policy-Audit"` auf `status: Pending-Google-Response` setzen.

**Doc-Inhalt (minimal):**
- Datum der Einreichung (Google Developer Support)
- Konkrete Frage: Zulässigkeit von AccessibilityService für non-assistive-use-case
  (Klarvo: Hotkey-to-paste-workflow) unter aktueller Play-Store-Policy
- Response-Log (leer bis Google antwortet)
- Eskalations-Pfad bei abgelehnter Policy

**Engineering-Time-Cap:** < 2h. Rest ist externes Warten.  
**Phase-3-Dependency:** AccessibilityPasteBackend-Implementierung darf erst starten,
wenn Policy-Klärung positiv oder geklärter Workaround vorliegt
(`memory/project_play_store_phase3_blocker`).

---

## Story-Files-Status

| Story | File | Status |
|-------|------|--------|
| A4 | `story-2a-A4-settings-panel-foundation.md` | ✅ done |
| A8-Sub | `story-2a-A8-sub-tray-language-switcher.md` | ✅ Geschrieben (2026-04-29) |
| C2 | `story-2a-C2-hotkey-konflikt-erkennung.md` | ✅ Geschrieben (2026-04-29) |
| C3 | `story-2a-C3-live-locale-switch.md` | ✅ Geschrieben (2026-04-29) |
| D2 | `story-2a-D2-arc-wrapping-fix.md` | ✅ Geschrieben (2026-04-29) |
| D3 | `story-2a-D3-graceful-shutdown.md` | ✅ Geschrieben (2026-04-29) |
| E1 | `story-2a-E1-windows-compile-ci.md` | ✅ Geschrieben (2026-04-29) |
| F2 | `story-2a-F2-jni-regression-triage.md` | ✅ Geschrieben (2026-04-29) |
| C1 | `story-2a-C1-signed-msi-installer.md` | ✅ Geschrieben (2026-04-29) |
| F1 | (kein Story-File) | ✅ `docs/phase3-android-policy-audit.md` + Backlog updated (2026-04-29) |

**Status (2026-04-29):** Alle Welle-2-Story-Files geschrieben. Nächster Schritt: Impl-Dispatch.
- **Stream-B** (D2/D3/E1): sofort dispatchbar (dependency-frei, klarer Scope)
- **Stream-B** (F2): Triage-Story, separate JDK-Umgebung erforderlich
- **Stream-B** (C1): MSI-Build, Windows-CI-Setup erforderlich (nach E1)
- **Stream-A** (A8-Sub/C2/C3): A4-abhängig ✅, ready for Impl-Dispatch
