# Workflow & Konventionen

Wie Andy arbeitet, wie wir zusammenarbeiten, und Lektionen aus vergangenen Sessions.
Diese Datei wird bei jedem Sessionstart gelesen und waechst organisch.

## Andys Entwicklungsumgebung

- Windows-Laptop mit NVIDIA GPU
- GPU nur am Strom, CPU auf Akku
- WSL2 fuer CLI/Git, PowerShell fuer Builds
- Projektpfad Windows: `D:\Apps\voxlit\`
- Projektpfad WSL: `~/claude-projects/voxlit/`

## Build & Test -- Die 3 Wege

1. **`cargo test`** — Automatisierte Unit-Tests (aktuell 239). Laeuft in WSL.
2. **`tauri dev`** — Dev-Modus mit Hot-Reload. Fuer schnelles Frontend-Testen.
3. **`voxlit.exe` direkt** — Andys primaerer Test-Weg. `sync-and-build.ps1` in PowerShell ausfuehren, dann `D:\Apps\voxlit\src-tauri\target\release\voxlit.exe` starten. Kein Installer noetig. Das ist die fertige App wie sie beim Nutzer laeuft.

**Wichtig:** Andy nutzt fast immer Weg 3. Wenn er fragt "kann ich testen?", meint er: Ist ein frischer Build moeglich? Die Antwort ist immer `sync-and-build.ps1` auf Windows, dann `voxlit.exe` starten.

**Vor dem Build:** Immer zuerst `taskkill.exe /IM voxlit.exe /F` ausfuehren (geht aus WSL). Sonst schlaegt der Build fehl mit "Zugriff verweigert" weil die .exe noch laeuft. Der Befehl ist harmlos wenn die App nicht laeuft (gibt nur "nicht gefunden" Fehler).

**Builds selbst ausfuehren, nie Andy fragen.** Der Windows-Build geht aus WSL:
```bash
powershell.exe -Command "Get-Process voxlit -ErrorAction SilentlyContinue | Stop-Process -Force" 2>/dev/null
powershell.exe -ExecutionPolicy Bypass -File '\\wsl$\Ubuntu\home\andyon2\claude-projects\voxlit\scripts\sync-and-build.ps1' 2>&1
```
Andy nur einbeziehen wenn etwas interaktive Eingabe auf Windows erfordert.

## Lektionen (was schon mal schiefging)

- **2026-03-10:** Mehrfach nicht gewusst, dass Andy ueber `voxlit.exe` testet statt ueber `tauri dev` oder Installer. Fuehrte zu falschen Anweisungen ("du musst auf Windows bauen und installieren"). Merke: Die nackte .exe im Release-Ordner ist der Standard-Testweg.
- **2026-03-11:** Release-Notes nur mit Aenderungen der aktuellen Session erstellt, statt den vollen Changelog seit dem letzten oeffentlichen Release zu pruefen. Merke: Immer `git log v<LETZTE_VERSION>..HEAD` nutzen, um ALLE Aenderungen zu sammeln. Im Release-Skill unter Schritt 8 dokumentiert.
- **2026-03-11:** Dev-Tooling (UI Preview Mode) in Release-Notes aufgenommen. Gehoert nicht rein — Nutzer interessiert nur, was sich fuer sie aendert. Filter-Regel im Release-Skill ergaenzt.
- **2026-03-12:** Bei erstem GitHub-Issue sofort Explore-Agent losgeschickt ohne Andy zu fragen. Zu viel Eigeninitiative. Merke: Bei neuen Issues/Feedback nur zusammenfassen und Andy fragen was passieren soll. Keine eigenmaechtigen Untersuchungen oder Agent-Delegationen.
- **2026-03-15:** Wiederholt Andy gebeten den Windows-Build zu starten, obwohl das aus WSL geht (`powershell.exe -ExecutionPolicy Bypass -File ...`). Die Build- und Release-Skills dokumentieren den Befehl bereits. Merke: Builds immer selbst ausfuehren. Andy nur einbeziehen wenn etwas interaktive Eingabe auf Windows erfordert.
- **2026-03-18:** Agents vergessen bei Trait-Signatur-Aenderungen alle Konsumenten anzupassen (Regel 8). Passierte 2x: `paste_snippet` nach PasteResult-Umbau, `IsWindow(Option<HWND>)` nur auf Windows sichtbar. Merke: Nach Agent-Output immer grep nach geaenderten Signaturen/Typen. Besonders bei `#[cfg]`-Code der auf Linux nicht kompiliert wird.
- **2026-03-18:** Return-to-Current Bug: `capture_foreground_window()` wurde NACH `paste()` aufgerufen — da hatte Focus schon gewechselt. Timing bei Focus-Capture ist kritisch: immer VOR der Operation die den Focus aendert.
- **2026-03-18:** Manueller Version-Bump hat `package.json` vergessen (nur Cargo.toml + tauri.conf.json geaendert). Dadurch landete der Installer im falschen Dropbox-Ordner. Merke: Version-Bump NIE manuell machen — immer `/release` Skill nutzen, der alle drei Dateien synchron haelt (Cargo.toml, tauri.conf.json, package.json).
- **2026-03-18:** Andy verstand "Release" als "Publish auf GitHub Public" — ich meinte den ganzen Prozess (Bump+Build+Publish). Merke: Beim Wort "Release" immer klaeren was genau gemeint ist. Schritte explizit benennen.
- **2026-03-19:** Kumpel hatte persistent API-Fehler. Wir haben Auto-Fallback fuer fehlende Keys gebaut — war aber nicht das Problem. Er nutzte Anthropic als LLM-Provider, dessen Implementierung nie mit echten API-Calls getestet wurde (anderes API-Format als DeepSeek/Groq/OpenAI). Merke: Bei Bug-Reports von Testern ZUERST fragen welchen Provider / welche Settings sie nutzen. Keine ungetesteten Optionen im UI anbieten — Anthropic aus dem Dropdown entfernt bis verifiziert.
- **2026-03-20:** Android Turso-Sync-Fix (Hintergrundprozess statt blockierend) nicht in Release Notes aufgenommen, weil vorherige Session es nicht dokumentiert hatte. Merke: Jede Performance-Aenderung sofort in project-status.md festhalten. Bei Release immer Andy fragen ob etwas fehlt.
- **2026-03-20:** README mehrfach ueberarbeitet basierend auf Halbwissen statt Code-Audit. Fuehrte zu falschen Aussagen ("du brauchst zwei API-Keys" — stimmt nicht, Groq reicht alleine). Erst nach explizitem Code-Audit (Explore-Agent ueber alle Provider) war die Datenlage korrekt. Merke: README und Marketing-Texte IMMER gegen den tatsaechlichen Code verifizieren, nie aus dem Gedaechtnis schreiben. Feature-Inventar als zentrales Dokument fuehren.
- **2026-03-20:** README Feature-Sektionen erneut aus dem Kopf geschrieben statt feature-inventory.md als Checkliste zu nutzen. Ergebnis: Android Bubble-Beschreibung falsch (fehlende Keyboard-Detection, feste Gesten-Zuordnung statt konfigurierbarer Per-Geste-Modi). Feature-Inventar war 30 Minuten vorher erstellt worden — trotzdem nicht als Checkliste benutzt. Merke: Bei README/Marketing-Texten jede Sektion Zeile fuer Zeile gegen das Inventar abgleichen. Nicht "ich weiss das schon" — die Datei offen haben und abhaken.
- **2026-03-20:** Waveform-Animation war nie mit echten Audio-Daten verbunden — CSS bar-bounce Keyframes ueberdeckten die tatsaechlichen Level-Werte. Fiel erst auf als die Animation entfernt wurde und die Bars statisch waren (Amplitude zu niedrig: Sprach-RMS 0.01-0.05 bei Scaling x2.8 = nur 3px). Merke: Visuelle Features immer mit echten Daten testen, nicht nur pruefen ob "etwas passiert".
- **2026-03-20:** "Selbst bauen"-Sektion blind aus alter README uebernommen, ohne zu pruefen ob sie auf voxlit-app gehoert. Merke: Bei jeder README-Sektion fragen "Braucht ein Nutzer das?" — nicht einfach alte Inhalte kopieren.
- **2026-03-20:** Lizenz-Begriffe ("Open Source", "oeffentlich einsehbar") unscharf verwendet. Fuehrte zu Verwirrung ob der Quellcode frei nutzbar ist. Merke: Lizenz ist BSL 1.1 (source-available). Begriffe "Open Source", "MIT", "GPL" NIEMALS in user-facing Texten. Abgesichert durch CLAUDE.md Regel 13 + architecture.md + LICENSE Datei.
- **2026-03-21:** Explore-Agent meldete "Hallucination Blocklist nicht implementiert", obwohl sie bereits inline in `pipeline.rs` existierte (Funktion `is_hallucination` + Blocklist + Tests). Ergebnis: Falschen Status an Andy kommuniziert ("Task 4 offen"), rust-core Agent losgeschickt der dann ein Refactoring statt Neuimplementierung machte. Merke: Wenn ein Agent den Implementierungsstatus eines Features pruefen soll, reicht "Datei X hat keine Funktion namens Y" nicht — auch in benachbarten Dateien suchen (grep nach Funktionsname/Keyword im ganzen `src-tauri/src/`). Bei Statusberichten an Andy: Explore-Agent-Output gegen `git log` und `grep` gegenpruefen bevor man "nicht implementiert" sagt.
- **2026-03-21:** `cargo build --release` direkt aufgerufen statt `tauri build` via sync-and-build.ps1. Ergebnis: Binary ohne Frontend-Assets (App zeigt "localhost nicht erreichbar"). Danach hat `tauri build` die Binary als aktuell erkannt und nicht neu gebaut. Fix: Fingerprints loeschen + Binary loeschen. Merke: NIE `cargo build` direkt aufrufen fuer Tauri-Apps. Immer den Build-Skill oder sync-and-build.ps1 nutzen.
- **2026-03-21:** Voice Command Mode: Lokales Whisper (whisper.cpp) ist zu langsam fuer interaktive Command-Erkennung (15-46s pro Snippet mit `small`, sogar `base` braucht 10-15s auf CPU). Auch erkennt Whisper das Kunstwort "Voxlit" nicht — transkribiert stattdessen "slipstab", "Oh hello", "Let's start". Merke: Whisper ist ein General-Purpose-Transcriber, kein Keyword-Spotter. Fuer Echtzeit-Command-Erkennung Cloud-API (Groq) nutzen, nicht lokales Modell.
- **2026-03-21:** `tauri signer sign` haengt auf Windows/WSL nach "Signing without password." — reproduzierbar, sowohl standalone als auch innerhalb von `tauri build`. Betrifft alle Builds seit Signing eingefuehrt wurde. Root Cause: Vermutlich stdin/pipe-Bug in Tauri CLI v2 auf Windows. Workaround: `rsign` direkt aus WSL nutzen (`rsign sign -W -s key ...`), Output von .minisig nach base64 .sig konvertieren. Fix: `sync-and-build.ps1` setzt `TAURI_SIGNING_PRIVATE_KEY` NICHT mehr (verhindert Tauri-internen Signing-Versuch), ruft stattdessen `scripts/sign-installer.sh` via `wsl` auf. Merke: Wenn ein Tool persistent haengt, das darunterliegende Tool direkt nutzen statt endlos den Wrapper zu debuggen.

## Engineering-Prinzipien (aus AI-Hero-Research, 2026-03-19)

### Characterization Tests vor Refactoring
- Bestehendes Verhalten als Golden Master erfassen BEVOR refactored wird — verhindert versehentliche Verhaltensaenderungen
- Approval-Test-Ansatz: Aktuelles Output als "korrekt" snapshotten, dann gegen Snapshot testen
- Besonders wichtig wenn AI bestehenden Code umstrukturiert — implizites Verhalten wird nicht immer erkannt
- Fuer Rust: `insta` Crate fuer Snapshot-Tests

### Task-Groesse: 5-15 Minuten
- Ideale Task-Groesse fuer AI-Agents: 5-15 Minuten (human-equivalent), Maximum 35 Minuten
- METR-Studie: Verdopplung der Task-Dauer = 4-fache Fehlerrate
- Grosse Aufgaben in kleinere, abgeschlossene Einheiten aufteilen BEVOR der Agent startet

### 60% Kontextauslastungs-Limit
- Ab 40-60% Kontextauslastung degradiert die Output-Qualitaet merklich
- Frischer Kontext (neue Session) schlaegt ueberfuellten Kontext
- Sessions neu starten BEVOR Compaction einsetzt — danach ist Qualitaetsverlust bereits eingetreten
- CLAUDE.md kurz halten (≤60 Zeilen empirisch begruendet)

### TDD-Subagent-Pattern
- Separate Subagents fuer Test-Schreiben, Implementierung und Refactoring verhindern Context Pollution
- Red-Green-Refactor mit getrennten Agent-Kontexten: Test-Agent → Implementierungs-Agent → Refactoring-Agent
- Jeder Agent hat frischen Kontext und klaren Fokus
- Besonders wertvoll bei Rust wo Compile-Fehler + Test-Output den Kontext schnell fuellen

### Pre-Commit Hooks als Guardrails
- Pre-Commit Hooks zuverlaessiger als CLAUDE.md-Instruktionen — Hooks werden IMMER ausgefuehrt
- Drei-Schichten-Modell: Prompts verduennen / Skills bleiben frisch / Hooks sind extern und unbestechlich
- Empfohlene Hooks: `cargo fmt --check && cargo clippy`, `cargo test`, Commit-Message-Validation
- In `.claude/settings.json` konfigurieren (`hooks.pre_commit`), nicht in git hooks
