---
title: "Product Brief: Klarvo"
status: "complete"
created: "2026-04-17"
updated: "2026-04-17"
inputs:
  - docs/rebuild-discussion.md
  - docs/project-overview.md
  - docs/v1-architecture-snapshot.md
  - memory/project_klarvo_v2_rebuild.md
  - memory/project_plugin_architecture.md
  - memory/feedback_polished_designschwaeche.md
  - memory/project_android_bypass.md
  - memory/project_ea_withdrawn.md
---

# Product Brief: Klarvo

## Executive Summary

Klarvo ist eine plattformübergreifende, erweiterbare Diktat-Anwendung: Drücken, sprechen, loslassen — der Text erscheint dort, wo der Cursor steht. Nach einer einjährigen Validierungsphase mit einer Tauri-basierten v1 (71 Features, funktionsfähige Pipeline, aber architektonisch überstrapaziert) wird Klarvo als **Klarvo 1.0** von Grund auf neu gebaut. Ziel ist eine produkt- und plattformstrategisch tragfähige Basis, die für die nächsten Jahre trägt — nicht ein weiterer inkrementeller Refit.

Der Rebuild steht auf drei Säulen: einem **Shared Rust Core**, der die gesamte Geschäftslogik plattform-agnostisch hält, **nativen Shells** (Tauri auf Windows, Kotlin auf Android, später Swift auf iOS und macOS) und einer **Trait-basierten Plugin-Architektur** mit deklarativem Pipeline-Manifest. Damit wird aus einem gewachsenen Nischen-Tool eine Plattform, auf der Provider, Ausgabeziele und Cleanup-Stile als austauschbare Plugins existieren.

Warum jetzt: Early Access wurde bewusst zurückgezogen, es gibt keine aktiven Tester, keinen Release-Druck. Genau dieses Fenster macht den sauberen Schnitt möglich — mit einer konservativ geschätzten Timeline von 3–5 Monaten Vollzeit (Puffer für JNI-Stolpersteine, Plattform-Überraschungen und Lernkurve eingeschlossen) und einem klar priorisierten MVP-Scope von ~40–45 der bisherigen 107 Features. Zeitliche Dringlichkeit kommt nicht vom Release-Druck, sondern vom Marktfenster: in den letzten Wochen tauchen zunehmend vergleichbare Diktat-Apps auf. Der horizontale Diktat-Markt sättigt. Klarvos Antwort darauf ist nicht, schneller zu werden — sondern architektonisch in eine Richtung zu investieren, die horizontale Wettbewerber nicht nachziehen können: vertikale Nischen-Varianten über die Plugin-Architektur.

## Das Problem

Menschen sprechen vier- bis fünfmal schneller als sie tippen. Trotzdem ist Diktat auf Windows und Android heute ein Kompromiss:

- **Microsoft Voice Typing / Windows Voice Access** ist eingebaut, aber qualitativ inkonsistent, nicht anpassbar und lässt keine Wahl über Modelle, Sprachen oder Post-Processing.
- **Dragon NaturallySpeaking** ist Enterprise-preislich, Windows-only und für heutige Workflow-Erwartungen (LLM-Cleanup, BYOK, Plugin-Extensibility) nicht gebaut.
- **Superwhisper / WhisperFlow / Aiko** sind exzellent — aber Apple-exklusiv. Ein Windows-User mit Android-Phone hat keinen äquivalenten Workflow.
- **Cloud-gebundene Lösungen** zwingen Nutzer in eine spezifische Anbieter-Beziehung (Audio verlässt das Gerät, ohne Wahlmöglichkeit über STT-Engine oder LLM).
- **Starre Post-Processing-Pipelines** formulieren aggressiv um, statt die eigene Stimme zu bewahren. Wer diktiert, will **seinen** Text — nur lesbar gemacht.
- **Englisch-zentrische Tools** behandeln Deutsch, Französisch, Spanisch als Nachgedanken. Für Power-User mit Domänen-Vokabular im Nicht-Englischen (Fach-Termini, Eigennamen) fehlt feingranulare Dictionary-Kontrolle.

Konkret für den Power-User: Wer täglich tausende Wörter schreibt — Mails, Dokumentation, Code-Kommentare, Notion-Seiten —, braucht einen Diktat-Flow, der **überall gleich funktioniert** (Desktop & Mobil), **auf die eigene Sprache und Domäne eingestellt werden kann** (Dictionary, Provider, Prompt-Style) und **die Kontrolle beim Nutzer lässt** (BYOK-API-Keys, eigenes Cloud-Backend, Offline-Option). Diese Kombination gibt es bisher nicht.

Klarvo v1 hat gezeigt, dass der Bedarf real ist und das Produkt-Konzept funktioniert. Was v1 nicht lösen konnte: die Architektur, die das auf vier Plattformen konsistent trägt. Aus dem Deep-Scan: **85 % Android-Bypass**, **~2.000 LOC zwischen Rust und Kotlin dupliziert**, **44 % des Rust-Codes plattform-fragmentiert**. Jedes neue Feature produziert Regressionen in benachbarten Features. Das ist kein Bug — das ist ein Framework-Mismatch zwischen Tauri v2 und einer Multi-Plattform-Strategie.

## Die Lösung

Klarvo ist ein Diktat-Tool, das sich wie ein nativer Teil jedes Betriebssystems anfühlt und technisch trotzdem eine einzige Plattform ist:

**Für den Nutzer**: Ein Hotkey (Desktop) oder eine persistente Bubble (Android). Drücken, sprechen, loslassen. Der Text erscheint im fokussierten Feld — ob das Slack, VS Code, ein Mail-Client oder ein Android-Messenger ist. Dazwischen: freie Wahl von STT-Engine (Groq Whisper, OpenAI, lokal), LLM-Cleanup (Verbatim, Chat, Polished), Cleanup-Style und Ausgabekanal.

**Für die Architektur**: Ein `klarvo-core`-Crate enthält die Pipeline (Audio → VAD → STT → Text-Filter → Cleanup → Output → History), das Plugin-System und die Datenpersistenz — vollständig headless, plattform-agnostisch, testbar ohne UI. Plattform-Shells sind minimal: Tauri hostet WebView + FloatingBar auf Windows, Kotlin hostet Overlay-Bubble und AccessibilityService auf Android, zukünftige Swift/Tauri-Shells für iOS und macOS sind additiv statt disruptiv.

**Für Power-User und Extension**: Jeder Erweiterungspunkt ist ein Rust-Trait (SttProvider, CleanupStyle, OutputTarget, VoiceCommandHandler, AudioFilter, TextFilter, …). Plugins sind eigene Crates, registrieren sich beim Start über eine zentrale `PluginRegistry`. Die Pipeline selbst ist ein deklaratives TOML-Manifest — Änderungen an der Reihenfolge oder Zusammensetzung brauchen keinen Code-Eingriff. Eine WASM-Erweiterung für Third-Party-Plugins bleibt als v2.x-Option offen, ohne jetzt blockiert zu werden.

## Was Klarvo unterscheidet

1. **Architektonische Konsistenz über Plattformen**: Kein anderer ernst zu nehmender Diktat-Client teilt seine Geschäftslogik als Library zwischen Windows, Android und iOS. Dieser Split ist der einzig nachhaltige Weg, Parität zu halten — und wird zum Moat, weil er langsam zu bauen und schwer nachzuholen ist.
2. **Echte Modularität statt konfigurierbarer Monolith**: Das Trait-basierte Plugin-System plus Pipeline-Manifest ist keine Marketing-Fassade. Neue Provider, neue Cleanup-Stile, neue Integrationen (Notion, Todoist, Obsidian) werden als eigenständige Crates entwickelt und per Cargo-Feature eingebunden. Das erlaubt auch klare Lizenz-Differenzierung (Free/Paid Builds) auf Build-Zeit-Ebene.
3. **„Deine Stimme, nur lesbar gemacht"**: Anders als Tools, die Diktiertes aggressiv in „professionelle Prosa" umformulieren, ist Klarvos neuer Default **Verbatim** — minimale Eingriffe, Stimme bleibt. Der optionale Polished-Stil wird vollständig neu konzipiert: Filler weg, Grammatik korrekt, Tonalität erhalten. Das spiegelt echte Nutzung statt Feature-Listen.
4. **Eigentümer-Kontrolle über Daten und Infrastruktur**: BYOK-API-Keys als Default, eigenes Cloud-Sync via Turso (libsql), optional volloffline. PolyForm Noncommercial 1.0.0 als faire, transparente Lizenz. Keine Lock-ins auf Anbieter-APIs oder zentrale Cloud-Dienste.
5. **Windows + Android als gleichrangiger MVP**: Die meisten Cross-Platform-Diktat-Tools starten Desktop-only und versuchen Mobil später — meist erfolglos, weil die Architektur es nicht trägt. Klarvo validiert den geteilten Core ab Tag 1 auf beiden Plattformen. Für Nutzer, die zwischen Laptop und Phone wechseln, ist das faktisch Alleinstellungsmerkmal.
6. **Multi-Language als First-Class-Feature**: Custom Dictionary, Output-Language-Wahl und Provider-gestützte Erkennung sind auf multilinguale Power-User ausgelegt — nicht als Übersetzungs-Overlay englischer Tools nachgerüstet.

## Zielnutzer

Klarvo richtet sich bewusst nicht an den Massenmarkt. BYOK-Reibung ist Akzeptanz-Merkmal, nicht Schwäche — sie filtert auf Nutzer, die die Kontrolle wollen.

**Primär — der schreibende Power-User (Andy-Archetyp)**: Produziert täglich große Textmengen für unterschiedliche Kanäle (Mails, Dokumentation, Code, Notion, Chat). Bewegt sich zwischen Windows-Desktop und Android-Phone. Will Kontrolle über Provider und Kosten (BYOK), hat Abneigung gegen Blackbox-Cloud-Dienste, schätzt klare Keyboard-Shortcuts. Misst Erfolg an: „Wie viel Zeit spart mir das pro Tag gegenüber Tippen?" Der „Aha-Moment" kommt beim ersten langen Mail-Entwurf, der in 90 Sekunden statt 8 Minuten steht.

**Sekundär — der modulare Entwickler**: Nutzt Klarvo als Basis für eigene Pipeline-Erweiterungen. Will Custom Cleanup-Prompts, eigene Output-Targets (Webhook, Notion-API, Todoist), eigene Voice-Commands. Klarvo ist für diesen Nutzer bewusst designed — Plugin-Ökosystem und Pipeline-Manifest sind genau sein Interface.

**Tertiär — Institute und Organisationen**: IT-Abteilung oder Einzelverantwortliche setzen Klarvo einmalig für ein Team auf (z. B. Kanzlei, Arztpraxis, Forschungsgruppe, Redaktion). Der einmalige BYOK- und Dictionary-Aufwand ist akzeptabel, weil er sich auf viele Endnutzer verteilt. Die Cargo-Feature-Architektur erlaubt dabei eigene Custom-Builds mit firmen-spezifischem Dictionary, domäneneigenen Cleanup-Stilen oder internen LLM-Endpoints — ohne Forks, ohne Fork-Maintenance.

**Signifikante Sekundärgruppe — Nutzer mit RSI und motorischen Einschränkungen**: Für diese Gruppe ist Diktat keine Komfort-Option, sondern primärer Eingabekanal. Architektonisch ist Klarvo first-class für sie: nativer AccessibilityService auf Android, saubere Keyboard-Shortcut-Ergonomie auf Windows, tiefes Dictionary für individuelle Sprech-Muster. Sie sind nicht Primärfokus, aber der Produkt-Value greift für sie stärker als für jede andere Gruppe — und das wird in Kommunikation und Onboarding sichtbar sein.

## Erfolgskriterien

**MVP-Abschluss (Phase 4, 3–5 Monate Vollzeit konservativ geschätzt)**:
- Pipeline läuft durchgehend auf Windows und Android mit allen Recording-Modi (Hold/Toggle/AutoStop) und kompletter Pill-Bar bzw. Bubble-UX.
- Ein Nutzer kommt vom Fresh Install in unter zwei Minuten zum ersten erfolgreichen Diktat (Groq + DeepSeek als BYOK-Default).
- Der v1-Import-Button migriert History, Dictionary, API-Keys und Hotkey-Config in einem Klick.
- Lizenz-System (HMAC, Trial, 30-Tage-Cache, 48h-Grace) funktioniert auf beiden Plattformen.

**Architektonische Metriken**:
- Null Geschäftslogik-Duplikation zwischen Rust-Core und Shells (Ziel: 0 LOC doppelt, verglichen mit v1s ~2.000).
- `klarvo-core` ist headless testbar mit sinnvoller Test-Coverage für die Pipeline.
- Neue Feature-Entwicklung (z. B. ein neuer STT-Provider oder Cleanup-Style) erfordert keine Änderung an Shell-Code.

**Regressions-Disziplin (Mechanismus statt Metrik)**:
- Der Erfolgsindikator ist nicht eine Zahl, sondern ein Verhaltenswechsel: Feature-Entwicklung fühlt sich nicht mehr an wie Brand-Löschen. Operationalisiert durch Specs-vor-Code-Workflow (BMad-gestützt), headless testbaren Core ab Phase 0, und Review-Disziplin bei jeder Pipeline-Änderung. Eine v1-Baseline existiert nicht sauber — die qualitative Aussage „Änderungen in einem Feature brechen keine anderen" ist der Erfolgs-Anker.

**Langfristig (Post-MVP, P1/P2)**:
- Zweites Gerät vom selben Nutzer via Turso-Sync nahtlos angeschlossen.
- Erste First-Party-Plugins jenseits der Provider (Webhook, Reformats) ohne Core-Änderungen ausgeliefert.
- iOS-Shell im P1/P2-Zeitraum — als Validierung, dass der Shared-Core-Ansatz skaliert.

## Markteintritt

Der horizontale Diktat-Markt sättigt sich sichtbar — in den letzten Wochen erscheinen zunehmend vergleichbare Apps. Klarvos Antwort ist nicht, mit ihnen in der Breite zu konkurrieren, sondern die Plattform-Natur als Markteintritts-Hebel zu nutzen:

- **Nischen-Strategie statt Massenmarkt**: Die Cargo-Feature-Architektur erlaubt Klarvo-Varianten mit spezifischem Fokus (z. B. medizinische Terminologie, juristische Aktendiktate, wissenschaftliche Fach-Vokabulare, redaktionelle Workflows, Accessibility-Fokus). Jede Nischen-Variante ist ein eigenes Build mit eigener Positionierung — nicht ein Feature im Settings-Panel.
- **Launch-Kanäle (MVP)**: Re-Aktivierung informierter v1-Tester als First-Wave-Kohorte; Product Hunt / Hacker News für Developer-Sekundärpersona; gezielte Ansprache in Multi-Platform-orientierten Communities (r/windows-Power-User, Android-Power-User-Foren).
- **Nischen-Anbahnung parallel zum MVP**: Konkrete Nischen-Märkte werden separat identifiziert und angegangen (B2B/Institutional-Channel, Domänen-spezifische Beratung / Partnerschaften). Diese Arbeitsstränge sind nicht Teil dieses Briefs, aber die Architektur ist explizit darauf ausgelegt.
- **Pricing-Signal**: BYOK + PolyForm-NC signalisiert Ernsthaftigkeit und filtert auf qualifizierte Nutzer. Reibung ist Akzeptanz-Merkmal, nicht Hürde, die entfernt werden müsste.

## Scope

**MVP — enthalten (~40–45 Features)**:
- Core Pipeline: STT, LLM-Cleanup, Auto-Paste + Clipboard-Fallback, Insert-Send, History-Save, Min-Duration, Hallucination-Filter, Prompt-Stripping, Output-Language.
- Recording-Modi: Hold, Toggle, AutoStop (Windows); alle fünf Android-Modi (Tap-HOLD/TOGGLE, Long-Press PTT/AUTOSTOP).
- Hotkey-System: 2 Slots, Pause/Resume, ShortcutRecorder, Active-Mode-Badge.
- Text-Processing: Verbatim (neuer Default), Chat, Polished (neu gebaut), Auto-Capitalize.
- Audio: Device-Selection, RMS-Silence-Detection, Live-Audio-Events, WAV-Encoding.
- Providers: Groq Whisper + DeepSeek LLM als Default, STT-Priority-List + Fallback, Live-API-Key-Validation.
- UI: Minimales Settings-Panel, komplette Floating Pill Bar (Windows), komplette Bubble (Android, alle 17 Features), Return-Focus, Tray, Onboarding-Stub, StylePicker, History-Panel.
- Sonstige: Custom Dictionary (capped), Lizenz-System (HMAC + Trial + 30-Tage-Cache + 48h-Grace), v1-Import-Button.

**P1 (kurz nach MVP)**: Auto Turso-Sync, OpenAI Whisper/LLM, Groq LLM, Reformate (Email/Bullets/Summary), Whisper-Mode (Gain), Stats-Panel, History-Search, Cost-Tracking, unlimitiertes Dictionary, Whisper-Model-Manager (small/medium), Webhook-Integration, Auto-Loop, UI-Scale, Autostart, Hot-Reload-Providers.

**P2 (Power-Features)**: Anthropic, OpenRouter, Provider-Model-Overrides, Custom Prompts, App-Profiles, Command-Mode/Hotkey, Voice-Notes, Snippets, Filler-Word-Analysis, Local Whisper Large + GPU/CUDA, alle Threshold-Configs.

**Explizit nicht in v2 (Deferred)**:
- Live-Transcription-Preview (war in v1 deaktiviert).
- Integrations-Panel als zentrale Kachel-UI (Integrationen kommen als Plugins).
- Early-Adopter 60-Tage-Grace (v1-spezifisch).
- Voice-Commands in der ursprünglichen v1-Form — wird als natives Plugin neu konzipiert.

**Plattform-Reihenfolge**: Windows + Android parallel im MVP. iOS nach MVP. macOS nach iOS. Linux opportunistisch.

## Vision

Drei Jahre nach Klarvo 1.0 ist Klarvo die Standard-Antwort für alle, die Plattform-unabhängiges, privatsphäre-freundliches, hackbares Diktat wollen — das, was Superwhisper für Apple-Nutzer ist, für den Rest der Welt.

Konkret:

- **Alle vier großen Plattformen ausgeliefert** (Windows, Android, iOS, macOS), mit konsistenter Kernfunktion und plattform-nativen Eigenheiten.
- **Ein lebendiges Plugin-Ökosystem**: First-Party-Plugins für Notion, Todoist, Obsidian, Slack; WASM-basierte Third-Party-Plugins als Erweiterung des Trait-Systems. Der Pipeline-Manifest wird zum Power-User-Interface, in dem sich Diktat-Workflows komplett customisieren lassen.
- **Voll offline-fähig** als ernsthafte Alternative: Local-Whisper-Large mit GPU/CUDA auf Windows, optimierte Mobil-Modelle auf Android und iOS. Privacy ist keine Nische, sondern gleichrangige Option.
- **Commercial Tier** (über Lemon Squeezy oder Äquivalent), das die nachhaltige Weiterentwicklung trägt, ohne die Free-Tier-Substanz auszuhöhlen — umgesetzt als Cargo-Feature-Split, nicht als künstliche Runtime-Limits.
- **Portfolio an Nischen-Varianten** auf derselben Architektur: Klarvo Medical, Klarvo Legal, Klarvo Accessibility o. Ä. — nicht als Marketing-Etiketten, sondern als echte Custom-Builds mit domänen-spezifischen Plugins und Dictionaries. Der horizontale Wettbewerb kann das architektonisch nicht nachbauen.
- **Accessibility-Leadership**: Klarvo ist die erste Wahl für Nutzer mit RSI oder motorischen Einschränkungen auf Windows und Android — mit offizieller Kommunikation, Community-Partnerschaften und gezielten UX-Features für diese Gruppe.
- **Eine bekannte, zitierte Architektur-Entscheidung** in der Diktat- und Voice-Tool-Community: „Shared Rust Core + native Shells + Trait-basierte Plugins" als Referenz-Muster für ähnliche Produkte.

Klarvo 1.0 ist der Fundament-Release, auf dem das alles aufbaut. Die Architektur-Investitionen der ersten 3–5 Monate zahlen sich in allen folgenden Quartalen aus.
