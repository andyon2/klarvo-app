# Projektstatus

## Aktueller Stand
Projekt-Setup abgeschlossen. Agent-Team, Skills, Knowledge-Base und Projektstruktur stehen. Noch kein Code geschrieben. Naechster Schritt: Tauri v2 Projekt initialisieren und Phase 1 (Foundation) starten.

## Offene Aufgaben
- [ ] Tauri v2 Projekt initialisieren (create-tauri-app mit React + TypeScript)
- [ ] Rust-Abhaengigkeiten definieren (Cargo.toml: cpal, whisper-rs, reqwest, serde, thiserror, anyhow, rusqlite)
- [ ] Frontend-Abhaengigkeiten definieren (package.json: Tailwind, React)
- [ ] API-Provider-Docs vollstaendig recherchieren (/research-api fuer Groq + DeepSeek)
- [ ] Audio-Capture Proof-of-Concept (Mikrofon aufnehmen, WAV speichern)
- [ ] Groq Whisper API Proof-of-Concept (WAV hochladen, Transkript zurueckbekommen)
- [ ] DeepSeek Cleanup Proof-of-Concept (Roh-Text rein, bereinigter Text raus)
- [ ] Minimale Overlay-UI (Recording-Indikator)

## Entscheidungen
- [2026-03-06]: Tech-Stack: Tauri v2 + React/TS + Rust. Begruendung: Ein Codebase fuer Windows + Android, Rust fuer Performance, Web-Frontend fuer Flexibilitaet.
- [2026-03-06]: API-Strategie: Groq Whisper (STT) + DeepSeek (Cleanup) primaer, whisper.cpp lokal als Fallback.
- [2026-03-06]: GPU nur am Strom, Cloud-API auf Akku.
- [2026-03-06]: Android IME-Architektur (Tauri-Bridge vs. Native Kotlin) wird in Phase 5 entschieden.

## Naechste Session
Phase 1 starten: Tauri v2 Projekt initialisieren, Abhaengigkeiten einrichten, Audio-Capture PoC bauen. Zuerst /research-api fuer Tauri v2 Setup ausfuehren, damit wir die aktuellsten Docs haben.
