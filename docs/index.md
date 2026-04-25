# Klarvo — Projektdokumentation

Generiert: 2026-04-13 | Version: 0.5.0 | Scan: Deep

---

## Projektuebersicht

- **Typ:** Monolith (Desktop + Mobile App)
- **Primaere Sprache:** Rust (Backend) + TypeScript (Frontend) + Kotlin (Android)
- **Framework:** Tauri v2
- **Architektur:** Desktop-App mit IPC-Bridge (60+ Commands)
- **Plattformen:** Windows, Android

## Quick Reference

- **Frontend:** React 19 + TypeScript 5.8 + Vite 7 + Tailwind CSS 4
- **Backend:** Rust (tokio, rusqlite, reqwest, whisper-rs, llama-cpp-2, cpal)
- **Android:** Kotlin (Floating Bubble, AccessibilityService, Silero VAD, JNI)
- **Datenbank:** SQLite (lokal) + Turso (Cloud-Sync)
- **STT:** Groq Whisper (Cloud), OpenAI Whisper, whisper.cpp (Offline)
- **LLM:** DeepSeek, Groq, OpenAI, Anthropic, OpenRouter (Cloud) + llama.cpp/MNN (Offline)
- **Entry Point Frontend:** `src/main.tsx`
- **Entry Point Backend:** `src-tauri/src/lib.rs`
- **Entry Point Android:** `android/kotlin-src/.../MainActivity.kt`

## Onboarding

- [Sanity-Tester Onboarding](./sanity-tester-onboarding.md) — Erste Schritte für nicht-Developer-Tester (deutsch): Cold-Start-Pfad, Smoke-Checklist, bekannte Phase-1-Einschränkungen

## Generierte Dokumentation

- [Projektuebersicht](./project-overview.md) — Zusammenfassung, Features, Tech-Stack, Status
- [Architektur](./v1-architecture-snapshot.md) — Plattform-Architektur, Pipeline-Datenfluss, State-Management, Module, DB-Schema, Sicherheit
- [Source-Tree-Analyse](./source-tree-analysis.md) — Annotierter Verzeichnisbaum mit Beschreibungen
- [Komponenteninventar](./component-inventory.md) — React-Komponenten, Hooks, Rust-Module, Kotlin-Klassen, Design-System
- [Entwicklungs-Guide](./development-guide.md) — Voraussetzungen, Installation, Build, Tests, Konventionen

## Bestehende Dokumentation (im Repo)

- [README.md](../README.md) — Produkt-Beschreibung, Features, Downloads, Tech-Stack
- [Pre-Launch Checklist](../pre-launch-checklist.md) — v1.0 Launch-Planung (Features, Polish, QA)
- [Security Report](../security-report.txt) — Security-Audit (3 kritisch, 6 hoch, PI-Tests)

## Team-Dokumentation (externes Repo)

Das Team-Repo (`~/workspace/teams/klarvo/`) enthaelt:

- **14 Knowledge-Dateien:** architecture.md, product-strategy.md, api-providers.md, feature-inventory.md, workflow.md, competitors.md u.a.
- **20+ Briefings:** Geplante Features (Offline Whisper, Onboarding, i18n, Lizenz, VAD Overhaul)
- **Projekt-Status:** project-status.md (Echtzeit-Tracker, aktive Blocker, naechste Sessions)
- **5 Agenten:** rust-core, ui-dev, android-platform, product-strategist, web-dev
- **14 Skills:** /build, /run-tests, /commit-progress, /release, /debug-error u.a.
- **UX-Research:** Onboarding-Simulationen, Konkurrenz-Analyse

## Einstieg

### Fuer Entwicklung:
1. Diesen Index lesen fuer Gesamtueberblick
2. [Entwicklungs-Guide](./development-guide.md) fuer Setup
3. [Architektur](./v1-architecture-snapshot.md) fuer technischen Tiefgang
4. [Komponenteninventar](./component-inventory.md) fuer Modul-Referenz

### Fuer Brownfield-PRD:
1. Diesen Index als Eingabe fuer den PRD-Workflow verwenden
2. [Architektur](./v1-architecture-snapshot.md) fuer technische Constraints
3. [Source-Tree-Analyse](./source-tree-analysis.md) fuer Datei-Navigation

### Fuer Feature-Arbeit:
- **Frontend-Feature:** `src/components/` + `src/hooks/` + `src/tauri-commands.ts`
- **Backend-Feature:** `src-tauri/src/commands/` + relevantes Modul
- **Android-Feature:** `android/kotlin-src/` (eigener HTTP-Stack, NICHT Tauri IPC)
- **Full-Stack:** Alle drei Schichten + [Architektur](./v1-architecture-snapshot.md) Pipeline-Datenfluss
