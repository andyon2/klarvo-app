# Architecture Decision Records (ADR) — Klarvo v1-ship

Dieses Verzeichnis enthält Architecture Decision Records für das **v1-ship-Produkt** (`src-tauri/`, `android/`, `src/`).

## Format

Pro ADR eine eigene Datei: `NNNN-kurztitel.md` mit aufsteigender Nummer.

Minimum-Struktur:

```
# ADR-NNNN: Titel

**Status:** Proposed | Accepted | Superseded by ADR-MMMM | Deprecated
**Date:** YYYY-MM-DD
## Context     (welches Problem, welche Rahmenbedingungen)
## Decision    (kurz und direkt, ggf. nummerierte Teilentscheidungen + verworfene Alternativen)
## Consequences (positiv / negativ / mitigations)
```

Stubs mit `Status: Proposed` dürfen anfangs unvollständig sein — Vollständigkeit kommt spätestens bei `Accepted`.

## Nummerierung & Provenienz

Die globale Sequenz wird fortgeführt: **0001–0014** dokumentieren die (seit Pivot 2026-05-29 geshelvte) **v2/klarvo-core-Architektur** und liegen auf dem v2-Branch — sie sind **nicht** auf `v1-ship` und betreffen einen anderen Codebase. **0015+** betreffen das v1-ship-Produkt. Numerische Eindeutigkeit über die gesamte Projekt-Historie verhindert „welche ADR-0001?"-Kollisionen.

## Referenz

`docs/robustness-audit-2026-05-30.md` ist der Brownfield-Audit-Input. ADRs dokumentieren Entscheidungen, die daraus folgen — ergänzend oder als bewusste Abweichung.

## Index (v1-ship)

| ADR | Titel | Status |
|-----|-------|--------|
| [0015](0015-state-file-write-convention.md) | Schreib-/Recovery-Konvention für State-Dateien (atomar + Backup) | Accepted |
| [0016](0016-android-path-parity-strategy.md) | Android-Pfad-Paritäts-Strategie — Linie + Wächter-Ausnahmen (A1 2026-06-10, A2 2026-06-12) | Accepted |
| [0017](0017-shared-core-stt-path.md) | Shared-Core STT-Pfad — ein Rust-STT-Request + Guards über JNI (Hard Rule) | Accepted |
| [0018](0018-android-bubble-rendering-tech.md) | Android Bubble Rendering Tech — View+Canvas vs ComposeView | Accepted |
| [0019](0019-cross-platform-design-ssot.md) | Cross-Platform Design-SSOT — Tokens (Codegen) · Farb-Semantik (rot=Abbrechen) · Interaktions-Parität | Accepted |
| [0020](0020-webview2-fixed-runtime-pin.md) | WebView2 Fixed-Runtime-Pin (.62) — Overlay-Occlusion-Regression Evergreen 149.69+ (durch 0021 superseded) | Superseded |
| [0021](0021-native-desktop-overlays.md) | Native Desktop Overlays — Pille+Preview als native Layered-Windows statt WebView2 (überholt 0020) | Accepted |
