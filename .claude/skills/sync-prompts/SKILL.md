---
name: sync-prompts
description: Vergleicht LLM-Cleanup-Prompts in Rust (src-tauri/src/llm/mod.rs) und Kotlin (android/kotlin-src/com/dikta/voice/DiktaApi.kt) und zeigt Unterschiede. Aufrufen ohne Argumente.
allowed-tools: Read, Bash, Grep
context: fork
model: haiku
---

Vergleiche die LLM-Cleanup-Prompts in Rust und Kotlin auf Abweichungen.

## Hintergrund

Dikta hat Prompt-Logik in ZWEI Dateien dupliziert:
- **Rust:** `src-tauri/src/llm/mod.rs` (Desktop-Pipeline)
- **Kotlin:** `android/kotlin-src/com/dikta/voice/DiktaApi.kt` (Android-Pipeline)

Bei Aenderungen muessen BEIDE Dateien synchron gehalten werden. Dieser Skill macht Drift sichtbar.

## Vorgehensweise

1. Lies `src-tauri/src/llm/mod.rs` -- suche nach System-Prompt-Strings und Cleanup-Prompt-Templates (typisch: String-Literale mit Anweisungen wie "clean up", "polish", "verbatim", Stil-Definitionen)

2. Lies `android/kotlin-src/com/dikta/voice/DiktaApi.kt` -- suche nach denselben Prompt-Patterns (typisch: String-Templates fuer API-Calls an DeepSeek/OpenAI)

3. Vergleiche:
   - Sind die System-Prompts identisch?
   - Sind die Stil-Definitionen (Polished/Verbatim/Chat) identisch?
   - Gibt es Prompts die nur in einer Datei existieren?
   - Gibt es subtile Unterschiede (Wortlaut, Reihenfolge, fehlende Anweisungen)?

4. Melde strukturiert:

```
PROMPT-SYNC CHECK

Status: SYNCHRON | DRIFT ERKANNT

[Falls Drift:]
Unterschied 1: [Was in Rust steht] vs. [Was in Kotlin steht]
Unterschied 2: ...

[Falls nur in einer Datei:]
Nur in Rust: [Prompt-Fragment]
Nur in Kotlin: [Prompt-Fragment]

Empfehlung: [Welche Datei als Referenz nehmen, was angepasst werden muss]
```

Falls beide Dateien synchron sind: Kurze Bestaetigung, keine Details noetig.
