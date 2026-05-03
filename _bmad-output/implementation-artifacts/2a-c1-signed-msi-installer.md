---
name: Story 2.A.C1 — Signierter MSI-Installer
phase: 2
wave: A
story_id: "2.A.C1"
status: backlog
dependencies: []
adr_refs: []
source_ref: "Backlog Signierter Installer / MSI-Distribution"
deferred:
  date: "2026-05-01"
  reason: "Code-Signing-Zertifikat-Beschaffung (extern) noch nicht geklärt; Build-Track ohne Signing-Track liefert keinen Phase-2-A-Mehrwert über Status quo. Re-aktivieren sobald Cert-Pfad entschieden."
---

# Story 2.A.C1: Signierter MSI-Installer

## Outcome

Klarvo kann als MSI-Installer verteilt werden. Zwei parallele Tracks:

**Build-Track (sofort):** CI-Workflow baut unsigned MSI auf jedem Release-Tag.
Extern testbar (mit SmartScreen-Warning-Workaround für frühe Tester).

**Signing-Track (extern-warten):** Sobald Code-Signing-Zertifikat beschafft, wird
Signing-Step via CI-Secrets aktiviert. Signiertes MSI ohne SmartScreen-Block.

Phase-2-A-Erfolgs-Kriterium: unsigned MSI-Build in CI grün.
Signing ist kein Phase-2-A-Blocker für tägliche Engineering-Arbeit.

## Scope-Fence

**In-Scope:**
- `.github/workflows/release-msi.yml` (oder Erweiterung von `release.yml`) —
  MSI-Build-Job für Windows via `tauri build`
- Unsigned-MSI als CI-Artifact (GitHub Actions artifact upload)
- Dokumentierter Signing-Stub für spätere Cert-Integration

**Nicht-in-Scope:**
- Code-Signing-Zertifikat-Beschaffung (externes Prozessschritt)
- Signierter Build-Aktivierung (Follow-Up nach Cert-Beschaffung)
- Auto-Update-Mechanismus (Phase-4)
- NSIS vs WiX Toolset-Entscheidung falls Tauri v2 Default reicht (Default nehmen)

## Acceptance Criteria

### AC-1 — MSI-Build-Workflow existiert

**Given** `.github/workflows/` im Repo  
**When** Release-Tag (`v*`) gepusht wird (oder `workflow_dispatch`)  
**Then**
- Workflow-Datei vorhanden (z.B. `release-msi.yml`).
- Job läuft auf `windows-latest`.
- Trigger: `workflow_dispatch` + optional `push: tags: ['v*']`.

---

### AC-2 — `tauri build` produziert MSI

**Given** Workflow läuft auf `windows-latest`  
**When** `tauri build` (oder `npx tauri build`) ausgeführt wird  
**Then**
- MSI-Artifact wird erzeugt (Tauri v2 Default: `tauri-build` mit `bundle: [msi]` oder NSIS).
- Kein Signing-Fehler bei fehlendem Cert (unsigned-fallback ist default wenn kein Cert konfiguriert).
- Build-Exit-Code 0.

---

### AC-3 — MSI als GitHub Actions Artifact hochgeladen

**Given** erfolgreicher `tauri build`  
**When** Upload-Step läuft  
**Then**
- MSI-Datei via `actions/upload-artifact@v4` als Artifact hochgeladen.
- Artifact-Name enthält Version + Plattform (z.B. `klarvo-windows-x64-msi`).
- Artifact bleibt 30 Tage verfügbar (GitHub default).

---

### AC-4 — Signing-Stub dokumentiert

**Given** Workflow-Datei  
**When** Signing-Secrets verfügbar werden (`WINDOWS_SIGNING_CERT`, `WINDOWS_SIGNING_PASSWORD`)  
**Then**
- Workflow enthält auskommentierten Signing-Step mit Kommentar:
  "Aktivieren sobald Code-Signing-Cert in Secrets hinterlegt; Story 2.A.C1 Signing-Track."
- Oder: Signing-Step ist conditional auf `secrets.WINDOWS_SIGNING_CERT != ''`
  (wenn GitHub Actions conditional secrets check unterstützt).
- Keine hartkodierten Cert-Daten im Workflow.

---

### AC-5 — Tauri v2 `tauri.conf.json` MSI-Bundle-Config

**Given** `shells/windows/src-tauri/tauri.conf.json`  
**When** MSI-Build läuft  
**Then**
- `bundle.targets` enthält `"msi"` (Tauri v2 Default-Format) oder `"nsis"` falls MSI nicht
  direkt verfügbar — Tauri v2 Default nehmen, kein Custom-Toolset-Setup.
- `bundle.identifier` = `com.klarvo.voice` (aus `memory/reference_klarvo_v1_tauri_identifier`)
  oder bereits gesetzter v2-Identifier — nicht neu einführen, bestehenden Wert konsistent nutzen.

### AC-6 — AppUserModelID-Registrierung für Toast-Notifications (Story 9.4 Cross-Ref)

**Given** der MSI-Installer wird auf Windows ausgeführt  
**When** Installation abgeschlossen  
**Then**
- Eine AppUserModelID (AUMID) mit dem `bundle.identifier`-Wert wird im Windows-Registry
  registriert (Standard-Verhalten von WiX-Toolset für MSI mit Start-Menu-Shortcut, sofern
  `bundle.windows.shortcut` aktiviert ist — verifizieren).
- Konsequenz: `tauri-plugin-notification`-Toasts aus `NotificationService` (Story 9.4) werden
  vom Windows Action Center angenommen und sichtbar dargestellt. Ohne AUMID liefert
  notify-rust `Ok(())`, der Toast wird aber verworfen — siehe Story-9.4-AC-1-Visibility-Caveat.
- Smoke-Test in der MSI-Installations-Doku: nach Installation einen WaitAndType-Diktat
  durchführen und sicherstellen, dass der Toast `"Diktat bereit: …"` im Action Center erscheint.

---

## Technical Notes

- Tauri v2 Build für Windows: `tauri build` via `@tauri-apps/cli` NPM-Package oder
  `cargo tauri build`. Node.js muss im Workflow installiert sein falls npm-Variante.
- `actions/cache` für node_modules + cargo target (getrennte Keys von E1-Cache).
- WiX Toolset ist Tauri-Default für MSI auf Windows. Kein explizites WiX-Setup nötig
  (Tauri-CI-Pipeline bringt es mit).
- Für frühe externe Tester: Workaround-Note in Onboarding-Doc: "Unsigned MSI → SmartScreen-Block
  → 'More info' → 'Run anyway'." Unsigned ist Phase-2-A akzeptabel.
- Signing-Track (extern-warten): EV Code Signing Certificate (für SmartScreen-Trust-Score)
  kostet ~$300-500/Jahr; Standard OV Certificate reicht für basic Signing.
